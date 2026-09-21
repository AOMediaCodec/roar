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

//! Processing groups grouping audio elements.
//!
//! Audio elements that share the same Ambisonic order and binaural filter profile are grouped
//! into a single `ProcessingGroup`. This optimizes DSP processing by reducing the number of
//! rotations, convolutions, and FFT operations.

use super::audio_element_config::{AudioElementConfig, BinauralFilterProfile};
use crate::common::definitions::{LinearGain, OarError, Quaternion, SampleRate, Samples};
use crate::renderer::obr::ambisonic_binaural_decoder::{
    create_sh_hrirs_from_assets, AmbisonicBinauralDecoder, FftManager, Resampler,
};
use crate::renderer::obr::ambisonic_encoder::AmbisonicEncoder;
use crate::renderer::obr::ambisonic_rotator::AmbisonicRotator;
use crate::renderer::obr::audio_buffer::AudioBuffer;
use crate::renderer::obr::common::ambisonic_utils::get_num_periphonic_components;
use crate::renderer::obr::common::constants::{
    MAX_SUPPORTED_AMBISONIC_ORDER, MIN_SUPPORTED_AMBISONIC_ORDER, NUM_BINAURAL_CHANNELS,
};

/// Key representing the unique attributes of a processing group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProcessingGroupKey {
    /// The Ambisonic order of the group's input elements.
    pub ambisonic_order: i32,
    /// The target binaural filter profile.
    pub filter_profile: BinauralFilterProfile,
}

/// Coordinator for DSP operations within a single processing group.
///
/// Manages the Ambisonic encoder, rotator, and binaural decoder resources shared
/// by all audio elements assigned to this group.
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessingGroup {
    key: ProcessingGroupKey,
    audio_element_indices: Vec<usize>,
    buffer_size_per_channel: Samples,
    sampling_rate: SampleRate,
    ambisonic_mix_bed: AudioBuffer,
    ambisonic_mix_bed_head_locked: AudioBuffer,
    ambisonic_encoder: Option<AmbisonicEncoder>,
    ambisonic_encoder_input_buffer: AudioBuffer,
    ambisonic_rotator: Option<AmbisonicRotator>,
    ambisonic_binaural_decoder: Option<AmbisonicBinauralDecoder>,
    world_locked_indices: Vec<usize>,
    head_locked_indices: Vec<usize>,
}

impl ProcessingGroup {
    /// Constructs a new `ProcessingGroup`.
    pub fn new(
        key: ProcessingGroupKey,
        audio_element_indices: Vec<usize>,
        buffer_size_per_channel: Samples,
        sampling_rate: SampleRate,
    ) -> Self {
        let size_usize = buffer_size_per_channel.value() as usize;
        let num_ambisonic_channels = get_num_periphonic_components(key.ambisonic_order);
        Self {
            key,
            audio_element_indices,
            buffer_size_per_channel,
            sampling_rate,
            ambisonic_mix_bed: AudioBuffer::new(num_ambisonic_channels, size_usize),
            ambisonic_mix_bed_head_locked: AudioBuffer::new(num_ambisonic_channels, size_usize),
            ambisonic_encoder: None,
            ambisonic_encoder_input_buffer: AudioBuffer::default(),
            ambisonic_rotator: None,
            ambisonic_binaural_decoder: None,
            // TODO(b/525080422): Pre-allocate these vectors to avoid heap allocations in the render
            // path. Could we know the number of input channels before creation to avoid allocating
            // a maximum capacity?
            world_locked_indices: Vec::new(),
            head_locked_indices: Vec::new(),
        }
    }

    /// Initializes DSP resources (rotator and binaural decoder) using given assets/resampler.
    ///
    /// # Parameters
    /// * `fft_manager` - Context for FFT and IFFT transforms.
    /// * `resampler` - State resampler for loading filters at custom sample rates.
    pub fn initialize(
        &mut self,
        fft_manager: &mut FftManager,
        resampler: &mut Resampler,
    ) -> Result<(), OarError> {
        let order = self.key.ambisonic_order;
        if !(MIN_SUPPORTED_AMBISONIC_ORDER..=MAX_SUPPORTED_AMBISONIC_ORDER).contains(&order) {
            return Err(OarError::InvalidParameter);
        }

        let frames_per_buffer = self.buffer_size_per_channel.value() as usize;
        self.ambisonic_rotator = Some(AmbisonicRotator::new(order, frames_per_buffer));

        let profile_str = match self.key.filter_profile {
            BinauralFilterProfile::Direct => "Direct",
            BinauralFilterProfile::Ambient => "Ambient",
            BinauralFilterProfile::Reverberant => "Reverberant",
        };

        let hrir_l_name = format!("{}OA{}L", order, profile_str);
        let hrir_r_name = format!("{}OA{}R", order, profile_str);

        let sh_hrirs_l = create_sh_hrirs_from_assets(&hrir_l_name, self.sampling_rate, resampler)
            .ok_or(OarError::InvalidParameter)?;
        let sh_hrirs_r = create_sh_hrirs_from_assets(&hrir_r_name, self.sampling_rate, resampler)
            .ok_or(OarError::InvalidParameter)?;

        assert_eq!(sh_hrirs_l.num_channels(), sh_hrirs_r.num_channels());
        assert_eq!(sh_hrirs_l.num_frames(), sh_hrirs_r.num_frames());

        self.ambisonic_binaural_decoder = Some(AmbisonicBinauralDecoder::new(
            &sh_hrirs_l,
            &sh_hrirs_r,
            frames_per_buffer,
            fft_manager,
        ));

        Ok(())
    }

