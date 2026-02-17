//! Beat Grid Extraction and Alignment
//!
//! This module provides functionality to extract beat grids from audio and align
//! patterns/samples to a reference tempo grid.
//!
//! A **beat grid** is a temporal map of beat positions in audio:
//! - **Beat positions**: Exact times where beats occur in the source audio
//! - **Downbeats**: First beat of each bar (measure)
//! - **Tempo map**: BPM values that may vary throughout the track
//! - **Phase offset**: Where beat 1 begins relative to audio start
//!
//! ## Key Concepts
//!
//! - **Beat tracking**: Analyzing audio to find where beats occur
//! - **Beat alignment**: Mapping beats to a quantized grid
//! - **Warp markers**: Points that define tempo changes for elastic audio
//! - **Phase alignment**: Syncing the downbeat to cycle boundaries
//!
//! ## Use Cases
//!
//! 1. **Sample sync**: Align loops/samples to your project's tempo
//! 2. **Beat matching**: DJ-style tempo/phase matching
//! 3. **Groove extraction**: Extract timing feel from reference audio
//! 4. **Time stretching**: Warp audio to fit a different tempo
//!
//! ## Usage
//!
//! ```phonon
//! -- Analyze a sample's beat grid
//! ~grid $ beat_grid "drums.wav"
//!
//! -- Get the detected BPM
//! ~bpm $ grid_bpm ~grid
//!
//! -- Align sample to project tempo
//! out $ s "drums.wav" $ align_to_tempo ~grid 140
//!
//! -- Sync phase to project
//! out $ s "loop.wav" $ phase_sync ~grid
//! ```

#![allow(clippy::manual_abs_diff)]
use rustfft::{num_complex::Complex, FftPlanner};
use std::collections::HashMap;
use std::f32::consts::PI;

/// A detected beat with timing information
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Beat {
    /// Time in seconds from audio start
    pub time: f64,
    /// Confidence of beat detection (0.0 to 1.0)
    pub confidence: f32,
    /// Beat strength/accent (for detecting downbeats)
    pub strength: f32,
    /// Is this a downbeat (first beat of bar)?
    pub is_downbeat: bool,
    /// Beat number within the bar (0-indexed, 0 = downbeat)
    pub beat_in_bar: u32,
}

impl Beat {
    pub fn new(time: f64, confidence: f32, strength: f32) -> Self {
        Self {
            time,
            confidence,
            strength,
            is_downbeat: false,
            beat_in_bar: 0,
        }
    }
}

/// Warp marker for elastic audio time-stretching
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WarpMarker {
    /// Original time in source audio (seconds)
    pub source_time: f64,
    /// Target time in warped audio (seconds)
    pub target_time: f64,
    /// Type of warp point
    pub marker_type: WarpMarkerType,
}

/// Type of warp marker
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarpMarkerType {
    /// Automatic beat-aligned marker
    Beat,
    /// User-defined anchor point
    Anchor,
    /// Start of audio
    Start,
    /// End of audio
    End,
}

/// Beat grid for a piece of audio
#[derive(Debug, Clone)]
pub struct BeatGrid {
    /// Name/identifier for this beat grid
    pub name: String,

    /// Sample rate of the analyzed audio
    pub sample_rate: f32,

    /// Duration of the audio in seconds
    pub duration: f64,

    /// Detected beats with timing and metadata
    pub beats: Vec<Beat>,

    /// Estimated tempo in BPM (may be average if tempo varies)
    pub bpm: f64,

    /// Tempo confidence (0.0 to 1.0)
    pub tempo_confidence: f32,

    /// Time signature numerator (beats per bar)
    pub time_signature_numerator: u32,

    /// Time signature denominator (beat value, e.g., 4 = quarter note)
    pub time_signature_denominator: u32,

    /// Phase offset: time of first downbeat (seconds)
    pub phase_offset: f64,

    /// Warp markers for time-stretching
    pub warp_markers: Vec<WarpMarker>,

    /// Optional tempo map for variable-tempo tracks
    /// Maps time (seconds) to BPM
    pub tempo_map: Vec<(f64, f64)>,

    /// Metadata (source file, analysis parameters, etc.)
    pub metadata: HashMap<String, String>,
}

