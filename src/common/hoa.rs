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

/// High Order Ambisonics (HOA) orders, equivalent of C API `oar_hoa_t`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighOrderAmbisonics {
    /// Zeroth order Ambisonics (ZOA).
    Zoa = 0,
    /// First order Ambisonics (1OA).
    Order1 = 1,
    /// Second order Ambisonics (2OA).
    Order2 = 2,
    /// Third order Ambisonics (3OA).
    Order3 = 3,
    /// Fourth order Ambisonics (4OA).
    Order4 = 4,
}

impl HighOrderAmbisonics {
    /// Returns the number of channels for this HOA order.
    pub fn channels(&self) -> usize {
        match self {
            HighOrderAmbisonics::Zoa => 1,
            HighOrderAmbisonics::Order1 => 4,
            HighOrderAmbisonics::Order2 => 9,
            HighOrderAmbisonics::Order3 => 16,
            HighOrderAmbisonics::Order4 => 25,
        }
    }
}

// ===== Tests =====

#[cfg(test)]
mod test {

    use super::*;
    use googletest::prelude::*;

    #[gtest]
    fn test_oar_hoa_discriminants() {
        expect_that!(HighOrderAmbisonics::Zoa as i32, eq(0));
        expect_that!(HighOrderAmbisonics::Order1 as i32, eq(1));
        expect_that!(HighOrderAmbisonics::Order2 as i32, eq(2));
        expect_that!(HighOrderAmbisonics::Order3 as i32, eq(3));
        expect_that!(HighOrderAmbisonics::Order4 as i32, eq(4));
    }
}
