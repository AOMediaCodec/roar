// Copyright (c) 2026, Alliance for Open Media. All rights reserved
//
// This source code is subject to the terms of the BSD 3-Clause Clear License
// and the Alliance for Open Media Patent License 1.0. If the BSD 3-Clause Clear
// License was not distributed with this source code in the LICENSE file, you
// can obtain it at www.aomedia.org/license/software-license/bsd-3-c-c. If the
// Alliance for Open Media Patent License 1.0 was not distributed with this
// source code in the PATENTS file, you can obtain it at
// www.aomedia.org/license/patent.

//! Integration tests for the peak clipping limiter.

#[cfg(test)]
mod test {

    use googletest::prelude::*;
    use roar::{
        c_types::{oar_audio_block_t, oar_layout_t},
        ffi::{
            roar_add_audio_element, roar_add_audio_group, roar_enable_limiter, roar_render,
            roar_update_audio_element_data,
        },
        AudioElementConfig, ChannelBasedConfig, Config, Layout, RoarRenderer, SampleRate, Samples,
    };
    use test_helpers::{
        create_channel_based_config, create_planar_buffer_mut, create_planar_buffer_ref, TestRoar,
    };

    const LIMITER_VALUE: f32 = 1.0;
    const VALUE_GREATER_THAN_LIMIT: f32 = 2.0;
    const VALUE_UNDER_LIMITER_VALUE: f32 = 0.75;

    // ===== Helpers =====
    fn get_max_abs(buffer: &[f32]) -> f32 {
        buffer.iter().map(|x| x.abs()).fold(0.0, f32::max)
    }

    // ===== C FFI API Tests =====

    /// Verifies that the peak limiter, when enabled via the C FFI API, keeps the output
    /// amplitude <= limiter value even when fed with high-amplitude inputs, and that it passes the
    /// unattenuated signal when disabled.
    #[gtest]
    fn ffi_limiter_enabled_prevents_output_clipping() {
        let frame_size: usize = 128;
        let sample_rate: u32 = 48000;
        let input_channels: usize = 2;
        let output_channels: usize = 2;
        let element_id: u32 = 10;

        // Create test ROAR instance.
        let oar = TestRoar::new(oar_layout_t::ck_oar_layout_stereo, frame_size as u32, sample_rate);

        // SAFETY: oar.ptr is valid.
        let group_id = unsafe { roar_add_audio_group(oar.ptr) };
        assert!(group_id >= 0);

        let element_cfg = create_channel_based_config(oar_layout_t::ck_oar_layout_stereo);
        // SAFETY: config and oar.ptr are valid.
        let ret =
            unsafe { roar_add_audio_element(oar.ptr, group_id as u32, element_id, &element_cfg) };
        assert_eq!(ret, 0);

        // Very loud input.
        let mut input_buffer = vec![VALUE_GREATER_THAN_LIMIT; input_channels * frame_size];
        let mut input_data = oar_audio_block_t {
            data: input_buffer.as_mut_ptr(),
            channels: input_channels as u32,
            samples_per_channel: frame_size as u32,
        };

        // 1. Render with limiter DISABLED
        // SAFETY: oar.ptr is valid.
        let ret = unsafe { roar_enable_limiter(oar.ptr, 0) };
        assert_eq!(ret, 0);
        let mut output_no_limiter = vec![0.0f32; output_channels * frame_size];
        let mut out_block_1 = oar_audio_block_t {
            data: output_no_limiter.as_mut_ptr(),
            channels: output_channels as u32,
            samples_per_channel: frame_size as u32,
        };

        // SAFETY: input data and oar.ptr are valid.
        let update_ret =
            unsafe { roar_update_audio_element_data(oar.ptr, element_id, &mut input_data) };
        // SAFETY: render test.
        let render_ret = unsafe { roar_render(oar.ptr, &mut out_block_1) };
        assert_eq!(update_ret, 0);
        assert_eq!(render_ret, 0);

        let max_no_limiter = get_max_abs(&output_no_limiter);
        expect_gt!(max_no_limiter, LIMITER_VALUE);

        // 2. Render with limiter ENABLED
        // SAFETY: oar.ptr is valid.
        let ret = unsafe { roar_enable_limiter(oar.ptr, 1) };
        assert_eq!(ret, 0);

        let mut output_with_limiter = vec![0.0f32; output_channels * frame_size];
        let mut out_block_2 = oar_audio_block_t {
            data: output_with_limiter.as_mut_ptr(),
            channels: output_channels as u32,
            samples_per_channel: frame_size as u32,
        };

        // Render a few blocks to let the limiter stabilize
        for _ in 0..10 {
            // SAFETY: input data and oar.ptr are valid.
            let update_ret =
                unsafe { roar_update_audio_element_data(oar.ptr, element_id, &mut input_data) };
            // SAFETY: render test.
            let render_ret = unsafe { roar_render(oar.ptr, &mut out_block_2) };
            assert_eq!(update_ret, 0);
            assert_eq!(render_ret, 0);
        }

        let max_with_limiter = get_max_abs(&output_with_limiter);
        expect_le!(max_with_limiter, LIMITER_VALUE);
        expect_gt!(max_with_limiter, VALUE_UNDER_LIMITER_VALUE);
    }

