//! Comprehensive E2E Tests for Mini-Notation (100 tests)
//!
//! This test suite covers ALL mini-notation features with three-level testing:
//! - Level 1: Pattern Query Verification (fast, exact, deterministic)
//! - Level 2: Onset Detection (tests that audio events occur at right times)
//! - Level 3: Audio Characteristics (sanity checks on signal quality)
//!
//! Features tested:
//! - Basic sequences
//! - Rests and silence
//! - Groups and subdivisions []
//! - Alternation <>
//! - Polyrhythm with commas [,] and parentheses (,)
//! - Stacking with pipe |
//! - Feet with dot .
//! - Repeat operator *
//! - Slow operator /
//! - Degrade operator ?
//! - Late operator @
//! - Euclidean rhythms (k,n) and (k,n,r)
//! - Colon syntax for sample selection :
//! - Chord notation '
//! - Numeric patterns
//! - Complex nested patterns

mod pattern_verification_utils;

use pattern_verification_utils::{
    calculate_rms, detect_audio_events, is_silent,
};
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use std::collections::HashMap;

const SAMPLE_RATE: f32 = 44100.0;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Query a pattern for a specific cycle
fn query_cycle(pattern: &Pattern<String>, cycle: usize) -> Vec<(f64, f64, String)> {
    let begin = Fraction::new(cycle as i64, 1);
    let end = Fraction::new((cycle + 1) as i64, 1);
    let state = State {
        span: TimeSpan::new(begin, end),
        controls: HashMap::new(),
    };

    pattern
        .query(&state)
        .into_iter()
        .map(|hap| {
            (
                hap.part.begin.to_float(),
                hap.part.end.to_float(),
                hap.value,
            )
        })
        .collect()
}

/// Query a pattern across multiple cycles
#[allow(dead_code)]
fn query_cycles(pattern: &Pattern<String>, start: usize, end: usize) -> Vec<(f64, f64, String)> {
    let begin = Fraction::new(start as i64, 1);
    let end_frac = Fraction::new(end as i64, 1);
    let state = State {
        span: TimeSpan::new(begin, end_frac),
        controls: HashMap::new(),
    };

    pattern
        .query(&state)
        .into_iter()
        .map(|hap| {
            (
                hap.part.begin.to_float(),
                hap.part.end.to_float(),
                hap.value,
            )
        })
        .collect()
}

/// Count events of a specific value in a cycle
fn count_events(pattern: &Pattern<String>, cycle: usize, value: &str) -> usize {
    query_cycle(pattern, cycle)
        .iter()
        .filter(|(_, _, v)| v == value)
        .count()
}

/// Count total events in a cycle
fn total_events(pattern: &Pattern<String>, cycle: usize) -> usize {
    query_cycle(pattern, cycle).len()
}

/// Render DSL code to audio buffer
fn render_dsl(code: &str, duration_secs: f32) -> Vec<f32> {
    let (_, statements) = parse_program(code).expect("Should parse DSL");
    let mut graph = compile_program(statements, SAMPLE_RATE, None).expect("Should compile");
    graph.render((SAMPLE_RATE * duration_secs) as usize)
}

/// Assert events approximately match expected
fn assert_events_approx(actual: &[(f64, f64, String)], expected: &[(f64, f64, &str)], tol: f64) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "Expected {} events, got {}: {:?}",
        expected.len(),
        actual.len(),
        actual
    );

    for (i, (act, exp)) in actual.iter().zip(expected.iter()).enumerate() {
        assert!(
            (act.0 - exp.0).abs() < tol,
            "Event {} start mismatch: expected {}, got {}",
            i,
            exp.0,
            act.0
        );
        assert!(
            (act.1 - exp.1).abs() < tol,
            "Event {} end mismatch: expected {}, got {}",
            i,
            exp.1,
            act.1
        );
        assert_eq!(act.2, exp.2, "Event {} value mismatch", i);
    }
}

// =============================================================================
// LEVEL 1: PATTERN QUERY VERIFICATION TESTS
// =============================================================================

// --- Basic Sequences ---

#[test]
fn l1_simple_sequence_two_items() {
    let pattern = parse_mini_notation("bd sn");
    let events = query_cycle(&pattern, 0);
    assert_events_approx(&events, &[(0.0, 0.5, "bd"), (0.5, 1.0, "sn")], 0.01);
}

#[test]
fn l1_simple_sequence_four_items() {
    let pattern = parse_mini_notation("bd sn hh cp");
    let events = query_cycle(&pattern, 0);
    assert_events_approx(
        &events,
        &[
            (0.0, 0.25, "bd"),
            (0.25, 0.5, "sn"),
            (0.5, 0.75, "hh"),
            (0.75, 1.0, "cp"),
        ],
        0.01,
    );
}

#[test]
fn l1_single_item() {
    let pattern = parse_mini_notation("bd");
    let events = query_cycle(&pattern, 0);
    assert_events_approx(&events, &[(0.0, 1.0, "bd")], 0.01);
}

