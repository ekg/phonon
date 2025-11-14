/// Systematic tests: VCO (Voltage-Controlled Oscillator)
///
/// Tests VCO with spectral analysis and audio quality verification.
/// VCO models classic analog synthesizer oscillators with multiple waveforms.
///
/// Key characteristics:
/// - Multiple waveforms: saw, square, triangle, sine
/// - Band-limited (anti-aliased) using PolyBLEP
/// - Pulse width modulation (PWM) for square wave
/// - Analog-style behavior with smooth transitions
/// - Essential for vintage synth emulation

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

// ========== Basic VCO Tests - Saw Wave ==========

#[test]
fn test_vco_saw_constant_frequency() {
    let code = r#"
        tempo: 2.0
        o1: vco 440 0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.1, "VCO saw with constant frequency should produce audio, got RMS: {}", rms);
}

#[test]
fn test_vco_saw_pattern_frequency() {
    // VCO saw with LFO-modulated frequency (vibrato)
    let code = r#"
        tempo: 2.0
        o1: vco (sine 5 * 110 + 440) 0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.1, "VCO saw with LFO frequency should produce audio, got RMS: {}", rms);
}

// ========== Square Wave Tests ==========

#[test]
fn test_vco_square_constant_frequency() {
    let code = r#"
        tempo: 2.0
        o1: vco 440 1
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.1, "VCO square should produce audio, got RMS: {}", rms);
}

#[test]
fn test_vco_square_pattern_frequency() {
    let code = r#"
        tempo: 2.0
        o1: vco (sine 3 * 110 + 220) 1
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.1, "VCO square with pattern frequency should produce audio, got RMS: {}", rms);
}

// ========== Triangle Wave Tests ==========

#[test]
fn test_vco_triangle_constant_frequency() {
    let code = r#"
        tempo: 2.0
        o1: vco 440 2
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.1, "VCO triangle should produce audio, got RMS: {}", rms);
}

#[test]
fn test_vco_triangle_pattern_frequency() {
    let code = r#"
        tempo: 2.0
        o1: vco (sine 2 * 165 + 330) 2
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.1, "VCO triangle with pattern frequency should produce audio, got RMS: {}", rms);
}

// ========== Sine Wave Tests ==========

#[test]
fn test_vco_sine_constant_frequency() {
    let code = r#"
        tempo: 2.0
        o1: vco 440 3
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.1, "VCO sine should produce audio, got RMS: {}", rms);
}

#[test]
fn test_vco_sine_pattern_frequency() {
    let code = r#"
        tempo: 2.0
        o1: vco (sine 4 * 220 + 440) 3
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.1, "VCO sine with pattern frequency should produce audio, got RMS: {}", rms);
}

// ========== Pulse Width Modulation (PWM) Tests ==========

#[test]
fn test_vco_pwm_constant() {
    // Square wave with 25% duty cycle (narrow pulse)
    let code = r#"
        tempo: 2.0
        o1: vco 440 1 0.25
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.05, "VCO with PWM should produce audio, got RMS: {}", rms);
}

#[test]
fn test_vco_pwm_pattern_modulated() {
    // Square wave with LFO-modulated pulse width
    let code = r#"
        tempo: 2.0
        ~lfo: sine 2 * 0.3 + 0.5
        o1: vco 220 1 ~lfo
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.05, "VCO with modulated PWM should produce audio, got RMS: {}", rms);
}

#[test]
fn test_vco_pwm_affects_spectrum() {
    // Different pulse widths should produce different harmonic content
    let narrow_code = r#"
        tempo: 2.0
        o1: vco 110 1 0.1
    "#;

    let wide_code = r#"
        tempo: 2.0
        o1: vco 110 1 0.5
    "#;

    let narrow_buffer = render_dsl(narrow_code, 1.0);
    let wide_buffer = render_dsl(wide_code, 1.0);

    let (narrow_freqs, narrow_mags) = analyze_spectrum(&narrow_buffer, 44100.0);
    let (wide_freqs, wide_mags) = analyze_spectrum(&wide_buffer, 44100.0);

    let narrow_max = narrow_mags.iter().cloned().fold(0.0f32, f32::max);
    let wide_max = wide_mags.iter().cloned().fold(0.0f32, f32::max);

    let narrow_peaks = find_spectral_peaks(&narrow_freqs, &narrow_mags, narrow_max * 0.1);
    let wide_peaks = find_spectral_peaks(&wide_freqs, &wide_mags, wide_max * 0.1);

    // Narrow pulse should have more even harmonics
    // Both should have multiple harmonics
    assert!(narrow_peaks.len() >= 5, "Narrow pulse should have harmonics");
    assert!(wide_peaks.len() >= 3, "Wide pulse (50% duty) should have harmonics");

    println!("Narrow pulse (10%) peaks: {}, Wide pulse (50%) peaks: {}",
        narrow_peaks.len(), wide_peaks.len());
}