impl BeatGrid {
    /// Create a new empty beat grid
    pub fn new(name: String, sample_rate: f32, duration: f64) -> Self {
        Self {
            name,
            sample_rate,
            duration,
            beats: Vec::new(),
            bpm: 120.0,
            tempo_confidence: 0.0,
            time_signature_numerator: 4,
            time_signature_denominator: 4,
            phase_offset: 0.0,
            warp_markers: Vec::new(),
            tempo_map: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Create a beat grid from known BPM (useful for pre-quantized samples)
    pub fn from_bpm(
        name: String,
        sample_rate: f32,
        duration: f64,
        bpm: f64,
        phase_offset: f64,
    ) -> Self {
        let mut grid = Self::new(name, sample_rate, duration);
        grid.bpm = bpm;
        grid.tempo_confidence = 1.0;
        grid.phase_offset = phase_offset;

        // Generate beat positions
        let beat_duration = 60.0 / bpm;
        let mut time = phase_offset;
        let mut beat_count = 0u32;

        while time < duration {
            let is_downbeat = beat_count.is_multiple_of(grid.time_signature_numerator);
            grid.beats.push(Beat {
                time,
                confidence: 1.0,
                strength: if is_downbeat { 1.0 } else { 0.5 },
                is_downbeat,
                beat_in_bar: beat_count % grid.time_signature_numerator,
            });

            time += beat_duration;
            beat_count += 1;
        }

        grid
    }

    /// Get the beat duration in seconds
    pub fn beat_duration(&self) -> f64 {
        60.0 / self.bpm
    }

    /// Get the bar/measure duration in seconds
    pub fn bar_duration(&self) -> f64 {
        self.beat_duration() * self.time_signature_numerator as f64
    }

    /// Get the number of complete bars in the audio
    pub fn bar_count(&self) -> u32 {
        let bar_dur = self.bar_duration();
        ((self.duration - self.phase_offset) / bar_dur).floor() as u32
    }

    /// Find the nearest beat to a given time
    pub fn nearest_beat(&self, time: f64) -> Option<&Beat> {
        if self.beats.is_empty() {
            return None;
        }

        self.beats.iter().min_by(|a, b| {
            let dist_a = (a.time - time).abs();
            let dist_b = (b.time - time).abs();
            dist_a.partial_cmp(&dist_b).unwrap()
        })
    }

    /// Find the beat just before a given time
    pub fn beat_before(&self, time: f64) -> Option<&Beat> {
        self.beats.iter().rev().find(|b| b.time <= time)
    }

    /// Find the beat just after a given time
    pub fn beat_after(&self, time: f64) -> Option<&Beat> {
        self.beats.iter().find(|b| b.time > time)
    }

    /// Get all downbeats
    pub fn downbeats(&self) -> Vec<&Beat> {
        self.beats.iter().filter(|b| b.is_downbeat).collect()
    }

    /// Get the beat index at a given time
    pub fn beat_at_time(&self, time: f64) -> Option<usize> {
        // Use phase offset and BPM to calculate beat index
        if time < self.phase_offset {
            return None;
        }

        let beat_duration = self.beat_duration();
        let beat_index = ((time - self.phase_offset) / beat_duration).floor() as usize;

        if beat_index < self.beats.len() {
            Some(beat_index)
        } else {
            None
        }
    }

    /// Get the bar index at a given time
    pub fn bar_at_time(&self, time: f64) -> Option<usize> {
        self.beat_at_time(time)
            .map(|beat_idx| beat_idx / self.time_signature_numerator as usize)
    }

    /// Get the position within the bar (0.0 to 1.0)
    pub fn position_in_bar(&self, time: f64) -> f64 {
        let bar_duration = self.bar_duration();
        let time_since_phase = time - self.phase_offset;
        if time_since_phase < 0.0 {
            return 0.0;
        }
        (time_since_phase % bar_duration) / bar_duration
    }

    /// Calculate time for a given beat index
    pub fn time_for_beat(&self, beat_index: usize) -> f64 {
        if beat_index < self.beats.len() {
            self.beats[beat_index].time
        } else {
            // Extrapolate beyond detected beats
            self.phase_offset + (beat_index as f64 * self.beat_duration())
        }
    }

    /// Calculate time for a given bar and beat position
    pub fn time_for_bar_beat(&self, bar: usize, beat_in_bar: u32) -> f64 {
        let beat_index = bar * self.time_signature_numerator as usize + beat_in_bar as usize;
        self.time_for_beat(beat_index)
    }

    /// Create warp markers to align this beat grid to a target tempo
    pub fn create_warp_markers(&mut self, target_bpm: f64) {
        self.warp_markers.clear();

        let source_beat_duration = self.beat_duration();
        let target_beat_duration = 60.0 / target_bpm;

        // Add start marker
        self.warp_markers.push(WarpMarker {
            source_time: 0.0,
            target_time: 0.0,
            marker_type: WarpMarkerType::Start,
        });

        // Add beat markers
        for (i, beat) in self.beats.iter().enumerate() {
            let target_time = self.phase_offset + (i as f64 * target_beat_duration);
            self.warp_markers.push(WarpMarker {
                source_time: beat.time,
                target_time,
                marker_type: WarpMarkerType::Beat,
            });
        }

        // Add end marker
        let source_end = self.duration;
        let target_end = if !self.beats.is_empty() {
            let last_beat_idx = self.beats.len() - 1;
            let target_last_beat =
                self.phase_offset + (last_beat_idx as f64 * target_beat_duration);
            target_last_beat
                + (source_end - self.beats.last().unwrap().time)
                    * (target_beat_duration / source_beat_duration)
        } else {
            source_end * (target_bpm / self.bpm)
        };

        self.warp_markers.push(WarpMarker {
            source_time: source_end,
            target_time: target_end,
            marker_type: WarpMarkerType::End,
        });
    }

    /// Map a source time to warped time using warp markers
    pub fn warp_time(&self, source_time: f64) -> f64 {
        if self.warp_markers.is_empty() {
            return source_time;
        }

        // Find surrounding warp markers
        let mut prev_marker = &self.warp_markers[0];
        let mut next_marker = &self.warp_markers[0];

        for marker in &self.warp_markers {
            if marker.source_time <= source_time {
                prev_marker = marker;
            } else {
                next_marker = marker;
                break;
            }
        }

        // Handle edge cases
        if source_time <= prev_marker.source_time {
            return prev_marker.target_time;
        }
        if prev_marker.source_time == next_marker.source_time {
            return prev_marker.target_time;
        }

        // Linear interpolation between markers
        let t = (source_time - prev_marker.source_time)
            / (next_marker.source_time - prev_marker.source_time);
        prev_marker.target_time + t * (next_marker.target_time - prev_marker.target_time)
    }

    /// Get tempo at a specific time (for variable-tempo tracks)
    pub fn tempo_at_time(&self, time: f64) -> f64 {
        if self.tempo_map.is_empty() {
            return self.bpm;
        }

        // Find the tempo at or before this time
        let mut current_bpm = self.bpm;
        for &(map_time, map_bpm) in &self.tempo_map {
            if map_time <= time {
                current_bpm = map_bpm;
            } else {
                break;
            }
        }
        current_bpm
    }

    /// Set time signature
    pub fn set_time_signature(&mut self, numerator: u32, denominator: u32) {
        self.time_signature_numerator = numerator;
        self.time_signature_denominator = denominator;

        // Update beat metadata
        for (i, beat) in self.beats.iter_mut().enumerate() {
            beat.beat_in_bar = (i as u32) % numerator;
            beat.is_downbeat = beat.beat_in_bar == 0;
        }
    }

    /// Adjust phase offset (shift where beat 1 falls)
    pub fn set_phase_offset(&mut self, new_offset: f64) {
        let delta = new_offset - self.phase_offset;
        self.phase_offset = new_offset;

        // Shift all beat times
        for beat in &mut self.beats {
            beat.time += delta;
        }

        // Shift warp markers
        for marker in &mut self.warp_markers {
            marker.source_time += delta;
            marker.target_time += delta;
        }
    }

    /// Scale tempo (and update beat positions)
    pub fn scale_tempo(&mut self, factor: f64) {
        let _original_bpm = self.bpm;
        self.bpm *= factor;

        // Scale beat positions relative to phase offset
        for beat in &mut self.beats {
            let relative_time = beat.time - self.phase_offset;
            beat.time = self.phase_offset + (relative_time / factor);
        }

        // Update tempo map
        for entry in &mut self.tempo_map {
            entry.1 *= factor;
        }
    }
}

/// Beat grid analyzer using onset detection and tempo estimation
pub struct BeatGridAnalyzer {
    sample_rate: f32,
    /// FFT size for spectral analysis
    fft_size: usize,
    /// Hop size (overlap)
    hop_size: usize,
    /// Minimum BPM to consider
    min_bpm: f64,
    /// Maximum BPM to consider
    max_bpm: f64,
}

impl BeatGridAnalyzer {
    /// Create a new beat grid analyzer
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            fft_size: 2048,
            hop_size: 512,
            min_bpm: 60.0,
            max_bpm: 200.0,
        }
    }

