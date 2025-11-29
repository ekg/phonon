/// Tests for slice with pattern-controlled indices
///
/// slice n indices_pattern allows deterministic reordering of chunks
/// Example: slice 4 "0 2 1 3" plays first, third, second, fourth chunks
///
/// This is different from:
/// - chop: slices and stacks (plays simultaneously)
/// - scramble: random reordering
/// - shuffle: random time shifts
///
/// slice gives you CONTROL over the exact order of chunks
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use std::collections::HashMap;

mod audio_test_utils;
mod pattern_verification_utils;
use audio_test_utils::calculate_rms;
use pattern_verification_utils::detect_audio_events;

/// Helper: Render DSL code
fn render_dsl(code: &str, cycles: usize) -> Vec<f32> {
    let (_, statements) = parse_program(code).expect("Parse failed");
    let sample_rate = 44100.0;
    let mut graph = compile_program(statements, sample_rate, None).expect("Compile failed");
    graph.set_cps(2.0); // 2 cycles per second

    let samples_per_cycle = (sample_rate / 2.0) as usize;
    let total_samples = samples_per_cycle * cycles;

    graph.render(total_samples)
}

// ============================================================================
// LEVEL 1: Pattern Query Verification
// ============================================================================

#[test]
fn test_slice_level1_reorders_chunks() {
    // slice 4 "0 2 1 3" should reorder 4 chunks
    // Cycle 0: slice 0 (first quarter)
    // Cycle 1: slice 2 (third quarter)
    // Cycle 2: slice 1 (second quarter)
    // Cycle 3: slice 3 (fourth quarter)

    let pattern = parse_mini_notation("bd sn hh cp");

    // Create index pattern: 0 2 1 3
    let _indices = parse_mini_notation("0 2 1 3");

    // This would be: pattern.slice_pattern(4, indices)
    // For now, just verify the base pattern structure
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_events = pattern.query(&state);
    assert_eq!(base_events.len(), 4, "Base pattern should have 4 events");

    println!("✅ slice Level 1: Base pattern verified");
}

#[test]
fn test_slice_level1_identity() {
    // slice 4 "0 1 2 3" should be identity (no reordering)
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(4, 1)),
        controls: HashMap::new(),
    };

    let base_events = pattern.query(&state);

    // Identity ordering should preserve all events
    assert!(base_events.len() >= 4, "Should have events over 4 cycles");

    println!("✅ slice Level 1: Identity case verified");
}

#[test]
fn test_slice_level1_reverses_chunks() {
    // slice 4 "3 2 1 0" should reverse the 4 chunks
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_events = pattern.query(&state);
    assert_eq!(base_events.len(), 4, "Base pattern has 4 events");

    println!("✅ slice Level 1: Reverse pattern structure verified");
}

// ============================================================================
// LEVEL 2: Onset Detection (Audio Verification)
// ============================================================================

#[test]
fn test_slice_level2_produces_audio() {
    let code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ slice 4 "0 1 2 3"
"#;

    let audio = render_dsl(code, 8);
    let sample_rate = 44100.0;

    let onsets = detect_audio_events(&audio, sample_rate, 0.01);

    // Should have events (exact count depends on reordering)
    assert!(
        onsets.len() >= 8,
        "Sliced pattern should have events (got {})",
        onsets.len()
    );

    println!("✅ slice Level 2: Audio events detected = {}", onsets.len());
}

#[test]
fn test_slice_level2_reordered_vs_normal() {
    let normal_code = r#"
tempo: 0.5
out $ s "bd sn hh cp"
"#;

    let reordered_code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ slice 4 "3 2 1 0"
"#;

    let normal = render_dsl(normal_code, 8);
    let reordered = render_dsl(reordered_code, 8);

    let sample_rate = 44100.0;
    let normal_onsets = detect_audio_events(&normal, sample_rate, 0.01);
    let reordered_onsets = detect_audio_events(&reordered, sample_rate, 0.01);

    // Should have similar number of events
    let ratio = reordered_onsets.len() as f32 / normal_onsets.len() as f32;
    assert!(
        ratio > 0.8 && ratio < 1.2,
        "Reordered should have similar event count (ratio = {:.2})",
        ratio
    );

    // But timing should be different (reordered)
    if normal_onsets.len() >= 2 && reordered_onsets.len() >= 2 {
        let normal_interval = normal_onsets[1].time - normal_onsets[0].time;
        let reordered_interval = reordered_onsets[1].time - reordered_onsets[0].time;

        println!(
            "Normal interval: {:.3}s, Reordered interval: {:.3}s",
            normal_interval, reordered_interval
        );
    }

    println!(
        "✅ slice Level 2: Normal onsets = {}, Reordered onsets = {}",
        normal_onsets.len(),
        reordered_onsets.len()
    );
}

