/// Combined tests for `early`, `late`, and `offset` - time shifting transforms
/// - late: shifts events forward in time
/// - early: shifts events backward in time (alias for late with negative amount)
/// - offset: alias for late
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
// LEVEL 1: Pattern Query Verification (Timing Shifts)
// ============================================================================

#[test]
fn test_late_level1_shifts_forward() {
    // late should shift all events forward in time
    let base_pattern = parse_mini_notation("bd sn hh cp");
    let late_pattern = base_pattern.clone().late(Pattern::pure(0.1));

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = base_pattern.query(&state);
    let late_haps = late_pattern.query(&state);

    // Same number of events
    assert_eq!(
        late_haps.len(),
        base_haps.len(),
        "late should preserve event count"
    );

    // Each event should be shifted forward by 0.1
    for i in 0..late_haps.len() {
        let base_begin = base_haps[i].part.begin.to_float();
        let late_begin = late_haps[i].part.begin.to_float();
        let shift = late_begin - base_begin;

        assert!(
            (shift - 0.1).abs() < 0.001,
            "Event {} should be shifted forward by 0.1, got shift of {}",
            i,
            shift
        );
    }

    println!("✅ late Level 1: All events shifted forward by 0.1");
}

#[test]
fn test_early_level1_shifts_backward() {
    // early should shift all events backward in time
    let base_pattern = parse_mini_notation("bd sn hh cp");
    let early_pattern = base_pattern.clone().early(Pattern::pure(0.1));

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = base_pattern.query(&state);
    let early_haps = early_pattern.query(&state);

    // Same number of events
    assert_eq!(
        early_haps.len(),
        base_haps.len(),
        "early should preserve event count"
    );

    // Each event should be shifted backward by 0.1
    for i in 0..early_haps.len() {
        let base_begin = base_haps[i].part.begin.to_float();
        let early_begin = early_haps[i].part.begin.to_float();
        let shift = early_begin - base_begin;

        assert!(
            (shift + 0.1).abs() < 0.001,
            "Event {} should be shifted backward by 0.1, got shift of {}",
            i,
            shift
        );
    }

    println!("✅ early Level 1: All events shifted backward by 0.1");
}

#[test]
fn test_offset_level1_alias_for_late() {
    // offset should behave identically to late
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let late_haps = pattern.clone().late(Pattern::pure(0.15)).query(&state);
    let offset_haps = pattern.clone().offset(0.15).query(&state);

    assert_eq!(late_haps.len(), offset_haps.len(), "Same event count");

    for i in 0..late_haps.len() {
        assert_eq!(
            late_haps[i].part.begin, offset_haps[i].part.begin,
            "offset and late should produce identical timing"
        );
        assert_eq!(
            late_haps[i].part.end, offset_haps[i].part.end,
            "offset and late should produce identical timing"
        );
    }

    println!("✅ offset Level 1: Verified as alias for late");
}

#[test]
fn test_late_level1_preserves_duration() {
    // late should only shift timing, not change duration
    let base_pattern = parse_mini_notation("bd sn hh cp");
    let late_pattern = base_pattern.clone().late(Pattern::pure(0.2));

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = base_pattern.query(&state);
    let late_haps = late_pattern.query(&state);

    for i in 0..late_haps.len() {
        let base_duration = base_haps[i].part.duration().to_float();
        let late_duration = late_haps[i].part.duration().to_float();

        assert!(
            (late_duration - base_duration).abs() < 0.001,
            "Event {} duration should be preserved",
            i
        );
    }

    println!("✅ late Level 1: Duration preserved");
}

#[test]
fn test_early_level1_event_count() {
    // early should preserve all events over multiple cycles
    let pattern = parse_mini_notation("bd sn hh cp bd sn hh cp");

    let mut base_total = 0;
    let mut early_total = 0;

    for cycle in 0..8 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        base_total += pattern.query(&state).len();
        early_total += pattern
            .clone()
            .early(Pattern::pure(0.05))
            .query(&state)
            .len();
    }

    assert_eq!(early_total, base_total, "early should preserve all events");

    println!("✅ early Level 1: Event count preserved: {}", base_total);
}

#[test]
fn test_late_level1_event_count() {
    // late should preserve all events over multiple cycles
    let pattern = parse_mini_notation("bd sn hh cp bd sn hh cp");

    let mut base_total = 0;
    let mut late_total = 0;

    for cycle in 0..8 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        base_total += pattern.query(&state).len();
        late_total += pattern
            .clone()
            .late(Pattern::pure(0.05))
            .query(&state)
            .len();
    }

    assert_eq!(late_total, base_total, "late should preserve all events");

    println!("✅ late Level 1: Event count preserved: {}", base_total);
}

