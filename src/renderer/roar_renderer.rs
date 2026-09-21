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

//! RoarRenderer, main orchestrator of the rendering pipeline.
//!
//! This module implements runtime state boundaries, dynamic updates (gain, head rotation, object
//! position, etc) and rendering loops.

use crate::common::definitions::{
    AudioElementConfig, Config, Decibels, DownmixMode, Gain, GroupId, HeadphonesRenderingMode,
    Layout, LinearGain, Milliseconds, OarError, ObjectPosition, PlanarBufferMut, PlanarBufferRef,
    Quaternion, Samples,
};
use crate::limiter::oar_limiter::OarLimiter;
use crate::renderer::audio_elements_renderer::AudioElementsRenderer;
use crate::renderer::audio_renderer_api::AudioRenderer;
use crate::renderer::downmix::downmix_renderer::DownmixRenderer;
use crate::renderer::ear::ear_renderer::EarRenderer;
use crate::renderer::gain_parameter::{generate_gain_multipliers, GainParameter, GainUpdate};
use crate::renderer::obr::common::constants::MAX_SUPPORTED_NUM_INPUT_CHANNELS;
use crate::renderer::obr::renderer::ObrAudioRendererAdapter;
use crate::renderer::olr::object_renderer::ObjectRenderer;
use std::collections::HashMap;

const MAX_OUTPUT_CHANNELS: usize = 25;
const LIMITER_RELEASE_TIME_MS: f64 = 50.0;
const LIMITER_CEILING_DB: f32 = -1.0;

/// Status of the OAR rendering engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Configuring,
    Rendering,
}

/// Internal helper representing an active renderer instance for an element.
pub(crate) struct ActiveRenderer {
    pub(crate) element_id: Option<u32>,
    pub(crate) renderer: Box<dyn AudioRenderer>,
}

/// Central rendering orchestrator for ROAR.
///
/// Manages the lifecycle of audio elements and groups, coordinates
/// the audio rendering process, and applies metadata updates.
/// Uses an internal state machine (`Status`) to transition from
/// configuration to rendering, ensuring allocations are performed eagerly.
///
/// # Examples
///
/// ```
/// use roar::common::definitions::{Config, Layout, AudioElementConfig, ChannelBasedConfig};
/// use roar::renderer::RoarRenderer;
///
/// let config = Config::new(Layout::Stereo, Samples::new(256)?, SampleRate::new(48000)?)?;
///
/// let mut rdr = RoarRenderer::create(&config).unwrap();
/// let group_id = rdr.add_audio_group().unwrap();
///
/// let element_cfg = AudioElementConfig::ChannelBased(ChannelBasedConfig {
///     layout: Layout::Stereo,
///     downmix_info: None,
/// });
/// rdr.add_element(group_id, 42, &element_cfg).unwrap();
///
/// let chans = [vec![0.0f32; 256], vec![0.0f32; 256]];
/// let inputs = [(42, &[&chans[0][..], &chans[1][..]][..])];
/// let mut output = vec![0.0f32; 512];
/// rdr.render(&inputs, &mut output).unwrap();
/// ```
pub struct RoarRenderer {
    /// Renderer configuration (output layout, sample rate, etc.).
    config: Config,
    /// Current state of the renderer lifecycle.
    status: Status,
    /// Active audio elements and their configurations.
    active_elements: HashMap<u32, AudioElementConfig>,
    /// List of active element IDs.
    active_element_ids: Vec<u32>,
    /// Mapping from element ID to its parent group ID.
    pub(crate) element_to_group: HashMap<u32, GroupId>,
    /// List of active group IDs.
    active_groups: Vec<GroupId>,
    /// Renderers associated with each group.
    group_renderers: HashMap<GroupId, Vec<ActiveRenderer>>,
    /// Scratch buffer for element-level rendering.
    renderer_scratch: Vec<f32>,
    /// Scratch buffer for group-level mixing.
    group_scratch: Vec<f32>,
    /// Toggle for the output peak limiter.
    pub(crate) limiter_enabled: bool,
    /// Toggle for listener head tracking (binaural output only).
    pub(crate) head_tracking_enabled: bool,
    /// Toggle for group loudness normalization.
    pub(crate) loudness_processor_enabled: bool,
    /// Loudness normalization gain adjustments per group.
    pub(crate) group_loudness: HashMap<GroupId, LinearGain>,
    /// Output peak limiter.
    limiter: Option<OarLimiter>,
    /// Dynamic gain parameters for elements, keyed by element ID and gain ID.
    element_gains: HashMap<u32, HashMap<u32, GainParameter>>,
    /// Dynamic gain parameters for groups.
    group_gains: HashMap<GroupId, GainParameter>,
    /// Scratch buffer for gain interpolation.
    gain_scratch_buf: Vec<f32>,
    /// Interval in samples for processing object position metadata.
    pub(crate) object_position_metadata_samples: u32,
}

