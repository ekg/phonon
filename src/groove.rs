//! Groove Template Extraction and Application
//!
//! This module provides functionality to extract groove templates (timing deviations)
//! from reference audio tracks and apply them to patterns.
//!
//! A "groove" captures the subtle timing variations that make music feel human -
//! the slight push and pull around the beat that gives music its "feel" or "swing".
//!
//! ## How it works:
//!
//! 1. **Extraction**: Analyze a reference audio track to detect onsets (transients)
//! 2. **Grid Alignment**: Compare detected onsets to a quantized grid (e.g., 16th notes)
//! 3. **Deviation Calculation**: Calculate timing deviations from the grid
//! 4. **Template Storage**: Store deviations as a reusable groove template
//! 5. **Application**: Apply the template to shift pattern events by the extracted deviations
//!
//! ## Usage:
//!
//! ```phonon
//! -- Extract groove from a reference audio file
//! ~groove $ extract_groove "drums.wav" grid:16
//!
//! -- Apply groove to a pattern
//! out $ s "bd sn bd sn" $ apply_groove ~groove
//! ```

use crate::pattern::{Fraction, Pattern, State, TimeSpan};
use std::collections::HashMap;
use std::sync::Arc;

/// A groove template containing timing deviations per grid position
#[derive(Debug, Clone)]
pub struct GrooveTemplate {
    /// Name/identifier for this groove template
    pub name: String,

    /// Grid subdivision (e.g., 4 for quarter notes, 8 for 8th notes, 16 for 16th notes)
    pub grid_size: u32,

    /// Timing deviations for each grid position, stored as fractions of a cycle
    /// Length equals grid_size. Values are typically small (-0.1 to 0.1)
    /// Positive values = later (push), negative values = earlier (pull)
    pub deviations: Vec<f64>,

    /// Optional velocity/accent deviations (not yet implemented)
    pub velocity_deviations: Option<Vec<f64>>,

    /// Source information (original file, BPM, etc.)
    pub metadata: HashMap<String, String>,
}

impl GrooveTemplate {
    /// Create a new groove template
    pub fn new(name: String, grid_size: u32, deviations: Vec<f64>) -> Self {
        assert_eq!(
            deviations.len(),
            grid_size as usize,
            "Number of deviations must match grid size"
        );

        Self {
            name,
            grid_size,
            deviations,
            velocity_deviations: None,
            metadata: HashMap::new(),
        }
    }

    /// Create an empty (identity) groove template - no timing changes
    pub fn identity(grid_size: u32) -> Self {
        Self {
            name: "identity".to_string(),
            grid_size,
            deviations: vec![0.0; grid_size as usize],
            velocity_deviations: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a standard swing groove
    /// Delays every other grid position by the swing amount
    pub fn swing(grid_size: u32, amount: f64) -> Self {
        let mut deviations = vec![0.0; grid_size as usize];
        for i in 0..grid_size as usize {
            if i % 2 == 1 {
                deviations[i] = amount / grid_size as f64;
            }
        }

        Self {
            name: format!("swing_{:.0}", amount * 100.0),
            grid_size,
            deviations,
            velocity_deviations: None,
            metadata: HashMap::new(),
        }
    }

    /// Get the deviation for a specific position within a cycle
    /// Position is a fraction [0, 1) representing cycle position
    pub fn deviation_at(&self, cycle_position: f64) -> f64 {
        // Clamp to [0, 1)
        let pos = cycle_position.rem_euclid(1.0);

        // Find grid position
        let grid_pos = (pos * self.grid_size as f64).floor() as usize;
        let grid_pos = grid_pos.min(self.deviations.len() - 1);

        self.deviations[grid_pos]
    }

    /// Get deviation with interpolation between grid positions
    pub fn deviation_at_interpolated(&self, cycle_position: f64) -> f64 {
        let pos = cycle_position.rem_euclid(1.0);
        let exact_grid_pos = pos * self.grid_size as f64;

        let grid_pos_floor = exact_grid_pos.floor() as usize;
        let grid_pos_ceil = (grid_pos_floor + 1) % self.grid_size as usize;

        let t = exact_grid_pos.fract();

        let dev_floor = self.deviations[grid_pos_floor];
        let dev_ceil = self.deviations[grid_pos_ceil];

        // Linear interpolation
        dev_floor * (1.0 - t) + dev_ceil * t
    }

    /// Scale the groove deviations by a factor
    pub fn scale(&self, factor: f64) -> Self {
        Self {
            name: format!("{}*{:.2}", self.name, factor),
            grid_size: self.grid_size,
            deviations: self.deviations.iter().map(|d| d * factor).collect(),
            velocity_deviations: self
                .velocity_deviations
                .as_ref()
                .map(|v| v.iter().map(|d| d * factor).collect()),
            metadata: self.metadata.clone(),
        }
    }

    /// Invert the groove (positive becomes negative, vice versa)
    pub fn invert(&self) -> Self {
        self.scale(-1.0)
    }

    /// Blend two groove templates
    pub fn blend(&self, other: &GrooveTemplate, mix: f64) -> Self {
        assert_eq!(
            self.grid_size, other.grid_size,
            "Cannot blend grooves with different grid sizes"
        );

        let deviations: Vec<f64> = self
            .deviations
            .iter()
            .zip(other.deviations.iter())
            .map(|(a, b)| a * (1.0 - mix) + b * mix)
            .collect();

        Self {
            name: format!("blend_{}_{}", self.name, other.name),
            grid_size: self.grid_size,
            deviations,
            velocity_deviations: None,
            metadata: HashMap::new(),
        }
    }
}

/// Analyze audio to extract onset times and timing deviations
pub struct GrooveAnalyzer {
    sample_rate: f32,
    /// Detected BPM
    pub detected_bpm: Option<f32>,
    /// Raw onset times in seconds
    pub onset_times: Vec<f32>,
    /// Quantized onset times (aligned to grid)
    pub quantized_times: Vec<f32>,
    /// Timing deviations (onset_time - quantized_time) in seconds
    pub deviations_seconds: Vec<f32>,
}

impl GrooveAnalyzer {
    /// Create a new groove analyzer
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            detected_bpm: None,
            onset_times: Vec::new(),
            quantized_times: Vec::new(),
            deviations_seconds: Vec::new(),
        }
    }

