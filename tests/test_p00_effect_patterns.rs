/// P0.0 Test: Effect parameters accept patterns
///
/// This test verifies that ALL effect parameters accept pattern modulation,
/// not just bare numbers. This is a fundamental design principle:
/// "Patterns ARE control signals" - they can modulate any parameter at sample rate.

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
fn test_lpf_pattern_cutoff() {
    // LPF with pattern-modulated cutoff frequency
    let code = r#"
        tempo: 0.5
        o1: saw 110 # lpf (sine 0.5 * 1500 + 500) 0.8
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.01, "LPF with pattern cutoff should produce audio, got RMS: {}", rms);
}

#[test]
fn test_hpf_pattern_cutoff() {
    // HPF with pattern-modulated cutoff frequency
    let code = r#"
        tempo: 0.5
        o1: saw 110 # hpf (sine 0.5 * 1000 + 2000) 0.8
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.01, "HPF with pattern cutoff should produce audio, got RMS: {}", rms);
}

#[test]
fn test_bpf_pattern_center() {
    // BPF with pattern-modulated center frequency
    let code = r#"
        tempo: 0.5
        o1: saw 110 # bpf (sine 0.5 * 1000 + 1500) 2.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.01, "BPF with pattern center should produce audio, got RMS: {}", rms);
}

#[test]
fn test_delay_pattern_time() {
    // Delay with pattern-modulated delay time
    let code = r#"
        tempo: 0.5
        o1: s "bd*4" # delay (sine 1.0 * 0.2 + 0.1) 0.3
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.01, "Delay with pattern time should produce audio, got RMS: {}", rms);
}

#[test]
fn test_reverb_pattern_room_size() {
    // Reverb with pattern-modulated room size
    let code = r#"
        tempo: 0.5
        o1: s "sn*2" # reverb (sine 0.25 * 0.5 + 0.3) 0.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.01, "Reverb with pattern room_size should produce audio, got RMS: {}", rms);
}

#[test]
fn test_distortion_pattern_drive() {
    // Distortion with pattern-modulated drive
    let code = r#"
        tempo: 0.5
        o1: s "bd*4" # dist (sine 2.0 * 2.0 + 1.0)
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.01, "Distortion with pattern drive should produce audio, got RMS: {}", rms);
}

#[test]
fn test_multiple_pattern_parameters() {
    // Multiple effects with pattern-modulated parameters
    let code = r#"
        tempo: 0.5
        o1: saw 110 # lpf (sine 0.5 * 1500 + 500) 0.8 # dist (sine 2.0 + 1.5) # delay (sine 1.0 * 0.15 + 0.1) 0.3
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.01, "Multiple pattern effects should produce audio, got RMS: {}", rms);
}

#[test]
fn test_pattern_vs_constant() {
    // Verify pattern modulation produces different result than constant
    let constant_code = r#"
        tempo: 0.5
        o1: saw 110 # lpf 1000 0.8
    "#;

    let pattern_code = r#"
        tempo: 0.5
        o1: saw 110 # lpf (sine 0.5 * 1500 + 500) 0.8
    "#;

    let constant_buffer = render_dsl(constant_code, 2.0);
    let pattern_buffer = render_dsl(pattern_code, 2.0);

    let constant_rms = calculate_rms(&constant_buffer);
    let pattern_rms = calculate_rms(&pattern_buffer);

    // Both should have audio
    assert!(constant_rms > 0.01, "Constant cutoff should have audio");
    assert!(pattern_rms > 0.01, "Pattern cutoff should have audio");

    // They should be different
    let diff_ratio = (constant_rms - pattern_rms).abs() / constant_rms;
    assert!(
        diff_ratio > 0.01,
        "Pattern modulation should differ from constant, const RMS: {}, pattern RMS: {}, diff: {}",
        constant_rms,
        pattern_rms,
        diff_ratio
    );
}
