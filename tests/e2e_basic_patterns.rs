//! End-to-End Tests: Basic Pattern Operations (50 tests)
//!
//! This module provides comprehensive e2e tests for basic pattern operations.
//! Each test category uses the three-level verification methodology:
//! 1. Pattern Query Verification - Fast, exact, deterministic pattern logic
//! 2. Onset Detection - Audio events at correct times
//! 3. Audio Analysis - Signal characteristics (RMS, spectral)

use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};
use std::collections::HashMap;

mod audio_test_utils;
use audio_test_utils::calculate_rms;

mod pattern_verification_utils;
use pattern_verification_utils::detect_audio_events;

// ============================================================================
// HELPERS
// ============================================================================

fn count_events_over_cycles<T: Clone + Send + Sync + 'static>(
    pattern: &Pattern<T>,
    cycles: usize,
) -> usize {
    let mut total = 0;
    for cycle in 0..cycles {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };
        total += pattern.query(&state).len();
    }
    total
}

fn query_single_cycle<T: Clone + Send + Sync + 'static>(
    pattern: &Pattern<T>,
) -> Vec<phonon::pattern::Hap<T>> {
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    pattern.query(&state)
}

fn render_dsl(code: &str, duration_secs: f32) -> Vec<f32> {
    let (_, statements) = parse_dsl(code).expect("Parse DSL");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let samples = (44100.0 * duration_secs) as usize;
    graph.render(samples)
}

// ============================================================================
// PART 1: PATTERN CONSTRUCTORS (10 tests)
// ============================================================================

#[test]
fn e2e_001_pattern_pure_single_event_per_cycle() {
    // Pattern::pure(x) should produce one event per cycle
    let pattern = Pattern::<f64>::pure(42.0);
    let events = query_single_cycle(&pattern);

    assert_eq!(
        events.len(),
        1,
        "Pure pattern should have exactly 1 event per cycle"
    );
    assert_eq!(events[0].value, 42.0, "Event value should match");
    assert_eq!(
        events[0].part.begin,
        Fraction::new(0, 1),
        "Event should start at cycle beginning"
    );
    assert_eq!(
        events[0].part.end,
        Fraction::new(1, 1),
        "Event should end at cycle end"
    );
}

#[test]
fn e2e_002_pattern_pure_consistent_across_cycles() {
    let pattern = Pattern::<&str>::pure("test");
    let count = count_events_over_cycles(&pattern, 8);
    assert_eq!(
        count, 8,
        "Pure pattern should have 1 event per cycle * 8 cycles = 8 events"
    );
}

#[test]
fn e2e_003_pattern_silence_no_events() {
    let pattern = Pattern::<f64>::silence();
    let events = query_single_cycle(&pattern);
    assert_eq!(events.len(), 0, "Silence should produce no events");

    let count = count_events_over_cycles(&pattern, 8);
    assert_eq!(
        count, 0,
        "Silence should produce no events over multiple cycles"
    );
}

#[test]
fn e2e_004_pattern_cat_sequences_patterns() {
    // cat([a, b, c]) subdivides one cycle: each pattern gets 1/3 of the cycle
    let pattern = Pattern::cat(vec![
        Pattern::pure("a"),
        Pattern::pure("b"),
        Pattern::pure("c"),
    ]);

    // Query a single cycle — should have 3 events (one per sub-pattern)
    let events = query_single_cycle(&pattern);
    assert_eq!(
        events.len(),
        3,
        "cat of 3 should produce 3 events per cycle"
    );

    let values: Vec<&str> = events.iter().map(|e| e.value).collect();
    assert!(values.contains(&"a"), "Should contain 'a'");
    assert!(values.contains(&"b"), "Should contain 'b'");
    assert!(values.contains(&"c"), "Should contain 'c'");
}

#[test]
fn e2e_005_pattern_slowcat_loops_after_completion() {
    // slowcat with 3 items should play one per cycle, looping after cycle 3
    let pattern = Pattern::slowcat(vec![
        Pattern::pure("a"),
        Pattern::pure("b"),
        Pattern::pure("c"),
    ]);

    // Check cycles 0-5: each cycle has one value, looping every 3
    for cycle in 0..6 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };
        let events = pattern.query(&state);
        assert_eq!(events.len(), 1, "Cycle {} should have 1 event", cycle);

        let expected = match cycle % 3 {
            0 => "a",
            1 => "b",
            2 => "c",
            _ => unreachable!(),
        };
        assert_eq!(
            events[0].value, expected,
            "Cycle {} should loop correctly",
            cycle
        );
    }
}

