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
#include <memory>
#include <optional>
#include <string>
#include <string_view>
#include <vector>

#include "absl/strings/str_format.h"
#include "equivalence_testing/test_helpers.h"
#include "gmock/gmock.h"
#include "gtest/gtest.h"

extern "C" {
#include "include/oar.h"
#include "include/oar_base.h"
#include "include/oar_metadata.h"
}

namespace roar_equivalence_testing {
namespace {

using ::testing::FloatNear;
using ::testing::NotNull;
using ::testing::Pointwise;

// Tolerable error level.
constexpr float kEpsilon = 1e-6f;
constexpr uint32_t kSamplesPerChannel = 1024;

// Sample rates that differ from the 48 kHz native HRIR WAV assets, exercising
// both downsampling (< 48 kHz) and upsampling (> 48 kHz) during binaural
// initialization.
constexpr uint32_t kNonNativeSampleRates[] = {16000, 24000, 32000,
                                              44100, 88200, 96000};

// Pairs an audio element configuration with optional initial metadata.
struct ElementWithMetadata {
  oar_audio_element_config_t element_config;
  std::optional<oar_metadata_t> metadata;
};

// Runs a single equivalence test comparing OAR and ROAR for `config` and
// `elements`.
void RunTest(oar_config_t config,
             const std::vector<ElementWithMetadata>& elements,
             std::string_view error_message) {
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

  const int c_group_id = oar_add_audio_group(oar.get());
  ASSERT_GE(c_group_id, 0) << error_message;
  const int r_group_id = roar_add_audio_group(roar.get());
  ASSERT_GE(r_group_id, 0) << error_message;

  for (size_t i = 0; i < elements.size(); ++i) {
    uint32_t element_id = i;
    const auto& element = elements[i];
    ASSERT_EQ(oar_add_audio_element(oar.get(), c_group_id, element_id,
                                    &element.element_config),
              0)
        << error_message;
    ASSERT_EQ(roar_add_audio_element(roar.get(), r_group_id, element_id,
                                     &element.element_config),
              0)
        << error_message;
    if (element.metadata) {
      ASSERT_EQ(oar_update_audio_element_metadata(oar.get(), element_id,
                                                  &element.metadata.value()),
                0)
          << error_message;
      ASSERT_EQ(roar_update_audio_element_metadata(roar.get(), element_id,
                                                   &element.metadata.value()),
                0)
          << error_message;
    }
  }

  std::vector<std::vector<float>> all_input_data;
  for (size_t i = 0; i < elements.size(); ++i) {
    uint32_t element_id = i;
    const uint32_t input_channels =
        oar_get_number_of_audio_element_channels(oar.get(), element_id);
    all_input_data.push_back(GetInputData(c_samples_per_channel, input_channels,
                                          config.sampling_rate));
    oar_audio_block_t input_block = {};
    input_block.channels = input_channels;
    input_block.samples_per_channel = c_samples_per_channel;
    input_block.data = all_input_data.back().data();

    ASSERT_EQ(
        oar_update_audio_element_data(oar.get(), element_id, &input_block), 0)
        << error_message;
    ASSERT_EQ(
        roar_update_audio_element_data(roar.get(), element_id, &input_block), 0)
        << error_message;
  }

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

  ASSERT_EQ(oar_render(oar.get(), &c_output_block), 0) << error_message;
  ASSERT_EQ(roar_render(roar.get(), &r_output_block), 0) << error_message;

  EXPECT_THAT(c_output, Pointwise(FloatNear(kEpsilon), r_output))
      << error_message;
}

// ===== Tests =====

TEST(RoarEquivalence, BinauralChannelBasedAtNonNativeSampleRatesMatchesOar) {
  constexpr oar_binaural_filter_profile_t kProfiles[] = {ck_ambient, ck_direct,
                                                         ck_reverberant};
  constexpr oar_layout_t kInputLayouts[] = {
      ck_oar_layout_stereo, ck_oar_layout_51, ck_oar_layout_714};

  for (uint32_t sample_rate : kNonNativeSampleRates) {
    for (oar_layout_t input_layout : kInputLayouts) {
      for (oar_binaural_filter_profile_t profile : kProfiles) {
        oar_config_t config = {};
        config.target_layout = ck_oar_layout_binaural;
        config.samples_per_channel = kSamplesPerChannel;
        config.sampling_rate = sample_rate;

        oar_audio_element_config_t element_config = {};
        element_config.type = ck_channel_based;
        element_config.cbc.layout = input_layout;
        element_config.parameters.flags =
            def_parameter_set_flag_iamf_element_rendering_config;
        element_config.parameters.element_rendering_config
            .headphones_rendering_mode = ck_world_locked;
        element_config.parameters.element_rendering_config
            .binaural_filter_profile = profile;

        std::string error_message = absl::StrFormat(
            "ChannelBased binaural (rate=%u, layout=%d, profile=%d)",
            sample_rate, input_layout, profile);
        RunTest(config, {{element_config, std::nullopt}}, error_message);
      }
    }
  }
}

TEST(RoarEquivalence, BinauralSceneBasedAtNonNativeSampleRatesMatchesOar) {
  for (uint32_t sample_rate : kNonNativeSampleRates) {
    for (oar_hoa_t order : GetAllAmbisonicOrders()) {
      oar_config_t config = {};
      config.target_layout = ck_oar_layout_binaural;
      config.samples_per_channel = kSamplesPerChannel;
      config.sampling_rate = sample_rate;

      oar_audio_element_config_t element_config = {};
      element_config.type = ck_scene_based;
      element_config.sbc.order = order;

      std::string error_message = absl::StrFormat(
          "SceneBased binaural (rate=%u, order=%d)", sample_rate, order);
      RunTest(config, {{element_config, std::nullopt}}, error_message);
    }
  }
}

TEST(RoarEquivalence, BinauralObjectBasedAtNonNativeSampleRatesMatchesOar) {
  for (uint32_t sample_rate : kNonNativeSampleRates) {
    oar_config_t config = {};
    config.target_layout = ck_oar_layout_binaural;
    config.samples_per_channel = kSamplesPerChannel;
    config.sampling_rate = sample_rate;

    oar_audio_element_config_t element_config = {};
    element_config.type = ck_object_based;
    element_config.obc.num_objects = 2;

    std::vector<polar_t> positions = {{50.0f, 0.5f, 1.0f},
                                      {-105.0f, -1.3f, 3.0f}};
    oar_metadata_t metadata =
        CreateObjectMetadata(positions, config.samples_per_channel);

    std::string error_message =
        absl::StrFormat("ObjectBased binaural (rate=%u)", sample_rate);
    RunTest(config, {{element_config, metadata}}, error_message);
  }
}

}  // namespace
}  // namespace roar_equivalence_testing
