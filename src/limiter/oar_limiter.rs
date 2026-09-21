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

//! High-performance dynamics peak limiter implementation.
//!
//! This module implements a point-by-point instantaneous attack peak limiter designed
//! for high-performance real-time audio block processing. It is guaranteed to perform
//! zero heap allocations during real-time processing blocks (`#alloc` safe).

use crate::common::audio_buffer::simd_utils;
use crate::common::definitions::{
    Decibels, LinearGain, Milliseconds, OarError, PlanarBufferMut, SampleRate, Samples,
};

// The decay time constant of -3.0 corresponds to the time to decay to e^-3 (~0.05 or -26 dB,
// which represents 95% settling time).
const DECAY_TIME_CONSTANT: f64 = -3.0;
const MS_PER_SECOND: f64 = 1000.0;

/// High-performance look-ahead dynamics peak limiter with zero dynamic allocation inside real-time
/// block loops.  Exactly mirrors `oar_limiter_t` in `liboar/src/limiter/oar_limiter.h`.
#[derive(Debug, Clone, PartialEq)]
pub struct OarLimiter {
    /// Audio sample rate.
    sample_rate: SampleRate,
    /// Threshold ceiling value as a linear amplitude scale.
    ceiling: f64,
    /// Release time constant coefficient.
    release_time_constant: f64,
    /// Currently active envelope gain scale value.
    env: f64,
    /// Scratch buffer for peaks to avoid stack/heap allocation during render.
    peaks: Vec<f32>,
}

impl OarLimiter {
    /// Creates a new `OarLimiter` with sampling rate, release time, and ceiling.
    ///
    /// # Arguments
    /// * `sample_rate` - The audio sampling rate.
    /// * `release_time` - The release time constant.
    /// * `ceiling_db` - The limiter threshold ceiling.
    /// * `block_size` - The maximum block size for the scratch buffer.
    pub fn new(
        sample_rate: SampleRate,
        release_time: Milliseconds,
        ceiling_db: Decibels,
        block_size: Samples,
    ) -> Result<Self, OarError> {
        let linear_gain: LinearGain = ceiling_db.into();
        let ceiling = linear_gain.0 as f64;

        let release_time_constant = (DECAY_TIME_CONSTANT
            / (sample_rate.value() as f64 * release_time.0 / MS_PER_SECOND))
            .exp();

        Ok(Self {
            sample_rate,
            ceiling,
            release_time_constant,
            env: 1.0,
            peaks: vec![0.0f32; block_size.value() as usize],
        })
    }

    /// Processes multi-channel planar sample buffers in-place, applying peak limiting and envelope
    /// decay. Guaranteed to perform zero heap allocations during real-time block loops.
    ///
    /// # Arguments
    /// * `buffer` - A mutable reference to the planar audio buffer to process.
    ///
    /// # Errors
    /// Returns `Err(OarError::InvalidParameter)` if `buffer` has no channels
    /// or contains zero sample frames, matching the behaviour of `oar_limiter_process`.
    pub fn process_block(&mut self, buffer: &mut PlanarBufferMut<'_, '_>) -> Result<(), OarError> {
        let num_channels = buffer.num_channels();
        if num_channels == 0 {
            return Err(OarError::InvalidParameter);
        }
        let samples_count = buffer.num_samples();
        if samples_count != self.peaks.len() {
            return Err(OarError::InvalidParameter);
        }

        self.peaks.fill(0.0);
        let peaks_slice = &mut self.peaks;

        for ch_idx in 0..num_channels {
            let ch = buffer.channel_mut(ch_idx);
            simd_utils::pointwise_max_abs(peaks_slice, ch);
        }

        let ceiling = self.ceiling;
        let release_time_constant = self.release_time_constant;
        let mut env = self.env;
        for p in peaks_slice.iter_mut() {
            let sample_peak = *p as f64;
            let max_req_gain = if sample_peak > ceiling { ceiling / sample_peak } else { 1.0 };

            env = if max_req_gain < env {
                max_req_gain
            } else {
                release_time_constant * (env - max_req_gain) + max_req_gain
            };
            *p = env as f32;
        }
        self.env = env;

        for ch_idx in 0..num_channels {
            let ch = buffer.channel_mut(ch_idx);
            simd_utils::multiply_pointwise_in_place(ch, peaks_slice);
        }

        Ok(())
    }
}

// ===== Tests =====

#[cfg(test)]
mod test {

    use super::*;
    use crate::common::definitions::AudioBuffer;
    use googletest::prelude::*;

    const BLOCK_SIZE: usize = 480;
    const EPSILON: f32 = 1e-5;

