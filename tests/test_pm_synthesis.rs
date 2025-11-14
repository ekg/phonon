/// Systematic tests: Phase Modulation (PM) synthesis
///
/// Tests PM oscillator with spectral analysis and audio quality verification.
/// PM differs from FM by using external modulation source directly.
///
/// Spectral characteristics:
/// - PM produces sidebands like FM
/// - Sideband spacing = modulator frequency
/// - Sideband amplitude controlled by modulation index
/// - Can use any waveform as modulator (not just sine)

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use std::f32::consts::PI;

mod audio_test_utils;
use audio_test_utils::calculate_rms;

/// Helper function to render DSL code to audio buffer
fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph = compile_program(statements, sample_rate).expect("Failed to compile DSL code");
    let num_samples = (duration * sample_rate) as usize;
    graph.render(num_samples)
}

/// Perform FFT and analyze spectrum
/// Returns (frequency_bins, magnitudes)
fn analyze_spectrum(buffer: &[f32], sample_rate: f32) -> (Vec<f32>, Vec<f32>) {
    use rustfft::{FftPlanner, num_complex::Complex};

    let fft_size = 8192.min(buffer.len());
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);

    // Prepare input with Hann window
    let mut input: Vec<Complex<f32>> = buffer[..fft_size]
        .iter()
        .enumerate()
        .map(|(i, &sample)| {
            let window = 0.5 * (1.0 - (2.0 * PI * i as f32 / fft_size as f32).cos());
            Complex::new(sample * window, 0.0)
        })
        .collect();

    fft.process(&mut input);

    // Calculate magnitudes and frequencies
    let magnitudes: Vec<f32> = input[..fft_size / 2]
        .iter()
        .map(|c| (c.re * c.re + c.im * c.im).sqrt())
        .collect();

    let frequencies: Vec<f32> = (0..fft_size / 2)
        .map(|i| i as f32 * sample_rate / fft_size as f32)
        .collect();

    (frequencies, magnitudes)
}

/// Find peaks in spectrum above threshold
/// Returns (frequency, magnitude) pairs
fn find_spectral_peaks(frequencies: &[f32], magnitudes: &[f32], threshold: f32) -> Vec<(f32, f32)> {
    let mut peaks = Vec::new();

    for i in 1..magnitudes.len() - 1 {
        if magnitudes[i] > threshold
            && magnitudes[i] > magnitudes[i - 1]
            && magnitudes[i] > magnitudes[i + 1]
        {
            peaks.push((frequencies[i], magnitudes[i]));
        }
    }

    peaks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    peaks
}

// ========== Basic PM Tests ==========

#[test]
fn test_pm_constant_parameters() {
    let code = r#"
        tempo: 2.0
        ~mod: sine 5
        o1: pm 440 ~mod 2.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.1, "PM with constant params should produce audio, got RMS: {}", rms);
}

#[test]
fn test_pm_pattern_carrier_frequency() {
    // PM with pattern-modulated carrier frequency
    let code = r#"
        tempo: 2.0
        ~mod: sine 5
        o1: pm (sine 1.0 * 110 + 440) ~mod 2.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.1, "PM with pattern carrier frequency should produce audio, got RMS: {}", rms);
}

#[test]
fn test_pm_pattern_modulation_index() {
    // PM with pattern-modulated index
    let code = r#"
        tempo: 2.0
        ~mod: sine 5
        o1: pm 440 ~mod (sine 0.5 * 3.0 + 2.0)
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.1, "PM with pattern mod_index should produce audio, got RMS: {}", rms);
}

#[test]
fn test_pm_all_patterns() {
    // PM with all parameters as patterns
    let code = r#"
        tempo: 2.0
        ~mod: sine 5
        o1: pm (sine 0.5 * 220 + 440) ~mod (sine 1.0 * 2.0 + 2.0)
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.1, "PM with all pattern params should produce audio, got RMS: {}", rms);
}

// ========== Different Modulator Sources ==========

#[test]
fn test_pm_sine_modulator() {
    // PM with sine wave modulator
    let code = r#"
        tempo: 2.0
        ~mod: sine 100
        o1: pm 440 ~mod 3.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.1, "PM with sine modulator should produce audio, got RMS: {}", rms);
}

#[test]
fn test_pm_saw_modulator() {
    // PM with saw wave modulator (brighter, more harmonics)
    let code = r#"
        tempo: 2.0
        ~mod: saw 100
        o1: pm 440 ~mod 1.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.1, "PM with saw modulator should produce audio, got RMS: {}", rms);
}

#[test]
fn test_pm_square_modulator() {
    // PM with square wave modulator (odd harmonics)
    let code = r#"
        tempo: 2.0
        ~mod: square 100
        o1: pm 440 ~mod 2.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.1, "PM with square modulator should produce audio, got RMS: {}", rms);
}

#[test]
fn test_pm_noise_modulator() {
    // PM with noise modulator (randomized phase shifts)
    let code = r#"
        tempo: 2.0
        ~mod: noise
        o1: pm 440 ~mod 0.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.1, "PM with noise modulator should produce audio, got RMS: {}", rms);
}

// ========== Spectral Analysis Tests ==========

