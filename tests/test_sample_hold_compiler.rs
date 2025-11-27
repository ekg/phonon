/// Integration test for sample_hold compiler support
///
/// Verifies that sample_hold can be compiled and renders correctly in the DSL

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

/// Helper function to render DSL code to audio buffer
fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph = compile_program(statements, sample_rate, None).expect("Failed to compile DSL code");
    let num_samples = (duration * sample_rate) as usize;
    graph.render(num_samples)
}

#[test]
fn test_sample_hold_compiles() {
    // Test that sample_hold compiles correctly
    let code = "
        tempo: 0.5
        ~noise: white_noise
        ~clock: square 4.0
        ~sh: sample_hold ~noise ~clock
        o1: ~sh * 0.1
    ";

    let buffer = render_dsl(code, 0.5);
    assert!(buffer.len() > 0, "Should produce audio");
    assert!(buffer.iter().any(|&x| x != 0.0), "Should produce non-zero audio");
}

#[test]
fn test_sample_hold_with_sine_trigger() {
    // Test sample_hold with sine wave trigger (crosses zero)
    let code = "
        tempo: 0.5
        ~lfo: sine 2.0
        ~trigger: sine 4.0
        ~sh: sample_hold ~lfo ~trigger
        o1: ~sh
    ";

    let buffer = render_dsl(code, 0.5);
    assert!(buffer.len() > 0, "Should produce audio");
}

#[test]
fn test_sample_hold_stepped_modulation() {
    // Test sample_hold for stepped modulation (classic use case)
    let code = "
        tempo: 0.5
        ~noise: white_noise
        ~clock: square 8.0
        ~sh: sample_hold ~noise ~clock
        ~freq: (~sh + 1.0) * 220.0
        ~osc: sine ~freq
        o1: ~osc * 0.1
    ";

    let buffer = render_dsl(code, 1.0);

    // Verify audio is valid
    for &sample in &buffer {
        assert!(sample.is_finite(), "Output should be finite");
    }
    assert!(buffer.iter().any(|&x| x.abs() > 0.001), "Should produce audible signal");
}

#[test]
fn test_sample_hold_multiple_instances() {
    // Test multiple sample_hold nodes in same graph
    let code = "
        tempo: 0.5
        ~noise1: white_noise
        ~noise2: white_noise
        ~clock: square 4.0
        ~sh1: sample_hold ~noise1 ~clock
        ~sh2: sample_hold ~noise2 ~clock
        o1: (~sh1 + ~sh2) * 0.25
    ";

    let buffer = render_dsl(code, 0.5);

    // Verify audio is valid
    for &sample in &buffer {
        assert!(sample.is_finite(), "Output should be finite");
    }
}