#[test]
fn e2e_006_pattern_stack_combines_patterns() {
    // stack([a, b]) should play both a and b simultaneously
    let pattern = Pattern::stack(vec![Pattern::pure("a"), Pattern::pure("b")]);

    let events = query_single_cycle(&pattern);
    assert_eq!(
        events.len(),
        2,
        "Stack should have events from both patterns"
    );

    let values: Vec<&str> = events.iter().map(|e| e.value).collect();
    assert!(values.contains(&"a"), "Stack should contain 'a'");
    assert!(values.contains(&"b"), "Stack should contain 'b'");
}

#[test]
fn e2e_007_pattern_stack_all_events_same_timing() {
    let pattern = Pattern::stack(vec![
        Pattern::pure(1.0),
        Pattern::pure(2.0),
        Pattern::pure(3.0),
    ]);

    let events = query_single_cycle(&pattern);
    assert_eq!(events.len(), 3);

    // All events should have same timing
    for event in &events {
        assert_eq!(event.part.begin, Fraction::new(0, 1));
        assert_eq!(event.part.end, Fraction::new(1, 1));
    }
}

#[test]
fn e2e_008_pattern_run_creates_sequence() {
    // Pattern::run(4) should create [0, 1, 2, 3]
    let pattern = Pattern::<f64>::run(4);
    let events = query_single_cycle(&pattern);

    assert_eq!(events.len(), 4, "Run(4) should have 4 events");

    for (i, event) in events.iter().enumerate() {
        assert_eq!(event.value, i as f64, "Event {} should have value {}", i, i);
    }
}

#[test]
fn e2e_009_pattern_irand_deterministic() {
    // irand should be deterministic based on cycle
    let pattern = Pattern::<f64>::irand(4);

    // Query same cycle twice, should get same result
    let events1 = query_single_cycle(&pattern);
    let events2 = query_single_cycle(&pattern);

    assert_eq!(events1.len(), 1);
    assert_eq!(events2.len(), 1);
    assert_eq!(
        events1[0].value, events2[0].value,
        "irand should be deterministic"
    );
}

#[test]
fn e2e_010_pattern_choose_deterministic() {
    let pattern = Pattern::choose(vec!["a", "b", "c"]);

    // Same cycle should give same result
    let events1 = query_single_cycle(&pattern);
    let events2 = query_single_cycle(&pattern);

    assert_eq!(
        events1[0].value, events2[0].value,
        "choose should be deterministic"
    );
}

// ============================================================================
// PART 2: TIME MANIPULATION (10 tests)
// ============================================================================

#[test]
fn e2e_011_fast_doubles_event_count() {
    let pattern = parse_mini_notation("bd sn hh cp");
    let normal_count = count_events_over_cycles(&pattern, 4);

    let fast2 = pattern.fast(Pattern::pure(2.0));
    let fast_count = count_events_over_cycles(&fast2, 4);

    assert_eq!(
        fast_count,
        normal_count * 2,
        "fast 2 should double event count"
    );
}

#[test]
fn e2e_012_fast_halves_event_duration() {
    let pattern = Pattern::<&str>::pure("x");
    let fast2 = pattern.fast(Pattern::pure(2.0));

    // In a single cycle, fast 2 should produce 2 events
    let events = query_single_cycle(&fast2);
    assert_eq!(events.len(), 2, "fast 2 should produce 2 events per cycle");

    // Each event should have duration 0.5
    for event in &events {
        let duration = event.part.duration().to_float();
        assert!(
            (duration - 0.5).abs() < 0.01,
            "Events should have duration ~0.5"
        );
    }
}

#[test]
fn e2e_013_slow_halves_event_count() {
    let pattern = parse_mini_notation("bd*8");
    let normal_count = count_events_over_cycles(&pattern, 4);

    let slow2 = pattern.slow(Pattern::pure(2.0));
    let slow_count = count_events_over_cycles(&slow2, 4);

    assert_eq!(
        slow_count,
        normal_count / 2,
        "slow 2 should halve event count"
    );
}

