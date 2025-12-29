//! Tests for multi-output system (o1, o2, etc.)
//!
//! Uses the compositional_compiler path which is the main parser/compiler.
//! The syntax is: o1 $, o2 $, o3 $, etc.

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

fn render_dsl(code: &str, num_samples: usize) -> Vec<f32> {
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile DSL code");

    // Render in small chunks like the continuous synthesis tests
    let buffer_size = 128;
    let num_buffers = num_samples / buffer_size;
    let mut full_audio = Vec::with_capacity(num_samples);
    for _ in 0..num_buffers {
        let buffer = graph.render(buffer_size);
        full_audio.extend_from_slice(&buffer);
    }
    full_audio
}

fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = samples.iter().map(|&s| s * s).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

#[test]
fn test_multi_output_two_channels() {
    let input = r#"
        tempo: 0.5
        o1 $ s "bd ~ bd ~" * 0.5
        o2 $ s "~ sn ~ sn" * 0.5
    "#;

    let buffer = render_dsl(input, 44100);
    let rms = calculate_rms(&buffer);

    // Should produce audio from both channels
    assert!(
        rms > 0.05,
        "Multi-output should produce audio, got RMS: {}",
        rms
    );
    println!("✅ test_multi_output_two_channels: RMS = {:.6}", rms);
}

#[test]
fn test_multi_output_single_channel() {
    let input = r#"
        tempo: 0.5
        o1 $ s "bd sn hh cp" * 0.5
    "#;

    let buffer = render_dsl(input, 22050);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.01,
        "Single numbered output should produce audio, got RMS: {}",
        rms
    );
    println!("✅ test_multi_output_single_channel: RMS = {:.6}", rms);
}

#[test]
fn test_multi_output_three_channels() {
    let input = r#"
        tempo: 0.5
        o1 $ s "bd ~ bd ~" * 0.3
        o2 $ s "~ sn ~ sn" * 0.3
        o3 $ s "hh hh hh hh" * 0.3
    "#;

    let buffer = render_dsl(input, 44100);
    let rms = calculate_rms(&buffer);

    // Three channels should produce combined output
    assert!(
        rms > 0.01,
        "Three-channel output should produce audio, got RMS: {}",
        rms
    );
    println!("✅ test_multi_output_three_channels: RMS = {:.6}", rms);
}

#[test]
fn test_multi_output_with_plain_out() {
    // Test that plain "out" still works alongside numbered outputs
    let input = r#"
        tempo: 0.5
        out $ s "bd ~ bd ~" * 0.3
        o1 $ s "~ sn ~ sn" * 0.3
    "#;

    let buffer = render_dsl(input, 44100);
    let rms = calculate_rms(&buffer);

    // Both plain "out" and numbered outputs should work together
    assert!(
        rms > 0.01,
        "Mixed plain and numbered output should produce audio, got RMS: {}",
        rms
    );
    println!("✅ test_multi_output_with_plain_out: RMS = {:.6}", rms);
}

#[test]
fn test_multi_output_different_patterns() {
    // Test with different types of patterns (synthesis + samples)
    let input = r#"
        tempo: 0.5
        o1 $ sine 440 * 0.2
        o2 $ s "bd sn" * 0.3
    "#;

    let buffer = render_dsl(input, 22050);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.05,
        "Multi-output with different pattern types should work, got RMS: {}",
        rms
    );
    println!("✅ test_multi_output_different_patterns: RMS = {:.6}", rms);
}

#[test]
fn test_multi_output_synthesis_only() {
    // Test numbered outputs with synthesis only (no samples)
    let input = r#"
        tempo: 0.5
        o1 $ sine 220 * 0.3
        o2 $ saw 330 * 0.2
        o3 $ tri 440 * 0.2
    "#;

    let buffer = render_dsl(input, 22050);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.1,
        "Multi-output synthesis should work, got RMS: {}",
        rms
    );
    println!("✅ test_multi_output_synthesis_only: RMS = {:.6}", rms);
}
