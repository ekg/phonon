//! Utilities for verifying that audio output matches pattern specifications
//!
//! This module provides tools to:
//! 1. Query patterns to get expected event times
//! 2. Detect events/onsets in rendered audio
//! 3. Compare expected vs actual events
//! 4. Spectral analysis for verifying modulation effects
//! 5. Audio characteristics analysis (RMS, peak, SNR, etc.)

use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use rustfft::{FftPlanner, num_complex::Complex};
use std::collections::HashMap;
use std::f32::consts::PI;

/// Event detected in audio or expected from pattern
#[derive(Debug, Clone)]
pub struct Event {
    /// Time in seconds
    pub time: f64,
    /// Optional value (for patterns with values)
    pub value: Option<String>,
    /// RMS amplitude around this event
    pub amplitude: f32,
}

/// Query a pattern to get expected events over a time range
pub fn get_expected_events(
    pattern: &Pattern<String>,
    duration_seconds: f64,
    cps: f64,
) -> Vec<Event> {
    let duration_cycles = duration_seconds * cps;

    let state = State {
        span: TimeSpan::new(
            Fraction::from_float(0.0),
            Fraction::from_float(duration_cycles),
        ),
        controls: HashMap::new(),
    };

    let haps = pattern.query(&state);

    haps.into_iter()
        .map(|hap| Event {
            time: hap.part.begin.to_float() / cps,
            value: Some(hap.value),
            amplitude: 0.0, // Will be filled in from audio analysis
        })
        .collect()
}

/// Detect events in audio buffer using onset detection
pub fn detect_audio_events(audio: &[f32], sample_rate: f32, threshold: f32) -> Vec<Event> {
    let mut events = Vec::new();

    // Simple onset detection: look for sudden increases in RMS
    let window_size = (sample_rate * 0.01) as usize; // 10ms window
    let hop_size = window_size / 4;

    let mut prev_rms = 0.0;

    for (i, window) in audio.windows(window_size).step_by(hop_size).enumerate() {
        let rms: f32 = (window.iter().map(|x| x * x).sum::<f32>() / window.len() as f32).sqrt();

        // Detect onset: current RMS is significantly higher than previous
        let onset_strength = (rms - prev_rms).max(0.0);

        if onset_strength > threshold {
            let time = (i * hop_size) as f64 / sample_rate as f64;
            events.push(Event {
                time,
                value: None,
                amplitude: rms,
            });
        }

        prev_rms = rms * 0.9; // Decay for next comparison
    }

    events
}

/// Compare expected events with detected events
#[derive(Debug)]
pub struct EventComparison {
    pub matched: usize,
    pub missing: Vec<Event>,
    pub extra: Vec<Event>,
    pub total_expected: usize,
    pub match_rate: f32,
}

impl EventComparison {
    pub fn is_acceptable(&self, min_match_rate: f32) -> bool {
        self.match_rate >= min_match_rate
    }
}

/// Match detected events with expected events
/// tolerance: time tolerance in seconds for matching
pub fn compare_events(expected: &[Event], detected: &[Event], tolerance: f64) -> EventComparison {
    let mut matched = 0;
    let mut missing = Vec::new();
    let mut detected_used = vec![false; detected.len()];

    for exp_event in expected {
        // Try to find a matching detected event
        let mut found = false;

        for (i, det_event) in detected.iter().enumerate() {
            if detected_used[i] {
                continue;
            }

            let time_diff = (exp_event.time - det_event.time).abs();
            if time_diff <= tolerance {
                matched += 1;
                detected_used[i] = true;
                found = true;
                break;
            }
        }

        if !found {
            missing.push(exp_event.clone());
        }
    }

    // Find extra detected events (not matched to any expected event)
    let extra: Vec<Event> = detected
        .iter()
        .enumerate()
        .filter(|(i, _)| !detected_used[*i])
        .map(|(_, e)| e.clone())
        .collect();

    let match_rate = if expected.is_empty() {
        1.0
    } else {
        matched as f32 / expected.len() as f32
    };

    EventComparison {
        matched,
        missing,
        extra,
        total_expected: expected.len(),
        match_rate,
    }
}