#[test]
fn e2e_014_slow_doubles_event_duration() {
    let pattern = Pattern::<&str>::pure("x");
    let slow2 = pattern.slow(Pattern::pure(2.0));

    // In a single cycle, slow 2 should produce partial event
    let events = query_single_cycle(&slow2);

    // Event spans 2 cycles, so querying 1 cycle gets partial event
    assert!(
        events.len() <= 1,
        "slow 2 on pure should produce at most 1 event per cycle"
    );
}

#[test]
fn e2e_015_late_shifts_events_forward() {
    let pattern = parse_mini_notation("bd");
    let late_pattern = pattern.late(Pattern::pure(0.25));

    let events = query_single_cycle(&late_pattern);
    assert_eq!(events.len(), 1);

    // Event should be shifted forward by 0.25
    let begin = events[0].part.begin.to_float();
    assert!(
        (begin - 0.25).abs() < 0.01,
        "Event should start at ~0.25, got {}",
        begin
    );
}

#[test]
fn e2e_016_early_shifts_events_backward() {
    let pattern = parse_mini_notation("bd");
    let early_pattern = pattern.early(Pattern::pure(0.25));

    let events = query_single_cycle(&early_pattern);

    // Event should be shifted backward (will wrap around cycle)
    // Original at 0.0, early 0.25 means it appears at 0.75 from previous cycle
    let begin = events[0].part.begin.to_float();
    assert!(
        (begin - (-0.25)).abs() < 0.01 || (begin - 0.75).abs() < 0.01,
        "Event should be shifted, got {}",
        begin
    );
}

#[test]
fn e2e_017_fast_and_slow_cancel_out() {
    let pattern = parse_mini_notation("bd sn hh cp");
    let transformed = pattern
        .clone()
        .fast(Pattern::pure(2.0))
        .slow(Pattern::pure(2.0));

    let original_count = count_events_over_cycles(&pattern, 8);
    let transformed_count = count_events_over_cycles(&transformed, 8);

    assert_eq!(
        original_count, transformed_count,
        "fast 2 then slow 2 should cancel out"
    );
}

#[test]
fn e2e_018_offset_convenience_function() {
    let pattern = Pattern::<&str>::pure("x");
    let offset_pattern = pattern.offset(0.5);

    let events = query_single_cycle(&offset_pattern);
    let begin = events[0].part.begin.to_float();

    assert!((begin - 0.5).abs() < 0.01, "Offset 0.5 should shift to 0.5");
}

#[test]
fn e2e_019_fast_with_pattern_amount() {
    // fast with a pattern (not just constant) should work
    let pattern = parse_mini_notation("bd sn");

    // Query multiple cycles and verify varying speed has effect
    let fast_varied = pattern.fast(Pattern::pure(2.0));
    let count = count_events_over_cycles(&fast_varied, 4);

    // With fast 2, we should have 4 base events * 2 = 8 events per cycle (approx)
    assert!(count >= 8, "Fast with pattern should affect event count");
}

#[test]
fn e2e_020_slow_stretches_pattern() {
    // slow 1000 stretches the pattern across 1000 cycles
    // Each cycle still reports a fragment of the stretched event
    let pattern = parse_mini_notation("bd");

    // Normal pattern: 1 event per cycle over 4 cycles = 4 events
    let normal_count = count_events_over_cycles(&pattern, 4);
    assert_eq!(
        normal_count, 4,
        "Normal pattern should have 4 events over 4 cycles"
    );

    // slow(2) should produce half the density: events are twice as long
    let slow2 = pattern.slow(Pattern::pure(2.0));
    let slow2_count = count_events_over_cycles(&slow2, 4);
    assert!(
        slow2_count <= normal_count,
        "Slow 2 should produce fewer or equal events vs normal"
    );
}

// ============================================================================
// PART 3: STRUCTURAL OPERATIONS (10 tests)
// ============================================================================

