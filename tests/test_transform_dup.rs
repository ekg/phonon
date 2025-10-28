/// Three-Level Verification Tests for `dup` Transform
///
/// `dup n` repeats the pattern n times (equivalent to fast n or bd*n)
/// Example: "a b" $ dup 3
/// - Cycle 0-1 contains: a b a b a b (3 repetitions)
/// - Each repetition is 1/3 of the original duration
/// Total: 6 events instead of 2
///
/// Implementation: dup n = fast n

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
fn test_dup_level1_event_count() {
    // dup n should multiply event count by n (same as fast n)
    let base_pattern = "a b c d"; // 4 events per cycle
    let cycles = 4;

    let pattern = parse_mini_notation(base_pattern);
    let dup_pattern = pattern.clone().dup(3);

    // Count events over 4 cycles
    let mut base_total = 0;
    let mut dup_total = 0;

    for cycle in 0..cycles {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        base_total += pattern.query(&state).len();
        dup_total += dup_pattern.query(&state).len();
    }

    // dup 3 should triple event count
    assert_eq!(dup_total, base_total * 3,
        "dup 3 should triple event count: base={}, dup={}",
        base_total, dup_total);

    println!("✅ dup Level 1: Base events = {}, dup events = {}",
             base_total, dup_total);
}

#[test]
fn test_dup_level1_event_timing() {
    // Verify dup compresses pattern correctly
    let pattern = Pattern::from_string("a b");
    let dup_pattern = pattern.dup(4);

    // Query single cycle
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let haps = dup_pattern.query(&state);

    // Should have 8 events (2 original * 4 dup)
    assert_eq!(haps.len(), 8, "dup 4 of 2 events should produce 8 events");

    // Events should repeat in pattern: a b a b a b a b
    assert_eq!(haps[0].value, "a");
    assert_eq!(haps[1].value, "b");
    assert_eq!(haps[2].value, "a");
    assert_eq!(haps[3].value, "b");
    assert_eq!(haps[4].value, "a");
    assert_eq!(haps[5].value, "b");
    assert_eq!(haps[6].value, "a");
    assert_eq!(haps[7].value, "b");

    // Each event should be shorter (0.5 / 4 = 0.125)
    for (i, hap) in haps.iter().enumerate() {
        let duration = hap.part.duration().to_float();
        assert!((duration - 0.125).abs() < 0.001,
            "Event {} should have duration 0.125, got {}", i, duration);
    }

    println!("✅ dup Level 1: Event timing and pattern verified");
}

#[test]
fn test_dup_equivalence_to_fast() {
    // dup n should produce identical results to fast n
    let pattern = parse_mini_notation("a b c d");
    let dup_pattern = pattern.clone().dup(3);
    let fast_pattern = pattern.clone().fast(3.0);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(2, 1)),
        controls: HashMap::new(),
    };

    let dup_haps = dup_pattern.query(&state);
    let fast_haps = fast_pattern.query(&state);

    assert_eq!(dup_haps.len(), fast_haps.len(),
        "dup and fast should produce same event count");

    // Verify values match
    for (i, (dup_hap, fast_hap)) in dup_haps.iter().zip(fast_haps.iter()).enumerate() {
        assert_eq!(dup_hap.value, fast_hap.value,
            "Event {} value mismatch: dup='{}', fast='{}'",
            i, dup_hap.value, fast_hap.value);
    }

    println!("✅ dup Level 1: dup 3 ≡ fast 3 (identical results)");
}

// ============================================================================
// LEVEL 2: Onset Detection (Audio Event Verification)
// ============================================================================

#[test]
fn test_dup_level2_audio_onsets() {
    let base_code = r#"
tempo: 0.5
out: s "bd sn hh cp"
"#;

    let dup_code = r#"
tempo: 0.5
out: s "bd sn hh cp" $ dup 3
"#;

    let cycles = 8;
    let base_audio = render_dsl(base_code, cycles);
    let dup_audio = render_dsl(dup_code, cycles);
    let sample_rate = 44100.0;

    // Detect audio onsets
    let base_onsets = detect_audio_events(&base_audio, sample_rate, 0.01);
    let dup_onsets = detect_audio_events(&dup_audio, sample_rate, 0.01);

    // dup 3 should roughly triple onset count
    let ratio = dup_onsets.len() as f32 / base_onsets.len() as f32;
    assert!(
        ratio > 2.1 && ratio < 3.9,
        "dup 3 should roughly triple onset count: base={}, dup={}, ratio={:.2}",
        base_onsets.len(), dup_onsets.len(), ratio
    );

    println!("✅ dup Level 2: Base onsets = {}, dup onsets = {}, ratio = {:.2}",
             base_onsets.len(), dup_onsets.len(), ratio);
}

