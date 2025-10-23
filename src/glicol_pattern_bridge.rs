#![allow(unused_assignments, unused_mut)]
#![allow(dead_code)]
//! Bridge between Glicol DSP and TidalCycles patterns
//!
//! This module enables seamless integration between pattern sequences
//! and DSP synthesis, allowing patterns to control synthesis parameters
//! and synthesis to modulate patterns.

use crate::glicol_dsp::{DspChain, DspEnvironment};
use crate::mini_notation::parse_mini_notation;
use crate::pattern::{Fraction, Pattern, State, TimeSpan};
use crate::signal_graph::SignalGraph;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Pattern-controlled DSP parameter
#[derive(Clone, Debug)]
pub struct PatternParam {
    pub pattern: Pattern<f64>,
    pub target: String,    // Parameter name to control
    pub range: (f64, f64), // Min, max values
}

/// DSP voice that can be triggered by patterns
#[derive(Clone)]
pub struct DspVoice {
    pub chain: DspChain,
    pub params: HashMap<String, PatternParam>,
    pub envelope: Option<Pattern<f64>>, // Optional amplitude envelope
}

/// Hybrid pattern-DSP node
#[derive(Clone)]
pub enum HybridNode {
    /// Pure pattern (e.g., drum samples)
    Pattern(Pattern<String>),

    /// DSP synthesis triggered by pattern
    PatternDsp {
        trigger_pattern: Pattern<String>,
        voice: DspVoice,
    },

    /// Pattern modulating DSP parameter
    Modulation {
        pattern: Pattern<f64>,
        target_chain: String,
        target_param: String,
    },

    /// DSP processing pattern output
    ProcessedPattern {
        source_pattern: Pattern<String>,
        dsp_chain: DspChain,
    },

    /// Crossfade between patterns based on DSP
    Crossfade {
        pattern_a: Pattern<String>,
        pattern_b: Pattern<String>,
        control: DspChain, // Outputs 0-1 for crossfade
    },
}

/// Pattern-DSP integration engine
pub struct PatternDspEngine {
    /// Named pattern-DSP nodes
    nodes: HashMap<String, HybridNode>,

    /// Global DSP environment
    dsp_env: DspEnvironment,

    /// Current tempo
    tempo_bpm: f32,

    /// Global controls accessible to patterns
    controls: Arc<Mutex<HashMap<String, f64>>>,
}