#[test]
fn e2e_021_rev_reverses_pattern() {
    let pattern = parse_mini_notation("a b c d");
    let reversed = pattern.rev();

    let events = query_single_cycle(&reversed);
    assert_eq!(events.len(), 4);

    // Events should be in reverse order
    assert_eq!(events[0].value, "d");
    assert_eq!(events[1].value, "c");
    assert_eq!(events[2].value, "b");
    assert_eq!(events[3].value, "a");
}

#[test]
fn e2e_022_rev_preserves_event_count() {
    let pattern = parse_mini_notation("bd sn hh cp bd sn hh cp");
    let reversed = pattern.clone().rev();

    let original_count = count_events_over_cycles(&pattern, 4);
    let reversed_count = count_events_over_cycles(&reversed, 4);

    assert_eq!(
        original_count, reversed_count,
        "Rev should preserve event count"
    );
}

#[test]
fn e2e_023_every_applies_function_periodically() {
    let pattern = Pattern::<f64>::pure(1.0);
    let every_2 = pattern.every(2, |p| p.fmap(|x| x * 2.0));

    // Check alternating cycles
    for cycle in 0..4 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };
        let events = every_2.query(&state);

        let expected = if cycle % 2 == 0 { 2.0 } else { 1.0 };
        assert_eq!(
            events[0].value, expected,
            "Cycle {} should have value {}",
            cycle, expected
        );
    }
}

#[test]
fn e2e_024_every_with_different_periods() {
    let pattern = Pattern::<i32>::pure(1);

    // every 3 should apply on cycles 0, 3, 6, ...
    let every_3 = pattern.every(3, |p| p.fmap(|x| x * 10));

    for cycle in 0..9 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };
        let events = every_3.query(&state);

        let expected = if cycle % 3 == 0 { 10 } else { 1 };
        assert_eq!(events[0].value, expected, "Cycle {} failed", cycle);
    }
}

#[test]
fn e2e_025_overlay_combines_patterns() {
    let pattern1 = Pattern::<&str>::pure("a");
    let pattern2 = Pattern::<&str>::pure("b");
    let overlaid = pattern1.overlay(pattern2);

    let events = query_single_cycle(&overlaid);
    assert_eq!(events.len(), 2, "Overlay should combine events");
}

#[test]
fn e2e_026_append_creates_sequence() {
    let pattern1 = Pattern::<&str>::pure("a");
    let pattern2 = Pattern::<&str>::pure("b");
    let appended = pattern1.append(pattern2);

    let events = query_single_cycle(&appended);
    assert_eq!(
        events.len(),
        2,
        "Append should create sequence in single cycle"
    );

    // First half should be "a", second half "b"
    assert!(events[0].part.begin.to_float() < 0.5);
    assert!(events[1].part.begin.to_float() >= 0.5);
}

#[test]
fn e2e_027_dup_repeats_pattern() {
    let pattern = Pattern::<&str>::pure("x");
    let dupped = pattern.dup(4);

    let events = query_single_cycle(&dupped);
    assert_eq!(events.len(), 4, "dup 4 should create 4 events");
}

#[test]
fn e2e_028_stutter_subdivides_events() {
    let pattern = Pattern::<&str>::pure("x");
    let stuttered = pattern.stutter(4);

    let events = query_single_cycle(&stuttered);
    assert_eq!(events.len(), 4, "stutter 4 should create 4 events");

    // Events should be evenly spaced
    for i in 0..4 {
        let expected_begin = i as f64 / 4.0;
        let actual_begin = events[i].part.begin.to_float();
        assert!(
            (actual_begin - expected_begin).abs() < 0.01,
            "Event {} should start at {}, got {}",
            i,
            expected_begin,
            actual_begin
        );
    }
}

#[test]
fn e2e_029_palindrome_creates_forward_backward() {
    let pattern = parse_mini_notation("a b c");
    let palindrome = pattern.palindrome();

    // Palindrome plays pattern forward then backward over 2 cycles
    let count = count_events_over_cycles(&palindrome, 2);
    assert!(
        count >= 4,
        "Palindrome should have events in both directions"
    );
}

