// Copyright (c) 2026, Alliance for Open Media. All rights reserved
//
// This source code is subject to the terms of the BSD 3-Clause Clear License
// and the Alliance for Open Media Patent License 1.0. If the BSD 3-Clause Clear
// License was not distributed with this source code in the LICENSE file, you
// can obtain it at www.aomedia.org/license/software-license/bsd-3-c-c. If the
// Alliance for Open Media Patent License 1.0 was not distributed with this
// source code in the PATENTS file, you can obtain it at
// www.aomedia.org/license/patent.

//! C-compatible type layout structures and unions for FFI compatibility.
//!
//! This module contains the exact `#[repr(C)]` layouts of OAR structures,
//! enums, and unions to ensure strict binary compatibility with the legacy C APIs.
//! All raw pointer conversions and dynamic typestate bridging are handled here or
//! delegated to other safe Layer 2 modules.

#![allow(non_camel_case_types, non_upper_case_globals, non_snake_case)]

use std::os::raw::c_int;

/// Maximum number of objects supported in object position metadata.
pub const def_max_number_of_objects: usize = 2;

/// Opaque wrapper structure for `Oar` C-consumers.
///
/// Under the hood, this corresponds to the raw handle managed by the OAR tracker.
#[repr(C)]
pub struct oar_t {
    /// Opaque private fields to prevent instantiation or inspection from C/FFI side.
    pub _private: [u8; 0],
}

/// Audio element layouts and target layouts.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum oar_layout_t {
    /// No layout specified.
    ck_oar_layout_none = 0,
    /// Mono channel layout.
    ck_oar_layout_mono = 1,
    /// Stereo channel layout.
    ck_oar_layout_stereo = 2,
    /// 5.1 channel layout.
    ck_oar_layout_51 = 3,
    /// 5.1.2 channel layout.
    ck_oar_layout_512 = 4,
    /// 5.1.4 channel layout.
    ck_oar_layout_514 = 5,
    /// 7.1 channel layout.
    ck_oar_layout_71 = 6,
    /// 7.1.2 channel layout.
    ck_oar_layout_712 = 7,
    /// 7.1.4 channel layout.
    ck_oar_layout_714 = 8,
    /// 3.1.2 channel layout.
    ck_oar_layout_312 = 9,
    /// 9.1.6 channel layout.
    ck_oar_layout_916 = 10,
    /// 10.2.9.3 (A293) channel layout.
    ck_oar_layout_a293 = 11,
    /// 7.1.5.4 channel layout.
    ck_oar_layout_7154 = 12,

    /// Sound system E (4.5.1) layout.
    ck_oar_layout_sound_system_e_451 = 0x81,
    /// Sound system F (3.7.0) layout.
    ck_oar_layout_sound_system_f_370 = 0x82,
    /// Sound system G (4.9.0) layout.
    ck_oar_layout_sound_system_g_490 = 0x83,

    /// Binaural output layout.
    ck_oar_layout_binaural = 0xFF,

    /// Beginning of the IAMF expanded/subset layouts range.
    ck_oar_layout_subset_start = 0x100,
    /// Low-frequency effects (LFE) subset of 7.1.4.
    ck_oar_layout_lfe = 0x101,
    /// Surround subset (Ls/Rs) of 5.1.4.
    ck_oar_layout_stereo_s = 0x102,
    /// Side surround subset (Lss/Rss) of 7.1.4.
    ck_oar_layout_stereo_ss = 0x103,
    /// Rear surround subset (Lrs/Rrs) of 7.1.4.
    ck_oar_layout_stereo_rs = 0x104,
    /// Top front subset (Ltf/Rtf) of 7.1.4.
    ck_oar_layout_stereo_tf = 0x105,
    /// Top back subset (Ltb/Rtb) of 7.1.4.
    ck_oar_layout_stereo_tb = 0x106,
    /// Top 4 channels (Ltf/Rtf/Ltb/Rtb) of 7.1.4.
    ck_oar_layout_top_4ch = 0x107,
    /// Front 3 channels (L/C/R) of 7.1.4.
    ck_oar_layout_3ch = 0x108,
    /// Front subset (FL/FR) of 9.1.6.
    ck_oar_layout_stereo_f = 0x109,
    /// Side subset (SiL/SiR) of 9.1.6.
    ck_oar_layout_stereo_si = 0x10a,
    /// Top side subset (TpSiL/TpSiR) of 9.1.6.
    ck_oar_layout_stereo_tpsi = 0x10b,
    /// Top 6 channels (TpFL/TpFR/TpBL/TpBR/TpSiL/TpSiR) of 9.1.6.
    ck_oar_layout_top_6ch = 0x10c,
    /// Low-frequency effects subset (LFE1/LFE2) of 10.2.9.3.
    ck_oar_layout_lfe_pair = 0x10d,
    /// Bottom 3 channels (BtFC/BtFL/BtFR) of 10.2.9.3.
    ck_oar_layout_bottom_3ch = 0x10e,
    /// Bottom 4 channels (BtFL/BtFR/BtBL/BtBR) of 7.1.5.4.
    ck_oar_layout_bottom_4ch = 0x10f,
    /// Top subset (TpC) of 7.1.5.4.
    ck_oar_layout_top_1ch = 0x110,
    /// Top 5 channels (Ltf/Rtf/TpC/Ltb/Rtb) of 7.1.5.4.
    ck_oar_layout_top_5ch = 0x111,
}