    /// Processes world/head-locked audio elements in this group into a binaural `output_buffer`.
    pub fn process(
        &mut self,
        input_buffer: &AudioBuffer,
        audio_elements: &[AudioElementConfig],
        head_tracking_enabled: bool,
        world_rotation: &Quaternion,
        output_buffer: &mut AudioBuffer,
        fft_manager: &mut FftManager,
    ) {
        assert_eq!(output_buffer.num_channels(), NUM_BINAURAL_CHANNELS);
        assert_eq!(output_buffer.num_frames(), self.buffer_size_per_channel.value() as usize);

        self.ambisonic_mix_bed.clear();
        self.ambisonic_mix_bed_head_locked.clear();

        self.world_locked_indices.clear();
        Self::populate_encoder_source_channel_indices(
            &self.audio_element_indices,
            audio_elements,
            Some(false),
            &mut self.world_locked_indices,
        );
        if !self.world_locked_indices.is_empty() && self.ambisonic_encoder.is_some() {
            for (i, &src_idx) in self.world_locked_indices.iter().enumerate() {
                let src = input_buffer.channel(src_idx);
                let mut dst = self.ambisonic_encoder_input_buffer.channel_mut(i);
                dst.as_mut_slice().copy_from_slice(src.as_slice());
            }
            if let Some(enc) = &mut self.ambisonic_encoder {
                enc.process_planar_audio_data(
                    &self.ambisonic_encoder_input_buffer,
                    &mut self.ambisonic_mix_bed,
                );
            }
        }

        for &ae_index in &self.audio_element_indices {
            let audio_element = &audio_elements[ae_index];
            if audio_element.element_type().is_ambisonics() && !audio_element.is_head_locked() {
                for channel in 0..audio_element.get_number_of_input_channels() {
                    let mut bed_ch = self.ambisonic_mix_bed.channel_mut(channel);
                    let src_ch =
                        input_buffer.channel(audio_element.get_first_channel_index() + channel);
                    bed_ch.add_assign_view(&src_ch);
                }
            }
        }

        self.head_locked_indices.clear();
        Self::populate_encoder_source_channel_indices(
            &self.audio_element_indices,
            audio_elements,
            Some(true),
            &mut self.head_locked_indices,
        );
        if !self.head_locked_indices.is_empty() && self.ambisonic_encoder.is_some() {
            for (i, &src_idx) in self.head_locked_indices.iter().enumerate() {
                let src = input_buffer.channel(src_idx);
                let mut dst = self.ambisonic_encoder_input_buffer.channel_mut(i);
                dst.as_mut_slice().copy_from_slice(src.as_slice());
            }
            if let Some(enc) = &mut self.ambisonic_encoder {
                enc.process_planar_audio_data(
                    &self.ambisonic_encoder_input_buffer,
                    &mut self.ambisonic_mix_bed_head_locked,
                );
            }
        }

        for &ae_index in &self.audio_element_indices {
            let audio_element = &audio_elements[ae_index];
            if audio_element.element_type().is_ambisonics() && audio_element.is_head_locked() {
                for channel in 0..audio_element.get_number_of_input_channels() {
                    let mut bed_ch = self.ambisonic_mix_bed_head_locked.channel_mut(channel);
                    let src_ch =
                        input_buffer.channel(audio_element.get_first_channel_index() + channel);
                    bed_ch.add_assign_view(&src_ch);
                }
            }
        }

        if head_tracking_enabled && let Some(rot) = &mut self.ambisonic_rotator {
            rot.process_in_place(world_rotation, &mut self.ambisonic_mix_bed);
        }

        self.ambisonic_mix_bed += &self.ambisonic_mix_bed_head_locked;

        if let Some(dec) = &mut self.ambisonic_binaural_decoder {
            dec.process_audio_buffer(&self.ambisonic_mix_bed, output_buffer, fft_manager);
        }
    }

