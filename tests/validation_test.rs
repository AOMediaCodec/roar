// Copyright (c) 2026, Alliance for Open Media. All rights reserved
//
// This source code is subject to the terms of the BSD 3-Clause Clear License
// and the Alliance for Open Media Patent License 1.0. If the BSD 3-Clause Clear
// License was not distributed with this source code in the LICENSE file, you
// can obtain it at www.aomedia.org/license/software-license/bsd-3-c-c. If the
// Alliance for Open Media Patent License 1.0 was not distributed with this
// source code in the PATENTS file, you can obtain it at
// www.aomedia.org/license/patent.

//! Test validation of input values for both C and Rust APIs.

#[cfg(test)]
mod test {

    use googletest::prelude::*;
    use roar::c_types::{
        audio_element_type_t, oar_audio_element_config_t, oar_config_t, oar_layout_t,
    };
    use roar::ffi::*;
    use roar::OarError;
    use roar::{
        AudioElementConfig, ChannelBasedConfig, Config, GroupId, Layout, RoarRenderer, SampleRate,
        Samples,
    };

    // ===== FFI C API Validation Tests =====

    /// Verifies that the FFI layer enforces the limit of 2 audio groups.
    #[gtest]
    fn ffi_adding_third_group_fails_with_invalid_parameter() {
        let config = oar_config_t {
            target_layout: oar_layout_t::ck_oar_layout_stereo as i32,
            samples_per_channel: 256,
            sampling_rate: 48000,
        };
        let oar = unsafe { roar_create(&config) };
        assert!(!oar.is_null());

        // SAFETY: oar is valid. Adding 2 groups should succeed.
        let group_id_0 = unsafe { roar_add_audio_group(oar) };
        let group_id_1 = unsafe { roar_add_audio_group(oar) };
        // Adding 3rd group should fail with InvalidParameter (-22).
        let group_id_2 = unsafe { roar_add_audio_group(oar) };

        assert_eq!(group_id_0, 0);
        assert_eq!(group_id_1, 1);
        assert_eq!(group_id_2, OarError::InvalidParameter.into());

        unsafe { roar_destroy(oar) };
    }

    /// Verifies that the FFI layer prevents adding elements with duplicate IDs.
    #[gtest]
    fn ffi_adding_duplicate_element_id_fails_with_busy() {
        let config = oar_config_t {
            target_layout: oar_layout_t::ck_oar_layout_stereo as i32,
            samples_per_channel: 256,
            sampling_rate: 48000,
        };
        let oar = unsafe { roar_create(&config) };
        assert!(!oar.is_null());

        let group_id_0 = unsafe { roar_add_audio_group(oar) };
        assert!(group_id_0 >= 0);

        let mut element_cfg: oar_audio_element_config_t = unsafe { std::mem::zeroed() };
        element_cfg.r#type = audio_element_type_t::ck_channel_based as i32;
        element_cfg.config.cbc.layout = oar_layout_t::ck_oar_layout_stereo as i32;

        // Add element 10
        let add_first_result =
            unsafe { roar_add_audio_element(oar, group_id_0 as u32, 10, &element_cfg) };
        // Attempt to add element 10 again (duplicate ID) -> should fail with Busy (-16)
        let add_second_result =
            unsafe { roar_add_audio_element(oar, group_id_0 as u32, 10, &element_cfg) };

        assert_eq!(add_first_result, 0);
        assert_eq!(add_second_result, OarError::Busy.into());

        unsafe { roar_destroy(oar) };
    }

    /// Verifies that the FFI layer errors when adding an element to a non-existent group.
    #[gtest]
    fn ffi_adding_element_to_non_existent_group_fails_with_invalid_parameter() {
        let config = oar_config_t {
            target_layout: oar_layout_t::ck_oar_layout_stereo as i32,
            samples_per_channel: 256,
            sampling_rate: 48000,
        };
        let oar = unsafe { roar_create(&config) };
        assert!(!oar.is_null());

        let mut element_cfg: oar_audio_element_config_t = unsafe { std::mem::zeroed() };
        element_cfg.r#type = audio_element_type_t::ck_channel_based as i32;
        element_cfg.config.cbc.layout = oar_layout_t::ck_oar_layout_stereo as i32;

        // Add to group 99 (non-existent) -> should fail
        let add_result = unsafe { roar_add_audio_element(oar, 99, 10, &element_cfg) };

        assert_eq!(add_result, OarError::InvalidParameter.into());

        unsafe { roar_destroy(oar) };
    }

