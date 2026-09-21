// Copyright (c) 2026, Alliance for Open Media. All rights reserved
//
// This source code is subject to the terms of the BSD 3-Clause Clear License
// and the Alliance for Open Media Patent License 1.0. If the BSD 3-Clause Clear
// License was not distributed with this source code in the LICENSE file, you
// can obtain it at www.aomedia.org/license/software-license/bsd-3-c-c. If the
// Alliance for Open Media Patent License 1.0 was not distributed with this
// source code in the PATENTS file, you can obtain it at
// www.aomedia.org/license/patent.

//! Translations between C and Rust types for the C API.

use crate::c_types::oar_audio_block_t;
use crate::c_types::oar_audio_element_config_t;

impl oar_audio_block_t {
    /// Views the planar audio block as a collection of safe, immutable float slices.
    ///
    /// # Returns
    /// A `Vec` of immutable slices, one for each channel. If `data` is null, or if
    /// `channels` or `samples_per_channel` are 0, an empty `Vec` is returned.
    ///
    /// # Safety
    /// The caller must guarantee that:
    /// - `self.data` is either null or points to a valid block of contiguous memory
    ///   containing at least `channels * samples_per_channel` elements of type `f32`.
    /// - The memory region remains valid and is not modified for the lifetime of
    ///   the returned slices.
    pub unsafe fn as_slices(&self) -> Vec<&[f32]> {
        if self.data.is_null() || self.channels == 0 || self.samples_per_channel == 0 {
            return Vec::new();
        }

        let total_samples = match self.channels.checked_mul(self.samples_per_channel) {
            Some(t) => t as usize,
            None => return Vec::new(),
        };

        // SAFETY:
        // - We checked that `self.data` is not null and computed a safe `total_samples`.
        // - The caller guarantees that `self.data` points to a block of memory containing
        //   at least `channels * samples_per_channel` valid float values.
        // - The memory region remains unmodified for the duration of the returned immutable slice.
        let full_slice =
            unsafe { std::slice::from_raw_parts(self.data as *const f32, total_samples) };

        full_slice.chunks_exact(self.samples_per_channel as usize).collect()
    }

    /// Views the planar audio block as a collection of safe, mutable maybe uninitialized float
    /// slices.
    ///
    /// # Returns
    /// A `Vec` of mutable slices, one for each channel. If `data` is null, or if
    /// `channels` or `samples_per_channel` are 0, an empty `Vec` is returned.
    ///
    /// # Safety
    /// The caller must guarantee that:
    /// - `self.data` is either null or points to a contiguous block of allocated memory
    ///   containing at least `channels * samples_per_channel` elements of size `f32`.
    /// - The memory region is not accessed via any other pointers or references
    ///   for the lifetime of the returned mutable slices.
    pub unsafe fn as_slices_mut(&mut self) -> Vec<&mut [std::mem::MaybeUninit<f32>]> {
        if self.data.is_null() || self.channels == 0 || self.samples_per_channel == 0 {
            return Vec::new();
        }

        let total_samples = match self.channels.checked_mul(self.samples_per_channel) {
            Some(t) => t as usize,
            None => return Vec::new(),
        };

        // SAFETY:
        // - We checked that `self.data` is not null and computed a safe `total_samples`.
        // - The caller guarantees that `self.data` points to an allocated block of memory
        //   containing at least `channels * samples_per_channel` float elements.
        // - The memory region is accessed exclusively through the returned mutable slice for its
        //   lifetime.
        let full_slice = unsafe {
            std::slice::from_raw_parts_mut(
                self.data as *mut std::mem::MaybeUninit<f32>,
                total_samples,
            )
        };

        full_slice.chunks_exact_mut(self.samples_per_channel as usize).collect()
    }
}

