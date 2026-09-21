// Copyright (c) 2026, Alliance for Open Media. All rights reserved
//
// This source code is subject to the terms of the BSD 3-Clause Clear License
// and the Alliance for Open Media Patent License 1.0. If the BSD 3-Clause Clear
// License was not distributed with this source code in the LICENSE file, you
// can obtain it at www.aomedia.org/license/software-license/bsd-3-c-c. If the
// Alliance for Open Media Patent License 1.0 was not distributed with this
// source code in the PATENTS file, you can obtain it at
// www.aomedia.org/license/patent.

//! C API for Rust Open Audio Renderer (ROAR)
//!
//! All `#[unsafe(no_mangle)] extern "C"` functions live here and nowhere else.
//! Every function is a thin dispatcher that validates raw pointer arguments,
//! delegates to pure-safe functions, and converts `Result` back to C-style error codes.
//! All functions wrap their execution inside `std::panic::catch_unwind`.

use crate::c_types::{
    oar_audio_block_t, oar_audio_element_config_t, oar_config_t, oar_metadata_t,
    oar_metadata_type_t, oar_t, FfiWrapper,
};
use crate::common::definitions::{
    Decibels, GroupId, OarError, PlanarBufferMut, PlanarBufferRef, Samples,
    MAX_OUTPUT_CHANNEL_COUNT, OAR_STATUS_OK,
};
use std::os::raw::c_int;

/// Creates a new OAR renderer object.
///
/// Returns a pointer to the created `oar_t` object on success, or `null_mut()` on failure.
///
/// # Safety
///
/// The caller must guarantee that:
/// - `config` points to a valid, initialized `oar_config_t` structure.
/// - The pointer is safe to read for the duration of this function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roar_create(config: *const oar_config_t) -> *mut oar_t {
    std::panic::catch_unwind(|| {
        if config.is_null() {
            return std::ptr::null_mut();
        }
        // SAFETY: We checked that `config` is not null. The caller guarantees that `config`
        // points to a valid, initialized `oar_config_t` structure.
        let config_ref = unsafe { &*config };
        let oar_config = match crate::common::definitions::Config::try_from(*config_ref) {
            Ok(c) => c,
            Err(_) => return std::ptr::null_mut(),
        };
        let renderer = match crate::renderer::RoarRenderer::create(&oar_config) {
            Ok(r) => r,
            Err(_) => return std::ptr::null_mut(),
        };
        let wrapper = FfiWrapper { renderer, inputs: std::collections::HashMap::new() };
        // Box the wrapper and return it as an `oar_t` pointer.
        Box::into_raw(Box::new(wrapper)) as *mut oar_t
    })
    .unwrap_or(std::ptr::null_mut())
}

/// Destroys an OAR renderer object.
///
/// # Safety
///
/// The caller must guarantee that:
/// - `oar` points to a valid OAR object returned by `oar_create`.
/// - The object has not been destroyed already and is not accessed concurrently.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roar_destroy(oar: *mut oar_t) {
    let _ = std::panic::catch_unwind(|| {
        if oar.is_null() {
            return;
        }
        // SAFETY: The caller guarantees `oar` is a valid pointer returned by `roar_create`
        // and has not been destroyed yet.
        unsafe {
            // Cast the oar_t to its real type, FfiWrapper, and reconstruct the Box for deallocation
            let _ = Box::from_raw(oar as *mut FfiWrapper);
        }
    });
}

/// Adds a new audio group to the renderer.
///
/// Returns the group ID (0 or 1) on success, or a negative error code (e.g. `-22` for invalid
/// param, `-12` for out of memory, `-16` if group limit reached) on failure.
///
/// # Safety
///
/// The caller must guarantee that:
/// - `oar` points to a valid, initialized OAR object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roar_add_audio_group(oar: *mut oar_t) -> c_int {
    std::panic::catch_unwind(|| {
        if oar.is_null() {
            return OarError::InvalidParameter.into();
        }
        // SAFETY: The caller guarantees `oar` points to a valid FfiWrapper.
        let wrapper = unsafe { &mut *(oar as *mut FfiWrapper) };
        match wrapper.renderer.add_audio_group() {
            Ok(new_id) => new_id as c_int,
            Err(e) => e.into(),
        }
    })
    .unwrap_or(OarError::InvalidParameter.into())
}

