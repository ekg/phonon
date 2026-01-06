//! Parameter Automation
//!
//! Provides sample-accurate parameter automation from Phonon patterns.
//! Maps parameter names to indices and generates per-sample automation data.

use super::types::*;
use std::collections::HashMap;

/// Maps parameter names to indices with caching
pub struct ParameterMapper {
    /// Full name to index mapping (lowercase)
    name_to_index: HashMap<String, usize>,
    /// Short name to index mapping (lowercase)
    short_name_to_index: HashMap<String, usize>,
    /// Index to name mapping (for reverse lookup)
    index_to_name: Vec<String>,
}

impl ParameterMapper {
    /// Create a new parameter mapper from plugin info
    pub fn from_plugin_info(info: &PluginInfo) -> Self {
        let mut name_to_index = HashMap::new();
        let mut short_name_to_index = HashMap::new();
        let mut index_to_name = Vec::new();

        for param in &info.parameters {
            let name_lower = param.name.to_lowercase();
            let short_lower = param.short_name.to_lowercase();

            name_to_index.insert(name_lower, param.index);
            short_name_to_index.insert(short_lower, param.index);

            // Extend index_to_name to fit this index
            while index_to_name.len() <= param.index {
                index_to_name.push(String::new());
            }
            index_to_name[param.index] = param.name.clone();
        }

        Self {
            name_to_index,
            short_name_to_index,
            index_to_name,
        }
    }

    /// Create an empty mapper
    pub fn new() -> Self {
        Self {
            name_to_index: HashMap::new(),
            short_name_to_index: HashMap::new(),
            index_to_name: Vec::new(),
        }
    }

    /// Resolve parameter name to index
    /// Tries exact match, then short name, then prefix match
    pub fn resolve(&self, name: &str) -> Option<usize> {
        let name_lower = name.to_lowercase();

        // Exact full name match
        if let Some(&index) = self.name_to_index.get(&name_lower) {
            return Some(index);
        }

        // Exact short name match
        if let Some(&index) = self.short_name_to_index.get(&name_lower) {
            return Some(index);
        }

        // Prefix match on full name
        for (key, &index) in &self.name_to_index {
            if key.starts_with(&name_lower) {
                return Some(index);
            }
        }

        // Try parsing as index (p0, p1, etc.)
        if name_lower.starts_with('p') {
            if let Ok(index) = name_lower[1..].parse::<usize>() {
                if index < self.index_to_name.len() {
                    return Some(index);
                }
            }
        }

        None
    }

    /// Get parameter name by index
    pub fn get_name(&self, index: usize) -> Option<&str> {
        self.index_to_name.get(index).map(|s| s.as_str())
    }

    /// Get all parameter names
    pub fn all_names(&self) -> Vec<&str> {
        self.index_to_name.iter().map(|s| s.as_str()).collect()
    }

    /// Number of parameters
    pub fn len(&self) -> usize {
        self.index_to_name.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.index_to_name.is_empty()
    }
}

impl Default for ParameterMapper {
    fn default() -> Self {
        Self::new()
    }
}

/// Sample-accurate parameter automation
#[derive(Clone, Debug)]
pub struct ParameterAutomation {
    /// Parameter index
    pub param_index: usize,
    /// Automation points: (sample_offset, value)
    pub points: Vec<(usize, f32)>,
}

impl ParameterAutomation {
    /// Create new automation for a parameter
    pub fn new(param_index: usize) -> Self {
        Self {
            param_index,
            points: Vec::new(),
        }
    }

    /// Add an automation point
    pub fn add_point(&mut self, sample_offset: usize, value: f32) {
        self.points.push((sample_offset, value));
    }

    /// Sort points by sample offset
    pub fn sort(&mut self) {
        self.points.sort_by_key(|(offset, _)| *offset);
    }

    /// Get value at sample offset (with interpolation)
    pub fn get_value_at(&self, sample_offset: usize) -> Option<f32> {
        if self.points.is_empty() {
            return None;
        }

        // Find surrounding points
        let mut prev: Option<(usize, f32)> = None;
        let mut next: Option<(usize, f32)> = None;

        for &(offset, value) in &self.points {
            if offset <= sample_offset {
                prev = Some((offset, value));
            } else if next.is_none() {
                next = Some((offset, value));
                break;
            }
        }

        match (prev, next) {
            (Some((_, value)), None) => Some(value), // After last point
            (None, Some((_, value))) => Some(value), // Before first point
            (Some((t0, v0)), Some((t1, v1))) => {
                // Linear interpolation
                let t = (sample_offset - t0) as f32 / (t1 - t0) as f32;
                Some(v0 + t * (v1 - v0))
            }
            (None, None) => None,
        }
    }

    /// Get value at sample offset (stepped, no interpolation)
    pub fn get_value_stepped(&self, sample_offset: usize) -> Option<f32> {
        self.points
            .iter()
            .filter(|(offset, _)| *offset <= sample_offset)
            .last()
            .map(|(_, value)| *value)
    }
}

