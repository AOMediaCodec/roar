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

//! Top-level OBR (Open Binaural RoarRenderer) orchestrator.
//!
//! This module implements the main entry point and coordinator for the OBR subsystem.
//! It manages audio elements, groups them into processing groups, performs Ambisonic
//! encoding, handles head tracking rotation, performs frequency-domain binaural
//! decoding, and applies a peak limiter to the final output.
//!
//! Crucially, `ObrImpl` is designed to be allocation-free on its real-time audio thread
//! processing loop once initialized.

use super::audio_element_config::{AudioElementConfig, BinauralFilterProfile};
use super::audio_element_type::AudioElementType;
use super::processing_group::{ProcessingGroup, ProcessingGroupKey};
use crate::common::definitions::{
    PlanarBufferMut, PlanarBufferRef, PolarCoordinate, Quaternion, SampleRate, Samples,
};
use crate::renderer::obr::ambisonic_binaural_decoder::{FftManager, Resampler};
use crate::renderer::obr::audio_buffer::simd_utils::add_pointwise_in_place;
use crate::renderer::obr::audio_buffer::AudioBuffer;
use crate::renderer::obr::common::constants::{
    MAX_SUPPORTED_NUM_INPUT_CHANNELS, NUM_BINAURAL_CHANNELS,
};
use crate::renderer::obr::peak_limiter::PeakLimiter;
use std::collections::BTreeMap;

/// Top-level coordinator for the OBR rendering pipeline.
///
/// Orchestrates the process of mapping and rendering multiple configured audio elements.
/// It groups elements by their filter profiles and Ambisonic orders to optimize the number
/// of DSP operations required. Supports real-time updates for object positions and
/// head rotation.
#[derive(Debug)]
pub struct ObrImpl {
    buffer_size_per_channel: Samples,
    sampling_rate: SampleRate,
    head_tracking_enabled: bool,
    limiter_enabled: bool,
    world_rotation: Quaternion,
    audio_elements: Vec<AudioElementConfig>,
    processing_groups: Vec<ProcessingGroup>,
    resampler: Resampler,
    fft_manager: FftManager,
    peak_limiter: Option<PeakLimiter>,
    scratch_group_output: AudioBuffer,
    scratch_limiter_buffer: AudioBuffer,
}

impl ObrImpl {
    /// Constructs a new `ObrImpl` instance.
    ///
    /// # Parameters
    /// * `buffer_size_per_channel` - Number of audio frames per block.
    /// * `sampling_rate` - Audio sample rate.
    pub fn new(buffer_size_per_channel: Samples, sampling_rate: SampleRate) -> Self {
        let size_usize = buffer_size_per_channel.value() as usize;
        Self {
            buffer_size_per_channel,
            sampling_rate,
            head_tracking_enabled: false,
            limiter_enabled: true,
            world_rotation: Quaternion::identity(),
            audio_elements: Vec::new(),
            processing_groups: Vec::new(),
            resampler: Resampler::new(),
            fft_manager: FftManager::new(size_usize),
            peak_limiter: Some(PeakLimiter::new_with_defaults(sampling_rate)),
            scratch_group_output: AudioBuffer::new(2, size_usize),
            scratch_limiter_buffer: AudioBuffer::new(2, size_usize),
        }
    }

    /// Resets all DSP state and clears processing groups.
    pub fn reset_dsp(&mut self) {
        self.processing_groups.clear();
        self.peak_limiter = None;
    }

    /// Re-initialises DSP resources after configuration changes.
    pub fn initialize_dsp(&mut self) -> Result<(), OarError> {
        self.reset_dsp();
        if self.audio_elements.is_empty() {
            return Err(OarError::InvalidParameter);
        }
        let input_channels = self.get_number_of_input_channels();
        if input_channels == 0 {
            return Err(OarError::InvalidParameter);
        }
        self.create_processing_groups();
        for group in &mut self.processing_groups {
            group.initialize(&mut self.fft_manager, &mut self.resampler)?;
            group.update_ambisonic_encoder(&mut self.audio_elements)?;
        }
        self.peak_limiter = Some(PeakLimiter::new_with_defaults(self.sampling_rate));
        Ok(())
    }