impl RoarRenderer {
    /// Returns a reference to the list of active group IDs.
    pub fn active_groups(&self) -> &[GroupId] {
        &self.active_groups
    }

    /// Returns a reference to the configuration.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Returns a reference to the map of active audio elements.
    pub fn active_elements(&self) -> &HashMap<u32, AudioElementConfig> {
        &self.active_elements
    }

    /// Checks if a group ID is active.
    pub fn has_group(&self, gid: GroupId) -> bool {
        self.active_groups.contains(&gid)
    }

    /// Enables or disables output peak limiter.
    pub fn enable_limiter(&mut self, enable: bool) -> Result<(), OarError> {
        self.limiter_enabled = enable;
        if enable && self.limiter.is_none() {
            self.limiter = Some(OarLimiter::new(
                self.config.sample_rate(),
                Milliseconds::new(LIMITER_RELEASE_TIME_MS)?,
                Decibels::new(LIMITER_CEILING_DB)?,
                self.config.samples_per_channel(),
            )?);
        }
        for renderers in self.group_renderers.values_mut() {
            for active_rdr in renderers {
                let _ = active_rdr.renderer.enable_limiter(enable);
            }
        }
        Ok(())
    }

    /// Enables or disables listener head tracking.
    pub fn enable_head_tracking(&mut self, enable: bool) -> Result<(), OarError> {
        if self.config.output_layout() != Layout::Binaural {
            return Err(OarError::NotSupported);
        }
        self.head_tracking_enabled = enable;
        for renderers in self.group_renderers.values_mut() {
            for active_rdr in renderers {
                let _ = active_rdr.renderer.enable_head_tracking(enable);
            }
        }
        Ok(())
    }

    /// Sets listener head rotation quaternion.
    pub fn set_head_rotation(&mut self, rotation: Quaternion) -> Result<(), OarError> {
        if !self.head_tracking_enabled {
            // This error matches the original liboar behavior.
            return Err(OarError::Busy);
        }
        for renderers in self.group_renderers.values_mut() {
            for active_rdr in renderers {
                match active_rdr.renderer.set_head_rotation(rotation) {
                    Ok(()) | Err(OarError::NotSupported) => {}
                    Err(e) => return Err(e),
                }
            }
        }
        Ok(())
    }

    /// Enables or disables group loudness normalization.
    pub fn enable_loudness_processor(&mut self, enable: bool) -> Result<(), OarError> {
        self.loudness_processor_enabled = enable;
        Ok(())
    }

    /// Sets loudness adjustments for a group.
    pub fn set_loudness(
        &mut self,
        gid: GroupId,
        loudness: Decibels,
        target_loudness: Decibels,
    ) -> Result<(), OarError> {
        if !self.has_group(gid) {
            return Err(OarError::InvalidParameter);
        }
        let gain: LinearGain = (target_loudness - loudness).into();
        self.group_loudness.insert(gid, gain);
        Ok(())
    }

    /// Sets the metadata processing unit size in samples for object positions.
    pub fn set_object_position_metadata_unit_to_process(
        &mut self,
        samples: u32,
    ) -> Result<(), OarError> {
        self.object_position_metadata_samples = samples;
        for renderers in self.group_renderers.values_mut() {
            for rdr in renderers {
                rdr.renderer.set_metadata_unit_to_process(samples)?;
            }
        }
        Ok(())
    }

    fn post_process_render(
        &mut self,
        output: &mut PlanarBufferMut<'_, '_>,
    ) -> Result<(), OarError> {
        if self.limiter_enabled
            && let Some(ref mut limiter) = self.limiter
        {
            limiter.process_block(output)?;
        }
        Ok(())
    }

