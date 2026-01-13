//! Real Plugin Instance using rack crate
//!
//! Wraps VST3 plugins via the rack crate for actual audio processing.
//! Provides conversion between Phonon's plugin types and rack's types.

use super::instance::MidiEvent as PhononMidiEvent;
use super::types::{
    ParameterInfo as PhononParameterInfo, PluginCategory, PluginError, PluginFormat, PluginId,
    PluginInfo as PhononPluginInfo, PluginResult,
};

// Import rack types - only available when vst3 feature is enabled
#[cfg(feature = "vst3")]
use rack::{Error as RackError, PluginInstance, PluginScanner};

/// Real VST3 plugin instance using rack crate
#[cfg(feature = "vst3")]
pub struct RealPluginInstance {
    /// The rack plugin instance (Option so we can take it for leaking)
    plugin: Option<rack::Plugin>,
    /// Phonon-style plugin info
    info: PhononPluginInfo,
    /// Sample rate
    sample_rate: f32,
    /// Max block size
    max_block_size: usize,
    /// Whether initialized
    initialized: bool,
}

#[cfg(feature = "vst3")]
impl RealPluginInstance {
    /// Create a new real plugin instance from rack PluginInfo
    pub fn from_rack_info(
        scanner: &rack::Scanner,
        rack_info: &rack::PluginInfo,
    ) -> PluginResult<Self> {
        let plugin = scanner
            .load(rack_info)
            .map_err(|e: RackError| PluginError::LoadError(e.to_string()))?;

        let info = convert_plugin_info(rack_info);

        Ok(Self {
            plugin: Some(plugin),
            info,
            sample_rate: 44100.0,
            max_block_size: 512,
            initialized: false,
        })
    }

    /// Leak the plugin instance to prevent cleanup crash
    /// Call this before dropping to avoid double-free bugs in VST3 SDK
    pub fn leak(mut self) {
        if let Some(plugin) = self.plugin.take() {
            std::mem::forget(plugin);
        }
    }

    /// Get a reference to the plugin, panics if leaked
    fn plugin(&self) -> &rack::Plugin {
        self.plugin.as_ref().expect("Plugin was leaked")
    }

    /// Get a mutable reference to the plugin, panics if leaked
    fn plugin_mut(&mut self) -> &mut rack::Plugin {
        self.plugin.as_mut().expect("Plugin was leaked")
    }

    /// Initialize the plugin with sample rate and block size
    pub fn initialize(&mut self, sample_rate: f32, max_block_size: usize) -> PluginResult<()> {
        self.plugin_mut()
            .initialize(sample_rate as f64, max_block_size)
            .map_err(|e| PluginError::InitError(e.to_string()))?;

        self.sample_rate = sample_rate;
        self.max_block_size = max_block_size;
        self.initialized = true;
        Ok(())
    }

    /// Check if plugin is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Get plugin info
    pub fn info(&self) -> &PhononPluginInfo {
        &self.info
    }

    /// Process audio through the plugin (for effects)
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

