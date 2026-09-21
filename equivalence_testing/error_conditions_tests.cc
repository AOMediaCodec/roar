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
#include <cstdint>
#include <memory>
#include <string>

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

using ::testing::Ge;
using ::testing::NotNull;

const uint32_t samples_per_channel = 256;
const uint32_t sampling_rate = 48000;

using OarPtr = std::unique_ptr<oar_t, decltype(&oar_destroy)>;
using RoarPtr = std::unique_ptr<oar_t, decltype(&roar_destroy)>;

TEST(ErrorConditionsTest, UpdateHeadPositionWithoutHeadTrackingEnabled) {
  oar_config_t config = {};
  config.target_layout = ck_oar_layout_stereo;
  config.samples_per_channel = samples_per_channel;
  config.sampling_rate = sampling_rate;

  OarPtr oar(oar_create(&config), oar_destroy);
  ASSERT_THAT(oar, NotNull());
  RoarPtr roar(roar_create(&config), roar_destroy);
  ASSERT_THAT(roar, NotNull());

  // Create head rotation metadata
  oar_metadata_t metadata = {};
  metadata.type = ck_metadata_head_rotation;
  metadata.duration = 256;
  metadata.head_rotation.w = 1.0f;
  metadata.head_rotation.x = 0.0f;
  metadata.head_rotation.y = 0.0f;
  metadata.head_rotation.z = 0.0f;

  auto c_group_id = oar_add_audio_group(oar.get());
  ASSERT_THAT(c_group_id, Ge(0));
  auto r_group_id = roar_add_audio_group(roar.get());
  ASSERT_THAT(r_group_id, Ge(0));

  int c_res = oar_update_metadata(oar.get(), c_group_id, &metadata);
  int r_res = roar_update_metadata(roar.get(), r_group_id, &metadata);

  EXPECT_EQ(c_res, r_res);
  EXPECT_LT(c_res, 0);  // Should be an error (Busy, i.e. -16)
}

TEST(ErrorConditionsTest, EnableHeadTrackingOnNonBinauralLayout) {
  oar_config_t config = {};
  config.target_layout = ck_oar_layout_stereo;
  config.samples_per_channel = samples_per_channel;
  config.sampling_rate = sampling_rate;

  OarPtr oar(oar_create(&config), oar_destroy);
  ASSERT_THAT(oar, NotNull());
  RoarPtr roar(roar_create(&config), roar_destroy);
  ASSERT_THAT(roar, NotNull());

  int c_res = oar_enable_head_tracking(oar.get(), true);
  int r_res = roar_enable_head_tracking(roar.get(), true);

  EXPECT_EQ(c_res, r_res);
  EXPECT_LT(c_res, 0);  // Should be NotSupported (-95)
}

TEST(ErrorConditionsTest, AddElementWithInvalidGroupId) {
  oar_config_t config = {};
  config.target_layout = ck_oar_layout_stereo;
  config.samples_per_channel = samples_per_channel;
  config.sampling_rate = sampling_rate;

  OarPtr oar(oar_create(&config), oar_destroy);
  ASSERT_THAT(oar, NotNull());
  RoarPtr roar(roar_create(&config), roar_destroy);
  ASSERT_THAT(roar, NotNull());

  oar_audio_element_config_t element_config = {};
  element_config.type = ck_channel_based;
  element_config.cbc.layout = ck_oar_layout_stereo;

  // Group 0 does not exist because no group has been created.
  uint32_t group_id = 0;
  uint32_t element_id = 10;
  int c_res =
      oar_add_audio_element(oar.get(), group_id, element_id, &element_config);
  int r_res =
      roar_add_audio_element(roar.get(), group_id, element_id, &element_config);

  EXPECT_EQ(c_res, r_res);
  EXPECT_LT(c_res, 0);  // Should be InvalidParameter (-22)
}

TEST(ErrorConditionsTest, RemoveNonExistentElement) {
  oar_config_t config = {};
  config.target_layout = ck_oar_layout_stereo;
  config.samples_per_channel = samples_per_channel;
  config.sampling_rate = sampling_rate;

  OarPtr oar(oar_create(&config), oar_destroy);
  ASSERT_THAT(oar, NotNull());
  RoarPtr roar(roar_create(&config), roar_destroy);
  ASSERT_THAT(roar, NotNull());

  // Do successfully add a group.
  int c_gid = oar_add_audio_group(oar.get());
  ASSERT_THAT(c_gid, Ge(0));
  int r_gid = roar_add_audio_group(roar.get());
  ASSERT_THAT(r_gid, Ge(0));

  // Element ID does not exist because no element has been created.
  uint32_t element_id = 10;
  int c_res = oar_remove_audio_element(oar.get(), element_id);
  int r_res = roar_remove_audio_element(roar.get(), element_id);

  EXPECT_EQ(c_res, r_res);
  EXPECT_LT(c_res, 0);  // Should be InvalidParameter (-22)
}

