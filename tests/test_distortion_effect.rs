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

#[test]
fn test_distortion_effect_dist_alias() {
    // Test that 'dist' alias works for distortion
    let code = r#"
        tempo: 2.0
        o1: s "bd*4" # dist 1
    "#;

    let buffer = render_dsl(code, 2.0); // 2 seconds = 4 cycles at tempo 2.0
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.01, "Distortion should produce audio, got RMS: {}", rms);
}

#[test]
fn test_distortion_effect_distort_alias() {
    // Test that 'distort' alias also works
    let code = r#"
        tempo: 2.0
        o1: s "bd*4" # distort 1
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.01, "Distortion should produce audio, got RMS: {}", rms);
}

#[test]
fn test_distortion_effect_distortion_alias() {
    // Test that 'distortion' alias also works
    let code = r#"
        tempo: 2.0
        o1: s "bd*4" # distortion 1
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.01, "Distortion should produce audio, got RMS: {}", rms);
}

#[test]
fn test_distortion_effect_increases_harmonics() {
    // Test that distortion actually affects the signal
    // Compare clean vs distorted signal

    let clean_code = r#"
        tempo: 2.0
        o1: s "bd*4"
    "#;

    let distorted_code = r#"
        tempo: 2.0
        o1: s "bd*4" # dist 5
    "#;

    let clean_buffer = render_dsl(clean_code, 2.0);
    let distorted_buffer = render_dsl(distorted_code, 2.0);

    let clean_rms = calculate_rms(&clean_buffer);
    let distorted_rms = calculate_rms(&distorted_buffer);

    // Both should have audio
    assert!(clean_rms > 0.01, "Clean signal should have audio");
    assert!(distorted_rms > 0.01, "Distorted signal should have audio");

    // They should be different (distortion changes the signal)
    let diff_ratio = (clean_rms - distorted_rms).abs() / clean_rms;
    assert!(
        diff_ratio > 0.01,
        "Distortion should affect the signal, clean RMS: {}, distorted RMS: {}, diff ratio: {}",
        clean_rms,
        distorted_rms,
        diff_ratio
    );
}