impl oar_audio_element_config_t {
    /// Translates the C-compatible `oar_audio_element_config_t` configuration into a safe
    /// Rust `AudioElementConfig` representation.
    ///
    /// # Returns
    /// - `Ok(AudioElementConfig)` if the translation is successful.
    /// - `Err(OarError::InvalidParameter)` if the element type is unsupported, unrecognized,
    ///   or if an object-based configuration does not specify between 1 and 2 objects.
    pub fn to_safe(
        &self,
    ) -> Result<crate::common::definitions::AudioElementConfig, crate::common::definitions::OarError>
    {
        use crate::common::definitions::{
            AudioElementConfig, BinauralFilterProfile, ChannelBasedConfig, DownmixInfo,
            ElementRenderingConfig, HeadphonesRenderingMode, HighOrderAmbisonics, Layout, OarError,
            ObjectBasedConfig, SceneBasedConfig,
        };

        let rendering_config = if (self.parameters.flags
            & crate::c_types::parameter_set_t::def_parameter_set_flag_iamf_element_rendering_config)
            != 0
        {
            use crate::c_types::{oar_binaural_filter_profile_t, oar_headphones_rendering_mode_t};
            let mode = match self.parameters.element_rendering_config.headphones_rendering_mode {
                oar_headphones_rendering_mode_t::ck_world_locked_restricted => {
                    HeadphonesRenderingMode::WorldLockedRestricted
                }
                oar_headphones_rendering_mode_t::ck_world_locked => {
                    HeadphonesRenderingMode::WorldLocked
                }
                oar_headphones_rendering_mode_t::ck_head_locked => {
                    HeadphonesRenderingMode::HeadLocked
                }
                oar_headphones_rendering_mode_t::ck_reserved => HeadphonesRenderingMode::Reserved,
            };
            let profile = match self.parameters.element_rendering_config.binaural_filter_profile {
                oar_binaural_filter_profile_t::ck_ambient => BinauralFilterProfile::Ambient,
                oar_binaural_filter_profile_t::ck_direct => BinauralFilterProfile::Direct,
                oar_binaural_filter_profile_t::ck_reverberant => BinauralFilterProfile::Reverberant,
            };
            Some(ElementRenderingConfig {
                headphones_rendering_mode: mode,
                binaural_filter_profile: profile,
            })
        } else {
            None
        };

        match self.r#type {
            0 => {
                // SAFETY:
                // Accessing the `cbc` union field is safe because we have verified that
                // `self.type` is 0 (corresponding to ck_channel_based), meaning this variant of the
                // union is active.
                let cbc = unsafe { self.config.cbc };
                let layout = Layout::try_from(cbc.layout)?;
                match layout {
                    Layout::SoundSystemE451 | Layout::SoundSystemF370 | Layout::SoundSystemG490 => {
                        return Err(OarError::InvalidParameter)
                    }
                    _ => {}
                }
                let downmix_info = if (self.parameters.flags
                    & crate::c_types::parameter_set_t::def_parameter_set_flag_iamf_downmix_info)
                    != 0
                {
                    Some(DownmixInfo::try_from(self.parameters.downmix_info)?)
                } else {
                    None
                };
                Ok(AudioElementConfig::ChannelBased(ChannelBasedConfig {
                    layout,
                    downmix_info,
                    rendering_config,
                }))
            }
            1 => {
                // SAFETY:
                // Accessing the `sbc` union field is safe because we have verified that
                // `self.type` is 1 (corresponding to ck_scene_based), meaning this variant of the
                // union is active.
                let sbc = unsafe { self.config.sbc };
                let order = HighOrderAmbisonics::try_from(sbc.order)?;

                Ok(AudioElementConfig::SceneBased(SceneBasedConfig { order, rendering_config }))
            }
            2 => {
                // SAFETY:
                // Accessing the `obc` union field is safe because we have verified that
                // `self.type` is 2 (corresponding to ck_object_based), meaning this variant of the
                // union is active.
                let obc = unsafe { self.config.obc };
                if obc.num_objects < 1 || obc.num_objects > 2 {
                    return Err(OarError::InvalidParameter);
                }
                Ok(AudioElementConfig::ObjectBased(ObjectBasedConfig {
                    num_objects: obc.num_objects as u32,
                    rendering_config,
                }))
            }
            _ => Err(OarError::InvalidParameter),
        }
    }
}

impl crate::c_types::oar_metadata_t {
    /// Extracts head rotation from C-compatible `oar_metadata_t`.
    pub fn to_head_rotation(
        &self,
    ) -> Result<crate::common::definitions::Quaternion, crate::common::definitions::OarError> {
        use crate::common::definitions::{OarError, Quaternion};
        match self.r#type {
            4 => {
                // SAFETY:
                // Accessing the `head_rotation` union field is safe because we have verified that
                // `self.type` is 4 (corresponding to `ck_metadata_head_rotation`), meaning this
                // variant of the union is active.
                let q = unsafe { self.value.head_rotation };
                Ok(Quaternion { w: q.w, x: q.x, y: q.y, z: q.z })
            }
            _ => Err(OarError::NotSupported),
        }
    }
}