TEST(ErrorConditionsTest, AddDuplicateElementId) {
  oar_config_t config = {};
  config.target_layout = ck_oar_layout_stereo;
  config.samples_per_channel = samples_per_channel;
  config.sampling_rate = sampling_rate;

  OarPtr oar(oar_create(&config), oar_destroy);
  ASSERT_THAT(oar, NotNull());
  RoarPtr roar(roar_create(&config), roar_destroy);
  ASSERT_THAT(roar, NotNull());

  int c_gid = oar_add_audio_group(oar.get());
  int r_gid = roar_add_audio_group(roar.get());

  oar_audio_element_config_t element_config = {};
  element_config.type = ck_channel_based;
  element_config.cbc.layout = ck_oar_layout_stereo;

  // Add first element successfully
  uint32_t element_id = 10;
  ASSERT_EQ(
      oar_add_audio_element(oar.get(), c_gid, element_id, &element_config), 0);
  ASSERT_EQ(
      roar_add_audio_element(roar.get(), r_gid, element_id, &element_config),
      0);

  // Add duplicate element ID
  int c_res =
      oar_add_audio_element(oar.get(), c_gid, element_id, &element_config);
  int r_res =
      roar_add_audio_element(roar.get(), r_gid, element_id, &element_config);

  EXPECT_EQ(c_res, r_res);
  EXPECT_LT(c_res, 0);  // Should be InvalidParameter (-22)
}

TEST(ErrorConditionsTest, UpdateMetadataForNonExistentElement) {
  oar_config_t config = {};
  config.target_layout = ck_oar_layout_stereo;
  config.samples_per_channel = samples_per_channel;
  config.sampling_rate = sampling_rate;

  OarPtr oar(oar_create(&config), oar_destroy);
  ASSERT_THAT(oar, NotNull());
  RoarPtr roar(roar_create(&config), roar_destroy);
  ASSERT_THAT(roar, NotNull());

  oar_metadata_t metadata = {};
  metadata.type = ck_metadata_gain;
  metadata.duration = 256;
  metadata.gain.param_type = ck_param_constant;
  metadata.gain.constant_gain = 0.5f;

  uint32_t element_id = 10;
  int c_res =
      oar_update_audio_element_metadata(oar.get(), element_id, &metadata);
  int r_res =
      roar_update_audio_element_metadata(roar.get(), element_id, &metadata);

  EXPECT_EQ(c_res, r_res);
  EXPECT_LT(c_res, 0);  // Should be InvalidParameter (-22)
}

TEST(ErrorConditionsTest, UpdateMetadataForNonExistentGroup) {
  oar_config_t config = {};
  config.target_layout = ck_oar_layout_stereo;
  config.samples_per_channel = samples_per_channel;
  config.sampling_rate = sampling_rate;

  OarPtr oar(oar_create(&config), oar_destroy);
  ASSERT_THAT(oar, NotNull());
  RoarPtr roar(roar_create(&config), roar_destroy);
  ASSERT_THAT(roar, NotNull());

  oar_metadata_t metadata = {};
  metadata.type = ck_metadata_gain;
  metadata.duration = 256;
  metadata.gain.param_type = ck_param_constant;
  metadata.gain.constant_gain = 0.5f;

  uint32_t group_id = 0;
  int c_res = oar_update_metadata(oar.get(), group_id, &metadata);
  int r_res = roar_update_metadata(roar.get(), group_id, &metadata);

  EXPECT_EQ(c_res, r_res);
  EXPECT_LT(c_res, 0);  // Should be an error.
}

TEST(ErrorConditionsTest, UpdateMetadataWithInvalidType) {
  oar_config_t config = {};
  config.target_layout = ck_oar_layout_stereo;
  config.samples_per_channel = samples_per_channel;
  config.sampling_rate = sampling_rate;

  OarPtr oar(oar_create(&config), oar_destroy);
  ASSERT_THAT(oar, NotNull());
  RoarPtr roar(roar_create(&config), roar_destroy);
  ASSERT_THAT(roar, NotNull());

  int c_gid = oar_add_audio_group(oar.get());
  int r_gid = roar_add_audio_group(roar.get());

  oar_audio_element_config_t element_config = {};
  element_config.type = ck_channel_based;
  element_config.cbc.layout = ck_oar_layout_stereo;

  uint32_t element_id = 10;
  ASSERT_EQ(
      oar_add_audio_element(oar.get(), c_gid, element_id, &element_config), 0);
  ASSERT_EQ(
      roar_add_audio_element(roar.get(), r_gid, element_id, &element_config),
      0);

  // Invalid metadata type (e.g. 999)
  oar_metadata_t metadata = {};
  metadata.type = (oar_metadata_type_t)999;
  metadata.duration = 256;

  int c_res =
      oar_update_audio_element_metadata(oar.get(), element_id, &metadata);
  int r_res =
      roar_update_audio_element_metadata(roar.get(), element_id, &metadata);

  EXPECT_EQ(c_res, r_res);
  EXPECT_LT(c_res, 0);  // Should be NotSupported (-95)
}

}  // namespace
}  // namespace roar_equivalence_testing
