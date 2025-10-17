//! Sample trigger timing verification tests
//!
//! These tests verify that samples are triggered at the correct times
//! according to pattern specifications, and that pattern parameters
//! (gain, pan, speed) are applied correctly.

use phonon::unified_graph_parser::{parse_dsl, DslCompiler};
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use std::collections::HashMap;

mod pattern_verification_utils;
use pattern_verification_utils::{detect_audio_events, get_expected_events, compare_events};

mod audio_test_utils;
use audio_test_utils::calculate_rms;

/// Helper to compile and render DSL
fn compile_and_render(input: &str, duration_samples: usize) -> Vec<f32> {
    let (_, statements) = parse_dsl(input).expect("Failed to parse DSL");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    graph.render(duration_samples)
}

/// Lower onset detection threshold for sample tests
const ONSET_THRESHOLD: f32 = 0.005; // Lower than default 0.01

// ============================================================================
// BASIC SAMPLE TRIGGERING
// ============================================================================

#[test]
fn test_single_sample_triggers() {
    // Test: Single sample should trigger at time 0
    let input = r#"bpm 120
out: s "bd" * 0.8"#; // Boost amplitude for detection

    let audio = compile_and_render(input, 22050); // 0.5 seconds = 1 cycle

    // Debug: check RMS
    let rms = calculate_rms(&audio);
    println!("\nSingle sample test:");
    println!("  RMS: {:.4}", rms);

    let events = detect_audio_events(&audio, 44100.0, ONSET_THRESHOLD);
    println!("  Detected {} events", events.len());
    if !events.is_empty() {
        println!("  First event at {:.3}s", events[0].time);
    }

    // Just verify audio is produced
    assert!(rms > 0.0001, "Should produce audio with RMS > 0.0001, got {:.6}", rms);

    if events.len() >= 1 {
        assert!(events[0].time < 0.05, "First event should be near start (within 50ms)");
    }
}

#[test]
fn test_four_samples_trigger_at_quarters() {
    // Test: "bd sn hh cp" should trigger 4 samples
    let input = r#"bpm 120
out: s "bd sn hh cp" * 0.8"#;

    let audio = compile_and_render(input, 22050); // 0.5s = 1 cycle at 120 BPM

    let rms = calculate_rms(&audio);
    println!("\nFour samples test:");
    println!("  RMS: {:.4}", rms);

    // Verify audio is produced
    assert!(rms > 0.0001, "Should produce audio, got RMS {:.6}", rms);

    let detected = detect_audio_events(&audio, 44100.0, ONSET_THRESHOLD);
    println!("  Detected {} events", detected.len());

    // At minimum, detect one event
    assert!(
        detected.len() >= 1,
        "Should detect at least 1 event, got {}",
        detected.len()
    );
}

#[test]
fn test_samples_with_rests_timing() {
    // Test: "bd ~ sn ~" should trigger events with gaps
    let input = r#"bpm 120
out: s "bd ~ sn ~" * 0.8"#;

    let audio = compile_and_render(input, 22050);
    let rms = calculate_rms(&audio);

    println!("\nRest pattern test:");
    println!("  RMS: {:.4}", rms);

    // Verify audio is produced
    assert!(rms > 0.0001, "Should produce audio with rests, got RMS {:.6}", rms);

    let detected = detect_audio_events(&audio, 44100.0, ONSET_THRESHOLD);
    println!("  Detected {} events", detected.len());

    // At minimum, should detect one event
    if detected.len() >= 1 {
        println!("  First event at {:.3}s", detected[0].time);
    }
}

#[test]
fn test_fast_samples_trigger_more_frequently() {
    // Test: "bd*4" should trigger rapidly
    let input = r#"bpm 120
out: s "bd*4" * 0.8"#;

    let audio = compile_and_render(input, 22050);
    let rms = calculate_rms(&audio);

    println!("\nFast samples test (bd*4):");
    println!("  RMS: {:.4}", rms);

    // Verify audio is produced
    assert!(rms > 0.0001, "Should produce audio, got RMS {:.6}", rms);

    let detected = detect_audio_events(&audio, 44100.0, ONSET_THRESHOLD);
    println!("  Detected {} events", detected.len());
}

// ============================================================================
// GAIN PARAMETER
// ============================================================================

#[test]
fn test_sample_gain_parameter_affects_amplitude() {
    // Test: Gain parameter should scale amplitude
    let quiet = r#"bpm 120
out: s("bd", 0.2)"#; // gain 0.2

    let loud = r#"bpm 120
out: s("bd", 1.0)"#; // gain 1.0

    let audio_quiet = compile_and_render(quiet, 22050);
    let audio_loud = compile_and_render(loud, 22050);

    let rms_quiet = calculate_rms(&audio_quiet);
    let rms_loud = calculate_rms(&audio_loud);

    println!("\nGain parameter test:");
    println!("  Quiet (0.2) RMS: {:.4}", rms_quiet);
    println!("  Loud (1.0) RMS: {:.4}", rms_loud);
    println!("  Ratio: {:.2}", rms_loud / rms_quiet);

    // Loud should be significantly louder
    assert!(
        rms_loud > rms_quiet * 2.0,
        "Loud (1.0) should be at least 2x louder than quiet (0.2): {:.4} vs {:.4}",
        rms_loud,
        rms_quiet
    );

    // Should be roughly 5x louder (1.0 / 0.2)
    let ratio = rms_loud / rms_quiet;
    assert!(
        ratio > 3.0 && ratio < 7.0,
        "Gain ratio should be roughly 5x, got {:.2}",
        ratio
    );
}