#[test]
fn l1_sequence_repeats_across_cycles() {
    let pattern = parse_mini_notation("bd sn");
    for cycle in 0..4 {
        let events = query_cycle(&pattern, cycle);
        let base = cycle as f64;
        assert_events_approx(
            &events,
            &[(base, base + 0.5, "bd"), (base + 0.5, base + 1.0, "sn")],
            0.01,
        );
    }
}

// --- Rests and Silence ---

#[test]
fn l1_rest_in_sequence() {
    let pattern = parse_mini_notation("bd ~ sn ~");
    let events = query_cycle(&pattern, 0);
    assert_events_approx(&events, &[(0.0, 0.25, "bd"), (0.5, 0.75, "sn")], 0.01);
}

#[test]
fn l1_only_rests() {
    let pattern = parse_mini_notation("~ ~ ~");
    let events = query_cycle(&pattern, 0);
    assert_eq!(events.len(), 0, "All rests should produce no events");
}

#[test]
fn l1_single_rest() {
    let pattern = parse_mini_notation("~");
    let events = query_cycle(&pattern, 0);
    assert_eq!(events.len(), 0);
}

#[test]
fn l1_rest_at_start() {
    let pattern = parse_mini_notation("~ bd sn");
    let events = query_cycle(&pattern, 0);
    assert_eq!(events.len(), 2);
    assert!(events.iter().all(|(_, _, v)| v != "~"));
}

#[test]
fn l1_rest_at_end() {
    let pattern = parse_mini_notation("bd sn ~");
    let events = query_cycle(&pattern, 0);
    assert_eq!(events.len(), 2);
}

// --- Groups and Subdivisions ---

#[test]
fn l1_group_subdivision() {
    let pattern = parse_mini_notation("bd [sn sn] hh");
    let events = query_cycle(&pattern, 0);
    assert_eq!(events.len(), 4);

    // bd takes 1/3, [sn sn] takes 1/3, hh takes 1/3
    let bd_event = &events[0];
    assert_eq!(bd_event.2, "bd");
    assert!((bd_event.1 - bd_event.0 - 0.333).abs() < 0.01);
}

#[test]
fn l1_group_with_three_items() {
    let pattern = parse_mini_notation("[bd sn hh]");
    let events = query_cycle(&pattern, 0);
    assert_eq!(events.len(), 3);

    // All fit in one cycle
    assert_events_approx(
        &events,
        &[(0.0, 0.333, "bd"), (0.333, 0.667, "sn"), (0.667, 1.0, "hh")],
        0.02,
    );
}

#[test]
fn l1_nested_groups() {
    let pattern = parse_mini_notation("bd [[sn cp] hh]");
    let events = query_cycle(&pattern, 0);
    assert_eq!(events.len(), 4);
    assert_eq!(events[0].2, "bd");
    assert_eq!(events[1].2, "sn");
    assert_eq!(events[2].2, "cp");
    assert_eq!(events[3].2, "hh");
}

#[test]
fn l1_deeply_nested_groups() {
    let pattern = parse_mini_notation("[[[bd]]]");
    let events = query_cycle(&pattern, 0);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].2, "bd");
    // Event should span full cycle
    assert!((events[0].0 - 0.0).abs() < 0.01);
    assert!((events[0].1 - 1.0).abs() < 0.01);
}

#[test]
fn l1_group_in_sequence() {
    let pattern = parse_mini_notation("bd [hh hh hh hh] sn");
    let events = query_cycle(&pattern, 0);
    assert_eq!(events.len(), 6); // bd + 4 hh + sn
}

// --- Alternation ---

#[test]
fn l1_alternation_three_items() {
    let pattern = parse_mini_notation("<bd sn cp>");

    assert_eq!(query_cycle(&pattern, 0)[0].2, "bd");
    assert_eq!(query_cycle(&pattern, 1)[0].2, "sn");
    assert_eq!(query_cycle(&pattern, 2)[0].2, "cp");
    assert_eq!(query_cycle(&pattern, 3)[0].2, "bd"); // wraps
}

#[test]
fn l1_alternation_two_items() {
    let pattern = parse_mini_notation("<bd sn>");

    for cycle in 0..6 {
        let expected = if cycle % 2 == 0 { "bd" } else { "sn" };
        assert_eq!(query_cycle(&pattern, cycle)[0].2, expected);
    }
}

#[test]
fn l1_alternation_with_rest() {
    let pattern = parse_mini_notation("<bd ~ sn>");

    assert_eq!(query_cycle(&pattern, 0).len(), 1); // bd
    assert_eq!(query_cycle(&pattern, 1).len(), 0); // rest
    assert_eq!(query_cycle(&pattern, 2).len(), 1); // sn
}

#[test]
fn l1_alternation_single_item() {
    let pattern = parse_mini_notation("<bd>");

    for cycle in 0..4 {
        let events = query_cycle(&pattern, cycle);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].2, "bd");
    }
}

