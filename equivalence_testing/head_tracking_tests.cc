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
#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <memory>
#include <optional>
#include <string>
#include <string_view>
#include <vector>

#include "absl/strings/str_format.h"
#include "gmock/gmock.h"
#include "gtest/gtest.h"
#include "roar/equivalence_testing/test_helpers.h"

extern "C" {
#include "oar/include/oar.h"
#include "oar/include/oar_base.h"
#include "oar/include/oar_metadata.h"
}

namespace roar_equivalence_testing {
namespace {

using ::std::vector;
using ::testing::FloatNear;
using ::testing::NotNull;
using ::testing::Pointwise;

// Tolerable error level.  Binaural seems to have a higher error than other
// renders, perhaps because of use of FFT.
const float kEpsilon = 2e-6;
const uint32_t kSamplesPerChannel = 1024;

// Simple struct to pair element and metadata.
struct ElementWithMetadata {
  oar_audio_element_config_t element_config;
  std::optional<oar_metadata_t> metadata;
};

vector<oar_metadata_t> GetHeadTrackingMetadata() {
  vector<oar_metadata_t> metadata_list;
  struct Rotation {
    float w, x, y, z;
  };
  const vector<Rotation> rotations = {
      {1.0f, 0.0f, 0.0f, 0.0f},     {0.707f, 0.0f, 0.707f, 0.0f},
      {0.707f, 0.707f, 0.0f, 0.0f}, {0.707f, 0.0f, 0.0f, 0.707f},
      {-0.7f, 0.3f, -0.2f, 0.6f},
  };
  for (const auto& rot : rotations) {
    oar_metadata_t meta = {};
    meta.type = ck_metadata_head_rotation;
    meta.head_rotation.w = rot.w;
    meta.head_rotation.x = rot.x;
    meta.head_rotation.y = rot.y;
    meta.head_rotation.z = rot.z;
    meta.duration = kSamplesPerChannel;
    metadata_list.push_back(meta);
  }
  return metadata_list;
}

// Run the test with head rotations with a given config and elements.
void RunTest(const oar_config_t& config, const ElementWithMetadata& element,
             std::string_view error_message) {
  // Wrap the C pointers in unique_ptrs with custom deleters for automatic
  // memory management.
  using OarPtr = std::unique_ptr<oar_t, decltype(&oar_destroy)>;
  OarPtr oar(oar_create(&config), oar_destroy);
  ASSERT_THAT(oar, NotNull()) << error_message;

  using RoarPtr = std::unique_ptr<oar_t, decltype(&roar_destroy)>;
  RoarPtr roar(roar_create(&config), roar_destroy);
  ASSERT_THAT(roar, NotNull()) << error_message;

  const uint32_t c_samples_per_channel = oar_get_samples_per_channel(oar.get());
  const uint32_t r_samples_per_channel =
      roar_get_samples_per_channel(roar.get());
  ASSERT_EQ(c_samples_per_channel, r_samples_per_channel) << error_message;
  ASSERT_EQ(config.samples_per_channel, c_samples_per_channel) << error_message;

  // Add groups
  const int c_group_id = oar_add_audio_group(oar.get());
  ASSERT_GE(c_group_id, 0) << error_message;
  const int r_group_id = roar_add_audio_group(roar.get());
  ASSERT_GE(r_group_id, 0) << error_message;

  // Add elements
  uint32_t element_id = 42;
  const int c_add = oar_add_audio_element(oar.get(), c_group_id, element_id,
                                          &element.element_config);
  ASSERT_EQ(c_add, 0) << error_message;
  const int r_add = roar_add_audio_element(roar.get(), r_group_id, element_id,
                                           &element.element_config);
  ASSERT_EQ(r_add, 0) << error_message;

  if (element.metadata.has_value()) {
    ASSERT_EQ(oar_update_audio_element_metadata(oar.get(), element_id,
                                                &element.metadata.value()),
              0)
        << error_message;
    ASSERT_EQ(roar_update_audio_element_metadata(roar.get(), element_id,
                                                 &element.metadata.value()),
              0)
        << error_message;
  }

  // Feed input audio the audio element.
  const uint32_t input_channels =
      oar_get_number_of_audio_element_channels(oar.get(), element_id);
  auto input_data =
      GetInputData(c_samples_per_channel, input_channels, config.sampling_rate);
  oar_audio_block_t input_block = {};
  input_block.channels = input_channels;
  input_block.samples_per_channel = c_samples_per_channel;
  input_block.data = input_data.data();

  ASSERT_EQ(oar_update_audio_element_data(oar.get(), element_id, &input_block),
            0)
      << error_message;
  ASSERT_EQ(
      roar_update_audio_element_data(roar.get(), element_id, &input_block), 0)
      << error_message;

  // Get output buffers ready
  const uint32_t c_output_channels =
      oar_get_number_of_output_channels(oar.get());
  const uint32_t r_output_channels =
      roar_get_number_of_output_channels(roar.get());
  ASSERT_EQ(c_output_channels, r_output_channels) << error_message;
  auto c_output = GetOutputBuffer(c_samples_per_channel, c_output_channels);

  oar_audio_block_t c_output_block = {};
  c_output_block.channels = c_output_channels;
  c_output_block.samples_per_channel = c_samples_per_channel;
  c_output_block.data = c_output.data();

  auto r_output = GetOutputBuffer(r_samples_per_channel, r_output_channels);
  oar_audio_block_t r_output_block = {};
  r_output_block.channels = r_output_channels;
  r_output_block.samples_per_channel = r_samples_per_channel;
  r_output_block.data = r_output.data();

  // Enable head tracking
  ASSERT_EQ(oar_enable_head_tracking(oar.get(), true), 0) << error_message;
  ASSERT_EQ(roar_enable_head_tracking(roar.get(), true), 0) << error_message;

  const auto head_metadata_list = GetHeadTrackingMetadata();

  for (size_t step = 0; step < head_metadata_list.size(); ++step) {
    const auto& head_metadata = head_metadata_list[step];

    // Update head position before render
    ASSERT_EQ(oar_update_metadata(oar.get(), c_group_id, &head_metadata), 0)
        << error_message << " at step " << step;
    ASSERT_EQ(roar_update_metadata(roar.get(), r_group_id, &head_metadata), 0)
        << error_message << " at step " << step;

    // Render
    ASSERT_EQ(oar_render(oar.get(), &c_output_block), 0)
        << error_message << " at step " << step;
    ASSERT_EQ(roar_render(roar.get(), &r_output_block), 0)
        << error_message << " at step " << step;

    EXPECT_THAT(c_output, Pointwise(FloatNear(kEpsilon), r_output))
        << error_message;

    // Zero output before next call.
    std::fill(c_output.begin(), c_output.end(), 0.0f);
    std::fill(r_output.begin(), r_output.end(), 0.0f);
  }
}

vector<ElementWithMetadata> GetSelectionOfElements() {
  vector<ElementWithMetadata> output;
  // All channel-based.
  for (auto input : GetAllChannelBasedInputs()) {
    oar_audio_element_config_t element_config = {};
    element_config.type = ck_channel_based;
    element_config.cbc.layout = input;
    output.push_back({element_config, std::nullopt});
  }
  // All ambisonic.
  for (auto order : GetAllAmbisonicOrders()) {
    oar_audio_element_config_t element_config = {};
    element_config.type = ck_scene_based;
    element_config.sbc.order = order;
    output.push_back({element_config, std::nullopt});
  }

  // Object-based with one (polar) object.
  {
    oar_audio_element_config_t object_element_config = {};
    object_element_config.type = ck_object_based;
    object_element_config.obc.num_objects = 1;
    std::vector<polar_t> positions = {{45.0f, 0.0f, 1.0f}};
    oar_metadata_t metadata =
        CreateObjectMetadata(positions, kSamplesPerChannel);
    output.push_back({object_element_config, metadata});
  }

  // Object-based with two (cartesian) objects.
  {
    oar_audio_element_config_t object_element_config = {};
    object_element_config.type = ck_object_based;
    object_element_config.obc.num_objects = 2;
    std::vector<cartesian_t> positions = {{0.0f, 0.0f, 1.0f},
                                          {0.0f, 1.0f, 0.0f}};
    oar_metadata_t metadata =
        CreateObjectMetadata(positions, kSamplesPerChannel);
    output.push_back({object_element_config, metadata});
  }

  return output;
}

// Tests rendering with head tracking enabled and updated.
TEST(RoarEquivalence, RenderingToBinauralWithRenderingConfig) {
  // Always target binaural output.
  oar_config_t config = {};
  config.target_layout = ck_oar_layout_binaural;
  config.samples_per_channel = kSamplesPerChannel;
  config.sampling_rate = 48000;

  const vector<oar_headphones_rendering_mode_t> modes = {
      ck_world_locked_restricted, ck_world_locked, ck_head_locked};
  const vector<oar_binaural_filter_profile_t> profiles = {ck_ambient, ck_direct,
                                                          ck_reverberant};

  for (auto mode : modes) {
    for (auto profile : profiles) {
      for (auto input : GetSelectionOfElements()) {
        input.element_config.parameters.flags |=
            def_parameter_set_flag_iamf_element_rendering_config;
        input.element_config.parameters.element_rendering_config
            .headphones_rendering_mode = mode;
        input.element_config.parameters.element_rendering_config
            .binaural_filter_profile = profile;

        std::string error_message =
            absl::StrFormat("Element: %s, rendering mode: %d, profile: %d",
                            Describe(input.element_config), mode, profile);

        RunTest(config, input, error_message);
      }
    }
  }
}

}  // namespace
}  // namespace roar_equivalence_testing