    /// Verifies that creating a renderer with zero sample rate or zero samples per channel
    /// via the C FFI API fails by returning a null pointer.
    #[gtest]
    fn ffi_creating_renderer_with_zero_sample_rate_or_samples_fails() {
        // Zero sample rate
        let config_zero_rate = oar_config_t {
            target_layout: oar_layout_t::ck_oar_layout_stereo as i32,
            samples_per_channel: 256,
            sampling_rate: 0,
        };
        // SAFETY: testing error paths.
        let oar_1 = unsafe { roar_create(&config_zero_rate) };
        expect_true!(oar_1.is_null());

        // Zero samples per channel
        let config_zero_samples = oar_config_t {
            target_layout: oar_layout_t::ck_oar_layout_stereo as i32,
            samples_per_channel: 0,
            sampling_rate: 48000,
        };
        // SAFETY: testing error paths.
        let oar_2 = unsafe { roar_create(&config_zero_samples) };
        expect_true!(oar_2.is_null());
    }

    // ===== Rust API Validation Tests =====

    /// Verifies that sample rates and sample counts must be greater than zero.
    #[gtest]
    fn rust_creating_zero_sample_rate_or_samples_fails() {
        expect_ok!(SampleRate::new(48000));
        expect_that!(SampleRate::new(0), err(eq(OarError::InvalidParameter)));
        expect_that!(Samples::new(0), err(eq(OarError::InvalidParameter)));
    }

    /// Verifies that the Rust API restricts audio groups to a maximum of 2.
    #[gtest]
    fn rust_adding_third_group_fails_with_invalid_parameter() {
        let config = Config::new(
            Layout::Stereo,
            Samples::new(128).unwrap(),
            SampleRate::new(48000).unwrap(),
        )
        .unwrap();
        let mut renderer = RoarRenderer::create(&config).unwrap();

        let group_id_0 = renderer.add_audio_group();
        let group_id_1 = renderer.add_audio_group();
        // 3rd group should fail
        let group_id_2 = renderer.add_audio_group();

        expect_ok!(group_id_0);
        expect_ok!(group_id_1);
        expect_that!(group_id_2, err(eq(OarError::InvalidParameter)));
    }

    /// Verifies that the Rust API prevents registering duplicate audio element IDs.
    #[gtest]
    fn rust_adding_duplicate_element_id_fails_with_busy() {
        let config = Config::new(
            Layout::Stereo,
            Samples::new(128).unwrap(),
            SampleRate::new(48000).unwrap(),
        )
        .unwrap();
        let mut renderer = RoarRenderer::create(&config).unwrap();
        let group_id_0 = renderer.add_audio_group().unwrap();

        let element_cfg = AudioElementConfig::ChannelBased(ChannelBasedConfig {
            layout: Layout::Stereo,
            downmix_info: None,
            rendering_config: None,
        });
        let element_id = 10;
        let add_first_result = renderer.add_element(group_id_0, element_id, &element_cfg);
        let add_second_result = renderer.add_element(group_id_0, element_id, &element_cfg);

        expect_ok!(add_first_result);
        expect_that!(add_second_result, err(eq(OarError::Busy)));
    }

    /// Verifies that the Rust API rejects adding elements to a group before the group has been
    /// activated.
    #[gtest]
    fn rust_adding_element_to_inactive_group_fails_with_invalid_parameter() {
        let config = Config::new(
            Layout::Stereo,
            Samples::new(128).unwrap(),
            SampleRate::new(48000).unwrap(),
        )
        .unwrap();
        let mut renderer = RoarRenderer::create(&config).unwrap();

        let element_cfg = AudioElementConfig::ChannelBased(ChannelBasedConfig {
            layout: Layout::Stereo,
            downmix_info: None,
            rendering_config: None,
        });
        // We haven't added any groups yet, so GroupId::Zero is inactive.
        let element_id = 10;
        let add_result = renderer.add_element(GroupId::Zero, element_id, &element_cfg);

        expect_that!(add_result, err(eq(OarError::InvalidParameter)));
    }
}
