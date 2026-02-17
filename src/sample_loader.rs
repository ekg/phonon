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

#![allow(clippy::collapsible_if)]
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
        Self {
            left: data,
            right: None,
        }
    }

    /// Create a stereo sample from left and right channels
    pub fn stereo(left: Vec<f32>, right: Vec<f32>) -> Self {
        Self {
            left,
            right: Some(right),
        }
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
    /// List of directories to search for samples, in priority order
    sample_dirs: Vec<PathBuf>,
}

impl Clone for SampleBank {
    fn clone(&self) -> Self {
        Self {
            samples: self.samples.clone(), // Arc makes this cheap - just increments ref count
            sample_dirs: self.sample_dirs.clone(),
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
        // Build list of sample directories to search, in priority order:
        // 1. ./samples/ (bundled repo samples - highest priority for testing)
        // 2. ~/phonon/samples/ (user's custom samples)
        // 3. ~/phonon/dirt-samples/ (SuperDirt compatibility)
        // 4. ./dirt-samples/ (fallback)
        // 5. ~/dirt-samples/ (another common location)
        let mut sample_dirs = Vec::new();

        // Bundled samples (highest priority for tests)
        let bundled = PathBuf::from("samples");
        if bundled.exists() {
            sample_dirs.push(bundled);
        }

        if let Some(home) = dirs::home_dir() {
            // User's phonon samples
            let user_samples = home.join("phonon").join("samples");
            if user_samples.exists() {
                sample_dirs.push(user_samples);
            }

            // SuperDirt compatibility
            let phonon_dirt = home.join("phonon").join("dirt-samples");
            if phonon_dirt.exists() {
                sample_dirs.push(phonon_dirt);
            }

            // Common dirt-samples location
            let home_dirt = home.join("dirt-samples");
            if home_dirt.exists() {
                sample_dirs.push(home_dirt);
            }
        }

        // Fallback to local dirt-samples
        let local_dirt = PathBuf::from("dirt-samples");
        if local_dirt.exists() {
            sample_dirs.push(local_dirt);
        }

        let mut bank = Self {
            samples: HashMap::new(),
            sample_dirs,
        };

        // Pre-load common samples
        let _ = bank.load_default_samples();
        bank
    }

    /// Load default drum samples from first available directory
    fn load_default_samples(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Sample names to pre-load (common drum sounds)
        let sample_names = ["bd", "sn", "hh", "cp", "oh", "lt", "mt", "ht", "blip"];

        for name in sample_names {
            // Try to load from any available directory
            for sample_dir in &self.sample_dirs {
                let sample_subdir = sample_dir.join(name);
                if sample_subdir.exists() && sample_subdir.is_dir() {
                    // Find first .wav file in the directory
                    if let Ok(entries) = std::fs::read_dir(&sample_subdir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                                if ext.eq_ignore_ascii_case("wav") {
                                    if self.load_sample(name, &path).is_ok() {
                                        break; // Loaded successfully, stop searching
                                    }
                                }
                            }
                        }
                    }
                    break; // Found the directory, even if loading failed
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

    /// Get a sample by name, searching all sample directories
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

        // Search across all sample directories
        for sample_dir_root in self.sample_dirs.clone() {
            let sample_dir = sample_dir_root.join(base_name);

            if !sample_dir.exists() || !sample_dir.is_dir() {
                continue;
            }

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

                if wav_files.is_empty() {
                    continue;
                }

                // Sort by filename for consistent ordering
                wav_files.sort_by_key(|entry| entry.file_name());

                // Determine which file to load
                let file_index = if let Some(index) = sample_index {
                    // Wrap index if larger than available files
                    index % wav_files.len()
                } else {
                    0 // Default to first file
                };

                if let Some(wav_file) = wav_files.get(file_index) {
                    if self.load_sample(name, &wav_file.path()).is_ok() {
                        return self.samples.get(name).cloned();
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    // =========================================================================
    // StereoSample: Construction
    // =========================================================================

    #[test]
    fn test_mono_sample_creation() {
        let data = vec![0.1, 0.2, 0.3, 0.4];
        let sample = StereoSample::mono(data.clone());
        assert_eq!(sample.left, data);
        assert!(sample.right.is_none());
        assert!(!sample.is_stereo());
    }

    #[test]
    fn test_stereo_sample_creation() {
        let left = vec![0.1, 0.2, 0.3];
        let right = vec![0.4, 0.5, 0.6];
        let sample = StereoSample::stereo(left.clone(), right.clone());
        assert_eq!(sample.left, left);
        assert_eq!(sample.right.as_ref().unwrap(), &right);
        assert!(sample.is_stereo());
    }

    #[test]
    fn test_from_vec_f32_creates_mono() {
        let data = vec![1.0, 2.0, 3.0];
        let sample: StereoSample = data.clone().into();
        assert_eq!(sample.left, data);
        assert!(!sample.is_stereo());
    }

    #[test]
    fn test_empty_sample() {
        let sample = StereoSample::mono(vec![]);
        assert!(sample.is_empty());
        assert_eq!(sample.len(), 0);
    }

    #[test]
    fn test_len_returns_frame_count() {
        let sample = StereoSample::mono(vec![0.0; 100]);
        assert_eq!(sample.len(), 100);

        let sample = StereoSample::stereo(vec![0.0; 50], vec![0.0; 50]);
        assert_eq!(sample.len(), 50);
    }

    // =========================================================================
    // StereoSample: Interpolation
    // =========================================================================

    #[test]
    fn test_interpolation_empty_sample_returns_zero() {
        let sample = StereoSample::mono(vec![]);
        let (l, r) = sample.get_interpolated(0.0);
        assert_eq!(l, 0.0);
        assert_eq!(r, 0.0);
    }

    #[test]
    fn test_interpolation_exact_index_mono() {
        let sample = StereoSample::mono(vec![1.0, 2.0, 3.0, 4.0]);
        let (l, r) = sample.get_interpolated(0.0);
        // At exact integer position with frac=0, we get: val * (1-0) + next * 0 = val
        assert!((l - 1.0).abs() < 1e-6);
        assert_eq!(l, r); // mono: left == right
    }

    #[test]
    fn test_interpolation_midpoint_mono() {
        let sample = StereoSample::mono(vec![0.0, 1.0]);
        let (l, _) = sample.get_interpolated(0.5);
        // Interpolation: 0.0 * 0.5 + 1.0 * 0.5 = 0.5
        assert!((l - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_interpolation_quarter_point() {
        let sample = StereoSample::mono(vec![0.0, 4.0]);
        let (l, _) = sample.get_interpolated(0.25);
        // 0.0 * 0.75 + 4.0 * 0.25 = 1.0
        assert!((l - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_interpolation_stereo_channels_independent() {
        let sample = StereoSample::stereo(vec![0.0, 1.0], vec![1.0, 0.0]);
        let (l, r) = sample.get_interpolated(0.5);
        // Left: 0.0*0.5 + 1.0*0.5 = 0.5
        assert!((l - 0.5).abs() < 1e-6);
        // Right: 1.0*0.5 + 0.0*0.5 = 0.5
        assert!((r - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_interpolation_at_last_sample() {
        let sample = StereoSample::mono(vec![1.0, 2.0, 3.0]);
        // At index 2, no next sample to interpolate with
        let (l, _) = sample.get_interpolated(2.0);
        // val * (1 - 0) = 3.0
        assert!((l - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_interpolation_beyond_bounds_returns_zero() {
        let sample = StereoSample::mono(vec![1.0, 2.0]);
        let (l, r) = sample.get_interpolated(5.0);
        assert_eq!(l, 0.0);
        assert_eq!(r, 0.0);
    }

    #[test]
    fn test_mono_interpolated_averages_channels() {
        let sample = StereoSample::stereo(vec![0.0, 1.0], vec![1.0, 0.0]);
        let mono = sample.get_mono_interpolated(0.5);
        // (0.5 + 0.5) * 0.5 = 0.5
        assert!((mono - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_mono_interpolated_for_mono_sample() {
        let sample = StereoSample::mono(vec![0.0, 1.0]);
        let mono = sample.get_mono_interpolated(0.5);
        // mono: both channels are same, (0.5 + 0.5) * 0.5 = 0.5
        assert!((mono - 0.5).abs() < 1e-6);
    }

    // =========================================================================
    // StereoSample: Slice
    // =========================================================================

    #[test]
    fn test_slice_mono() {
        let sample = StereoSample::mono(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        let sliced = sample.slice(1, 4);
        assert_eq!(sliced.left, vec![2.0, 3.0, 4.0]);
        assert!(!sliced.is_stereo());
    }

    #[test]
    fn test_slice_stereo_preserves_both_channels() {
        let sample = StereoSample::stereo(
            vec![1.0, 2.0, 3.0, 4.0],
            vec![5.0, 6.0, 7.0, 8.0],
        );
        let sliced = sample.slice(1, 3);
        assert_eq!(sliced.left, vec![2.0, 3.0]);
        assert_eq!(sliced.right.as_ref().unwrap(), &vec![6.0, 7.0]);
    }

    #[test]
    fn test_slice_clamps_beyond_bounds() {
        let sample = StereoSample::mono(vec![1.0, 2.0, 3.0]);
        let sliced = sample.slice(0, 100);
        assert_eq!(sliced.left, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_slice_begin_beyond_end_returns_empty() {
        let sample = StereoSample::mono(vec![1.0, 2.0, 3.0]);
        let sliced = sample.slice(5, 10);
        assert!(sliced.is_empty());
    }

    #[test]
    fn test_slice_begin_equals_end_returns_empty() {
        let sample = StereoSample::mono(vec![1.0, 2.0, 3.0]);
        let sliced = sample.slice(1, 1);
        assert!(sliced.is_empty());
    }

    #[test]
    fn test_slice_full_range() {
        let data = vec![1.0, 2.0, 3.0];
        let sample = StereoSample::mono(data.clone());
        let sliced = sample.slice(0, 3);
        assert_eq!(sliced.left, data);
    }

    // =========================================================================
    // StereoSample: Index trait
    // =========================================================================

    #[test]
    fn test_index_returns_left_channel() {
        let sample = StereoSample::stereo(vec![10.0, 20.0], vec![30.0, 40.0]);
        assert_eq!(sample[0], 10.0);
        assert_eq!(sample[1], 20.0);
    }

    #[test]
    #[should_panic]
    fn test_index_out_of_bounds_panics() {
        let sample = StereoSample::mono(vec![1.0]);
        let _ = sample[5];
    }

    // =========================================================================
    // StereoSample: Iterator / as_slice
    // =========================================================================

    #[test]
    fn test_iter_yields_left_channel() {
        let sample = StereoSample::stereo(vec![1.0, 2.0], vec![3.0, 4.0]);
        let collected: Vec<f32> = sample.iter().copied().collect();
        assert_eq!(collected, vec![1.0, 2.0]);
    }

    #[test]
    fn test_as_slice_returns_left_data() {
        let data = vec![5.0, 6.0, 7.0];
        let sample = StereoSample::mono(data.clone());
        assert_eq!(sample.as_slice(), &data[..]);
    }

    // =========================================================================
    // StereoSample: Clone
    // =========================================================================

    #[test]
    fn test_clone_produces_independent_copy() {
        let original = StereoSample::stereo(vec![1.0, 2.0], vec![3.0, 4.0]);
        let cloned = original.clone();
        assert_eq!(cloned.left, original.left);
        assert_eq!(cloned.right, original.right);
    }

    // =========================================================================
    // Helper: Create temporary WAV files for SampleBank tests
    // =========================================================================

    fn create_test_wav(path: &Path, samples: &[f32], channels: u16) {
        let spec = hound::WavSpec {
            channels,
            sample_rate: 44100,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        for &s in samples {
            writer.write_sample(s).unwrap();
        }
        writer.finalize().unwrap();
    }

    fn create_test_wav_i16(path: &Path, samples: &[i16], channels: u16) {
        let spec = hound::WavSpec {
            channels,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        for &s in samples {
            writer.write_sample(s as i32).unwrap();
        }
        writer.finalize().unwrap();
    }

    // =========================================================================
    // SampleBank: load_sample
    // =========================================================================

    #[test]
    fn test_load_sample_mono_float32() {
        let dir = tempfile::tempdir().unwrap();
        let wav_path = dir.path().join("test.wav");
        let audio = vec![0.5, -0.5, 0.25, -0.25];
        create_test_wav(&wav_path, &audio, 1);

        let mut bank = SampleBank {
            samples: HashMap::new(),
            sample_dirs: vec![],
        };
        bank.load_sample("test_mono", &wav_path).unwrap();

        let sample = bank.samples.get("test_mono").unwrap();
        assert!(!sample.is_stereo());
        assert_eq!(sample.len(), 4);
        for (i, &expected) in audio.iter().enumerate() {
            assert!((sample.left[i] - expected).abs() < 1e-5,
                "sample[{}] = {}, expected {}", i, sample.left[i], expected);
        }
    }

    #[test]
    fn test_load_sample_stereo_float32() {
        let dir = tempfile::tempdir().unwrap();
        let wav_path = dir.path().join("stereo.wav");
        // Interleaved stereo: L R L R
        let interleaved = vec![0.1, 0.9, 0.2, 0.8, 0.3, 0.7];
        create_test_wav(&wav_path, &interleaved, 2);

        let mut bank = SampleBank {
            samples: HashMap::new(),
            sample_dirs: vec![],
        };
        bank.load_sample("test_stereo", &wav_path).unwrap();

        let sample = bank.samples.get("test_stereo").unwrap();
        assert!(sample.is_stereo());
        assert_eq!(sample.len(), 3); // 3 frames
        assert!((sample.left[0] - 0.1).abs() < 1e-5);
        assert!((sample.left[1] - 0.2).abs() < 1e-5);
        assert!((sample.left[2] - 0.3).abs() < 1e-5);
        let right = sample.right.as_ref().unwrap();
        assert!((right[0] - 0.9).abs() < 1e-5);
        assert!((right[1] - 0.8).abs() < 1e-5);
        assert!((right[2] - 0.7).abs() < 1e-5);
    }

    #[test]
    fn test_load_sample_int16_format() {
        let dir = tempfile::tempdir().unwrap();
        let wav_path = dir.path().join("int16.wav");
        // i16 max is 32767
        let samples_i16: Vec<i16> = vec![16384, -16384, 0];
        create_test_wav_i16(&wav_path, &samples_i16, 1);

        let mut bank = SampleBank {
            samples: HashMap::new(),
            sample_dirs: vec![],
        };
        bank.load_sample("test_i16", &wav_path).unwrap();

        let sample = bank.samples.get("test_i16").unwrap();
        assert_eq!(sample.len(), 3);
        // 16384 / 32768 = 0.5
        assert!((sample.left[0] - 0.5).abs() < 0.01);
        assert!((sample.left[1] + 0.5).abs() < 0.01);
        assert!((sample.left[2]).abs() < 0.01);
    }

    #[test]
    fn test_load_sample_skips_if_already_cached() {
        let dir = tempfile::tempdir().unwrap();
        let wav1 = dir.path().join("first.wav");
        let wav2 = dir.path().join("second.wav");
        create_test_wav(&wav1, &[1.0, 1.0], 1);
        create_test_wav(&wav2, &[0.0, 0.0], 1);

        let mut bank = SampleBank {
            samples: HashMap::new(),
            sample_dirs: vec![],
        };

        // Load first file
        bank.load_sample("cached", &wav1).unwrap();
        let first_val = bank.samples.get("cached").unwrap().left[0];

        // Try to load again with different file - should be no-op
        bank.load_sample("cached", &wav2).unwrap();
        let second_val = bank.samples.get("cached").unwrap().left[0];

        assert_eq!(first_val, second_val, "Second load should not replace cached sample");
        assert!((first_val - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_load_sample_invalid_path_returns_error() {
        let mut bank = SampleBank {
            samples: HashMap::new(),
            sample_dirs: vec![],
        };
        let result = bank.load_sample("nonexistent", Path::new("/no/such/file.wav"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_sample_invalid_wav_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let bad_wav = dir.path().join("bad.wav");
        // Write garbage bytes, not a valid WAV
        let mut f = std::fs::File::create(&bad_wav).unwrap();
        f.write_all(b"this is not a wav file").unwrap();

        let mut bank = SampleBank {
            samples: HashMap::new(),
            sample_dirs: vec![],
        };
        let result = bank.load_sample("bad", &bad_wav);
        assert!(result.is_err());
    }

    // =========================================================================
    // SampleBank: get_sample name:index parsing
    // =========================================================================

    #[test]
    fn test_get_sample_parses_name_index() {
        let dir = tempfile::tempdir().unwrap();
        let sample_dir = dir.path().join("bd");
        std::fs::create_dir(&sample_dir).unwrap();

        // Create 3 WAV files with distinct content
        for (i, val) in [0.1f32, 0.2, 0.3].iter().enumerate() {
            let path = sample_dir.join(format!("bd{}.wav", i));
            create_test_wav(&path, &[*val; 10], 1);
        }

        let mut bank = SampleBank {
            samples: HashMap::new(),
            sample_dirs: vec![dir.path().to_path_buf()],
        };

        let s0 = bank.get_sample("bd:0").expect("bd:0 should load");
        let s1 = bank.get_sample("bd:1").expect("bd:1 should load");
        let s2 = bank.get_sample("bd:2").expect("bd:2 should load");

        // Each should have distinct content
        assert!((s0.left[0] - 0.1).abs() < 1e-5);
        assert!((s1.left[0] - 0.2).abs() < 1e-5);
        assert!((s2.left[0] - 0.3).abs() < 1e-5);
    }

    #[test]
    fn test_get_sample_index_wraps_around() {
        let dir = tempfile::tempdir().unwrap();
        let sample_dir = dir.path().join("hh");
        std::fs::create_dir(&sample_dir).unwrap();

        // Create 2 files
        create_test_wav(&sample_dir.join("hh0.wav"), &[0.1; 10], 1);
        create_test_wav(&sample_dir.join("hh1.wav"), &[0.2; 10], 1);

        let mut bank = SampleBank {
            samples: HashMap::new(),
            sample_dirs: vec![dir.path().to_path_buf()],
        };

        // Index 2 should wrap to 0 (2 % 2 = 0)
        let s_wrapped = bank.get_sample("hh:2").expect("hh:2 should wrap");
        let s0 = bank.get_sample("hh:0").expect("hh:0 should load");
        assert!((s_wrapped.left[0] - s0.left[0]).abs() < 1e-5);
    }

    #[test]
    fn test_get_sample_no_index_loads_first() {
        let dir = tempfile::tempdir().unwrap();
        let sample_dir = dir.path().join("cp");
        std::fs::create_dir(&sample_dir).unwrap();

        create_test_wav(&sample_dir.join("a_first.wav"), &[0.42; 10], 1);
        create_test_wav(&sample_dir.join("b_second.wav"), &[0.99; 10], 1);

        let mut bank = SampleBank {
            samples: HashMap::new(),
            sample_dirs: vec![dir.path().to_path_buf()],
        };

        let sample = bank.get_sample("cp").expect("cp should load");
        // Files are sorted by name, "a_first.wav" comes first
        assert!((sample.left[0] - 0.42).abs() < 1e-5);
    }

    #[test]
    fn test_get_sample_invalid_index_defaults_to_zero() {
        let dir = tempfile::tempdir().unwrap();
        let sample_dir = dir.path().join("sn");
        std::fs::create_dir(&sample_dir).unwrap();
        create_test_wav(&sample_dir.join("sn0.wav"), &[0.77; 10], 1);

        let mut bank = SampleBank {
            samples: HashMap::new(),
            sample_dirs: vec![dir.path().to_path_buf()],
        };

        // "bd:abc" should parse index as 0 (unwrap_or(0))
        let sample = bank.get_sample("sn:abc").expect("sn:abc should fallback");
        assert!((sample.left[0] - 0.77).abs() < 1e-5);
    }

    // =========================================================================
    // SampleBank: Cache behavior
    // =========================================================================

    #[test]
    fn test_get_sample_caches_result() {
        let dir = tempfile::tempdir().unwrap();
        let sample_dir = dir.path().join("bd");
        std::fs::create_dir(&sample_dir).unwrap();
        create_test_wav(&sample_dir.join("bd0.wav"), &[0.5; 10], 1);

        let mut bank = SampleBank {
            samples: HashMap::new(),
            sample_dirs: vec![dir.path().to_path_buf()],
        };

        let first = bank.get_sample("bd:0").expect("should load");
        let second = bank.get_sample("bd:0").expect("should be cached");

        // Same Arc pointer means it came from cache
        assert!(Arc::ptr_eq(&first, &second), "Second call should return cached Arc");
    }

    #[test]
    fn test_different_indices_cached_separately() {
        let dir = tempfile::tempdir().unwrap();
        let sample_dir = dir.path().join("bd");
        std::fs::create_dir(&sample_dir).unwrap();
        create_test_wav(&sample_dir.join("bd0.wav"), &[0.1; 10], 1);
        create_test_wav(&sample_dir.join("bd1.wav"), &[0.9; 10], 1);

        let mut bank = SampleBank {
            samples: HashMap::new(),
            sample_dirs: vec![dir.path().to_path_buf()],
        };

        let s0 = bank.get_sample("bd:0").expect("bd:0");
        let s1 = bank.get_sample("bd:1").expect("bd:1");

        assert!(!Arc::ptr_eq(&s0, &s1), "Different indices should be different Arcs");
        // Verify they're cached under their full names
        assert!(bank.samples.contains_key("bd:0"));
        assert!(bank.samples.contains_key("bd:1"));
    }

    // =========================================================================
    // SampleBank: Missing / nonexistent samples
    // =========================================================================

    #[test]
    fn test_get_sample_nonexistent_returns_none() {
        let mut bank = SampleBank {
            samples: HashMap::new(),
            sample_dirs: vec![],
        };
        assert!(bank.get_sample("nonexistent_sample").is_none());
    }

    #[test]
    fn test_get_sample_empty_dir_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let sample_dir = dir.path().join("empty");
        std::fs::create_dir(&sample_dir).unwrap();
        // No WAV files in this directory

        let mut bank = SampleBank {
            samples: HashMap::new(),
            sample_dirs: vec![dir.path().to_path_buf()],
        };
        assert!(bank.get_sample("empty").is_none());
    }

    #[test]
    fn test_get_sample_ignores_non_wav_files() {
        let dir = tempfile::tempdir().unwrap();
        let sample_dir = dir.path().join("txt");
        std::fs::create_dir(&sample_dir).unwrap();
        std::fs::write(sample_dir.join("notes.txt"), "not audio").unwrap();
        std::fs::write(sample_dir.join("data.mp3"), "fake mp3").unwrap();

        let mut bank = SampleBank {
            samples: HashMap::new(),
            sample_dirs: vec![dir.path().to_path_buf()],
        };
        assert!(bank.get_sample("txt").is_none());
    }

    // =========================================================================
    // SampleBank: Directory priority
    // =========================================================================

    #[test]
    fn test_sample_dirs_searched_in_order() {
        let dir1 = tempfile::tempdir().unwrap();
        let dir2 = tempfile::tempdir().unwrap();

        // Both dirs have a "kick" sample subdir
        let kick1 = dir1.path().join("kick");
        let kick2 = dir2.path().join("kick");
        std::fs::create_dir(&kick1).unwrap();
        std::fs::create_dir(&kick2).unwrap();

        create_test_wav(&kick1.join("kick.wav"), &[0.1; 10], 1);
        create_test_wav(&kick2.join("kick.wav"), &[0.9; 10], 1);

        let mut bank = SampleBank {
            samples: HashMap::new(),
            sample_dirs: vec![dir1.path().to_path_buf(), dir2.path().to_path_buf()],
        };

        let sample = bank.get_sample("kick").expect("should find kick");
        // First directory should win
        assert!((sample.left[0] - 0.1).abs() < 1e-5,
            "Should load from first dir, got {}", sample.left[0]);
    }

    // =========================================================================
    // SampleBank: wav file sorting (alphabetical)
    // =========================================================================

    #[test]
    fn test_wav_files_sorted_alphabetically() {
        let dir = tempfile::tempdir().unwrap();
        let sample_dir = dir.path().join("perc");
        std::fs::create_dir(&sample_dir).unwrap();

        // Create files in non-alphabetical order
        create_test_wav(&sample_dir.join("c_third.wav"), &[0.3; 10], 1);
        create_test_wav(&sample_dir.join("a_first.wav"), &[0.1; 10], 1);
        create_test_wav(&sample_dir.join("b_second.wav"), &[0.2; 10], 1);

        let mut bank = SampleBank {
            samples: HashMap::new(),
            sample_dirs: vec![dir.path().to_path_buf()],
        };

        let s0 = bank.get_sample("perc:0").expect("perc:0");
        let s1 = bank.get_sample("perc:1").expect("perc:1");
        let s2 = bank.get_sample("perc:2").expect("perc:2");

        assert!((s0.left[0] - 0.1).abs() < 1e-5, "Index 0 should be a_first");
        assert!((s1.left[0] - 0.2).abs() < 1e-5, "Index 1 should be b_second");
        assert!((s2.left[0] - 0.3).abs() < 1e-5, "Index 2 should be c_third");
    }

    // =========================================================================
    // SampleBank: WAV case insensitive extension
    // =========================================================================

    #[test]
    fn test_wav_extension_case_insensitive() {
        let dir = tempfile::tempdir().unwrap();
        let sample_dir = dir.path().join("mix");
        std::fs::create_dir(&sample_dir).unwrap();

        create_test_wav(&sample_dir.join("lower.wav"), &[0.1; 10], 1);
        // Create a file with .WAV extension
        create_test_wav(&sample_dir.join("upper.WAV"), &[0.2; 10], 1);

        let mut bank = SampleBank {
            samples: HashMap::new(),
            sample_dirs: vec![dir.path().to_path_buf()],
        };

        // Should find 2 files (both .wav and .WAV)
        let s0 = bank.get_sample("mix:0").expect("should find first");
        let s1 = bank.get_sample("mix:1").expect("should find second");
        assert!(s0.len() > 0);
        assert!(s1.len() > 0);
    }

    // =========================================================================
    // SampleBank: Clone uses Arc (cheap)
    // =========================================================================

    #[test]
    fn test_bank_clone_shares_arc_samples() {
        let dir = tempfile::tempdir().unwrap();
        let wav_path = dir.path().join("test.wav");
        create_test_wav(&wav_path, &[0.5; 10], 1);

        let mut bank = SampleBank {
            samples: HashMap::new(),
            sample_dirs: vec![],
        };
        bank.load_sample("shared", &wav_path).unwrap();

        let cloned = bank.clone();
        let original_arc = bank.samples.get("shared").unwrap();
        let cloned_arc = cloned.samples.get("shared").unwrap();

        assert!(Arc::ptr_eq(original_arc, cloned_arc),
            "Cloned bank should share Arc pointers, not deep-copy data");
    }

    // =========================================================================
    // SampleBank: Default trait
    // =========================================================================

    #[test]
    fn test_default_is_same_as_new() {
        // Just verify Default doesn't panic - can't deeply compare since it loads
        // from filesystem, but it should construct without error
        let _bank = SampleBank::default();
    }

    // =========================================================================
    // sample_player function
    // =========================================================================

    #[test]
    fn test_sample_player_empty_returns_zero() {
        use fundsp::audiounit::AudioUnit;
        let empty = Arc::new(vec![]);
        let mut player = sample_player(empty);
        // Process one sample - should produce 0.0
        let output = player.get_mono();
        assert!((output).abs() < 1e-6, "Empty sample player should produce silence");
    }

    #[test]
    fn test_sample_player_nonempty_returns_audio_unit() {
        let data = Arc::new(vec![0.5; 100]);
        let mut player = sample_player(data);
        // Just verify it doesn't panic and produces something
        let _output = player.get_mono();
    }
}