    /// Helper to wrap an `AudioBuffer` in a `PlanarBufferMut` for the test.
    fn wrap_audio_buffer<'a, 'b>(
        buf: &'a mut AudioBuffer,
        slices: &'b mut [&'a mut [f32]],
    ) -> PlanarBufferMut<'a, 'b> {
        let chans = buf.num_channels();
        let samples = buf.num_frames();
        buf.as_slices_mut(slices);
        PlanarBufferMut::new(&mut slices[..chans], chans, Samples::new(samples as u32).unwrap())
            .unwrap()
    }

    /// Helper to make a limiter for the tests.
    fn make_limiter() -> OarLimiter {
        OarLimiter::new(
            SampleRate::new(48000).unwrap(),
            Milliseconds::new(100.0).unwrap(),
            Decibels::new(0.0).unwrap(),
            Samples::new(BLOCK_SIZE as u32).unwrap(),
        )
        .unwrap()
    }

    #[gtest]
    fn process_block_attenuates_peak_above_ceiling() {
        let mut limiter = make_limiter();
        let mut signal = AudioBuffer::new(2, BLOCK_SIZE);
        signal.fill(2.0);

        let mut slices: [&mut [f32]; 2] = std::array::from_fn(|_| &mut [] as &mut [f32]);
        let mut buf = wrap_audio_buffer(&mut signal, &mut slices);
        limiter.process_block(&mut buf).unwrap();

        let threshold_limit = 1.0 + EPSILON;
        for ch_idx in 0..signal.num_channels() {
            let ch = signal.channel(ch_idx);
            for &sample in ch.as_slice().iter() {
                expect_that!(sample, le(threshold_limit));
            }
        }
    }

    #[gtest]
    fn process_block_recovers_gain_gradually_on_quiet_signal() {
        let mut limiter = make_limiter();
        let mut signal = AudioBuffer::new(2, BLOCK_SIZE);
        signal.fill(2.0);

        let mut slices1: [&mut [f32]; 2] = std::array::from_fn(|_| &mut [] as &mut [f32]);
        let mut buf1 = wrap_audio_buffer(&mut signal, &mut slices1);
        limiter.process_block(&mut buf1).unwrap();
        // Feed one quiet block and check that it is attenuated.
        let mut quiet_block = AudioBuffer::new(2, BLOCK_SIZE);
        quiet_block.fill(0.1);
        let mut slices2: [&mut [f32]; 2] = std::array::from_fn(|_| &mut [] as &mut [f32]);
        let mut buf2 = wrap_audio_buffer(&mut quiet_block, &mut slices2);
        limiter.process_block(&mut buf2).unwrap();

        // Output should be attenuated because limiter env is still low.
        for ch in 0..2 {
            for &sample in quiet_block.channel(ch).as_slice().iter() {
                expect_that!(sample, lt(0.09)); // attenuated from 0.1
            }
        }

        // Feed many more quiet blocks to allow recovery.
        for _ in 0..20 {
            quiet_block.fill(0.1);
            let mut slices2: [&mut [f32]; 2] = std::array::from_fn(|_| &mut [] as &mut [f32]);
            let mut buf2 = wrap_audio_buffer(&mut quiet_block, &mut slices2);
            limiter.process_block(&mut buf2).unwrap();
        }

        // Output should be recovered close to 0.1.
        for ch in 0..2 {
            for &sample in quiet_block.channel(ch).as_slice().iter() {
                expect_that!(sample, gt(0.099)); // recovered close to 0.1
            }
        }
    }

    #[gtest]
    fn process_block_keeps_sub_threshold_signal_unchanged() {
        let mut limiter = make_limiter();
        let mut signal = AudioBuffer::new(2, BLOCK_SIZE);
        signal.fill(0.5);
        let original_data = signal.to_2d();

        let mut slices: [&mut [f32]; 2] = std::array::from_fn(|_| &mut [] as &mut [f32]);
        let mut buf = wrap_audio_buffer(&mut signal, &mut slices);
        limiter.process_block(&mut buf).unwrap();

        expect_eq!(signal.to_2d(), original_data);
    }

    #[gtest]
    fn process_block_returns_invalid_parameter_for_empty_channels() {
        let mut limiter = make_limiter();
        let mut empty_buffer = AudioBuffer::new(0, BLOCK_SIZE);

        let mut slices = [];
        let mut buf = wrap_audio_buffer(&mut empty_buffer, &mut slices);
        let result = limiter.process_block(&mut buf);

        expect_eq!(result.unwrap_err(), OarError::InvalidParameter);
    }

    #[gtest]
    fn process_block_returns_invalid_parameter_for_block_size_exceeding_maximum() {
        let mut limiter = make_limiter();
        let large_block_size = BLOCK_SIZE + 1;
        let mut large_buffer = AudioBuffer::new(2, large_block_size);

        let mut slices: [&mut [f32]; 2] = std::array::from_fn(|_| &mut [] as &mut [f32]);
        let mut buf = wrap_audio_buffer(&mut large_buffer, &mut slices);
        let result = limiter.process_block(&mut buf);

        expect_eq!(result.unwrap_err(), OarError::InvalidParameter);
    }

    #[gtest]
    fn new_returns_invalid_parameter_for_zero_or_negative_values() {
        let sample_rate_err = SampleRate::new(0);
        let ms_zero_err = Milliseconds::new(0.0);
        let ms_neg_err = Milliseconds::new(-5.0);

        expect_eq!(sample_rate_err.unwrap_err(), OarError::InvalidParameter);
        expect_eq!(ms_zero_err.unwrap_err(), OarError::InvalidParameter);
        expect_eq!(ms_neg_err.unwrap_err(), OarError::InvalidParameter);
    }
}
