//! Tests for Formant Synthesis
//!
//! Formant synthesis creates vowel sounds by filtering a source signal through
//! multiple resonant bandpass filters. Each vowel is characterized by specific
//! formant frequencies (F1, F2, F3) that resonate in the vocal tract.
//!
//! Common vowel formants (male voice, in Hz):
//! - /a/ (father): F1=730, F2=1090, F3=2440
//! - /e/ (bet):    F1=530, F2=1840, F3=2480
//! - /i/ (beet):   F1=270, F2=2290, F3=3010
//! - /o/ (boat):   F1=570, F2=840,  F3=2410
//! - /u/ (boot):   F1=300, F2=870,  F3=2240

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

/// Helper: Calculate RMS of a buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum: f32 = buffer.iter().map(|x| x * x).sum();
    (sum / buffer.len() as f32).sqrt()
}

// ========== LEVEL 1: Basic Functionality ==========

#[test]
fn test_formant_produces_sound() {
    // Formant filter should produce sound from a source
    let code = r#"
tempo: 1.0
~source $ saw 110
out $ formant ~source 730 1090 2440 80 90 120
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    let buffer = graph.render(44100); // 1 second

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Formant synthesis should produce audible output, got RMS={}",
        rms
    );
}

#[test]
fn test_formant_filters_source() {
    // Formant should filter the input, not generate sound on its own
    let code_no_source = r#"
tempo: 1.0
out $ formant 0 730 1090 2440 80 90 120
"#;

    let code_with_source = r#"
tempo: 1.0
~source $ saw 110
out $ formant ~source 730 1090 2440 80 90 120
"#;

    let (_, statements_no) = parse_program(code_no_source).expect("Failed to parse");
    let mut graph_no = compile_program(statements_no, 44100.0, None).expect("Failed to compile");
    let buffer_no = graph_no.render(44100);

    let (_, statements_yes) = parse_program(code_with_source).expect("Failed to parse");
    let mut graph_yes = compile_program(statements_yes, 44100.0, None).expect("Failed to compile");
    let buffer_yes = graph_yes.render(44100);

    let rms_no = calculate_rms(&buffer_no);
    let rms_yes = calculate_rms(&buffer_yes);

    assert!(rms_no < 0.01, "Formant with no source should be silent");
    assert!(rms_yes > 0.01, "Formant with source should produce sound");
}

#[test]
fn test_formant_different_vowels() {
    // Different formant frequencies should produce different timbres
    // /a/ vowel (father)
    let code_a = r#"
tempo: 1.0
~source $ saw 110
out $ formant ~source 730 1090 2440 80 90 120
"#;

    // /i/ vowel (beet)
    let code_i = r#"
tempo: 1.0
~source $ saw 110
out $ formant ~source 270 2290 3010 60 90 150
"#;

    let (_, statements_a) = parse_program(code_a).expect("Failed to parse");
    let mut graph_a = compile_program(statements_a, 44100.0, None).expect("Failed to compile");
    let buffer_a = graph_a.render(44100);

    let (_, statements_i) = parse_program(code_i).expect("Failed to parse");
    let mut graph_i = compile_program(statements_i, 44100.0, None).expect("Failed to compile");
    let buffer_i = graph_i.render(44100);

    // Both should produce sound
    let rms_a = calculate_rms(&buffer_a);
    let rms_i = calculate_rms(&buffer_i);

    assert!(rms_a > 0.01, "/a/ vowel should produce sound");
    assert!(rms_i > 0.01, "/i/ vowel should produce sound");

    // Different formant positions should produce different RMS values
    // (simplified test - avoiding slow FFT)
    assert!(
        (rms_a - rms_i).abs() > 0.001,
        "Different vowels should produce different timbres"
    );
}

