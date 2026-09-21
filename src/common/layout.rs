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

use crate::common::definitions::IAChannel;

/// Loudspeaker layouts and target layouts, equivalent of C API `oar_layout_t`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Layout {
    /// Mono layout (1.0).
    Mono = 1,
    /// Stereo layout (2.0).
    Stereo = 2,
    /// 5.1 surround sound layout.
    Layout51 = 3,
    /// 5.1.2 surround sound layout (with height channels).
    Layout512 = 4,
    /// 5.1.4 surround sound layout (with height channels).
    Layout514 = 5,
    /// 7.1 surround sound layout.
    Layout71 = 6,
    /// 7.1.2 surround sound layout (with height channels).
    Layout712 = 7,
    /// 7.1.4 surround sound layout (with height channels).
    Layout714 = 8,
    /// 3.1.2 surround sound layout.
    Layout312 = 9,
    /// 9.1.6 surround sound layout.
    Layout916 = 10,
    /// 22.2 surround sound layout (A293).
    LayoutA293 = 11,
    /// 7.1.5.4 surround sound layout.
    Layout7154 = 12,

    /// Target layout: Sound system E (4.5.1).
    SoundSystemE451 = 129,
    /// Target layout: Sound system F (3.7.0).
    SoundSystemF370 = 130,
    /// Target layout: Sound system G (4.9.0).
    SoundSystemG490 = 131,

    /// Binaural headphones rendering layout.
    Binaural = 255,

    // Start of loudspeaker subset layouts for IAMF.
    // TODO(b/525080422): Move subsets to their own enum.
    /// Low-frequency effects subset (LFE) of 7.1.4ch.
    Lfe = 257,
    /// Surround subset (Ls/Rs) of 5.1.4ch.
    StereoS = 258,
    /// Side surround subset (Lss/Rss) of 7.1.4ch.
    StereoSs = 259,
    /// Rear surround subset (Lrs/Rrs) of 7.1.4ch.
    StereoRs = 260,
    /// Top front subset (Ltf/Rtf) of 7.1.4ch.
    StereoTf = 261,
    /// Top back subset (Ltb/Rtb) of 7.1.4ch.
    StereoTb = 262,
    /// Top 4 channels (Ltf/Rtf/Ltb/Rtb) of 7.1.4ch.
    Top4ch = 263,
    /// Front 3 channels (L/C/R) of 7.1.4ch.
    ThreeCh = 264,
    /// Front subset (FL/FR) of 9.1.6ch.
    StereoF = 265,
    /// Side subset (SiL/SiR) of 9.1.6ch.
    StereoSi = 266,
    /// Top side subset (TpSiL/TpSiR) of 9.1.6ch.
    StereoTpsi = 267,
    /// Top 6 channels (TpFL/TpFR/TpBL/TpBR/TpSiL/TpSiR) of 9.1.6ch.
    Top6ch = 268,
    /// Low-frequency effects subset (LFE1/LFE2) of 10.2.9.3ch.
    LfePair = 269,
    /// Bottom 3 channels (BtFC/BtFL/BtFR) of 10.2.9.3ch.
    Bottom3ch = 270,
    /// Bottom 4 channels (BtFL/BtFR/BtBL/BtBR) of 7.1.5.4ch.
    Bottom4ch = 271,
    /// Top subset (TpC) of 7.1.5.4ch.
    Top1ch = 272,
    /// Top 5 channels (Ltf/Rtf/TpC/Ltb/Rtb) of 7.1.5.4ch.
    Top5ch = 273,
}

impl Layout {
    /// Sound system A (0.2.0), mapping to Stereo.
    pub const SOUND_SYSTEM_A_020: Layout = Layout::Stereo;
    /// Sound system B (0.5.0), mapping to Layout51.
    pub const SOUND_SYSTEM_B_050: Layout = Layout::Layout51;
    /// Sound system C (2.5.0), mapping to Layout512.
    pub const SOUND_SYSTEM_C_250: Layout = Layout::Layout512;
    /// Sound system D (4.5.0), mapping to Layout514.
    pub const SOUND_SYSTEM_D_450: Layout = Layout::Layout514;
    /// Sound system I (0.7.0), mapping to Layout71.
    pub const SOUND_SYSTEM_I_070: Layout = Layout::Layout71;
    /// Sound system J (4.7.0), mapping to Layout714.
    pub const SOUND_SYSTEM_J_470: Layout = Layout::Layout714;
    /// Sound system H (9.a.3), mapping to LayoutA293.
    pub const SOUND_SYSTEM_H_9A3: Layout = Layout::LayoutA293;
}