impl oar_layout_t {
    /// Sound system A (0.2.0) layout (Stereo alias).
    pub const ck_oar_layout_sound_system_a_020: Self = Self::ck_oar_layout_stereo;
    /// Sound system B (0.5.0) layout (5.1 alias).
    pub const ck_oar_layout_sound_system_b_050: Self = Self::ck_oar_layout_51;
    /// Sound system C (2.5.0) layout (5.1.2 alias).
    pub const ck_oar_layout_sound_system_c_250: Self = Self::ck_oar_layout_512;
    /// Sound system D (4.5.0) layout (5.1.4 alias).
    pub const ck_oar_layout_sound_system_d_450: Self = Self::ck_oar_layout_514;
    /// Sound system I (0.7.0) layout (7.1 alias).
    pub const ck_oar_layout_sound_system_i_070: Self = Self::ck_oar_layout_71;
    /// Sound system J (4.7.0) layout (7.1.4 alias).
    pub const ck_oar_layout_sound_system_j_470: Self = Self::ck_oar_layout_714;
    /// Sound system H (9.a.3) layout (10.2.9.3 / A293 alias).
    pub const ck_oar_layout_sound_system_h_9a3: Self = Self::ck_oar_layout_a293;
}

/// Higher-Order Ambisonics (HOA) orders.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum oar_hoa_t {
    /// No HOA order specified.
    ck_oar_hoa_none = -1,
    /// Zeroth-Order Ambisonics (1 channel).
    ck_oar_zoa = 0,
    /// First-Order Ambisonics (4 channels).
    ck_oar_1oa = 1,
    /// Second-Order Ambisonics (9 channels).
    ck_oar_2oa = 2,
    /// Third-Order Ambisonics (16 channels).
    ck_oar_3oa = 3,
    /// Fourth-Order Ambisonics (25 channels).
    ck_oar_4oa = 4,
}

/// Types of audio elements.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum audio_element_type_t {
    /// No audio element type specified.
    ck_oar_element_type_none = -1,
    /// Channel-based audio element (static speaker layout).
    ck_channel_based = 0,
    /// Scene-based audio element (Ambisonics).
    ck_scene_based = 1,
    /// Object-based audio element (spatialized 3D point source).
    ck_object_based = 2,
}