/// Collection of automation data for multiple parameters
#[derive(Clone, Debug, Default)]
pub struct AutomationBuffer {
    /// Automation data per parameter index
    automations: HashMap<usize, ParameterAutomation>,
}

impl AutomationBuffer {
    /// Create new empty automation buffer
    pub fn new() -> Self {
        Self {
            automations: HashMap::new(),
        }
    }

    /// Add automation for a parameter
    pub fn add(&mut self, automation: ParameterAutomation) {
        self.automations.insert(automation.param_index, automation);
    }

    /// Get automation for a parameter
    pub fn get(&self, param_index: usize) -> Option<&ParameterAutomation> {
        self.automations.get(&param_index)
    }

    /// Get mutable automation for a parameter (creates if needed)
    pub fn get_or_create(&mut self, param_index: usize) -> &mut ParameterAutomation {
        self.automations
            .entry(param_index)
            .or_insert_with(|| ParameterAutomation::new(param_index))
    }

    /// Get all parameter values at a sample offset
    pub fn get_values_at(&self, sample_offset: usize) -> HashMap<usize, f32> {
        self.automations
            .iter()
            .filter_map(|(&index, auto)| auto.get_value_at(sample_offset).map(|v| (index, v)))
            .collect()
    }

    /// Number of automated parameters
    pub fn len(&self) -> usize {
        self.automations.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.automations.is_empty()
    }

    /// Clear all automation data
    pub fn clear(&mut self) {
        self.automations.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_info() -> PluginInfo {
        PluginInfo {
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
                ParameterInfo {
                    index: 0,
                    name: "Filter Cutoff".to_string(),
                    short_name: "Cutoff".to_string(),
                    default_value: 0.5,
                    min_value: 0.0,
                    max_value: 1.0,
                    unit: "Hz".to_string(),
                    step_count: 0,
                    automatable: true,
                },
                ParameterInfo {
                    index: 1,
                    name: "Filter Resonance".to_string(),
                    short_name: "Reso".to_string(),
                    default_value: 0.0,
                    min_value: 0.0,
                    max_value: 1.0,
                    unit: "".to_string(),
                    step_count: 0,
                    automatable: true,
                },
            ],
            factory_presets: vec![],
            has_gui: true,
            path: "/path".to_string(),
        }
    }

    #[test]
    fn test_parameter_mapper_resolve() {
        let info = make_test_info();
        let mapper = ParameterMapper::from_plugin_info(&info);

        // Full name
        assert_eq!(mapper.resolve("Filter Cutoff"), Some(0));
        assert_eq!(mapper.resolve("filter cutoff"), Some(0)); // case insensitive

        // Short name
        assert_eq!(mapper.resolve("Cutoff"), Some(0));
        assert_eq!(mapper.resolve("Reso"), Some(1));

        // Prefix match (use unambiguous prefix)
        assert_eq!(mapper.resolve("Filter C"), Some(0));
        assert_eq!(mapper.resolve("Filter R"), Some(1));

        // Index syntax
        assert_eq!(mapper.resolve("p0"), Some(0));
        assert_eq!(mapper.resolve("p1"), Some(1));

        // Not found
        assert_eq!(mapper.resolve("Volume"), None);
    }

    #[test]
    fn test_automation_interpolation() {
        let mut auto = ParameterAutomation::new(0);
        auto.add_point(0, 0.0);
        auto.add_point(100, 1.0);
        auto.sort();

        // At points
        assert_eq!(auto.get_value_at(0), Some(0.0));
        assert_eq!(auto.get_value_at(100), Some(1.0));

        // Interpolated
        let mid = auto.get_value_at(50).unwrap();
        assert!((mid - 0.5).abs() < 0.01);

        // Beyond
        assert_eq!(auto.get_value_at(150), Some(1.0));
    }

    #[test]
    fn test_automation_stepped() {
        let mut auto = ParameterAutomation::new(0);
        auto.add_point(0, 0.0);
        auto.add_point(50, 0.5);
        auto.add_point(100, 1.0);
        auto.sort();

        assert_eq!(auto.get_value_stepped(0), Some(0.0));
        assert_eq!(auto.get_value_stepped(25), Some(0.0)); // Before next point
        assert_eq!(auto.get_value_stepped(50), Some(0.5));
        assert_eq!(auto.get_value_stepped(75), Some(0.5)); // Before next point
        assert_eq!(auto.get_value_stepped(100), Some(1.0));
    }

    #[test]
    fn test_automation_buffer() {
        let mut buffer = AutomationBuffer::new();

        let mut auto0 = ParameterAutomation::new(0);
        auto0.add_point(0, 0.5);
        buffer.add(auto0);

        let mut auto1 = ParameterAutomation::new(1);
        auto1.add_point(0, 0.7);
        buffer.add(auto1);

        let values = buffer.get_values_at(0);
        assert_eq!(values.get(&0), Some(&0.5));
        assert_eq!(values.get(&1), Some(&0.7));
    }
}
