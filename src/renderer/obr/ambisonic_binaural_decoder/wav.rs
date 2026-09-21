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

//! RIFF WAVE audio asset reader.
//!
//! Provides a simple parser `Wav` to decode 16-bit PCM WAV audio data streams used for loading
//! HRIR filters.

/// Decoded 16-bit PCM WAV audio representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wav {
    pub num_channels: usize,
    pub sample_rate: i32,
    pub interleaved_samples: Vec<i16>,
}

impl Wav {
    /// Parses a RIFF WAVE format byte stream into a `Wav` instance.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        const MIN_WAV_HEADER_SIZE: usize = 44;
        const RIFF_HEADER_SIZE: usize = 12;
        const SUBCHUNK_HEADER_SIZE: usize = 8;
        const MIN_FMT_CHUNK_SIZE: usize = 16;
        const PCM_FORMAT_TAG: u16 = 1;
        const WAVE_FORMAT_EXTENSIBLE_TAG: u16 = 0xFFFE;
        const BYTES_PER_SAMPLE_16BIT: usize = 2;

        if bytes.len() < MIN_WAV_HEADER_SIZE || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE"
        {
            return None;
        }

        let mut idx = RIFF_HEADER_SIZE;
        let mut num_channels = 0_usize;
        let mut sample_rate = 0_i32;
        let mut bits_per_sample = 0_u16;
        let mut samples: Option<Vec<i16>> = None;

        while idx + SUBCHUNK_HEADER_SIZE <= bytes.len() {
            let chunk_id = &bytes[idx..idx + 4];
            let chunk_size = u32::from_le_bytes([
                bytes[idx + 4],
                bytes[idx + 5],
                bytes[idx + 6],
                bytes[idx + 7],
            ]) as usize;
            idx += SUBCHUNK_HEADER_SIZE;

            if idx + chunk_size > bytes.len() {
                break;
            }

            if chunk_id == b"fmt " {
                if chunk_size < MIN_FMT_CHUNK_SIZE {
                    return None;
                }
                let format_tag = u16::from_le_bytes([bytes[idx], bytes[idx + 1]]);
                if format_tag != PCM_FORMAT_TAG && format_tag != WAVE_FORMAT_EXTENSIBLE_TAG {
                    return None;
                }
                num_channels = u16::from_le_bytes([bytes[idx + 2], bytes[idx + 3]]) as usize;
                sample_rate = u32::from_le_bytes([
                    bytes[idx + 4],
                    bytes[idx + 5],
                    bytes[idx + 6],
                    bytes[idx + 7],
                ]) as i32;
                bits_per_sample = u16::from_le_bytes([bytes[idx + 14], bytes[idx + 15]]);
                if bits_per_sample != 16 || num_channels == 0 || sample_rate <= 0 {
                    return None;
                }
            } else if chunk_id == b"data" {
                if num_channels == 0 || bits_per_sample != 16 {
                    return None;
                }
                let num_samples = chunk_size / BYTES_PER_SAMPLE_16BIT;
                let mut data_vec = Vec::with_capacity(num_samples);
                // TODO(b/525080422): Optimize by using chunks_exact to parse bytes instead of
                // manual index offset calculations.
                for i in 0..num_samples {
                    let off = idx + i * BYTES_PER_SAMPLE_16BIT;
                    let sample = i16::from_le_bytes([bytes[off], bytes[off + 1]]);
                    data_vec.push(sample);
                }
                samples = Some(data_vec);
                break;
            }

            idx += chunk_size + (chunk_size % 2);
        }

        samples.map(|interleaved_samples| Self { num_channels, sample_rate, interleaved_samples })
    }

    /// Returns the number of channels.
    pub fn num_channels(&self) -> usize {
        self.num_channels
    }

    /// Returns the audio sampling rate in Hz.
    pub fn sample_rate(&self) -> i32 {
        self.sample_rate
    }

    /// Returns a slice of the raw interleaved 16-bit PCM samples.
    pub fn interleaved_samples(&self) -> &[i16] {
        &self.interleaved_samples
    }
}

// ===== Tests =====

#[cfg(test)]
mod test {

    use super::*;
    use googletest::prelude::*;

    #[gtest]
    fn test_wav_parse_valid() {
        let mut wav_bytes = Vec::new();
        wav_bytes.extend_from_slice(b"RIFF");
        wav_bytes.extend_from_slice(&(36 + 8_u32).to_le_bytes());
        wav_bytes.extend_from_slice(b"WAVE");

        // fmt subchunk
        wav_bytes.extend_from_slice(b"fmt ");
        wav_bytes.extend_from_slice(&16_u32.to_le_bytes());
        wav_bytes.extend_from_slice(&1_u16.to_le_bytes());
        wav_bytes.extend_from_slice(&2_u16.to_le_bytes());
        wav_bytes.extend_from_slice(&48000_u32.to_le_bytes());
        wav_bytes.extend_from_slice(&(48000_u32 * 4).to_le_bytes());
        wav_bytes.extend_from_slice(&4_u16.to_le_bytes());
        wav_bytes.extend_from_slice(&16_u16.to_le_bytes());

        // data subchunk
        wav_bytes.extend_from_slice(b"data");
        wav_bytes.extend_from_slice(&8_u32.to_le_bytes());
        wav_bytes.extend_from_slice(&[0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04, 0x00]);

        let wav = Wav::from_bytes(&wav_bytes);
        expect_true!(wav.is_some());
        let wav = wav.unwrap();
        expect_eq!(wav.num_channels(), 2);
        expect_eq!(wav.sample_rate(), 48000);
        expect_eq!(wav.interleaved_samples(), &[1, 2, 3, 4]);
    }

    #[gtest]
    fn test_wav_parse_invalid_header() {
        let bad_bytes = b"RIFFxxxxWAVXfmt ";
        expect_true!(Wav::from_bytes(bad_bytes).is_none());
    }

    #[gtest]
    fn test_wav_parse_non_pcm() {
        let mut wav_bytes = Vec::new();
        wav_bytes.extend_from_slice(b"RIFF");
        wav_bytes.extend_from_slice(&(36 + 8_u32).to_le_bytes());
        wav_bytes.extend_from_slice(b"WAVE");

        // fmt subchunk
        wav_bytes.extend_from_slice(b"fmt ");
        wav_bytes.extend_from_slice(&16_u32.to_le_bytes());
        wav_bytes.extend_from_slice(&2_u16.to_le_bytes()); // Format tag = 2 (not PCM)
        wav_bytes.extend_from_slice(&2_u16.to_le_bytes());
        wav_bytes.extend_from_slice(&48000_u32.to_le_bytes());
        wav_bytes.extend_from_slice(&(48000_u32 * 4).to_le_bytes());
        wav_bytes.extend_from_slice(&4_u16.to_le_bytes());
        wav_bytes.extend_from_slice(&16_u16.to_le_bytes());

        expect_true!(Wav::from_bytes(&wav_bytes).is_none());
    }
}