    /// Updates Ambisonic encoder source positions and channel inputs.
    pub fn update_ambisonic_encoder(
        &mut self,
        audio_elements: &mut [AudioElementConfig],
    ) -> Result<(), OarError> {
        let expected_channels = self.get_encoder_channels_count(audio_elements);
        if expected_channels == 0 {
            self.ambisonic_encoder = None;
            self.ambisonic_encoder_input_buffer = AudioBuffer::default();
            return Ok(());
        }

        if self.ambisonic_encoder.is_none()
            || self.ambisonic_encoder_input_buffer.num_channels() != expected_channels
        {
            self.ambisonic_encoder_input_buffer =
                AudioBuffer::new(expected_channels, self.buffer_size_per_channel.value() as usize);
            self.ambisonic_encoder =
                Some(AmbisonicEncoder::new(expected_channels, self.key.ambisonic_order as usize));
        }

        if let Some(enc) = &mut self.ambisonic_encoder {
            let mut enc_idx = 0;
            for &ae_index in &self.audio_element_indices {
                let audio_element = &mut audio_elements[ae_index];
                for source in audio_element.get_loudspeaker_channels() {
                    enc.set_source(
                        enc_idx,
                        LinearGain(1.0),
                        source.azimuth,
                        source.elevation,
                        source.distance,
                    );
                    enc_idx += 1;
                }
                for source in audio_element.get_object_channels() {
                    enc.set_source(
                        enc_idx,
                        LinearGain(1.0),
                        source.azimuth,
                        source.elevation,
                        source.distance,
                    );
                    enc_idx += 1;
                }
            }
        }
        Ok(())
    }
}

impl ProcessingGroup {
    fn get_encoder_channels_count(&self, audio_elements: &[AudioElementConfig]) -> usize {
        let mut count = 0;
        for &ae_index in &self.audio_element_indices {
            let ae = &audio_elements[ae_index];
            if ae.element_type().is_loudspeaker_layout() || ae.element_type().is_object() {
                count += ae.get_number_of_input_channels();
            }
        }
        count
    }

    fn populate_encoder_source_channel_indices(
        audio_element_indices: &[usize],
        audio_elements: &[AudioElementConfig],
        filter_head_locked: Option<bool>,
        out_indices: &mut Vec<usize>,
    ) {
        for &ae_index in audio_element_indices {
            let ae = &audio_elements[ae_index];
            if let Some(target_locked) = filter_head_locked
                && ae.is_head_locked() != target_locked
            {
                continue;
            }
            if ae.element_type().is_loudspeaker_layout() || ae.element_type().is_object() {
                for i in 0..ae.get_number_of_input_channels() {
                    out_indices.push(ae.get_first_channel_index() + i);
                }
            }
        }
    }
}

#[cfg(test)]
impl ProcessingGroup {
    pub fn get_key(&self) -> ProcessingGroupKey {
        self.key
    }

    pub fn get_audio_element_indices(&self) -> &[usize] {
        &self.audio_element_indices
    }

    pub fn get_ambisonic_order(&self) -> i32 {
        self.key.ambisonic_order
    }

    pub fn get_filter_profile(&self) -> BinauralFilterProfile {
        self.key.filter_profile
    }
}

// ===== Tests =====

#[cfg(test)]
mod test {

    use super::*;
    use crate::common::definitions::{Degrees, Distance, Quaternion, SampleRate};
    use crate::renderer::obr::renderer::audio_element_type::AudioElementType;
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
    fn test_equality() {
        let key1 = ProcessingGroupKey {
            ambisonic_order: 1,
            filter_profile: BinauralFilterProfile::Direct,
        };
        let key2 = ProcessingGroupKey {
            ambisonic_order: 1,
            filter_profile: BinauralFilterProfile::Direct,
        };
        let key3 = ProcessingGroupKey {
            ambisonic_order: 2,
            filter_profile: BinauralFilterProfile::Direct,
        };
        let key4 = ProcessingGroupKey {
            ambisonic_order: 1,
            filter_profile: BinauralFilterProfile::Ambient,
        };

        expect_that!(key1 == key2, eq(true));
        expect_that!(key1 == key3, eq(false));
        expect_that!(key1 == key4, eq(false));
    }

    #[gtest]
    fn test_less_than() {
        let key1 = ProcessingGroupKey {
            ambisonic_order: 1,
            filter_profile: BinauralFilterProfile::Direct,
        };
        let key2 = ProcessingGroupKey {
            ambisonic_order: 2,
            filter_profile: BinauralFilterProfile::Direct,
        };
        let key3 = ProcessingGroupKey {
            ambisonic_order: 1,
            filter_profile: BinauralFilterProfile::Ambient,
        };

        expect_that!(key1 < key2, eq(true));
        expect_that!(key2 < key1, eq(false));
        expect_that!(key1 < key3, eq(true));
    }

