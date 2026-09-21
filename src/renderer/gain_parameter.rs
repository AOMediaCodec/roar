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

use crate::common::definitions::{Samples, METADATA_QUEUE_CAPACITY};
use crate::common::gains::GainVariant;
use std::collections::VecDeque;

/// Represents a queued gain change event in the renderer's DSP timeline.
///
/// A `GainUpdate` is created from a user-requested `Gain` metadata update
/// and a target duration. It tracks the progress of the gain transition
/// (whether constant, multi-channel, or animated) as audio samples are processed.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GainUpdate {
    pub gain: GainVariant,
    pub duration: Samples,
    pub elapsed: u32,
}

/// Manages the live rendering state and pending update queue for a single gain parameter.
///
/// Each gain control (e.g., a specific element's gain or a group's loudness gain)
/// has an associated `GainParameter`. It queues incoming `GainUpdate`s and
/// ensures smooth transitions between them during rendering.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GainParameter {
    pub id: u32,
    pub updates: VecDeque<GainUpdate>,
    pub last_value: f32, // holds linear gain
}

impl GainParameter {
    /// Creates a new `GainParameter` with a default gain of 1.0 (0 dB) and an empty update queue.
    pub fn new(id: u32) -> Self {
        GainParameter {
            id,
            updates: VecDeque::with_capacity(METADATA_QUEUE_CAPACITY),
            last_value: 1.0, // default gain is 1.0 (0 dB)
        }
    }
}

/// Generates a slice of gain multipliers for a block of audio samples by consuming
/// or progressing the updates in the `GainParameter`.
///
/// This function interpolates animated gains or applies constant gains across the
/// requested number of `samples`, updating the `GainParameter`'s internal progress
/// and popping finished updates from its queue.
///
/// # Arguments
/// * `param` - The active gain parameter state to progress.
/// * `samples` - The number of samples to generate multipliers for.
/// * `out_multipliers` - The output slice to fill with the calculated gain multipliers.
// TODO(b/525080422): Avoid manual indexing in the gain multiplier generation loops (e.g. using
// out_multipliers[idx + i]) to allow compiler auto-vectorization and avoid bounds checks.
pub(crate) fn generate_gain_multipliers(
    param: &mut GainParameter,
    samples: usize,
    out_multipliers: &mut [f32],
) {
    let mut idx = 0;
    while idx < samples {
        if let Some(update) = param.updates.front_mut() {
            let dur = u32::from(update.duration);
            let remaining_in_update = (dur - update.elapsed) as usize;
            let count = remaining_in_update.min(samples - idx);

            match &update.gain {
                GainVariant::Constant(g) => {
                    if count > 0 {
                        out_multipliers[idx..idx + count].fill(g.0);
                        param.last_value = g.0;
                    }
                }
                GainVariant::Multiple(g_arr) => {
                    let start = update.elapsed as usize;
                    for i in 0..count {
                        out_multipliers[idx + i] = g_arr[start + i].0;
                    }
                    if count > 0 {
                        param.last_value = g_arr[start + count - 1].0;
                    }
                }
                GainVariant::Animated(anim) => {
                    for i in 0..count {
                        let t = (update.elapsed + i as u32) as f32 / dur as f32;
                        let g = anim.sample(t);
                        out_multipliers[idx + i] = g.0;
                        param.last_value = g.0;
                    }
                }
            }
            // TODO(b/525080422): Refactor the update progression loop to use iterators/chunks
            // instead of manual index (`idx`) and elapsed tracking to improve readability and
            // safety.
            update.elapsed += count as u32;
            idx += count;
            let need_pop = update.elapsed >= dur;
            if need_pop {
                param.updates.pop_front();
            }
        } else {
            // Queue empty, hold last value
            out_multipliers[idx..samples].fill(param.last_value);
            break;
        }
    }
}

// ===== Tests =====

#[cfg(test)]
mod test {

    use super::*;
    use crate::common::animation::Animated;
    use crate::common::units::LinearGain;
    use googletest::prelude::*;

