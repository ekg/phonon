//! Plugin Instance
//!
//! Manages a loaded plugin instance, handling initialization, audio processing,
//! parameter control, and state management.

use super::types::*;
use std::sync::{Arc, Mutex};

/// Handle to a loaded plugin instance
#[derive(Debug)]
pub struct PluginInstanceHandle {
    /// Plugin info
    info: PluginInfo,
    /// Sample rate
    sample_rate: f32,
    /// Maximum block size
    max_block_size: usize,
    /// Current parameter values (cached)
    param_values: Vec<f32>,
    /// Whether the plugin is initialized
    initialized: bool,
    // TODO: Add actual rack::Plugin handle when implementing
    // plugin: Option<rack::Plugin>,
}

impl Clone for PluginInstanceHandle {
    fn clone(&self) -> Self {
        // Clone creates a new uninitialized instance with the same plugin info
        // Actual plugin state cannot be cloned - new instance must be re-initialized
        Self::new(self.info.clone())
    }
}

impl PluginInstanceHandle {
    /// Create a new plugin instance (not yet initialized)
    pub fn new(info: PluginInfo) -> Self {
        let num_params = info.parameters.len();
        let param_values = info
            .parameters
            .iter()
            .map(|p| p.default_value)
            .collect();

        Self {
            info,
            sample_rate: 44100.0,
            max_block_size: 512,
            param_values,
            initialized: false,
        }
    }

    /// Initialize the plugin with sample rate and block size
    pub fn initialize(&mut self, sample_rate: f32, max_block_size: usize) -> PluginResult<()> {
        self.sample_rate = sample_rate;
        self.max_block_size = max_block_size;

        // TODO: Initialize rack::Plugin
        // self.plugin = Some(rack::load(&self.info.path)?);
        // self.plugin.as_mut().unwrap().initialize(sample_rate, max_block_size)?;

        self.initialized = true;
        Ok(())
    }

    /// Check if plugin is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Get plugin info
    pub fn info(&self) -> &PluginInfo {
        &self.info
    }

    /// Process audio through the plugin
    ///
    /// # Arguments
    /// * `inputs` - Input audio buffers (one per channel)
    /// * `outputs` - Output audio buffers (one per channel)
    /// * `num_samples` - Number of samples to process
    pub fn process(
        &mut self,
        inputs: &[&[f32]],
        outputs: &mut [&mut [f32]],
        num_samples: usize,
    ) -> PluginResult<()> {
        if !self.initialized {
            return Err(PluginError::ProcessError(
                "Plugin not initialized".to_string(),
            ));
        }

        // TODO: Process through rack::Plugin
        // self.plugin.as_mut().unwrap().process(inputs, outputs, num_samples)?;

        // For now, pass through or generate silence
        if self.info.is_instrument() {
            // Instruments generate audio - for now, silence
            for output in outputs.iter_mut() {
                for sample in output.iter_mut().take(num_samples) {
                    *sample = 0.0;
                }
            }
        } else {
            // Effects pass through input
            let num_channels = inputs.len().min(outputs.len());
            for ch in 0..num_channels {
                for i in 0..num_samples {
                    outputs[ch][i] = inputs[ch][i];
                }
            }
        }

        Ok(())
    }

    /// Process with MIDI input (for instruments)
    pub fn process_with_midi(
        &mut self,
        midi_events: &[MidiEvent],
        outputs: &mut [&mut [f32]],
        num_samples: usize,
    ) -> PluginResult<()> {
        if !self.initialized {
            return Err(PluginError::ProcessError(
                "Plugin not initialized".to_string(),
            ));
        }

        // TODO: Send MIDI events and process through rack::Plugin

        // For now, generate silence
        for output in outputs.iter_mut() {
            for sample in output.iter_mut().take(num_samples) {
                *sample = 0.0;
            }
        }

        Ok(())
    }

    /// Get parameter value by index
    pub fn get_parameter(&self, index: usize) -> PluginResult<f32> {
        self.param_values
            .get(index)
            .copied()
            .ok_or_else(|| PluginError::ParameterError(format!("Invalid parameter index: {}", index)))
    }

    /// Set parameter value by index
    pub fn set_parameter(&mut self, index: usize, value: f32) -> PluginResult<()> {
        if index >= self.param_values.len() {
            return Err(PluginError::ParameterError(format!(
                "Invalid parameter index: {}",
                index
            )));
        }

        // Clamp value to valid range
        let param_info = &self.info.parameters[index];
        let clamped = value.clamp(param_info.min_value, param_info.max_value);
        self.param_values[index] = clamped;

        // TODO: Send to rack::Plugin
        // self.plugin.as_mut().unwrap().set_parameter(index, clamped)?;

        Ok(())
    }

    /// Set parameter value by name
    pub fn set_parameter_by_name(&mut self, name: &str, value: f32) -> PluginResult<()> {
        let index = self
            .info
            .find_parameter(name)
            .ok_or_else(|| PluginError::ParameterError(format!("Unknown parameter: {}", name)))?
            .index;
        self.set_parameter(index, value)
    }

    /// Get current state as bytes
    pub fn get_state(&self) -> PluginResult<PresetState> {
        // TODO: Get state from rack::Plugin
        // let data = self.plugin.as_ref().unwrap().get_state()?;

        Ok(PresetState::new("Current", self.param_values_to_bytes()))
    }

    /// Restore state from bytes
    pub fn set_state(&mut self, state: &PresetState) -> PluginResult<()> {
        // TODO: Set state on rack::Plugin
        // self.plugin.as_mut().unwrap().set_state(&state.data)?;

        // For now, just update cached values
        let values = self.bytes_to_param_values(&state.data);
        for (i, v) in values.into_iter().enumerate() {
            if i < self.param_values.len() {
                self.param_values[i] = v;
            }
        }

        Ok(())
    }

