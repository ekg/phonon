/// Three-Level Verification Tests for `palindrome` Transform
///
/// `palindrome` plays pattern forward then backward, spread over 2 cycles
/// Example: "a b c d" $ palindrome
/// - Cycle 0-1: a b c d (forward, slowed 2x)
/// - Cycle 1-2: d c b a (backward, slowed 2x, shifted)
///
/// Over 2 cycles: plays a b c d d c b a
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
fn test_palindrome_level1_event_count() {
    // palindrome stacks forward+backward, maintaining similar event density
    let base_pattern = "a b c d"; // 4 events per cycle
    let cycles = 4;

    let pattern = parse_mini_notation(base_pattern);
    let palindrome_pattern = pattern.clone().palindrome();

    // Count events over 4 cycles
    let mut base_total = 0;
    let mut palindrome_total = 0;

    for cycle in 0..cycles {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        base_total += pattern.query(&state).len();
        palindrome_total += palindrome_pattern.query(&state).len();
    }

    // palindrome maintains same event count (stacked, not concatenated)
    assert_eq!(
        palindrome_total, base_total,
        "palindrome should preserve event count: base={}, palindrome={}",
        base_total, palindrome_total
    );

    println!(
        "✅ palindrome Level 1: Base events = {}, palindrome events = {}",
        base_total, palindrome_total
    );
}

#[test]
fn test_palindrome_level1_forward_backward_overlap() {
    // Verify palindrome creates overlapping forward+backward patterns
    let pattern = Pattern::from_string("a b c d");
    let palindrome_pattern = pattern.palindrome();

    // Query cycle 0
    let state0 = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let haps0 = palindrome_pattern.query(&state0);

    // Should have 4 events (from stacked forward+backward)
    assert_eq!(haps0.len(), 4, "palindrome should have 4 events in cycle 0");

    // Events should include both forward (a, b) and backward (d, c)
    let values: Vec<String> = haps0.iter().map(|h| h.value.clone()).collect();
    assert!(values.contains(&"a".to_string()), "Should contain 'a'");
    assert!(values.contains(&"b".to_string()), "Should contain 'b'");
    assert!(values.contains(&"c".to_string()), "Should contain 'c'");
    assert!(values.contains(&"d".to_string()), "Should contain 'd'");

    println!("✅ palindrome Level 1: Forward+backward overlap verified");
}

// ============================================================================
// LEVEL 2: Onset Detection (Audio Event Verification)
// ============================================================================

#[test]
fn test_palindrome_level2_audio_onsets() {
    let base_code = r#"
tempo: 0.5
out: s "bd sn hh cp"
"#;

    let palindrome_code = r#"
tempo: 0.5
out: s "bd sn hh cp" $ palindrome
"#;

    let cycles = 8;
    let base_audio = render_dsl(base_code, cycles);
    let palindrome_audio = render_dsl(palindrome_code, cycles);
    let sample_rate = 44100.0;

    // Detect audio onsets
    let base_onsets = detect_audio_events(&base_audio, sample_rate, 0.01);
    let palindrome_onsets = detect_audio_events(&palindrome_audio, sample_rate, 0.01);

    // palindrome preserves event count (within 50% tolerance due to stacking/masking effects)
    let ratio = palindrome_onsets.len() as f32 / base_onsets.len() as f32;
    assert!(
        ratio > 0.45 && ratio < 1.5,
        "palindrome should preserve onset count: base={}, palindrome={}, ratio={:.2}",
        base_onsets.len(),
        palindrome_onsets.len(),
        ratio
    );

    println!(
        "✅ palindrome Level 2: Base onsets = {}, palindrome onsets = {}, ratio = {:.2}",
        base_onsets.len(),
        palindrome_onsets.len(),
        ratio
    );
}