    #[gtest]
    fn test_new_parameter() {
        let param = GainParameter::new(42);

        expect_eq!(param.id, 42);
        expect_true!(param.updates.is_empty());
        expect_eq!(param.last_value, 1.0);
    }

    #[gtest]
    fn test_generate_multipliers_empty_queue() {
        let mut param = GainParameter::new(42);
        param.last_value = 0.5;
        let mut multipliers = vec![0.0f32; 10];

        generate_gain_multipliers(&mut param, 10, &mut multipliers);

        expect_eq!(multipliers, vec![0.5f32; 10]);
        expect_eq!(param.last_value, 0.5);
    }

    #[gtest]
    fn test_generate_multipliers_constant_gain() {
        let mut param = GainParameter::new(42);
        param.updates.push_back(GainUpdate {
            gain: GainVariant::Constant(LinearGain(0.8)),
            duration: Samples(5),
            elapsed: 0,
        });
        let mut multipliers = vec![0.0f32; 10];

        generate_gain_multipliers(&mut param, 10, &mut multipliers);

        // First 5 samples should be 0.8, remaining should hold last_value (0.8)
        expect_eq!(multipliers, vec![0.8f32; 10]);
        expect_true!(param.updates.is_empty());
        expect_eq!(param.last_value, 0.8);
    }

    #[gtest]
    fn test_generate_multipliers_constant_gain_split() {
        let mut param = GainParameter::new(42);
        param.updates.push_back(GainUpdate {
            gain: GainVariant::Constant(LinearGain(0.8)),
            duration: Samples(5),
            elapsed: 0,
        });
        param.updates.push_back(GainUpdate {
            gain: GainVariant::Constant(LinearGain(0.3)),
            duration: Samples(5),
            elapsed: 0,
        });
        let mut multipliers = vec![0.0f32; 8];

        generate_gain_multipliers(&mut param, 8, &mut multipliers);

        // First 5 samples should be 0.8, next 3 should be 0.3
        expect_eq!(multipliers, vec![0.8, 0.8, 0.8, 0.8, 0.8, 0.3, 0.3, 0.3]);
        expect_eq!(param.updates.len(), 1);
        expect_eq!(param.updates[0].elapsed, 3);
        expect_eq!(param.last_value, 0.3);
    }

    #[gtest]
    fn test_generate_multipliers_multiple_gains() {
        let mut param = GainParameter::new(42);
        param.updates.push_back(GainUpdate {
            gain: GainVariant::Multiple(vec![LinearGain(0.1), LinearGain(0.2), LinearGain(0.3)]),
            duration: Samples(3),
            elapsed: 0,
        });
        let mut multipliers = vec![0.0f32; 5];

        generate_gain_multipliers(&mut param, 5, &mut multipliers);

        expect_eq!(multipliers, vec![0.1, 0.2, 0.3, 0.3, 0.3]);
        expect_true!(param.updates.is_empty());
        expect_eq!(param.last_value, 0.3);
    }

    #[gtest]
    fn test_generate_multipliers_animated_linear() {
        let mut param = GainParameter::new(42);
        param.updates.push_back(GainUpdate {
            gain: GainVariant::Animated(Animated::Linear {
                start: LinearGain(0.0),
                end: LinearGain(1.0),
            }),
            duration: Samples(5),
            elapsed: 0,
        });
        let mut multipliers = vec![0.0f32; 7];

        generate_gain_multipliers(&mut param, 7, &mut multipliers);

        // Duration is 5.
        // i=0: t=0.0 -> 0.0
        // i=1: t=0.2 -> 0.2
        // i=2: t=0.4 -> 0.4
        // i=3: t=0.6 -> 0.6
        // i=4: t=0.8 -> 0.8
        // i=5..7: hold last_value (0.8)
        expect_eq!(multipliers, vec![0.0, 0.2, 0.4, 0.6, 0.8, 0.8, 0.8]);
    }
}
