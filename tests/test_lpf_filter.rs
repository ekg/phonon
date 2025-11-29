/// Systematic tests: Lowpass Filter (LPF)
///
/// Tests lowpass filter with frequency response analysis and audio verification.
/// Lowpass filters attenuate frequencies above the cutoff frequency.
///
/// Key characteristics:
/// - Passes low frequencies, attenuates high frequencies
/// - Cutoff frequency (-3dB point)
/// - Rolloff slope (dB per octave)
/// - Used for bass enhancement, removing harshness, mellowing sounds

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use std::f32::consts::PI;

mod audio_test_utils;
use audio_test_utils::calculate_rms;

fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph = compile_program(statements, sample_rate, None).expect("Failed to compile DSL code");
    let num_samples = (duration * sample_rate) as usize;
    graph.render(num_samples)
}

/// Perform FFT and analyze spectrum
fn analyze_spectrum(buffer: &[f32], sample_rate: f32) -> (Vec<f32>, Vec<f32>) {
    use rustfft::{FftPlanner, num_complex::Complex};

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

// ========== Basic LPF Tests ==========

#[test]
fn test_lpf_compiles() {
    let code = r#"
        tempo: 0.5
        ~filtered $ sine 440 # lpf 1000
        out $ ~filtered
    "#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);
    assert!(result.is_ok(), "LPF should compile: {:?}", result.err());
}

#[test]
fn test_lpf_generates_audio() {
    let code = r#"
        tempo: 0.5
        ~filtered $ sine 440 # lpf 2000
        out $ ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "LPF should produce audio, got RMS: {}", rms);
    println!("LPF RMS: {}", rms);
}

// ========== Frequency Response Tests ==========

#[test]
fn test_lpf_passes_low_frequencies() {
    // Sine at 200Hz through 1000Hz LPF should pass mostly unaffected
    let code_filtered = r#"
        tempo: 0.5
        ~filtered $ sine 200 # lpf 1000
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
    
    assert!(attenuation > 0.8,
        "LPF should pass low frequencies mostly unaffected, attenuation: {}",
        attenuation);

    println!("Low frequency attenuation: {}", attenuation);
}

#[test]
fn test_lpf_attenuates_high_frequencies() {
    // Sine at 4000Hz through 1000Hz LPF should be heavily attenuated
    let code_filtered = r#"
        tempo: 0.5
        ~filtered $ sine 4000 # lpf 1000
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
    
    assert!(attenuation < 0.5,
        "LPF should attenuate high frequencies, attenuation: {}",
        attenuation);

    println!("High frequency attenuation: {}", attenuation);
}

#[test]
fn test_lpf_frequency_response_curve() {
    // Test LPF response across frequency spectrum using white noise
    let code_filtered = r#"
        tempo: 0.5
        ~filtered $ white_noise # lpf 1000
        out $ ~filtered * 0.3
    "#;

    let code_unfiltered = r#"
        tempo: 0.5
        out $ white_noise * 0.3
    "#;

    let buffer_filtered = render_dsl(code_filtered, 2.0);
    let buffer_unfiltered = render_dsl(code_unfiltered, 2.0);

    let (frequencies, magnitudes_filtered) = analyze_spectrum(&buffer_filtered, 44100.0);
    let (_, _magnitudes_unfiltered) = analyze_spectrum(&buffer_unfiltered, 44100.0);

    // Calculate energy in frequency bands
    let below_cutoff: f32 = frequencies.iter()
        .zip(magnitudes_filtered.iter())
        .filter(|(f, _)| **f < 800.0)
        .map(|(_, m)| m * m)
        .sum();

    let above_cutoff: f32 = frequencies.iter()
        .zip(magnitudes_filtered.iter())
        .filter(|(f, _)| **f > 2000.0 && **f < 10000.0)
        .map(|(_, m)| m * m)
        .sum();

    let ratio = below_cutoff / above_cutoff.max(0.001);
    
    assert!(ratio > 2.0,
        "LPF should favor frequencies below cutoff, ratio: {}",
        ratio);

    println!("Below/above cutoff energy ratio: {}", ratio);
}

// ========== Cutoff Frequency Tests ==========

#[test]
fn test_lpf_cutoff_500() {
    let code = r#"
        tempo: 0.5
        ~filtered $ white_noise # lpf 500
        out $ ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    let low: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f < 400.0)
        .map(|(_, m)| m * m)
        .sum();

    let high: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 1000.0 && **f < 8000.0)
        .map(|(_, m)| m * m)
        .sum();

    assert!(low > high * 2.0,
        "LPF 500Hz should favor low frequencies, low: {}, high: {}",
        low, high);

    println!("LPF 500Hz - Low: {}, High: {}", low, high);
}

#[test]
fn test_lpf_cutoff_2000() {
    let code = r#"
        tempo: 0.5
        ~filtered $ white_noise # lpf 2000
        out $ ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    let mid: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 500.0 && **f < 1800.0)
        .map(|(_, m)| m * m)
        .sum();

    let high: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 5000.0 && **f < 12000.0)
        .map(|(_, m)| m * m)
        .sum();

    assert!(mid > high,
        "LPF 2000Hz should favor mid over high frequencies, mid: {}, high: {}",
        mid, high);

    println!("LPF 2000Hz - Mid: {}, High: {}", mid, high);
}