/// Core configuration settings for initializing an OAR renderer instance.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct oar_config_t {
    /// Target playback layout (e.g., Stereo, 5.1, Binaural).
    pub target_layout: i32,
    /// Number of samples to process per channel per block.
    pub samples_per_channel: u32,
    /// Audio sampling rate in Hz (e.g., 48000).
    pub sampling_rate: u32,
}

/// Configuration details for a channel-based audio element.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct oar_channel_based_config_t {
    /// The input channel layout configuration.
    pub layout: i32,
}

/// Configuration details for a scene-based (Ambisonics) audio element.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct oar_scene_based_config_t {
    /// The Ambisonic order of the scene input.
    pub order: i32,
}

/// Configuration details for an object-based audio element.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct oar_object_based_config_t {
    /// Number of audio objects inside this element (must be between 1 and 2).
    pub num_objects: c_int,
}

/// Union of specific configurations for different audio element types.
#[repr(C)]
#[derive(Clone, Copy)]
pub union oar_audio_element_config_union_t {
    /// Configuration for channel-based rendering.
    pub cbc: oar_channel_based_config_t,
    /// Configuration for scene-based (Ambisonic) rendering.
    pub sbc: oar_scene_based_config_t,
    /// Configuration for object-based rendering.
    pub obc: oar_object_based_config_t,
}

/// Unified structure describing an audio element's configuration.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct oar_audio_element_config_t {
    /// The specific type of the audio element.
    pub r#type: i32,
    /// Type-specific inner configuration parameters.
    pub config: oar_audio_element_config_union_t,
    /// Additional parameters (such as demixing info and headphone rendering configurations).
    pub parameters: parameter_set_t,
}

/// Headphone rendering modes for channel-based audio element playback over headphones.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum oar_headphones_rendering_mode_t {
    /// World-locked restricted rendering.
    ck_world_locked_restricted = 0,
    /// Fully world-locked rendering (compensates for head tracking).
    ck_world_locked = 1,
    /// Head-locked rendering (fixed to listener head coordinates).
    ck_head_locked = 2,
    /// Reserved mode.
    ck_reserved = 3,
}

/// Binaural filter profile settings for binaural rendering.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum oar_binaural_filter_profile_t {
    /// Ambient binaural filter profile.
    ck_ambient = 0,
    /// Direct binaural filter profile.
    ck_direct = 1,
    /// Reverberant binaural filter profile.
    ck_reverberant = 2,
}

impl oar_binaural_filter_profile_t {
    /// Default binaural filter profile.
    pub const ck_binaural_filter_profile_default: Self = Self::ck_ambient;
}

/// Coordinate systems used for positioning audio objects.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum coordinate_type_t {
    /// Spherical polar coordinate system (azimuth, elevation, distance).
    ck_polar = 0,
    /// Cartesian coordinate system (x, y, z).
    ck_cartesian = 1,
}

/// 3D spherical polar coordinates.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct polar_t {
    /// Horizontal angle in degrees, range: -180.0 to 180.0.
    pub azimuth: f32,
    /// Vertical angle in degrees, range: -90.0 to 90.0.
    pub elevation: f32,
    /// Distance from listener, range: 0.0 to 1.0 (normalized).
    pub distance: f32,
}

/// 3D Cartesian coordinates.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct cartesian_t {
    /// X coordinate (left/right), range: -1.0 to 1.0.
    pub x: f32,
    /// Y coordinate (front/back), range: -1.0 to 1.0.
    pub y: f32,
    /// Z coordinate (up/down), range: -1.0 to 1.0.
    pub z: f32,
}

/// Quaternion representation of 3D rotations (typically head orientations).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct quaternion_t {
    /// Scalar component.
    pub w: f32,
    /// X imaginary component.
    pub x: f32,
    /// Y imaginary component.
    pub y: f32,
    /// Z imaginary component.
    pub z: f32,
}

