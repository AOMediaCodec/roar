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

//! Higher-Order Ambisonic (HOA) sound field rotator.
//!
//! Rotates a sound field block by a given target rotation. Used to align the spatial audio
//! scene with the listener's head movements.

use crate::common::definitions::Quaternion;
use crate::renderer::obr::audio_buffer::AudioBuffer;
use crate::renderer::obr::common::ambisonic_utils::{
    get_num_nth_order_periphonic_components, get_num_periphonic_components,
};

const ROTATION_QUANTIZATION_RAD: f32 = 1.0f32.to_radians();
const SLERP_FRAME_INTERVAL: usize = 32;

/// Sound field rotator for Higher-Order Ambisonics.
///
/// Employs recurrence relations to construct block-diagonal rotation matrices mapping input
/// Ambisonic channels to rotated output channels. Supports smooth spherical linear interpolation
/// (slerp) to prevent clicks when the orientation changes.
#[derive(Debug, Clone, PartialEq)]
pub struct AmbisonicRotator {
    /// The ambisonic order of the sound field.
    ambisonic_order: i32,
    /// The current rotation of the sound field to be applied.
    current_rotation: Quaternion,
    /// Rotation matrices for each band of the sound field.
    /// `rotation_matrices[l]` is the (2l + 1) x (2l + 1) sub-matrix for band `l`.
    rotation_matrices: Vec<Vec<f32>>,
    /// Temporary buffer for storing intermediate results.
    scratch_col: Vec<f32>,
}

impl AmbisonicRotator {
    /// Constructs a new `AmbisonicRotator` instance.
    pub fn new(ambisonic_order: i32) -> Self {
        assert!(ambisonic_order >= 1);
        // The maximum sub-matrix size is the number of channels added in the highest order.
        // (e.g. 25 - 16 = 9 for 4th order).
        let max_submatrix_size = get_num_nth_order_periphonic_components(ambisonic_order);
        let mut rotation_matrices = vec![Vec::new(); (ambisonic_order + 1) as usize];

        // Band 0 (order 0) is 1x1 identity.
        rotation_matrices[0] = vec![1.0_f32];

        // Initialize sub-matrices to identity.
        for l in 1..=ambisonic_order {
            let submatrix_size = get_num_nth_order_periphonic_components(l);
            let mut r = vec![0.0_f32; submatrix_size * submatrix_size];
            for i in 0..submatrix_size {
                r[i * submatrix_size + i] = 1.0;
            }
            rotation_matrices[l as usize] = r;
        }

        Self {
            ambisonic_order,
            current_rotation: Quaternion::identity(),
            rotation_matrices,
            scratch_col: vec![0.0; max_submatrix_size],
        }
    }

    /// Performs a smooth in-place rotation (`buffer = R * buffer`) of the sound field buffer.
    pub fn process_in_place(
        &mut self,
        target_rotation: &Quaternion,
        buffer: &mut AudioBuffer,
    ) -> bool {
        let num_channels = get_num_periphonic_components(self.ambisonic_order);
        assert_eq!(buffer.num_channels(), num_channels);

        let identity = Quaternion::identity();
        if self.current_rotation.angular_difference_rad(&identity) < ROTATION_QUANTIZATION_RAD
            && target_rotation.angular_difference_rad(&identity) < ROTATION_QUANTIZATION_RAD
        {
            return false;
        }

        let num_frames = buffer.num_frames();
        if self.current_rotation.angular_difference_rad(target_rotation) < ROTATION_QUANTIZATION_RAD
        {
            self.multiply_channels_in_place(buffer, 0, num_frames);
            return true;
        }

        let mut i = 0;
        while i < num_frames {
            let duration = (num_frames - i).min(SLERP_FRAME_INTERVAL);
            let interpolation_factor = (i + duration) as f32 / num_frames as f32;
            let slerp_rot = self.current_rotation.slerp(interpolation_factor, target_rotation);
            self.update_rotation_matrix(&slerp_rot);
            self.multiply_channels_in_place(buffer, i, duration);
            i += duration;
        }
        self.current_rotation = *target_rotation;
        true
    }

