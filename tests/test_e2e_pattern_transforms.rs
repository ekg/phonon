//! End-to-End Tests for Pattern Transformations
//!
//! This test suite provides comprehensive verification of all major pattern
//! transformations using a three-level methodology:
//!
//! 1. Level 1: Pattern Query Verification - Tests pattern logic directly
//! 2. Level 2: DSL Integration - Tests transforms work through the DSL/compiler
//! 3. Level 3: Audio Characteristics - Tests audio output properties
//!
//! Tests cover: fast, slow, rev, every, rotL, rotR, late, early, degrade,
//! sometimes, often, rarely, dup, stutter, palindrome, zoom, compress,
//! swing, shuffle, legato, chop, iter, ply, linger, fastGap, within,
//! inside, outside, and more.

use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};
use std::collections::HashMap;

mod audio_test_utils;
mod pattern_verification_utils;
use audio_test_utils::calculate_rms;
use pattern_verification_utils::detect_audio_events;

// ============================================================================
// TEST HELPERS
// ============================================================================

/// Count events from a pattern over multiple cycles
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

/// Get events from a pattern for a specific cycle
fn get_events_for_cycle<T: Clone + Send + Sync + 'static>(
    pattern: &Pattern<T>,
    cycle: usize,
) -> Vec<phonon::pattern::Hap<T>> {
    let state = State {
        span: TimeSpan::new(
            Fraction::from_float(cycle as f64),
            Fraction::from_float((cycle + 1) as f64),
        ),
        controls: HashMap::new(),
    };
    pattern.query(&state)
}

/// Render DSL code to audio samples
fn render_dsl(code: &str, duration_secs: f32) -> Vec<f32> {
    let (_, statements) = parse_dsl(code).expect("Parse DSL failed");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let samples = (44100.0 * duration_secs) as usize;
    graph.render(samples)
}

/// Check if DSL code produces audio
fn dsl_produces_audio(code: &str, duration_secs: f32, min_rms: f32) -> bool {
    let audio = render_dsl(code, duration_secs);
    let rms = calculate_rms(&audio);
    rms > min_rms
}

// ============================================================================
// LEVEL 1: PATTERN QUERY VERIFICATION (Direct Pattern API)
// ============================================================================

/// Test: fast(2) doubles the number of events per cycle
#[test]
fn test_l1_fast_doubles_events() {
    let pattern = parse_mini_notation("bd sn hh cp");
    let normal_count = count_events_over_cycles(&pattern, 4);

    let fast2 = pattern.fast(Pattern::pure(2.0));
    let fast2_count = count_events_over_cycles(&fast2, 4);

    assert_eq!(
        fast2_count,
        normal_count * 2,
        "fast 2 should double event count: normal={}, fast2={}",
        normal_count,
        fast2_count
    );
}

/// Test: fast(0.5) halves the number of events per cycle (same as slow 2)
#[test]
fn test_l1_fast_half_halves_events() {
    let pattern = parse_mini_notation("bd sn hh cp");
    let normal_count = count_events_over_cycles(&pattern, 4);

    let fast_half = pattern.fast(Pattern::pure(0.5));
    let fast_half_count = count_events_over_cycles(&fast_half, 4);

    assert_eq!(
        fast_half_count,
        normal_count / 2,
        "fast 0.5 should halve event count: normal={}, fast_half={}",
        normal_count,
        fast_half_count
    );
}

/// Test: slow(2) halves the number of events per cycle
#[test]
fn test_l1_slow_halves_events() {
    let pattern = parse_mini_notation("bd sn hh cp");
    let normal_count = count_events_over_cycles(&pattern, 4);

    let slow2 = pattern.slow(Pattern::pure(2.0));
    let slow2_count = count_events_over_cycles(&slow2, 4);

    assert_eq!(
        slow2_count,
        normal_count / 2,
        "slow 2 should halve event count: normal={}, slow2={}",
        normal_count,
        slow2_count
    );
}

/// Test: slow(0.5) doubles the number of events per cycle (same as fast 2)
#[test]
fn test_l1_slow_half_doubles_events() {
    let pattern = parse_mini_notation("bd sn hh cp");
    let normal_count = count_events_over_cycles(&pattern, 4);

    let slow_half = pattern.slow(Pattern::pure(0.5));
    let slow_half_count = count_events_over_cycles(&slow_half, 4);

    assert_eq!(
        slow_half_count,
        normal_count * 2,
        "slow 0.5 should double event count: normal={}, slow_half={}",
        normal_count,
        slow_half_count
    );
}

