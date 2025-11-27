/// Systematic tests: Dynamics effects accept pattern parameters
///
/// Tests compressor and bitcrush with pattern modulation.
/// Verifies P0.0: ALL parameters accept patterns.

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

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

// ========== Compressor Tests ==========

#[test]
fn test_compressor_constant_parameters() {
    let code = r#"
        tempo: 0.5
        o1: saw 110 # compressor -12 4.0 0.01 0.1 1.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.01, "Compressor with constant params should produce audio, got RMS: {}", rms);
}

#[test]
fn test_compressor_pattern_threshold() {
    // Compressor with pattern-modulated threshold
    let code = r#"
        tempo: 0.5
        o1: saw 110 # compressor (sine 0.5 * -20 + -10) 4.0 0.01 0.1 1.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.01, "Compressor with pattern threshold should produce audio, got RMS: {}", rms);
}

#[test]
fn test_compressor_pattern_ratio() {
    // Compressor with pattern-modulated ratio
    let code = r#"
        tempo: 0.5
        o1: saw 110 # compressor -12 (sine 1.0 * 4 + 4) 0.01 0.1 1.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.01, "Compressor with pattern ratio should produce audio, got RMS: {}", rms);
}

#[test]
fn test_compressor_pattern_attack() {
    // Compressor with pattern-modulated attack
    let code = r#"
        tempo: 0.5
        o1: saw 110 # compressor -12 4.0 (sine 2.0 * 0.01 + 0.01) 0.1 1.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.01, "Compressor with pattern attack should produce audio, got RMS: {}", rms);
}

#[test]
fn test_compressor_pattern_release() {
    // Compressor with pattern-modulated release
    let code = r#"
        tempo: 0.5
        o1: saw 110 # compressor -12 4.0 0.01 (sine 1.0 * 0.1 + 0.1) 1.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.01, "Compressor with pattern release should produce audio, got RMS: {}", rms);
}

#[test]
fn test_compressor_all_patterns() {
    // Compressor with all parameters as patterns
    let code = r#"
        tempo: 0.5
        o1: saw 110 # compressor (sine 0.5 * -20 + -10) (sine 1.0 * 4 + 4) (sine 2.0 * 0.01 + 0.01) (sine 1.0 * 0.1 + 0.1) 1.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.01, "Compressor with all pattern params should produce audio, got RMS: {}", rms);
}

// ========== Bitcrush Tests ==========

#[test]
fn test_bitcrush_constant_bits() {
    let code = r#"
        tempo: 0.5
        o1: saw 110 # bitcrush 8 44100
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.01, "Bitcrush with constant bits should produce audio, got RMS: {}", rms);
}

#[test]
fn test_bitcrush_pattern_bits() {
    // Bitcrush with pattern-modulated bit depth
    let code = r#"
        tempo: 0.5
        o1: saw 110 # bitcrush (sine 0.5 * 8 + 8) 44100
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.01, "Bitcrush with pattern bits should produce audio, got RMS: {}", rms);
}

#[test]
fn test_bitcrush_constant_sample_rate() {
    let code = r#"
        tempo: 0.5
        o1: saw 110 # bitcrush 8 22050
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.01, "Bitcrush with constant sample_rate should produce audio, got RMS: {}", rms);
}

#[test]
fn test_bitcrush_pattern_sample_rate() {
    // Bitcrush with pattern-modulated sample rate
    let code = r#"
        tempo: 0.5
        o1: saw 110 # bitcrush 8 (sine 1.0 * 20000 + 22050)
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.01, "Bitcrush with pattern sample_rate should produce audio, got RMS: {}", rms);
}

#[test]
fn test_bitcrush_both_patterns() {
    // Bitcrush with both parameters as patterns
    let code = r#"
        tempo: 0.5
        o1: saw 110 # bitcrush (sine 0.5 * 8 + 8) 44100   -- FIXED: Added sample_rate
        o1: saw 110 # bitcrush (sine 0.5 * 8 + 8) (sine 1.0 * 20000 + 22050)
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.01, "Bitcrush with both pattern params should produce audio, got RMS: {}", rms);
}

// ========== Verification: Pattern vs Constant ==========

#[test]
fn test_compressor_pattern_differs_from_constant() {
    // Verify pattern modulation produces different result than constant
    let constant_code = r#"
        tempo: 0.5
        o1: saw 110 # compressor -12 4.0 0.01 0.1 1.0
    "#;

    let pattern_code = r#"
        tempo: 0.5
        o1: saw 110 # compressor (sine 0.5 * -20 + -10) 4.0 0.01 0.1 1.0
    "#;

    let constant_buffer = render_dsl(constant_code, 2.0);
    let pattern_buffer = render_dsl(pattern_code, 2.0);

    let constant_rms = calculate_rms(&constant_buffer);
    let pattern_rms = calculate_rms(&pattern_buffer);

    // Both should have audio
    assert!(constant_rms > 0.01, "Constant compressor should have audio");
    assert!(pattern_rms > 0.01, "Pattern compressor should have audio");

    // They should be different (at least 1% difference)
    let diff_ratio = (constant_rms - pattern_rms).abs() / constant_rms;
    assert!(
        diff_ratio > 0.01,
        "Pattern modulation should differ from constant, const RMS: {}, pattern RMS: {}, diff: {}",
        constant_rms,
        pattern_rms,
        diff_ratio
    );
}

#[test]
fn test_bitcrush_pattern_differs_from_constant() {
    // Verify pattern modulation produces different result than constant
    let constant_code = r#"
        tempo: 0.5
        o1: saw 110 # bitcrush 8 44100
    "#;

    let pattern_code = r#"
        tempo: 0.5
        o1: saw 110 # bitcrush (sine 0.5 * 8 + 8) 44100
    "#;

    let constant_buffer = render_dsl(constant_code, 2.0);
    let pattern_buffer = render_dsl(pattern_code, 2.0);

    let constant_rms = calculate_rms(&constant_buffer);
    let pattern_rms = calculate_rms(&pattern_buffer);

    // Both should have audio
    assert!(constant_rms > 0.01, "Constant bitcrush should have audio");
    assert!(pattern_rms > 0.01, "Pattern bitcrush should have audio");

    // They should be different (at least 1% difference)
    let diff_ratio = (constant_rms - pattern_rms).abs() / constant_rms;
    assert!(
        diff_ratio > 0.01,
        "Pattern modulation should differ from constant, const RMS: {}, pattern RMS: {}, diff: {}",
        constant_rms,
        pattern_rms,
        diff_ratio
    );
}