#[test]
fn test_dup_level2_timing_compression() {
    // Verify that dup compresses timing correctly
    let code = r#"
tempo: 0.5
out: s "bd sn" $ dup 4
"#;

    let cycles = 4;
    let audio = render_dsl(code, cycles);
    let sample_rate = 44100.0;

    let onsets = detect_audio_events(&audio, sample_rate, 0.01);

    // Should have many events (onset detection may find more due to sample transients)
    // Just verify we have significantly more than base (8 events over 4 cycles)
    assert!(onsets.len() >= 16,
        "dup 4 should produce many events (got {})", onsets.len());

    // Check that events are evenly spaced (compressed timing)
    if onsets.len() >= 3 {
        let interval1 = onsets[1].time - onsets[0].time;
        let interval2 = onsets[2].time - onsets[1].time;

        // Intervals should be consistent (compressed)
        assert!(interval1 < 0.6 && interval2 < 0.6,
            "dup should compress event timing (intervals: {:.3}s, {:.3}s)",
            interval1, interval2);
    }

    println!("✅ dup Level 2: Timing compression verified, {} onsets detected", onsets.len());
}

// ============================================================================
// LEVEL 3: Audio Characteristics (Quality Checks)
// ============================================================================

#[test]
fn test_dup_level3_audio_quality() {
    let code = r#"
tempo: 0.5
out: s "bd sn hh cp" $ dup 3
"#;

    let audio = render_dsl(code, 8);

    // Calculate audio characteristics
    let rms = calculate_rms(&audio);
    let peak = audio.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    let dc_offset = audio.iter().sum::<f32>() / audio.len() as f32;

    // Verify audio quality
    assert!(rms > 0.01, "dup should produce audible audio (RMS = {})", rms);
    assert!(peak > 0.1, "dup should have audible peaks (peak = {})", peak);
    assert!(dc_offset.abs() < 0.1, "dup should not have excessive DC offset (DC = {})", dc_offset);

    println!("✅ dup Level 3: RMS = {:.4}, Peak = {:.4}, DC = {:.4}", rms, peak, dc_offset);
}

#[test]
fn test_dup_level3_compare_to_base() {
    // dup should have higher energy than base (more events)
    let base_code = r#"
tempo: 0.5
out: s "bd sn hh cp"
"#;

    let dup_code = r#"
tempo: 0.5
out: s "bd sn hh cp" $ dup 3
"#;

    let base_audio = render_dsl(base_code, 8);
    let dup_audio = render_dsl(dup_code, 8);

    let base_rms = calculate_rms(&base_audio);
    let dup_rms = calculate_rms(&dup_audio);

    // dup triples event count, but energy increase is less due to compression
    // Shorter events = less overlap, so energy increase is moderate
    let ratio = dup_rms / base_rms;
    assert!(
        ratio > 1.5 && ratio < 3.0,
        "dup energy should be 1.5-3x base: base RMS = {:.4}, dup RMS = {:.4}, ratio = {:.2}",
        base_rms, dup_rms, ratio
    );

    println!("✅ dup Level 3: Base RMS = {:.4}, dup RMS = {:.4}, ratio = {:.2}",
             base_rms, dup_rms, ratio);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_dup_with_dup_1() {
    // dup 1 should be identity (no change)
    let pattern = Pattern::from_string("a b c");
    let dup_pattern = pattern.clone().dup(1);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(2, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let dup_haps = dup_pattern.query(&state);

    assert_eq!(base_haps.len(), dup_haps.len(),
        "dup 1 should preserve event count");

    println!("✅ dup edge case: dup 1 behaves as identity");
}

#[test]
fn test_dup_with_dup_0() {
    // dup 0 should produce silence
    let pattern = Pattern::from_string("a b c");
    let dup_pattern = pattern.dup(0);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(2, 1)),
        controls: HashMap::new(),
    };

    let haps = dup_pattern.query(&state);

    assert_eq!(haps.len(), 0, "dup 0 should produce no events (silence)");

    println!("✅ dup edge case: dup 0 produces silence");
}

#[test]
fn test_dup_with_large_n() {
    // Test dup with large n
    let code = r#"
tempo: 0.5
out: s "bd sn" $ dup 16
"#;

    let audio = render_dsl(code, 4);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "dup with large n should still produce audio");

    println!("✅ dup edge case: dup 16 works correctly");
}

#[test]
fn test_dup_preserves_pattern_structure() {
    // Verify dup repeats the entire pattern
    let pattern = Pattern::from_string("a b c");
    let dup_pattern = pattern.dup(2);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let haps = dup_pattern.query(&state);

    // Should have 6 events (3 * 2)
    assert_eq!(haps.len(), 6, "Should have 6 events");

    // Should repeat: a b c a b c
    assert_eq!(haps[0].value, "a");
    assert_eq!(haps[1].value, "b");
    assert_eq!(haps[2].value, "c");
    assert_eq!(haps[3].value, "a");
    assert_eq!(haps[4].value, "b");
    assert_eq!(haps[5].value, "c");

    println!("✅ dup edge case: pattern structure preserved (a b c a b c)");
}