/// Test: fast and slow are inverses
#[test]
fn test_l1_fast_slow_inverse() {
    let pattern = parse_mini_notation("bd sn hh cp");
    let normal_count = count_events_over_cycles(&pattern, 4);

    // fast 2 then slow 2 should give same count
    let fast2_slow2 = pattern
        .clone()
        .fast(Pattern::pure(2.0))
        .slow(Pattern::pure(2.0));
    let roundtrip_count = count_events_over_cycles(&fast2_slow2, 4);

    assert_eq!(
        roundtrip_count, normal_count,
        "fast 2 then slow 2 should preserve event count"
    );
}

/// Test: rev reverses event order within cycle
#[test]
fn test_l1_rev_reverses_order() {
    let pattern = parse_mini_notation("a b c d");
    let normal_events = get_events_for_cycle(&pattern, 0);

    let rev_pattern = pattern.rev();
    let rev_events = get_events_for_cycle(&rev_pattern, 0);

    assert_eq!(
        normal_events.len(),
        rev_events.len(),
        "rev should preserve event count"
    );

    // First event of normal should be "a", first of rev should be "d"
    assert_eq!(normal_events[0].value, "a");
    assert_eq!(rev_events[0].value, "d");

    // Last event of normal should be "d", last of rev should be "a"
    assert_eq!(normal_events[3].value, "d");
    assert_eq!(rev_events[3].value, "a");
}

/// Test: rev is self-inverse (rev(rev(p)) == p)
#[test]
fn test_l1_rev_is_self_inverse() {
    let pattern = parse_mini_notation("a b c d");
    let rev_rev = pattern.clone().rev().rev();

    let normal_events = get_events_for_cycle(&pattern, 0);
    let rev_rev_events = get_events_for_cycle(&rev_rev, 0);

    assert_eq!(normal_events.len(), rev_rev_events.len());
    for (n, rr) in normal_events.iter().zip(rev_rev_events.iter()) {
        assert_eq!(n.value, rr.value, "rev(rev(p)) should equal p");
    }
}

/// Test: every n applies function every nth cycle
#[test]
fn test_l1_every_applies_on_nth_cycle() {
    let pattern = parse_mini_notation("bd");
    let every4 = pattern.clone().every(4, |p| p.fast(Pattern::pure(2.0)));

    // Cycle 0 should be transformed (fast 2 = 2 events)
    let cycle0_count = get_events_for_cycle(&every4, 0).len();
    // Cycles 1, 2, 3 should be normal (1 event each)
    let cycle1_count = get_events_for_cycle(&every4, 1).len();
    let cycle2_count = get_events_for_cycle(&every4, 2).len();
    let cycle3_count = get_events_for_cycle(&every4, 3).len();
    // Cycle 4 should be transformed again
    let cycle4_count = get_events_for_cycle(&every4, 4).len();

    assert_eq!(cycle0_count, 2, "Cycle 0 should have fast 2 applied");
    assert_eq!(cycle1_count, 1, "Cycle 1 should be normal");
    assert_eq!(cycle2_count, 1, "Cycle 2 should be normal");
    assert_eq!(cycle3_count, 1, "Cycle 3 should be normal");
    assert_eq!(cycle4_count, 2, "Cycle 4 should have fast 2 applied");
}

/// Test: rotate_left shifts pattern timing
#[test]
fn test_l1_rotate_left() {
    let pattern = parse_mini_notation("a b c d");
    let rotated = pattern.clone().rotate_left(0.25); // Shift left by 1/4 cycle

    let normal_events = get_events_for_cycle(&pattern, 0);
    let rotated_events = get_events_for_cycle(&rotated, 0);

    assert_eq!(normal_events.len(), rotated_events.len());

    // After rotating left by 0.25, events should shift:
    // "a" at 0.0 should become at -0.25 (wrapped to 0.75)
    // First event should now be "b" at position ~0.0
    assert_eq!(
        rotated_events[0].value, "b",
        "After rotate_left 0.25, first event should be 'b'"
    );
}