    fn create_processing_groups(&mut self) {
        self.processing_groups.clear();
        let mut groups_map: BTreeMap<ProcessingGroupKey, Vec<usize>> = BTreeMap::new();
        for (i, elem) in self.audio_elements.iter().enumerate() {
            if elem.element_type().is_passthrough() {
                continue;
            }
            let key = ProcessingGroupKey {
                ambisonic_order: elem.get_binaural_filters_ambisonic_order(),
                filter_profile: elem.get_binaural_filter_profile(),
            };
            groups_map.entry(key).or_default().push(i);
        }
        for (key, indices) in groups_map {
            self.processing_groups.push(ProcessingGroup::new(
                key,
                indices,
                self.buffer_size_per_channel,
                self.sampling_rate,
            ));
        }
    }

    /// Adds an audio element to the renderer and updates the DSP lifecycle.
    pub fn add_audio_element(
        &mut self,
        element_type: AudioElementType,
        filter_profile: BinauralFilterProfile,
    ) -> Result<(), OarError> {
        let mut cfg = AudioElementConfig::new(element_type, filter_profile);
        if !self.audio_elements.is_empty() {
            let last = self.audio_elements.last().unwrap();
            let first_idx = last.get_first_channel_index() + last.get_number_of_input_channels();
            cfg.set_first_channel_index(first_idx);
        }
        let total_inputs: usize =
            self.audio_elements.iter().map(|e| e.get_number_of_input_channels()).sum::<usize>()
                + cfg.get_number_of_input_channels();
        if total_inputs > MAX_SUPPORTED_NUM_INPUT_CHANNELS {
            return Err(OarError::InvalidParameter);
        }
        self.audio_elements.push(cfg);
        self.initialize_dsp()
    }

    // TODO(b/543004593): Unify the way that audio elements are removed.
    /// Removes the last added audio element and re-initialises DSP resources.
    #[allow(dead_code)]
    pub fn remove_last_audio_element(&mut self) -> Result<(), OarError> {
        if self.audio_elements.is_empty() {
            return Err(OarError::InvalidParameter);
        }
        self.audio_elements.pop();
        if self.audio_elements.is_empty() {
            self.reset_dsp();
            Ok(())
        } else {
            self.initialize_dsp()
        }
    }

    /// Sets the position of an audio object.
    ///
    /// # Parameters
    /// * `audio_element_index` - Index of the audio element.
    /// * `azimuth` - Azimuth angle in degrees.
    /// * `elevation` - Elevation angle in degrees.
    /// * `distance` - Distance from the listener.

    /// Sets the position of a specific channel inside a multi-channel object.
    ///
    /// # Parameters
    /// * `audio_element_index` - Index of the audio element.
    /// * `channel_index` - Channel index within the object.
    /// * `azimuth` - Azimuth angle in degrees.
    /// * `elevation` - Elevation angle in degrees.
    /// * `distance` - Distance from the listener.
    pub fn update_object_channel_position(
        &mut self,
        audio_element_index: usize,
        channel_index: usize,
        azimuth: f32,
        elevation: f32,
        distance: f32,
    ) -> Result<(), OarError> {
        if audio_element_index >= self.audio_elements.len() {
            return Err(OarError::InvalidParameter);
        }
        let obj_chans = self.audio_elements[audio_element_index].get_object_channels();
        if obj_chans.is_empty() || channel_index >= obj_chans.len() {
            return Err(OarError::InvalidParameter);
        }
        let pos = PolarCoordinate::new_from_floats(azimuth, elevation, distance)?;
        obj_chans[channel_index].azimuth = pos.azimuth();
        obj_chans[channel_index].elevation = pos.elevation();
        obj_chans[channel_index].distance = pos.distance();
        for g in &mut self.processing_groups {
            g.update_ambisonic_encoder(&mut self.audio_elements)?;
        }
        Ok(())
    }

    /// Enables or disables head tracking rotation of the sound field.
    pub fn enable_head_tracking(&mut self, enable: bool) {
        self.head_tracking_enabled = enable;
    }

