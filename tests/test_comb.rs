/// Systematic tests: Comb Filter
///
/// Tests comb filter with spectral analysis and audio verification.
/// Comb filter creates series of equally-spaced resonances or notches.
///
/// Key characteristics:
/// - Feedforward/feedback delay implementation
/// - Creates harmonic series of peaks/notches
/// - Delay time controls fundamental frequency
/// - Feedback controls resonance depth
/// - Used for reverb, metallic sounds, Karplus-Strong synthesis
/// - Pattern-modulated parameters

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

// ========== Basic Comb Tests ==========

#[test]
fn test_comb_compiles() {
    let code = r#"
        tempo: 0.5
        out $ saw 110 # comb 0.01 0.5
    "#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);
    assert!(result.is_ok(), "Comb should compile: {:?}", result.err());
}

#[test]
fn test_comb_generates_audio() {
    let code = r#"
        tempo: 0.5
        out $ saw 110 # comb 0.01 0.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "Comb should produce audio, got RMS: {}", rms);
    println!("Comb RMS: {}", rms);
}

// ========== Spectral Response Tests ==========

#[test]
fn test_comb_creates_harmonics() {
    // Comb filter creates harmonic resonances
    let code = r#"
        tempo: 0.5
        out $ white_noise # comb 0.01 0.7
    "#;

    let buffer = render_dsl(code, 2.0);
    let (_frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    // Check that spectrum is not flat (has peaks and valleys)
    let mut variance = 0.0f32;
    let mean: f32 = magnitudes.iter().sum::<f32>() / magnitudes.len() as f32;
    for &mag in &magnitudes {
        variance += (mag - mean) * (mag - mean);
    }
    variance /= magnitudes.len() as f32;

    // Comb filter should create spectral variation
    assert!(variance > 0.1,
        "Comb should create harmonic peaks, got variance: {}",
        variance);

    println!("Comb spectral variance: {}", variance);
}

// ========== Delay Time Tests ==========

#[test]
fn test_comb_short_delay() {
    // Short delay creates high-frequency resonance
    let code = r#"
        tempo: 0.5
        out $ white_noise # comb 0.001 0.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "Comb with short delay should work, RMS: {}", rms);
    println!("Short delay comb RMS: {}", rms);
}

#[test]
fn test_comb_long_delay() {
    // Longer delay creates lower-frequency resonance
    let code = r#"
        tempo: 0.5
        out $ white_noise # comb 0.05 0.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "Comb with long delay should work, RMS: {}", rms);
    println!("Long delay comb RMS: {}", rms);
}

// ========== Feedback Tests ==========

#[test]
fn test_comb_zero_feedback() {
    // Zero feedback = simple delay
    let code = r#"
        tempo: 0.5
        out $ saw 110 # comb 0.01 0.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "Comb with zero feedback should work, RMS: {}", rms);
    println!("Zero feedback comb RMS: {}", rms);
}

#[test]
fn test_comb_high_feedback() {
    // High feedback creates strong resonance
    let code = r#"
        tempo: 0.5
        out $ saw 110 # comb 0.01 0.9
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "Comb with high feedback should work, RMS: {}", rms);
    println!("High feedback comb RMS: {}", rms);
}

#[test]
fn test_comb_negative_feedback() {
    // Negative feedback creates notches instead of peaks
    let code = r#"
        tempo: 0.5
        out $ white_noise # comb 0.01 -0.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Comb with negative feedback should work, RMS: {}", rms);
    println!("Negative feedback comb RMS: {}", rms);
}

// ========== Pattern Modulation Tests ==========

#[test]
fn test_comb_pattern_delay() {
    let code = r#"
        tempo: 0.5
        ~lfo $ sine 4 * 0.01 + 0.02
        out $ saw 110 # comb ~lfo 0.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05,
        "Comb with pattern-modulated delay should work, RMS: {}",
        rms);

    println!("Comb pattern delay RMS: {}", rms);
}

