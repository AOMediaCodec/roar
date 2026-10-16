// Copyright (c) 2026, Alliance for Open Media. All rights reserved
//
// This source code is subject to the terms of the BSD 3-Clause Clear License
// and the Alliance for Open Media Patent License 1.0. If the BSD 3-Clause Clear
// License was not distributed with this source code in the LICENSE file, you
// can obtain it at www.aomedia.org/license/software-license/bsd-3-c-c. If the
// Alliance for Open Media Patent License 1.0 was not distributed with this
// source code in the PATENTS file, you can obtain it at
// www.aomedia.org/license/patent.

// Because these are only used in the integration tests, they appear to be dead code.
#![allow(dead_code)]

/// Helpers for ROAR integration tests.
use aom_roar::c_types::{
    audio_element_rendering_config_t, audio_element_type_t, coordinate_type_t,
    def_max_number_of_objects, downmix_info_t, oar_audio_element_config_t,
    oar_audio_element_config_union_t, oar_binaural_filter_profile_t, oar_channel_based_config_t,
    oar_config_t, oar_headphones_rendering_mode_t, oar_hoa_t, oar_layout_t,
    oar_metadata_object_positions_t, oar_metadata_object_positions_union_t, oar_metadata_t,
    oar_metadata_type_t, oar_metadata_union_t, oar_object_based_config_t, oar_param_type_t,
    oar_scene_based_config_t, oar_t, parameter_set_t, polar_t,
};
use aom_roar::ffi::{roar_create, roar_destroy};
use aom_roar::{ObjectPosition, PlanarBufferMut, PlanarBufferRef, PolarCoordinate, Samples};

/// Convenience wrapper for creating and destroying the C API test renderer.

pub struct TestRoar {
    // Safety invariant: `ptr` was returned by `roar_create`, is non-null, is uniquely
    // owned by this instance, and has not yet been destroyed.
    ptr: *mut oar_t,
}

impl TestRoar {
    pub fn new(layout: oar_layout_t, samples: u32, sample_rate: u32) -> Self {
        let config = oar_config_t {
            target_layout: layout as i32,
            samples_per_channel: samples,
            sampling_rate: sample_rate,
        };
        // SAFETY:
        // - `&config` is a reference to a local stack variable, guaranteeing it is non-null,
        //   properly aligned for `oar_config_t`, and points to a fully initialized `oar_config_t`.
        // - `config` remains live, immutable, and unmoved across the synchronous call to
        //   `roar_create`, ensuring it is safe to read for the entire duration of the call.
        let ptr = unsafe { roar_create(&config) };
        assert!(!ptr.is_null());
        Self { ptr }
    }

    /// Returns the raw pointer to the underlying renderer for FFI calls.
    pub fn ptr(&self) -> *mut oar_t {
        self.ptr
    }
}

impl Drop for TestRoar {
    fn drop(&mut self) {
        // SAFETY:
        // - By the struct safety invariant and private field encapsulation, `self.ptr` was returned
        //   by `roar_create`, is valid, and has not been destroyed yet.
        // - `Drop::drop` takes `&mut self`, ensuring exclusive ownership and access; no concurrent
        //   access can occur during destruction.
        // - `roar_destroy` is called exactly once as `TestRoar` is dropped.
        unsafe { roar_destroy(self.ptr) };
    }
}

/// Creates a mutable planar buffer from a flat buffer slice.
pub fn create_planar_buffer_mut<'a, 'b>(
    flat_buffer: &'a mut [f32],
    channels: usize,
    samples: usize,
    channel_slices: &'b mut [&'a mut [f32]],
) -> PlanarBufferMut<'a, 'b> {
    let mut chunks = flat_buffer.chunks_exact_mut(samples);
    channel_slices[..channels]
        .fill_with(|| chunks.next().expect("Flat buffer has fewer chunks than channels"));
    PlanarBufferMut::new(channel_slices, channels, Samples::new(samples as u32).unwrap()).unwrap()
}

/// Creates a reference-based planar buffer from channel slices.
pub fn create_planar_buffer_ref<'a, 'b>(
    channel_slices: &'b [&'a [f32]],
    samples: usize,
) -> PlanarBufferRef<'a, 'b> {
    PlanarBufferRef::new(
        channel_slices,
        channel_slices.len(),
        Samples::new(samples as u32).unwrap(),
    )
    .unwrap()
}

/// Creates a nested vector of channels filled with a sine wave for Rust API tests.
pub fn create_sine_input_channels(
    channels: usize,
    samples: usize,
    amplitude: f32,
    freq: f32,
    sample_rate: f32,
) -> Vec<Vec<f32>> {
    let mut inputs = vec![vec![0.0f32; samples]; channels];
    for chan in &mut inputs {
        fill_with_sine(chan, amplitude, freq, sample_rate);
    }
    inputs
}