/// Adds a new audio element to the renderer.
///
/// Returns `OAR_STATUS_OK` (0) on success, or a negative error code (e.g. `-22` for invalid param,
/// `-12` for out of memory) on failure.
///
/// # Safety
///
/// The caller must guarantee that:
/// - `oar` points to a valid, initialized OAR object.
/// - `config` points to a valid, initialized `oar_audio_element_config_t` structure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roar_add_audio_element(
    oar: *mut oar_t,
    gid: u32,
    id: u32,
    config: *const oar_audio_element_config_t,
) -> c_int {
    std::panic::catch_unwind(|| {
        if oar.is_null() || config.is_null() {
            return OarError::InvalidParameter.into();
        }
        // SAFETY: The caller guarantees `oar` points to a valid FfiWrapper.
        let wrapper = unsafe { &mut *(oar as *mut FfiWrapper) };
        // SAFETY: We checked that `config` is not null. The caller guarantees that `config`
        // points to a valid, initialized `oar_audio_element_config_t` structure.
        let config_ref = unsafe { &*config };
        let safe_config = match config_ref.to_safe() {
            Ok(c) => c,
            Err(e) => return e.into(),
        };

        let group_id = match GroupId::try_from(gid) {
            Ok(id) => id,
            Err(e) => return e.into(),
        };

        match wrapper.renderer.add_element(group_id, id, &safe_config) {
            Ok(()) => OAR_STATUS_OK,
            Err(e) => e.into(),
        }
    })
    .unwrap_or(OarError::InvalidParameter.into())
}

/// Removes an audio element from the renderer.
///
/// Returns `OAR_STATUS_OK` (0) on success, or a negative error code (e.g. `-22` if element not
/// found or invalid param) on failure.
///
/// # Safety
///
/// The caller must guarantee that:
/// - `oar` points to a valid, initialized OAR object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roar_remove_audio_element(oar: *mut oar_t, id: u32) -> c_int {
    std::panic::catch_unwind(|| {
        if oar.is_null() {
            return OarError::InvalidParameter.into();
        }
        // SAFETY: The caller guarantees `oar` points to a valid FfiWrapper.
        let wrapper = unsafe { &mut *(oar as *mut FfiWrapper) };
        match wrapper.renderer.remove_element(id) {
            Ok(()) => {
                wrapper.inputs.remove(&id);
                OAR_STATUS_OK
            }
            Err(e) => e.into(),
        }
    })
    .unwrap_or(OarError::InvalidParameter.into())
}

