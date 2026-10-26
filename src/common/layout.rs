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

//! Public output and input layout configurations.

use crate::common::definitions::{ChannelCount, IAChannel, OarError};

/// Target output loudspeaker and headphone layouts supported by the renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OutputLayout {
    /// Mono layout (1.0).
    Mono,
    /// Stereo layout (2.0).
    Stereo,
    /// 5.1 surround sound layout.
    Layout51,
    /// 5.1.2 surround sound layout (with height channels).
    Layout512,
    /// 5.1.4 surround sound layout (with height channels).
    Layout514,
    /// 7.1 surround sound layout.
    Layout71,
    /// 7.1.2 surround sound layout (with height channels).
    Layout712,
    /// 7.1.4 surround sound layout (with height channels).
    Layout714,
    /// 3.1.2 surround sound layout.
    Layout312,
    /// 9.1.6 surround sound layout.
    Layout916,
    /// 22.2 surround sound layout (10.2.9.3 / A293).
    LayoutA293,
    /// 7.1.5.4 surround sound layout.
    Layout7154,
    /// ITU-R BS.2051 Sound System E (4+5+1).
    SoundSystemE451,
    /// ITU-R BS.2051 Sound System F (3+7+0 with 2 LFE channels).
    SoundSystemF370,
    /// ITU-R BS.2051 Sound System G (4+9+0).
    SoundSystemG490,
    /// Binaural headphones rendering layout (2 channels).
    Binaural,
}

impl OutputLayout {
    /// Sound system A (0.2.0), mapping to [`OutputLayout::Stereo`].
    pub const SOUND_SYSTEM_A_020: Self = Self::Stereo;
    /// Sound system B (0.5.0), mapping to [`OutputLayout::Layout51`].
    pub const SOUND_SYSTEM_B_050: Self = Self::Layout51;
    /// Sound system C (2.5.0), mapping to [`OutputLayout::Layout512`].
    pub const SOUND_SYSTEM_C_250: Self = Self::Layout512;
    /// Sound system D (4.5.0), mapping to [`OutputLayout::Layout514`].
    pub const SOUND_SYSTEM_D_450: Self = Self::Layout514;
    /// Sound system I (0.7.0), mapping to [`OutputLayout::Layout71`].
    pub const SOUND_SYSTEM_I_070: Self = Self::Layout71;
    /// Sound system J (4.7.0), mapping to [`OutputLayout::Layout714`].
    pub const SOUND_SYSTEM_J_470: Self = Self::Layout714;
    /// Sound system H (9.a.3), mapping to [`OutputLayout::LayoutA293`].
    pub const SOUND_SYSTEM_H_9A3: Self = Self::LayoutA293;

    /// Returns the number of output channels for this layout.
    pub fn channels(&self) -> ChannelCount {
        let count = match self {
            Self::Mono => 1,
            Self::Stereo => 2,
            Self::Layout51 => 6,
            Self::Layout512 => 8,
            Self::Layout514 => 10,
            Self::Layout71 => 8,
            Self::Layout712 => 10,
            Self::Layout714 => 12,
            Self::Layout312 => 6,
            Self::Layout916 => 16,
            Self::LayoutA293 => 24,
            Self::Layout7154 => 17,
            Self::SoundSystemE451 => 11,
            Self::SoundSystemF370 => 12,
            Self::SoundSystemG490 => 14,
            Self::Binaural => 2,
        };
        ChannelCount::new(count).unwrap()
    }

    /// Returns the height and surround channel counts for downmix validation.
    pub fn height_and_surround(&self) -> Option<(usize, usize)> {
        InputLayout::try_from(*self).ok().and_then(|l| l.height_and_surround())
    }

    /// Physical channel definitions per layout (`_get_channels` equivalent in `liboar`).
    pub fn physical_channels(&self) -> Option<&'static [IAChannel]> {
        InputLayout::try_from(*self).ok().and_then(|l| l.physical_channels())
    }
}

