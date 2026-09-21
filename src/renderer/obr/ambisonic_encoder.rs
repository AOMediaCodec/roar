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

//! Higher-Order Ambisonic (HOA) encoder.
//!
//! Encodes mono input channels with spatial properties (azimuth, elevation, gain, distance)
//! into planar HOA representations using spherical harmonics.

pub mod associated_legendre_polynomials_generator;
use crate::renderer::obr::audio_buffer::simd_utils::{
    ramp_multiply_and_accumulate, scalar_multiply_and_accumulate,
};
use crate::renderer::obr::audio_buffer::AudioBuffer;
use crate::renderer::obr::common::ambisonic_utils::{
    acn_sequence, get_num_periphonic_components, sn3d_normalization,
};
use crate::renderer::obr::common::constants::NEGATIVE_120DB_IN_AMPLITUDE;
pub use associated_legendre_polynomials_generator::AssociatedLegendrePolynomialsGenerator;

use crate::common::definitions::{Degrees, Distance, LinearGain, Radians};

/// Parameters defining the spatial location and gain of a single audio source.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SingleSourceParams {
    /// Linear volume gain factor.
    pub gain: LinearGain,
    /// Spatial azimuth angle.
    pub azimuth: Degrees,
    /// Spatial elevation angle.
    pub elevation: Degrees,
    /// Distance of the source from the listener.
    pub distance: Distance,
}

/// Current and target spatial parameters for an active audio source.
///
/// Used to interpolate (ramp) spatial updates smoothly across rendering blocks.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SourceProperties {
    /// Current spatial parameters used at the block start.
    pub current: SingleSourceParams,
    /// Target spatial parameters to reach by the block end.
    pub target: SingleSourceParams,
}

/// Spatial encoder for mapping mono inputs to Ambisonic channels.
///
/// Computes and applies a spherical harmonics encoding matrix to map spatialized inputs
/// to Higher-Order Ambisonics format. Guaranteed to be allocation-free on real-time loops.
#[derive(Debug, Clone, PartialEq)]
pub struct AmbisonicEncoder {
    number_of_input_channels: usize,
    number_of_output_channels: usize,
    ambisonic_order: usize,
    sources: Vec<Option<SourceProperties>>,
    alp_generator: AssociatedLegendrePolynomialsGenerator,
    encoding_matrix: Vec<f32>,
    scratch_column_start: Vec<f32>,
    scratch_column_end: Vec<f32>,
    scratch_needs_interpolation: Vec<bool>,
    scratch_sh_coeffs: Vec<f32>,
    scratch_alp_values: Vec<f32>,
    scratch_col_temp: Vec<f32>,
}

impl AmbisonicEncoder {
    /// Constructs a new `AmbisonicEncoder` instance.
    pub fn new(number_of_input_channels: usize, ambisonic_order: usize) -> Self {
        assert!(number_of_input_channels > 0);
        assert!(ambisonic_order > 0);
        let number_of_output_channels = get_num_periphonic_components(ambisonic_order as i32);
        let alp_generator =
            AssociatedLegendrePolynomialsGenerator::new(ambisonic_order as i32, false, false);
        let num_alp = alp_generator.get_num_values();

        let mat_size = number_of_output_channels * number_of_input_channels;
        Self {
            number_of_input_channels,
            number_of_output_channels,
            ambisonic_order,
            sources: vec![None; number_of_input_channels],
            alp_generator,
            encoding_matrix: vec![0.0; mat_size],
            scratch_column_start: vec![0.0; mat_size],
            scratch_column_end: vec![0.0; mat_size],
            scratch_needs_interpolation: vec![false; number_of_input_channels],
            scratch_sh_coeffs: vec![0.0; number_of_output_channels],
            scratch_alp_values: vec![0.0; num_alp],
            scratch_col_temp: vec![0.0; number_of_output_channels],
        }
    }

    /// Sets the spatial parameters of a single directional mono source channel.
    ///
    /// # Parameters
    /// * `input_channel` - Input channel index.
    /// * `gain` - Linear gain factor.
    /// * `azimuth` - Azimuth angle in degrees.
    /// * `elevation` - Elevation angle in degrees.
    /// * `distance` - Distance of the source.
    pub fn set_source(
        &mut self,
        input_channel: usize,
        gain: LinearGain,
        azimuth: Degrees,
        elevation: Degrees,
        distance: Distance,
    ) {
        assert!(input_channel < self.number_of_input_channels);

        let params = SingleSourceParams { gain, azimuth, elevation, distance };
        if let Some(prop) = &mut self.sources[input_channel] {
            if prop.target.gain == gain
                && prop.target.azimuth == azimuth
                && prop.target.elevation == elevation
                && prop.target.distance == distance
            {
                return;
            }
            prop.target = params;
        } else {
            self.sources[input_channel] =
                Some(SourceProperties { current: params, target: params });
            if gain.0 < NEGATIVE_120DB_IN_AMPLITUDE {
                self.zero_encoding_column(input_channel);
            }
            return;
        }

        if gain.0 < NEGATIVE_120DB_IN_AMPLITUDE {
            self.zero_encoding_column(input_channel);
        }
    }

