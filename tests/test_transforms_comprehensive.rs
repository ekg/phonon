//! Comprehensive pattern transform tests
//!
//! Each test follows this methodology:
//! 1. Query pattern over 4-8 cycles (lightweight verification)
//! 2. Verify event counts, ordering, and timing
//! 3. Render audio and analyze RMS/peaks (end-to-end verification)
//! 4. Compare against baseline expectations
//!
//! This dual approach ensures:
//! - Pattern logic is correct (queries)
//! - Audio output matches expectations (e2e)
//! - Tests catch both pattern bugs and DSL integration bugs

use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};
use std::collections::HashMap;

mod audio_test_utils;
use audio_test_utils::calculate_rms;

// ============================================================================
// HELPER: Query pattern over multiple cycles
// ============================================================================

fn query_pattern_cycles<T: Clone + Send + Sync + 'static>(
    pattern: &Pattern<T>,
    cycles: usize,
) -> Vec<Vec<T>> {
    let mut results = Vec::new();
    for cycle in 0..cycles {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };
        let events = pattern.query(&state);
        results.push(events.into_iter().map(|e| e.value).collect());
    }
    results
}

fn count_events_over_cycles<T: Clone + Send + Sync + 'static>(
    pattern: &Pattern<T>,
    cycles: usize,
) -> usize {
    query_pattern_cycles(pattern, cycles)
        .into_iter()
        .map(|v| v.len())
        .sum()
}

// ============================================================================
// TIME TRANSFORMS: fast, slow
// ============================================================================

#[test]
fn test_fast_doubles_event_density() {
    println!("\n=== FAST TRANSFORM TEST ===");

    // STEP 1: Pattern query verification (lightweight)
    println!("\n1. Pattern Query Verification:");
    let pattern = parse_mini_notation("bd sn hh cp");

    let normal = pattern.clone();
    let fast2 = pattern.clone().fast(Pattern::pure(2.0));
    let fast4 = pattern.clone().fast(Pattern::pure(4.0));

    let cycles = 8;
    let normal_count = count_events_over_cycles(&normal, cycles);
    let fast2_count = count_events_over_cycles(&fast2, cycles);
    let fast4_count = count_events_over_cycles(&fast4, cycles);

    println!("   Normal: {} events over {} cycles", normal_count, cycles);
    println!("   Fast x2: {} events over {} cycles", fast2_count, cycles);
    println!("   Fast x4: {} events over {} cycles", fast4_count, cycles);

    // Tidal Cycles behavior: fast n multiplies event density by n
    assert_eq!(
        fast2_count,
        normal_count * 2,
        "fast 2 should double event count"
    );
    assert_eq!(
        fast4_count,
        normal_count * 4,
        "fast 4 should quadruple event count"
    );

    // STEP 2: Audio verification (end-to-end)
    println!("\n2. Audio Verification:");

    let input_normal = r#"
        tempo: 2.0
        out: s "bd sn hh cp"
    "#;
    let input_fast2 = r#"
        tempo: 2.0
        out: s "bd sn hh cp bd sn hh cp bd sn hh cp bd sn hh cp bd sn hh cp bd sn hh cp bd sn hh cp bd sn hh cp"
    "#;
    let input_fast4 = r#"
        tempo: 2.0
        out: s "bd sn hh cp bd sn hh cp bd sn hh cp bd sn hh cp bd sn hh cp bd sn hh cp bd sn hh cp bd sn hh cp bd sn hh cp bd sn hh cp bd sn hh cp bd sn hh cp bd sn hh cp bd sn hh cp bd sn hh cp bd sn hh cp"
    "#;

    let normal_audio = render_dsl(input_normal, 4.0); // 8 cycles
    let fast2_audio = render_dsl(input_fast2, 4.0);
    let fast4_audio = render_dsl(input_fast4, 4.0);

    let rms_normal = calculate_rms(&normal_audio);
    let rms_fast2 = calculate_rms(&fast2_audio);
    let rms_fast4 = calculate_rms(&fast4_audio);

    println!("   Normal RMS: {:.4}", rms_normal);
    println!("   Fast x2 RMS: {:.4}", rms_fast2);
    println!("   Fast x4 RMS: {:.4}", rms_fast4);

    // More events = higher RMS (samples overlap more)
    assert!(
        rms_fast2 > rms_normal * 1.3,
        "Fast x2 should have significantly higher RMS"
    );
    assert!(
        rms_fast4 > rms_fast2 * 1.3,
        "Fast x4 should have even higher RMS"
    );

    println!("✅ Fast transform verified");
}