impl Layout {
    /// Returns the number of channels for this layout.
    pub fn channels(&self) -> usize {
        match self {
            Layout::Mono => 1,
            Layout::Stereo => 2,
            Layout::Layout51 => 6,
            Layout::Layout512 => 8,
            Layout::Layout514 => 10,
            Layout::Layout71 => 8,
            Layout::Layout712 => 10,
            Layout::Layout714 => 12,
            Layout::Layout312 => 6,
            Layout::Layout916 => 16,
            Layout::LayoutA293 => 24,
            Layout::Layout7154 => 17,
            Layout::SoundSystemE451 => 11,
            Layout::SoundSystemF370 => 12,
            Layout::SoundSystemG490 => 14,
            Layout::Binaural => 2,
            Layout::Lfe => 1,
            Layout::StereoS => 2,
            Layout::StereoSs => 2,
            Layout::StereoRs => 2,
            Layout::StereoTf => 2,
            Layout::StereoTb => 2,
            Layout::Top4ch => 4,
            Layout::ThreeCh => 3,
            Layout::StereoF => 2,
            Layout::StereoSi => 2,
            Layout::StereoTpsi => 2,
            Layout::Top6ch => 6,
            Layout::LfePair => 2,
            Layout::Bottom3ch => 3,
            Layout::Bottom4ch => 4,
            Layout::Top1ch => 1,
            Layout::Top5ch => 5,
        }
    }

    /// Returns the height and surround channel counts.
    pub fn height_and_surround(&self) -> Option<(usize, usize)> {
        match self {
            Layout::Mono => Some((0, 1)),
            Layout::Stereo => Some((0, 2)),
            Layout::Layout51 => Some((0, 5)),
            Layout::Layout512 => Some((2, 5)),
            Layout::Layout514 => Some((4, 5)),
            Layout::Layout71 => Some((0, 7)),
            Layout::Layout712 => Some((2, 7)),
            Layout::Layout714 => Some((4, 7)),
            Layout::Layout312 => Some((2, 3)),
            _ => None,
        }
    }

    /// Checks whether downmix from this layout to `target` is valid.
    pub fn is_valid_downmix_to(&self, target: Layout) -> bool {
        let (Some((hi, si)), Some((ho, so))) =
            (self.height_and_surround(), target.height_and_surround())
        else {
            return false;
        };
        if hi > 0 && ho == 0 {
            return false;
        }
        si >= so && hi >= ho
    }