/// Updates metadata parameters for an audio element.
///
/// Supported metadata types:
/// - **Object positions**
///   - Duration must be specified.  A value <= 0 means the update will be silently dropped.
/// - **Gain** (`oar_metadata_gain_t`): See the struct for details.
///   - Duration must be specified.  A value <= 0 means the update will be silently dropped.
/// - **IAMF downmix mode** (`oar_metadata_object_positions_t): See the struct for details.
///   - Duration is optional.  A value <= 0 means the update will persist indefinitely (until a new
///     update is provided).
///
/// To update head rotation (which is always applied globally), use `roar_update_metadata` instead.
///
/// Returns `OAR_STATUS_OK` (0) on success, or a negative error code (e.g. `-22` for invalid param,
/// `-95` for unsupported metadata type) on failure.
///
/// # Safety
///
/// The caller must guarantee that:
/// - `oar` points to a valid, initialized OAR object.
/// - `metadata` points to a valid, initialized `oar_metadata_t` structure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roar_update_audio_element_metadata(
    oar: *mut oar_t,
    id: u32,
    metadata: *const oar_metadata_t,
) -> c_int {
    std::panic::catch_unwind(|| {
        if oar.is_null() || metadata.is_null() {
            return OarError::InvalidParameter.into();
        }
        // SAFETY: The caller guarantees `oar` points to a valid FfiWrapper.
        let wrapper = unsafe { &mut *(oar as *mut FfiWrapper) };
        // SAFETY: We checked that `metadata` is not null. The caller guarantees that `metadata`
        // points to a valid, initialized `oar_metadata_t` structure.
        let meta_ref = unsafe { &*metadata };
        let meta_type = meta_ref.r#type;

        if meta_type == (oar_metadata_type_t::ck_metadata_gain as u32) {
            // C liboar ignores gain updates with duration <= 0.
            if meta_ref.duration <= 0 {
                return OAR_STATUS_OK;
            }
            let duration = match Samples::new(meta_ref.duration as u32) {
                Ok(s) => s,
                Err(e) => return e.into(),
            };
            let gain = match meta_ref.to_gain() {
                Ok(g) => g,
                Err(e) => return e.into(),
            };
            // SAFETY: The caller guarantees that metadata is valid the type of metadata indicates
            // it is a gain.
            let gain_meta = unsafe { meta_ref.value.gain };
            let gain_id = gain_meta.id;
            match wrapper.renderer.update_element_gain(id, gain_id, &gain, duration) {
                Ok(()) => OAR_STATUS_OK,
                Err(e) => e.into(),
            }
        } else if meta_type == (oar_metadata_type_t::ck_metadata_object_positions as u32) {
            // C liboar ignores position updates with duration <= 0.
            if meta_ref.duration <= 0 {
                return OAR_STATUS_OK;
            }
            let duration = match Samples::new(meta_ref.duration as u32) {
                Ok(s) => s,
                Err(e) => return e.into(),
            };
            let positions = match meta_ref.to_positions() {
                Ok(p) => p,
                Err(e) => return e.into(),
            };
            match wrapper.renderer.update_element_positions(id, &positions, duration) {
                Ok(()) => OAR_STATUS_OK,
                Err(e) => e.into(),
            }
        } else if meta_type == (oar_metadata_type_t::ck_metadata_iamf_downmix_mode as u32) {
            // C liboar treats downmix mode updates with duration <= 0 as infinite
            // (block-persisting).
            let duration = if meta_ref.duration <= 0 {
                None
            } else {
                match Samples::new(meta_ref.duration as u32) {
                    Ok(s) => Some(s),
                    Err(e) => return e.into(),
                }
            };
            let mode = match meta_ref.to_downmix_mode() {
                Ok(m) => m,
                Err(e) => return e.into(),
            };
            match wrapper.renderer.update_element_downmix_mode(id, mode, duration) {
                Ok(()) => OAR_STATUS_OK,
                Err(e) => e.into(),
            }
        } else {
            OarError::NotSupported.into()
        }
    })
    .unwrap_or(OarError::InvalidParameter.into())
}

/// Updates audio element data blocks for rendering.
///
/// Returns `OAR_STATUS_OK` (0) on success, or a negative error code (e.g. `-22` for invalid param /
/// channels count) on failure.
///
/// # Safety
///
/// The caller must guarantee that:
/// - `oar` points to a valid, initialized OAR object.
/// - `data` points to a valid `oar_audio_block_t` structure containing active planar f32 input
///   buffers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roar_update_audio_element_data(
    oar: *mut oar_t,
    id: u32,
    data: *mut oar_audio_block_t,
) -> c_int {
    std::panic::catch_unwind(|| {
        if oar.is_null() || data.is_null() {
            return OarError::InvalidParameter.into();
        }
        // SAFETY: The caller guarantees `oar` points to a valid FfiWrapper.
        let wrapper = unsafe { &mut *(oar as *mut FfiWrapper) };

        let expected_channels =
            wrapper.renderer.active_elements().get(&id).map(|c| c.channels()).unwrap_or(0);

        if expected_channels == 0 {
            return OarError::InvalidParameter.into();
        }

        // SAFETY: We checked that `data` is not null. The caller guarantees that `data`
        // points to a valid `oar_audio_block_t` structure containing active planar f32 input
        // buffers, which ensures `as_slices()` can safely dereference the inner data pointers.
        let data_slices = unsafe { (*data).as_slices() };
        if data_slices.len() != expected_channels {
            return OarError::InvalidParameter.into();
        }

        // TODO(b/525080422): Could we reuse the previously allocated memory (if applicable) rather
        // than reallocating?
        let copied_data: Vec<Vec<f32>> = data_slices.iter().map(|slice| slice.to_vec()).collect();

        wrapper.inputs.insert(id, copied_data);
        OAR_STATUS_OK
    })
    .unwrap_or(OarError::InvalidParameter.into())
}