    /// Instantiates a new, uninitialized `RoarRenderer`.
    ///
    /// # Errors
    ///
    /// Returns `OarError::InvalidParameter` if `sample_rate` or `samples_per_channel` is 0.
    pub fn create(config: &Config) -> Result<RoarRenderer, OarError> {
        let samples = config.samples_per_channel().value() as usize;
        let out_channels = config.output_layout().channels();
        let total_samples = out_channels * samples;

        Ok(RoarRenderer {
            config: *config,
            status: Status::Configuring,
            active_elements: HashMap::new(),
            active_element_ids: Vec::new(),
            element_to_group: HashMap::new(),
            active_groups: Vec::new(),
            group_renderers: HashMap::new(),
            renderer_scratch: vec![0.0f32; total_samples],
            group_scratch: vec![0.0f32; total_samples],
            limiter_enabled: false,
            head_tracking_enabled: false,
            loudness_processor_enabled: false,
            group_loudness: HashMap::new(),
            limiter: None,
            element_gains: HashMap::new(),
            group_gains: HashMap::new(),
            gain_scratch_buf: vec![1.0f32; samples],
            object_position_metadata_samples: config.samples_per_channel().value(),
        })
    }

    fn is_binaural_route_for_config(&self, config: &AudioElementConfig) -> bool {
        if self.config.output_layout() != Layout::Binaural {
            return false;
        }
        if let AudioElementConfig::ChannelBased(cbc) = config
            && let Some(rc) = cbc.rendering_config
            && rc.headphones_rendering_mode == HeadphonesRenderingMode::WorldLockedRestricted
        {
            return false;
        }
        true
    }

    fn is_binaural_route(&self, id: u32) -> Result<bool, OarError> {
        let config = self.active_elements.get(&id).ok_or(OarError::InvalidParameter)?;
        Ok(self.is_binaural_route_for_config(config))
    }

    // TODO(b/525080422): The groups must be added before elements, so the API can be used wrong if
    // elements are added before adding groups, but it doesn't have to be that way.  I think get rid
    // of this method, create the group the first time an element is pushed to that group.
    pub fn add_audio_group(&mut self) -> Result<GroupId, OarError> {
        let new_id = GroupId::try_from(self.active_groups.len() as u32)?;
        self.active_groups.push(new_id);
        self.group_renderers.insert(new_id, Vec::new());
        Ok(new_id)
    }

    fn instantiate_sub_renderer(
        &self,
        id: u32,
        config: &AudioElementConfig,
        target_layout: Layout,
    ) -> Result<Box<dyn AudioRenderer>, OarError> {
        let sub: Box<dyn AudioRenderer> = match config {
            AudioElementConfig::ChannelBased(cbc) => {
                let use_dm = target_layout != Layout::Binaural
                    && cbc.downmix_info.is_some()
                    && cbc.layout.is_valid_downmix_to(target_layout);

                if use_dm {
                    let info = cbc.downmix_info.unwrap();
                    let mut dm = DownmixRenderer::new(cbc.layout, target_layout);
                    dm.set_mode_weight(info.mode(), info.weight_index())?;
                    Box::new(dm)
                } else {
                    Box::new(EarRenderer::with_output_layout(target_layout))
                }
            }
            AudioElementConfig::SceneBased(_) => {
                Box::new(EarRenderer::with_output_layout(target_layout))
            }
            AudioElementConfig::ObjectBased(_obc) => Box::new(ObjectRenderer::new(
                target_layout,
                self.config.sample_rate(),
                self.config.samples_per_channel(),
            )?),
        };

        let sub_frame_samples = if self.object_position_metadata_samples == 0 {
            None
        } else {
            Some(Samples::new(self.object_position_metadata_samples)?)
        };
        let router =
            AudioElementsRenderer::with_backend(sub, target_layout.channels(), sub_frame_samples);

        let mut active_sub: Box<dyn AudioRenderer> = Box::new(router);
        active_sub.add_element(id, config)?;
        let _ = active_sub.enable_limiter(self.limiter_enabled);
        let _ = active_sub.enable_head_tracking(self.head_tracking_enabled);
        Ok(active_sub)
    }