    /// Physical channel definitions per layout (`_get_channels` equivalent in `liboar`).
    pub fn physical_channels(&self) -> Option<&'static [IAChannel]> {
        match self {
            Layout::Mono => Some(&[IAChannel::Mono]),
            Layout::Stereo => Some(&[IAChannel::L2, IAChannel::R2]),
            Layout::Layout51 => Some(&[
                IAChannel::L5,
                IAChannel::R5,
                IAChannel::C,
                IAChannel::Lfe,
                IAChannel::Sl5,
                IAChannel::Sr5,
            ]),
            Layout::Layout512 => Some(&[
                IAChannel::L5,
                IAChannel::R5,
                IAChannel::C,
                IAChannel::Lfe,
                IAChannel::Sl5,
                IAChannel::Sr5,
                IAChannel::Hl,
                IAChannel::Hr,
            ]),
            Layout::Layout514 => Some(&[
                IAChannel::L5,
                IAChannel::R5,
                IAChannel::C,
                IAChannel::Lfe,
                IAChannel::Sl5,
                IAChannel::Sr5,
                IAChannel::Hfl,
                IAChannel::Hfr,
                IAChannel::Hbl,
                IAChannel::Hbr,
            ]),
            Layout::Layout71 => Some(&[
                IAChannel::L7,
                IAChannel::R7,
                IAChannel::C,
                IAChannel::Lfe,
                IAChannel::Sl7,
                IAChannel::Sr7,
                IAChannel::Bl7,
                IAChannel::Br7,
            ]),
            Layout::Layout712 => Some(&[
                IAChannel::L7,
                IAChannel::R7,
                IAChannel::C,
                IAChannel::Lfe,
                IAChannel::Sl7,
                IAChannel::Sr7,
                IAChannel::Bl7,
                IAChannel::Br7,
                IAChannel::Hl,
                IAChannel::Hr,
            ]),
            Layout::Layout714 => Some(&[
                IAChannel::L7,
                IAChannel::R7,
                IAChannel::C,
                IAChannel::Lfe,
                IAChannel::Sl7,
                IAChannel::Sr7,
                IAChannel::Bl7,
                IAChannel::Br7,
                IAChannel::Hfl,
                IAChannel::Hfr,
                IAChannel::Hbl,
                IAChannel::Hbr,
            ]),
            Layout::Layout312 => Some(&[
                IAChannel::L3,
                IAChannel::R3,
                IAChannel::C,
                IAChannel::Lfe,
                IAChannel::Tl,
                IAChannel::Tr,
            ]),
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
    fn test_oar_layout_discriminants() {
        expect_that!(Layout::Mono as i32, eq(1));
        expect_that!(Layout::Stereo as i32, eq(2));
        expect_that!(Layout::Layout51 as i32, eq(3));
        expect_that!(Layout::Layout512 as i32, eq(4));
        expect_that!(Layout::Layout514 as i32, eq(5));
        expect_that!(Layout::Layout71 as i32, eq(6));
        expect_that!(Layout::Layout712 as i32, eq(7));
        expect_that!(Layout::Layout714 as i32, eq(8));
        expect_that!(Layout::Layout312 as i32, eq(9));
        expect_that!(Layout::Layout916 as i32, eq(10));
        expect_that!(Layout::LayoutA293 as i32, eq(11));
        expect_that!(Layout::Layout7154 as i32, eq(12));

        expect_that!(Layout::SoundSystemE451 as i32, eq(129));
        expect_that!(Layout::SoundSystemF370 as i32, eq(130));
        expect_that!(Layout::SoundSystemG490 as i32, eq(131));

        expect_that!(Layout::Binaural as i32, eq(255));

        expect_that!(Layout::Lfe as i32, eq(257));
        expect_that!(Layout::StereoS as i32, eq(258));
        expect_that!(Layout::StereoSs as i32, eq(259));
        expect_that!(Layout::StereoRs as i32, eq(260));
        expect_that!(Layout::StereoTf as i32, eq(261));
        expect_that!(Layout::StereoTb as i32, eq(262));
        expect_that!(Layout::Top4ch as i32, eq(263));
        expect_that!(Layout::ThreeCh as i32, eq(264));
        expect_that!(Layout::StereoF as i32, eq(265));
        expect_that!(Layout::StereoSi as i32, eq(266));
        expect_that!(Layout::StereoTpsi as i32, eq(267));
        expect_that!(Layout::Top6ch as i32, eq(268));
        expect_that!(Layout::LfePair as i32, eq(269));
        expect_that!(Layout::Bottom3ch as i32, eq(270));
        expect_that!(Layout::Bottom4ch as i32, eq(271));
        expect_that!(Layout::Top1ch as i32, eq(272));
        expect_that!(Layout::Top5ch as i32, eq(273));
    }

    #[gtest]
    fn test_oar_layout_associated_constants() {
        expect_that!(Layout::SOUND_SYSTEM_A_020, eq(Layout::Stereo));
        expect_that!(Layout::SOUND_SYSTEM_B_050, eq(Layout::Layout51));
        expect_that!(Layout::SOUND_SYSTEM_C_250, eq(Layout::Layout512));
        expect_that!(Layout::SOUND_SYSTEM_D_450, eq(Layout::Layout514));
        expect_that!(Layout::SOUND_SYSTEM_I_070, eq(Layout::Layout71));
        expect_that!(Layout::SOUND_SYSTEM_J_470, eq(Layout::Layout714));
        expect_that!(Layout::SOUND_SYSTEM_H_9A3, eq(Layout::LayoutA293));
    }

    #[gtest]
    fn test_oar_layout_height_and_surround() {
        expect_that!(Layout::Stereo.height_and_surround(), eq(Some((0, 2))));
        expect_that!(Layout::Layout512.height_and_surround(), eq(Some((2, 5))));
        expect_that!(Layout::Layout714.height_and_surround(), eq(Some((4, 7))));
        expect_that!(Layout::Binaural.height_and_surround(), none());
    }

    #[gtest]
    fn test_oar_layout_is_valid_downmix_to() {
        expect_true!(Layout::Layout71.is_valid_downmix_to(Layout::Stereo));
        expect_true!(Layout::Layout714.is_valid_downmix_to(Layout::Layout512));
        expect_false!(Layout::Layout512.is_valid_downmix_to(Layout::Layout51));
        expect_false!(Layout::Layout512.is_valid_downmix_to(Layout::Stereo));
        expect_false!(Layout::Stereo.is_valid_downmix_to(Layout::Layout51));
        expect_false!(Layout::Layout512.is_valid_downmix_to(Layout::Layout712));
        expect_false!(Layout::Layout512.is_valid_downmix_to(Layout::Layout514));
    }
}
