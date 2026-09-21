// Copyright (c) 2026, Alliance for Open Media. All rights reserved
//
// This source code is subject to the terms of the BSD 3-Clause Clear License
// and the Alliance for Open Media Patent License 1.0. If the BSD 3-Clause Clear
// License was not distributed with this source code in the LICENSE file, you
// can obtain it at www.aomedia.org/license/software-license/bsd-3-c-c. If the
// Alliance for Open Media Patent License 1.0 was not distributed with this
// source code in the PATENTS file, you can obtain it at
// www.aomedia.org/license/patent.

#![forbid(unsafe_code)]

//! A spatially-oriented headphone/binaural renderer (`EAR` library port).
//!
//! Direct Rust port of `liboar/src/renderer/ear/ear.c` and `ear.h`.
//! Responsible for applying exact multi-channel to multi-channel (M2M) and
//! High-Order Ambisonics to multi-channel (H2M) rendering to channel-based
//! and scene-based audio elements according to target loudspeaker layouts.

use crate::common::definitions::{
    AudioElementConfig, HighOrderAmbisonics, Layout, OarError, PlanarBufferMut, PlanarBufferRef,
    SampleRate, MAX_OUTPUT_CHANNEL_COUNT,
};
use crate::renderer::audio_renderer_api::AudioRenderer;
use crate::renderer::ear::ambisonic_to_channel_renderer::{
    element_renderer_get_h2m_matrix, element_renderer_render_h2m, H2mRdr, HoaWithLfeFlag,
};
use crate::renderer::ear::channel_to_channel_renderer::{
    element_renderer_get_m2m_custom_matrix, element_renderer_get_m2m_matrix,
    element_renderer_render_m2m, element_renderer_render_m2m_custom, IamfCustomSpLayout,
    IamfPredefinedSpLayout, IamfSpLayout, IamfSpLayoutData, M2mRdr,
};

/// A spatially-oriented headphone/binaural renderer.
///
/// `EarRenderer` applies target-layout conversion matrices (`M2M` and `H2M`)
/// to spatialized planar audio outputs, matching `ear_renderer_t` of `ear.c`.
#[derive(Debug, Clone)]
pub struct EarRenderer {
    /// Target output layout for this renderer instance.
    pub target_layout: Layout,
    /// Sample rate configured in Hz.
    pub sample_rate: SampleRate,
    /// Active rendering matrix mode configured for the current audio element.
    active_mode: Option<EarRenderingMode>,
}

/// Enumeration of configured internal rendering algorithms and tables.
#[derive(Debug, Clone)]
pub enum EarRenderingMode {
    /// Predefined channel-to-channel matrix (`sp_type == 0`).
    ChannelPredefined(M2mRdr),
    /// Custom subset layout channel matrix (`sp_type == 1`).
    ChannelCustom {
        /// Matrix structure containing matching strides.
        matrix: M2mRdr,
        /// Physical channel mapping index slice.
        channels_map: Vec<usize>,
    },
    /// High-Order Ambisonics to multichannel matrix (`H2M`).
    Scene(H2mRdr),
}

impl EarRenderer {
    /// Creates a new `EarRenderer` configured with default Stereo output and 48 kHz sample rate.
    ///
    /// # Examples
    ///
    /// ```
    /// use roar::renderer::ear::ear_renderer::EarRenderer;
    ///
    /// let rdr = EarRenderer::new();
    /// assert_eq!(rdr.sample_rate.value(), 48000);
    /// ```
    pub fn new() -> Self {
        Self::with_params(Layout::Stereo, SampleRate::new(48000).unwrap())
    }

    /// Creates a new `EarRenderer` with specified target output layout and sample rate.
    pub fn with_params(target_layout: Layout, sample_rate: SampleRate) -> Self {
        Self { target_layout, sample_rate, active_mode: None }
    }

    /// Creates a new `EarRenderer` with specified target output layout using default 48 kHz sample rate.
    pub fn with_output_layout(target_layout: Layout) -> Self {
        Self::with_params(target_layout, SampleRate::new(48000).unwrap())
    }
}