    /// Enables or disables the output peak limiter.
    pub fn enable_limiter(&mut self, enable: bool) {
        self.limiter_enabled = enable;
    }

    /// Sets the head rotation quaternion for counter-rotating sound fields.
    ///
    /// # Parameters
    /// * `w`, `x`, `y`, `z` - Quaternion components representing head orientation.
    pub fn set_head_rotation(&mut self, rotation: Quaternion) -> Result<(), OarError> {
        self.world_rotation =
            Quaternion { w: rotation.w, x: -rotation.x, y: -rotation.y, z: -rotation.z };
        Ok(())
    }

    /// Sets head-locked rendering mode for a specific audio element.
    pub fn set_element_head_locked(
        &mut self,
        audio_element_index: usize,
        head_locked: bool,
    ) -> Result<(), OarError> {
        if audio_element_index >= self.audio_elements.len() {
            return Err(OarError::InvalidParameter);
        }
        if self.audio_elements[audio_element_index].element_type().is_passthrough() {
            return Ok(());
        }
        self.audio_elements[audio_element_index].set_head_locked(head_locked);
        Ok(())
    }

    /// Processes planar audio data from `input_buffer` to a 2-channel binaural `output_buffer`.
    pub fn process(&mut self, input_buffer: &AudioBuffer, output_buffer: &mut AudioBuffer) {
        let frames_usize = self.buffer_size_per_channel.value() as usize;
        assert_eq!(input_buffer.num_frames(), frames_usize);
        assert_eq!(output_buffer.num_channels(), 2);
        assert_eq!(output_buffer.num_frames(), frames_usize);
        output_buffer.clear();

        let expected_channels = self.get_number_of_input_channels();
        if input_buffer.num_channels() != expected_channels {
            return;
        }

        for audio_element in &self.audio_elements {
            if audio_element.element_type().is_passthrough() {
                let first_ch = audio_element.get_first_channel_index();
                // TODO(b/525080422): Optimize by retrieving output buffer slices once at the start
                // of process using as_slices_mut to avoid repeated channel_mut calls in the loop.
                if audio_element.element_type() == AudioElementType::PassthroughMono {
                    let in_ch = input_buffer.channel(first_ch);
                    let out_l = output_buffer.channel_mut(0);
                    add_pointwise_in_place(out_l.data, in_ch.data);
                    let out_r = output_buffer.channel_mut(1);
                    add_pointwise_in_place(out_r.data, in_ch.data);
                } else if audio_element.element_type() == AudioElementType::PassthroughStereo {
                    let in_l = input_buffer.channel(first_ch);
                    let in_r = input_buffer.channel(first_ch + 1);
                    let out_l = output_buffer.channel_mut(0);
                    add_pointwise_in_place(out_l.data, in_l.data);
                    let out_r = output_buffer.channel_mut(1);
                    add_pointwise_in_place(out_r.data, in_r.data);
                }
            }
        }

        if self.processing_groups.is_empty() {
            if self.limiter_enabled
                && let Some(lim) = &mut self.peak_limiter
            {
                self.scratch_limiter_buffer.clear();
                lim.process(output_buffer, &mut self.scratch_limiter_buffer);
                output_buffer.copy_from_2d(&self.scratch_limiter_buffer.to_2d());
            }
            return;
        }

        let audio_elements = &self.audio_elements;
        let head_tracking_enabled = self.head_tracking_enabled;
        let world_rotation = &self.world_rotation;

        for group in &mut self.processing_groups {
            self.scratch_group_output.clear();
            group.process(
                input_buffer,
                audio_elements,
                head_tracking_enabled,
                world_rotation,
                &mut self.scratch_group_output,
                &mut self.fft_manager,
            );

            // TODO(b/525080422): Optimize by retrieving output buffer slices once at the start of
            // process using as_slices_mut to avoid repeated channel_mut calls in the loop.
            {
                let src = self.scratch_group_output.channel(0);
                let dst = output_buffer.channel_mut(0);
                add_pointwise_in_place(dst.data, src.data);
            }
            {
                let src = self.scratch_group_output.channel(1);
                let dst = output_buffer.channel_mut(1);
                add_pointwise_in_place(dst.data, src.data);
            }
        }

        if self.limiter_enabled
            && let Some(lim) = &mut self.peak_limiter
        {
            self.scratch_limiter_buffer.clear();
            lim.process(output_buffer, &mut self.scratch_limiter_buffer);
            output_buffer.copy_from_2d(&self.scratch_limiter_buffer.to_2d());
        }
    }

