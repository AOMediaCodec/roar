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

#ifndef EQUIVALENCE_TESTING_TEST_HELPERS_H_
#define EQUIVALENCE_TESTING_TEST_HELPERS_H_

#include <cstddef>
#include <cstdint>
#include <string>
#include <vector>

extern "C" {
#include "oar/include/animation.h"
#include "oar/include/oar.h"
#include "oar/include/oar_base.h"
#include "oar/include/oar_metadata.h"

// Rust port declarations (`roar_*`) directly exported by roar (`ffi.rs`)
oar_t* roar_create(const oar_config_t* config);
void roar_destroy(oar_t* oar);
int roar_add_audio_group(oar_t* oar);
int roar_add_audio_element(oar_t* oar, uint32_t gid, uint32_t id,
                           const oar_audio_element_config_t* config);
int roar_remove_audio_element(oar_t* oar, uint32_t id);
int roar_update_audio_element_metadata(oar_t* oar, uint32_t id,
                                       const oar_metadata_t* metadata);
int roar_update_audio_element_data(oar_t* oar, uint32_t id,
                                   oar_audio_block_t* data);
int roar_set_metadata_unit_to_process(oar_t* oar, oar_metadata_type_t type,
                                      uint32_t samples);
int roar_update_metadata(oar_t* oar, uint32_t gid,
                         const oar_metadata_t* metadata);
int roar_render(oar_t* oar, oar_audio_block_t* output);
int roar_enable_loudness_processor(oar_t* oar, int enable);
int roar_set_loudness(oar_t* oar, uint32_t gid, float loudness,
                      float target_loudness);
int roar_enable_limiter(oar_t* oar, int enable);
int roar_enable_head_tracking(oar_t* oar, int enable);
uint32_t roar_get_samples_per_channel(oar_t* oar);
uint32_t roar_get_sampling_rate(oar_t* oar);
uint32_t roar_get_number_of_audio_element_channels(oar_t* oar, uint32_t id);
uint32_t roar_get_number_of_output_channels(oar_t* oar);
uint32_t roar_get_number_of_audio_elements(oar_t* oar);
}

namespace roar_equivalence_testing {

// Returns all valid ambisonic orders.
std::vector<oar_hoa_t> GetAllAmbisonicOrders();

// Returns all valid channel-based input layouts.
std::vector<oar_layout_t> GetAllChannelBasedInputs();

// Returns all valid output layouts (including binaural).
std::vector<oar_layout_t> GetAllOutputLayouts();

// Returns all valid output layouts, wrapped into the config.
std::vector<oar_config_t> GetAllOutputConfigs(
    uint32_t samples_per_channel = 1024, uint32_t sampling_rate = 48000);

// Gets a vector of input data with sine waves on each channel.
std::vector<float> GetInputData(size_t samples_per_channel, size_t num_channels,
                                int sample_rate, float base_frequency = 200.0f);

// Gets a buffer to receive output from the rendering.
std::vector<float> GetOutputBuffer(size_t samples_per_channel,
                                   size_t num_channels);

// Creates an IAMF downmix mode metadata block.
oar_metadata_t CreateDownmixModeMetadata(int mode, uint32_t duration);

// Creates a metadata instance to configure polar object-based element.
oar_metadata_t CreateObjectMetadata(const std::vector<polar_t>& positions,
                                    uint32_t duration);

// Creates a metadata instance to configure cartesian object-based element.
oar_metadata_t CreateObjectMetadata(const std::vector<cartesian_t>& positions,
                                    uint32_t duration);

// Creates a metadata instance to configure animated polar object(s).
oar_metadata_t CreateAnimatedObjectMetadata(
    const std::vector<animated_polar_t>& positions, uint32_t duration);

// Creates a metadata instance to configure animated cartesian object(s).
oar_metadata_t CreateAnimatedObjectMetadata(
    const std::vector<animated_cartesian_t>& positions, uint32_t duration);

// Creates a metadata instance to configure constant gain.
oar_metadata_t CreateConstantGainMetadata(uint32_t id, float gain_db,
                                          uint32_t duration);

// Creates a metadata instance to configure multiple gains.
oar_metadata_t CreateMultipleGainMetadata(uint32_t id,
                                          const std::vector<float>& gains_db,
                                          uint32_t duration);

// Creates a metadata instance to configure animated gain.
oar_metadata_t CreateAnimatedGainMetadata(uint32_t id,
                                          animation_type_t anim_type,
                                          float start, float end, float control,
                                          float control_relative_time,
                                          uint32_t duration);

// Creates a debug string for the audio element config.
std::string Describe(oar_audio_element_config_t element_config);

}  // namespace roar_equivalence_testing
#endif  // EQUIVALENCE_TESTING_TEST_HELPERS_H_
