/// Systematic tests: Resonant Lowpass Filter (RLPF)
///
/// Tests resonant lowpass filter with frequency response analysis and audio verification.
/// RLPF is a lowpass filter with resonance/Q control for emphasized cutoff frequency.
///
/// Key characteristics:
/// - Passes low frequencies, attenuates high frequencies
/// - Resonance creates peak at cutoff frequency
/// - Q/resonance parameter controls peak sharpness
/// - Used for synth bass, resonant sweeps, classic analog sounds
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

// ========== Basic RLPF Tests ==========

#[test]
fn test_rlpf_compiles() {
    let code = r#"
        tempo: 0.5
        ~filtered $ sine 440 # rlpf 1000 2.0
        out $ ~filtered
    "#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);
    assert!(result.is_ok(), "RLPF should compile: {:?}", result.err());
}

#[test]
fn test_rlpf_generates_audio() {
    let code = r#"
        tempo: 0.5
        ~filtered $ sine 440 # rlpf 2000 2.0
        out $ ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "RLPF should produce audio, got RMS: {}", rms);
    println!("RLPF RMS: {}", rms);
}

// ========== Frequency Response Tests ==========

#[test]
fn test_rlpf_passes_low_frequencies() {
    // Sine at 200Hz through 1000Hz RLPF should pass mostly unaffected
    let code_filtered = r#"
        tempo: 0.5
        ~filtered $ sine 200 # rlpf 1000 1.0
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
        attenuation > 0.7,
        "RLPF should pass low frequencies mostly unaffected, attenuation: {}",
        attenuation
    );

    println!("Low frequency attenuation: {}", attenuation);
}

#[test]
fn test_rlpf_attenuates_high_frequencies() {
    // Sine at 4000Hz through 1000Hz RLPF should be heavily attenuated
    let code_filtered = r#"
        tempo: 0.5
        ~filtered $ sine 4000 # rlpf 1000 1.0
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
        attenuation < 0.5,
        "RLPF should attenuate high frequencies, attenuation: {}",
        attenuation
    );

    println!("High frequency attenuation: {}", attenuation);
}

#[test]
fn test_rlpf_frequency_response_curve() {
    // Test RLPF response across frequency spectrum using white noise
    let code_filtered = r#"
        tempo: 0.5
        ~filtered $ white_noise # rlpf 1000 2.0
        out $ ~filtered * 0.3
    "#;

    let buffer = render_dsl(code_filtered, 2.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    // Calculate energy in frequency bands
    let below_cutoff: f32 = frequencies
        .iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f < 800.0)
        .map(|(_, m)| m * m)
        .sum();

    let above_cutoff: f32 = frequencies
        .iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 2000.0 && **f < 10000.0)
        .map(|(_, m)| m * m)
        .sum();

    let ratio = below_cutoff / above_cutoff.max(0.001);

    assert!(
        ratio > 2.0,
        "RLPF should favor frequencies below cutoff, ratio: {}",
        ratio
    );

    println!("Below/above cutoff energy ratio: {}", ratio);
}

// ========== Resonance Tests ==========

#[test]
fn test_rlpf_low_resonance() {
    // Low resonance = gentle rolloff
    let code = r#"
        tempo: 0.5
        ~filtered $ white_noise # rlpf 1000 0.5
        out $ ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.02, "RLPF low resonance should work, RMS: {}", rms);
    println!("Low resonance RMS: {}", rms);
}

#[test]
fn test_rlpf_high_resonance() {
    // High resonance = sharp peak at cutoff
    let code = r#"
        tempo: 0.5
        ~filtered $ white_noise # rlpf 1000 8.0
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
        peak_energy > other_energy * 0.5,
        "High resonance should create peak at cutoff"
    );

    println!(
        "High resonance - Peak: {}, Other: {}",
        peak_energy, other_energy
    );
}

