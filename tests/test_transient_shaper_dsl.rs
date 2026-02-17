/// Integration tests for transient_shaper DSL keyword
///
/// Tests that the transient_shaper/tshaper effect can be used from DSL syntax
/// and produces correct audio modifications.
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

fn calculate_rms(buffer: &[f32]) -> f32 {
    if buffer.is_empty() {
        return 0.0;
    }
    let sum: f32 = buffer.iter().map(|x| x * x).sum();
    (sum / buffer.len() as f32).sqrt()
}

/// Helper function to render DSL code to audio buffer
fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph =
        compile_program(statements, sample_rate, None).expect("Failed to compile DSL code");
    let num_samples = (duration * sample_rate) as usize;
    graph.render(num_samples)
}

#[test]
fn test_transient_shaper_neutral_produces_audio() {
    // Neutral settings (0 dB attack, 0 dB sustain) should pass through audio
    let code = r#"
        tempo: 0.5
        out $ saw 110 # transient_shaper 0 0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Transient shaper with neutral settings should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_transient_shaper_boost_attack() {
    // Boosting attack should produce audio
    let code = r#"
        tempo: 0.5
        out $ saw 110 # transient_shaper 12 0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Transient shaper with boosted attack should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_transient_shaper_boost_sustain() {
    // Boosting sustain should produce audio
    let code = r#"
        tempo: 0.5
        out $ saw 110 # transient_shaper 0 12
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Transient shaper with boosted sustain should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_transient_shaper_reduce_both() {
    // Reducing both should still produce audio (just quieter)
    let code = r#"
        tempo: 0.5
        out $ saw 110 # transient_shaper -12 -12
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.001,
        "Transient shaper with reduced attack+sustain should still produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_tshaper_alias() {
    // Test that the 'tshaper' alias works
    let code = r#"
        tempo: 0.5
        out $ saw 110 # tshaper 6 -3
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "tshaper alias should work and produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_transient_shaper_with_pattern_params() {
    // Attack and sustain should accept patterns (Phonon rule: EVERY parameter is a pattern)
    let code = r#"
        tempo: 0.5
        out $ saw 110 # transient_shaper (sine 1.0 * 12) (sine 0.5 * 6)
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Transient shaper with pattern params should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_transient_shaper_on_samples() {
    // Test with sample playback (the typical use case for transient shaping)
    let code = r#"
        tempo: 0.5
        out $ s "bd sn" # transient_shaper 6 0
    "#;

    let buffer = render_dsl(code, 2.0);
    // Sample playback may or may not produce audio depending on sample availability,
    // but the compilation and graph execution should succeed
    assert!(
        buffer.iter().all(|s| s.is_finite()),
        "All samples should be finite"
    );
}

#[test]
fn test_transient_shaper_chained_with_effects() {
    // Test chaining transient shaper with other effects
    let code = r#"
        tempo: 0.5
        out $ saw 110 # transient_shaper 6 -3 # lpf 2000 0.7
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Transient shaper chained with LPF should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_transient_shaper_boost_vs_neutral_level_difference() {
    // Boosting attack should result in different RMS compared to neutral
    let code_neutral = r#"
        tempo: 0.5
        out $ saw 110 # transient_shaper 0 0
    "#;
    let code_boosted = r#"
        tempo: 0.5
        out $ saw 110 # transient_shaper 12 12
    "#;

    let buffer_neutral = render_dsl(code_neutral, 2.0);
    let buffer_boosted = render_dsl(code_boosted, 2.0);

    let rms_neutral = calculate_rms(&buffer_neutral);
    let rms_boosted = calculate_rms(&buffer_boosted);

    // Both should produce audio
    assert!(rms_neutral > 0.01, "Neutral should produce audio");
    assert!(rms_boosted > 0.01, "Boosted should produce audio");

    // Boosted should be louder than neutral
    assert!(
        rms_boosted > rms_neutral,
        "Boosted (+12dB attack+sustain) should be louder than neutral: boosted={}, neutral={}",
        rms_boosted,
        rms_neutral
    );
}