    // ===== Rust API Tests =====

    /// Verifies that the peak limiter, when enabled via the Rust API, keeps the output
    /// amplitude <= limited value even when fed with high-amplitude inputs, and that it passes the
    /// unattenuated signal when disabled.
    #[gtest]
    fn rust_limiter_enabled_prevents_output_clipping() {
        let frame_size: usize = 128;
        let sample_rate: u32 = 48000;
        let input_channels: usize = 2;
        let output_channels: usize = 2;
        let element_id: u32 = 10;

        let config = Config::new(
            Layout::Stereo,
            Samples::new(frame_size as u32).unwrap(),
            SampleRate::new(sample_rate).unwrap(),
        )
        .unwrap();
        let mut renderer = RoarRenderer::create(&config).unwrap();
        let group_id = renderer.add_audio_group().unwrap();

        let element_cfg = AudioElementConfig::ChannelBased(ChannelBasedConfig {
            layout: Layout::Stereo,
            downmix_info: None,
            rendering_config: None,
        });
        renderer.add_element(group_id, element_id, &element_cfg).unwrap();

        // Very loud input
        let inputs = vec![vec![VALUE_GREATER_THAN_LIMIT; frame_size]; input_channels];
        let input_refs: Vec<&[f32]> = inputs.iter().map(|v| v.as_slice()).collect();
        let input_buffer_ref = create_planar_buffer_ref(&input_refs[..], frame_size);

        // 1. Render with limiter DISABLED
        renderer.enable_limiter(false).unwrap();

        let mut output_no_limiter = vec![0.0; output_channels * frame_size];
        let mut output_slices_1: Vec<&mut [f32]> =
            (0..output_channels).map(|_| &mut [] as &mut [f32]).collect();
        let mut output_buffer_1 = create_planar_buffer_mut(
            &mut output_no_limiter,
            output_channels,
            frame_size,
            &mut output_slices_1,
        );

        renderer.render(&[(element_id, input_buffer_ref)], &mut output_buffer_1).unwrap();

        let max_no_limiter = get_max_abs(&output_no_limiter);
        expect_gt!(max_no_limiter, LIMITER_VALUE);

        // 2. Render with limiter ENABLED
        renderer.enable_limiter(true).unwrap();

        let mut output_with_limiter = vec![0.0; output_channels * frame_size];
        let mut output_slices_2: Vec<&mut [f32]> =
            (0..output_channels).map(|_| &mut [] as &mut [f32]).collect();
        let mut output_buffer_2 = create_planar_buffer_mut(
            &mut output_with_limiter,
            output_channels,
            frame_size,
            &mut output_slices_2,
        );

        // Render a few blocks to let the limiter stabilize
        for _ in 0..10 {
            let input_buffer_ref_loop = create_planar_buffer_ref(&input_refs[..], frame_size);
            renderer.render(&[(element_id, input_buffer_ref_loop)], &mut output_buffer_2).unwrap();
        }

        let max_with_limiter = get_max_abs(&output_with_limiter);
        expect_le!(max_with_limiter, LIMITER_VALUE);
        expect_gt!(max_with_limiter, VALUE_UNDER_LIMITER_VALUE);
    }
}
