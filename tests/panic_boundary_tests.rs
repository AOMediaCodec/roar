// Copyright (c) 2026, Alliance for Open Media. All rights reserved
//
// This source code is subject to the terms of the BSD 3-Clause Clear License
// and the Alliance for Open Media Patent License 1.0. If the BSD 3-Clause Clear
// License was not distributed with this source code in the LICENSE file, you
// can obtain it at www.aomedia.org/license/software-license/bsd-3-c-c. If the
// Alliance for Open Media Patent License 1.0 was not distributed with this
// source code in the PATENTS file, you can obtain it at
// www.aomedia.org/license/patent.

//! Panic boundary and FFI safety tests for raw pointers.
//!
//! Verifies that all public FFI entry points handle null pointers gracefully,
//! returning `InvalidParameter` or returning early instead of panicking.

#[cfg(test)]
mod test {

    use googletest::prelude::*;

    use roar::c_types::{oar_audio_block_t, oar_layout_t, oar_metadata_t, oar_metadata_type_t};
    use roar::ffi::*;
    use roar::OarError;
    use test_helpers::{create_channel_based_config, TestRoar};

    #[gtest]
    fn test_roar_create_handles_null_config_gracefully() {
        // SAFETY: passing null config to create should return null.
        let oar = unsafe { roar_create(std::ptr::null()) };
        assert!(oar.is_null());
    }

    #[gtest]
    fn test_roar_destroy_handles_null_gracefully() {
        // SAFETY: passing null to destroy should return immediately without panicking.
        unsafe { roar_destroy(std::ptr::null_mut()) };
    }

    #[gtest]
    fn test_roar_add_audio_group_handles_null_gracefully() {
        // SAFETY: passing null to group addition should return invalid status.
        let ret = unsafe { roar_add_audio_group(std::ptr::null_mut()) };
        assert_eq!(ret, OarError::InvalidParameter.into());
    }

    #[gtest]
    fn test_roar_add_audio_element_handles_nulls_gracefully() {
        let oar = TestRoar::new(oar_layout_t::ck_oar_layout_stereo, 128, 48000);
        // SAFETY: oar.ptr is valid.
        let group_id = unsafe { roar_add_audio_group(oar.ptr) };
        assert!(group_id >= 0);
        let valid_config = create_channel_based_config(oar_layout_t::ck_oar_layout_stereo);

        // SAFETY: passing null renderer should return invalid status.
        let ret = unsafe {
            roar_add_audio_element(std::ptr::null_mut(), group_id as u32, 10, &valid_config)
        };
        assert_eq!(ret, OarError::InvalidParameter.into());

        // SAFETY: passing null config should return invalid status.
        let ret = unsafe { roar_add_audio_element(oar.ptr, group_id as u32, 10, std::ptr::null()) };
        assert_eq!(ret, OarError::InvalidParameter.into());
    }

    #[gtest]
    fn test_roar_remove_audio_element_handles_null_gracefully() {
        // SAFETY: passing null renderer should return invalid status.
        let ret = unsafe { roar_remove_audio_element(std::ptr::null_mut(), 10) };
        assert_eq!(ret, OarError::InvalidParameter.into());
    }

    #[gtest]
    fn test_roar_update_audio_element_metadata_handles_nulls_gracefully() {
        let oar = TestRoar::new(oar_layout_t::ck_oar_layout_stereo, 128, 48000);
        // SAFETY: oar.ptr is valid.
        let group_id = unsafe { roar_add_audio_group(oar.ptr) };
        assert!(group_id >= 0);
        let valid_config = create_channel_based_config(oar_layout_t::ck_oar_layout_stereo);
        let element_id = 10u32;
        // SAFETY: oar.ptr, group_id, and valid_config are valid.
        let ret =
            unsafe { roar_add_audio_element(oar.ptr, group_id as u32, element_id, &valid_config) };
        assert_eq!(ret, 0);

        let valid_metadata = oar_metadata_t {
            r#type: oar_metadata_type_t::ck_metadata_object_positions as u32,
            value: unsafe { std::mem::zeroed() },
            duration: 128,
        };

        // SAFETY: passing null renderer should return invalid status.
        let ret = unsafe {
            roar_update_audio_element_metadata(std::ptr::null_mut(), element_id, &valid_metadata)
        };
        assert_eq!(ret, OarError::InvalidParameter.into());

        // SAFETY: passing null metadata should return invalid status.
        let ret =
            unsafe { roar_update_audio_element_metadata(oar.ptr, element_id, std::ptr::null()) };
        assert_eq!(ret, OarError::InvalidParameter.into());
    }

    #[gtest]
    fn test_roar_update_audio_element_data_handles_nulls_gracefully() {
        let oar = TestRoar::new(oar_layout_t::ck_oar_layout_stereo, 128, 48000);
        // SAFETY: oar.ptr is valid.
        let group_id = unsafe { roar_add_audio_group(oar.ptr) };
        assert!(group_id >= 0);
        let valid_config = create_channel_based_config(oar_layout_t::ck_oar_layout_stereo);
        let element_id = 10u32;
        // SAFETY: oar.ptr, group_id, and valid_config are valid.
        let ret =
            unsafe { roar_add_audio_element(oar.ptr, group_id as u32, element_id, &valid_config) };
        assert_eq!(ret, 0);

        let mut input_buffer = vec![0.0f32; 256];
        let mut valid_block = oar_audio_block_t {
            data: input_buffer.as_mut_ptr(),
            channels: 2,
            samples_per_channel: 128,
        };

        // SAFETY: passing null renderer should return invalid status.
        let ret = unsafe {
            roar_update_audio_element_data(std::ptr::null_mut(), element_id, &mut valid_block)
        };
        assert_eq!(ret, OarError::InvalidParameter.into());

        // SAFETY: passing null data block should return invalid status.
        let ret =
            unsafe { roar_update_audio_element_data(oar.ptr, element_id, std::ptr::null_mut()) };
        assert_eq!(ret, OarError::InvalidParameter.into());
    }