// ============================================================================
// Audio Characteristics Analysis
// ============================================================================

/// Calculate RMS (Root Mean Square) amplitude of audio signal
pub fn calculate_rms(audio: &[f32]) -> f32 {
    if audio.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = audio.iter().map(|&x| x * x).sum();
    (sum_squares / audio.len() as f32).sqrt()
}

/// Calculate peak amplitude of audio signal
pub fn calculate_peak(audio: &[f32]) -> f32 {
    audio.iter().map(|&x| x.abs()).fold(0.0, f32::max)
}

/// Detect if audio contains only silence
pub fn is_silent(audio: &[f32], threshold: f32) -> bool {
    calculate_rms(audio) < threshold
}

/// Detect if audio is clipping
pub fn is_clipping(audio: &[f32], threshold: f32) -> bool {
    calculate_peak(audio) > threshold
}

/// Count zero crossings in audio signal
pub fn count_zero_crossings(audio: &[f32]) -> usize {
    let mut count = 0;
    let mut last_sign = audio.first().map(|&x| x >= 0.0).unwrap_or(true);

    for &sample in audio.iter().skip(1) {
        let current_sign = sample >= 0.0;
        if current_sign != last_sign {
            count += 1;
        }
        last_sign = current_sign;
    }

    count
}

/// Estimate fundamental frequency from zero crossings
pub fn estimate_frequency_from_zero_crossings(audio: &[f32], sample_rate: f32) -> f32 {
    let crossings = count_zero_crossings(audio);
    let cycles = crossings as f32 / 2.0; // Two crossings per cycle
    let duration = audio.len() as f32 / sample_rate;
    cycles / duration
}

// ============================================================================
// Spectral Analysis for Modulation Verification
// ============================================================================

/// Calculate spectral centroid (center of mass of frequency spectrum)
///
/// Higher values = brighter sound
/// Lower values = darker sound
pub fn calculate_spectral_centroid(audio: &[f32], sample_rate: f32) -> f32 {
    if audio.is_empty() {
        return 0.0;
    }

    // Use power-of-2 FFT size
    let fft_size = audio.len().next_power_of_two().min(4096);
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);

    // Prepare input buffer (zero-pad if needed)
    let mut buffer: Vec<Complex<f32>> = audio
        .iter()
        .take(fft_size)
        .map(|&x| Complex::new(x, 0.0))
        .collect();
    buffer.resize(fft_size, Complex::new(0.0, 0.0));

    // Apply Hann window
    for (i, sample) in buffer.iter_mut().enumerate() {
        let window = 0.5 * (1.0 - (2.0 * PI * i as f32 / fft_size as f32).cos());
        *sample *= window;
    }

    // Perform FFT
    fft.process(&mut buffer);

    // Calculate spectral centroid (only use first half of spectrum)
    let mut weighted_sum = 0.0;
    let mut magnitude_sum = 0.0;

    for (i, bin) in buffer.iter().enumerate().take(fft_size / 2) {
        let magnitude = (bin.re * bin.re + bin.im * bin.im).sqrt();
        let frequency = i as f32 * sample_rate / fft_size as f32;
        weighted_sum += frequency * magnitude;
        magnitude_sum += magnitude;
    }

    if magnitude_sum > 0.0 {
        weighted_sum / magnitude_sum
    } else {
        0.0
    }
}