    #[gtest]
    fn test_construction() {
        let key = ProcessingGroupKey {
            ambisonic_order: 2,
            filter_profile: BinauralFilterProfile::Direct,
        };
        let indices = vec![0, 1];

        let group =
            ProcessingGroup::new(key, indices.clone(), get_buffer_size(), get_sample_rate());

        expect_that!(group.get_key().ambisonic_order, eq(2));
        expect_that!(group.get_key().filter_profile, eq(BinauralFilterProfile::Direct));
        expect_that!(group.get_audio_element_indices(), eq(&indices));
        expect_that!(group.get_ambisonic_order(), eq(2));
        expect_that!(group.get_filter_profile(), eq(BinauralFilterProfile::Direct));
    }

    #[gtest]
    fn test_initialize_direct_filter() {
        let key = ProcessingGroupKey {
            ambisonic_order: 1,
            filter_profile: BinauralFilterProfile::Direct,
        };
        let indices = vec![0];
        let mut group = ProcessingGroup::new(key, indices, get_buffer_size(), get_sample_rate());
        let mut fft_manager = FftManager::new(BUFFER_SIZE);
        let mut resampler = Resampler::default();

        expect_true!(group.initialize(&mut fft_manager, &mut resampler).is_ok());
    }

    #[gtest]
    fn test_initialize_ambient_filter() {
        let key = ProcessingGroupKey {
            ambisonic_order: 2,
            filter_profile: BinauralFilterProfile::Ambient,
        };
        let indices = vec![0, 1];
        let mut group = ProcessingGroup::new(key, indices, get_buffer_size(), get_sample_rate());
        let mut fft_manager = FftManager::new(BUFFER_SIZE);
        let mut resampler = Resampler::default();

        expect_true!(group.initialize(&mut fft_manager, &mut resampler).is_ok());
    }

    #[gtest]
    fn test_initialize_reverberant_filter() {
        let key = ProcessingGroupKey {
            ambisonic_order: 3,
            filter_profile: BinauralFilterProfile::Reverberant,
        };
        let indices = vec![0];
        let mut group = ProcessingGroup::new(key, indices, get_buffer_size(), get_sample_rate());
        let mut fft_manager = FftManager::new(BUFFER_SIZE);
        let mut resampler = Resampler::default();

        expect_true!(group.initialize(&mut fft_manager, &mut resampler).is_ok());
    }

    #[gtest]
    fn test_initialize_different_orders() {
        let orders = vec![1, 2, 3, 4];
        let mut fft_manager = FftManager::new(BUFFER_SIZE);
        let mut resampler = Resampler::default();

        for order in orders {
            let key = ProcessingGroupKey {
                ambisonic_order: order,
                filter_profile: BinauralFilterProfile::Direct,
            };
            let indices = vec![0];
            let mut group =
                ProcessingGroup::new(key, indices, get_buffer_size(), get_sample_rate());

            expect_true!(group.initialize(&mut fft_manager, &mut resampler).is_ok());
        }
    }

    #[gtest]
    fn test_process_ambisonics_only() {
        let key = ProcessingGroupKey {
            ambisonic_order: 1,
            filter_profile: BinauralFilterProfile::Direct,
        };
        let indices = vec![0];
        let mut group = ProcessingGroup::new(key, indices, get_buffer_size(), get_sample_rate());
        let mut fft_manager = FftManager::new(BUFFER_SIZE);
        let mut resampler = Resampler::default();
        assert!(group.initialize(&mut fft_manager, &mut resampler).is_ok());

        let audio_elements =
            vec![AudioElementConfig::new(AudioElementType::Oa1, BinauralFilterProfile::Direct)];

        let mut input_buffer = AudioBuffer::new(4, BUFFER_SIZE);
        let mut output_buffer = AudioBuffer::new(2, BUFFER_SIZE);

        for ch in 0..input_buffer.num_channels() {
            generate_sine(SAMPLING_RATE, input_buffer.channel_mut(ch).as_mut_slice());
        }

        let rotation = Quaternion::default();
        group.process(
            &input_buffer,
            &audio_elements,
            false,
            &rotation,
            &mut output_buffer,
            &mut fft_manager,
        );

        expect_that!(has_non_zero_output(&output_buffer), eq(true));
    }

    #[gtest]
    fn test_process_loudspeaker_layout() {
        let key = ProcessingGroupKey {
            ambisonic_order: 2,
            filter_profile: BinauralFilterProfile::Direct,
        };
        let indices = vec![0];
        let mut group = ProcessingGroup::new(key, indices, get_buffer_size(), get_sample_rate());
        let mut fft_manager = FftManager::new(BUFFER_SIZE);
        let mut resampler = Resampler::default();
        assert!(group.initialize(&mut fft_manager, &mut resampler).is_ok());

        let mut audio_elements = vec![AudioElementConfig::new(
            AudioElementType::LayoutStereo,
            BinauralFilterProfile::Direct,
        )];

        expect_true!(group.update_ambisonic_encoder(&mut audio_elements).is_ok());

        let mut input_buffer = AudioBuffer::new(2, BUFFER_SIZE);
        let mut output_buffer = AudioBuffer::new(2, BUFFER_SIZE);

        for ch in 0..input_buffer.num_channels() {
            generate_sine(SAMPLING_RATE, input_buffer.channel_mut(ch).as_mut_slice());
        }

        let rotation = Quaternion::default();
        group.process(
            &input_buffer,
            &audio_elements,
            false,
            &rotation,
            &mut output_buffer,
            &mut fft_manager,
        );

        expect_that!(has_non_zero_output(&output_buffer), eq(true));
    }

