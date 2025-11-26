/// Systematic tests: FM (Frequency Modulation) Oscillator
///
/// Tests FM synthesis with spectral analysis and audio verification.
/// FM creates complex harmonic and inharmonic timbres by modulating carrier frequency.
///
/// Key characteristics:
/// - Carrier frequency: Base pitch
/// - Modulator frequency: Frequency of modulation
/// - Modulation index: Depth of modulation (brightness)
/// - C:M ratio determines harmonic/inharmonic spectrum
/// - All parameters pattern-modulated
/// - Used for bells, brass, electric piano, complex pads

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

// ========== Basic FM Tests ==========

#[test]
fn test_fm_compiles() {
    let code = r#"
        tempo: 2.0
        o1: fm 440 220 1.0
    "#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0);
    assert!(result.is_ok(), "FM should compile: {:?}", result.err());
}

#[test]
fn test_fm_generates_audio() {
    let code = r#"
        tempo: 2.0
        o1: fm 440 220 1.0 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "FM should produce audio, got RMS: {}", rms);
    println!("FM RMS: {}", rms);
}

// ========== Modulation Index Tests ==========

#[test]
fn test_fm_zero_index_is_sine() {
    // Index=0 should produce pure sine wave at carrier frequency
    let code = r#"
        tempo: 2.0
        o1: fm 440 220 0.0 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    // Find peak around 440Hz
    let mut peak_freq = 0.0f32;
    let mut peak_mag = 0.0f32;
    for (i, &freq) in frequencies.iter().enumerate() {
        if freq > 400.0 && freq < 480.0 {
            if magnitudes[i] > peak_mag {
                peak_mag = magnitudes[i];
                peak_freq = freq;
            }
        }
    }

    assert!((peak_freq - 440.0).abs() < 20.0,
        "FM with index=0 should peak near 440Hz, got {}Hz",
        peak_freq);

    println!("Zero index peak: {}Hz", peak_freq);
}

#[test]
fn test_fm_low_index_bright() {
    // Low index (1.0) creates bright but simple timbre
    let code = r#"
        tempo: 2.0
        o1: fm 440 220 1.0 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "FM with low index should work, RMS: {}", rms);
    println!("Low index RMS: {}", rms);
}

#[test]
fn test_fm_high_index_complex() {
    // High index (5.0) creates complex timbre with many sidebands
    let code = r#"
        tempo: 2.0
        o1: fm 440 220 5.0 * 0.2
    "#;

    let buffer = render_dsl(code, 1.0);
    let (_frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    // High index should create spectral energy across wide range
    let total_energy: f32 = magnitudes.iter().map(|m| m * m).sum();
    assert!(total_energy > 0.01,
        "FM with high index should have spectral energy, got {}",
        total_energy);

    println!("High index total energy: {}", total_energy);
}

// ========== C:M Ratio Tests (Harmonic vs Inharmonic) ==========

#[test]
fn test_fm_harmonic_1_1_ratio() {
    // C:M = 1:1 produces harmonic spectrum
    let code = r#"
        tempo: 2.0
        o1: fm 440 440 2.0 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "FM 1:1 ratio should work, RMS: {}", rms);
    println!("1:1 ratio RMS: {}", rms);
}

#[test]
fn test_fm_harmonic_2_1_ratio() {
    // C:M = 2:1 produces harmonic spectrum
    let code = r#"
        tempo: 2.0
        o1: fm 440 220 2.0 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "FM 2:1 ratio should work, RMS: {}", rms);
    println!("2:1 ratio RMS: {}", rms);
}

#[test]
fn test_fm_inharmonic_ratio() {
    // C:M = irrational produces inharmonic (bell-like) spectrum
    let code = r#"
        tempo: 2.0
        o1: fm 440 314.159 3.0 * 0.2
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "FM inharmonic ratio should work, RMS: {}", rms);
    println!("Inharmonic ratio RMS: {}", rms);
}

// ========== Musical Applications ==========