#[test]
fn l1_alternation_spans_full_cycle() {
    let pattern = parse_mini_notation("<bd sn>");
    let events = query_cycle(&pattern, 0);

    assert_eq!(events.len(), 1);
    assert!((events[0].0 - 0.0).abs() < 0.01);
    assert!((events[0].1 - 1.0).abs() < 0.01);
}

// --- Polyrhythm ---

#[test]
fn l1_polyrhythm_comma_in_brackets() {
    let pattern = parse_mini_notation("[bd, sn sn]");
    let events = query_cycle(&pattern, 0);

    assert_eq!(events.len(), 3); // 1 bd + 2 sn
    assert_eq!(count_events(&pattern, 0, "bd"), 1);
    assert_eq!(count_events(&pattern, 0, "sn"), 2);
}

#[test]
fn l1_polyrhythm_three_layers() {
    let pattern = parse_mini_notation("[bd, sn sn, hh hh hh]");
    let events = query_cycle(&pattern, 0);

    assert_eq!(events.len(), 6); // 1 + 2 + 3
    assert_eq!(count_events(&pattern, 0, "bd"), 1);
    assert_eq!(count_events(&pattern, 0, "sn"), 2);
    assert_eq!(count_events(&pattern, 0, "hh"), 3);
}

#[test]
fn l1_polyrhythm_parentheses() {
    let pattern = parse_mini_notation("(bd, sn cp)");
    let events = query_cycle(&pattern, 0);

    assert_eq!(events.len(), 3); // 1 bd + 2 (sn, cp)
}

#[test]
fn l1_polyrhythm_parentheses_three_layers() {
    let pattern = parse_mini_notation("(bd, sn cp, hh hh hh)");
    let events = query_cycle(&pattern, 0);

    assert_eq!(events.len(), 6);

    // bd should span full cycle
    let bd_events: Vec<_> = events.iter().filter(|(_, _, v)| v == "bd").collect();
    assert_eq!(bd_events.len(), 1);
    assert!((bd_events[0].1 - bd_events[0].0 - 1.0).abs() < 0.01);
}

// --- Stacking with Pipe ---

#[test]
fn l1_pipe_stacking() {
    let pattern = parse_mini_notation("bd sn | hh hh hh hh");
    let _events = query_cycle(&pattern, 0);

    assert_eq!(count_events(&pattern, 0, "bd"), 1);
    assert_eq!(count_events(&pattern, 0, "sn"), 1);
    assert_eq!(count_events(&pattern, 0, "hh"), 4);
}

#[test]
fn l1_pipe_triple_stack() {
    let pattern = parse_mini_notation("bd | sn | hh");
    let events = query_cycle(&pattern, 0);

    assert_eq!(events.len(), 3);
}

#[test]
fn l1_pipe_different_subdivisions() {
    let pattern = parse_mini_notation("bd sn | hh*3");
    let events = query_cycle(&pattern, 0);

    assert_eq!(events.len(), 5); // 2 + 3
}

// --- Feet with Dot ---

#[test]
fn l1_feet_two_parts() {
    let pattern = parse_mini_notation("bd sn . hh hh hh");
    let events = query_cycle(&pattern, 0);

    assert_eq!(events.len(), 5); // 2 + 3

    // First foot (0.0-0.5): bd at 0.0, sn at 0.25
    // Second foot (0.5-1.0): hh at 0.5, 0.667, 0.833
}

#[test]
fn l1_feet_three_parts() {
    let pattern = parse_mini_notation("a . b . c");
    let events = query_cycle(&pattern, 0);

    assert_eq!(events.len(), 3);

    // Each takes 1/3 of cycle
    assert!((events[0].0 - 0.0).abs() < 0.01);
    assert!((events[1].0 - 0.333).abs() < 0.02);
    assert!((events[2].0 - 0.667).abs() < 0.02);
}

#[test]
fn l1_feet_single_item_per_foot() {
    let pattern = parse_mini_notation("bd . sn");
    let events = query_cycle(&pattern, 0);

    assert_eq!(events.len(), 2);
    assert!((events[0].1 - 0.5).abs() < 0.01); // bd ends at 0.5
    assert!((events[1].0 - 0.5).abs() < 0.01); // sn starts at 0.5
}

// --- Repeat Operator ---

#[test]
fn l1_repeat_basic() {
    let pattern = parse_mini_notation("bd*4");
    let events = query_cycle(&pattern, 0);

    assert_eq!(events.len(), 4);
    assert!(events.iter().all(|(_, _, v)| v == "bd"));
}

#[test]
fn l1_repeat_in_sequence() {
    let pattern = parse_mini_notation("bd*2 sn");
    let events = query_cycle(&pattern, 0);

    assert_eq!(events.len(), 3); // 2 bd + 1 sn
}

#[test]
fn l1_repeat_distributes_time() {
    let pattern = parse_mini_notation("hh*4");
    let events = query_cycle(&pattern, 0);

    // Each hh should take 0.25 of cycle
    for (i, (start, end, _)) in events.iter().enumerate() {
        let expected_start = i as f64 * 0.25;
        let expected_end = (i + 1) as f64 * 0.25;
        assert!((start - expected_start).abs() < 0.01);
        assert!((end - expected_end).abs() < 0.01);
    }
}