impl crate::c_types::oar_metadata_t {
    pub(crate) fn to_gain(
        &self,
    ) -> Result<crate::common::definitions::Gain, crate::common::definitions::OarError> {
        use crate::c_types::oar_metadata_type_t;
        use crate::c_types::oar_param_type_t;
        use crate::common::definitions::{AnimatedFloat32, Decibels, Gain, OarError};

        if self.r#type != (oar_metadata_type_t::ck_metadata_gain as u32) {
            return Err(OarError::InvalidParameter);
        }

        // SAFETY: Accessing the `gain` union field is safe because we have verified that
        // `self.type` is `ck_metadata_gain`.
        let gain_meta = unsafe { self.value.gain };
        match gain_meta.param_type {
            oar_param_type_t::ck_param_constant => {
                // SAFETY: Accessing the `constant_gain` union field is safe because we have
                // verified that `gain_meta.param_type` is `ck_param_constant`.
                let db_gain = unsafe { gain_meta.value.constant_gain };
                Gain::new_constant(Decibels::new(db_gain)?)
            }
            oar_param_type_t::ck_param_multiple => {
                // SAFETY: Accessing the `gain_array` union field is safe because we have
                // verified that `gain_meta.param_type` is `ck_param_multiple`.
                let gain_ptr = unsafe { gain_meta.value.gain_array };
                if gain_ptr.is_null() || self.duration <= 0 {
                    return Err(OarError::InvalidParameter);
                }
                // SAFETY: The caller guarantees that `gain_ptr` points to a valid array
                // of size at least `self.duration`.
                let db_gains =
                    unsafe { std::slice::from_raw_parts(gain_ptr, self.duration as usize) };
                let decibel_gains =
                    db_gains.iter().map(|&g| Decibels::new(g)).collect::<Result<Vec<_>, _>>()?;
                Gain::new_multiple(decibel_gains)
            }
            oar_param_type_t::ck_param_animated => {
                // SAFETY: Accessing the `animated_gains` union field is safe because we have
                // verified that `gain_meta.param_type` is `ck_param_animated`.
                let anim_gain = unsafe { gain_meta.value.animated_gains };
                let safe_anim = AnimatedFloat32::try_from(anim_gain)?;
                Gain::new_animated(safe_anim)
            }
        }
    }

    pub(crate) fn to_positions(
        &self,
    ) -> Result<crate::common::definitions::ObjectPosition, crate::common::definitions::OarError>
    {
        use crate::c_types::coordinate_type_t;
        use crate::c_types::oar_metadata_type_t;
        use crate::c_types::oar_param_type_t;
        use crate::common::definitions::{
            AnimatedCartesian, AnimatedPolar, CartesianCoordinate, OarError, ObjectPosition,
            PolarCoordinate,
        };

        if self.r#type != (oar_metadata_type_t::ck_metadata_object_positions as u32) {
            return Err(OarError::InvalidParameter);
        }

        // SAFETY: Accessing the `object_positions` union field is safe because we have verified
        // that `self.type` is `ck_metadata_object_positions`.
        let pos_meta = unsafe { self.value.object_positions };
        let num_objects = pos_meta.num_objects as usize;
        if num_objects > crate::common::definitions::DEF_MAX_NUMBER_OF_OBJECTS {
            return Err(OarError::InvalidParameter);
        }

        match pos_meta.param_type {
            oar_param_type_t::ck_param_constant => match pos_meta.position_type {
                coordinate_type_t::ck_polar => {
                    // SAFETY: Accessing the `polar_positions` union field is safe because we have
                    // verified that `pos_meta.position_type` is `ck_polar`.
                    let polar_positions = unsafe { pos_meta.positions.polar_positions };
                    let mut safe_positions = Vec::with_capacity(num_objects);
                    for p in &polar_positions[..num_objects] {
                        safe_positions.push(PolarCoordinate::new_from_floats(
                            p.azimuth,
                            p.elevation,
                            p.distance,
                        )?);
                    }
                    Ok(ObjectPosition::Polar(safe_positions))
                }
                coordinate_type_t::ck_cartesian => {
                    // SAFETY: Accessing the `cartesian_positions` union field is safe because we
                    // have verified that `pos_meta.position_type` is `ck_cartesian`.
                    let cartesian_positions = unsafe { pos_meta.positions.cartesian_positions };
                    let mut safe_positions = Vec::with_capacity(num_objects);
                    for c in &cartesian_positions[..num_objects] {
                        safe_positions.push(CartesianCoordinate { x: c.x, y: c.y, z: c.z });
                    }
                    Ok(ObjectPosition::Cartesian(safe_positions))
                }
            },
            oar_param_type_t::ck_param_animated => match pos_meta.position_type {
                coordinate_type_t::ck_polar => {
                    // SAFETY: Accessing the `animated_polar_positions` union field is safe because
                    // we have verified that `pos_meta.position_type` is `ck_polar`.
                    let anim_polar = unsafe { pos_meta.positions.animated_polar_positions };
                    let mut safe_positions = Vec::with_capacity(num_objects);
                    for &ap in &anim_polar[..num_objects] {
                        safe_positions.push(AnimatedPolar::try_from(ap)?);
                    }
                    Ok(ObjectPosition::AnimatedPolar(safe_positions))
                }
                coordinate_type_t::ck_cartesian => {
                    // SAFETY: Accessing the `animated_cartesian_positions` union field is safe
                    // because we have verified that `pos_meta.position_type` is `ck_cartesian`.
                    let anim_cart = unsafe { pos_meta.positions.animated_cartesian_positions };
                    let mut safe_positions = Vec::with_capacity(num_objects);
                    for &ac in &anim_cart[..num_objects] {
                        safe_positions.push(AnimatedCartesian::try_from(ac)?);
                    }
                    Ok(ObjectPosition::AnimatedCartesian(safe_positions))
                }
            },
            _ => Err(OarError::NotSupported),
        }
    }
    pub(crate) fn to_downmix_mode(
        &self,
    ) -> Result<crate::common::definitions::DownmixMode, crate::common::definitions::OarError> {
        use crate::c_types::oar_metadata_type_t;
        use crate::common::definitions::{DownmixMode, OarError};
        if self.r#type != (oar_metadata_type_t::ck_metadata_iamf_downmix_mode as u32) {
            return Err(OarError::InvalidParameter);
        }
        // SAFETY: Accessing the `iamf_downmix_mode` union field is safe because we have verified
        // that `self.type` is `ck_metadata_iamf_downmix_mode`.
        let mode = unsafe { self.value.iamf_downmix_mode.mode };
        DownmixMode::try_from(mode)
    }
}