/// Block of planar audio data passed across the FFI boundary.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct oar_audio_block_t {
    /// Raw mutable pointer to an array of channel pointers (planar audio buffer layout).
    pub data: *mut f32,
    /// Number of audio channels in the block.
    pub channels: u32,
    /// Number of samples per channel.
    pub samples_per_channel: u32,
}

/// IAMF downmix and demixing parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct downmix_info_t {
    /// Downmix mode index.
    pub mode: c_int,
    /// Demixing weight parameter index.
    pub weight_index: c_int,
}

/// Headphone and binaural rendering instructions for channel-based elements.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct audio_element_rendering_config_t {
    /// Headphone rendering spatialization mode.
    pub headphones_rendering_mode: oar_headphones_rendering_mode_t,
    /// Selected binaural filter profile.
    pub binaural_filter_profile: oar_binaural_filter_profile_t,
}

/// Group parameter set flag flags and config values.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct parameter_set_t {
    /// Flags indicating which configuration components are active.
    pub flags: u32,
    /// Demixing parameter block.
    pub downmix_info: downmix_info_t,
    /// Audio element rendering configuration block.
    pub element_rendering_config: audio_element_rendering_config_t,
}

impl parameter_set_t {
    /// Flag indicating IAMF downmix info is active.
    pub const def_parameter_set_flag_iamf_downmix_info: u32 = 0x01;
    /// Flag indicating IAMF element rendering configuration is active.
    pub const def_parameter_set_flag_iamf_element_rendering_config: u32 = 0x02;
}

/// OAR metadata category definitions.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum oar_metadata_type_t {
    /// Unspecified metadata type.
    ck_metadata_none = 0,
    /// Gain/Volume adjustment metadata.
    ck_metadata_gain = 1,
    /// IAMF downmix configuration mode metadata.
    ck_metadata_iamf_downmix_mode = 2,
    /// 3D coordinates for spatial audio objects.
    ck_metadata_object_positions = 3,
    /// Rotational tracking offsets for head orientation.
    ck_metadata_head_rotation = 4,
    /// Count of supported metadata categories.
    ck_metadata_count = 5,
}

/// Dynamic parameter temporal characteristics.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum oar_param_type_t {
    /// Constant parameter value across the block.
    ck_param_constant = 0,
    /// Multiple distinct values (e.g., per-sample array).
    ck_param_multiple = 1,
    /// Parameter values generated via dynamic animation keyframes.
    ck_param_animated = 2,
}

/// Union of underlying value types for audio gain control metadata.
#[repr(C)]
#[derive(Clone, Copy)]
pub union oar_metadata_gain_union_t {
    /// Fixed/Static gain value (specified in dB).
    pub constant_gain: f32,
    /// Raw pointer to an array containing one gain value per sample.
    pub gain_array: *mut f32,
    /// Dynamically interpolated animated gains.
    pub animated_gains: animated_float32_t,
}

/// Structure managing volume/gain adjustments with flexible temporal controls.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct oar_metadata_gain_t {
    /// Unique identifier for this gain parameter instance.
    pub id: u32,
    /// Selection representing how the gain value varies across samples.
    pub param_type: oar_param_type_t,
    /// The underlying static, sample-array, or animated gain data.
    pub value: oar_metadata_gain_union_t,
}

/// Structure representing IAMF downmix parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct oar_metadata_iamf_downmix_mode_t {
    /// Downmix mode identifier as specified by IAMF standards.
    pub mode: c_int,
}

/// Union containing polar, Cartesian, or animated coordinate blocks for up to 2 audio objects.
#[repr(C)]
#[derive(Clone, Copy)]
pub union oar_metadata_object_positions_union_t {
    /// Static polar coordinates array.
    pub polar_positions: [polar_t; def_max_number_of_objects],
    /// Static Cartesian coordinates array.
    pub cartesian_positions: [cartesian_t; def_max_number_of_objects],
    /// Animated polar coordinates array.
    pub animated_polar_positions: [animated_polar_t; def_max_number_of_objects],
    /// Animated Cartesian coordinates array.
    pub animated_cartesian_positions: [animated_cartesian_t; def_max_number_of_objects],
}

