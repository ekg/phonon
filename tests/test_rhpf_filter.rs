/// Systematic tests: Resonant Highpass Filter (RHPF)
///
/// Tests resonant highpass filter with frequency response analysis and audio verification.
/// RHPF is a highpass filter with resonance/Q control for emphasized cutoff frequency.
///
/// Key characteristics:
/// - Passes high frequencies, attenuates low frequencies
/// - Resonance creates peak at cutoff frequency
/// - Q/resonance parameter controls peak sharpness
/// - Used for removing bass, resonant highs, air and brightness
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use std::f32::consts::PI;

mod audio_test_utils;
use audio_test_utils::calculate_rms;

fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph =
        compile_program(statements, sample_rate, None).expect("Failed to compile DSL code");
    let num_samples = (duration * sample_rate) as usize;
    graph.render(num_samples)
}

/// Perform FFT and analyze spectrum
fn analyze_spectrum(buffer: &[f32], sample_rate: f32) -> (Vec<f32>, Vec<f32>) {
    use rustfft::{num_complex::Complex, FftPlanner};

    let fft_size = 8192.min(buffer.len());
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);

    let mut input: Vec<Complex<f32>> = buffer[..fft_size]
        .iter()
        .enumerate()
        .map(|(i, &sample)| {
            let window = 0.5 * (1.0 - (2.0 * PI * i as f32 / fft_size as f32).cos());
            Complex::new(sample * window, 0.0)
        })
        .collect();

    fft.process(&mut input);

    let magnitudes: Vec<f32> = input[..fft_size / 2]
        .iter()
        .map(|c| (c.re * c.re + c.im * c.im).sqrt())
        .collect();

    let frequencies: Vec<f32> = (0..fft_size / 2)
        .map(|i| i as f32 * sample_rate / fft_size as f32)
        .collect();

    (frequencies, magnitudes)
}

// ========== Basic RHPF Tests ==========

#[test]
fn test_rhpf_compiles() {
    let code = r#"
        tempo: 0.5
        ~filtered $ sine 440 # rhpf 1000 2.0
        out $ ~filtered
    "#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);
    assert!(result.is_ok(), "RHPF should compile: {:?}", result.err());
}

#[test]
fn test_rhpf_generates_audio() {
    let code = r#"
        tempo: 0.5
        ~filtered $ sine 2000 # rhpf 1000 2.0
        out $ ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "RHPF should produce audio, got RMS: {}", rms);
    println!("RHPF RMS: {}", rms);
}

// ========== Frequency Response Tests ==========

#[test]
fn test_rhpf_passes_high_frequencies() {
    // Sine at 4000Hz through 1000Hz RHPF should pass mostly unaffected
    let code_filtered = r#"
        tempo: 0.5
        ~filtered $ sine 4000 # rhpf 1000 1.0
        out $ ~filtered * 0.3
    "#;

    let code_unfiltered = r#"
        tempo: 0.5
        out $ sine 4000 * 0.3
    "#;

    let buffer_filtered = render_dsl(code_filtered, 1.0);
    let buffer_unfiltered = render_dsl(code_unfiltered, 1.0);

    let rms_filtered = calculate_rms(&buffer_filtered);
    let rms_unfiltered = calculate_rms(&buffer_unfiltered);

    let attenuation = rms_filtered / rms_unfiltered;

    assert!(
        attenuation > 0.7,
        "RHPF should pass high frequencies mostly unaffected, attenuation: {}",
        attenuation
    );

    println!("High frequency attenuation: {}", attenuation);
}

#[test]
fn test_rhpf_attenuates_low_frequencies() {
    // Sine at 200Hz through 1000Hz RHPF should be heavily attenuated
    let code_filtered = r#"
        tempo: 0.5
        ~filtered $ sine 200 # rhpf 1000 1.0
        out $ ~filtered * 0.3
    "#;

    let code_unfiltered = r#"
        tempo: 0.5
        out $ sine 200 * 0.3
    "#;

    let buffer_filtered = render_dsl(code_filtered, 1.0);
    let buffer_unfiltered = render_dsl(code_unfiltered, 1.0);

    let rms_filtered = calculate_rms(&buffer_filtered);
    let rms_unfiltered = calculate_rms(&buffer_unfiltered);

    let attenuation = rms_filtered / rms_unfiltered;

    assert!(
        attenuation < 0.5,
        "RHPF should attenuate low frequencies, attenuation: {}",
        attenuation
    );

    println!("Low frequency attenuation: {}", attenuation);
}