    fn zero_encoding_column(&mut self, input_channel: usize) {
        let offset = input_channel * self.number_of_output_channels;
        for i in 0..self.number_of_output_channels {
            self.encoding_matrix[offset + i] = 0.0;
        }
    }

    /// Processes planar audio data from `input_buffer` into encoded HOA format in `output_buffer`.
    pub fn process_planar_audio_data(
        &mut self,
        input_buffer: &AudioBuffer,
        output_buffer: &mut AudioBuffer,
    ) {
        assert_eq!(self.number_of_input_channels, input_buffer.num_channels());
        assert_eq!(self.number_of_output_channels, output_buffer.num_channels());
        assert_eq!(input_buffer.num_frames(), output_buffer.num_frames());

        let num_out = self.number_of_output_channels;

        let params_equal = |a: &SingleSourceParams, b: &SingleSourceParams| -> bool {
            if (a.gain.0 - b.gain.0).abs() > 1e-6 {
                return false;
            }
            if (a.distance.value() - b.distance.value()).abs() > 1e-6 {
                return false;
            }
            if (a.elevation.0 - b.elevation.0).abs() > 1e-3 {
                return false;
            }
            let delta = ((b.azimuth.0 - a.azimuth.0 + 540.0) % 360.0) - 180.0;
            delta.abs() <= 1e-3
        };

        self.scratch_column_start.fill(0.0);
        self.scratch_column_end.fill(0.0);
        self.scratch_needs_interpolation.fill(false);

        let mut temp_sources =
            [None; crate::renderer::obr::common::constants::MAX_SUPPORTED_NUM_INPUT_CHANNELS];
        let num_chans = self.number_of_input_channels;
        temp_sources[..num_chans].copy_from_slice(&self.sources[..num_chans]);

        for (in_ch, prop_opt) in temp_sources.iter().enumerate().take(num_chans) {
            if let Some(prop) = prop_opt {
                self.fill_column_to_temp(&prop.target);
                self.scratch_column_end[in_ch * num_out..(in_ch + 1) * num_out]
                    .copy_from_slice(&self.scratch_col_temp);

                if !params_equal(&prop.current, &prop.target) {
                    self.fill_column_to_temp(&prop.current);
                    self.scratch_column_start[in_ch * num_out..(in_ch + 1) * num_out]
                        .copy_from_slice(&self.scratch_col_temp);
                    self.scratch_needs_interpolation[in_ch] = true;
                } else {
                    let start_idx = in_ch * num_out;
                    let end_idx = (in_ch + 1) * num_out;
                    self.scratch_column_start[start_idx..end_idx]
                        .copy_from_slice(&self.scratch_column_end[start_idx..end_idx]);
                    self.scratch_needs_interpolation[in_ch] = false;
                }
            }
        }

        output_buffer.clear();

        for in_ch in 0..self.number_of_input_channels {
            let start_offset = in_ch * num_out;
            let start_slice = &self.scratch_column_start[start_offset..start_offset + num_out];
            let end_slice = &self.scratch_column_end[start_offset..start_offset + num_out];

            // Skip inactive input channels whose gain is zero for all spherical harmonic
            // components.
            if start_slice.iter().all(|&x| x == 0.0) && end_slice.iter().all(|&x| x == 0.0) {
                continue;
            }

            let input_channel = input_buffer.channel(in_ch).as_slice();
            let needs_interp = self.scratch_needs_interpolation[in_ch];

            for (out_ch, (&start_gain, &end_gain)) in
                start_slice.iter().zip(end_slice.iter()).enumerate()
            {
                if start_gain == 0.0 && end_gain == 0.0 {
                    continue;
                }

                let mut out_ch_view = output_buffer.channel_mut(out_ch);
                let output_channel = out_ch_view.as_mut_slice();
                if !needs_interp || start_gain == end_gain {
                    scalar_multiply_and_accumulate(end_gain, input_channel, output_channel);
                } else {
                    ramp_multiply_and_accumulate(
                        start_gain,
                        end_gain,
                        input_channel,
                        output_channel,
                    );
                }
            }
        }

        self.encoding_matrix.copy_from_slice(&self.scratch_column_end);

        for prop in self.sources.iter_mut().flatten() {
            prop.current = prop.target;
        }
    }

    fn fill_column_to_temp(&mut self, p: &SingleSourceParams) {
        if p.gain.0 < NEGATIVE_120DB_IN_AMPLITUDE {
            self.scratch_col_temp.fill(0.0);
            return;
        }

        self.get_sh_coeffs_internal(p.azimuth, p.elevation);
        self.apply_within_head_sh_coefficients_damping(p.distance.value(), 0.1);
        for i in 0..self.number_of_output_channels {
            self.scratch_col_temp[i] = self.scratch_sh_coeffs[i] * p.gain.0;
        }
    }