/// Creates a flat vector filled with sine waves (channel-by-channel) for FFI C API tests.
pub fn create_flat_sine_input(
    channels: usize,
    samples: usize,
    amplitude: f32,
    freq: f32,
    sample_rate: f32,
) -> Vec<f32> {
    let mut data = vec![0.0f32; channels * samples];
    for chan in 0..channels {
        let offset = chan * samples;
        fill_with_sine(&mut data[offset..offset + samples], amplitude, freq, sample_rate);
    }
    data
}

/// Generates a sine wave into a channel buffer.
pub fn fill_with_sine(channel: &mut [f32], amplitude: f32, freq: f32, sample_rate: f32) {
    let multiplier: f32 = std::f32::consts::TAU * freq / sample_rate;
    for (i, sample) in channel.iter_mut().enumerate() {
        *sample = amplitude * (i as f32 * multiplier).sin();
    }
}

/// Creates a default parameter set with zeroed flags and default rendering config.
pub fn default_parameter_set() -> parameter_set_t {
    parameter_set_t {
        flags: 0,
        downmix_info: downmix_info_t { mode: 0, weight_index: 0 },
        element_rendering_config: audio_element_rendering_config_t {
            headphones_rendering_mode: oar_headphones_rendering_mode_t::ck_world_locked_restricted,
            binaural_filter_profile: oar_binaural_filter_profile_t::ck_ambient,
        },
    }
}

/// Creates a channel-based audio element configuration.
pub fn create_channel_based_config(layout: oar_layout_t) -> oar_audio_element_config_t {
    oar_audio_element_config_t {
        r#type: audio_element_type_t::ck_channel_based as i32,
        config: oar_audio_element_config_union_t {
            cbc: oar_channel_based_config_t { layout: layout as i32 },
        },
        parameters: default_parameter_set(),
    }
}

/// Creates a channel-based audio element configuration with downmix info.
pub fn create_channel_based_config_with_downmix(
    layout: oar_layout_t,
    downmix_mode: i32,
) -> oar_audio_element_config_t {
    let mut params = default_parameter_set();
    params.flags = parameter_set_t::def_parameter_set_flag_iamf_downmix_info;
    params.downmix_info.mode = downmix_mode;
    oar_audio_element_config_t {
        r#type: audio_element_type_t::ck_channel_based as i32,
        config: oar_audio_element_config_union_t {
            cbc: oar_channel_based_config_t { layout: layout as i32 },
        },
        parameters: params,
    }
}

/// Creates a scene-based (Ambisonics) audio element configuration.
pub fn create_scene_based_config(order: oar_hoa_t) -> oar_audio_element_config_t {
    oar_audio_element_config_t {
        r#type: audio_element_type_t::ck_scene_based as i32,
        config: oar_audio_element_config_union_t {
            sbc: oar_scene_based_config_t { order: order as i32 },
        },
        parameters: default_parameter_set(),
    }
}

/// Creates an object-based audio element configuration.
pub fn create_object_based_config(num_objects: usize) -> oar_audio_element_config_t {
    oar_audio_element_config_t {
        r#type: audio_element_type_t::ck_object_based as i32,
        config: oar_audio_element_config_union_t {
            obc: oar_object_based_config_t { num_objects: num_objects as i32 },
        },
        parameters: default_parameter_set(),
    }
}

/// Creates a C FFI metadata struct (`oar_metadata_t`) for constant polar object positions.
///
/// # Parameters
/// * `positions` - Slice of `(azimuth, elevation, distance)` tuples in degrees and normalized
///   distance.
/// * `duration` - Transition duration in samples.
pub fn create_polar_position_metadata(
    positions: &[(f32, f32, f32)],
    duration: usize,
) -> oar_metadata_t {
    let mut polar_pos =
        [polar_t { azimuth: 0.0, elevation: 0.0, distance: 0.0 }; def_max_number_of_objects];
    for (i, &(azimuth, elevation, distance)) in positions.iter().enumerate() {
        polar_pos[i] = polar_t { azimuth, elevation, distance };
    }
    oar_metadata_t {
        r#type: oar_metadata_type_t::ck_metadata_object_positions as u32,
        value: oar_metadata_union_t {
            object_positions: oar_metadata_object_positions_t {
                param_type: oar_param_type_t::ck_param_constant,
                position_type: coordinate_type_t::ck_polar,
                num_objects: positions.len() as u32,
                positions: oar_metadata_object_positions_union_t { polar_positions: polar_pos },
            },
        },
        duration: duration as i32,
    }
}

/// Creates a Rust `ObjectPosition::Polar` from `(azimuth, elevation, distance)` tuples.
pub fn create_polar_positions(positions: &[(f32, f32, f32)]) -> ObjectPosition {
    ObjectPosition::Polar(
        positions
            .iter()
            .map(|&(az, el, dist)| PolarCoordinate::new_from_floats(az, el, dist).unwrap())
            .collect(),
    )
}

/// Computes the Root Mean Square (RMS) of an audio channel.
pub fn rms(channel: &[f32]) -> f32 {
    if channel.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = channel.iter().map(|&x| x * x).sum();
    (sum_sq / channel.len() as f32).sqrt()
}