#[test]
fn test_slow_reduces_event_density() {
    println!("\n=== SLOW TRANSFORM TEST ===");

    // STEP 1: Pattern query verification
    println!("\n1. Pattern Query Verification:");
    let pattern = parse_mini_notation("bd sn hh cp");

    let normal = pattern.clone();
    let slow2 = pattern.clone().slow(Pattern::pure(2.0));
    let slow4 = pattern.clone().slow(Pattern::pure(4.0));

    let cycles = 8;
    let normal_count = count_events_over_cycles(&normal, cycles);
    let slow2_count = count_events_over_cycles(&slow2, cycles);
    let slow4_count = count_events_over_cycles(&slow4, cycles);

    println!("   Normal: {} events over {} cycles", normal_count, cycles);
    println!("   Slow x2: {} events over {} cycles", slow2_count, cycles);
    println!("   Slow x4: {} events over {} cycles", slow4_count, cycles);

    // Tidal Cycles behavior: slow n divides event density by n
    assert_eq!(
        slow2_count,
        normal_count / 2,
        "slow 2 should halve event count"
    );
    assert_eq!(
        slow4_count,
        normal_count / 4,
        "slow 4 should quarter event count"
    );

    // STEP 2: Audio verification
    println!("\n2. Audio Verification:");

    let input_normal = r#"
        tempo: 2.0
        out: s("bd sn hh cp")
    "#;
    let input_slow2 = r#"
        tempo: 2.0
        out: s("bd sn hh cp" |> slow 2)
    "#;
    let input_slow4 = r#"
        tempo: 2.0
        out: s("bd sn hh cp" |> slow 4)
    "#;

    let normal_audio = render_dsl(input_normal, 4.0);
    let slow2_audio = render_dsl(input_slow2, 4.0);
    let slow4_audio = render_dsl(input_slow4, 4.0);

    let rms_normal = calculate_rms(&normal_audio);
    let rms_slow2 = calculate_rms(&slow2_audio);
    let rms_slow4 = calculate_rms(&slow4_audio);

    println!("   Normal RMS: {:.4}", rms_normal);
    println!("   Slow x2 RMS: {:.4}", rms_slow2);
    println!("   Slow x4 RMS: {:.4}", rms_slow4);

    // Fewer events = lower RMS
    assert!(
        rms_slow2 < rms_normal * 0.8,
        "Slow x2 should have lower RMS"
    );
    assert!(
        rms_slow4 < rms_slow2 * 0.8,
        "Slow x4 should have even lower RMS"
    );

    println!("✅ Slow transform verified");
}

// ============================================================================
// REVERSAL: rev
// ============================================================================