#[test]
fn test_lpf_cutoff_8000() {
    // Very high cutoff - should pass most audio
    let code = r#"
        tempo: 0.5
        ~filtered $ white_noise # lpf 8000
        out $ ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05,
        "LPF 8000Hz should pass most audio, RMS: {}",
        rms);

    println!("LPF 8000Hz RMS: {}", rms);
}

// ========== Musical Applications ==========

#[test]
fn test_lpf_mellow_bass() {
    let code = r#"
        tempo: 0.5
        ~env $ ad 0.01 0.3
        ~bass $ saw 55 # lpf 300
        out $ ~bass * ~env * 0.4
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "LPF mellow bass should work, RMS: {}", rms);
    println!("Mellow bass RMS: {}", rms);
}

#[test]
fn test_lpf_removing_harshness() {
    // Square wave through LPF becomes more sine-like
    let code = r#"
        tempo: 0.5
        ~mellowed $ square 440 # lpf 800
        out $ ~mellowed * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    // Calculate high frequency content
    let high_content: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 2000.0 && **f < 8000.0)
        .map(|(_, m)| m * m)
        .sum();

    assert!(high_content < 2000.0,
        "Filtered square should have reduced high frequencies, got: {}",
        high_content);

    println!("High frequency content: {}", high_content);
}

#[test]
fn test_lpf_warm_pad() {
    let code = r#"
        tempo: 1.0
        ~env $ ad 0.5 0.5
        ~pad $ saw 220 # lpf 1200
        out $ ~pad * ~env * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.03, "LPF warm pad should work, RMS: {}", rms);
    println!("Warm pad RMS: {}", rms);
}

#[test]
fn test_lpf_telephone_effect() {
    // Narrow LPF creates telephone/lo-fi effect
    let code = r#"
        tempo: 0.5
        ~telephone $ sine 440 # lpf 3000
        out $ ~telephone * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "LPF telephone effect should work, RMS: {}", rms);
    println!("Telephone effect RMS: {}", rms);
}

// ========== Pattern Modulation Tests ==========

#[test]
fn test_lpf_swept_cutoff() {
    // Cutoff frequency modulated by LFO
    let code = r#"
        tempo: 0.5
        ~lfo $ sine 0.5 * 1000 + 1500
        ~swept $ saw 110 # lpf ~lfo
        out $ ~swept * 0.3
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05,
        "LPF with swept cutoff should work, RMS: {}",
        rms);

    println!("Swept cutoff RMS: {}", rms);
}

#[test]
fn test_lpf_envelope_controlled_cutoff() {
    // Classic filter envelope
    let code = r#"
        tempo: 0.5
        ~env $ ad 0.01 0.3
        ~cutoff $ ~env * 3000 + 200
        ~synth $ saw 110 # lpf ~cutoff
        out $ ~synth * ~env * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.02,
        "LPF with envelope-controlled cutoff should work, RMS: {}",
        rms);

    println!("Envelope-controlled cutoff RMS: {}", rms);
}

// ========== Cascaded Filters ==========

#[test]
fn test_lpf_cascade() {
    // Two LPFs in series create steeper rolloff
    let code = r#"
        tempo: 0.5
        ~filtered $ white_noise # lpf 1000 # lpf 1000
        out $ ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    let below: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f < 800.0)
        .map(|(_, m)| m * m)
        .sum();

    let above: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 3000.0 && **f < 10000.0)
        .map(|(_, m)| m * m)
        .sum();

    let ratio = below / above.max(0.001);
    
    assert!(ratio > 5.0,
        "Cascaded LPFs should have steeper rolloff, ratio: {}",
        ratio);

    println!("Cascaded LPF below/above ratio: {}", ratio);
}

// ========== Stability Tests ==========

#[test]
fn test_lpf_no_clipping() {
    let code = r#"
        tempo: 0.5
        ~filtered $ sine 440 # lpf 2000
        out $ ~filtered * 0.5
    "#;

    let buffer = render_dsl(code, 1.0);
    let max_amplitude = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    assert!(max_amplitude <= 0.7,
        "LPF should not cause clipping, max: {}",
        max_amplitude);

    println!("LPF max amplitude: {}", max_amplitude);
}

#[test]
fn test_lpf_consistent_output() {
    let code = r#"
        tempo: 0.5
        ~filtered $ sine 440 # lpf 1000
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
    assert!(identity_ratio > 0.99,
        "LPF should produce consistent output, identity: {}",
        identity_ratio);

    println!("LPF identity ratio: {}", identity_ratio);
}

// ========== Edge Cases ==========

#[test]
fn test_lpf_very_low_cutoff() {
    // Very low cutoff removes almost everything
    let code = r#"
        tempo: 0.5
        ~filtered $ white_noise # lpf 50
        out $ ~filtered * 0.5
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    // Should be very quiet
    assert!(rms < 0.3,
        "LPF 50Hz should heavily attenuate, RMS: {}",
        rms);

    println!("LPF 50Hz RMS: {}", rms);
}

#[test]
fn test_lpf_nyquist_cutoff() {
}