#[test]
fn test_early_late_inverse() {
    // early(x) should be inverse of late(x)
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let shifted = pattern
        .clone()
        .late(Pattern::pure(0.1))
        .early(Pattern::pure(0.1))
        .query(&state);

    assert_eq!(base_haps.len(), shifted.len());

    for i in 0..base_haps.len() {
        let base_time = base_haps[i].part.begin.to_float();
        let shifted_time = shifted[i].part.begin.to_float();

        assert!(
            (shifted_time - base_time).abs() < 0.001,
            "late then early should return to original timing"
        );
    }

    println!("✅ early/late Level 1: Verified as inverse operations");
}

#[test]
fn test_late_level1_preserves_values() {
    // late should only affect timing, not values
    let base_pattern = parse_mini_notation("bd sn hh cp");
    let late_pattern = base_pattern.clone().late(Pattern::pure(0.1));

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = base_pattern.query(&state);
    let late_haps = late_pattern.query(&state);

    for i in 0..base_haps.len() {
        assert_eq!(
            late_haps[i].value, base_haps[i].value,
            "late should preserve event values"
        );
    }

    println!("✅ late Level 1: Values preserved");
}

// ============================================================================
// LEVEL 2: Onset Detection (Audio Event Verification)
// ============================================================================

#[test]
fn test_late_level2_audio_onsets() {
    let base_code = r#"
tempo: 0.5
out $ s "bd sn hh cp bd sn hh cp"
"#;

    let late_code = r#"
tempo: 0.5
out $ s "bd sn hh cp bd sn hh cp" $ late 0.05
"#;

    let cycles = 8;
    let base_audio = render_dsl(base_code, cycles);
    let late_audio = render_dsl(late_code, cycles);
    let sample_rate = 44100.0;

    let base_onsets = detect_audio_events(&base_audio, sample_rate, 0.01);
    let late_onsets = detect_audio_events(&late_audio, sample_rate, 0.01);

    // Onset count should be similar (allowing ~5% variance)
    let ratio = late_onsets.len() as f32 / base_onsets.len() as f32;
    assert!(
        ratio > 0.95 && ratio < 1.05,
        "late should preserve most onsets: base={}, late={}, ratio={:.3}",
        base_onsets.len(),
        late_onsets.len(),
        ratio
    );

    println!(
        "✅ late Level 2: Onsets detected: base={}, late={}",
        base_onsets.len(),
        late_onsets.len()
    );
}

#[test]
fn test_early_level2_audio_onsets() {
    let base_code = r#"
tempo: 0.5
out $ s "bd sn hh cp"
"#;

    let early_code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ early 0.05
"#;

    let cycles = 8;
    let base_audio = render_dsl(base_code, cycles);
    let early_audio = render_dsl(early_code, cycles);
    let sample_rate = 44100.0;

    let base_onsets = detect_audio_events(&base_audio, sample_rate, 0.01);
    let early_onsets = detect_audio_events(&early_audio, sample_rate, 0.01);

    // Onset count should be similar (allowing ~5% variance)
    let ratio = early_onsets.len() as f32 / base_onsets.len() as f32;
    assert!(
        ratio > 0.95 && ratio < 1.05,
        "early should preserve most onsets: base={}, early={}, ratio={:.3}",
        base_onsets.len(),
        early_onsets.len(),
        ratio
    );

    println!(
        "✅ early Level 2: Onsets detected: base={}, early={}",
        base_onsets.len(),
        early_onsets.len()
    );
}

#[test]
fn test_late_level2_timing_shift() {
    // Verify that late actually shifts onset timing forward
    let base_code = r#"
tempo: 0.5
out $ s "bd ~ ~ ~"
"#;

    let late_code = r#"
tempo: 0.5
out $ s "bd ~ ~ ~" $ late 0.25
"#;

    let cycles = 4;
    let base_audio = render_dsl(base_code, cycles);
    let late_audio = render_dsl(late_code, cycles);
    let sample_rate = 44100.0;

    let base_onsets = detect_audio_events(&base_audio, sample_rate, 0.01);
    let late_onsets = detect_audio_events(&late_audio, sample_rate, 0.01);

    // Should have similar number of onsets
    assert!(
        base_onsets.len() > 0 && late_onsets.len() > 0,
        "Should detect onsets in both patterns"
    );

    // First onset in late should be later than first onset in base
    if base_onsets.len() > 0 && late_onsets.len() > 0 {
        let base_first = base_onsets[0].time;
        let late_first = late_onsets[0].time;

        // late(0.25) with tempo 0.5 means 0.25 cycles = 0.5 seconds
        let expected_shift = 0.5; // seconds
        let actual_shift = late_first - base_first;

        assert!(
            (actual_shift - expected_shift).abs() < 0.1,
            "First onset should be shifted by ~{} seconds, got {}",
            expected_shift,
            actual_shift
        );
    }

    println!("✅ late Level 2: Timing shift verified");
}

