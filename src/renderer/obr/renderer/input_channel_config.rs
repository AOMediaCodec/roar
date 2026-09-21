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

//! Input channel configuration abstractions.
//!
//! Provides the base structures and variations (Ambisonic, loudspeaker, passthrough,
//! and object channels) representing inputs fed into the OBR renderer.

use crate::common::definitions::{Degrees, Distance, LinearGain};

/// Base properties shared by all input channel configurations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputChannelConfigBase {
    pub id: String,
    pub input_channel_index: usize,
}

impl InputChannelConfigBase {
    /// Constructs a new `InputChannelConfigBase` with a unique ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into(), input_channel_index: 0 }
    }
}

/// Configuration for an Ambisonic scene input channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmbisonicSceneInputChannel {
    pub base: InputChannelConfigBase,
}

impl AmbisonicSceneInputChannel {
    /// Constructs a new `AmbisonicSceneInputChannel` with a unique ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self { base: InputChannelConfigBase::new(id) }
    }
}

/// Configuration for a loudspeaker layout input channel.
#[derive(Debug, Clone, PartialEq)]
pub struct LoudspeakerLayoutInputChannel {
    pub base: InputChannelConfigBase,
    /// Azimuth angle of the virtual speaker.
    pub azimuth: Degrees,
    /// Elevation angle of the virtual speaker.
    pub elevation: Degrees,
    /// Distance of the virtual speaker from the listener.
    pub distance: Distance,
    /// Indicates whether this channel represents a Low Frequency Effects (LFE) channel.
    pub is_lfe: bool,
}

impl LoudspeakerLayoutInputChannel {
    /// Constructs a new `LoudspeakerLayoutInputChannel`.
    pub fn new(
        id: impl Into<String>,
        azimuth: Degrees,
        elevation: Degrees,
        distance: Distance,
        is_lfe: bool,
    ) -> Self {
        Self { base: InputChannelConfigBase::new(id), azimuth, elevation, distance, is_lfe }
    }
}

/// Configuration for a passthrough input channel (e.g. stereo L/R).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassthroughInputChannel {
    pub base: InputChannelConfigBase,
}

impl PassthroughInputChannel {
    /// Constructs a new `PassthroughInputChannel` with a unique ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self { base: InputChannelConfigBase::new(id) }
    }
}

/// Configuration for a spatialized audio object input channel.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioObjectInputChannel {
    pub base: InputChannelConfigBase,
    /// Linear volume gain applied to this object channel.
    pub gain: LinearGain,
    /// Spatial azimuth angle.
    pub azimuth: Degrees,
    /// Spatial elevation angle.
    pub elevation: Degrees,
    /// Distance of the object from the listener.
    pub distance: Distance,
}

impl AudioObjectInputChannel {
    /// Constructs a new `AudioObjectInputChannel` with default unit gain.
    pub fn new(
        id: impl Into<String>,
        azimuth: Degrees,
        elevation: Degrees,
        distance: Distance,
    ) -> Self {
        Self {
            base: InputChannelConfigBase::new(id),
            gain: LinearGain(1.0),
            azimuth,
            elevation,
            distance,
        }
    }
}

// ===== Tests =====

#[cfg(test)]
mod test {

    use super::*;
    use googletest::prelude::*;

    #[gtest]
    fn test_input_channel_config_base() {
        let base = InputChannelConfigBase::new("test_base");
        expect_eq!(base.id, "test_base");
        expect_eq!(base.input_channel_index, 0);
    }

    #[gtest]
    fn test_ambisonic_scene_input_channel() {
        let amb = AmbisonicSceneInputChannel::new("amb_ch");
        expect_eq!(amb.base.id, "amb_ch");
    }

    #[gtest]
    fn test_loudspeaker_layout_input_channel() {
        let az = Degrees(30.0);
        let el = Degrees(0.0);
        let dist = Distance::new(1.0).unwrap();
        let ch = LoudspeakerLayoutInputChannel::new("L30", az, el, dist, false);
        expect_eq!(ch.base.id, "L30");
        expect_eq!(ch.azimuth.0, 30.0);
        expect_eq!(ch.elevation.0, 0.0);
        expect_eq!(ch.distance.value(), 1.0);
        expect_false!(ch.is_lfe);
    }

    #[gtest]
    fn test_passthrough_input_channel() {
        let pass = PassthroughInputChannel::new("pass_ch");
        expect_eq!(pass.base.id, "pass_ch");
    }

    #[gtest]
    fn test_audio_object_input_channel() {
        let az = Degrees(-45.0);
        let el = Degrees(10.0);
        let dist = Distance::new(2.5).unwrap();
        let obj = AudioObjectInputChannel::new("obj_ch", az, el, dist);
        expect_eq!(obj.base.id, "obj_ch");
        expect_eq!(obj.gain.0, 1.0); // Default unit gain
        expect_eq!(obj.azimuth.0, -45.0);
        expect_eq!(obj.elevation.0, 10.0);
        expect_eq!(obj.distance.value(), 2.5);
    }
}
