//! Onset Timing Accuracy Validator
//!
//! This module provides robust onset detection and timing accuracy validation
//! for verifying that audio output matches expected pattern timing.
//!
//! ## Features
//!
//! - **Spectral Flux Detection**: State-of-the-art onset detection using spectral flux
//!   with log compression and half-wave rectification
//! - **High Frequency Content (HFC)**: Optimized for percussive onsets
//! - **Timing Accuracy Metrics**: Mean/max deviation, jitter, match rate
//! - **Pattern Integration**: Compare detected onsets against expected pattern events
//!
//! ## Usage
//!
//! ```rust
//! use phonon::onset_timing::{OnsetDetector, OnsetDetectorConfig, TimingValidator};
//!
//! // Detect onsets in audio
//! let detector = OnsetDetector::new(44100.0, OnsetDetectorConfig::default());
//! let onsets = detector.detect(&audio_samples);
//!
//! // Validate timing against expected events
//! let validator = TimingValidator::new(50.0); // 50ms tolerance
//! let result = validator.validate(&expected_times_ms, &onsets);
//! assert!(result.is_acceptable(0.9)); // 90% match rate
//! ```

use rustfft::{num_complex::Complex, FftPlanner};
use std::f32::consts::PI;

/// Configuration for onset detection
#[derive(Debug, Clone)]
pub struct OnsetDetectorConfig {
    /// FFT window size in samples (default: 2048)
    pub fft_size: usize,
    /// Hop size in samples (default: 512)
    pub hop_size: usize,
    /// Log compression gamma (default: 100.0)
    pub gamma: f32,
    /// Peak picking threshold multiplier (default: 1.5)
    pub threshold_multiplier: f32,
    /// Minimum time between onsets in seconds (default: 0.03)
    pub min_onset_distance_sec: f32,
    /// Detection mode
    pub mode: OnsetDetectionMode,
    /// Adaptive threshold window size in frames (default: 10)
    pub adaptive_window: usize,
}

impl Default for OnsetDetectorConfig {
    fn default() -> Self {
        Self {
            fft_size: 2048,
            hop_size: 512,
            gamma: 100.0,
            threshold_multiplier: 1.5,
            min_onset_distance_sec: 0.03,
            mode: OnsetDetectionMode::SpectralFlux,
            adaptive_window: 10,
        }
    }
}

impl OnsetDetectorConfig {
    /// Configuration optimized for percussive sounds (drums, plucks)
    pub fn percussive() -> Self {
        Self {
            fft_size: 1024,
            hop_size: 256,
            gamma: 100.0,
            threshold_multiplier: 1.3,
            min_onset_distance_sec: 0.025,
            mode: OnsetDetectionMode::HighFrequencyContent,
            adaptive_window: 8,
        }
    }

    /// Configuration optimized for tonal/pitched sounds (synths, bass)
    pub fn tonal() -> Self {
        Self {
            fft_size: 2048,
            hop_size: 512,
            gamma: 100.0,
            threshold_multiplier: 1.8,
            min_onset_distance_sec: 0.04,
            mode: OnsetDetectionMode::SpectralFlux,
            adaptive_window: 12,
        }
    }

    /// Configuration for dense patterns with fast events
    pub fn dense() -> Self {
        Self {
            fft_size: 1024,
            hop_size: 256,
            gamma: 80.0,
            threshold_multiplier: 1.2,
            min_onset_distance_sec: 0.015,
            mode: OnsetDetectionMode::SpectralFlux,
            adaptive_window: 6,
        }
    }
}

/// Onset detection algorithm mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OnsetDetectionMode {
    /// Spectral flux with log compression (general purpose)
    SpectralFlux,
    /// High frequency content (best for percussive)
    HighFrequencyContent,
    /// Combined: use both and take the union
    Combined,
}

/// A detected onset with timing and strength information
#[derive(Debug, Clone, Copy)]
pub struct DetectedOnset {
    /// Time in seconds from start of audio
    pub time_sec: f64,
    /// Time in milliseconds from start of audio
    pub time_ms: f64,
    /// Onset strength (normalized 0-1)
    pub strength: f32,
    /// Frame index where onset was detected
    pub frame: usize,
}

/// Onset detector using spectral flux and/or HFC
pub struct OnsetDetector {
    sample_rate: f32,
    config: OnsetDetectorConfig,
    window: Vec<f32>,
}

