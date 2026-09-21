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

use crate::common::definitions::OarError;
use crate::utility::oar_utils::validate_float;

// ===== Types =====

/// Represents an angle in degrees.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Degrees(pub f32);

/// Represents an angle in radians.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Radians(pub f32);

/// Represents a value in decibels (dB).
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Decibels(pub f32);

/// Represents a duration in milliseconds.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Milliseconds(pub f64);

/// Represents a duration or offset measured in audio samples.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Samples(pub u32);

// ===== Degrees =====

impl Degrees {
    /// Creates a new Degrees value if it is finite.
    pub fn new(val: f32) -> Result<Self, OarError> {
        Ok(Self(validate_float(val)?))
    }
}

// ===== Decibels =====

impl Decibels {
    /// Minimum decibel floor representing silence/zero gain.
    pub const MIN_DECIBELS: f32 = -120.0;

    /// Creates a new Decibels value if it is finite.
    pub fn new(val: f32) -> Result<Self, OarError> {
        Ok(Self(validate_float(val)?))
    }
}

// ===== Radians =====

impl Radians {
    /// Creates a new Radians value if it is finite.
    pub fn new(val: f32) -> Result<Self, OarError> {
        Ok(Self(validate_float(val)?))
    }
}

/// Represents a linear gain/amplitude factor.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct LinearGain(pub f32);

impl From<Degrees> for Radians {
    fn from(deg: Degrees) -> Self {
        Radians(deg.0.to_radians())
    }
}

impl From<Radians> for Degrees {
    fn from(rad: Radians) -> Self {
        Degrees(rad.0.to_degrees())
    }
}

// ===== LinearGain =====

impl LinearGain {
    /// Creates a new LinearGain value if it is finite.
    pub fn new(val: f32) -> Result<Self, OarError> {
        Ok(Self(validate_float(val)?))
    }
}

impl std::ops::Sub for Decibels {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Decibels(self.0 - rhs.0)
    }
}

impl From<Decibels> for LinearGain {
    fn from(db: Decibels) -> Self {
        LinearGain(10.0_f32.powf(0.05 * db.0))
    }
}

impl From<LinearGain> for Decibels {
    fn from(lin: LinearGain) -> Self {
        if lin.0 <= 0.0 {
            Decibels(Decibels::MIN_DECIBELS)
        } else {
            Decibels(20.0 * lin.0.log10())
        }
    }
}

impl std::ops::Add for LinearGain {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        LinearGain(self.0 + rhs.0)
    }
}

impl std::ops::Sub for LinearGain {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        LinearGain(self.0 - rhs.0)
    }
}

impl std::ops::Mul<f32> for LinearGain {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        LinearGain(self.0 * rhs)
    }
}

impl std::ops::MulAssign<LinearGain> for f32 {
    fn mul_assign(&mut self, rhs: LinearGain) {
        *self *= rhs.0;
    }
}

impl std::ops::Sub<f32> for LinearGain {
    type Output = f32;
    fn sub(self, rhs: f32) -> Self::Output {
        self.0 - rhs
    }
}

// ===== Milliseconds =====

impl Milliseconds {
    /// Creates a new Milliseconds value if it is finite and positive.
    pub fn new(val: f64) -> Result<Self, OarError> {
        if val.is_finite() && val > 0.0 {
            Ok(Milliseconds(val))
        } else {
            Err(OarError::InvalidParameter)
        }
    }
}

// ===== Samples =====

impl Samples {
    /// Creates a new Samples value if it is within `1..=i32::MAX`.
    pub fn new(val: u32) -> Result<Self, OarError> {
        if val > 0 && val <= i32::MAX as u32 {
            Ok(Samples(val))
        } else {
            Err(OarError::InvalidParameter)
        }
    }

    /// Helper to get the inner u32 value.
    pub fn value(&self) -> u32 {
        self.0
    }
}

impl From<Samples> for u32 {
    fn from(s: Samples) -> Self {
        s.0
    }
}

impl From<Samples> for usize {
    fn from(s: Samples) -> Self {
        s.0 as usize
    }
}

// ===== Tests =====

#[cfg(test)]
mod test {

