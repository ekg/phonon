//! Tests for Pitch Shifter
//!
//! Pitch shifting changes the pitch/frequency of audio without changing its duration.
//! This is different from simple resampling which changes both pitch and speed.
//!
//! Common techniques:
//! - Granular synthesis with variable playback rates
//! - Phase vocoder (FFT-based frequency domain processing)
//! - PSOLA (Pitch Synchronous Overlap-Add)
//!
//! This implementation uses granular synthesis for real-time pitch shifting.
//! Classic uses: Harmonizer effects, vocal pitch correction, creative sound design

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

/// Helper: Calculate RMS of a buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum: f32 = buffer.iter().map(|x| x * x).sum();
    (sum / buffer.len() as f32).sqrt()
}

// ========== LEVEL 1: Basic Functionality ==========

#[test]
fn test_pitch_shifter_produces_sound() {
    // Basic pitch shifter should produce sound
    // pitch_shift source semitones
    let code = r#"
tempo: 1.0
~source $ saw 220
out $ pitch_shift ~source 0
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    let buffer = graph.render(44100); // 1 second

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Pitch shifter should produce audible output, got RMS={}",
        rms
    );
}

#[test]
fn test_pitch_shifter_zero_shift() {
    // Zero semitone shift should approximate the input
    let code_original = r#"
tempo: 1.0
out $ saw 220
"#;

    let code_shifted = r#"
tempo: 1.0
~source $ saw 220
out $ pitch_shift ~source 0
"#;

    let (_, statements_orig) = parse_program(code_original).expect("Failed to parse");
    let mut graph_orig =
        compile_program(statements_orig, 44100.0, None).expect("Failed to compile");
    let buffer_orig = graph_orig.render(44100);

    let (_, statements_shift) = parse_program(code_shifted).expect("Failed to parse");
    let mut graph_shift =
        compile_program(statements_shift, 44100.0, None).expect("Failed to compile");
    let buffer_shift = graph_shift.render(44100);

    let rms_orig = calculate_rms(&buffer_orig);
    let rms_shift = calculate_rms(&buffer_shift);

    // Both should produce sound with similar energy
    assert!(rms_orig > 0.01, "Original should produce sound");
    assert!(rms_shift > 0.01, "Pitch shifted should produce sound");

    // RMS should be similar (within 60% due to granular artifacts and windowing)
    let ratio = rms_shift / rms_orig;
    assert!(
        ratio > 0.4 && ratio < 2.5,
        "Zero-shift should preserve energy approximately: ratio={}",
        ratio
    );
}

#[test]
fn test_pitch_shifter_octave_up() {
    // +12 semitones = octave up
    let code = r#"
tempo: 1.0
~source $ saw 220
out $ pitch_shift ~source 12
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    let buffer = graph.render(44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Octave up should produce sound, got RMS={}",
        rms
    );
}

#[test]
fn test_pitch_shifter_octave_down() {
    // -12 semitones = octave down
    let code = r#"
tempo: 1.0
~source $ saw 220
out $ pitch_shift ~source -12
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    let buffer = graph.render(44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Octave down should produce sound, got RMS={}",
        rms
    );
}

// ========== LEVEL 2: Different Shift Amounts ==========

#[test]
fn test_pitch_shifter_various_shifts() {
    // Test different semitone shifts
    let shifts = [-12, -7, -5, 0, 5, 7, 12]; // Octave, perfect fifth, perfect fourth, etc.

    for semitones in &shifts {
        let code = format!(
            r#"
tempo: 1.0
~source $ saw 220
out $ pitch_shift ~source {}
"#,
            semitones
        );

        let (rest, statements) = parse_program(&code).expect("Failed to parse");
        assert_eq!(rest.trim(), "", "Parser should consume all input");

        let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
        let buffer = graph.render(44100);

        let rms = calculate_rms(&buffer);
        assert!(
            rms > 0.01,
            "Pitch shift by {} semitones should produce sound, got RMS={}",
            semitones,
            rms
        );
    }
}

// ========== LEVEL 3: Pattern Modulation ==========

#[test]
fn test_pitch_shifter_pattern_shift() {
    // Pattern-modulated pitch shift (arpeggiator effect)
    let code = r#"
tempo: 0.5
~source $ saw 220
~shifts $ "0 7 12 7"
out $ pitch_shift ~source ~shifts
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(44100); // 1 second = 2 cycles

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Pattern-modulated pitch shift should produce sound, got RMS={}",
        rms
    );
}

// ========== LEVEL 4: Musical Examples ==========

#[test]
fn test_pitch_shifter_harmonizer() {
    // Harmonizer: mix original with shifted version
    let code = r#"
tempo: 1.0
~source $ saw 220
~shifted $ pitch_shift ~source 7
out $ (~source + ~shifted) * 0.5
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    let buffer = graph.render(44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Harmonizer should produce sound, got RMS={}",
        rms
    );
}

#[test]
fn test_pitch_shifter_chord() {
    // Create a chord from a single voice
    let code = r#"
tempo: 1.0
~source $ saw 220
~root $ pitch_shift ~source 0
~third $ pitch_shift ~source 4
~fifth $ pitch_shift ~source 7
out $ (~root + ~third + ~fifth) * 0.33
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    let buffer = graph.render(44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Chord from pitch shifter should produce sound, got RMS={}",
        rms
    );
}
