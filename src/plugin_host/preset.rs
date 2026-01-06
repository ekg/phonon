//! Preset Management
//!
//! Handles loading and saving plugin presets in various formats:
//! - Phonon preset files (.ph) - human-readable, version-controllable
//! - FXP/FXB files - standard VST preset format
//! - Plugin's native state (opaque binary)

use super::types::*;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Phonon preset file format
/// Human-readable, designed for version control
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PhononPreset {
    /// Plugin name this preset is for
    pub plugin_name: String,
    /// Plugin version (for compatibility checking)
    pub plugin_version: Option<String>,
    /// Human-readable parameter values
    pub parameters: HashMap<String, f64>,
    /// Plugin's opaque binary state (base64 encoded)
    /// Used for parameters we can't map by name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_state: Option<String>,
    /// Preset metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<PresetMetadata>,
}

/// Optional preset metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PresetMetadata {
    /// Preset author
    pub author: Option<String>,
    /// Description
    pub description: Option<String>,
    /// Tags for categorization
    pub tags: Option<Vec<String>>,
}

impl PhononPreset {
    /// Create a new empty preset for a plugin
    pub fn new(plugin_name: &str) -> Self {
        Self {
            plugin_name: plugin_name.to_string(),
            plugin_version: None,
            parameters: HashMap::new(),
            binary_state: None,
            metadata: None,
        }
    }

    /// Create preset from plugin instance
    pub fn from_instance(
        info: &PluginInfo,
        param_values: &[f32],
        binary_state: Option<Vec<u8>>,
    ) -> Self {
        let mut parameters = HashMap::new();

        for (i, param) in info.parameters.iter().enumerate() {
            if i < param_values.len() {
                parameters.insert(param.name.clone(), param_values[i] as f64);
            }
        }

        Self {
            plugin_name: info.id.name.clone(),
            plugin_version: Some(info.version.clone()),
            parameters,
            binary_state: binary_state.map(|data| BASE64.encode(&data)),
            metadata: None,
        }
    }

    /// Set a parameter value
    pub fn set_parameter(&mut self, name: &str, value: f64) {
        self.parameters.insert(name.to_string(), value);
    }

    /// Get a parameter value
    pub fn get_parameter(&self, name: &str) -> Option<f64> {
        self.parameters.get(name).copied()
    }

    /// Get binary state as bytes
    pub fn get_binary_state(&self) -> Option<Vec<u8>> {
        self.binary_state
            .as_ref()
            .and_then(|s| BASE64.decode(s).ok())
    }

    /// Set binary state from bytes
    pub fn set_binary_state(&mut self, data: Vec<u8>) {
        self.binary_state = Some(BASE64.encode(&data));
    }