/// Sets the metadata unit processing size in samples.
///
/// Returns `OAR_STATUS_OK` (0) on success, or a negative error code (e.g. `-22` for invalid param,
/// `-95` for unsupported metadata type) on failure.
///
/// Matching liboar, only object positions metadata type is supported.
///
/// # Safety
///
/// The caller must guarantee that:
/// - `oar` points to a valid, initialized OAR object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roar_set_metadata_unit_to_process(
    oar: *mut oar_t,
    r#type: oar_metadata_type_t,
    samples: u32,
) -> c_int {
    std::panic::catch_unwind(|| {
        if oar.is_null() {
            return OarError::InvalidParameter.into();
        }
        if r#type != oar_metadata_type_t::ck_metadata_object_positions {
            return OarError::NotSupported.into();
        }
        // SAFETY: The caller guarantees `oar` points to a valid FfiWrapper.
        let wrapper = unsafe { &mut *(oar as *mut FfiWrapper) };
        match wrapper.renderer.set_object_position_metadata_unit_to_process(samples) {
            Ok(()) => OAR_STATUS_OK,
            Err(e) => e.into(),
        }
    })
    .unwrap_or(OarError::InvalidParameter.into())
}

/// Updates metadata parameters for an audio group.
///
/// Supported metadata and value constraints:
/// - **Head rotation** (`ck_metadata_head_rotation`):
///   - Quaternion `w, x, y, z` representing head orientation.
///   - Expected to be a unit quaternion (sum of squares is 1.0).
///   - Uses the ADM object coordinate system to orient the listener's head.
///   - Note: Duration is not used.
/// - **Gain** (`ck_metadata_gain`):
///   - Gain in dB, applied to all channels in the group.
///   - Duration must be specified.  A value <= 0 means the update will be silently dropped!
///
/// Returns `OAR_STATUS_OK` (0) on success, or a negative error code (e.g. `-22` for invalid param,
/// `-16` if renderer not in Rendering state, `-95` for unsupported metadata type) on failure.
///
/// # Safety
///
/// The caller must guarantee that:
/// - `oar` points to a valid, initialized OAR object.
/// - `metadata` points to a valid, initialized `oar_metadata_t` structure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roar_update_metadata(
    oar: *mut oar_t,
    gid: u32,
    metadata: *const oar_metadata_t,
) -> c_int {
    std::panic::catch_unwind(|| {
        if oar.is_null() || metadata.is_null() {
            return OarError::InvalidParameter.into();
        }
        // SAFETY: The caller guarantees `oar` points to a valid FfiWrapper.
        let wrapper = unsafe { &mut *(oar as *mut FfiWrapper) };
        // SAFETY: We checked that `metadata` is not null. The caller guarantees that `metadata`
        // points to a valid, initialized `oar_metadata_t` structure.
        let meta_ref = unsafe { &*metadata };

        let group_id = match GroupId::try_from(gid) {
            Ok(id) => id,
            Err(e) => return e.into(),
        };

        if !wrapper.renderer.has_group(group_id) {
            return OarError::InvalidParameter.into();
        }

        if meta_ref.r#type == (oar_metadata_type_t::ck_metadata_gain as u32) {
            // C liboar ignores group gain updates with duration <= 0.
            if meta_ref.duration <= 0 {
                return OAR_STATUS_OK;
            }
            let gain = match meta_ref.to_gain() {
                Ok(g) => g,
                Err(e) => return e.into(),
            };
            let duration = match Samples::new(meta_ref.duration as u32) {
                Ok(s) => s,
                Err(e) => return e.into(),
            };
            match wrapper.renderer.update_group_gain(group_id, &gain, duration) {
                Ok(()) => OAR_STATUS_OK,
                Err(e) => e.into(),
            }
        } else if meta_ref.r#type == (oar_metadata_type_t::ck_metadata_head_rotation as u32) {
            if !wrapper.renderer.head_tracking_enabled {
                // liboar returns `ck_oar_error_busy` if head tracking is disabled.
                return OarError::Busy.into();
            }
            match meta_ref.to_head_rotation() {
                Ok(q) => {
                    if let Err(e) = wrapper.renderer.set_head_rotation(q) {
                        return e.into();
                    }
                }
                Err(e) => return e.into(),
            }
            OAR_STATUS_OK
        } else {
            OarError::NotSupported.into()
        }
    })
    .unwrap_or(OarError::InvalidParameter.into())
}

