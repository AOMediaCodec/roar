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

//! Matrix rendering module for EAR spatialization.
//!
//! Provides channel multiplication by transformation matrices without heap allocation.
//! This is a direct Rust port of `liboar/src/renderer/ear/arch/x86/matrix_render_x86.c`
//! and `liboar/src/renderer/ear/matrix_render.c`, serving as the exact functional
//! equivalent to the C reference.

use crate::common::audio_buffer::simd_utils;
use crate::common::definitions::{
    OarError, PlanarBufferMut, PlanarBufferRef, MAX_OUTPUT_CHANNEL_COUNT,
};

/// Describes the striding and mapping layout of a transformation matrix.
pub struct MatrixView<'a> {
    /// Flattened matrix weights indexed by `out_idx * out_stride + in_mapped_idx * in_stride`.
    pub data: &'a [f32],
    /// Number of input channels to process (`inputs.len()`).
    pub in_dim: usize,
    /// Stride in `data` corresponding to moving to the next input channel column (`in_next`).
    pub in_stride: usize,
    /// Number of output channels to process (`outputs.len()`).
    pub out_dim: usize,
    /// Stride in `data` corresponding to moving to the next output channel row (`out_next`).
    pub out_stride: usize,
    /// Optional slice mapping physical input channel index to logical matrix column index.
    pub in_idx_map: Option<&'a [usize]>,
}

/// Multiplies planar input channels by a transformation matrix and accumulates into output channels.
///
/// Port of `multiply_channels_by_matrix_c` from `matrix_render_x86.c`.
pub fn multiply_channels_by_matrix_c(
    matrix: &MatrixView<'_>,
    inputs: PlanarBufferRef<'_, '_>,
    outputs: &mut PlanarBufferMut<'_, '_>,
) -> Result<(), OarError> {
    if matrix.in_dim == 0 || matrix.out_dim == 0 {
        return Ok(());
    }
    if inputs.num_channels() < matrix.in_dim || outputs.num_channels() < matrix.out_dim {
        return Err(OarError::InvalidParameter);
    }
    if let Some(map) = matrix.in_idx_map
        && map.len() < matrix.in_dim
    {
        return Err(OarError::InvalidParameter);
    }
    if outputs.num_samples() != inputs.num_samples() {
        return Err(OarError::InvalidParameter);
    }

    let num_out_channels = outputs.num_channels();
    let mut out_slices: [&mut [f32]; MAX_OUTPUT_CHANNEL_COUNT] =
        std::array::from_fn(|_| &mut [] as &mut [f32]);
    if num_out_channels > MAX_OUTPUT_CHANNEL_COUNT {
        return Err(OarError::InvalidParameter);
    }
    outputs.as_slices_mut(&mut out_slices[..num_out_channels]);

    // Initialize outputs to 0.0
    for out_slice in out_slices.iter_mut().take(matrix.out_dim) {
        out_slice.fill(0.0);
    }

    for in_idx in 0..matrix.in_dim {
        let in_slice = inputs.channel(in_idx);
        let in_mapped_idx = match matrix.in_idx_map {
            Some(map) => *map.get(in_idx).ok_or(OarError::InvalidParameter)?,
            None => in_idx,
        };

        for (out_idx, out_slice) in out_slices.iter_mut().enumerate().take(matrix.out_dim) {
            let mat_idx = out_idx * matrix.out_stride + in_mapped_idx * matrix.in_stride;
            let c = match matrix.data.get(mat_idx) {
                Some(&val) => val,
                None => continue,
            };
            simd_utils::scalar_multiply_and_accumulate(c, in_slice, out_slice);
        }
    }
    Ok(())
}