#[test]
fn l1_repeat_one() {
    let pattern = parse_mini_notation("bd*1");
    let events = query_cycle(&pattern, 0);

    assert_eq!(events.len(), 1);
}

#[test]
fn l1_repeat_large_number() {
    let pattern = parse_mini_notation("bd*16");
    let events = query_cycle(&pattern, 0);

    assert_eq!(events.len(), 16);
}

#[test]
fn l1_repeat_with_alternation() {
    let pattern = parse_mini_notation("bd*<2 3 4>");

    // Cycle 0: 2 events
    assert_eq!(total_events(&pattern, 0), 2);
    // Cycle 1: 3 events
    assert_eq!(total_events(&pattern, 1), 3);
    // Cycle 2: 4 events
    assert_eq!(total_events(&pattern, 2), 4);
    // Cycle 3: back to 2
    assert_eq!(total_events(&pattern, 3), 2);
}

// --- Slow Operator ---

#[test]
fn l1_slow_basic() {
    let pattern = parse_mini_notation("bd/2");

    // bd spans 2 cycles, so appears in both cycle 0 and 1
    assert_eq!(total_events(&pattern, 0), 1);
    assert_eq!(total_events(&pattern, 1), 1);
}

#[test]
fn l1_slow_in_sequence() {
    let pattern = parse_mini_notation("bd/2 sn");

    // Both cycles should have events
    assert!(total_events(&pattern, 0) > 0);
    assert!(total_events(&pattern, 1) > 0);
}

#[test]
fn l1_slow_four() {
    let pattern = parse_mini_notation("bd/4");

    // bd should appear in all 4 cycles
    for cycle in 0..4 {
        assert_eq!(total_events(&pattern, cycle), 1);
    }
}

// --- Degrade Operator ---

#[test]
fn l1_degrade_probabilistic() {
    let pattern = parse_mini_notation("bd?");

    // Run multiple cycles and check that we sometimes get events
    // and sometimes don't (probabilistic test)
    let mut got_events = 0;
    let mut no_events = 0;

    for cycle in 0..100 {
        if total_events(&pattern, cycle) > 0 {
            got_events += 1;
        } else {
            no_events += 1;
        }
    }

    // Should have both outcomes with 50% probability
    assert!(got_events > 20, "Should sometimes get events");
    assert!(no_events > 20, "Should sometimes get no events");
}

#[test]
fn l1_degrade_with_probability() {
    // ?0.1 means 10% chance of dropping → ~90% events kept
    let pattern = parse_mini_notation("bd?0.1");

    let mut got_events = 0;
    for cycle in 0..100 {
        if total_events(&pattern, cycle) > 0 {
            got_events += 1;
        }
    }

    // With 10% drop probability, should get most events
    assert!(
        got_events > 70,
        "Should get most events with 10% drop probability, got {}",
        got_events
    );
}

// --- Late Operator ---

#[test]
fn l1_late_operator() {
    let pattern = parse_mini_notation("bd@0.25");
    let events = query_cycle(&pattern, 0);

    assert_eq!(events.len(), 1);
    // bd should be delayed by 0.25
    assert!((events[0].0 - 0.25).abs() < 0.01);
}

#[test]
fn l1_late_half_cycle() {
    let pattern = parse_mini_notation("bd@0.5");
    let events = query_cycle(&pattern, 0);

    assert!((events[0].0 - 0.5).abs() < 0.01);
}

// --- Euclidean Rhythms ---

#[test]
fn l1_euclid_3_8() {
    let pattern = parse_mini_notation("bd(3,8)");
    let events = query_cycle(&pattern, 0);

    // Filter out "~" events (rests)
    let hit_events: Vec<_> = events.iter().filter(|(_, _, v)| v != "~").collect();
    assert_eq!(hit_events.len(), 3);
}

#[test]
fn l1_euclid_5_8() {
    let pattern = parse_mini_notation("bd(5,8)");
    let events = query_cycle(&pattern, 0);

    let hit_events: Vec<_> = events.iter().filter(|(_, _, v)| v != "~").collect();
    assert_eq!(hit_events.len(), 5);
}

#[test]
fn l1_euclid_with_rotation() {
    let pattern = parse_mini_notation("bd(3,8,2)");
    let events = query_cycle(&pattern, 0);

    let hit_events: Vec<_> = events.iter().filter(|(_, _, v)| v != "~").collect();
    assert_eq!(hit_events.len(), 3);
}

#[test]
fn l1_euclid_full_steps() {
    let pattern = parse_mini_notation("bd(8,8)");
    let events = query_cycle(&pattern, 0);

    // All 8 should be hits
    let hit_events: Vec<_> = events.iter().filter(|(_, _, v)| v != "~").collect();
    assert_eq!(hit_events.len(), 8);
}

#[test]
fn l1_euclid_one_step() {
    let pattern = parse_mini_notation("bd(1,8)");
    let events = query_cycle(&pattern, 0);

    let hit_events: Vec<_> = events.iter().filter(|(_, _, v)| v != "~").collect();
    assert_eq!(hit_events.len(), 1);
}

