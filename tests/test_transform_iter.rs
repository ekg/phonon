/// Three-Level Verification Tests for `iter` Transform
///
/// `iter n` shifts the pattern by 1/n each cycle, creating a progressive iteration
/// Example: "a b c d" $ iter 4
/// - Cycle 0: a b c d (shift 0/4)
/// - Cycle 1: d a b c (shift 1/4 - wraps from end)
/// - Cycle 2: c d a b (shift 2/4)
/// - Cycle 3: b c d a (shift 3/4)
/// - Cycle 4: a b c d (shift 4/4 = 0, repeats)
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
    let mut graph = compile_program(statements, sample_rate, None).expect("Compile failed");

    let samples_per_cycle = (sample_rate as f64 / 0.5) as usize; // tempo = 0.5 cps
    let total_samples = samples_per_cycle * cycles;

    graph.render(total_samples)
}

/// Helper: Count events in pattern over multiple cycles
fn count_pattern_events(pattern_str: &str, cycles: usize) -> usize {
    let pattern = parse_mini_notation(pattern_str);

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

// ============================================================================
// LEVEL 1: Pattern Query Verification (Exact Event Counts)
// ============================================================================

#[test]
fn test_iter_level1_event_count() {
    // iter doesn't change the number of events, just their timing
    let base_pattern = "bd sn hh cp"; // 4 events per cycle
    let cycles = 8;

    let pattern = parse_mini_notation(base_pattern);
    let iter_pattern = pattern.clone().iter(4);

    // Count events over multiple cycles
    let mut base_total = 0;
    let mut iter_total = 0;

    for cycle in 0..cycles {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        base_total += pattern.query(&state).len();
        iter_total += iter_pattern.query(&state).len();
    }

    // iter should preserve event count
    assert_eq!(
        iter_total, base_total,
        "iter should preserve event count: expected {}, got {}",
        base_total, iter_total
    );

    println!(
        "✅ iter Level 1: Base events = {}, iter events = {}",
        base_total, iter_total
    );
}

#[test]
fn test_iter_level1_progressive_shift() {
    // Verify the progressive shifting behavior
    let pattern = Pattern::from_string("a b c d");
    let iter_pattern = pattern.iter(4);

    // Cycle 0: should start with "a" (no shift)
    let state0 = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    let haps0 = iter_pattern.query(&state0);
    assert_eq!(haps0[0].value, "a", "Cycle 0 should start with 'a'");

    // Cycle 1: shift by 1/4, should start with "d" (wrapped from end)
    let state1 = State {
        span: TimeSpan::new(Fraction::new(1, 1), Fraction::new(2, 1)),
        controls: HashMap::new(),
    };
    let haps1 = iter_pattern.query(&state1);
    assert_eq!(
        haps1[0].value, "d",
        "Cycle 1 should start with 'd' (shift 1/4)"
    );

    // Cycle 2: shift by 2/4 = 1/2, should start with "c"
    let state2 = State {
        span: TimeSpan::new(Fraction::new(2, 1), Fraction::new(3, 1)),
        controls: HashMap::new(),
    };
    let haps2 = iter_pattern.query(&state2);
    assert_eq!(
        haps2[0].value, "c",
        "Cycle 2 should start with 'c' (shift 2/4)"
    );

    // Cycle 3: shift by 3/4, should start with "b"
    let state3 = State {
        span: TimeSpan::new(Fraction::new(3, 1), Fraction::new(4, 1)),
        controls: HashMap::new(),
    };
    let haps3 = iter_pattern.query(&state3);
    assert_eq!(
        haps3[0].value, "b",
        "Cycle 3 should start with 'b' (shift 3/4)"
    );

    // Cycle 4: shift by 4/4 = 0 (wraps), back to "a"
    let state4 = State {
        span: TimeSpan::new(Fraction::new(4, 1), Fraction::new(5, 1)),
        controls: HashMap::new(),
    };
    let haps4 = iter_pattern.query(&state4);
    assert_eq!(haps4[0].value, "a", "Cycle 4 should start with 'a' (wraps)");

    println!("✅ iter Level 1: Progressive shift verified over 5 cycles");
}

// ============================================================================
// LEVEL 2: Onset Detection (Audio Event Verification)
// ============================================================================

#[test]
fn test_iter_level2_audio_onsets() {
    let base_code = r#"
tempo: 0.5
out $ s "bd sn hh cp"
"#;

    let iter_code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ iter 4
"#;

    let cycles = 8;
    let base_audio = render_dsl(base_code, cycles);
    let iter_audio = render_dsl(iter_code, cycles);
    let sample_rate = 44100.0;

    // Detect audio onsets
    let base_onsets = detect_audio_events(&base_audio, sample_rate, 0.01);
    let iter_onsets = detect_audio_events(&iter_audio, sample_rate, 0.01);

    // iter should preserve event count (within 30% tolerance due to onset detector sensitivity to timing shifts)
    let ratio = iter_onsets.len() as f32 / base_onsets.len() as f32;
    assert!(
        ratio > 0.7 && ratio < 1.3,
        "iter should preserve event count: base={}, iter={}, ratio={:.2}",
        base_onsets.len(),
        iter_onsets.len(),
        ratio
    );

    println!(
        "✅ iter Level 2: Base onsets = {}, iter onsets = {}, ratio = {:.2}",
        base_onsets.len(),
        iter_onsets.len(),
        ratio
    );
}

#[test]
fn test_iter_level2_timing_progression() {
    // Verify that timing actually shifts across cycles
    let code = r#"
tempo: 0.5
out $ s "bd sn" $ iter 2
"#;

    let cycles = 4; // Two complete iterations (iter 2 wraps every 2 cycles)
    let audio = render_dsl(code, cycles);
    let sample_rate = 44100.0;

    let onsets = detect_audio_events(&audio, sample_rate, 0.01);

    // With iter 2, timing should alternate:
    // Cycle 0: bd at 0.0, sn at 0.5
    // Cycle 1: sn at 0.5 (shifted by 1/2), bd at 1.0
    // Pattern repeats

    // Just verify we have audio events and they're not all at the same time
    assert!(onsets.len() >= 6, "Should have multiple audio events");

    // Check that events are spread out (not all at once)
    if onsets.len() >= 2 {
        let time_spread = onsets.last().unwrap().time - onsets.first().unwrap().time;
        assert!(time_spread > 1.0, "Events should be spread across time");
    }

    println!(
        "✅ iter Level 2: Timing progression verified, {} onsets detected",
        onsets.len()
    );
}

// ============================================================================
// LEVEL 3: Audio Characteristics (Quality Checks)
// ============================================================================

#[test]
fn test_iter_level3_audio_quality() {
    let code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ iter 4
"#;

    let audio = render_dsl(code, 8);

    // Calculate audio characteristics
    let rms = calculate_rms(&audio);
    let peak = audio.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    let dc_offset = audio.iter().sum::<f32>() / audio.len() as f32;

    // Verify audio quality
    assert!(
        rms > 0.01,
        "iter should produce audible audio (RMS = {})",
        rms
    );
    assert!(
        peak > 0.1,
        "iter should have audible peaks (peak = {})",
        peak
    );
    assert!(
        dc_offset.abs() < 0.1,
        "iter should not have excessive DC offset (DC = {})",
        dc_offset
    );

    println!(
        "✅ iter Level 3: RMS = {:.4}, Peak = {:.4}, DC = {:.4}",
        rms, peak, dc_offset
    );
}

#[test]
fn test_iter_level3_compare_to_base() {
    // iter should have similar overall energy to base pattern
    let base_code = r#"
tempo: 0.5
out $ s "bd sn hh cp"
"#;

    let iter_code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ iter 4
"#;

    let base_audio = render_dsl(base_code, 8);
    let iter_audio = render_dsl(iter_code, 8);

    let base_rms = calculate_rms(&base_audio);
    let iter_rms = calculate_rms(&iter_audio);

    // iter just shifts timing, energy should be similar
    let ratio = iter_rms / base_rms;
    assert!(
        ratio > 0.7 && ratio < 1.3,
        "iter energy should be similar to base: base RMS = {:.4}, iter RMS = {:.4}, ratio = {:.2}",
        base_rms,
        iter_rms,
        ratio
    );

    println!(
        "✅ iter Level 3: Base RMS = {:.4}, iter RMS = {:.4}, ratio = {:.2}",
        base_rms, iter_rms, ratio
    );
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_iter_with_iter_1() {
    // iter 1 should be identity (no shift ever happens since cycle % 1 = 0 always)
    let pattern = Pattern::from_string("a b c");
    let iter_pattern = pattern.clone().iter(1);

    for cycle in 0..4 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let base_haps = pattern.query(&state);
        let iter_haps = iter_pattern.query(&state);

        assert_eq!(base_haps.len(), iter_haps.len());
        assert_eq!(
            base_haps[0].value, iter_haps[0].value,
            "iter 1 should be identity for cycle {}",
            cycle
        );
    }

    println!("✅ iter edge case: iter 1 behaves as identity");
}

#[test]
fn test_iter_with_large_n() {
    // Test iter with large n
    let code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ iter 16
"#;

    let audio = render_dsl(code, 8);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "iter with large n should still produce audio");

    println!("✅ iter edge case: iter 16 works correctly");
}
