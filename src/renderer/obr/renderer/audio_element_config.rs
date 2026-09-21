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
//! Defines the structure for configuring properties, layout type, and filter profile
//! for each individual audio element (input block) processed by the OBR.

use super::audio_element_type::AudioElementType;
use super::input_channel_config::{
    AmbisonicSceneInputChannel, AudioObjectInputChannel, LoudspeakerLayoutInputChannel,
    PassthroughInputChannel,
};
use super::loudspeaker_layouts::LoudspeakerLayouts;
use crate::common::definitions::{Degrees, Distance};
use crate::renderer::obr::common::ambisonic_utils::get_num_periphonic_components;

/// Rendering profile selector for binaural filters.
///
/// Determines which set of spherical harmonic Head-Related Impulse Response (HRIR)
/// filters is loaded for decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum BinauralFilterProfile {
    Direct = 0,
    #[default]
    Ambient = 1,
    Reverberant = 2,
}

/// Configuration settings for an individual audio element.
///
/// Manages the mapping of input channels to different spatial representations
/// (e.g. Ambisonic scene channels, virtual speaker channels, or dynamic objects)
/// and specifies target filter profiles.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioElementConfig {
    element_type: AudioElementType,
    first_channel_index: usize,
    number_of_input_channels: usize,
    ambisonic_channels: Vec<AmbisonicSceneInputChannel>,
    loudspeaker_channels: Vec<LoudspeakerLayoutInputChannel>,
    object_channels: Vec<AudioObjectInputChannel>,
    passthrough_channels: Vec<PassthroughInputChannel>,
    binaural_filters_ambisonic_order: i32,
    binaural_filter_profile: BinauralFilterProfile,
    head_locked: bool,
}

impl AudioElementConfig {
    /// Creates a new `AudioElementConfig` instance.
    pub fn new(element_type: AudioElementType, filter_profile: BinauralFilterProfile) -> Self {
        let mut cfg = Self {
            element_type,
            first_channel_index: 0,
            number_of_input_channels: 0,
            ambisonic_channels: Vec::new(),
            loudspeaker_channels: Vec::new(),
            object_channels: Vec::new(),
            passthrough_channels: Vec::new(),
            binaural_filters_ambisonic_order: 3,
            binaural_filter_profile: filter_profile,
            head_locked: false,
        };
        cfg.init_channels();
        cfg
    }

    fn init_channels(&mut self) {
        if self.element_type.is_ambisonics() {
            let order = self.element_type.get_ambisonic_order().unwrap();
            let count = get_num_periphonic_components(order);
            for i in 0..count {
                self.ambisonic_channels.push(AmbisonicSceneInputChannel::new(format!("acn_{}", i)));
            }
            self.number_of_input_channels = count;
            self.binaural_filters_ambisonic_order = order;
        } else if self.element_type.is_loudspeaker_layout() {
            let layouts = LoudspeakerLayouts::new();
            self.loudspeaker_channels = layouts.get_loudspeaker_layout(self.element_type);
            self.number_of_input_channels = self.loudspeaker_channels.len();
            self.binaural_filters_ambisonic_order = 3;
        } else if self.element_type.is_object() {
            let count = if self.element_type == AudioElementType::ObjectMono { 1 } else { 2 };
            for i in 0..count {
                self.object_channels.push(AudioObjectInputChannel::new(
                    format!("obj_{}", i),
                    Degrees(0.0),
                    Degrees(0.0),
                    Distance::new(1.0).unwrap(),
                ));
            }
            self.number_of_input_channels = count;
            self.binaural_filters_ambisonic_order = 3;
        } else if self.element_type.is_passthrough() {
            let count = if self.element_type == AudioElementType::PassthroughMono { 1 } else { 2 };
            for i in 0..count {
                self.passthrough_channels.push(PassthroughInputChannel::new(format!("pt_{}", i)));
            }
            self.number_of_input_channels = count;
            self.binaural_filters_ambisonic_order = 0;
        }
        self.set_first_channel_index(0);
    }

