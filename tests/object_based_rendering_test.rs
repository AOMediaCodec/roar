// Copyright (c) 2026, Alliance for Open Media. All rights reserved
//
// This source code is subject to the terms of the BSD 3-Clause Clear License
// and the Alliance for Open Media Patent License 1.0. If the BSD 3-Clause Clear
// License was not distributed with this source code in the LICENSE file, you
// can obtain it at www.aomedia.org/license/software-license/bsd-3-c-c. If the
// Alliance for Open Media Patent License 1.0 was not distributed with this
// source code in the PATENTS file, you can obtain it at
// www.aomedia.org/license/patent.

//! Integration tests for object-based audio rendering.

#[cfg(test)]
mod test {

    use googletest::prelude::*;
    use roar::c_types::{
        coordinate_type_t, def_max_number_of_objects, oar_audio_block_t, oar_layout_t,
        oar_metadata_object_positions_t, oar_metadata_object_positions_union_t, oar_metadata_t,
        oar_metadata_type_t, oar_metadata_union_t, oar_param_type_t, polar_t,
    };
    use roar::ffi::*;
    use roar::{
        AudioElementConfig, Config, Layout, ObjectBasedConfig, ObjectPosition, PolarCoordinate,
        RoarRenderer, SampleRate, Samples,
    };
    use test_helpers::{
        create_flat_sine_input, create_object_based_config, create_planar_buffer_mut,
        create_planar_buffer_ref, create_sine_input_channels, rms, TestRoar,
    };

    /// Expect at least this value for non-silent output.
    const NON_SILENT_RMS_THRESHOLD: f32 = 0.1;

    // ===== FFI C API Tests =====

    /// Verifies object-based rendering via the C FFI API with 2 objects positioned at left and
    /// right 45°, and asserts that output channels contain non-zero energy.
    #[gtest]
    fn ffi_rendering_multi_object_input_to_stereo_output_pans_objects() {
        let frame_size: usize = 128;
        let sample_rate: u32 = 48000;
        let num_objects: usize = 2;
        let output_channels: usize = 2;
        let element_id: u32 = 10;

        let amplitude: f32 = 1.0;
        let freq: f32 = 440.0;

        // Create test ROAR instance.
        let oar = TestRoar::new(oar_layout_t::ck_oar_layout_stereo, frame_size as u32, sample_rate);
        // Create audio group and audio element.
        let element_cfg = create_object_based_config(num_objects);
        // SAFETY: `oar.ptr` is a valid pointer initialized by `TestRoar::new`.
        let group_id = unsafe { roar_add_audio_group(oar.ptr) };
        assert!(group_id >= 0);
        // SAFETY: `oar.ptr` is valid, and `&element_cfg` is a valid temporary reference.
        let ret =
            unsafe { roar_add_audio_element(oar.ptr, group_id as u32, element_id, &element_cfg) };
        assert_eq!(ret, 0);
        // Set the position of the two objects.
        let azimuth_left = 45.0;
        let azimuth_right = -45.0;
        let elevation = 0.0;
        let distance = 1.0;
        let mut polar_pos =
            [polar_t { azimuth: 0.0, elevation: 0.0, distance: 0.0 }; def_max_number_of_objects];
        polar_pos[0] = polar_t { azimuth: azimuth_left, elevation, distance };
        polar_pos[1] = polar_t { azimuth: azimuth_right, elevation, distance };
        let pos_meta = oar_metadata_t {
            r#type: oar_metadata_type_t::ck_metadata_object_positions as u32,
            value: oar_metadata_union_t {
                object_positions: oar_metadata_object_positions_t {
                    param_type: oar_param_type_t::ck_param_constant,
                    position_type: coordinate_type_t::ck_polar,
                    num_objects: num_objects as u32,
                    positions: oar_metadata_object_positions_union_t { polar_positions: polar_pos },
                },
            },
            duration: frame_size as i32,
        };
        // Create input buffer, fill with sine wave, create input audio block struct.
        let mut input_buffer =
            create_flat_sine_input(num_objects, frame_size, amplitude, freq, sample_rate as f32);
        let mut input_data = oar_audio_block_t {
            data: input_buffer.as_mut_ptr(),
            channels: num_objects as u32,
            samples_per_channel: frame_size as u32,
        };
        // Create output audio block.
        let mut output_buffer = vec![0.0f32; output_channels * frame_size];
        let mut output_data = oar_audio_block_t {
            data: output_buffer.as_mut_ptr(),
            channels: output_channels as u32,
            samples_per_channel: frame_size as u32,
        };

        // SAFETY: `oar.ptr` is valid, and `pos_meta` reference is valid for this call.
        let update_meta_ret =
            unsafe { roar_update_audio_element_metadata(oar.ptr, element_id, &pos_meta) };
        // SAFETY: `oar.ptr` is valid; `input_data` buffer pointer is valid for this call.
        let update_data_ret =
            unsafe { roar_update_audio_element_data(oar.ptr, element_id, &mut input_data) };
        // SAFETY: `oar.ptr` is valid; `output_data` buffer pointer is valid for this call.
        let render_ret = unsafe { roar_render(oar.ptr, &mut output_data) };

        assert_eq!(update_meta_ret, 0);
        assert_eq!(update_data_ret, 0);
        assert_eq!(render_ret, 0);
        // Verify output is non-silent
        expect_that!(rms(&output_buffer[..frame_size]), gt(NON_SILENT_RMS_THRESHOLD));
        expect_that!(rms(&output_buffer[frame_size..]), gt(NON_SILENT_RMS_THRESHOLD));
    }

