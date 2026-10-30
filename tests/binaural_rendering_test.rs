// Copyright (c) 2026, Alliance for Open Media. All rights reserved
//
// This source code is subject to the terms of the BSD 3-Clause Clear License
// and the Alliance for Open Media Patent License 1.0. If the BSD 3-Clause Clear
// License was not distributed with this source code in the LICENSE file, you
// can obtain it at www.aomedia.org/license/software-license/bsd-3-c-c. If the
// Alliance for Open Media Patent License 1.0 was not distributed with this
// source code in the PATENTS file, you can obtain it at
// www.aomedia.org/license/patent.

mod test_helpers;

/// Tests that rendering (and functions that would be called during rendering) do not allocate.
///
/// These tests use a custom allocator and a closure to ensure that calls are not allocating memory
/// during the rendering phase.
#[cfg(test)]
mod test {

    use crate::test_helpers::{create_planar_buffer_mut, create_planar_buffer_ref};
    use aom_roar::{
        AudioElementConfig, Config, HighOrderAmbisonics, OutputLayout, RoarRenderer, SampleRate,
        Samples, SceneBasedConfig,
    };
    use googletest::prelude::*;
    use rstest::rstest;

    /// Verifies that binaural can accept a small and large samples_per_channel.
    #[rstest]
    #[case(8)]
    #[case(65536)]
    #[googletest::test]
    fn binaural_rendering_small_and_large_samples_per_channel(#[case] samples_per_channel: usize) {
        let sample_rate: u32 = 48000;
        let input_channels: usize = 4;
        let output_channels: usize = 2;
        let element_id: u32 = 1;

        let config = Config::new(
            OutputLayout::Binaural,
            Samples::new(samples_per_channel as u32).unwrap(),
            SampleRate::new(sample_rate).unwrap(),
        )
        .unwrap();
        let mut renderer = RoarRenderer::create(&config).unwrap();
        let group_id = renderer.add_audio_group().unwrap();

        let element_cfg = AudioElementConfig::SceneBased(SceneBasedConfig {
            order: HighOrderAmbisonics::Order1,
            rendering_config: None,
        });
        renderer.add_element(group_id, element_id, &element_cfg).unwrap();

        let input_data = vec![vec![0.1f32; samples_per_channel]; input_channels];
        let input_slices: Vec<&[f32]> = input_data.iter().map(|v| v.as_slice()).collect();
        let inputs = [(element_id, create_planar_buffer_ref(&input_slices, samples_per_channel))];
        let mut output = vec![0.0f32; output_channels * samples_per_channel];
        let mut output_slices: Vec<&mut [f32]> =
            (0..output_channels).map(|_| &mut [] as &mut [f32]).collect();
        let mut output_buffer = create_planar_buffer_mut(
            &mut output,
            output_channels,
            samples_per_channel,
            &mut output_slices,
        );

        let render_result = renderer.render(&inputs, &mut output_buffer);
        expect_ok!(render_result);
    }
}
