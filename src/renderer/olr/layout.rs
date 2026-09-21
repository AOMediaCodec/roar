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

//! Loudspeaker layout definitions for OLR.
//!
//! This module defines standard loudspeaker configurations (such as Stereo, 5.1, 7.1.4,
//! and other multi-channel layouts) matching the C reference implementation, including
//! spatial coordinates and LFE status for each speaker position.

use crate::common::definitions::Layout;

// TODO(b/525080422): Combine this Layout information with the common and OBR layout information and
// use one unified representation (where it makes sense).

/// The 2D or 3D position of a loudspeaker in a layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpeakerPosition {
    /// The standard name/label of the speaker.
    pub name: &'static str,
    /// The azimuth angle in degrees.
    pub azimuth: f32,
    /// The elevation angle in degrees.
    pub elevation: f32,
    /// Whether this is a Low-Frequency Effects (LFE) channel speaker.
    pub is_lfe: bool,
}

/// A collection of speaker positions representing a physical loudspeaker layout.
#[derive(Clone, Copy)]
pub struct SpeakerLayout {
    /// The descriptive name of the layout.
    pub name: &'static str,
    /// The corresponding Layout identifier.
    pub layout: Layout,
    /// The array of speakers included in this layout.
    pub speakers: &'static [SpeakerPosition],
}

impl std::fmt::Debug for SpeakerLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpeakerLayout")
            .field("name", &self.name)
            .field("layout", &self.layout)
            .field("speakers", &self.speakers)
            .finish()
    }
}

// Speakers definition matching C layout.c
const SP_MP000: SpeakerPosition =
    SpeakerPosition { name: "M+000", azimuth: 0.0, elevation: 0.0, is_lfe: false };
const SP_MP025: SpeakerPosition =
    SpeakerPosition { name: "M+025", azimuth: 25.0, elevation: 0.0, is_lfe: false };
const SP_MM025: SpeakerPosition =
    SpeakerPosition { name: "M-025", azimuth: -25.0, elevation: 0.0, is_lfe: false };
const SP_MM030: SpeakerPosition =
    SpeakerPosition { name: "M-030", azimuth: -30.0, elevation: 0.0, is_lfe: false };
const SP_MP030: SpeakerPosition =
    SpeakerPosition { name: "M+030", azimuth: 30.0, elevation: 0.0, is_lfe: false };
const SP_MM060: SpeakerPosition =
    SpeakerPosition { name: "M-060", azimuth: -60.0, elevation: 0.0, is_lfe: false };
const SP_MP060: SpeakerPosition =
    SpeakerPosition { name: "M+060", azimuth: 60.0, elevation: 0.0, is_lfe: false };
const SP_MM090: SpeakerPosition =
    SpeakerPosition { name: "M-090", azimuth: -90.0, elevation: 0.0, is_lfe: false };
const SP_MP090: SpeakerPosition =
    SpeakerPosition { name: "M+090", azimuth: 90.0, elevation: 0.0, is_lfe: false };
const SP_MM110: SpeakerPosition =
    SpeakerPosition { name: "M-110", azimuth: -110.0, elevation: 0.0, is_lfe: false };
const SP_MP110: SpeakerPosition =
    SpeakerPosition { name: "M+110", azimuth: 110.0, elevation: 0.0, is_lfe: false };
const SP_MM135: SpeakerPosition =
    SpeakerPosition { name: "M-135", azimuth: -135.0, elevation: 0.0, is_lfe: false };
const SP_MP135: SpeakerPosition =
    SpeakerPosition { name: "M+135", azimuth: 135.0, elevation: 0.0, is_lfe: false };
const SP_MP180: SpeakerPosition =
    SpeakerPosition { name: "M+180", azimuth: 180.0, elevation: 0.0, is_lfe: false };

const SP_UP000: SpeakerPosition =
    SpeakerPosition { name: "U+000", azimuth: 0.0, elevation: 30.0, is_lfe: false };
const SP_UM030: SpeakerPosition =
    SpeakerPosition { name: "U-030", azimuth: -30.0, elevation: 30.0, is_lfe: false };
const SP_UP030: SpeakerPosition =
    SpeakerPosition { name: "U+030", azimuth: 30.0, elevation: 30.0, is_lfe: false };
const SP_UM045: SpeakerPosition =
    SpeakerPosition { name: "U-045", azimuth: -45.0, elevation: 30.0, is_lfe: false };
const SP_UP045: SpeakerPosition =
    SpeakerPosition { name: "U+045", azimuth: 45.0, elevation: 30.0, is_lfe: false };
const SP_UM090: SpeakerPosition =
    SpeakerPosition { name: "U-090", azimuth: -90.0, elevation: 30.0, is_lfe: false };
const SP_UP090: SpeakerPosition =
    SpeakerPosition { name: "U+090", azimuth: 90.0, elevation: 30.0, is_lfe: false };