    // TODO(b/525080422): Optimize by avoiding manual indexing in nested loops. Swap loops to
    // process channel-by-channel and use zip iterators to allow auto-vectorization and avoid bounds
    // checks.
    fn multiply_channels_in_place(
        &mut self,
        buffer: &mut AudioBuffer,
        start_frame: usize,
        duration: usize,
    ) {
        // Ambisonic channel 0 is spherically symmetric (omnidirectional) and invariant to rotation.
        // We iterate over channels 1 up to ambisonic_order, multiplying only the rotation matrix
        // for each order (flattened square matrix with side length of 2 * order + 1).
        for order in 1..=self.ambisonic_order as usize {
            let submatrix = &self.rotation_matrices[order];
            let submatrix_size = 2 * order + 1;
            let start_channel = order * order;
            for frame in start_frame..start_frame + duration {
                for row in 0..submatrix_size {
                    let mut sum = 0.0_f32;
                    let row_offset = row * submatrix_size;
                    for col in 0..submatrix_size {
                        sum += submatrix[row_offset + col] * buffer[start_channel + col][frame];
                    }
                    self.scratch_col[row] = sum;
                }
                for row in 0..submatrix_size {
                    buffer.channel_mut(start_channel + row)[frame] = self.scratch_col[row];
                }
            }
        }
    }

    /// Updates the rotation matrix using the supplied `Quaternion`.
    pub fn update_rotation_matrix(&mut self, rotation: &Quaternion) {
        // Convert from ADM object coordinates (X=right, Y=forward, Z=up) to AmbiX (X=front, Y=left,
        // Z=up).
        let ambix_rotation =
            Quaternion { w: rotation.w, x: -rotation.x, y: rotation.z, z: rotation.y };
        let r_3x3 = ambix_rotation.to_rotation_matrix();

        // Band 1 (order 1) submatrix.
        let r1 = &mut self.rotation_matrices[1];
        for r in 0..3 {
            for c in 0..3 {
                r1[r * 3 + c] = r_3x3[r][c];
            }
        }

        for current_order in 2..=self.ambisonic_order {
            self.compute_band_rotation(current_order);
        }
    }

    fn compute_band_rotation(&mut self, l: i32) {
        let size = (2 * l + 1) as usize;
        let (prev_matrices, target_slice) = self.rotation_matrices.split_at_mut(l as usize);
        let target = &mut target_slice[0];

        for m in -l..=l {
            for n in -l..=l {
                let (u_coeff, v_coeff, w_coeff) = compute_uvw_coeff(m, n, l);
                let mut term_u = 0.0_f32;
                let mut term_v = 0.0_f32;
                let mut term_w = 0.0_f32;
                if u_coeff.abs() > 0.0 {
                    term_u = u_coeff * u_func(m, n, l, prev_matrices);
                }
                if v_coeff.abs() > 0.0 {
                    term_v = v_coeff * v_func(m, n, l, prev_matrices);
                }
                if w_coeff.abs() > 0.0 {
                    term_w = w_coeff * w_func(m, n, l, prev_matrices);
                }
                let row = (m + l) as usize;
                let col = (n + l) as usize;
                target[row * size + col] = term_u + term_v + term_w;
            }
        }
    }
}

fn kronecker_delta(i: i32, j: i32) -> f32 {
    if i == j {
        1.0
    } else {
        0.0
    }
}

fn get_centered_element(r: &[f32], band: i32, i: i32, j: i32) -> f32 {
    let row = (i + band) as usize;
    let col = (j + band) as usize;
    let stride = (2 * band + 1) as usize;
    r[row * stride + col]
}

