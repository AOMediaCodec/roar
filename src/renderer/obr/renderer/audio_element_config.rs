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

//! Configuration representations for OBR audio elements.
//!
//! Defines the variant enum and configuration structure for each individual audio
//! element (input block) processed by the OBR subsystem.

use super::input_channel_config::AudioObjectInputChannel;
use crate::common::definitions::{
    BinauralFilterProfile, Degrees, Distance, HighOrderAmbisonics, InputLayout, OarError,
    SpeakerPosition,
};

/// Necessary configuration information dependent on the type of AudioElement.
///
/// Distinguishes between periphonic Ambisonic scenes (orders 1..=4), channel-based
/// loudspeaker layouts, spatialized mono/dual objects, and mono/stereo passthrough signals.
#[derive(Debug, Clone, PartialEq)]
pub enum AudioElementVariant {
    /// Periphonic Ambisonic scene (orders 1..=4).
    Ambisonics(HighOrderAmbisonics),
    /// Channel-based loudspeaker or subset layout with virtual speaker positions.
    LoudspeakerLayout(Vec<SpeakerPosition>),
    /// Spatialized audio object channels (mono or dual).
    Objects(Vec<AudioObjectInputChannel>),
    /// Single-channel mono passthrough signal (e.g. zeroth-order Ambisonics).
    PassthroughMono,
    /// Two-channel stereo passthrough signal (binaural input).
    PassthroughStereo,
}

impl AudioElementVariant {
    /// Creates an Ambisonic or mono passthrough variant from a `HighOrderAmbisonics` order.
    ///
    /// Zeroth-order Ambisonics (`Zoa`) maps to `PassthroughMono`; orders 1 through 4 map
    /// to `Ambisonics(order)`.
    pub fn ambisonics(order: HighOrderAmbisonics) -> Self {
        match order {
            HighOrderAmbisonics::Zoa => Self::PassthroughMono,
            HighOrderAmbisonics::Order1
            | HighOrderAmbisonics::Order2
            | HighOrderAmbisonics::Order3
            | HighOrderAmbisonics::Order4 => Self::Ambisonics(order),
        }
    }

    /// Creates a loudspeaker layout or stereo passthrough variant for `layout`.
    ///
    /// `InputLayout::Binaural` (which has no virtual speaker positions) maps to
    /// `PassthroughStereo`; all loudspeaker and subset layouts map to `LoudspeakerLayout`.
    pub fn loudspeaker_layout(layout: InputLayout) -> Self {
        layout.obr_speakers().map_or(Self::PassthroughStereo, Self::LoudspeakerLayout)
    }

    /// Creates an object variant with `num_objects` channels.
    ///
    /// # Parameters
    /// * `num_objects` - Number of object channels; must be 1 (mono) or 2 (dual).
    pub fn objects(num_objects: usize) -> Result<Self, OarError> {
        if !(1..=2).contains(&num_objects) {
            return Err(OarError::InvalidParameter);
        }
        let default_channel =
            AudioObjectInputChannel::new(Degrees::ZERO, Degrees::ZERO, Distance::new(1.0).unwrap());
        Ok(Self::Objects(vec![default_channel; num_objects]))
    }

    /// Creates a single-channel spatialized audio object variant.
    pub fn object_mono() -> Self {
        Self::objects(1).unwrap()
    }

    /// Creates a dual-channel spatialized audio object variant.
    pub fn object_dual() -> Self {
        Self::objects(2).unwrap()
    }
}

/// Configuration settings for an individual audio element.
///
/// Manages the mapping of input channels to spatial representations (Ambisonic scene
/// channels, virtual speaker channels, dynamic objects, or mono passthrough) and
/// specifies target binaural filter profiles.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioElementConfig {
    variant: AudioElementVariant,
    first_channel_index: usize,
    binaural_filter_profile: BinauralFilterProfile,
    head_locked: bool,
}

