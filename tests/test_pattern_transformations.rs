//! Tests for pattern transformations: degradeBy, stutter, chop, scramble
//!
//! These tests verify the pattern methods called by compositional_compiler.rs

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use std::collections::HashMap;

// ========== Level 1: Pattern Query Tests ==========

#[test]
fn test_degrade_by_removes_events() {
    // degradeBy should remove events probabilistically
    // With degradeBy 1.0, all events should be removed
    // With degradeBy 0.0, no events should be removed

    let pattern = parse_mini_notation("bd sn hh cp");

    // Test with degradeBy 0.0 (no removal)
    let pattern_no_degrade = pattern.clone().degrade_by(Pattern::pure(0.0));
    let mut total_no_degrade = 0;
    for cycle in 0..4 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };
        total_no_degrade += pattern_no_degrade.query(&state).len();
    }

    // Should have all events (4 events × 4 cycles = 16)
    assert_eq!(total_no_degrade, 16, "degradeBy 0.0 should keep all events");

    // Test with degradeBy 1.0 (full removal)
    let pattern_full_degrade = pattern.clone().degrade_by(Pattern::pure(1.0));
    let mut total_full_degrade = 0;
    for cycle in 0..4 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };
        total_full_degrade += pattern_full_degrade.query(&state).len();
    }

    // Should have no events
    assert_eq!(
        total_full_degrade, 0,
        "degradeBy 1.0 should remove all events"
    );

    // Test with degradeBy 0.5 (probabilistic removal)
    let pattern_half_degrade = pattern.clone().degrade_by(Pattern::pure(0.5));
    let mut total_half_degrade = 0;
    for cycle in 0..8 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };
        total_half_degrade += pattern_half_degrade.query(&state).len();
    }

    // Should have roughly half the events (with some variance)
    // 4 events × 8 cycles = 32 total, expect ~16 with 50% probability
    assert!(
        total_half_degrade < 32,
        "degradeBy 0.5 should remove some events, got {} out of 32",
        total_half_degrade
    );
    assert!(
        total_half_degrade > 0,
        "degradeBy 0.5 should keep some events, got {} out of 32",
        total_half_degrade
    );
}

#[test]
fn test_stutter_repeats_events() {
    // stutter should repeat each event n times in quick succession

    let pattern = parse_mini_notation("bd sn");

    // Test with stutter 3 - each event should appear 3 times
    let pattern_stutter = pattern.clone().stutter(3);
    let mut total = 0;
    for cycle in 0..4 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };
        total += pattern_stutter.query(&state).len();
    }

    // Should have 2 events × 3 repetitions × 4 cycles = 24 events
    assert_eq!(total, 24, "stutter 3 should triple the event count");
}

#[test]
fn test_chop_slices_pattern() {
    // chop n should slice each cycle into n equal parts

    let pattern = parse_mini_notation("bd");

    // Test with chop 4 - one event should become 4 slices
    let pattern_chop = pattern.clone().chop(4);
    let state = State {
        span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(1.0)),
        controls: HashMap::new(),
    };
    let events = pattern_chop.query(&state);

    // Should have 4 slices in one cycle
    assert_eq!(events.len(), 4, "chop 4 should create 4 slices per cycle");

    // Verify slices are evenly spaced
    if events.len() == 4 {
        let first_duration = events[0].part.duration().to_float();
        assert!(
            (first_duration - 0.25).abs() < 0.01,
            "Each chop slice should be 1/4 cycle duration, got {}",
            first_duration
        );
    }
}

#[test]
fn test_scramble_randomizes_events() {
    // scramble should randomize the order of events within each cycle

    let pattern = parse_mini_notation("bd sn hh cp");

    // Test with scramble 4
    let pattern_scramble = pattern.clone().scramble(4);
    let mut total = 0;
    for cycle in 0..4 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };
        let events = pattern_scramble.query(&state);
        total += events.len();
    }

    // Should preserve event count (4 events × 4 cycles = 16)
    assert_eq!(total, 16, "scramble should preserve event count");

    // Note: We can't easily test randomness deterministically,
    // but we verify the count is preserved
}

// ========== Level 2: DSL Integration Tests ==========

#[test]
fn test_degrade_by_dsl() {
    // Test degradeBy through the DSL
    let code = r#"
tempo: 2.0
out: s "bd*8" $ degradeBy 1.0
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    // Render 1 second - should be silent since degradeBy 1.0 removes all events
    let buffer = graph.render(44100);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    assert!(
        rms < 0.001,
        "degradeBy 1.0 should produce near-silence, got RMS={}",
        rms
    );
}

#[test]
fn test_stutter_dsl() {
    // Test stutter through the DSL
    let code = r#"
tempo: 2.0
out: s "bd" $ stutter 3
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    // Render 1 second
    let buffer = graph.render(44100);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    assert!(rms > 0.01, "stutter should produce audio, got RMS={}", rms);
}

#[test]
fn test_chop_dsl() {
    // Test chop through the DSL
    let code = r#"
tempo: 2.0
out: s "bd" $ chop 4
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    // Render 1 second
    let buffer = graph.render(44100);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    assert!(rms > 0.01, "chop should produce audio, got RMS={}", rms);
}

#[test]
fn test_scramble_dsl() {
    // Test scramble through the DSL
    let code = r#"
tempo: 2.0
out: s "bd sn hh cp" $ scramble 4
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    // Render 1 second
    let buffer = graph.render(44100);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    assert!(rms > 0.01, "scramble should produce audio, got RMS={}", rms);
}
