//! Core types for plugin hosting
//!
//! Defines the fundamental types used throughout the plugin host system:
//! - Plugin identification and formats
//! - Plugin metadata (info, parameters, presets)
//! - Error types

use serde::{Deserialize, Serialize};
use std::fmt;

/// Supported plugin formats
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluginFormat {
    /// Legacy Steinberg VST2 format (deprecated but widely used)
    Vst2,
    /// Steinberg VST3 format
    Vst3,
    /// Apple Audio Unit (macOS/iOS)
    AudioUnit,
    /// CLAP (CLever Audio Plugin)
    Clap,
    /// Linux Audio Plugins (LV2)
    Lv2,
}

impl fmt::Display for PluginFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginFormat::Vst2 => write!(f, "VST2"),
            PluginFormat::Vst3 => write!(f, "VST3"),
            PluginFormat::AudioUnit => write!(f, "AU"),
            PluginFormat::Clap => write!(f, "CLAP"),
            PluginFormat::Lv2 => write!(f, "LV2"),
        }
    }
}

/// Unique plugin identifier
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginId {
    /// Plugin format (VST3, AU, etc.)
    pub format: PluginFormat,
    /// Unique identifier (bundle ID for AU, path for VST3)
    pub identifier: String,
    /// Human-readable name
    pub name: String,
}

impl fmt::Display for PluginId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.format)
    }
}

/// Plugin category
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginCategory {
    /// Synthesizer (generates audio from MIDI)
    Instrument,
    /// Audio effect (processes audio)
    Effect,
    /// MIDI effect (processes MIDI)
    MidiEffect,
    /// Audio analyzer (metering, spectrum, etc.)
    Analyzer,
    /// Unknown/uncategorized
    Unknown,
}

/// Plugin metadata from scanning
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginInfo {
    /// Unique plugin identifier
    pub id: PluginId,
    /// Plugin vendor/manufacturer
    pub vendor: String,
    /// Plugin version string
    pub version: String,
    /// Plugin category
    pub category: PluginCategory,
    /// Number of audio inputs
    pub num_inputs: usize,
    /// Number of audio outputs
    pub num_outputs: usize,
    /// Parameter metadata
    pub parameters: Vec<ParameterInfo>,
    /// Factory preset names
    pub factory_presets: Vec<String>,
    /// Whether the plugin has a GUI
    pub has_gui: bool,
    /// File path or bundle path
    pub path: String,
}

impl PluginInfo {
    /// Check if this is an instrument (synth)
    pub fn is_instrument(&self) -> bool {
        matches!(self.category, PluginCategory::Instrument)
    }

    /// Check if this is an effect
    pub fn is_effect(&self) -> bool {
        matches!(self.category, PluginCategory::Effect)
    }

    /// Find parameter by name (case-insensitive, fuzzy)
    pub fn find_parameter(&self, name: &str) -> Option<&ParameterInfo> {
        let name_lower = name.to_lowercase();

        // Exact match first
        if let Some(p) = self.parameters.iter().find(|p| p.name.to_lowercase() == name_lower) {
            return Some(p);
        }

        // Short name match
        if let Some(p) = self
            .parameters
            .iter()
            .find(|p| p.short_name.to_lowercase() == name_lower)
        {
            return Some(p);
        }

        // Prefix match
        self.parameters
            .iter()
            .find(|p| p.name.to_lowercase().starts_with(&name_lower))
    }
}

/// Parameter metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParameterInfo {
    /// Parameter index
    pub index: usize,
    /// Full parameter name
    pub name: String,
    /// Short name (for display)
    pub short_name: String,
    /// Default value (normalized 0.0-1.0)
    pub default_value: f32,
    /// Minimum value (normalized)
    pub min_value: f32,
    /// Maximum value (normalized)
    pub max_value: f32,
    /// Unit label (Hz, dB, %, etc.)
    pub unit: String,
    /// Number of steps (0 = continuous)
    pub step_count: u32,
    /// Whether this is an automatable parameter
    pub automatable: bool,
}

impl ParameterInfo {
    /// Create a simple continuous parameter
    pub fn new(index: usize, name: &str) -> Self {
        Self {
            index,
            name: name.to_string(),
            short_name: name.to_string(),
            default_value: 0.0,
            min_value: 0.0,
            max_value: 1.0,
            unit: String::new(),
            step_count: 0,
            automatable: true,
        }
    }
}

/// Preset state (can be opaque binary or structured)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PresetState {
    /// Preset name
    pub name: String,
    /// Binary state data (plugin-specific format)
    pub data: Vec<u8>,
}

