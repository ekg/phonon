//! Shared audio testing utilities for FFT-based signal analysis
//!
//! This module provides reusable functions for analyzing audio signals in tests,
//! particularly for verifying frequency-related DSP parameters.

use rustfft::{num_complex::Complex, FftPlanner};

/// Find the dominant frequency in an audio buffer using FFT
///
/// # Arguments
/// * `buffer` - Audio samples to analyze
/// * `sample_rate` - Sample rate in Hz
///
/// # Returns
/// The frequency (in Hz) with the highest magnitude in the spectrum
///
/// # Example
/// ```ignore
/// let buffer = graph.render(44100);
/// let freq = find_dominant_frequency(&buffer, 44100.0);
/// assert!((freq - 440.0).abs() < 5.0); // Verify it's A4
/// ```
pub fn find_dominant_frequency(buffer: &[f32], sample_rate: f32) -> f32 {
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(buffer.len());

    let mut complex_input: Vec<Complex<f32>> =
        buffer.iter().map(|&x| Complex { re: x, im: 0.0 }).collect();

    fft.process(&mut complex_input);

    // Find peak in FFT (skip DC bin)
    let magnitudes: Vec<f32> = complex_input[1..complex_input.len() / 2]
        .iter()
        .map(|c| (c.re * c.re + c.im * c.im).sqrt())
        .collect();

    let max_idx = magnitudes
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(i, _)| i)
        .unwrap_or(0);

    (max_idx + 1) as f32 * sample_rate / buffer.len() as f32
}

/// Compute the spectral centroid of an audio buffer using FFT
///
/// The spectral centroid is the "center of mass" of the spectrum,
/// calculated as the weighted mean of frequencies present in the signal.
///
/// # Arguments
/// * `buffer` - Audio samples to analyze
/// * `sample_rate` - Sample rate in Hz
///
/// # Returns
/// The spectral centroid frequency in Hz. Higher values indicate brighter sounds.
///
/// # Example
/// ```ignore
/// let lowpass_audio = render_lowpass_filter(cutoff_hz: 500.0);
/// let highpass_audio = render_highpass_filter(cutoff_hz: 5000.0);
///
/// let low_centroid = compute_spectral_centroid(&lowpass_audio, 44100.0);
/// let high_centroid = compute_spectral_centroid(&highpass_audio, 44100.0);
///
/// assert!(high_centroid > low_centroid * 2.0);
/// ```
pub fn compute_spectral_centroid(buffer: &[f32], sample_rate: f32) -> f32 {
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(buffer.len());

    let mut complex_input: Vec<Complex<f32>> =
        buffer.iter().map(|&x| Complex { re: x, im: 0.0 }).collect();

    fft.process(&mut complex_input);

    let mut weighted_sum = 0.0;
    let mut magnitude_sum = 0.0;

    for (i, c) in complex_input[1..complex_input.len() / 2].iter().enumerate() {
        let magnitude = (c.re * c.re + c.im * c.im).sqrt();
        let frequency = (i + 1) as f32 * sample_rate / buffer.len() as f32;
        weighted_sum += frequency * magnitude;
        magnitude_sum += magnitude;
    }

    if magnitude_sum > 0.0 {
        weighted_sum / magnitude_sum
    } else {
        0.0
    }
}

/// Measure the frequency spread (bandwidth) of an audio buffer using FFT
///
/// This function finds the frequency range that contains most of the signal energy.
/// Useful for verifying effects like detuning that spread energy across frequencies.
///
/// # Arguments
/// * `buffer` - Audio samples to analyze
/// * `sample_rate` - Sample rate in Hz
///
/// # Returns
/// The bandwidth in Hz (difference between highest and lowest significant frequencies)
///
/// # Example
/// ```ignore
/// let saw_normal = render_saw(detune: 0.0);
/// let saw_detuned = render_saw(detune: 0.5);
///
/// let spread_normal = measure_frequency_spread(&saw_normal, 44100.0);
/// let spread_detuned = measure_frequency_spread(&saw_detuned, 44100.0);
///
/// assert!(spread_detuned > spread_normal * 1.5);
/// ```
pub fn measure_frequency_spread(buffer: &[f32], sample_rate: f32) -> f32 {
    // Compute FFT
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(buffer.len());

    let mut complex_input: Vec<Complex<f32>> =
        buffer.iter().map(|&x| Complex { re: x, im: 0.0 }).collect();

    fft.process(&mut complex_input);

    // Find frequency range containing significant energy (above 5% of total)
    let magnitudes: Vec<f32> = complex_input[1..complex_input.len() / 2]
        .iter()
        .map(|c| c.re * c.re + c.im * c.im)
        .collect();

    let total_energy: f32 = magnitudes.iter().sum();
    let threshold = 0.05 * total_energy / magnitudes.len() as f32;

    let mut low_idx = 0;
    let mut high_idx = magnitudes.len() - 1;

    // Find lowest significant frequency
    for (i, &mag) in magnitudes.iter().enumerate() {
        if mag > threshold {
            low_idx = i;
            break;
        }
    }

    // Find highest significant frequency
    for (i, &mag) in magnitudes.iter().enumerate().rev() {
        if mag > threshold {
            high_idx = i;
            break;
        }
    }

    let low_freq = (low_idx + 1) as f32 * sample_rate / buffer.len() as f32;
    let high_freq = (high_idx + 1) as f32 * sample_rate / buffer.len() as f32;

    high_freq - low_freq
}