    /// Verifies object-based rendering via the C FFI API by rendering a single-object input.
    #[gtest]
    fn ffi_rendering_single_object_input_to_stereo_output_succeeds() {
        let frame_size: usize = 128;
        let sample_rate: u32 = 48000;
        let num_objects: usize = 1;
        let output_channels: usize = 2;
        let element_id: u32 = 10;

        let amplitude: f32 = 1.0;
        let freq: f32 = 440.0;

        // Create test ROAR instance.
        let oar = TestRoar::new(oar_layout_t::ck_oar_layout_stereo, frame_size as u32, sample_rate);
        // Create audio group and audio element.
        let element_cfg = create_object_based_config(num_objects);
        // SAFETY: `oar.ptr` is a valid pointer initialized by `TestRoar::new`.
        let group_id = unsafe { roar_add_audio_group(oar.ptr) };
        assert!(group_id >= 0);
        // SAFETY: `oar.ptr` is valid, and `&element_cfg` is a valid temporary reference.
        let ret =
            unsafe { roar_add_audio_element(oar.ptr, group_id as u32, element_id, &element_cfg) };
        assert_eq!(ret, 0);
        // Set the position of the object.
        let azimuth = 15.0;
        let elevation = 0.0;
        let distance = 1.0;
        let mut polar_pos =
            [polar_t { azimuth: 0.0, elevation: 0.0, distance: 0.0 }; def_max_number_of_objects];
        polar_pos[0] = polar_t { azimuth, elevation, distance };
        let pos_meta = oar_metadata_t {
            r#type: oar_metadata_type_t::ck_metadata_object_positions as u32,
            value: oar_metadata_union_t {
                object_positions: oar_metadata_object_positions_t {
                    param_type: oar_param_type_t::ck_param_constant,
                    position_type: coordinate_type_t::ck_polar,
                    num_objects: num_objects as u32,
                    positions: oar_metadata_object_positions_union_t { polar_positions: polar_pos },
                },
            },
            duration: frame_size as i32,
        };
        // Create input buffer, fill with sine wave, create input audio block struct.
        let mut input_buffer =
            create_flat_sine_input(num_objects, frame_size, amplitude, freq, sample_rate as f32);
        let mut input_data = oar_audio_block_t {
            data: input_buffer.as_mut_ptr(),
            channels: num_objects as u32,
            samples_per_channel: frame_size as u32,
        };
        // Create output audio block.
        let mut output_buffer = vec![0.0f32; output_channels * frame_size];
        let mut output_data = oar_audio_block_t {
            data: output_buffer.as_mut_ptr(),
            channels: output_channels as u32,
            samples_per_channel: frame_size as u32,
        };

        // SAFETY: `oar.ptr` is valid, and `pos_meta` reference is valid for this call.
        let update_meta_ret =
            unsafe { roar_update_audio_element_metadata(oar.ptr, element_id, &pos_meta) };
        // SAFETY: `oar.ptr` is valid; `input_data` buffer pointer is valid for this call.
        let update_data_ret =
            unsafe { roar_update_audio_element_data(oar.ptr, element_id, &mut input_data) };
        // SAFETY: `oar.ptr` is valid; `output_data` buffer pointer is valid for this call.
        let render_ret = unsafe { roar_render(oar.ptr, &mut output_data) };

        assert_eq!(update_meta_ret, 0);
        assert_eq!(update_data_ret, 0);
        assert_eq!(render_ret, 0);
        // Verify output is louder on the left side and not silent.
        expect_that!(rms(&output_buffer[..frame_size]), gt(rms(&output_buffer[frame_size..])));
        expect_that!(rms(&output_buffer[frame_size..]), gt(NON_SILENT_RMS_THRESHOLD));
    }

    // ===== Rust API Tests =====