const SP_UM110: SpeakerPosition =
    SpeakerPosition { name: "U-110", azimuth: -110.0, elevation: 30.0, is_lfe: false };
const SP_UP110: SpeakerPosition =
    SpeakerPosition { name: "U+110", azimuth: 110.0, elevation: 30.0, is_lfe: false };
const SP_UM135: SpeakerPosition =
    SpeakerPosition { name: "U-135", azimuth: -135.0, elevation: 30.0, is_lfe: false };
const SP_UP135: SpeakerPosition =
    SpeakerPosition { name: "U+135", azimuth: 135.0, elevation: 30.0, is_lfe: false };
const SP_UP180: SpeakerPosition =
    SpeakerPosition { name: "U+180", azimuth: 180.0, elevation: 30.0, is_lfe: false };

const SP_UHP180: SpeakerPosition =
    SpeakerPosition { name: "U+180", azimuth: 180.0, elevation: 45.0, is_lfe: false };

const SP_BP000: SpeakerPosition =
    SpeakerPosition { name: "B+000", azimuth: 0.0, elevation: -30.0, is_lfe: false };
const SP_BM045: SpeakerPosition =
    SpeakerPosition { name: "B-045", azimuth: -45.0, elevation: -30.0, is_lfe: false };
const SP_BP045: SpeakerPosition =
    SpeakerPosition { name: "B+045", azimuth: 45.0, elevation: -30.0, is_lfe: false };
const SP_BM135: SpeakerPosition =
    SpeakerPosition { name: "B-135", azimuth: -135.0, elevation: -30.0, is_lfe: false };
const SP_BP135: SpeakerPosition =
    SpeakerPosition { name: "B+135", azimuth: 135.0, elevation: -30.0, is_lfe: false };

const SP_TP000: SpeakerPosition =
    SpeakerPosition { name: "T+000", azimuth: 0.0, elevation: 90.0, is_lfe: false };
const SP_LFE1: SpeakerPosition =
    SpeakerPosition { name: "LFE1", azimuth: 45.0, elevation: -30.0, is_lfe: true };
const SP_LFE2: SpeakerPosition =
    SpeakerPosition { name: "LFE2", azimuth: -45.0, elevation: -30.0, is_lfe: true };

