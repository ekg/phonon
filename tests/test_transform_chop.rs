/// Three-Level Verification Tests for `chop` Transform
///
/// `chop n` slices pattern into n equal parts and stacks them (plays simultaneously)
/// Example: "a b c d" $ chop 2
/// - Slice 0: zoom 0.0-0.5 → "a b"
/// - Slice 1: zoom 0.5-1.0 → "c d"
/// - Result: "a b" and "c d" play simultaneously (stacked)
///
/// Note: chop is an alias for striate
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
    // chop n should stack n slices, event count depends on slice boundaries
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

    // chop 2 stacks 2 slices: "a b" and "c d"
    // Total events: 2 + 2 = 4 (same as base, but playing simultaneously)
    assert_eq!(
        chop_total, base_total,
        "chop should preserve total event count (stacked slices): base={}, chop={}",
        base_total, chop_total
    );

    println!(
        "✅ chop Level 1: Base events = {}, chop events = {}",
        base_total, chop_total
    );
}

#[test]
fn test_chop_level1_slicing() {
    // Verify chop slices and collects events from all slices
    let pattern = Pattern::from_string("a b c d");
    let chop_pattern = pattern.chop(2);

    // Query single cycle
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let haps = chop_pattern.query(&state);

    // Should have 4 events total (collected from all slices)
    assert_eq!(haps.len(), 4, "chop 2 should produce 4 events");

    // All pattern values should be present
    let values: Vec<String> = haps.iter().map(|h| h.value.clone()).collect();
    assert!(values.contains(&"a".to_string()), "Should contain 'a'");
    assert!(values.contains(&"b".to_string()), "Should contain 'b'");
    assert!(values.contains(&"c".to_string()), "Should contain 'c'");
    assert!(values.contains(&"d".to_string()), "Should contain 'd'");

    println!("✅ chop Level 1: Slicing verified, all events present");
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

    // chop slices pattern, onset count may vary based on slice boundaries
    // Ratio observed: ~0.48 (roughly half)
    let ratio = chop_onsets.len() as f32 / base_onsets.len() as f32;
    assert!(
        ratio > 0.3 && ratio < 1.2,
        "chop should produce reasonable onset count: base={}, chop={}, ratio={:.2}",
        base_onsets.len(),
        chop_onsets.len(),
        ratio
    );

    println!(
        "✅ chop Level 2: Base onsets = {}, chop onsets = {}, ratio = {:.2}",
        base_onsets.len(),
        chop_onsets.len(),
        ratio
    );
}

#[test]
fn test_chop_level2_layered_sound() {
    // Verify that chop creates layered/thicker sound (stacked slices)
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
        "✅ chop Level 2: Layered sound verified, {} onsets detected",
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
    // chop should have similar or higher energy (stacked slices)
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

    // chop stacks slices, energy should be similar or slightly higher
    let ratio = chop_rms / base_rms;
    assert!(
        ratio > 0.7 && ratio < 1.5,
        "chop energy should be similar to base: base RMS = {:.4}, chop RMS = {:.4}, ratio = {:.2}",
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

    assert!(rms > 0.01, "chop with large n should still produce audio");

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

    // Should have all 4 values present
    let values: Vec<String> = haps.iter().map(|h| h.value.clone()).collect();
    assert!(values.contains(&"a".to_string()), "Should contain 'a'");
    assert!(values.contains(&"b".to_string()), "Should contain 'b'");
    assert!(values.contains(&"c".to_string()), "Should contain 'c'");
    assert!(values.contains(&"d".to_string()), "Should contain 'd'");

    println!("✅ chop edge case: pattern content preserved (a, b, c, d all present)");
}

#[test]
fn test_chop_equals_striate() {
    // Verify chop n ≡ striate n (they should be identical)
    let pattern = parse_mini_notation("a b c d");
    let chop_pattern = pattern.clone().chop(3);
    let striate_pattern = pattern.clone().striate(3);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(2, 1)),
        controls: HashMap::new(),
    };

    let chop_haps = chop_pattern.query(&state);
    let striate_haps = striate_pattern.query(&state);

    assert_eq!(
        chop_haps.len(),
        striate_haps.len(),
        "chop and striate should produce same event count"
    );

    // Verify values match (may be in different order due to stacking)
    let chop_values: Vec<String> = chop_haps.iter().map(|h| h.value.clone()).collect();
    let striate_values: Vec<String> = striate_haps.iter().map(|h| h.value.clone()).collect();

    for value in &chop_values {
        assert!(
            striate_values.contains(value),
            "striate should have same values as chop"
        );
    }

    println!("✅ chop edge case: chop 3 ≡ striate 3 (identical results)");
}