    /// Configures an audio element configuration in the manager.
    ///
    /// # Errors
    ///
    /// Returns `OarError::InvalidParameter` if the group ID does not exist, or the ID is already
    /// registered.
    pub fn add_element(
        &mut self,
        group_id: GroupId,
        id: u32,
        config: &AudioElementConfig,
    ) -> Result<(), OarError> {
        if !self.has_group(group_id) {
            return Err(OarError::InvalidParameter);
        }
        if self.active_elements.contains_key(&id) {
            return Err(OarError::Busy);
        }

        let is_binaural = self.config.output_layout() == Layout::Binaural;
        let is_binaural_route = self.is_binaural_route_for_config(config);

        if is_binaural_route {
            let renderers = self.group_renderers.entry(group_id).or_default();
            let shared_obr_idx = renderers.iter().position(|r| r.element_id.is_none());
            if let Some(idx) = shared_obr_idx {
                renderers[idx].renderer.add_element(id, config)?;
            } else {
                let obr = ObrAudioRendererAdapter::new(
                    self.config.samples_per_channel(),
                    self.config.sample_rate(),
                );
                let sub_frame_samples = if self.object_position_metadata_samples == 0 {
                    None
                } else {
                    Some(Samples::new(self.object_position_metadata_samples)?)
                };
                let router = AudioElementsRenderer::with_backend(
                    Box::new(obr),
                    self.config.output_layout().channels(),
                    sub_frame_samples,
                );

                let mut active_sub: Box<dyn AudioRenderer> = Box::new(router);
                active_sub.add_element(id, config)?;
                let _ = active_sub.enable_limiter(self.limiter_enabled);
                let _ = active_sub.enable_head_tracking(self.head_tracking_enabled);
                renderers.push(ActiveRenderer { element_id: None, renderer: active_sub });
            }
        } else {
            let target_layout =
                if is_binaural { Layout::Stereo } else { self.config.output_layout() };
            let sub = self.instantiate_sub_renderer(id, config, target_layout)?;
            let renderers = self.group_renderers.entry(group_id).or_default();
            renderers.push(ActiveRenderer { element_id: Some(id), renderer: sub });
        }

        self.active_elements.insert(id, *config);
        self.active_element_ids.push(id);
        self.element_to_group.insert(id, group_id);

        Ok(())
    }

    /// Removes an audio element from the active elements list.
    pub fn remove_element(&mut self, id: u32) -> Result<(), OarError> {
        if !self.active_elements.contains_key(&id) {
            return Err(OarError::InvalidParameter);
        }
        let is_obr_element = self.is_binaural_route(id)?;
        self.active_elements.remove(&id);
        self.active_element_ids.retain(|&x| x != id);
        let group_id = self.element_to_group.remove(&id).ok_or(OarError::InvalidParameter)?;

        // TODO(b/543004593): Unify the way that audio elements are removed.
        if let Some(renderers) = self.group_renderers.get_mut(&group_id) {
            if is_obr_element {
                renderers.retain(|r| r.element_id.is_some());
            } else {
                renderers.retain(|r| r.element_id != Some(id));
            }
        }

        Ok(())
    }