        self.plugin_mut()
            .process(inputs, outputs, num_samples)
            .map_err(|e| PluginError::ProcessError(e.to_string()))
    }

    /// Process with MIDI input (for instruments)
    pub fn process_with_midi(
        &mut self,
        midi_events: &[PhononMidiEvent],
        outputs: &mut [&mut [f32]],
        num_samples: usize,
    ) -> PluginResult<()> {
        if !self.initialized {
            return Err(PluginError::ProcessError(
                "Plugin not initialized".to_string(),
            ));
        }

        // Convert Phonon MIDI events to rack MIDI events
        let rack_events: Vec<rack::MidiEvent> = midi_events
            .iter()
            .map(convert_midi_event)
            .collect();

        // Send MIDI events to the plugin
        if !rack_events.is_empty() {
            self.plugin_mut()
                .send_midi(&rack_events)
                .map_err(|e| PluginError::ProcessError(format!("MIDI error: {}", e)))?;
        }

        // VST3 requires input buffers even for instruments (they may be zero-filled)
        // Create silent stereo input buffers
        let mut input_left = vec![0.0f32; num_samples];
        let mut input_right = vec![0.0f32; num_samples];
        let inputs: Vec<&[f32]> = vec![&input_left, &input_right];

        self.plugin_mut()
            .process(&inputs, outputs, num_samples)
            .map_err(|e| PluginError::ProcessError(e.to_string()))
    }

    /// Get parameter value by index
    pub fn get_parameter(&self, index: usize) -> PluginResult<f32> {
        self.plugin()
            .get_parameter(index)
            .map_err(|e| PluginError::ParameterError(e.to_string()))
    }

    /// Set parameter value by index (normalized 0.0 - 1.0)
    pub fn set_parameter(&mut self, index: usize, value: f32) -> PluginResult<()> {
        self.plugin_mut()
            .set_parameter(index, value)
            .map_err(|e| PluginError::ParameterError(e.to_string()))
    }

    /// Get number of parameters
    pub fn parameter_count(&self) -> usize {
        self.plugin().parameter_count()
    }

    /// Get parameter info
    pub fn parameter_info(&self, index: usize) -> PluginResult<PhononParameterInfo> {
        let rack_param = self
            .plugin()
            .parameter_info(index)
            .map_err(|e| PluginError::ParameterError(e.to_string()))?;

        Ok(PhononParameterInfo {
            index: rack_param.index,
            name: rack_param.name.clone(),
            short_name: rack_param.name.clone(), // rack doesn't have short_name
            default_value: rack_param.default,
            min_value: rack_param.min,
            max_value: rack_param.max,
            unit: rack_param.unit.clone(),
            step_count: 0,
            automatable: true,
        })
    }

    /// Get plugin state as bytes
    pub fn get_state(&self) -> PluginResult<Vec<u8>> {
        self.plugin()
            .get_state()
            .map_err(|e| PluginError::PresetError(e.to_string()))
    }

    /// Set plugin state from bytes
    pub fn set_state(&mut self, data: &[u8]) -> PluginResult<()> {
        self.plugin_mut()
            .set_state(data)
            .map_err(|e| PluginError::PresetError(e.to_string()))
    }

    /// Reset the plugin state
    pub fn reset(&mut self) -> PluginResult<()> {
        self.plugin_mut()
            .reset()
            .map_err(|e| PluginError::ProcessError(e.to_string()))
    }

    /// Get parameter changes from plugin GUI since last call
    ///
    /// Returns a list of (param_id, value) tuples for parameters that were
    /// changed by the user in the plugin GUI.
    #[cfg(target_os = "linux")]
    pub fn get_param_changes(&mut self) -> PluginResult<Vec<(u32, f64)>> {
        self.plugin_mut()
            .get_param_changes()
            .map_err(|e| PluginError::ParameterError(e.to_string()))
    }

    /// Get number of factory presets
    pub fn preset_count(&self) -> PluginResult<usize> {
        self.plugin()
            .preset_count()
            .map_err(|e| PluginError::PresetError(e.to_string()))
    }

    /// Load factory preset by index
    pub fn load_preset(&mut self, preset_number: i32) -> PluginResult<()> {
        self.plugin_mut()
            .load_preset(preset_number)
            .map_err(|e| PluginError::PresetError(e.to_string()))
    }

    /// Create a GUI window for this plugin
    #[cfg(target_os = "linux")]
    pub fn create_gui(&mut self) -> PluginResult<rack::Vst3Gui> {
        rack::Vst3Gui::create(self.plugin_mut())
            .map_err(|e| PluginError::ProcessError(format!("GUI error: {}", e)))
    }

    /// Get plugin name
    pub fn name(&self) -> &str {
        &self.info.id.name
    }
}

