/// Systematic tests: Biquad Filter
///
/// Tests biquad filter (second-order IIR) with spectral analysis.
/// Biquad is a versatile filter that can implement multiple filter types.
///
/// Key characteristics:
/// - High-quality second-order filtering
/// - Multiple modes: LP, HP, BP, Notch, Peaking, Shelving
/// - Efficient IIR implementation
/// - Pattern-modulated parameters
/// - Based on RBJ Audio EQ Cookbook

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use std::f32::consts::PI;

mod audio_test_utils;
use audio_test_utils::calculate_rms;

/// Helper function to render DSL code to audio buffer
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

// ========== Basic Biquad Tests ==========

#[test]
fn test_biquad_lowpass_compiles() {
    let code = r#"
        tempo: 2.0
        o1: saw 110 # bq_lp 1000 0.7
    "#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);
    assert!(result.is_ok(), "Biquad lowpass should compile: {:?}", result.err());
}

#[test]
fn test_biquad_highpass_compiles() {
    let code = r#"
        tempo: 2.0
        o1: saw 110 # bq_hp 1000 0.7
    "#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);
    assert!(result.is_ok(), "Biquad highpass should compile: {:?}", result.err());
}

#[test]
fn test_biquad_bandpass_compiles() {
    let code = r#"
        tempo: 2.0
        o1: saw 110 # bq_bp 1000 2.0
    "#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);
    assert!(result.is_ok(), "Biquad bandpass should compile: {:?}", result.err());
}

#[test]
fn test_biquad_notch_compiles() {
    let code = r#"
        tempo: 2.0
        o1: saw 110 # bq_notch 1000 2.0
    "#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);
    assert!(result.is_ok(), "Biquad notch should compile: {:?}", result.err());
}

// ========== Lowpass Mode Tests ==========

#[test]
fn test_biquad_lowpass_attenuates_highs() {
    let code = r#"
        tempo: 2.0
        o1: white_noise # bq_lp 1000 0.7
    "#;

    let buffer = render_dsl(code, 2.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    let low_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f < 800.0)
        .map(|(_, m)| m * m)
        .sum();

    let high_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 2000.0)
        .map(|(_, m)| m * m)
        .sum();

    let ratio = low_energy / high_energy;
    assert!(ratio > 2.0,
        "Biquad lowpass should attenuate high frequencies, low/high ratio: {}",
        ratio);

    println!("Biquad LP - Low energy: {}, High energy: {}, Ratio: {}", low_energy, high_energy, ratio);
}

#[test]
fn test_biquad_lowpass_generates_audio() {
    let code = r#"
        tempo: 2.0
        o1: saw 110 # bq_lp 1000 0.7
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "Biquad lowpass should produce audio, got RMS: {}", rms);
    println!("Biquad LP RMS: {}", rms);
}

// ========== Highpass Mode Tests ==========

#[test]
fn test_biquad_highpass_attenuates_lows() {
    let code = r#"
        tempo: 2.0
        o1: white_noise # bq_hp 1000 0.7
    "#;

    let buffer = render_dsl(code, 2.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    let low_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f < 500.0)
        .map(|(_, m)| m * m)
        .sum();

    let high_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 2000.0)
        .map(|(_, m)| m * m)
        .sum();

    let ratio = high_energy / low_energy;
    assert!(ratio > 2.0,
        "Biquad highpass should attenuate low frequencies, high/low ratio: {}",
        ratio);

    println!("Biquad HP - Low energy: {}, High energy: {}, Ratio: {}", low_energy, high_energy, ratio);
}

#[test]
fn test_biquad_highpass_generates_audio() {
    let code = r#"
        tempo: 2.0
        o1: saw 110 # bq_hp 500 0.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Biquad highpass should produce audio, got RMS: {}", rms);
    println!("Biquad HP RMS: {}", rms);
}

// ========== Bandpass Mode Tests ==========

#[test]
fn test_biquad_bandpass_passes_band() {
    let code = r#"
        tempo: 2.0
        o1: white_noise # bq_bp 1000 3.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    let center_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 800.0 && **f < 1200.0)
        .map(|(_, m)| m * m)
        .sum();

    let side_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f < 500.0 || **f > 2000.0)
        .map(|(_, m)| m * m)
        .sum();

    // With high Q, center should dominate
    assert!(center_energy > side_energy * 0.5,
        "Biquad bandpass should pass center frequencies");

    println!("Biquad BP - Center: {}, Side: {}", center_energy, side_energy);
}

#[test]
fn test_biquad_bandpass_generates_audio() {
    let code = r#"
        tempo: 2.0
        o1: saw 110 # bq_bp 1000 2.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Biquad bandpass should produce audio, got RMS: {}", rms);
    println!("Biquad BP RMS: {}", rms);
}

// ========== Notch Mode Tests ==========

