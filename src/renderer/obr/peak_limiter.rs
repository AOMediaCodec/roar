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

use crate::common::definitions::{Decibels, LinearGain, Milliseconds, SampleRate};
use crate::renderer::obr::audio_buffer::AudioBuffer;
use crate::renderer::obr::common::constants::{
    PEAK_LIMITER_DEFAULT_CEILING, PEAK_LIMITER_DEFAULT_RELEASE_TIME,
};

// TODO(b/525080422): Move these constants to definitions.rs, use Decibels and Milliseconds types.
const DEFAULT_THRESHOLD_DB: f64 = -3.0;
const MS_PER_SECOND: f64 = 1000.0;

/// Envelope-tracking true-peak audio signal limiter.
///
/// Designed to prevent clipping and signal distortion. Operates in real-time
/// without performing dynamic memory allocations on the process loop.
#[derive(Debug, Clone, PartialEq)]
pub struct PeakLimiter {
    sampling_rate: SampleRate,
    ceiling: f64,
    release_time_constant: f64,
    env: f64,
    // Pre-allocated scratch work buffers guaranteeing zero dynamic allocation inside `process()`.
    max_samples: Vec<f32>,
    limiter_env: Vec<f32>,
}

impl PeakLimiter {
    /// Constructs a new `PeakLimiter` instance.
    ///
    /// # Parameters
    /// * `sampling_rate` - Audio sample rate.
    /// * `release_time` - Limiter release time.
    /// * `ceiling_db` - Output signal ceiling level.
    pub fn new(
        sampling_rate: SampleRate,
        release_time: Milliseconds,
        ceiling_db: Decibels,
    ) -> Self {
        let linear_gain: LinearGain = ceiling_db.into();
        let ceiling = linear_gain.0 as f64;
        let release_time_constant = (DEFAULT_THRESHOLD_DB
            / (sampling_rate.value() as f64 * release_time.0 / MS_PER_SECOND))
            .exp();
        Self {
            sampling_rate,
            ceiling,
            release_time_constant,
            env: 1.0,
            max_samples: Vec::with_capacity(1024),
            limiter_env: Vec::with_capacity(1024),
        }
    }

    /// Constructs a new `PeakLimiter` instance with default parameters.
    ///
    /// # Parameters
    /// * `sampling_rate` - Audio sample rate.
    pub fn new_with_defaults(sampling_rate: SampleRate) -> Self {
        Self::new(sampling_rate, PEAK_LIMITER_DEFAULT_RELEASE_TIME, PEAK_LIMITER_DEFAULT_CEILING)
    }

    /// Processes the input audio block and applies peak limiting to `output`.
    ///
    /// Safe for execution on real-time threads. Buffer capacities must match.
    pub fn process(&mut self, input: &AudioBuffer, output: &mut AudioBuffer) {
        assert_eq!(input.num_channels(), output.num_channels());
        assert_eq!(input.num_frames(), output.num_frames());
        let num_channels = input.num_channels();
        let num_frames = input.num_frames();

        // Populate `max_samples` across all channels without allocating new heap memory.
        self.max_samples.clear();
        self.max_samples.resize(num_frames, 0.0);
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
        self.limiter_env.clear();
        self.limiter_env.resize(num_frames, 0.0);
        for frame in 0..num_frames {
            let max_req_gain = self.get_maximum_required_gain(self.max_samples[frame] as f64);
            if max_req_gain < self.env {
                self.env = max_req_gain;
            } else {
                self.env = self.release_time_constant * (self.env - max_req_gain) + max_req_gain;
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
    use crate::common::definitions::{Decibels, Milliseconds, SampleRate};
    use crate::renderer::obr::audio_buffer::AudioBuffer;
    use googletest::prelude::*;

    #[gtest]
    fn test_peak_limiter_no_attenuation_below_ceiling() {
        let sample_rate = SampleRate::new(48000).unwrap();
        // Ceiling at -3 dB (~0.707 linear)
        let ceiling = Decibels(-3.0);
        let release = Milliseconds(100.0);
        let mut limiter = PeakLimiter::new(sample_rate, release, ceiling);

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
        let ceiling = Decibels(-6.0);
        let release = Milliseconds(100.0);
        let mut limiter = PeakLimiter::new(sample_rate, release, ceiling);

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