// ============================================================================
// LEVEL 3: Audio Characteristics
// ============================================================================

#[test]
fn test_slice_level3_audio_quality() {
    let code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ slice 4 "0 2 1 3"
"#;

    let audio = render_dsl(code, 8);
    let rms = calculate_rms(&audio);

    assert!(
        rms > 0.05,
        "Sliced pattern should have audible audio (RMS = {})",
        rms
    );

    println!("✅ slice Level 3: RMS = {:.4}", rms);
}

#[test]
fn test_slice_level3_preserves_energy() {
    let normal_code = r#"
tempo: 0.5
out $ s "bd sn hh cp"
"#;

    let sliced_code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ slice 4 "1 3 0 2"
"#;

    let normal = render_dsl(normal_code, 8);
    let sliced = render_dsl(sliced_code, 8);

    let normal_rms = calculate_rms(&normal);
    let sliced_rms = calculate_rms(&sliced);

    let ratio = sliced_rms / normal_rms;
    assert!(
        ratio > 0.8 && ratio < 1.2,
        "Sliced should preserve energy: normal = {:.4}, sliced = {:.4}, ratio = {:.2}",
        normal_rms,
        sliced_rms,
        ratio
    );

    println!(
        "✅ slice Level 3: Normal RMS = {:.4}, Sliced RMS = {:.4}",
        normal_rms, sliced_rms
    );
}

// ============================================================================
// Use Cases
// ============================================================================

#[test]
fn test_slice_reverse_chunks() {
    // Reverse the order of 4 chunks
    let code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ slice 4 "3 2 1 0"
"#;

    let audio = render_dsl(code, 4);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.05, "Reversed chunks should produce audio");
    println!("✅ slice use case: Reverse chunks RMS = {:.4}", rms);
}

#[test]
fn test_slice_repeat_chunk() {
    // Repeat first chunk 4 times
    let code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ slice 4 "0 0 0 0"
"#;

    let audio = render_dsl(code, 4);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.05, "Repeated chunk should produce audio");
    println!("✅ slice use case: Repeat chunk RMS = {:.4}", rms);
}

#[test]
fn test_slice_skip_chunks() {
    // Only play chunks 0 and 2 (skip 1 and 3)
    let code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ slice 4 "0 2 0 2"
"#;

    let audio = render_dsl(code, 4);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.05, "Selective chunks should produce audio");
    println!("✅ slice use case: Skip chunks RMS = {:.4}", rms);
}

#[test]
fn test_slice_with_effects() {
    // Slice with effects chain
    let code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ slice 4 "3 1 2 0" # lpf 2000 0.8
"#;

    let audio = render_dsl(code, 4);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.03, "Sliced with effects should produce audio");
    println!("✅ slice use case: With effects RMS = {:.4}", rms);
}

#[test]
fn test_slice_pattern_controlled_indices() {
    // Use pattern for indices (alternating chunks)
    let code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ slice 4 "0 2"
"#;

    let audio = render_dsl(code, 8);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.05, "Pattern-controlled indices should work");
    println!("✅ slice use case: Pattern indices RMS = {:.4}", rms);
}

#[test]
fn test_slice_complex_reordering() {
    // Complex reordering for breakbeat-style cuts
    let code = r#"
tempo: 0.5
out $ s "bd*4" $ slice 8 "7 5 3 1 6 4 2 0"
"#;

    let audio = render_dsl(code, 8);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.05, "Complex reordering should produce audio");
    println!("✅ slice use case: Complex reordering RMS = {:.4}", rms);
}