    /// Set BPM range for tempo estimation
    pub fn set_bpm_range(&mut self, min: f64, max: f64) {
        self.min_bpm = min;
        self.max_bpm = max;
    }

    /// Analyze audio and extract beat grid
    pub fn analyze(&self, samples: &[f32], name: String) -> BeatGrid {
        let duration = samples.len() as f64 / self.sample_rate as f64;
        let mut grid = BeatGrid::new(name, self.sample_rate, duration);

        // Step 1: Compute onset detection function
        let onset_env = self.compute_onset_envelope(samples);

        // Step 2: Estimate tempo from onset envelope
        let (bpm, tempo_confidence) = self.estimate_tempo(&onset_env);
        grid.bpm = bpm;
        grid.tempo_confidence = tempo_confidence;

        // Step 3: Detect beats using dynamic programming
        let beats = self.detect_beats(&onset_env, bpm);
        grid.beats = beats;

        // Step 4: Detect downbeats and phase
        self.detect_downbeats(&mut grid);

        grid
    }

    /// Compute onset detection envelope using spectral flux
    fn compute_onset_envelope(&self, samples: &[f32]) -> Vec<f32> {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(self.fft_size);

        // Create Hann window
        let window: Vec<f32> = (0..self.fft_size)
            .map(|i| 0.5 * (1.0 - (2.0 * PI * i as f32 / (self.fft_size - 1) as f32).cos()))
            .collect();

        let mut prev_magnitude = vec![0.0f32; self.fft_size / 2];
        let mut onset_env = Vec::new();

        let mut pos = 0;
        while pos + self.fft_size <= samples.len() {
            // Apply window and prepare FFT buffer
            let mut fft_buffer: Vec<Complex<f32>> = (0..self.fft_size)
                .map(|i| Complex::new(samples[pos + i] * window[i], 0.0))
                .collect();

            fft.process(&mut fft_buffer);

            // Calculate spectral flux (positive half-wave rectified difference)
            let mut flux = 0.0f32;
            for i in 0..self.fft_size / 2 {
                let magnitude = (fft_buffer[i].re.powi(2) + fft_buffer[i].im.powi(2)).sqrt();
                let diff = magnitude - prev_magnitude[i];
                if diff > 0.0 {
                    flux += diff;
                }
                prev_magnitude[i] = magnitude;
            }

            onset_env.push(flux);
            pos += self.hop_size;
        }

        // Normalize
        let max_flux = onset_env.iter().cloned().fold(0.0f32, f32::max);
        if max_flux > 0.0 {
            for v in &mut onset_env {
                *v /= max_flux;
            }
        }

        onset_env
    }