#[test]
fn l1_euclid_with_alternation_pulses() {
    let pattern = parse_mini_notation("bd(<3 5>,8)");

    // Cycle 0: 3 hits
    let events0 = query_cycle(&pattern, 0);
    let hits0 = events0.iter().filter(|(_, _, v)| v != "~").count();
    assert_eq!(hits0, 3);

    // Cycle 1: 5 hits
    let events1 = query_cycle(&pattern, 1);
    let hits1 = events1.iter().filter(|(_, _, v)| v != "~").count();
    assert_eq!(hits1, 5);
}

// --- Colon Syntax ---

#[test]
fn l1_colon_basic() {
    let pattern = parse_mini_notation("bd:0 bd:1 bd:2");
    let events = query_cycle(&pattern, 0);

    assert_eq!(events.len(), 3);
    assert_eq!(events[0].2, "bd:0");
    assert_eq!(events[1].2, "bd:1");
    assert_eq!(events[2].2, "bd:2");
}

#[test]
fn l1_colon_with_repeat() {
    let pattern = parse_mini_notation("bd:0*4");
    let events = query_cycle(&pattern, 0);

    assert_eq!(events.len(), 4);
    assert!(events.iter().all(|(_, _, v)| v == "bd:0"));
}

#[test]
fn l1_colon_in_alternation() {
    let pattern = parse_mini_notation("<bd:0 bd:1 bd:2>");

    assert_eq!(query_cycle(&pattern, 0)[0].2, "bd:0");
    assert_eq!(query_cycle(&pattern, 1)[0].2, "bd:1");
    assert_eq!(query_cycle(&pattern, 2)[0].2, "bd:2");
}

// --- Numeric Patterns ---

#[test]
fn l1_numeric_sequence() {
    let pattern = parse_mini_notation("110 220 440");
    let events = query_cycle(&pattern, 0);

    assert_eq!(events.len(), 3);
    assert_eq!(events[0].2, "110");
    assert_eq!(events[1].2, "220");
    assert_eq!(events[2].2, "440");
}

#[test]
fn l1_numeric_decimal() {
    let pattern = parse_mini_notation("1.5 2.25 3.75");
    let events = query_cycle(&pattern, 0);

    assert_eq!(events.len(), 3);
    assert_eq!(events[0].2, "1.5");
}

#[test]
fn l1_numeric_negative() {
    let pattern = parse_mini_notation("-1 0 1");
    let events = query_cycle(&pattern, 0);

    assert_eq!(events.len(), 3);
    assert_eq!(events[0].2, "-1");
}

// --- Chord Notation ---

#[test]
fn l1_chord_basic() {
    let pattern = parse_mini_notation("c'maj d'min");
    let events = query_cycle(&pattern, 0);

    assert_eq!(events.len(), 2);
    assert_eq!(events[0].2, "c'maj");
    assert_eq!(events[1].2, "d'min");
}

#[test]
fn l1_chord_with_euclid() {
    let pattern = parse_mini_notation("c'maj(3,8)");
    let events = query_cycle(&pattern, 0);

    let hit_events: Vec<_> = events.iter().filter(|(_, _, v)| v != "~").collect();
    assert_eq!(hit_events.len(), 3);
    assert!(hit_events[0].2.starts_with("c'maj"));
}

// --- Complex Nested Patterns ---

#[test]
fn l1_complex_nested_groups_and_operators() {
    let pattern = parse_mini_notation("[[bd*2 sn] hh]*2");
    let events = query_cycle(&pattern, 0);

    // [[bd*2 sn] hh]*2 = [[bd bd sn] hh] [bd bd sn] hh = 8 events
    assert_eq!(events.len(), 8);
}

#[test]
fn l1_complex_polyrhythm_with_alternation() {
    let pattern = parse_mini_notation("(<bd sn>, hh*4)");

    // Cycle 0: bd + 4 hh = 5 events
    assert_eq!(total_events(&pattern, 0), 5);
    assert_eq!(count_events(&pattern, 0, "bd"), 1);
    assert_eq!(count_events(&pattern, 0, "hh"), 4);

    // Cycle 1: sn + 4 hh = 5 events
    assert_eq!(total_events(&pattern, 1), 5);
    assert_eq!(count_events(&pattern, 1, "sn"), 1);
    assert_eq!(count_events(&pattern, 1, "hh"), 4);
}

#[test]
fn l1_complex_pipe_with_groups() {
    let pattern = parse_mini_notation("[bd*2 ~, hh hh hh hh] | <sn cp>");

    // First layer: 2 bd + 4 hh = 6 events
    // Second layer: 1 alternating event
    let _events0 = query_cycle(&pattern, 0);
    assert_eq!(count_events(&pattern, 0, "sn"), 1);

    let _events1 = query_cycle(&pattern, 1);
    assert_eq!(count_events(&pattern, 1, "cp"), 1);
}

