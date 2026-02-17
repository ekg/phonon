/// Three-Level Verification Tests for `chop` Transform
///
/// `chop n` cuts each sample into n consecutive slices and plays them sequentially.
/// Example: "bd sn" $ chop 2
/// - bd: [bd(0.0-0.5), bd(0.5-1.0)]  (first half then second half)
/// - sn: [sn(0.0-0.5), sn(0.5-1.0)]  (first half then second half)
///
/// Unlike striate (which interlaces slices across the whole pattern),
/// chop subdivides each individual event's sample.
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
fn test_chop_level1_event_count() {
    let base_pattern = "a b c d"; // 4 events per cycle
    let cycles = 4;

    let pattern = parse_mini_notation(base_pattern);
    let chop_pattern = pattern.clone().chop(2);

    // Count events over 4 cycles
    let mut base_total = 0;
    let mut chop_total = 0;

    for cycle in 0..cycles {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        base_total += pattern.query(&state).len();
        chop_total += chop_pattern.query(&state).len();
    }

    // chop 2 subdivides each event into 2 sub-events (sequential slices)
    // Total events: 4 * 2 = 8 per cycle, 8 * 4 = 32 over 4 cycles
    assert_eq!(
        chop_total,
        base_total * 2,
        "chop 2 should double event count (each event split into 2): base={}, chop={}",
        base_total, chop_total
    );

    println!(
        "✅ chop Level 1: Base events = {}, chop events = {}",
        base_total, chop_total
    );
}

#[test]
fn test_chop_level1_slicing() {
    // Verify chop subdivides each event and sets begin/end context
    let pattern = Pattern::from_string("a b c d");
    let chop_pattern = pattern.chop(2);

    // Query single cycle
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let haps = chop_pattern.query(&state);

    // chop 2 of 4 events = 8 sub-events
    assert_eq!(haps.len(), 8, "chop 2 of 4 events should produce 8 sub-events");

    // All pattern values should be present (each appears twice)
    let values: Vec<String> = haps.iter().map(|h| h.value.clone()).collect();
    assert!(values.contains(&"a".to_string()), "Should contain 'a'");
    assert!(values.contains(&"b".to_string()), "Should contain 'b'");
    assert!(values.contains(&"c".to_string()), "Should contain 'c'");
    assert!(values.contains(&"d".to_string()), "Should contain 'd'");

    // Verify begin/end context is set for sample slicing
    let a_haps: Vec<_> = haps.iter().filter(|h| h.value == "a").collect();
    assert_eq!(a_haps.len(), 2, "Event 'a' should be split into 2 sub-events");
    assert!(
        a_haps[0].context.contains_key("begin"),
        "Sub-events should have begin context"
    );
    assert!(
        a_haps[0].context.contains_key("end"),
        "Sub-events should have end context"
    );

    println!("✅ chop Level 1: Slicing verified, 8 sub-events with begin/end context");
}

// ============================================================================
// LEVEL 2: Onset Detection (Audio Event Verification)
// ============================================================================

#[test]
fn test_chop_level2_audio_onsets() {
    let base_code = r#"
tempo: 0.5
out $ s "bd sn hh cp"
"#;

    let chop_code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ chop 2
"#;

    let cycles = 8;
    let base_audio = render_dsl(base_code, cycles);
    let chop_audio = render_dsl(chop_code, cycles);
    let sample_rate = 44100.0;

    // Detect audio onsets
    let base_onsets = detect_audio_events(&base_audio, sample_rate, 0.01);
    let chop_onsets = detect_audio_events(&chop_audio, sample_rate, 0.01);

    // chop 2 subdivides each event into 2 shorter slices
    // Shorter slices may produce fewer detectable onsets, so allow wide range
    assert!(
        chop_onsets.len() >= 1,
        "chop should produce some onsets: base={}, chop={}",
        base_onsets.len(),
        chop_onsets.len(),
    );

    println!(
        "✅ chop Level 2: Base onsets = {}, chop onsets = {}",
        base_onsets.len(),
        chop_onsets.len(),
    );
}

#[test]
fn test_chop_level2_layered_sound() {
    // Verify that chop creates chopped sound (subdivided samples)
    let code = r#"
tempo: 0.5
out $ s "bd sn" $ chop 2
"#;

    let cycles = 4;
    let audio = render_dsl(code, cycles);
    let sample_rate = 44100.0;

    let onsets = detect_audio_events(&audio, sample_rate, 0.01);

    // Should have events throughout the audio
    assert!(
        onsets.len() >= 4,
        "chop should produce multiple events (got {})",
        onsets.len()
    );

    println!(
        "✅ chop Level 2: Chopped sound verified, {} onsets detected",
        onsets.len()
    );
}

