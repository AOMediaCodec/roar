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

//! Loudspeaker layout definitions and configurations.
//!
//! Provides the definitions and mapping rules to translate standard multichannel loudspeaker
//! layouts (e.g. 5.1.0, 7.1.4, 9.1.6) into virtual speaker positions for OBR binaural rendering.

use super::audio_element_type::AudioElementType;
use super::input_channel_config::LoudspeakerLayoutInputChannel;
use crate::common::definitions::{Degrees, Distance};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)]
enum VirtualLoudspeaker {
    C,
    L30,
    R30,
    L45,
    R45,
    L60,
    R60,
    L90,
    R90,
    L110,
    R110,
    L135,
    R135,
    Deg180,
    Uc,
    Ul30,
    Ur30,
    Ul45,
    Ur45,
    Ul90,
    Ur90,
    Ul135,
    Ur135,
    U180,
    Bc,
    Bl45,
    Br45,
    Bl135,
    Br135,
    T,
    Lfe,
    Lfe1,
    Lfe2,
}

impl VirtualLoudspeaker {
    fn to_channel(self) -> LoudspeakerLayoutInputChannel {
        let d1 = Distance::new(1.0).unwrap();
        match self {
            Self::C => {
                LoudspeakerLayoutInputChannel::new("kC", Degrees(0.0), Degrees(0.0), d1, false)
            }
            Self::L30 => {
                LoudspeakerLayoutInputChannel::new("kL30", Degrees(30.0), Degrees(0.0), d1, false)
            }
            Self::R30 => {
                LoudspeakerLayoutInputChannel::new("kR30", Degrees(-30.0), Degrees(0.0), d1, false)
            }
            Self::L45 => {
                LoudspeakerLayoutInputChannel::new("kL45", Degrees(45.0), Degrees(0.0), d1, false)
            }
            Self::R45 => {
                LoudspeakerLayoutInputChannel::new("kR45", Degrees(-45.0), Degrees(0.0), d1, false)
            }
            Self::L60 => {
                LoudspeakerLayoutInputChannel::new("kL60", Degrees(60.0), Degrees(0.0), d1, false)
            }
            Self::R60 => {
                LoudspeakerLayoutInputChannel::new("kR60", Degrees(-60.0), Degrees(0.0), d1, false)
            }
            Self::L90 => {
                LoudspeakerLayoutInputChannel::new("kL90", Degrees(90.0), Degrees(0.0), d1, false)
            }
            Self::R90 => {
                LoudspeakerLayoutInputChannel::new("kR90", Degrees(-90.0), Degrees(0.0), d1, false)
            }
            Self::L110 => {
                LoudspeakerLayoutInputChannel::new("kL110", Degrees(110.0), Degrees(0.0), d1, false)
            }
            Self::R110 => LoudspeakerLayoutInputChannel::new(
                "kR110",
                Degrees(-110.0),
                Degrees(0.0),
                d1,
                false,
            ),
            Self::L135 => {
                LoudspeakerLayoutInputChannel::new("kL135", Degrees(135.0), Degrees(0.0), d1, false)
            }
            Self::R135 => LoudspeakerLayoutInputChannel::new(
                "kR135",
                Degrees(-135.0),
                Degrees(0.0),
                d1,
                false,
            ),
            Self::Deg180 => {
                LoudspeakerLayoutInputChannel::new("k180", Degrees(180.0), Degrees(0.0), d1, false)
            }
            Self::Uc => {
                LoudspeakerLayoutInputChannel::new("kUC", Degrees(0.0), Degrees(45.0), d1, false)
            }
            Self::Ul30 => {
                LoudspeakerLayoutInputChannel::new("kUL30", Degrees(30.0), Degrees(45.0), d1, false)
            }
            Self::Ur30 => LoudspeakerLayoutInputChannel::new(
                "kUR30",
                Degrees(-30.0),
                Degrees(45.0),
                d1,
                false,
            ),
            Self::Ul45 => {
                LoudspeakerLayoutInputChannel::new("kUL45", Degrees(45.0), Degrees(45.0), d1, false)
            }
            Self::Ur45 => LoudspeakerLayoutInputChannel::new(
                "kUR45",
                Degrees(-45.0),
                Degrees(45.0),
                d1,
                false,
            ),
            Self::Ul90 => {
                LoudspeakerLayoutInputChannel::new("kUL90", Degrees(90.0), Degrees(45.0), d1, false)
            }
            Self::Ur90 => LoudspeakerLayoutInputChannel::new(
                "kUR90",
                Degrees(-90.0),
                Degrees(45.0),
                d1,
                false,
            ),
            Self::Ul135 => LoudspeakerLayoutInputChannel::new(
                "kUL135",
                Degrees(135.0),
                Degrees(45.0),
                d1,
                false,
            ),
            Self::Ur135 => LoudspeakerLayoutInputChannel::new(
                "kUR135",
                Degrees(-135.0),
                Degrees(45.0),
                d1,
                false,
            ),
            Self::U180 => LoudspeakerLayoutInputChannel::new(
                "kU180",
                Degrees(180.0),
                Degrees(45.0),
                d1,
                false,
            ),
            Self::Bc => {
                LoudspeakerLayoutInputChannel::new("kBC", Degrees(0.0), Degrees(-30.0), d1, false)
            }
            Self::Bl45 => LoudspeakerLayoutInputChannel::new(
                "kBL45",
                Degrees(45.0),
                Degrees(-30.0),
                d1,
                false,
            ),
            Self::Br45 => LoudspeakerLayoutInputChannel::new(
                "kBR45",
                Degrees(-45.0),
                Degrees(-30.0),
                d1,
                false,
            ),
            Self::Bl135 => LoudspeakerLayoutInputChannel::new(
                "kBL135",
                Degrees(135.0),
                Degrees(-30.0),
                d1,
                false,
            ),
            Self::Br135 => LoudspeakerLayoutInputChannel::new(
                "kBR135",
                Degrees(-135.0),
                Degrees(-30.0),
                d1,
                false,
            ),
            Self::T => {
                LoudspeakerLayoutInputChannel::new("kT", Degrees(0.0), Degrees(90.0), d1, false)
            }
            Self::Lfe => {
                LoudspeakerLayoutInputChannel::new("kLFE", Degrees(0.0), Degrees(-30.0), d1, true)
            }
            Self::Lfe1 => {
                LoudspeakerLayoutInputChannel::new("kLFE1", Degrees(45.0), Degrees(-30.0), d1, true)
            }
            Self::Lfe2 => LoudspeakerLayoutInputChannel::new(
                "kLFE2",
                Degrees(-45.0),
                Degrees(-30.0),
                d1,
                true,
            ),
        }
    }
}