impl Default for EarRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioRenderer for EarRenderer {
    /// Configures an audio element within the renderer (`_open` equivalent).
    fn add_element(&mut self, _id: u32, config: &AudioElementConfig) -> Result<(), OarError> {
        self.active_mode = None;
        if !is_predefined_output_layout(self.target_layout) {
            return Err(OarError::InvalidParameter);
        }
        let out_system = self.target_layout;
        let (out_lfe1, out_lfe2) = layout_lfe_indices(self.target_layout);
        let out_pre = IamfPredefinedSpLayout { system: out_system, lfe1: out_lfe1, lfe2: out_lfe2 };
        let out_layout =
            IamfSpLayout { sp_type: 0, layout: IamfSpLayoutData::Predefined(&out_pre) };

        match *config {
            AudioElementConfig::ChannelBased(cb) => {
                // Determine if this is a full predefined input layout or custom subset
                if is_predefined_input_layout(cb.layout) {
                    let (in_lfe1, in_lfe2) = layout_lfe_indices(cb.layout);
                    let in_pre =
                        IamfPredefinedSpLayout { system: cb.layout, lfe1: in_lfe1, lfe2: in_lfe2 };
                    let in_layout =
                        IamfSpLayout { sp_type: 0, layout: IamfSpLayoutData::Predefined(&in_pre) };
                    let mut m2m = M2mRdr {
                        in_system: Layout::Mono,
                        out_system: Layout::Mono,
                        mat: &[],
                        m: 0,
                        n: 0,
                    };
                    if element_renderer_get_m2m_matrix(&in_layout, &out_layout, &mut m2m).is_ok() {
                        self.active_mode = Some(EarRenderingMode::ChannelPredefined(m2m));
                        return Ok(());
                    }
                }

                // Try subset custom layout check (`_rid_sp_labels_map`)
                if let Some((base_system, flags)) = layout_to_subset_custom(cb.layout) {
                    let in_cust = IamfCustomSpLayout { system: base_system, sp_flags: flags };
                    let in_layout =
                        IamfSpLayout { sp_type: 1, layout: IamfSpLayoutData::Custom(&in_cust) };
                    let mut m2m = M2mRdr {
                        in_system: Layout::Mono,
                        out_system: Layout::Mono,
                        mat: &[],
                        m: 0,
                        n: 0,
                    };
                    let mut chmap_buf = [0usize; MAX_OUTPUT_CHANNEL_COUNT];
                    if element_renderer_get_m2m_custom_matrix(
                        &in_layout,
                        &out_layout,
                        &mut m2m,
                        &mut chmap_buf,
                    )
                    .is_ok()
                    {
                        self.active_mode = Some(EarRenderingMode::ChannelCustom {
                            matrix: m2m,
                            channels_map: chmap_buf[..m2m.m].to_vec(),
                        });
                        return Ok(());
                    }
                }

                Err(OarError::NotSupported)
            }
            AudioElementConfig::SceneBased(sb) => {
                let hin = HoaWithLfeFlag { order: sb.order, lfe_on: false };
                let mut h2m = H2mRdr {
                    in_order: HighOrderAmbisonics::Zoa,
                    out_system: Layout::Mono,
                    channels: 0,
                    lfe1: 0,
                    lfe2: 0,
                    mat: &[],
                    m: 0,
                    n: 0,
                };
                if element_renderer_get_h2m_matrix(&hin, &out_pre, &mut h2m).is_ok() {
                    self.active_mode = Some(EarRenderingMode::Scene(h2m));
                    Ok(())
                } else {
                    Err(OarError::NotSupported)
                }
            }
            AudioElementConfig::ObjectBased(_) => Err(OarError::NotSupported),
        }
    }

    /// Processes input planar buffers (`_render` equivalent) and writes spatialized outputs.
    ///
    /// Executes allocation-free on the real-time audio thread by packing slice
    /// references into a fixed stack array (`[MAX_OUTPUT_CHANNEL_COUNT]`).
    fn render(
        &mut self,
        inputs: PlanarBufferRef<'_, '_>,
        output: &mut PlanarBufferMut<'_, '_>,
    ) -> Result<(), OarError> {
        if output.num_samples() != inputs.num_samples() {
            return Err(OarError::InvalidParameter);
        }
        let (expected_in_channels, expected_out_channels) = match &self.active_mode {
            Some(EarRenderingMode::ChannelPredefined(m2m)) => (m2m.m, m2m.n),
            Some(EarRenderingMode::ChannelCustom { matrix, .. }) => (matrix.m, matrix.n),
            Some(EarRenderingMode::Scene(h2m)) => (h2m.m, h2m.channels),
            None => return Err(OarError::NotSupported),
        };
        if inputs.num_channels() != expected_in_channels
            || output.num_channels() != expected_out_channels
            || inputs.num_channels() > MAX_OUTPUT_CHANNEL_COUNT
            || output.num_channels() > MAX_OUTPUT_CHANNEL_COUNT
        {
            return Err(OarError::InvalidParameter);
        }

        match &self.active_mode {
            Some(EarRenderingMode::ChannelPredefined(m2m)) => {
                element_renderer_render_m2m(m2m, inputs, output)
            }
            Some(EarRenderingMode::ChannelCustom { matrix, channels_map }) => {
                element_renderer_render_m2m_custom(matrix, inputs, output, channels_map)
            }
            Some(EarRenderingMode::Scene(h2m)) => element_renderer_render_h2m(h2m, inputs, output),
            None => Err(OarError::NotSupported),
        }
    }
}