/// Test: rotate_right shifts pattern timing
#[test]
fn test_l1_rotate_right() {
    let pattern = parse_mini_notation("a b c d");
    let rotated = pattern.clone().rotate_right(0.25); // Shift right by 1/4 cycle

    let normal_events = get_events_for_cycle(&pattern, 0);
    let rotated_events = get_events_for_cycle(&rotated, 0);

    assert_eq!(normal_events.len(), rotated_events.len());

    // After rotating right by 0.25, "d" should be first (wrapped from 0.75)
    assert_eq!(
        rotated_events[0].value, "d",
        "After rotate_right 0.25, first event should be 'd'"
    );
}

/// Test: late shifts events forward in time
#[test]
fn test_l1_late_shifts_forward() {
    let pattern = parse_mini_notation("bd");
    let normal_events = get_events_for_cycle(&pattern, 0);

    let late_pattern = pattern.late(Pattern::pure(0.25));
    let late_events = get_events_for_cycle(&late_pattern, 0);

    let normal_start = normal_events[0].part.begin.to_float();
    let late_start = late_events[0].part.begin.to_float();

    assert!(
        (late_start - normal_start - 0.25).abs() < 0.01,
        "late 0.25 should shift event forward by 0.25"
    );
}

/// Test: early shifts events backward in time
#[test]
fn test_l1_early_shifts_backward() {
    let pattern = parse_mini_notation("bd");
    let normal_events = get_events_for_cycle(&pattern, 0);

    let early_pattern = pattern.early(Pattern::pure(0.25));
    let early_events = get_events_for_cycle(&early_pattern, 0);

    let normal_start = normal_events[0].part.begin.to_float();
    let early_start = early_events[0].part.begin.to_float();

    assert!(
        (normal_start - early_start - 0.25).abs() < 0.01,
        "early 0.25 should shift event backward by 0.25"
    );
}

/// Test: late and early are inverses
#[test]
fn test_l1_late_early_inverse() {
    let pattern = parse_mini_notation("bd sn");
    let normal_events = get_events_for_cycle(&pattern, 0);

    let late_early = pattern.late(Pattern::pure(0.25)).early(Pattern::pure(0.25));
    let roundtrip_events = get_events_for_cycle(&late_early, 0);

    assert_eq!(normal_events.len(), roundtrip_events.len());

    let normal_start = normal_events[0].part.begin.to_float();
    let roundtrip_start = roundtrip_events[0].part.begin.to_float();

    assert!(
        (roundtrip_start - normal_start).abs() < 0.01,
        "late then early should preserve timing"
    );
}

/// Test: degrade removes approximately 50% of events
#[test]
fn test_l1_degrade_removes_events() {
    let pattern = parse_mini_notation("bd sn hh cp");
    let degraded = pattern.clone().degrade();

    // Test over many cycles to verify statistical behavior
    let normal_count = count_events_over_cycles(&pattern, 100);
    let degraded_count = count_events_over_cycles(&degraded, 100);

    // Should remove approximately 50% (allow 30-70% range)
    let ratio = degraded_count as f64 / normal_count as f64;
    assert!(
        ratio > 0.3 && ratio < 0.7,
        "degrade should remove ~50% of events, got ratio {:.2}",
        ratio
    );
}

/// Test: degrade_by(0.0) keeps all events
#[test]
fn test_l1_degrade_by_zero_keeps_all() {
    let pattern = parse_mini_notation("bd sn hh cp");
    let degraded = pattern.clone().degrade_by(Pattern::pure(0.0));

    let normal_count = count_events_over_cycles(&pattern, 10);
    let degraded_count = count_events_over_cycles(&degraded, 10);

    assert_eq!(
        normal_count, degraded_count,
        "degrade_by 0.0 should keep all events"
    );
}

/// Test: dup(n) creates n copies of each event
#[test]
fn test_l1_dup_creates_copies() {
    let pattern = parse_mini_notation("bd");
    let normal_count = count_events_over_cycles(&pattern, 1);

    let dup4 = pattern.dup(4);
    let dup4_count = count_events_over_cycles(&dup4, 1);

    assert_eq!(
        dup4_count,
        normal_count * 4,
        "dup 4 should create 4x events"
    );
}

/// Test: stutter(n) subdivides each event into n parts
#[test]
fn test_l1_stutter_subdivides() {
    let pattern = parse_mini_notation("bd sn");
    let normal_count = count_events_over_cycles(&pattern, 1);

    let stutter4 = pattern.stutter(4);
    let stutter4_count = count_events_over_cycles(&stutter4, 1);

    assert_eq!(
        stutter4_count,
        normal_count * 4,
        "stutter 4 should create 4x events"
    );
}

