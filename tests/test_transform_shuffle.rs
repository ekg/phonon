/// Tests for `shuffle` transform - randomizes event timing within a range
/// Shuffle adds humanization/groove by shifting each event randomly
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use std::collections::HashMap;

mod audio_test_utils;
mod pattern_verification_utils;
use audio_test_utils::calculate_rms;
use pattern_verification_utils::detect_audio_events;

fn render_dsl(code: &str, cycles: usize) -> Vec<f32> {
    let (_, statements) = parse_program(code).expect("Parse failed");
    let sample_rate = 44100.0;
    let mut graph = compile_program(statements, sample_rate, None).expect("Compile failed");
    let samples_per_cycle = (sample_rate as f64 / 0.5) as usize;
    let total_samples = samples_per_cycle * cycles;
    graph.render(total_samples)
}

// ============================================================================
// LEVEL 1: Pattern Query Verification (Timing Randomization)
// ============================================================================

#[test]
fn test_shuffle_level1_shifts_timing() {
    // shuffle should shift each event's timing randomly
    let base_pattern = parse_mini_notation("bd sn hh cp");
    let shuffle_pattern = base_pattern.clone().shuffle(Pattern::pure(0.05));

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = base_pattern.query(&state);
    let shuffle_haps = shuffle_pattern.query(&state);

    // Same number of events
    assert_eq!(
        shuffle_haps.len(),
        base_haps.len(),
        "shuffle should preserve event count"
    );

    // At least some events should have different timing
    let mut timing_changed = 0;
    for i in 0..shuffle_haps.len() {
        if (shuffle_haps[i].part.begin.to_float() - base_haps[i].part.begin.to_float()).abs()
            > 0.0001
        {
            timing_changed += 1;
        }
    }

    assert!(
        timing_changed > 0,
        "shuffle should change timing of at least some events"
    );

    println!(
        "✅ shuffle Level 1: {}/{} events had timing changed",
        timing_changed,
        shuffle_haps.len()
    );
}

#[test]
fn test_shuffle_level1_event_count() {
    // shuffle should preserve all events, just shift timing
    let pattern = parse_mini_notation("bd sn hh cp bd sn hh cp");

    let mut base_total = 0;
    let mut shuffle_total = 0;

    for cycle in 0..8 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        base_total += pattern.query(&state).len();
        shuffle_total += pattern
            .clone()
            .shuffle(Pattern::pure(0.05))
            .query(&state)
            .len();
    }

    assert_eq!(
        shuffle_total, base_total,
        "shuffle should preserve all events"
    );

    println!("✅ shuffle Level 1: Event count preserved: {}", base_total);
}

#[test]
fn test_shuffle_level1_deterministic() {
    // shuffle should be deterministic (same random seed per cycle)
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let shuffle_pattern = pattern.clone().shuffle(Pattern::pure(0.1));
    let haps1 = shuffle_pattern.query(&state);
    let haps2 = shuffle_pattern.query(&state);

    assert_eq!(haps1.len(), haps2.len(), "Should have same event count");

    for i in 0..haps1.len() {
        assert_eq!(
            haps1[i].part.begin, haps2[i].part.begin,
            "shuffle should be deterministic for same query"
        );
    }

    println!("✅ shuffle Level 1: Deterministic behavior verified");
}

#[test]
fn test_shuffle_level1_timing_bounds() {
    // Verify timing shifts stay within specified range
    let pattern = parse_mini_notation("bd sn hh cp");
    let amount = 0.1;

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let shuffle_haps = pattern.clone().shuffle(Pattern::pure(amount)).query(&state);

    for i in 0..shuffle_haps.len() {
        let shift = shuffle_haps[i].part.begin.to_float() - base_haps[i].part.begin.to_float();
        assert!(
            shift >= -amount && shift <= amount,
            "Shift {} should be within +/- {} range",
            shift,
            amount
        );
    }

    println!("✅ shuffle Level 1: Timing shifts within bounds");
}

// ============================================================================
// LEVEL 2: Onset Detection (Audio Event Verification)
// ============================================================================

#[test]
fn test_shuffle_level2_audio_onsets() {
    let base_code = r#"
tempo: 0.5
out $ s "bd sn hh cp bd sn hh cp"
"#;

    let shuffle_code = r#"
tempo: 0.5
out $ s "bd sn hh cp bd sn hh cp" $ shuffle 0.02
"#;

    let cycles = 8;
    let base_audio = render_dsl(base_code, cycles);
    let shuffle_audio = render_dsl(shuffle_code, cycles);
    let sample_rate = 44100.0;

    let base_onsets = detect_audio_events(&base_audio, sample_rate, 0.01);
    let shuffle_onsets = detect_audio_events(&shuffle_audio, sample_rate, 0.01);

    // Onset count should be similar (allowing ~5% variance)
    let ratio = shuffle_onsets.len() as f32 / base_onsets.len() as f32;
    assert!(
        ratio > 0.95 && ratio < 1.05,
        "shuffle should preserve most onsets: base={}, shuffle={}, ratio={:.3}",
        base_onsets.len(),
        shuffle_onsets.len(),
        ratio
    );

    println!(
        "✅ shuffle Level 2: Onsets detected: base={}, shuffle={}",
        base_onsets.len(),
        shuffle_onsets.len()
    );
}