    /// Returns the element type associated with this configuration.
    pub fn element_type(&self) -> AudioElementType {
        self.element_type
    }

    /// Sets the base `first_channel_index` for the element and updates children.
    pub fn set_first_channel_index(&mut self, first_channel: usize) {
        self.first_channel_index = first_channel;
        for (i, ch) in self.ambisonic_channels.iter_mut().enumerate() {
            ch.base.input_channel_index = first_channel + i;
        }
        for (i, ch) in self.loudspeaker_channels.iter_mut().enumerate() {
            ch.base.input_channel_index = first_channel + i;
        }
        for (i, ch) in self.object_channels.iter_mut().enumerate() {
            ch.base.input_channel_index = first_channel + i;
        }
        for (i, ch) in self.passthrough_channels.iter_mut().enumerate() {
            ch.base.input_channel_index = first_channel + i;
        }
    }

    /// Returns the offset index of the first input channel for this element.
    pub fn get_first_channel_index(&self) -> usize {
        self.first_channel_index
    }

    /// Returns the total number of input channels for this element.
    pub fn get_number_of_input_channels(&self) -> usize {
        self.number_of_input_channels
    }

    /// Returns a mutable slice of loudspeaker layout input channels.
    pub fn get_loudspeaker_channels(&mut self) -> &mut [LoudspeakerLayoutInputChannel] {
        &mut self.loudspeaker_channels
    }

    /// Returns a mutable slice of audio object input channels.
    pub fn get_object_channels(&mut self) -> &mut [AudioObjectInputChannel] {
        &mut self.object_channels
    }

    /// Returns the Ambisonic order used for rendering this element's filters.
    pub fn get_binaural_filters_ambisonic_order(&self) -> i32 {
        self.binaural_filters_ambisonic_order
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
    fn test_audio_element_config_ambisonic_init() {
        let config = AudioElementConfig::new(AudioElementType::Oa1, BinauralFilterProfile::Direct);

        expect_eq!(config.element_type(), AudioElementType::Oa1);
        expect_eq!(config.get_number_of_input_channels(), 4); // (1+1)^2 = 4
        expect_eq!(config.get_binaural_filters_ambisonic_order(), 1);
        expect_eq!(config.get_binaural_filter_profile(), BinauralFilterProfile::Direct);
        expect_false!(config.is_head_locked());
    }

    #[gtest]
    fn test_audio_element_config_object_init() {
        let config =
            AudioElementConfig::new(AudioElementType::ObjectMono, BinauralFilterProfile::Ambient);

        expect_eq!(config.element_type(), AudioElementType::ObjectMono);
        expect_eq!(config.get_number_of_input_channels(), 1);
        expect_eq!(config.get_binaural_filters_ambisonic_order(), 3);
        expect_eq!(config.get_binaural_filter_profile(), BinauralFilterProfile::Ambient);
    }

    #[gtest]
    fn test_audio_element_config_passthrough_init() {
        let config = AudioElementConfig::new(
            AudioElementType::PassthroughStereo,
            BinauralFilterProfile::Reverberant,
        );

        expect_eq!(config.element_type(), AudioElementType::PassthroughStereo);
        expect_eq!(config.get_number_of_input_channels(), 2);
        expect_eq!(config.get_binaural_filters_ambisonic_order(), 0);
    }

    #[gtest]
    fn test_audio_element_config_set_first_channel_index() {
        let mut config =
            AudioElementConfig::new(AudioElementType::Oa1, BinauralFilterProfile::Direct);
        config.set_first_channel_index(10);

        expect_eq!(config.get_first_channel_index(), 10);
    }

    #[gtest]
    fn test_audio_element_config_head_locked() {
        let mut config =
            AudioElementConfig::new(AudioElementType::Oa2, BinauralFilterProfile::Direct);
        config.set_head_locked(true);

        expect_true!(config.is_head_locked());
    }
}