    /// Load factory preset by name
    pub fn load_factory_preset(&mut self, name: &str) -> PluginResult<()> {
        if !self.info.factory_presets.contains(&name.to_string()) {
            return Err(PluginError::PresetError(format!(
                "Factory preset not found: {}",
                name
            )));
        }

        // TODO: Load factory preset from rack::Plugin

        Ok(())
    }

    /// Convert parameter values to bytes for state storage
    fn param_values_to_bytes(&self) -> Vec<u8> {
        self.param_values
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect()
    }

    /// Convert bytes back to parameter values
    fn bytes_to_param_values(&self, data: &[u8]) -> Vec<f32> {
        data.chunks(4)
            .map(|chunk| {
                if chunk.len() == 4 {
                    f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
                } else {
                    0.0
                }
            })
            .collect()
    }
}

/// Thread-safe wrapper for plugin instance
pub type SharedPluginInstance = Arc<Mutex<PluginInstanceHandle>>;

/// MIDI event for plugin input
#[derive(Clone, Debug, PartialEq)]
pub struct MidiEvent {
    /// Sample offset within the buffer
    pub sample_offset: usize,
    /// MIDI status byte
    pub status: u8,
    /// First data byte
    pub data1: u8,
    /// Second data byte
    pub data2: u8,
}

impl MidiEvent {
    /// Create a note-on event
    pub fn note_on(sample_offset: usize, channel: u8, note: u8, velocity: u8) -> Self {
        Self {
            sample_offset,
            status: 0x90 | (channel & 0x0F),
            data1: note & 0x7F,
            data2: velocity & 0x7F,
        }
    }

    /// Create a note-off event
    pub fn note_off(sample_offset: usize, channel: u8, note: u8) -> Self {
        Self {
            sample_offset,
            status: 0x80 | (channel & 0x0F),
            data1: note & 0x7F,
            data2: 0,
        }
    }

    /// Create a control change event
    pub fn control_change(sample_offset: usize, channel: u8, controller: u8, value: u8) -> Self {
        Self {
            sample_offset,
            status: 0xB0 | (channel & 0x0F),
            data1: controller & 0x7F,
            data2: value & 0x7F,
        }
    }

    /// Create a pitch bend event
    pub fn pitch_bend(sample_offset: usize, channel: u8, value: i16) -> Self {
        // Pitch bend is 14-bit, centered at 0x2000
        let unsigned = (value as i32 + 0x2000).clamp(0, 0x3FFF) as u16;
        Self {
            sample_offset,
            status: 0xE0 | (channel & 0x0F),
            data1: (unsigned & 0x7F) as u8,
            data2: ((unsigned >> 7) & 0x7F) as u8,
        }
    }

    /// Check if this is a note-on event
    pub fn is_note_on(&self) -> bool {
        (self.status & 0xF0) == 0x90 && self.data2 > 0
    }

    /// Check if this is a note-off event
    pub fn is_note_off(&self) -> bool {
        (self.status & 0xF0) == 0x80 || ((self.status & 0xF0) == 0x90 && self.data2 == 0)
    }

    /// Get MIDI channel (0-15)
    pub fn channel(&self) -> u8 {
        self.status & 0x0F
    }

    /// Convert to raw bytes
    pub fn to_bytes(&self) -> [u8; 3] {
        [self.status, self.data1, self.data2]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_info() -> PluginInfo {
        PluginInfo {
            id: PluginId {
                format: PluginFormat::Vst3,
                identifier: "/path/to/test.vst3".to_string(),
                name: "Test Synth".to_string(),
            },
            vendor: "Test".to_string(),
            version: "1.0".to_string(),
            category: PluginCategory::Instrument,
            num_inputs: 0,
            num_outputs: 2,
            parameters: vec![
                ParameterInfo::new(0, "Cutoff"),
                ParameterInfo::new(1, "Resonance"),
            ],
            factory_presets: vec!["Init".to_string()],
            has_gui: true,
            path: "/path/to/test.vst3".to_string(),
        }
    }

    #[test]
    fn test_plugin_instance_creation() {
        let info = make_test_info();
        let instance = PluginInstanceHandle::new(info);

        assert!(!instance.is_initialized());
        assert_eq!(instance.info().id.name, "Test Synth");
    }

    #[test]
    fn test_plugin_instance_initialize() {
        let info = make_test_info();
        let mut instance = PluginInstanceHandle::new(info);

        assert!(instance.initialize(44100.0, 512).is_ok());
        assert!(instance.is_initialized());
    }

    #[test]
    fn test_parameter_access() {
        let info = make_test_info();
        let mut instance = PluginInstanceHandle::new(info);

        // Set by index
        assert!(instance.set_parameter(0, 0.5).is_ok());
        assert_eq!(instance.get_parameter(0).unwrap(), 0.5);

        // Set by name
        assert!(instance.set_parameter_by_name("Resonance", 0.7).is_ok());
        assert_eq!(instance.get_parameter(1).unwrap(), 0.7);

        // Invalid index
        assert!(instance.set_parameter(99, 0.5).is_err());
        assert!(instance.get_parameter(99).is_err());
    }

    #[test]
    fn test_midi_events() {
        let note_on = MidiEvent::note_on(0, 0, 60, 100);
        assert!(note_on.is_note_on());
        assert!(!note_on.is_note_off());
        assert_eq!(note_on.channel(), 0);

        let note_off = MidiEvent::note_off(100, 0, 60);
        assert!(note_off.is_note_off());
        assert!(!note_off.is_note_on());

        // Note-on with velocity 0 is note-off
        let vel0 = MidiEvent::note_on(0, 0, 60, 0);
        assert!(vel0.is_note_off());
    }
}
