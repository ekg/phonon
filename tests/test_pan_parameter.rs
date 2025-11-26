use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Pattern, State, TimeSpan, Fraction};
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use std::collections::HashMap;

/// Render DSL code to audio buffer using compositional compiler
fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph = compile_program(statements, sample_rate, None).expect("Failed to compile DSL code");
    let num_samples = (duration * sample_rate) as usize;
    graph.render(num_samples)
}

/// Calculate RMS (root mean square) of audio buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    if buffer.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Count events in a cycle for a pattern
fn count_events_in_cycle(pattern: &Pattern<String>, cycle: i32) -> usize {
    let state = State {
        span: TimeSpan::new(
            Fraction::from_float(cycle as f64),
            Fraction::from_float((cycle + 1) as f64),
        ),
        controls: HashMap::new(),
    };
    pattern.query(&state).len()
}

// NOTE: Pan parameter tests verify that pan values don't break audio rendering.
// Full stereo separation testing requires stereo output routing (out1:/out2:).
// Current architecture: VoiceManager applies pan → mixes to mono → outputs to out:
// This is correct behavior for mono output, pan just affects internal stereo field.

#[test]
fn test_pan_center() {
    // LEVEL 1: Pattern query verification
    let pattern = parse_mini_notation("bd");
    let mut total_events = 0;
    for cycle in 0..4 {
        total_events += count_events_in_cycle(&pattern, cycle);
    }
    assert_eq!(total_events, 4, "Should have 4 events over 4 cycles");

    // LEVEL 2 & 3: Audio verification
    let code = r#"
tempo: 2.0
out: s "bd" # pan 0.0
"#;

    let buffer = render_dsl(code, 2.0); // 4 cycles

    let rms = calculate_rms(&buffer);

    // Center pan should produce audible audio
    assert!(rms > 0.01, "Center pan should produce audio, got RMS={}", rms);
}

#[test]
fn test_pan_full_left() {
    // Test that pan -1.0 doesn't break audio
    let code = r#"
tempo: 2.0
out: s "bd" # pan -1.0
"#;

    let buffer = render_dsl(code, 0.5); // 1 cycle

    let rms = calculate_rms(&buffer);

    // Full left should still produce audible audio (mixed to mono)
    assert!(rms > 0.01, "Full left pan should produce audio, got RMS={}", rms);
}

#[test]
fn test_pan_full_right() {
    // Test that pan 1.0 doesn't break audio
    let code = r#"
tempo: 2.0
out: s "bd" # pan 1.0
"#;

    let buffer = render_dsl(code, 0.5);

    let rms = calculate_rms(&buffer);

    // Full right should still produce audible audio (mixed to mono)
    assert!(rms > 0.01, "Full right pan should produce audio, got RMS={}", rms);
}

#[test]
fn test_pan_values_preserve_energy() {
    // Test that different pan values produce similar energy when mixed to mono
    // This verifies the equal-power panning law is working correctly
    let pan_left_code = r#"
tempo: 2.0
out: s "bd" # pan -1.0
"#;

    let pan_center_code = r#"
tempo: 2.0
out: s "bd" # pan 0.0
"#;

    let pan_right_code = r#"
tempo: 2.0
out: s "bd" # pan 1.0
"#;

    let left = render_dsl(pan_left_code, 0.5);
    let center = render_dsl(pan_center_code, 0.5);
    let right = render_dsl(pan_right_code, 0.5);

    let left_rms = calculate_rms(&left);
    let center_rms = calculate_rms(&center);
    let right_rms = calculate_rms(&right);

    // All pan positions should produce similar RMS when mixed to mono
    // Equal-power panning ensures energy is preserved across the stereo field
    assert!(left_rms > 0.01, "Pan left should produce audio, got {}", left_rms);
    assert!(center_rms > 0.01, "Pan center should produce audio, got {}", center_rms);
    assert!(right_rms > 0.01, "Pan right should produce audio, got {}", right_rms);

    // RMS values should be within reasonable range (factor of 2)
    // Due to equal-power panning, they should actually be very similar
    let max_rms = left_rms.max(center_rms).max(right_rms);
    let min_rms = left_rms.min(center_rms).min(right_rms);

    assert!(max_rms / min_rms < 2.0,
        "Pan values should preserve energy: left={}, center={}, right={}, ratio={}",
        left_rms, center_rms, right_rms, max_rms / min_rms);
}

#[test]
fn test_pan_pattern_based() {
    // Test pattern-based pan values
    let code = r#"
tempo: 2.0
out: s "bd*4" # pan "-1 -0.5 0.5 1"
"#;

    let buffer = render_dsl(code, 0.5); // 1 cycle

    let rms = calculate_rms(&buffer);

    // Pattern-based pan should produce audio
    assert!(rms > 0.01, "Pattern-based pan should produce audio, got RMS={}", rms);
}

#[test]
fn test_pan_extreme_values() {
    // Test that pan values outside normal range are clamped properly
    let code = r#"
tempo: 2.0
out: s "bd*3" # pan "-5 0 5"
"#;

    let buffer = render_dsl(code, 0.5);

    let rms = calculate_rms(&buffer);

    // Extreme pan values should be clamped and still produce audio
    assert!(rms > 0.01, "Extreme pan values should be clamped and produce audio, got RMS={}", rms);
}

#[test]
fn test_pan_with_gain() {
    // Test that pan works together with gain modifier
    let code = r#"
tempo: 2.0
out: s "bd" # gain 0.5 # pan -1.0
"#;

    let buffer = render_dsl(code, 0.5);

    let rms = calculate_rms(&buffer);

    // Pan with gain should produce audio
    assert!(rms > 0.01, "Pan with gain should produce audio, got RMS={}", rms);
    assert!(rms < 0.5, "Gain 0.5 should reduce RMS, got RMS={}", rms);
}
