//! Tests for FFT/Spectral Processing
//!
//! FFT (Fast Fourier Transform) converts time-domain audio to frequency domain,
//! enabling spectral manipulation like freeze, filtering, and phase vocoder effects.
//!
//! Common applications:
//! - Spectral freeze (hold current spectrum)
//! - FFT filtering (frequency-selective processing)
//! - Phase vocoder (time/pitch manipulation)
//! - Spectral effects (robotization, whisperization)
//!
//! NOTE: FFT/Spectral processing requires significant infrastructure:
//! - FFT library integration (realfft)
//! - Overlap-add buffering (typically 75% overlap)
//! - Phase preservation/manipulation
//! - IFFT reconstruction
//! - Window functions (Hann, Hamming)
//!
//! This is marked as TODO/future work due to complexity.
//! All tests are ignored until implementation is complete.

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

/// Helper: Calculate RMS of a buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum: f32 = buffer.iter().map(|x| x * x).sum();
    (sum / buffer.len() as f32).sqrt()
}

// ========== LEVEL 1: Basic Functionality ==========

#[test]
fn test_spectral_freeze_produces_sound() {
    // Spectral freeze should capture and hold a spectrum
    // freeze source trigger
    let code = r#"
tempo: 1.0
~source: sine 440
~trigger: "x ~ ~ ~"
out: freeze ~source ~trigger
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    let buffer = graph.render(44100); // 1 second

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Spectral freeze should produce audible output, got RMS={}",
        rms
    );
}

#[test]
fn test_spectral_freeze_without_trigger() {
    // Without trigger, freeze should pass through or be silent
    let code = r#"
tempo: 1.0
~source: sine 440
~trigger: "~ ~ ~ ~"
out: freeze ~source ~trigger
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    let buffer = graph.render(44100);

    let rms = calculate_rms(&buffer);
    // Should be silent or very quiet since no freeze triggers
    println!("Freeze without trigger RMS: {}", rms);
}

// ========== LEVEL 2: Different Sources ==========

#[test]
fn test_spectral_freeze_complex_source() {
    // Freeze should work with complex sounds (chords, noise, etc.)
    let code = r#"
tempo: 2.0
~chord: saw "110 220 330"
~trigger: "x ~ x ~"
out: freeze ~chord ~trigger * 0.3
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);
    let buffer = graph.render(44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Spectral freeze with complex source should produce sound, got RMS={}",
        rms
    );
}

// ========== LEVEL 3: Pattern Modulation ==========

#[test]
fn test_spectral_freeze_pattern_trigger() {
    // Pattern-modulated freeze triggers
    let code = r#"
tempo: 4.0
~source: sine "220 330 440 550"
~trigger: "x x ~ x"
out: freeze ~source ~trigger * 0.3
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(4.0);
    let buffer = graph.render(44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Pattern-modulated freeze should produce sound, got RMS={}",
        rms
    );
}
