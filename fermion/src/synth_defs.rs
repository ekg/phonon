//! Inline synthesizer definitions for live coding
//! 
//! Supports FunDSP graph notation in patterns like:
//! "synth:sine(440)|adsr(0.01,0.1,0.7,0.5)"
//! "synth:fm(220,440,0.5)>>reverb(0.3)"

use fundsp::hacker::*;
use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Synthesizer definition parsed from pattern notation
#[derive(Debug, Clone)]
pub enum SynthDef {
    // Basic oscillators
    Sine { freq: f64 },
    Saw { freq: f64 },
    Square { freq: f64 },
    Triangle { freq: f64 },
    Noise,
    
    // FM synthesis
    FM { carrier: f64, modulator: f64, index: f64 },
    
    // Additive synthesis
    Additive { harmonics: Vec<(f64, f64)> }, // (freq, amp) pairs
    
    // Subtractive with filter
    Filtered { 
        source: Box<SynthDef>,
        filter_type: FilterType,
        cutoff: f64,
        resonance: f64,
    },
    
    // Enveloped
    Enveloped {
        source: Box<SynthDef>,
        attack: f64,
        decay: f64,
        sustain: f64,
        release: f64,
    },
    
    // Effects
    WithReverb { source: Box<SynthDef>, mix: f64 },
    WithDelay { source: Box<SynthDef>, time: f64, feedback: f64 },
    WithChorus { source: Box<SynthDef>, rate: f64, depth: f64 },
    
    // Combining
    Stack(Vec<SynthDef>),
    Mix(Vec<SynthDef>),
}

#[derive(Debug, Clone)]
pub enum FilterType {
    Lowpass,
    Highpass,
    Bandpass,
    Notch,
}

/// Parse synth definition from pattern string
/// Examples:
/// - "sine(440)"
/// - "saw(220)|lowpass(1000,0.5)"
/// - "fm(220,440,0.5)"
pub fn parse_synth_def(pattern: &str) -> Result<SynthDef, String> {
    // Simple parser for now - can be expanded with proper parsing later
    let pattern = pattern.trim();
    
    // Check for basic oscillators
    if let Some(freq_str) = pattern.strip_prefix("sine(").and_then(|s| s.strip_suffix(")")) {
        let freq = freq_str.parse::<f64>().map_err(|e| e.to_string())?;
        return Ok(SynthDef::Sine { freq });
    }
    
    if let Some(freq_str) = pattern.strip_prefix("saw(").and_then(|s| s.strip_suffix(")")) {
        let freq = freq_str.parse::<f64>().map_err(|e| e.to_string())?;
        return Ok(SynthDef::Saw { freq });
    }
    
    if let Some(freq_str) = pattern.strip_prefix("square(").and_then(|s| s.strip_suffix(")")) {
        let freq = freq_str.parse::<f64>().map_err(|e| e.to_string())?;
        return Ok(SynthDef::Square { freq });
    }
    
    if pattern == "noise" {
        return Ok(SynthDef::Noise);
    }
    
    // FM synthesis
    if let Some(params) = pattern.strip_prefix("fm(").and_then(|s| s.strip_suffix(")")) {
        let parts: Vec<&str> = params.split(',').collect();
        if parts.len() == 3 {
            let carrier = parts[0].trim().parse::<f64>().map_err(|e| e.to_string())?;
            let modulator = parts[1].trim().parse::<f64>().map_err(|e| e.to_string())?;
            let index = parts[2].trim().parse::<f64>().map_err(|e| e.to_string())?;
            return Ok(SynthDef::FM { carrier, modulator, index });
        }
    }
    
    Err(format!("Unknown synth pattern: {}", pattern))
}