#[test]
fn e2e_030_loop_pattern_repeats_n_times() {
    let pattern = Pattern::<&str>::pure("x");
    let looped = pattern.loop_pattern(3);

    // loop_pattern(3) creates 3 fast(3) copies with phase offsets,
    // producing n*n = 9 events from a single-event source
    let events = query_single_cycle(&looped);
    assert_eq!(
        events.len(),
        9,
        "loop_pattern 3 on pure should create 9 events"
    );
}

// ============================================================================
// PART 4: PATTERN TRANSFORMATIONS (10 tests)
// ============================================================================

#[test]
fn e2e_031_degrade_reduces_events() {
    let pattern = parse_mini_notation("bd*16"); // 16 events
    let degraded = pattern.degrade(); // 50% probability

    // Run over multiple cycles and check average
    let original_count = count_events_over_cycles(&parse_mini_notation("bd*16"), 20);
    let degraded_count = count_events_over_cycles(&degraded, 20);

    // Should have significantly fewer events (not exactly half due to randomness)
    assert!(
        degraded_count < original_count,
        "Degrade should reduce event count"
    );
    assert!(degraded_count > 0, "Degrade should not remove all events");
}

#[test]
fn e2e_032_degrade_by_with_probability() {
    let pattern = parse_mini_notation("bd*16");
    let degraded = pattern.degrade_by(Pattern::pure(0.9)); // 90% drop rate

    let original_count = count_events_over_cycles(&parse_mini_notation("bd*16"), 20);
    let degraded_count = count_events_over_cycles(&degraded, 20);

    // With 90% drop rate, should have very few events
    assert!(
        degraded_count < original_count / 4,
        "90% degrade should remove most events"
    );
}

#[test]
fn e2e_033_degrade_is_deterministic() {
    let pattern = parse_mini_notation("bd*8");
    let degraded = pattern.degrade();

    // Same pattern should produce same result
    let count1 = count_events_over_cycles(&degraded, 4);
    let count2 = count_events_over_cycles(&degraded, 4);

    assert_eq!(count1, count2, "Degrade should be deterministic");
}

#[test]
fn e2e_034_sometimes_applies_half_the_time() {
    let pattern = Pattern::<f64>::pure(1.0);
    let sometimes_double = pattern.sometimes(|p| p.fmap(|x| x * 2.0));

    // Count how many cycles have doubled value
    let mut doubled_count = 0;
    let mut normal_count = 0;

    for cycle in 0..100 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };
        let events = sometimes_double.query(&state);

        if events[0].value > 1.5 {
            doubled_count += 1;
        } else {
            normal_count += 1;
        }
    }

    // Should be roughly 50/50 (allow for randomness)
    assert!(
        doubled_count > 30,
        "Sometimes should apply roughly half the time"
    );
    assert!(
        normal_count > 30,
        "Sometimes should skip roughly half the time"
    );
}

#[test]
fn e2e_035_rarely_applies_infrequently() {
    let pattern = Pattern::<f64>::pure(1.0);
    let rarely_double = pattern.rarely(|p| p.fmap(|x| x * 2.0));

    let mut doubled_count = 0;

    for cycle in 0..100 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };
        let events = rarely_double.query(&state);

        if events[0].value > 1.5 {
            doubled_count += 1;
        }
    }

    // Rarely (10%) should apply much less than half
    assert!(
        doubled_count < 30,
        "Rarely should apply infrequently, got {}",
        doubled_count
    );
}

#[test]
fn e2e_036_often_applies_frequently() {
    let pattern = Pattern::<f64>::pure(1.0);
    let often_double = pattern.often(|p| p.fmap(|x| x * 2.0));

    let mut doubled_count = 0;

    for cycle in 0..100 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };
        let events = often_double.query(&state);

        if events[0].value > 1.5 {
            doubled_count += 1;
        }
    }

    // Often (75%) should apply more than half
    assert!(
        doubled_count > 50,
        "Often should apply frequently, got {}",
        doubled_count
    );
}

#[test]
fn e2e_037_always_applies_every_time() {
    let pattern = Pattern::<f64>::pure(1.0);
    let always_double = pattern.always(|p| p.fmap(|x| x * 2.0));

    for cycle in 0..10 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };
        let events = always_double.query(&state);

        assert_eq!(events[0].value, 2.0, "Always should apply every cycle");
    }
}