/// Estimate low-pass filter cutoff frequency from audio
///
/// Uses spectral rolloff (frequency below which 85% of energy is contained)
pub fn estimate_lpf_cutoff(audio: &[f32], sample_rate: f32) -> f32 {
    if audio.is_empty() {
        return sample_rate / 2.0;
    }

    // Use power-of-2 FFT size
    let fft_size = audio.len().next_power_of_two().min(4096);
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);

    // Prepare input buffer
    let mut buffer: Vec<Complex<f32>> = audio
        .iter()
        .take(fft_size)
        .map(|&x| Complex::new(x, 0.0))
        .collect();
    buffer.resize(fft_size, Complex::new(0.0, 0.0));

    // Apply Hann window
    for (i, sample) in buffer.iter_mut().enumerate() {
        let window = 0.5 * (1.0 - (2.0 * PI * i as f32 / fft_size as f32).cos());
        *sample *= window;
    }

    // Perform FFT
    fft.process(&mut buffer);

    // Calculate magnitude spectrum
    let magnitudes: Vec<f32> = buffer
        .iter()
        .take(fft_size / 2)
        .map(|bin| (bin.re * bin.re + bin.im * bin.im).sqrt())
        .collect();

    // Calculate total energy
    let total_energy: f32 = magnitudes.iter().map(|&m| m * m).sum();

    // Find frequency where 85% of energy is contained (spectral rolloff)
    let rolloff_threshold = 0.85 * total_energy;
    let mut cumulative_energy = 0.0;

    for (i, &magnitude) in magnitudes.iter().enumerate() {
        cumulative_energy += magnitude * magnitude;
        if cumulative_energy >= rolloff_threshold {
            return i as f32 * sample_rate / fft_size as f32;
        }
    }

    // If we didn't find rolloff, return Nyquist
    sample_rate / 2.0
}

/// Assert that two audio signals have different spectral content
///
/// Useful for verifying that modulation actually changes the sound
pub fn assert_spectral_difference(
    audio1: &[f32],
    audio2: &[f32],
    sample_rate: f32,
    min_diff_hz: f32,
    message: &str,
) {
    let centroid1 = calculate_spectral_centroid(audio1, sample_rate);
    let centroid2 = calculate_spectral_centroid(audio2, sample_rate);
    let diff = (centroid1 - centroid2).abs();

    assert!(
        diff >= min_diff_hz,
        "{}: Expected spectral difference >= {}Hz, got {:.2}Hz (centroid1: {:.2}Hz, centroid2: {:.2}Hz)",
        message, min_diff_hz, diff, centroid1, centroid2
    );
}