    /// Returns the buffer size per channel.
    pub fn get_buffer_size_per_channel(&self) -> Samples {
        self.buffer_size_per_channel
    }

    /// Returns the total number of input channels across all elements.
    pub fn get_number_of_input_channels(&self) -> usize {
        self.audio_elements.iter().map(|e| e.get_number_of_input_channels()).sum()
    }

    /// Returns the number of output channels (always 2 for stereo binaural).
    #[cfg(test)]
    pub fn get_number_of_output_channels(&self) -> usize {
        NUM_BINAURAL_CHANNELS
    }

    /// Returns the number of configured audio elements.
    pub fn get_number_of_audio_elements(&self) -> usize {
        self.audio_elements.len()
    }
}

use crate::common::definitions::{
    AudioElementConfig as OarAudioElementConfig, Gain, HighOrderAmbisonics, Layout, OarError,
    ObjectPosition,
};
use crate::renderer::audio_renderer_api::AudioRenderer;
use crate::renderer::gain_parameter::{generate_gain_multipliers, GainParameter, GainUpdate};
use std::collections::HashMap;

/// Struct to adapt and expose OBR through the AudioRenderer Trait.
#[derive(Debug)]
pub struct ObrAudioRendererAdapter {
    pub obr: ObrImpl,
    pub id_to_element_index: HashMap<u32, usize>,
    element_channels: HashMap<u32, usize>,
    scratch_input: AudioBuffer,
    scratch_output: AudioBuffer,
    element_gains: HashMap<u32, HashMap<u32, GainParameter>>,
    gain_scratch_buf: Vec<f32>,
}

impl ObrAudioRendererAdapter {
    /// Constructs a new `ObrAudioRendererAdapter` instance.
    pub fn new(samples_per_channel: Samples, sampling_rate: SampleRate) -> Self {
        let size_usize = samples_per_channel.value() as usize;
        let mut adapter = Self {
            obr: ObrImpl::new(samples_per_channel, sampling_rate),
            id_to_element_index: HashMap::new(),
            element_channels: HashMap::new(),
            scratch_input: AudioBuffer::default(),
            scratch_output: AudioBuffer::new(NUM_BINAURAL_CHANNELS, size_usize),
            element_gains: HashMap::new(),
            gain_scratch_buf: vec![1.0f32; size_usize],
        };
        adapter.obr.enable_limiter(false);
        adapter
    }
    fn get_element_channel_offset(&self, id: u32) -> usize {
        let target_idx = self.id_to_element_index.get(&id).copied().unwrap_or(0);
        let mut offset = 0;
        for (&eid, &idx) in &self.id_to_element_index {
            if idx < target_idx {
                offset += self.element_channels.get(&eid).copied().unwrap_or(0);
            }
        }
        offset
    }
}

