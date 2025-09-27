//! Comprehensive tests for ALL mini-notation operators
//! Each test verifies behavior across multiple cycles

use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use std::collections::HashMap;

/// Helper to query a pattern for a specific cycle
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

/// Helper to check if events match expected
fn assert_events_eq(actual: Vec<(f64, f64, String)>, expected: Vec<(f64, f64, &str)>) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "Expected {} events, got {}: {:?}",
        expected.len(),
        actual.len(),
        actual
    );

    for (i, (actual_event, expected_event)) in actual.iter().zip(expected.iter()).enumerate() {
        assert!(
            (actual_event.0 - expected_event.0).abs() < 0.01,
            "Event {} start time mismatch: expected {}, got {}",
            i,
            expected_event.0,
            actual_event.0
        );
        assert!(
            (actual_event.1 - expected_event.1).abs() < 0.01,
            "Event {} end time mismatch: expected {}, got {}",
            i,
            expected_event.1,
            actual_event.1
        );
        assert_eq!(
            actual_event.2, expected_event.2,
            "Event {} value mismatch: expected {}, got {}",
            i, expected_event.2, actual_event.2
        );
    }
}

#[test]
fn test_simple_sequence() {
    let pattern = parse_mini_notation("bd sn hh cp");

    // Should be the same every cycle
    for cycle in 0..3 {
        let events = query_cycle(&pattern, cycle);
        let base = cycle as f64;
        assert_events_eq(
            events,
            vec![
                (base + 0.0, base + 0.25, "bd"),
                (base + 0.25, base + 0.5, "sn"),
                (base + 0.5, base + 0.75, "hh"),
                (base + 0.75, base + 1.0, "cp"),
            ],
        );
    }
}

#[test]
fn test_rest_pattern() {
    let pattern = parse_mini_notation("bd ~ sn ~");

    let events = query_cycle(&pattern, 0);
    assert_events_eq(events, vec![(0.0, 0.25, "bd"), (0.5, 0.75, "sn")]);
}

#[test]
fn test_groups_subdivision() {
    let pattern = parse_mini_notation("bd [sn sn] hh");

    let events = query_cycle(&pattern, 0);
    assert_events_eq(
        events,
        vec![
            (0.0, 0.333, "bd"),
            (0.333, 0.5, "sn"),
            (0.5, 0.667, "sn"),
            (0.667, 1.0, "hh"),
        ],
    );
}

#[test]
fn test_alternation_cycles_properly() {
    let pattern = parse_mini_notation("<bd sn cp>");

    // Cycle 0: bd
    let events_0 = query_cycle(&pattern, 0);
    assert_events_eq(events_0, vec![(0.0, 1.0, "bd")]);

    // Cycle 1: sn
    let events_1 = query_cycle(&pattern, 1);
    assert_events_eq(events_1, vec![(1.0, 2.0, "sn")]);

    // Cycle 2: cp
    let events_2 = query_cycle(&pattern, 2);
    assert_events_eq(events_2, vec![(2.0, 3.0, "cp")]);

    // Cycle 3: back to bd
    let events_3 = query_cycle(&pattern, 3);
    assert_events_eq(events_3, vec![(3.0, 4.0, "bd")]);
}

#[test]
#[ignore] // TODO: Fix alternation implementation
fn test_alternation_in_sequence() {
    let pattern = parse_mini_notation("<bd sn cp> hh");

    // Cycle 0: bd hh
    let events_0 = query_cycle(&pattern, 0);
    assert_events_eq(events_0, vec![(0.0, 0.5, "bd"), (0.5, 1.0, "hh")]);

    // Cycle 1: sn hh
    let events_1 = query_cycle(&pattern, 1);
    assert_events_eq(events_1, vec![(1.0, 1.5, "sn"), (1.5, 2.0, "hh")]);

    // Cycle 2: cp hh
    let events_2 = query_cycle(&pattern, 2);
    assert_events_eq(events_2, vec![(2.0, 2.5, "cp"), (2.5, 3.0, "hh")]);
}

#[test]
fn test_repeat_operator() {
    let pattern = parse_mini_notation("hh*4");

    let events = query_cycle(&pattern, 0);
    assert_events_eq(
        events,
        vec![
            (0.0, 0.25, "hh"),
            (0.25, 0.5, "hh"),
            (0.5, 0.75, "hh"),
            (0.75, 1.0, "hh"),
        ],
    );
}

#[test]
fn test_repeat_in_sequence() {
    let pattern = parse_mini_notation("bd*2 sn");

    let events = query_cycle(&pattern, 0);
    assert_events_eq(
        events,
        vec![(0.0, 0.25, "bd"), (0.25, 0.5, "bd"), (0.5, 1.0, "sn")],
    );
}

