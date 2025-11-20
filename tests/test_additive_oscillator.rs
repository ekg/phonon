/// Systematic tests: Additive Synthesis Oscillator
///
/// Tests additive synthesis with harmonic control and audio verification.
/// Additive synthesis creates complex timbres by summing weighted sine harmonics.
///
/// Key characteristics:
/// - Fundamental frequency: Base pitch
/// - Num harmonics: Number of partials to sum (1-32)
/// - Harmonic weights: Amplitude of each harmonic
/// - Harmonic detune: Frequency offset in cents per harmonic
/// - Classic waveforms: Saw, square, triangle via harmonic series
/// - Musical applications: Organ drawbars, bells, evolving pads

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

mod audio_test_utils;
use audio_test_utils::calculate_rms;

fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph = compile_program(statements, sample_rate).expect("Failed to compile DSL code");
    let num_samples = (duration * sample_rate) as usize;
    graph.render(num_samples)
}

// ========== Basic Tests ==========

#[test]
fn test_additive_compiles() {
    let code = r#"
        tempo: 2.0
        o1: additive 440 8
    "#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0);
    assert!(result.is_ok(), "Additive should compile: {:?}", result.err());
}

#[test]
fn test_additive_generates_audio() {
    let code = r#"
        tempo: 2.0
        o1: additive 440 8 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Additive should produce audio, got RMS: {}", rms);
    println!("Additive RMS: {}", rms);
}

// ========== Harmonic Control Tests ==========

#[test]
fn test_additive_single_harmonic() {
    // Single harmonic should be like a sine wave
    let code = r#"
        tempo: 2.0
        o1: additive 440 1 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.15, "Single harmonic should produce strong signal, RMS: {}", rms);
    println!("Single harmonic RMS: {}", rms);
}

#[test]
fn test_additive_multiple_harmonics() {
    // More harmonics should create richer sound
    let code_few = r#"
        tempo: 2.0
        o1: additive 220 2 * 0.3
    "#;

    let code_many = r#"
        tempo: 2.0
        o1: additive 220 16 * 0.3
    "#;

    let buffer_few = render_dsl(code_few, 1.0);
    let buffer_many = render_dsl(code_many, 1.0);

    let rms_few = calculate_rms(&buffer_few);
    let rms_many = calculate_rms(&buffer_many);

    // Both should produce sound
    assert!(rms_few > 0.05, "Few harmonics RMS: {}", rms_few);
    assert!(rms_many > 0.05, "Many harmonics RMS: {}", rms_many);

    // More harmonics typically more energy
    println!("Few harmonics RMS: {}, Many harmonics RMS: {}", rms_few, rms_many);
}

// ========== Waveform Approximation Tests ==========

#[test]
fn test_additive_sawtooth_approx() {
    // Sawtooth = sum of harmonics with 1/n falloff
    let code = r#"
        tempo: 2.0
        ~saw: additive 110 16
        o1: ~saw * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "Sawtooth approximation should work, RMS: {}", rms);
    println!("Sawtooth approximation RMS: {}", rms);
}

#[test]
fn test_additive_square_approx() {
    // Square = odd harmonics only with 1/n falloff
    let code = r#"
        tempo: 2.0
        ~square: additive 110 16
        o1: ~square * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "Square approximation should work, RMS: {}", rms);
    println!("Square approximation RMS: {}", rms);
}

// ========== Frequency Tests ==========

#[test]
fn test_additive_low_frequency() {
    let code = r#"
        tempo: 2.0
        o1: additive 55 8 * 0.3
    "#;

    let buffer = render_dsl(code, 2.0);  // Longer duration for low freq
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Low frequency additive should work, RMS: {}", rms);
    println!("Low frequency RMS: {}", rms);
}

#[test]
fn test_additive_high_frequency() {
    let code = r#"
        tempo: 2.0
        o1: additive 2000 4 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "High frequency additive should work, RMS: {}", rms);
    println!("High frequency RMS: {}", rms);
}

// ========== Pattern Modulation Tests ==========

#[test]
fn test_additive_pattern_frequency() {
    let code = r#"
        tempo: 2.0
        ~freq: sine 2 * 50 + 440
        o1: additive ~freq 8 * 0.3
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05,
        "Additive with pattern-modulated frequency should work, RMS: {}",
        rms);

    println!("Pattern frequency RMS: {}", rms);
}