    /// Load from a .ph file
    pub fn load(path: &Path) -> PluginResult<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::parse(&content)
    }

    /// Parse from string content
    pub fn parse(content: &str) -> PluginResult<Self> {
        // Try TOML format first
        if let Ok(preset) = toml::from_str(content) {
            return Ok(preset);
        }

        // Try JSON format
        if let Ok(preset) = serde_json::from_str(content) {
            return Ok(preset);
        }

        // Try simple key=value format
        Self::parse_simple(content)
    }

    /// Parse simple key=value format
    fn parse_simple(content: &str) -> PluginResult<Self> {
        let mut preset = PhononPreset::new("");
        let mut in_preset_block = false;

        for line in content.lines() {
            let line = line.trim();

            // Skip comments
            if line.starts_with("--") || line.starts_with('#') || line.is_empty() {
                continue;
            }

            // Check for vst_preset header
            if line.starts_with("vst_preset") {
                if let Some(name_start) = line.find('"') {
                    if let Some(name_end) = line[name_start + 1..].find('"') {
                        preset.plugin_name = line[name_start + 1..name_start + 1 + name_end].to_string();
                    }
                }
                in_preset_block = line.ends_with('{');
                continue;
            }

            // Check for block end
            if line == "}" {
                in_preset_block = false;
                continue;
            }

            // Parse key: value pairs
            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim();
                let value = line[colon_pos + 1..].trim();

                // Skip internal keys
                if key.starts_with('_') {
                    if key == "_binary" {
                        preset.binary_state = Some(value.trim_matches('"').to_string());
                    }
                    continue;
                }

                // Try to parse as number
                if let Ok(num) = value.parse::<f64>() {
                    preset.parameters.insert(key.to_string(), num);
                }
            }
        }

        if preset.plugin_name.is_empty() {
            return Err(PluginError::PresetError(
                "Could not parse preset: missing plugin name".to_string(),
            ));
        }

        Ok(preset)
    }

    /// Save to a .ph file (TOML format)
    pub fn save(&self, path: &Path) -> PluginResult<()> {
        let content = self.to_toml()?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Convert to TOML string
    pub fn to_toml(&self) -> PluginResult<String> {
        toml::to_string_pretty(self).map_err(|e| PluginError::SerdeError(e.to_string()))
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> PluginResult<String> {
        serde_json::to_string_pretty(self).map_err(|e| PluginError::SerdeError(e.to_string()))
    }

    /// Convert to human-readable format
    pub fn to_readable(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("-- Phonon preset for {}", self.plugin_name));
        if let Some(ref version) = self.plugin_version {
            lines.push(format!("-- Plugin version: {}", version));
        }
        lines.push(String::new());
        lines.push(format!("vst_preset \"{}\" {{", self.plugin_name));

        // Sort parameters by name for consistent output
        let mut params: Vec<_> = self.parameters.iter().collect();
        params.sort_by_key(|(k, _)| k.as_str());

        for (name, value) in params {
            lines.push(format!("    {}: {:.6}", name, value));
        }

        if let Some(ref binary) = self.binary_state {
            lines.push(String::new());
            lines.push(format!("    _binary: \"{}\"", binary));
        }

        lines.push("}".to_string());

        lines.join("\n")
    }
}

/// Import/export FXP (VST preset) format
pub struct FxpFormat;

impl FxpFormat {
    /// FXP magic number
    const MAGIC: [u8; 4] = [b'C', b'c', b'n', b'K'];
    /// FXP version for regular preset
    const FX_PRESET: [u8; 4] = [b'F', b'x', b'C', b'k'];
    /// FXP version for opaque chunk
    const FX_CHUNK: [u8; 4] = [b'F', b'P', b'C', b'h'];

    /// Parse FXP data
    pub fn parse(data: &[u8]) -> PluginResult<PhononPreset> {
        if data.len() < 60 {
            return Err(PluginError::PresetError("FXP file too short".to_string()));
        }

        // Verify magic
        if &data[0..4] != &Self::MAGIC {
            return Err(PluginError::PresetError("Invalid FXP magic".to_string()));
        }

        let fxp_type = &data[8..12];
        let _version = u32::from_be_bytes([data[12], data[13], data[14], data[15]]);
        let _plugin_id = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
        let _plugin_version = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);

        // Extract preset name (28 bytes, null-terminated)
        let name_bytes = &data[28..56];
        let name_end = name_bytes.iter().position(|&b| b == 0).unwrap_or(28);
        let preset_name = String::from_utf8_lossy(&name_bytes[..name_end]).to_string();

        let mut preset = PhononPreset::new(&preset_name);

        if fxp_type == &Self::FX_PRESET {
            // Regular preset with float parameters
            let num_params = u32::from_be_bytes([data[24], data[25], data[26], data[27]]) as usize;
            let param_data = &data[56..];

            for i in 0..num_params {
                if (i + 1) * 4 <= param_data.len() {
                    let value = f32::from_be_bytes([
                        param_data[i * 4],
                        param_data[i * 4 + 1],
                        param_data[i * 4 + 2],
                        param_data[i * 4 + 3],
                    ]);
                    preset.parameters.insert(format!("p{}", i), value as f64);
                }
            }
        } else if fxp_type == &Self::FX_CHUNK {
            // Opaque chunk preset
            let chunk_size = u32::from_be_bytes([data[56], data[57], data[58], data[59]]) as usize;
            if data.len() >= 60 + chunk_size {
                preset.set_binary_state(data[60..60 + chunk_size].to_vec());
            }
        } else {
            return Err(PluginError::PresetError(format!(
                "Unknown FXP type: {:?}",
                fxp_type
            )));
        }