/// Complete 3D object positioning parameter metadata block.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct oar_metadata_object_positions_t {
    /// Selection representing how the positions vary across samples (constant vs. animated).
    pub param_type: oar_param_type_t,
    /// Selected coordinate layout scheme (Polar vs Cartesian).
    pub position_type: coordinate_type_t,
    /// Count of valid objects mapped (valid range: 1 to 2).
    pub num_objects: u32,
    /// Raw polar or Cartesian layout collections representing object points.
    pub positions: oar_metadata_object_positions_union_t,
}

/// Union representing specific metadata values based on metadata type selection.
#[repr(C)]
#[derive(Clone, Copy)]
pub union oar_metadata_union_t {
    /// Active gain adjustment properties.
    pub gain: oar_metadata_gain_t,
    /// Active IAMF downmix settings.
    pub iamf_downmix_mode: oar_metadata_iamf_downmix_mode_t,
    /// Active 3D object positions coordinates block.
    pub object_positions: oar_metadata_object_positions_t,
    /// Active rotational adjustment properties (for head tracking).
    pub head_rotation: quaternion_t,
}

/// Unified API metadata structure passed for dynamic rendering adjustments.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct oar_metadata_t {
    /// Specifies which variant of metadata is active in the value union.
    pub r#type: u32,
    /// Target metadata properties block.
    pub value: oar_metadata_union_t,
    /// Duration in samples for which this metadata payload is valid.
    pub duration: c_int,
}

/// Supported interpolation/animation schemes.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum animation_type_t {
    /// Step transition: value remains constant at `start` until next keyframe.
    ck_animation_type_step = 0,
    /// Linear transition: value is interpolated smoothly from `start` to `end`.
    ck_animation_type_linear = 1,
    /// Bezier transition: value curve is generated using control points.
    ck_animation_type_bezier = 2,
}

/// Interpolation parameters defining a single dimensional float animation segment.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct animated_data_float32_t {
    /// Starting parameter value.
    pub start: f32,
    /// Ending target parameter value (ignored in step animation).
    pub end: f32,
    /// Curved control point value (ignored in step/linear animation).
    pub control: f32,
    /// Normalized timing offset of the control point (0.0 to 1.0).
    pub control_relative_time: f32,
}

/// Animated 1D floating-point value.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct animated_float32_t {
    /// Selected interpolation curve type.
    pub animation_type: animation_type_t,
    /// Underlying animation segment parameters.
    pub data: animated_data_float32_t,
}

/// Animated polar coordinates.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct animated_polar_t {
    /// Global interpolation curve applied across all dimensional components.
    pub animation_type: animation_type_t,
    /// Horizontal angle animation parameters.
    pub azimuth: animated_data_float32_t,
    /// Vertical angle animation parameters.
    pub elevation: animated_data_float32_t,
    /// Distance animation parameters.
    pub distance: animated_data_float32_t,
}

/// Animated Cartesian coordinates.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct animated_cartesian_t {
    /// Global interpolation curve applied across all dimensional components.
    pub animation_type: animation_type_t,
    /// X axis position animation parameters.
    pub x: animated_data_float32_t,
    /// Y axis position animation parameters.
    pub y: animated_data_float32_t,
    /// Z axis position animation parameters.
    pub z: animated_data_float32_t,
}

/// A wrapper that manages the audio renderer and input buffers for FFI integration.
///
/// This struct bridges the C API and the safe Rust `RoarRenderer` implementation. It encapsulates
/// the renderer's state and caches input planar buffers for subsequent render calls.
pub struct FfiWrapper {
    /// The underlying active RoarRenderer instance.
    pub renderer: crate::renderer::RoarRenderer,
    /// Cache of input planar buffers for each active element ID.
    pub inputs: std::collections::HashMap<u32, Vec<Vec<f32>>>,
}
