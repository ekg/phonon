//! Plugin Registry
//!
//! Scans system paths for audio plugins and maintains a searchable registry.
//! Supports caching scan results for faster startup.

use super::types::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[cfg(feature = "vst3")]
use super::real_plugin::{convert_plugin_info, RealPluginScanner};

/// Plugin registry - scans, caches, and provides lookup for installed plugins
pub struct PluginRegistry {
    /// Plugins indexed by lowercase name
    plugins_by_name: HashMap<String, PluginInfo>,
    /// Plugins indexed by identifier
    plugins_by_id: HashMap<String, PluginInfo>,
    /// Cache file path
    cache_path: Option<PathBuf>,
    /// Whether registry has been scanned
    scanned: bool,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            plugins_by_name: HashMap::new(),
            plugins_by_id: HashMap::new(),
            cache_path: None,
            scanned: false,
        }
    }

    /// Create registry with cache path
    pub fn with_cache(cache_path: PathBuf) -> Self {
        Self {
            plugins_by_name: HashMap::new(),
            plugins_by_id: HashMap::new(),
            cache_path: Some(cache_path),
            scanned: false,
        }
    }

    /// Scan system paths for plugins (basic filesystem scan)
    pub fn scan(&mut self) -> PluginResult<usize> {
        // Get platform-specific scan paths
        let paths = Self::get_scan_paths();

        let mut count = 0;

        for path in paths {
            if path.exists() {
                count += self.scan_directory(&path)?;
            }
        }

        self.scanned = true;

        // Save cache if configured
        if let Some(cache_path) = &self.cache_path {
            let _ = self.save_cache(cache_path);
        }

        Ok(count)
    }

    /// Scan system paths for VST3 plugins using rack crate
    /// This provides full plugin metadata including parameters and presets
    #[cfg(feature = "vst3")]
    pub fn scan_with_rack(&mut self) -> PluginResult<usize> {
        let scanner = RealPluginScanner::new()?;
        let plugins = scanner.scan()?;

        let mut count = 0;
        for rack_info in &plugins {
            let info = convert_plugin_info(rack_info);
            self.add_plugin(info);
            count += 1;
        }

        self.scanned = true;

        // Save cache if configured
        if let Some(cache_path) = &self.cache_path {
            let _ = self.save_cache(cache_path);
        }

        Ok(count)
    }

    /// Scan a specific path for VST3 plugins using rack crate
    #[cfg(feature = "vst3")]
    pub fn scan_path_with_rack(&mut self, path: &Path) -> PluginResult<usize> {
        let scanner = RealPluginScanner::new()?;
        let plugins = scanner.scan_path(path)?;

        let mut count = 0;
        for rack_info in &plugins {
            let info = convert_plugin_info(rack_info);
            self.add_plugin(info);
            count += 1;
        }

        Ok(count)
    }

    /// Scan a directory for plugins
    fn scan_directory(&mut self, path: &Path) -> PluginResult<usize> {
        let mut count = 0;

        let entries = match std::fs::read_dir(path) {
            Ok(e) => e,
            Err(_) => return Ok(0),
        };

        for entry in entries.flatten() {
            let entry_path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            // VST3 bundles are directories ending in .vst3
            if entry_path.is_dir() && name.ends_with(".vst3") {
                let plugin_name = name.trim_end_matches(".vst3").to_string();
                let info = PluginInfo {
                    id: PluginId {
                        format: PluginFormat::Vst3,
                        identifier: entry_path.to_string_lossy().to_string(),
                        name: plugin_name.clone(),
                    },
                    vendor: "Unknown".to_string(),
                    version: "1.0.0".to_string(),
                    category: PluginCategory::Instrument, // Default to instrument
                    num_inputs: 0,
                    num_outputs: 2,
                    parameters: vec![], // TODO: Query from plugin
                    factory_presets: vec![],
                    has_gui: true,
                    path: entry_path.to_string_lossy().to_string(),
                };
                self.add_plugin(info);
                count += 1;
            }

            // CLAP plugins are single files ending in .clap
            if entry_path.is_file() && name.ends_with(".clap") {
                let plugin_name = name.trim_end_matches(".clap").to_string();
                let info = PluginInfo {
                    id: PluginId {
                        format: PluginFormat::Clap,
                        identifier: entry_path.to_string_lossy().to_string(),
                        name: plugin_name.clone(),
                    },
                    vendor: "Unknown".to_string(),
                    version: "1.0.0".to_string(),
                    category: PluginCategory::Instrument,
                    num_inputs: 0,
                    num_outputs: 2,
                    parameters: vec![],
                    factory_presets: vec![],
                    has_gui: true,
                    path: entry_path.to_string_lossy().to_string(),
                };
                self.add_plugin(info);
                count += 1;
            }

            // LV2 bundles are directories ending in .lv2
            if entry_path.is_dir() && name.ends_with(".lv2") {
                let plugin_name = name.trim_end_matches(".lv2").to_string();
                let info = PluginInfo {
                    id: PluginId {
                        format: PluginFormat::Lv2,
                        identifier: entry_path.to_string_lossy().to_string(),
                        name: plugin_name.clone(),
                    },
                    vendor: "Unknown".to_string(),
                    version: "1.0.0".to_string(),
                    category: PluginCategory::Instrument,
                    num_inputs: 0,
                    num_outputs: 2,
                    parameters: vec![],
                    factory_presets: vec![],
                    has_gui: true,
                    path: entry_path.to_string_lossy().to_string(),
                };
                self.add_plugin(info);
                count += 1;
            }
        }

        Ok(count)
    }

    /// Get platform-specific plugin scan paths
    fn get_scan_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        #[cfg(target_os = "linux")]
        {
            // VST3 paths
            if let Ok(home) = std::env::var("HOME") {
                paths.push(PathBuf::from(format!("{}/.vst3", home)));
            }
            paths.push(PathBuf::from("/usr/lib/vst3"));
            paths.push(PathBuf::from("/usr/local/lib/vst3"));

            // LV2 paths
            if let Ok(home) = std::env::var("HOME") {
                paths.push(PathBuf::from(format!("{}/.lv2", home)));
            }
            paths.push(PathBuf::from("/usr/lib/lv2"));
            paths.push(PathBuf::from("/usr/local/lib/lv2"));

            // CLAP paths
            if let Ok(home) = std::env::var("HOME") {
                paths.push(PathBuf::from(format!("{}/.clap", home)));
            }
            paths.push(PathBuf::from("/usr/lib/clap"));
        }

        #[cfg(target_os = "macos")]
        {
            // Audio Units
            if let Ok(home) = std::env::var("HOME") {
                paths.push(PathBuf::from(format!(
                    "{}/Library/Audio/Plug-Ins/Components",
                    home
                )));
            }
            paths.push(PathBuf::from("/Library/Audio/Plug-Ins/Components"));

            // VST3
            if let Ok(home) = std::env::var("HOME") {
                paths.push(PathBuf::from(format!(
                    "{}/Library/Audio/Plug-Ins/VST3",
                    home
                )));
            }
            paths.push(PathBuf::from("/Library/Audio/Plug-Ins/VST3"));

            // CLAP
            if let Ok(home) = std::env::var("HOME") {
                paths.push(PathBuf::from(format!(
                    "{}/Library/Audio/Plug-Ins/CLAP",
                    home
                )));
            }
            paths.push(PathBuf::from("/Library/Audio/Plug-Ins/CLAP"));
        }

        #[cfg(target_os = "windows")]
        {
            // VST3 paths
            paths.push(PathBuf::from("C:\\Program Files\\Common Files\\VST3"));
            paths.push(PathBuf::from(
                "C:\\Program Files (x86)\\Common Files\\VST3",
            ));

            // CLAP paths
            paths.push(PathBuf::from("C:\\Program Files\\Common Files\\CLAP"));
        }

        paths
    }

    /// Load registry from cache
    pub fn load_cache(&mut self, path: &Path) -> PluginResult<usize> {
        if !path.exists() {
            return Ok(0);
        }

        let data = std::fs::read_to_string(path)?;
        let plugins: Vec<PluginInfo> =
            serde_json::from_str(&data).map_err(|e| PluginError::SerdeError(e.to_string()))?;

        let count = plugins.len();
        for plugin in plugins {
            self.add_plugin(plugin);
        }

        self.scanned = true;
        Ok(count)
    }

    /// Save registry to cache
    pub fn save_cache(&self, path: &Path) -> PluginResult<()> {
        let plugins: Vec<&PluginInfo> = self.plugins_by_id.values().collect();
        let data =
            serde_json::to_string_pretty(&plugins).map_err(|e| PluginError::SerdeError(e.to_string()))?;
        std::fs::write(path, data)?;
        Ok(())
    }

    /// Add a plugin to the registry
    pub fn add_plugin(&mut self, info: PluginInfo) {
        let name_key = info.id.name.to_lowercase();
        self.plugins_by_name.insert(name_key, info.clone());
        self.plugins_by_id.insert(info.id.identifier.clone(), info);
    }

    /// Find plugin by name (case-insensitive, supports partial match)
    pub fn find(&self, name: &str) -> Option<&PluginInfo> {
        let name_lower = name.to_lowercase();

        // Exact match
        if let Some(info) = self.plugins_by_name.get(&name_lower) {
            return Some(info);
        }

        // Partial match (starts with)
        for (key, info) in &self.plugins_by_name {
            if key.starts_with(&name_lower) {
                return Some(info);
            }
        }

        // Fuzzy match (contains)
        for (key, info) in &self.plugins_by_name {
            if key.contains(&name_lower) {
                return Some(info);
            }
        }

        None
    }

    /// Search for plugins matching a pattern
    pub fn search(&self, pattern: &str) -> Vec<&PluginInfo> {
        let pattern_lower = pattern.to_lowercase();
        self.plugins_by_name
            .iter()
            .filter(|(key, _)| key.contains(&pattern_lower))
            .map(|(_, info)| info)
            .collect()
    }

    /// List all plugins
    pub fn list(&self) -> Vec<&PluginInfo> {
        self.plugins_by_id.values().collect()
    }

    /// List plugins by category
    pub fn list_by_category(&self, category: PluginCategory) -> Vec<&PluginInfo> {
        self.plugins_by_id
            .values()
            .filter(|p| p.category == category)
            .collect()
    }

    /// Get number of registered plugins
    pub fn len(&self) -> usize {
        self.plugins_by_id.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.plugins_by_id.is_empty()
    }

    /// Check if registry has been scanned
    pub fn is_scanned(&self) -> bool {
        self.scanned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_plugin(name: &str, category: PluginCategory) -> PluginInfo {
        let num_inputs = if matches!(category, PluginCategory::Instrument) {
            0
        } else {
            2
        };
        PluginInfo {
            id: PluginId {
                format: PluginFormat::Vst3,
                identifier: format!("/path/to/{}.vst3", name.to_lowercase()),
                name: name.to_string(),
            },
            vendor: "Test Vendor".to_string(),
            version: "1.0.0".to_string(),
            category,
            num_inputs,
            num_outputs: 2,
            parameters: vec![
                ParameterInfo::new(0, "Cutoff"),
                ParameterInfo::new(1, "Resonance"),
            ],
            factory_presets: vec!["Init".to_string(), "Lead".to_string()],
            has_gui: true,
            path: format!("/path/to/{}.vst3", name.to_lowercase()),
        }
    }

    #[test]
    fn test_registry_add_and_find() {
        let mut registry = PluginRegistry::new();

        registry.add_plugin(make_test_plugin("Osirus", PluginCategory::Instrument));
        registry.add_plugin(make_test_plugin("Diva", PluginCategory::Instrument));
        registry.add_plugin(make_test_plugin("Valhalla Room", PluginCategory::Effect));

        // Exact match
        assert!(registry.find("Osirus").is_some());
        assert!(registry.find("osirus").is_some()); // case insensitive

        // Partial match
        assert!(registry.find("Osi").is_some());

        // Fuzzy match
        assert!(registry.find("Room").is_some());

        // Not found
        assert!(registry.find("NotAPlugin").is_none());
    }

    #[test]
    fn test_registry_search() {
        let mut registry = PluginRegistry::new();

        registry.add_plugin(make_test_plugin("Osirus", PluginCategory::Instrument));
        registry.add_plugin(make_test_plugin("OsTIrus", PluginCategory::Instrument));
        registry.add_plugin(make_test_plugin("Diva", PluginCategory::Instrument));

        let results = registry.search("irus");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_registry_list_by_category() {
        let mut registry = PluginRegistry::new();

        registry.add_plugin(make_test_plugin("Osirus", PluginCategory::Instrument));
        registry.add_plugin(make_test_plugin("Valhalla Room", PluginCategory::Effect));
        registry.add_plugin(make_test_plugin("FabFilter Pro-Q", PluginCategory::Effect));

        let instruments = registry.list_by_category(PluginCategory::Instrument);
        assert_eq!(instruments.len(), 1);

        let effects = registry.list_by_category(PluginCategory::Effect);
        assert_eq!(effects.len(), 2);
    }
}
