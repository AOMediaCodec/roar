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

//! Envelope-tracking true-peak signal limiter.
//!
//! Provides the DSP logic for dynamic range compression and peak signal limiting
//! used on OBR stereo binaural outputs.

use crate::common::audio_buffer::AudioBuffer;
use crate::common::definitions::{Decibels, LinearGain, Milliseconds, SampleRate, Samples};
use crate::renderer::obr::common::constants::{
    PEAK_LIMITER_DEFAULT_CEILING, PEAK_LIMITER_DEFAULT_RELEASE_TIME,
};

/// Release log-decay target in nepers (`-3 Np` => `e^-3 ≈ 0.05`, or ~26 dB / 95% settling).
/// Expresses how much the signal should decay during the `release_time` contructor argument.
const RELEASE_DECAY_NEPERS: f64 = -3.0;

/// Envelope-tracking true-peak audio signal limiter.
///
/// Designed to prevent clipping and signal distortion. Operates in real-time
/// without performing dynamic memory allocations on the process loop.
#[derive(Debug, Clone, PartialEq)]
pub struct PeakLimiter {
    /// Audio sample rate in Hz.
    sample_rate: SampleRate,
    /// Configured output signal ceiling level.
    ceiling: f64,
    /// The per-sample decay of the signal during the release.
    release_coeff: f64,
    /// Current envelope level, updated during processing.
    env: f64,
    /// Scratch work buffer for peak sample values per channel.
    max_samples: Vec<f32>,
    /// Scratch work buffer for limiter envelope values per channel.
    limiter_env: Vec<f32>,
}

impl PeakLimiter {
    /// Constructs a new `PeakLimiter` instance.
    ///
    /// # Parameters
    /// * `sample_rate` - Audio sample rate.
    /// * `release_time` - Limiter release time.
    /// * `ceiling_db` - Output signal ceiling level.
    /// * `block_size` - Number of audio frames per processing block.
    pub fn new(
        sample_rate: SampleRate,
        release_time: Milliseconds,
        ceiling_db: Decibels,
        block_size: Samples,
    ) -> Self {
        let linear_gain: LinearGain = ceiling_db.into();
        let ceiling = linear_gain.value() as f64;
        let release_samples = sample_rate * release_time;
        let release_coeff = (RELEASE_DECAY_NEPERS / release_samples.value() as f64).exp();
        let num_frames = block_size.value() as usize;
        Self {
            sample_rate,
            ceiling,
            release_coeff,
            env: 1.0,
            max_samples: vec![0.0; num_frames],
            limiter_env: vec![0.0; num_frames],
        }
    }

    /// Constructs a new `PeakLimiter` instance with default parameters.
    ///
    /// # Parameters
    /// * `sample_rate` - Audio sample rate.
    /// * `block_size` - Number of audio frames per processing block.
    pub fn new_with_defaults(sample_rate: SampleRate, block_size: Samples) -> Self {
        Self::new(
            sample_rate,
            PEAK_LIMITER_DEFAULT_RELEASE_TIME,
            PEAK_LIMITER_DEFAULT_CEILING,
            block_size,
        )
    }