impl OnsetDetector {
    /// Create a new onset detector
    pub fn new(sample_rate: f32, config: OnsetDetectorConfig) -> Self {
        // Pre-compute Hann window
        let window: Vec<f32> = (0..config.fft_size)
            .map(|i| 0.5 * (1.0 - (2.0 * PI * i as f32 / (config.fft_size - 1) as f32).cos()))
            .collect();

        Self {
            sample_rate,
            config,
            window,
        }
    }

    /// Create with default configuration
    pub fn with_defaults(sample_rate: f32) -> Self {
        Self::new(sample_rate, OnsetDetectorConfig::default())
    }

    /// Detect onsets in audio samples
    pub fn detect(&self, audio: &[f32]) -> Vec<DetectedOnset> {
        if audio.len() < self.config.fft_size {
            return Vec::new();
        }

        match self.config.mode {
            OnsetDetectionMode::SpectralFlux => self.detect_spectral_flux(audio),
            OnsetDetectionMode::HighFrequencyContent => self.detect_hfc(audio),
            OnsetDetectionMode::Combined => {
                let sf_onsets = self.detect_spectral_flux(audio);
                let hfc_onsets = self.detect_hfc(audio);
                self.merge_onsets(sf_onsets, hfc_onsets)
            }
        }
    }

    /// Spectral flux onset detection with log compression
    fn detect_spectral_flux(&self, audio: &[f32]) -> Vec<DetectedOnset> {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(self.config.fft_size);
        let mut detection_function = Vec::new();

        let mut prev_magnitude: Option<Vec<f32>> = None;
        let mut frame_idx = 0;

        // Compute spectral flux for each frame
        let mut pos = 0;
        while pos + self.config.fft_size <= audio.len() {
            // Apply window and prepare FFT input
            let mut buffer: Vec<Complex<f32>> = (0..self.config.fft_size)
                .map(|i| Complex::new(audio[pos + i] * self.window[i], 0.0))
                .collect();

            // Perform FFT
            fft.process(&mut buffer);

            // Compute magnitude spectrum with log compression
            let magnitude: Vec<f32> = buffer
                .iter()
                .take(self.config.fft_size / 2)
                .map(|bin| {
                    let mag = (bin.re * bin.re + bin.im * bin.im).sqrt();
                    // Log compression: log(1 + gamma * |X|)
                    (1.0 + self.config.gamma * mag).ln()
                })
                .collect();

            // Compute spectral flux (positive differences only - half-wave rectification)
            if let Some(ref prev) = prev_magnitude {
                let flux: f32 = magnitude
                    .iter()
                    .zip(prev.iter())
                    .map(|(&curr, &prev)| (curr - prev).max(0.0))
                    .sum();
                detection_function.push((frame_idx, flux));
            }

            prev_magnitude = Some(magnitude);
            pos += self.config.hop_size;
            frame_idx += 1;
        }

        self.peak_pick(&detection_function)
    }

    /// High frequency content onset detection
    fn detect_hfc(&self, audio: &[f32]) -> Vec<DetectedOnset> {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(self.config.fft_size);
        let mut detection_function = Vec::new();

        let mut frame_idx = 0;

        // Compute HFC for each frame
        let mut pos = 0;
        while pos + self.config.fft_size <= audio.len() {
            // Apply window and prepare FFT input
            let mut buffer: Vec<Complex<f32>> = (0..self.config.fft_size)
                .map(|i| Complex::new(audio[pos + i] * self.window[i], 0.0))
                .collect();

            // Perform FFT
            fft.process(&mut buffer);

            // Compute HFC: sum of (k * |X(k)|^2)
            // Higher frequencies are weighted more heavily
            let hfc: f32 = buffer
                .iter()
                .enumerate()
                .take(self.config.fft_size / 2)
                .map(|(k, bin)| {
                    let mag_sq = bin.re * bin.re + bin.im * bin.im;
                    (k as f32) * mag_sq
                })
                .sum();

            detection_function.push((frame_idx, hfc));

            pos += self.config.hop_size;
            frame_idx += 1;
        }

        // For HFC, we look at the derivative (change from previous frame)
        let mut diff_function = Vec::new();
        for i in 1..detection_function.len() {
            let diff = (detection_function[i].1 - detection_function[i - 1].1).max(0.0);
            diff_function.push((detection_function[i].0, diff));
        }

        self.peak_pick(&diff_function)
    }