impl PatternDspEngine {
    /// Create a new pattern-DSP engine
    pub fn new(tempo_bpm: f32) -> Self {
        Self {
            nodes: HashMap::new(),
            dsp_env: DspEnvironment::new(),
            tempo_bpm,
            controls: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Parse combined pattern-DSP expression
    pub fn parse_hybrid(&mut self, input: &str) -> Result<(), String> {
        // Parse different forms:
        // 1. Pattern only: "bd*4 cp hh*8"
        // 2. DSP only: "sine(440) >> lpf(1000, 0.8)"
        // 3. Pattern with DSP: "bd*4 >> reverb(0.3)"
        // 4. Pattern controlling DSP: "{0 0.5 1}%8 >> ~cutoff"
        // 5. DSP in pattern: "bd [sine(440):0.1] cp"

        // Check for >> operator (DSP chain or routing)
        if input.contains(">>") {
            let parts: Vec<&str> = input.split(">>").collect();
            if parts.len() != 2 {
                return Err("Invalid >> syntax".to_string());
            }

            let left = parts[0].trim();
            let right = parts[1].trim();

            // Check if left is a pattern
            if Self::is_pattern(left) {
                // Pattern >> DSP or Pattern >> Control
                if right.starts_with('~') {
                    // Routing to control
                    self.parse_pattern_to_control(left, right)?;
                } else {
                    // Pattern through DSP
                    self.parse_pattern_through_dsp(left, right)?;
                }
            } else {
                // DSP chain
                self.parse_dsp_chain(input)?;
            }
        } else if input.starts_with('~') {
            // Reference chain
            self.parse_reference_chain(input)?;
        } else if Self::is_pattern(input) {
            // Pure pattern
            self.parse_pure_pattern(input)?;
        } else {
            // Try as DSP
            self.parse_dsp_chain(input)?;
        }

        Ok(())
    }

    /// Check if input looks like a pattern
    fn is_pattern(input: &str) -> bool {
        // DSP function names that are definitely not patterns
        let dsp_functions = ["sine", "sin", "saw", "square", "lpf", "hpf", "mul", "add"];
        for func in &dsp_functions {
            if input.trim().starts_with(func) {
                return false;
            }
        }

        // Heuristics: contains pattern-like tokens
        input.contains('[') ||
        input.contains('<') ||
        input.contains('*') ||
        input.contains("bd") || 
        input.contains("cp") ||
        input.contains("hh") ||
        input.contains("sn") ||
        // Euclidean patterns like bd(3,8)
        (input.contains('(') && input.contains(',') && input.chars().filter(|c| c.is_alphabetic()).count() < 5)
    }

    /// Parse pure pattern
    fn parse_pure_pattern(&mut self, input: &str) -> Result<(), String> {
        let pattern = parse_mini_notation(input);
        let name = format!("pattern_{}", self.nodes.len());
        self.nodes.insert(name, HybridNode::Pattern(pattern));
        Ok(())
    }

    /// Parse DSP chain
    fn parse_dsp_chain(&mut self, input: &str) -> Result<(), String> {
        // This would use the glicol_parser
        // For now, create a simple chain
        let chain = DspChain::new();
        self.dsp_env.set_output(chain);
        Ok(())
    }

    /// Parse reference chain (starts with ~)
    fn parse_reference_chain(&mut self, input: &str) -> Result<(), String> {
        let parts: Vec<&str> = input.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err("Invalid reference syntax".to_string());
        }

        let name = parts[0].trim();
        let chain_str = parts[1].trim();

        // Parse the chain
        let chain = DspChain::new(); // Would parse chain_str
        self.dsp_env.add_ref(name.to_string(), chain);
        Ok(())
    }

    /// Parse pattern routed to control
    fn parse_pattern_to_control(&mut self, pattern_str: &str, control: &str) -> Result<(), String> {
        let pattern = parse_mini_notation(pattern_str);

        // Convert string pattern to f64 pattern
        let value_pattern = pattern.fmap(|s| s.parse::<f64>().unwrap_or(0.0));

        let node = HybridNode::Modulation {
            pattern: value_pattern,
            target_chain: "output".to_string(),
            target_param: control.trim_start_matches('~').to_string(),
        };

        let name = format!("mod_{}", self.nodes.len());
        self.nodes.insert(name, node);
        Ok(())
    }

    /// Parse pattern through DSP processing
    fn parse_pattern_through_dsp(
        &mut self,
        pattern_str: &str,
        dsp_str: &str,
    ) -> Result<(), String> {
        let pattern = parse_mini_notation(pattern_str);
        let chain = DspChain::new(); // Would parse dsp_str

        let node = HybridNode::ProcessedPattern {
            source_pattern: pattern,
            dsp_chain: chain,
        };

        let name = format!("processed_{}", self.nodes.len());
        self.nodes.insert(name, node);
        Ok(())
    }

    /// Create pattern-triggered synthesis voice
    pub fn create_voice(
        &mut self,
        name: &str,
        trigger_pattern: &str,
        synth_def: &str,
    ) -> Result<(), String> {
        let pattern = parse_mini_notation(trigger_pattern);
        let chain = DspChain::new(); // Would parse synth_def

        let voice = DspVoice {
            chain,
            params: HashMap::new(),
            envelope: None,
        };

        let node = HybridNode::PatternDsp {
            trigger_pattern: pattern,
            voice,
        };

        self.nodes.insert(name.to_string(), node);
        Ok(())
    }

    /// Add pattern modulation to a voice parameter
    pub fn add_modulation(
        &mut self,
        voice_name: &str,
        param_name: &str,
        pattern: &str,
        range: (f64, f64),
    ) -> Result<(), String> {
        // Parse pattern as numeric
        let parts: Vec<&str> = pattern.split_whitespace().collect();
        let values: Vec<f64> = parts.iter().filter_map(|s| s.parse().ok()).collect();

        if values.is_empty() {
            return Err("No numeric values in pattern".to_string());
        }

        // Create pattern from values
        let patterns: Vec<Pattern<f64>> = values.into_iter().map(Pattern::pure).collect();
        let value_pattern = Pattern::cat(patterns);

        // Add to voice
        if let Some(HybridNode::PatternDsp { voice, .. }) = self.nodes.get_mut(voice_name) {
            voice.params.insert(
                param_name.to_string(),
                PatternParam {
                    pattern: value_pattern,
                    target: param_name.to_string(),
                    range,
                },
            );
            Ok(())
        } else {
            Err(format!("Voice '{voice_name}' not found"))
        }
    }

    /// Query hybrid nodes at current time
    pub fn query(&self, beat: f64) -> Vec<(String, Vec<String>)> {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(beat),
                Fraction::from_float(beat + 0.125),
            ),
            controls: self.controls.lock().unwrap().clone(),
        };