fn p_func(i: i32, a: i32, b: i32, l: i32, r: &[Vec<f32>]) -> f32 {
    let r1 = &r[1];
    let r_prev = &r[(l - 1) as usize];
    if b == l {
        get_centered_element(r1, 1, i, 1) * get_centered_element(r_prev, l - 1, a, l - 1)
            - get_centered_element(r1, 1, i, -1) * get_centered_element(r_prev, l - 1, a, -l + 1)
    } else if b == -l {
        get_centered_element(r1, 1, i, 1) * get_centered_element(r_prev, l - 1, a, -l + 1)
            + get_centered_element(r1, 1, i, -1) * get_centered_element(r_prev, l - 1, a, l - 1)
    } else {
        get_centered_element(r1, 1, i, 0) * get_centered_element(r_prev, l - 1, a, b)
    }
}

fn u_func(m: i32, n: i32, l: i32, r: &[Vec<f32>]) -> f32 {
    p_func(0, m, n, l, r)
}

fn v_func(m: i32, n: i32, l: i32, r: &[Vec<f32>]) -> f32 {
    if m == 0 {
        p_func(1, 1, n, l, r) + p_func(-1, -1, n, l, r)
    } else if m > 0 {
        let d = kronecker_delta(m, 1);
        p_func(1, m - 1, n, l, r) * (1.0 + d).sqrt() - p_func(-1, -m + 1, n, l, r) * (1.0 - d)
    } else {
        let d = kronecker_delta(m, -1);
        p_func(1, m + 1, n, l, r) * (1.0 - d) + p_func(-1, -m - 1, n, l, r) * (1.0 + d).sqrt()
    }
}

fn w_func(m: i32, n: i32, l: i32, r: &[Vec<f32>]) -> f32 {
    if m == 0 {
        0.0
    } else if m > 0 {
        p_func(1, m + 1, n, l, r) + p_func(-1, -m - 1, n, l, r)
    } else {
        p_func(1, m - 1, n, l, r) - p_func(-1, -m + 1, n, l, r)
    }
}

fn compute_uvw_coeff(m: i32, n: i32, l: i32) -> (f32, f32, f32) {
    let d = kronecker_delta(m, 0);
    let denom =
        if n.abs() == l { (2 * l * (2 * l - 1)) as f32 } else { ((l + n) * (l - n)) as f32 };
    let one_over_denom = 1.0 / denom;

    let u_coeff = (((l + m) * (l - m)) as f32 * one_over_denom).sqrt();
    let v_coeff = 0.5
        * ((1.0 + d) * (l + m.abs() - 1) as f32 * (l + m.abs()) as f32 * one_over_denom).sqrt()
        * (1.0 - 2.0 * d);
    let w_coeff =
        -0.5 * (((l - m.abs() - 1) * (l - m.abs())) as f32 * one_over_denom).sqrt() * (1.0 - d);
    (u_coeff, v_coeff, w_coeff)
}

// ===== Tests =====

#[cfg(test)]
mod test {

    use super::*;
    use crate::common::definitions::{Degrees, Distance, LinearGain};
    use crate::renderer::obr::ambisonic_encoder::AmbisonicEncoder;
    use googletest::prelude::*;

    const AMBISONIC_ORDER: i32 = 3;
    const ANGLE_DEGREES: f32 = 90.0;
    const SLERP_FRAME_INTERVAL: usize = 32;
    const EPSILON: f32 = 1e-4;

    fn from_angle_axis(angle_rad: f32, axis: [f32; 3]) -> Quaternion {
        let half_angle = angle_rad * 0.5;
        let s = half_angle.sin();
        let c = half_angle.cos();
        let norm = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
        if norm > 0.0 {
            let x = axis[0] / norm * s;
            let y = axis[1] / norm * s;
            let z = axis[2] / norm * s;
            Quaternion { w: c, x, y, z }
        } else {
            Quaternion::identity()
        }
    }

    const INITIAL_SOURCE_ANGLE: (f32, f32) = (22.0, 33.0);
    const X_ROTATED_SOURCE_ANGLE: (f32, f32) = (150.02178, 51.04152);
    const Y_ROTATED_SOURCE_ANGLE: (f32, f32) = (-35.00773, 18.310807);
    const Z_ROTATED_SOURCE_ANGLE: (f32, f32) = (112.0, 33.0);