    /// Peak picking with adaptive threshold
    fn peak_pick(&self, detection_function: &[(usize, f32)]) -> Vec<DetectedOnset> {
        if detection_function.is_empty() {
            return Vec::new();
        }

        let mut onsets = Vec::new();
        let min_distance_frames = (self.config.min_onset_distance_sec * self.sample_rate
            / self.config.hop_size as f32) as usize;

        // Compute local statistics for adaptive threshold
        let window_size = self.config.adaptive_window;

        for (i, &(frame, value)) in detection_function.iter().enumerate() {
            // Compute local mean and std dev
            let start = i.saturating_sub(window_size);
            let end = (i + window_size + 1).min(detection_function.len());
            let local_values: Vec<f32> = detection_function[start..end]
                .iter()
                .map(|(_, v)| *v)
                .collect();

            let mean = local_values.iter().sum::<f32>() / local_values.len() as f32;
            let variance = local_values
                .iter()
                .map(|&v| (v - mean).powi(2))
                .sum::<f32>()
                / local_values.len() as f32;
            let std_dev = variance.sqrt();

            // Adaptive threshold
            let threshold = mean + self.config.threshold_multiplier * std_dev;

            // Check if this is a peak (local maximum above threshold)
            let is_local_max = if i > 0 && i < detection_function.len() - 1 {
                value > detection_function[i - 1].1 && value >= detection_function[i + 1].1
            } else {
                false
            };

            if value > threshold && is_local_max {
                // Check minimum distance from last onset
                let far_enough = onsets
                    .last()
                    .map(|last: &DetectedOnset| frame - last.frame >= min_distance_frames)
                    .unwrap_or(true);

                if far_enough {
                    let time_sec =
                        frame as f64 * self.config.hop_size as f64 / self.sample_rate as f64;

                    // Normalize strength to 0-1 range
                    let max_value = detection_function
                        .iter()
                        .map(|(_, v)| *v)
                        .fold(0.0f32, f32::max);
                    let strength = if max_value > 0.0 {
                        (value / max_value).min(1.0)
                    } else {
                        0.0
                    };

                    onsets.push(DetectedOnset {
                        time_sec,
                        time_ms: time_sec * 1000.0,
                        strength,
                        frame,
                    });
                }
            }
        }

        onsets
    }

    /// Merge onsets from multiple detection methods
    fn merge_onsets(
        &self,
        mut onsets1: Vec<DetectedOnset>,
        onsets2: Vec<DetectedOnset>,
    ) -> Vec<DetectedOnset> {
        let merge_tolerance_sec = self.config.min_onset_distance_sec;

        for onset2 in onsets2 {
            let already_detected = onsets1
                .iter()
                .any(|o| (o.time_sec - onset2.time_sec).abs() < merge_tolerance_sec as f64);

            if !already_detected {
                onsets1.push(onset2);
            }
        }

        // Sort by time
        onsets1.sort_by(|a, b| a.time_sec.partial_cmp(&b.time_sec).unwrap());
        onsets1
    }

    /// Get the frame time resolution in milliseconds
    pub fn frame_resolution_ms(&self) -> f64 {
        self.config.hop_size as f64 / self.sample_rate as f64 * 1000.0
    }
}

/// Result of timing validation
#[derive(Debug, Clone)]
pub struct TimingValidationResult {
    /// Number of expected onsets that were matched
    pub matched: usize,
    /// Expected onsets that were not found
    pub missing: Vec<f64>,
    /// Detected onsets that don't match expected
    pub extra: Vec<f64>,
    /// Total expected onsets
    pub total_expected: usize,
    /// Total detected onsets
    pub total_detected: usize,
    /// Match rate (0-1)
    pub match_rate: f32,
    /// Timing deviations for matched onsets (in ms)
    pub deviations_ms: Vec<f64>,
    /// Mean timing deviation (ms)
    pub mean_deviation_ms: f64,
    /// Maximum timing deviation (ms)
    pub max_deviation_ms: f64,
    /// Timing jitter (standard deviation of deviations, ms)
    pub jitter_ms: f64,
}