/// Convert rack PluginInfo to Phonon PluginInfo
#[cfg(feature = "vst3")]
pub fn convert_plugin_info(rack_info: &rack::PluginInfo) -> PhononPluginInfo {
    let category = match rack_info.plugin_type {
        rack::PluginType::Instrument => PluginCategory::Instrument,
        rack::PluginType::Effect => PluginCategory::Effect,
        rack::PluginType::Mixer => PluginCategory::Effect,
        rack::PluginType::Analyzer => PluginCategory::Analyzer,
        _ => PluginCategory::Unknown,
    };

    let (num_inputs, num_outputs) = match rack_info.plugin_type {
        rack::PluginType::Instrument => (0, 2),
        rack::PluginType::Effect => (2, 2),
        _ => (2, 2),
    };

    PhononPluginInfo {
        id: PluginId {
            format: PluginFormat::Vst3,
            identifier: rack_info.unique_id.clone(),
            name: rack_info.name.clone(),
        },
        vendor: rack_info.manufacturer.clone(),
        version: format!("{}", rack_info.version),
        category,
        num_inputs,
        num_outputs,
        parameters: vec![], // Will be populated when plugin is loaded
        factory_presets: vec![],
        has_gui: true, // Assume GUI support
        path: rack_info.path.to_string_lossy().to_string(),
    }
}

/// Convert Phonon MidiEvent to rack MidiEvent
#[cfg(feature = "vst3")]
pub fn convert_midi_event(event: &PhononMidiEvent) -> rack::MidiEvent {
    let sample_offset = event.sample_offset as u32;

    if event.is_note_on() {
        rack::MidiEvent::note_on(
            event.data1, // note
            event.data2, // velocity
            event.channel(),
            sample_offset,
        )
    } else if event.is_note_off() {
        rack::MidiEvent::note_off(
            event.data1, // note
            0,           // release velocity
            event.channel(),
            sample_offset,
        )
    } else {
        // Control change or other
        let status_type = event.status & 0xF0;
        match status_type {
            0xB0 => rack::MidiEvent::control_change(
                event.data1,
                event.data2,
                event.channel(),
                sample_offset,
            ),
            0xC0 => rack::MidiEvent::program_change(event.data1, event.channel(), sample_offset),
            0xE0 => {
                // Pitch bend - combine data1 and data2 into 14-bit value
                let value = (event.data2 as u16) << 7 | (event.data1 as u16);
                rack::MidiEvent::pitch_bend(value, event.channel(), sample_offset)
            }
            _ => {
                // Default to note on with the raw data
                rack::MidiEvent::note_on(event.data1, event.data2, event.channel(), sample_offset)
            }
        }
    }
}

/// Create a RealPluginInstance from a plugin path
#[cfg(feature = "vst3")]
pub fn create_real_plugin_from_path(path: &std::path::Path) -> PluginResult<RealPluginInstance> {
    // Create a fresh scanner for each load - avoids shared state issues
    let scanner = rack::Scanner::new()
        .map_err(|e: RackError| PluginError::ScanError(e.to_string()))?;

    // Scan the specific path to find the plugin
    let plugins = scanner.scan_path(path)
        .map_err(|e: RackError| PluginError::ScanError(e.to_string()))?;

    if plugins.is_empty() {
        return Err(PluginError::NotFound(format!("No plugin found at: {}", path.display())));
    }

    RealPluginInstance::from_rack_info(&scanner, &plugins[0])
}

/// Create a RealPluginInstance by name (scans system paths)
#[cfg(feature = "vst3")]
pub fn create_real_plugin_by_name(name: &str) -> PluginResult<RealPluginInstance> {
    // Create a fresh scanner for each load - avoids shared state issues
    let scanner = rack::Scanner::new()
        .map_err(|e: RackError| PluginError::ScanError(e.to_string()))?;

    // Scan for all plugins
    let plugins = scanner.scan()
        .map_err(|e: RackError| PluginError::ScanError(e.to_string()))?;

    // Find the plugin by name (case-insensitive, prefer exact match)
    let name_lower = name.to_lowercase();
    let exact_match = plugins.iter().find(|p| p.name.to_lowercase() == name_lower);
    let prefix_match = plugins.iter().find(|p| p.name.to_lowercase().starts_with(&name_lower));

    let matching = exact_match.or(prefix_match);

    match matching {
        Some(info) => {
            tracing::info!("Loading plugin: {} from {}", info.name, info.path.display());
            RealPluginInstance::from_rack_info(&scanner, info)
        }
        None => Err(PluginError::NotFound(format!("Plugin not found: {}", name))),
    }
}