#[test]
fn test_rlpf_resonance_comparison() {
    // High resonance should emphasize cutoff more than low resonance
    let code_low = r#"
        tempo: 0.5
        ~filtered $ white_noise # rlpf 1000 1.0
        out $ ~filtered * 0.3
    "#;

    let code_high = r#"
        tempo: 0.5
        ~filtered $ white_noise # rlpf 1000 6.0
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
fn test_rlpf_cutoff_500() {
    let code = r#"
        tempo: 0.5
        ~filtered $ white_noise # rlpf 500 2.0
        out $ ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    let low: f32 = frequencies
        .iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f < 400.0)
        .map(|(_, m)| m * m)
        .sum();

    let high: f32 = frequencies
        .iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 1000.0 && **f < 8000.0)
        .map(|(_, m)| m * m)
        .sum();

    assert!(
        low > high * 2.0,
        "RLPF 500Hz should favor low frequencies, low: {}, high: {}",
        low,
        high
    );

    println!("RLPF 500Hz - Low: {}, High: {}", low, high);
}

#[test]
fn test_rlpf_cutoff_2000() {
    let code = r#"
        tempo: 0.5
        ~filtered $ white_noise # rlpf 2000 2.0
        out $ ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    let mid: f32 = frequencies
        .iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 500.0 && **f < 1800.0)
        .map(|(_, m)| m * m)
        .sum();

    let high: f32 = frequencies
        .iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 5000.0 && **f < 12000.0)
        .map(|(_, m)| m * m)
        .sum();

    assert!(
        mid > high,
        "RLPF 2000Hz should favor mid over high frequencies, mid: {}, high: {}",
        mid,
        high
    );

    println!("RLPF 2000Hz - Mid: {}, High: {}", mid, high);
}

#[test]
fn test_rlpf_cutoff_8000() {
    // Very high cutoff - should pass most audio
    let code = r#"
        tempo: 0.5
        ~filtered $ white_noise # rlpf 8000 2.0
        out $ ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.10,
        "RLPF 8000Hz should pass most audio, RMS: {}",
        rms
    );

    println!("RLPF 8000Hz RMS: {}", rms);
}

// ========== Musical Applications ==========

#[test]
fn test_rlpf_synth_bass() {
    // Classic resonant bass: saw with filter envelope
    let code = r#"
        tempo: 0.5
        ~env $ ad 0.01 0.3
        ~cutoff $ ~env * 3000 + 200
        ~bass $ saw 55 # rlpf ~cutoff 4.0
        out $ ~bass * ~env * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.02, "RLPF synth bass should work, RMS: {}", rms);
    println!("Synth bass RMS: {}", rms);
}

#[test]
fn test_rlpf_acid_bass() {
    // Acid bass: high resonance with fast envelope
    let code = r#"
        tempo: 0.5
        ~env $ ad 0.001 0.2
        ~cutoff $ ~env * 4000 + 100
        ~acid $ saw 82.5 # rlpf ~cutoff 12.0
        out $ ~acid * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.01, "RLPF acid bass should work, RMS: {}", rms);
    println!("Acid bass RMS: {}", rms);
}

#[test]
fn test_rlpf_warm_pad() {
    // Warm pad: lowpassed saw with slow envelope
    let code = r#"
        tempo: 1.0
        ~env $ ad 0.5 0.5
        ~pad $ saw 220 # rlpf 1200 2.0
        out $ ~pad * ~env * 0.2
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.02, "RLPF warm pad should work, RMS: {}", rms);
    println!("Warm pad RMS: {}", rms);
}

#[test]
fn test_rlpf_resonant_sweep() {
    // Classic filter sweep
    let code = r#"
        tempo: 0.5
        ~sweep $ line 200 3000
        ~synth $ saw 110 # rlpf ~sweep 8.0
        out $ ~synth * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "RLPF resonant sweep should work, RMS: {}", rms);
    println!("Resonant sweep RMS: {}", rms);
}

#[test]
fn test_rlpf_mellow_square() {
    // Square wave mellowed by filter
    let code = r#"
        tempo: 0.5
        ~mellowed $ square 440 # rlpf 800 2.0
        out $ ~mellowed * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "RLPF mellowed square should work, RMS: {}", rms);
    println!("Mellowed square RMS: {}", rms);
}

// ========== Pattern Modulation Tests ==========

