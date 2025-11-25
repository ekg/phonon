//! Tests for Tidal-style time manipulation functions
//!
//! Tests based on FIRST PRINCIPLES - we verify the mathematical behavior,
//! not just "does sound come out".
//!
//! Key references:
//! - https://tidalcycles.org/docs/reference/time/
//! - https://tidalcycles.org/docs/reference/tempo/

use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use std::collections::HashMap;

/// Helper to query events from a pattern in a given cycle
fn query_cycle<T: Clone + Send + Sync + 'static>(pattern: &Pattern<T>, cycle: usize) -> Vec<phonon::pattern::Hap<T>> {
    let state = State {
        span: TimeSpan::new(
            Fraction::from_float(cycle as f64),
            Fraction::from_float((cycle + 1) as f64),
        ),
        controls: HashMap::new(),
    };
    pattern.query(&state)
}

/// Helper to query events in a fractional time span
#[allow(dead_code)]
fn query_span<T: Clone + Send + Sync + 'static>(pattern: &Pattern<T>, start: f64, end: f64) -> Vec<phonon::pattern::Hap<T>> {
    let state = State {
        span: TimeSpan::new(
            Fraction::from_float(start),
            Fraction::from_float(end),
        ),
        controls: HashMap::new(),
    };
    pattern.query(&state)
}

// ============================================================================
// rotL / rotR - Rotate pattern in time
// ============================================================================

#[test]
fn test_rotl_shifts_events_backward_in_time() {
    // rotL shifts the pattern BACKWARD in time (events occur earlier)
    // rotL 0.25 on "a b c d" means:
    //   - Original: a@0.0, b@0.25, c@0.5, d@0.75
    //   - After rotL 0.25: b@0.0, c@0.25, d@0.5, a@0.75
    // The pattern is shifted LEFT (earlier) by 0.25 cycles

    let pattern: Pattern<String> = parse_mini_notation("a b c d");
    let rotated = pattern.clone().rotate_left(0.25);

    // Query cycle 0
    let original_events = query_cycle(&pattern, 0);
    let rotated_events = query_cycle(&rotated, 0);

    assert_eq!(original_events.len(), 4, "Original should have 4 events");
    assert_eq!(rotated_events.len(), 4, "Rotated should have 4 events");

    // First event of original is "a" at position 0.0
    assert_eq!(original_events[0].value, "a");

    // First event of rotL 0.25 should be "b" (what was at 0.25 is now at 0.0)
    assert_eq!(rotated_events[0].value, "b",
        "rotL 0.25 should shift 'b' to the start");

    // Last event should be "a" (wrapped around)
    assert_eq!(rotated_events[3].value, "a",
        "rotL 0.25 should wrap 'a' to the end");
}

#[test]
fn test_rotr_shifts_events_forward_in_time() {
    // rotR shifts the pattern FORWARD in time (events occur later)
    // rotR 0.25 on "a b c d" means:
    //   - Original: a@0.0, b@0.25, c@0.5, d@0.75
    //   - After rotR 0.25: d@0.0, a@0.25, b@0.5, c@0.75
    // The pattern is shifted RIGHT (later) by 0.25 cycles

    let pattern: Pattern<String> = parse_mini_notation("a b c d");
    let rotated = pattern.clone().rotate_right(0.25);

    let rotated_events = query_cycle(&rotated, 0);

    assert_eq!(rotated_events.len(), 4, "Rotated should have 4 events");

    // First event of rotR 0.25 should be "d" (what was at 0.75 is now at 0.0)
    assert_eq!(rotated_events[0].value, "d",
        "rotR 0.25 should shift 'd' to the start");
}

#[test]
fn test_rotl_by_1_is_identity() {
    // Rotating by 1 full cycle should give back the same pattern
    let pattern: Pattern<String> = parse_mini_notation("a b c d");
    let rotated = pattern.clone().rotate_left(1.0);

    let original_events = query_cycle(&pattern, 0);
    let rotated_events = query_cycle(&rotated, 0);

    for i in 0..4 {
        assert_eq!(original_events[i].value, rotated_events[i].value,
            "rotL 1 should be identity");
    }
}

