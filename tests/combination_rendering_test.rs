// Copyright (c) 2026, Alliance for Open Media. All rights reserved
//
// This source code is subject to the terms of the BSD 3-Clause Clear License
// and the Alliance for Open Media Patent License 1.0. If the BSD 3-Clause Clear
// License was not distributed with this source code in the LICENSE file, you
// can obtain it at www.aomedia.org/license/software-license/bsd-3-c-c. If the
// Alliance for Open Media Patent License 1.0 was not distributed with this
// source code in the PATENTS file, you can obtain it at
// www.aomedia.org/license/patent.

//! Integration tests for rendering a combination of channel-based, scene-based, and
//! object-based elements simultaneously.

#[cfg(test)]
mod test {

    use googletest::prelude::*;
    use roar::c_types::{
        coordinate_type_t, def_max_number_of_objects, oar_audio_block_t, oar_hoa_t, oar_layout_t,
        oar_metadata_object_positions_t, oar_metadata_object_positions_union_t, oar_metadata_t,
        oar_metadata_type_t, oar_metadata_union_t, oar_param_type_t, polar_t,
    };
    use roar::ffi::*;
    use roar::{
        AudioElementConfig, ChannelBasedConfig, Config, HighOrderAmbisonics, Layout,
        ObjectBasedConfig, ObjectPosition, PolarCoordinate, RoarRenderer, SampleRate, Samples,
        SceneBasedConfig,
    };
    use test_helpers::{
        create_channel_based_config, create_object_based_config, create_planar_buffer_mut,
        create_planar_buffer_ref, create_scene_based_config, rms, TestRoar,
    };

    /// Expect at least this value for non-silent output.
    const NON_SILENT_RMS_THRESHOLD: f32 = 0.01;

    // ===== C FFI API Tests =====

    /// Verifies that rendering a combination of a ChannelBased, a SceneBased, and an ObjectBased
    /// element simultaneously via the C FFI API succeeds and produces output.
    #[gtest]
    fn ffi_rendering_combination_of_all_element_types_succeeds() {
        let frame_size: usize = 128;
        let sample_rate: u32 = 48000;
        let output_channels: usize = 2;
        let chan_based_element_id: u32 = 10;
        let scene_based_element_id: u32 = 20;
        let obj_based_element_id: u32 = 30;

        // Create test ROAR instance.
        let oar = TestRoar::new(oar_layout_t::ck_oar_layout_stereo, frame_size as u32, sample_rate);

        // SAFETY: oar is valid.
        let group_id = unsafe { roar_add_audio_group(oar.ptr) };
        assert_eq!(group_id, 0);

        // 1. Channel-Based Element (Stereo) -> ID 10
        let chan_based_cfg = create_channel_based_config(oar_layout_t::ck_oar_layout_stereo);
        // SAFETY: config and oar are valid.
        let ret = unsafe {
            roar_add_audio_element(oar.ptr, group_id as u32, chan_based_element_id, &chan_based_cfg)
        };
        assert_eq!(ret, 0);

        // 2. Scene-Based Element (1OA) -> ID 20
        let scene_based_cfg = create_scene_based_config(oar_hoa_t::ck_oar_1oa);
        // SAFETY: config and oar are valid.
        let ret = unsafe {
            roar_add_audio_element(
                oar.ptr,
                group_id as u32,
                scene_based_element_id,
                &scene_based_cfg,
            )
        };
        assert_eq!(ret, 0);

        // 3. Object-Based Element (1 object) -> ID 30
        let obj_based_cfg = create_object_based_config(1);
        // SAFETY: config and oar are valid.
        let ret = unsafe {
            roar_add_audio_element(oar.ptr, group_id as u32, obj_based_element_id, &obj_based_cfg)
        };
        assert_eq!(ret, 0);

        // Set position for Object (ID 30)
        let mut polar_pos =
            [polar_t { azimuth: 0.0, elevation: 0.0, distance: 0.0 }; def_max_number_of_objects];
        polar_pos[0] = polar_t { azimuth: 0.0, elevation: 0.0, distance: 1.0 };
        let pos_meta = oar_metadata_t {
            r#type: oar_metadata_type_t::ck_metadata_object_positions as u32,
            value: oar_metadata_union_t {
                object_positions: oar_metadata_object_positions_t {
                    param_type: oar_param_type_t::ck_param_constant,
                    position_type: coordinate_type_t::ck_polar,
                    num_objects: 1,
                    positions: oar_metadata_object_positions_union_t { polar_positions: polar_pos },
                },
            },
            duration: frame_size as i32,
        };
        // SAFETY: metadata and oar are valid.
        let ret =
            unsafe { roar_update_audio_element_metadata(oar.ptr, obj_based_element_id, &pos_meta) };
        assert_eq!(ret, 0);

        let mut chan_based_input = vec![0.2f32; 2 * frame_size];
        let mut scene_based_input = vec![0.0f32; 4 * frame_size];
        scene_based_input[..frame_size].fill(0.1); // Fill first channel
        let mut obj_based_input = vec![0.3f32; frame_size];

        let mut chan_based_block = oar_audio_block_t {
            data: chan_based_input.as_mut_ptr(),
            channels: 2,
            samples_per_channel: frame_size as u32,
        };
        let mut scene_based_block = oar_audio_block_t {
            data: scene_based_input.as_mut_ptr(),
            channels: 4,
            samples_per_channel: frame_size as u32,
        };
        let mut obj_based_block = oar_audio_block_t {
            data: obj_based_input.as_mut_ptr(),
            channels: 1,
            samples_per_channel: frame_size as u32,
        };

        let mut output = vec![0.0f32; output_channels * frame_size];
        let mut out_block = oar_audio_block_t {
            data: output.as_mut_ptr(),
            channels: output_channels as u32,
            samples_per_channel: frame_size as u32,
        };

        // SAFETY: data and oar are valid.
        let ret = unsafe {
            roar_update_audio_element_data(oar.ptr, chan_based_element_id, &mut chan_based_block)
        };
        assert_eq!(ret, 0);
        // SAFETY: data and oar are valid.
        let ret = unsafe {
            roar_update_audio_element_data(oar.ptr, scene_based_element_id, &mut scene_based_block)
        };
        assert_eq!(ret, 0);
        // SAFETY: data and oar are valid.
        let ret = unsafe {
            roar_update_audio_element_data(oar.ptr, obj_based_element_id, &mut obj_based_block)
        };
        assert_eq!(ret, 0);

        // SAFETY: output block and oar are valid.
        let ret = unsafe { roar_render(oar.ptr, &mut out_block) };
        assert_eq!(ret, 0);

        // Verify output is non-silent in planar layout.
        expect_that!(rms(&output[..frame_size]), gt(NON_SILENT_RMS_THRESHOLD));
        expect_that!(rms(&output[frame_size..]), gt(NON_SILENT_RMS_THRESHOLD));
    }