/// Dispatches multi-channel matrix rendering to the underlying implementation.
///
/// Port of `matrix_render` from `liboar/src/renderer/ear/matrix_render.c`.
/// Currently delegates directly to [`multiply_channels_by_matrix_c`] as SIMD
/// optimizations (`NEON`/`x86`) are bypassed for initial porting.
///
/// # Parameters
///
/// * `matrix`: Configuration descriptor describing matrix weights, mapping, and striding dimensions.
/// * `inputs`: Validated read-only view of planar input channels.
/// * `outputs`: Validated mutable view of planar output channels to be accumulated or overwritten.
pub fn matrix_render(
    matrix: &MatrixView<'_>,
    inputs: PlanarBufferRef<'_, '_>,
    outputs: &mut PlanarBufferMut<'_, '_>,
) -> Result<(), OarError> {
    multiply_channels_by_matrix_c(matrix, inputs, outputs)
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::units::Samples;
    use googletest::prelude::*;

    #[gtest]
    fn multiply_channels_by_matrix_multiple_inputs_no_map_clears_first_and_accumulates_second() {
        let mat = [1.0f32, 2.0f32, 3.0f32, 4.0f32];
        let in_left = [10.0f32, 20.0f32];
        let in_right = [5.0f32, 10.0f32];
        let mut out_left = [999.0f32, 999.0f32];
        let mut out_right = [888.0f32, 888.0f32];
        let inputs = [&in_left[..], &in_right[..]];
        let mut out_slices = [&mut out_left[..], &mut out_right[..]];
        let inputs_buf = PlanarBufferRef::new(&inputs, 2, Samples(2)).unwrap();
        let mut outputs_buf = PlanarBufferMut::new(&mut out_slices, 2, Samples(2)).unwrap();

        let mv = MatrixView {
            data: &mat,
            in_dim: 2,
            in_stride: 1,
            out_dim: 2,
            out_stride: 2,
            in_idx_map: None,
        };

        let result = multiply_channels_by_matrix_c(&mv, inputs_buf, &mut outputs_buf);

        expect_ok!(result);
        expect_that!(out_left[0], eq(20.0f32));
        expect_that!(out_left[1], eq(40.0f32));
        expect_that!(out_right[0], eq(50.0f32));
        expect_that!(out_right[1], eq(100.0f32));
    }

    #[gtest]
    fn multiply_channels_by_matrix_with_channel_map_maps_physical_input_to_logical_column() {
        let mat = [10.0f32, 20.0f32];
        let map = [1usize];
        let in_channel = [2.0f32];
        let mut out_channel = [100.0f32];
        let inputs = [&in_channel[..]];
        let mut out_slices = [&mut out_channel[..]];
        let inputs_buf = PlanarBufferRef::new(&inputs, 1, Samples(1)).unwrap();
        let mut outputs_buf = PlanarBufferMut::new(&mut out_slices, 1, Samples(1)).unwrap();

        let mv = MatrixView {
            data: &mat,
            in_dim: 1,
            in_stride: 1,
            out_dim: 1,
            out_stride: 2,
            in_idx_map: Some(&map),
        };

        let result = multiply_channels_by_matrix_c(&mv, inputs_buf, &mut outputs_buf);

        expect_ok!(result);
        expect_that!(out_channel[0], eq(40.0f32));
    }

    #[gtest]
    fn multiply_channels_by_matrix_zero_dimensions_does_not_modify_output() {
        let mat = [1.0f32];
        let in_channel = [10.0f32];
        let mut out_channel = [42.0f32];
        let inputs = [&in_channel[..]];
        let mut out_slices = [&mut out_channel[..]];
        let inputs_buf = PlanarBufferRef::new(&inputs, 1, Samples(1)).unwrap();
        let mut outputs_buf = PlanarBufferMut::new(&mut out_slices, 1, Samples(1)).unwrap();

        let mv = MatrixView {
            data: &mat,
            in_dim: 0,
            in_stride: 1,
            out_dim: 1,
            out_stride: 1,
            in_idx_map: None,
        };

        let result = multiply_channels_by_matrix_c(&mv, inputs_buf, &mut outputs_buf);

        expect_ok!(result);
        expect_that!(out_channel[0], eq(42.0f32));
    }

    #[gtest]
    fn multiply_channels_by_matrix_processes_expected_samples() {
        let mat = [2.0f32];
        let in_channel = [3.0f32, 4.0f32];
        let mut out_channel = [0.0f32; 2];
        let inputs = [&in_channel[..]];
        let mut out_slices = [&mut out_channel[..]];
        let inputs_buf = PlanarBufferRef::new(&inputs, 1, Samples(2)).unwrap();
        let mut outputs_buf = PlanarBufferMut::new(&mut out_slices, 1, Samples(2)).unwrap();

        let mv = MatrixView {
            data: &mat,
            in_dim: 1,
            in_stride: 1,
            out_dim: 1,
            out_stride: 1,
            in_idx_map: None,
        };

        let result = multiply_channels_by_matrix_c(&mv, inputs_buf, &mut outputs_buf);

        expect_ok!(result);
        expect_that!(out_channel[0], eq(6.0f32));
        expect_that!(out_channel[1], eq(8.0f32));
    }

    #[gtest]
    fn matrix_render_dispatches_to_accumulation_produces_expected_scaled_outputs() {
        let mat = [3.0f32, -1.0f32];
        let in_first = [4.0f32];
        let in_second = [2.0f32];
        let mut out_channel = [0.0f32];
        let inputs = [&in_first[..], &in_second[..]];
        let mut out_slices = [&mut out_channel[..]];
        let inputs_buf = PlanarBufferRef::new(&inputs, 2, Samples(1)).unwrap();
        let mut outputs_buf = PlanarBufferMut::new(&mut out_slices, 1, Samples(1)).unwrap();

        let mv = MatrixView {
            data: &mat,
            in_dim: 2,
            in_stride: 1,
            out_dim: 1,
            out_stride: 2,
            in_idx_map: None,
        };

        let result = matrix_render(&mv, inputs_buf, &mut outputs_buf);

        expect_ok!(result);
        expect_that!(out_channel[0], eq(10.0f32));
    }
}
