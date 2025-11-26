/// Test for P1.2: ar (attack/release) parameter
///
/// The `ar` parameter should be a shorthand for setting both attack and release.
/// This is common in Tidal Cycles and SuperCollider for quick envelope shaping.

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

    // Render in chunks to avoid buffer size issues
    let chunk_size = 128;
    let mut result = Vec::with_capacity(num_samples);
    for _ in 0..(num_samples / chunk_size) {
        result.extend_from_slice(&graph.render(chunk_size));
    }
    result
}

#[test]
fn test_ar_constant_values() {
    // ar with constant attack and release values
    let code = r#"
        tempo: 2.0
        o1: s "bd*4" # ar 0.01 0.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.01, "ar with constant values should produce audio, got RMS: {}", rms);
}

#[test]
fn test_ar_vs_separate_attack_release() {
    // Verify that ar 0.01 0.5 produces same result as attack 0.01 # release 0.5
    let ar_code = r#"
        tempo: 2.0
        o1: s "bd*4" # ar 0.01 0.3
    "#;

    let separate_code = r#"
        tempo: 2.0
        o1: s "bd*4" # attack 0.01 # release 0.3
    "#;

    let ar_buffer = render_dsl(ar_code, 2.0);
    let separate_buffer = render_dsl(separate_code, 2.0);

    let ar_rms = calculate_rms(&ar_buffer);
    let separate_rms = calculate_rms(&separate_buffer);

    // Both should produce audio
    assert!(ar_rms > 0.01, "ar shorthand should have audio");
    assert!(separate_rms > 0.01, "separate attack/release should have audio");

    // They should be very similar (within 5% due to floating point differences)
    let diff_ratio = (ar_rms - separate_rms).abs() / separate_rms;
    assert!(
        diff_ratio < 0.05,
        "ar shorthand should match separate attack/release, ar RMS: {}, separate RMS: {}, diff: {}",
        ar_rms,
        separate_rms,
        diff_ratio
    );
}

#[test]
fn test_ar_pattern_values() {
    // ar with pattern strings for both attack and release
    let code = r#"
        tempo: 2.0
        o1: s "bd*8" # ar "0.01 0.1" "0.1 0.5"
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.01, "ar with pattern values should produce audio, got RMS: {}", rms);
}

#[test]
fn test_ar_affects_envelope() {
    // Verify that ar actually affects the envelope (longer release = more sustain)
    let short_release = r#"
        tempo: 2.0
        o1: s "bd*4" # ar 0.01 0.1
    "#;

    let long_release = r#"
        tempo: 2.0
        o1: s "bd*4" # ar 0.01 0.8
    "#;

    let short_buffer = render_dsl(short_release, 2.0);
    let long_buffer = render_dsl(long_release, 2.0);

    let short_rms = calculate_rms(&short_buffer);
    let long_rms = calculate_rms(&long_buffer);

    // Both should have audio
    assert!(short_rms > 0.01, "Short release should have audio");
    assert!(long_rms > 0.01, "Long release should have audio");

    // Longer release should have noticeably more energy (at least 20% more)
    assert!(
        long_rms > short_rms * 1.2,
        "Longer release should have more energy: short RMS: {}, long RMS: {}",
        short_rms,
        long_rms
    );
}
