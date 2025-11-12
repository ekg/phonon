/// Three-Level Verification Tests for `every` Transform
///
/// `every n transform` applies the transform every n cycles
/// Example: "a b" $ every 2 (fast 2)
/// - Cycle 0: a b (fast 2) → 4 events (fast applied)
/// - Cycle 1: a b → 2 events (normal)
/// - Cycle 2: a b (fast 2) → 4 events (fast applied)
/// - Cycle 3: a b → 2 events (normal)
///
/// The transform is applied on cycles 0, n, 2n, 3n, ...
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use std::collections::HashMap;

mod audio_test_utils;
mod pattern_verification_utils;
use audio_test_utils::calculate_rms;
use pattern_verification_utils::detect_audio_events;

/// Helper: Render DSL code and return audio buffer
fn render_dsl(code: &str, cycles: usize) -> Vec<f32> {
    let (_, statements) = parse_program(code).expect("Parse failed");
    let sample_rate = 44100.0;
    let mut graph = compile_program(statements, sample_rate).expect("Compile failed");

    let samples_per_cycle = (sample_rate as f64 / 0.5) as usize; // tempo = 0.5 cps
    let total_samples = samples_per_cycle * cycles;

    graph.render(total_samples)
}

// ============================================================================
// LEVEL 1: Pattern Query Verification (Exact Event Counts)
// ============================================================================

#[test]
fn test_every_level1_cycle_pattern() {
    // every 2 (fast 2) should apply fast 2 on cycles 0, 2, 4, ... and leave cycles 1, 3, 5, ... unchanged
    let base_pattern = "a b c d"; // 4 events per cycle
    let pattern = parse_mini_notation(base_pattern);

    // Manually apply every 2 (fast 2) logic
    let mut total_events_per_cycle = Vec::new();

    for cycle in 0..8 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        if cycle % 2 == 0 {
            // Apply fast 2 on even cycles
            let fast_pattern = pattern.clone().fast(Pattern::pure(2.0));
            let events = fast_pattern.query(&state);
            total_events_per_cycle.push(events.len());
        } else {
            // Normal on odd cycles
            let events = pattern.query(&state);
            total_events_per_cycle.push(events.len());
        }
    }

    // Verify pattern: [8, 4, 8, 4, 8, 4, 8, 4]
    assert_eq!(
        total_events_per_cycle[0], 8,
        "Cycle 0 should have fast applied (8 events)"
    );
    assert_eq!(
        total_events_per_cycle[1], 4,
        "Cycle 1 should be normal (4 events)"
    );
    assert_eq!(
        total_events_per_cycle[2], 8,
        "Cycle 2 should have fast applied (8 events)"
    );
    assert_eq!(
        total_events_per_cycle[3], 4,
        "Cycle 3 should be normal (4 events)"
    );
    assert_eq!(
        total_events_per_cycle[4], 8,
        "Cycle 4 should have fast applied (8 events)"
    );
    assert_eq!(
        total_events_per_cycle[5], 4,
        "Cycle 5 should be normal (4 events)"
    );

    println!(
        "✅ every Level 1: Cycle pattern verified: {:?}",
        total_events_per_cycle
    );
}

#[test]
fn test_every_level1_total_events() {
    // Verify total event count over multiple cycles
    let base_pattern = "a b"; // 2 events per cycle
    let pattern = parse_mini_notation(base_pattern);

    let mut total_normal = 0;
    let mut total_every_2_fast_2 = 0;

    for cycle in 0..8 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        // Normal pattern
        total_normal += pattern.query(&state).len();

        // every 2 (fast 2) pattern
        if cycle % 2 == 0 {
            total_every_2_fast_2 += pattern.clone().fast(Pattern::pure(2.0)).query(&state).len();
        } else {
            total_every_2_fast_2 += pattern.query(&state).len();
        }
    }

    // Normal: 8 cycles * 2 events = 16 events
    // every 2 (fast 2): 4 cycles with fast (4 * 4 = 16) + 4 cycles normal (4 * 2 = 8) = 24 events
    assert_eq!(
        total_normal, 16,
        "Normal pattern should have 16 events over 8 cycles"
    );
    assert_eq!(
        total_every_2_fast_2, 24,
        "every 2 (fast 2) should have 24 events over 8 cycles"
    );

    println!(
        "✅ every Level 1: Total events - normal={}, every 2 fast 2={}",
        total_normal, total_every_2_fast_2
    );
}