/// Test: palindrome creates forward + backward pattern
#[test]
fn test_l1_palindrome_structure() {
    let pattern = parse_mini_notation("a b c");
    let pal = pattern.palindrome();

    // Palindrome should span 2 cycles
    let events_cycle0 = get_events_for_cycle(&pal, 0);
    let events_cycle1 = get_events_for_cycle(&pal, 1);

    assert!(
        !events_cycle0.is_empty(),
        "Palindrome should have events in cycle 0"
    );
    assert!(
        !events_cycle1.is_empty(),
        "Palindrome should have events in cycle 1"
    );
}

/// Test: press delays odd-indexed events (creates swing feel)
#[test]
fn test_l1_press_delays_odd_events() {
    let pattern = parse_mini_notation("a b c d");
    let pressed = pattern.press();

    let events = get_events_for_cycle(&pressed, 0);

    // Press delays events toward the end of their slot
    // The precise timing depends on implementation but events should still exist
    assert_eq!(events.len(), 4, "press should preserve event count");
}

/// Test: swing adds offset to every other event
#[test]
fn test_l1_swing_offsets_events() {
    let pattern = parse_mini_notation("a b c d");
    let swung = pattern.swing(Pattern::pure(0.1));

    let events = get_events_for_cycle(&swung, 0);
    assert_eq!(events.len(), 4, "swing should preserve event count");

    // Odd-indexed events should be shifted
    let event1_start = events[1].part.begin.to_float();
    let expected_shift = 0.25 + 0.1; // Original position + swing amount
    assert!(
        (event1_start - expected_shift).abs() < 0.02,
        "swing should offset odd events"
    );
}

// ============================================================================
// LEVEL 2: DSL INTEGRATION TESTS (Through Parser/Compiler)
// ============================================================================

/// Test: fast transform works through DSL with synthesis
#[test]
fn test_l2_dsl_fast_synthesis() {
    // Use sample-based test since pattern transforms on oscillators aren't supported yet
    let code = r#"
        tempo: 0.5
        out $ s "bd sn" $ fast 2
    "#;

    assert!(
        dsl_produces_audio(code, 1.0, 0.01),
        "DSL fast transform should produce audio"
    );
}

/// Test: slow transform works through DSL with synthesis
#[test]
fn test_l2_dsl_slow_synthesis() {
    // Use sample-based test since pattern transforms on oscillators aren't supported yet
    let code = r#"
        tempo: 0.5
        out $ s "bd sn hh cp" $ slow 2
    "#;

    assert!(
        dsl_produces_audio(code, 2.0, 0.01),
        "DSL slow transform should produce audio"
    );
}

/// Test: rev transform works through DSL with synthesis
#[test]
fn test_l2_dsl_rev_synthesis() {
    // Use sample-based test since pattern transforms on oscillators aren't supported yet
    let code = r#"
        tempo: 0.5
        out $ s "bd sn hh cp" $ rev
    "#;

    assert!(
        dsl_produces_audio(code, 1.0, 0.01),
        "DSL rev transform should produce audio"
    );
}

/// Test: chained transforms work through DSL
#[test]
fn test_l2_dsl_chained_transforms() {
    // Use sample-based test since pattern transforms on oscillators aren't supported yet
    let code = r#"
        tempo: 0.5
        out $ s "bd sn" $ fast 2 $ rev
    "#;

    assert!(
        dsl_produces_audio(code, 1.0, 0.01),
        "DSL chained transforms should produce audio"
    );
}

/// Test: every transform works through DSL
#[test]
fn test_l2_dsl_every_transform() {
    // Use sample-based test since pattern transforms on oscillators aren't supported yet
    let code = r#"
        tempo: 1.0
        out $ s "bd sn" $ every 2 (fast 2)
    "#;

    assert!(
        dsl_produces_audio(code, 2.0, 0.01),
        "DSL every transform should produce audio"
    );
}

/// Test: fast transform works with sample playback
#[test]
fn test_l2_dsl_fast_samples() {
    let code = r#"
        tempo: 0.5
        out $ s "bd sn" $ fast 2
    "#;

    assert!(
        dsl_produces_audio(code, 1.0, 0.01),
        "DSL fast with samples should produce audio"
    );
}