    fn compare_rotated_and_reference_soundfields(
        input_data: &[f32],
        rotation_angle_deg: f32,
        rotation_axis: [f32; 3],
        expected_source_angle: (f32, f32),
    ) {
        let buffer_size = input_data.len();
        let mut input_buffer = AudioBuffer::new(1, buffer_size);
        input_buffer.channel_mut(0).as_mut_slice().copy_from_slice(input_data);

        let mut rotated_source_mono_codec = AmbisonicEncoder::new(1, AMBISONIC_ORDER as usize);
        rotated_source_mono_codec.set_source(
            0,
            LinearGain(1.0),
            Degrees(INITIAL_SOURCE_ANGLE.0),
            Degrees(INITIAL_SOURCE_ANGLE.1),
            Distance::new(1.0).unwrap(),
        );

        let mut reference_source_mono_codec = AmbisonicEncoder::new(1, AMBISONIC_ORDER as usize);
        reference_source_mono_codec.set_source(
            0,
            LinearGain(1.0),
            Degrees(expected_source_angle.0),
            Degrees(expected_source_angle.1),
            Distance::new(1.0).unwrap(),
        );

        let num_channels = get_num_periphonic_components(AMBISONIC_ORDER);
        let mut encoded_rotated_buffer = AudioBuffer::new(num_channels, buffer_size);
        let mut encoded_reference_buffer = AudioBuffer::new(num_channels, buffer_size);

        rotated_source_mono_codec
            .process_planar_audio_data(&input_buffer, &mut encoded_rotated_buffer);
        reference_source_mono_codec
            .process_planar_audio_data(&input_buffer, &mut encoded_reference_buffer);

        let rotation = from_angle_axis(rotation_angle_deg.to_radians(), rotation_axis);

        let mut rotator = AmbisonicRotator::new(AMBISONIC_ORDER);
        let result = rotator.process_in_place(&rotation, &mut encoded_rotated_buffer);
        expect_true!(result);

        for channel in 0..num_channels {
            let num_frames = encoded_rotated_buffer.num_frames();
            let interval = SLERP_FRAME_INTERVAL;
            let frames_to_compare =
                if !num_frames.is_multiple_of(interval) { num_frames % interval } else { interval };
            let start_frame = num_frames.saturating_sub(frames_to_compare);

            for frame in start_frame..num_frames {
                expect_that!(
                    encoded_rotated_buffer[channel][frame],
                    near(encoded_reference_buffer[channel][frame], EPSILON)
                );
            }
        }
    }

    #[gtest]
    fn test_rotation_threshold() {
        let num_channels = 16;
        let frames_per_buffer = 16;
        let mut buffer = AudioBuffer::new(num_channels, frames_per_buffer);
        for ch in 0..num_channels {
            buffer.channel_mut(ch).as_mut_slice().fill(1.0);
        }

        let small_rotation = Quaternion { w: 1.0, x: 0.001, y: 0.001, z: 0.001 };
        let large_rotation = Quaternion { w: 1.0, x: 0.1, y: 0.1, z: 0.1 };

        let mut rotator = AmbisonicRotator::new(AMBISONIC_ORDER);

        expect_false!(rotator.process_in_place(&small_rotation, &mut buffer));
        expect_true!(rotator.process_in_place(&large_rotation, &mut buffer));
    }

    #[gtest]
    fn test_axes_rotation_long_buffer() {
        let input_data = vec![1.0; 512];
        compare_rotated_and_reference_soundfields(
            &input_data,
            ANGLE_DEGREES,
            [1.0, 0.0, 0.0],
            X_ROTATED_SOURCE_ANGLE,
        );
        compare_rotated_and_reference_soundfields(
            &input_data,
            ANGLE_DEGREES,
            [0.0, 1.0, 0.0],
            Y_ROTATED_SOURCE_ANGLE,
        );
        compare_rotated_and_reference_soundfields(
            &input_data,
            ANGLE_DEGREES,
            [0.0, 0.0, 1.0],
            Z_ROTATED_SOURCE_ANGLE,
        );
    }