impl TryFrom<i32> for crate::common::definitions::HighOrderAmbisonics {
    type Error = crate::common::definitions::OarError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        use crate::c_types::oar_hoa_t;
        use crate::common::definitions::{HighOrderAmbisonics, OarError};

        const ZOA: i32 = oar_hoa_t::ck_oar_zoa as i32;
        const OA1: i32 = oar_hoa_t::ck_oar_1oa as i32;
        const OA2: i32 = oar_hoa_t::ck_oar_2oa as i32;
        const OA3: i32 = oar_hoa_t::ck_oar_3oa as i32;
        const OA4: i32 = oar_hoa_t::ck_oar_4oa as i32;

        match value {
            ZOA => Ok(HighOrderAmbisonics::Zoa),
            OA1 => Ok(HighOrderAmbisonics::Order1),
            OA2 => Ok(HighOrderAmbisonics::Order2),
            OA3 => Ok(HighOrderAmbisonics::Order3),
            OA4 => Ok(HighOrderAmbisonics::Order4),
            _ => Err(OarError::InvalidParameter),
        }
    }
}

impl TryFrom<crate::c_types::polar_t> for crate::common::definitions::PolarCoordinate {
    type Error = crate::common::definitions::OarError;

    fn try_from(c: crate::c_types::polar_t) -> Result<Self, Self::Error> {
        crate::common::definitions::PolarCoordinate::new_from_floats(
            c.azimuth,
            c.elevation,
            c.distance,
        )
    }
}

impl TryFrom<crate::c_types::cartesian_t> for crate::common::definitions::CartesianCoordinate {
    type Error = crate::common::definitions::OarError;

    fn try_from(c: crate::c_types::cartesian_t) -> Result<Self, Self::Error> {
        crate::common::definitions::CartesianCoordinate::new(c.x, c.y, c.z)
    }
}

