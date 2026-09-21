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

#include "equivalence_testing/benchmarks/benchmark_helpers.h"

#include <cstddef>
#include <cstdint>
#include <memory>
#include <vector>

#include "absl/log/absl_check.h"
#include "absl/types/span.h"
#include "equivalence_testing/test_helpers.h"

extern "C" {
#include "include/oar.h"
#include "include/oar_base.h"
#include "include/oar_metadata.h"
}

namespace roar_equivalence_testing {

std::unique_ptr<RendererInstance> RendererInstance::Create(
    RendererEngine engine, const oar_config_t& config) {
  oar_t* handle = (engine == RendererEngine::kOar) ? oar_create(&config)
                                                   : roar_create(&config);
  if (!handle) {
    return nullptr;
  }
  return std::unique_ptr<RendererInstance>(
      new RendererInstance(engine, handle));
}

RendererInstance::RendererInstance(RendererEngine engine, oar_t* handle)
    : engine_(engine), handle_(handle) {}

RendererInstance::~RendererInstance() {
  if (handle_ != nullptr) {
    if (engine_ == RendererEngine::kOar) {
      oar_destroy(handle_);
    } else {
      roar_destroy(handle_);
    }
  }
}

int RendererInstance::AddAudioGroup() {
  return (engine_ == RendererEngine::kOar) ? oar_add_audio_group(handle_)
                                           : roar_add_audio_group(handle_);
}

int RendererInstance::AddAudioElement(
    uint32_t gid, uint32_t id, const oar_audio_element_config_t& config) {
  return (engine_ == RendererEngine::kOar)
             ? oar_add_audio_element(handle_, gid, id, &config)
             : roar_add_audio_element(handle_, gid, id, &config);
}

int RendererInstance::UpdateAudioElementMetadata(
    uint32_t id, const oar_metadata_t& metadata) {
  return (engine_ == RendererEngine::kOar)
             ? oar_update_audio_element_metadata(handle_, id, &metadata)
             : roar_update_audio_element_metadata(handle_, id, &metadata);
}

int RendererInstance::UpdateMetadata(uint32_t gid,
                                     const oar_metadata_t& metadata) {
  return (engine_ == RendererEngine::kOar)
             ? oar_update_metadata(handle_, gid, &metadata)
             : roar_update_metadata(handle_, gid, &metadata);
}

int RendererInstance::EnableHeadTracking(bool enable) {
  return (engine_ == RendererEngine::kOar)
             ? oar_enable_head_tracking(handle_, enable ? 1 : 0)
             : roar_enable_head_tracking(handle_, enable ? 1 : 0);
}

int RendererInstance::UpdateAudioElementData(uint32_t id,
                                             oar_audio_block_t* data) {
  return (engine_ == RendererEngine::kOar)
             ? oar_update_audio_element_data(handle_, id, data)
             : roar_update_audio_element_data(handle_, id, data);
}

int RendererInstance::Render(oar_audio_block_t* output) {
  return (engine_ == RendererEngine::kOar) ? oar_render(handle_, output)
                                           : roar_render(handle_, output);
}

AudioBlock AudioBlock::CreateInput(uint32_t channels,
                                   uint32_t samples_per_channel,
                                   uint32_t sample_rate) {
  AudioBlock ab;
  ab.data = GetInputData(samples_per_channel, channels, sample_rate);
  ab.block = {
      .data = ab.data.data(),
      .channels = channels,
      .samples_per_channel = samples_per_channel,
  };
  return ab;
}

AudioBlock AudioBlock::CreateOutput(uint32_t channels,
                                    uint32_t samples_per_channel) {
  AudioBlock ab;
  ab.data = GetOutputBuffer(samples_per_channel, channels);
  ab.block = {
      .data = ab.data.data(),
      .channels = channels,
      .samples_per_channel = samples_per_channel,
  };
  return ab;
}

oar_audio_element_config_t CreateChannelConfig(oar_layout_t layout) {
  oar_audio_element_config_t cfg = {};
  cfg.type = ck_channel_based;
  cfg.cbc.layout = layout;
  return cfg;
}

oar_audio_element_config_t CreateAmbisonicConfig(oar_hoa_t order) {
  oar_audio_element_config_t cfg = {};
  cfg.type = ck_scene_based;
  cfg.sbc.order = order;
  return cfg;
}

oar_audio_element_config_t CreateObjectConfig(uint32_t num_objects) {
  oar_audio_element_config_t cfg = {};
  cfg.type = ck_object_based;
  cfg.obc.num_objects = num_objects;
  return cfg;
}

void AddWorldLockedParameter(oar_audio_element_config_t& config) {
  config.parameters.flags |=
      def_parameter_set_flag_iamf_element_rendering_config;
  config.parameters.element_rendering_config.headphones_rendering_mode =
      ck_world_locked;
  config.parameters.element_rendering_config.binaural_filter_profile =
      ck_ambient;
}

std::vector<oar_metadata_t> CreateObjectPairMetadatas(size_t num_pairs,
                                                      uint32_t frame_size) {
  std::vector<oar_metadata_t> metadatas;
  metadatas.reserve(num_pairs);
  for (size_t i = 0; i < num_pairs; ++i) {
    const float azimuth0 = 10.0f * i;
    const float azimuth1 = -azimuth0;
    std::vector<polar_t> positions = {
        {.azimuth = azimuth0, .elevation = 0.0f, .distance = 1.0f},
        {.azimuth = azimuth1, .elevation = 0.0f, .distance = 1.0f},
    };
    metadatas.push_back(CreateObjectMetadata(positions, frame_size));
  }
  return metadatas;
}

std::vector<oar_metadata_t> CreateHeadRotationMetadatas(uint32_t frame_size) {
  struct Rotation {
    float w, x, y, z;
  };
  const std::vector<Rotation> rotations = {
      {1.0f, 0.0f, 0.0f, 0.0f},
      {0.7071068f, 0.0f, 0.7071068f, 0.0f},
      {0.7071068f, 0.7071068f, 0.0f, 0.0f},
      {0.7071068f, 0.0f, 0.0f, 0.7071068f},
      {-0.7f, 0.3f, -0.2f, 0.6f},
  };
  std::vector<oar_metadata_t> metadata_list;
  metadata_list.reserve(rotations.size());
  for (const auto& rot : rotations) {
    oar_metadata_t meta = {};
    meta.type = ck_metadata_head_rotation;
    meta.head_rotation.w = rot.w;
    meta.head_rotation.x = rot.x;
    meta.head_rotation.y = rot.y;
    meta.head_rotation.z = rot.z;
    meta.duration = frame_size;
    metadata_list.push_back(meta);
  }
  return metadata_list;
}

std::vector<uint32_t> SetupObjectElements(
    RendererInstance& renderer, int group_id, size_t num_pairs,
    const oar_audio_element_config_t& element_cfg,
    oar_audio_block_t* input_block,
    absl::Span<const oar_metadata_t> metadatas) {
  ABSL_CHECK_EQ(metadatas.size(), num_pairs);

  std::vector<uint32_t> element_ids;
  element_ids.reserve(num_pairs);

  for (size_t i = 0; i < num_pairs; ++i) {
    const uint32_t element_id = 10 + i;
    ABSL_CHECK_EQ(renderer.AddAudioElement(group_id, element_id, element_cfg),
                  0);
    ABSL_CHECK_EQ(renderer.UpdateAudioElementMetadata(element_id, metadatas[i]),
                  0);
    if (input_block != nullptr) {
      ABSL_CHECK_EQ(renderer.UpdateAudioElementData(element_id, input_block),
                    0);
    }
    element_ids.push_back(element_id);
  }
  return element_ids;
}

}  // namespace roar_equivalence_testing
