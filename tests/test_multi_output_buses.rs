/// Test multi-output bus assignments (out $, o2:, etc.) via compositional compiler
///
/// Verifies that:
/// 1. out $, o2:, etc. compile correctly
/// 2. Multiple outputs are mixed together automatically
/// 3. Mixed output produces correct audio
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

/// Helper: Compile and render DSL code using compositional compiler
fn render_compositional(code: &str, num_samples: usize) -> Vec<f32> {
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph =
        compile_program(statements, 44100.0, None).expect("Failed to compile DSL code");
    let buffer_size = 128;
    let num_buffers = num_samples / buffer_size;
    let mut full_audio = Vec::with_capacity(num_samples);
    for _ in 0..num_buffers {
        let buffer = graph.render(buffer_size);
        full_audio.extend_from_slice(&buffer);
    }
    full_audio
}

/// Helper: Calculate RMS of audio buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

#[test]
fn test_single_output_o1() {
    // Single out $ output should produce audio
    let audio = render_compositional("out $ sine 440 * 0.5", 22050);
    let rms = calculate_rms(&audio);
    assert!(rms > 0.1, "Single out $ should produce audio, got RMS: {}", rms);
}

#[test]
fn test_two_outputs_o1_o2() {
    // Two outputs should be mixed together
    let audio = render_compositional(
        r#"
        out $ sine 220 * 0.3
        o2 $ sine 440 * 0.3
    "#,
        22050,
    );
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.1,
        "out $ + o2 $ should produce combined audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_three_outputs_mixed() {
    // Three outputs should all be mixed together
    let audio = render_compositional(
        r#"
        out $ sine 110 * 0.3
        o2 $ sine 220 * 0.3
        o3 $ sine 440 * 0.3
    "#,
        22050,
    );
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.1,
        "Three outputs should produce combined audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_output_with_samples() {
    // Multi-output with sample playback
    let audio = render_compositional(
        r#"
        tempo: 0.5
        out $ s "bd"
        o2 $ s "sn"
    "#,
        44100,
    );
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.001,
        "Sample outputs should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_output_with_synthesis() {
    // Multi-output with synthesizers
    let audio = render_compositional(
        r#"
        out $ sine 220 * 0.5
        o2 $ sine 440 * 0.5
    "#,
        22050,
    );
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.3,
        "Two sine wave outputs should produce substantial audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_explicit_out_overrides_numbered() {
    // Second out $ should override the first
    let audio = render_compositional(
        r#"
        out $ sine 220 * 0.1
        out $ sine 440 * 0.5
    "#,
        22050,
    );
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.2,
        "Last out $ should win with higher amplitude, got RMS: {}",
        rms
    );
}

#[test]
fn test_bus_references_in_outputs() {
    // Outputs can reference buses
    let audio = render_compositional(
        r#"
        ~bass $ sine 110 * 0.3
        ~lead $ sine 440 * 0.3
        out $ ~bass
        o2 $ ~lead
    "#,
        22050,
    );
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.1,
        "Bus references in outputs should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_complex_multi_output() {
    // Complex example with multiple buses and outputs
    let audio = render_compositional(
        r#"
        ~bass $ sine 55 * 0.3
        ~lead $ sine 220 * 0.3
        ~lfo $ sine 2 * 0.1

        out $ ~bass
        o2 $ ~lead
        o3 $ ~lfo
    "#,
        22050,
    );
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.05,
        "Complex multi-output should produce audio, got RMS: {}",
        rms
    );
}