impl TryFrom<crate::c_types::quaternion_t> for crate::common::definitions::Quaternion {
    type Error = crate::common::definitions::OarError;

    fn try_from(c: crate::c_types::quaternion_t) -> Result<Self, Self::Error> {
        crate::common::definitions::Quaternion::new(c.w, c.x, c.y, c.z)
    }
}

impl TryFrom<i32> for crate::common::definitions::Layout {
    type Error = crate::common::definitions::OarError;

    fn try_from(layout: i32) -> Result<Self, Self::Error> {
        use crate::c_types::oar_layout_t;
        use crate::common::definitions::{Layout, OarError};

        const MONO: i32 = oar_layout_t::ck_oar_layout_mono as i32;
        const STEREO: i32 = oar_layout_t::ck_oar_layout_stereo as i32;
        const L51: i32 = oar_layout_t::ck_oar_layout_51 as i32;
        const L512: i32 = oar_layout_t::ck_oar_layout_512 as i32;
        const L514: i32 = oar_layout_t::ck_oar_layout_514 as i32;
        const L71: i32 = oar_layout_t::ck_oar_layout_71 as i32;
        const L712: i32 = oar_layout_t::ck_oar_layout_712 as i32;
        const L714: i32 = oar_layout_t::ck_oar_layout_714 as i32;
        const L312: i32 = oar_layout_t::ck_oar_layout_312 as i32;
        const L916: i32 = oar_layout_t::ck_oar_layout_916 as i32;
        const LA293: i32 = oar_layout_t::ck_oar_layout_a293 as i32;
        const L7154: i32 = oar_layout_t::ck_oar_layout_7154 as i32;
        const E451: i32 = oar_layout_t::ck_oar_layout_sound_system_e_451 as i32;
        const F370: i32 = oar_layout_t::ck_oar_layout_sound_system_f_370 as i32;
        const G490: i32 = oar_layout_t::ck_oar_layout_sound_system_g_490 as i32;
        const BINAURAL: i32 = oar_layout_t::ck_oar_layout_binaural as i32;
        const LFE: i32 = oar_layout_t::ck_oar_layout_lfe as i32;
        const STEREOS: i32 = oar_layout_t::ck_oar_layout_stereo_s as i32;
        const STEREOSS: i32 = oar_layout_t::ck_oar_layout_stereo_ss as i32;
        const STEREO_RS: i32 = oar_layout_t::ck_oar_layout_stereo_rs as i32;
        const STEREO_TF: i32 = oar_layout_t::ck_oar_layout_stereo_tf as i32;
        const STEREO_TB: i32 = oar_layout_t::ck_oar_layout_stereo_tb as i32;
        const TOP4CH: i32 = oar_layout_t::ck_oar_layout_top_4ch as i32;
        const THREECH: i32 = oar_layout_t::ck_oar_layout_3ch as i32;
        const STEREO_F: i32 = oar_layout_t::ck_oar_layout_stereo_f as i32;
        const STEREO_SI: i32 = oar_layout_t::ck_oar_layout_stereo_si as i32;
        const STEREO_TPSI: i32 = oar_layout_t::ck_oar_layout_stereo_tpsi as i32;
        const TOP6CH: i32 = oar_layout_t::ck_oar_layout_top_6ch as i32;
        const LFE_PAIR: i32 = oar_layout_t::ck_oar_layout_lfe_pair as i32;
        const BOTTOM3CH: i32 = oar_layout_t::ck_oar_layout_bottom_3ch as i32;
        const BOTTOM4CH: i32 = oar_layout_t::ck_oar_layout_bottom_4ch as i32;
        const TOP1CH: i32 = oar_layout_t::ck_oar_layout_top_1ch as i32;
        const TOP5CH: i32 = oar_layout_t::ck_oar_layout_top_5ch as i32;

        match layout {
            MONO => Ok(Layout::Mono),
            STEREO => Ok(Layout::Stereo),
            L51 => Ok(Layout::Layout51),
            L512 => Ok(Layout::Layout512),
            L514 => Ok(Layout::Layout514),
            L71 => Ok(Layout::Layout71),
            L712 => Ok(Layout::Layout712),
            L714 => Ok(Layout::Layout714),
            L312 => Ok(Layout::Layout312),
            L916 => Ok(Layout::Layout916),
            LA293 => Ok(Layout::LayoutA293),
            L7154 => Ok(Layout::Layout7154),
            E451 => Ok(Layout::SoundSystemE451),
            F370 => Ok(Layout::SoundSystemF370),
            G490 => Ok(Layout::SoundSystemG490),
            BINAURAL => Ok(Layout::Binaural),
            LFE => Ok(Layout::Lfe),
            STEREOS => Ok(Layout::StereoS),
            STEREOSS => Ok(Layout::StereoSs),
            STEREO_RS => Ok(Layout::StereoRs),
            STEREO_TF => Ok(Layout::StereoTf),
            STEREO_TB => Ok(Layout::StereoTb),
            TOP4CH => Ok(Layout::Top4ch),
            THREECH => Ok(Layout::ThreeCh),
            STEREO_F => Ok(Layout::StereoF),
            STEREO_SI => Ok(Layout::StereoSi),
            STEREO_TPSI => Ok(Layout::StereoTpsi),
            TOP6CH => Ok(Layout::Top6ch),
            LFE_PAIR => Ok(Layout::LfePair),
            BOTTOM3CH => Ok(Layout::Bottom3ch),
            BOTTOM4CH => Ok(Layout::Bottom4ch),
            TOP1CH => Ok(Layout::Top1ch),
            TOP5CH => Ok(Layout::Top5ch),
            _ => Err(OarError::InvalidParameter),
        }
    }
}