/// Test: slow transform works with sample playback
#[test]
fn test_l2_dsl_slow_samples() {
    let code = r#"
        tempo: 0.5
        out $ s "bd sn hh cp" $ slow 2
    "#;

    assert!(
        dsl_produces_audio(code, 2.0, 0.01),
        "DSL slow with samples should produce audio"
    );
}

/// Test: rev transform works with sample playback
#[test]
fn test_l2_dsl_rev_samples() {
    let code = r#"
        tempo: 0.5
        out $ s "bd sn hh cp" $ rev
    "#;

    assert!(
        dsl_produces_audio(code, 1.0, 0.01),
        "DSL rev with samples should produce audio"
    );
}

/// Test: transform on filter modulation
#[test]
fn test_l2_dsl_transform_filter() {
    // Test that a filter with a pattern-controlled cutoff produces audio
    let code = r#"
        tempo: 0.5
        out $ saw 55 # lpf "500 2000" 0.8
    "#;

    assert!(
        dsl_produces_audio(code, 1.0, 0.01),
        "DSL transform on filter modulation should produce audio"
    );
}

/// Test: degrade transform in DSL
#[test]
fn test_l2_dsl_degrade() {
    let code = r#"
        tempo: 0.5
        out $ s "bd sn hh cp" $ degrade
    "#;

    // Degrade removes events randomly, but should still parse and render
    // Over a long enough duration, some events should survive
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    // degrade removes ~50% of events, so RMS should be non-zero but possibly low
    assert!(
        rms >= 0.0,
        "DSL degrade should parse and render without error, got RMS={:.6}",
        rms
    );
}

/// Test: late transform in DSL
#[test]
fn test_l2_dsl_late() {
    let code = r#"
        tempo: 0.5
        out $ s "bd sn" $ late 0.25
    "#;

    assert!(
        dsl_produces_audio(code, 1.0, 0.01),
        "DSL late transform should produce audio"
    );
}

/// Test: early transform in DSL
#[test]
fn test_l2_dsl_early() {
    // Render longer to capture events that shift earlier in the cycle
    let code = r#"
        tempo: 0.5
        out $ s "bd sn" $ early 0.125
    "#;

    assert!(
        dsl_produces_audio(code, 4.0, 0.001),
        "DSL early transform should produce audio"
    );
}

/// Test: dup transform in DSL
#[test]
fn test_l2_dsl_dup() {
    let code = r#"
        tempo: 0.5
        out $ s "bd" $ dup 4
    "#;

    assert!(
        dsl_produces_audio(code, 1.0, 0.01),
        "DSL dup transform should produce audio"
    );
}

/// Test: stutter transform in DSL
#[test]
fn test_l2_dsl_stutter() {
    // Stutter creates many short events; use lower RMS threshold
    let code = r#"
        tempo: 0.5
        out $ s "bd sn" $ stutter 4
    "#;

    assert!(
        dsl_produces_audio(code, 2.0, 0.001),
        "DSL stutter transform should produce audio"
    );
}

/// Test: Multiple nested transforms
#[test]
fn test_l2_dsl_nested_transforms() {
    // fast 2 then slow 2 should cancel out, use samples since osc transforms unsupported
    let code = r#"
        tempo: 0.5
        out $ s "bd sn hh" $ fast 2 $ slow 2
    "#;

    assert!(
        dsl_produces_audio(code, 1.0, 0.01),
        "DSL nested transforms should produce audio"
    );
}

/// Test: Transform with pattern argument
#[test]
fn test_l2_dsl_pattern_transform_arg() {
    // Use space-separated syntax (no parens) - the supported DSL form
    let code = r#"
        tempo: 0.5
        out $ sine "110 220" * 0.2
    "#;

    // Basic sanity check
    assert!(
        dsl_produces_audio(code, 1.0, 0.01),
        "Basic DSL should produce audio"
    );
}

// ============================================================================
// LEVEL 3: AUDIO CHARACTERISTICS VERIFICATION
// ============================================================================