/// Input loudspeaker and IAMF channel-subset layouts supported for channel-based audio elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputLayout {
    /// Mono layout (1.0).
    Mono,
    /// Stereo layout (2.0).
    Stereo,
    /// 5.1 surround sound layout.
    Layout51,
    /// 5.1.2 surround sound layout (with height channels).
    Layout512,
    /// 5.1.4 surround sound layout (with height channels).
    Layout514,
    /// 7.1 surround sound layout.
    Layout71,
    /// 7.1.2 surround sound layout (with height channels).
    Layout712,
    /// 7.1.4 surround sound layout (with height channels).
    Layout714,
    /// 3.1.2 surround sound layout.
    Layout312,
    /// 9.1.6 surround sound layout.
    Layout916,
    /// 22.2 surround sound layout (10.2.9.3 / A293).
    LayoutA293,
    /// 7.1.5.4 surround sound layout.
    Layout7154,
    /// Binaural 2-channel layout.
    Binaural,
    /// Low-frequency effects subset (LFE) of 7.1.4ch.
    Lfe,
    /// Surround subset (Ls/Rs) of 5.1.4ch.
    StereoS,
    /// Side surround subset (Lss/Rss) of 7.1.4ch.
    StereoSs,
    /// Rear surround subset (Lrs/Rrs) of 7.1.4ch.
    StereoRs,
    /// Top front subset (Ltf/Rtf) of 7.1.4ch.
    StereoTf,
    /// Top back subset (Ltb/Rtb) of 7.1.4ch.
    StereoTb,
    /// Top 4 channels (Ltf/Rtf/Ltb/Rtb) of 7.1.4ch.
    Top4ch,
    /// Front 3 channels (L/C/R) of 7.1.4ch.
    ThreeCh,
    /// Front subset (FL/FR) of 9.1.6ch.
    StereoF,
    /// Side subset (SiL/SiR) of 9.1.6ch.
    StereoSi,
    /// Top side subset (TpSiL/TpSiR) of 9.1.6ch.
    StereoTpsi,
    /// Top 6 channels (TpFL/TpFR/TpBL/TpBR/TpSiL/TpSiR) of 9.1.6ch.
    Top6ch,
    /// Low-frequency effects subset (LFE1/LFE2) of 10.2.9.3ch.
    LfePair,
    /// Bottom 3 channels (BtFC/BtFL/BtFR) of 10.2.9.3ch.
    Bottom3ch,
    /// Bottom 4 channels (BtFL/BtFR/BtBL/BtBR) of 7.1.5.4ch.
    Bottom4ch,
    /// Top subset (TpC) of 7.1.5.4ch.
    Top1ch,
    /// Top 5 channels (Ltf/Rtf/TpC/Ltb/Rtb) of 7.1.5.4ch.
    Top5ch,
}

impl InputLayout {
    /// Sound system A (0.2.0), mapping to [`InputLayout::Stereo`].
    pub const SOUND_SYSTEM_A_020: Self = Self::Stereo;
    /// Sound system B (0.5.0), mapping to [`InputLayout::Layout51`].
    pub const SOUND_SYSTEM_B_050: Self = Self::Layout51;
    /// Sound system C (2.5.0), mapping to [`InputLayout::Layout512`].
    pub const SOUND_SYSTEM_C_250: Self = Self::Layout512;
    /// Sound system D (4.5.0), mapping to [`InputLayout::Layout514`].
    pub const SOUND_SYSTEM_D_450: Self = Self::Layout514;
    /// Sound system I (0.7.0), mapping to [`InputLayout::Layout71`].
    pub const SOUND_SYSTEM_I_070: Self = Self::Layout71;
    /// Sound system J (4.7.0), mapping to [`InputLayout::Layout714`].
    pub const SOUND_SYSTEM_J_470: Self = Self::Layout714;
    /// Sound system H (9.a.3), mapping to [`InputLayout::LayoutA293`].
    pub const SOUND_SYSTEM_H_9A3: Self = Self::LayoutA293;