impl TryFrom<crate::c_types::oar_config_t> for crate::common::definitions::Config {
    type Error = crate::common::definitions::OarError;
    fn try_from(c: crate::c_types::oar_config_t) -> Result<Self, Self::Error> {
        use crate::common::definitions::{Config, Layout, SampleRate, Samples};
        let target_layout = Layout::try_from(c.target_layout)?;
        let spc = Samples::new(c.samples_per_channel)?;
        let sr = SampleRate::new(c.sampling_rate)?;
        Config::new(target_layout, spc, sr)
    }
}

impl TryFrom<i32> for crate::common::definitions::DownmixMode {
    type Error = crate::common::definitions::OarError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        use crate::common::definitions::{DownmixMode, OarError};
        match value {
            0 => Ok(DownmixMode::Mode1NegOffset),
            1 => Ok(DownmixMode::Mode2NegOffset),
            2 => Ok(DownmixMode::Mode3NegOffset),
            4 => Ok(DownmixMode::Mode1PosOffset),
            5 => Ok(DownmixMode::Mode2PosOffset),
            6 => Ok(DownmixMode::Mode3PosOffset),
            _ => Err(OarError::InvalidParameter),
        }
    }
}

impl TryFrom<crate::c_types::downmix_info_t> for crate::common::definitions::DownmixInfo {
    type Error = crate::common::definitions::OarError;

    fn try_from(c: crate::c_types::downmix_info_t) -> Result<Self, Self::Error> {
        use crate::common::definitions::{DownmixInfo, DownmixMode, WeightIndex};
        let mode = DownmixMode::try_from(c.mode)?;
        let weight_index = if (0..=10).contains(&c.weight_index) {
            Some(WeightIndex::new(c.weight_index)?)
        } else {
            None
        };
        Ok(DownmixInfo::new(mode, weight_index))
    }
}

impl TryFrom<crate::c_types::oar_headphones_rendering_mode_t>
    for crate::common::definitions::HeadphonesRenderingMode
{
    type Error = crate::common::definitions::OarError;

    fn try_from(val: crate::c_types::oar_headphones_rendering_mode_t) -> Result<Self, Self::Error> {
        use crate::c_types::oar_headphones_rendering_mode_t;
        use crate::common::definitions::HeadphonesRenderingMode;
        match val {
            oar_headphones_rendering_mode_t::ck_world_locked_restricted => {
                Ok(HeadphonesRenderingMode::WorldLockedRestricted)
            }
            oar_headphones_rendering_mode_t::ck_world_locked => {
                Ok(HeadphonesRenderingMode::WorldLocked)
            }
            oar_headphones_rendering_mode_t::ck_head_locked => {
                Ok(HeadphonesRenderingMode::HeadLocked)
            }
            oar_headphones_rendering_mode_t::ck_reserved => Ok(HeadphonesRenderingMode::Reserved),
        }
    }
}