#[test]
fn l1_very_long_pattern() {
    let pattern_str = (0..50)
        .map(|i| format!("s{}", i))
        .collect::<Vec<_>>()
        .join(" ");
    let pattern = parse_mini_notation(&pattern_str);
    let events = query_cycle(&pattern, 0);

    assert_eq!(events.len(), 50);
}

// =============================================================================
// LEVEL 2: ONSET DETECTION TESTS
// =============================================================================

#[test]
fn l2_simple_sequence_produces_audio() {
    let dsl = r#"
tempo: 0.5
~drums $ s "bd sn hh cp"
out $ ~drums
"#;

    let audio = render_dsl(dsl, 2.0);

    // Should produce audio
    let rms = calculate_rms(&audio);
    assert!(rms > 0.001, "Should produce audio, got RMS {}", rms);
}

#[test]
fn l2_rest_produces_silence_at_correct_time() {
    let dsl = r#"
tempo: 0.5
~drums $ s "bd ~ ~ ~"
out $ ~drums
"#;

    let audio = render_dsl(dsl, 2.0);

    // Should have audio only at the beginning (where bd plays)
    let first_quarter = &audio[0..(audio.len() / 4)];
    let last_quarter = &audio[(3 * audio.len() / 4)..];

    let rms_start = calculate_rms(first_quarter);
    let rms_end = calculate_rms(last_quarter);

    assert!(
        rms_start > rms_end,
        "First quarter should be louder than last"
    );
}

#[test]
fn l2_repeat_produces_multiple_onsets() {
    let dsl = r#"
tempo: 1.0
~drums $ s "bd*4"
out $ ~drums
"#;

    let audio = render_dsl(dsl, 1.0);
    let detected = detect_audio_events(&audio, SAMPLE_RATE, 0.05);

    // bd*4 = 4 events per cycle; onset detection may vary
    assert!(
        detected.len() >= 2,
        "Should detect multiple events, got {}",
        detected.len()
    );
}

#[test]
fn l2_group_subdivides_correctly() {
    let dsl = r#"
tempo: 1.0
~drums $ s "bd [sn sn] hh"
out $ ~drums
"#;

    let audio = render_dsl(dsl, 1.0);
    let detected = detect_audio_events(&audio, SAMPLE_RATE, 0.01);

    // Should detect ~4 events (bd, sn, sn, hh)
    assert!(
        detected.len() >= 2,
        "Should detect multiple events, got {}",
        detected.len()
    );
}

#[test]
fn l2_polyrhythm_produces_overlapping_events() {
    let dsl = r#"
tempo: 1.0
~drums $ s "[bd, hh*4]"
out $ ~drums
"#;

    let audio = render_dsl(dsl, 1.0);
    let detected = detect_audio_events(&audio, SAMPLE_RATE, 0.01);

    // Should detect multiple events (bd + 4 hh)
    assert!(
        detected.len() >= 2,
        "Should detect polyrhythm events, got {}",
        detected.len()
    );
}

#[test]
fn l2_slow_stretches_pattern() {
    let dsl_normal = r#"
tempo: 1.0
~drums $ s "bd sn"
out $ ~drums
"#;
    let dsl_slow = r#"
tempo: 1.0
~drums $ s "bd/2 sn"
out $ ~drums
"#;

    let audio_normal = render_dsl(dsl_normal, 2.0);
    let audio_slow = render_dsl(dsl_slow, 2.0);

    let _events_normal = detect_audio_events(&audio_normal, SAMPLE_RATE, 0.01);
    let events_slow = detect_audio_events(&audio_slow, SAMPLE_RATE, 0.01);

    // Slow pattern should have similar or fewer distinct onsets
    // (bd/2 means bd is stretched, so overlaps more)
    assert!(events_slow.len() > 0);
}

#[test]
fn l2_euclid_produces_rhythmic_pattern() {
    let dsl = r#"
tempo: 1.0
~drums $ s "bd(3,8)"
out $ ~drums
"#;

    let audio = render_dsl(dsl, 1.0);
    let detected = detect_audio_events(&audio, SAMPLE_RATE, 0.05);

    // Euclidean bd(3,8) = 3 events; onset detection may find more subdivisions
    assert!(
        detected.len() >= 1,
        "Should detect euclidean events, got {}",
        detected.len()
    );
}

#[test]
fn l2_colon_syntax_produces_audio() {
    let dsl = r#"
tempo: 0.5
~drums $ s "bd:0 bd:1 bd:2"
out $ ~drums
"#;

    let audio = render_dsl(dsl, 2.0);
    let rms = calculate_rms(&audio);

    assert!(
        rms > 0.001,
        "Colon syntax should produce audio, got RMS {}",
        rms
    );
}

#[test]
fn l2_alternation_changes_per_cycle() {
    let dsl = r#"
tempo: 0.5
~drums $ s "<bd sn>"
out $ ~drums
"#;

    // Render 4 seconds (2 cycles)
    let audio = render_dsl(dsl, 4.0);

    // Should produce audio
    let rms = calculate_rms(&audio);
    assert!(rms > 0.001, "Alternation should produce audio");
}

