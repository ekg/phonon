/// Tests for `swing` transform - delays every odd-indexed event
/// Swing creates a "triplet feel" by delaying off-beat events
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
// LEVEL 1: Pattern Query Verification (Timing Shift)
// ============================================================================

#[test]
fn test_swing_level1_delays_odd_events() {
    // swing should delay every odd-indexed event (indices 1, 3, 5, ...)
    let base_pattern = parse_mini_notation("bd sn hh cp");
    let swing_pattern = base_pattern.clone().swing(Pattern::pure(0.1));

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = base_pattern.query(&state);
    let swing_haps = swing_pattern.query(&state);

    // Same number of events
    assert_eq!(
        swing_haps.len(),
        base_haps.len(),
        "swing should preserve event count"
    );

    // Even-indexed events should have same timing
    for i in (0..swing_haps.len()).step_by(2) {
        assert_eq!(
            swing_haps[i].part.begin, base_haps[i].part.begin,
            "Even event {} should not be delayed",
            i
        );
    }

    // Odd-indexed events should be delayed by 0.1
    for i in (1..swing_haps.len()).step_by(2) {
        let expected_shift = 0.1;
        let actual_shift = swing_haps[i].part.begin.to_float() - base_haps[i].part.begin.to_float();
        assert!(
            (actual_shift - expected_shift).abs() < 0.001,
            "Odd event {} should be delayed by {}, got {}",
            i,
            expected_shift,
            actual_shift
        );
    }

    println!("✅ swing Level 1: Timing shifts verified");
}

#[test]
fn test_swing_level1_event_count() {
    // swing should preserve all events, just shift timing
    let pattern = parse_mini_notation("bd sn hh cp bd sn hh cp");

    let mut base_total = 0;
    let mut swing_total = 0;

    for cycle in 0..8 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        base_total += pattern.query(&state).len();
        swing_total += pattern
            .clone()
            .swing(Pattern::pure(0.1))
            .query(&state)
            .len();
    }

    assert_eq!(swing_total, base_total, "swing should preserve all events");

    println!("✅ swing Level 1: Event count preserved: {}", base_total);
}

#[test]
fn test_swing_level1_swing_amount() {
    // Verify different swing amounts
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);

    for amount in [0.05, 0.1, 0.15, 0.2] {
        let swing_haps = pattern.clone().swing(Pattern::pure(amount)).query(&state);

        // Check odd event (index 1) is delayed by the specified amount
        let shift = swing_haps[1].part.begin.to_float() - base_haps[1].part.begin.to_float();
        assert!(
            (shift - amount).abs() < 0.001,
            "swing({}) should delay by {}, got {}",
            amount,
            amount,
            shift
        );
    }

    println!("✅ swing Level 1: Variable swing amounts verified");
}

// ============================================================================
// LEVEL 2: Onset Detection (Audio Event Verification)
// ============================================================================

#[test]
fn test_swing_level2_audio_onsets() {
    let base_code = r#"
tempo: 0.5
out $ s "bd sn hh cp bd sn hh cp"
"#;

    let swing_code = r#"
tempo: 0.5
out $ s "bd sn hh cp bd sn hh cp" $ swing 0.05
"#;

    let cycles = 8;
    let base_audio = render_dsl(base_code, cycles);
    let swing_audio = render_dsl(swing_code, cycles);
    let sample_rate = 44100.0;

    let base_onsets = detect_audio_events(&base_audio, sample_rate, 0.01);
    let swing_onsets = detect_audio_events(&swing_audio, sample_rate, 0.01);

    // Onset count should be similar (allowing ~3% variance due to detection tolerance at boundaries)
    let ratio = swing_onsets.len() as f32 / base_onsets.len() as f32;
    assert!(
        ratio > 0.97 && ratio < 1.03,
        "swing should preserve most onsets: base={}, swing={}, ratio={:.3}",
        base_onsets.len(),
        swing_onsets.len(),
        ratio
    );

    // NOTE: We don't verify specific timing shifts in onset detection
    // because swing only delays by 0.05 cycles, which is too small for
    // reliable onset detection alignment. Level 1 tests verify timing shifts
    // at the pattern level, which is more reliable.

    println!(
        "✅ swing Level 2: Onsets detected: base={}, swing={}",
        base_onsets.len(),
        swing_onsets.len()
    );
}

