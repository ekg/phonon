/// Systematic tests: White Noise Generator
///
/// Tests white noise with spectral analysis and statistical verification.
/// White noise has equal energy at all frequencies (flat spectrum).
///
/// Key characteristics:
/// - Flat frequency spectrum (equal energy per frequency bin)
/// - Random amplitude distribution (Gaussian)
/// - Mean value near zero
/// - No parameters (just generates noise)
/// - Used for percussion, hi-hats, synthesis building block

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

// ========== Basic White Noise Tests ==========

#[test]
fn test_white_noise_compiles() {
    let code = r#"
        tempo: 0.5
        o1: white_noise
    "#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);
    assert!(result.is_ok(), "White noise should compile: {:?}", result.err());
}

#[test]
fn test_white_noise_generates_audio() {
    let code = r#"
        tempo: 0.5
        o1: white_noise * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "White noise should produce audio, got RMS: {}", rms);
    println!("White noise RMS: {}", rms);
}

// ========== Statistical Properties ==========

#[test]
fn test_white_noise_mean_near_zero() {
    let code = r#"
        tempo: 0.5
        o1: white_noise
    "#;

    let buffer = render_dsl(code, 2.0);
    let mean: f32 = buffer.iter().sum::<f32>() / buffer.len() as f32;

    assert!(mean.abs() < 0.05,
        "White noise mean should be near 0, got {}",
        mean);

    println!("White noise mean: {}", mean);
}

#[test]
fn test_white_noise_has_variance() {
    let code = r#"
        tempo: 0.5
        o1: white_noise
    "#;

    let buffer = render_dsl(code, 2.0);
    let mean: f32 = buffer.iter().sum::<f32>() / buffer.len() as f32;
    let variance: f32 = buffer.iter()
        .map(|&x| (x - mean) * (x - mean))
        .sum::<f32>() / buffer.len() as f32;

    assert!(variance > 0.05,
        "White noise should have variance, got {}",
        variance);

    println!("White noise variance: {}", variance);
}

// ========== Spectral Properties ==========

#[test]
fn test_white_noise_flat_spectrum() {
    let code = r#"
        tempo: 0.5
        o1: white_noise
    "#;

    let buffer = render_dsl(code, 2.0);
    let (_frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    // White noise should have relatively flat spectrum
    // Calculate variance of magnitudes (should be low for flat spectrum)
    let mean_mag: f32 = magnitudes.iter().sum::<f32>() / magnitudes.len() as f32;
    let variance: f32 = magnitudes.iter()
        .map(|&m| (m - mean_mag) * (m - mean_mag))
        .sum::<f32>() / magnitudes.len() as f32;
    let std_dev = variance.sqrt();
    let coefficient_of_variation = std_dev / mean_mag;

    // White noise should have relatively consistent energy across frequencies
    assert!(coefficient_of_variation < 2.0,
        "White noise spectrum should be relatively flat, CV: {}",
        coefficient_of_variation);

    println!("Spectral flatness CV: {}", coefficient_of_variation);
}

#[test]
fn test_white_noise_full_bandwidth() {
    let code = r#"
        tempo: 0.5
        o1: white_noise
    "#;

    let buffer = render_dsl(code, 2.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    // Check energy in different frequency bands
    let low_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f < 1000.0)
        .map(|(_, m)| m * m)
        .sum();

    let mid_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f >= 1000.0 && **f < 5000.0)
        .map(|(_, m)| m * m)
        .sum();

    let high_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f >= 5000.0 && **f < 15000.0)
        .map(|(_, m)| m * m)
        .sum();

    // All bands should have energy
    assert!(low_energy > 0.01, "Low frequencies should have energy");
    assert!(mid_energy > 0.01, "Mid frequencies should have energy");
    assert!(high_energy > 0.01, "High frequencies should have energy");

    println!("Energy - Low: {}, Mid: {}, High: {}", low_energy, mid_energy, high_energy);
}