impl TryFrom<crate::c_types::oar_binaural_filter_profile_t>
    for crate::common::definitions::BinauralFilterProfile
{
    type Error = crate::common::definitions::OarError;

    fn try_from(val: crate::c_types::oar_binaural_filter_profile_t) -> Result<Self, Self::Error> {
        use crate::c_types::oar_binaural_filter_profile_t;
        use crate::common::definitions::BinauralFilterProfile;
        match val {
            oar_binaural_filter_profile_t::ck_ambient => Ok(BinauralFilterProfile::Ambient),
            oar_binaural_filter_profile_t::ck_direct => Ok(BinauralFilterProfile::Direct),
            oar_binaural_filter_profile_t::ck_reverberant => Ok(BinauralFilterProfile::Reverberant),
        }
    }
}

impl TryFrom<crate::c_types::audio_element_rendering_config_t>
    for crate::common::definitions::ElementRenderingConfig
{
    type Error = crate::common::definitions::OarError;

    fn try_from(c: crate::c_types::audio_element_rendering_config_t) -> Result<Self, Self::Error> {
        use crate::common::definitions::{
            BinauralFilterProfile, ElementRenderingConfig, HeadphonesRenderingMode,
        };
        let headphones_rendering_mode =
            HeadphonesRenderingMode::try_from(c.headphones_rendering_mode)?;
        let binaural_filter_profile = BinauralFilterProfile::try_from(c.binaural_filter_profile)?;
        Ok(ElementRenderingConfig { headphones_rendering_mode, binaural_filter_profile })
    }
}

impl TryFrom<crate::c_types::animated_float32_t> for crate::common::definitions::AnimatedFloat32 {
    type Error = crate::common::definitions::OarError;

    fn try_from(c: crate::c_types::animated_float32_t) -> Result<Self, Self::Error> {
        use crate::c_types::animation_type_t;
        use crate::common::definitions::{Animated, OarError};

        let d = c.data;
        if !d.start.is_finite() {
            return Err(OarError::InvalidParameter);
        }

        match c.animation_type {
            animation_type_t::ck_animation_type_step => Ok(Animated::Step { value: d.start }),
            animation_type_t::ck_animation_type_linear => {
                if !d.end.is_finite() {
                    return Err(OarError::InvalidParameter);
                }
                Ok(Animated::Linear { start: d.start, end: d.end })
            }
            animation_type_t::ck_animation_type_bezier => {
                if !d.end.is_finite()
                    || !d.control.is_finite()
                    || !d.control_relative_time.is_finite()
                    || !(0.0..=1.0).contains(&d.control_relative_time)
                {
                    return Err(OarError::InvalidParameter);
                }
                Ok(Animated::Bezier {
                    start: d.start,
                    end: d.end,
                    control: d.control,
                    control_relative_time: d.control_relative_time,
                })
            }
        }
    }
}

impl TryFrom<crate::c_types::animated_polar_t> for crate::common::definitions::AnimatedPolar {
    type Error = crate::common::definitions::OarError;

    fn try_from(c: crate::c_types::animated_polar_t) -> Result<Self, Self::Error> {
        use crate::c_types::animation_type_t;
        use crate::common::definitions::{
            AnimatedPolar, OarError, PolarControlRelativeTime, PolarCoordinate,
        };

        fn to_polar(az: f32, el: f32, dist: f32) -> Result<PolarCoordinate, OarError> {
            if !az.is_finite() || !el.is_finite() || !dist.is_finite() {
                return Err(OarError::InvalidParameter);
            }
            if !(-90.0..=90.0).contains(&el) || dist < 0.0 {
                return Err(OarError::InvalidParameter);
            }
            let mut norm_az = az % 360.0;
            if norm_az > 180.0 {
                norm_az -= 360.0;
            } else if norm_az < -180.0 {
                norm_az += 360.0;
            }
            PolarCoordinate::new_from_floats(norm_az, el, dist)
        }

        let start = to_polar(c.azimuth.start, c.elevation.start, c.distance.start)?;
        match c.animation_type {
            animation_type_t::ck_animation_type_step => Ok(AnimatedPolar::Step { value: start }),
            animation_type_t::ck_animation_type_linear => {
                let end = to_polar(c.azimuth.end, c.elevation.end, c.distance.end)?;
                Ok(AnimatedPolar::Linear { start, end })
            }
            animation_type_t::ck_animation_type_bezier => {
                let end = to_polar(c.azimuth.end, c.elevation.end, c.distance.end)?;
                let control = to_polar(c.azimuth.control, c.elevation.control, c.distance.control)?;
                let t_ctrl_az = c.azimuth.control_relative_time;
                let t_ctrl_el = c.elevation.control_relative_time;
                let t_ctrl_dist = c.distance.control_relative_time;
                if !t_ctrl_az.is_finite()
                    || !(0.0..=1.0).contains(&t_ctrl_az)
                    || !t_ctrl_el.is_finite()
                    || !(0.0..=1.0).contains(&t_ctrl_el)
                    || !t_ctrl_dist.is_finite()
                    || !(0.0..=1.0).contains(&t_ctrl_dist)
                {
                    return Err(OarError::InvalidParameter);
                }
                Ok(AnimatedPolar::Bezier {
                    start,
                    end,
                    control,
                    control_relative_time: PolarControlRelativeTime {
                        azimuth: t_ctrl_az,
                        elevation: t_ctrl_el,
                        distance: t_ctrl_dist,
                    },
                })
            }
        }
    }
}