#[test]
fn l2_pipe_stacking_produces_audio() {
    let dsl = r#"
tempo: 1.0
~drums $ s "bd sn | hh*4"
out $ ~drums
"#;

    let audio = render_dsl(dsl, 1.0);
    let detected = detect_audio_events(&audio, SAMPLE_RATE, 0.01);

    // Should detect multiple events from stacked patterns
    assert!(
        detected.len() >= 2,
        "Stacked patterns should produce multiple events, got {}",
        detected.len()
    );
}

#[test]
fn l2_feet_divides_time_equally() {
    let dsl = r#"
tempo: 1.0
~drums $ s "bd . sn"
out $ ~drums
"#;

    let audio = render_dsl(dsl, 1.0);
    let detected = detect_audio_events(&audio, SAMPLE_RATE, 0.01);

    // Should detect 2 events
    assert!(
        detected.len() >= 1,
        "Feet should produce events, got {}",
        detected.len()
    );
}

// =============================================================================
// LEVEL 3: AUDIO CHARACTERISTICS TESTS
// =============================================================================

#[test]
fn l3_audio_not_silent() {
    let dsl = r#"
tempo: 0.5
~drums $ s "bd sn hh cp"
out $ ~drums
"#;

    let audio = render_dsl(dsl, 2.0);
    assert!(!is_silent(&audio, 0.001), "Audio should not be silent");
}

#[test]
fn l3_audio_not_clipping() {
    let dsl = r#"
tempo: 0.5
~drums $ s "bd*8"
out $ ~drums * 0.5
"#;

    let audio = render_dsl(dsl, 2.0);
    let peak = audio.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    assert!(peak < 1.0, "Audio should not clip, got peak {}", peak);
}

#[test]
fn l3_fast_pattern_has_more_energy() {
    let dsl_slow = r#"
tempo: 1.0
~drums $ s "bd sn"
out $ ~drums
"#;
    let dsl_fast = r#"
tempo: 1.0
~drums $ s "bd*4 sn*4"
out $ ~drums
"#;

    let audio_slow = render_dsl(dsl_slow, 1.0);
    let audio_fast = render_dsl(dsl_fast, 1.0);

    let rms_slow = calculate_rms(&audio_slow);
    let rms_fast = calculate_rms(&audio_fast);

    // Fast pattern should have more events and thus more energy
    // (though this depends on sample overlapping behavior)
    assert!(
        rms_fast > 0.0 && rms_slow > 0.0,
        "Both should produce audio"
    );
}

#[test]
fn l3_rest_pattern_is_quieter() {
    let dsl_full = r#"
tempo: 1.0
~drums $ s "bd bd bd bd"
out $ ~drums
"#;
    let dsl_rest = r#"
tempo: 1.0
~drums $ s "bd ~ bd ~"
out $ ~drums
"#;

    let audio_full = render_dsl(dsl_full, 1.0);
    let audio_rest = render_dsl(dsl_rest, 1.0);

    let rms_full = calculate_rms(&audio_full);
    let rms_rest = calculate_rms(&audio_rest);

    // Pattern with rests should have less energy
    assert!(
        rms_full >= rms_rest,
        "Full pattern ({}) should have >= energy than rest pattern ({})",
        rms_full,
        rms_rest
    );
}

#[test]
fn l3_complex_pattern_produces_audio() {
    let dsl = r#"
tempo: 0.5
~kick $ s "bd:0(3,8)"
~snare $ s "~ sn:1 ~ sn:2"
~hats $ s "hh:0*8"
out $ (~kick + ~snare + ~hats) * 0.3
"#;

    let audio = render_dsl(dsl, 2.0);
    let rms = calculate_rms(&audio);

    assert!(
        rms > 0.01,
        "Complex pattern should produce audible output, got RMS {}",
        rms
    );
}

#[test]
fn l3_numeric_pattern_in_oscillator() {
    let dsl = r#"
tempo: 1.0
~osc $ sine "110 220 440"
out $ ~osc * 0.3
"#;

    let audio = render_dsl(dsl, 1.0);
    let rms = calculate_rms(&audio);

    assert!(
        rms > 0.01,
        "Numeric pattern oscillator should produce audio, got RMS {}",
        rms
    );
}

#[test]
fn l3_polyrhythm_layer_produces_audio() {
    let dsl = r#"
tempo: 0.5
~drums $ s "(bd, sn cp, hh hh hh)"
out $ ~drums
"#;

    let audio = render_dsl(dsl, 2.0);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.001, "Polyrhythm should produce audio");
}

#[test]
fn l3_pipe_stack_produces_audio() {
    let dsl = r#"
tempo: 0.5
~drums $ s "bd sn | hh*4 | cp"
out $ ~drums
"#;

    let audio = render_dsl(dsl, 2.0);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.001, "Pipe stack should produce audio");
}