impl AudioRenderer for ObrAudioRendererAdapter {
    fn add_element(&mut self, id: u32, config: &OarAudioElementConfig) -> Result<(), OarError> {
        if self.id_to_element_index.contains_key(&id) {
            return Err(OarError::InvalidParameter);
        }
        let elem_type = match config {
            OarAudioElementConfig::ChannelBased(cbc) => match cbc.layout {
                Layout::Mono => AudioElementType::LayoutMono,
                Layout::Stereo => AudioElementType::LayoutStereo,
                Layout::Layout51 => AudioElementType::Layout5_1_0,
                Layout::Layout512 => AudioElementType::Layout5_1_2,
                Layout::Layout514 => AudioElementType::Layout5_1_4,
                Layout::Layout71 => AudioElementType::Layout7_1_0,
                Layout::Layout712 => AudioElementType::Layout7_1_2,
                Layout::Layout714 => AudioElementType::Layout7_1_4,
                Layout::Layout312 => AudioElementType::Layout3_1_2,
                Layout::Layout916 => AudioElementType::Layout9_1_6,
                Layout::Layout7154 => AudioElementType::Layout7_1_5_4,
                Layout::LayoutA293 => AudioElementType::Layout10_2_9_3,
                _ => return Err(OarError::InvalidParameter),
            },
            OarAudioElementConfig::SceneBased(sbc) => match sbc.order {
                HighOrderAmbisonics::Zoa => AudioElementType::PassthroughMono,
                HighOrderAmbisonics::Order1 => AudioElementType::Oa1,
                HighOrderAmbisonics::Order2 => AudioElementType::Oa2,
                HighOrderAmbisonics::Order3 => AudioElementType::Oa3,
                HighOrderAmbisonics::Order4 => AudioElementType::Oa4,
            },
            OarAudioElementConfig::ObjectBased(obc) => match obc.num_objects {
                1 => AudioElementType::ObjectMono,
                2 => AudioElementType::ObjectDual,
                _ => return Err(OarError::InvalidParameter),
            },
        };
        let profile = if let Some(rc) = config.rendering_config() {
            match rc.binaural_filter_profile {
                crate::common::definitions::BinauralFilterProfile::Ambient => {
                    BinauralFilterProfile::Ambient
                }
                crate::common::definitions::BinauralFilterProfile::Direct => {
                    BinauralFilterProfile::Direct
                }
                crate::common::definitions::BinauralFilterProfile::Reverberant => {
                    BinauralFilterProfile::Reverberant
                }
            }
        } else {
            BinauralFilterProfile::Ambient
        };

        self.obr.add_audio_element(elem_type, profile)?;
        let idx = self.obr.get_number_of_audio_elements() - 1;
        self.id_to_element_index.insert(id, idx);
        self.element_channels.insert(id, config.channels());

        let expected_ins = self.obr.get_number_of_input_channels();
        let frames_usize = self.obr.get_buffer_size_per_channel().value() as usize;
        self.scratch_input.resize(expected_ins, frames_usize);

        let head_locked = if let Some(rc) = config.rendering_config() {
            rc.headphones_rendering_mode
                == crate::common::definitions::HeadphonesRenderingMode::HeadLocked
        } else {
            false
        };
        self.obr.set_element_head_locked(idx, head_locked)?;
        Ok(())
    }

    fn update_element_gain(
        &mut self,
        id: u32,
        gain_id: u32,
        gain: &Gain,
        duration: Samples,
    ) -> Result<(), OarError> {
        if !self.id_to_element_index.contains_key(&id) {
            return Err(OarError::InvalidParameter);
        }
        let params = self.element_gains.entry(id).or_default();
        let param = params.entry(gain_id).or_insert_with(|| GainParameter::new(gain_id));
        param.updates.push_back(GainUpdate { gain: gain.variant.clone(), duration, elapsed: 0 });
        Ok(())
    }

    fn update_element_positions(
        &mut self,
        id: u32,
        positions: &ObjectPosition,
        _duration: Samples,
    ) -> Result<(), OarError> {
        let &idx = self.id_to_element_index.get(&id).ok_or(OarError::InvalidParameter)?;
        match positions {
            ObjectPosition::Polar(polar_coords) => {
                for (i, polar_coord) in
                    polar_coords.iter().enumerate().take(polar_coords.len().min(2))
                {
                    self.obr.update_object_channel_position(
                        idx,
                        i,
                        polar_coord.azimuth().0,
                        polar_coord.elevation().0,
                        polar_coord.distance().value(),
                    )?;
                }
                Ok(())
            }
            _ => Err(OarError::NotSupported),
        }
    }