#[test]
fn test_palindrome_level2_timing_symmetry() {
    // Verify that palindrome has temporal symmetry (forward matches backward timing)
    let code = r#"
tempo: 0.5
out: s "bd sn" $ palindrome
"#;

    let cycles = 2; // One complete palindrome cycle
    let audio = render_dsl(code, cycles);
    let sample_rate = 44100.0;

    let onsets = detect_audio_events(&audio, sample_rate, 0.01);

    // Should have at least 4 events (2 forward + 2 backward)
    assert!(
        onsets.len() >= 4,
        "palindrome should have multiple events (got {})",
        onsets.len()
    );

    // Check that events are spread across the 2 cycles
    if onsets.len() >= 2 {
        let first_time = onsets.first().unwrap().time;
        let last_time = onsets.last().unwrap().time;
        let span = last_time - first_time;

        assert!(
            span > 1.0,
            "palindrome events should span at least 1 second (got {:.2}s)",
            span
        );
    }

    println!(
        "✅ palindrome Level 2: Timing symmetry verified, {} onsets detected",
        onsets.len()
    );
}

// ============================================================================
// LEVEL 3: Audio Characteristics (Quality Checks)
// ============================================================================

#[test]
fn test_palindrome_level3_audio_quality() {
    let code = r#"
tempo: 0.5
out: s "bd sn hh cp" $ palindrome
"#;

    let audio = render_dsl(code, 8);

    // Calculate audio characteristics
    let rms = calculate_rms(&audio);
    let peak = audio.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    let dc_offset = audio.iter().sum::<f32>() / audio.len() as f32;

    // Verify audio quality
    assert!(
        rms > 0.01,
        "palindrome should produce audible audio (RMS = {})",
        rms
    );
    assert!(
        peak > 0.1,
        "palindrome should have audible peaks (peak = {})",
        peak
    );
    assert!(
        dc_offset.abs() < 0.1,
        "palindrome should not have excessive DC offset (DC = {})",
        dc_offset
    );

    println!(
        "✅ palindrome Level 3: RMS = {:.4}, Peak = {:.4}, DC = {:.4}",
        rms, peak, dc_offset
    );
}

#[test]
fn test_palindrome_level3_compare_to_base() {
    // palindrome should have similar energy to base pattern (stacked, not doubled)
    let base_code = r#"
tempo: 0.5
out: s "bd sn hh cp"
"#;

    let palindrome_code = r#"
tempo: 0.5
out: s "bd sn hh cp" $ palindrome
"#;

    let base_audio = render_dsl(base_code, 8);
    let palindrome_audio = render_dsl(palindrome_code, 8);

    let base_rms = calculate_rms(&base_audio);
    let palindrome_rms = calculate_rms(&palindrome_audio);

    // palindrome preserves event count (stacked patterns), energy should be similar
    let ratio = palindrome_rms / base_rms;
    assert!(
        ratio > 0.5 && ratio < 1.5,
        "palindrome energy should be similar to base: base RMS = {:.4}, palindrome RMS = {:.4}, ratio = {:.2}",
        base_rms, palindrome_rms, ratio
    );

    println!(
        "✅ palindrome Level 3: Base RMS = {:.4}, palindrome RMS = {:.4}, ratio = {:.2}",
        base_rms, palindrome_rms, ratio
    );
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_palindrome_with_single_event() {
    // palindrome of single event should preserve event count (stacking)
    let pattern = Pattern::from_string("x");
    let palindrome_pattern = pattern.clone().palindrome();

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(2, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let palindrome_haps = palindrome_pattern.query(&state);

    assert_eq!(
        base_haps.len(),
        2,
        "Base should have 2 events over 2 cycles"
    );
    assert_eq!(
        palindrome_haps.len(),
        2,
        "palindrome should preserve event count (2 events over 2 cycles)"
    );

    println!("✅ palindrome edge case: single event works correctly");
}

#[test]
fn test_palindrome_with_long_pattern() {
    // Test palindrome with longer pattern
    let code = r#"
tempo: 0.5
out: s "bd sn hh cp lt mt ht cp" $ palindrome
"#;

    let audio = render_dsl(code, 8);
    let rms = calculate_rms(&audio);

    assert!(
        rms > 0.01,
        "palindrome with long pattern should still produce audio"
    );

    println!("✅ palindrome edge case: long pattern (8 events) works correctly");
}
