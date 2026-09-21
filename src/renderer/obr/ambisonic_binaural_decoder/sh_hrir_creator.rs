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

//! Spherical harmonic HRIR loading and creation.
//!
//! Provides routines to synthesize and load spherical harmonic Head-Related Impulse Response (HRIR)
//! filters from audio files or embedded assets.

use super::binaural_filters::BinauralFiltersWrapper;
use super::planar_interleaved_conversion::fill_audio_buffer_from_i16;
use super::resampler::Resampler;
use super::wav::Wav;
use crate::renderer::obr::audio_buffer::AudioBuffer;
use crate::renderer::obr::common::ambisonic_utils::is_valid_ambisonic_order;

use crate::common::definitions::SampleRate;

/// Synthesizes an `AudioBuffer` of planar SH-HRIR filters from a parsed `Wav` structure.
///
/// Resamples the filters if the target rate differs from the WAV's sampling rate.
pub fn create_sh_hrirs_from_wav(
    wav: &Wav,
    target_sample_rate: SampleRate,
    resampler: &mut Resampler,
) -> AudioBuffer {
    let num_channels = wav.num_channels();
    assert!(is_valid_ambisonic_order(num_channels));

    let sh_hrir_length = wav.interleaved_samples().len() / num_channels;
    let mut sh_hrirs = AudioBuffer::new(num_channels, sh_hrir_length);
    fill_audio_buffer_from_i16(
        wav.interleaved_samples(),
        sh_hrir_length,
        num_channels,
        &mut sh_hrirs,
    );

    let wav_rate = wav.sample_rate();
    let target_sample_rate_hz = target_sample_rate.value() as i32;
    assert!(wav_rate > 0 && target_sample_rate_hz > 0);
    if wav_rate != target_sample_rate_hz {
        assert!(
            Resampler::are_sample_rates_supported(wav_rate, target_sample_rate_hz),
            "Unsupported sampling rates for loading HRIRs: {}, {}",
            wav_rate,
            target_sample_rate_hz
        );
        resampler.reset_state();
        resampler.set_rate_and_num_channels(wav_rate, target_sample_rate_hz, num_channels);
        let mut resampled =
            AudioBuffer::new(num_channels, resampler.get_next_output_length(sh_hrir_length));
        resampler.process(&sh_hrirs, &mut resampled);
        return resampled;
    }
    sh_hrirs
}

/// Synthesizes an `AudioBuffer` of planar SH-HRIR filters from embedded binary WAV assets.
pub fn create_sh_hrirs_from_assets(
    filename: &str,
    target_sample_rate: SampleRate,
    resampler: &mut Resampler,
) -> Option<AudioBuffer> {
    let wrapper = BinauralFiltersWrapper::new();
    let data = wrapper.get_file(filename)?;
    let wav = Wav::from_bytes(data)?;
    Some(create_sh_hrirs_from_wav(&wav, target_sample_rate, resampler))
}