/// Virtual loudspeaker layout configurations registry.
///
/// Maps layout types defined by `AudioElementType` into virtual speaker configurations,
/// complete with standard azimuth, elevation, and distance coordinates.
#[derive(Debug, Default, Clone, Copy)]
pub struct LoudspeakerLayouts;

impl LoudspeakerLayouts {
    /// Constructs a new `LoudspeakerLayouts` registry.
    pub fn new() -> Self {
        Self
    }

    /// Returns the vector of `LoudspeakerLayoutInputChannel` corresponding to `layout_type`.
    pub fn get_loudspeaker_layout(
        &self,
        layout_type: AudioElementType,
    ) -> Vec<LoudspeakerLayoutInputChannel> {
        use VirtualLoudspeaker::*;
        let spks: &[VirtualLoudspeaker] = match layout_type {
            AudioElementType::LayoutMono => &[C],
            AudioElementType::LayoutStereo => &[L30, R30],
            AudioElementType::Layout5_1_0 => &[L30, R30, C, Lfe, L110, R110],
            AudioElementType::Layout5_1_2 => &[L30, R30, C, Lfe, L110, R110, Ul30, Ur30],
            AudioElementType::Layout5_1_4 => {
                &[L30, R30, C, Lfe, L110, R110, Ul45, Ur45, Ul135, Ur135]
            }
            AudioElementType::Layout7_1_0 => &[L30, R30, C, Lfe, L90, R90, L135, R135],
            AudioElementType::Layout7_1_2 => &[L30, R30, C, Lfe, L90, R90, L135, R135, Ul45, Ur45],
            AudioElementType::Layout7_1_4 => {
                &[L30, R30, C, Lfe, L90, R90, L135, R135, Ul45, Ur45, Ul135, Ur135]
            }
            AudioElementType::Layout3_1_2 => &[L30, R30, C, Lfe, Ul45, Ur45],
            AudioElementType::Layout9_1_6 => &[
                L60, R60, C, Lfe1, L135, R135, L30, R30, L90, R90, Ul45, Ur45, Ul135, Ur135, Ul90,
                Ur90,
            ],
            AudioElementType::Layout7_1_5_4 => &[
                L30, R30, C, Lfe, L90, R90, L135, R135, Ul45, Ur45, T, Ul135, Ur135, Bl45, Br45,
                Bl135, Br135,
            ],
            AudioElementType::Layout10_2_9_3 => &[
                L60, R60, C, Lfe1, L135, R135, L30, R30, Deg180, Lfe2, L90, R90, Ul45, Ur45, Uc, T,
                Ul135, Ur135, Ul90, Ur90, U180, Bc, Bl45, Br45,
            ],
            _ => &[],
        };

        spks.iter().map(|&v| v.to_channel()).collect()
    }
}

// ===== Tests =====

#[cfg(test)]
mod test {

    use super::*;
    use googletest::prelude::*;

    #[gtest]
    fn test_virtual_loudspeaker_to_channel() {
        let ch_c = VirtualLoudspeaker::C.to_channel();
        expect_eq!(ch_c.base.id, "kC");
        expect_eq!(ch_c.azimuth.0, 0.0);
        expect_eq!(ch_c.elevation.0, 0.0);
        expect_eq!(ch_c.distance.value(), 1.0);
        expect_false!(ch_c.is_lfe);

        let ch_lfe = VirtualLoudspeaker::Lfe.to_channel();
        expect_eq!(ch_lfe.base.id, "kLFE");
        expect_eq!(ch_lfe.azimuth.0, 0.0);
        expect_eq!(ch_lfe.elevation.0, -30.0);
        expect_true!(ch_lfe.is_lfe);
    }

    #[gtest]
    fn test_get_loudspeaker_layout() {
        let layouts = LoudspeakerLayouts::new();

        let mono = layouts.get_loudspeaker_layout(AudioElementType::LayoutMono);
        expect_eq!(mono.len(), 1);
        expect_eq!(mono[0].base.id, "kC");

        let stereo = layouts.get_loudspeaker_layout(AudioElementType::LayoutStereo);
        expect_eq!(stereo.len(), 2);
        expect_eq!(stereo[0].base.id, "kL30");
        expect_eq!(stereo[1].base.id, "kR30");

        let l51 = layouts.get_loudspeaker_layout(AudioElementType::Layout5_1_0);
        expect_eq!(l51.len(), 6);

        let invalid = layouts.get_loudspeaker_layout(AudioElementType::InvalidType);
        expect_true!(invalid.is_empty());
    }
}