pub use crate::renderer::ear::channel_to_channel_renderer::{
    is_predefined_input_layout, is_predefined_output_layout, layout_lfe_indices,
    layout_to_subset_custom,
};

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::definitions::{
        ChannelBasedConfig, HighOrderAmbisonics, Samples, SceneBasedConfig,
    };
    use googletest::prelude::*;

    /// Helper to run a function with PlanarBufferRef and PlanarBufferMut with the given buffers.
    fn with_planar_buffers<F, R>(
        in_buf: &[f32],
        in_channels: usize,
        out_buf: &mut [f32],
        out_channels: usize,
        num_samples: usize,
        f: F,
    ) -> R
    where
        F: FnOnce(PlanarBufferRef<'_, '_>, &mut PlanarBufferMut<'_, '_>) -> R,
    {
        assert_eq!(in_buf.len(), in_channels * num_samples);
        assert_eq!(out_buf.len(), out_channels * num_samples);

        let mut in_chunks = in_buf.chunks_exact(num_samples);
        let in_slices: Vec<&[f32]> = (0..in_channels).map(|_| in_chunks.next().unwrap()).collect();
        let in_planar = PlanarBufferRef::new(
            &in_slices,
            in_channels,
            Samples::new(num_samples as u32).unwrap(),
        )
        .unwrap();

        let mut out_chunks = out_buf.chunks_exact_mut(num_samples);
        let mut out_slices: Vec<&mut [f32]> =
            (0..out_channels).map(|_| out_chunks.next().unwrap()).collect();
        let mut out_planar = PlanarBufferMut::new(
            &mut out_slices,
            out_channels,
            Samples::new(num_samples as u32).unwrap(),
        )
        .unwrap();

        f(in_planar, &mut out_planar)
    }

    #[gtest]
    fn ear_renderer_add_element_channel_predefined_sets_active_mode_and_renders() {
        let mut rdr = EarRenderer::new();
        let config = AudioElementConfig::ChannelBased(ChannelBasedConfig {
            layout: Layout::Mono,
            downmix_info: None,
            rendering_config: None,
        });
        let in_buf = [10.0f32];
        let mut out_buf = [0.0f32; 2];
        let add_result = rdr.add_element(10, &config);
        let render_result =
            with_planar_buffers(&in_buf, 1, &mut out_buf, 2, 1, |inputs, outputs| {
                rdr.render(inputs, outputs)
            });

        assert_ok!(add_result);
        assert_ok!(render_result);
        expect_that!(out_buf[0], near(7.071068f32, 1e-4));
        expect_that!(out_buf[1], near(7.071068f32, 1e-4));
    }

    #[gtest]
    fn ear_renderer_add_element_scene_based_sets_active_mode_and_renders() {
        let mut rdr = EarRenderer::with_params(Layout::Layout51, SampleRate::new(48000).unwrap());
        let config = AudioElementConfig::SceneBased(SceneBasedConfig {
            order: HighOrderAmbisonics::Zoa,
            rendering_config: None,
        });
        let in_buf = [2.0f32];
        let mut out_buf = [0.0f32; 6];
        let add_result = rdr.add_element(20, &config);
        let render_result =
            with_planar_buffers(&in_buf, 1, &mut out_buf, 6, 1, |inputs, outputs| {
                rdr.render(inputs, outputs)
            });

        assert_ok!(add_result);
        assert_ok!(render_result);
        expect_that!(out_buf[0], near(0.6526711f32, 1e-4));
        expect_that!(out_buf[1], near(0.6526767f32, 1e-4));
        expect_that!(out_buf[2], near(0.43026306f32, 1e-4));
        expect_that!(out_buf[3], eq(0.0f32));
    }

    #[gtest]
    fn ear_renderer_add_element_unsupported_layout_returns_not_supported() {
        let mut rdr = EarRenderer::new();
        let config =
            AudioElementConfig::ObjectBased(crate::common::definitions::ObjectBasedConfig {
                num_objects: 1,
                rendering_config: None,
            });

        let result = rdr.add_element(30, &config);

        expect_that!(result, eq(Err(OarError::NotSupported)));
    }

    #[gtest]
    fn ear_renderer_add_element_channel_custom_sets_active_mode_and_renders() {
        let mut rdr = EarRenderer::with_params(Layout::Layout51, SampleRate::new(48000).unwrap());
        let config = AudioElementConfig::ChannelBased(ChannelBasedConfig {
            layout: Layout::StereoS,
            downmix_info: None,
            rendering_config: None,
        });
        let in_buf = [1.0f32, 2.0f32];
        let mut out_buf = [0.0f32; 6];
        let add_result = rdr.add_element(40, &config);
        let render_result =
            with_planar_buffers(&in_buf, 2, &mut out_buf, 6, 1, |inputs, outputs| {
                rdr.render(inputs, outputs)
            });

        assert_ok!(add_result);
        assert_ok!(render_result);
        expect_that!(out_buf[0], eq(0.0f32));
        expect_that!(out_buf[1], eq(0.0f32));
        expect_that!(out_buf[2], eq(0.0f32));
        expect_that!(out_buf[3], eq(0.0f32));
        expect_that!(out_buf[4], eq(1.0f32));
        expect_that!(out_buf[5], eq(2.0f32));
    }
}