impl AudioElementConfig {
    /// Creates a new `AudioElementConfig` instance.
    pub fn new(variant: AudioElementVariant, filter_profile: BinauralFilterProfile) -> Self {
        let variant = match variant {
            AudioElementVariant::Ambisonics(HighOrderAmbisonics::Zoa) => {
                AudioElementVariant::PassthroughMono
            }
            other => other,
        };
        Self {
            variant,
            first_channel_index: 0,
            binaural_filter_profile: filter_profile,
            head_locked: false,
        }
    }

    /// Returns a reference to the element variant.
    pub fn variant(&self) -> &AudioElementVariant {
        &self.variant
    }

    /// Returns `true` if this element is a periphonic Ambisonic scene (orders 1..=4).
    pub fn is_ambisonics(&self) -> bool {
        matches!(self.variant, AudioElementVariant::Ambisonics(_))
    }

    /// Returns `true` if this element is a mono or stereo passthrough signal.
    pub fn is_passthrough(&self) -> bool {
        matches!(
            self.variant,
            AudioElementVariant::PassthroughMono | AudioElementVariant::PassthroughStereo
        )
    }

    /// Returns `true` if this element is encoded via the Ambisonic encoder (loudspeaker layout or
    /// objects).
    pub fn uses_ambisonic_encoder(&self) -> bool {
        matches!(
            self.variant,
            AudioElementVariant::LoudspeakerLayout(_) | AudioElementVariant::Objects(_)
        )
    }

    /// Sets the base `first_channel_index` for the element.
    pub fn set_first_channel_index(&mut self, first_channel: usize) {
        self.first_channel_index = first_channel;
    }

    /// Returns the offset index of the first input channel for this element.
    pub fn get_first_channel_index(&self) -> usize {
        self.first_channel_index
    }

    /// Returns the total number of input channels for this element.
    pub fn get_number_of_input_channels(&self) -> usize {
        match &self.variant {
            AudioElementVariant::Ambisonics(order) => order.channels().get(),
            AudioElementVariant::LoudspeakerLayout(channels) => channels.len(),
            AudioElementVariant::Objects(channels) => channels.len(),
            AudioElementVariant::PassthroughMono => 1,
            AudioElementVariant::PassthroughStereo => 2,
        }
    }

    /// Returns the loudspeaker layout speaker positions if this is a loudspeaker layout element.
    pub fn get_loudspeaker_channels(&self) -> &[SpeakerPosition] {
        match &self.variant {
            AudioElementVariant::LoudspeakerLayout(channels) => channels,
            _ => &[],
        }
    }

    /// Returns a mutable slice of audio object input channels if this is an object element.
    pub fn get_object_channels(&mut self) -> &mut [AudioObjectInputChannel] {
        match &mut self.variant {
            AudioElementVariant::Objects(channels) => channels,
            _ => &mut [],
        }
    }

    /// Returns the Ambisonic order used for rendering this element's filters.
    pub fn get_binaural_filters_ambisonic_order(&self) -> i32 {
        match &self.variant {
            AudioElementVariant::Ambisonics(order) => *order as i32,
            AudioElementVariant::LoudspeakerLayout(_) | AudioElementVariant::Objects(_) => 3,
            AudioElementVariant::PassthroughMono | AudioElementVariant::PassthroughStereo => 0,
        }
    }

    /// Returns the filter profile assigned to this element.
    pub fn get_binaural_filter_profile(&self) -> BinauralFilterProfile {
        self.binaural_filter_profile
    }

    /// Returns whether this element's channels are locked to the listener's head.
    pub fn is_head_locked(&self) -> bool {
        self.head_locked
    }

    /// Sets whether this element's channels should be locked to the listener's head.
    pub fn set_head_locked(&mut self, head_locked: bool) {
        self.head_locked = head_locked;
    }
}

// ===== Tests =====

#[cfg(test)]
mod test {

    use super::*;
    use googletest::prelude::*;