    #[gtest]
    fn test_process_audio_object() {
        let key = ProcessingGroupKey {
            ambisonic_order: 1,
            filter_profile: BinauralFilterProfile::Direct,
        };
        let indices = vec![0];
        let mut group = ProcessingGroup::new(key, indices, get_buffer_size(), get_sample_rate());
        let mut fft_manager = FftManager::new(BUFFER_SIZE);
        let mut resampler = Resampler::default();
        assert!(group.initialize(&mut fft_manager, &mut resampler).is_ok());

        let mut audio_elements = vec![AudioElementConfig::new(
            AudioElementType::ObjectMono,
            BinauralFilterProfile::Direct,
        )];

        let object_channels = audio_elements[0].get_object_channels();
        for obj in object_channels {
            obj.azimuth = Degrees(45.0);
            obj.elevation = Degrees(0.0);
            obj.distance = Distance::new(1.0).unwrap();
        }

        expect_true!(group.update_ambisonic_encoder(&mut audio_elements).is_ok());

        let mut input_buffer = AudioBuffer::new(1, BUFFER_SIZE);
        let mut output_buffer = AudioBuffer::new(2, BUFFER_SIZE);

        generate_sine(SAMPLING_RATE, input_buffer.channel_mut(0).as_mut_slice());

        let rotation = Quaternion::default();
        group.process(
            &input_buffer,
            &audio_elements,
            false,
            &rotation,
            &mut output_buffer,
            &mut fft_manager,
        );

        expect_that!(has_non_zero_output(&output_buffer), eq(true));
    }

    #[gtest]
    fn test_process_with_head_tracking() {
        let key = ProcessingGroupKey {
            ambisonic_order: 1,
            filter_profile: BinauralFilterProfile::Direct,
        };
        let indices = vec![0];
        let mut group = ProcessingGroup::new(key, indices, get_buffer_size(), get_sample_rate());
        let mut fft_manager = FftManager::new(BUFFER_SIZE);
        let mut resampler = Resampler::default();
        assert!(group.initialize(&mut fft_manager, &mut resampler).is_ok());

        let audio_elements =
            vec![AudioElementConfig::new(AudioElementType::Oa1, BinauralFilterProfile::Direct)];

        let mut input_buffer = AudioBuffer::new(4, BUFFER_SIZE);
        let mut output_buffer = AudioBuffer::new(2, BUFFER_SIZE);

        for ch in 0..input_buffer.num_channels() {
            generate_sine(SAMPLING_RATE, input_buffer.channel_mut(ch).as_mut_slice());
        }

        let rotation = Quaternion { w: 0.707, x: 0.0, y: 0.707, z: 0.0 };

        group.process(
            &input_buffer,
            &audio_elements,
            true,
            &rotation,
            &mut output_buffer,
            &mut fft_manager,
        );

        expect_that!(has_non_zero_output(&output_buffer), eq(true));
    }

    #[gtest]
    fn test_process_mixed_content() {
        let key = ProcessingGroupKey {
            ambisonic_order: 2,
            filter_profile: BinauralFilterProfile::Direct,
        };
        let indices = vec![0, 1];
        let mut group = ProcessingGroup::new(key, indices, get_buffer_size(), get_sample_rate());
        let mut fft_manager = FftManager::new(BUFFER_SIZE);
        let mut resampler = Resampler::default();
        assert!(group.initialize(&mut fft_manager, &mut resampler).is_ok());

        let mut audio_elements = vec![
            AudioElementConfig::new(AudioElementType::Oa2, BinauralFilterProfile::Direct),
            AudioElementConfig::new(AudioElementType::ObjectMono, BinauralFilterProfile::Direct),
        ];

        audio_elements[1].set_first_channel_index(9);

        let object_channels = audio_elements[1].get_object_channels();
        for obj in object_channels {
            obj.azimuth = Degrees(90.0);
            obj.elevation = Degrees(30.0);
            obj.distance = Distance::new(0.5).unwrap();
        }

        expect_true!(group.update_ambisonic_encoder(&mut audio_elements).is_ok());

        let mut input_buffer = AudioBuffer::new(10, BUFFER_SIZE);
        let mut output_buffer = AudioBuffer::new(2, BUFFER_SIZE);

        for ch in 0..input_buffer.num_channels() {
            generate_sine(SAMPLING_RATE, input_buffer.channel_mut(ch).as_mut_slice());
        }

        let rotation = Quaternion::default();
        group.process(
            &input_buffer,
            &audio_elements,
            false,
            &rotation,
            &mut output_buffer,
            &mut fft_manager,
        );

        expect_that!(has_non_zero_output(&output_buffer), eq(true));
    }