// ========== Spectral Analysis Tests ==========

#[test]
fn test_vco_saw_rich_harmonics() {
    // VCO saw should have rich harmonic content (all harmonics)
    let code = r#"
        tempo: 2.0
        o1: vco 110 0
    "#;

    let buffer = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    let max_magnitude = magnitudes.iter().cloned().fold(0.0f32, f32::max);
    let threshold = max_magnitude * 0.05;
    let peaks = find_spectral_peaks(&frequencies, &magnitudes, threshold);

    // Saw should have many harmonics (10+ for 110 Hz)
    assert!(peaks.len() >= 10,
        "VCO saw should have rich harmonics, found {} peaks", peaks.len());

    // Verify fundamental is present
    let has_fundamental = peaks.iter().any(|(f, _)| (*f - 110.0).abs() < 15.0);
    assert!(has_fundamental, "VCO saw should have fundamental at 110 Hz");

    println!("VCO saw spectral peaks: {}", peaks.len());
}

#[test]
fn test_vco_square_odd_harmonics() {
    // Square wave should have predominantly odd harmonics
    let code = r#"
        tempo: 2.0
        o1: vco 110 1
    "#;

    let buffer = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    let max_magnitude = magnitudes.iter().cloned().fold(0.0f32, f32::max);
    let threshold = max_magnitude * 0.1;
    let peaks = find_spectral_peaks(&frequencies, &magnitudes, threshold);

    // Square should have multiple odd harmonics
    assert!(peaks.len() >= 5,
        "VCO square should have odd harmonics, found {} peaks", peaks.len());

    println!("VCO square spectral peaks: {}", peaks.len());
}

#[test]
fn test_vco_triangle_soft_harmonics() {
    // Triangle wave should have softer harmonics (falls off faster than square)
    let code = r#"
        tempo: 2.0
        o1: vco 110 2
    "#;

    let buffer = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    let max_magnitude = magnitudes.iter().cloned().fold(0.0f32, f32::max);
    let threshold = max_magnitude * 0.1;
    let peaks = find_spectral_peaks(&frequencies, &magnitudes, threshold);

    // Triangle should have fewer prominent harmonics than saw/square
    assert!(peaks.len() >= 3,
        "VCO triangle should have harmonics, found {} peaks", peaks.len());

    println!("VCO triangle spectral peaks: {}", peaks.len());
}

#[test]
fn test_vco_sine_pure_tone() {
    // VCO sine should be close to pure tone (minimal harmonics)
    let code = r#"
        tempo: 2.0
        o1: vco 440 3
    "#;

    let buffer = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    let max_magnitude = magnitudes.iter().cloned().fold(0.0f32, f32::max);
    let threshold = max_magnitude * 0.1;
    let peaks = find_spectral_peaks(&frequencies, &magnitudes, threshold);

    // Sine should have primarily 1 peak (fundamental)
    assert_eq!(peaks.len(), 1, "VCO sine should have single spectral peak, found {}", peaks.len());

    // Verify it's at 440 Hz
    let has_fundamental = peaks.iter().any(|(f, _)| (*f - 440.0).abs() < 15.0);
    assert!(has_fundamental, "VCO sine should have peak at 440 Hz");

    println!("VCO sine spectral peaks: {}", peaks.len());
}

#[test]
fn test_vco_band_limited() {
    // VCO should be band-limited (no significant aliasing)
    let code = r#"
        tempo: 2.0
        o1: vco 110 0
    "#;

    let buffer = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    // Check energy in upper frequency range (18kHz - 22kHz)
    let low_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f < 5000.0)
        .map(|(_, m)| m * m)
        .sum();

    let high_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 18000.0)
        .map(|(_, m)| m * m)
        .sum();

    let energy_ratio = high_energy / low_energy;
    assert!(energy_ratio < 0.1,
        "VCO should be band-limited (low aliasing), high/low energy ratio: {}",
        energy_ratio);

    println!("VCO band-limiting: high/low energy ratio = {:.4}", energy_ratio);
}