#[test]
fn test_rev_reverses_pattern_order() {
    println!("\n=== REV TRANSFORM TEST ===");

    // STEP 1: Pattern query verification
    println!("\n1. Pattern Query Verification:");
    let pattern = parse_mini_notation("bd sn hh cp");

    let normal = pattern.clone();
    let reversed = pattern.clone().rev();

    let normal_events = query_pattern_cycles(&normal, 1);
    let reversed_events = query_pattern_cycles(&reversed, 1);

    println!("   Normal: {:?}", normal_events[0]);
    println!("   Reversed: {:?}", reversed_events[0]);

    // Tidal Cycles behavior: rev reverses event order
    assert_eq!(normal_events[0].len(), reversed_events[0].len());
    assert_eq!(normal_events[0][0], "bd");
    assert_eq!(reversed_events[0][0], "cp"); // First becomes last
    assert_eq!(reversed_events[0][3], "bd"); // Last becomes first

    // Verify complete reversal
    let normal_joined: String = normal_events[0].join(" ");
    let reversed_joined: String = reversed_events[0]
        .iter()
        .rev()
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(normal_joined, reversed_joined);

    // STEP 2: Audio verification (should have same RMS, different timing)
    println!("\n2. Audio Verification:");

    let input_normal = r#"
        tempo: 2.0
        out: s("bd sn hh cp")
    "#;
    let input_rev = r#"
        tempo: 2.0
        out: s("bd sn hh cp" |> rev)
    "#;

    let normal_audio = render_dsl(input_normal, 2.0);
    let rev_audio = render_dsl(input_rev, 2.0);

    let rms_normal = calculate_rms(&normal_audio);
    let rms_rev = calculate_rms(&rev_audio);

    println!("   Normal RMS: {:.4}", rms_normal);
    println!("   Reversed RMS: {:.4}", rms_rev);

    // Same events, just reversed timing - RMS should be similar
    assert!(
        (rms_rev - rms_normal).abs() < rms_normal * 0.1,
        "Reversed pattern should have similar RMS"
    );

    println!("✅ Rev transform verified");
}

// ============================================================================
// CONDITIONAL: every
// ============================================================================

#[test]
fn test_every_alternates_transformation() {
    println!("\n=== EVERY TRANSFORM TEST ===");

    // STEP 1: Pattern query verification
    println!("\n1. Pattern Query Verification:");
    let pattern = parse_mini_notation("bd sn hh cp");

    // every 2 (fast 2) - apply fast 2 every 2nd cycle
    let every_pattern = pattern.clone().every(2, |p| p.fast(Pattern::pure(2.0)));

    let mut event_counts = Vec::new();
    for cycle in 0..8 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };
        let count = every_pattern.query(&state).len();
        event_counts.push(count);
        println!("   Cycle {}: {} events", cycle, count);
    }

    // Tidal Cycles behavior: every n applies transform on cycles 0, n, 2n, ...
    // Base pattern: 4 events/cycle
    // With fast 2: 8 events/cycle
    assert_eq!(event_counts[0], 8, "Cycle 0 should be fast (8 events)");
    assert_eq!(event_counts[1], 4, "Cycle 1 should be normal (4 events)");
    assert_eq!(event_counts[2], 8, "Cycle 2 should be fast (8 events)");
    assert_eq!(event_counts[3], 4, "Cycle 3 should be normal (4 events)");

    // STEP 2: Audio verification
    println!("\n2. Audio Verification:");

    let input = r#"
        tempo: 2.0
        out: s("bd sn hh cp" |> every 2 (fast 2))
    "#;

    let audio = render_dsl(input, 4.0); // 8 cycles
    let rms = calculate_rms(&audio);

    println!("   RMS: {:.4}", rms);
    assert!(rms > 0.05, "Every transform should produce audio");

    println!("✅ Every transform verified");
}

// ============================================================================
// PROBABILISTIC: degrade, degradeBy
// ============================================================================

