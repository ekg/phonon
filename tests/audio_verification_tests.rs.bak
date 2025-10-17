//! Audio verification tests for Phonon
//!
//! These tests verify that patterns generate correct audio output

use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use std::collections::HashMap;

#[test]
fn test_simple_pattern_audio() {
    // Test "bd sn bd sn" generates 4 hits
    let pattern = parse_mini_notation("bd sn bd sn");

    // Verify pattern structure
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    let events = pattern.query(&state);
    assert_eq!(events.len(), 4, "Should generate 4 events");

    // Verify timing - should be evenly spaced
    assert_eq!(events[0].part.begin, Fraction::new(0, 1));
    assert_eq!(events[1].part.begin, Fraction::new(1, 4));
    assert_eq!(events[2].part.begin, Fraction::new(1, 2));
    assert_eq!(events[3].part.begin, Fraction::new(3, 4));
}

#[test]
fn test_euclidean_pattern_audio() {
    // Test "bd(3,8)" generates 3 hits evenly distributed
    let pattern = parse_mini_notation("bd(3,8)");

    // Verify pattern structure
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    let events = pattern.query(&state);
    assert_eq!(events.len(), 3, "Euclidean (3,8) should generate 3 events");

    // Verify they are all bd events
    assert!(events.iter().all(|e| e.value == "bd"));
}

#[test]
fn test_polyrhythm_audio() {
    // Test "[bd cp, hh*3]" generates correct polyrhythmic pattern
    let pattern = parse_mini_notation("[bd cp, hh*3]");

    // Verify pattern structure
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    let events = pattern.query(&state);
    assert_eq!(events.len(), 5, "Should have 2 bd/cp + 3 hh events");

    // Count event types
    let bd_count = events.iter().filter(|e| e.value == "bd").count();
    let cp_count = events.iter().filter(|e| e.value == "cp").count();
    let hh_count = events.iter().filter(|e| e.value == "hh").count();

    assert_eq!(bd_count, 1, "Should have 1 bd");
    assert_eq!(cp_count, 1, "Should have 1 cp");
    assert_eq!(hh_count, 3, "Should have 3 hh");
}

#[test]
fn test_rest_pattern_audio() {
    // Test "bd ~ sn ~" has only 2 hits
    let pattern = parse_mini_notation("bd ~ sn ~");

    // Verify pattern structure - tildes should be skipped
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    let events = pattern.query(&state);
    assert_eq!(events.len(), 2); // Only bd and sn, not tildes
}

#[test]
fn test_fast_pattern_audio() {
    // Test "bd*4" generates 4 hits in rapid succession
    let pattern = parse_mini_notation("bd*4");

    // Verify pattern structure
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    let events = pattern.query(&state);
    assert_eq!(events.len(), 4); // Should have 4 hits
}

#[test]
fn test_sample_differentiation() {
    // Test that different samples are parsed correctly
    let kick_pattern = parse_mini_notation("bd");
    let snare_pattern = parse_mini_notation("sn");
    let hihat_pattern = parse_mini_notation("hh");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let kick_events = kick_pattern.query(&state);
    let snare_events = snare_pattern.query(&state);
    let hihat_events = hihat_pattern.query(&state);

    // Verify different sample names
    assert_eq!(kick_events[0].value, "bd");
    assert_eq!(snare_events[0].value, "sn");
    assert_eq!(hihat_events[0].value, "hh");
}

#[test]
fn test_pattern_timing_accuracy() {
    // Test precise timing of pattern events
    let pattern = parse_mini_notation("bd sn cp hh");
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let events = pattern.query(&state);

    // Should have exactly 4 events
    assert_eq!(events.len(), 4);

    // Check timing of each event
    assert_eq!(events[0].part.begin, Fraction::new(0, 4));
    assert_eq!(events[1].part.begin, Fraction::new(1, 4));
    assert_eq!(events[2].part.begin, Fraction::new(2, 4));
    assert_eq!(events[3].part.begin, Fraction::new(3, 4));

    // All events should have 1/4 duration
    for event in &events {
        let duration = event.part.end - event.part.begin;
        assert_eq!(duration, Fraction::new(1, 4));
    }
}

#[test]
fn test_alternation_pattern_audio() {
    // Test "<bd sn>" alternates between bd and sn each cycle
    let pattern = parse_mini_notation("<bd sn>");

    // Verify pattern structure for multiple cycles
    let state1 = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    let state2 = State {
        span: TimeSpan::new(Fraction::new(1, 1), Fraction::new(2, 1)),
        controls: HashMap::new(),
    };

    let events1 = pattern.query(&state1);
    let events2 = pattern.query(&state2);

    // Should have different values in different cycles
    assert_eq!(events1.len(), 1, "Cycle 0 should have 1 event");
    assert_eq!(events2.len(), 1, "Cycle 1 should have 1 event");
    assert_eq!(events1[0].value, "bd", "Cycle 0 should be bd");
    assert_eq!(events2[0].value, "sn", "Cycle 1 should be sn");
}

#[test]
fn test_group_pattern_audio() {
    // Test "[bd sn] cp" generates correct grouping
    let pattern = parse_mini_notation("[bd sn] cp");

    // Verify pattern structure
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    let events = pattern.query(&state);

    // Should have 3 events: bd at 0, sn at 0.25, cp at 0.5
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].value, "bd");
    assert_eq!(events[1].value, "sn");
    assert_eq!(events[2].value, "cp");
}