        let mut results = Vec::new();

        for (name, node) in &self.nodes {
            match node {
                HybridNode::Pattern(pattern) => {
                    let events = pattern.query(&state);
                    let values: Vec<String> = events.into_iter().map(|e| e.value).collect();
                    if !values.is_empty() {
                        results.push((name.clone(), values));
                    }
                }
                HybridNode::PatternDsp {
                    trigger_pattern, ..
                } => {
                    let events = trigger_pattern.query(&state);
                    let values: Vec<String> = events
                        .into_iter()
                        .map(|e| format!("synth:{}", e.value))
                        .collect();
                    if !values.is_empty() {
                        results.push((name.clone(), values));
                    }
                }
                HybridNode::ProcessedPattern { source_pattern, .. } => {
                    let events = source_pattern.query(&state);
                    let values: Vec<String> = events
                        .into_iter()
                        .map(|e| format!("fx:{}", e.value))
                        .collect();
                    if !values.is_empty() {
                        results.push((name.clone(), values));
                    }
                }
                _ => {}
            }
        }

        results
    }

    /// Build complete signal graph
    pub fn build_graph(&self, sample_rate: f32) -> Result<SignalGraph, String> {
        self.dsp_env.build_complete_graph(sample_rate)
    }
}

/// Parse Glicol-enhanced pattern notation
pub fn parse_enhanced(input: &str) -> Result<Pattern<String>, String> {
    // Enhanced notation examples:
    // "bd*4 [sine(440):0.1] cp"  - Embed synth in pattern
    // "{c4 e4 g4}'min"            - Apply scale
    // "bd >> reverb(0.3)"         - Pattern through effect
    // "[1 0.5 0.25] >> ~cutoff"   - Pattern to control

    // For now, parse as regular pattern
    Ok(parse_mini_notation(input))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_detection() {
        assert!(PatternDspEngine::is_pattern("bd*4 cp hh"));
        assert!(PatternDspEngine::is_pattern("[bd cp] hh"));
        assert!(PatternDspEngine::is_pattern("<c4 e4 g4>"));
        assert!(PatternDspEngine::is_pattern("bd(3,8)"));
        assert!(!PatternDspEngine::is_pattern("sine(440)"));
        assert!(!PatternDspEngine::is_pattern("lpf(1000,0.8)"));
    }

    #[test]
    fn test_hybrid_parsing() {
        let mut engine = PatternDspEngine::new(120.0);

        // Pure pattern
        assert!(engine.parse_hybrid("bd*4 cp hh").is_ok());

        // Pattern through DSP
        assert!(engine.parse_hybrid("bd*4 >> reverb(0.3)").is_ok());

        // Pattern to control
        assert!(engine.parse_hybrid("0 0.5 1 >> ~cutoff").is_ok());
    }

    #[test]
    fn test_voice_creation() {
        let mut engine = PatternDspEngine::new(120.0);

        // Create a voice
        assert!(engine
            .create_voice("bass", "c2 e2 g2", "saw(55) >> lpf(1000, 0.8)")
            .is_ok());

        // Add modulation
        assert!(engine
            .add_modulation("bass", "cutoff", "0.2 0.5 0.8 1.0", (200.0, 2000.0))
            .is_ok());
    }

    #[test]
    fn test_query() {
        let mut engine = PatternDspEngine::new(120.0);

        engine.parse_hybrid("bd cp hh").unwrap();

        let results = engine.query(0.0);
        assert!(!results.is_empty());
    }
}
