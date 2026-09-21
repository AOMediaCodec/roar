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

use super::animation::{Animated, AnimatedFloat32, AnimatedLinearGain};
use super::units::{Decibels, LinearGain};
use crate::common::definitions::OarError;

/// Dynamic gain control, enforcing Decibels for creation and storing LinearGain internally.
#[derive(Debug, Clone, PartialEq)]
pub struct Gain {
    pub(crate) variant: GainVariant,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum GainVariant {
    Constant(LinearGain),
    Multiple(Vec<LinearGain>),
    Animated(AnimatedLinearGain),
}

impl Gain {
    /// Creates a new constant gain metadata from Decibels.
    pub fn new_constant(val: Decibels) -> Result<Self, OarError> {
        if val.0.is_finite() {
            Ok(Gain { variant: GainVariant::Constant(LinearGain::from(val)) })
        } else {
            Err(OarError::InvalidParameter)
        }
    }

    /// Creates a new per-channel static gain metadata from Decibels.
    pub fn new_multiple(vals: Vec<Decibels>) -> Result<Self, OarError> {
        if vals.iter().all(|v| v.0.is_finite()) {
            let lin_vals = vals.into_iter().map(LinearGain::from).collect();
            Ok(Gain { variant: GainVariant::Multiple(lin_vals) })
        } else {
            Err(OarError::InvalidParameter)
        }
    }

    /// Creates a new animated gain metadata where the animation start/end/control are in Decibels.
    pub fn new_animated(anim: AnimatedFloat32) -> Result<Self, OarError> {
        let variant = match anim {
            Animated::Step { value } => {
                if !value.is_finite() {
                    return Err(OarError::InvalidParameter);
                }
                Animated::Step { value: LinearGain::from(Decibels(value)) }
            }
            Animated::Linear { start, end } => {
                if !start.is_finite() || !end.is_finite() {
                    return Err(OarError::InvalidParameter);
                }
                Animated::Linear {
                    start: LinearGain::from(Decibels(start)),
                    end: LinearGain::from(Decibels(end)),
                }
            }
            Animated::Bezier { start, end, control, control_relative_time } => {
                if !start.is_finite()
                    || !end.is_finite()
                    || !control.is_finite()
                    || !control_relative_time.is_finite()
                    || !(0.0..=1.0).contains(&control_relative_time)
                {
                    return Err(OarError::InvalidParameter);
                }
                Animated::Bezier {
                    start: LinearGain::from(Decibels(start)),
                    end: LinearGain::from(Decibels(end)),
                    control: LinearGain::from(Decibels(control)),
                    control_relative_time,
                }
            }
        };
        Ok(Gain { variant: GainVariant::Animated(variant) })
    }
}

// ===== Tests =====

#[cfg(test)]
mod test {

    use super::*;
    use googletest::prelude::*;

    #[gtest]
    fn test_gain_new_constant_success() {
        let gain = Gain::new_constant(Decibels(0.0));
        expect_ok!(&gain);
        expect_eq!(
            gain.unwrap(),
            Gain { variant: GainVariant::Constant(LinearGain::from(Decibels(0.0))) }
        );
    }

    #[gtest]
    fn test_gain_new_constant_invalid() {
        let gain = Gain::new_constant(Decibels(f32::NAN));
        expect_true!(gain.is_err());
    }

    #[gtest]
    fn test_gain_new_multiple_success() {
        let gain = Gain::new_multiple(vec![Decibels(-3.0), Decibels(0.0), Decibels(3.0)]);
        expect_ok!(&gain);
        expect_eq!(
            gain.unwrap(),
            Gain {
                variant: GainVariant::Multiple(vec![
                    LinearGain::from(Decibels(-3.0)),
                    LinearGain::from(Decibels(0.0)),
                    LinearGain::from(Decibels(3.0)),
                ]),
            }
        );
    }

    #[gtest]
    fn test_gain_new_multiple_invalid() {
        let gain = Gain::new_multiple(vec![Decibels(0.1), Decibels(f32::INFINITY), Decibels(0.3)]);
        expect_true!(gain.is_err());
    }
}
