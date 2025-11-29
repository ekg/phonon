use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use std::collections::HashMap;

/// Render DSL code to audio buffer using compositional compiler
fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph =
        compile_program(statements, sample_rate, None).expect("Failed to compile DSL code");
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

/// Calculate peak level of audio buffer
fn calculate_peak(buffer: &[f32]) -> f32 {
    buffer.iter().map(|&x| x.abs()).fold(0.0f32, f32::max)
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

#[test]
fn test_gain_normal_level() {
    // LEVEL 1: Pattern query verification
    let pattern = parse_mini_notation("bd");
    let mut total_events = 0;
    for cycle in 0..4 {
        total_events += count_events_in_cycle(&pattern, cycle);
    }
    assert_eq!(total_events, 4, "Should have 4 events over 4 cycles");

    // LEVEL 2 & 3: Audio verification
    let code = r#"
tempo: 0.5
out $ s "bd" # gain 1.0
"#;

    let buffer = render_dsl(code, 2.0); // 4 cycles at tempo 2.0

    // Should produce audio at normal level
    let rms = calculate_rms(&buffer);
    let peak = calculate_peak(&buffer);

    assert!(
        rms > 0.01,
        "Normal gain should produce audible audio, got RMS={}",
        rms
    );
    assert!(
        peak > 0.3,
        "Normal gain should have significant peak, got peak={}",
        peak
    );
}

#[test]
fn test_gain_double_level() {
    // Test that gain 2.0 doubles amplitude
    let normal_code = r#"
tempo: 0.5
out $ s "bd" # gain 1.0
"#;

    let double_code = r#"
tempo: 0.5
out $ s "bd" # gain 2.0
"#;

    let normal = render_dsl(normal_code, 0.5); // 1 cycle
    let double = render_dsl(double_code, 0.5);

    let normal_rms = calculate_rms(&normal);
    let double_rms = calculate_rms(&double);

    // Double gain should roughly double RMS (allowing for some variation)
    // RMS scales linearly with gain for the same signal
    assert!(
        double_rms > normal_rms * 1.5,
        "Double gain ({:.3}) should be ~2x normal gain ({:.3}), ratio: {:.2}",
        double_rms,
        normal_rms,
        double_rms / normal_rms
    );
}

#[test]
fn test_gain_half_level() {
    // Test that gain 0.5 halves amplitude
    let normal_code = r#"
tempo: 0.5
out $ s "bd" # gain 1.0
"#;

    let half_code = r#"
tempo: 0.5
out $ s "bd" # gain 0.5
"#;

    let normal = render_dsl(normal_code, 0.5);
    let half = render_dsl(half_code, 0.5);

    let normal_rms = calculate_rms(&normal);
    let half_rms = calculate_rms(&half);

    // Half gain should roughly halve RMS
    assert!(
        half_rms < normal_rms * 0.7,
        "Half gain ({:.3}) should be ~0.5x normal gain ({:.3}), ratio: {:.2}",
        half_rms,
        normal_rms,
        half_rms / normal_rms
    );

    assert!(
        half_rms > normal_rms * 0.3,
        "Half gain ({:.3}) should not be too quiet compared to normal ({:.3}), ratio: {:.2}",
        half_rms,
        normal_rms,
        half_rms / normal_rms
    );
}

#[test]
fn test_gain_quiet_level() {
    // Test that gain 0.1 produces very quiet audio
    let normal_code = r#"
tempo: 0.5
out $ s "bd" # gain 1.0
"#;

    let quiet_code = r#"
tempo: 0.5
out $ s "bd" # gain 0.1
"#;

    let normal = render_dsl(normal_code, 0.5);
    let quiet = render_dsl(quiet_code, 0.5);

    let normal_rms = calculate_rms(&normal);
    let quiet_rms = calculate_rms(&quiet);

    // 0.1 gain should be much quieter than normal
    assert!(
        quiet_rms < normal_rms * 0.2,
        "Quiet gain ({:.3}) should be much less than normal gain ({:.3}), ratio: {:.2}",
        quiet_rms,
        normal_rms,
        quiet_rms / normal_rms
    );
}

#[test]
fn test_gain_comparison() {
    // Compare multiple gain levels
    let gain_0_5_code = r#"
tempo: 0.5
out $ s "bd" # gain 0.5
"#;

    let gain_1_0_code = r#"
tempo: 0.5
out $ s "bd" # gain 1.0
"#;

    let gain_2_0_code = r#"
tempo: 0.5
out $ s "bd" # gain 2.0
"#;

    let gain_0_5 = render_dsl(gain_0_5_code, 0.5);
    let gain_1_0 = render_dsl(gain_1_0_code, 0.5);
    let gain_2_0 = render_dsl(gain_2_0_code, 0.5);

    let rms_0_5 = calculate_rms(&gain_0_5);
    let rms_1_0 = calculate_rms(&gain_1_0);
    let rms_2_0 = calculate_rms(&gain_2_0);

    // RMS should increase monotonically with gain
    assert!(
        rms_0_5 < rms_1_0,
        "Gain 0.5 RMS ({:.3}) should be less than gain 1.0 RMS ({:.3})",
        rms_0_5,
        rms_1_0
    );

    assert!(
        rms_1_0 < rms_2_0,
        "Gain 1.0 RMS ({:.3}) should be less than gain 2.0 RMS ({:.3})",
        rms_1_0,
        rms_2_0
    );
}

#[test]
fn test_gain_pattern_based() {
    // Test pattern-based gain values
    let code = r#"
tempo: 0.5
out $ s "bd*4" # gain "0.5 1.0 1.5 2.0"
"#;

    let buffer = render_dsl(code, 0.5); // 1 cycle

    let rms = calculate_rms(&buffer);
    let peak = calculate_peak(&buffer);

    assert!(
        rms > 0.01,
        "Pattern-based gain should produce audio, got RMS={}",
        rms
    );
    assert!(
        peak > 0.3,
        "Pattern-based gain should have significant peak, got peak={}",
        peak
    );

    // The highest gain (2.0) should create a significant peak
    assert!(
        peak > 0.8,
        "With gain up to 2.0, peak should be substantial, got peak={}",
        peak
    );
}