    #[gtest]
    fn test_axes_rotation_short_buffer() {
        let input_data = vec![1.0; SLERP_FRAME_INTERVAL / 2];
        compare_rotated_and_reference_soundfields(
            &input_data,
            ANGLE_DEGREES,
            [1.0, 0.0, 0.0],
            X_ROTATED_SOURCE_ANGLE,
        );
        compare_rotated_and_reference_soundfields(
            &input_data,
            ANGLE_DEGREES,
            [0.0, 1.0, 0.0],
            Y_ROTATED_SOURCE_ANGLE,
        );
        compare_rotated_and_reference_soundfields(
            &input_data,
            ANGLE_DEGREES,
            [0.0, 0.0, 1.0],
            Z_ROTATED_SOURCE_ANGLE,
        );
    }

    #[gtest]
    fn test_axes_rotation_odd_buffer_size() {
        let input_data = vec![1.0; SLERP_FRAME_INTERVAL + 3];
        compare_rotated_and_reference_soundfields(
            &input_data,
            ANGLE_DEGREES,
            [1.0, 0.0, 0.0],
            X_ROTATED_SOURCE_ANGLE,
        );
        compare_rotated_and_reference_soundfields(
            &input_data,
            ANGLE_DEGREES,
            [0.0, 1.0, 0.0],
            Y_ROTATED_SOURCE_ANGLE,
        );
        compare_rotated_and_reference_soundfields(
            &input_data,
            ANGLE_DEGREES,
            [0.0, 0.0, 1.0],
            Z_ROTATED_SOURCE_ANGLE,
        );
    }

    #[gtest]
    fn test_channel_0_remains_invariant_under_rotation() {
        let order = 3;
        let num_channels = get_num_periphonic_components(order);
        let num_frames = 64;
        let rotation = from_angle_axis(45.0f32.to_radians(), [0.0, 1.0, 0.0]);
        let mut buffer = AudioBuffer::new(num_channels, num_frames);
        buffer.channel_mut(0).as_mut_slice().fill(1.5);
        let mut rotator = AmbisonicRotator::new(order);

        rotator.process_in_place(&rotation, &mut buffer);

        expect_that!(buffer[0], each(eq(&1.5)));
        for ch in 1..num_channels {
            expect_that!(buffer[ch], each(eq(&0.0)));
        }
    }

    #[gtest]
    fn test_channels_1_to_3_conserve_energy_and_rotate_without_leakage() {
        let order = 3;
        let num_channels = get_num_periphonic_components(order);
        let num_frames = 64;
        let rotation = from_angle_axis(45.0f32.to_radians(), [0.0, 1.0, 0.0]);
        let mut buffer = AudioBuffer::new(num_channels, num_frames);
        buffer.channel_mut(1).as_mut_slice().fill(1.0);
        buffer.channel_mut(2).as_mut_slice().fill(2.0);
        buffer.channel_mut(3).as_mut_slice().fill(3.0);
        let mut rotator = AmbisonicRotator::new(order);

        rotator.process_in_place(&rotation, &mut buffer);

        expect_that!(buffer[0], each(eq(&0.0)));
        for ch in 4..num_channels {
            expect_that!(buffer[ch], each(eq(&0.0)));
        }
        // Energy is conserved within order 1 under 3D rotation (1.0^2 + 2.0^2 + 3.0^2 = 14.0).
        for ((&c1, &c2), &c3) in buffer[1].iter().zip(&buffer[2]).zip(&buffer[3]) {
            let energy = c1 * c1 + c2 * c2 + c3 * c3;
            expect_that!(energy, near(14.0, EPSILON));
        }
        // The rotation axis has an orthogonal component to channels 1 and 2, causing them
        // to rotate away from their initial values.
        expect_that!(buffer[1], each(not(near(1.0, EPSILON))));
        expect_that!(buffer[2], each(not(near(2.0, EPSILON))));
    }
}