        Ok(preset)
    }

    /// Export to FXP format (opaque chunk)
    pub fn export(preset: &PhononPreset) -> PluginResult<Vec<u8>> {
        let chunk_data = preset.get_binary_state().unwrap_or_default();
        let total_size = 60 + chunk_data.len();

        let mut data = vec![0u8; total_size];

        // Magic
        data[0..4].copy_from_slice(&Self::MAGIC);

        // Size (big endian)
        let size = (total_size - 8) as u32;
        data[4..8].copy_from_slice(&size.to_be_bytes());

        // FXP type (chunk)
        data[8..12].copy_from_slice(&Self::FX_CHUNK);

        // Version
        data[12..16].copy_from_slice(&1u32.to_be_bytes());

        // Plugin ID (placeholder)
        data[16..20].copy_from_slice(&0u32.to_be_bytes());

        // Plugin version (placeholder)
        data[20..24].copy_from_slice(&1u32.to_be_bytes());

        // Num programs
        data[24..28].copy_from_slice(&1u32.to_be_bytes());

        // Preset name (28 bytes max)
        let name_bytes = preset.plugin_name.as_bytes();
        let name_len = name_bytes.len().min(27);
        data[28..28 + name_len].copy_from_slice(&name_bytes[..name_len]);

        // Chunk size
        data[56..60].copy_from_slice(&(chunk_data.len() as u32).to_be_bytes());

        // Chunk data
        data[60..].copy_from_slice(&chunk_data);

        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phonon_preset_basic() {
        let mut preset = PhononPreset::new("Test Synth");
        preset.set_parameter("cutoff", 0.5);
        preset.set_parameter("resonance", 0.7);

        assert_eq!(preset.get_parameter("cutoff"), Some(0.5));
        assert_eq!(preset.get_parameter("resonance"), Some(0.7));
        assert_eq!(preset.get_parameter("unknown"), None);
    }

    #[test]
    fn test_phonon_preset_toml() {
        let mut preset = PhononPreset::new("Test Synth");
        preset.plugin_version = Some("1.0.0".to_string());
        preset.set_parameter("cutoff", 0.5);

        let toml = preset.to_toml().unwrap();
        assert!(toml.contains("plugin_name = \"Test Synth\""));
        assert!(toml.contains("cutoff = 0.5"));

        let parsed = PhononPreset::parse(&toml).unwrap();
        assert_eq!(parsed.plugin_name, "Test Synth");
        assert_eq!(parsed.get_parameter("cutoff"), Some(0.5));
    }

    #[test]
    fn test_phonon_preset_readable() {
        let mut preset = PhononPreset::new("Osirus");
        preset.set_parameter("cutoff", 0.35);
        preset.set_parameter("resonance", 0.72);

        let readable = preset.to_readable();
        assert!(readable.contains("vst_preset \"Osirus\""));
        assert!(readable.contains("cutoff: 0.350000"));
        assert!(readable.contains("resonance: 0.720000"));
    }

    #[test]
    fn test_phonon_preset_binary_state() {
        let mut preset = PhononPreset::new("Test");
        let data = vec![1, 2, 3, 4, 5];
        preset.set_binary_state(data.clone());

        assert_eq!(preset.get_binary_state(), Some(data));
    }

    #[test]
    fn test_parse_simple_format() {
        let content = r#"
-- Phonon preset for Osirus
vst_preset "Osirus" {
    cutoff: 0.5
    resonance: 0.7
}
"#;

        let preset = PhononPreset::parse(content).unwrap();
        assert_eq!(preset.plugin_name, "Osirus");
        assert_eq!(preset.get_parameter("cutoff"), Some(0.5));
        assert_eq!(preset.get_parameter("resonance"), Some(0.7));
    }
}
