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

//! Audio element types representation.
//!
//! Defines the enum and methods for categorization of audio inputs supported by the OBR subsystem.

/// Classification of audio elements configured in the OBR.
///
/// Distinguishes between Ambisonic scenes (orders 1-4), standard loudspeaker layouts,
/// spatialized mono/dual objects, and passthrough stereo signals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum AudioElementType {
    #[default]
    InvalidType = 0,
    // Ambisonics
    Oa1 = 101,
    Oa2 = 102,
    Oa3 = 103,
    Oa4 = 104,
    // Loudspeaker layouts
    LayoutMono = 201,
    LayoutStereo = 202,
    Layout5_1_0 = 203,
    Layout5_1_2 = 204,
    Layout5_1_4 = 205,
    Layout7_1_0 = 206,
    Layout7_1_2 = 207,
    Layout7_1_4 = 208,
    Layout3_1_2 = 209,
    Layout9_1_6 = 210,
    Layout9_1_6Alt = 211,
    Layout7_1_5_4 = 212,
    Layout10_2_9_3 = 213,
    SubsetLfe = 214,
    SubsetStereoS = 215,
    SubsetStereoSs = 216,
    SubsetStereoRs = 217,
    SubsetStereoTf = 218,
    SubsetStereoTb = 219,
    SubsetTop4Ch = 220,
    Subset3_0Ch = 221,
    SubsetStereoF = 222,
    SubsetStereoSi = 223,
    SubsetStereoTpSi = 224,
    SubsetTop6Ch = 225,
    SubsetLfePair = 226,
    SubsetBottom3Ch = 227,
    SubsetBottom4Ch = 228,
    SubsetTop1Ch = 229,
    SubsetTop5Ch = 230,
    // Objects
    ObjectMono = 301,
    ObjectDual = 302,
    // Passthrough
    PassthroughMono = 401,
    PassthroughStereo = 402,
}

impl AudioElementType {
    /// Returns `true` if this type is an Ambisonics type.
    pub fn is_ambisonics(&self) -> bool {
        matches!(self, Self::Oa1 | Self::Oa2 | Self::Oa3 | Self::Oa4)
    }

    /// Returns `true` if this type is a loudspeaker layout.
    pub fn is_loudspeaker_layout(&self) -> bool {
        matches!(
            self,
            Self::LayoutMono
                | Self::LayoutStereo
                | Self::Layout5_1_0
                | Self::Layout5_1_2
                | Self::Layout5_1_4
                | Self::Layout7_1_0
                | Self::Layout7_1_2
                | Self::Layout7_1_4
                | Self::Layout3_1_2
                | Self::Layout9_1_6
                | Self::Layout9_1_6Alt
                | Self::Layout7_1_5_4
                | Self::Layout10_2_9_3
                | Self::SubsetLfe
                | Self::SubsetStereoS
                | Self::SubsetStereoSs
                | Self::SubsetStereoRs
                | Self::SubsetStereoTf
                | Self::SubsetStereoTb
                | Self::SubsetTop4Ch
                | Self::Subset3_0Ch
                | Self::SubsetStereoF
                | Self::SubsetStereoSi
                | Self::SubsetStereoTpSi
                | Self::SubsetTop6Ch
                | Self::SubsetLfePair
                | Self::SubsetBottom3Ch
                | Self::SubsetBottom4Ch
                | Self::SubsetTop1Ch
                | Self::SubsetTop5Ch
        )
    }

    /// Returns `true` if this type is an object.
    pub fn is_object(&self) -> bool {
        matches!(self, Self::ObjectMono | Self::ObjectDual)
    }

    /// Returns `true` if this type is passthrough.
    pub fn is_passthrough(&self) -> bool {
        matches!(self, Self::PassthroughMono | Self::PassthroughStereo)
    }

    /// Returns the Ambisonic order if this is an Ambisonics type.
    pub fn get_ambisonic_order(&self) -> Option<i32> {
        match *self {
            Self::Oa1 => Some(1),
            Self::Oa2 => Some(2),
            Self::Oa3 => Some(3),
            Self::Oa4 => Some(4),
            _ => None,
        }
    }
}

// ===== Tests =====

#[cfg(test)]
mod test {

    use super::*;
    use googletest::prelude::*;

    #[gtest]
    fn test_is_ambisonics() {
        expect_true!(AudioElementType::Oa1.is_ambisonics());
        expect_true!(AudioElementType::Oa4.is_ambisonics());
        expect_false!(AudioElementType::LayoutStereo.is_ambisonics());
        expect_false!(AudioElementType::InvalidType.is_ambisonics());
    }

    #[gtest]
    fn test_is_loudspeaker_layout() {
        expect_true!(AudioElementType::LayoutMono.is_loudspeaker_layout());
        expect_true!(AudioElementType::Layout10_2_9_3.is_loudspeaker_layout());
        expect_true!(AudioElementType::SubsetTop5Ch.is_loudspeaker_layout());
        expect_false!(AudioElementType::Oa2.is_loudspeaker_layout());
    }

    #[gtest]
    fn test_is_object() {
        expect_true!(AudioElementType::ObjectMono.is_object());
        expect_true!(AudioElementType::ObjectDual.is_object());
        expect_false!(AudioElementType::Layout5_1_0.is_object());
    }

    #[gtest]
    fn test_is_passthrough() {
        expect_true!(AudioElementType::PassthroughMono.is_passthrough());
        expect_true!(AudioElementType::PassthroughStereo.is_passthrough());
        expect_false!(AudioElementType::ObjectMono.is_passthrough());
    }

    #[gtest]
    fn test_get_ambisonic_order() {
        expect_eq!(AudioElementType::Oa1.get_ambisonic_order(), Some(1));
        expect_eq!(AudioElementType::Oa3.get_ambisonic_order(), Some(3));
        expect_eq!(AudioElementType::LayoutStereo.get_ambisonic_order(), None);
    }
}
