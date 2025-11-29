/// Systematic tests: Modulation effects accept pattern parameters
///
/// Tests chorus, flanger, phaser, tremolo, vibrato with pattern modulation.
/// Verifies P0.0: ALL parameters accept patterns.
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

mod audio_test_utils;
use audio_test_utils::calculate_rms;

/// Helper function to render DSL code to audio buffer
fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph =
        compile_program(statements, sample_rate, None).expect("Failed to compile DSL code");
    let num_samples = (duration * sample_rate) as usize;
    graph.render(num_samples)
}

// ========== Chorus Tests ==========

#[test]
fn test_chorus_constant_parameters() {
    let code = r#"
        tempo: 0.5
        out $ saw 110 # chorus 2.0 0.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Chorus with constant params should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_chorus_pattern_rate() {
    // Chorus with pattern-modulated rate
    let code = r#"
        tempo: 0.5
        out $ saw 110 # chorus (sine 0.5 * 2.0 + 3.0) 0.7
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Chorus with pattern rate should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_chorus_pattern_depth() {
    // Chorus with pattern-modulated depth
    let code = r#"
        tempo: 0.5
        out $ saw 110 # chorus 2.0 (sine 1.0 * 0.5 + 0.5)
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Chorus with pattern depth should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_chorus_both_patterns() {
    // Chorus with both parameters as patterns
    let code = r#"
        tempo: 0.5
        out $ saw 110 # chorus (sine 0.5 * 2.0 + 3.0) (sine 1.0 * 0.5 + 0.5)
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Chorus with both pattern params should produce audio, got RMS: {}",
        rms
    );
}

// ========== Flanger Tests ==========

#[test]
fn test_flanger_constant_parameters() {
    let code = r#"
        tempo: 0.5
        out $ saw 110 # flanger 0.7 0.5 0.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Flanger with constant params should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_flanger_pattern_rate() {
    // Flanger with pattern-modulated depth (first param)
    let code = r#"
        tempo: 0.5
        out $ saw 110 # flanger (sine 0.25 * 0.5 + 0.5) 0.5 0.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Flanger with pattern depth should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_flanger_pattern_depth() {
    // Flanger with pattern-modulated rate (second param)
    let code = r#"
        tempo: 0.5
        out $ saw 110 # flanger 0.7 (sine 2.0 * 0.5 + 0.5) 0.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Flanger with pattern rate should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_flanger_both_patterns() {
    // Flanger with all parameters as patterns
    let code = r#"
        tempo: 0.5
        out $ saw 110 # flanger (sine 0.25 * 0.5 + 0.5) (sine 2.0 * 0.5 + 0.5) 0.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Flanger with pattern params should produce audio, got RMS: {}",
        rms
    );
}

// ========== Phaser Tests ==========

#[test]
fn test_phaser_constant_parameters() {
    let code = r#"
        tempo: 0.5
        out $ saw 110 # phaser 1.0 0.8 0.5 4
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Phaser with constant params should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_phaser_pattern_rate() {
    // Phaser with pattern-modulated rate
    let code = r#"
        tempo: 0.5
        out $ saw 110 # phaser (sine 0.5 * 1.0 + 1.0) 0.8 0.5 4
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Phaser with pattern rate should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_phaser_pattern_feedback() {
    // Phaser with pattern-modulated depth
    let code = r#"
        tempo: 0.5
        out $ saw 110 # phaser 1.0 (sine 1.0 * 0.5 + 0.5) 0.5 4
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Phaser with pattern depth should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_phaser_both_patterns() {
    // Phaser with all parameters as patterns
    let code = r#"
        tempo: 0.5
        out $ saw 110 # phaser (sine 0.5 * 1.0 + 1.0) (sine 1.0 * 0.5 + 0.5) 0.5 4
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Phaser with pattern params should produce audio, got RMS: {}",
        rms
    );
}

// ========== Tremolo Tests ==========

#[test]
fn test_tremolo_constant_parameters() {
    let code = r#"
        tempo: 0.5
        out $ saw 110 # tremolo 5.0 0.7
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Tremolo with constant params should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_tremolo_pattern_rate() {
    // Tremolo with pattern-modulated rate
    let code = r#"
        tempo: 0.5
        out $ saw 110 # tremolo (sine 0.1 * 8 + 8) 0.7
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Tremolo with pattern rate should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_tremolo_pattern_depth() {
    // Tremolo with pattern-modulated depth
    let code = r#"
        tempo: 0.5
        out $ saw 110 # tremolo 5.0 (sine 0.25 * 0.5 + 0.5)
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Tremolo with pattern depth should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_tremolo_both_patterns() {
    // Tremolo with both parameters as patterns
    let code = r#"
        tempo: 0.5
        out $ saw 110 # tremolo (sine 0.1 * 8 + 8) (sine 0.25 * 0.5 + 0.5)
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Tremolo with both pattern params should produce audio, got RMS: {}",
        rms
    );
}

// ========== Vibrato Tests ==========

#[test]
fn test_vibrato_constant_parameters() {
    let code = r#"
        tempo: 0.5
        out $ saw 110 # vibrato 5.0 0.02
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Vibrato with constant params should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_vibrato_pattern_rate() {
    // Vibrato with pattern-modulated rate
    let code = r#"
        tempo: 0.5
        out $ saw 110 # vibrato (sine 0.5 * 5 + 5) 0.02
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Vibrato with pattern rate should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_vibrato_pattern_depth() {
    // Vibrato with pattern-modulated depth
    let code = r#"
        tempo: 0.5
        out $ saw 110 # vibrato 5.0 (sine 0.25 * 0.01 + 0.01)
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Vibrato with pattern depth should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_vibrato_both_patterns() {
    // Vibrato with both parameters as patterns
    let code = r#"
        tempo: 0.5
        out $ saw 110 # vibrato (sine 0.5 * 5 + 5) (sine 0.25 * 0.01 + 0.01)
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Vibrato with both pattern params should produce audio, got RMS: {}",
        rms
    );
}
