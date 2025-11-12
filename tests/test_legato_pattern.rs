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

#[test]
fn test_legato_constant() {
    // Test that legato with a constant number works
    let code = r#"
        tempo: 2.0
        o1: s "bd*4" $ legato 0.5
    "#;

    let buffer = render_dsl(code, 2.0); // 2 seconds = 4 cycles at tempo 2.0
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.01, "Legato with constant should produce audio, got RMS: {}", rms);
}

#[test]
fn test_legato_pattern() {
    // Test that legato with a pattern works
    let code = r#"
        tempo: 2.0
        o1: s "bd*4" $ legato "0.5 1.5"
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.01, "Legato with pattern should produce audio, got RMS: {}", rms);
}

#[test]
fn test_legato_affects_duration() {
    // Test that different legato values produce different results
    let short_code = r#"
        tempo: 2.0
        o1: s "bd*8" $ legato 0.1
    "#;

    let long_code = r#"
        tempo: 2.0
        o1: s "bd*8" $ legato 1.5
    "#;

    let short_buffer = render_dsl(short_code, 2.0);
    let long_buffer = render_dsl(long_code, 2.0);

    let short_rms = calculate_rms(&short_buffer);
    let long_rms = calculate_rms(&long_buffer);

    // Both should have audio
    assert!(short_rms > 0.01, "Short legato should have audio");
    assert!(long_rms > 0.01, "Long legato should have audio");

    // Longer legato should have higher RMS (more sustained notes)
    assert!(
        long_rms > short_rms * 1.2,
        "Long legato (RMS: {}) should have higher RMS than short legato (RMS: {})",
        long_rms,
        short_rms
    );
}

#[test]
fn test_legato_pattern_alternating() {
    // Test that pattern-based legato alternates correctly
    let alternating_code = r#"
        tempo: 2.0
        o1: s "bd*8" $ legato "0.1 1.5"
    "#;

    let buffer = render_dsl(alternating_code, 2.0);
    let rms = calculate_rms(&buffer);

    // Should produce audio with alternating short and long notes
    assert!(rms > 0.01, "Alternating legato should produce audio");

    // RMS should be between the extremes (0.1 and 1.5)
    // This is a weak test but verifies basic functionality
    assert!(rms > 0.05, "Alternating legato RMS should be above minimum");
}