    #[gtest]
    fn test_update_encoder_no_sources() {
        let key = ProcessingGroupKey {
            ambisonic_order: 1,
            filter_profile: BinauralFilterProfile::Direct,
        };
        let indices = vec![0];
        let mut group = ProcessingGroup::new(key, indices, get_buffer_size(), get_sample_rate());
        let mut fft_manager = FftManager::new(BUFFER_SIZE);
        let mut resampler = Resampler::default();
        assert!(group.initialize(&mut fft_manager, &mut resampler).is_ok());

        let mut audio_elements =
            vec![AudioElementConfig::new(AudioElementType::Oa1, BinauralFilterProfile::Direct)];

        expect_true!(group.update_ambisonic_encoder(&mut audio_elements).is_ok());
    }

    #[gtest]
    fn test_update_encoder_object_positions() {
        let key = ProcessingGroupKey {
            ambisonic_order: 2,
            filter_profile: BinauralFilterProfile::Direct,
        };
        let indices = vec![0];
        let mut group = ProcessingGroup::new(key, indices, get_buffer_size(), get_sample_rate());
        let mut fft_manager = FftManager::new(BUFFER_SIZE);
        let mut resampler = Resampler::default();
        assert!(group.initialize(&mut fft_manager, &mut resampler).is_ok());

        let mut audio_elements = vec![AudioElementConfig::new(
            AudioElementType::ObjectMono,
            BinauralFilterProfile::Direct,
        )];

        let object_channels = audio_elements[0].get_object_channels();
        for obj in object_channels {
            obj.azimuth = Degrees(0.0);
            obj.elevation = Degrees(0.0);
            obj.distance = Distance::new(1.0).unwrap();
        }

        expect_true!(group.update_ambisonic_encoder(&mut audio_elements).is_ok());

        let object_channels = audio_elements[0].get_object_channels();
        for obj in object_channels {
            obj.azimuth = Degrees(180.0);
            obj.elevation = Degrees(45.0);
            obj.distance = Distance::new(0.8).unwrap();
        }

        expect_true!(group.update_ambisonic_encoder(&mut audio_elements).is_ok());
    }

    #[gtest]
    fn test_all_filter_profiles_with_same_input() {
        let profiles = vec![
            BinauralFilterProfile::Direct,
            BinauralFilterProfile::Ambient,
            BinauralFilterProfile::Reverberant,
        ];
        let mut fft_manager = FftManager::new(BUFFER_SIZE);
        let mut resampler = Resampler::default();

        for profile in profiles {
            let key = ProcessingGroupKey { ambisonic_order: 1, filter_profile: profile };
            let indices = vec![0];
            let mut group =
                ProcessingGroup::new(key, indices, get_buffer_size(), get_sample_rate());
            assert!(group.initialize(&mut fft_manager, &mut resampler).is_ok());

            let audio_elements = vec![AudioElementConfig::new(AudioElementType::Oa1, profile)];

            let mut input_buffer = AudioBuffer::new(4, BUFFER_SIZE);
            let mut output_buffer = AudioBuffer::new(2, BUFFER_SIZE);

            for ch in 0..input_buffer.num_channels() {
                generate_sine(SAMPLING_RATE, input_buffer.channel_mut(ch).as_mut_slice());
            }

            let rotation = Quaternion::default();
            group.process(
                &input_buffer,
                &audio_elements,
                false,
                &rotation,
                &mut output_buffer,
                &mut fft_manager,
            );

            expect_that!(has_non_zero_output(&output_buffer), eq(true));
        }
    }

