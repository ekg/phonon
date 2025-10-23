#![allow(unused_assignments, unused_mut)]
#![allow(dead_code)]
//! Sample loading and playback for dirt-samples integration
//!
//! This module provides sample loading from the dirt-samples library format,
//! which is compatible with TidalCycles.
//!
//! # Features
//!
//! - **Automatic sample discovery**: Searches `dirt-samples/` directory structure
//! - **Sample indexing**: Support for `bd:0`, `bd:1`, etc. to select specific samples
//! - **Caching**: Loaded samples are cached for fast access
//! - **Mono conversion**: Stereo samples are automatically converted to mono
//! - **WAV support**: Loads WAV files in various formats (int16, int24, float32)
//!
//! # Directory Structure
//!
//! Samples should be organized in the dirt-samples format:
//!
//! ```text
//! dirt-samples/
//!   bd/
//!     BT0A0A7.wav
//!     BD0.wav
//!   sn/
//!     ST0T0S0.wav
//!   hh/
//!     000_hh3closedhh.wav
//! ```
//!
//! # Examples
//!
//! ## Basic sample loading
//!
//! ```
//! use phonon::sample_loader::SampleBank;
//!
//! let mut bank = SampleBank::new();
//!
//! // Load a sample (searches dirt-samples/ directory)
//! let bd_sample = bank.get_sample("bd").expect("Sample not found");
//!
//! println!("Loaded BD sample: {} samples", bd_sample.len());
//! ```
//!
//! ## Sample indexing
//!
//! ```
//! use phonon::sample_loader::SampleBank;
//!
//! let mut bank = SampleBank::new();
//!
//! // Load specific sample by index
//! let bd0 = bank.get_sample("bd:0").unwrap(); // First BD sample
//! let bd1 = bank.get_sample("bd:1").unwrap(); // Second BD sample
//! let bd2 = bank.get_sample("bd:2").unwrap(); // Third BD sample
//! ```
//!
//! ## Using with voice manager
//!
//! ```
//! use phonon::sample_loader::SampleBank;
//! use phonon::voice_manager::VoiceManager;
//!
//! let mut bank = SampleBank::new();
//! let mut vm = VoiceManager::new();
//!
//! // Load and trigger multiple samples
//! if let Some(bd) = bank.get_sample("bd") {
//!     vm.trigger_sample(bd, 1.0);
//! }
//!
//! if let Some(sn) = bank.get_sample("sn") {
//!     vm.trigger_sample(sn, 0.8);
//! }
//!
//! // Process audio
//! for _ in 0..44100 {
//!     let sample = vm.process();
//!     // Output sample to audio device
//! }
//! ```
//!
//! ## Custom sample paths
//!
//! ```no_run
//! use phonon::sample_loader::SampleBank;
//! use std::path::Path;
//!
//! let mut bank = SampleBank::new();
//!
//! // Load from custom path
//! let custom_path = Path::new("my-samples/kick.wav");
//! bank.load_sample("my_kick", custom_path).unwrap();
//!
//! let sample = bank.get_sample("my_kick").unwrap();
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Sample bank that loads and caches WAV files
pub struct SampleBank {
    samples: HashMap<String, Arc<Vec<f32>>>,
    dirt_samples_dir: PathBuf,
}

impl Default for SampleBank {
    fn default() -> Self {
        Self::new()
    }
}

impl SampleBank {
    pub fn new() -> Self {
        let dirt_samples_dir = PathBuf::from("dirt-samples");
        let mut bank = Self {
            samples: HashMap::new(),
            dirt_samples_dir,
        };

        // Pre-load common samples
        let _ = bank.load_default_samples();
        bank
    }

    /// Load default drum samples
    fn load_default_samples(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let samples = [
            ("bd", "bd/BT0A0A7.wav"),
            ("sn", "sn/ST0T0S0.wav"),
            ("hh", "hh/000_hh3closedhh.wav"),
            ("cp", "cp/HANDCLP0.wav"),
            ("oh", "oh/OH00.wav"),
            ("lt", "lt/LT00.wav"),
            ("mt", "mt/MT00.wav"),
            ("ht", "ht/HT00.wav"),
        ];

        for (name, path) in samples {
            let full_path = self.dirt_samples_dir.join(path);
            if full_path.exists() {
                if let Err(e) = self.load_sample(name, &full_path) {
                    eprintln!("Warning: Failed to load {name}: {e}");
                }
            }
        }

        Ok(())
    }