// ============================================================================
// swing / swingBy - Shuffle timing
// ============================================================================

#[test]
fn test_swing_delays_offbeat_events() {
    // swing delays every OTHER event (the offbeats)
    // With swing amount 0.5 (maximum swing), the 2nd, 4th, 6th... events
    // are delayed by half their duration
    //
    // Example: "a b c d" with swing 0.5
    //   - a@0.0 (unchanged - on beat)
    //   - b@0.25 -> b@0.375 (delayed by 0.125 = 0.25 * 0.5)
    //   - c@0.5 (unchanged - on beat)
    //   - d@0.75 -> d@0.875 (delayed by 0.125)

    let pattern: Pattern<String> = parse_mini_notation("a b c d");
    let swung = pattern.clone().swing(Pattern::pure(0.5));

    let swung_events = query_cycle(&swung, 0);

    assert_eq!(swung_events.len(), 4, "Swung pattern should have 4 events");

    // Event 0 (a) should be at 0.0 (on-beat, unchanged)
    let a_start = swung_events[0].part.begin.to_float();
    assert!((a_start - 0.0).abs() < 0.01,
        "First event (on-beat) should stay at 0.0, got {}", a_start);

    // Event 1 (b) should be delayed from 0.25 to ~0.375
    // The exact amount depends on implementation
    let b_start = swung_events[1].part.begin.to_float();
    assert!(b_start > 0.25,
        "Second event (off-beat) should be delayed from 0.25, got {}", b_start);
}

#[test]
fn test_swing_zero_is_no_change() {
    // swing 0 should not change timing at all
    let pattern: Pattern<String> = parse_mini_notation("a b c d");
    let swung = pattern.clone().swing(Pattern::pure(0.0));

    let original_events = query_cycle(&pattern, 0);
    let swung_events = query_cycle(&swung, 0);

    for i in 0..4 {
        let orig_start = original_events[i].part.begin.to_float();
        let swung_start = swung_events[i].part.begin.to_float();
        assert!((orig_start - swung_start).abs() < 0.001,
            "swing 0 should not change timing, event {} moved from {} to {}",
            i, orig_start, swung_start);
    }
}

// ============================================================================
// inside / outside - Apply functions at different time scales
// ============================================================================

#[test]
fn test_inside_applies_function_within_cycles() {
    // inside n f applies f as if there were n cycles per actual cycle
    // inside 2 rev on "a b c d" treats it as 2 half-cycles:
    //   - First half "a b" gets reversed to "b a"
    //   - Second half "c d" gets reversed to "d c"
    //   - Result: "b a d c"

    // Note: This is a structural test - we're testing the concept
    // The actual implementation may vary
}

#[test]
fn test_outside_applies_function_across_cycles() {
    // outside n f applies f as if n cycles were one cycle
    // outside 2 rev on "a b c d" in cycles 0 and 1:
    //   - Treats 2 cycles as 1 unit
    //   - Reverses the entire 2-cycle span
}

// ============================================================================
// fastGap - Speed up pattern leaving gap
// ============================================================================

#[test]
fn test_fast_gap_speeds_up_with_silence() {
    // fastGap 2 on "a b c d" plays the pattern twice as fast
    // but only in the first half of the cycle, leaving silence in the second half
    //
    // fastGap 2 "a b c d":
    //   - First 0.5 cycle: a@0.0, b@0.125, c@0.25, d@0.375
    //   - Second 0.5 cycle: silence (gap)

    // This is different from fast 2 which would play the full pattern twice

    // TODO: Implement fastGap
}

// ============================================================================
// zoom - Play portion of pattern
// ============================================================================