#[test]
fn test_pm_produces_sidebands() {
    // PM should create sidebands around carrier frequency
    // Carrier = 440 Hz, Modulator = 100 Hz
    // Expected peaks: 440±100, 440±200, 440±300, etc.
    let code = r#"
        tempo: 2.0
        ~mod: sine 100
        o1: pm 440 ~mod 2.0
    "#;

    let buffer = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    // Find peaks
    let max_magnitude = magnitudes.iter().cloned().fold(0.0f32, f32::max);
    let threshold = max_magnitude * 0.1; // 10% of max
    let peaks = find_spectral_peaks(&frequencies, &magnitudes, threshold);

    // Should have multiple peaks (carrier + sidebands)
    assert!(peaks.len() >= 3, "PM should produce carrier and sidebands, found {} peaks", peaks.len());

    // Verify carrier frequency is present (440 Hz ± 10 Hz tolerance)
    let has_carrier = peaks.iter().any(|(f, _)| (*f - 440.0).abs() < 10.0);
    assert!(has_carrier, "PM should have carrier frequency at 440 Hz, peaks: {:?}",
        peaks.iter().take(5).collect::<Vec<_>>());

    println!("PM spectral peaks (top 10): {:?}", peaks.iter().take(10).collect::<Vec<_>>());
}

#[test]
fn test_pm_modulation_index_affects_sidebands() {
    // Higher modulation index = more/stronger sidebands
    let low_index_code = r#"
        tempo: 2.0
        ~mod: sine 100
        o1: pm 440 ~mod 0.5
    "#;

    let high_index_code = r#"
        tempo: 2.0
        ~mod: sine 100
        o1: pm 440 ~mod 5.0
    "#;

    let low_buffer = render_dsl(low_index_code, 1.0);
    let high_buffer = render_dsl(high_index_code, 1.0);

    let (low_frequencies, low_magnitudes) = analyze_spectrum(&low_buffer, 44100.0);
    let (high_frequencies, high_magnitudes) = analyze_spectrum(&high_buffer, 44100.0);

    let low_max = low_magnitudes.iter().cloned().fold(0.0f32, f32::max);
    let high_max = high_magnitudes.iter().cloned().fold(0.0f32, f32::max);

    let low_peaks = find_spectral_peaks(&low_frequencies, &low_magnitudes, low_max * 0.1);
    let high_peaks = find_spectral_peaks(&high_frequencies, &high_magnitudes, high_max * 0.1);

    // Higher index should produce more sidebands
    assert!(high_peaks.len() > low_peaks.len(),
        "Higher mod_index should produce more sidebands: low={}, high={}",
        low_peaks.len(), high_peaks.len());

    println!("Low index peaks: {}, High index peaks: {}", low_peaks.len(), high_peaks.len());
}

#[test]
fn test_pm_vs_sine_spectral_difference() {
    // PM should have richer spectrum than pure sine
    let sine_code = r#"
        tempo: 2.0
        o1: sine 440
    "#;

    let pm_code = r#"
        tempo: 2.0
        ~mod: sine 100
        o1: pm 440 ~mod 3.0
    "#;

    let sine_buffer = render_dsl(sine_code, 1.0);
    let pm_buffer = render_dsl(pm_code, 1.0);

    let (sine_frequencies, sine_magnitudes) = analyze_spectrum(&sine_buffer, 44100.0);
    let (pm_frequencies, pm_magnitudes) = analyze_spectrum(&pm_buffer, 44100.0);

    let sine_max = sine_magnitudes.iter().cloned().fold(0.0f32, f32::max);
    let pm_max = pm_magnitudes.iter().cloned().fold(0.0f32, f32::max);

    let sine_peaks = find_spectral_peaks(&sine_frequencies, &sine_magnitudes, sine_max * 0.1);
    let pm_peaks = find_spectral_peaks(&pm_frequencies, &pm_magnitudes, pm_max * 0.1);

    // Sine should have ~1 peak, PM should have many
    assert_eq!(sine_peaks.len(), 1, "Pure sine should have 1 spectral peak");
    assert!(pm_peaks.len() >= 5, "PM should have multiple sidebands, found {}", pm_peaks.len());

    println!("Sine peaks: {}, PM peaks: {}", sine_peaks.len(), pm_peaks.len());
}

// ========== Audio Quality Tests ==========

#[test]
fn test_pm_no_dc_offset() {
    // PM output should be centered around zero (no DC offset)
    let code = r#"
        tempo: 2.0
        ~mod: sine 100
        o1: pm 440 ~mod 2.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let dc_offset: f32 = buffer.iter().sum::<f32>() / buffer.len() as f32;

    assert!(dc_offset.abs() < 0.01, "PM should have no DC offset, got {}", dc_offset);
}

#[test]
fn test_pm_no_clipping() {
    // PM output should not clip (stay within -1.0 to 1.0)
    let code = r#"
        tempo: 2.0
        ~mod: sine 100
        o1: pm 440 ~mod 5.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let max_amplitude = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    assert!(max_amplitude <= 1.0, "PM should not clip, max amplitude: {}", max_amplitude);
}

#[test]
fn test_pm_continuous_output() {
    // PM should produce continuous output without silence gaps
    let code = r#"
        tempo: 2.0
        ~mod: sine 100
        o1: pm 440 ~mod 2.0
    "#;

    let buffer = render_dsl(code, 2.0);

    // Check for consecutive zero samples (would indicate gaps)
    let mut max_zero_run = 0;
    let mut current_zero_run = 0;

    for sample in buffer.iter() {
        if sample.abs() < 0.001 {
            current_zero_run += 1;
            max_zero_run = max_zero_run.max(current_zero_run);
        } else {
            current_zero_run = 0;
        }
    }

    // Allow up to 10 consecutive near-zero samples (phase crossings)
    assert!(max_zero_run < 10, "PM should not have silence gaps, max zero run: {}", max_zero_run);
}
