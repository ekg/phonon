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

#[test]
fn test_legato_constant() {
    // Test that legato with a constant number works
    let code = r#"
        tempo: 0.5
        out $ s "bd*4" $ legato 0.5
    "#;

    let buffer = render_dsl(code, 2.0); // 2 seconds = 4 cycles at tempo 2.0
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Legato with constant should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_legato_pattern() {
    // Test that legato with a pattern works
    let code = r#"
        tempo: 0.5
        out $ s "bd*4" $ legato "0.5 1.5"
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Legato with pattern should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_legato_affects_duration() {
    // Test that different legato values produce different results
    let short_code = r#"
        tempo: 0.5
        out $ s "bd*8" $ legato 0.1
    "#;

    let long_code = r#"
        tempo: 0.5
        out $ s "bd*8" $ legato 1.5
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
        tempo: 0.5
        out $ s "bd*8" $ legato "0.1 1.5"
    "#;

    let buffer = render_dsl(alternating_code, 2.0);
    let rms = calculate_rms(&buffer);

    // Should produce audio with alternating short and long notes
    assert!(rms > 0.01, "Alternating legato should produce audio");

    // RMS should be between the extremes (0.1 and 1.5)
    // This is a weak test but verifies basic functionality
    assert!(rms > 0.05, "Alternating legato RMS should be above minimum");
}

#[test]
fn test_dur_constant() {
    // Test that dur (absolute duration in seconds) with a constant works
    let code = r#"
        tempo: 2.0
        out $ s "bd*4" $ dur 0.05
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Dur with constant should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_dur_pattern() {
    // Test that dur with a pattern works
    let code = r#"
        tempo: 2.0
        out $ s "bd*4" $ dur "0.05 0.2"
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Dur with pattern should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_dur_affects_duration() {
    // Test that different dur values produce different results
    // dur sets absolute duration in seconds
    let short_code = r#"
        tempo: 2.0
        out $ s "bd*8" $ dur 0.02
    "#;

    let long_code = r#"
        tempo: 2.0
        out $ s "bd*8" $ dur 0.3
    "#;

    let short_buffer = render_dsl(short_code, 2.0);
    let long_buffer = render_dsl(long_code, 2.0);

    let short_rms = calculate_rms(&short_buffer);
    let long_rms = calculate_rms(&long_buffer);

    // Both should have audio
    assert!(short_rms > 0.01, "Short dur should have audio: {}", short_rms);
    assert!(long_rms > 0.01, "Long dur should have audio: {}", long_rms);

    // Longer dur should have higher RMS (more sustained notes)
    assert!(
        long_rms > short_rms * 1.2,
        "Long dur (RMS: {}) should have higher RMS than short dur (RMS: {})",
        long_rms,
        short_rms
    );
}

#[test]
fn test_dur_vs_legato() {
    // Test that dur provides absolute timing while legato is relative
    // At tempo 2.0 cps, each quarter note slot is 0.125 seconds
    // So legato 1.0 = 0.125s, while dur 0.125 should be the same duration
    let legato_code = r#"
        tempo: 2.0
        out $ s "bd*4" $ legato 1.0
    "#;

    // dur 0.125 = 125ms per note (same as one slot at tempo 2.0)
    let dur_code = r#"
        tempo: 2.0
        out $ s "bd*4" $ dur 0.125
    "#;

    let legato_buffer = render_dsl(legato_code, 2.0);
    let dur_buffer = render_dsl(dur_code, 2.0);

    let legato_rms = calculate_rms(&legato_buffer);
    let dur_rms = calculate_rms(&dur_buffer);

    // Both should have audio
    assert!(legato_rms > 0.01, "Legato should have audio");
    assert!(dur_rms > 0.01, "Dur should have audio");

    // They should produce similar RMS (within 50% tolerance)
    let ratio = if legato_rms > dur_rms {
        legato_rms / dur_rms
    } else {
        dur_rms / legato_rms
    };
    assert!(
        ratio < 1.5,
        "Legato 1.0 and dur 0.125 should produce similar RMS at tempo 2.0 (ratio: {})",
        ratio
    );
}