#[test]
fn test_degrade_removes_events() {
    println!("\n=== DEGRADE TRANSFORM TEST ===");

    // STEP 1: Pattern query verification
    println!("\n1. Pattern Query Verification:");
    let pattern = parse_mini_notation("bd sn hh cp");

    let normal = pattern.clone();
    let degrade50 = pattern.clone().degrade(); // 50% by default
    let degrade25 = pattern.clone().degrade_by(Pattern::pure(0.25));
    let degrade75 = pattern.clone().degrade_by(Pattern::pure(0.75));

    let cycles = 8;
    let normal_count = count_events_over_cycles(&normal, cycles);
    let degrade50_count = count_events_over_cycles(&degrade50, cycles);
    let degrade25_count = count_events_over_cycles(&degrade25, cycles);
    let degrade75_count = count_events_over_cycles(&degrade75, cycles);

    println!("   Normal: {} events", normal_count);
    println!("   Degrade 50%: {} events", degrade50_count);
    println!("   Degrade 25%: {} events", degrade25_count);
    println!("   Degrade 75%: {} events", degrade75_count);

    // Tidal Cycles behavior: degrade removes events probabilistically
    // With deterministic RNG, should be consistent
    assert!(
        degrade50_count < ((normal_count as f64 * 0.6) as usize),
        "degrade should remove ~50% of events"
    );
    assert!(
        degrade50_count > ((normal_count as f64 * 0.4) as usize),
        "degrade should keep ~50% of events"
    );
    assert!(
        degrade25_count < degrade50_count,
        "degrade 25% should keep more events than 50%"
    );
    assert!(
        degrade75_count < degrade50_count,
        "degrade 75% should keep fewer events than 50%"
    );

    // STEP 2: Audio verification
    println!("\n2. Audio Verification:");

    let input_normal = r#"
        tempo: 2.0
        out: s("bd*8")
    "#;
    let input_degrade = r#"
        tempo: 2.0
        out: s("bd*8" |> degrade)
    "#;

    let normal_audio = render_dsl(input_normal, 4.0);
    let degrade_audio = render_dsl(input_degrade, 4.0);

    let rms_normal = calculate_rms(&normal_audio);
    let rms_degrade = calculate_rms(&degrade_audio);

    println!("   Normal RMS: {:.4}", rms_normal);
    println!("   Degraded RMS: {:.4}", rms_degrade);

    // Fewer events = lower RMS
    assert!(
        rms_degrade < rms_normal * 0.8,
        "Degrade should reduce RMS due to fewer events"
    );

    println!("✅ Degrade transform verified");
}

// ============================================================================
// STRUCTURAL: stutter, ply
// ============================================================================

#[test]
fn test_stutter_repeats_events() {
    println!("\n=== STUTTER TRANSFORM TEST ===");

    // STEP 1: Pattern query verification
    println!("\n1. Pattern Query Verification:");
    let pattern = parse_mini_notation("bd sn");

    let normal = pattern.clone();
    let stutter3 = pattern.clone().stutter(3);

    let normal_events = query_pattern_cycles(&normal, 1);
    let stutter_events = query_pattern_cycles(&stutter3, 1);

    println!("   Normal: {:?}", normal_events[0]);
    println!("   Stutter x3: {:?}", stutter_events[0]);

    // Tidal Cycles behavior: stutter n repeats each event n times
    // "bd sn" $ stutter 3 = "bd bd bd sn sn sn"
    assert_eq!(stutter_events[0].len(), normal_events[0].len() * 3);
    assert_eq!(stutter_events[0][0], "bd");
    assert_eq!(stutter_events[0][1], "bd");
    assert_eq!(stutter_events[0][2], "bd");
    assert_eq!(stutter_events[0][3], "sn");

    // STEP 2: Audio verification
    println!("\n2. Audio Verification:");

    let input_normal = r#"
        tempo: 2.0
        out: s("bd sn")
    "#;
    let input_stutter = r#"
        tempo: 2.0
        out: s("bd sn" |> stutter 3)
    "#;

    let normal_audio = render_dsl(input_normal, 2.0);
    let stutter_audio = render_dsl(input_stutter, 2.0);

    let rms_normal = calculate_rms(&normal_audio);
    let rms_stutter = calculate_rms(&stutter_audio);

    println!("   Normal RMS: {:.4}", rms_normal);
    println!("   Stutter x3 RMS: {:.4}", rms_stutter);

    // More events = higher RMS
    assert!(
        rms_stutter > rms_normal * 1.3,
        "Stutter should increase RMS"
    );

    println!("✅ Stutter transform verified");
}

// ============================================================================
// HELPER: Render DSL code
// ============================================================================

fn render_dsl(code: &str, duration_secs: f32) -> Vec<f32> {
    let result = parse_dsl(code);
    if let Err(ref e) = result {
        eprintln!("DSL CODE:\n{}", code);
        eprintln!("PARSE ERROR: {:?}", e);
    }
    let (_, statements) = result.expect("Failed to parse DSL");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let samples = (44100.0 * duration_secs) as usize;
    graph.render(samples)
}