    /// Estimate tempo using autocorrelation of onset envelope
    fn estimate_tempo(&self, onset_env: &[f32]) -> (f64, f32) {
        if onset_env.len() < 100 {
            return (120.0, 0.0);
        }

        let frame_rate = self.sample_rate / self.hop_size as f32;

        // Convert BPM range to frame lags
        let min_lag = (frame_rate * 60.0 / self.max_bpm as f32) as usize;
        let max_lag = (frame_rate * 60.0 / self.min_bpm as f32) as usize;
        let max_lag = max_lag.min(onset_env.len() / 2);

        if min_lag >= max_lag {
            return (120.0, 0.0);
        }

        // Compute autocorrelation
        let mut best_lag = min_lag;
        let mut best_correlation = 0.0f32;

        for lag in min_lag..=max_lag {
            let mut correlation = 0.0f32;
            let mut count = 0;

            for i in 0..onset_env.len() - lag {
                correlation += onset_env[i] * onset_env[i + lag];
                count += 1;
            }

            if count > 0 {
                correlation /= count as f32;
            }

            if correlation > best_correlation {
                best_correlation = correlation;
                best_lag = lag;
            }
        }

        let bpm = (frame_rate * 60.0 / best_lag as f32) as f64;

        // Adjust to reasonable range
        let adjusted_bpm = if bpm < 80.0 {
            bpm * 2.0
        } else if bpm > 160.0 {
            bpm / 2.0
        } else {
            bpm
        };

        (adjusted_bpm, best_correlation)
    }