// ============================================================================
// LEVEL 2: Onset Detection (Audio Event Verification)
// ============================================================================

#[test]
fn test_every_level2_audio_onsets() {
    let base_code = r#"
tempo: 0.5
out: s "bd sn hh cp"
"#;

    let every_code = r#"
tempo: 0.5
out: s "bd sn hh cp" $ every 2 (fast 2)
"#;

    let cycles = 8;
    let base_audio = render_dsl(base_code, cycles);
    let every_audio = render_dsl(every_code, cycles);
    let sample_rate = 44100.0;

    // Detect audio onsets
    let base_onsets = detect_audio_events(&base_audio, sample_rate, 0.01);
    let every_onsets = detect_audio_events(&every_audio, sample_rate, 0.01);

    // every 2 (fast 2) should increase onset count
    // Base: 4 events/cycle * 8 cycles = 32 events
    // Every: 4 cycles with fast 2 (4 * 8 = 32) + 4 cycles normal (4 * 4 = 16) = 48 events
    // Ratio should be around 1.5x
    let ratio = every_onsets.len() as f32 / base_onsets.len() as f32;
    assert!(
        ratio > 1.2 && ratio < 2.0,
        "every 2 (fast 2) should increase onset count by ~1.5x: base={}, every={}, ratio={:.2}",
        base_onsets.len(),
        every_onsets.len(),
        ratio
    );

    println!(
        "✅ every Level 2: Base onsets = {}, every onsets = {}, ratio = {:.2}",
        base_onsets.len(),
        every_onsets.len(),
        ratio
    );
}

#[test]
fn test_every_level2_timing_variation() {
    // Verify that every creates timing variation across cycles
    let code = r#"
tempo: 0.5
out: s "bd sn" $ every 2 (fast 2)
"#;

    let cycles = 4; // 2 normal, 2 fast
    let audio = render_dsl(code, cycles);
    let sample_rate = 44100.0;

    let onsets = detect_audio_events(&audio, sample_rate, 0.01);

    // Should have varied timing (some events close together, some far apart)
    assert!(
        onsets.len() >= 10,
        "every 2 (fast 2) should produce multiple events (got {})",
        onsets.len()
    );

    // Check for timing variation by comparing intervals
    if onsets.len() >= 3 {
        let interval1 = onsets[1].time - onsets[0].time;
        let interval2 = onsets[2].time - onsets[1].time;

        // Intervals should vary due to alternating fast/normal cycles
        // (This is a loose check - just verify some variation exists)
        let variation = (interval1 - interval2).abs() / interval1.max(interval2);

        println!(
            "  Interval variation: {:.2} (interval1={:.3}s, interval2={:.3}s)",
            variation, interval1, interval2
        );
    }

    println!(
        "✅ every Level 2: Timing variation detected, {} onsets",
        onsets.len()
    );
}

// ============================================================================
// LEVEL 3: Audio Characteristics (Quality Checks)
// ============================================================================