    use super::*;
    use googletest::prelude::*;

    #[gtest]
    fn test_angle_conversions() {
        let deg = Degrees(180.0);
        let rad = Radians::from(deg);
        expect_that!(rad.0, near(std::f32::consts::PI, 1e-6));

        let round_trip = Degrees::from(rad);
        expect_that!(round_trip.0, near(180.0, 1e-6));
    }

    #[gtest]
    fn test_gain_conversions() {
        let db = Decibels(-6.0206);
        let lin = LinearGain::from(db);
        expect_that!(lin.0, near(0.5, 1e-4));

        let round_trip = Decibels::from(lin);
        expect_that!(round_trip.0, near(-6.0206, 1e-4));

        let zero_lin = LinearGain(0.0);
        let zero_db = Decibels::from(zero_lin);
        expect_that!(zero_db.0, eq(Decibels::MIN_DECIBELS));

        let neg_lin = LinearGain(-1.5);
        let neg_db = Decibels::from(neg_lin);
        expect_that!(neg_db.0, eq(Decibels::MIN_DECIBELS));
    }

    #[gtest]
    fn test_degrees_validation() {
        expect_ok!(Degrees::new(0.0));
        expect_ok!(Degrees::new(360.0));
        expect_ok!(Degrees::new(-180.0));

        expect_eq!(Degrees::new(f32::NAN).unwrap_err(), OarError::InvalidParameter);
        expect_eq!(Degrees::new(f32::INFINITY).unwrap_err(), OarError::InvalidParameter);
    }

    #[gtest]
    fn test_radians_validation() {
        expect_ok!(Radians::new(0.0));
        expect_ok!(Radians::new(std::f32::consts::PI));

        expect_eq!(Radians::new(f32::NAN).unwrap_err(), OarError::InvalidParameter);
        expect_eq!(Radians::new(f32::INFINITY).unwrap_err(), OarError::InvalidParameter);
    }

    #[gtest]
    fn test_decibels_validation() {
        expect_ok!(Decibels::new(0.0));
        expect_ok!(Decibels::new(-120.0));
        expect_ok!(Decibels::new(12.0));

        expect_eq!(Decibels::new(f32::NAN).unwrap_err(), OarError::InvalidParameter);
        expect_eq!(Decibels::new(f32::INFINITY).unwrap_err(), OarError::InvalidParameter);
    }

    #[gtest]
    fn test_lineargain_validation() {
        expect_ok!(LinearGain::new(0.0));
        expect_ok!(LinearGain::new(1.0));

        expect_eq!(LinearGain::new(f32::NAN).unwrap_err(), OarError::InvalidParameter);
        expect_eq!(LinearGain::new(f32::INFINITY).unwrap_err(), OarError::InvalidParameter);
    }

    #[gtest]
    fn test_milliseconds_validation() {
        expect_ok!(Milliseconds::new(1.0));
        expect_ok!(Milliseconds::new(100.5));

        expect_eq!(Milliseconds::new(0.0).unwrap_err(), OarError::InvalidParameter);
        expect_eq!(Milliseconds::new(-10.0).unwrap_err(), OarError::InvalidParameter);
        expect_eq!(Milliseconds::new(f64::NAN).unwrap_err(), OarError::InvalidParameter);
        expect_eq!(Milliseconds::new(f64::INFINITY).unwrap_err(), OarError::InvalidParameter);
    }

    #[gtest]
    fn test_lineargain_operators() {
        let g1 = LinearGain(0.5);
        let g2 = LinearGain(0.25);

        expect_that!((g1 + g2).0, near(0.75, 1e-6));
        expect_that!((g1 - g2).0, near(0.25, 1e-6));
        expect_that!((g1 * 2.0).0, near(1.0, 1e-6));
    }

    #[gtest]
    fn test_samples_validation() {
        expect_eq!(Samples::new(0).unwrap_err(), OarError::InvalidParameter);
        expect_ok!(Samples::new(1));
        expect_ok!(Samples::new(i32::MAX as u32));
        expect_eq!(Samples::new(i32::MAX as u32 + 1).unwrap_err(), OarError::InvalidParameter);
    }
}