    fn render(
        &mut self,
        inputs: PlanarBufferRef<'_, '_>,
        output: &mut PlanarBufferMut<'_, '_>,
    ) -> Result<(), OarError> {
        let expected_ins = self.obr.get_number_of_input_channels();
        if inputs.num_channels() != expected_ins {
            return Err(OarError::InvalidParameter);
        }
        let frames = self.obr.get_buffer_size_per_channel();
        let frames_usize = frames.value() as usize;
        // TODO(b/525080422): Avoid resizing scratch buffers in the render path as it can cause heap
        // allocations. We should know the number of samples needed from the first creation of the
        // ROAR instance and can use that to initialize buffers as needed.
        if self.scratch_input.num_channels() != expected_ins
            || self.scratch_input.num_frames() != frames_usize
        {
            self.scratch_input.resize(expected_ins, frames_usize);
        }
        for ch in 0..expected_ins {
            let src = inputs.channel(ch);
            let mut dst = self.scratch_input.channel_mut(ch);
            let len = src.len().min(frames_usize);
            dst.as_mut_slice()[..len].copy_from_slice(&src[..len]);
        }
        for &eid in self.id_to_element_index.keys() {
            let offset = self.get_element_channel_offset(eid);
            let num_chans = self.element_channels.get(&eid).copied().unwrap_or(0);
            if let Some(gains) = self.element_gains.get_mut(&eid) {
                for param in gains.values_mut() {
                    generate_gain_multipliers(param, frames_usize, &mut self.gain_scratch_buf);
                    for ch in offset..offset + num_chans {
                        let mut dst = self.scratch_input.channel_mut(ch);
                        for (sample, &gain) in
                            dst.as_mut_slice().iter_mut().zip(self.gain_scratch_buf.iter())
                        {
                            *sample *= gain;
                        }
                    }
                }
            }
        }
        self.obr.process(&self.scratch_input, &mut self.scratch_output);
        if output.num_channels() < 2 || output.num_samples() < frames_usize {
            return Err(OarError::InvalidParameter);
        }
        let ch0 = self.scratch_output.channel(0);
        let ch1 = self.scratch_output.channel(1);
        let s0 = ch0.as_slice();
        let s1 = ch1.as_slice();
        output.channel_mut(0)[..frames_usize].copy_from_slice(&s0[..frames_usize]);
        output.channel_mut(1)[..frames_usize].copy_from_slice(&s1[..frames_usize]);
        Ok(())
    }

    fn enable_head_tracking(&mut self, enable: bool) -> Result<(), OarError> {
        self.obr.enable_head_tracking(enable);
        Ok(())
    }

    fn set_head_rotation(&mut self, rotation: Quaternion) -> Result<(), OarError> {
        self.obr.set_head_rotation(rotation)
    }

    fn enable_limiter(&mut self, enable: bool) -> Result<(), OarError> {
        self.obr.enable_limiter(enable);
        Ok(())
    }
}

// ===== Tests =====

#[cfg(test)]
mod test {

    use super::*;
    use googletest::prelude::*;
    use std::f32::consts::PI;

    const BUFFER_SIZE: usize = 128;
    const SAMPLING_RATE: i32 = 48000;

    fn get_sample_rate() -> SampleRate {
        SampleRate::new(SAMPLING_RATE as u32).unwrap()
    }

    fn get_buffer_size() -> Samples {
        Samples::new(BUFFER_SIZE as u32).unwrap()
    }

    fn generate_sine(sampling_rate: i32, channel: &mut [f32]) {
        let len = channel.len();
        let freq = 440.0;
        let amplitude = 0.5;
        for (i, chan) in channel.iter_mut().enumerate().take(len) {
            *chan = amplitude * (2.0 * PI * freq * (i as f32) / (sampling_rate as f32)).sin();
        }
    }

    fn has_non_zero_output(buffer: &AudioBuffer) -> bool {
        for ch in 0..buffer.num_channels() {
            for &sample in buffer.channel(ch).as_slice() {
                if sample.abs() > 1e-6 {
                    return true;
                }
            }
        }
        false
    }

    #[gtest]
    fn test_initialization() {
        let renderer = ObrImpl::new(get_buffer_size(), get_sample_rate());
        expect_that!(renderer.buffer_size_per_channel, eq(get_buffer_size()));
        expect_that!(renderer.sampling_rate, eq(get_sample_rate()));
        expect_that!(renderer.get_number_of_input_channels(), eq(0));
        expect_that!(renderer.get_number_of_output_channels(), eq(2));
    }

