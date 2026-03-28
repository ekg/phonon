//! Tests designed to expose implementation gaps in Phonon
//! These tests should fail until the corresponding features are properly implemented

use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use std::collections::HashMap;

#[test]
fn test_euclidean_rotation() {
    // Euclidean patterns with rotation parameter
    let pattern = parse_mini_notation("bd(3,8,2)"); // 3 hits, 8 steps, rotated by 2

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let events = pattern.query(&state);

    // Bjorklund E(3,8) = slots 0, 3, 6, rotated left by 2 = slots 1, 4, 6
    assert_eq!(events.len(), 3, "Should have 3 events");
    assert_eq!(events[0].part.begin, Fraction::new(1, 8)); // Step 1
    assert_eq!(events[1].part.begin, Fraction::new(1, 2)); // Step 4 (4/8 = 1/2)
    assert_eq!(events[2].part.begin, Fraction::new(3, 4)); // Step 6 (6/8 = 3/4)
}

#[test]
fn test_nested_groups() {
    // Nested grouping should work properly
    let pattern = parse_mini_notation("[[bd sn] cp] hh");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let events = pattern.query(&state);

    // [[bd sn] cp] should take 1/2, hh should take 1/2
    // Within first half: [bd sn] takes 1/4, cp takes 1/4
    assert_eq!(events.len(), 4);
    assert_eq!(events[0].value, "bd"); // at 0
    assert_eq!(events[0].part.end, Fraction::new(1, 8)); // bd ends at 1/8
    assert_eq!(events[1].value, "sn");
    assert_eq!(events[1].part.begin, Fraction::new(1, 8));
    assert_eq!(events[2].value, "cp");
    assert_eq!(events[2].part.begin, Fraction::new(1, 4));
    assert_eq!(events[3].value, "hh");
    assert_eq!(events[3].part.begin, Fraction::new(1, 2));
}

#[test]
fn test_alternation_pattern() {
    // <a b c> should alternate between a, b, c each cycle
    let pattern = parse_mini_notation("<bd sn cp>");

    // First cycle
    let state1 = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    let events1 = pattern.query(&state1);
    assert_eq!(events1.len(), 1);
    assert_eq!(events1[0].value, "bd");

    // Second cycle
    let state2 = State {
        span: TimeSpan::new(Fraction::new(1, 1), Fraction::new(2, 1)),
        controls: HashMap::new(),
    };
    let events2 = pattern.query(&state2);
    assert_eq!(events2.len(), 1);
    assert_eq!(events2[0].value, "sn");

    // Third cycle
    let state3 = State {
        span: TimeSpan::new(Fraction::new(2, 1), Fraction::new(3, 1)),
        controls: HashMap::new(),
    };
    let events3 = pattern.query(&state3);
    assert_eq!(events3.len(), 1);
    assert_eq!(events3[0].value, "cp");
}

#[test]
fn test_probability_operator() {
    // bd? should play bd with 50% probability
    let pattern = parse_mini_notation("bd? sn hh");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(100, 1)), // 100 cycles
        controls: HashMap::new(),
    };

    let events = pattern.query(&state);

    // Count events
    let bd_count = events.iter().filter(|e| e.value == "bd").count();
    let sn_count = events.iter().filter(|e| e.value == "sn").count();
    let hh_count = events.iter().filter(|e| e.value == "hh").count();

    // All should appear 100 times in current implementation
    assert_eq!(sn_count, 100, "sn should appear in every cycle");
    assert_eq!(hh_count, 100, "hh should appear in every cycle");

    // bd? has 50% probability - should be roughly 50 out of 100
    assert!(
        bd_count > 20 && bd_count < 80,
        "bd? should appear ~50% of the time, got {}/100",
        bd_count
    );
}