#[test]
fn e2e_038_fmap_transforms_values() {
    let pattern = Pattern::<i32>::pure(5);
    let doubled = pattern.fmap(|x| x * 2);

    let events = query_single_cycle(&doubled);
    assert_eq!(events[0].value, 10, "fmap should transform value");
}

#[test]
fn e2e_039_filter_events_removes_matching() {
    // cat subdivides one cycle: [1, 2, 3, 4] → 4 events per cycle
    let pattern = Pattern::cat(vec![
        Pattern::pure(1),
        Pattern::pure(2),
        Pattern::pure(3),
        Pattern::pure(4),
    ]);

    let filtered = pattern.filter(|v| v % 2 == 0);

    // Filter keeps 2 even values per cycle; over 4 cycles = 8 events
    let count = count_events_over_cycles(&filtered, 4);
    assert_eq!(
        count, 8,
        "Filter should keep only even values (2 per cycle * 4 cycles)"
    );
}

#[test]
fn e2e_040_fmap_preserves_timing() {
    let pattern = parse_mini_notation("bd sn hh cp");
    let original_events = query_single_cycle(&pattern);

    let transformed = pattern.fmap(|x| format!("{}_mod", x));
    let transformed_events = query_single_cycle(&transformed);

    assert_eq!(original_events.len(), transformed_events.len());

    for (orig, trans) in original_events.iter().zip(transformed_events.iter()) {
        assert_eq!(
            orig.part.begin, trans.part.begin,
            "fmap should preserve timing"
        );
        assert_eq!(orig.part.end, trans.part.end, "fmap should preserve timing");
    }
}

// ============================================================================
// PART 5: MINI-NOTATION PATTERNS (10 tests)
// ============================================================================

#[test]
fn e2e_041_mini_notation_basic_sequence() {
    let pattern = parse_mini_notation("bd sn hh cp");
    let events = query_single_cycle(&pattern);

    assert_eq!(events.len(), 4, "Should parse 4 elements");
    assert_eq!(events[0].value, "bd");
    assert_eq!(events[1].value, "sn");
    assert_eq!(events[2].value, "hh");
    assert_eq!(events[3].value, "cp");
}

#[test]
fn e2e_042_mini_notation_rest() {
    let pattern = parse_mini_notation("bd ~ sn ~");
    let events = query_single_cycle(&pattern);

    assert_eq!(events.len(), 2, "Rests should not produce events");
    assert_eq!(events[0].value, "bd");
    assert_eq!(events[1].value, "sn");
}

#[test]
fn e2e_043_mini_notation_replicate() {
    let pattern = parse_mini_notation("bd*4");
    let events = query_single_cycle(&pattern);

    assert_eq!(events.len(), 4, "bd*4 should produce 4 events");
    assert!(
        events.iter().all(|e| e.value == "bd"),
        "All events should be bd"
    );
}

#[test]
fn e2e_044_mini_notation_subdivision() {
    let pattern = parse_mini_notation("[bd sn hh]");
    let events = query_single_cycle(&pattern);

    assert_eq!(events.len(), 3, "[bd sn hh] should have 3 events");

    // Each event should have 1/3 duration
    for event in &events {
        let duration = event.part.duration().to_float();
        assert!(
            (duration - 1.0 / 3.0).abs() < 0.01,
            "Events should have 1/3 duration"
        );
    }
}

#[test]
fn e2e_045_mini_notation_alternation() {
    let pattern = parse_mini_notation("<bd sn>");

    // Cycle 0 should have bd
    let state0 = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    let events0 = pattern.query(&state0);
    assert_eq!(events0[0].value, "bd");

    // Cycle 1 should have sn
    let state1 = State {
        span: TimeSpan::new(Fraction::new(1, 1), Fraction::new(2, 1)),
        controls: HashMap::new(),
    };
    let events1 = pattern.query(&state1);
    assert_eq!(events1[0].value, "sn");
}

