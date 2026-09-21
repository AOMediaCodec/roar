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

use super::downmix::DownmixInfo;
use super::hoa::HighOrderAmbisonics;
use super::layout::Layout;
use super::units::Samples;
use crate::common::definitions::{OarError, MAX_SAMPLES_PER_CHANNEL};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SampleRate(u32);

impl SampleRate {
    pub fn new(val: u32) -> Result<Self, OarError> {
        if val > 0 {
            Ok(SampleRate(val))
        } else {
            Err(OarError::InvalidParameter)
        }
    }

    pub fn value(&self) -> u32 {
        self.0
    }
}

/// Configuration for creating the ROAR instance, equivalent of C API `oar_config_t`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    /// The target output layout.
    target_layout: Layout,
    /// Number of samples per channel per render call.
    samples_per_channel: Samples,
    /// The sample rate in Hz.
    sample_rate: SampleRate,
}

impl Config {
    pub fn new(
        target_layout: Layout,
        samples_per_channel: Samples,
        sample_rate: SampleRate,
    ) -> Result<Self, OarError> {
        if samples_per_channel.value() > MAX_SAMPLES_PER_CHANNEL {
            return Err(OarError::InvalidParameter);
        }
        Ok(Config { target_layout, samples_per_channel, sample_rate })
    }

    /// Get output (target) layout.
    pub fn output_layout(&self) -> Layout {
        self.target_layout
    }

    /// Get samples per channel (per render call).
    pub fn samples_per_channel(&self) -> Samples {
        self.samples_per_channel
    }

    /// Get sample rate.
    pub fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }
}

/// Headphones rendering mode as specified by IAMF.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HeadphonesRenderingMode {
    /// Stereo rendering for this element (no binaural rendering).
    #[default]
    WorldLockedRestricted = 0,
    /// Binaural rendering with full head-tracking (world-locked).
    WorldLocked = 1,
    /// Binaural rendering locked to the listener's head.
    HeadLocked = 2,
    /// Reserved mode.
    Reserved = 3,
}

/// Selected binaural filter profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BinauralFilterProfile {
    /// Ambient binaural filter profile.
    #[default]
    Ambient = 0,
    /// Direct binaural filter profile.
    Direct = 1,
    /// Reverberant binaural filter profile.
    Reverberant = 2,
}

/// Configures how a channel-based audio element is rendered on headphones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ElementRenderingConfig {
    /// The headphones rendering mode to use.
    pub headphones_rendering_mode: HeadphonesRenderingMode,
    /// The binaural filter profile to apply.
    pub binaural_filter_profile: BinauralFilterProfile,
}

/// Configuration for channel-based audio element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelBasedConfig {
    /// The loudspeaker layout configuration.
    pub layout: Layout,
    /// Optional initial downmix parameters.
    pub downmix_info: Option<DownmixInfo>,
    ///Optional binaural rendering config.
    pub rendering_config: Option<ElementRenderingConfig>,
}

/// Configuration for scene-based (HOA) audio element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneBasedConfig {
    /// The higher order ambisonics order.
    pub order: HighOrderAmbisonics,
    ///Optional binaural rendering config.
    pub rendering_config: Option<ElementRenderingConfig>,
}

/// Configuration for object-based audio element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectBasedConfig {
    /// The number of objects.
    pub num_objects: u32,
    ///Optional binaural rendering config.
    pub rendering_config: Option<ElementRenderingConfig>,
}

/// Configuration for an audio element, equivalent of C API `oar_audio_element_config_t`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioElementConfig {
    /// Channel-based layout configuration.
    ChannelBased(ChannelBasedConfig),
    /// Scene-based (Ambisonics) configuration.
    SceneBased(SceneBasedConfig),
    /// Object-based configuration.
    ObjectBased(ObjectBasedConfig),
}

impl AudioElementConfig {
    /// Returns the number of channels for this element configuration.
    pub fn channels(&self) -> usize {
        match self {
            AudioElementConfig::ChannelBased(cbc) => cbc.layout.channels(),
            AudioElementConfig::SceneBased(sbc) => sbc.order.channels(),
            AudioElementConfig::ObjectBased(obc) => obc.num_objects as usize,
        }
    }

    /// Returns the headphones rendering configuration if present.
    pub fn rendering_config(&self) -> Option<ElementRenderingConfig> {
        match self {
            AudioElementConfig::ChannelBased(cbc) => cbc.rendering_config,
            AudioElementConfig::SceneBased(sbc) => sbc.rendering_config,
            AudioElementConfig::ObjectBased(obc) => obc.rendering_config,
        }
    }
}

// ===== Tests =====

#[cfg(test)]
mod test {

    use super::*;
    use googletest::prelude::*;

    #[gtest]
    fn test_oar_config_accessors() {
        let config = Config::new(
            Layout::Layout714,
            Samples::new(512).unwrap(),
            SampleRate::new(48000).unwrap(),
        )
        .unwrap();

        expect_that!(config.output_layout(), eq(Layout::Layout714));
        expect_that!(config.sample_rate().value(), eq(48000));
        expect_that!(config.samples_per_channel().value(), eq(512));
    }

    #[gtest]
    fn test_oar_config_validation() {
        // Valid configuration
        expect_ok!(Config::new(
            Layout::Stereo,
            Samples::new(512).unwrap(),
            SampleRate::new(48000).unwrap(),
        ));
        // Invalid samples (too large for config limit of 16384)
        expect_true!(Config::new(
            Layout::Stereo,
            Samples::new(20000).unwrap(),
            SampleRate::new(48000).unwrap(),
        )
        .is_err());
    }

    #[gtest]
    fn test_sample_rate_validation() {
        // Valid SampleRate
        expect_ok!(SampleRate::new(48000));
        // Invalid sample rate (0)
        expect_true!(SampleRate::new(0).is_err());
    }

    #[gtest]
    fn test_audio_element_config_comparisons_and_traits() {
        let ch_config = ChannelBasedConfig {
            layout: Layout::Stereo,
            downmix_info: None,
            rendering_config: None,
        };
        let sc_config =
            SceneBasedConfig { order: HighOrderAmbisonics::Order1, rendering_config: None };
        let obj_config = ObjectBasedConfig { num_objects: 8, rendering_config: None };

        let el_ch = AudioElementConfig::ChannelBased(ch_config);
        let el_sc = AudioElementConfig::SceneBased(sc_config);
        let el_obj = AudioElementConfig::ObjectBased(obj_config);

        expect_that!(el_ch, not(eq(el_sc)));
        expect_that!(el_ch, not(eq(el_obj)));

        let el_ch_clone = el_ch;
        expect_that!(el_ch, eq(el_ch_clone));
    }
}