    /// Returns the number of input channels for this layout.
    pub fn channels(&self) -> ChannelCount {
        let count = match self {
            Self::Mono => 1,
            Self::Stereo => 2,
            Self::Layout51 => 6,
            Self::Layout512 => 8,
            Self::Layout514 => 10,
            Self::Layout71 => 8,
            Self::Layout712 => 10,
            Self::Layout714 => 12,
            Self::Layout312 => 6,
            Self::Layout916 => 16,
            Self::LayoutA293 => 24,
            Self::Layout7154 => 17,
            Self::Binaural => 2,
            Self::Lfe => 1,
            Self::StereoS => 2,
            Self::StereoSs => 2,
            Self::StereoRs => 2,
            Self::StereoTf => 2,
            Self::StereoTb => 2,
            Self::Top4ch => 4,
            Self::ThreeCh => 3,
            Self::StereoF => 2,
            Self::StereoSi => 2,
            Self::StereoTpsi => 2,
            Self::Top6ch => 6,
            Self::LfePair => 2,
            Self::Bottom3ch => 3,
            Self::Bottom4ch => 4,
            Self::Top1ch => 1,
            Self::Top5ch => 5,
        };
        ChannelCount::new(count).unwrap()
    }

    /// Returns the height and surround channel counts for downmix validation.
    pub fn height_and_surround(&self) -> Option<(usize, usize)> {
        match self {
            Self::Mono => Some((0, 1)),
            Self::Stereo => Some((0, 2)),
            Self::Layout51 => Some((0, 5)),
            Self::Layout512 => Some((2, 5)),
            Self::Layout514 => Some((4, 5)),
            Self::Layout71 => Some((0, 7)),
            Self::Layout712 => Some((2, 7)),
            Self::Layout714 => Some((4, 7)),
            Self::Layout312 => Some((2, 3)),
            _ => None,
        }
    }