    #[gtest]
    fn test_world_locked_element_with_rotation() {
        let key = ProcessingGroupKey {
            ambisonic_order: 1,
            filter_profile: BinauralFilterProfile::Direct,
        };
        let indices = vec![0];
        let mut group = ProcessingGroup::new(key, indices, get_buffer_size(), get_sample_rate());
        let mut fft_manager = FftManager::new(BUFFER_SIZE);
        let mut resampler = Resampler::default();
        assert!(group.initialize(&mut fft_manager, &mut resampler).is_ok());

        let audio_elements =
            vec![AudioElementConfig::new(AudioElementType::Oa1, BinauralFilterProfile::Direct)];
        expect_that!(audio_elements[0].is_head_locked(), eq(false));

        let mut input_buffer = AudioBuffer::new(4, BUFFER_SIZE);
        let mut output_buffer1 = AudioBuffer::new(2, BUFFER_SIZE);
        let mut output_buffer2 = AudioBuffer::new(2, BUFFER_SIZE);

        for ch in 0..input_buffer.num_channels() {
            generate_sine(SAMPLING_RATE, input_buffer.channel_mut(ch).as_mut_slice());
        }

        let identity_rotation = Quaternion::default();
        group.process(
            &input_buffer,
            &audio_elements,
            true,
            &identity_rotation,
            &mut output_buffer1,
            &mut fft_manager,
        );

        let rotated = Quaternion { w: 0.707, x: 0.0, y: 0.707, z: 0.0 };
        group.process(
            &input_buffer,
            &audio_elements,
            true,
            &rotated,
            &mut output_buffer2,
            &mut fft_manager,
        );

        expect_that!(has_non_zero_output(&output_buffer1), eq(true));
        expect_that!(has_non_zero_output(&output_buffer2), eq(true));

        let mut outputs_differ = false;
        for ch in 0..2 {
            for i in 0..BUFFER_SIZE {
                if (output_buffer1.channel(ch).as_slice()[i]
                    - output_buffer2.channel(ch).as_slice()[i])
                    .abs()
                    > 1e-5
                {
                    outputs_differ = true;
                    break;
                }
            }
            if outputs_differ {
                break;
            }
        }
        expect_that!(outputs_differ, eq(true));
    }

    #[gtest]
    fn test_head_locked_element_with_rotation() {
        let key = ProcessingGroupKey {
            ambisonic_order: 1,
            filter_profile: BinauralFilterProfile::Direct,
        };
        let indices = vec![0];
        let mut group = ProcessingGroup::new(key, indices, get_buffer_size(), get_sample_rate());
        let mut fft_manager = FftManager::new(BUFFER_SIZE);
        let mut resampler = Resampler::default();
        assert!(group.initialize(&mut fft_manager, &mut resampler).is_ok());

        let mut audio_elements =
            vec![AudioElementConfig::new(AudioElementType::Oa1, BinauralFilterProfile::Direct)];
        audio_elements[0].set_head_locked(true);
        expect_that!(audio_elements[0].is_head_locked(), eq(true));

        let mut input_buffer = AudioBuffer::new(4, BUFFER_SIZE);
        let mut output_buffer = AudioBuffer::new(2, BUFFER_SIZE);

        for ch in 0..input_buffer.num_channels() {
            generate_sine(SAMPLING_RATE, input_buffer.channel_mut(ch).as_mut_slice());
        }

        let rotated = Quaternion { w: 0.707, x: 0.0, y: 0.707, z: 0.0 };
        group.process(
            &input_buffer,
            &audio_elements,
            true,
            &rotated,
            &mut output_buffer,
            &mut fft_manager,
        );

        expect_that!(has_non_zero_output(&output_buffer), eq(true));
    }

    #[gtest]
    fn test_mixed_world_and_head_locked_elements() {
        let key = ProcessingGroupKey {
            ambisonic_order: 2,
            filter_profile: BinauralFilterProfile::Direct,
        };
        let indices = vec![0, 1];
        let mut group = ProcessingGroup::new(key, indices, get_buffer_size(), get_sample_rate());
        let mut fft_manager = FftManager::new(BUFFER_SIZE);
        let mut resampler = Resampler::default();
        assert!(group.initialize(&mut fft_manager, &mut resampler).is_ok());

        let mut audio_elements = vec![
            AudioElementConfig::new(AudioElementType::Oa2, BinauralFilterProfile::Direct),
            AudioElementConfig::new(AudioElementType::ObjectMono, BinauralFilterProfile::Direct),
        ];
        audio_elements[0].set_head_locked(false);
        audio_elements[1].set_head_locked(true);
        audio_elements[1].set_first_channel_index(9);

        let object_channels = audio_elements[1].get_object_channels();
        for obj in object_channels {
            obj.azimuth = Degrees(90.0);
            obj.elevation = Degrees(0.0);
            obj.distance = Distance::new(1.0).unwrap();
        }

        expect_true!(group.update_ambisonic_encoder(&mut audio_elements).is_ok());

        let mut input_buffer = AudioBuffer::new(10, BUFFER_SIZE);
        let mut output_buffer = AudioBuffer::new(2, BUFFER_SIZE);

        for ch in 0..input_buffer.num_channels() {
            generate_sine(SAMPLING_RATE, input_buffer.channel_mut(ch).as_mut_slice());
        }

        let rotated = Quaternion { w: 0.707, x: 0.0, y: 0.707, z: 0.0 };
        group.process(
            &input_buffer,
            &audio_elements,
            true,
            &rotated,
            &mut output_buffer,
            &mut fft_manager,
        );

        expect_that!(has_non_zero_output(&output_buffer), eq(true));
    }

