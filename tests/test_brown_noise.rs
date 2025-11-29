/// Systematic tests: Brown Noise Generator
///
/// Tests brown noise (red noise, random walk) with spectral analysis and statistical verification.
/// Brown noise has -6dB per octave rolloff (1/f² spectrum).
///
/// Key characteristics:
/// - -6dB per octave rolloff (1/f² spectrum)
/// - Even more bass energy than pink noise
/// - Very deep, rumbling sound
/// - Used for deep bass, thunder, distant ocean
/// - Random walk amplitude distribution
/// - Mean value near zero

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

// ========== Basic Brown Noise Tests ==========

#[test]
fn test_brown_noise_compiles() {
    let code = r#"
        tempo: 0.5
        out $ brown_noise
    "#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);
    assert!(result.is_ok(), "Brown noise should compile: {:?}", result.err());
}

#[test]
fn test_brown_noise_generates_audio() {
    let code = r#"
        tempo: 0.5
        out $ brown_noise * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Brown noise should produce audio, got RMS: {}", rms);
    println!("Brown noise RMS: {}", rms);
}

// ========== Statistical Properties ==========

#[test]
fn test_brown_noise_mean_near_zero() {
    let code = r#"
        tempo: 0.5
        out $ brown_noise
    "#;

    let buffer = render_dsl(code, 2.0);
    let mean: f32 = buffer.iter().sum::<f32>() / buffer.len() as f32;

    assert!(mean.abs() < 0.1,
        "Brown noise mean should be near 0, got {}",
        mean);

    println!("Brown noise mean: {}", mean);
}

#[test]
fn test_brown_noise_has_variance() {
    let code = r#"
        tempo: 0.5
        out $ brown_noise
    "#;

    let buffer = render_dsl(code, 2.0);
    let mean: f32 = buffer.iter().sum::<f32>() / buffer.len() as f32;
    let variance: f32 = buffer.iter()
        .map(|&x| (x - mean) * (x - mean))
        .sum::<f32>() / buffer.len() as f32;

    assert!(variance > 0.05,
        "Brown noise should have variance, got {}",
        variance);

    println!("Brown noise variance: {}", variance);
}

// ========== Spectral Properties ==========

#[test]
fn test_brown_noise_1_over_f_squared_spectrum() {
    // Brown noise should have -6dB/octave rolloff
    let code = r#"
        tempo: 0.5
        out $ brown_noise
    "#;

    let buffer = render_dsl(code, 2.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    // Calculate energy in octave bands
    let low_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 100.0 && **f < 200.0)
        .map(|(_, m)| m * m)
        .sum();

    let mid_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 800.0 && **f < 1600.0)
        .map(|(_, m)| m * m)
        .sum();

    let high_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 6400.0 && **f < 12800.0)
        .map(|(_, m)| m * m)
        .sum();

    // Brown noise should have strong low-frequency dominance
    assert!(low_energy > mid_energy * 2.0,
        "Brown noise should have strong low-frequency energy. Low: {}, Mid: {}",
        low_energy, mid_energy);

    assert!(mid_energy > high_energy,
        "Brown noise mid frequencies should have more energy than high. Mid: {}, High: {}",
        mid_energy, high_energy);

    println!("Energy - Low: {}, Mid: {}, High: {}", low_energy, mid_energy, high_energy);
}