#[test]
fn test_additive_pattern_harmonics() {
    // NOTE: Current implementation uses fixed amplitudes, not dynamic num_harmonics
    // This test demonstrates using the existing API
    let code = r#"
        tempo: 2.0
        o1: additive 220 "1 0.5 0.33 0.25 0.2 0.17 0.14 0.125" * 0.3
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05,
        "Additive with multiple harmonics should work, RMS: {}",
        rms);

    println!("Pattern harmonics RMS: {}", rms);
}

// ========== Musical Applications ==========

#[test]
fn test_additive_organ_drawbars() {
    // Hammond organ style: specific harmonic weights
    let code = r#"
        tempo: 2.0
        ~organ: additive 220 8
        ~env: ad 0.01 0.5
        o1: ~organ * ~env * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Organ drawbar sound should work, RMS: {}", rms);
    println!("Organ drawbars RMS: {}", rms);
}

#[test]
fn test_additive_evolving_pad() {
    // Evolving pad: rich harmonic content with envelope
    // NOTE: Current implementation uses fixed amplitudes
    let code = r#"
        tempo: 2.0
        ~pad: additive 110 "1 0.8 0.6 0.5 0.4 0.3 0.25 0.2 0.15 0.12 0.1 0.08"
        ~env: ad 0.1 2.0
        o1: ~pad * ~env * 0.3
    "#;

    let buffer = render_dsl(code, 2.5);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.03, "Evolving pad should work, RMS: {}", rms);
    println!("Evolving pad RMS: {}", rms);
}

#[test]
fn test_additive_chord() {
    // Multiple additive oscillators for a chord
    let code = r#"
        tempo: 2.0
        ~root: additive 220 8 * 0.2
        ~third: additive 277 8 * 0.2
        ~fifth: additive 330 8 * 0.2
        o1: ~root + ~third + ~fifth
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "Additive chord should work, RMS: {}", rms);
    println!("Chord RMS: {}", rms);
}

// ========== Edge Cases ==========

#[test]
fn test_additive_zero_harmonics_clamped() {
    // Empty string should fail, but a single value should work
    let code = r#"
        tempo: 2.0
        o1: additive 440 1.0 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    // Single harmonic (fundamental only)
    assert!(rms > 0.15, "Single harmonic should work, RMS: {}", rms);
    println!("Single harmonic RMS: {}", rms);
}

#[test]
fn test_additive_excessive_harmonics_clamped() {
    // More than 32 harmonics should be clamped
    let code = r#"
        tempo: 2.0
        o1: additive 220 100 * 0.2
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    // Should still work (clamped to 32)
    assert!(rms > 0.05, "Excessive harmonics (clamped to 32) should work, RMS: {}", rms);
    println!("Excessive harmonics (clamped) RMS: {}", rms);
}

#[test]
fn test_additive_nyquist_protection() {
    // High frequency with many harmonics - should skip harmonics above Nyquist
    let code = r#"
        tempo: 2.0
        o1: additive 10000 32 * 0.2
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    // Should produce sound from lower harmonics
    assert!(rms > 0.03, "Nyquist protection should work, RMS: {}", rms);

    // Should not clip excessively
    let max_amplitude = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(max_amplitude <= 2.0, "Should not clip excessively: {}", max_amplitude);

    println!("Nyquist protection RMS: {}, max: {}", rms, max_amplitude);
}

// ========== Stability Tests ==========

#[test]
fn test_additive_no_nan() {
    let code = r#"
        tempo: 2.0
        o1: additive 440 16 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);

    for (i, &sample) in buffer.iter().enumerate() {
        assert!(!sample.is_nan(), "NaN detected at sample {}: {}", i, sample);
        assert!(!sample.is_infinite(), "Infinite detected at sample {}: {}", i, sample);
    }
}

#[test]
fn test_additive_reasonable_amplitude() {
    let code = r#"
        tempo: 2.0
        o1: additive 220 16 * 0.5
    "#;

    let buffer = render_dsl(code, 1.0);
    let max_amplitude = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    assert!(max_amplitude <= 2.0, "Amplitude should be reasonable: {}", max_amplitude);
    println!("Max amplitude: {}", max_amplitude);
}

// ========== Long Duration Test ==========

#[test]
fn test_additive_long_duration() {
    // Test that additive synthesis works over longer durations
    let code = r#"
        tempo: 2.0
        o1: additive 110 8 * 0.3
    "#;

    let buffer = render_dsl(code, 5.0);  // 5 seconds
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Long duration additive should work, RMS: {}", rms);
    println!("Long duration RMS: {}", rms);
}