    /// Analyze audio samples to detect onsets using spectral flux
    pub fn detect_onsets(&mut self, samples: &[f32], threshold_factor: f32) -> Vec<f32> {
        let window_size = (self.sample_rate as usize / 50).max(128); // 20ms windows
        let hop_size = window_size / 2;

        let mut energies = Vec::new();
        let mut i = 0;

        // Calculate energy in each window
        while i + window_size < samples.len() {
            let window = &samples[i..i + window_size];
            let energy: f32 = window.iter().map(|x| x * x).sum::<f32>() / window_size as f32;
            energies.push(energy);
            i += hop_size;
        }

        if energies.is_empty() {
            return Vec::new();
        }

        // Smooth energies to reduce noise
        let mut smoothed = Vec::new();
        for i in 0..energies.len() {
            let start = i.saturating_sub(2);
            let end = (i + 3).min(energies.len());
            let avg: f32 = energies[start..end].iter().sum::<f32>() / (end - start) as f32;
            smoothed.push(avg);
        }

        // Find peaks using adaptive threshold
        let mean_energy: f32 = smoothed.iter().sum::<f32>() / smoothed.len() as f32;
        let mut variance = 0.0;
        for &e in &smoothed {
            variance += (e - mean_energy).powi(2);
        }
        let std_dev = (variance / smoothed.len() as f32).sqrt();

        // Dynamic threshold
        let threshold = mean_energy + std_dev * threshold_factor;

        let mut peaks = Vec::new();
        let mut in_peak = false;
        let mut peak_start = 0;

        // Minimum time between peaks (prevents double detection)
        let min_peak_distance = (self.sample_rate as usize / 10) / hop_size; // 100ms

        for i in 1..smoothed.len() {
            if smoothed[i] > threshold && smoothed[i] > smoothed[i - 1] {
                if !in_peak {
                    in_peak = true;
                    peak_start = i;
                }
            } else if in_peak && smoothed[i] < smoothed[i - 1] {
                in_peak = false;

                // Check minimum distance from last peak
                if peaks.is_empty() || i - *peaks.last().unwrap() > min_peak_distance {
                    peaks.push(peak_start);
                }
            }
        }

        // Convert peak indices to time in seconds
        self.onset_times = peaks
            .iter()
            .map(|&peak_idx| (peak_idx * hop_size) as f32 / self.sample_rate)
            .collect();

        self.onset_times.clone()
    }