#[test]
fn test_brown_vs_pink_vs_white_spectrum() {
    // Brown should have most bass, then pink, then white
    let code_brown = r#"
        tempo: 0.5
        out $ brown_noise
    "#;

    let code_pink = r#"
        tempo: 0.5
        out $ pink_noise
    "#;

    let code_white = r#"
        tempo: 0.5
        out $ white_noise
    "#;

    let buffer_brown = render_dsl(code_brown, 2.0);
    let buffer_pink = render_dsl(code_pink, 2.0);
    let buffer_white = render_dsl(code_white, 2.0);

    let (frequencies, magnitudes_brown) = analyze_spectrum(&buffer_brown, 44100.0);
    let (_, magnitudes_pink) = analyze_spectrum(&buffer_pink, 44100.0);
    let (_, magnitudes_white) = analyze_spectrum(&buffer_white, 44100.0);

    // Calculate low-frequency energy
    let brown_low: f32 = frequencies.iter()
        .zip(magnitudes_brown.iter())
        .filter(|(f, _)| **f < 500.0)
        .map(|(_, m)| m * m)
        .sum();

    let pink_low: f32 = frequencies.iter()
        .zip(magnitudes_pink.iter())
        .filter(|(f, _)| **f < 500.0)
        .map(|(_, m)| m * m)
        .sum();

    let white_low: f32 = frequencies.iter()
        .zip(magnitudes_white.iter())
        .filter(|(f, _)| **f < 500.0)
        .map(|(_, m)| m * m)
        .sum();

    // Calculate high-frequency energy
    let brown_high: f32 = frequencies.iter()
        .zip(magnitudes_brown.iter())
        .filter(|(f, _)| **f > 5000.0 && **f < 15000.0)
        .map(|(_, m)| m * m)
        .sum();

    let pink_high: f32 = frequencies.iter()
        .zip(magnitudes_pink.iter())
        .filter(|(f, _)| **f > 5000.0 && **f < 15000.0)
        .map(|(_, m)| m * m)
        .sum();

    let white_high: f32 = frequencies.iter()
        .zip(magnitudes_white.iter())
        .filter(|(f, _)| **f > 5000.0 && **f < 15000.0)
        .map(|(_, m)| m * m)
        .sum();

    // Brown should have most bass relative to highs
    let brown_ratio = brown_low / brown_high.max(0.001);
    let pink_ratio = pink_low / pink_high.max(0.001);
    let white_ratio = white_low / white_high.max(0.001);

    assert!(brown_ratio > pink_ratio,
        "Brown noise should have more bass relative to highs than pink. Brown: {}, Pink: {}",
        brown_ratio, pink_ratio);

    assert!(pink_ratio > white_ratio,
        "Pink noise should have more bass relative to highs than white. Pink: {}, White: {}",
        pink_ratio, white_ratio);

    println!("Low/High ratio - Brown: {}, Pink: {}, White: {}", brown_ratio, pink_ratio, white_ratio);
}

// ========== Musical Applications ==========

#[test]
fn test_brown_noise_thunder() {
    // Thunder rumble: brown noise with envelope
    let code = r#"
        tempo: 1.0
        ~env $ ad 0.5 2.0
        ~thunder $ brown_noise # rlpf 150 0.5
        out $ ~thunder * ~env * 0.4
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Brown noise thunder should work, RMS: {}", rms);
    println!("Thunder RMS: {}", rms);
}

#[test]
fn test_brown_noise_sub_bass() {
    // Deep sub-bass texture
    let code = r#"
        tempo: 0.5
        ~env $ ad 0.01 0.5
        ~bass $ brown_noise # rlpf 80 1.0
        out $ ~bass * ~env * 0.5
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.03, "Brown noise sub-bass should work, RMS: {}", rms);
    println!("Sub-bass RMS: {}", rms);
}

#[test]
fn test_brown_noise_distant_ocean() {
    // Distant ocean waves with brown noise
    let code = r#"
        tempo: 0.5
        ~env $ sine 0.05 * 0.4 + 0.6
        ~ocean $ brown_noise # rlpf 200 0.5
        out $ ~ocean * ~env * 0.3
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.03, "Brown noise distant ocean should work, RMS: {}", rms);
    println!("Distant ocean RMS: {}", rms);
}

#[test]
fn test_brown_noise_rumble() {
    // Low rumble for cinematic effects
    let code = r#"
        tempo: 1.0
        ~lfo $ sine 0.2 * 0.3 + 0.5
        ~rumble $ brown_noise # rlpf 120 1.0
        out $ ~rumble * ~lfo * 0.4
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.03, "Brown noise rumble should work, RMS: {}", rms);
    println!("Rumble RMS: {}", rms);
}

#[test]
fn test_brown_noise_wind_gust() {
    // Deep wind gust
    let code = r#"
        tempo: 1.0
        ~env $ ad 1.0 1.5
        ~wind $ brown_noise # rlpf 300 0.8
        out $ ~wind * ~env * 0.3
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.02, "Brown noise wind gust should work, RMS: {}", rms);
    println!("Wind gust RMS: {}", rms);
}

// ========== Filtering Tests ==========