#[test]
fn test_early_level2_timing_shift() {
    // Verify that early actually shifts onset timing backward
    let base_code = r#"
tempo: 0.5
out $ s "~ bd ~ ~"
"#;

    let early_code = r#"
tempo: 0.5
out $ s "~ bd ~ ~" $ early 0.25
"#;

    let cycles = 4;
    let base_audio = render_dsl(base_code, cycles);
    let early_audio = render_dsl(early_code, cycles);
    let sample_rate = 44100.0;

    let base_onsets = detect_audio_events(&base_audio, sample_rate, 0.01);
    let early_onsets = detect_audio_events(&early_audio, sample_rate, 0.01);

    // Should have similar number of onsets
    assert!(
        base_onsets.len() > 0 && early_onsets.len() > 0,
        "Should detect onsets in both patterns"
    );

    // First onset in early should be earlier than first onset in base
    if base_onsets.len() > 0 && early_onsets.len() > 0 {
        let base_first = base_onsets[0].time;
        let early_first = early_onsets[0].time;

        // early(0.25) with tempo 0.5 means 0.25 cycles = -0.5 seconds
        let expected_shift = -0.5; // seconds
        let actual_shift = early_first - base_first;

        assert!(
            (actual_shift - expected_shift).abs() < 0.1,
            "First onset should be shifted by ~{} seconds, got {}",
            expected_shift,
            actual_shift
        );
    }

    println!("✅ early Level 2: Timing shift verified");
}

// ============================================================================
// LEVEL 3: Audio Characteristics (Signal Quality)
// ============================================================================

#[test]
fn test_late_level3_audio_quality() {
    let code = r#"
tempo: 0.5
out $ s "bd sn hh cp bd sn hh cp" $ late 0.1
"#;

    let audio = render_dsl(code, 8);

    let rms = calculate_rms(&audio);
    let peak = audio.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    let dc_offset = audio.iter().sum::<f32>() / audio.len() as f32;

    assert!(
        rms > 0.01,
        "late should produce audible audio (RMS = {})",
        rms
    );
    assert!(
        peak > 0.1,
        "late should have audible peaks (peak = {})",
        peak
    );
    assert!(
        dc_offset.abs() < 0.1,
        "late should not have excessive DC offset (DC = {})",
        dc_offset
    );

    println!(
        "✅ late Level 3: RMS = {:.4}, Peak = {:.4}, DC = {:.4}",
        rms, peak, dc_offset
    );
}

#[test]
fn test_early_level3_audio_quality() {
    let code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ early 0.05
"#;

    let audio = render_dsl(code, 8);

    let rms = calculate_rms(&audio);
    let peak = audio.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    let dc_offset = audio.iter().sum::<f32>() / audio.len() as f32;

    assert!(
        rms > 0.01,
        "early should produce audible audio (RMS = {})",
        rms
    );
    assert!(
        peak > 0.1,
        "early should have audible peaks (peak = {})",
        peak
    );
    assert!(
        dc_offset.abs() < 0.1,
        "early should not have excessive DC offset (DC = {})",
        dc_offset
    );

    println!(
        "✅ early Level 3: RMS = {:.4}, Peak = {:.4}, DC = {:.4}",
        rms, peak, dc_offset
    );
}

#[test]
fn test_late_level3_energy_preservation() {
    let base_code = r#"
tempo: 0.5
out $ s "bd sn hh cp bd sn hh cp"
"#;

    let late_code = r#"
tempo: 0.5
out $ s "bd sn hh cp bd sn hh cp" $ late 0.1
"#;

    let base_audio = render_dsl(base_code, 8);
    let late_audio = render_dsl(late_code, 8);

    let base_rms = calculate_rms(&base_audio);
    let late_rms = calculate_rms(&late_audio);

    // late only changes timing, not amplitude, so energy should be similar
    let ratio = late_rms / base_rms;
    assert!(
        ratio > 0.8 && ratio < 1.2,
        "late should preserve energy: base RMS = {:.4}, late RMS = {:.4}, ratio = {:.2}",
        base_rms,
        late_rms,
        ratio
    );

    println!(
        "✅ late Level 3: Energy preserved: base RMS = {:.4}, late RMS = {:.4}, ratio = {:.2}",
        base_rms, late_rms, ratio
    );
}

