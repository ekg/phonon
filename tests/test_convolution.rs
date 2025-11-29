//! Tests for Convolution Reverb
//!
//! Convolution reverb uses impulse responses (IRs) to recreate realistic acoustic spaces.
//! This is the gold standard for realistic reverb - captures actual room acoustics.
//!
//! Implementation approaches:
//! 1. Direct time-domain convolution (simple but slow for long IRs)
//! 2. FFT-based fast convolution (efficient for long IRs)
//! 3. Partitioned convolution (optimal for real-time, splits IR into chunks)
//!
//! For now, we'll use direct convolution with shorter IRs for simplicity.

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

/// Helper: Calculate RMS of a buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum: f32 = buffer.iter().map(|x| x * x).sum();
    (sum / buffer.len() as f32).sqrt()
}

// ========== LEVEL 1: Basic Functionality ==========

#[test]
fn test_convolution_produces_sound() {
    // Simple convolution with built-in IR (short delay-like response)
    // convolve input
    let code = r#"
tempo: 1.0
~source $ saw 220
out $ convolve ~source
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    let buffer = graph.render(44100); // 1 second

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Convolution should produce audible output, got RMS={}",
        rms
    );
}

#[test]
fn test_convolution_with_impulse() {
    // Convolution should work as a simple reverb
    let code_dry = r#"
tempo: 1.0
~source $ saw 220
out $ ~source
"#;

    let code_wet = r#"
tempo: 1.0
~source $ saw 220
out $ convolve ~source
"#;

    let (_, statements_dry) = parse_program(code_dry).expect("Failed to parse");
    let mut graph_dry = compile_program(statements_dry, 44100.0, None).expect("Failed to compile");
    let buffer_dry = graph_dry.render(44100);

    let (_, statements_wet) = parse_program(code_wet).expect("Failed to parse");
    let mut graph_wet = compile_program(statements_wet, 44100.0, None).expect("Failed to compile");
    let buffer_wet = graph_wet.render(44100);

    let rms_dry = calculate_rms(&buffer_dry);
    let rms_wet = calculate_rms(&buffer_wet);

    // Both should produce sound
    assert!(rms_dry > 0.01, "Dry signal should produce sound");
    assert!(rms_wet > 0.01, "Convolution should produce sound");

    println!("Dry RMS: {}, Wet RMS: {}", rms_dry, rms_wet);
}

// ========== LEVEL 2: Different Sources ==========

#[test]
fn test_convolution_percussion() {
    // Convolution creates realistic room acoustics for percussion
    // Using a simple impulse (clap-like sound)
    let code = r#"
tempo: 0.5
~kick $ saw "55 ~ 82.5 ~" * 0.5
out $ convolve ~kick * 0.3
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);
    let buffer = graph.render(88200); // 2 seconds

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.005,
        "Convolution with percussion should produce sound, got RMS={}",
        rms
    );
}

// ========== LEVEL 3: Musical Examples ==========

#[test]
fn test_convolution_with_chords() {
    // Convolution adds space and depth to chord progressions
    let code = r#"
tempo: 1.0
~chord $ saw "110 165 220" $ slow 2
out $ convolve ~chord * 0.2
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    let buffer = graph.render(88200); // 2 seconds

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Convolution with chords should produce sound, got RMS={}",
        rms
    );
}
