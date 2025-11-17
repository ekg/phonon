// Tests for Tidal Cycles parity functions
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{State, TimeSpan, Fraction};
use std::collections::HashMap;

#[test]
fn test_rot_pattern_query() {
    // Test rot 1: "bd sn hh cp" should rotate to "sn hh cp bd"
    let pattern = parse_mini_notation("bd sn hh cp");
    let rot_pattern = pattern.rot(phonon::pattern::Pattern::pure("1".to_string()));

    let state = State {
        span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(1.0)),
        controls: HashMap::new(),
    };

    let events = rot_pattern.query(&state);

    // Should have 4 events
    assert_eq!(events.len(), 4, "rot should preserve event count");

    // Verify rotation: bd sn hh cp -> sn hh cp bd
    assert_eq!(events[0].value, "sn", "First event should be sn");
    assert_eq!(events[1].value, "hh", "Second event should be hh");
    assert_eq!(events[2].value, "cp", "Third event should be cp");
    assert_eq!(events[3].value, "bd", "Fourth event should be bd");

    println!("✅ rot 1 pattern query verified");
}

#[test]
fn test_rot_negative() {
    // Test rot -1: "bd sn hh cp" should rotate to "cp bd sn hh"
    let pattern = parse_mini_notation("bd sn hh cp");
    let rot_pattern = pattern.rot(phonon::pattern::Pattern::pure("-1".to_string()));

    let state = State {
        span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(1.0)),
        controls: HashMap::new(),
    };

    let events = rot_pattern.query(&state);

    assert_eq!(events.len(), 4, "rot -1 should preserve event count");
    assert_eq!(events[0].value, "cp", "First event should be cp");
    assert_eq!(events[1].value, "bd", "Second event should be bd");
    assert_eq!(events[2].value, "sn", "Third event should be sn");
    assert_eq!(events[3].value, "hh", "Fourth event should be hh");

    println!("✅ rot -1 pattern query verified");
}

#[test]
fn test_trunc_pattern_query() {
    // Test trunc 0.5: should play only first half of cycle
    let pattern = parse_mini_notation("bd sn hh cp");
    let trunc_pattern = pattern.trunc(phonon::pattern::Pattern::pure(0.5));

    let state = State {
        span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(1.0)),
        controls: HashMap::new(),
    };

    let events = trunc_pattern.query(&state);

    // Should have 2 events (first half only)
    assert_eq!(events.len(), 2, "trunc 0.5 should give 2 of 4 events");
    assert_eq!(events[0].value, "bd", "First event should be bd");
    assert_eq!(events[1].value, "sn", "Second event should be sn");

    println!("✅ trunc 0.5 pattern query verified");
}

#[test]
fn test_trunc_quarter() {
    // Test trunc 0.25: should play only first quarter
    let pattern = parse_mini_notation("bd sn hh cp");
    let trunc_pattern = pattern.trunc(phonon::pattern::Pattern::pure(0.25));

    let state = State {
        span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(1.0)),
        controls: HashMap::new(),
    };

    let events = trunc_pattern.query(&state);

    // Should have 1 event (first quarter only)
    assert_eq!(events.len(), 1, "trunc 0.25 should give 1 of 4 events");
    assert_eq!(events[0].value, "bd", "First event should be bd");

    println!("✅ trunc 0.25 pattern query verified");
}

#[test]
fn test_sew_pattern_query() {
    // Test sew with "t f" boolean pattern
    let bool_pattern = parse_mini_notation("t f");
    let pat_true = parse_mini_notation("bd*2");
    let pat_false = parse_mini_notation("sn*2");

    let sew_pattern = phonon::pattern::Pattern::sew(bool_pattern, pat_true, pat_false);

    let state = State {
        span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(1.0)),
        controls: HashMap::new(),
    };

    let events = sew_pattern.query(&state);

    // Boolean pattern has 2 events (t, f), each queries its source pattern
    // for that time span, so we get 2 total events (1 bd when t, 1 sn when f)
    assert_eq!(events.len(), 2, "sew should produce 2 events");

    // First half should be bd (when t), second half should be sn (when f)
    assert_eq!(events[0].value, "bd", "First should be bd");
    assert_eq!(events[1].value, "sn", "Second should be sn");

    println!("✅ sew pattern query verified");
}
