//! Tests for Granular Synthesis
//!
//! Granular synthesis breaks audio into small grains (5-100ms) and overlaps them
//! with varying speeds, pitches, and densities. Classic technique used in Reaktor,
//! Ableton Granulator, Max/MSP.

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

/// Helper: Calculate RMS of a buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum: f32 = buffer.iter().map(|x| x * x).sum();
    (sum / buffer.len() as f32).sqrt()
}

// ========== LEVEL 1: Basic Functionality ==========

#[test]
fn test_granular_produces_sound() {
    // Simple test: granular synthesizer produces non-zero output
    // Using a sine wave as source
    let code = r#"
tempo: 1.0
~source $ sine 440
out $ granular ~source 50 0.1 1.0
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    let buffer = graph.render(44100); // 1 second

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Granular should produce audible output, got RMS={}",
        rms
    );
}

#[test]
fn test_granular_grain_size() {
    // Test different grain sizes
    let grain_sizes = [10.0, 50.0, 100.0]; // milliseconds

    for grain_ms in &grain_sizes {
        let code = format!(
            r#"
tempo: 1.0
~source $ sine 440
out $ granular ~source {} 0.5 1.0
"#,
            grain_ms
        );

        let (rest, statements) = parse_program(&code).expect("Failed to parse");
        assert_eq!(rest.trim(), "", "Parser should consume all input");

        let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
        let buffer = graph.render(44100);

        let rms = calculate_rms(&buffer);
        assert!(
            rms > 0.01,
            "Granular with grain_size={}ms should produce sound, got RMS={}",
            grain_ms,
            rms
        );
    }
}

#[test]
fn test_granular_density() {
    // Higher density should produce louder output (more overlapping grains)
    let code_sparse = r#"
tempo: 1.0
~source $ sine 440
out $ granular ~source 50 0.1 1.0
"#;

    let code_dense = r#"
tempo: 1.0
~source $ sine 440
out $ granular ~source 50 0.9 1.0
"#;

    let (_, statements_sparse) = parse_program(code_sparse).expect("Failed to parse");
    let mut graph_sparse =
        compile_program(statements_sparse, 44100.0, None).expect("Failed to compile");
    let buffer_sparse = graph_sparse.render(44100);
    let rms_sparse = calculate_rms(&buffer_sparse);

    let (_, statements_dense) = parse_program(code_dense).expect("Failed to parse");
    let mut graph_dense =
        compile_program(statements_dense, 44100.0, None).expect("Failed to compile");
    let buffer_dense = graph_dense.render(44100);
    let rms_dense = calculate_rms(&buffer_dense);

    assert!(
        rms_dense > rms_sparse,
        "Dense granular should be louder than sparse: {} vs {}",
        rms_dense,
        rms_sparse
    );
}

// ========== LEVEL 2: Pitch/Speed Control ==========

#[test]
fn test_granular_pitch_shift() {
    // Pitch parameter should affect playback rate
    let code = r#"
tempo: 1.0
~source $ sine 440
out $ granular ~source 50 0.5 2.0
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    let buffer = graph.render(44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Pitch-shifted granular should produce sound, got RMS={}",
        rms
    );
}

// ========== LEVEL 3: Pattern Modulation ==========

#[test]
fn test_granular_pattern_grain_size() {
    // Pattern-modulated grain size
    let code = r#"
tempo: 0.5
~source $ sine 440
out $ granular ~source "25 50 100" 0.5 1.0
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Pattern-modulated grain size should produce sound, got RMS={}",
        rms
    );
}

#[test]
fn test_granular_pattern_density() {
    // Pattern-modulated density
    let code = r#"
tempo: 0.5
~source $ sine 440
out $ granular ~source 50 "0.3 0.7 0.5" 1.0
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Pattern-modulated density should produce sound, got RMS={}",
        rms
    );
}

// ========== LEVEL 4: Musical Examples ==========

#[test]
fn test_granular_ambient_pad() {
    // Granular synthesis for ambient textures
    let code = r#"
tempo: 1.0
~source $ sine "110 165 220" $ slow 4
out $ granular ~source 100 0.7 0.8 * 0.3
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    let buffer = graph.render(44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.005,
        "Granular ambient pad should produce sound, got RMS={}",
        rms
    );
}

#[test]
fn test_granular_rhythmic() {
    // Granular synthesis with rhythmic patterns
    let code = r#"
tempo: 0.5
~source $ sine 220
~density_pattern $ "0.9 0.3 0.6 0.2"
out $ granular ~source 30 ~density_pattern 1.0 * 0.4
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.005,
        "Rhythmic granular should produce sound, got RMS={}",
        rms
    );
}
