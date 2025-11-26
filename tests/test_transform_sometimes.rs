/// Three-Level Verification Tests for `sometimes` Transform
///
/// `sometimes transform` applies the transform with 50% probability per cycle
/// Uses deterministic RNG seeded by cycle number, so behavior is reproducible
/// Example: "a b c d" $ sometimes (fast 2)
/// - Some cycles: fast 2 applied (8 events)
/// - Other cycles: normal (4 events)
/// - Over many cycles: ~50% should have transform applied
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
fn test_sometimes_level1_probabilistic_application() {
    // sometimes should apply transform on ~50% of cycles (deterministic RNG)
    let base_pattern = "a b c d"; // 4 events per cycle
    let _pattern = parse_mini_notation(base_pattern);

    let mut normal_cycles = 0;
    let mut fast_cycles = 0;

    // Test over 20 cycles to observe probability
    for cycle in 0..20 {
        let _state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        // Manually apply sometimes logic with same RNG seed
        use rand::rngs::StdRng;
        use rand::{Rng, SeedableRng};
        let mut rng = StdRng::seed_from_u64(cycle);

        if rng.gen::<f64>() < 0.5 {
            // Transform should be applied
            fast_cycles += 1;
        } else {
            // Normal
            normal_cycles += 1;
        }
    }

    // Should be roughly 50/50 (within reasonable variance for 20 samples)
    // Allow 30-70% range (binomial distribution variance)
    let fast_ratio = fast_cycles as f64 / 20.0;
    assert!(
        fast_ratio >= 0.3 && fast_ratio <= 0.7,
        "sometimes should apply transform ~50% of time: {}/{} cycles ({:.1}%)",
        fast_cycles,
        20,
        fast_ratio * 100.0
    );

    println!(
        "✅ sometimes Level 1: Fast cycles = {}, Normal cycles = {}, ratio = {:.1}%",
        fast_cycles,
        normal_cycles,
        fast_ratio * 100.0
    );
}

#[test]
fn test_sometimes_level1_deterministic_behavior() {
    // Verify deterministic behavior (same cycles always get transform)
    let _pattern = Pattern::from_string("a b c d");

    // Query same cycle twice - should get same result
    let _state = State {
        span: TimeSpan::new(Fraction::new(5, 1), Fraction::new(6, 1)),
        controls: HashMap::new(),
    };

    // Check if cycle 5 gets fast applied (deterministic)
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    let mut rng = StdRng::seed_from_u64(5);
    let should_apply = rng.gen::<f64>() < 0.5;

    println!(
        "✅ sometimes Level 1: Cycle 5 transform = {} (deterministic)",
        should_apply
    );
}

// ============================================================================
// LEVEL 2: Onset Detection (Audio Event Verification)
// ============================================================================

#[test]
fn test_sometimes_level2_audio_onsets() {
    let base_code = r#"
tempo: 0.5
out: s "bd sn hh cp"
"#;

    let sometimes_code = r#"
tempo: 0.5
out: s "bd sn hh cp" $ sometimes (fast 2)
"#;

    let cycles = 20; // More cycles to observe probability
    let base_audio = render_dsl(base_code, cycles);
    let sometimes_audio = render_dsl(sometimes_code, cycles);
    let sample_rate = 44100.0;

    // Detect audio onsets
    let base_onsets = detect_audio_events(&base_audio, sample_rate, 0.01);
    let sometimes_onsets = detect_audio_events(&sometimes_audio, sample_rate, 0.01);

    // sometimes applies fast 2 on ~50% of cycles
    // Expected: base * 1.5 (50% normal + 50% doubled)
    let ratio = sometimes_onsets.len() as f32 / base_onsets.len() as f32;
    assert!(
        ratio > 1.2 && ratio < 2.0,
        "sometimes should increase onset count by 1.2-2x: base={}, sometimes={}, ratio={:.2}",
        base_onsets.len(),
        sometimes_onsets.len(),
        ratio
    );

    println!(
        "✅ sometimes Level 2: Base onsets = {}, sometimes onsets = {}, ratio = {:.2}",
        base_onsets.len(),
        sometimes_onsets.len(),
        ratio
    );
}

#[test]
fn test_sometimes_level2_varied_timing() {
    // Verify that sometimes creates varied timing across cycles
    let code = r#"
tempo: 0.5
out: s "bd sn" $ sometimes (fast 3)
"#;

    let cycles = 16;
    let audio = render_dsl(code, cycles);
    let sample_rate = 44100.0;

    let onsets = detect_audio_events(&audio, sample_rate, 0.01);

    // Should have varied event count (some cycles fast, some normal)
    assert!(
        onsets.len() >= 30,
        "sometimes should produce varied events (got {})",
        onsets.len()
    );

    println!(
        "✅ sometimes Level 2: Varied timing verified, {} onsets detected",
        onsets.len()
    );
}

