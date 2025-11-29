/// Three-Level Verification Tests for `ply` Transform
///
/// `ply n` repeats each event n times within its original duration (like stutter)
/// Example: "a b" $ ply 3
/// - "a" (0-0.5) becomes: a a a (each 0.167 long)
/// - "b" (0.5-1.0) becomes: b b b (each 0.167 long)
/// Total: 6 events instead of 2
///
/// Note: ply is functionally equivalent to stutter in Phonon
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

// ============================================================================
// LEVEL 1: Pattern Query Verification (Exact Event Counts)
// ============================================================================

#[test]
fn test_ply_level1_event_count() {
    // ply n should multiply event count by n
    let base_pattern = "a b c d"; // 4 events per cycle
    let cycles = 4;

    let pattern = parse_mini_notation(base_pattern);
    let ply_pattern = pattern.clone().ply(3);

    // Count events over 4 cycles
    let mut base_total = 0;
    let mut ply_total = 0;

    for cycle in 0..cycles {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        base_total += pattern.query(&state).len();
        ply_total += ply_pattern.query(&state).len();
    }

    // ply 3 should triple event count
    assert_eq!(
        ply_total,
        base_total * 3,
        "ply 3 should triple event count: base={}, ply={}",
        base_total,
        ply_total
    );

    println!(
        "✅ ply Level 1: Base events = {}, ply events = {}",
        base_total, ply_total
    );
}

#[test]
fn test_ply_level1_event_timing() {
    // Verify ply divides events correctly
    let pattern = Pattern::from_string("x");
    let ply_pattern = pattern.ply(4);

    // Query single cycle
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let haps = ply_pattern.query(&state);

    // Should have 4 events (1 original * 4 ply)
    assert_eq!(haps.len(), 4, "ply 4 of 1 event should produce 4 events");

    // Each event should be 0.25 long (1.0 / 4)
    for (i, hap) in haps.iter().enumerate() {
        let duration = hap.part.duration().to_float();
        assert!(
            (duration - 0.25).abs() < 0.001,
            "Event {} should have duration 0.25, got {}",
            i,
            duration
        );
    }

    // Events should be sequential
    assert!((haps[0].part.begin.to_float() - 0.0).abs() < 0.001);
    assert!((haps[1].part.begin.to_float() - 0.25).abs() < 0.001);
    assert!((haps[2].part.begin.to_float() - 0.5).abs() < 0.001);
    assert!((haps[3].part.begin.to_float() - 0.75).abs() < 0.001);

    println!("✅ ply Level 1: Event timing verified");
}

// ============================================================================
// LEVEL 2: Onset Detection (Audio Event Verification)
// ============================================================================

#[test]
fn test_ply_level2_audio_onsets() {
    let base_code = r#"
tempo: 0.5
out $ s "bd sn hh cp"
"#;

    let ply_code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ ply 3
"#;

    let cycles = 8;
    let base_audio = render_dsl(base_code, cycles);
    let ply_audio = render_dsl(ply_code, cycles);
    let sample_rate = 44100.0;

    // Detect audio onsets
    let base_onsets = detect_audio_events(&base_audio, sample_rate, 0.01);
    let ply_onsets = detect_audio_events(&ply_audio, sample_rate, 0.01);

    // ply 3 should significantly increase onset count (rapid events may blend in detection)
    let ratio = ply_onsets.len() as f32 / base_onsets.len() as f32;
    assert!(
        ratio > 1.5 && ratio < 4.0,
        "ply 3 should significantly increase onset count: base={}, ply={}, ratio={:.2}",
        base_onsets.len(),
        ply_onsets.len(),
        ratio
    );

    println!(
        "✅ ply Level 2: Base onsets = {}, ply onsets = {}, ratio = {:.2}",
        base_onsets.len(),
        ply_onsets.len(),
        ratio
    );
}