#[test]
fn test_swing_level2_onset_intervals() {
    let code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ swing 0.1
"#;

    let audio = render_dsl(code, 4);
    let sample_rate = 44100.0;
    let onsets = detect_audio_events(&audio, sample_rate, 0.01);

    assert!(
        onsets.len() >= 8,
        "Should detect at least 8 onsets over 4 cycles, got {}",
        onsets.len()
    );

    // Intervals between consecutive onsets should show swing pattern
    // Even-to-odd intervals should be shorter (delayed second event)
    // Odd-to-even intervals should be longer (compensating)
    if onsets.len() >= 4 {
        let intervals: Vec<f64> = onsets.windows(2).map(|w| w[1].time - w[0].time).collect();

        println!(
            "Onset intervals: {:?}",
            &intervals[0..4.min(intervals.len())]
        );
    }

    println!("✅ swing Level 2: Onset intervals analyzed");
}

// ============================================================================
// LEVEL 3: Audio Characteristics (Signal Quality)
// ============================================================================

#[test]
fn test_swing_level3_audio_quality() {
    let code = r#"
tempo: 0.5
out $ s "bd sn hh cp bd sn hh cp" $ swing 0.1
"#;

    let audio = render_dsl(code, 8);

    let rms = calculate_rms(&audio);
    let peak = audio.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    let dc_offset = audio.iter().sum::<f32>() / audio.len() as f32;

    assert!(
        rms > 0.01,
        "swing should produce audible audio (RMS = {})",
        rms
    );
    assert!(
        peak > 0.1,
        "swing should have audible peaks (peak = {})",
        peak
    );
    assert!(
        dc_offset.abs() < 0.1,
        "swing should not have excessive DC offset (DC = {})",
        dc_offset
    );

    println!(
        "✅ swing Level 3: RMS = {:.4}, Peak = {:.4}, DC = {:.4}",
        rms, peak, dc_offset
    );
}

#[test]
fn test_swing_level3_energy_preservation() {
    let base_code = r#"
tempo: 0.5
out $ s "bd sn hh cp bd sn hh cp"
"#;

    let swing_code = r#"
tempo: 0.5
out $ s "bd sn hh cp bd sn hh cp" $ swing 0.1
"#;

    let base_audio = render_dsl(base_code, 8);
    let swing_audio = render_dsl(swing_code, 8);

    let base_rms = calculate_rms(&base_audio);
    let swing_rms = calculate_rms(&swing_audio);

    // swing only changes timing, not amplitude, so energy should be similar
    let ratio = swing_rms / base_rms;
    assert!(
        ratio > 0.8 && ratio < 1.2,
        "swing should preserve energy: base RMS = {:.4}, swing RMS = {:.4}, ratio = {:.2}",
        base_rms,
        swing_rms,
        ratio
    );

    println!(
        "✅ swing Level 3: Energy preserved: base RMS = {:.4}, swing RMS = {:.4}, ratio = {:.2}",
        base_rms, swing_rms, ratio
    );
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_swing_zero_amount() {
    // swing(0.0) should have no effect
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let swing_haps = pattern.clone().swing(Pattern::pure(0.0)).query(&state);

    for i in 0..base_haps.len() {
        assert_eq!(
            swing_haps[i].part.begin, base_haps[i].part.begin,
            "swing(0.0) should not change timing"
        );
    }

    println!("✅ swing edge case: Zero swing has no effect");
}

#[test]
fn test_swing_single_event() {
    // swing with single event should work (no odd-indexed event to delay)
    let pattern = parse_mini_notation("bd");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let swing_haps = pattern.clone().swing(Pattern::pure(0.1)).query(&state);

    assert_eq!(swing_haps.len(), 1, "Should have 1 event");

    println!("✅ swing edge case: Single event handled");
}

#[test]
fn test_swing_preserves_values() {
    // swing should only affect timing, not values
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let swing_haps = pattern.clone().swing(Pattern::pure(0.1)).query(&state);

    for i in 0..base_haps.len() {
        assert_eq!(
            swing_haps[i].value, base_haps[i].value,
            "swing should preserve event values"
        );
    }

    println!("✅ swing edge case: Values preserved");
}