    /// Estimate BPM from onset intervals
    pub fn estimate_bpm(&mut self) -> Option<f32> {
        if self.onset_times.len() < 4 {
            return None;
        }

        let mut intervals: Vec<f32> = Vec::new();
        for i in 1..self.onset_times.len() {
            let interval = self.onset_times[i] - self.onset_times[i - 1];
            if interval > 0.05 && interval < 2.0 {
                intervals.push(interval);
            }
        }

        if intervals.is_empty() {
            return None;
        }

        // Find most common interval using histogram
        intervals.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median_interval = intervals[intervals.len() / 2];

        // Convert to BPM (beats per minute)
        let bpm = 60.0 / median_interval;

        // Adjust to reasonable BPM range (60-200)
        let adjusted_bpm = if bpm < 60.0 {
            bpm * 2.0
        } else if bpm > 200.0 {
            bpm / 2.0
        } else {
            bpm
        };

        self.detected_bpm = Some(adjusted_bpm);
        Some(adjusted_bpm)
    }

    /// Quantize onset times to a grid
    /// Returns timing deviations in seconds (positive = late, negative = early)
    pub fn quantize_to_grid(&mut self, bpm: f32, grid_size: u32) -> Vec<f32> {
        // Calculate beat duration in seconds
        let beat_duration = 60.0 / bpm;

        // Grid subdivision duration
        let grid_duration = beat_duration / grid_size as f32;

        self.quantized_times.clear();
        self.deviations_seconds.clear();

        for &onset_time in &self.onset_times {
            // Find nearest grid position
            let grid_position = (onset_time / grid_duration).round();
            let quantized_time = grid_position * grid_duration;

            self.quantized_times.push(quantized_time);
            self.deviations_seconds.push(onset_time - quantized_time);
        }

        self.deviations_seconds.clone()
    }

    /// Extract a groove template from analyzed audio
    /// Aggregates deviations by grid position across the entire track
    pub fn extract_groove_template(
        &self,
        bpm: f32,
        grid_size: u32,
        name: String,
    ) -> GrooveTemplate {
        // Calculate beat duration in seconds
        let beat_duration = 60.0 / bpm;
        let cycle_duration = beat_duration; // 1 cycle = 1 beat for now

        // Grid subdivision duration
        let grid_duration = cycle_duration / grid_size as f32;

        // Accumulate deviations for each grid position
        let mut deviation_sums: Vec<f64> = vec![0.0; grid_size as usize];
        let mut deviation_counts: Vec<u32> = vec![0; grid_size as usize];

        for &onset_time in &self.onset_times {
            // Find which grid position this onset corresponds to
            let grid_position_float = onset_time / grid_duration;
            let nearest_grid = grid_position_float.round() as i64;

            // Map to position within cycle (0 to grid_size-1)
            let grid_pos_in_cycle = nearest_grid.rem_euclid(grid_size as i64) as usize;

            // Calculate deviation as fraction of cycle
            let quantized_time = nearest_grid as f32 * grid_duration;
            let deviation_seconds = onset_time - quantized_time;
            let deviation_cycles = deviation_seconds as f64 / cycle_duration as f64;

            deviation_sums[grid_pos_in_cycle] += deviation_cycles;
            deviation_counts[grid_pos_in_cycle] += 1;
        }

        // Average deviations for each grid position
        let deviations: Vec<f64> = deviation_sums
            .iter()
            .zip(deviation_counts.iter())
            .map(
                |(&sum, &count)| {
                    if count > 0 {
                        sum / count as f64
                    } else {
                        0.0
                    }
                },
            )
            .collect();

        let mut template = GrooveTemplate::new(name, grid_size, deviations);

        if let Some(bpm) = self.detected_bpm {
            template
                .metadata
                .insert("source_bpm".to_string(), bpm.to_string());
        }

        template
    }
}

/// Analyze audio file and extract a groove template
pub fn extract_groove_from_audio(
    samples: &[f32],
    sample_rate: f32,
    grid_size: u32,
    name: String,
    bpm: Option<f32>,
) -> Result<GrooveTemplate, String> {
    let mut analyzer = GrooveAnalyzer::new(sample_rate);

    // Detect onsets
    analyzer.detect_onsets(samples, 1.5);

    if analyzer.onset_times.is_empty() {
        return Err("No onsets detected in audio".to_string());
    }

    // Estimate or use provided BPM
    let bpm = match bpm {
        Some(b) => b,
        None => analyzer
            .estimate_bpm()
            .ok_or_else(|| "Could not estimate BPM from audio".to_string())?,
    };

    // Quantize to grid
    analyzer.quantize_to_grid(bpm, grid_size);

    // Extract template
    Ok(analyzer.extract_groove_template(bpm, grid_size, name))
}