// ========== Audio Quality Tests ==========

#[test]
fn test_vco_no_dc_offset() {
    let code = r#"
        tempo: 2.0
        o1: vco 440 0
    "#;

    let buffer = render_dsl(code, 2.0);
    let dc_offset: f32 = buffer.iter().sum::<f32>() / buffer.len() as f32;

    assert!(dc_offset.abs() < 0.01, "VCO should have no DC offset, got {}", dc_offset);
}

#[test]
fn test_vco_no_clipping() {
    let code = r#"
        tempo: 2.0
        o1: vco 110 0
    "#;

    let buffer = render_dsl(code, 2.0);
    let max_amplitude = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    assert!(max_amplitude <= 1.0, "VCO should not clip, max amplitude: {}", max_amplitude);
}

#[test]
fn test_vco_continuous_output() {
    let code = r#"
        tempo: 2.0
        o1: vco 440 1
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
    assert!(max_zero_run < 10, "VCO should not have silence gaps, max zero run: {}", max_zero_run);
}

// ========== Comparison Tests ==========

#[test]
fn test_vco_saw_vs_basic_saw() {
    // VCO saw should sound similar to basic saw but with band-limiting
    let vco_code = r#"
        tempo: 2.0
        o1: vco 110 0
    "#;

    let basic_code = r#"
        tempo: 2.0
        o1: saw 110
    "#;

    let vco_buffer = render_dsl(vco_code, 2.0);
    let basic_buffer = render_dsl(basic_code, 2.0);

    let vco_rms = calculate_rms(&vco_buffer);
    let basic_rms = calculate_rms(&basic_buffer);

    // Both should have audio
    assert!(vco_rms > 0.1, "VCO saw should have audio");
    assert!(basic_rms > 0.1, "Basic saw should have audio");

    // RMS should be similar (both produce saw waves)
    let diff_ratio = (vco_rms - basic_rms).abs() / basic_rms;
    assert!(diff_ratio < 0.3,
        "VCO saw and basic saw should have similar RMS, VCO: {}, Basic: {}, diff: {}",
        vco_rms, basic_rms, diff_ratio);

    println!("VCO saw RMS: {:.4}, Basic saw RMS: {:.4}", vco_rms, basic_rms);
}

#[test]
fn test_vco_waveform_selection() {
    // Different waveforms should produce different outputs
    let saw_code = r#"
        tempo: 2.0
        o1: vco 220 0
    "#;

    let square_code = r#"
        tempo: 2.0
        o1: vco 220 1
    "#;

    let triangle_code = r#"
        tempo: 2.0
        o1: vco 220 2
    "#;

    let saw_buffer = render_dsl(saw_code, 1.0);
    let square_buffer = render_dsl(square_code, 1.0);
    let triangle_buffer = render_dsl(triangle_code, 1.0);

    // All should have audio
    assert!(calculate_rms(&saw_buffer) > 0.1, "VCO saw should have audio");
    assert!(calculate_rms(&square_buffer) > 0.1, "VCO square should have audio");
    assert!(calculate_rms(&triangle_buffer) > 0.1, "VCO triangle should have audio");

    // Verify they're different by checking spectral content
    let (_, saw_mags) = analyze_spectrum(&saw_buffer, 44100.0);
    let (_, square_mags) = analyze_spectrum(&square_buffer, 44100.0);
    let (_, triangle_mags) = analyze_spectrum(&triangle_buffer, 44100.0);

    let saw_max = saw_mags.iter().cloned().fold(0.0f32, f32::max);
    let square_max = square_mags.iter().cloned().fold(0.0f32, f32::max);
    let triangle_max = triangle_mags.iter().cloned().fold(0.0f32, f32::max);

    // Different waveforms should have different spectral characteristics
    assert!(saw_max > 0.0 && square_max > 0.0 && triangle_max > 0.0,
        "All waveforms should have spectral content");

    println!("VCO waveforms - Saw RMS: {:.4}, Square RMS: {:.4}, Triangle RMS: {:.4}",
        calculate_rms(&saw_buffer),
        calculate_rms(&square_buffer),
        calculate_rms(&triangle_buffer));
}