    #[gtest]
    fn test_all_elements_world_locked() {
        let key = ProcessingGroupKey {
            ambisonic_order: 1,
            filter_profile: BinauralFilterProfile::Direct,
        };
        let indices = vec![0];
        let mut group = ProcessingGroup::new(key, indices, get_buffer_size(), get_sample_rate());
        let mut fft_manager = FftManager::new(BUFFER_SIZE);
        let mut resampler = Resampler::default();
        assert!(group.initialize(&mut fft_manager, &mut resampler).is_ok());

        let mut audio_elements =
            vec![AudioElementConfig::new(AudioElementType::Oa1, BinauralFilterProfile::Direct)];
        audio_elements[0].set_head_locked(false);

        let mut input_buffer = AudioBuffer::new(4, BUFFER_SIZE);
        let mut output_buffer = AudioBuffer::new(2, BUFFER_SIZE);

        for ch in 0..input_buffer.num_channels() {
            generate_sine(SAMPLING_RATE, input_buffer.channel_mut(ch).as_mut_slice());
        }

        let rotated = Quaternion { w: 0.707, x: 0.0, y: 0.707, z: 0.0 };
        group.process(
            &input_buffer,
            &audio_elements,
            true,
            &rotated,
            &mut output_buffer,
            &mut fft_manager,
        );

        expect_that!(has_non_zero_output(&output_buffer), eq(true));
    }

    #[gtest]
    fn test_all_elements_head_locked() {
        let key = ProcessingGroupKey {
            ambisonic_order: 1,
            filter_profile: BinauralFilterProfile::Direct,
        };
        let indices = vec![0];
        let mut group = ProcessingGroup::new(key, indices, get_buffer_size(), get_sample_rate());
        let mut fft_manager = FftManager::new(BUFFER_SIZE);
        let mut resampler = Resampler::default();
        assert!(group.initialize(&mut fft_manager, &mut resampler).is_ok());

        let mut audio_elements =
            vec![AudioElementConfig::new(AudioElementType::Oa1, BinauralFilterProfile::Direct)];
        audio_elements[0].set_head_locked(true);

        let mut input_buffer = AudioBuffer::new(4, BUFFER_SIZE);
        let mut output_buffer = AudioBuffer::new(2, BUFFER_SIZE);

        for ch in 0..input_buffer.num_channels() {
            generate_sine(SAMPLING_RATE, input_buffer.channel_mut(ch).as_mut_slice());
        }

        let rotated = Quaternion { w: 0.707, x: 0.0, y: 0.707, z: 0.0 };
        group.process(
            &input_buffer,
            &audio_elements,
            true,
            &rotated,
            &mut output_buffer,
            &mut fft_manager,
        );

        expect_that!(has_non_zero_output(&output_buffer), eq(true));
    }

    #[gtest]
    fn test_head_tracking_disabled_override() {
        let key = ProcessingGroupKey {
            ambisonic_order: 1,
            filter_profile: BinauralFilterProfile::Direct,
        };
        let indices = vec![0];
        let mut group = ProcessingGroup::new(key, indices, get_buffer_size(), get_sample_rate());
        let mut fft_manager = FftManager::new(BUFFER_SIZE);
        let mut resampler = Resampler::default();
        assert!(group.initialize(&mut fft_manager, &mut resampler).is_ok());

        let mut audio_elements =
            vec![AudioElementConfig::new(AudioElementType::Oa1, BinauralFilterProfile::Direct)];
        audio_elements[0].set_head_locked(false);

        let mut input_buffer = AudioBuffer::new(4, BUFFER_SIZE);
        let mut output_buffer1 = AudioBuffer::new(2, BUFFER_SIZE);
        let mut output_buffer2 = AudioBuffer::new(2, BUFFER_SIZE);

        for ch in 0..input_buffer.num_channels() {
            generate_sine(SAMPLING_RATE, input_buffer.channel_mut(ch).as_mut_slice());
        }

        let rotated = Quaternion { w: 0.707, x: 0.0, y: 0.707, z: 0.0 };

        group.process(
            &input_buffer,
            &audio_elements,
            true,
            &rotated,
            &mut output_buffer1,
            &mut fft_manager,
        );
        group.process(
            &input_buffer,
            &audio_elements,
            false,
            &rotated,
            &mut output_buffer2,
            &mut fft_manager,
        );

        let mut outputs_differ = false;
        for ch in 0..2 {
            for i in 0..BUFFER_SIZE {
                if (output_buffer1.channel(ch).as_slice()[i]
                    - output_buffer2.channel(ch).as_slice()[i])
                    .abs()
                    > 1e-5
                {
                    outputs_differ = true;
                    break;
                }
            }
            if outputs_differ {
                break;
            }
        }
        expect_that!(outputs_differ, eq(true));
    }
}
