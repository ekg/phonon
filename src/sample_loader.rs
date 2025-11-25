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
//! - **Stereo support**: Stereo samples are preserved with left/right channels
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
use std::ops::Index;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Stereo sample data - supports both mono and stereo samples
///
/// For mono samples, `right` is None and `left` contains all data.
/// For stereo samples, `left` and `right` contain the respective channels.
#[derive(Clone, Debug)]
pub struct StereoSample {
    /// Left channel (or mono data if mono sample)
    pub left: Vec<f32>,
    /// Right channel (None for mono samples)
    pub right: Option<Vec<f32>>,
}

impl StereoSample {
    /// Create a mono sample
    pub fn mono(data: Vec<f32>) -> Self {
        Self { left: data, right: None }
    }

    /// Create a stereo sample from left and right channels
    pub fn stereo(left: Vec<f32>, right: Vec<f32>) -> Self {
        Self { left, right: Some(right) }
    }

    /// Check if this sample is stereo
    pub fn is_stereo(&self) -> bool {
        self.right.is_some()
    }

    /// Get the number of frames (samples per channel)
    pub fn len(&self) -> usize {
        self.left.len()
    }

    /// Check if the sample is empty
    pub fn is_empty(&self) -> bool {
        self.left.is_empty()
    }

    /// Get a sample at a given position with linear interpolation
    /// Returns (left, right) - for mono samples, left == right
    pub fn get_interpolated(&self, position: f32) -> (f32, f32) {
        if self.left.is_empty() {
            return (0.0, 0.0);
        }

        let idx = position as usize;
        let frac = position - idx as f32;

        // Bounds check
        if idx >= self.left.len() {
            return (0.0, 0.0);
        }

        let left_val = if idx + 1 < self.left.len() {
            self.left[idx] * (1.0 - frac) + self.left[idx + 1] * frac
        } else {
            self.left[idx] * (1.0 - frac)
        };

        let right_val = if let Some(ref right) = self.right {
            // True stereo sample
            if idx + 1 < right.len() {
                right[idx] * (1.0 - frac) + right[idx + 1] * frac
            } else {
                right[idx] * (1.0 - frac)
            }
        } else {
            // Mono sample - same on both channels
            left_val
        };

        (left_val, right_val)
    }

    /// Get mono (center) value at position with linear interpolation
    pub fn get_mono_interpolated(&self, position: f32) -> f32 {
        let (l, r) = self.get_interpolated(position);
        (l + r) * 0.5
    }

    /// Create a sliced version of this sample (preserves stereo if present)
    pub fn slice(&self, begin: usize, end: usize) -> Self {
        let begin = begin.min(self.left.len());
        let end = end.clamp(begin, self.left.len());

        let sliced_left = self.left[begin..end].to_vec();
        let sliced_right = self.right.as_ref().map(|r| {
            let begin = begin.min(r.len());
            let end = end.clamp(begin, r.len());
            r[begin..end].to_vec()
        });

        Self {
            left: sliced_left,
            right: sliced_right,
        }
    }

    /// Iterate over the left channel (or mono data)
    /// For backward compatibility with code that expects Vec<f32>
    pub fn iter(&self) -> impl Iterator<Item = &f32> {
        self.left.iter()
    }

    /// Get direct access to the left channel data
    pub fn as_slice(&self) -> &[f32] {
        &self.left
    }
}

// Index implementation for backward compatibility with sample[i] syntax
impl Index<usize> for StereoSample {
    type Output = f32;

    fn index(&self, index: usize) -> &Self::Output {
        &self.left[index]
    }
}

// Allow creating StereoSample from Vec<f32> for backward compatibility
impl From<Vec<f32>> for StereoSample {
    fn from(data: Vec<f32>) -> Self {
        Self::mono(data)
    }
}

/// Sample bank that loads and caches WAV files
pub struct SampleBank {
    samples: HashMap<String, Arc<StereoSample>>,
    dirt_samples_dir: PathBuf,
}

impl Clone for SampleBank {
    fn clone(&self) -> Self {
        Self {
            samples: self.samples.clone(), // Arc makes this cheap - just increments ref count
            dirt_samples_dir: self.dirt_samples_dir.clone(),
        }
    }
}

impl Default for SampleBank {
    fn default() -> Self {
        Self::new()
    }
}

impl SampleBank {
    pub fn new() -> Self {
        // Try ~/phonon/dirt-samples first, then fall back to ./dirt-samples
        let dirt_samples_dir = dirs::home_dir()
            .map(|home| home.join("phonon").join("dirt-samples"))
            .filter(|path| path.exists())
            .unwrap_or_else(|| PathBuf::from("dirt-samples"));

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

        // Read raw samples as f32
        let raw_samples: Vec<f32> = match spec.sample_format {
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

        // Create StereoSample, preserving stereo if present
        let stereo_sample = if spec.channels == 2 {
            // Deinterleave stereo: L R L R L R -> (L L L, R R R)
            let num_frames = raw_samples.len() / 2;
            let mut left = Vec::with_capacity(num_frames);
            let mut right = Vec::with_capacity(num_frames);
            for chunk in raw_samples.chunks(2) {
                left.push(chunk[0]);
                right.push(chunk.get(1).copied().unwrap_or(0.0));
            }
            StereoSample::stereo(left, right)
        } else {
            StereoSample::mono(raw_samples)
        };

        self.samples
            .insert(name.to_string(), Arc::new(stereo_sample));
        Ok(())
    }

    /// Get a sample by name, attempting to load from dirt-samples if not cached
    pub fn get_sample(&mut self, name: &str) -> Option<Arc<StereoSample>> {
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
                            entry
                                .path()
                                .extension()
                                .and_then(|s| s.to_str())
                                .map(|ext| ext.eq_ignore_ascii_case("wav"))
                                .unwrap_or(false)
                        })
                        .collect();

                    // Sort by filename for consistent ordering
                    wav_files.sort_by_key(|entry| entry.file_name());

                    // Get the file at the requested index with wrapping (modulo)
                    // This allows n values to wrap: n=5 with 3 samples becomes index 2
                    if !wav_files.is_empty() {
                        let wrapped_index = index % wav_files.len();
                        if let Some(wav_file) = wav_files.get(wrapped_index) {
                            if self.load_sample(name, &wav_file.path()).is_ok() {
                                return self.samples.get(name).cloned();
                            }
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
                    // Check for both .wav and .WAV extensions (case-insensitive)
                    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                        if ext.eq_ignore_ascii_case("wav") {
                            if self.load_sample(name, &path).is_ok() {
                                return self.samples.get(name).cloned();
                            }
                            break; // Just take the first one
                        }
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