/// Find the top N frequency peaks in an audio buffer using FFT
///
/// Returns a list of (frequency, magnitude) pairs for the strongest peaks,
/// sorted by magnitude (strongest first).
///
/// # Arguments
/// * `buffer` - Audio samples to analyze
/// * `sample_rate` - Sample rate in Hz
/// * `num_peaks` - Number of peaks to return
///
/// # Returns
/// Vector of (frequency_hz, magnitude) tuples, sorted by magnitude descending
///
/// # Example
/// ```ignore
/// let chord = render_chord([261.63, 329.63, 392.00]); // C major chord
/// let peaks = find_frequency_peaks(&chord, 44100.0, 3);
///
/// assert!((peaks[0].0 - 261.63).abs() < 5.0); // C
/// assert!((peaks[1].0 - 329.63).abs() < 5.0); // E
/// assert!((peaks[2].0 - 392.00).abs() < 5.0); // G
/// ```
pub fn find_frequency_peaks(buffer: &[f32], sample_rate: f32, num_peaks: usize) -> Vec<(f32, f32)> {
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(buffer.len());

    let mut complex_input: Vec<Complex<f32>> =
        buffer.iter().map(|&x| Complex { re: x, im: 0.0 }).collect();

    fft.process(&mut complex_input);

    // Compute magnitudes and pair with frequencies
    let mut freq_mags: Vec<(f32, f32)> = complex_input[1..complex_input.len() / 2]
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let magnitude = (c.re * c.re + c.im * c.im).sqrt();
            let frequency = (i + 1) as f32 * sample_rate / buffer.len() as f32;
            (frequency, magnitude)
        })
        .collect();

    // Sort by magnitude descending
    freq_mags.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // Return top N peaks
    freq_mags.into_iter().take(num_peaks).collect()
}

/// Calculate RMS (Root Mean Square) amplitude of a buffer
///
/// While RMS is appropriate for amplitude-based parameters (gain, volume),
/// it should NOT be used to verify frequency-based parameters (pitch, detune, cutoff).
///
/// # Arguments
/// * `buffer` - Audio samples to analyze
///
/// # Returns
/// RMS amplitude value
pub fn calculate_rms(buffer: &[f32]) -> f32 {
    if buffer.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = buffer.iter().map(|x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Find the peak amplitude in a buffer
///
/// Useful for envelope and amplitude tests.
///
/// # Arguments
/// * `buffer` - Audio samples to analyze
///
/// # Returns
/// Peak absolute amplitude
pub fn find_peak(buffer: &[f32]) -> f32 {
    buffer.iter().map(|x| x.abs()).fold(0.0, f32::max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    /// Generate a sine wave for testing
    fn generate_sine(freq: f32, sample_rate: f32, duration: f32) -> Vec<f32> {
        let num_samples = (sample_rate * duration) as usize;
        (0..num_samples)
            .map(|i| {
                let t = i as f32 / sample_rate;
                (2.0 * PI * freq * t).sin()
            })
            .collect()
    }

    #[test]
    fn test_find_dominant_frequency_440hz() {
        let buffer = generate_sine(440.0, 44100.0, 0.5);
        let detected = find_dominant_frequency(&buffer, 44100.0);
        assert!(
            (detected - 440.0).abs() < 5.0,
            "Expected 440Hz, got {}Hz",
            detected
        );
    }

    #[test]
    fn test_spectral_centroid_sine() {
        let buffer = generate_sine(1000.0, 44100.0, 0.5);
        let centroid = compute_spectral_centroid(&buffer, 44100.0);
        // For a pure sine wave, centroid should be near the fundamental
        assert!(
            (centroid - 1000.0).abs() < 100.0,
            "Expected centroid near 1000Hz, got {}Hz",
            centroid
        );
    }

    #[test]
    fn test_rms_sine() {
        let buffer = generate_sine(440.0, 44100.0, 1.0);
        let rms = calculate_rms(&buffer);
        // Sine wave RMS should be amplitude/sqrt(2) ≈ 0.707
        assert!(
            (rms - 0.707).abs() < 0.01,
            "Expected RMS ~0.707, got {}",
            rms
        );
    }

    #[test]
    fn test_peak_sine() {
        let buffer = generate_sine(440.0, 44100.0, 1.0);
        let peak = find_peak(&buffer);
        // Peak of unit sine wave should be ~1.0
        assert!(
            (peak - 1.0).abs() < 0.01,
            "Expected peak ~1.0, got {}",
            peak
        );
    }
}