    /// Detect beats using dynamic programming (similar to librosa beat tracking)
    fn detect_beats(&self, onset_env: &[f32], bpm: f64) -> Vec<Beat> {
        if onset_env.is_empty() {
            return Vec::new();
        }

        let frame_rate = self.sample_rate as f64 / self.hop_size as f64;
        let beat_period_frames = (frame_rate * 60.0 / bpm) as usize;

        if beat_period_frames == 0 {
            return Vec::new();
        }

        // Use local maxima of onset envelope as beat candidates
        let mut candidates: Vec<(usize, f32)> = Vec::new();
        let window = beat_period_frames / 4;

        for i in window..onset_env.len() - window {
            let is_peak = onset_env[i] >= onset_env[i - 1]
                && onset_env[i] >= onset_env[i + 1]
                && onset_env[i] > 0.1;

            if is_peak {
                // Check if local maximum in larger window
                let local_max = onset_env[i - window..=i + window]
                    .iter()
                    .cloned()
                    .fold(0.0f32, f32::max);

                if (onset_env[i] - local_max).abs() < 0.01 {
                    candidates.push((i, onset_env[i]));
                }
            }
        }

        // Select beats using dynamic programming
        // Score function: onset strength + penalty for deviation from expected beat period
        let mut selected: Vec<usize> = Vec::new();

        if candidates.is_empty() {
            // Fall back to regular grid if no candidates found
            let mut frame = 0;
            while frame < onset_env.len() {
                selected.push(frame);
                frame += beat_period_frames;
            }
        } else {
            // Greedy selection with period constraint
            let tolerance = beat_period_frames / 4;
            let mut last_selected: Option<usize> = None;

            for &(frame, _strength) in &candidates {
                let should_select = match last_selected {
                    None => true,
                    Some(last) => {
                        let expected = last + beat_period_frames;
                        let diff = if frame > expected {
                            frame - expected
                        } else {
                            expected - frame
                        };
                        diff <= tolerance || frame >= last + beat_period_frames - tolerance
                    }
                };

                if should_select {
                    selected.push(frame);
                    last_selected = Some(frame);
                }
            }
        }

        // Convert frame indices to Beat structs
        selected
            .into_iter()
            .map(|frame| {
                let time = frame as f64 / frame_rate;
                let strength = if frame < onset_env.len() {
                    onset_env[frame]
                } else {
                    0.5
                };

                Beat::new(time, 0.8, strength)
            })
            .collect()
    }

    /// Detect downbeats and set phase offset
    fn detect_downbeats(&self, grid: &mut BeatGrid) {
        if grid.beats.is_empty() {
            return;
        }

        // Look for periodicity at bar level (every N beats, where N = time signature numerator)
        let beats_per_bar = grid.time_signature_numerator as usize;

        // Find the strongest beat in the first bar as downbeat candidate
        let first_bar_beats: Vec<&Beat> = grid.beats.iter().take(beats_per_bar).collect();

        let strongest_idx = first_bar_beats
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.strength.partial_cmp(&b.strength).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0);

        // Set phase offset to the strongest beat
        grid.phase_offset = if strongest_idx < grid.beats.len() {
            grid.beats[strongest_idx].time
        } else {
            grid.beats[0].time
        };

        // Mark downbeats
        for (i, beat) in grid.beats.iter_mut().enumerate() {
            beat.beat_in_bar = ((i + beats_per_bar - strongest_idx) % beats_per_bar) as u32;
            beat.is_downbeat = beat.beat_in_bar == 0;
        }
    }
}

/// Analyze audio and extract a beat grid
pub fn analyze_beat_grid(samples: &[f32], sample_rate: f32, name: String) -> BeatGrid {
    let analyzer = BeatGridAnalyzer::new(sample_rate);
    analyzer.analyze(samples, name)
}

/// Create a beat grid from known BPM
pub fn beat_grid_from_bpm(
    name: String,
    sample_rate: f32,
    duration: f64,
    bpm: f64,
    phase_offset: f64,
) -> BeatGrid {
    BeatGrid::from_bpm(name, sample_rate, duration, bpm, phase_offset)
}

/// Align two beat grids (compute phase difference)
pub fn compute_phase_difference(grid_a: &BeatGrid, grid_b: &BeatGrid) -> f64 {
    // Normalize to bar position
    let pos_a = grid_a.phase_offset % grid_a.bar_duration();
    let pos_b = grid_b.phase_offset % grid_b.bar_duration();

    // Calculate phase difference in terms of grid_a's bar duration
    let diff = pos_a - pos_b;

    // Normalize to [-0.5, 0.5) bar
    let bar_dur = grid_a.bar_duration();
    let normalized = diff / bar_dur;
    if normalized > 0.5 {
        normalized - 1.0
    } else if normalized < -0.5 {
        normalized + 1.0
    } else {
        normalized
    }
}

