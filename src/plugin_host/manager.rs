//! Plugin Instance Manager
//!
//! Manages named plugin instances that can be referenced in Phonon code.
//! Handles instance lifecycle, naming, and state persistence.

use super::instance::{PluginInstanceHandle, SharedPluginInstance};
use super::registry::PluginRegistry;
use super::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// A named plugin instance that can be referenced in code
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NamedPluginInstance {
    /// User-assigned name (e.g., "osirus:1")
    pub name: String,
    /// Plugin ID (used to reload the plugin)
    pub plugin_id: PluginId,
    /// Current parameter values
    pub param_values: Vec<f32>,
    /// Current preset name (if any)
    pub preset_name: Option<String>,
    /// User notes/description
    pub notes: String,
}

impl NamedPluginInstance {
    /// Create a new named instance
    pub fn new(name: String, plugin_id: PluginId) -> Self {
        Self {
            name,
            plugin_id,
            param_values: Vec::new(),
            preset_name: None,
            notes: String::new(),
        }
    }
}

/// Plugin settings file format
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PluginSettings {
    /// Version for future compatibility
    pub version: u32,
    /// Named plugin instances
    pub instances: Vec<NamedPluginInstance>,
}

impl PluginSettings {
    /// Current settings version
    pub const CURRENT_VERSION: u32 = 1;

    /// Create empty settings
    pub fn new() -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            instances: Vec::new(),
        }
    }

    /// Load settings from file
    pub fn load(path: &Path) -> PluginResult<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }
        let data = std::fs::read_to_string(path)?;
        serde_json::from_str(&data).map_err(|e| PluginError::SerdeError(e.to_string()))
    }

    /// Save settings to file
    pub fn save(&self, path: &Path) -> PluginResult<()> {
        let data =
            serde_json::to_string_pretty(self).map_err(|e| PluginError::SerdeError(e.to_string()))?;
        std::fs::write(path, data)?;
        Ok(())
    }
}

/// Manages all plugin instances for a Phonon session
pub struct PluginInstanceManager {
    /// Plugin registry for looking up plugins
    registry: PluginRegistry,
    /// Named instances (user-facing name -> instance)
    instances: HashMap<String, SharedPluginInstance>,
    /// Settings (for persistence)
    settings: PluginSettings,
    /// Path to settings file (derived from .ph file)
    settings_path: Option<PathBuf>,
    /// Sample rate for initializing plugins
    sample_rate: f32,
    /// Block size for initializing plugins
    block_size: usize,
    /// Counter for auto-generating unique names
    instance_counter: HashMap<String, usize>,
}

impl Default for PluginInstanceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginInstanceManager {
    /// Create a new plugin instance manager
    pub fn new() -> Self {
        Self {
            registry: PluginRegistry::new(),
            instances: HashMap::new(),
            settings: PluginSettings::new(),
            settings_path: None,
            sample_rate: 44100.0,
            block_size: 512,
            instance_counter: HashMap::new(),
        }
    }

    /// Create a manager with a settings file path
    pub fn with_settings_path(path: PathBuf) -> Self {
        let mut manager = Self::new();
        manager.settings_path = Some(path);
        manager
    }