impl PresetState {
    /// Create a new preset state
    pub fn new(name: &str, data: Vec<u8>) -> Self {
        Self {
            name: name.to_string(),
            data,
        }
    }

    /// Create an empty preset
    pub fn empty(name: &str) -> Self {
        Self {
            name: name.to_string(),
            data: Vec::new(),
        }
    }
}

/// Plugin host error types
#[derive(Debug)]
pub enum PluginError {
    /// Plugin not found in registry
    NotFound(String),
    /// Failed to load plugin
    LoadFailed(String),
    /// Failed to load plugin (alternate)
    LoadError(String),
    /// Plugin initialization failed
    InitFailed(String),
    /// Plugin initialization failed (alternate)
    InitError(String),
    /// Processing error
    ProcessError(String),
    /// Parameter error
    ParameterError(String),
    /// Preset error
    PresetError(String),
    /// IO error
    IoError(std::io::Error),
    /// Serialization error
    SerdeError(String),
    /// Plugin format not supported on this platform
    UnsupportedFormat(PluginFormat),
    /// Feature not supported (e.g., VST3 SDK not available)
    NotSupported(String),
    /// Plugin scanning error
    ScanError(String),
}

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginError::NotFound(name) => write!(f, "Plugin not found: {}", name),
            PluginError::LoadFailed(msg) => write!(f, "Failed to load plugin: {}", msg),
            PluginError::LoadError(msg) => write!(f, "Failed to load plugin: {}", msg),
            PluginError::InitFailed(msg) => write!(f, "Plugin initialization failed: {}", msg),
            PluginError::InitError(msg) => write!(f, "Plugin initialization failed: {}", msg),
            PluginError::ProcessError(msg) => write!(f, "Processing error: {}", msg),
            PluginError::ParameterError(msg) => write!(f, "Parameter error: {}", msg),
            PluginError::PresetError(msg) => write!(f, "Preset error: {}", msg),
            PluginError::IoError(e) => write!(f, "IO error: {}", e),
            PluginError::SerdeError(msg) => write!(f, "Serialization error: {}", msg),
            PluginError::UnsupportedFormat(fmt) => {
                write!(f, "Plugin format {} not supported on this platform", fmt)
            }
            PluginError::NotSupported(msg) => write!(f, "Not supported: {}", msg),
            PluginError::ScanError(msg) => write!(f, "Plugin scan error: {}", msg),
        }
    }
}

impl std::error::Error for PluginError {}

impl From<std::io::Error> for PluginError {
    fn from(e: std::io::Error) -> Self {
        PluginError::IoError(e)
    }
}

/// Result type for plugin operations
pub type PluginResult<T> = Result<T, PluginError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_format_display() {
        assert_eq!(format!("{}", PluginFormat::Vst3), "VST3");
        assert_eq!(format!("{}", PluginFormat::AudioUnit), "AU");
        assert_eq!(format!("{}", PluginFormat::Clap), "CLAP");
        assert_eq!(format!("{}", PluginFormat::Lv2), "LV2");
    }

    #[test]
    fn test_plugin_id_display() {
        let id = PluginId {
            format: PluginFormat::Vst3,
            identifier: "/path/to/plugin.vst3".to_string(),
            name: "Test Synth".to_string(),
        };
        assert_eq!(format!("{}", id), "Test Synth (VST3)");
    }

    #[test]
    fn test_find_parameter() {
        let info = PluginInfo {
            id: PluginId {
                format: PluginFormat::Vst3,
                identifier: "test".to_string(),
                name: "Test".to_string(),
            },
            vendor: "Test".to_string(),
            version: "1.0".to_string(),
            category: PluginCategory::Instrument,
            num_inputs: 0,
            num_outputs: 2,
            parameters: vec![
                ParameterInfo::new(0, "Cutoff"),
                ParameterInfo::new(1, "Resonance"),
                ParameterInfo::new(2, "Filter Envelope"),
            ],
            factory_presets: vec![],
            has_gui: true,
            path: "/path".to_string(),
        };

        // Exact match
        assert!(info.find_parameter("Cutoff").is_some());

        // Case insensitive
        assert!(info.find_parameter("cutoff").is_some());

        // Prefix match
        assert!(info.find_parameter("Filter").is_some());
        assert_eq!(info.find_parameter("Filter").unwrap().index, 2);

        // Not found
        assert!(info.find_parameter("Volume").is_none());
    }
}