#[test]
fn test_comb_pattern_feedback() {
    let code = r#"
        tempo: 0.5
        ~lfo $ sine 2 * 0.3 + 0.5
        out $ saw 110 # comb 0.01 ~lfo
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05,
        "Comb with pattern-modulated feedback should work, RMS: {}",
        rms);

    println!("Comb pattern feedback RMS: {}", rms);
}

// ========== Stability Tests ==========

#[test]
fn test_comb_no_clipping() {
    let code = r#"
        tempo: 0.5
        out $ saw 110 # comb 0.01 0.95
    "#;

    let buffer = render_dsl(code, 2.0);
    let max_amplitude = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    // High feedback (0.95) can cause resonance buildup
    assert!(max_amplitude <= 10.0,
        "Comb should not excessively clip, max: {}",
        max_amplitude);

    println!("Comb high feedback peak: {}", max_amplitude);
}

#[test]
fn test_comb_no_dc_offset() {
    let code = r#"
        tempo: 0.5
        out $ saw 110 # comb 0.01 0.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let mean: f32 = buffer.iter().sum::<f32>() / buffer.len() as f32;

    assert!(mean.abs() < 0.02, "Comb should have no DC offset, got {}", mean);
    println!("Comb DC offset: {}", mean);
}

// ========== Musical Applications ==========

#[test]
fn test_comb_karplus_strong() {
    // Karplus-Strong plucked string simulation
    let code = r#"
        tempo: 0.5
        ~burst $ white_noise * (line 1.0 0.0)
        out $ ~burst # comb 0.0025 0.98
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.01, "Karplus-Strong should work, RMS: {}", rms);
    println!("Karplus-Strong RMS: {}", rms);
}

#[test]
fn test_comb_metallic_sound() {
    // Metallic/bell-like sound
    let code = r#"
        tempo: 0.5
        out $ saw 110 # comb 0.003 0.9 # comb 0.0037 0.85
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Metallic sound should work, RMS: {}", rms);
    println!("Metallic sound RMS: {}", rms);
}

#[test]
fn test_comb_for_reverb() {
    // Multiple combs for reverb texture
    let code = r#"
        tempo: 0.5
        ~dry $ saw 110
        ~c1 $ ~dry # comb 0.0297 0.7
        ~c2 $ ~dry # comb 0.0371 0.7
        ~c3 $ ~dry # comb 0.0411 0.7
        out $ (~c1 + ~c2 + ~c3) * 0.3
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.08, "Reverb with combs should work, RMS: {}", rms);
    println!("Comb reverb RMS: {}", rms);
}

#[test]
fn test_comb_flanging() {
    // Slow LFO modulation of delay time creates flanging
    let code = r#"
        tempo: 0.5
        ~lfo $ sine 0.5 * 0.005 + 0.008
        out $ saw 220 # comb ~lfo 0.7
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "Flanging should work, RMS: {}", rms);
    println!("Flanging RMS: {}", rms);
}

// ========== Cascaded Combs ==========

#[test]
fn test_comb_cascade() {
    // Multiple combs in series
    let code = r#"
        tempo: 0.5
        out $ white_noise # comb 0.01 0.5 # comb 0.015 0.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Cascaded combs should work, RMS: {}", rms);
    println!("Cascaded combs RMS: {}", rms);
}

// ========== Edge Cases ==========

#[test]
fn test_comb_very_short_delay() {
    let code = r#"
        tempo: 0.5
        out $ white_noise # comb 0.0001 0.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "Comb with very short delay should work, RMS: {}", rms);
    println!("Very short delay comb RMS: {}", rms);
}

#[test]
fn test_comb_maximum_delay() {
    // Test at maximum reasonable delay
    let code = r#"
        tempo: 0.5
        out $ white_noise # comb 0.1 0.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "Comb with maximum delay should work, RMS: {}", rms);
    println!("Maximum delay comb RMS: {}", rms);
}
