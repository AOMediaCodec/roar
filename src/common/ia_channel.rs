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

//! Physical IAMF channel definitions.

/// Internal representation of physical IAMF channels (`IAChannel` equivalent in `liboar`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IAChannel {
    /// Left channel (7.x loudspeaker configuration).
    L7 = 1,
    /// Right channel (7.x loudspeaker configuration).
    R7 = 2,
    /// Center channel.
    C = 3,
    /// Low-frequency effects (LFE) channel.
    Lfe = 4,
    /// Surround Left channel (7.x loudspeaker configuration).
    Sl7 = 5,
    /// Surround Right channel (7.x loudspeaker configuration).
    Sr7 = 6,
    /// Back Left channel (7.x loudspeaker configuration).
    Bl7 = 7,
    /// Back Right channel (7.x loudspeaker configuration).
    Br7 = 8,
    /// Height Front Left channel.
    Hfl = 9,
    /// Height Front Right channel.
    Hfr = 10,
    /// Height Back Left channel.
    Hbl = 11,
    /// Height Back Right channel.
    Hbr = 12,
    /// Mono channel.
    Mono = 13,
    /// Left channel (Stereo/2.0 loudspeaker configuration).
    L2 = 14,
    /// Right channel (Stereo/2.0 loudspeaker configuration).
    R2 = 15,
    /// Top Left channel.
    Tl = 16,
    /// Top Right channel.
    Tr = 17,
    /// Left channel (3.x loudspeaker configuration).
    L3 = 18,
    /// Right channel (3.x loudspeaker configuration).
    R3 = 19,
    /// Surround Left channel (5.x loudspeaker configuration).
    Sl5 = 20,
    /// Surround Right channel (5.x loudspeaker configuration).
    Sr5 = 21,
    /// Height Left channel.
    Hl = 22,
    /// Height Right channel.
    Hr = 23,
}

impl IAChannel {
    pub const L5: Self = Self::L7;
    pub const R5: Self = Self::R7;
}