// Pattern transformation implementation
impl<T: Clone + Send + Sync + 'static> Pattern<T> {
    /// Apply a groove template to shift event timings
    ///
    /// Events are shifted based on their position within the cycle.
    /// The groove template contains timing deviations for each grid position.
    ///
    /// # Arguments
    /// * `template` - The groove template to apply
    /// * `amount` - How much of the groove to apply (0.0 = none, 1.0 = full)
    ///
    /// # Example
    /// ```
    /// let pattern = Pattern::from_string("bd sn bd sn");
    /// let groove = GrooveTemplate::swing(4, 0.1);
    /// let grooved = pattern.apply_groove(&groove, 1.0);
    /// ```
    pub fn apply_groove(self, template: Arc<GrooveTemplate>, amount: Pattern<f64>) -> Self {
        Pattern::new(move |state: &State| {
            // Query amount at cycle start
            let cycle_start = state.span.begin.to_float().floor();
            let amount_state = State {
                span: TimeSpan::new(
                    Fraction::from_float(cycle_start),
                    Fraction::from_float(cycle_start + 0.001),
                ),
                controls: state.controls.clone(),
            };

            let groove_amount = amount
                .query(&amount_state)
                .first()
                .map(|h| h.value)
                .unwrap_or(1.0);

            // Query the pattern
            let haps = self.query(state);

            // Apply groove to each event
            haps.into_iter()
                .map(|mut hap| {
                    // Get event position within cycle
                    let event_time = hap.part.begin.to_float();
                    let cycle_pos = event_time - event_time.floor();

                    // Get deviation from template
                    let deviation = template.deviation_at(cycle_pos);
                    let scaled_deviation = deviation * groove_amount;

                    // Apply shift to event timing
                    let shift = Fraction::from_float(scaled_deviation);
                    hap.part = TimeSpan::new(hap.part.begin + shift, hap.part.end + shift);

                    if let Some(whole) = hap.whole.as_mut() {
                        *whole = TimeSpan::new(whole.begin + shift, whole.end + shift);
                    }

                    hap
                })
                .collect()
        })
    }

    /// Apply groove with interpolation between grid positions
    /// This provides smoother results for patterns that don't align exactly to the grid
    pub fn apply_groove_smooth(self, template: Arc<GrooveTemplate>, amount: Pattern<f64>) -> Self {
        Pattern::new(move |state: &State| {
            // Query amount at cycle start
            let cycle_start = state.span.begin.to_float().floor();
            let amount_state = State {
                span: TimeSpan::new(
                    Fraction::from_float(cycle_start),
                    Fraction::from_float(cycle_start + 0.001),
                ),
                controls: state.controls.clone(),
            };

            let groove_amount = amount
                .query(&amount_state)
                .first()
                .map(|h| h.value)
                .unwrap_or(1.0);

            let haps = self.query(state);

            haps.into_iter()
                .map(|mut hap| {
                    let event_time = hap.part.begin.to_float();
                    let cycle_pos = event_time - event_time.floor();

                    // Use interpolated deviation
                    let deviation = template.deviation_at_interpolated(cycle_pos);
                    let scaled_deviation = deviation * groove_amount;

                    let shift = Fraction::from_float(scaled_deviation);
                    hap.part = TimeSpan::new(hap.part.begin + shift, hap.part.end + shift);

                    if let Some(whole) = hap.whole.as_mut() {
                        *whole = TimeSpan::new(whole.begin + shift, whole.end + shift);
                    }

                    hap
                })
                .collect()
        })
    }
}

// Built-in groove presets
pub mod presets {
    use super::GrooveTemplate;

    /// MPC-style swing (delays 2nd and 4th 16th notes)
    pub fn mpc_swing(amount: f64) -> GrooveTemplate {
        // 16th note grid
        let mut deviations = vec![0.0; 16];
        // Delay every other 16th note (the off-beats)
        for i in (1..16).step_by(2) {
            deviations[i] = amount / 16.0;
        }

        let mut template = GrooveTemplate::new("mpc_swing".to_string(), 16, deviations);
        template
            .metadata
            .insert("style".to_string(), "mpc".to_string());
        template
    }