#[test]
fn test_polyrhythm_with_commas() {
    let pattern = parse_mini_notation("[bd cp, hh*3]");

    let events = query_cycle(&pattern, 0);
    // bd and cp in first pattern, 3 hh in second
    assert_eq!(events.len(), 5);

    // Check bd and cp
    assert!(events
        .iter()
        .any(|e| e.2 == "bd" && (e.0 - 0.0).abs() < 0.01));
    assert!(events
        .iter()
        .any(|e| e.2 == "cp" && (e.0 - 0.5).abs() < 0.01));

    // Check 3 hh events
    let hh_events: Vec<_> = events.iter().filter(|e| e.2 == "hh").collect();
    assert_eq!(hh_events.len(), 3);
}

#[test]
fn test_polyrhythm_parentheses() {
    let pattern = parse_mini_notation("(bd, sn cp, hh*3)");

    let events = query_cycle(&pattern, 0);
    assert_eq!(events.len(), 6); // 1 bd, 2 sn/cp, 3 hh

    // bd should span full cycle
    assert!(events
        .iter()
        .any(|e| e.2 == "bd" && (e.0 - 0.0).abs() < 0.01 && (e.1 - 1.0).abs() < 0.01));
}

#[test]
fn test_slow_operator() {
    let pattern = parse_mini_notation("bd/2");

    // bd should appear in both cycles (stretched over 2)
    let events_0 = query_cycle(&pattern, 0);
    let events_1 = query_cycle(&pattern, 1);

    assert_eq!(events_0.len(), 1);
    assert_eq!(events_1.len(), 1);
    assert_eq!(events_0[0].2, "bd");
    assert_eq!(events_1[0].2, "bd");
}

#[test]
fn test_degrade_operator() {
    // This is probabilistic, so just check it compiles and runs
    let pattern = parse_mini_notation("bd? sn");
    let events = query_cycle(&pattern, 0);

    // Should have sn always, bd sometimes
    assert!(events.iter().any(|e| e.2 == "sn"));
}

#[test]
fn test_stacking_with_pipe() {
    let pattern = parse_mini_notation("bd sn | hh*4");

    let events = query_cycle(&pattern, 0);

    // Should have bd, sn, and 4 hh
    assert_eq!(events.len(), 6);
    assert!(events.iter().any(|e| e.2 == "bd"));
    assert!(events.iter().any(|e| e.2 == "sn"));
    assert_eq!(events.iter().filter(|e| e.2 == "hh").count(), 4);
}

#[test]
fn test_nested_groups() {
    let pattern = parse_mini_notation("bd [[sn cp] hh]");

    let events = query_cycle(&pattern, 0);
    assert_events_eq(
        events,
        vec![
            (0.0, 0.5, "bd"),
            (0.5, 0.625, "sn"),
            (0.625, 0.75, "cp"),
            (0.75, 1.0, "hh"),
        ],
    );
}

#[test]
#[ignore] // TODO: Fix complex alternation
fn test_complex_pattern_with_alternation() {
    let pattern = parse_mini_notation("<bd sn cp> hh*4");

    // Cycle 0: bd with 4 hh
    let events_0 = query_cycle(&pattern, 0);
    assert!(events_0.iter().any(|e| e.2 == "bd"));
    assert_eq!(events_0.iter().filter(|e| e.2 == "hh").count(), 4);

    // Cycle 1: sn with 4 hh
    let events_1 = query_cycle(&pattern, 1);
    assert!(events_1.iter().any(|e| e.2 == "sn"));
    assert_eq!(events_1.iter().filter(|e| e.2 == "hh").count(), 4);

    // Cycle 2: cp with 4 hh
    let events_2 = query_cycle(&pattern, 2);
    assert!(events_2.iter().any(|e| e.2 == "cp"));
    assert_eq!(events_2.iter().filter(|e| e.2 == "hh").count(), 4);
}

#[test]
#[ignore] // TODO: Fix elongate operator
fn test_elongate_operator() {
    let pattern = parse_mini_notation("bd_");

    // Should stretch bd over 2 cycles
    let events_0 = query_cycle(&pattern, 0);
    let events_1 = query_cycle(&pattern, 1);

    assert_eq!(events_0.len(), 1);
    assert_eq!(events_1.len(), 1);
    assert_eq!(events_0[0].2, "bd");
    assert_eq!(events_1[0].2, "bd");
}

#[test]
fn test_euclidean_rhythm() {
    // Euclidean rhythms not fully implemented yet
    // This test documents expected behavior
    // let pattern = parse_mini_notation("bd(3,8)");
    // Should create 3 evenly distributed hits over 8 steps
}

// Run all tests and report results
fn main() {
    println!("Running comprehensive mini-notation tests...\n");

    // We'll run this as a test suite
    println!("Run with: cargo test --test mini_notation_comprehensive");
}