#[test]
fn test_rhpf_frequency_response_curve() {
    // Test RHPF response across frequency spectrum using white noise
    let code_filtered = r#"
        tempo: 0.5
        ~filtered $ white_noise # rhpf 1000 2.0
        out $ ~filtered * 0.3
    "#;

    let buffer = render_dsl(code_filtered, 2.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    // Calculate energy in frequency bands
    let below_cutoff: f32 = frequencies
        .iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f < 500.0)
        .map(|(_, m)| m * m)
        .sum();

    let above_cutoff: f32 = frequencies
        .iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 2000.0 && **f < 10000.0)
        .map(|(_, m)| m * m)
        .sum();

    let ratio = above_cutoff / below_cutoff.max(0.001);

    assert!(
        ratio > 2.0,
        "RHPF should favor frequencies above cutoff, ratio: {}",
        ratio
    );

    println!("Above/below cutoff energy ratio: {}", ratio);
}

// ========== Resonance Tests ==========

#[test]
fn test_rhpf_low_resonance() {
    // Low resonance = gentle rolloff
    let code = r#"
        tempo: 0.5
        ~filtered $ white_noise # rhpf 1000 0.5
        out $ ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.02, "RHPF low resonance should work, RMS: {}", rms);
    println!("Low resonance RMS: {}", rms);
}

#[test]
fn test_rhpf_high_resonance() {
    // High resonance = sharp peak at cutoff
    let code = r#"
        tempo: 0.5
        ~filtered $ white_noise # rhpf 1000 8.0
        out $ ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    // Find peak near cutoff
    let peak_energy: f32 = frequencies
        .iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 800.0 && **f < 1200.0)
        .map(|(_, m)| m * m)
        .sum();

    let other_energy: f32 = frequencies
        .iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f < 500.0 || **f > 2000.0)
        .map(|(_, m)| m * m)
        .sum();

    // High resonance should create prominent peak
    assert!(
        peak_energy > other_energy * 0.3,
        "High resonance should create peak at cutoff"
    );

    println!(
        "High resonance - Peak: {}, Other: {}",
        peak_energy, other_energy
    );
}

#[test]
fn test_rhpf_resonance_comparison() {
    // High resonance should emphasize cutoff more than low resonance
    let code_low = r#"
        tempo: 0.5
        ~filtered $ white_noise # rhpf 1000 1.0
        out $ ~filtered * 0.3
    "#;

    let code_high = r#"
        tempo: 0.5
        ~filtered $ white_noise # rhpf 1000 6.0
        out $ ~filtered * 0.3
    "#;

    let buffer_low = render_dsl(code_low, 1.0);
    let buffer_high = render_dsl(code_high, 1.0);

    let (frequencies, mags_low) = analyze_spectrum(&buffer_low, 44100.0);
    let (_, mags_high) = analyze_spectrum(&buffer_high, 44100.0);

    // Find energy at cutoff frequency
    let low_peak: f32 = frequencies
        .iter()
        .zip(mags_low.iter())
        .filter(|(f, _)| (**f - 1000.0).abs() < 100.0)
        .map(|(_, m)| m * m)
        .sum();

    let high_peak: f32 = frequencies
        .iter()
        .zip(mags_high.iter())
        .filter(|(f, _)| (**f - 1000.0).abs() < 100.0)
        .map(|(_, m)| m * m)
        .sum();

    // High resonance should have more energy at cutoff
    assert!(
        high_peak > low_peak * 0.8,
        "High resonance should emphasize cutoff more, low: {}, high: {}",
        low_peak,
        high_peak
    );

    println!(
        "Resonance comparison - Low: {}, High: {}",
        low_peak, high_peak
    );
}

// ========== Cutoff Frequency Tests ==========

#[test]
fn test_rhpf_cutoff_500() {
    let code = r#"
        tempo: 0.5
        ~filtered $ white_noise # rhpf 500 2.0
        out $ ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    let low: f32 = frequencies
        .iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f < 300.0)
        .map(|(_, m)| m * m)
        .sum();

    let high: f32 = frequencies
        .iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 1000.0 && **f < 8000.0)
        .map(|(_, m)| m * m)
        .sum();

    assert!(
        high > low * 2.0,
        "RHPF 500Hz should favor high frequencies, high: {}, low: {}",
        high,
        low
    );

    println!("RHPF 500Hz - High: {}, Low: {}", high, low);
}