    /// Processes planar input channels and writes spatialized outputs.
    ///
    /// Adheres to zero-heap-allocation guidelines.
    pub fn render(
        &mut self,
        inputs: &[(u32, PlanarBufferRef<'_, '_>)],
        output: &mut PlanarBufferMut<'_, '_>,
    ) -> Result<(), OarError> {
        self.status = Status::Rendering;

        let samples = self.config.samples_per_channel().value() as usize;
        let out_channels = self.config.output_layout().channels();

        if output.num_channels() != out_channels || output.num_samples() != samples {
            return Err(OarError::InvalidParameter);
        }

        // Validate that all active elements have inputs provided with correct channel count
        for &id in &self.active_element_ids {
            let config = self.active_elements.get(&id).ok_or(OarError::InvalidParameter)?;
            let expected_chans = config.channels();
            let elem_input = inputs
                .iter()
                .find(|(eid, _)| *eid == id)
                .map(|&(_, data)| data)
                .ok_or(OarError::InvalidParameter)?;
            if elem_input.num_channels() != expected_chans {
                return Err(OarError::InvalidParameter);
            }
        }

        output.fill(0.0);

        let active_elements = &self.active_elements;
        let element_to_group = &self.element_to_group;
        let output_layout = self.config.output_layout();

        for (&group_id, renderers) in &mut self.group_renderers {
            self.group_scratch.fill(0.0);

            for active_rdr in renderers {
                self.renderer_scratch.fill(0.0);
                let mut scratch_slices_array: [&mut [f32]; MAX_OUTPUT_CHANNELS] =
                    std::array::from_fn(|_| &mut [] as &mut [f32]);
                let mut chunks = self.renderer_scratch.chunks_exact_mut(samples);
                for slice in scratch_slices_array.iter_mut().take(out_channels) {
                    *slice = chunks.next().ok_or(OarError::InvalidParameter)?;
                }
                let mut renderer_scratch_buf = PlanarBufferMut::new(
                    &mut scratch_slices_array[..out_channels],
                    out_channels,
                    self.config.samples_per_channel(),
                )?;
                if let Some(elem_id) = active_rdr.element_id {
                    // Individual renderer (restricted element in binaural output, or loudspeaker
                    // output)
                    let elem_input = inputs
                        .iter()
                        .find(|(eid, _)| *eid == elem_id)
                        .map(|&(_, data)| data)
                        .ok_or(OarError::InvalidParameter)?;
                    active_rdr.renderer.render(elem_input, &mut renderer_scratch_buf)?;
                    if let Some(gains) = self.element_gains.get_mut(&elem_id) {
                        for param in gains.values_mut() {
                            generate_gain_multipliers(param, samples, &mut self.gain_scratch_buf);
                            for c in 0..out_channels {
                                let start = c * samples;
                                let end = start + samples;
                                for (sample, &gain) in self.renderer_scratch[start..end]
                                    .iter_mut()
                                    .zip(self.gain_scratch_buf.iter())
                                {
                                    *sample *= gain;
                                }
                            }
                        }
                    }
                } else {
                    // Shared OBR
                    // TODO(b/512062316): Don't we know how many input channels we have?
                    let mut group_inputs: [&[f32]; MAX_SUPPORTED_NUM_INPUT_CHANNELS] =
                        [&[]; MAX_SUPPORTED_NUM_INPUT_CHANNELS];
                    let mut input_idx = 0;

                    for &id in &self.active_element_ids {
                        let is_binaural_route = if output_layout != Layout::Binaural {
                            false
                        } else {
                            let config =
                                active_elements.get(&id).ok_or(OarError::InvalidParameter)?;
                            if let AudioElementConfig::ChannelBased(cbc) = config {
                                if let Some(rc) = cbc.rendering_config {
                                    rc.headphones_rendering_mode
                                        != HeadphonesRenderingMode::WorldLockedRestricted
                                } else {
                                    true
                                }
                            } else {
                                true
                            }
                        };

                        if element_to_group.get(&id) == Some(&group_id) && is_binaural_route {
                            let elem_input = inputs
                                .iter()
                                .find(|(eid, _)| *eid == id)
                                .map(|&(_, data)| data)
                                .ok_or(OarError::InvalidParameter)?;
                            for c in 0..elem_input.num_channels() {
                                let chan = elem_input.channel(c);
                                if input_idx < MAX_SUPPORTED_NUM_INPUT_CHANNELS {
                                    group_inputs[input_idx] = chan;
                                    input_idx += 1;
                                } else {
                                    return Err(OarError::InvalidParameter);
                                }
                            }
                        }
                    }

                    let active_inputs = &group_inputs[..input_idx];
                    let validated_inputs = PlanarBufferRef::new(
                        active_inputs,
                        input_idx,
                        self.config.samples_per_channel(),
                    )?;
                    active_rdr.renderer.render(validated_inputs, &mut renderer_scratch_buf)?;
                }

                for (dst, src) in self.group_scratch.iter_mut().zip(self.renderer_scratch.iter()) {
                    *dst += *src;
                }
            }

            // Apply group gain to group_scratch
            if let Some(param) = self.group_gains.get_mut(&group_id) {
                generate_gain_multipliers(param, samples, &mut self.gain_scratch_buf);
                for c in 0..out_channels {
                    let start = c * samples;
                    let end = start + samples;
                    for (sample, &gain) in
                        self.group_scratch[start..end].iter_mut().zip(self.gain_scratch_buf.iter())
                    {
                        *sample *= gain;
                    }
                }
            }

            if self.loudness_processor_enabled
                && let Some(&gain) = self.group_loudness.get(&group_id)
            {
                // Only apply if the gain is significantly different from 1.0.
                if (gain - 1.0).abs() > 1e-6 {
                    for sample in &mut self.group_scratch {
                        *sample *= gain;
                    }
                }
            }

            for out_idx in 0..out_channels {
                let out_slice = output.channel_mut(out_idx);
                let start = out_idx * samples;
                let src_slice = &self.group_scratch[start..start + samples];
                for (dst, src) in out_slice.iter_mut().zip(src_slice.iter()) {
                    *dst += *src;
                }
            }
        }

        // Post-process final output (limiter)
        self.post_process_render(output)?;

        Ok(())
    }