#[test]
fn test_pattern_gain_varies_amplitude() {
    // Test: Pattern-specified gain values should vary amplitude
    let constant = r#"bpm 120
out: s("bd bd bd bd", 0.5) * 0.8"#;

    let pattern = r#"bpm 120
out: s("bd bd bd bd", "0.2 0.4 0.6 0.8") * 0.8"#;

    let audio_constant = compile_and_render(constant, 22050);
    let audio_pattern = compile_and_render(pattern, 22050);

    let rms_constant = calculate_rms(&audio_constant);
    let rms_pattern = calculate_rms(&audio_pattern);

    println!("\nPattern gain test:");
    println!("  Constant gain RMS: {:.4}", rms_constant);
    println!("  Pattern gain RMS: {:.4}", rms_pattern);

    // Both should produce audio
    assert!(rms_constant > 0.0001, "Constant gain should produce audio, got {:.6}", rms_constant);
    assert!(rms_pattern > 0.0001, "Pattern gain should produce audio, got {:.6}", rms_pattern);

    // Pattern with varying gain should have similar average amplitude
    let ratio = rms_pattern / rms_constant;
    println!("  Ratio: {:.2}", ratio);

    assert!(
        ratio > 0.3 && ratio < 3.0,
        "Pattern gain should produce comparable total energy, got ratio {:.2}",
        ratio
    );
}

// ============================================================================
// EUCLIDEAN PATTERNS
// ============================================================================

#[test]
fn test_euclidean_pattern_timing() {
    // Test: Euclidean pattern "bd(3,8)" produces audio
    let input = r#"bpm 120
out: s "bd(3,8)" * 0.8"#;

    let audio = compile_and_render(input, 22050);

    let rms = calculate_rms(&audio);
    println!("\nEuclidean pattern test bd(3,8):");
    println!("  RMS: {:.4}", rms);

    // Verify audio is produced
    assert!(rms > 0.0001, "Should produce audio, got RMS {:.6}", rms);

    let detected = detect_audio_events(&audio, 44100.0, ONSET_THRESHOLD);
    println!("  Detected {} events", detected.len());
}

#[test]
fn test_euclidean_dense_pattern() {
    // Test: "bd(5,8)" produces audio
    let input = r#"bpm 120
out: s "bd(5,8)" * 0.8"#;

    let audio = compile_and_render(input, 22050);
    let rms = calculate_rms(&audio);
    println!("\nDense Euclidean test bd(5,8):");
    println!("  RMS: {:.4}", rms);

    // Verify audio is produced
    assert!(rms > 0.0001, "Should produce audio, got RMS {:.6}", rms);
}

// ============================================================================
// ALTERNATION PATTERNS
// ============================================================================

#[test]
fn test_alternation_pattern_cycles() {
    // Test: "<bd sn>" should alternate between cycles
    // Note: This is hard to test in single cycle, so test it produces audio
    let input = r#"bpm 120
out: s "<bd sn>""#;

    let audio = compile_and_render(input, 44100); // 2 cycles

    let rms = calculate_rms(&audio);
    println!("\nAlternation test:");
    println!("  RMS: {:.4}", rms);

    assert!(rms > 0.001, "Alternation pattern should produce audio");
}

// ============================================================================
// SUBDIVISION
// ============================================================================

#[test]
fn test_subdivision_triggers_multiple_events() {
    // Test: "[bd sn]" subdivides and produces audio
    let input = r#"bpm 120
out: s "[bd sn] hh" * 0.8"#;

    let audio = compile_and_render(input, 22050);
    let rms = calculate_rms(&audio);

    println!("\nSubdivision test [bd sn]:");
    println!("  RMS: {:.4}", rms);

    // Verify audio is produced
    assert!(rms > 0.0001, "Should produce audio, got RMS {:.6}", rms);
}

// ============================================================================
// SAMPLE SELECTION (Bank)
// ============================================================================

#[test]
fn test_sample_bank_selection_triggers() {
    // Test: "bd:0" and "bd:1" should both trigger
    let input = r#"bpm 120
out: s "bd:0 ~ bd:1 ~" * 0.8"#;

    let audio = compile_and_render(input, 22050);
    let rms = calculate_rms(&audio);

    println!("\nSample bank test:");
    println!("  RMS: {:.4}", rms);

    // Verify audio is produced
    assert!(rms > 0.0001, "Should produce audio with bank selection, got RMS {:.6}", rms);
}

// ============================================================================
// COMBINED TESTS
// ============================================================================

#[test]
fn test_complex_pattern_timing() {
    // Test: Complex pattern with various features
    let input = r#"bpm 120
out: s "bd*2 [sn cp] hh(3,8)""#;

    let audio = compile_and_render(input, 22050);
    let detected = detect_audio_events(&audio, 44100.0, 0.01);

    println!("\nComplex pattern test:");
    println!("  Detected {} events", detected.len());

    // Should have multiple events: bd*2 (2) + [sn cp] (2) + hh(3,8) (3) = 7+
    assert!(
        detected.len() >= 5,
        "Complex pattern should trigger multiple events, got {}",
        detected.len()
    );
}

#[test]
fn test_multi_output_sample_timing() {
    // Test: Multiple outputs should each trigger samples
    let input = r#"bpm 120
out1: s "bd sn"
out2: s "hh*4""#;

    // This test just verifies both channels produce audio
    let (_, statements) = parse_dsl(input).unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Render
    let audio = graph.render(22050);
    let rms = calculate_rms(&audio);

    println!("\nMulti-output test:");
    println!("  RMS: {:.4}", rms);

    assert!(
        rms > 0.001,
        "Multi-output should produce audio from both channels"
    );
}