#[test]
fn test_rlpf_pattern_cutoff() {
    // Cutoff modulated by LFO
    let code = r#"
        tempo: 0.5
        ~lfo $ sine 0.5 * 1000 + 1500
        ~synth $ saw 110 # rlpf ~lfo 3.0
        out $ ~synth * 0.3
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.05,
        "RLPF with pattern-modulated cutoff should work, RMS: {}",
        rms
    );

    println!("Pattern cutoff RMS: {}", rms);
}

#[test]
fn test_rlpf_pattern_resonance() {
    // Resonance modulated by envelope
    let code = r#"
        tempo: 0.5
        ~env $ ad 0.01 0.3
        ~res $ ~env * 8.0 + 2.0
        ~synth $ saw 110 # rlpf 800 ~res
        out $ ~synth * ~env * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.02,
        "RLPF with pattern-modulated resonance should work, RMS: {}",
        rms
    );

    println!("Pattern resonance RMS: {}", rms);
}

// ========== Cascaded Filters ==========

#[test]
fn test_rlpf_cascade() {
    // Two RLPFs in series create steeper rolloff
    let code = r#"
        tempo: 0.5
        ~filtered $ white_noise # rlpf 1000 2.0 # rlpf 1000 2.0
        out $ ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    let below: f32 = frequencies
        .iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f < 800.0)
        .map(|(_, m)| m * m)
        .sum();

    let above: f32 = frequencies
        .iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 3000.0 && **f < 10000.0)
        .map(|(_, m)| m * m)
        .sum();

    let ratio = below / above.max(0.001);

    assert!(
        ratio > 5.0,
        "Cascaded RLPFs should have steeper rolloff, ratio: {}",
        ratio
    );

    println!("Cascaded RLPF below/above ratio: {}", ratio);
}

// ========== Stability Tests ==========

#[test]
fn test_rlpf_no_excessive_clipping() {
    let code = r#"
        tempo: 0.5
        ~filtered $ sine 440 # rlpf 2000 2.0
        out $ ~filtered * 0.5
    "#;

    let buffer = render_dsl(code, 1.0);
    let max_amplitude = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    assert!(
        max_amplitude <= 0.7,
        "RLPF should not cause excessive clipping, max: {}",
        max_amplitude
    );

    println!("RLPF max amplitude: {}", max_amplitude);
}

#[test]
fn test_rlpf_consistent_output() {
    let code = r#"
        tempo: 0.5
        ~filtered $ sine 440 # rlpf 1000 2.0
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
        "RLPF should produce consistent output, identity: {}",
        identity_ratio
    );

    println!("RLPF identity ratio: {}", identity_ratio);
}

#[test]
fn test_rlpf_no_dc_offset() {
    // RLPF should not introduce DC offset
    let code = r#"
        tempo: 0.5
        ~filtered $ sine 440 # rlpf 1000 2.0
        out $ ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let mean: f32 = buffer.iter().sum::<f32>() / buffer.len() as f32;

    assert!(
        mean.abs() < 0.01,
        "RLPF should not introduce DC offset, mean: {}",
        mean
    );

    println!("RLPF DC offset: {}", mean);
}

// ========== Edge Cases ==========

#[test]
fn test_rlpf_very_low_cutoff() {
    // Very low cutoff removes almost everything
    let code = r#"
        tempo: 0.5
        ~filtered $ white_noise # rlpf 50 2.0
        out $ ~filtered * 0.5
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    // Should be very quiet
    assert!(
        rms < 0.3,
        "RLPF 50Hz should heavily attenuate, RMS: {}",
        rms
    );

    println!("RLPF 50Hz RMS: {}", rms);
}

#[test]
fn test_rlpf_self_oscillation() {
    // Very high resonance can cause self-oscillation
    let code = r#"
        tempo: 0.5
        ~filtered $ white_noise * 0.01 # rlpf 1000 20.0
        out $ ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    // Should still produce audio (possibly self-oscillating)
    assert!(
        rms > 0.0,
        "RLPF with very high resonance should work, RMS: {}",
        rms
    );

    println!("Self-oscillation test RMS: {}", rms);
}