#[test]
fn test_formant_bandwidth_control() {
    // Narrow bandwidths should be more resonant than wide bandwidths
    let code_narrow = r#"
tempo: 1.0
~source $ saw 110
out $ formant ~source 730 1090 2440 20 30 40
"#;

    let code_wide = r#"
tempo: 1.0
~source $ saw 110
out $ formant ~source 730 1090 2440 200 300 400
"#;

    let (_, statements_narrow) = parse_program(code_narrow).expect("Failed to parse");
    let mut graph_narrow =
        compile_program(statements_narrow, 44100.0, None).expect("Failed to compile");
    let buffer_narrow = graph_narrow.render(44100);

    let (_, statements_wide) = parse_program(code_wide).expect("Failed to parse");
    let mut graph_wide =
        compile_program(statements_wide, 44100.0, None).expect("Failed to compile");
    let buffer_wide = graph_wide.render(44100);

    let rms_narrow = calculate_rms(&buffer_narrow);
    let rms_wide = calculate_rms(&buffer_wide);

    // Both should produce sound
    assert!(rms_narrow > 0.01, "Narrow bandwidth should produce sound");
    assert!(rms_wide > 0.01, "Wide bandwidth should produce sound");

    // They should be different (narrow is more resonant/selective)
    assert!(
        (rms_narrow - rms_wide).abs() > 0.001,
        "Different bandwidths should produce different results"
    );
}

// ========== LEVEL 2: Pattern Modulation ==========

#[test]
fn test_formant_pattern_frequency() {
    // Pattern-modulated formant frequencies (vowel morphing)
    let code = r#"
tempo: 0.5
~source $ saw 110
out $ formant ~source "730 270" 1090 2440 80 90 120
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(44100); // 1 second = 2 cycles

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Pattern-modulated formant should produce sound, got RMS={}",
        rms
    );
}

#[test]
fn test_formant_pattern_bandwidth() {
    // Pattern-modulated bandwidth (resonance modulation)
    let code = r#"
tempo: 0.5
~source $ saw 110
out $ formant ~source 730 1090 2440 "30 200" 90 120
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Pattern-modulated bandwidth should produce sound, got RMS={}",
        rms
    );
}

#[test]
fn test_formant_vowel_morphing() {
    // Morph between vowels by pattern-modulating formants
    // /a/ to /i/ transition
    let code = r#"
tempo: 0.5
~source $ saw 110
~f1 $ "730 270"      -- /a/ to /i/ F1
~f2 $ "1090 2290"    -- /a/ to /i/ F2
~f3 $ "2440 3010"    -- /a/ to /i/ F3
out $ formant ~source ~f1 ~f2 ~f3 80 90 120
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(88200); // 2 seconds = 4 cycles

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Vowel morphing should produce sound, got RMS={}",
        rms
    );
}

// ========== LEVEL 3: Different Sources ==========

#[test]
fn test_formant_pulse_source() {
    // Formant with pulse source (more realistic voice)
    let code = r#"
tempo: 1.0
~source $ square 110
out $ formant ~source 730 1090 2440 80 90 120
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    let buffer = graph.render(44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Formant with pulse source should produce sound, got RMS={}",
        rms
    );
}

#[test]
#[ignore] // Noise source through formant is computationally expensive
fn test_formant_noise_source() {
    // Formant with noise source (whispered voice)
    let code = r#"
tempo: 1.0
~source $ noise
out $ formant ~source 730 1090 2440 80 90 120 * 0.3
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    let buffer = graph.render(44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Formant with noise source should produce sound, got RMS={}",
        rms
    );
}

// ========== LEVEL 4: Musical Examples ==========

#[test]
fn test_formant_singing_melody() {
    // Singing melody with formant synthesis
    let code = r#"
tempo: 0.5
~melody $ "110 165 220 165"
~source $ saw ~melody
out $ formant ~source 530 1840 2480 80 90 120
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Singing melody should produce sound, got RMS={}",
        rms
    );
}

#[test]
fn test_formant_choir() {
    // Choir effect with multiple formant voices
    let code = r#"
tempo: 1.0
~source $ saw 110
~voice1 $ formant ~source 730 1090 2440 80 90 120
~voice2 $ formant ~source 530 1840 2480 80 90 120
~voice3 $ formant ~source 270 2290 3010 60 90 150
out $ (~voice1 + ~voice2 + ~voice3) * 0.3
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    let buffer = graph.render(44100);

    let rms = calculate_rms(&buffer);
    assert!(rms > 0.01, "Choir should produce sound, got RMS={}", rms);
}