    #[gtest]
    fn test_add_and_remove_audio_element() {
        let mut renderer = ObrImpl::new(get_buffer_size(), get_sample_rate());
        let add_result =
            renderer.add_audio_element(AudioElementType::Oa3, BinauralFilterProfile::Direct);

        expect_true!(add_result.is_ok());
        expect_that!(renderer.get_number_of_input_channels(), eq(16));

        let remove_result = renderer.remove_last_audio_element();

        expect_true!(remove_result.is_ok());
        expect_that!(renderer.get_number_of_input_channels(), eq(0));
    }

    #[gtest]
    fn test_add_and_remove_multiple_audio_elements() {
        let mut renderer = ObrImpl::new(get_buffer_size(), get_sample_rate());
        let add1 = renderer.add_audio_element(AudioElementType::Oa3, BinauralFilterProfile::Direct);

        expect_true!(add1.is_ok());
        expect_that!(renderer.get_number_of_input_channels(), eq(16));

        let add2 = renderer
            .add_audio_element(AudioElementType::Layout7_1_4, BinauralFilterProfile::Direct);

        expect_true!(add2.is_ok());
        expect_that!(renderer.get_number_of_input_channels(), eq(28));
        expect_that!(renderer.audio_elements.len(), eq(2));

        let remove1 = renderer.remove_last_audio_element();

        expect_true!(remove1.is_ok());
        expect_that!(renderer.get_number_of_input_channels(), eq(16));
        expect_that!(renderer.audio_elements.len(), eq(1));

        let remove2 = renderer.remove_last_audio_element();

        expect_true!(remove2.is_ok());
        expect_that!(renderer.get_number_of_input_channels(), eq(0));
        expect_that!(renderer.audio_elements.len(), eq(0));

        let remove3 = renderer.remove_last_audio_element();

        expect_true!(remove3.is_err());
    }

    #[gtest]
    fn test_process_audio_buffer_with_wrong_number_of_channels() {
        let mut renderer = ObrImpl::new(get_buffer_size(), get_sample_rate());
        let add = renderer.add_audio_element(AudioElementType::Oa3, BinauralFilterProfile::Direct);

        expect_true!(add.is_ok());

        let input_buffer = AudioBuffer::new(17, BUFFER_SIZE);
        let mut output_buffer = AudioBuffer::new(2, BUFFER_SIZE);
        output_buffer.channel_mut(0).as_mut_slice().fill(1.0);
        output_buffer.channel_mut(1).as_mut_slice().fill(1.0);

        renderer.process(&input_buffer, &mut output_buffer);

        for ch in 0..2 {
            for &sample in output_buffer.channel(ch).as_slice() {
                expect_that!(sample, eq(0.0));
            }
        }
    }

    #[gtest]
    fn test_add_multiple_audio_elements_with_different_types_same_filter_type_succeeds() {
        let mut renderer = ObrImpl::new(get_buffer_size(), get_sample_rate());
        let add1 = renderer.add_audio_element(AudioElementType::Oa3, BinauralFilterProfile::Direct);

        expect_true!(add1.is_ok());

        let add2 = renderer.add_audio_element(AudioElementType::Oa1, BinauralFilterProfile::Direct);

        expect_true!(add2.is_ok());

        expect_that!(renderer.get_number_of_input_channels(), eq(20));
        expect_that!(renderer.audio_elements.len(), eq(2));
        expect_that!(renderer.processing_groups.len(), eq(2));
    }

    #[gtest]
    fn test_add_multiple_audio_elements_same_type_different_filter_types_succeeds() {
        let mut renderer = ObrImpl::new(get_buffer_size(), get_sample_rate());
        let add1 = renderer.add_audio_element(AudioElementType::Oa3, BinauralFilterProfile::Direct);

        expect_true!(add1.is_ok());

        let add2 =
            renderer.add_audio_element(AudioElementType::Oa3, BinauralFilterProfile::Reverberant);

        expect_true!(add2.is_ok());

        expect_that!(renderer.get_number_of_input_channels(), eq(32));
        expect_that!(renderer.audio_elements.len(), eq(2));
        expect_that!(renderer.processing_groups.len(), eq(2));
    }