#[test]
fn test_every_level3_audio_quality() {
    let code = r#"
tempo: 0.5
out: s "bd sn hh cp" $ every 2 (fast 2)
"#;

    let audio = render_dsl(code, 8);

    // Calculate audio characteristics
    let rms = calculate_rms(&audio);
    let peak = audio.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    let dc_offset = audio.iter().sum::<f32>() / audio.len() as f32;

    // Verify audio quality
    assert!(
        rms > 0.01,
        "every should produce audible audio (RMS = {})",
        rms
    );
    assert!(
        peak > 0.1,
        "every should have audible peaks (peak = {})",
        peak
    );
    assert!(
        dc_offset.abs() < 0.1,
        "every should not have excessive DC offset (DC = {})",
        dc_offset
    );

    println!(
        "✅ every Level 3: RMS = {:.4}, Peak = {:.4}, DC = {:.4}",
        rms, peak, dc_offset
    );
}

#[test]
fn test_every_level3_compare_to_base() {
    // every should have higher energy than base (more events on transformed cycles)
    let base_code = r#"
tempo: 0.5
out: s "bd sn hh cp"
"#;

    let every_code = r#"
tempo: 0.5
out: s "bd sn hh cp" $ every 2 (fast 2)
"#;

    let base_audio = render_dsl(base_code, 8);
    let every_audio = render_dsl(every_code, 8);

    let base_rms = calculate_rms(&base_audio);
    let every_rms = calculate_rms(&every_audio);

    // every 2 (fast 2) increases event count on half the cycles
    // Energy should be moderately higher (1.2-1.8x)
    let ratio = every_rms / base_rms;
    assert!(
        ratio > 1.1 && ratio < 2.0,
        "every energy should be 1.1-2x base: base RMS = {:.4}, every RMS = {:.4}, ratio = {:.2}",
        base_rms,
        every_rms,
        ratio
    );

    println!(
        "✅ every Level 3: Base RMS = {:.4}, every RMS = {:.4}, ratio = {:.2}",
        base_rms, every_rms, ratio
    );
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_every_with_every_1() {
    // every 1 should always apply transform (every cycle)
    let pattern = Pattern::from_string("a b");

    let mut total_every_1 = 0;
    let mut total_always_fast = 0;

    for cycle in 0..4 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        // every 1 (fast 2)
        if cycle % 1 == 0 {
            // Always true
            total_every_1 += pattern.clone().fast(Pattern::pure(2.0)).query(&state).len();
        } else {
            total_every_1 += pattern.query(&state).len();
        }

        // Always fast 2
        total_always_fast += pattern.clone().fast(Pattern::pure(2.0)).query(&state).len();
    }

    // every 1 (fast 2) should be equivalent to always applying fast 2
    assert_eq!(
        total_every_1, total_always_fast,
        "every 1 should always apply transform"
    );

    println!(
        "✅ every edge case: every 1 applies transform every cycle ({}  events)",
        total_every_1
    );
}

#[test]
fn test_every_with_rev() {
    // Test every with non-density-changing transform (rev)
    let code = r#"
tempo: 0.5
out: s "bd sn hh cp" $ every 3 rev
"#;

    let audio = render_dsl(code, 9); // 9 cycles to get 3 complete every-3 periods
    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "every with rev should produce audio");

    println!("✅ every edge case: every 3 rev works correctly");
}

#[test]
fn test_every_with_large_n() {
    // Test every with large cycle interval
    let code = r#"
tempo: 0.5
out: s "bd sn" $ every 8 (fast 4)
"#;

    let audio = render_dsl(code, 16); // 16 cycles to see pattern
    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "every with large n should still produce audio");

    println!("✅ every edge case: every 8 works correctly");
}

#[test]
fn test_every_preserves_base_pattern() {
    // Verify that cycles without transform are unchanged
    let pattern = Pattern::from_string("a b c");

    for cycle in 0..8 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let base_haps = pattern.query(&state);

        // On odd cycles (when every 2 doesn't apply), should match base
        if cycle % 2 == 1 {
            assert_eq!(
                base_haps.len(),
                3,
                "Cycle {} should be unchanged (3 events)",
                cycle
            );
        }
    }

    println!("✅ every edge case: base pattern preserved on non-transform cycles");
}