    /// Derive settings path from a .ph file path
    /// e.g., "foo.ph" -> "foo.ph.plugins"
    pub fn settings_path_for_ph_file(ph_path: &Path) -> PathBuf {
        let mut path = ph_path.to_path_buf();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "untitled".to_string());
        path.set_file_name(format!("{}.plugins", name));
        path
    }

    /// Initialize the manager with audio settings
    pub fn initialize(&mut self, sample_rate: f32, block_size: usize) -> PluginResult<()> {
        self.sample_rate = sample_rate;
        self.block_size = block_size;

        // Scan for plugins if not already done
        if !self.registry.is_scanned() {
            self.registry.scan()?;
        }

        Ok(())
    }

    /// Load settings and restore instances
    pub fn load_settings(&mut self, path: &Path) -> PluginResult<usize> {
        self.settings_path = Some(path.to_path_buf());
        self.settings = PluginSettings::load(path)?;

        let mut loaded = 0;
        for instance_data in &self.settings.instances {
            // Find the plugin in registry
            if let Some(info) = self.registry.find(&instance_data.plugin_id.name) {
                // Create and initialize instance
                let mut handle = PluginInstanceHandle::new(info.clone());
                if handle.initialize(self.sample_rate, self.block_size).is_ok() {
                    // Restore parameter values
                    for (i, &value) in instance_data.param_values.iter().enumerate() {
                        let _ = handle.set_parameter(i, value);
                    }

                    // Store as shared instance
                    self.instances
                        .insert(instance_data.name.clone(), Arc::new(Mutex::new(handle)));
                    loaded += 1;
                }
            }
        }

        Ok(loaded)
    }

    /// Save current settings to file
    pub fn save_settings(&mut self) -> PluginResult<()> {
        // Update settings from current instances
        self.settings.instances.clear();
        for (name, instance) in &self.instances {
            let handle = instance.lock().unwrap();
            let info = handle.info();

            // Get current parameter values
            let param_values: Vec<f32> = (0..info.parameters.len())
                .filter_map(|i| handle.get_parameter(i).ok())
                .collect();

            self.settings.instances.push(NamedPluginInstance {
                name: name.clone(),
                plugin_id: info.id.clone(),
                param_values,
                preset_name: None, // TODO: Track preset name
                notes: String::new(),
            });
        }

        // Save to file
        if let Some(path) = &self.settings_path {
            self.settings.save(path)?;
        }

        Ok(())
    }

    /// Create a new plugin instance with a unique name
    pub fn create_instance(&mut self, plugin_name: &str) -> PluginResult<String> {
        // Find the plugin
        let info = self
            .registry
            .find(plugin_name)
            .ok_or_else(|| PluginError::NotFound(plugin_name.to_string()))?
            .clone();

        // Generate unique instance name
        let base_name = info.id.name.to_lowercase().replace(' ', "_");
        let counter = self.instance_counter.entry(base_name.clone()).or_insert(0);
        *counter += 1;
        let instance_name = format!("{}:{}", base_name, counter);

        // Create and initialize instance
        let mut handle = PluginInstanceHandle::new(info);
        handle.initialize(self.sample_rate, self.block_size)?;

        // Store
        self.instances
            .insert(instance_name.clone(), Arc::new(Mutex::new(handle)));

        Ok(instance_name)
    }

    /// Create an instance with a specific name
    pub fn create_named_instance(
        &mut self,
        plugin_name: &str,
        instance_name: &str,
    ) -> PluginResult<()> {
        // Check for duplicate name
        if self.instances.contains_key(instance_name) {
            return Err(PluginError::InitFailed(format!(
                "Instance '{}' already exists",
                instance_name
            )));
        }

        // Find the plugin
        let info = self
            .registry
            .find(plugin_name)
            .ok_or_else(|| PluginError::NotFound(plugin_name.to_string()))?
            .clone();

        // Create and initialize instance
        let mut handle = PluginInstanceHandle::new(info);
        handle.initialize(self.sample_rate, self.block_size)?;

        // Store
        self.instances
            .insert(instance_name.to_string(), Arc::new(Mutex::new(handle)));

        Ok(())
    }

    /// Get an instance by name
    pub fn get_instance(&self, name: &str) -> Option<SharedPluginInstance> {
        // Handle both with and without ~ prefix
        let name = name.trim_start_matches('~');
        self.instances.get(name).cloned()
    }

    /// Remove an instance by name
    pub fn remove_instance(&mut self, name: &str) -> bool {
        let name = name.trim_start_matches('~');
        self.instances.remove(name).is_some()
    }

    /// List all instance names
    pub fn list_instances(&self) -> Vec<String> {
        self.instances.keys().cloned().collect()
    }

    /// List available plugins
    pub fn list_plugins(&self) -> Vec<&PluginInfo> {
        self.registry.list()
    }

    /// Search for plugins by name
    pub fn search_plugins(&self, pattern: &str) -> Vec<&PluginInfo> {
        self.registry.search(pattern)
    }

    /// Get the registry for direct access
    pub fn registry(&self) -> &PluginRegistry {
        &self.registry
    }

    /// Get mutable registry for scanning
    pub fn registry_mut(&mut self) -> &mut PluginRegistry {
        &mut self.registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_registry() -> PluginRegistry {
        let mut registry = PluginRegistry::new();

        // Add some test plugins
        let plugin1 = PluginInfo {
            id: PluginId {
                format: PluginFormat::Vst3,
                identifier: "/path/to/osirus.vst3".to_string(),
                name: "Osirus".to_string(),
            },
            vendor: "TUS".to_string(),
            version: "1.0".to_string(),
            category: PluginCategory::Instrument,
            num_inputs: 0,
            num_outputs: 2,
            parameters: vec![ParameterInfo::new(0, "Cutoff"), ParameterInfo::new(1, "Resonance")],
            factory_presets: vec![],
            has_gui: true,
            path: "/path/to/osirus.vst3".to_string(),
        };
        registry.add_plugin(plugin1);

        let plugin2 = PluginInfo {
            id: PluginId {
                format: PluginFormat::Vst3,
                identifier: "/path/to/diva.vst3".to_string(),
                name: "Diva".to_string(),
            },
            vendor: "u-he".to_string(),
            version: "1.0".to_string(),
            category: PluginCategory::Instrument,
            num_inputs: 0,
            num_outputs: 2,
            parameters: vec![],
            factory_presets: vec![],
            has_gui: true,
            path: "/path/to/diva.vst3".to_string(),
        };
        registry.add_plugin(plugin2);

        registry
    }

    #[test]
    fn test_create_instance() {
        let mut manager = PluginInstanceManager::new();
        manager.registry = make_test_registry();

        let name = manager.create_instance("Osirus").unwrap();
        assert_eq!(name, "osirus:1");

        // Second instance gets :2
        let name2 = manager.create_instance("Osirus").unwrap();
        assert_eq!(name2, "osirus:2");
    }

    #[test]
    fn test_named_instance() {
        let mut manager = PluginInstanceManager::new();
        manager.registry = make_test_registry();

        manager
            .create_named_instance("Osirus", "my_virus")
            .unwrap();

        assert!(manager.get_instance("my_virus").is_some());
        assert!(manager.get_instance("~my_virus").is_some()); // With ~ prefix

        // Duplicate name fails
        assert!(manager
            .create_named_instance("Diva", "my_virus")
            .is_err());
    }

    #[test]
    fn test_list_instances() {
        let mut manager = PluginInstanceManager::new();
        manager.registry = make_test_registry();

        manager.create_instance("Osirus").unwrap();
        manager.create_named_instance("Diva", "bass").unwrap();

        let names = manager.list_instances();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"osirus:1".to_string()));
        assert!(names.contains(&"bass".to_string()));
    }

    #[test]
    fn test_settings_path() {
        let ph_path = PathBuf::from("/home/user/music/song.ph");
        let settings_path = PluginInstanceManager::settings_path_for_ph_file(&ph_path);
        assert_eq!(settings_path, PathBuf::from("/home/user/music/song.ph.plugins"));
    }
}