    /// Updates dynamic metadata parameters on the active sub-renderers.
    pub fn update_element_gain(
        &mut self,
        id: u32,
        gain_id: u32,
        gain: &Gain,
        duration: Samples,
    ) -> Result<(), OarError> {
        if !self.active_elements.contains_key(&id) {
            return Err(OarError::InvalidParameter);
        }
        let params = self.element_gains.entry(id).or_default();
        let param = params.entry(gain_id).or_insert_with(|| GainParameter::new(gain_id));
        param.updates.push_back(GainUpdate { gain: gain.variant.clone(), duration, elapsed: 0 });

        let group_id = self.element_to_group.get(&id).copied().ok_or(OarError::InvalidParameter)?;
        let is_obr_element = self.is_binaural_route(id)?;
        if is_obr_element
            && let Some(renderers) = self.group_renderers.get_mut(&group_id)
            && let Some(rdr) = renderers.iter_mut().find(|r| r.element_id.is_none())
        {
            rdr.renderer.update_element_gain(id, gain_id, gain, duration)?;
        }
        Ok(())
    }

    pub fn update_element_positions(
        &mut self,
        id: u32,
        positions: &ObjectPosition,
        duration: Samples,
    ) -> Result<(), OarError> {
        let config = self.active_elements.get(&id).ok_or(OarError::InvalidParameter)?;
        if !matches!(config, AudioElementConfig::ObjectBased(_)) {
            return Err(OarError::NotSupported);
        }
        let group_id = self.element_to_group.get(&id).copied().ok_or(OarError::InvalidParameter)?;
        let is_obr_element = self.is_binaural_route(id)?;
        if let Some(renderers) = self.group_renderers.get_mut(&group_id) {
            let active_rdr = if is_obr_element {
                renderers.iter_mut().find(|r| r.element_id.is_none())
            } else {
                renderers.iter_mut().find(|r| r.element_id == Some(id))
            };
            if let Some(rdr) = active_rdr {
                rdr.renderer.update_element_positions(id, positions, duration)?;
            }
        }
        Ok(())
    }

    pub fn update_element_downmix_mode(
        &mut self,
        id: u32,
        mode: DownmixMode,
        duration: Option<Samples>,
    ) -> Result<(), OarError> {
        if !self.active_elements.contains_key(&id) {
            return Err(OarError::InvalidParameter);
        }
        if self.config.output_layout() == Layout::Binaural {
            return Err(OarError::NotSupported);
        }
        if !self.element_supports_downmix(id) {
            return Err(OarError::NotSupported);
        }
        let group_id = self.element_to_group.get(&id).copied().ok_or(OarError::InvalidParameter)?;
        if let Some(renderers) = self.group_renderers.get_mut(&group_id)
            && let Some(rdr) = renderers.iter_mut().find(|r| r.element_id == Some(id))
        {
            rdr.renderer.update_element_downmix_mode(id, mode, duration)?;
        }
        Ok(())
    }

    pub fn update_group_gain(
        &mut self,
        gid: GroupId,
        gain: &Gain,
        duration: Samples,
    ) -> Result<(), OarError> {
        if !self.active_groups.contains(&gid) {
            return Err(OarError::InvalidParameter);
        }
        let param = self.group_gains.entry(gid).or_insert_with(|| GainParameter::new(0));
        param.updates.push_back(GainUpdate { gain: gain.variant.clone(), duration, elapsed: 0 });
        Ok(())
    }

    /// Returns the total number of active audio elements (actually renderers).
    pub fn number_of_audio_elements(&self) -> usize {
        self.group_renderers.values().map(|v| v.len()).sum()
    }

    /// Returns the number of channels for a given element ID.
    /// Handles Binaural group-sum mapping.
    pub fn element_channels(&self, id: u32) -> Option<usize> {
        if self.config.output_layout() == Layout::Binaural {
            let group_id = self.element_to_group.get(&id)?;
            let sum: usize = self
                .element_to_group
                .iter()
                .filter(|&(_, &g)| g == *group_id)
                .map(|(&e_id, _)| {
                    self.active_elements.get(&e_id).map(|c| c.channels()).unwrap_or(0)
                })
                .sum();
            Some(sum)
        } else {
            self.active_elements.get(&id).map(|c| c.channels())
        }
    }

    fn element_supports_downmix(&self, id: u32) -> bool {
        let Some(config) = self.active_elements.get(&id) else {
            return false;
        };
        match config {
            AudioElementConfig::ChannelBased(cbc) => {
                self.config.output_layout() != Layout::Binaural
                    && cbc.downmix_info.is_some()
                    && cbc.layout.is_valid_downmix_to(self.config.output_layout())
            }
            _ => false,
        }
    }
}