/// Renders all active audio elements and writes results to planar outputs.
///
/// Returns `OAR_STATUS_OK` (0) on success, or a negative error code (e.g. `-22` for invalid
/// parameters, or sub-renderer errors) on failure.
///
/// # Safety
///
/// The caller must guarantee that:
/// - `oar` points to a valid, initialized OAR object.
/// - `output` points to a valid `oar_audio_block_t` with correct `channels` and `samples_per_channel`
///   and allocated planar float memory.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roar_render(oar: *mut oar_t, output: *mut oar_audio_block_t) -> c_int {
    std::panic::catch_unwind(|| {
        if oar.is_null() || output.is_null() {
            return OarError::InvalidParameter.into();
        }
        // SAFETY: The caller guarantees `oar` points to a valid FfiWrapper.
        let wrapper = unsafe { &mut *(oar as *mut FfiWrapper) };

        // SAFETY: We checked that `output` is not null. The caller guarantees that `output`
        // points to a valid `oar_audio_block_t` structure.
        let output_block = unsafe { &*output };
        if output_block.channels == 0 || output_block.samples_per_channel == 0 {
            return OarError::InvalidParameter.into();
        }

        // TODO(b/525080422):  Could we simplify the preparation of memory for rendering (i.e. the
        // creation the PlanarBufRef), maybe by changing the way we store inside `wrapper.inputs`?
        let collected_inputs: Vec<(&u32, &Vec<Vec<f32>>)> = wrapper.inputs.iter().collect();

        let mut element_slices: Vec<Vec<&[f32]>> = Vec::new();
        for (_, chans) in &collected_inputs {
            let slices: Vec<&[f32]> = chans.iter().map(|c| c.as_slice()).collect();
            element_slices.push(slices);
        }

        let mut input_refs: Vec<(u32, PlanarBufferRef<'_, '_>)> = Vec::new();
        for (i, (&id, chans)) in collected_inputs.into_iter().enumerate() {
            let slices_ref = match PlanarBufferRef::new(
                element_slices[i].as_slice(),
                chans.len(),
                wrapper.renderer.config().samples_per_channel(),
            ) {
                Ok(r) => r,
                Err(e) => return e.into(),
            };
            input_refs.push((id, slices_ref));
        }

        if wrapper.renderer.active_groups().is_empty() {
            return OarError::InvalidParameter.into();
        }

        // SAFETY: We checked that `output` is not null. The caller guarantees that `output`
        // points to a valid `oar_audio_block_t` structure with allocated planar float memory,
        // which ensures `as_slices_mut()` can safely construct mutable slices.
        let dest_slices = unsafe { (*output).as_slices_mut() };
        let out_channels = output_block.channels as usize;
        if dest_slices.len() < out_channels {
            return OarError::InvalidParameter.into();
        }

        let mut slices_array: [&mut [f32]; MAX_OUTPUT_CHANNEL_COUNT] =
            std::array::from_fn(|_| &mut [] as &mut [f32]);
        if out_channels > MAX_OUTPUT_CHANNEL_COUNT {
            return OarError::InvalidParameter.into();
        }
        for (c, slice) in dest_slices.into_iter().enumerate().take(out_channels) {
            slices_array[c] = unsafe {
                // Safety: MaybeUninit<f32> has same layout as f32,
                // and we initialize it fully during render.
                std::slice::from_raw_parts_mut(slice.as_mut_ptr() as *mut f32, slice.len())
            };
        }

        let mut out_buf = match PlanarBufferMut::new(
            &mut slices_array[..out_channels],
            out_channels,
            wrapper.renderer.config().samples_per_channel(),
        ) {
            Ok(r) => r,
            Err(e) => return e.into(),
        };

        match wrapper.renderer.render(&input_refs, &mut out_buf) {
            Ok(()) => OAR_STATUS_OK,
            Err(e) => e.into(),
        }
    })
    .unwrap_or(OarError::InvalidParameter.into())
}