    #[gtest]
    fn audio_element_config_ambisonics_initializes_order_and_channel_count() {
        let config = AudioElementConfig::new(
            AudioElementVariant::ambisonics(HighOrderAmbisonics::Order1),
            BinauralFilterProfile::Direct,
        );

        expect_true!(config.is_ambisonics());
        expect_false!(config.is_passthrough());
        expect_false!(config.uses_ambisonic_encoder());
        expect_eq!(config.get_number_of_input_channels(), 4);
        expect_eq!(config.get_binaural_filters_ambisonic_order(), 1);
        expect_eq!(config.get_binaural_filter_profile(), BinauralFilterProfile::Direct);
        expect_false!(config.is_head_locked());
    }

    #[gtest]
    fn audio_element_config_object_initializes_order_and_channels() {
        let config = AudioElementConfig::new(
            AudioElementVariant::object_mono(),
            BinauralFilterProfile::Ambient,
        );

        expect_false!(config.is_ambisonics());
        expect_true!(config.uses_ambisonic_encoder());
        expect_eq!(config.get_number_of_input_channels(), 1);
        expect_eq!(config.get_binaural_filters_ambisonic_order(), 3);
        expect_eq!(config.get_binaural_filter_profile(), BinauralFilterProfile::Ambient);
    }

    #[gtest]
    fn audio_element_variant_objects_rejects_unsupported_counts() {
        let zero_objects = AudioElementVariant::objects(0);
        let three_objects = AudioElementVariant::objects(3);
        let dual_objects = AudioElementVariant::objects(2);

        expect_eq!(zero_objects, Err(OarError::InvalidParameter));
        expect_eq!(three_objects, Err(OarError::InvalidParameter));
        expect_true!(dual_objects.is_ok());
    }

    #[gtest]
    fn audio_element_config_passthrough_initializes_single_channel_order_zero() {
        let config = AudioElementConfig::new(
            AudioElementVariant::ambisonics(HighOrderAmbisonics::Zoa),
            BinauralFilterProfile::Reverberant,
        );

        expect_true!(config.is_passthrough());
        expect_false!(config.is_ambisonics());
        expect_eq!(config.get_number_of_input_channels(), 1);
        expect_eq!(config.get_binaural_filters_ambisonic_order(), 0);
    }

    #[gtest]
    fn audio_element_config_initializes_subset_loudspeaker_layout_channels() {
        let config = AudioElementConfig::new(
            AudioElementVariant::loudspeaker_layout(InputLayout::Top4ch),
            BinauralFilterProfile::Direct,
        );

        expect_true!(config.uses_ambisonic_encoder());
        expect_eq!(config.get_number_of_input_channels(), 4);
        expect_eq!(config.get_loudspeaker_channels().len(), 4);
        expect_eq!(config.get_binaural_filters_ambisonic_order(), 3);
    }

    #[gtest]
    fn audio_element_config_binaural_layout_initializes_stereo_passthrough() {
        let config = AudioElementConfig::new(
            AudioElementVariant::loudspeaker_layout(InputLayout::Binaural),
            BinauralFilterProfile::Direct,
        );

        expect_eq!(*config.variant(), AudioElementVariant::PassthroughStereo);
        expect_true!(config.is_passthrough());
        expect_false!(config.uses_ambisonic_encoder());
        expect_eq!(config.get_number_of_input_channels(), 2);
        expect_eq!(config.get_binaural_filters_ambisonic_order(), 0);
    }

    #[gtest]
    fn set_first_channel_index_updates_index() {
        let mut config = AudioElementConfig::new(
            AudioElementVariant::ambisonics(HighOrderAmbisonics::Order1),
            BinauralFilterProfile::Direct,
        );

        config.set_first_channel_index(10);

        expect_eq!(config.get_first_channel_index(), 10);
    }

    #[gtest]
    fn set_head_locked_updates_flag() {
        let mut config = AudioElementConfig::new(
            AudioElementVariant::ambisonics(HighOrderAmbisonics::Order2),
            BinauralFilterProfile::Direct,
        );

        config.set_head_locked(true);

        expect_true!(config.is_head_locked());
    }
}