#[test]
fn test_biquad_notch_rejects_center() {
    let code = r#"
        tempo: 2.0
        o1: white_noise # bq_notch 1000 3.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    let center_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 900.0 && **f < 1100.0)
        .map(|(_, m)| m * m)
        .sum();

    let side_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| (**f > 400.0 && **f < 700.0) || (**f > 1500.0 && **f < 2000.0))
        .map(|(_, m)| m * m)
        .sum();

    let ratio = side_energy / center_energy.max(0.001);
    assert!(ratio > 1.5,
        "Biquad notch should reject center frequency, side/center ratio: {}",
        ratio);

    println!("Biquad Notch - Center: {}, Side: {}, Ratio: {}", center_energy, side_energy, ratio);
}

#[test]
fn test_biquad_notch_generates_audio() {
    let code = r#"
        tempo: 2.0
        o1: saw 110 # bq_notch 440 2.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "Biquad notch should produce audio, got RMS: {}", rms);
    println!("Biquad Notch RMS: {}", rms);
}

// ========== Pattern Modulation Tests ==========

#[test]
fn test_biquad_pattern_frequency() {
    let code = r#"
        tempo: 2.0
        ~lfo: sine 4 * 500 + 1000
        o1: saw 110 # bq_lp ~lfo 0.7
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1,
        "Biquad with pattern-modulated frequency should work, RMS: {}",
        rms);

    println!("Biquad pattern frequency RMS: {}", rms);
}

#[test]
fn test_biquad_pattern_q() {
    let code = r#"
        tempo: 2.0
        ~lfo: sine 2 * 2.0 + 3.0
        o1: saw 110 # bq_lp 1000 ~lfo
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1,
        "Biquad with pattern-modulated Q should work, RMS: {}",
        rms);

    println!("Biquad pattern Q RMS: {}", rms);
}

// ========== Stability Tests ==========

#[test]
fn test_biquad_no_clipping() {
    let code = r#"
        tempo: 2.0
        o1: saw 110 # bq_lp 1000 10.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let max_amplitude = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    // High Q can boost near cutoff
    assert!(max_amplitude <= 5.0,
        "Biquad should not excessively clip, max: {}",
        max_amplitude);

    println!("Biquad high Q peak: {}", max_amplitude);
}

#[test]
fn test_biquad_no_dc_offset() {
    let code = r#"
        tempo: 2.0
        o1: saw 110 # bq_lp 500 1.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let mean: f32 = buffer.iter().sum::<f32>() / buffer.len() as f32;

    assert!(mean.abs() < 0.02, "Biquad should have no DC offset, got {}", mean);
    println!("Biquad DC offset: {}", mean);
}

// ========== Cascaded Filters ==========

#[test]
fn test_biquad_cascade() {
    let code = r#"
        tempo: 2.0
        o1: saw 110 # bq_lp 1000 0.7 # bq_hp 200 0.7
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Cascaded biquad should work, RMS: {}", rms);
    println!("Cascaded biquad RMS: {}", rms);
}

// ========== Musical Applications ==========

#[test]
fn test_biquad_resonant_bass() {
    let code = r#"
        tempo: 2.0
        o1: saw 55 # bq_lp 300 5.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "Biquad resonant bass should work, RMS: {}", rms);
    println!("Biquad resonant bass RMS: {}", rms);
}

#[test]
fn test_biquad_filter_sweep() {
    let code = r#"
        tempo: 2.0
        ~sweep: line 200 5000
        o1: saw 55 # bq_lp ~sweep 1.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "Biquad filter sweep should work, RMS: {}", rms);
    println!("Biquad sweep RMS: {}", rms);
}

// ========== Edge Cases ==========

#[test]
fn test_biquad_very_low_frequency() {
    let code = r#"
        tempo: 2.0
        o1: white_noise # bq_lp 50 0.7
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.01, "Biquad should work at very low frequencies, RMS: {}", rms);
    println!("Biquad very low cutoff RMS: {}", rms);
}

#[test]
fn test_biquad_very_high_frequency() {
    let code = r#"
        tempo: 2.0
        o1: white_noise # bq_lp 15000 0.7
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "Biquad should work at very high frequencies, RMS: {}", rms);
    println!("Biquad very high cutoff RMS: {}", rms);
}

#[test]
fn test_biquad_low_q() {
    let code = r#"
        tempo: 2.0
        o1: saw 110 # bq_lp 1000 0.1
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "Biquad should work with low Q, RMS: {}", rms);
    println!("Biquad low Q RMS: {}", rms);
}

#[test]
fn test_biquad_high_q() {
    let code = r#"
        tempo: 2.0
        o1: saw 110 # bq_bp 1000 20.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.01, "Biquad should work with high Q, RMS: {}", rms);
    println!("Biquad high Q RMS: {}", rms);
}