/// Enables/disables the loudness processor.
///
/// Returns `OAR_STATUS_OK` (0) on success, or a negative error code (e.g. `-22` for invalid param)
/// on failure.
///
/// # Safety
///
/// The caller must guarantee that:
/// - `oar` points to a valid, initialized OAR object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roar_enable_loudness_processor(oar: *mut oar_t, enable: c_int) -> c_int {
    std::panic::catch_unwind(|| {
        if oar.is_null() {
            return OarError::InvalidParameter.into();
        }
        // SAFETY: The caller guarantees `oar` points to a valid FfiWrapper.
        let wrapper = unsafe { &mut *(oar as *mut FfiWrapper) };
        let enable_bool = enable != 0;
        match wrapper.renderer.enable_loudness_processor(enable_bool) {
            Ok(()) => OAR_STATUS_OK,
            Err(e) => e.into(),
        }
    })
    .unwrap_or(OarError::InvalidParameter.into())
}

/// Configures loudness adjustments for a group.
///
/// Returns `OAR_STATUS_OK` (0) on success, or a negative error code (e.g. `-22` for invalid param/
/// group ID) on failure.
///
/// # Arguments
/// * `oar` - Pointer to the ROAR object.
/// * `gid` - The group ID to configure.
/// * `loudness` - The current loudness of the group, in dB.
/// * `target_loudness` - The target loudness for the group, in dB.
///
/// # Safety
///
/// The caller must guarantee that:
/// - `oar` points to a valid, initialized OAR object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roar_set_loudness(
    oar: *mut oar_t,
    gid: u32,
    loudness: f32,
    target_loudness: f32,
) -> c_int {
    std::panic::catch_unwind(|| {
        if oar.is_null() {
            return OarError::InvalidParameter.into();
        }
        // SAFETY: The caller guarantees `oar` points to a valid FfiWrapper.
        let wrapper = unsafe { &mut *(oar as *mut FfiWrapper) };
        let group_id = match GroupId::try_from(gid) {
            Ok(id) => id,
            Err(e) => return e.into(),
        };
        let loudness = match Decibels::new(loudness) {
            Ok(l) => l,
            Err(e) => return e.into(),
        };
        let target_loudness = match Decibels::new(target_loudness) {
            Ok(l) => l,
            Err(e) => return e.into(),
        };
        match wrapper.renderer.set_loudness(group_id, loudness, target_loudness) {
            Ok(()) => OAR_STATUS_OK,
            Err(e) => e.into(),
        }
    })
    .unwrap_or(OarError::InvalidParameter.into())
}

/// Enables/disables the dynamics limiter.
///
/// Returns `OAR_STATUS_OK` (0) on success, or a negative error code (e.g. `-22` for invalid param)
/// on failure.
///
/// # Safety
///
/// The caller must guarantee that:
/// - `oar` points to a valid, initialized OAR object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roar_enable_limiter(oar: *mut oar_t, enable: c_int) -> c_int {
    std::panic::catch_unwind(|| {
        if oar.is_null() {
            return OarError::InvalidParameter.into();
        }
        // SAFETY: The caller guarantees `oar` points to a valid FfiWrapper.
        let wrapper = unsafe { &mut *(oar as *mut FfiWrapper) };
        let enable_bool = enable != 0;
        match wrapper.renderer.enable_limiter(enable_bool) {
            Ok(()) => OAR_STATUS_OK,
            Err(e) => e.into(),
        }
    })
    .unwrap_or(OarError::InvalidParameter.into())
}

