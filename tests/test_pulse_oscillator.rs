/// Systematic tests: Pulse Oscillator (Variable Pulse Width)
///
/// Tests pulse wave oscillator with spectral analysis and audio verification.
/// Pulse waves create rich harmonic content with variable timbre based on pulse width.
///
/// Key characteristics:
/// - Output: +1 when phase < width, -1 otherwise
/// - Width = 0.5 creates square wave (only odd harmonics)
/// - Width ≠ 0.5 creates asymmetric waveform (even + odd harmonics)
/// - Pulse width modulation (PWM) creates chorusing/detuning effect
/// - All parameters pattern-modulated
/// - Used for analog synth bass, pads, PWM effects

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

// ========== Basic Pulse Tests ==========

#[test]
fn test_pulse_compiles() {
    let code = r#"
        tempo: 0.5
        o1: pulse 440 0.5
    "#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);
    assert!(result.is_ok(), "Pulse should compile: {:?}", result.err());
}

#[test]
fn test_pulse_generates_audio() {
    let code = r#"
        tempo: 0.5
        o1: pulse 440 0.5 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "Pulse should produce audio, got RMS: {}", rms);
    println!("Pulse RMS: {}", rms);
}

// ========== Pulse Width Tests ==========

#[test]
fn test_pulse_width_50_is_square() {
    // Width = 0.5 should produce square wave (only odd harmonics)
    let code = r#"
        tempo: 0.5
        o1: pulse 440 0.5 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    // Find fundamental (440 Hz)
    let mut fundamental_mag = 0.0f32;
    for (i, &freq) in frequencies.iter().enumerate() {
        if (freq - 440.0).abs() < 20.0 {
            fundamental_mag = fundamental_mag.max(magnitudes[i]);
        }
    }

    assert!(fundamental_mag > 0.1,
        "Square wave should have strong fundamental, got {}",
        fundamental_mag);

    println!("Square wave fundamental: {}", fundamental_mag);
}

#[test]
fn test_pulse_width_narrow() {
    // Narrow pulse (width = 0.1) creates brighter sound
    let code = r#"
        tempo: 0.5
        o1: pulse 440 0.1 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Narrow pulse should work, RMS: {}", rms);
    println!("Narrow pulse (0.1) RMS: {}", rms);
}

#[test]
fn test_pulse_width_wide() {
    // Wide pulse (width = 0.9) creates different timbre
    let code = r#"
        tempo: 0.5
        o1: pulse 440 0.9 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Wide pulse should work, RMS: {}", rms);
    println!("Wide pulse (0.9) RMS: {}", rms);
}

// ========== PWM (Pulse Width Modulation) Tests ==========

#[test]
fn test_pwm_slow_modulation() {
    // Slow PWM creates chorusing effect
    let code = r#"
        tempo: 0.5
        ~width: sine 0.5 * 0.2 + 0.5
        o1: pulse 440 ~width * 0.3
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Slow PWM should work, RMS: {}", rms);
    println!("Slow PWM RMS: {}", rms);
}

#[test]
fn test_pwm_fast_modulation() {
    // Fast PWM creates complex timbre
    let code = r#"
        tempo: 0.5
        ~width: sine 4 * 0.25 + 0.5
        o1: pulse 440 ~width * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Fast PWM should work, RMS: {}", rms);
    println!("Fast PWM RMS: {}", rms);
}

// ========== Musical Applications ==========

#[test]
fn test_pulse_bass() {
    // Classic analog bass sound: square wave with envelope
    let code = r#"
        tempo: 0.5
        ~env: adsr 0.01 0.1 0.5 0.2
        o1: pulse 55 0.5 * ~env * 0.4
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.02, "Pulse bass should work, RMS: {}", rms);
    println!("Pulse bass RMS: {}", rms);
}

#[test]
fn test_pulse_pad() {
    // Pad sound with slow PWM
    let code = r#"
        tempo: 1.0
        ~width: sine 0.2 * 0.15 + 0.5
        ~env: adsr 0.5 0.2 0.8 1.0
        o1: pulse 220 ~width * ~env * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.02, "Pulse pad should work, RMS: {}", rms);
    println!("Pulse pad RMS: {}", rms);
}

#[test]
fn test_pulse_lead() {
    // Lead synth: narrow pulse, fast attack
    let code = r#"
        tempo: 0.5
        ~env: adsr 0.001 0.05 0.7 0.1
        o1: pulse 440 0.3 * ~env * 0.4
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Pulse lead should work, RMS: {}", rms);
    println!("Pulse lead RMS: {}", rms);
}

// ========== Pattern Modulation Tests ==========

#[test]
fn test_pulse_pattern_frequency() {
    let code = r#"
        tempo: 0.5
        ~freq: sine 1 * 50 + 220
        o1: pulse ~freq 0.5 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05,
        "Pulse with pattern-modulated frequency should work, RMS: {}",
        rms);

    println!("Pattern frequency RMS: {}", rms);
}

#[test]
fn test_pulse_pattern_width() {
    let code = r#"
        tempo: 0.5
        ~width_pat: sine 2 * 0.3 + 0.5
        o1: pulse 440 ~width_pat * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05,
        "Pulse with pattern-modulated width should work, RMS: {}",
        rms);

    println!("Pattern width RMS: {}", rms);
}

// ========== Filtered Pulse Tests ==========

#[test]
fn test_pulse_lowpass_filter() {
    // Lowpassed pulse creates mellower sound
    let code = r#"
        tempo: 0.5
        ~filtered: pulse 220 0.5 # rlpf 1000 2.0
        o1: ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.01, "Filtered pulse should work, RMS: {}", rms);
    println!("Lowpassed pulse RMS: {}", rms);
}

#[test]
fn test_pulse_resonant_filter() {
    // Pulse through resonant filter
    let code = r#"
        tempo: 0.5
        ~env: adsr 0.01 0.2 0.3 0.2
        ~cutoff: ~env * 3000 + 200
        ~synth: pulse 110 0.5 # rlpf ~cutoff 8.0
        o1: ~synth * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.005, "Pulse with resonant filter should work, RMS: {}", rms);
    println!("Pulse + resonant filter RMS: {}", rms);
}

// ========== Edge Cases ==========

#[test]
fn test_pulse_very_narrow() {
    // Very narrow pulse (width = 0.01)
    let code = r#"
        tempo: 0.5
        o1: pulse 440 0.01 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.01, "Very narrow pulse should work, RMS: {}", rms);
    println!("Very narrow pulse RMS: {}", rms);
}

#[test]
fn test_pulse_very_wide() {
    // Very wide pulse (width = 0.99)
    let code = r#"
        tempo: 0.5
        o1: pulse 440 0.99 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.01, "Very wide pulse should work, RMS: {}", rms);
    println!("Very wide pulse RMS: {}", rms);
}

#[test]
fn test_pulse_low_frequency() {
    let code = r#"
        tempo: 0.5
        o1: pulse 55 0.5 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.03, "Low frequency pulse should work, RMS: {}", rms);
    println!("Low frequency pulse RMS: {}", rms);
}

#[test]
fn test_pulse_high_frequency() {
    let code = r#"
        tempo: 0.5
        o1: pulse 4000 0.5 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "High frequency pulse should work, RMS: {}", rms);
    println!("High frequency pulse RMS: {}", rms);
}
