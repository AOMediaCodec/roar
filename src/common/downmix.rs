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

//! Downmix configurations and mode types.

use crate::common::definitions::OarError;

/// Standard IAMF downmix modes, mapping to configuration weights and default shifts.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownmixMode {
    /// Mode 1 with negative index shift offset.
    Mode1NegOffset = 0,
    /// Mode 2 with negative index shift offset.
    Mode2NegOffset = 1,
    /// Mode 3 with negative index shift offset.
    Mode3NegOffset = 2,
    /// Mode 1 with positive index shift offset.
    Mode1PosOffset = 4,
    /// Mode 2 with positive index shift offset.
    Mode2PosOffset = 5,
    /// Mode 3 with positive index shift offset.
    Mode3PosOffset = 6,
}

/// Direction of weight index shift.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeightIndexShift {
    /// Shift index down (decrease weight).
    Negative,
    /// Shift index up (increase weight).
    Positive,
}

/// IAMF downmix coefficients and weight shift parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MixFactors {
    /// Alpha gain coefficient (typically used for side surround mixdown).
    pub alpha: f32,
    /// Beta gain coefficient (typically used for back surround mixdown).
    pub beta: f32,
    /// Gamma gain coefficient (typically used for height channels mixdown).
    pub gamma: f32,
    /// Delta gain coefficient (typically used for L/R downmix balance).
    pub delta: f32,
    /// The default index shift direction for dynamic metadata updates.
    pub weight_index_shift: WeightIndexShift,
}

impl DownmixMode {
    pub fn value(&self) -> i32 {
        *self as i32
    }

    pub fn mix_factors(&self) -> MixFactors {
        match self {
            DownmixMode::Mode1NegOffset => MixFactors {
                alpha: 1.0,
                beta: 1.0,
                gamma: 0.707,
                delta: 0.707,
                weight_index_shift: WeightIndexShift::Negative,
            },
            DownmixMode::Mode2NegOffset => MixFactors {
                alpha: 0.707,
                beta: 0.707,
                gamma: 0.707,
                delta: 0.707,
                weight_index_shift: WeightIndexShift::Negative,
            },
            DownmixMode::Mode3NegOffset => MixFactors {
                alpha: 1.0,
                beta: 0.866,
                gamma: 0.866,
                delta: 0.866,
                weight_index_shift: WeightIndexShift::Negative,
            },
            DownmixMode::Mode1PosOffset => MixFactors {
                alpha: 1.0,
                beta: 1.0,
                gamma: 0.707,
                delta: 0.707,
                weight_index_shift: WeightIndexShift::Positive,
            },
            DownmixMode::Mode2PosOffset => MixFactors {
                alpha: 0.707,
                beta: 0.707,
                gamma: 0.707,
                delta: 0.707,
                weight_index_shift: WeightIndexShift::Positive,
            },
            DownmixMode::Mode3PosOffset => MixFactors {
                alpha: 1.0,
                beta: 0.866,
                gamma: 0.866,
                delta: 0.866,
                weight_index_shift: WeightIndexShift::Positive,
            },
        }
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct WeightIndex(i32);

impl WeightIndex {
    pub const MIN: Self = WeightIndex(0);
    pub const MAX: Self = WeightIndex(10);

    /// Creates a new `WeightIndex` if it lies within the valid range `[0, 10]`.
    pub fn new(val: i32) -> Result<Self, OarError> {
        if (0..=10).contains(&val) {
            Ok(WeightIndex(val))
        } else {
            Err(OarError::InvalidParameter)
        }
    }

    pub fn value(&self) -> i32 {
        self.0
    }

    /// Increments the weight index by 1 (clamped to 10).
    pub fn increment(&self) -> Self {
        WeightIndex((self.0 + 1).min(10))
    }

    /// Decrements the weight index by 1 (clamped to 0).
    pub fn decrement(&self) -> Self {
        WeightIndex((self.0 - 1).max(0))
    }
}

/// Downmix parameters for channel-based audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DownmixInfo {
    /// Demixing mode.
    pub mode: DownmixMode,
    /// Downmix weight index.
    pub weight_index: Option<WeightIndex>,
}

impl DownmixInfo {
    pub fn new(mode: DownmixMode, weight_index: Option<WeightIndex>) -> Self {
        DownmixInfo { mode, weight_index }
    }

    pub fn mode(&self) -> DownmixMode {
        self.mode
    }

    pub fn weight_index(&self) -> Option<WeightIndex> {
        self.weight_index
    }
}

/// Dynamic matrix coefficient type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Coeff {
    /// Fixed scaling factor.
    Fixed(f32),
    /// Alpha dynamic mixdown scaling parameter.
    Alpha,
    /// Beta dynamic mixdown scaling parameter.
    Beta,
    /// Gamma dynamic mixdown scaling parameter.
    Gamma,
    /// Delta dynamic mixdown scaling parameter.
    Delta,
    /// Delta scaled by weight index.
    DeltaW,
}