#[test]
fn test_brown_noise_lowpass_filter() {
    let code = r#"
        tempo: 0.5
        ~filtered $ brown_noise # rlpf 500 2.0
        out $ ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    // Low frequencies should dominate
    let low_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f < 400.0)
        .map(|(_, m)| m * m)
        .sum();

    let high_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 2000.0 && **f < 8000.0)
        .map(|(_, m)| m * m)
        .sum();

    let ratio = low_energy / high_energy.max(0.001);
    assert!(ratio > 3.0,
        "Lowpassed brown noise should strongly favor low frequencies, ratio: {}",
        ratio);

    println!("Lowpass - Low/High ratio: {}", ratio);
}

#[test]
fn test_brown_noise_highpass_filter() {
    // Highpass removes most of brown noise energy
    let code = r#"
        tempo: 0.5
        ~filtered $ brown_noise # rhpf 2000 2.0
        out $ ~filtered * 0.5
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    // Brown noise through highpass should be very quiet
    assert!(rms < 0.3,
        "Highpassed brown noise should be attenuated, got RMS: {}",
        rms);

    println!("Highpassed brown noise RMS: {}", rms);
}

#[test]
fn test_brown_noise_very_low_filter() {
    // Brown noise with very low cutoff creates deep rumble
    let code = r#"
        tempo: 0.5
        ~filtered $ brown_noise # rlpf 60 1.0
        out $ ~filtered * 0.5
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Very low-passed brown noise should work, RMS: {}", rms);
    println!("Very low-passed brown noise RMS: {}", rms);
}

// ========== Amplitude Control ==========

#[test]
fn test_brown_noise_amplitude_scaling() {
    let code = r#"
        tempo: 0.5
        out $ brown_noise * 0.1
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    // Scaled down noise should have lower RMS
    assert!(rms < 0.15 && rms > 0.01,
        "Scaled brown noise should have appropriate RMS, got {}",
        rms);

    println!("Scaled noise RMS: {}", rms);
}

#[test]
fn test_brown_noise_envelope_shaping() {
    let code = r#"
        tempo: 0.5
        ~env $ ad 0.02 0.3
        out $ brown_noise * ~env * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    // Envelope should shape the noise
    assert!(rms > 0.01,
        "Envelope-shaped brown noise should work, RMS: {}",
        rms);

    println!("Envelope-shaped noise RMS: {}", rms);
}

// ========== Stability Tests ==========

#[test]
fn test_brown_noise_no_excessive_clipping() {
    let code = r#"
        tempo: 0.5
        out $ brown_noise * 0.5
    "#;

    let buffer = render_dsl(code, 1.0);
    let max_amplitude = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    assert!(max_amplitude <= 3.0,
        "Brown noise should not excessively clip, max: {}",
        max_amplitude);

    println!("Brown noise max amplitude: {}", max_amplitude);
}

#[test]
fn test_brown_noise_consistent_output() {
    // Generate two separate buffers, they should be different (not stuck)
    let code = r#"
        tempo: 0.5
        out $ brown_noise
    "#;

    let buffer1 = render_dsl(code, 0.1);
    let buffer2 = render_dsl(code, 0.1);

    // Buffers should be different (randomness working)
    let mut differences = 0;
    for i in 0..buffer1.len().min(buffer2.len()) {
        if (buffer1[i] - buffer2[i]).abs() > 0.01 {
            differences += 1;
        }
    }

    let diff_ratio = differences as f32 / buffer1.len() as f32;
    assert!(diff_ratio > 0.9,
        "Brown noise should produce different output each time, similarity: {}",
        1.0 - diff_ratio);

    println!("Brown noise difference ratio: {}", diff_ratio);
}

// ========== Bass Energy Tests ==========

#[test]
fn test_brown_noise_bass_dominance() {
    // Brown noise should have very strong bass presence
    let code = r#"
        tempo: 0.5
        out $ brown_noise
    "#;

    let buffer = render_dsl(code, 2.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    // Calculate bass vs treble energy
    let bass: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f < 250.0)
        .map(|(_, m)| m * m)
        .sum();

    let treble: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 4000.0 && **f < 12000.0)
        .map(|(_, m)| m * m)
        .sum();

    let ratio = bass / treble.max(0.001);
    assert!(ratio > 5.0,
        "Brown noise should have very strong bass dominance, bass/treble ratio: {}",
        ratio);

    println!("Bass dominance ratio: {}", ratio);
}
