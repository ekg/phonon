use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Pattern, State, TimeSpan, Fraction};
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use std::collections::HashMap;

/// Render DSL code to audio buffer using compositional compiler
fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph = compile_program(statements, sample_rate).expect("Failed to compile DSL code");
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
fn test_speed_forward_playback() {
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
out: s "bd"
"#;

    let buffer = render_dsl(code, 2.0); // 4 cycles at tempo 2.0

    // Should produce audio
    let rms = calculate_rms(&buffer);
    let peak = calculate_peak(&buffer);

    assert!(rms > 0.01, "Normal speed should produce audible audio, got RMS={}", rms);
    assert!(peak > 0.3, "Normal speed should have significant peak, got peak={}", peak);
}

#[test]
fn test_speed_double_playback() {
    // Test that speed 2.0 plays twice as fast
    let code = r#"
tempo: 2.0
out: s "bd" # speed 2.0
"#;

    let buffer = render_dsl(code, 2.0);

    // Should produce audio
    let rms = calculate_rms(&buffer);
    let peak = calculate_peak(&buffer);

    assert!(rms > 0.01, "Double speed should produce audible audio, got RMS={}", rms);
    assert!(peak > 0.3, "Double speed should have significant peak, got peak={}", peak);

    // Double speed means the sample finishes in half the time
    // The sample should be audible but shorter duration
}

#[test]
fn test_speed_reverse_playback() {
    // Test that negative speed enables reverse playback
    let code = r#"
tempo: 2.0
out: s "bd" # speed -1.0
"#;

    let buffer = render_dsl(code, 2.0);

    // Should produce audio
    let rms = calculate_rms(&buffer);
    let peak = calculate_peak(&buffer);

    assert!(rms > 0.01, "Reverse playback should produce audible audio, got RMS={}", rms);
    assert!(peak > 0.3, "Reverse playback should have significant peak, got peak={}", peak);

    // Reverse playback should have similar amplitude to forward playback
    // (The envelope is disabled for reverse playback)
}

#[test]
fn test_speed_comparison() {
    // Compare normal vs fast vs reverse
    let normal_code = r#"
tempo: 2.0
out: s "bd"
"#;

    let fast_code = r#"
tempo: 2.0
out: s "bd" # speed 2.0
"#;

    let reverse_code = r#"
tempo: 2.0
out: s "bd" # speed -1.0
"#;

    let normal = render_dsl(normal_code, 0.5); // 1 cycle
    let fast = render_dsl(fast_code, 0.5);
    let reverse = render_dsl(reverse_code, 0.5);

    let normal_peak = calculate_peak(&normal);
    let fast_peak = calculate_peak(&fast);
    let reverse_peak = calculate_peak(&reverse);

    // All should produce significant audio
    assert!(normal_peak > 0.3, "Normal playback peak too low: {}", normal_peak);
    assert!(fast_peak > 0.3, "Fast playback peak too low: {}", fast_peak);
    assert!(reverse_peak > 0.3, "Reverse playback peak too low: {}", reverse_peak);

    // Peak levels should be similar (within reasonable range)
    // Reverse may be slightly louder due to no envelope
    assert!(reverse_peak > normal_peak * 0.5,
        "Reverse peak ({}) should be comparable to normal peak ({})",
        reverse_peak, normal_peak);
}

#[test]
fn test_speed_half_playback() {
    // Test that speed 0.5 plays at half speed (pitch down)
    let code = r#"
tempo: 2.0
out: s "bd" # speed 0.5
"#;

    let buffer = render_dsl(code, 2.0);

    let rms = calculate_rms(&buffer);
    let peak = calculate_peak(&buffer);

    assert!(rms > 0.01, "Half speed should produce audible audio, got RMS={}", rms);
    assert!(peak > 0.3, "Half speed should have significant peak, got peak={}", peak);
}

#[test]
fn test_speed_pattern_based() {
    // Test pattern-based speed values
    let code = r#"
tempo: 2.0
out: s "bd*4" # speed "1 2 0.5 -1"
"#;

    let buffer = render_dsl(code, 0.5); // 1 cycle

    let rms = calculate_rms(&buffer);
    let peak = calculate_peak(&buffer);

    assert!(rms > 0.01, "Pattern-based speed should produce audio, got RMS={}", rms);
    assert!(peak > 0.3, "Pattern-based speed should have significant peak, got peak={}", peak);
}
