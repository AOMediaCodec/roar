#include <cstdint>
#include <memory>
#include <optional>
#include <string>
#include <vector>

#include "equivalence_testing/test_helpers.h"
#include "gmock/gmock.h"
#include "gtest/gtest.h"

extern "C" {
#include "include/oar.h"
#include "include/oar_base.h"
}

namespace roar_equivalence_testing {
namespace {

using ::testing::FloatNear;
using ::testing::NotNull;
using ::testing::Pointwise;

// Tolerable error level.
const float kEpsilon = 1e-6;

const uint32_t samples_per_channel = 1024;
const uint32_t sampling_rate = 48000;

// Optional params for loudness processor.
struct LoudnessConfig {
  float current_loudness;
  float target_loudness;
};

/**
 * Runs a test using the same config/params/input data for both oar and roar.
 *
 * @param config Config to create both both oar and roar.
 * @param element_config Config for the input audio.
 * @param loudness_config Optional values for loudness processor.
 * @param enable_limiter Value for `*_enable_limiter`.
 * @param input_amplitude The amplitude to scale the sinusoidal input audio.
 */
void RunLoudnessTest(const oar_config_t& config,
                     const oar_audio_element_config_t& element_config,
                     std::optional<LoudnessConfig> loudness_config,
                     bool enable_limiter, float input_amplitude) {
  // Wrap the C pointers in unique_ptrs with custom deleters for automatic
  // memory management.
  using OarPtr = std::unique_ptr<oar_t, decltype(&oar_destroy)>;
  OarPtr oar(oar_create(&config), oar_destroy);
  ASSERT_THAT(oar, NotNull());

  using RoarPtr = std::unique_ptr<oar_t, decltype(&roar_destroy)>;
  RoarPtr roar(roar_create(&config), roar_destroy);
  ASSERT_THAT(roar, NotNull());

  const uint32_t c_samples_per_channel = oar_get_samples_per_channel(oar.get());
  const uint32_t r_samples_per_channel =
      roar_get_samples_per_channel(roar.get());
  ASSERT_EQ(c_samples_per_channel, r_samples_per_channel);
  ASSERT_EQ(config.samples_per_channel, c_samples_per_channel);

  // Add groups
  const int c_group_id = oar_add_audio_group(oar.get());
  ASSERT_GE(c_group_id, 0);
  const int r_group_id = roar_add_audio_group(roar.get());
  ASSERT_GE(r_group_id, 0);

  // Add element
  uint32_t element_id = 0;
  const int c_add =
      oar_add_audio_element(oar.get(), c_group_id, element_id, &element_config);
  ASSERT_EQ(c_add, 0);
  const int r_add = roar_add_audio_element(roar.get(), r_group_id, element_id,
                                           &element_config);
  ASSERT_EQ(r_add, 0);

  // Enable/Configure Loudness Processor
  if (loudness_config.has_value()) {
    ASSERT_EQ(oar_enable_loudness_processor(oar.get(), true), 0);
    ASSERT_EQ(roar_enable_loudness_processor(roar.get(), true), 0);
    auto current_loudness = loudness_config->current_loudness;
    auto target_loudness = loudness_config->target_loudness;
    ASSERT_EQ(oar_set_loudness(oar.get(), c_group_id, current_loudness,
                               target_loudness),
              0);
    ASSERT_EQ(roar_set_loudness(roar.get(), r_group_id, current_loudness,
                                target_loudness),
              0);
  }

  // Enable Limiter
  ASSERT_EQ(oar_enable_limiter(oar.get(), enable_limiter), 0);
  ASSERT_EQ(roar_enable_limiter(roar.get(), enable_limiter), 0);

  // Feed input audio (scaled by amplitude)
  const uint32_t input_channels =
      oar_get_number_of_audio_element_channels(oar.get(), element_id);
  auto input_data =
      GetInputData(c_samples_per_channel, input_channels, config.sampling_rate);
  for (auto& val : input_data) {
    val *= input_amplitude;
  }

  oar_audio_block_t input_block = {};
  input_block.channels = input_channels;
  input_block.samples_per_channel = c_samples_per_channel;
  input_block.data = input_data.data();

  ASSERT_EQ(oar_update_audio_element_data(oar.get(), element_id, &input_block),
            0);
  ASSERT_EQ(
      roar_update_audio_element_data(roar.get(), element_id, &input_block), 0);

  // Get output buffers ready
  const uint32_t c_output_channels =
      oar_get_number_of_output_channels(oar.get());
  const uint32_t r_output_channels =
      roar_get_number_of_output_channels(roar.get());
  ASSERT_EQ(c_output_channels, r_output_channels);

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

  // Render
  ASSERT_EQ(oar_render(oar.get(), &c_output_block), 0);
  ASSERT_EQ(roar_render(roar.get(), &r_output_block), 0);

  // Compare
  EXPECT_THAT(c_output, Pointwise(FloatNear(kEpsilon), r_output));
}

TEST(LoudnessTest, LoudnessDisabled) {
  oar_config_t config = {};
  config.target_layout = ck_oar_layout_stereo;
  config.samples_per_channel = samples_per_channel;
  config.sampling_rate = sampling_rate;

  oar_audio_element_config_t element_config = {};
  element_config.type = ck_channel_based;
  element_config.cbc.layout = ck_oar_layout_stereo;

  // Loudness processor disabled. Even if we set parameters (we won't), output
  // shouldn't scale.
  RunLoudnessTest(config, element_config, /*loudness_config=*/std::nullopt,
                  /*enable_limiter=*/false, 1.0f);
}

TEST(LoudnessTest, LoudnessBoost) {
  oar_config_t config = {};
  config.target_layout = ck_oar_layout_stereo;
  config.samples_per_channel = samples_per_channel;
  config.sampling_rate = sampling_rate;

  oar_audio_element_config_t element_config = {};
  element_config.type = ck_channel_based;
  element_config.cbc.layout = ck_oar_layout_stereo;

  // Boost: current = -30dB, target = -24dB -> +6dB gain (scaling by ~2.0)
  RunLoudnessTest(config, element_config, LoudnessConfig{-30.0f, -24.0f},
                  /*enable_limiter=*/false, 0.4f);
}

TEST(LoudnessTest, LoudnessAttenuation) {
  oar_config_t config = {};
  config.target_layout = ck_oar_layout_stereo;
  config.samples_per_channel = samples_per_channel;
  config.sampling_rate = sampling_rate;

  oar_audio_element_config_t element_config = {};
  element_config.type = ck_channel_based;
  element_config.cbc.layout = ck_oar_layout_stereo;

  // Attenuation: current = -18dB, target = -24dB -> -6dB gain (scaling by ~0.5)
  RunLoudnessTest(config, element_config, LoudnessConfig{-18.0f, -24.0f},
                  /*enable_limiter=*/false, 1.0f);
}

TEST(LimiterTest, LimiterDisabled) {
  oar_config_t config = {};
  config.target_layout = ck_oar_layout_stereo;
  config.samples_per_channel = samples_per_channel;
  config.sampling_rate = sampling_rate;

  oar_audio_element_config_t element_config = {};
  element_config.type = ck_channel_based;
  element_config.cbc.layout = ck_oar_layout_stereo;

  // Limiter disabled, high amplitude input (5.0).
  // The output floats should be matching but exceeding 1.0.
  RunLoudnessTest(config, element_config, /*loudness_config=*/std::nullopt,
                  /*enable_limiter=*/false, 5.0f);
}

TEST(LimiterTest, LimiterEnabled) {
  oar_config_t config = {};
  config.target_layout = ck_oar_layout_stereo;
  config.samples_per_channel = samples_per_channel;
  config.sampling_rate = sampling_rate;

  oar_audio_element_config_t element_config = {};
  element_config.type = ck_channel_based;
  element_config.cbc.layout = ck_oar_layout_stereo;

  // Limiter enabled, high amplitude input (5.0).
  // Output should be compressed/limited and match.
  RunLoudnessTest(config, element_config,
                  /*loudness_config=*/std::nullopt, /*enable_limiter=*/true,
                  5.0f);
}

TEST(LimiterTest, LoudnessAndLimiterTogether) {
  oar_config_t config = {};
  config.target_layout = ck_oar_layout_stereo;
  config.samples_per_channel = samples_per_channel;
  config.sampling_rate = sampling_rate;

  oar_audio_element_config_t element_config = {};
  element_config.type = ck_channel_based;
  element_config.cbc.layout = ck_oar_layout_stereo;

  // Loudness processor enabled and limiter enabled.
  // High amplitude input (5.0).
  // Output should be boosted but limited and match.
  RunLoudnessTest(config, element_config, LoudnessConfig{-30.0f, -24.0f},
                  /*enable_limiter=*/true, 0.7f);
}

}  // namespace
}  // namespace roar_equivalence_testing
