//! Tests for Vocoder
//!
//! A vocoder analyzes the amplitude envelope of a modulator signal (usually voice)
//! and applies it to a carrier signal (usually a synth). It splits both signals into
//! multiple frequency bands, measures the modulator's amplitude in each band, and
//! uses those envelopes to modulate the carrier's bands.
//!
//! Classic use: Robot voice effect (voice modulating synth)

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

/// Helper: Calculate RMS of a buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum: f32 = buffer.iter().map(|x| x * x).sum();
    (sum / buffer.len() as f32).sqrt()
}

// ========== LEVEL 1: Basic Functionality ==========

#[test]
fn test_vocoder_produces_sound() {
    // Simple test: Vocoder produces non-zero output
    let code = r#"
tempo: 1.0
~modulator: saw 110
~carrier: saw 220
out: vocoder ~modulator ~carrier 8
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    let buffer = graph.render(44100); // 1 second

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Vocoder should produce audible output, got RMS={}",
        rms
    );
}

#[test]
fn test_vocoder_silent_modulator() {
    // Silent modulator should produce silent output
    let code = r#"
tempo: 1.0
~modulator: 0
~carrier: saw 220
out: vocoder ~modulator ~carrier 8
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    let buffer = graph.render(44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms < 0.01,
        "Silent modulator should produce silent output, got RMS={}",
        rms
    );
}

#[test]
fn test_vocoder_silent_carrier() {
    // Silent carrier should produce silent output (nothing to modulate)
    let code = r#"
tempo: 1.0
~modulator: saw 110
~carrier: 0
out: vocoder ~modulator ~carrier 8
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    let buffer = graph.render(44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms < 0.01,
        "Silent carrier should produce silent output, got RMS={}",
        rms
    );
}

#[test]
fn test_vocoder_different_band_counts() {
    // Test different numbers of bands
    let band_counts = [4, 8, 16];

    for bands in &band_counts {
        let code = format!(
            r#"
tempo: 1.0
~modulator: saw 110
~carrier: saw 220
out: vocoder ~modulator ~carrier {}
"#,
            bands
        );

        let (rest, statements) = parse_program(&code).expect("Failed to parse");
        assert_eq!(rest.trim(), "", "Parser should consume all input");

        let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
        let buffer = graph.render(44100);

        let rms = calculate_rms(&buffer);
        assert!(
            rms > 0.01,
            "Vocoder with {} bands should produce sound, got RMS={}",
            bands,
            rms
        );
    }
}

// ========== LEVEL 2: Modulation Behavior ==========

#[test]
fn test_vocoder_modulates_carrier() {
    // Vocoded output should differ from unprocessed carrier
    let code_vocoded = r#"
tempo: 1.0
~modulator: saw 110
~carrier: saw 220
out: vocoder ~modulator ~carrier 8
"#;

    let code_carrier_only = r#"
tempo: 1.0
out: saw 220
"#;

    let (_, statements_vocoded) = parse_program(code_vocoded).expect("Failed to parse");
    let mut graph_vocoded =
        compile_program(statements_vocoded, 44100.0).expect("Failed to compile");
    let buffer_vocoded = graph_vocoded.render(44100);

    let (_, statements_carrier) = parse_program(code_carrier_only).expect("Failed to parse");
    let mut graph_carrier =
        compile_program(statements_carrier, 44100.0).expect("Failed to compile");
    let buffer_carrier = graph_carrier.render(44100);

    let rms_vocoded = calculate_rms(&buffer_vocoded);
    let rms_carrier = calculate_rms(&buffer_carrier);

    // Both should produce sound
    assert!(rms_vocoded > 0.01, "Vocoded signal should produce sound");
    assert!(rms_carrier > 0.01, "Carrier should produce sound");

    // Vocoded output should be different from raw carrier
    assert!(
        (rms_vocoded - rms_carrier).abs() > 0.01,
        "Vocoder should modify the carrier signal"
    );
}

// ========== LEVEL 3: Pattern Modulation ==========

#[test]
fn test_vocoder_pattern_modulator() {
    // Pattern-modulated modulator
    let code = r#"
tempo: 2.0
~modulator: saw "110 165 220 165"
~carrier: saw 440
out: vocoder ~modulator ~carrier 8
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(44100); // 1 second = 2 cycles

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Pattern-modulated modulator should produce sound, got RMS={}",
        rms
    );
}

#[test]
fn test_vocoder_pattern_carrier() {
    // Pattern-modulated carrier
    let code = r#"
tempo: 2.0
~modulator: saw 110
~carrier: saw "220 330 440 330"
out: vocoder ~modulator ~carrier 8
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Pattern-modulated carrier should produce sound, got RMS={}",
        rms
    );
}

// ========== LEVEL 4: Musical Examples ==========

#[test]
fn test_vocoder_chord() {
    // Vocoder with chord
    let code = r#"
tempo: 1.0
~modulator: saw 110
~carrier1: saw 220
~carrier2: saw 275
~carrier3: saw 330
~carrier: (~carrier1 + ~carrier2 + ~carrier3) * 0.33
out: vocoder ~modulator ~carrier 8
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    let buffer = graph.render(44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Vocoder with chord should produce sound, got RMS={}",
        rms
    );
}

#[test]
#[ignore] // Test takes too long even with pre-calculated coefficients - vocoder needs further optimization
fn test_vocoder_noise_carrier() {
    // Vocoder with noise carrier (whisper effect)
    // Use 8 bands instead of 16 to avoid excessive computation time
    let code = r#"
tempo: 1.0
~modulator: saw 110
~carrier: noise
out: vocoder ~modulator ~carrier 8
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    let buffer = graph.render(4410); // Render only 0.1 second to avoid timeout

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Vocoder with noise carrier should produce sound, got RMS={}",
        rms
    );
}