/// Plugin scanner using rack
#[cfg(feature = "vst3")]
pub struct RealPluginScanner {
    scanner: rack::Scanner,
}

#[cfg(feature = "vst3")]
impl RealPluginScanner {
    /// Create a new scanner
    pub fn new() -> PluginResult<Self> {
        let scanner = rack::Scanner::new()
            .map_err(|e: RackError| PluginError::ScanError(e.to_string()))?;
        Ok(Self { scanner })
    }

    /// Scan for plugins in system paths
    pub fn scan(&self) -> PluginResult<Vec<rack::PluginInfo>> {
        self.scanner
            .scan()
            .map_err(|e: RackError| PluginError::ScanError(e.to_string()))
    }

    /// Scan a specific path
    pub fn scan_path(&self, path: &std::path::Path) -> PluginResult<Vec<rack::PluginInfo>> {
        self.scanner
            .scan_path(path)
            .map_err(|e: RackError| PluginError::ScanError(e.to_string()))
    }

    /// Load a plugin from PluginInfo
    pub fn load(&self, info: &rack::PluginInfo) -> PluginResult<RealPluginInstance> {
        RealPluginInstance::from_rack_info(&self.scanner, info)
    }

    /// Get the underlying rack scanner
    pub fn inner(&self) -> &rack::Scanner {
        &self.scanner
    }
}

#[cfg(feature = "vst3")]
impl Default for RealPluginScanner {
    fn default() -> Self {
        Self::new().expect("Failed to create plugin scanner")
    }
}

// Stub implementations when VST3 feature is not enabled
#[cfg(not(feature = "vst3"))]
pub struct RealPluginInstance {
    _private: (),
}

#[cfg(not(feature = "vst3"))]
impl RealPluginInstance {
    pub fn initialize(&mut self, _sample_rate: f32, _max_block_size: usize) -> PluginResult<()> {
        Err(PluginError::NotSupported(
            "VST3 support not available (feature not enabled)".to_string(),
        ))
    }

    pub fn is_initialized(&self) -> bool {
        false
    }

    pub fn info(&self) -> &PhononPluginInfo {
        unimplemented!("VST3 not available")
    }

    pub fn process(
        &mut self,
        _inputs: &[&[f32]],
        _outputs: &mut [&mut [f32]],
        _num_samples: usize,
    ) -> PluginResult<()> {
        Err(PluginError::NotSupported(
            "VST3 support not available".to_string(),
        ))
    }

    pub fn process_with_midi(
        &mut self,
        _midi_events: &[PhononMidiEvent],
        _outputs: &mut [&mut [f32]],
        _num_samples: usize,
    ) -> PluginResult<()> {
        Err(PluginError::NotSupported(
            "VST3 support not available".to_string(),
        ))
    }
}

#[cfg(not(feature = "vst3"))]
pub struct RealPluginScanner {
    _private: (),
}

#[cfg(not(feature = "vst3"))]
impl RealPluginScanner {
    pub fn new() -> PluginResult<Self> {
        Err(PluginError::NotSupported(
            "VST3 support not available (feature not enabled)".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "vst3")]
    fn test_scanner_creation() {
        // This test only runs when VST3 feature is enabled
        let result = RealPluginScanner::new();
        assert!(result.is_ok(), "Should be able to create scanner");
    }

    #[test]
    fn test_midi_event_conversion() {
        // Test note on conversion
        let note_on = PhononMidiEvent::note_on(0, 0, 60, 100);
        assert!(note_on.is_note_on());

        // Test note off conversion
        let note_off = PhononMidiEvent::note_off(100, 0, 60);
        assert!(note_off.is_note_off());
    }
}