    // ===== Rust API Tests =====

    /// Verifies that rendering a combination of a ChannelBased, a SceneBased, and an ObjectBased
    /// element simultaneously using the Rust API succeeds and produces a non-silent stereo output.
    #[gtest]
    fn rust_rendering_combination_of_all_element_types_succeeds() {
        let frame_size: usize = 128;
        let sample_rate: u32 = 48000;
        let output_channels: usize = 2;

        let config = Config::new(
            Layout::Stereo,
            Samples::new(frame_size as u32).unwrap(),
            SampleRate::new(sample_rate).unwrap(),
        )
        .unwrap();
        let mut renderer = RoarRenderer::create(&config).unwrap();
        let group_id = renderer.add_audio_group().unwrap();

        // 1. Channel-Based Element (Stereo)
        let chan_based_element_id: u32 = 10;
        let chan_based_cfg = AudioElementConfig::ChannelBased(ChannelBasedConfig {
            layout: Layout::Stereo,
            downmix_info: None,
            rendering_config: None,
        });
        renderer.add_element(group_id, chan_based_element_id, &chan_based_cfg).unwrap();

        // 2. Scene-Based Element (1OA - 4 channels)
        let scene_based_element_id: u32 = 20;
        let scene_based_cfg = AudioElementConfig::SceneBased(SceneBasedConfig {
            order: HighOrderAmbisonics::Order1,
            rendering_config: None,
        });
        renderer.add_element(group_id, scene_based_element_id, &scene_based_cfg).unwrap();

        // 3. Object-Based Element (1 object)
        let obj_based_element_id: u32 = 30;
        let obj_based_cfg = AudioElementConfig::ObjectBased(ObjectBasedConfig {
            num_objects: 1,
            rendering_config: None,
        });
        renderer.add_element(group_id, obj_based_element_id, &obj_based_cfg).unwrap();

        // Set position for Object
        let pos =
            ObjectPosition::Polar(vec![PolarCoordinate::new_from_floats(0.0, 0.0, 1.0).unwrap()]);
        renderer
            .update_element_positions(obj_based_element_id, &pos, Samples(frame_size as u32))
            .unwrap();

        let chan_based_inputs = vec![vec![0.2; frame_size]; 2];
        let mut scene_based_inputs = vec![vec![0.0; frame_size]; 4];
        scene_based_inputs[0].fill(0.1);
        let obj_based_inputs = vec![vec![0.3; frame_size]; 1];

        let chan_based_refs: Vec<&[f32]> = chan_based_inputs.iter().map(|v| v.as_slice()).collect();
        let scene_based_refs: Vec<&[f32]> =
            scene_based_inputs.iter().map(|v| v.as_slice()).collect();
        let obj_based_refs: Vec<&[f32]> = obj_based_inputs.iter().map(|v| v.as_slice()).collect();

        let chan_based_in_view = create_planar_buffer_ref(&chan_based_refs[..], frame_size);
        let scene_based_in_view = create_planar_buffer_ref(&scene_based_refs[..], frame_size);
        let obj_based_in_view = create_planar_buffer_ref(&obj_based_refs[..], frame_size);

        let mut output = vec![0.0; output_channels * frame_size];
        let mut output_slices: Vec<&mut [f32]> =
            (0..output_channels).map(|_| &mut [] as &mut [f32]).collect();
        let mut output_buffer =
            create_planar_buffer_mut(&mut output, output_channels, frame_size, &mut output_slices);

        let result = renderer.render(
            &[
                (chan_based_element_id, chan_based_in_view),
                (scene_based_element_id, scene_based_in_view),
                (obj_based_element_id, obj_based_in_view),
            ],
            &mut output_buffer,
        );

        expect_ok!(result);
        // Verify output is non-silent (has energy from all elements)
        expect_that!(rms(output_buffer.channel_mut(0)), gt(NON_SILENT_RMS_THRESHOLD));
        expect_that!(rms(output_buffer.channel_mut(1)), gt(NON_SILENT_RMS_THRESHOLD));
    }
}