impl TimingValidationResult {
    /// Check if timing accuracy is acceptable
    pub fn is_acceptable(&self, min_match_rate: f32) -> bool {
        self.match_rate >= min_match_rate
    }

    /// Check if timing accuracy meets strict criteria
    pub fn is_accurate(&self, max_mean_deviation_ms: f64, max_jitter_ms: f64) -> bool {
        self.mean_deviation_ms <= max_mean_deviation_ms && self.jitter_ms <= max_jitter_ms
    }
}

/// Validates onset timing against expected pattern events
pub struct TimingValidator {
    /// Tolerance for matching onsets (in milliseconds)
    tolerance_ms: f64,
}

impl TimingValidator {
    /// Create a new timing validator with specified tolerance
    pub fn new(tolerance_ms: f64) -> Self {
        Self { tolerance_ms }
    }

    /// Default validator with 50ms tolerance (suitable for most drum patterns)
    pub fn default_tolerance() -> Self {
        Self::new(50.0)
    }

    /// Strict validator with 20ms tolerance (for precise timing verification)
    pub fn strict() -> Self {
        Self::new(20.0)
    }

    /// Very strict validator with 10ms tolerance (for critical timing tests)
    pub fn very_strict() -> Self {
        Self::new(10.0)
    }

    /// Validate detected onsets against expected onset times
    pub fn validate(
        &self,
        expected_times_ms: &[f64],
        detected: &[DetectedOnset],
    ) -> TimingValidationResult {
        let detected_times_ms: Vec<f64> = detected.iter().map(|o| o.time_ms).collect();
        self.validate_times(expected_times_ms, &detected_times_ms)
    }

    /// Validate onset times directly (both in milliseconds)
    pub fn validate_times(
        &self,
        expected_ms: &[f64],
        detected_ms: &[f64],
    ) -> TimingValidationResult {
        let mut matched = 0;
        let mut missing = Vec::new();
        let mut deviations_ms = Vec::new();
        let mut detected_used = vec![false; detected_ms.len()];

        // Match expected to detected
        for &expected_time in expected_ms {
            let mut best_match: Option<(usize, f64)> = None;

            for (i, &detected_time) in detected_ms.iter().enumerate() {
                if detected_used[i] {
                    continue;
                }

                let diff = (detected_time - expected_time).abs();
                if diff <= self.tolerance_ms {
                    match best_match {
                        None => best_match = Some((i, diff)),
                        Some((_, prev_diff)) if diff < prev_diff => best_match = Some((i, diff)),
                        _ => {}
                    }
                }
            }

            if let Some((idx, deviation)) = best_match {
                matched += 1;
                detected_used[idx] = true;
                deviations_ms.push(deviation);
            } else {
                missing.push(expected_time);
            }
        }

        // Find extra detected onsets
        let extra: Vec<f64> = detected_ms
            .iter()
            .enumerate()
            .filter(|(i, _)| !detected_used[*i])
            .map(|(_, &t)| t)
            .collect();

        // Calculate statistics
        let match_rate = if expected_ms.is_empty() {
            1.0
        } else {
            matched as f32 / expected_ms.len() as f32
        };

        let (mean_deviation_ms, max_deviation_ms, jitter_ms) = if deviations_ms.is_empty() {
            (0.0, 0.0, 0.0)
        } else {
            let mean = deviations_ms.iter().sum::<f64>() / deviations_ms.len() as f64;
            let max = deviations_ms.iter().cloned().fold(0.0f64, f64::max);
            let variance = deviations_ms
                .iter()
                .map(|&d| (d - mean).powi(2))
                .sum::<f64>()
                / deviations_ms.len() as f64;
            let jitter = variance.sqrt();

            (mean, max, jitter)
        };

        TimingValidationResult {
            matched,
            missing,
            extra,
            total_expected: expected_ms.len(),
            total_detected: detected_ms.len(),
            match_rate,
            deviations_ms,
            mean_deviation_ms,
            max_deviation_ms,
            jitter_ms,
        }
    }
}

/// Convenience function for simple onset detection
pub fn detect_onsets(audio: &[f32], sample_rate: f32) -> Vec<DetectedOnset> {
    OnsetDetector::with_defaults(sample_rate).detect(audio)
}