    #[gtest]
    fn test_roar_set_metadata_unit_to_process_handles_null_gracefully() {
        // SAFETY: passing null renderer should return invalid status.
        let ret = unsafe {
            roar_set_metadata_unit_to_process(
                std::ptr::null_mut(),
                oar_metadata_type_t::ck_metadata_object_positions,
                128,
            )
        };
        assert_eq!(ret, OarError::InvalidParameter.into());
    }

    #[gtest]
    fn test_roar_update_metadata_handles_nulls_gracefully() {
        let oar = TestRoar::new(oar_layout_t::ck_oar_layout_stereo, 128, 48000);
        // SAFETY: oar.ptr is valid.
        let group_id = unsafe { roar_add_audio_group(oar.ptr) };
        assert!(group_id >= 0);

        let valid_metadata = oar_metadata_t {
            r#type: oar_metadata_type_t::ck_metadata_object_positions as u32,
            value: unsafe { std::mem::zeroed() },
            duration: 128,
        };

        // SAFETY: passing null renderer should return invalid status.
        let ret =
            unsafe { roar_update_metadata(std::ptr::null_mut(), group_id as u32, &valid_metadata) };
        assert_eq!(ret, OarError::InvalidParameter.into());

        // SAFETY: passing null metadata should return invalid status.
        let ret = unsafe { roar_update_metadata(oar.ptr, group_id as u32, std::ptr::null()) };
        assert_eq!(ret, OarError::InvalidParameter.into());
    }

    #[gtest]
    fn test_roar_render_handles_nulls_gracefully() {
        let oar = TestRoar::new(oar_layout_t::ck_oar_layout_stereo, 128, 48000);
        let mut input_buffer = vec![0.0f32; 256];
        let mut valid_block = oar_audio_block_t {
            data: input_buffer.as_mut_ptr(),
            channels: 2,
            samples_per_channel: 128,
        };

        // SAFETY: passing null renderer should return invalid status.
        let ret = unsafe { roar_render(std::ptr::null_mut(), &mut valid_block) };
        assert_eq!(ret, OarError::InvalidParameter.into());

        // SAFETY: passing null output block should return invalid status.
        let ret = unsafe { roar_render(oar.ptr, std::ptr::null_mut()) };
        assert_eq!(ret, OarError::InvalidParameter.into());
    }

    #[gtest]
    fn test_roar_enable_loudness_processor_handles_null_gracefully() {
        // SAFETY: passing null renderer should return invalid status.
        let ret = unsafe { roar_enable_loudness_processor(std::ptr::null_mut(), 1) };
        assert_eq!(ret, OarError::InvalidParameter.into());
    }

    #[gtest]
    fn test_roar_set_loudness_handles_null_gracefully() {
        // SAFETY: passing null renderer should return invalid status.
        let ret = unsafe { roar_set_loudness(std::ptr::null_mut(), 0, 0.0, -20.0) };
        assert_eq!(ret, OarError::InvalidParameter.into());
    }

    #[gtest]
    fn test_roar_enable_limiter_handles_null_gracefully() {
        // SAFETY: passing null renderer should return invalid status.
        let ret = unsafe { roar_enable_limiter(std::ptr::null_mut(), 1) };
        assert_eq!(ret, OarError::InvalidParameter.into());
    }

    #[gtest]
    fn test_roar_enable_head_tracking_handles_null_gracefully() {
        // SAFETY: passing null renderer should return invalid status.
        let ret = unsafe { roar_enable_head_tracking(std::ptr::null_mut(), 1) };
        assert_eq!(ret, OarError::InvalidParameter.into());
    }

    #[gtest]
    fn test_roar_get_samples_per_channel_handles_null_gracefully() {
        // SAFETY: passing null renderer should return 0.
        let ret = unsafe { roar_get_samples_per_channel(std::ptr::null()) };
        assert_eq!(ret, 0);
    }

    #[gtest]
    fn test_roar_get_sampling_rate_handles_null_gracefully() {
        // SAFETY: passing null renderer should return 0.
        let ret = unsafe { roar_get_sampling_rate(std::ptr::null()) };
        assert_eq!(ret, 0);
    }

    #[gtest]
    fn test_roar_get_number_of_audio_element_channels_handles_null_gracefully() {
        // SAFETY: passing null renderer should return 0.
        let ret = unsafe { roar_get_number_of_audio_element_channels(std::ptr::null(), 10) };
        assert_eq!(ret, 0);
    }

    #[gtest]
    fn test_roar_get_number_of_output_channels_handles_null_gracefully() {
        // SAFETY: passing null renderer should return 0.
        let ret = unsafe { roar_get_number_of_output_channels(std::ptr::null()) };
        assert_eq!(ret, 0);
    }

    #[gtest]
    fn test_roar_get_number_of_audio_elements_handles_null_gracefully() {
        // SAFETY: passing null renderer should return 0.
        let ret = unsafe { roar_get_number_of_audio_elements(std::ptr::null()) };
        assert_eq!(ret, 0);
    }
}