// Speaker layouts with LFE
static SPEAKER_LAYOUTS: &[SpeakerLayout] = &[
    SpeakerLayout { name: "layout_1.0.0(mono)", layout: Layout::Mono, speakers: &[SP_MP000] },
    SpeakerLayout {
        name: "layout_2.0.0(stereo)",
        layout: Layout::Stereo,
        speakers: &[SP_MP030, SP_MM030],
    },
    SpeakerLayout {
        name: "layout_5.1.0",
        layout: Layout::Layout51,
        speakers: &[SP_MP030, SP_MM030, SP_MP000, SP_LFE1, SP_MP110, SP_MM110],
    },
    SpeakerLayout {
        name: "layout_5.1.2",
        layout: Layout::Layout512,
        speakers: &[SP_MP030, SP_MM030, SP_MP000, SP_LFE1, SP_MP110, SP_MM110, SP_UP030, SP_UM030],
    },
    SpeakerLayout {
        name: "layout_5.1.4",
        layout: Layout::Layout514,
        speakers: &[
            SP_MP030, SP_MM030, SP_MP000, SP_LFE1, SP_MP110, SP_MM110, SP_UP030, SP_UM030,
            SP_UP110, SP_UM110,
        ],
    },
    SpeakerLayout {
        name: "layout_7.1.0",
        layout: Layout::Layout71,
        speakers: &[SP_MP030, SP_MM030, SP_MP000, SP_LFE1, SP_MP090, SP_MM090, SP_MP135, SP_MM135],
    },
    SpeakerLayout {
        name: "layout_7.1.2",
        layout: Layout::Layout712,
        speakers: &[
            SP_MP030, SP_MM030, SP_MP000, SP_LFE1, SP_MP090, SP_MM090, SP_MP135, SP_MM135,
            SP_UP045, SP_UM045,
        ],
    },
    SpeakerLayout {
        name: "layout_7.1.4",
        layout: Layout::Layout714,
        speakers: &[
            SP_MP030, SP_MM030, SP_MP000, SP_LFE1, SP_MP090, SP_MM090, SP_MP135, SP_MM135,
            SP_UP045, SP_UM045, SP_UP135, SP_UM135,
        ],
    },
    SpeakerLayout {
        name: "layout_3.1.2",
        layout: Layout::Layout312,
        speakers: &[SP_MP030, SP_MM030, SP_MP000, SP_LFE1, SP_UP045, SP_UM045],
    },
    SpeakerLayout {
        name: "layout_9.1.6",
        layout: Layout::Layout916,
        speakers: &[
            SP_MP060, SP_MM060, SP_MP000, SP_LFE1, SP_MP135, SP_MM135, SP_MP030, SP_MM030,
            SP_MP090, SP_MM090, SP_UP045, SP_UM045, SP_UP135, SP_UM135, SP_UP090, SP_UM090,
        ],
    },
    SpeakerLayout {
        name: "layout_7.1.5.4",
        layout: Layout::Layout7154,
        speakers: &[
            SP_MP030, SP_MM030, SP_MP000, SP_LFE1, SP_MP090, SP_MM090, SP_MP135, SP_MM135,
            SP_UP045, SP_UM045, SP_TP000, SP_UP135, SP_UM135, SP_BP045, SP_BM045, SP_BP135,
            SP_BM135,
        ],
    },
    SpeakerLayout {
        name: "layout_10.2.9.3",
        layout: Layout::LayoutA293,
        speakers: &[
            SP_MP060, SP_MM060, SP_MP000, SP_LFE1, SP_MP135, SP_MM135, SP_MP030, SP_MM030,
            SP_MP180, SP_LFE2, SP_MP090, SP_MM090, SP_UP045, SP_UM045, SP_UP000, SP_TP000,
            SP_UP135, SP_UM135, SP_UP090, SP_UM090, SP_UP180, SP_BP000, SP_BP045, SP_BM045,
        ],
    },
    SpeakerLayout {
        name: "layout_sound_system_e",
        layout: Layout::SoundSystemE451,
        speakers: &[
            SP_MP030, SP_MM030, SP_MP000, SP_LFE1, SP_MP110, SP_MM110, SP_UP030, SP_UM030,
            SP_UP110, SP_UM110, SP_BP000,
        ],
    },
    SpeakerLayout {
        name: "layout_sound_system_f",
        layout: Layout::SoundSystemF370,
        speakers: &[
            SP_MP000, SP_MP030, SP_MM030, SP_UP045, SP_UM045, SP_MP090, SP_MM090, SP_MP135,
            SP_MM135, SP_UHP180, SP_LFE1, SP_LFE2,
        ],
    },
    SpeakerLayout {
        name: "layout_sound_system_g",
        layout: Layout::SoundSystemG490,
        speakers: &[
            SP_MP030, SP_MM030, SP_MP000, SP_LFE1, SP_MP090, SP_MM090, SP_MP135, SP_MM135,
            SP_UP045, SP_UM045, SP_UP135, SP_UM135, SP_MP025, SP_MM025,
        ],
    },
];

