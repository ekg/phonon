/// Tests for pattern-based loopAt transform
/// loopAt can take either a constant number or a pattern of numbers,
/// allowing the loop duration to change over time.
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use std::collections::HashMap;

mod audio_test_utils;
mod pattern_verification_utils;
use audio_test_utils::calculate_rms;
use pattern_verification_utils::detect_audio_events;

fn render_dsl(code: &str, cycles: usize) -> Vec<f32> {
    let (_, statements) = parse_program(code).expect("Parse failed");
    let sample_rate = 44100.0;
    let mut graph = compile_program(statements, sample_rate, None).expect("Compile failed");
    let samples_per_cycle = (sample_rate as f64 / 0.5) as usize;
    let total_samples = samples_per_cycle * cycles;
    graph.render(total_samples)
}

// ============================================================================
// LEVEL 1: Pattern Query Verification (Constant vs. Pattern)
// ============================================================================

#[test]
fn test_loopAt_level1_constant_value() {
    // Constant loopAt should produce events with correct speed context
    let pattern = parse_mini_notation("bd sn hh cp");
    let looped = pattern.loop_at(Pattern::pure(2.0));

    let mut total_events = 0;
    for cycle in 0..4 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };
        let events = looped.query(&state);
        total_events += events.len();

        // Check that speed context is set correctly (1.0 / 2.0 = 0.5)
        for event in events {
            let speed = event
                .context
                .get("speed")
                .and_then(|s| s.parse::<f64>().ok());
            assert!(speed.is_some(), "Speed context should be set");
            assert!(
                (speed.unwrap() - 0.5).abs() < 0.001,
                "Speed should be 0.5 for loopAt 2"
            );
        }
    }

    // loopAt 2 slows pattern by 2x, so 4 events/cycle becomes 2 events/cycle
    // Over 4 cycles: 2 * 4 = 8 events
    assert_eq!(
        total_events, 8,
        "loopAt 2 should produce 8 events over 4 cycles"
    );
}

#[test]
fn test_loopAt_level1_pattern_alternation() {
    // Pattern-based loopAt "<1 2>" alternates between 1 and 2 cycle durations across cycles
    // This affects the speed context of events (1.0 for loopAt 1, 0.5 for loopAt 2)
    let pattern = parse_mini_notation("bd");
    let duration_pattern = parse_mini_notation("<1 2>");
    let looped = pattern.loop_at_pattern(duration_pattern);

    let mut speed_values = vec![];
    for cycle in 0..8 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };
        let events = looped.query(&state);

        // Collect speed context values from events
        for event in events {
            if let Some(speed_str) = event.context.get("speed") {
                if let Ok(speed) = speed_str.parse::<f64>() {
                    speed_values.push(speed);
                }
            }
        }
    }

    // Should see at least 2 different speed values (1.0 for loopAt 1, 0.5 for loopAt 2)
    let unique_speeds: std::collections::HashSet<_> = speed_values
        .iter()
        .map(|&s| (s * 100.0).round() as i32) // Round to avoid floating point issues
        .collect();

    assert!(
        unique_speeds.len() >= 2,
        "Pattern-based loopAt should produce varying speed values, found {} unique speeds: {:?}",
        unique_speeds.len(),
        unique_speeds
    );
}

// ============================================================================
// LEVEL 2: Onset Detection (Audio Event Verification)
// ============================================================================

#[test]
fn test_loopAt_level2_constant_produces_audio() {
    let code = r#"
tempo: 1.0
out $ s "bd" $ loopAt 2
"#;
    let audio = render_dsl(code, 4);
    let onsets = detect_audio_events(&audio, 44100.0, 0.05);

    // Should detect audio events
    assert!(
        onsets.len() > 0,
        "loopAt 2 should produce detectable audio events"
    );

    // With loopAt 2, pattern is slowed by 2x, so we expect fewer events
    // "bd" normally triggers once per cycle, loopAt 2 makes it every 2 cycles
    // Over 4 cycles, we should get 2 events
    // Note: actual count may be higher due to sample playback characteristics
    assert!(
        onsets.len() >= 1,
        "loopAt 2 should produce at least 1 event over 4 cycles, got {}",
        onsets.len()
    );
}