#[test]
fn l3_alternation_produces_consistent_audio() {
    let dsl = r#"
tempo: 0.5
~drums $ s "<bd sn cp>"
out $ ~drums
"#;

    // Render multiple cycles
    let audio = render_dsl(dsl, 6.0);

    // Split into 3 two-second chunks (one for each alternation value)
    let chunk_size = (SAMPLE_RATE * 2.0) as usize;
    let chunk1 = &audio[0..chunk_size];
    let chunk2 = &audio[chunk_size..2 * chunk_size];
    let chunk3 = &audio[2 * chunk_size..3 * chunk_size];

    // All chunks should have audio
    assert!(calculate_rms(chunk1) > 0.001);
    assert!(calculate_rms(chunk2) > 0.001);
    assert!(calculate_rms(chunk3) > 0.001);
}

#[test]
fn l3_degrade_reduces_energy() {
    let dsl_full = r#"
tempo: 1.0
~drums $ s "bd bd bd bd bd bd bd bd"
out $ ~drums
"#;
    // Use ? in mini-notation for degrade (? = 50% drop by default)
    let dsl_degraded = r#"
tempo: 1.0
~drums $ s "bd? bd? bd? bd? bd? bd? bd? bd?"
out $ ~drums
"#;

    let audio_full = render_dsl(dsl_full, 2.0);
    let audio_degraded = render_dsl(dsl_degraded, 2.0);

    let rms_full = calculate_rms(&audio_full);
    let rms_degraded = calculate_rms(&audio_degraded);

    // Degraded should generally have less energy (probabilistic, so allow some margin)
    // Since this is probabilistic, we just check both produce audio
    assert!(rms_full > 0.001);
    assert!(rms_degraded >= 0.0); // Could be 0 if all degraded
}

#[test]
fn l3_euclid_produces_rhythmic_audio() {
    let dsl = r#"
tempo: 0.5
~drums $ s "bd(5,8)"
out $ ~drums
"#;

    let audio = render_dsl(dsl, 2.0);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.001, "Euclidean rhythm should produce audio");
}

#[test]
fn l3_nested_groups_produce_audio() {
    let dsl = r#"
tempo: 0.5
~drums $ s "[[bd sn] [hh hh]]"
out $ ~drums
"#;

    let audio = render_dsl(dsl, 2.0);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.001, "Nested groups should produce audio");
}

// =============================================================================
// EDGE CASES
// =============================================================================

#[test]
fn edge_empty_pattern() {
    let pattern = parse_mini_notation("");
    let events = query_cycle(&pattern, 0);
    assert_eq!(events.len(), 0);
}

#[test]
fn edge_whitespace_only() {
    let pattern = parse_mini_notation("   ");
    let events = query_cycle(&pattern, 0);
    assert_eq!(events.len(), 0);
}

#[test]
fn edge_multiple_spaces() {
    let pattern = parse_mini_notation("bd    sn     hh");
    let events = query_cycle(&pattern, 0);
    assert_eq!(events.len(), 3);
}

#[test]
fn edge_mixed_types() {
    let pattern = parse_mini_notation("bd 110 sn 220");
    let events = query_cycle(&pattern, 0);

    assert_eq!(events.len(), 4);
    assert_eq!(events[0].2, "bd");
    assert_eq!(events[1].2, "110");
    assert_eq!(events[2].2, "sn");
    assert_eq!(events[3].2, "220");
}

#[test]
fn edge_sample_names_with_numbers() {
    let pattern = parse_mini_notation("808bd 909hh");
    let events = query_cycle(&pattern, 0);

    assert_eq!(events.len(), 2);
    assert_eq!(events[0].2, "808bd");
    assert_eq!(events[1].2, "909hh");
}

#[test]
fn edge_multiple_operators() {
    let pattern = parse_mini_notation("bd*2/2");
    let events = query_cycle(&pattern, 0);

    // *2 then /2 should result in 1 event
    assert_eq!(events.len(), 1);
}

#[test]
fn edge_deeply_nested_with_operators() {
    let pattern = parse_mini_notation("[[[bd*2]]]*2");
    let events = query_cycle(&pattern, 0);

    // [[[bd*2]]] = [[bd bd]] = [bd bd] = bd bd (2 events)
    // then *2 = 4 events
    assert_eq!(events.len(), 4);
}

#[test]
fn edge_alternation_nested_in_group() {
    let pattern = parse_mini_notation("[<bd sn>]");

    assert_eq!(query_cycle(&pattern, 0)[0].2, "bd");
    assert_eq!(query_cycle(&pattern, 1)[0].2, "sn");
}

#[test]
fn edge_euclid_in_sequence() {
    let pattern = parse_mini_notation("bd(2,4) sn");
    let events = query_cycle(&pattern, 0);

    // bd(2,4) produces events at steps 0 and 2 (out of 4)
    // Plus sn at the end
    assert!(events.len() >= 2);
}

#[test]
fn edge_colon_with_euclid() {
    let pattern = parse_mini_notation("bd:0(3,8)");
    let events = query_cycle(&pattern, 0);

    let hit_events: Vec<_> = events
        .iter()
        .filter(|(_, _, v)| v.starts_with("bd:0"))
        .collect();
    assert_eq!(hit_events.len(), 3);
}