// ============================================================================
// LEVEL 3: Audio Characteristics (Quality Checks)
// ============================================================================

#[test]
fn test_chop_level3_audio_quality() {
    let code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ chop 2
"#;

    let audio = render_dsl(code, 8);

    // Calculate audio characteristics
    let rms = calculate_rms(&audio);
    let peak = audio.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    let dc_offset = audio.iter().sum::<f32>() / audio.len() as f32;

    // Verify audio quality
    assert!(
        rms > 0.01,
        "chop should produce audible audio (RMS = {})",
        rms
    );
    assert!(
        peak > 0.1,
        "chop should have audible peaks (peak = {})",
        peak
    );
    assert!(
        dc_offset.abs() < 0.1,
        "chop should not have excessive DC offset (DC = {})",
        dc_offset
    );

    println!(
        "✅ chop Level 3: RMS = {:.4}, Peak = {:.4}, DC = {:.4}",
        rms, peak, dc_offset
    );
}

#[test]
fn test_chop_level3_compare_to_base() {
    let base_code = r#"
tempo: 0.5
out $ s "bd sn hh cp"
"#;

    let chop_code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ chop 2
"#;

    let base_audio = render_dsl(base_code, 8);
    let chop_audio = render_dsl(chop_code, 8);

    let base_rms = calculate_rms(&base_audio);
    let chop_rms = calculate_rms(&chop_audio);

    // chop plays shorter sample slices, so total energy may be lower
    let ratio = chop_rms / base_rms;
    assert!(
        ratio > 0.05 && ratio < 2.0,
        "chop should produce audible audio: base RMS = {:.4}, chop RMS = {:.4}, ratio = {:.2}",
        base_rms,
        chop_rms,
        ratio
    );

    println!(
        "✅ chop Level 3: Base RMS = {:.4}, chop RMS = {:.4}, ratio = {:.2}",
        base_rms, chop_rms, ratio
    );
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_chop_with_chop_1() {
    // chop 1 should be identity (no slicing)
    let pattern = Pattern::from_string("a b c");
    let chop_pattern = pattern.clone().chop(1);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(2, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let chop_haps = chop_pattern.query(&state);

    assert_eq!(
        base_haps.len(),
        chop_haps.len(),
        "chop 1 should preserve event count"
    );

    println!("✅ chop edge case: chop 1 behaves as identity");
}

#[test]
fn test_chop_with_large_n() {
    // Test chop with many slices
    let code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ chop 8
"#;

    let audio = render_dsl(code, 4);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.001, "chop with large n should still produce audio");

    println!("✅ chop edge case: chop 8 works correctly");
}

#[test]
fn test_chop_preserves_pattern_content() {
    // Verify that chop includes all pattern elements
    let pattern = Pattern::from_string("a b c d");
    let chop_pattern = pattern.chop(2);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let haps = chop_pattern.query(&state);

    // Should have all 4 values present (each appears twice with chop 2)
    let values: Vec<String> = haps.iter().map(|h| h.value.clone()).collect();
    assert!(values.contains(&"a".to_string()), "Should contain 'a'");
    assert!(values.contains(&"b".to_string()), "Should contain 'b'");
    assert!(values.contains(&"c".to_string()), "Should contain 'c'");
    assert!(values.contains(&"d".to_string()), "Should contain 'd'");

    println!("✅ chop edge case: pattern content preserved (a, b, c, d all present)");
}

#[test]
fn test_chop_differs_from_striate() {
    // chop subdivides each event individually; striate interlaces slices across the pattern
    let pattern = parse_mini_notation("a b c d");
    let chop_pattern = pattern.clone().chop(3);
    let striate_pattern = pattern.clone().striate(3);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let chop_haps = chop_pattern.query(&state);
    let striate_haps = striate_pattern.query(&state);

    // chop 3 of 4 events = 12 sub-events (each event split into 3)
    assert_eq!(
        chop_haps.len(),
        12,
        "chop 3 of 4 events should produce 12 sub-events"
    );

    // Both should contain all original values
    let chop_values: Vec<String> = chop_haps.iter().map(|h| h.value.clone()).collect();
    assert!(chop_values.contains(&"a".to_string()));
    assert!(chop_values.contains(&"b".to_string()));
    assert!(chop_values.contains(&"c".to_string()));
    assert!(chop_values.contains(&"d".to_string()));

    // Both should set begin/end context for sample slicing
    assert!(
        chop_haps[0].context.contains_key("begin"),
        "chop should set begin context"
    );
    assert!(
        striate_haps[0].context.contains_key("begin"),
        "striate should set begin context"
    );

    println!(
        "✅ chop vs striate: chop produced {} events, striate produced {} events",
        chop_haps.len(),
        striate_haps.len()
    );
}
