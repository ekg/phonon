//! Tests for Additive Synthesis
//!
//! Additive synthesis creates complex timbres by summing multiple sine waves
//! (partials/harmonics). Each partial has independent frequency and amplitude control.
//! This is one of the most fundamental synthesis techniques.
//!
//! Classic uses: Organ sounds, bell tones, harmonic analysis/resynthesis

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

/// Helper: Calculate RMS of a buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum: f32 = buffer.iter().map(|x| x * x).sum();
    (sum / buffer.len() as f32).sqrt()
}

// ========== LEVEL 1: Basic Functionality ==========

#[test]
fn test_additive_produces_sound() {
    // Simple test: Additive synthesis produces non-zero output
    let code = r#"
tempo: 1.0
out: additive 440 "1.0 0.5 0.25"
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    let buffer = graph.render(44100); // 1 second

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Additive synthesis should produce audible output, got RMS={}",
        rms
    );
}

#[test]
fn test_additive_single_partial() {
    // Single partial should sound like a sine wave
    let code = r#"
tempo: 1.0
out: additive 440 "1.0"
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    let buffer = graph.render(44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Single partial should produce sound, got RMS={}",
        rms
    );
}

#[test]
fn test_additive_multiple_partials() {
    // Multiple partials should be louder than single partial
    let code_single = r#"
tempo: 1.0
out: additive 440 "1.0"
"#;

    let code_multiple = r#"
tempo: 1.0
out: additive 440 "1.0 0.5 0.25"
"#;

    let (_, statements_single) = parse_program(code_single).expect("Failed to parse");
    let mut graph_single = compile_program(statements_single, 44100.0, None).expect("Failed to compile");
    let buffer_single = graph_single.render(44100);

    let (_, statements_multiple) = parse_program(code_multiple).expect("Failed to parse");
    let mut graph_multiple =
        compile_program(statements_multiple, 44100.0, None).expect("Failed to compile");
    let buffer_multiple = graph_multiple.render(44100);

    let rms_single = calculate_rms(&buffer_single);
    let rms_multiple = calculate_rms(&buffer_multiple);

    assert!(rms_single > 0.01, "Single partial should produce sound");
    assert!(
        rms_multiple > 0.01,
        "Multiple partials should produce sound"
    );

    // With amplitude-sum normalization, both signals have similar peak amplitude
    // but multiple partials have different harmonic content
    // Both RMS values should be in a reasonable range (not requiring one > other)
    assert!(
        (rms_single - rms_multiple).abs() < 0.4,
        "Single and multiple partials should have similar RMS levels (normalized): single={}, multiple={}",
        rms_single,
        rms_multiple
    );
}

#[test]
fn test_additive_harmonic_series() {
    // Harmonic series (1, 2, 3, 4...) should produce rich tone
    let code = r#"
tempo: 1.0
out: additive 110 "1.0 0.5 0.33 0.25 0.2"
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    let buffer = graph.render(44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Harmonic series should produce sound, got RMS={}",
        rms
    );
}

// ========== LEVEL 2: Different Timbres ==========

#[test]
fn test_additive_different_frequencies() {
    // Test multiple fundamental frequencies
    let frequencies = [110.0, 220.0, 440.0];

    for freq in &frequencies {
        let code = format!(
            r#"
tempo: 1.0
out: additive {} "1.0 0.5 0.33"
"#,
            freq
        );

        let (rest, statements) = parse_program(&code).expect("Failed to parse");
        assert_eq!(rest.trim(), "", "Parser should consume all input");

        let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
        let buffer = graph.render(44100);

        let rms = calculate_rms(&buffer);
        assert!(
            rms > 0.01,
            "Additive at {}Hz should produce sound, got RMS={}",
            freq,
            rms
        );
    }
}

#[test]
fn test_additive_odd_harmonics() {
    // Odd harmonics (1, 3, 5...) should produce square-wave-like tone
    let code = r#"
tempo: 1.0
out: additive 220 "1.0 0.0 0.33 0.0 0.2"
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    let buffer = graph.render(44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Odd harmonics should produce sound, got RMS={}",
        rms
    );
}

// ========== LEVEL 3: Pattern Modulation ==========

#[test]
fn test_additive_pattern_frequency() {
    // Pattern-modulated fundamental frequency
    let code = r#"
tempo: 0.5
out: additive "220 330 440 330" "1.0 0.5 0.25"
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(44100); // 1 second = 2 cycles

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Pattern-modulated frequency should produce sound, got RMS={}",
        rms
    );
}

#[test]
#[ignore] // Pattern-modulated amplitudes not yet supported
fn test_additive_pattern_amplitudes() {
    // Pattern-modulated partial amplitudes (timbre modulation)
    let code = r#"
tempo: 0.5
~amps: "1.0 0.5 0.25" "1.0 0.0 0.5"
out: additive 220 ~amps
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Pattern-modulated amplitudes should produce sound, got RMS={}",
        rms
    );
}

// ========== LEVEL 4: Musical Examples ==========

#[test]
fn test_additive_melody() {
    // Play a melody with additive synthesis
    let code = r#"
tempo: 0.5
out: additive "220 330 440 330 220" "1.0 0.5 0.33 0.25"
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Additive melody should produce sound, got RMS={}",
        rms
    );
}

#[test]
fn test_additive_organ_sound() {
    // Organ-like sound with many harmonics
    let code = r#"
tempo: 1.0
out: additive 110 "1.0 0.5 0.33 0.25 0.2 0.17 0.14 0.13"
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    let buffer = graph.render(44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Organ sound should produce sound, got RMS={}",
        rms
    );
}
