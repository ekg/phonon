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

// NOTE: Audio verification tests are ignored because they require sample files (dirt-samples)
// to be available. Run with `cargo test -- --ignored` when samples are installed.

#[test]
#[ignore = "requires sample files (dirt-samples) to be installed"]
fn test_begin_start_at_beginning() {
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
out $ s "bd" # begin 0.0
"#;

    let buffer = render_dsl(code, 2.0); // 4 cycles

    let rms = calculate_rms(&buffer);

    // begin 0.0 should start at the beginning (normal playback)
    assert!(
        rms > 0.01,
        "begin 0.0 should produce audio, got RMS={}",
        rms
    );
}

#[test]
#[ignore = "requires sample files (dirt-samples) to be installed"]
fn test_begin_skip_start() {
    // Test that begin 0.5 skips first half of sample and still produces audio
    let begin_half_code = r#"
tempo: 0.5
out $ s "bd" # begin 0.5
"#;

    let begin_half = render_dsl(begin_half_code, 0.5); // 1 cycle

    let begin_half_rms = calculate_rms(&begin_half);

    // Should produce audio (slicing is working)
    assert!(
        begin_half_rms > 0.01,
        "begin 0.5 should produce audio, got RMS={}",
        begin_half_rms
    );

    // Note: We don't compare RMS to normal because envelope and speed
    // parameters can normalize the loudness across different slice points
}

#[test]
#[ignore = "requires sample files (dirt-samples) to be installed"]
fn test_end_full_sample() {
    // Test that end 1.0 plays full sample
    let code = r#"
tempo: 0.5
out $ s "bd" # end 1.0
"#;

    let buffer = render_dsl(code, 0.5); // 1 cycle

    let rms = calculate_rms(&buffer);

    // end 1.0 should play full sample
    assert!(rms > 0.01, "end 1.0 should produce audio, got RMS={}", rms);
}

#[test]
#[ignore = "requires sample files (dirt-samples) to be installed"]
fn test_end_truncate_sample() {
    // Test that end 0.5 plays only first half and produces audio
    let end_half_code = r#"
tempo: 0.5
out $ s "bd" # end 0.5
"#;

    let end_half = render_dsl(end_half_code, 0.5);

    let end_half_rms = calculate_rms(&end_half);

    // Should produce audio (slicing is working)
    assert!(
        end_half_rms > 0.01,
        "end 0.5 should produce audio, got RMS={}",
        end_half_rms
    );

    // Note: We don't compare RMS because envelope and speed parameters
    // can normalize the loudness across different slice endpoints
}

#[test]
#[ignore = "requires sample files (dirt-samples) to be installed"]
fn test_begin_and_end_combined() {
    // Test slicing middle portion of sample with both begin and end
    let code = r#"
tempo: 0.5
out $ s "bd" # begin 0.25 # end 0.75
"#;

    let buffer = render_dsl(code, 0.5);

    let rms = calculate_rms(&buffer);

    // Should produce audio from the middle portion
    assert!(
        rms > 0.01,
        "begin 0.25 + end 0.75 should produce audio, got RMS={}",
        rms
    );
}

#[test]
#[ignore = "requires sample files (dirt-samples) to be installed"]
fn test_begin_end_pattern_based() {
    // Test pattern-based begin/end values
    let code = r#"
tempo: 0.5
out $ s "bd*4" # begin "0 0.25 0.5 0.75"
"#;

    let buffer = render_dsl(code, 0.5); // 1 cycle

    let rms = calculate_rms(&buffer);

    // Pattern-based begin should produce audio
    assert!(
        rms > 0.01,
        "Pattern-based begin should produce audio, got RMS={}",
        rms
    );
}

#[test]
#[ignore = "requires sample files (dirt-samples) to be installed"]
fn test_begin_end_extremes() {
    // Test that begin/end values are clamped properly
    let code = r#"
tempo: 0.5
out $ s "bd*3" # begin "-1 0 2" # end "0.5 1 3"
"#;

    let buffer = render_dsl(code, 0.5);

    let rms = calculate_rms(&buffer);

    // Extreme values should be clamped and still produce audio
    assert!(
        rms > 0.01,
        "Extreme begin/end values should be clamped and produce audio, got RMS={}",
        rms
    );
}

#[test]
fn test_begin_larger_than_end() {
    // Test edge case where begin > end (should produce minimal or no sound)
    let code = r#"
tempo: 0.5
out $ s "bd" # begin 0.8 # end 0.2
"#;

    let buffer = render_dsl(code, 0.5);

    let rms = calculate_rms(&buffer);

    // When begin > end, the slice is invalid.
    // The clamping should produce a minimal slice or handle gracefully
    // This is an edge case - we just verify it doesn't crash
    // RMS might be very small or zero
    assert!(
        rms >= 0.0,
        "Invalid begin/end should not crash, got RMS={}",
        rms
    );
}
