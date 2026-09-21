/*
 * Copyright (c) 2026, Alliance for Open Media. All rights reserved
 *
 * This source code is subject to the terms of the BSD 3-Clause Clear License
 * and the Alliance for Open Media Patent License 1.0. If the BSD 3-Clause Clear
 * License was not distributed with this source code in the LICENSE file, you
 * can obtain it at www.aomedia.org/license/software-license/bsd-3-c-c. If the
 * Alliance for Open Media Patent License 1.0 was not distributed with this
 * source code in the PATENTS file, you can obtain it at
 * www.aomedia.org/license/patent.
 */

#include <cstddef>
#include <cstdint>

#include "absl/base/log_severity.h"
#include "absl/log/absl_check.h"
#include "absl/log/globals.h"
#include "absl/types/span.h"
#include "benchmark/benchmark.h"
#include "roar/equivalence_testing/benchmarks/benchmark_helpers.h"

extern "C" {
#include "oar/include/oar.h"
#include "oar/include/oar_base.h"
#include "oar/include/oar_metadata.h"
}

namespace roar_equivalence_testing {
namespace {

// Cut down on INFO log spam (OAR is noisy).
const bool kLogsSuppressed = []() {
  absl::SetMinLogLevel(absl::LogSeverityAtLeast::kWarning);
  return true;
}();

// Computes and records standard audio throughput counters (`xRealtime` and
// items processed).
void SetAudioMetrics(benchmark::State& state, int64_t num_frames,
                     uint32_t frame_size = kDefaultFrameSize,
                     uint32_t sample_rate = kDefaultSampleRate) {
  state.SetItemsProcessed(static_cast<int64_t>(state.iterations()) *
                          num_frames * frame_size);
  const double total_audio_duration =
      (static_cast<double>(frame_size) / sample_rate) *
      static_cast<double>(num_frames) * static_cast<double>(state.iterations());
  state.counters["xRealtime"] =
      benchmark::Counter(total_audio_duration, benchmark::Counter::kIsRate);
}

// Executes a benchmark render loop across `num_frames` consecutive frames.
void RunRenderLoop(benchmark::State& state, RendererInstance& renderer,
                   oar_audio_block_t& output_block, int64_t num_frames,
                   uint32_t frame_size = kDefaultFrameSize,
                   uint32_t sample_rate = kDefaultSampleRate) {
  // Warm-up pass.
  ABSL_CHECK_EQ(renderer.Render(&output_block), 0);

  for (auto unused : state) {
    for (int64_t f = 0; f < num_frames; ++f) {
      benchmark::DoNotOptimize(renderer.Render(&output_block));
    }
  }

  SetAudioMetrics(state, num_frames, frame_size, sample_rate);
}

// Executes a benchmark render loop with metadata updates prior to each frame.
void RunDynamicRenderLoop(benchmark::State& state, RendererInstance& renderer,
                          oar_audio_block_t& output_block,
                          absl::Span<const uint32_t> element_ids,
                          absl::Span<const oar_metadata_t> metadatas,
                          int64_t num_frames,
                          uint32_t frame_size = kDefaultFrameSize,
                          uint32_t sample_rate = kDefaultSampleRate) {
  ABSL_CHECK_EQ(element_ids.size(), metadatas.size());

  // Warm-up pass.
  ABSL_CHECK_EQ(renderer.Render(&output_block), 0);

  for (auto unused : state) {
    for (int64_t f = 0; f < num_frames; ++f) {
      for (size_t i = 0; i < element_ids.size(); ++i) {
        renderer.UpdateAudioElementMetadata(element_ids[i], metadatas[i]);
      }
      benchmark::DoNotOptimize(renderer.Render(&output_block));
    }
  }

  SetAudioMetrics(state, num_frames, frame_size, sample_rate);
}

// Executes a benchmark render loop with head rotation updates prior to each
// frame.
void RunHeadTrackingRenderLoop(benchmark::State& state,
                               RendererInstance& renderer, uint32_t group_id,
                               oar_audio_block_t& output_block,
                               absl::Span<const oar_metadata_t> head_rotations,
                               int64_t num_frames,
                               uint32_t frame_size = kDefaultFrameSize,
                               uint32_t sample_rate = kDefaultSampleRate) {
  ABSL_CHECK(!head_rotations.empty());

  // Warm-up pass.
  ABSL_CHECK_EQ(renderer.UpdateMetadata(group_id, head_rotations[0]), 0);
  ABSL_CHECK_EQ(renderer.Render(&output_block), 0);

  for (auto unused : state) {
    for (int64_t f = 0; f < num_frames; ++f) {
      const auto& metadata = head_rotations[f % head_rotations.size()];
      renderer.UpdateMetadata(group_id, metadata);
      benchmark::DoNotOptimize(renderer.Render(&output_block));
    }
  }

  SetAudioMetrics(state, num_frames, frame_size, sample_rate);
}

// -----------------------------------------------------------------------------
//   Audio Rendering Benchmarks (Setup executed outside the timed loop)
// -----------------------------------------------------------------------------

// World-locked 9.1.6 channel-based input to binaural rendering (OBR backend).
void Render9_1_6ToBinaural(benchmark::State& state, RendererEngine engine) {
  const int64_t num_frames = state.range(0);
  constexpr uint32_t kInputChannels = 16;  // 9.1.6
  constexpr uint32_t kOutputChannels = 2;  // Binaural
  constexpr uint32_t kElementId = 15;

  const oar_config_t config = {
      .target_layout = ck_oar_layout_binaural,
      .samples_per_channel = kDefaultFrameSize,
      .sampling_rate = kDefaultSampleRate,
  };
  auto renderer = RendererInstance::Create(engine, config);
  ABSL_CHECK(renderer != nullptr);

  const int group_id = renderer->AddAudioGroup();
  ABSL_CHECK_GE(group_id, 0);

  auto element_cfg = CreateChannelConfig(ck_oar_layout_916);
  AddWorldLockedParameter(element_cfg);
  ABSL_CHECK_EQ(renderer->AddAudioElement(group_id, kElementId, element_cfg),
                0);

  auto input = AudioBlock::CreateInput(kInputChannels);
  ABSL_CHECK_EQ(renderer->UpdateAudioElementData(kElementId, &input.block), 0);

  auto output = AudioBlock::CreateOutput(kOutputChannels);
  RunRenderLoop(state, *renderer, output.block, num_frames);
}
BENCHMARK_CAPTURE(Render9_1_6ToBinaural, oar, RendererEngine::kOar)->Arg(100);
BENCHMARK_CAPTURE(Render9_1_6ToBinaural, roar, RendererEngine::kRoar)->Arg(100);

// World-locked 9.1.6 channel-based input to binaural rendering at 44.1 kHz
// (exercising the resampled HRIR filter bank).
void Render9_1_6ToBinaural_Resampled_44100(benchmark::State& state,
                                           RendererEngine engine) {
  const int64_t num_frames = state.range(0);
  constexpr uint32_t kInputChannels = 16;  // 9.1.6
  constexpr uint32_t kOutputChannels = 2;  // Binaural
  constexpr uint32_t kElementId = 15;
  constexpr uint32_t kSampleRate44100 = 44100;

  const oar_config_t config = {
      .target_layout = ck_oar_layout_binaural,
      .samples_per_channel = kDefaultFrameSize,
      .sampling_rate = kSampleRate44100,
  };
  auto renderer = RendererInstance::Create(engine, config);
  ABSL_CHECK(renderer != nullptr);

  const int group_id = renderer->AddAudioGroup();
  ABSL_CHECK_GE(group_id, 0);

  auto element_cfg = CreateChannelConfig(ck_oar_layout_916);
  AddWorldLockedParameter(element_cfg);
  ABSL_CHECK_EQ(renderer->AddAudioElement(group_id, kElementId, element_cfg),
                0);

  auto input = AudioBlock::CreateInput(kInputChannels, kDefaultFrameSize,
                                       kSampleRate44100);
  ABSL_CHECK_EQ(renderer->UpdateAudioElementData(kElementId, &input.block), 0);

  auto output = AudioBlock::CreateOutput(kOutputChannels);
  RunRenderLoop(state, *renderer, output.block, num_frames, kDefaultFrameSize,
                kSampleRate44100);
}
BENCHMARK_CAPTURE(Render9_1_6ToBinaural_Resampled_44100, oar,
                  RendererEngine::kOar)
    ->Arg(100);
BENCHMARK_CAPTURE(Render9_1_6ToBinaural_Resampled_44100, roar,
                  RendererEngine::kRoar)
    ->Arg(100);

// World-locked 9.1.6 channel-based input to binaural rendering with dynamic
// head rotation updates per frame.
void Render9_1_6ToBinauralWithHeadRotation(benchmark::State& state,
                                           RendererEngine engine) {
  const int64_t num_frames = state.range(0);
  constexpr uint32_t kInputChannels = 16;  // 9.1.6
  constexpr uint32_t kOutputChannels = 2;  // Binaural
  constexpr uint32_t kElementId = 15;

  const oar_config_t config = {
      .target_layout = ck_oar_layout_binaural,
      .samples_per_channel = kDefaultFrameSize,
      .sampling_rate = kDefaultSampleRate,
  };
  auto renderer = RendererInstance::Create(engine, config);
  ABSL_CHECK(renderer != nullptr);

  const int group_id = renderer->AddAudioGroup();
  ABSL_CHECK_GE(group_id, 0);

  auto element_cfg = CreateChannelConfig(ck_oar_layout_916);
  AddWorldLockedParameter(element_cfg);
  ABSL_CHECK_EQ(renderer->AddAudioElement(group_id, kElementId, element_cfg),
                0);

  auto input = AudioBlock::CreateInput(kInputChannels);
  ABSL_CHECK_EQ(renderer->UpdateAudioElementData(kElementId, &input.block), 0);

  ABSL_CHECK_EQ(renderer->EnableHeadTracking(true), 0);

  const auto head_rotations = CreateHeadRotationMetadatas();
  auto output = AudioBlock::CreateOutput(kOutputChannels);
  RunHeadTrackingRenderLoop(state, *renderer, group_id, output.block,
                            head_rotations, num_frames);
}
BENCHMARK_CAPTURE(Render9_1_6ToBinauralWithHeadRotation, oar,
                  RendererEngine::kOar)
    ->Arg(100);
BENCHMARK_CAPTURE(Render9_1_6ToBinauralWithHeadRotation, roar,
                  RendererEngine::kRoar)
    ->Arg(100);

// Ambisonics input to binaural (OBR backend).
void Render4oaToBinaural(benchmark::State& state, RendererEngine engine) {
  const int64_t num_frames = state.range(0);
  constexpr uint32_t kInputChannels = 25;  // 4OA
  constexpr uint32_t kOutputChannels = 2;  // Binaural
  constexpr uint32_t kElementId = 15;

  const oar_config_t config = {
      .target_layout = ck_oar_layout_binaural,
      .samples_per_channel = kDefaultFrameSize,
      .sampling_rate = kDefaultSampleRate,
  };
  auto renderer = RendererInstance::Create(engine, config);
  ABSL_CHECK(renderer != nullptr);

  const int group_id = renderer->AddAudioGroup();
  ABSL_CHECK_GE(group_id, 0);

  auto element_cfg = CreateAmbisonicConfig(ck_oar_4oa);
  AddWorldLockedParameter(element_cfg);
  ABSL_CHECK_EQ(renderer->AddAudioElement(group_id, kElementId, element_cfg),
                0);

  auto input = AudioBlock::CreateInput(kInputChannels);
  ABSL_CHECK_EQ(renderer->UpdateAudioElementData(kElementId, &input.block), 0);

  auto output = AudioBlock::CreateOutput(kOutputChannels);
  RunRenderLoop(state, *renderer, output.block, num_frames);
}
BENCHMARK_CAPTURE(Render4oaToBinaural, oar, RendererEngine::kOar)->Arg(100);
BENCHMARK_CAPTURE(Render4oaToBinaural, roar, RendererEngine::kRoar)->Arg(100);

// World-locked ambisonics input to binaural with head rotation.
void Render4oaToBinauralWithHeadRotation(benchmark::State& state,
                                         RendererEngine engine) {
  const int64_t num_frames = state.range(0);
  constexpr uint32_t kInputChannels = 25;  // 4OA
  constexpr uint32_t kOutputChannels = 2;  // Binaural
  constexpr uint32_t kElementId = 15;

  const oar_config_t config = {
      .target_layout = ck_oar_layout_binaural,
      .samples_per_channel = kDefaultFrameSize,
      .sampling_rate = kDefaultSampleRate,
  };
  auto renderer = RendererInstance::Create(engine, config);
  ABSL_CHECK(renderer != nullptr);

  const int group_id = renderer->AddAudioGroup();
  ABSL_CHECK_GE(group_id, 0);

  auto element_cfg = CreateAmbisonicConfig(ck_oar_4oa);
  AddWorldLockedParameter(element_cfg);
  ABSL_CHECK_EQ(renderer->AddAudioElement(group_id, kElementId, element_cfg),
                0);

  auto input = AudioBlock::CreateInput(kInputChannels);
  ABSL_CHECK_EQ(renderer->UpdateAudioElementData(kElementId, &input.block), 0);

  ABSL_CHECK_EQ(renderer->EnableHeadTracking(true), 0);

  const auto head_rotations = CreateHeadRotationMetadatas();
  auto output = AudioBlock::CreateOutput(kOutputChannels);
  RunHeadTrackingRenderLoop(state, *renderer, group_id, output.block,
                            head_rotations, num_frames);
}
BENCHMARK_CAPTURE(Render4oaToBinauralWithHeadRotation, oar,
                  RendererEngine::kOar)
    ->Arg(100);
BENCHMARK_CAPTURE(Render4oaToBinauralWithHeadRotation, roar,
                  RendererEngine::kRoar)
    ->Arg(100);

// World-locked object-based input (8 pairs / 16 objects) to binaural rendering
// (OBR backend).
void RenderStaticObjectsToBinaural(benchmark::State& state,
                                   RendererEngine engine) {
  const int64_t num_frames = state.range(0);
  constexpr size_t kNumPairs = 8;
  constexpr uint32_t kOutputChannels = 2;  // Binaural

  const oar_config_t config = {
      .target_layout = ck_oar_layout_binaural,
      .samples_per_channel = kDefaultFrameSize,
      .sampling_rate = kDefaultSampleRate,
  };
  auto renderer = RendererInstance::Create(engine, config);
  ABSL_CHECK(renderer != nullptr);

  const int group_id = renderer->AddAudioGroup();
  ABSL_CHECK_GE(group_id, 0);

  const auto metadatas = CreateObjectPairMetadatas(kNumPairs);
  auto input = AudioBlock::CreateInput(/*channels=*/2);
  auto element_cfg = CreateObjectConfig();
  AddWorldLockedParameter(element_cfg);
  SetupObjectElements(*renderer, group_id, kNumPairs, element_cfg, &input.block,
                      metadatas);

  auto output = AudioBlock::CreateOutput(kOutputChannels);
  RunRenderLoop(state, *renderer, output.block, num_frames);
}
BENCHMARK_CAPTURE(RenderStaticObjectsToBinaural, oar, RendererEngine::kOar)
    ->Arg(100);
BENCHMARK_CAPTURE(RenderStaticObjectsToBinaural, roar, RendererEngine::kRoar)
    ->Arg(100);

// World-locked object-based input (8 pairs / 16 objects) with dynamic metadata
// updates per frame.
void RenderDynamicObjectsToBinaural(benchmark::State& state,
                                    RendererEngine engine) {
  const int64_t num_frames = state.range(0);
  constexpr size_t kNumPairs = 8;
  constexpr uint32_t kOutputChannels = 2;  // Binaural

  const oar_config_t config = {
      .target_layout = ck_oar_layout_binaural,
      .samples_per_channel = kDefaultFrameSize,
      .sampling_rate = kDefaultSampleRate,
  };
  auto renderer = RendererInstance::Create(engine, config);
  ABSL_CHECK(renderer != nullptr);

  const int group_id = renderer->AddAudioGroup();
  ABSL_CHECK_GE(group_id, 0);

  const auto metadatas = CreateObjectPairMetadatas(kNumPairs);
  auto input = AudioBlock::CreateInput(/*channels=*/2);
  auto element_cfg = CreateObjectConfig();
  AddWorldLockedParameter(element_cfg);
  const auto element_ids = SetupObjectElements(
      *renderer, group_id, kNumPairs, element_cfg, &input.block, metadatas);

  auto output = AudioBlock::CreateOutput(kOutputChannels);
  RunDynamicRenderLoop(state, *renderer, output.block, element_ids, metadatas,
                       num_frames);
}
BENCHMARK_CAPTURE(RenderDynamicObjectsToBinaural, oar, RendererEngine::kOar)
    ->Arg(100);
BENCHMARK_CAPTURE(RenderDynamicObjectsToBinaural, roar, RendererEngine::kRoar)
    ->Arg(100);

// 9.1.6 channel-based input downmixed to stereo loudspeakers (EAR backend).
void Render9_1_6ToStereo(benchmark::State& state, RendererEngine engine) {
  const int64_t num_frames = state.range(0);
  constexpr uint32_t kInputChannels = 16;  // 9.1.6
  constexpr uint32_t kOutputChannels = 2;  // Stereo
  constexpr uint32_t kElementId = 15;

  const oar_config_t config = {
      .target_layout = ck_oar_layout_stereo,
      .samples_per_channel = kDefaultFrameSize,
      .sampling_rate = kDefaultSampleRate,
  };
  auto renderer = RendererInstance::Create(engine, config);
  ABSL_CHECK(renderer != nullptr);

  const int group_id = renderer->AddAudioGroup();
  ABSL_CHECK_GE(group_id, 0);

  const auto element_cfg = CreateChannelConfig(ck_oar_layout_916);
  ABSL_CHECK_EQ(renderer->AddAudioElement(group_id, kElementId, element_cfg),
                0);

  auto input = AudioBlock::CreateInput(kInputChannels);
  ABSL_CHECK_EQ(renderer->UpdateAudioElementData(kElementId, &input.block), 0);

  auto output = AudioBlock::CreateOutput(kOutputChannels);
  RunRenderLoop(state, *renderer, output.block, num_frames);
}
BENCHMARK_CAPTURE(Render9_1_6ToStereo, oar, RendererEngine::kOar)->Arg(100);
BENCHMARK_CAPTURE(Render9_1_6ToStereo, roar, RendererEngine::kRoar)->Arg(100);

// Ambisonics input decoded to 9.1.6 loudspeakers (EAR backend).
void Render4oaTo9_1_6(benchmark::State& state, RendererEngine engine) {
  const int64_t num_frames = state.range(0);
  constexpr uint32_t kInputChannels = 25;   // 4OA
  constexpr uint32_t kOutputChannels = 16;  // 9.1.6
  constexpr uint32_t kElementId = 15;

  const oar_config_t config = {
      .target_layout = ck_oar_layout_916,
      .samples_per_channel = kDefaultFrameSize,
      .sampling_rate = kDefaultSampleRate,
  };
  auto renderer = RendererInstance::Create(engine, config);
  ABSL_CHECK(renderer != nullptr);

  const int group_id = renderer->AddAudioGroup();
  ABSL_CHECK_GE(group_id, 0);

  const auto element_cfg = CreateAmbisonicConfig(ck_oar_4oa);
  ABSL_CHECK_EQ(renderer->AddAudioElement(group_id, kElementId, element_cfg),
                0);

  auto input = AudioBlock::CreateInput(kInputChannels);
  ABSL_CHECK_EQ(renderer->UpdateAudioElementData(kElementId, &input.block), 0);

  auto output = AudioBlock::CreateOutput(kOutputChannels);
  RunRenderLoop(state, *renderer, output.block, num_frames);
}
BENCHMARK_CAPTURE(Render4oaTo9_1_6, oar, RendererEngine::kOar)->Arg(100);
BENCHMARK_CAPTURE(Render4oaTo9_1_6, roar, RendererEngine::kRoar)->Arg(100);

// Object-based input (8 pairs / 16 objects) rendered to 9.1.6 loudspeakers (OLR
// backend).
void RenderStaticObjectsTo9_1_6(benchmark::State& state,
                                RendererEngine engine) {
  const int64_t num_frames = state.range(0);
  constexpr size_t kNumPairs = 8;
  constexpr uint32_t kOutputChannels = 16;  // 9.1.6

  const oar_config_t config = {
      .target_layout = ck_oar_layout_916,
      .samples_per_channel = kDefaultFrameSize,
      .sampling_rate = kDefaultSampleRate,
  };
  auto renderer = RendererInstance::Create(engine, config);
  ABSL_CHECK(renderer != nullptr);

  const int group_id = renderer->AddAudioGroup();
  ABSL_CHECK_GE(group_id, 0);

  const auto metadatas = CreateObjectPairMetadatas(kNumPairs);
  auto input = AudioBlock::CreateInput(/*channels=*/2);
  SetupObjectElements(*renderer, group_id, kNumPairs, CreateObjectConfig(),
                      &input.block, metadatas);

  auto output = AudioBlock::CreateOutput(kOutputChannels);
  RunRenderLoop(state, *renderer, output.block, num_frames);
}
BENCHMARK_CAPTURE(RenderStaticObjectsTo9_1_6, oar, RendererEngine::kOar)
    ->Arg(100);
BENCHMARK_CAPTURE(RenderStaticObjectsTo9_1_6, roar, RendererEngine::kRoar)
    ->Arg(100);

// Object-based input (8 pairs / 16 objects) rendered to 9.1.6 loudspeakers with
// dynamic metadata updates.
void RenderDynamicObjectsTo9_1_6(benchmark::State& state,
                                 RendererEngine engine) {
  const int64_t num_frames = state.range(0);
  constexpr size_t kNumPairs = 8;
  constexpr uint32_t kOutputChannels = 16;  // 9.1.6

  const oar_config_t config = {
      .target_layout = ck_oar_layout_916,
      .samples_per_channel = kDefaultFrameSize,
      .sampling_rate = kDefaultSampleRate,
  };
  auto renderer = RendererInstance::Create(engine, config);
  ABSL_CHECK(renderer != nullptr);

  const int group_id = renderer->AddAudioGroup();
  ABSL_CHECK_GE(group_id, 0);

  const auto metadatas = CreateObjectPairMetadatas(kNumPairs);
  auto input = AudioBlock::CreateInput(/*channels=*/2);
  const auto element_ids =
      SetupObjectElements(*renderer, group_id, kNumPairs, CreateObjectConfig(),
                          &input.block, metadatas);

  auto output = AudioBlock::CreateOutput(kOutputChannels);
  RunDynamicRenderLoop(state, *renderer, output.block, element_ids, metadatas,
                       num_frames);
}
BENCHMARK_CAPTURE(RenderDynamicObjectsTo9_1_6, oar, RendererEngine::kOar)
    ->Arg(100);
BENCHMARK_CAPTURE(RenderDynamicObjectsTo9_1_6, roar, RendererEngine::kRoar)
    ->Arg(100);

// -----------------------------------------------------------------------------
//   Setup & Initialization Benchmarks (To explicitly isolate setup cost)
// -----------------------------------------------------------------------------

// Measures renderer creation, element registration, and DSP graph allocation
// time.
void Setup9_1_6ToBinaural(benchmark::State& state, RendererEngine engine) {
  constexpr uint32_t kInputChannels = 16;
  constexpr uint32_t kElementId = 15;

  const oar_config_t config = {
      .target_layout = ck_oar_layout_binaural,
      .samples_per_channel = kDefaultFrameSize,
      .sampling_rate = kDefaultSampleRate,
  };
  auto element_cfg = CreateChannelConfig(ck_oar_layout_916);
  AddWorldLockedParameter(element_cfg);
  auto input = AudioBlock::CreateInput(kInputChannels);

  for (auto _ : state) {
    auto renderer = RendererInstance::Create(engine, config);
    const int group_id = renderer->AddAudioGroup();
    renderer->AddAudioElement(group_id, kElementId, element_cfg);
    renderer->UpdateAudioElementData(kElementId, &input.block);
    benchmark::DoNotOptimize(renderer);
  }
}
BENCHMARK_CAPTURE(Setup9_1_6ToBinaural, oar, RendererEngine::kOar);
BENCHMARK_CAPTURE(Setup9_1_6ToBinaural, roar, RendererEngine::kRoar);

// Measures binaural setup and initialization time when polyphase sinc
// resampling the internal HRIR filter bank from 48 kHz to 44.1 kHz.
void Setup9_1_6ToBinaural_Resampled_44100(benchmark::State& state,
                                          RendererEngine engine) {
  constexpr uint32_t kInputChannels = 16;
  constexpr uint32_t kElementId = 15;
  constexpr uint32_t kSampleRate44100 = 44100;

  const oar_config_t config = {
      .target_layout = ck_oar_layout_binaural,
      .samples_per_channel = kDefaultFrameSize,
      .sampling_rate = kSampleRate44100,
  };
  auto element_cfg = CreateChannelConfig(ck_oar_layout_916);
  AddWorldLockedParameter(element_cfg);
  auto input = AudioBlock::CreateInput(kInputChannels, kDefaultFrameSize,
                                       kSampleRate44100);

  for (auto _ : state) {
    auto renderer = RendererInstance::Create(engine, config);
    const int group_id = renderer->AddAudioGroup();
    renderer->AddAudioElement(group_id, kElementId, element_cfg);
    renderer->UpdateAudioElementData(kElementId, &input.block);
    benchmark::DoNotOptimize(renderer);
  }
}
BENCHMARK_CAPTURE(Setup9_1_6ToBinaural_Resampled_44100, oar,
                  RendererEngine::kOar);
BENCHMARK_CAPTURE(Setup9_1_6ToBinaural_Resampled_44100, roar,
                  RendererEngine::kRoar);

// Measures EAR channel-to-channel downmixer setup and matrix initialization.
void Setup9_1_6ToStereo(benchmark::State& state, RendererEngine engine) {
  constexpr uint32_t kInputChannels = 16;
  constexpr uint32_t kElementId = 15;

  const oar_config_t config = {
      .target_layout = ck_oar_layout_stereo,
      .samples_per_channel = kDefaultFrameSize,
      .sampling_rate = kDefaultSampleRate,
  };
  const auto element_cfg = CreateChannelConfig(ck_oar_layout_916);
  auto input = AudioBlock::CreateInput(kInputChannels);

  for (auto _ : state) {
    auto renderer = RendererInstance::Create(engine, config);
    const int group_id = renderer->AddAudioGroup();
    renderer->AddAudioElement(group_id, kElementId, element_cfg);
    renderer->UpdateAudioElementData(kElementId, &input.block);
    benchmark::DoNotOptimize(renderer);
  }
}
BENCHMARK_CAPTURE(Setup9_1_6ToStereo, oar, RendererEngine::kOar);
BENCHMARK_CAPTURE(Setup9_1_6ToStereo, roar, RendererEngine::kRoar);

// Measures EAR ambisonic decoding matrix setup and element registration.
void Setup4oaTo9_1_6(benchmark::State& state, RendererEngine engine) {
  constexpr uint32_t kInputChannels = 25;  // 4OA
  constexpr uint32_t kElementId = 15;

  const oar_config_t config = {
      .target_layout = ck_oar_layout_916,
      .samples_per_channel = kDefaultFrameSize,
      .sampling_rate = kDefaultSampleRate,
  };
  const auto element_cfg = CreateAmbisonicConfig(ck_oar_4oa);
  auto input = AudioBlock::CreateInput(kInputChannels);

  for (auto _ : state) {
    auto renderer = RendererInstance::Create(engine, config);
    const int group_id = renderer->AddAudioGroup();
    renderer->AddAudioElement(group_id, kElementId, element_cfg);
    renderer->UpdateAudioElementData(kElementId, &input.block);
    benchmark::DoNotOptimize(renderer);
  }
}
BENCHMARK_CAPTURE(Setup4oaTo9_1_6, oar, RendererEngine::kOar);
BENCHMARK_CAPTURE(Setup4oaTo9_1_6, roar, RendererEngine::kRoar);

// Measures OLR 3D VBAP triangulation mesh and 16 object registrations with
// initial metadata.
void SetupObjectsTo9_1_6(benchmark::State& state, RendererEngine engine) {
  constexpr size_t kNumPairs = 8;

  const oar_config_t config = {
      .target_layout = ck_oar_layout_916,
      .samples_per_channel = kDefaultFrameSize,
      .sampling_rate = kDefaultSampleRate,
  };
  const auto metadatas = CreateObjectPairMetadatas(kNumPairs);
  auto input = AudioBlock::CreateInput(/*channels=*/2);

  for (auto _ : state) {
    auto renderer = RendererInstance::Create(engine, config);
    const int group_id = renderer->AddAudioGroup();
    SetupObjectElements(*renderer, group_id, kNumPairs, CreateObjectConfig(),
                        &input.block, metadatas);
    benchmark::DoNotOptimize(renderer);
  }
}
BENCHMARK_CAPTURE(SetupObjectsTo9_1_6, oar, RendererEngine::kOar);
BENCHMARK_CAPTURE(SetupObjectsTo9_1_6, roar, RendererEngine::kRoar);

}  // namespace
}  // namespace roar_equivalence_testing