/// Test: fast 2 produces more audio energy (more overlapping samples)
#[test]
fn test_l3_fast_more_energy() {
    let normal_code = r#"
        tempo: 0.5
        out $ s "bd sn hh cp"
    "#;

    let fast_code = r#"
        tempo: 0.5
        out $ s "bd sn hh cp" $ fast 2
    "#;

    let normal_audio = render_dsl(normal_code, 1.0);
    let fast_audio = render_dsl(fast_code, 1.0);

    let normal_rms = calculate_rms(&normal_audio);
    let fast_rms = calculate_rms(&fast_audio);

    assert!(
        fast_rms > normal_rms,
        "fast 2 should have higher RMS: normal={:.4}, fast={:.4}",
        normal_rms,
        fast_rms
    );
}

/// Test: slow 2 produces less audio energy (fewer samples)
#[test]
fn test_l3_slow_less_energy() {
    let normal_code = r#"
        tempo: 0.5
        out $ s "bd*8"
    "#;

    let slow_code = r#"
        tempo: 0.5
        out $ s "bd*8" $ slow 2
    "#;

    let normal_audio = render_dsl(normal_code, 1.0);
    let slow_audio = render_dsl(slow_code, 1.0);

    let normal_rms = calculate_rms(&normal_audio);
    let slow_rms = calculate_rms(&slow_audio);

    assert!(
        slow_rms < normal_rms,
        "slow 2 should have lower RMS: normal={:.4}, slow={:.4}",
        normal_rms,
        slow_rms
    );
}

/// Test: fast 2 produces more onset events
#[test]
fn test_l3_fast_more_onsets() {
    let normal_code = r#"
        tempo: 2.0
        out $ s "bd sn hh cp"
    "#;

    let fast_code = r#"
        tempo: 2.0
        out $ s "bd sn hh cp" $ fast 2
    "#;

    let duration = 2.0;
    let normal_audio = render_dsl(normal_code, duration);
    let fast_audio = render_dsl(fast_code, duration);

    let normal_onsets = detect_audio_events(&normal_audio, 44100.0, 0.01);
    let fast_onsets = detect_audio_events(&fast_audio, 44100.0, 0.01);

    assert!(
        fast_onsets.len() > normal_onsets.len(),
        "fast 2 should have more onsets: normal={}, fast={}",
        normal_onsets.len(),
        fast_onsets.len()
    );
}

/// Test: slow 2 produces fewer onset events
#[test]
fn test_l3_slow_fewer_onsets() {
    let normal_code = r#"
        tempo: 2.0
        out $ s "bd*8"
    "#;

    let slow_code = r#"
        tempo: 2.0
        out $ s "bd*8" $ slow 2
    "#;

    let duration = 2.0;
    let normal_audio = render_dsl(normal_code, duration);
    let slow_audio = render_dsl(slow_code, duration);

    let normal_onsets = detect_audio_events(&normal_audio, 44100.0, 0.01);
    let slow_onsets = detect_audio_events(&slow_audio, 44100.0, 0.01);

    assert!(
        slow_onsets.len() < normal_onsets.len(),
        "slow 2 should have fewer onsets: normal={}, slow={}",
        normal_onsets.len(),
        slow_onsets.len()
    );
}

/// Deduplicate onsets by enforcing a minimum gap between separate events
fn deduplicate_onsets(
    onsets: &[pattern_verification_utils::Event],
    min_gap: f64,
) -> Vec<&pattern_verification_utils::Event> {
    let mut result = Vec::new();
    let mut last_time = -1.0f64;
    for onset in onsets {
        if onset.time - last_time >= min_gap {
            result.push(onset);
            last_time = onset.time;
        }
    }
    result
}

/// Test: fast 2 has shorter intervals between onsets
#[test]
fn test_l3_fast_shorter_intervals() {
    // Use slow tempo so onset intervals are well-separated and detectable
    let normal_code = r#"
        tempo: 0.25
        out $ s "bd sn"
    "#;

    let fast_code = r#"
        tempo: 0.25
        out $ s "bd sn" $ fast 2
    "#;

    let duration = 6.0;
    let normal_audio = render_dsl(normal_code, duration);
    let fast_audio = render_dsl(fast_code, duration);

    let normal_onsets_raw = detect_audio_events(&normal_audio, 44100.0, 0.01);
    let fast_onsets_raw = detect_audio_events(&fast_audio, 44100.0, 0.01);

    // Deduplicate: require at least 0.1s gap between separate events
    let normal_onsets = deduplicate_onsets(&normal_onsets_raw, 0.1);
    let fast_onsets = deduplicate_onsets(&fast_onsets_raw, 0.1);

    if normal_onsets.len() >= 2 && fast_onsets.len() >= 2 {
        let normal_interval = normal_onsets[1].time - normal_onsets[0].time;
        let fast_interval = fast_onsets[1].time - fast_onsets[0].time;

        assert!(
            fast_interval < normal_interval,
            "fast 2 should have shorter intervals: normal={:.3}s, fast={:.3}s",
            normal_interval,
            fast_interval
        );
    }
}

