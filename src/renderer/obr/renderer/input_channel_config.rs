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

//! Input channel configuration abstractions for OBR.
//!
//! Provides the channel configuration structure for spatialized audio object channels
//! fed into the OBR Ambisonic encoder.

use crate::common::definitions::{Degrees, Distance, LinearGain};

/// Configuration for a spatialized audio object input channel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioObjectInputChannel {
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
    pub fn new(azimuth: Degrees, elevation: Degrees, distance: Distance) -> Self {
        Self { gain: LinearGain::ONE, azimuth, elevation, distance }
    }
}

// ===== Tests =====

#[cfg(test)]
mod test {

    use super::*;
    use googletest::prelude::*;

    #[gtest]
    fn audio_object_input_channel_initializes_with_unit_gain_and_coordinates() {
        let az = Degrees::new(-45.0).unwrap();
        let el = Degrees::new(10.0).unwrap();
        let dist = Distance::new(2.5).unwrap();

        let obj = AudioObjectInputChannel::new(az, el, dist);

        expect_eq!(obj.gain.value(), 1.0);
        expect_eq!(obj.azimuth.value(), -45.0);
        expect_eq!(obj.elevation.value(), 10.0);
        expect_eq!(obj.distance.value(), 2.5);
    }
}