/// Calculate tempo ratio between two beat grids
pub fn compute_tempo_ratio(source: &BeatGrid, target: &BeatGrid) -> f64 {
    target.bpm / source.bpm
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beat_grid_creation() {
        let grid = BeatGrid::new("test".to_string(), 44100.0, 10.0);

        assert_eq!(grid.name, "test");
        assert_eq!(grid.sample_rate, 44100.0);
        assert_eq!(grid.duration, 10.0);
        assert!(grid.beats.is_empty());
    }

    #[test]
    fn test_beat_grid_from_bpm() {
        let grid = BeatGrid::from_bpm("test".to_string(), 44100.0, 4.0, 120.0, 0.0);

        // 120 BPM = 2 beats per second
        // 4 seconds = 8 beats
        assert_eq!(grid.beats.len(), 8);
        assert_eq!(grid.bpm, 120.0);

        // Check beat positions (0.5 seconds apart)
        assert!((grid.beats[0].time - 0.0).abs() < 0.001);
        assert!((grid.beats[1].time - 0.5).abs() < 0.001);
        assert!((grid.beats[2].time - 1.0).abs() < 0.001);

        // Check downbeats (every 4 beats for 4/4)
        assert!(grid.beats[0].is_downbeat);
        assert!(!grid.beats[1].is_downbeat);
        assert!(!grid.beats[2].is_downbeat);
        assert!(!grid.beats[3].is_downbeat);
        assert!(grid.beats[4].is_downbeat);
    }

    #[test]
    fn test_beat_duration() {
        let grid = BeatGrid::from_bpm("test".to_string(), 44100.0, 4.0, 120.0, 0.0);

        // 120 BPM = 0.5 seconds per beat
        assert!((grid.beat_duration() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_bar_duration() {
        let grid = BeatGrid::from_bpm("test".to_string(), 44100.0, 4.0, 120.0, 0.0);

        // 120 BPM, 4/4 time = 2 seconds per bar
        assert!((grid.bar_duration() - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_nearest_beat() {
        let grid = BeatGrid::from_bpm("test".to_string(), 44100.0, 4.0, 120.0, 0.0);

        // Query at 0.3 seconds should return beat at 0.5
        let nearest = grid.nearest_beat(0.3).unwrap();
        assert!((nearest.time - 0.5).abs() < 0.001);

        // Query at 0.2 seconds should return beat at 0.0
        let nearest = grid.nearest_beat(0.2).unwrap();
        assert!((nearest.time - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_beat_before_after() {
        let grid = BeatGrid::from_bpm("test".to_string(), 44100.0, 4.0, 120.0, 0.0);

        // Beat before 0.7 should be at 0.5
        let before = grid.beat_before(0.7).unwrap();
        assert!((before.time - 0.5).abs() < 0.001);

        // Beat after 0.7 should be at 1.0
        let after = grid.beat_after(0.7).unwrap();
        assert!((after.time - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_downbeats() {
        let grid = BeatGrid::from_bpm("test".to_string(), 44100.0, 4.0, 120.0, 0.0);

        let downbeats = grid.downbeats();
        assert_eq!(downbeats.len(), 2); // 2 bars in 4 seconds at 120 BPM
        assert!((downbeats[0].time - 0.0).abs() < 0.001);
        assert!((downbeats[1].time - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_beat_at_time() {
        let grid = BeatGrid::from_bpm("test".to_string(), 44100.0, 4.0, 120.0, 0.0);

        assert_eq!(grid.beat_at_time(0.0), Some(0));
        assert_eq!(grid.beat_at_time(0.5), Some(1));
        assert_eq!(grid.beat_at_time(0.7), Some(1)); // Between beats, returns floor
        assert_eq!(grid.beat_at_time(1.0), Some(2));
    }

    #[test]
    fn test_bar_at_time() {
        let grid = BeatGrid::from_bpm("test".to_string(), 44100.0, 4.0, 120.0, 0.0);

        assert_eq!(grid.bar_at_time(0.0), Some(0));
        assert_eq!(grid.bar_at_time(1.5), Some(0)); // Still in first bar
        assert_eq!(grid.bar_at_time(2.0), Some(1)); // Second bar
        assert_eq!(grid.bar_at_time(3.0), Some(1)); // Still in second bar
    }

    #[test]
    fn test_position_in_bar() {
        let grid = BeatGrid::from_bpm("test".to_string(), 44100.0, 4.0, 120.0, 0.0);

        assert!((grid.position_in_bar(0.0) - 0.0).abs() < 0.001);
        assert!((grid.position_in_bar(0.5) - 0.25).abs() < 0.001); // 1/4 through bar
        assert!((grid.position_in_bar(1.0) - 0.5).abs() < 0.001); // Half through bar
        assert!((grid.position_in_bar(2.0) - 0.0).abs() < 0.001); // Start of next bar
    }

    #[test]
    fn test_time_for_beat() {
        let grid = BeatGrid::from_bpm("test".to_string(), 44100.0, 4.0, 120.0, 0.0);

        assert!((grid.time_for_beat(0) - 0.0).abs() < 0.001);
        assert!((grid.time_for_beat(1) - 0.5).abs() < 0.001);
        assert!((grid.time_for_beat(4) - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_time_for_bar_beat() {
        let grid = BeatGrid::from_bpm("test".to_string(), 44100.0, 4.0, 120.0, 0.0);

        // Bar 0, beat 0 = 0.0
        assert!((grid.time_for_bar_beat(0, 0) - 0.0).abs() < 0.001);
        // Bar 0, beat 2 = 1.0
        assert!((grid.time_for_bar_beat(0, 2) - 1.0).abs() < 0.001);
        // Bar 1, beat 0 = 2.0
        assert!((grid.time_for_bar_beat(1, 0) - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_create_warp_markers() {
        let mut grid = BeatGrid::from_bpm("test".to_string(), 44100.0, 2.0, 120.0, 0.0);

        // Create warp markers to 140 BPM
        grid.create_warp_markers(140.0);

        // Should have start, beat markers, and end
        assert!(!grid.warp_markers.is_empty());

        // First marker should be at start
        assert!(matches!(
            grid.warp_markers[0].marker_type,
            WarpMarkerType::Start
        ));
        assert!((grid.warp_markers[0].source_time - 0.0).abs() < 0.001);
        assert!((grid.warp_markers[0].target_time - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_warp_time() {
        let mut grid = BeatGrid::from_bpm("test".to_string(), 44100.0, 2.0, 120.0, 0.0);

        // Create warp markers for 2x speed (240 BPM)
        grid.create_warp_markers(240.0);

        // Time 1.0 at 120 BPM should become 0.5 at 240 BPM
        let warped = grid.warp_time(1.0);
        assert!((warped - 0.5).abs() < 0.1, "Expected ~0.5, got {}", warped);
    }

    #[test]
    fn test_set_time_signature() {
        let mut grid = BeatGrid::from_bpm("test".to_string(), 44100.0, 3.0, 120.0, 0.0);

        // Change to 3/4 time
        grid.set_time_signature(3, 4);

        // Check downbeats are now every 3 beats
        let downbeats = grid.downbeats();
        assert_eq!(downbeats.len(), 2); // 6 beats / 3 = 2 complete bars

        // Check beat_in_bar values
        assert_eq!(grid.beats[0].beat_in_bar, 0);
        assert_eq!(grid.beats[1].beat_in_bar, 1);
        assert_eq!(grid.beats[2].beat_in_bar, 2);
        assert_eq!(grid.beats[3].beat_in_bar, 0); // New bar
    }

    #[test]
    fn test_phase_offset() {
        let mut grid = BeatGrid::from_bpm("test".to_string(), 44100.0, 4.0, 120.0, 0.0);

        // Shift phase by 0.25 seconds
        grid.set_phase_offset(0.25);

        assert!((grid.phase_offset - 0.25).abs() < 0.001);
        assert!((grid.beats[0].time - 0.25).abs() < 0.001);
        assert!((grid.beats[1].time - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_scale_tempo() {
        let mut grid = BeatGrid::from_bpm("test".to_string(), 44100.0, 4.0, 120.0, 0.0);

        // Double tempo
        grid.scale_tempo(2.0);

        assert!((grid.bpm - 240.0).abs() < 0.001);
        // Beat positions should be halved (relative to phase offset)
        assert!((grid.beats[0].time - 0.0).abs() < 0.001);
        assert!((grid.beats[1].time - 0.25).abs() < 0.001); // Was 0.5
    }

    #[test]
    fn test_analyze_beat_grid_synthetic() {
        let sample_rate = 44100.0;
        let bpm = 120.0;
        let duration = 4.0;
        let num_samples = (sample_rate * duration) as usize;
        let mut samples = vec![0.0f32; num_samples];

        // Add clicks at beat positions (120 BPM = every 0.5 seconds)
        let beat_duration_samples = (sample_rate * 60.0 / bpm) as usize;
        let click_length = 100;

        let mut pos = 0;
        while pos < samples.len() {
            for i in 0..click_length.min(samples.len() - pos) {
                let env = (-(i as f32) / 50.0).exp();
                samples[pos + i] = env * 0.8;
            }
            pos += beat_duration_samples;
        }

        let grid = analyze_beat_grid(&samples, sample_rate, "test".to_string());

        // Check detected BPM is close to 120
        assert!(
            (grid.bpm - bpm as f64).abs() < 20.0,
            "Expected BPM near 120, got {}",
            grid.bpm
        );

        // Should have detected some beats
        assert!(!grid.beats.is_empty(), "Should have detected beats");
    }

    #[test]
    fn test_compute_phase_difference() {
        let grid_a = BeatGrid::from_bpm("a".to_string(), 44100.0, 4.0, 120.0, 0.0);
        let grid_b = BeatGrid::from_bpm("b".to_string(), 44100.0, 4.0, 120.0, 0.5);

        // Grid B starts 0.5 seconds later = 0.25 bar at 120 BPM in 4/4
        let phase_diff = compute_phase_difference(&grid_a, &grid_b);
        assert!(
            (phase_diff - (-0.25)).abs() < 0.01,
            "Expected -0.25, got {}",
            phase_diff
        );
    }

    #[test]
    fn test_compute_tempo_ratio() {
        let grid_a = BeatGrid::from_bpm("a".to_string(), 44100.0, 4.0, 120.0, 0.0);
        let grid_b = BeatGrid::from_bpm("b".to_string(), 44100.0, 4.0, 140.0, 0.0);

        let ratio = compute_tempo_ratio(&grid_a, &grid_b);
        assert!(
            (ratio - (140.0 / 120.0)).abs() < 0.001,
            "Expected ~1.167, got {}",
            ratio
        );
    }

    #[test]
    fn test_beat_grid_analyzer_onset_envelope() {
        let sample_rate = 44100.0;
        let analyzer = BeatGridAnalyzer::new(sample_rate);

        // Create test signal with transients
        let mut samples = vec![0.0f32; 44100]; // 1 second

        // Add transients at 0.0, 0.25, 0.5, 0.75 seconds
        for &onset_time in &[0.0, 0.25, 0.5, 0.75] {
            let pos = (onset_time * sample_rate) as usize;
            for i in 0..200.min(samples.len() - pos) {
                let env = (-(i as f32) / 100.0).exp();
                samples[pos + i] = env * 0.8;
            }
        }

        let envelope = analyzer.compute_onset_envelope(&samples);

        // Envelope should have values > 0 where transients occur
        assert!(!envelope.is_empty());
        assert!(envelope.iter().any(|&v| v > 0.0));
    }

    #[test]
    fn test_tempo_estimation() {
        let sample_rate = 44100.0;
        let analyzer = BeatGridAnalyzer::new(sample_rate);

        // Create a regular pulse at 120 BPM
        let duration = 8.0;
        let num_samples = (sample_rate * duration) as usize;
        let mut samples = vec![0.0f32; num_samples];

        let beat_interval = 0.5; // 120 BPM
        let mut time = 0.0;
        while time < duration {
            let pos = (time * sample_rate) as usize;
            for i in 0..100.min(samples.len() - pos) {
                let env = (-(i as f32) / 50.0).exp();
                samples[pos + i] = env * 0.8;
            }
            time += beat_interval;
        }

        let envelope = analyzer.compute_onset_envelope(&samples);
        let (bpm, confidence) = analyzer.estimate_tempo(&envelope);

        // Should detect roughly 120 BPM (allowing for octave errors)
        let is_correct_tempo =
            (bpm - 120.0).abs() < 10.0 || (bpm - 60.0).abs() < 10.0 || (bpm - 240.0).abs() < 10.0;

        assert!(
            is_correct_tempo,
            "Expected tempo near 120 BPM (or octave), got {}",
            bpm
        );
        assert!(confidence > 0.0, "Should have some confidence");
    }
}