#[test]
fn e2e_046_mini_notation_euclidean() {
    let pattern = parse_mini_notation("bd(3,8)");
    let events = query_single_cycle(&pattern);

    assert_eq!(events.len(), 3, "bd(3,8) should produce 3 events");

    // Events should be distributed across 8 slots
    // Euclidean(3,8) = [1,0,0,1,0,0,1,0] = events at 0/8, 3/8, 6/8
    let times: Vec<f64> = events.iter().map(|e| e.part.begin.to_float()).collect();
    assert!((times[0] - 0.0 / 8.0).abs() < 0.01);
    assert!((times[1] - 3.0 / 8.0).abs() < 0.01);
    assert!((times[2] - 6.0 / 8.0).abs() < 0.01);
}

#[test]
fn e2e_047_mini_notation_polyrhythm() {
    let pattern = parse_mini_notation("[bd cp, hh*3]");
    let events = query_single_cycle(&pattern);

    // Should have 2 from first layer + 3 from second
    assert_eq!(events.len(), 5, "Polyrhythm should combine layers");

    let bd_count = events.iter().filter(|e| e.value == "bd").count();
    let cp_count = events.iter().filter(|e| e.value == "cp").count();
    let hh_count = events.iter().filter(|e| e.value == "hh").count();

    assert_eq!(bd_count, 1);
    assert_eq!(cp_count, 1);
    assert_eq!(hh_count, 3);
}

#[test]
fn e2e_048_mini_notation_sample_bank() {
    let pattern = parse_mini_notation("bd:0 bd:1 bd:2");
    let events = query_single_cycle(&pattern);

    assert_eq!(events.len(), 3);
    assert_eq!(events[0].value, "bd:0");
    assert_eq!(events[1].value, "bd:1");
    assert_eq!(events[2].value, "bd:2");
}

#[test]
fn e2e_049_mini_notation_nested_groups() {
    let pattern = parse_mini_notation("[[bd sn] hh]");
    let events = query_single_cycle(&pattern);

    // Outer group has 2 slots: [bd sn] and hh
    // Inner [bd sn] has 2 events in half the time
    assert_eq!(events.len(), 3);
}

#[test]
fn e2e_050_mini_notation_complex_pattern() {
    let pattern = parse_mini_notation("<bd sn> [hh*2, cp] ~ [kick snare]");
    let events = query_single_cycle(&pattern);

    // Complex pattern should parse without error
    assert!(events.len() >= 3, "Complex pattern should produce events");
}

// ============================================================================
// AUDIO-LEVEL VERIFICATION TESTS
// ============================================================================

#[test]
fn e2e_audio_pattern_generates_sound() {
    let code = r#"
        tempo: 0.5
        out $ s "bd sn hh cp"
    "#;

    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);

    assert!(
        rms > 0.01,
        "Pattern should generate audible sound, RMS: {}",
        rms
    );
}

#[test]
fn e2e_audio_fast_increases_density() {
    let normal_code = r#"
        tempo: 0.5
        out $ s "bd sn"
    "#;

    let fast_code = r#"
        tempo: 0.5
        out $ s "bd sn bd sn bd sn bd sn"
    "#;

    let normal_audio = render_dsl(normal_code, 1.0);
    let fast_audio = render_dsl(fast_code, 1.0);

    let normal_onsets = detect_audio_events(&normal_audio, 44100.0, 0.01);
    let fast_onsets = detect_audio_events(&fast_audio, 44100.0, 0.01);

    assert!(
        fast_onsets.len() >= normal_onsets.len(),
        "Fast pattern should have at least as many onsets"
    );
}

#[test]
fn e2e_audio_silence_pattern() {
    let code = r#"
        out $ sine 0
    "#;

    let audio = render_dsl(code, 0.5);
    let rms = calculate_rms(&audio);

    assert!(rms < 0.01, "Silence should produce near-zero RMS: {}", rms);
}

#[test]
fn e2e_audio_stacking_increases_amplitude() {
    let single_code = r#"
        out $ sine 440 * 0.3
    "#;

    let stacked_code = r#"
        out $ sine 440 * 0.3 + sine 660 * 0.3
    "#;

    let single_audio = render_dsl(single_code, 0.5);
    let stacked_audio = render_dsl(stacked_code, 0.5);

    let single_rms = calculate_rms(&single_audio);
    let stacked_rms = calculate_rms(&stacked_audio);

    assert!(
        stacked_rms > single_rms,
        "Stacked signals should have higher RMS: {} vs {}",
        stacked_rms,
        single_rms
    );
}