    fn get_sh_coeffs_internal(&mut self, azimuth: Degrees, elevation: Degrees) {
        let azimuth_rad = Radians::from(azimuth).0;
        let elevation_rad = Radians::from(elevation).0;

        self.alp_generator.generate_into(elevation_rad.sin(), &mut self.scratch_alp_values);
        self.scratch_sh_coeffs.fill(0.0);

        for degree in 0..=self.ambisonic_order as i32 {
            for order in -degree..=degree {
                let row = acn_sequence(degree, order);
                if row == -1 {
                    continue;
                }
                let idx = row as usize;
                let angle = order as f32 * azimuth_rad;
                let last_term = if order >= 0 { angle.cos() } else { (-angle).sin() };
                let alp_idx = self.alp_generator.get_index(degree, order.abs());
                self.scratch_sh_coeffs[idx] = sn3d_normalization(degree, order)
                    * self.scratch_alp_values[alp_idx]
                    * last_term;
            }
        }
    }

    fn apply_within_head_sh_coefficients_damping(
        &mut self,
        distance: f32,
        normalized_head_radius: f32,
    ) {
        if normalized_head_radius <= 0.0 || distance >= normalized_head_radius {
            return;
        }
        let remapped = (distance / normalized_head_radius).clamp(0.0, 1.0);
        for degree in 0..=self.ambisonic_order as i32 {
            let damping_factor = remapped.powi(degree);
            for order in -degree..=degree {
                let row = acn_sequence(degree, order);
                if row != -1 {
                    self.scratch_sh_coeffs[row as usize] *= damping_factor;
                }
            }
        }
    }
}

// ===== Tests =====

#[cfg(test)]
mod test {

    use super::*;
    use googletest::prelude::*;

    const EPSILON: f32 = 1e-6;
    const BUFFER_SIZE: usize = 1;
    const NUMBER_OF_INPUT_CHANNELS: usize = 1;
    const AMBISONIC_ORDER: usize = 3;

    /// Helper to run an encoder test. It creates an encoder of order 3, sets a single source at
    /// the given azimuth and elevation, processes a 1-sample planar audio buffer containing 1.0,
    /// and verifies that the resulting periphonic (3D) Ambisonics coefficients match the expected
    /// values.  Expected coefficients match the SN3D-normalized Spherical Harmonics values for this
    /// direction, corresponding to the outputs from the C reference implementation.
    fn run_encoder_test(azimuth: f32, elevation: f32, expected_coefficients: &[f32]) {
        let mut encoder = AmbisonicEncoder::new(NUMBER_OF_INPUT_CHANNELS, AMBISONIC_ORDER);
        encoder.set_source(
            0,
            LinearGain(1.0),
            Degrees(azimuth),
            Degrees(elevation),
            Distance::new(1.0).unwrap(),
        );
        let mut input_buffer = AudioBuffer::new(NUMBER_OF_INPUT_CHANNELS, BUFFER_SIZE);
        input_buffer.channel_mut(0).as_mut_slice().fill(1.0);
        let num_out = get_num_periphonic_components(AMBISONIC_ORDER as i32);
        let mut output_buffer = AudioBuffer::new(num_out, BUFFER_SIZE);

        encoder.process_planar_audio_data(&input_buffer, &mut output_buffer);

        for ch in 0..num_out {
            expect_that!(output_buffer[ch][0], near(expected_coefficients[ch], EPSILON));
        }
    }

    #[gtest]
    fn test_one_sample_buffer_one_source_at_zero_degrees() {
        let expected = vec![
            1.0,
            0.0,
            0.0,
            1.0,
            0.0,
            0.0,
            -0.5,
            0.0,
            0.8660254,
            0.0,
            0.0,
            0.0,
            0.0,
            -0.61237244,
            0.0,
            0.7905694,
        ];

        run_encoder_test(0.0, 0.0, &expected);
    }

    /// Tests encoding a source at azimuth -45 degrees and elevation 30 degrees.
    #[gtest]
    fn test_one_sample_buffer_one_source_at_angled_position() {
        let expected = vec![
            1.0,
            -0.61237244,
            0.5,
            0.61237244,
            -0.6495191,
            -0.53033006,
            -0.125,
            0.53033006,
            0.0,
            -0.3630922,
            -0.72618437,
            -0.09375,
            -0.4375,
            0.09375,
            0.0,
            -0.3630922,
        ];

        run_encoder_test(-45.0, 30.0, &expected);
    }

    /// Tests encoding a source at azimuth 12 degrees and elevation 0 degrees.
    #[gtest]
    fn test_one_sample_buffer_one_source_at_small_azimuth() {
        let expected = vec![
            1.0,
            0.2079117,
            0.0,
            0.9781476,
            0.35224426,
            0.0,
            -0.5,
            0.0,
            0.79115355,
            0.46468505,
            0.0,
            -0.127_319_4,
            0.0,
            -0.5989906,
            0.0,
            0.6395841,
        ];

        run_encoder_test(12.0, 0.0, &expected);
    }

    /// Tests encoding a source at azimuth 120 degrees and elevation -90 degrees (directly down).
    #[gtest]
    fn test_one_sample_buffer_one_source_at_extreme_elevation() {
        let expected =
            vec![1.0, 0.0, -1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0];

        run_encoder_test(120.0, -90.0, &expected);
    }
}