    /// Verifies object-based rendering via the Rust API with 2 objects positioned at 45° left and
    /// right.
    #[gtest]
    fn rust_rendering_multi_object_input_to_stereo_output_pans_objects() {
        let frame_size: usize = 128;
        let sample_rate: u32 = 48000;
        let num_objects: usize = 2;
        let output_channels: usize = 2;
        let element_id: u32 = 10;

        let amplitude: f32 = 1.0;
        let freq: f32 = 440.0;

        // Create test ROAR instance and renderer.
        let config = Config::new(
            Layout::Stereo,
            Samples::new(frame_size as u32).unwrap(),
            SampleRate::new(sample_rate).unwrap(),
        )
        .unwrap();
        let mut renderer = RoarRenderer::create(&config).unwrap();
        // Create audio group and audio element.
        let group_id = renderer.add_audio_group().unwrap();
        let element_cfg = AudioElementConfig::ObjectBased(ObjectBasedConfig {
            num_objects: num_objects as u32,
            rendering_config: None,
        });
        renderer.add_element(group_id, element_id, &element_cfg).unwrap();
        // Set the position of the two objects.
        let azimuth_left = 45.0;
        let azimuth_right = -45.0;
        let elevation = 0.0;
        let distance = 1.0;
        let pos = ObjectPosition::Polar(vec![
            PolarCoordinate::new_from_floats(azimuth_left, elevation, distance).unwrap(),
            PolarCoordinate::new_from_floats(azimuth_right, elevation, distance).unwrap(),
        ]);
        // Create input buffer, fill with sine wave.
        let input_data = create_sine_input_channels(
            num_objects,
            frame_size,
            amplitude,
            freq,
            sample_rate as f32,
        );
        let input_refs: Vec<&[f32]> = input_data.iter().map(|v| v.as_slice()).collect();
        let input_buffer_ref = create_planar_buffer_ref(&input_refs[..], frame_size);
        // Create output buffer.
        let mut output = vec![0.0; output_channels * frame_size];
        let mut output_slices: Vec<&mut [f32]> =
            (0..output_channels).map(|_| &mut [] as &mut [f32]).collect();
        let mut output_buffer =
            create_planar_buffer_mut(&mut output, output_channels, frame_size, &mut output_slices);

        // Update positions and render.
        let position_update_result =
            renderer.update_element_positions(element_id, &pos, Samples(frame_size as u32));
        let render_result = renderer.render(&[(element_id, input_buffer_ref)], &mut output_buffer);

        expect_ok!(position_update_result);
        expect_ok!(render_result);
        // Verify output is non-silent.
        expect_that!(rms(output_buffer.channel_mut(0)), gt(NON_SILENT_RMS_THRESHOLD));
        expect_that!(rms(output_buffer.channel_mut(1)), gt(NON_SILENT_RMS_THRESHOLD));
    }

    /// Verifies object-based rendering via the Rust API by rendering a single-object input
    /// and checking that it succeeds.
    #[gtest]
    fn rust_rendering_single_object_input_to_stereo_output_succeeds() {
        let frame_size: usize = 128;
        let sample_rate: u32 = 48000;
        let num_objects: usize = 1;
        let output_channels: usize = 2;
        let element_id: u32 = 10;

        let amplitude: f32 = 1.0;
        let freq: f32 = 440.0;

        // Create test ROAR instance and renderer.
        let config = Config::new(
            Layout::Stereo,
            Samples::new(frame_size as u32).unwrap(),
            SampleRate::new(sample_rate).unwrap(),
        )
        .unwrap();
        let mut renderer = RoarRenderer::create(&config).unwrap();
        // Create audio group and audio element.
        let group_id = renderer.add_audio_group().unwrap();
        let element_cfg = AudioElementConfig::ObjectBased(ObjectBasedConfig {
            num_objects: num_objects as u32,
            rendering_config: None,
        });
        renderer.add_element(group_id, element_id, &element_cfg).unwrap();
        // Set the position of the object, 15° left.
        let azimuth = 15.0;
        let elevation = 0.0;
        let distance = 1.0;
        let pos = ObjectPosition::Polar(vec![PolarCoordinate::new_from_floats(
            azimuth, elevation, distance,
        )
        .unwrap()]);
        // Create input buffer, fill with sine wave.
        let input_data = create_sine_input_channels(
            num_objects,
            frame_size,
            amplitude,
            freq,
            sample_rate as f32,
        );
        let input_refs: Vec<&[f32]> = input_data.iter().map(|v| v.as_slice()).collect();
        let input_buffer_ref = create_planar_buffer_ref(&input_refs[..], frame_size);
        // Create output buffer.
        let mut output = vec![0.0; output_channels * frame_size];
        let mut output_slices: Vec<&mut [f32]> =
            (0..output_channels).map(|_| &mut [] as &mut [f32]).collect();
        let mut output_buffer =
            create_planar_buffer_mut(&mut output, output_channels, frame_size, &mut output_slices);

        // Update positions and render.
        let position_update_result =
            renderer.update_element_positions(element_id, &pos, Samples(frame_size as u32));
        let render_result = renderer.render(&[(element_id, input_buffer_ref)], &mut output_buffer);

        expect_ok!(position_update_result);
        expect_ok!(render_result);
        // Verify output is louder on the left side and not silent.
        expect_that!(rms(output_buffer.channel_mut(0)), gt(rms(output_buffer.channel_mut(1))));
        expect_that!(rms(output_buffer.channel_mut(0)), gt(NON_SILENT_RMS_THRESHOLD));
        expect_that!(rms(output_buffer.channel_mut(1)), gt(NON_SILENT_RMS_THRESHOLD));
    }
}
