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

//! Planar (separated by channel) and interleaved (different channels combined) buffer conversion.
//!
//! Provides routines to translate between interleaved sample buffers (used in external systems)
//! and internal planar representations managed by `AudioBuffer`.

use crate::renderer::obr::audio_buffer::simd_utils::float_from_int16;
use crate::renderer::obr::audio_buffer::AudioBuffer;

/// Copies interleaved 16-bit PCM samples into a planar `AudioBuffer`.
pub fn fill_audio_buffer_from_i16(
    interleaved_buffer: &[i16],
    num_input_frames: usize,
    num_input_channels: usize,
    output: &mut AudioBuffer,
) {
    if output.num_channels() != num_input_channels || output.num_frames() != num_input_frames {
        output.resize(num_input_channels, num_input_frames);
    }
    assert!(interleaved_buffer.len() >= num_input_frames * num_input_channels);

    // Convert from int16 and scatter across channels.
    let mut temp = vec![0.0_f32; num_input_frames * num_input_channels];
    float_from_int16(&interleaved_buffer[..temp.len()], &mut temp);

    // TODO(b/525080422): Optimize by avoiding manual indexing in nested loops. Use step_by and zip
    // iterators to allow auto-vectorization.
    for frame in 0..num_input_frames {
        for ch in 0..num_input_channels {
            output.channel_mut(ch)[frame] = temp[frame * num_input_channels + ch];
        }
    }
}