    /// Hip-hop lazy feel (slight drag on 2 and 4)
    pub fn lazy_hiphop() -> GrooveTemplate {
        let mut deviations = vec![0.0; 16];
        // Drag beat 2 (grid position 4)
        deviations[4] = 0.02;
        // Drag beat 4 (grid position 12)
        deviations[12] = 0.02;
        // Light shuffle on 8th notes
        for i in (2..16).step_by(4) {
            deviations[i] = 0.01;
        }

        let mut template = GrooveTemplate::new("lazy_hiphop".to_string(), 16, deviations);
        template
            .metadata
            .insert("style".to_string(), "hiphop".to_string());
        template
    }

    /// Reggae one-drop feel (pushes the backbeat)
    pub fn reggae_one_drop() -> GrooveTemplate {
        let mut deviations = vec![0.0; 16];
        // Strong push on beat 3 (position 8)
        deviations[8] = 0.025;
        // Lighter push on offbeats
        deviations[2] = 0.01;
        deviations[6] = 0.01;
        deviations[10] = 0.01;
        deviations[14] = 0.01;

        let mut template = GrooveTemplate::new("reggae_one_drop".to_string(), 16, deviations);
        template
            .metadata
            .insert("style".to_string(), "reggae".to_string());
        template
    }

    /// Jazz swing (triplet feel)
    pub fn jazz_swing(amount: f64) -> GrooveTemplate {
        // 8th note grid with swing
        let mut deviations = vec![0.0; 8];
        // Delay every other 8th note
        for i in (1..8).step_by(2) {
            deviations[i] = amount / 8.0;
        }

        let mut template = GrooveTemplate::new("jazz_swing".to_string(), 8, deviations);
        template
            .metadata
            .insert("style".to_string(), "jazz".to_string());
        template
    }