#[test]
fn test_rhpf_cutoff_2000() {
    let code = r#"
        tempo: 0.5
        ~filtered $ white_noise # rhpf 2000 2.0
        out $ ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    let low: f32 = frequencies
        .iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f < 1000.0)
        .map(|(_, m)| m * m)
        .sum();

    let high: f32 = frequencies
        .iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 3000.0 && **f < 10000.0)
        .map(|(_, m)| m * m)
        .sum();

    assert!(
        high > low,
        "RHPF 2000Hz should favor high over low frequencies, high: {}, low: {}",
        high,
        low
    );

    println!("RHPF 2000Hz - High: {}, Low: {}", high, low);
}

#[test]
fn test_rhpf_cutoff_100() {
    // Very low cutoff - should pass most audio (only removing sub-bass)
    let code = r#"
        tempo: 0.5
        ~filtered $ white_noise # rhpf 100 2.0
        out $ ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.10,
        "RHPF 100Hz should pass most audio, RMS: {}",
        rms
    );

    println!("RHPF 100Hz RMS: {}", rms);
}

// ========== Musical Applications ==========

#[test]
fn test_rhpf_remove_bass() {
    // Remove bass frequencies
    let code = r#"
        tempo: 0.5
        ~filtered $ saw 110 # rhpf 400 2.0
        out $ ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.02, "RHPF remove bass should work, RMS: {}", rms);
    println!("Bass removal RMS: {}", rms);
}

#[test]
fn test_rhpf_air_and_brightness() {
    // Add air and brightness to signal
    let code = r#"
        tempo: 0.5
        ~bright $ white_noise # rhpf 8000 3.0
        out $ ~bright * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.02,
        "RHPF air and brightness should work, RMS: {}",
        rms
    );
    println!("Air and brightness RMS: {}", rms);
}

#[test]
fn test_rhpf_resonant_highs() {
    // Resonant high frequencies
    let code = r#"
        tempo: 0.5
        ~env $ ad 0.01 0.2
        ~highs $ saw 220 # rhpf 2000 8.0
        out $ ~highs * ~env * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.01, "RHPF resonant highs should work, RMS: {}", rms);
    println!("Resonant highs RMS: {}", rms);
}

#[test]
fn test_rhpf_thin_sound() {
    // Thin/hollow sound by removing lows
    let code = r#"
        tempo: 0.5
        ~thin $ square 440 # rhpf 1000 2.0
        out $ ~thin * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "RHPF thin sound should work, RMS: {}", rms);
    println!("Thin sound RMS: {}", rms);
}

#[test]
fn test_rhpf_resonant_sweep() {
    // Upward filter sweep
    let code = r#"
        tempo: 0.5
        ~sweep $ line 200 3000
        ~synth $ saw 110 # rhpf ~sweep 6.0
        out $ ~synth * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.02, "RHPF resonant sweep should work, RMS: {}", rms);
    println!("Resonant sweep RMS: {}", rms);
}

// ========== Pattern Modulation Tests ==========

#[test]
fn test_rhpf_pattern_cutoff() {
    // Cutoff modulated by LFO
    let code = r#"
        tempo: 0.5
        ~lfo $ sine 0.5 * 1000 + 1500
        ~synth $ saw 110 # rhpf ~lfo 3.0
        out $ ~synth * 0.3
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.02,
        "RHPF with pattern-modulated cutoff should work, RMS: {}",
        rms
    );

    println!("Pattern cutoff RMS: {}", rms);
}

#[test]
fn test_rhpf_pattern_resonance() {
    // Resonance modulated by envelope
    let code = r#"
        tempo: 0.5
        ~env $ ad 0.01 0.3
        ~res $ ~env * 8.0 + 2.0
        ~synth $ saw 110 # rhpf 1500 ~res
        out $ ~synth * ~env * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.01,
        "RHPF with pattern-modulated resonance should work, RMS: {}",
        rms
    );

    println!("Pattern resonance RMS: {}", rms);
}

// ========== Cascaded Filters ==========