    #[gtest]
    fn test_ambisonics_and_loudspeaker_layouts_same_filter_profile() {
        let mut renderer = ObrImpl::new(get_buffer_size(), get_sample_rate());
        let add1 = renderer.add_audio_element(AudioElementType::Oa2, BinauralFilterProfile::Direct);

        expect_true!(add1.is_ok());

        let add2 = renderer
            .add_audio_element(AudioElementType::Layout7_1_4, BinauralFilterProfile::Direct);

        expect_true!(add2.is_ok());

        expect_that!(renderer.get_number_of_input_channels(), eq(21));
        expect_that!(renderer.processing_groups.len(), eq(2));
    }

    #[gtest]
    fn test_passthrough_mono() {
        let mut renderer = ObrImpl::new(get_buffer_size(), get_sample_rate());
        renderer.enable_limiter(false);
        let add = renderer
            .add_audio_element(AudioElementType::PassthroughMono, BinauralFilterProfile::Direct);

        expect_true!(add.is_ok());

        let mut input_buffer = AudioBuffer::new(1, BUFFER_SIZE);
        generate_sine(SAMPLING_RATE, input_buffer.channel_mut(0).as_mut_slice());

        let mut output_buffer = AudioBuffer::new(2, BUFFER_SIZE);
        renderer.process(&input_buffer, &mut output_buffer);

        for i in 0..BUFFER_SIZE {
            expect_that!(
                output_buffer.channel(0).as_slice()[i],
                eq(input_buffer.channel(0).as_slice()[i])
            );
            expect_that!(
                output_buffer.channel(1).as_slice()[i],
                eq(input_buffer.channel(0).as_slice()[i])
            );
        }
    }

    #[gtest]
    fn test_passthrough_stereo() {
        let mut renderer = ObrImpl::new(get_buffer_size(), get_sample_rate());
        renderer.enable_limiter(false);
        let add = renderer
            .add_audio_element(AudioElementType::PassthroughStereo, BinauralFilterProfile::Direct);

        expect_true!(add.is_ok());

        let mut input_buffer = AudioBuffer::new(2, BUFFER_SIZE);
        generate_sine(SAMPLING_RATE, input_buffer.channel_mut(0).as_mut_slice());
        for i in 0..BUFFER_SIZE {
            input_buffer.channel_mut(1).as_mut_slice()[i] = -input_buffer.channel(0).as_slice()[i];
        }

        let mut output_buffer = AudioBuffer::new(2, BUFFER_SIZE);
        renderer.process(&input_buffer, &mut output_buffer);

        for i in 0..BUFFER_SIZE {
            expect_that!(
                output_buffer.channel(0).as_slice()[i],
                eq(input_buffer.channel(0).as_slice()[i])
            );
            expect_that!(
                output_buffer.channel(1).as_slice()[i],
                eq(input_buffer.channel(1).as_slice()[i])
            );
        }
    }

    #[gtest]
    fn test_process_mixed_ambisonics_and_object() {
        let mut renderer = ObrImpl::new(get_buffer_size(), get_sample_rate());
        let add1 = renderer.add_audio_element(AudioElementType::Oa1, BinauralFilterProfile::Direct);

        expect_true!(add1.is_ok());

        let add2 =
            renderer.add_audio_element(AudioElementType::ObjectMono, BinauralFilterProfile::Direct);

        expect_true!(add2.is_ok());

        let mut input_buffer = AudioBuffer::new(5, BUFFER_SIZE);
        for ch in 0..5 {
            generate_sine(SAMPLING_RATE, input_buffer.channel_mut(ch).as_mut_slice());
        }

        let mut output_buffer = AudioBuffer::new(2, BUFFER_SIZE);
        renderer.process(&input_buffer, &mut output_buffer);

        expect_that!(has_non_zero_output(&output_buffer), eq(true));
    }
}