impl TryFrom<crate::c_types::animated_cartesian_t>
    for crate::common::definitions::AnimatedCartesian
{
    type Error = crate::common::definitions::OarError;

    fn try_from(c: crate::c_types::animated_cartesian_t) -> Result<Self, Self::Error> {
        use crate::c_types::animation_type_t;
        use crate::common::definitions::{
            AnimatedCartesian, CartesianControlRelativeTime, CartesianCoordinate, OarError,
        };

        fn to_cartesian(x: f32, y: f32, z: f32) -> Result<CartesianCoordinate, OarError> {
            if !x.is_finite() || !y.is_finite() || !z.is_finite() {
                return Err(OarError::InvalidParameter);
            }
            Ok(CartesianCoordinate { x, y, z })
        }

        let start = to_cartesian(c.x.start, c.y.start, c.z.start)?;
        match c.animation_type {
            animation_type_t::ck_animation_type_step => {
                Ok(AnimatedCartesian::Step { value: start })
            }
            animation_type_t::ck_animation_type_linear => {
                let end = to_cartesian(c.x.end, c.y.end, c.z.end)?;
                Ok(AnimatedCartesian::Linear { start, end })
            }
            animation_type_t::ck_animation_type_bezier => {
                let end = to_cartesian(c.x.end, c.y.end, c.z.end)?;
                let control = to_cartesian(c.x.control, c.y.control, c.z.control)?;
                let t_ctrl_x = c.x.control_relative_time;
                let t_ctrl_y = c.y.control_relative_time;
                let t_ctrl_z = c.z.control_relative_time;
                if !t_ctrl_x.is_finite()
                    || !(0.0..=1.0).contains(&t_ctrl_x)
                    || !t_ctrl_y.is_finite()
                    || !(0.0..=1.0).contains(&t_ctrl_y)
                    || !t_ctrl_z.is_finite()
                    || !(0.0..=1.0).contains(&t_ctrl_z)
                {
                    return Err(OarError::InvalidParameter);
                }
                Ok(AnimatedCartesian::Bezier {
                    start,
                    end,
                    control,
                    control_relative_time: CartesianControlRelativeTime {
                        x: t_ctrl_x,
                        y: t_ctrl_y,
                        z: t_ctrl_z,
                    },
                })
            }
        }
    }
}

// ===== Tests =====

#[cfg(test)]
mod test {
    use crate::c_types::{oar_metadata_t, oar_metadata_union_t, quaternion_t};
    use googletest::prelude::*;

    #[gtest]
    fn test_metadata_layout() {
        let rotation = quaternion_t { w: 0.0, x: 0.0, y: 0.0, z: 0.0 };
        let meta = oar_metadata_t {
            r#type: 0,
            value: oar_metadata_union_t { head_rotation: rotation },
            duration: 0,
        };
        let base = &meta as *const _ as usize;
        let duration = &meta.duration as *const _ as usize;
        let value = &meta.value as *const _ as usize;

        expect_that!(std::mem::size_of::<oar_metadata_t>(), eq(136));
        expect_that!(duration - base, eq(128));
        expect_that!(value - base, eq(8));
    }
}