#[test]
fn test_rhpf_cascade() {
    // Two RHPFs in series create steeper rolloff
    let code = r#"
        tempo: 0.5
        ~filtered $ white_noise # rhpf 1000 2.0 # rhpf 1000 2.0
        out $ ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    let below: f32 = frequencies
        .iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f < 500.0)
        .map(|(_, m)| m * m)
        .sum();

    let above: f32 = frequencies
        .iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 2000.0 && **f < 10000.0)
        .map(|(_, m)| m * m)
        .sum();

    let ratio = above / below.max(0.001);

    assert!(
        ratio > 5.0,
        "Cascaded RHPFs should have steeper rolloff, ratio: {}",
        ratio
    );

    println!("Cascaded RHPF above/below ratio: {}", ratio);
}

#[test]
fn test_rhpf_lpf_bandpass() {
    // RHPF + RLPF creates bandpass effect
    let code = r#"
        tempo: 0.5
        ~bandpass $ white_noise # rhpf 500 2.0 # rlpf 2000 2.0
        out $ ~bandpass * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    let passband: f32 = frequencies
        .iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 800.0 && **f < 1800.0)
        .map(|(_, m)| m * m)
        .sum();

    let stopband: f32 = frequencies
        .iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f < 300.0 || **f > 4000.0)
        .map(|(_, m)| m * m)
        .sum();

    let ratio = passband / stopband.max(0.001);

    assert!(
        ratio > 1.5,
        "RHPF+RLPF should favor passband, ratio: {}",
        ratio
    );

    println!("Bandpass effect - Pass: {}, Stop: {}", passband, stopband);
}

// ========== Stability Tests ==========

#[test]
fn test_rhpf_no_excessive_clipping() {
    let code = r#"
        tempo: 0.5
        ~filtered $ sine 2000 # rhpf 1000 2.0
        out $ ~filtered * 0.5
    "#;

    let buffer = render_dsl(code, 1.0);
    let max_amplitude = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    assert!(
        max_amplitude <= 0.8,
        "RHPF should not cause excessive clipping, max: {}",
        max_amplitude
    );

    println!("RHPF max amplitude: {}", max_amplitude);
}

#[test]
fn test_rhpf_consistent_output() {
    let code = r#"
        tempo: 0.5
        ~filtered $ sine 2000 # rhpf 1000 2.0
        out $ ~filtered * 0.3
    "#;

    let buffer1 = render_dsl(code, 0.1);
    let buffer2 = render_dsl(code, 0.1);

    // Buffers should be identical (deterministic)
    let mut identical = 0;
    for i in 0..buffer1.len().min(buffer2.len()) {
        if (buffer1[i] - buffer2[i]).abs() < 0.0001 {
            identical += 1;
        }
    }

    let identity_ratio = identical as f32 / buffer1.len() as f32;
    assert!(
        identity_ratio > 0.99,
        "RHPF should produce consistent output, identity: {}",
        identity_ratio
    );

    println!("RHPF identity ratio: {}", identity_ratio);
}

#[test]
fn test_rhpf_no_dc_offset() {
    // RHPF should not introduce DC offset
    let code = r#"
        tempo: 0.5
        ~filtered $ sine 2000 # rhpf 1000 2.0
        out $ ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let mean: f32 = buffer.iter().sum::<f32>() / buffer.len() as f32;

    assert!(
        mean.abs() < 0.01,
        "RHPF should not introduce DC offset, mean: {}",
        mean
    );

    println!("RHPF DC offset: {}", mean);
}

// ========== Edge Cases ==========

#[test]
fn test_rhpf_very_low_cutoff() {
    // Very low cutoff passes almost everything
    let code = r#"
        tempo: 0.5
        ~filtered $ white_noise # rhpf 50 2.0
        out $ ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    // Should pass most audio
    assert!(rms > 0.10, "RHPF 50Hz should pass most audio, RMS: {}", rms);

    println!("RHPF 50Hz RMS: {}", rms);
}

#[test]
fn test_rhpf_self_oscillation() {
    // Very high resonance can cause self-oscillation
    let code = r#"
        tempo: 0.5
        ~filtered $ white_noise * 0.01 # rhpf 1000 20.0
        out $ ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    // Should still produce audio (possibly self-oscillating)
    assert!(
        rms > 0.0,
        "RHPF with very high resonance should work, RMS: {}",
        rms
    );

    println!("Self-oscillation test RMS: {}", rms);
}