#[test]
fn test_early_level3_energy_preservation() {
    let base_code = r#"
tempo: 0.5
out $ s "bd sn hh cp"
"#;

    let early_code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ early 0.1
"#;

    let base_audio = render_dsl(base_code, 8);
    let early_audio = render_dsl(early_code, 8);

    let base_rms = calculate_rms(&base_audio);
    let early_rms = calculate_rms(&early_audio);

    // early only changes timing, not amplitude, so energy should be similar
    let ratio = early_rms / base_rms;
    assert!(
        ratio > 0.8 && ratio < 1.2,
        "early should preserve energy: base RMS = {:.4}, early RMS = {:.4}, ratio = {:.2}",
        base_rms,
        early_rms,
        ratio
    );

    println!(
        "✅ early Level 3: Energy preserved: base RMS = {:.4}, early RMS = {:.4}, ratio = {:.2}",
        base_rms, early_rms, ratio
    );
}

#[test]
fn test_offset_level3_audio_quality() {
    let code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ offset 0.15
"#;

    let audio = render_dsl(code, 8);

    let rms = calculate_rms(&audio);
    let peak = audio.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    let dc_offset = audio.iter().sum::<f32>() / audio.len() as f32;

    assert!(
        rms > 0.01,
        "offset should produce audible audio (RMS = {})",
        rms
    );
    assert!(
        peak > 0.1,
        "offset should have audible peaks (peak = {})",
        peak
    );
    assert!(
        dc_offset.abs() < 0.1,
        "offset should not have excessive DC offset (DC = {})",
        dc_offset
    );

    println!(
        "✅ offset Level 3: RMS = {:.4}, Peak = {:.4}, DC = {:.4}",
        rms, peak, dc_offset
    );
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_late_zero_amount() {
    // late(0.0) should have no effect
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let late_haps = pattern.clone().late(Pattern::pure(0.0)).query(&state);

    for i in 0..base_haps.len() {
        assert_eq!(
            late_haps[i].part.begin, base_haps[i].part.begin,
            "late(0.0) should not change timing"
        );
    }

    println!("✅ late edge case: Zero shift has no effect");
}

#[test]
fn test_early_zero_amount() {
    // early(0.0) should have no effect
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let early_haps = pattern.clone().early(Pattern::pure(0.0)).query(&state);

    for i in 0..base_haps.len() {
        assert_eq!(
            early_haps[i].part.begin, base_haps[i].part.begin,
            "early(0.0) should not change timing"
        );
    }

    println!("✅ early edge case: Zero shift has no effect");
}

#[test]
fn test_late_single_event() {
    // late with single event should work
    let pattern = parse_mini_notation("bd");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let late_haps = pattern.clone().late(Pattern::pure(0.1)).query(&state);

    assert_eq!(late_haps.len(), 1, "Should have 1 event");

    println!("✅ late edge case: Single event handled");
}

#[test]
fn test_early_single_event() {
    // early with single event should work
    let pattern = parse_mini_notation("bd");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let early_haps = pattern.clone().early(Pattern::pure(0.1)).query(&state);

    assert_eq!(early_haps.len(), 1, "Should have 1 event");

    println!("✅ early edge case: Single event handled");
}

#[test]
fn test_late_extreme_values() {
    // Test late with very large shift
    let pattern = parse_mini_notation("bd sn");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // Large shift - should still work
    let late_haps = pattern.clone().late(Pattern::pure(5.0)).query(&state);
    assert!(late_haps.len() > 0, "late should handle large shifts");

    // Verify shift amount
    let base_haps = pattern.query(&state);
    if base_haps.len() > 0 && late_haps.len() > 0 {
        let shift = late_haps[0].part.begin.to_float() - base_haps[0].part.begin.to_float();
        assert!((shift - 5.0).abs() < 0.001, "Should shift by exactly 5.0");
    }

    println!("✅ late edge case: Large shifts handled");
}

#[test]
fn test_early_extreme_values() {
    // Test early with very large shift
    let pattern = parse_mini_notation("bd sn");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // Large backward shift
    let early_haps = pattern.clone().early(Pattern::pure(5.0)).query(&state);
    assert!(early_haps.len() > 0, "early should handle large shifts");

    // Verify shift amount
    let base_haps = pattern.query(&state);
    if base_haps.len() > 0 && early_haps.len() > 0 {
        let shift = early_haps[0].part.begin.to_float() - base_haps[0].part.begin.to_float();
        assert!((shift + 5.0).abs() < 0.001, "Should shift by exactly -5.0");
    }

    println!("✅ early edge case: Large shifts handled");
}