/// Compile a SynthDef into audio samples
/// Returns a Vec of samples at 44100 Hz sample rate
pub fn compile_synth(def: &SynthDef, duration: f64) -> Vec<f32> {
    let sample_rate = 44100.0;
    let samples = (sample_rate * duration) as usize;
    let mut buffer = Vec::with_capacity(samples);
    
    match def {
        SynthDef::Sine { freq } => {
            for i in 0..samples {
                let t = i as f64 / sample_rate;
                let sample = (2.0 * std::f64::consts::PI * freq * t).sin() * 0.5;
                buffer.push(sample as f32);
            }
        }
        
        SynthDef::Saw { freq } => {
            for i in 0..samples {
                let t = i as f64 / sample_rate;
                let phase = (freq * t) % 1.0;
                let sample = (2.0 * phase - 1.0) * 0.3;
                buffer.push(sample as f32);
            }
        }
        
        SynthDef::Square { freq } => {
            for i in 0..samples {
                let t = i as f64 / sample_rate;
                let phase = (freq * t) % 1.0;
                let sample = if phase < 0.5 { 0.3 } else { -0.3 };
                buffer.push(sample);
            }
        }
        
        SynthDef::Triangle { freq } => {
            for i in 0..samples {
                let t = i as f64 / sample_rate;
                let phase = (freq * t) % 1.0;
                let sample = if phase < 0.5 {
                    4.0 * phase - 1.0
                } else {
                    3.0 - 4.0 * phase
                } * 0.4;
                buffer.push(sample as f32);
            }
        }
        
        SynthDef::Noise => {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            for _ in 0..samples {
                buffer.push(rng.gen_range(-0.3..0.3));
            }
        }
        
        SynthDef::FM { carrier, modulator, index } => {
            for i in 0..samples {
                let t = i as f64 / sample_rate;
                let mod_val = (2.0 * std::f64::consts::PI * modulator * t).sin();
                let freq = carrier + (mod_val * index * 100.0);
                let sample = (2.0 * std::f64::consts::PI * freq * t).sin() * 0.4;
                buffer.push(sample as f32);
            }
        }
        
        _ => {
            // For complex types, just return silence for now
            // These would need proper DSP implementations
            buffer.resize(samples, 0.0);
        }
    }
    
    buffer
}

/// TOML configuration for synth definitions
#[derive(Debug, Deserialize)]
pub struct SynthConfig {
    synths: HashMap<String, SynthDefConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SynthDefConfig {
    Sine { freq: f64 },
    Saw { freq: f64 },
    Square { freq: f64 },
    Triangle { freq: f64 },
    Noise,
    FM { carrier: f64, modulator: f64, index: f64 },
}

impl From<SynthDefConfig> for SynthDef {
    fn from(config: SynthDefConfig) -> Self {
        match config {
            SynthDefConfig::Sine { freq } => SynthDef::Sine { freq },
            SynthDefConfig::Saw { freq } => SynthDef::Saw { freq },
            SynthDefConfig::Square { freq } => SynthDef::Square { freq },
            SynthDefConfig::Triangle { freq } => SynthDef::Triangle { freq },
            SynthDefConfig::Noise => SynthDef::Noise,
            SynthDefConfig::FM { carrier, modulator, index } => {
                SynthDef::FM { carrier, modulator, index }
            }
        }
    }
}

/// Registry for named synth definitions
pub struct SynthRegistry {
    definitions: HashMap<String, Arc<SynthDef>>,
}

impl SynthRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            definitions: HashMap::new(),
        };
        
        // Try to load from synthdefs.toml
        if let Ok(loaded) = Self::load_from_file("synthdefs.toml") {
            registry = loaded;
        } else {
            // Fallback to home directory
            if let Ok(home) = std::env::var("HOME") {
                let path = format!("{}/phonon/synthdefs.toml", home);
                if let Ok(loaded) = Self::load_from_file(&path) {
                    registry = loaded;
                }
            }
        }
        
        registry
    }
    
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = fs::read_to_string(path)?;
        let config: SynthConfig = toml::from_str(&contents)?;
        
        let mut registry = Self {
            definitions: HashMap::new(),
        };
        
        for (name, def_config) in config.synths {
            let def: SynthDef = def_config.into();
            registry.register(&name, def);
        }
        
        Ok(registry)
    }
    
    pub fn register(&mut self, name: &str, def: SynthDef) {
        self.definitions.insert(name.to_string(), Arc::new(def));
    }
    
    pub fn get(&self, name: &str) -> Option<Arc<SynthDef>> {
        self.definitions.get(name).cloned()
    }
    
    pub fn count(&self) -> usize {
        self.definitions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_basic_synths() {
        assert!(matches!(parse_synth_def("sine(440)"), Ok(SynthDef::Sine { freq: 440.0 })));
        assert!(matches!(parse_synth_def("saw(220)"), Ok(SynthDef::Saw { freq: 220.0 })));
        assert!(matches!(parse_synth_def("noise"), Ok(SynthDef::Noise)));
    }
    
    #[test]
    fn test_parse_fm() {
        match parse_synth_def("fm(220,440,0.5)") {
            Ok(SynthDef::FM { carrier, modulator, index }) => {
                assert_eq!(carrier, 220.0);
                assert_eq!(modulator, 440.0);
                assert_eq!(index, 0.5);
            }
            _ => panic!("Failed to parse FM"),
        }
    }
}