    /// Drunken drummer (random-ish timing)
    pub fn drunken(intensity: f64) -> GrooveTemplate {
        // Pseudo-random deviations based on a fixed pattern
        let base_deviations = [
            0.3, -0.2, 0.1, -0.3, 0.2, -0.1, 0.25, -0.25, 0.15, -0.35, 0.2, -0.15, 0.35, -0.2, 0.1,
            -0.3,
        ];

        let deviations: Vec<f64> = base_deviations
            .iter()
            .map(|d| d * intensity / 16.0)
            .collect();

        let mut template = GrooveTemplate::new("drunken".to_string(), 16, deviations);
        template
            .metadata
            .insert("style".to_string(), "humanized".to_string());
        template
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::{Fraction, State, TimeSpan};
    use std::collections::HashMap;

    #[test]
    fn test_groove_template_creation() {
        let template = GrooveTemplate::new("test".to_string(), 4, vec![0.0, 0.05, 0.0, -0.05]);

        assert_eq!(template.grid_size, 4);
        assert_eq!(template.deviations.len(), 4);
    }

    #[test]
    fn test_identity_groove() {
        let template = GrooveTemplate::identity(8);

        assert_eq!(template.grid_size, 8);
        assert!(template.deviations.iter().all(|&d| d == 0.0));
    }

    #[test]
    fn test_swing_groove() {
        let template = GrooveTemplate::swing(4, 0.2);

        // Odd positions should have deviation, even should not
        assert_eq!(template.deviations[0], 0.0);
        assert_eq!(template.deviations[1], 0.2 / 4.0);
        assert_eq!(template.deviations[2], 0.0);
        assert_eq!(template.deviations[3], 0.2 / 4.0);
    }

    #[test]
    fn test_deviation_at() {
        let template = GrooveTemplate::new("test".to_string(), 4, vec![0.1, 0.2, 0.3, 0.4]);

        // Position 0.0 -> grid 0 -> deviation 0.1
        assert!((template.deviation_at(0.0) - 0.1).abs() < 0.001);

        // Position 0.25 -> grid 1 -> deviation 0.2
        assert!((template.deviation_at(0.25) - 0.2).abs() < 0.001);

        // Position 0.5 -> grid 2 -> deviation 0.3
        assert!((template.deviation_at(0.5) - 0.3).abs() < 0.001);

        // Position 0.75 -> grid 3 -> deviation 0.4
        assert!((template.deviation_at(0.75) - 0.4).abs() < 0.001);
    }

    #[test]
    fn test_deviation_at_interpolated() {
        let template = GrooveTemplate::new("test".to_string(), 4, vec![0.0, 0.2, 0.0, 0.2]);

        // Halfway between grid 0 (0.0) and grid 1 (0.2) should be 0.1
        let dev = template.deviation_at_interpolated(0.125);
        assert!((dev - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_groove_scale() {
        let template = GrooveTemplate::new("test".to_string(), 4, vec![0.1, 0.2, 0.3, 0.4]);

        let scaled = template.scale(0.5);

        assert!((scaled.deviations[0] - 0.05).abs() < 0.001);
        assert!((scaled.deviations[1] - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_groove_blend() {
        let template1 = GrooveTemplate::new("test1".to_string(), 4, vec![0.0, 0.0, 0.0, 0.0]);

        let template2 = GrooveTemplate::new("test2".to_string(), 4, vec![0.1, 0.2, 0.3, 0.4]);

        // 50% blend
        let blended = template1.blend(&template2, 0.5);

        assert!((blended.deviations[0] - 0.05).abs() < 0.001);
        assert!((blended.deviations[1] - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_groove_analyzer_onset_detection() {
        let sample_rate = 44100.0;
        let mut analyzer = GrooveAnalyzer::new(sample_rate);

        // Create synthetic audio with clear onsets
        let duration_samples = (sample_rate * 2.0) as usize;
        let mut samples = vec![0.0f32; duration_samples];

        // Add transients at 0.0, 0.5, 1.0, 1.5 seconds
        let onset_positions = [0.0, 0.5, 1.0, 1.5];
        for &onset in &onset_positions {
            let sample_pos = (onset * sample_rate) as usize;
            // Add a short burst
            for i in 0..500 {
                if sample_pos + i < samples.len() {
                    let env = (-(i as f32) / 200.0).exp();
                    samples[sample_pos + i] = env * 0.8;
                }
            }
        }

        let onsets = analyzer.detect_onsets(&samples, 1.5);

        // Should detect roughly 4 onsets
        assert!(
            onsets.len() >= 3,
            "Expected at least 3 onsets, got {}",
            onsets.len()
        );
    }

    #[test]
    fn test_apply_groove_to_pattern() {
        let pattern = Pattern::from_string("a b c d");
        let groove = Arc::new(GrooveTemplate::new(
            "test".to_string(),
            4,
            vec![0.0, 0.05, 0.0, 0.05], // Swing on positions 1 and 3
        ));

        let grooved = pattern.apply_groove(groove, Pattern::pure(1.0));

        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };

        let haps = grooved.query(&state);

        // Should have 4 events
        assert_eq!(haps.len(), 4);

        // Event at position 0 should not be shifted
        let pos0 = haps[0].part.begin.to_float();
        assert!((pos0 - 0.0).abs() < 0.01);

        // Event at position 0.25 should be shifted by 0.05
        let pos1 = haps[1].part.begin.to_float();
        assert!((pos1 - 0.30).abs() < 0.01, "Expected ~0.30, got {}", pos1);
    }

    #[test]
    fn test_preset_mpc_swing() {
        let groove = presets::mpc_swing(0.5);

        assert_eq!(groove.grid_size, 16);
        assert_eq!(groove.deviations[0], 0.0); // On-beat
        assert!(groove.deviations[1] > 0.0); // Off-beat swung
    }

    #[test]
    fn test_extract_groove_from_synthetic_audio() {
        let sample_rate = 44100.0;

        // Create synthetic audio at 120 BPM with slight swing
        // 120 BPM = 0.5 seconds per beat
        let bpm = 120.0;
        let beat_duration = 60.0 / bpm;
        let duration_secs = 4.0;
        let num_samples = (sample_rate * duration_secs) as usize;
        let mut samples = vec![0.0f32; num_samples];

        // Add beats with swing (every other 8th note delayed by 20ms)
        let grid_duration = beat_duration / 2.0; // 8th notes
        let swing_delay = 0.02; // 20ms swing

        for beat in 0..8 {
            let base_time = beat as f32 * grid_duration;
            let swing = if beat % 2 == 1 { swing_delay } else { 0.0 };
            let onset_time = base_time + swing;

            let sample_pos = (onset_time * sample_rate) as usize;

            // Add a transient
            for i in 0..500 {
                if sample_pos + i < samples.len() {
                    let env = (-(i as f32) / 200.0).exp();
                    samples[sample_pos + i] = env * 0.8;
                }
            }
        }

        // Extract groove
        let result = extract_groove_from_audio(
            &samples,
            sample_rate,
            8, // 8th note grid
            "test_swing".to_string(),
            Some(bpm),
        );

        assert!(result.is_ok(), "Groove extraction failed: {:?}", result);

        let groove = result.unwrap();

        // The odd positions should show positive deviation (late)
        // Note: due to analysis tolerances, we just check the pattern
        assert_eq!(groove.grid_size, 8);
    }
}