// Speaker layouts without LFE
static SPEAKER_LAYOUTS_WITHOUT_LFE: &[SpeakerLayout] = &[
    SpeakerLayout { name: "layout_1.0.0(mono)", layout: Layout::Mono, speakers: &[SP_MP000] },
    SpeakerLayout {
        name: "layout_2.0.0(stereo)",
        layout: Layout::Stereo,
        speakers: &[SP_MP030, SP_MM030],
    },
    SpeakerLayout {
        name: "layout_5.0.0",
        layout: Layout::Layout51,
        speakers: &[SP_MP030, SP_MM030, SP_MP000, SP_MP110, SP_MM110],
    },
    SpeakerLayout {
        name: "layout_5.0.2",
        layout: Layout::Layout512,
        speakers: &[SP_MP030, SP_MM030, SP_MP000, SP_MP110, SP_MM110, SP_UP030, SP_UM030],
    },
    SpeakerLayout {
        name: "layout_5.0.4",
        layout: Layout::Layout514,
        speakers: &[
            SP_MP030, SP_MM030, SP_MP000, SP_MP110, SP_MM110, SP_UP030, SP_UM030, SP_UP110,
            SP_UM110,
        ],
    },
    SpeakerLayout {
        name: "layout_7.0.0",
        layout: Layout::Layout71,
        speakers: &[SP_MP030, SP_MM030, SP_MP000, SP_MP090, SP_MM090, SP_MP135, SP_MM135],
    },
    SpeakerLayout {
        name: "layout_7.0.2",
        layout: Layout::Layout712,
        speakers: &[
            SP_MP030, SP_MM030, SP_MP000, SP_MP090, SP_MM090, SP_MP135, SP_MM135, SP_UP045,
            SP_UM045,
        ],
    },
    SpeakerLayout {
        name: "layout_7.0.4",
        layout: Layout::Layout714,
        speakers: &[
            SP_MP030, SP_MM030, SP_MP000, SP_MP090, SP_MM090, SP_MP135, SP_MM135, SP_UP045,
            SP_UM045, SP_UP135, SP_UM135,
        ],
    },
    SpeakerLayout {
        name: "layout_3.0.2",
        layout: Layout::Layout312,
        speakers: &[SP_MP030, SP_MM030, SP_MP000, SP_UP045, SP_UM045],
    },
    SpeakerLayout {
        name: "layout_9.0.6",
        layout: Layout::Layout916,
        speakers: &[
            SP_MP060, SP_MM060, SP_MP000, SP_MP135, SP_MM135, SP_MP030, SP_MM030, SP_MP090,
            SP_MM090, SP_UP045, SP_UM045, SP_UP135, SP_UM135, SP_UP090, SP_UM090,
        ],
    },
    SpeakerLayout {
        name: "layout_7.0.5.4",
        layout: Layout::Layout7154,
        speakers: &[
            SP_MP030, SP_MM030, SP_MP000, SP_MP090, SP_MM090, SP_MP135, SP_MM135, SP_UP045,
            SP_UM045, SP_TP000, SP_UP135, SP_UM135, SP_BP045, SP_BM045, SP_BP135, SP_BM135,
        ],
    },
    SpeakerLayout {
        name: "layout_10.0.9.3",
        layout: Layout::LayoutA293,
        speakers: &[
            SP_MP060, SP_MM060, SP_MP000, SP_MP135, SP_MM135, SP_MP030, SP_MM030, SP_MP180,
            SP_MP090, SP_MM090, SP_UP045, SP_UM045, SP_UP000, SP_TP000, SP_UP135, SP_UM135,
            SP_UP090, SP_UM090, SP_UP180, SP_BP000, SP_BP045, SP_BM045,
        ],
    },
    SpeakerLayout {
        name: "layout_sound_system_e",
        layout: Layout::SoundSystemE451,
        speakers: &[
            SP_MP030, SP_MM030, SP_MP000, SP_MP110, SP_MM110, SP_UP030, SP_UM030, SP_UP110,
            SP_UM110, SP_BP000,
        ],
    },
    SpeakerLayout {
        name: "layout_sound_system_f",
        layout: Layout::SoundSystemF370,
        speakers: &[
            SP_MP000, SP_MP030, SP_MM030, SP_UP045, SP_UM045, SP_MP090, SP_MM090, SP_MP135,
            SP_MM135, SP_UHP180,
        ],
    },
    SpeakerLayout {
        name: "layout_sound_system_g",
        layout: Layout::SoundSystemG490,
        speakers: &[
            SP_MP030, SP_MM030, SP_MP000, SP_MP090, SP_MM090, SP_MP135, SP_MM135, SP_UP045,
            SP_UM045, SP_UP135, SP_UM135, SP_MP025, SP_MM025,
        ],
    },
];

/// Returns the loudspeaker layout corresponding to the given layout enum value.
///
/// Returns `None` if the layout is not defined or supported.
pub fn get_layout(layout: Layout) -> Option<&'static SpeakerLayout> {
    SPEAKER_LAYOUTS.iter().find(|l| l.layout == layout)
}

/// Returns the loudspeaker layout corresponding to the given layout enum value,
/// excluding any Low-Frequency Effects (LFE) channels.
///
/// Returns `None` if the layout is not defined or supported.
pub fn get_layout_without_lfe(layout: Layout) -> Option<&'static SpeakerLayout> {
    SPEAKER_LAYOUTS_WITHOUT_LFE.iter().find(|l| l.layout == layout)
}

// ===== Tests =====

#[cfg(test)]
mod test {

    use super::*;
    use googletest::prelude::*;

    #[gtest]
    fn test_get_layout_returns_correct_stereo_config() {
        let stereo = get_layout(Layout::Stereo).unwrap();

        expect_eq!(stereo.speakers.len(), 2);
        expect_false!(stereo.speakers[0].is_lfe);
        expect_false!(stereo.speakers[1].is_lfe);
    }

    #[gtest]
    fn test_get_layout_returns_correct_51_config() {
        let layout51 = get_layout(Layout::Layout51).unwrap();

        expect_eq!(layout51.speakers.len(), 6);
        let lfe_count = layout51.speakers.iter().filter(|s| s.is_lfe).count();
        expect_eq!(lfe_count, 1);
    }

    #[gtest]
    fn test_get_layout_returns_none_for_unsupported_binaural() {
        expect_true!(get_layout(Layout::Binaural).is_none());
    }

    #[gtest]
    fn test_get_layout_without_lfe_returns_correct_51_config() {
        let layout50 = get_layout_without_lfe(Layout::Layout51).unwrap();

        expect_eq!(layout50.speakers.len(), 5);
        let lfe_count = layout50.speakers.iter().filter(|s| s.is_lfe).count();
        expect_eq!(lfe_count, 0);
    }

    #[gtest]
    fn test_get_layout_without_lfe_returns_correct_stereo_config() {
        let stereo = get_layout_without_lfe(Layout::Stereo).unwrap();

        expect_eq!(stereo.speakers.len(), 2);
    }
}