#[test]
fn test_ply_level2_rapid_succession() {
    // Verify that ply creates rapid succession of events
    let code = r#"
tempo: 0.5
out $ s "bd" $ ply 4
"#;

    let cycles = 2;
    let audio = render_dsl(code, cycles);
    let sample_rate = 44100.0;

    let onsets = detect_audio_events(&audio, sample_rate, 0.01);

    // Should have multiple events (at least 6 for 2 cycles with ply 4)
    assert!(
        onsets.len() >= 6,
        "ply 4 should produce multiple rapid events (got {})",
        onsets.len()
    );

    // Check that events are close together in time
    if onsets.len() >= 2 {
        let interval = onsets[1].time - onsets[0].time;
        assert!(
            interval < 0.6,
            "Ply events should be close together (interval = {:.3}s)",
            interval
        );
    }

    println!(
        "✅ ply Level 2: Rapid succession verified, {} onsets detected",
        onsets.len()
    );
}

// ============================================================================
// LEVEL 3: Audio Characteristics (Quality Checks)
// ============================================================================

#[test]
fn test_ply_level3_audio_quality() {
    let code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ ply 3
"#;

    let audio = render_dsl(code, 8);

    // Calculate audio characteristics
    let rms = calculate_rms(&audio);
    let peak = audio.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    let dc_offset = audio.iter().sum::<f32>() / audio.len() as f32;

    // Verify audio quality
    assert!(
        rms > 0.01,
        "ply should produce audible audio (RMS = {})",
        rms
    );
    assert!(
        peak > 0.1,
        "ply should have audible peaks (peak = {})",
        peak
    );
    assert!(
        dc_offset.abs() < 0.1,
        "ply should not have excessive DC offset (DC = {})",
        dc_offset
    );

    println!(
        "✅ ply Level 3: RMS = {:.4}, Peak = {:.4}, DC = {:.4}",
        rms, peak, dc_offset
    );
}

#[test]
fn test_ply_level3_compare_to_base() {
    // ply should have higher energy than base (more events)
    let base_code = r#"
tempo: 0.5
out $ s "bd sn hh cp"
"#;

    let ply_code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ ply 3
"#;

    let base_audio = render_dsl(base_code, 8);
    let ply_audio = render_dsl(ply_code, 8);

    let base_rms = calculate_rms(&base_audio);
    let ply_rms = calculate_rms(&ply_audio);

    // ply triples event count, energy should be roughly 1.5-4.5x (accounting for shorter durations)
    let ratio = ply_rms / base_rms;
    assert!(
        ratio > 1.5 && ratio < 4.5,
        "ply energy should be 1.5-4.5x base: base RMS = {:.4}, ply RMS = {:.4}, ratio = {:.2}",
        base_rms,
        ply_rms,
        ratio
    );

    println!(
        "✅ ply Level 3: Base RMS = {:.4}, ply RMS = {:.4}, ratio = {:.2}",
        base_rms, ply_rms, ratio
    );
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_ply_with_ply_1() {
    // ply 1 should be identity (no change)
    let pattern = Pattern::from_string("a b c");
    let ply_pattern = pattern.clone().ply(1);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(2, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let ply_haps = ply_pattern.query(&state);

    assert_eq!(
        base_haps.len(),
        ply_haps.len(),
        "ply 1 should preserve event count"
    );

    println!("✅ ply edge case: ply 1 behaves as identity");
}

#[test]
fn test_ply_with_large_n() {
    // Test ply with large n
    let code = r#"
tempo: 0.5
out $ s "bd sn" $ ply 16
"#;

    let audio = render_dsl(code, 4);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "ply with large n should still produce audio");

    println!("✅ ply edge case: ply 16 works correctly");
}

#[test]
fn test_ply_preserves_values() {
    // Verify ply repeats the same value
    let pattern = Pattern::from_string("a b");
    let ply_pattern = pattern.ply(3);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let haps = ply_pattern.query(&state);

    // Should have 6 events (2 * 3)
    assert_eq!(haps.len(), 6, "Should have 6 events");

    // First 3 should be 'a', next 3 should be 'b'
    assert_eq!(haps[0].value, "a");
    assert_eq!(haps[1].value, "a");
    assert_eq!(haps[2].value, "a");
    assert_eq!(haps[3].value, "b");
    assert_eq!(haps[4].value, "b");
    assert_eq!(haps[5].value, "b");

    println!("✅ ply edge case: values preserved correctly");
}