    /// Checks whether downmix from this input layout to `target` output layout is valid.
    pub fn is_valid_downmix_to(&self, target: OutputLayout) -> bool {
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
            Self::Mono => Some(&[IAChannel::Mono]),
            Self::Stereo => Some(&[IAChannel::L2, IAChannel::R2]),
            Self::Layout51 => Some(&[
                IAChannel::L5,
                IAChannel::R5,
                IAChannel::C,
                IAChannel::Lfe,
                IAChannel::Sl5,
                IAChannel::Sr5,
            ]),
            Self::Layout512 => Some(&[
                IAChannel::L5,
                IAChannel::R5,
                IAChannel::C,
                IAChannel::Lfe,
                IAChannel::Sl5,
                IAChannel::Sr5,
                IAChannel::Hl,
                IAChannel::Hr,
            ]),
            Self::Layout514 => Some(&[
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
            Self::Layout71 => Some(&[
                IAChannel::L7,
                IAChannel::R7,
                IAChannel::C,
                IAChannel::Lfe,
                IAChannel::Sl7,
                IAChannel::Sr7,
                IAChannel::Bl7,
                IAChannel::Br7,
            ]),
            Self::Layout712 => Some(&[
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
            Self::Layout714 => Some(&[
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
            Self::Layout312 => Some(&[
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

impl TryFrom<OutputLayout> for InputLayout {
    type Error = OarError;

    fn try_from(layout: OutputLayout) -> Result<Self, Self::Error> {
        match layout {
            OutputLayout::Mono => Ok(Self::Mono),
            OutputLayout::Stereo => Ok(Self::Stereo),
            OutputLayout::Layout51 => Ok(Self::Layout51),
            OutputLayout::Layout512 => Ok(Self::Layout512),
            OutputLayout::Layout514 => Ok(Self::Layout514),
            OutputLayout::Layout71 => Ok(Self::Layout71),
            OutputLayout::Layout712 => Ok(Self::Layout712),
            OutputLayout::Layout714 => Ok(Self::Layout714),
            OutputLayout::Layout312 => Ok(Self::Layout312),
            OutputLayout::Layout916 => Ok(Self::Layout916),
            OutputLayout::LayoutA293 => Ok(Self::LayoutA293),
            OutputLayout::Layout7154 => Ok(Self::Layout7154),
            OutputLayout::Binaural => Ok(Self::Binaural),
            OutputLayout::SoundSystemE451
            | OutputLayout::SoundSystemF370
            | OutputLayout::SoundSystemG490 => Err(OarError::InvalidParameter),
        }
    }
}

impl TryFrom<InputLayout> for OutputLayout {
    type Error = OarError;

    fn try_from(layout: InputLayout) -> Result<Self, Self::Error> {
        match layout {
            InputLayout::Mono => Ok(Self::Mono),
            InputLayout::Stereo => Ok(Self::Stereo),
            InputLayout::Layout51 => Ok(Self::Layout51),
            InputLayout::Layout512 => Ok(Self::Layout512),
            InputLayout::Layout514 => Ok(Self::Layout514),
            InputLayout::Layout71 => Ok(Self::Layout71),
            InputLayout::Layout712 => Ok(Self::Layout712),
            InputLayout::Layout714 => Ok(Self::Layout714),
            InputLayout::Layout312 => Ok(Self::Layout312),
            InputLayout::Layout916 => Ok(Self::Layout916),
            InputLayout::LayoutA293 => Ok(Self::LayoutA293),
            InputLayout::Layout7154 => Ok(Self::Layout7154),
            InputLayout::Binaural => Ok(Self::Binaural),
            InputLayout::Lfe
            | InputLayout::StereoS
            | InputLayout::StereoSs
            | InputLayout::StereoRs
            | InputLayout::StereoTf
            | InputLayout::StereoTb
            | InputLayout::Top4ch
            | InputLayout::ThreeCh
            | InputLayout::StereoF
            | InputLayout::StereoSi
            | InputLayout::StereoTpsi
            | InputLayout::Top6ch
            | InputLayout::LfePair
            | InputLayout::Bottom3ch
            | InputLayout::Bottom4ch
            | InputLayout::Top1ch
            | InputLayout::Top5ch => Err(OarError::InvalidParameter),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use googletest::prelude::*;

    #[gtest]
    fn output_layout_sound_system_aliases_match_standard_layouts() {
        expect_that!(OutputLayout::SOUND_SYSTEM_A_020, eq(OutputLayout::Stereo));
        expect_that!(OutputLayout::SOUND_SYSTEM_B_050, eq(OutputLayout::Layout51));
        expect_that!(OutputLayout::SOUND_SYSTEM_C_250, eq(OutputLayout::Layout512));
        expect_that!(OutputLayout::SOUND_SYSTEM_D_450, eq(OutputLayout::Layout514));
        expect_that!(OutputLayout::SOUND_SYSTEM_I_070, eq(OutputLayout::Layout71));
        expect_that!(OutputLayout::SOUND_SYSTEM_J_470, eq(OutputLayout::Layout714));
        expect_that!(OutputLayout::SOUND_SYSTEM_H_9A3, eq(OutputLayout::LayoutA293));
    }

    #[gtest]
    fn input_layout_sound_system_aliases_match_standard_layouts() {
        expect_that!(InputLayout::SOUND_SYSTEM_A_020, eq(InputLayout::Stereo));
        expect_that!(InputLayout::SOUND_SYSTEM_B_050, eq(InputLayout::Layout51));
        expect_that!(InputLayout::SOUND_SYSTEM_C_250, eq(InputLayout::Layout512));
        expect_that!(InputLayout::SOUND_SYSTEM_D_450, eq(InputLayout::Layout514));
        expect_that!(InputLayout::SOUND_SYSTEM_I_070, eq(InputLayout::Layout71));
        expect_that!(InputLayout::SOUND_SYSTEM_J_470, eq(InputLayout::Layout714));
        expect_that!(InputLayout::SOUND_SYSTEM_H_9A3, eq(InputLayout::LayoutA293));
    }

    #[gtest]
    fn output_layout_channels_returns_expected_channel_counts() {
        expect_that!(OutputLayout::Mono.channels().get(), eq(1));
        expect_that!(OutputLayout::Stereo.channels().get(), eq(2));
        expect_that!(OutputLayout::Layout51.channels().get(), eq(6));
        expect_that!(OutputLayout::Layout714.channels().get(), eq(12));
        expect_that!(OutputLayout::SoundSystemE451.channels().get(), eq(11));
        expect_that!(OutputLayout::SoundSystemF370.channels().get(), eq(12));
        expect_that!(OutputLayout::SoundSystemG490.channels().get(), eq(14));
        expect_that!(OutputLayout::LayoutA293.channels().get(), eq(24));
        expect_that!(OutputLayout::Binaural.channels().get(), eq(2));
    }

    #[gtest]
    fn input_layout_channels_returns_expected_channel_counts() {
        expect_that!(InputLayout::Mono.channels().get(), eq(1));
        expect_that!(InputLayout::Binaural.channels().get(), eq(2));
        expect_that!(InputLayout::Layout51.channels().get(), eq(6));
        expect_that!(InputLayout::Layout916.channels().get(), eq(16));
        expect_that!(InputLayout::Lfe.channels().get(), eq(1));
        expect_that!(InputLayout::Top4ch.channels().get(), eq(4));
        expect_that!(InputLayout::Top6ch.channels().get(), eq(6));
        expect_that!(InputLayout::Bottom3ch.channels().get(), eq(3));
        expect_that!(InputLayout::Top5ch.channels().get(), eq(5));
    }

    #[gtest]
    fn try_from_output_layout_converts_shared_layouts_and_rejects_output_only_layouts() {
        let converted = InputLayout::try_from(OutputLayout::Layout714);
        let converted_binaural = InputLayout::try_from(OutputLayout::Binaural);
        let rejected_e451 = InputLayout::try_from(OutputLayout::SoundSystemE451);

        expect_that!(converted, ok(eq(InputLayout::Layout714)));
        expect_that!(converted_binaural, ok(eq(InputLayout::Binaural)));
        expect_that!(rejected_e451, err(eq(OarError::InvalidParameter)));
    }

    #[gtest]
    fn try_from_input_layout_converts_shared_layouts_and_rejects_subset_layouts() {
        let converted = OutputLayout::try_from(InputLayout::Layout512);
        let converted_binaural = OutputLayout::try_from(InputLayout::Binaural);
        let rejected_subset = OutputLayout::try_from(InputLayout::Top4ch);

        expect_that!(converted, ok(eq(OutputLayout::Layout512)));
        expect_that!(converted_binaural, ok(eq(OutputLayout::Binaural)));
        expect_that!(rejected_subset, err(eq(OarError::InvalidParameter)));
    }

    #[gtest]
    fn layout_height_and_surround_returns_expected_tuples() {
        expect_that!(InputLayout::Stereo.height_and_surround(), eq(Some((0, 2))));
        expect_that!(InputLayout::Layout512.height_and_surround(), eq(Some((2, 5))));
        expect_that!(OutputLayout::Layout714.height_and_surround(), eq(Some((4, 7))));
        expect_that!(OutputLayout::Binaural.height_and_surround(), none());
    }

    #[gtest]
    fn input_layout_is_valid_downmix_to_validates_target_hierarchy() {
        expect_true!(InputLayout::Layout71.is_valid_downmix_to(OutputLayout::Stereo));
        expect_true!(InputLayout::Layout714.is_valid_downmix_to(OutputLayout::Layout512));
        expect_false!(InputLayout::Layout512.is_valid_downmix_to(OutputLayout::Layout51));
        expect_false!(InputLayout::Layout512.is_valid_downmix_to(OutputLayout::Stereo));
        expect_false!(InputLayout::Stereo.is_valid_downmix_to(OutputLayout::Layout51));
        expect_false!(InputLayout::Layout512.is_valid_downmix_to(OutputLayout::Layout712));
        expect_false!(InputLayout::Layout512.is_valid_downmix_to(OutputLayout::Layout514));
    }
}