// ========== Musical Applications ==========

#[test]
fn test_white_noise_hi_hat() {
    // Hi-hat: filtered white noise with envelope
    let code = r#"
        tempo: 0.5
        ~env: ad 0.001 0.05
        ~hh: white_noise # rhpf 8000 2.0
        o1: ~hh * ~env * 0.4
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.01, "White noise hi-hat should work, RMS: {}", rms);
    println!("Hi-hat RMS: {}", rms);
}

#[test]
fn test_white_noise_snare() {
    // Snare: mix of tone and filtered noise
    let code = r#"
        tempo: 0.5
        ~env: ad 0.001 0.15
        ~tone: sine 180
        ~noise: white_noise # rlpf 4000 2.0
        o1: ((~tone + ~noise) * ~env) * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.03, "White noise snare should work, RMS: {}", rms);
    println!("Snare RMS: {}", rms);
}

#[test]
fn test_white_noise_wind() {
    // Wind sound: low-passed noise with slow envelope
    let code = r#"
        tempo: 1.0
        ~env: line 0.3 0.8
        ~wind: white_noise # rlpf 800 0.5
        o1: ~wind * ~env * 0.2
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.005, "White noise wind should work, RMS: {}", rms);
    println!("Wind RMS: {}", rms);
}

#[test]
fn test_white_noise_crash() {
    // Crash cymbal: bandpassed noise with decay
    let code = r#"
        tempo: 0.5
        ~env: ad 0.001 0.8
        ~crash: white_noise # rhpf 3000 1.5 # rlpf 12000 1.5
        o1: ~crash * ~env * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.03, "White noise crash should work, RMS: {}", rms);
    println!("Crash RMS: {}", rms);
}

// ========== Filtering Tests ==========

#[test]
fn test_white_noise_lowpass_filter() {
    let code = r#"
        tempo: 0.5
        ~filtered: white_noise # rlpf 1000 2.0
        o1: ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    // Low frequencies should have more energy than high
    let low_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f < 800.0)
        .map(|(_, m)| m * m)
        .sum();

    let high_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 3000.0 && **f < 10000.0)
        .map(|(_, m)| m * m)
        .sum();

    let ratio = low_energy / high_energy.max(0.001);
    assert!(ratio > 2.0,
        "Lowpassed white noise should favor low frequencies, ratio: {}",
        ratio);

    println!("Lowpass - Low/High ratio: {}", ratio);
}

#[test]
fn test_white_noise_highpass_filter() {
    let code = r#"
        tempo: 0.5
        ~filtered: white_noise # rhpf 5000 2.0
        o1: ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    // High frequencies should have more energy than low
    let low_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f < 1000.0)
        .map(|(_, m)| m * m)
        .sum();

    let high_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 7000.0 && **f < 15000.0)
        .map(|(_, m)| m * m)
        .sum();

    let ratio = high_energy / low_energy.max(0.001);
    assert!(ratio > 2.0,
        "Highpassed white noise should favor high frequencies, ratio: {}",
        ratio);

    println!("Highpass - High/Low ratio: {}", ratio);
}

// ========== Amplitude Control ==========

#[test]
fn test_white_noise_amplitude_scaling() {
    let code = r#"
        tempo: 0.5
        o1: white_noise * 0.1
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    // Scaled down noise should have lower RMS
    assert!(rms < 0.15 && rms > 0.01,
        "Scaled white noise should have appropriate RMS, got {}",
        rms);

    println!("Scaled noise RMS: {}", rms);
}

#[test]
fn test_white_noise_envelope_shaping() {
    let code = r#"
        tempo: 0.5
        ~env: ad 0.01 0.2
        o1: white_noise * ~env * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    // Envelope should shape the noise
    assert!(rms > 0.01,
        "Envelope-shaped white noise should work, RMS: {}",
        rms);

    println!("Envelope-shaped noise RMS: {}", rms);
}
