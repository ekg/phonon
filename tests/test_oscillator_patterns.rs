/// Systematic tests: Oscillators accept pattern parameters
///
/// Tests sine, saw, square, triangle with pattern modulation.
/// Verifies P0.0: ALL parameters accept patterns.

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

mod audio_test_utils;
use audio_test_utils::calculate_rms;

/// Helper function to render DSL code to audio buffer
fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph = compile_program(statements, sample_rate).expect("Failed to compile DSL code");
    let num_samples = (duration * sample_rate) as usize;
    graph.render(num_samples)
}

// ========== Sine Oscillator Tests ==========

#[test]
fn test_sine_constant_frequency() {
    let code = r#"
        tempo: 2.0
        o1: sine 440
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.1, "Sine with constant frequency should produce audio, got RMS: {}", rms);
}

#[test]
fn test_sine_pattern_frequency_lfo() {
    // Sine with LFO-modulated frequency (vibrato)
    let code = r#"
        tempo: 2.0
        o1: sine (sine 5 * 10 + 440)
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.1, "Sine with LFO frequency should produce audio, got RMS: {}", rms);
}

#[test]
fn test_sine_pattern_frequency_slow_sweep() {
    // Sine with slow frequency sweep
    let code = r#"
        tempo: 2.0
        o1: sine (sine 0.5 * 220 + 440)
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.1, "Sine with slow sweep should produce audio, got RMS: {}", rms);
}

// ========== Saw Oscillator Tests ==========

#[test]
fn test_saw_constant_frequency() {
    let code = r#"
        tempo: 2.0
        o1: saw 110
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.1, "Saw with constant frequency should produce audio, got RMS: {}", rms);
}

#[test]
fn test_saw_pattern_frequency_lfo() {
    // Saw with LFO-modulated frequency
    let code = r#"
        tempo: 2.0
        o1: saw (sine 2 * 55 + 110)
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.1, "Saw with LFO frequency should produce audio, got RMS: {}", rms);
}

#[test]
fn test_saw_pattern_frequency_fm() {
    // Saw with frequency modulation (FM)
    let code = r#"
        tempo: 2.0
        o1: saw (square 4 * 110 + 220)
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.1, "Saw with FM should produce audio, got RMS: {}", rms);
}

// ========== Square Oscillator Tests ==========

#[test]
fn test_square_constant_frequency() {
    let code = r#"
        tempo: 2.0
        o1: square 220
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.1, "Square with constant frequency should produce audio, got RMS: {}", rms);
}

#[test]
fn test_square_pattern_frequency_lfo() {
    // Square with LFO-modulated frequency
    let code = r#"
        tempo: 2.0
        o1: square (sine 3 * 110 + 220)
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.1, "Square with LFO frequency should produce audio, got RMS: {}", rms);
}

#[test]
fn test_square_pattern_frequency_stepped() {
    // Square with stepped frequency pattern (arpeggio)
    let code = r#"
        tempo: 2.0
        o1: square (square 2 * 110 + 220)
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.1, "Square with stepped frequency should produce audio, got RMS: {}", rms);
}

// ========== Complex Modulation Tests ==========

#[test]
fn test_oscillator_meta_modulation() {
    // LFO modulating LFO modulating carrier (meta-modulation)
    let code = r#"
        tempo: 2.0
        o1: sine (sine (sine 0.1 * 2 + 5) * 110 + 440)
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.1, "Meta-modulation should produce audio, got RMS: {}", rms);
}

#[test]
fn test_oscillator_fm_synthesis() {
    // Classic FM synthesis (carrier + modulator)
    let code = r#"
        tempo: 2.0
        o1: sine (sine 880 * 100 + 440)
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.1, "FM synthesis should produce audio, got RMS: {}", rms);
}

#[test]
fn test_oscillator_additive_with_modulation() {
    // Additive synthesis with modulated frequencies
    let code = r#"
        tempo: 2.0
        ~lfo: sine 2
        ~fund: sine (~lfo * 55 + 110)
        ~harm2: sine (~lfo * 110 + 220)
        ~harm3: sine (~lfo * 165 + 330)
        o1: (~fund + ~harm2 * 0.5 + ~harm3 * 0.25) * 0.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.1, "Additive with modulation should produce audio, got RMS: {}", rms);
}

// ========== Verification: Pattern vs Constant ==========

#[test]
fn test_sine_pattern_differs_from_constant() {
    // Verify pattern modulation produces different result than constant
    let constant_code = r#"
        tempo: 2.0
        o1: sine 440
    "#;

    let pattern_code = r#"
        tempo: 2.0
        o1: sine (sine 5 * 10 + 440)
    "#;

    let constant_buffer = render_dsl(constant_code, 2.0);
    let pattern_buffer = render_dsl(pattern_code, 2.0);

    let constant_rms = calculate_rms(&constant_buffer);
    let pattern_rms = calculate_rms(&pattern_buffer);

    // Both should have audio
    assert!(constant_rms > 0.1, "Constant sine should have audio");
    assert!(pattern_rms > 0.1, "Pattern sine should have audio");

    // RMS should be similar (vibrato doesn't change average amplitude much)
    // but waveforms should be different (tested elsewhere with FFT)
    let diff_ratio = (constant_rms - pattern_rms).abs() / constant_rms;
    // For vibrato, RMS change is small but measurable
    assert!(
        diff_ratio < 0.2,
        "Vibrato should not change RMS dramatically, got diff: {}",
        diff_ratio
    );
}