#[test]
fn test_shuffle_level2_timing_spread() {
    let code = r#"
tempo: 0.5
out $ s "bd bd bd bd" $ shuffle 0.05
"#;

    let audio = render_dsl(code, 4);
    let sample_rate = 44100.0;
    let onsets = detect_audio_events(&audio, sample_rate, 0.01);

    // Should detect a reasonable number of onsets (exact count varies with sample playback)
    assert!(
        onsets.len() > 10,
        "Should detect onsets, got {}",
        onsets.len()
    );

    // Check that intervals vary (not all exactly equal)
    if onsets.len() >= 4 {
        let intervals: Vec<f64> = onsets.windows(2).map(|w| w[1].time - w[0].time).collect();

        let first_interval = intervals[0];
        let has_variation = intervals.iter().any(|&i| (i - first_interval).abs() > 0.01);

        assert!(
            has_variation,
            "shuffle should create variation in intervals"
        );
    }

    println!(
        "✅ shuffle Level 2: Timing variation detected ({} onsets)",
        onsets.len()
    );
}

// ============================================================================
// LEVEL 3: Audio Characteristics (Signal Quality)
// ============================================================================

#[test]
fn test_shuffle_level3_audio_quality() {
    let code = r#"
tempo: 0.5
out $ s "bd sn hh cp bd sn hh cp" $ shuffle 0.05
"#;

    let audio = render_dsl(code, 8);

    let rms = calculate_rms(&audio);
    let peak = audio.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    let dc_offset = audio.iter().sum::<f32>() / audio.len() as f32;

    assert!(
        rms > 0.01,
        "shuffle should produce audible audio (RMS = {})",
        rms
    );
    assert!(
        peak > 0.1,
        "shuffle should have audible peaks (peak = {})",
        peak
    );
    assert!(
        dc_offset.abs() < 0.1,
        "shuffle should not have excessive DC offset (DC = {})",
        dc_offset
    );

    println!(
        "✅ shuffle Level 3: RMS = {:.4}, Peak = {:.4}, DC = {:.4}",
        rms, peak, dc_offset
    );
}

#[test]
fn test_shuffle_level3_energy_preservation() {
    let base_code = r#"
tempo: 0.5
out $ s "bd sn hh cp bd sn hh cp"
"#;

    let shuffle_code = r#"
tempo: 0.5
out $ s "bd sn hh cp bd sn hh cp" $ shuffle 0.05
"#;

    let base_audio = render_dsl(base_code, 8);
    let shuffle_audio = render_dsl(shuffle_code, 8);

    let base_rms = calculate_rms(&base_audio);
    let shuffle_rms = calculate_rms(&shuffle_audio);

    // shuffle only changes timing, not amplitude, so energy should be similar
    let ratio = shuffle_rms / base_rms;
    assert!(
        ratio > 0.8 && ratio < 1.2,
        "shuffle should preserve energy: base RMS = {:.4}, shuffle RMS = {:.4}, ratio = {:.2}",
        base_rms,
        shuffle_rms,
        ratio
    );

    println!(
        "✅ shuffle Level 3: Energy preserved: base RMS = {:.4}, shuffle RMS = {:.4}, ratio = {:.2}",
        base_rms, shuffle_rms, ratio
    );
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_shuffle_zero_amount() {
    // shuffle(0.0) should have minimal effect (RNG still runs)
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let shuffle_haps = pattern.clone().shuffle(Pattern::pure(0.0)).query(&state);

    // With zero range, all shifts should be 0.0
    for i in 0..base_haps.len() {
        let shift = shuffle_haps[i].part.begin.to_float() - base_haps[i].part.begin.to_float();
        assert!(
            shift.abs() < 0.0001,
            "shuffle(0.0) should not change timing significantly"
        );
    }

    println!("✅ shuffle edge case: Zero shuffle has minimal effect");
}

#[test]
fn test_shuffle_single_event() {
    // shuffle with single event should work
    let pattern = parse_mini_notation("bd");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let shuffle_haps = pattern.clone().shuffle(Pattern::pure(0.1)).query(&state);

    assert_eq!(shuffle_haps.len(), 1, "Should have 1 event");

    println!("✅ shuffle edge case: Single event handled");
}

#[test]
fn test_shuffle_preserves_values() {
    // shuffle should only affect timing, not values
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let shuffle_haps = pattern.clone().shuffle(Pattern::pure(0.1)).query(&state);

    for i in 0..base_haps.len() {
        assert_eq!(
            shuffle_haps[i].value, base_haps[i].value,
            "shuffle should preserve event values"
        );
    }

    println!("✅ shuffle edge case: Values preserved");
}

#[test]
fn test_shuffle_different_per_cycle() {
    // shuffle should produce different results for different cycles
    let pattern = parse_mini_notation("bd sn hh cp");

    let state1 = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let state2 = State {
        span: TimeSpan::new(Fraction::new(1, 1), Fraction::new(2, 1)),
        controls: HashMap::new(),
    };

    let shuffle_pattern = pattern.clone().shuffle(Pattern::pure(0.1));
    let haps1 = shuffle_pattern.query(&state1);
    let haps2 = shuffle_pattern.query(&state2);

    // Events should have different timing in different cycles
    let mut timing_differs = false;
    for i in 0..haps1.len().min(haps2.len()) {
        let time1 = haps1[i].part.begin.to_float() % 1.0;
        let time2 = haps2[i].part.begin.to_float() % 1.0;
        if (time1 - time2).abs() > 0.001 {
            timing_differs = true;
            break;
        }
    }

    assert!(
        timing_differs,
        "shuffle should produce different timing across cycles"
    );

    println!("✅ shuffle edge case: Different timing per cycle");
}