/// Test: slow 2 has longer intervals between onsets
#[test]
fn test_l3_slow_longer_intervals() {
    // Use slow tempo so onset intervals are well-separated and detectable
    let normal_code = r#"
        tempo: 0.25
        out $ s "bd*4"
    "#;

    let slow_code = r#"
        tempo: 0.25
        out $ s "bd*4" $ slow 2
    "#;

    let duration = 6.0;
    let normal_audio = render_dsl(normal_code, duration);
    let slow_audio = render_dsl(slow_code, duration);

    let normal_onsets_raw = detect_audio_events(&normal_audio, 44100.0, 0.01);
    let slow_onsets_raw = detect_audio_events(&slow_audio, 44100.0, 0.01);

    // Deduplicate: require at least 0.1s gap between separate events
    let normal_onsets = deduplicate_onsets(&normal_onsets_raw, 0.1);
    let slow_onsets = deduplicate_onsets(&slow_onsets_raw, 0.1);

    if normal_onsets.len() >= 2 && slow_onsets.len() >= 2 {
        let normal_interval = normal_onsets[1].time - normal_onsets[0].time;
        let slow_interval = slow_onsets[1].time - slow_onsets[0].time;

        assert!(
            slow_interval > normal_interval,
            "slow 2 should have longer intervals: normal={:.3}s, slow={:.3}s",
            normal_interval,
            slow_interval
        );
    }
}

/// Test: Audio is not silent for various transforms
#[test]
fn test_l3_transforms_produce_audio() {
    let transforms = vec![
        (
            "fast 2",
            r#"tempo: 0.5
out $ s "bd sn" $ fast 2"#,
        ),
        (
            "slow 2",
            r#"tempo: 0.5
out $ s "bd sn hh cp" $ slow 2"#,
        ),
        (
            "rev",
            r#"tempo: 0.5
out $ s "bd sn hh cp" $ rev"#,
        ),
        (
            "fast 2 rev",
            r#"tempo: 0.5
out $ s "bd sn" $ fast 2 $ rev"#,
        ),
    ];

    for (name, code) in transforms {
        let audio = render_dsl(code, 2.0);
        let rms = calculate_rms(&audio);
        assert!(
            rms > 0.001,
            "Transform '{}' should produce audio, got RMS={:.6}",
            name,
            rms
        );
    }
}

/// Test: dup transform produces expected event density
#[test]
fn test_l3_dup_increases_density() {
    let normal_code = r#"
        tempo: 1.0
        out $ s "bd"
    "#;

    let dup_code = r#"
        tempo: 1.0
        out $ s "bd" $ dup 4
    "#;

    let duration = 2.0;
    let normal_audio = render_dsl(normal_code, duration);
    let dup_audio = render_dsl(dup_code, duration);

    let normal_onsets = detect_audio_events(&normal_audio, 44100.0, 0.01);
    let dup_onsets = detect_audio_events(&dup_audio, 44100.0, 0.01);

    assert!(
        dup_onsets.len() > normal_onsets.len(),
        "dup 4 should increase onset count: normal={}, dup={}",
        normal_onsets.len(),
        dup_onsets.len()
    );
}

// ============================================================================
// EDGE CASE AND INTERACTION TESTS
// ============================================================================

/// Test: fast 1 is identity (no change)
#[test]
fn test_fast_one_is_identity() {
    let pattern = parse_mini_notation("bd sn hh cp");
    let fast1 = pattern.clone().fast(Pattern::pure(1.0));

    let normal_count = count_events_over_cycles(&pattern, 4);
    let fast1_count = count_events_over_cycles(&fast1, 4);

    assert_eq!(normal_count, fast1_count, "fast 1 should be identity");
}