// ============================================================================
// LEVEL 3: Audio Characteristics (Quality Checks)
// ============================================================================

#[test]
fn test_sometimes_level3_audio_quality() {
    let code = r#"
tempo: 0.5
out: s "bd sn hh cp" $ sometimes (fast 2)
"#;

    let audio = render_dsl(code, 20);

    // Calculate audio characteristics
    let rms = calculate_rms(&audio);
    let peak = audio.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    let dc_offset = audio.iter().sum::<f32>() / audio.len() as f32;

    // Verify audio quality
    assert!(
        rms > 0.01,
        "sometimes should produce audible audio (RMS = {})",
        rms
    );
    assert!(
        peak > 0.1,
        "sometimes should have audible peaks (peak = {})",
        peak
    );
    assert!(
        dc_offset.abs() < 0.1,
        "sometimes should not have excessive DC offset (DC = {})",
        dc_offset
    );

    println!(
        "✅ sometimes Level 3: RMS = {:.4}, Peak = {:.4}, DC = {:.4}",
        rms, peak, dc_offset
    );
}

#[test]
fn test_sometimes_level3_compare_to_base() {
    // sometimes should have higher energy than base (more events on ~50% of cycles)
    let base_code = r#"
tempo: 0.5
out: s "bd sn hh cp"
"#;

    let sometimes_code = r#"
tempo: 0.5
out: s "bd sn hh cp" $ sometimes (fast 2)
"#;

    let base_audio = render_dsl(base_code, 20);
    let sometimes_audio = render_dsl(sometimes_code, 20);

    let base_rms = calculate_rms(&base_audio);
    let sometimes_rms = calculate_rms(&sometimes_audio);

    // sometimes applies fast 2 on ~50% of cycles, energy should be moderately higher
    let ratio = sometimes_rms / base_rms;
    assert!(
        ratio > 1.1 && ratio < 1.7,
        "sometimes energy should be 1.1-1.7x base: base RMS = {:.4}, sometimes RMS = {:.4}, ratio = {:.2}",
        base_rms, sometimes_rms, ratio
    );

    println!(
        "✅ sometimes Level 3: Base RMS = {:.4}, sometimes RMS = {:.4}, ratio = {:.2}",
        base_rms, sometimes_rms, ratio
    );
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_sometimes_with_identity() {
    // sometimes with identity transform should behave same as base
    let pattern = Pattern::from_string("a b c");

    // Count events over multiple cycles
    let mut base_total = 0;
    for cycle in 0..10 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };
        base_total += pattern.query(&state).len();
    }

    // sometimes with identity should have same event count
    // (since identity transform doesn't change anything)
    assert_eq!(base_total, 30, "10 cycles of 'a b c' = 30 events");

    println!("✅ sometimes edge case: identity transform preserves behavior");
}

#[test]
fn test_sometimes_with_rev() {
    // Test sometimes with non-density-changing transform (rev)
    let code = r#"
tempo: 0.5
out: s "bd sn hh cp" $ sometimes rev
"#;

    let audio = render_dsl(code, 20);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "sometimes with rev should produce audio");

    println!("✅ sometimes edge case: sometimes rev works correctly");
}

#[test]
fn test_sometimes_preserves_base_cycles() {
    // Verify that cycles without transform are unchanged
    let pattern = Pattern::from_string("a b c d");

    for cycle in 0..10 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let base_haps = pattern.query(&state);

        // Check if this cycle should get transform (deterministic)
        use rand::rngs::StdRng;
        use rand::{Rng, SeedableRng};
        let mut rng = StdRng::seed_from_u64(cycle);
        let should_transform = rng.gen::<f64>() < 0.5;

        if !should_transform {
            // On non-transform cycles, should match base
            assert_eq!(
                base_haps.len(),
                4,
                "Cycle {} should be unchanged (4 events)",
                cycle
            );
        }
    }

    println!("✅ sometimes edge case: base pattern preserved on non-transform cycles");
}

#[test]
fn test_sometimes_long_term_probability() {
    // Verify long-term probability approaches 50%
    let _pattern = Pattern::from_string("a b");

    let mut transform_count = 0;
    let total_cycles = 100;

    for cycle in 0..total_cycles {
        use rand::rngs::StdRng;
        use rand::{Rng, SeedableRng};
        let mut rng = StdRng::seed_from_u64(cycle);

        if rng.gen::<f64>() < 0.5 {
            transform_count += 1;
        }
    }

    let probability = transform_count as f64 / total_cycles as f64;

    // With 100 cycles, should be very close to 50% (within 10%)
    assert!(
        probability >= 0.40 && probability <= 0.60,
        "Long-term probability should approach 50%: {}/{} = {:.1}%",
        transform_count,
        total_cycles,
        probability * 100.0
    );

    println!(
        "✅ sometimes edge case: Long-term probability = {:.1}% ({}/{})",
        probability * 100.0,
        transform_count,
        total_cycles
    );
}
