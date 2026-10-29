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

    use aom_roar::c_types::{
        audio_element_type_t, coordinate_type_t, oar_audio_element_config_t, oar_config_t,
        oar_headphones_rendering_mode_t, oar_layout_t, oar_metadata_iamf_downmix_mode_t,
        oar_metadata_object_positions_t, oar_metadata_object_positions_union_t, oar_metadata_t,
        oar_metadata_type_t, oar_metadata_union_t, oar_param_type_t, parameter_set_t, polar_t,
    };
    use aom_roar::ffi::*;
    use aom_roar::OarError;
    use aom_roar::{
        AudioElementConfig, ChannelBasedConfig, Config, DownmixInfo, DownmixMode, GroupId,
        InputLayout, ObjectPosition, OutputLayout, PolarCoordinate, RoarRenderer, SampleRate,
        Samples,
    };
    use googletest::prelude::*;

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

    /// Verifies that invalid IAMF downmix mode values are rejected with `-22` when the element's
    /// active rendering path uses the Downmix renderer (`5.1 -> stereo` with `downmix_info`), and
    /// that a subsequent valid mode update still returns `0`.
    #[gtest]
    fn ffi_invalid_downmix_modes_with_active_downmix_consumer_return_invalid_parameter() {
        let config = oar_config_t {
            target_layout: oar_layout_t::ck_oar_layout_stereo as i32,
            samples_per_channel: 256,
            sampling_rate: 48000,
        };
        let oar = unsafe { roar_create(&config) };
        assert!(!oar.is_null());
        let group_id = unsafe { roar_add_audio_group(oar) };
        assert!(group_id >= 0);
        let mut element_cfg: oar_audio_element_config_t = unsafe { std::mem::zeroed() };
        element_cfg.r#type = audio_element_type_t::ck_channel_based as i32;
        element_cfg.config.cbc.layout = oar_layout_t::ck_oar_layout_51 as i32;
        element_cfg.parameters.flags = parameter_set_t::def_parameter_set_flag_iamf_downmix_info;
        element_cfg.parameters.downmix_info.mode = 0;
        element_cfg.parameters.downmix_info.weight_index = -1;
        let add_ret = unsafe { roar_add_audio_element(oar, group_id as u32, 1, &element_cfg) };
        assert_eq!(add_ret, 0);
        let neg_meta = oar_metadata_t {
            r#type: oar_metadata_type_t::ck_metadata_iamf_downmix_mode as u32,
            value: oar_metadata_union_t {
                iamf_downmix_mode: oar_metadata_iamf_downmix_mode_t { mode: -1 },
            },
            duration: 256,
        };
        let mode3_meta = oar_metadata_t {
            r#type: oar_metadata_type_t::ck_metadata_iamf_downmix_mode as u32,
            value: oar_metadata_union_t {
                iamf_downmix_mode: oar_metadata_iamf_downmix_mode_t { mode: 3 },
            },
            duration: 256,
        };
        let mode7_meta = oar_metadata_t {
            r#type: oar_metadata_type_t::ck_metadata_iamf_downmix_mode as u32,
            value: oar_metadata_union_t {
                iamf_downmix_mode: oar_metadata_iamf_downmix_mode_t { mode: 7 },
            },
            duration: 256,
        };
        let mode99_meta = oar_metadata_t {
            r#type: oar_metadata_type_t::ck_metadata_iamf_downmix_mode as u32,
            value: oar_metadata_union_t {
                iamf_downmix_mode: oar_metadata_iamf_downmix_mode_t { mode: 99 },
            },
            duration: 256,
        };
        let valid_meta = oar_metadata_t {
            r#type: oar_metadata_type_t::ck_metadata_iamf_downmix_mode as u32,
            value: oar_metadata_union_t {
                iamf_downmix_mode: oar_metadata_iamf_downmix_mode_t { mode: 1 },
            },
            duration: 256,
        };

        let neg_ret = unsafe { roar_update_audio_element_metadata(oar, 1, &neg_meta) };
        let mode3_ret = unsafe { roar_update_audio_element_metadata(oar, 1, &mode3_meta) };
        let mode7_ret = unsafe { roar_update_audio_element_metadata(oar, 1, &mode7_meta) };
        let mode99_ret = unsafe { roar_update_audio_element_metadata(oar, 1, &mode99_meta) };
        let valid_ret = unsafe { roar_update_audio_element_metadata(oar, 1, &valid_meta) };

        expect_that!(neg_ret, eq(-22));
        expect_that!(mode3_ret, eq(-22));
        expect_that!(mode7_ret, eq(-22));
        expect_that!(mode99_ret, eq(-22));
        expect_that!(valid_ret, eq(0));

        unsafe { roar_destroy(oar) };
    }

    /// Verifies that invalid IAMF downmix mode values are also rejected with `-22` on binaural
    /// output when a `5.1` channel element is configured as `ck_world_locked_restricted` with
    /// `downmix_info` (which selects the Downmix renderer targeting stereo).
    #[gtest]
    fn ffi_invalid_downmix_mode_on_binaural_world_locked_restricted_51_returns_invalid_parameter() {
        let config = oar_config_t {
            target_layout: oar_layout_t::ck_oar_layout_binaural as i32,
            samples_per_channel: 256,
            sampling_rate: 48000,
        };
        let oar = unsafe { roar_create(&config) };
        assert!(!oar.is_null());
        let group_id = unsafe { roar_add_audio_group(oar) };
        assert!(group_id >= 0);
        let mut element_cfg: oar_audio_element_config_t = unsafe { std::mem::zeroed() };
        element_cfg.r#type = audio_element_type_t::ck_channel_based as i32;
        element_cfg.config.cbc.layout = oar_layout_t::ck_oar_layout_51 as i32;
        element_cfg.parameters.flags = parameter_set_t::def_parameter_set_flag_iamf_downmix_info
            | parameter_set_t::def_parameter_set_flag_iamf_element_rendering_config;
        element_cfg.parameters.downmix_info.mode = 0;
        element_cfg.parameters.downmix_info.weight_index = -1;
        element_cfg.parameters.element_rendering_config.headphones_rendering_mode =
            oar_headphones_rendering_mode_t::ck_world_locked_restricted;
        let add_ret = unsafe { roar_add_audio_element(oar, group_id as u32, 1, &element_cfg) };
        assert_eq!(add_ret, 0);
        let invalid_meta = oar_metadata_t {
            r#type: oar_metadata_type_t::ck_metadata_iamf_downmix_mode as u32,
            value: oar_metadata_union_t {
                iamf_downmix_mode: oar_metadata_iamf_downmix_mode_t { mode: 3 },
            },
            duration: 256,
        };

        let ret = unsafe { roar_update_audio_element_metadata(oar, 1, &invalid_meta) };

        expect_that!(ret, eq(-22));

        unsafe { roar_destroy(oar) };
    }

    /// Verifies that invalid IAMF downmix mode values are accepted and ignored (`0`) when the
    /// element's active rendering path has no downmix consumer (`5.1.2 -> stereo` EAR fallback).
    #[gtest]
    fn ffi_invalid_downmix_modes_without_downmix_consumer_return_ok() {
        let config = oar_config_t {
            target_layout: oar_layout_t::ck_oar_layout_stereo as i32,
            samples_per_channel: 256,
            sampling_rate: 48000,
        };
        let oar = unsafe { roar_create(&config) };
        assert!(!oar.is_null());
        let group_id = unsafe { roar_add_audio_group(oar) };
        assert!(group_id >= 0);
        let mut element_cfg: oar_audio_element_config_t = unsafe { std::mem::zeroed() };
        element_cfg.r#type = audio_element_type_t::ck_channel_based as i32;
        element_cfg.config.cbc.layout = oar_layout_t::ck_oar_layout_512 as i32;
        element_cfg.parameters.flags = parameter_set_t::def_parameter_set_flag_iamf_downmix_info;
        element_cfg.parameters.downmix_info.mode = 0;
        element_cfg.parameters.downmix_info.weight_index = -1;
        let add_ret = unsafe { roar_add_audio_element(oar, group_id as u32, 1, &element_cfg) };
        assert_eq!(add_ret, 0);
        let neg_meta = oar_metadata_t {
            r#type: oar_metadata_type_t::ck_metadata_iamf_downmix_mode as u32,
            value: oar_metadata_union_t {
                iamf_downmix_mode: oar_metadata_iamf_downmix_mode_t { mode: -1 },
            },
            duration: 256,
        };
        let mode3_meta = oar_metadata_t {
            r#type: oar_metadata_type_t::ck_metadata_iamf_downmix_mode as u32,
            value: oar_metadata_union_t {
                iamf_downmix_mode: oar_metadata_iamf_downmix_mode_t { mode: 3 },
            },
            duration: 256,
        };
        let mode7_meta = oar_metadata_t {
            r#type: oar_metadata_type_t::ck_metadata_iamf_downmix_mode as u32,
            value: oar_metadata_union_t {
                iamf_downmix_mode: oar_metadata_iamf_downmix_mode_t { mode: 7 },
            },
            duration: 256,
        };
        let mode99_meta = oar_metadata_t {
            r#type: oar_metadata_type_t::ck_metadata_iamf_downmix_mode as u32,
            value: oar_metadata_union_t {
                iamf_downmix_mode: oar_metadata_iamf_downmix_mode_t { mode: 99 },
            },
            duration: 256,
        };

        let neg_ret = unsafe { roar_update_audio_element_metadata(oar, 1, &neg_meta) };
        let mode3_ret = unsafe { roar_update_audio_element_metadata(oar, 1, &mode3_meta) };
        let mode7_ret = unsafe { roar_update_audio_element_metadata(oar, 1, &mode7_meta) };
        let mode99_ret = unsafe { roar_update_audio_element_metadata(oar, 1, &mode99_meta) };

        expect_that!(neg_ret, eq(0));
        expect_that!(mode3_ret, eq(0));
        expect_that!(mode7_ret, eq(0));
        expect_that!(mode99_ret, eq(0));

        unsafe { roar_destroy(oar) };
    }

    /// Verifies that an invalid IAMF downmix mode value on a matching input/output layout
    /// (`5.1.2 -> 5.1.2`, which falls back to EAR) is accepted and ignored (`0`).
    #[gtest]
    fn ffi_invalid_downmix_mode_on_matching_512_layout_returns_ok() {
        let config = oar_config_t {
            target_layout: oar_layout_t::ck_oar_layout_512 as i32,
            samples_per_channel: 256,
            sampling_rate: 48000,
        };
        let oar = unsafe { roar_create(&config) };
        assert!(!oar.is_null());
        let group_id = unsafe { roar_add_audio_group(oar) };
        assert!(group_id >= 0);
        let mut element_cfg: oar_audio_element_config_t = unsafe { std::mem::zeroed() };
        element_cfg.r#type = audio_element_type_t::ck_channel_based as i32;
        element_cfg.config.cbc.layout = oar_layout_t::ck_oar_layout_512 as i32;
        element_cfg.parameters.flags = parameter_set_t::def_parameter_set_flag_iamf_downmix_info;
        element_cfg.parameters.downmix_info.mode = 0;
        element_cfg.parameters.downmix_info.weight_index = -1;
        let add_ret = unsafe { roar_add_audio_element(oar, group_id as u32, 1, &element_cfg) };
        assert_eq!(add_ret, 0);
        let mode3_meta = oar_metadata_t {
            r#type: oar_metadata_type_t::ck_metadata_iamf_downmix_mode as u32,
            value: oar_metadata_union_t {
                iamf_downmix_mode: oar_metadata_iamf_downmix_mode_t { mode: 3 },
            },
            duration: 256,
        };

        let ret = unsafe { roar_update_audio_element_metadata(oar, 1, &mode3_meta) };

        expect_that!(ret, eq(0));

        unsafe { roar_destroy(oar) };
    }

    /// Verifies that an advisory downmix mode update targeting a non-existent element ID still
    /// returns `-22` (`InvalidParameter`) rather than being silently ignored.
    #[gtest]
    fn ffi_downmix_mode_update_for_missing_element_returns_invalid_parameter() {
        let config = oar_config_t {
            target_layout: oar_layout_t::ck_oar_layout_stereo as i32,
            samples_per_channel: 256,
            sampling_rate: 48000,
        };
        let oar = unsafe { roar_create(&config) };
        assert!(!oar.is_null());
        let group_id = unsafe { roar_add_audio_group(oar) };
        assert!(group_id >= 0);
        let mut element_cfg: oar_audio_element_config_t = unsafe { std::mem::zeroed() };
        element_cfg.r#type = audio_element_type_t::ck_channel_based as i32;
        element_cfg.config.cbc.layout = oar_layout_t::ck_oar_layout_512 as i32;
        element_cfg.parameters.flags = parameter_set_t::def_parameter_set_flag_iamf_downmix_info;
        element_cfg.parameters.downmix_info.mode = 0;
        element_cfg.parameters.downmix_info.weight_index = -1;
        let add_ret = unsafe { roar_add_audio_element(oar, group_id as u32, 1, &element_cfg) };
        assert_eq!(add_ret, 0);
        let meta = oar_metadata_t {
            r#type: oar_metadata_type_t::ck_metadata_iamf_downmix_mode as u32,
            value: oar_metadata_union_t {
                iamf_downmix_mode: oar_metadata_iamf_downmix_mode_t { mode: 1 },
            },
            duration: 256,
        };

        let ret = unsafe { roar_update_audio_element_metadata(oar, 999, &meta) };

        expect_that!(ret, eq(-22));

        unsafe { roar_destroy(oar) };
    }

    /// Verifies that reserved and unknown element metadata types return `-95` (`NotSupported`).
    #[gtest]
    fn ffi_reserved_and_unknown_element_metadata_types_return_not_supported() {
        let config = oar_config_t {
            target_layout: oar_layout_t::ck_oar_layout_stereo as i32,
            samples_per_channel: 256,
            sampling_rate: 48000,
        };
        let oar = unsafe { roar_create(&config) };
        assert!(!oar.is_null());
        let group_id = unsafe { roar_add_audio_group(oar) };
        assert!(group_id >= 0);
        let mut element_cfg: oar_audio_element_config_t = unsafe { std::mem::zeroed() };
        element_cfg.r#type = audio_element_type_t::ck_channel_based as i32;
        element_cfg.config.cbc.layout = oar_layout_t::ck_oar_layout_512 as i32;
        let add_ret = unsafe { roar_add_audio_element(oar, group_id as u32, 1, &element_cfg) };
        assert_eq!(add_ret, 0);
        let mut reserved_meta: oar_metadata_t = unsafe { std::mem::zeroed() };
        reserved_meta.r#type = oar_metadata_type_t::ck_metadata_count as u32;
        reserved_meta.duration = 256;
        let mut unknown_meta: oar_metadata_t = unsafe { std::mem::zeroed() };
        unknown_meta.r#type = 99;
        unknown_meta.duration = 256;

        let reserved_ret = unsafe { roar_update_audio_element_metadata(oar, 1, &reserved_meta) };
        let unknown_ret = unsafe { roar_update_audio_element_metadata(oar, 1, &unknown_meta) };

        expect_that!(reserved_ret, eq(-95));
        expect_that!(unknown_ret, eq(-95));

        unsafe { roar_destroy(oar) };
    }

    /// Verifies that pushing object position metadata onto a channel-based element returns `-95`
    /// (`NotSupported`).
    #[gtest]
    fn ffi_object_positions_update_on_channel_element_returns_not_supported() {
        let config = oar_config_t {
            target_layout: oar_layout_t::ck_oar_layout_stereo as i32,
            samples_per_channel: 256,
            sampling_rate: 48000,
        };
        let oar = unsafe { roar_create(&config) };
        assert!(!oar.is_null());
        let group_id = unsafe { roar_add_audio_group(oar) };
        assert!(group_id >= 0);
        let mut element_cfg: oar_audio_element_config_t = unsafe { std::mem::zeroed() };
        element_cfg.r#type = audio_element_type_t::ck_channel_based as i32;
        element_cfg.config.cbc.layout = oar_layout_t::ck_oar_layout_512 as i32;
        let add_ret = unsafe { roar_add_audio_element(oar, group_id as u32, 1, &element_cfg) };
        assert_eq!(add_ret, 0);
        let pos_meta = oar_metadata_t {
            r#type: oar_metadata_type_t::ck_metadata_object_positions as u32,
            value: oar_metadata_union_t {
                object_positions: oar_metadata_object_positions_t {
                    param_type: oar_param_type_t::ck_param_constant,
                    position_type: coordinate_type_t::ck_polar,
                    num_objects: 1,
                    positions: oar_metadata_object_positions_union_t {
                        polar_positions: [
                            polar_t { azimuth: 30.0, elevation: 0.0, distance: 1.0 },
                            polar_t { azimuth: 0.0, elevation: 0.0, distance: 0.0 },
                        ],
                    },
                },
            },
            duration: 256,
        };

        let ret = unsafe { roar_update_audio_element_metadata(oar, 1, &pos_meta) };

        expect_that!(ret, eq(-95));

        unsafe { roar_destroy(oar) };
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
            OutputLayout::Stereo,
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
            OutputLayout::Stereo,
            Samples::new(128).unwrap(),
            SampleRate::new(48000).unwrap(),
        )
        .unwrap();
        let mut renderer = RoarRenderer::create(&config).unwrap();
        let group_id_0 = renderer.add_audio_group().unwrap();

        let element_cfg = AudioElementConfig::ChannelBased(ChannelBasedConfig {
            layout: InputLayout::Stereo,
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
            OutputLayout::Stereo,
            Samples::new(128).unwrap(),
            SampleRate::new(48000).unwrap(),
        )
        .unwrap();
        let mut renderer = RoarRenderer::create(&config).unwrap();

        let element_cfg = AudioElementConfig::ChannelBased(ChannelBasedConfig {
            layout: InputLayout::Stereo,
            downmix_info: None,
            rendering_config: None,
        });
        // We haven't added any groups yet, so GroupId::Zero is inactive.
        let element_id = 10;
        let add_result = renderer.add_element(GroupId::Zero, element_id, &element_cfg);

        expect_that!(add_result, err(eq(OarError::InvalidParameter)));
    }

    /// Verifies that `RoarRenderer::update_element_downmix_mode` returns
    /// `Err(OarError::InvalidParameter)` when targeting a non-existent element ID.
    #[gtest]
    fn rust_downmix_mode_update_for_missing_element_returns_invalid_parameter() {
        let config = Config::new(
            OutputLayout::Stereo,
            Samples::new(128).unwrap(),
            SampleRate::new(48000).unwrap(),
        )
        .unwrap();
        let mut renderer = RoarRenderer::create(&config).unwrap();
        let group_id = renderer.add_audio_group().unwrap();
        let element_cfg = AudioElementConfig::ChannelBased(ChannelBasedConfig {
            layout: InputLayout::Layout512,
            downmix_info: Some(DownmixInfo::new(DownmixMode::Mode1NegOffset, None)),
            rendering_config: None,
        });
        renderer.add_element(group_id, 1, &element_cfg).unwrap();

        let result = renderer.update_element_downmix_mode(
            999,
            DownmixMode::Mode2NegOffset,
            Some(Samples::new(128).unwrap()),
        );

        expect_that!(result, err(eq(OarError::InvalidParameter)));
    }

    /// Verifies that `RoarRenderer::update_element_positions` returns
    /// `Err(OarError::NotSupported)` when targeting a channel-based element.
    #[gtest]
    fn rust_object_positions_update_on_channel_element_returns_not_supported() {
        let config = Config::new(
            OutputLayout::Stereo,
            Samples::new(128).unwrap(),
            SampleRate::new(48000).unwrap(),
        )
        .unwrap();
        let mut renderer = RoarRenderer::create(&config).unwrap();
        let group_id = renderer.add_audio_group().unwrap();
        let element_cfg = AudioElementConfig::ChannelBased(ChannelBasedConfig {
            layout: InputLayout::Layout512,
            downmix_info: None,
            rendering_config: None,
        });
        renderer.add_element(group_id, 1, &element_cfg).unwrap();
        let positions =
            ObjectPosition::Polar(vec![PolarCoordinate::new_from_floats(30.0, 0.0, 1.0).unwrap()]);

        let result = renderer.update_element_positions(1, &positions, Samples::new(128).unwrap());

        expect_that!(result, err(eq(OarError::NotSupported)));
    }
}