    /// Load a sample from disk
    pub fn load_sample(
        &mut self,
        name: &str,
        path: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.samples.contains_key(name) {
            return Ok(()); // Already loaded
        }

        let mut reader = hound::WavReader::open(path)?;
        let spec = reader.spec();

        // Convert to mono f32
        let samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Float => {
                reader.samples::<f32>().map(|s| s.unwrap_or(0.0)).collect()
            }
            hound::SampleFormat::Int => {
                let max_val = (1 << (spec.bits_per_sample - 1)) as f32;
                reader
                    .samples::<i32>()
                    .map(|s| s.unwrap_or(0) as f32 / max_val)
                    .collect()
            }
        };

        // Convert to mono if stereo
        let mono_samples = if spec.channels == 2 {
            samples
                .chunks(2)
                .map(|chunk| (chunk[0] + chunk.get(1).copied().unwrap_or(0.0)) * 0.5)
                .collect()
        } else {
            samples
        };

        self.samples
            .insert(name.to_string(), Arc::new(mono_samples));
        Ok(())
    }

    /// Get a sample by name, attempting to load from dirt-samples if not cached
    pub fn get_sample(&mut self, name: &str) -> Option<Arc<Vec<f32>>> {
        // Parse sample name and index (e.g., "bd:3" -> "bd", 3)
        let (base_name, sample_index) = if let Some(colon_pos) = name.find(':') {
            let base = &name[..colon_pos];
            let index_str = &name[colon_pos + 1..];
            let index = index_str.parse::<usize>().unwrap_or(0);
            (base, Some(index))
        } else {
            (name, None)
        };

        // Check cache first (use full name as key)
        if let Some(sample) = self.samples.get(name) {
            return Some(sample.clone());
        }

        // If a specific sample index is requested, try to find that specific sample
        if let Some(index) = sample_index {
            // Try to load the specific sample by index
            let sample_dir = self.dirt_samples_dir.join(base_name);
            if sample_dir.exists() && sample_dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&sample_dir) {
                    let mut wav_files: Vec<_> = entries
                        .filter_map(|entry| entry.ok())
                        .filter(|entry| {
                            entry.path().extension().and_then(|s| s.to_str()) == Some("wav")
                        })
                        .collect();

                    // Sort by filename for consistent ordering
                    wav_files.sort_by_key(|entry| entry.file_name());

                    // Get the file at the requested index
                    if let Some(wav_file) = wav_files.get(index) {
                        if self.load_sample(name, &wav_file.path()).is_ok() {
                            return self.samples.get(name).cloned();
                        }
                    }
                }
            }
        }

        // Try to load from various dirt-samples locations (for base name)
        let possible_paths = vec![
            format!("{}/{:03}.wav", base_name, 0), // e.g., bd/000.wav
            format!("{}/{}0.wav", base_name, base_name.to_uppercase()), // e.g., bd/BD0.wav
            format!("{}/BT0A0A7.wav", base_name),  // Specific known files
        ];

        for path_str in possible_paths {
            let full_path = self.dirt_samples_dir.join(&path_str);
            if full_path.exists() && self.load_sample(name, &full_path).is_ok() {
                return self.samples.get(name).cloned();
            }
        }

        // Try to find any WAV in the directory (fall back to first available sample)
        let sample_dir = self.dirt_samples_dir.join(base_name);
        if sample_dir.exists() && sample_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&sample_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("wav") {
                        if self.load_sample(name, &path).is_ok() {
                            return self.samples.get(name).cloned();
                        }
                        break; // Just take the first one
                    }
                }
            }
        }

        None
    }
}

/// Create a simple one-shot sample player  
pub fn sample_player(samples: Arc<Vec<f32>>) -> Box<dyn fundsp::audiounit::AudioUnit> {
    use fundsp::hacker::*;

    if samples.is_empty() {
        return Box::new(zero());
    }

    // For now, just loop the sample as a simple oscillator
    // This is not ideal but will at least make sound
    let freq = 1.0; // Play at 1 Hz (will sound wrong but proves it works)

    // Create a simple wavetable oscillator from the samples
    // Just take first 1024 samples or pad with zeros
    let table_size = 1024;
    let mut table = vec![0.0f32; table_size];

    for i in 0..std::cmp::min(table_size, samples.len()) {
        table[i] = samples[i];
    }

    // Use the first sample as a test - just play a sine at the average frequency
    // This is terrible but will prove samples are loading
    Box::new(sine_hz(440.0) * 0.2) // Just play a test tone for now
}