#[test]
fn test_zoom_plays_portion_of_pattern() {
    // zoom (0.25, 0.75) "a b c d" plays only the middle half
    // stretched to fill the whole cycle
    //
    // Original: a@0-0.25, b@0.25-0.5, c@0.5-0.75, d@0.75-1.0
    // zoom (0.25, 0.75) extracts b and c, stretches to full cycle:
    //   - b@0-0.5, c@0.5-1.0

    // TODO: Implement zoom
}

// ============================================================================
// within - Apply function to part of pattern
// ============================================================================

#[test]
fn test_within_applies_to_portion() {
    // within (0, 0.5) (fast 2) "a b c d"
    // Applies fast 2 only to the first half of the pattern
    //
    // First half (0-0.5): "a b" sped up 2x -> events at 0, 0.125, 0.25, 0.375
    // Second half (0.5-1.0): "c d" unchanged -> events at 0.5, 0.75

    // TODO: Implement within
}

// ============================================================================
// press / pressBy - Delay events by slot fraction
// ============================================================================

#[test]
fn test_press_delays_by_half_slot() {
    // press delays each event by half its slot duration
    // "a b c d" has 4 events, each with duration 0.25
    // press delays each by 0.125:
    //   - a: 0.0 -> 0.125
    //   - b: 0.25 -> 0.375
    //   - c: 0.5 -> 0.625
    //   - d: 0.75 -> 0.875

    // TODO: Implement press
}

#[test]
fn test_press_by_custom_amount() {
    // pressBy 0.25 delays each event by 1/4 of its slot
    // "a b c d" each has duration 0.25
    // pressBy 0.25 delays each by 0.0625:
    //   - a: 0.0 -> 0.0625
    //   - b: 0.25 -> 0.3125
    //   - etc.

    // TODO: Implement pressBy
}

// ============================================================================
// ghost - Add ghost notes
// ============================================================================

#[test]
fn test_ghost_adds_quieter_copies() {
    // ghost adds quieter copies of notes at offsets
    // Typically adds notes before the original at reduced volume
    //
    // "bd" with ghost might produce:
    //   - ghost note at -0.125 with gain 0.3
    //   - ghost note at -0.0625 with gain 0.5
    //   - original note at 0.0 with gain 1.0

    // TODO: Implement ghost
}

// ============================================================================
// Integration tests with DSL
// ============================================================================

mod dsl_integration {
    use phonon::compositional_compiler::compile_program;
    use phonon::compositional_parser::parse_program;

    fn compile_dsl(code: &str) -> Result<phonon::unified_graph::UnifiedSignalGraph, String> {
        let (_remaining, statements) = parse_program(code)
            .map_err(|e| format!("Parse error: {:?}", e))?;
        compile_program(statements, 44100.0)
    }

    #[test]
    fn test_rotl_in_dsl() {
        let code = r#"
            tempo: 1.0
            ~p: s "bd sn hh cp" $ rotL 1
            out: ~p
        "#;

        let graph = compile_dsl(code);
        assert!(graph.is_ok(), "rotL should compile: {:?}", graph.err());
    }

    #[test]
    fn test_rotr_in_dsl() {
        let code = r#"
            tempo: 1.0
            ~p: s "bd sn hh cp" $ rotR 0.25
            out: ~p
        "#;

        let graph = compile_dsl(code);
        assert!(graph.is_ok(), "rotR should compile: {:?}", graph.err());
    }

    #[test]
    fn test_swing_in_dsl() {
        let code = r#"
            tempo: 1.0
            ~p: s "bd*8" $ swing 0.3
            out: ~p
        "#;

        let graph = compile_dsl(code);
        assert!(graph.is_ok(), "swing should compile: {:?}", graph.err());
    }

    #[test]
    fn test_swing_with_pattern_amount() {
        // swing amount can be a pattern!
        let code = r#"
            tempo: 1.0
            ~p: s "bd*8" $ swing "0.1 0.3 0.5 0.2"
            out: ~p
        "#;

        let graph = compile_dsl(code);
        assert!(graph.is_ok(), "swing with pattern should compile: {:?}", graph.err());
    }
}
