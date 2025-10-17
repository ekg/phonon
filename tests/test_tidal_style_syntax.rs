//! Tests for Tidal-style DSP parameter syntax
//!
//! Phonon uses Tidal/TidalCycles style syntax for DSP parameters:
//! - s "bd" # gain 0.5
//! - s "bd sn" # gain "0.5 1.0"
//! - s "bd" # pan "-1 1" # speed 2.0
//!
//! NOT positional args like s("bd", 0.5)

use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

mod audio_test_utils;
use audio_test_utils::calculate_rms;

/// Helper to compile and render DSL
fn compile_and_render(input: &str, duration_samples: usize) -> Vec<f32> {
    let (_, statements) = parse_dsl(input).expect("Failed to parse DSL");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    graph.render(duration_samples)
}

// ============================================================================
// GAIN - Tidal Style
// ============================================================================

#[test]
fn test_tidal_gain_constant() {
    // Test: s "bd" # gain 0.5
    let quiet = r#"bpm 120
out: s "bd" # gain 0.3 * 50"#;

    let loud = r#"bpm 120
out: s "bd" # gain 1.0 * 50"#;

    let audio_quiet = compile_and_render(quiet, 22050);
    let audio_loud = compile_and_render(loud, 22050);

    let rms_quiet = calculate_rms(&audio_quiet);
    let rms_loud = calculate_rms(&audio_loud);

    println!("\nTidal gain test:");
    println!("  Quiet (gain 0.3) RMS: {:.4}", rms_quiet);
    println!("  Loud (gain 1.0) RMS: {:.4}", rms_loud);
    println!("  Ratio: {:.2}", rms_loud / rms_quiet);

    // Both should produce audio
    assert!(rms_quiet > 0.0001, "Quiet should produce audio");
    assert!(rms_loud > 0.0001, "Loud should produce audio");

    // Loud should be significantly louder
    let ratio = rms_loud / rms_quiet;
    assert!(
        ratio > 2.0 && ratio < 5.0,
        "gain 1.0 should be ~3.3x louder than gain 0.3, got ratio {:.2}",
        ratio
    );
}

#[test]
fn test_tidal_gain_zero() {
    // Test: gain 0 should produce silence
    let silent = r#"bpm 120
out: s "bd" # gain 0.0 * 50"#;

    let audio = compile_and_render(silent, 22050);
    let rms = calculate_rms(&audio);

    println!("\nTidal gain=0 test:");
    println!("  RMS: {:.6}", rms);

    assert!(
        rms < 0.001,
        "gain 0 should produce silence, got RMS {:.6}",
        rms
    );
}

#[test]
fn test_tidal_gain_pattern() {
    // Test: s "bd bd bd bd" # gain "0.2 0.4 0.6 0.8"
    let pattern = r#"bpm 120
out: s "bd bd bd bd" # gain "0.2 0.4 0.6 0.8" * 50"#;

    let audio = compile_and_render(pattern, 22050);
    let rms = calculate_rms(&audio);

    println!("\nTidal gain pattern test:");
    println!("  RMS: {:.4}", rms);

    assert!(rms > 0.0005, "Gain pattern should produce audio");
}

// ============================================================================
// PAN - Tidal Style
// ============================================================================

#[test]
fn test_tidal_pan_left() {
    // Test: s "bd" # pan -1.0
    let left = r#"bpm 120
out: s "bd" # pan -1.0 * 50"#;

    let audio = compile_and_render(left, 22050);
    let rms = calculate_rms(&audio);

    println!("\nTidal pan left test:");
    println!("  RMS: {:.4}", rms);

    assert!(rms > 0.0001, "Pan left should produce audio");
}

#[test]
fn test_tidal_pan_pattern() {
    // Test: s "bd sn hh cp" # pan "-1 -0.5 0.5 1"
    let pattern = r#"bpm 120
out: s "bd sn hh cp" # pan "-1 -0.5 0.5 1" * 50"#;

    let audio = compile_and_render(pattern, 22050);
    let rms = calculate_rms(&audio);

    println!("\nTidal pan pattern test:");
    println!("  RMS: {:.4}", rms);

    assert!(rms > 0.0001, "Pan pattern should produce audio");
}

// ============================================================================
// SPEED - Tidal Style
// ============================================================================

#[test]
fn test_tidal_speed_double() {
    // Test: s "bd" # speed 2.0
    let fast = r#"bpm 120
out: s "bd" # speed 2.0 * 50"#;

    let audio = compile_and_render(fast, 22050);
    let rms = calculate_rms(&audio);

    println!("\nTidal speed 2.0 test:");
    println!("  RMS: {:.4}", rms);

    assert!(rms > 0.0001, "Speed 2.0 should produce audio");
}

#[test]
fn test_tidal_speed_pattern() {
    // Test: s "bd bd bd bd" # speed "1 0.5 2 1.5"
    let pattern = r#"bpm 120
out: s "bd bd bd bd" # speed "1 0.5 2 1.5" * 50"#;

    let audio = compile_and_render(pattern, 22050);
    let rms = calculate_rms(&audio);

    println!("\nTidal speed pattern test:");
    println!("  RMS: {:.4}", rms);

    assert!(rms > 0.0001, "Speed pattern should produce audio");
}

// ============================================================================
// CUT - Tidal Style
// ============================================================================

#[test]
fn test_tidal_cut_group() {
    // Test: s "hh*16" # cut 1
    let cut = r#"bpm 120
out: s "hh*16" # cut 1 * 50"#;

    let audio = compile_and_render(cut, 22050);
    let rms = calculate_rms(&audio);

    println!("\nTidal cut group test:");
    println!("  RMS: {:.4}", rms);

    assert!(rms > 0.0001, "Cut group should produce audio");
}

// ============================================================================
// CHAINED MODIFIERS - Tidal Style
// ============================================================================

#[test]
fn test_tidal_multiple_modifiers() {
    // Test: s "bd" # gain 0.8 # pan -0.5 # speed 1.2
    let multi = r#"bpm 120
out: s "bd" # gain 0.8 # pan -0.5 # speed 1.2 * 50"#;

    let audio = compile_and_render(multi, 22050);
    let rms = calculate_rms(&audio);

    println!("\nTidal multiple modifiers test:");
    println!("  RMS: {:.4}", rms);

    assert!(rms > 0.0001, "Multiple modifiers should produce audio");
}

#[test]
fn test_tidal_complex_example() {
    // Test: Full Tidal-style syntax with patterns
    let example = r#"bpm 120
~kick: s "bd*8" # gain "1.0 0.7 0.8 0.6" # pan "-1 -0.5 0 0.5"
~hats: s "hh*16" # gain "0.7 0.4" # pan "-1 1"
out: (~kick + ~hats) * 50"#;

    let audio = compile_and_render(example, 22050);
    let rms = calculate_rms(&audio);

    println!("\nTidal complex example test:");
    println!("  RMS: {:.4}", rms);

    assert!(rms > 0.0005, "Complex example should produce audio");
}