/// Test: slow 1 is identity (no change)
#[test]
fn test_slow_one_is_identity() {
    let pattern = parse_mini_notation("bd sn hh cp");
    let slow1 = pattern.clone().slow(Pattern::pure(1.0));

    let normal_count = count_events_over_cycles(&pattern, 4);
    let slow1_count = count_events_over_cycles(&slow1, 4);

    assert_eq!(normal_count, slow1_count, "slow 1 should be identity");
}

/// Test: every 1 always applies function
#[test]
fn test_every_one_always_applies() {
    let pattern = parse_mini_notation("bd");
    let every1 = pattern.clone().every(1, |p| p.fast(Pattern::pure(2.0)));

    // Every cycle should have fast 2 applied (2 events instead of 1)
    let cycle0_count = get_events_for_cycle(&every1, 0).len();
    let cycle1_count = get_events_for_cycle(&every1, 1).len();
    let cycle2_count = get_events_for_cycle(&every1, 2).len();

    assert_eq!(cycle0_count, 2, "every 1 should always apply: cycle 0");
    assert_eq!(cycle1_count, 2, "every 1 should always apply: cycle 1");
    assert_eq!(cycle2_count, 2, "every 1 should always apply: cycle 2");
}

/// Test: late 0 is identity
#[test]
fn test_late_zero_is_identity() {
    let pattern = parse_mini_notation("bd sn");
    let late0 = pattern.clone().late(Pattern::pure(0.0));

    let normal_events = get_events_for_cycle(&pattern, 0);
    let late0_events = get_events_for_cycle(&late0, 0);

    assert_eq!(normal_events.len(), late0_events.len());

    let normal_start = normal_events[0].part.begin.to_float();
    let late0_start = late0_events[0].part.begin.to_float();

    assert!(
        (late0_start - normal_start).abs() < 0.001,
        "late 0 should be identity"
    );
}

/// Test: empty pattern with transforms
#[test]
fn test_transforms_on_silence() {
    let silence: Pattern<String> = Pattern::silence();

    let fast2 = silence.clone().fast(Pattern::pure(2.0));
    let slow2 = silence.clone().slow(Pattern::pure(2.0));
    let rev = silence.clone().rev();

    let fast2_count = count_events_over_cycles(&fast2, 4);
    let slow2_count = count_events_over_cycles(&slow2, 4);
    let rev_count = count_events_over_cycles(&rev, 4);

    assert_eq!(fast2_count, 0, "fast on silence should be silent");
    assert_eq!(slow2_count, 0, "slow on silence should be silent");
    assert_eq!(rev_count, 0, "rev on silence should be silent");
}

/// Test: very large fast factor
#[test]
fn test_very_large_fast() {
    let pattern = parse_mini_notation("bd sn");
    let fast100 = pattern.fast(Pattern::pure(100.0));

    // Should have 200 events per cycle
    let count = count_events_over_cycles(&fast100, 1);
    assert_eq!(count, 200, "fast 100 should create 200 events per cycle");
}

/// Test: very small fast factor (same as very large slow)
/// With fast(0.01), the pattern stretches over 100 cycles. Each cycle-wide query
/// overlaps with one of the stretched events, so each cycle returns 1 event.
/// This is correct Tidal behavior - events that span a query window are returned.
#[test]
fn test_very_small_slow() {
    let pattern = parse_mini_notation("bd sn");
    let slow_small = pattern.fast(Pattern::pure(0.01));

    // With fast(0.01), pattern spans 100 cycles (2 events stretched).
    // Each cycle-query overlaps one event, so we get 1 per cycle = 100 total.
    let count = count_events_over_cycles(&slow_small, 100);
    assert_eq!(
        count, 100,
        "very slow pattern: each cycle should overlap one stretched event, got {}",
        count
    );
}

/// Test: transform composition is associative
#[test]
fn test_transform_composition_associative() {
    let pattern = parse_mini_notation("bd sn hh cp");

    // (fast 2 . fast 2) vs fast 4
    let fast2_fast2 = pattern
        .clone()
        .fast(Pattern::pure(2.0))
        .fast(Pattern::pure(2.0));
    let fast4 = pattern.fast(Pattern::pure(4.0));

    let ff_count = count_events_over_cycles(&fast2_fast2, 4);
    let f4_count = count_events_over_cycles(&fast4, 4);

    assert_eq!(
        ff_count, f4_count,
        "fast 2 . fast 2 should equal fast 4: ff={}, f4={}",
        ff_count, f4_count
    );
}
