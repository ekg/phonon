//! Pattern Bridge - Integration between Strudel patterns and signal graph
//!
//! This module enables bidirectional communication between pattern events
//! and the modular synthesis signal graph, allowing patterns to modulate
//! synthesis parameters and audio signals to affect pattern playback.

use crate::signal_executor::SignalExecutor;
use crate::signal_graph::{BusId, SignalGraph};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Pattern event from the Strudel engine
#[derive(Debug, Clone)]
pub struct PatternEvent {
    pub time: f64,
    pub value: PatternValue,
    pub duration: f64,
    pub velocity: f32,
}

/// Types of pattern values
#[derive(Debug, Clone)]
pub enum PatternValue {
    Note(String),    // Note name like "c3", "e4"
    Sample(String),  // Sample name like "bd", "sn"
    Number(f64),     // Numeric value
    Pattern(String), // Pattern string
}

/// Pattern state that can be used as a signal
#[derive(Debug, Clone)]
pub struct PatternSignal {
    pub current_value: f32,
    pub gate: bool,    // Is pattern active
    pub trigger: bool, // New event triggered
    pub velocity: f32, // Event velocity
}

/// Bridge between patterns and signal graph
pub struct PatternBridge {
    /// The signal graph being modulated
    signal_graph: Arc<RwLock<SignalGraph>>,

    /// Pattern signals that can be used as modulation sources
    pub pattern_signals: HashMap<String, PatternSignal>,

    /// Mapping from pattern names to bus IDs
    pattern_to_bus: HashMap<String, BusId>,

    /// Audio features extracted from signal graph for pattern use
    pub audio_features: HashMap<String, f32>,

    /// Sample rate for timing calculations
    sample_rate: f32,
}

impl PatternBridge {
    pub fn new(signal_graph: Arc<RwLock<SignalGraph>>, sample_rate: f32) -> Self {
        Self {
            signal_graph,
            pattern_signals: HashMap::new(),
            pattern_to_bus: HashMap::new(),
            audio_features: HashMap::new(),
            sample_rate,
        }
    }

    /// Register a pattern as a signal source
    pub fn register_pattern(&mut self, name: String) -> BusId {
        let bus_id = BusId(format!("pattern_{name}"));

        // Create pattern signal
        let signal = PatternSignal {
            current_value: 0.0,
            gate: false,
            trigger: false,
            velocity: 0.0,
        };

        self.pattern_signals.insert(name.clone(), signal);
        self.pattern_to_bus.insert(name, bus_id.clone());

        // Register bus in signal graph
        if let Ok(mut graph) = self.signal_graph.write() {
            graph.buses.insert(bus_id.clone(), 0.0);
        }

        bus_id
    }

    /// Process a pattern event and update the signal graph
    pub fn process_pattern_event(&mut self, pattern_name: &str, event: PatternEvent) {
        // Convert pattern value to signal value first
        let value = match event.value {
            PatternValue::Number(n) => n as f32,
            PatternValue::Note(ref note) => self.note_to_frequency(note),
            PatternValue::Sample(_) => 1.0, // Trigger value
            PatternValue::Pattern(_) => 1.0,
        };

        // Update pattern signal
        if let Some(signal) = self.pattern_signals.get_mut(pattern_name) {
            signal.trigger = true;
            signal.gate = true;
            signal.velocity = event.velocity;
            signal.current_value = value;
        }

        // Update bus value in signal graph
        if let Some(bus_id) = self.pattern_to_bus.get(pattern_name) {
            if let Ok(mut graph) = self.signal_graph.write() {
                graph.buses.insert(
                    bus_id.clone(),
                    self.pattern_signals[pattern_name].current_value,
                );
            }
        }
    }

    /// Release a pattern event (note off)
    pub fn release_pattern_event(&mut self, pattern_name: &str) {
        if let Some(signal) = self.pattern_signals.get_mut(pattern_name) {
            signal.gate = false;
            signal.trigger = false;
        }

        // Update bus value
        if let Some(bus_id) = self.pattern_to_bus.get(pattern_name) {
            if let Ok(mut graph) = self.signal_graph.write() {
                graph.buses.insert(bus_id.clone(), 0.0);
            }
        }
    }

    /// Extract audio features from the signal graph for pattern use
    pub fn update_audio_features(&mut self, executor: &SignalExecutor) {
        // Get RMS, pitch, and other features from analysis nodes
        if let Ok(graph) = self.signal_graph.read() {
            for bus_id in graph.buses.keys() {
                if bus_id.0.contains("_rms") || bus_id.0.contains("_pitch") {
                    let value = executor.get_bus_value(bus_id);
                    self.audio_features.insert(bus_id.0.clone(), value);
                }
            }
        }
    }

    /// Get an audio feature value for pattern modulation
    pub fn get_audio_feature(&self, name: &str) -> f32 {
        self.audio_features.get(name).copied().unwrap_or(0.0)
    }

    /// Check if a gate condition is met (for conditional pattern playback)
    pub fn check_gate_condition(&self, condition: &str) -> bool {
        // Parse simple conditions like "~bass_rms > 0.5"
        if let Some((feature, threshold)) = self.parse_condition(condition) {
            let value = self.get_audio_feature(&feature);
            value > threshold
        } else {
            true // Default to playing if condition can't be parsed
        }
    }