/// Convenience function for percussive onset detection
pub fn detect_percussive_onsets(audio: &[f32], sample_rate: f32) -> Vec<DetectedOnset> {
    OnsetDetector::new(sample_rate, OnsetDetectorConfig::percussive()).detect(audio)
}

/// Convenience function to get onset times in milliseconds
pub fn get_onset_times_ms(audio: &[f32], sample_rate: f32) -> Vec<f64> {
    detect_onsets(audio, sample_rate)
        .iter()
        .map(|o| o.time_ms)
        .collect()
}

/// Validate onset timing against expected pattern events (convenience function)
pub fn validate_onset_timing(
    audio: &[f32],
    sample_rate: f32,
    expected_times_ms: &[f64],
    tolerance_ms: f64,
) -> TimingValidationResult {
    let detector = OnsetDetector::with_defaults(sample_rate);
    let onsets = detector.detect(audio);
    let validator = TimingValidator::new(tolerance_ms);
    validator.validate(expected_times_ms, &onsets)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generate a click at a specific time
    fn generate_click(samples: &mut [f32], time_sec: f64, sample_rate: f32, amplitude: f32) {
        let sample_idx = (time_sec * sample_rate as f64) as usize;
        let click_duration = (sample_rate * 0.005) as usize; // 5ms click

        for i in 0..click_duration {
            if sample_idx + i < samples.len() {
                // Exponential decay click
                let decay = (-50.0 * i as f32 / click_duration as f32).exp();
                samples[sample_idx + i] = amplitude * decay;
            }
        }
    }

    /// Generate a drum-like transient
    fn generate_drum_hit(samples: &mut [f32], time_sec: f64, sample_rate: f32, amplitude: f32) {
        let sample_idx = (time_sec * sample_rate as f64) as usize;
        let attack = (sample_rate * 0.001) as usize; // 1ms attack
        let decay = (sample_rate * 0.050) as usize; // 50ms decay

        // Attack phase
        for i in 0..attack {
            if sample_idx + i < samples.len() {
                let env = i as f32 / attack as f32;
                let freq = 100.0 + 200.0 * (1.0 - env); // Pitch drop
                let phase = 2.0 * PI * freq * i as f32 / sample_rate;
                samples[sample_idx + i] += amplitude * env * phase.sin();
            }
        }

        // Decay phase
        for i in 0..decay {
            if sample_idx + attack + i < samples.len() {
                let env = (-8.0 * i as f32 / decay as f32).exp();
                let freq = 80.0;
                let phase = 2.0 * PI * freq * (attack + i) as f32 / sample_rate;
                samples[sample_idx + attack + i] += amplitude * env * phase.sin();
            }
        }
    }

    #[test]
    fn test_detect_single_click() {
        let sample_rate = 44100.0;
        let duration = 1.0;
        let mut audio = vec![0.0f32; (sample_rate * duration) as usize];

        // Place a click at 0.5 seconds
        generate_click(&mut audio, 0.5, sample_rate, 0.8);

        let detector = OnsetDetector::with_defaults(sample_rate);
        let onsets = detector.detect(&audio);

        assert!(!onsets.is_empty(), "Should detect at least one onset");

        // Check timing is close to 0.5 seconds (within 50ms)
        let closest = onsets
            .iter()
            .min_by_key(|o| ((o.time_sec - 0.5).abs() * 1000.0) as i64)
            .unwrap();
        assert!(
            (closest.time_sec - 0.5).abs() < 0.05,
            "Onset should be near 0.5s, got {}s",
            closest.time_sec
        );
    }

    #[test]
    fn test_detect_regular_pattern() {
        let sample_rate = 44100.0;
        let duration = 2.0;
        let mut audio = vec![0.0f32; (sample_rate * duration) as usize];

        // Place clicks at 0.0, 0.5, 1.0, 1.5 seconds
        let expected_times = [0.0, 0.5, 1.0, 1.5];
        for &t in &expected_times {
            generate_drum_hit(&mut audio, t, sample_rate, 0.8);
        }

        let detector = OnsetDetector::new(sample_rate, OnsetDetectorConfig::percussive());
        let onsets = detector.detect(&audio);

        assert!(
            onsets.len() >= 3,
            "Should detect at least 3 onsets (allowing for edge effects), got {}",
            onsets.len()
        );
    }

    #[test]
    fn test_timing_validator_perfect_match() {
        let expected_ms = vec![0.0, 500.0, 1000.0, 1500.0];
        let detected_ms = vec![5.0, 502.0, 998.0, 1503.0];

        let validator = TimingValidator::new(50.0);
        let result = validator.validate_times(&expected_ms, &detected_ms);

        assert_eq!(result.matched, 4, "All onsets should match");
        assert!(result.match_rate > 0.99, "Match rate should be ~100%");
        assert!(
            result.mean_deviation_ms < 10.0,
            "Mean deviation should be small"
        );
    }

    #[test]
    fn test_timing_validator_missing_onset() {
        let expected_ms = vec![0.0, 500.0, 1000.0, 1500.0];
        let detected_ms = vec![5.0, 502.0, 1503.0]; // Missing 1000.0

        let validator = TimingValidator::new(50.0);
        let result = validator.validate_times(&expected_ms, &detected_ms);

        assert_eq!(result.matched, 3);
        assert_eq!(result.missing.len(), 1);
        assert!(
            (result.missing[0] - 1000.0).abs() < 1.0,
            "Should identify 1000ms as missing"
        );
        assert!((result.match_rate - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_timing_validator_extra_onset() {
        let expected_ms = vec![0.0, 500.0, 1000.0];
        let detected_ms = vec![5.0, 300.0, 502.0, 998.0]; // Extra at 300.0

        let validator = TimingValidator::new(50.0);
        let result = validator.validate_times(&expected_ms, &detected_ms);

        assert_eq!(result.matched, 3, "All expected should match");
        assert_eq!(result.extra.len(), 1, "Should identify one extra");
        assert!(
            (result.extra[0] - 300.0).abs() < 1.0,
            "Extra onset should be at ~300ms"
        );
    }

    #[test]
    fn test_timing_validator_jitter_calculation() {
        let expected_ms = vec![0.0, 500.0, 1000.0, 1500.0];
        // Deliberately varied deviations: 5, 10, 15, 20
        let detected_ms = vec![5.0, 510.0, 1015.0, 1520.0];

        let validator = TimingValidator::new(50.0);
        let result = validator.validate_times(&expected_ms, &detected_ms);

        // Mean should be 12.5
        assert!(
            (result.mean_deviation_ms - 12.5).abs() < 0.1,
            "Mean deviation should be 12.5, got {}",
            result.mean_deviation_ms
        );

        // Max should be 20
        assert!(
            (result.max_deviation_ms - 20.0).abs() < 0.1,
            "Max deviation should be 20, got {}",
            result.max_deviation_ms
        );

        // Jitter should be calculated
        assert!(result.jitter_ms > 0.0, "Jitter should be positive");
    }

    #[test]
    fn test_is_acceptable() {
        let expected_ms = vec![0.0, 500.0, 1000.0, 1500.0];
        let detected_ms = vec![5.0, 502.0, 998.0]; // 75% match

        let validator = TimingValidator::new(50.0);
        let result = validator.validate_times(&expected_ms, &detected_ms);

        assert!(result.is_acceptable(0.7), "Should pass 70% threshold");
        assert!(!result.is_acceptable(0.9), "Should fail 90% threshold");
    }

    #[test]
    fn test_is_accurate() {
        let expected_ms = vec![0.0, 500.0, 1000.0, 1500.0];
        let detected_ms = vec![2.0, 501.0, 999.0, 1501.0]; // Very accurate

        let validator = TimingValidator::new(50.0);
        let result = validator.validate_times(&expected_ms, &detected_ms);

        assert!(
            result.is_accurate(5.0, 5.0),
            "Should be accurate with <5ms mean deviation and jitter"
        );
    }

    #[test]
    fn test_hfc_mode() {
        let sample_rate = 44100.0;
        let duration = 2.0;
        let mut audio = vec![0.0f32; (sample_rate * duration) as usize];

        // Drum hits should be detected well by HFC
        generate_drum_hit(&mut audio, 0.0, sample_rate, 0.8);
        generate_drum_hit(&mut audio, 0.5, sample_rate, 0.8);
        generate_drum_hit(&mut audio, 1.0, sample_rate, 0.8);

        let config = OnsetDetectorConfig {
            mode: OnsetDetectionMode::HighFrequencyContent,
            ..OnsetDetectorConfig::percussive()
        };
        let detector = OnsetDetector::new(sample_rate, config);
        let onsets = detector.detect(&audio);

        assert!(
            onsets.len() >= 2,
            "HFC should detect drum hits, got {}",
            onsets.len()
        );
    }

    #[test]
    fn test_combined_mode() {
        let sample_rate = 44100.0;
        let duration = 2.0;
        let mut audio = vec![0.0f32; (sample_rate * duration) as usize];

        generate_drum_hit(&mut audio, 0.25, sample_rate, 0.8);
        generate_drum_hit(&mut audio, 0.75, sample_rate, 0.8);
        generate_drum_hit(&mut audio, 1.25, sample_rate, 0.8);

        let config = OnsetDetectorConfig {
            mode: OnsetDetectionMode::Combined,
            ..OnsetDetectorConfig::default()
        };
        let detector = OnsetDetector::new(sample_rate, config);
        let onsets = detector.detect(&audio);

        assert!(
            onsets.len() >= 2,
            "Combined mode should detect onsets, got {}",
            onsets.len()
        );
    }

    #[test]
    fn test_frame_resolution() {
        let sample_rate = 44100.0;
        let config = OnsetDetectorConfig::default();
        let detector = OnsetDetector::new(sample_rate, config.clone());

        let expected_resolution = config.hop_size as f64 / sample_rate as f64 * 1000.0;
        assert!(
            (detector.frame_resolution_ms() - expected_resolution).abs() < 0.01,
            "Frame resolution should be ~{}ms",
            expected_resolution
        );
    }

    #[test]
    fn test_convenience_functions() {
        let sample_rate = 44100.0;
        let duration = 1.0;
        let mut audio = vec![0.0f32; (sample_rate * duration) as usize];

        generate_drum_hit(&mut audio, 0.25, sample_rate, 0.8);
        generate_drum_hit(&mut audio, 0.75, sample_rate, 0.8);

        // Test detect_onsets
        let onsets = detect_onsets(&audio, sample_rate);
        assert!(!onsets.is_empty(), "detect_onsets should work");

        // Test detect_percussive_onsets
        let perc_onsets = detect_percussive_onsets(&audio, sample_rate);
        assert!(
            !perc_onsets.is_empty(),
            "detect_percussive_onsets should work"
        );

        // Test get_onset_times_ms
        let times = get_onset_times_ms(&audio, sample_rate);
        assert!(!times.is_empty(), "get_onset_times_ms should work");
    }

    #[test]
    fn test_validate_onset_timing_function() {
        let sample_rate = 44100.0;
        let duration = 2.0;
        let mut audio = vec![0.0f32; (sample_rate * duration) as usize];

        let expected_times_ms = vec![250.0, 750.0, 1250.0, 1750.0];
        for &t in &expected_times_ms {
            generate_drum_hit(&mut audio, t / 1000.0, sample_rate, 0.8);
        }

        let result = validate_onset_timing(&audio, sample_rate, &expected_times_ms, 100.0);

        assert!(
            result.match_rate > 0.5,
            "Should match most expected onsets, got {}%",
            result.match_rate * 100.0
        );
    }

    #[test]
    fn test_empty_audio() {
        let audio: Vec<f32> = vec![];
        let detector = OnsetDetector::with_defaults(44100.0);
        let onsets = detector.detect(&audio);
        assert!(onsets.is_empty(), "Empty audio should have no onsets");
    }

    #[test]
    fn test_silent_audio() {
        let audio = vec![0.0f32; 44100];
        let detector = OnsetDetector::with_defaults(44100.0);
        let onsets = detector.detect(&audio);
        assert!(onsets.is_empty(), "Silent audio should have no onsets");
    }

    #[test]
    fn test_dense_config() {
        let sample_rate = 44100.0;
        let duration = 1.0;
        let mut audio = vec![0.0f32; (sample_rate * duration) as usize];

        // Dense pattern: clicks every 50ms
        for i in 0..20 {
            let t = i as f64 * 0.05;
            generate_click(&mut audio, t, sample_rate, 0.6);
        }

        let detector = OnsetDetector::new(sample_rate, OnsetDetectorConfig::dense());
        let onsets = detector.detect(&audio);

        // Should detect many of the dense clicks
        assert!(
            onsets.len() >= 10,
            "Dense config should detect many fast events, got {}",
            onsets.len()
        );
    }
}