/// Enables/disables listener head tracking.
///
/// Returns `OAR_STATUS_OK` (0) on success, or a negative error code (e.g. `-22` for invalid param,
/// `-95` if layout not Binaural) on failure.
///
/// # Safety
///
/// The caller must guarantee that:
/// - `oar` points to a valid, initialized OAR object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roar_enable_head_tracking(oar: *mut oar_t, enable: c_int) -> c_int {
    std::panic::catch_unwind(|| {
        if oar.is_null() {
            return OarError::InvalidParameter.into();
        }
        // SAFETY: The caller guarantees `oar` points to a valid FfiWrapper.
        let wrapper = unsafe { &mut *(oar as *mut FfiWrapper) };
        let enable_bool = enable != 0;
        match wrapper.renderer.enable_head_tracking(enable_bool) {
            Ok(()) => OAR_STATUS_OK,
            Err(e) => e.into(),
        }
    })
    .unwrap_or(OarError::InvalidParameter.into())
}

/// Returns the samples per channel for the renderer.
///
/// Returns the samples per channel value on success, or 0 if `oar` is invalid/null.
///
/// # Safety
///
/// The caller must guarantee that:
/// - `oar` points to a valid, initialized OAR object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roar_get_samples_per_channel(oar: *const oar_t) -> u32 {
    std::panic::catch_unwind(|| {
        if oar.is_null() {
            return 0;
        }
        // SAFETY: The caller guarantees `oar` points to a valid FfiWrapper.
        let wrapper = unsafe { &*(oar as *const FfiWrapper) };
        wrapper.renderer.config().samples_per_channel().value()
    })
    .unwrap_or(0)
}

/// Returns the sampling rate for the renderer.
///
/// Returns the sampling rate value on success, or 0 if `oar` is invalid/null.
///
/// # Safety
///
/// The caller must guarantee that:
/// - `oar` points to a valid, initialized OAR object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roar_get_sampling_rate(oar: *const oar_t) -> u32 {
    std::panic::catch_unwind(|| {
        if oar.is_null() {
            return 0;
        }
        // SAFETY: The caller guarantees `oar` points to a valid FfiWrapper.
        let wrapper = unsafe { &*(oar as *const FfiWrapper) };
        wrapper.renderer.config().sample_rate().value()
    })
    .unwrap_or(0)
}

/// Returns the channel count for a given audio element ID.
///
/// Returns the channel count on success, or 0 if `oar` is invalid/null or element not found.
///
/// # Safety
///
/// The caller must guarantee that:
/// - `oar` points to a valid, initialized OAR object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roar_get_number_of_audio_element_channels(
    oar: *const oar_t,
    id: u32,
) -> u32 {
    std::panic::catch_unwind(|| {
        if oar.is_null() {
            return 0;
        }
        // SAFETY: The caller guarantees `oar` points to a valid FfiWrapper.
        let wrapper = unsafe { &*(oar as *const FfiWrapper) };
        wrapper.renderer.element_channels(id).unwrap_or(0) as u32
    })
    .unwrap_or(0)
}

/// Returns the output channel count for the renderer.
///
/// Returns the output channel count on success, or 0 if `oar` is invalid/null.
///
/// # Safety
///
/// The caller must guarantee that:
/// - `oar` points to a valid, initialized OAR object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roar_get_number_of_output_channels(oar: *const oar_t) -> u32 {
    std::panic::catch_unwind(|| {
        if oar.is_null() {
            return 0;
        }
        // SAFETY: The caller guarantees `oar` points to a valid FfiWrapper.
        let wrapper = unsafe { &*(oar as *const FfiWrapper) };
        wrapper.renderer.config().output_layout().channels() as u32
    })
    .unwrap_or(0)
}

/// Returns the total count of registered audio elements.
///
/// Returns the total element count on success, or 0 if `oar` is invalid/null.
///
/// # Safety
///
/// The caller must guarantee that:
/// - `oar` points to a valid, initialized OAR object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roar_get_number_of_audio_elements(oar: *const oar_t) -> u32 {
    std::panic::catch_unwind(|| {
        if oar.is_null() {
            return 0;
        }
        // SAFETY: The caller guarantees `oar` points to a valid FfiWrapper.
        let wrapper = unsafe { &*(oar as *const FfiWrapper) };
        wrapper.renderer.number_of_audio_elements() as u32
    })
    .unwrap_or(0)
}