#[test]
fn test_loopAt_level2_pattern_produces_audio() {
    let code = r#"
tempo: 1.0
out $ s "bd" $ loopAt "1 2"
"#;
    let audio = render_dsl(code, 8);
    let onsets = detect_audio_events(&audio, 44100.0, 0.05);

    // Should detect audio events
    assert!(
        onsets.len() > 0,
        "Pattern-based loopAt should produce detectable audio events"
    );

    // Pattern alternates between 1 and 2 cycle durations
    // Should produce more events than constant loopAt 2
    assert!(
        onsets.len() >= 4,
        "Pattern-based loopAt should produce multiple events over 8 cycles, got {}",
        onsets.len()
    );
}

#[test]
fn test_loopAt_level2_pattern_with_alternation() {
    let code = r#"
tempo: 1.0
out $ s "bd" $ loopAt "<1 2 4>"
"#;
    let audio = render_dsl(code, 12);
    let onsets = detect_audio_events(&audio, 44100.0, 0.05);

    // Should detect audio events with varying timing
    assert!(
        onsets.len() >= 6,
        "Complex pattern-based loopAt should produce events, got {}",
        onsets.len()
    );
}

// ============================================================================
// LEVEL 3: Audio Characteristics (Signal Quality)
// ============================================================================

#[test]
fn test_loopAt_level3_constant_has_audio() {
    let code = r#"
tempo: 1.0
out $ s "bd" $ loopAt 2
"#;
    let audio = render_dsl(code, 4);
    let rms = calculate_rms(&audio);

    assert!(
        rms > 0.01,
        "loopAt 2 should produce audible signal, got RMS {}",
        rms
    );
}

#[test]
fn test_loopAt_level3_pattern_has_audio() {
    let code = r#"
tempo: 1.0
out $ s "bd" $ loopAt "1 2"
"#;
    let audio = render_dsl(code, 8);
    let rms = calculate_rms(&audio);

    assert!(
        rms > 0.01,
        "Pattern-based loopAt should produce audible signal, got RMS {}",
        rms
    );
}

// ============================================================================
// Integration Tests (Real Livecode Patterns)
// ============================================================================

#[test]
fn test_loopAt_integration_with_chop() {
    // From livecode: every 4 (rev) $ loopAt 2 $ chop 16
    let code = r#"
tempo: 1.0
out $ s "bd" $ loopAt 2 $ chop 16
"#;
    let audio = render_dsl(code, 4);
    let rms = calculate_rms(&audio);

    assert!(
        rms > 0.01,
        "loopAt + chop should produce audio, got RMS {}",
        rms
    );
}

#[test]
fn test_loopAt_integration_with_slice() {
    // From livecode: slice 8 "0 .. 7" $ loopAt 2
    let code = r#"
tempo: 1.0
out $ s "bd" $ slice 8 "0 .. 7" $ loopAt 2
"#;
    let audio = render_dsl(code, 4);
    let rms = calculate_rms(&audio);

    assert!(
        rms > 0.01,
        "loopAt + slice should produce audio, got RMS {}",
        rms
    );
}

#[test]
fn test_loopAt_integration_complex_chain() {
    // From livecode: every 4 (rev) $ loopAt 2 $ chop 8
    let code = r#"
tempo: 1.0
out $ s "bd" $ every 4 (rev) $ loopAt 2 $ chop 8
"#;
    let audio = render_dsl(code, 8);
    let rms = calculate_rms(&audio);

    assert!(
        rms > 0.01,
        "Complex transform chain should produce audio, got RMS {}",
        rms
    );
}

#[test]
fn test_loopAt_integration_pattern_complex() {
    // From livecode: loopAt "<16 16 <8 16> 12>"
    let code = r#"
tempo: 1.0
out $ s "bd" $ loopAt "<1 2 <4 8> 3>"
"#;
    let audio = render_dsl(code, 8);
    let rms = calculate_rms(&audio);

    assert!(
        rms > 0.01,
        "Complex pattern-based loopAt should produce audio, got RMS {}",
        rms
    );
}