#[test]
fn test_fm_electric_piano() {
    // Electric piano: C:M = 1:1 or 2:1, moderate index
    let code = r#"
        tempo: 2.0
        ~env: ad 0.001 0.5
        o1: fm 220 220 2.0 * ~env * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.03, "FM electric piano should work, RMS: {}", rms);
    println!("Electric piano RMS: {}", rms);
}

#[test]
fn test_fm_bell_sound() {
    // Bell sound: inharmonic ratio, envelope on index
    let code = r#"
        tempo: 2.0
        ~env: ad 0.001 1.0
        ~index: ~env * 8.0
        o1: fm 440 550 ~index * ~env * 0.2
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.03, "FM bell should work, RMS: {}", rms);
    println!("Bell RMS: {}", rms);
}

#[test]
fn test_fm_brass_sound() {
    // Brass: harmonic ratio, index varies with dynamics
    let code = r#"
        tempo: 2.0
        ~env: adsr 0.05 0.1 0.7 0.2
        ~index: ~env * 3.0 + 1.0
        o1: fm 220 220 ~index * ~env * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "FM brass should work, RMS: {}", rms);
    println!("Brass RMS: {}", rms);
}

#[test]
fn test_fm_bass_sound() {
    // FM bass: low carrier, moderate C:M ratio
    let code = r#"
        tempo: 2.0
        ~env: ad 0.01 0.3
        o1: fm 55 82.5 2.5 * ~env * 0.4
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "FM bass should work, RMS: {}", rms);
    println!("FM bass RMS: {}", rms);
}

// ========== Pattern Modulation Tests ==========

#[test]
fn test_fm_pattern_carrier() {
    let code = r#"
        tempo: 2.0
        ~carrier: sine 2 * 100 + 440
        o1: fm ~carrier 220 2.0 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05,
        "FM with pattern-modulated carrier should work, RMS: {}",
        rms);

    println!("Pattern carrier RMS: {}", rms);
}

#[test]
fn test_fm_pattern_modulator() {
    let code = r#"
        tempo: 2.0
        ~modulator: sine 1 * 50 + 200
        o1: fm 440 ~modulator 2.0 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05,
        "FM with pattern-modulated modulator should work, RMS: {}",
        rms);

    println!("Pattern modulator RMS: {}", rms);
}

#[test]
fn test_fm_pattern_index() {
    let code = r#"
        tempo: 2.0
        ~index: sine 1 * 2.0 + 3.0
        o1: fm 440 220 ~index * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05,
        "FM with pattern-modulated index should work, RMS: {}",
        rms);

    println!("Pattern index RMS: {}", rms);
}

// ========== Edge Cases ==========

#[test]
fn test_fm_very_low_frequencies() {
    let code = r#"
        tempo: 2.0
        o1: fm 20 10 1.0 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "FM with very low frequencies should work, RMS: {}", rms);
    println!("Very low frequencies RMS: {}", rms);
}

#[test]
fn test_fm_high_carrier() {
    let code = r#"
        tempo: 2.0
        o1: fm 8000 4000 1.0 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "FM with high carrier should work, RMS: {}", rms);
    println!("High carrier RMS: {}", rms);
}

#[test]
fn test_fm_extreme_index() {
    // Very high index creates very complex spectrum
    let code = r#"
        tempo: 2.0
        o1: fm 440 220 20.0 * 0.1
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.01, "FM with extreme index should work, RMS: {}", rms);
    println!("Extreme index RMS: {}", rms);
}

// ========== Stability Tests ==========

#[test]
fn test_fm_no_clipping() {
    let code = r#"
        tempo: 2.0
        o1: fm 440 220 5.0 * 0.5
    "#;

    let buffer = render_dsl(code, 1.0);
    let max_amplitude = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    assert!(max_amplitude <= 1.5,
        "FM should not excessively clip, max: {}",
        max_amplitude);

    println!("FM max amplitude: {}", max_amplitude);
}