    /// Convert note name to frequency
    fn note_to_frequency(&self, note: &str) -> f32 {
        // Simple note to frequency conversion
        // C4 = 261.63 Hz, each semitone is 2^(1/12) ratio
        let note_map: HashMap<&str, f32> = [
            ("c3", 130.81),
            ("d3", 146.83),
            ("e3", 164.81),
            ("f3", 174.61),
            ("g3", 196.00),
            ("a3", 220.00),
            ("b3", 246.94),
            ("c4", 261.63),
            ("d4", 293.66),
            ("e4", 329.63),
            ("f4", 349.23),
            ("g4", 392.00),
            ("a4", 440.00),
            ("b4", 493.88),
            ("c5", 523.25),
        ]
        .iter()
        .cloned()
        .collect();

        note_map
            .get(note.to_lowercase().as_str())
            .copied()
            .unwrap_or(440.0)
    }

    /// Parse a simple condition string
    fn parse_condition(&self, condition: &str) -> Option<(String, f32)> {
        // Parse conditions like "~bass_rms > 0.5"
        let parts: Vec<&str> = condition.split('>').collect();
        if parts.len() == 2 {
            let feature = parts[0].trim().trim_start_matches('~');
            if let Ok(threshold) = parts[1].trim().parse::<f32>() {
                return Some((feature.to_string(), threshold));
            }
        }
        None
    }

    /// Apply pattern modulation to a synthesis parameter
    pub fn apply_pattern_modulation(
        &self,
        pattern_name: &str,
        base_value: f32,
        mod_amount: f32,
    ) -> f32 {
        if let Some(signal) = self.pattern_signals.get(pattern_name) {
            base_value + (signal.current_value * mod_amount)
        } else {
            base_value
        }
    }

    /// Get pattern trigger state (for triggering envelopes, etc.)
    pub fn get_pattern_trigger(&self, pattern_name: &str) -> bool {
        self.pattern_signals
            .get(pattern_name)
            .map(|s| s.trigger)
            .unwrap_or(false)
    }

    /// Get pattern gate state (for sustained notes)
    pub fn get_pattern_gate(&self, pattern_name: &str) -> bool {
        self.pattern_signals
            .get(pattern_name)
            .map(|s| s.gate)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_registration() {
        let graph = Arc::new(RwLock::new(SignalGraph::new(44100.0)));
        let mut bridge = PatternBridge::new(graph.clone(), 44100.0);

        let bus_id = bridge.register_pattern("kick".to_string());
        assert_eq!(bus_id.0, "pattern_kick");

        // Check that pattern signal was created
        assert!(bridge.pattern_signals.contains_key("kick"));
    }

    #[test]
    fn test_pattern_event_processing() {
        let graph = Arc::new(RwLock::new(SignalGraph::new(44100.0)));
        let mut bridge = PatternBridge::new(graph.clone(), 44100.0);

        bridge.register_pattern("bass".to_string());

        let event = PatternEvent {
            time: 0.0,
            value: PatternValue::Note("c3".to_string()),
            duration: 0.5,
            velocity: 0.8,
        };

        bridge.process_pattern_event("bass", event);

        // Check that pattern signal was updated
        let signal = &bridge.pattern_signals["bass"];
        assert!(signal.gate);
        assert!(signal.trigger);
        assert_eq!(signal.velocity, 0.8);
        assert!(signal.current_value > 0.0); // Should be C3 frequency
    }

    #[test]
    fn test_note_to_frequency() {
        let graph = Arc::new(RwLock::new(SignalGraph::new(44100.0)));
        let bridge = PatternBridge::new(graph, 44100.0);

        assert_eq!(bridge.note_to_frequency("a4"), 440.0);
        assert_eq!(bridge.note_to_frequency("c4"), 261.63);
        assert_eq!(bridge.note_to_frequency("A4"), 440.0); // Case insensitive
    }

    #[test]
    fn test_condition_parsing() {
        let graph = Arc::new(RwLock::new(SignalGraph::new(44100.0)));
        let bridge = PatternBridge::new(graph, 44100.0);

        let result = bridge.parse_condition("~bass_rms > 0.5");
        assert_eq!(result, Some(("bass_rms".to_string(), 0.5)));

        let result = bridge.parse_condition("invalid condition");
        assert_eq!(result, None);
    }

    #[test]
    fn test_pattern_modulation() {
        let graph = Arc::new(RwLock::new(SignalGraph::new(44100.0)));
        let mut bridge = PatternBridge::new(graph, 44100.0);

        bridge.register_pattern("lfo".to_string());

        // Set pattern signal value
        bridge.pattern_signals.get_mut("lfo").unwrap().current_value = 0.5;

        // Apply modulation
        let modulated = bridge.apply_pattern_modulation("lfo", 1000.0, 500.0);
        assert_eq!(modulated, 1250.0); // 1000 + (0.5 * 500)
    }

    #[test]
    fn test_audio_feature_extraction() {
        let graph = Arc::new(RwLock::new(SignalGraph::new(44100.0)));
        let mut bridge = PatternBridge::new(graph.clone(), 44100.0);

        // Simulate audio features
        bridge.audio_features.insert("bass_rms".to_string(), 0.7);
        bridge
            .audio_features
            .insert("kick_transient".to_string(), 1.0);

        assert_eq!(bridge.get_audio_feature("bass_rms"), 0.7);
        assert_eq!(bridge.get_audio_feature("kick_transient"), 1.0);
        assert_eq!(bridge.get_audio_feature("nonexistent"), 0.0);
    }

    #[test]
    fn test_gate_condition() {
        let graph = Arc::new(RwLock::new(SignalGraph::new(44100.0)));
        let mut bridge = PatternBridge::new(graph, 44100.0);

        // Set up audio feature
        bridge.audio_features.insert("bass_rms".to_string(), 0.7);

        // Test condition that should pass
        assert!(bridge.check_gate_condition("~bass_rms > 0.5"));

        // Test condition that should fail
        assert!(!bridge.check_gate_condition("~bass_rms > 0.8"));
    }
}