    /// Processes the input audio block and applies peak limiting to `output`.
    ///
    /// Safe for execution on real-time threads. Buffer capacities must match.
    pub fn process(&mut self, input: &AudioBuffer, output: &mut AudioBuffer) {
        assert_eq!(input.num_channels(), output.num_channels());
        assert_eq!(input.num_frames(), output.num_frames());
        let num_channels = input.num_channels();
        let num_frames = input.num_frames();
        assert_eq!(num_frames, self.max_samples.len());

        // Populate `max_samples` across all channels without allocating new heap memory.
        self.max_samples.fill(0.0);
        for c in 0..num_channels {
            let channel = input.channel(c);
            assert!(channel.enabled);
            // TODO(b/525080422): Optimize by avoiding manual indexing in loops to allow compiler
            // auto-vectorization and avoid bounds checks (e.g. use zip iterators).
            for frame in 0..num_frames {
                self.max_samples[frame] = self.max_samples[frame].max(channel[frame].abs());
            }
        }

        // Calculate Limiter envelope.
        for frame in 0..num_frames {
            let max_req_gain = self.get_maximum_required_gain(self.max_samples[frame] as f64);
            if max_req_gain < self.env {
                self.env = max_req_gain;
            } else {
                self.env = self.release_coeff * (self.env - max_req_gain) + max_req_gain;
            }
            self.limiter_env[frame] = self.env as f32;
        }

        // Apply the limiter envelope to the input buffer and write to output.
        for c in 0..num_channels {
            let in_ch = input.channel(c);
            let mut out_ch = output.channel_mut(c);
            // TODO(b/525080422): Optimize by avoiding manual indexing in loops to allow compiler
            // auto-vectorization and avoid bounds checks (e.g. use zip iterators).
            for frame in 0..num_frames {
                out_ch[frame] = in_ch[frame] * self.limiter_env[frame];
            }
        }
    }

    /// Returns the maximum gain required to limit the peak.
    fn get_maximum_required_gain(&self, sample: f64) -> f64 {
        let abs_sample = sample.abs();
        if abs_sample > self.ceiling {
            self.ceiling / abs_sample
        } else {
            1.0
        }
    }
}

// ===== Tests =====

#[cfg(test)]
mod test {

    use super::*;
    use crate::common::audio_buffer::AudioBuffer;
    use crate::common::definitions::{Decibels, Milliseconds, SampleRate, Samples};
    use googletest::prelude::*;

    #[gtest]
    fn test_peak_limiter_no_attenuation_below_ceiling() {
        let sample_rate = SampleRate::new(48000).unwrap();
        // Ceiling at -3 dB (~0.707 linear)
        let ceiling = Decibels::new(-3.0).unwrap();
        let release = Milliseconds::new(100.0).unwrap();
        let block_size = Samples::new(4).unwrap();
        let mut limiter = PeakLimiter::new(sample_rate, release, ceiling, block_size);

        // Input signal peak at 0.5 (below 0.707)
        let mut input = AudioBuffer::new(2, 4);
        input.channel_mut(0).as_mut_slice().copy_from_slice(&[0.1, 0.5, -0.3, 0.2]);
        input.channel_mut(1).as_mut_slice().copy_from_slice(&[-0.2, 0.4, 0.5, -0.1]);

        let mut output = AudioBuffer::new(2, 4);
        limiter.process(&input, &mut output);

        // Output should be exactly equal to input
        expect_near!(output.channel(0)[0], 0.1, 1e-6);
        expect_near!(output.channel(0)[1], 0.5, 1e-6);
        expect_near!(output.channel(1)[2], 0.5, 1e-6);
    }

    #[gtest]
    fn test_peak_limiter_attenuates_above_ceiling() {
        let sample_rate = SampleRate::new(48000).unwrap();
        // Ceiling at -6 dB (~0.501 linear)
        let ceiling = Decibels::new(-6.0).unwrap();
        let release = Milliseconds::new(100.0).unwrap();
        let block_size = Samples::new(4).unwrap();
        let mut limiter = PeakLimiter::new(sample_rate, release, ceiling, block_size);

        // Input peak at 1.0 (above ~0.501)
        let mut input = AudioBuffer::new(1, 4);
        input.channel_mut(0).as_mut_slice().copy_from_slice(&[0.1, 1.0, 0.5, 0.1]);

        let mut output = AudioBuffer::new(1, 4);
        limiter.process(&input, &mut output);

        // Output at peak should be restricted to ceiling (~0.501)
        let ceiling_linear = 10.0f64.powf(-6.0 / 20.0) as f32;
        expect_near!(output.channel(0)[1], ceiling_linear, 1e-4);
        // And subsequent samples should be attenuated because envelope release is slow
        expect_lt!(output.channel(0)[2], 0.5);
    }
}