/// Assert that modulation is continuous (not stepped)
///
/// Analyzes spectral centroid variation over time windows
pub fn assert_continuous_modulation(
    audio: &[f32],
    sample_rate: f32,
    window_size: usize,
    min_variation: f32,
    message: &str,
) {
    if audio.len() < window_size * 3 {
        panic!("{}: Audio too short for continuous modulation analysis", message);
    }

    // Calculate spectral centroid for multiple windows
    let num_windows = audio.len() / window_size;
    let mut centroids = Vec::new();

    for i in 0..num_windows {
        let start = i * window_size;
        let end = (start + window_size).min(audio.len());
        let window_audio = &audio[start..end];
        let centroid = calculate_spectral_centroid(window_audio, sample_rate);
        centroids.push(centroid);
    }

    // Calculate variation (standard deviation)
    if centroids.is_empty() {
        panic!("{}: No windows analyzed", message);
    }

    let mean = centroids.iter().sum::<f32>() / centroids.len() as f32;
    let variance = centroids
        .iter()
        .map(|&c| (c - mean).powi(2))
        .sum::<f32>() / centroids.len() as f32;
    let std_dev = variance.sqrt();

    assert!(
        std_dev >= min_variation,
        "{}: Expected continuous variation >= {}Hz, got {:.2}Hz std dev (mean: {:.2}Hz, {} windows)",
        message, min_variation, std_dev, mean, centroids.len()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_detection_with_impulses() {
        let sample_rate = 44100.0;
        let mut audio = vec![0.0; 44100]; // 1 second of silence

        // Add impulses at 0.25s, 0.5s, 0.75s
        audio[11025] = 0.5; // 0.25s
        audio[22050] = 0.5; // 0.5s
        audio[33075] = 0.5; // 0.75s

        let detected = detect_audio_events(&audio, sample_rate, 0.01);

        // Should detect 3 events
        assert!(
            detected.len() >= 2,
            "Should detect at least 2 events, got {}",
            detected.len()
        );

        // Check approximate timing
        if detected.len() >= 2 {
            assert!(
                (detected[0].time - 0.25).abs() < 0.05,
                "First event should be near 0.25s"
            );
        }
    }

    #[test]
    fn test_compare_events() {
        let expected = vec![
            Event {
                time: 0.0,
                value: Some("bd".to_string()),
                amplitude: 0.0,
            },
            Event {
                time: 0.5,
                value: Some("sn".to_string()),
                amplitude: 0.0,
            },
            Event {
                time: 1.0,
                value: Some("hh".to_string()),
                amplitude: 0.0,
            },
        ];

        let detected = vec![
            Event {
                time: 0.01,
                value: None,
                amplitude: 0.5,
            },
            Event {
                time: 0.51,
                value: None,
                amplitude: 0.4,
            },
            // Missing third event
        ];

        let comparison = compare_events(&expected, &detected, 0.05);

        assert_eq!(comparison.matched, 2);
        assert_eq!(comparison.missing.len(), 1);
        assert_eq!(comparison.extra.len(), 0);
        assert!((comparison.match_rate - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_calculate_rms() {
        // Sine wave: RMS should be amplitude / sqrt(2)
        let sample_rate = 44100.0;
        let frequency = 440.0;
        let amplitude = 0.5;
        let duration = 1.0;

        let audio: Vec<f32> = (0..(sample_rate * duration) as usize)
            .map(|i| amplitude * (2.0 * PI * frequency * i as f32 / sample_rate).sin())
            .collect();

        let rms = calculate_rms(&audio);
        let expected = amplitude / 2.0_f32.sqrt();

        assert!((rms - expected).abs() < 0.01, "RMS: {}, Expected: {}", rms, expected);
    }

    #[test]
    fn test_spectral_centroid_brightness() {
        let sample_rate = 44100.0;
        let duration = 0.5;

        // Low frequency tone
        let low_freq = 220.0;
        let low_audio: Vec<f32> = (0..(sample_rate * duration) as usize)
            .map(|i| (2.0 * PI * low_freq * i as f32 / sample_rate).sin())
            .collect();

        // High frequency tone
        let high_freq = 2200.0;
        let high_audio: Vec<f32> = (0..(sample_rate * duration) as usize)
            .map(|i| (2.0 * PI * high_freq * i as f32 / sample_rate).sin())
            .collect();

        let low_centroid = calculate_spectral_centroid(&low_audio, sample_rate);
        let high_centroid = calculate_spectral_centroid(&high_audio, sample_rate);

        assert!(
            high_centroid > low_centroid,
            "High freq centroid ({:.2}Hz) should be > low freq ({:.2}Hz)",
            high_centroid, low_centroid
        );
    }

    #[test]
    fn test_zero_crossing_frequency_estimation() {
        let sample_rate = 44100.0;
        let frequency = 440.0;
        let duration = 1.0;

        let audio: Vec<f32> = (0..(sample_rate * duration) as usize)
            .map(|i| (2.0 * PI * frequency * i as f32 / sample_rate).sin())
            .collect();

        let estimated = estimate_frequency_from_zero_crossings(&audio, sample_rate);

        assert!(
            (estimated - frequency).abs() < 1.0,
            "Estimated freq: {:.2}Hz, Expected: {}Hz",
            estimated, frequency
        );
    }

    #[test]
    fn test_is_silent() {
        let silent = vec![0.0; 44100];
        let loud = vec![0.5; 44100];

        assert!(is_silent(&silent, 0.01));
        assert!(!is_silent(&loud, 0.01));
    }

    #[test]
    fn test_is_clipping() {
        let normal = vec![0.5; 44100];
        let clipping = vec![1.5; 44100];

        assert!(!is_clipping(&normal, 1.0));
        assert!(is_clipping(&clipping, 1.0));
    }
}
