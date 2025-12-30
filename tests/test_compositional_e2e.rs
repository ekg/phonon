//! End-to-end tests for the compositional parser and compiler
//!
//! These tests verify the complete pipeline:
//! 1. Parse DSL code with compositional parser
//! 2. Compile to UnifiedSignalGraph
//! 3. Render audio
//! 4. Verify audio output

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

/// Helper to test code compilation and rendering
fn test_code(code: &str, duration_seconds: f32) -> Vec<f32> {
    // Parse
    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    // Compile
    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0); // 2 cycles per second

    // Render
    let num_samples = (duration_seconds * 44100.0) as usize;
    graph.render(num_samples)
}

/// Verify audio has content (not silent)
fn assert_has_audio(buffer: &[f32], description: &str) {
    let max = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    let rms = (buffer.iter().map(|s| s * s).sum::<f32>() / buffer.len() as f32).sqrt();

    assert!(
        max > 0.001,
        "{}: Max amplitude too low: {}",
        description,
        max
    );
    assert!(rms > 0.0001, "{}: RMS too low: {}", description, rms);
}

// ========== Comment Tests ==========

#[test]
fn test_e2e_comments() {
    let code = r#"
-- This is a comment
tempo: 0.5

-- Another comment
~freq $ 440

-- Final comment
out $ sine ~freq
"#;

    let buffer = test_code(code, 0.5);
    assert_has_audio(&buffer, "Comments test");
}

// ========== Effect Tests ==========

#[test]
fn test_e2e_reverb() {
    let code = r#"
tempo: 0.5
out $ sine 440 # reverb 0.7 0.5 0.3
"#;

    let buffer = test_code(code, 1.0);
    assert_has_audio(&buffer, "Reverb test");
}

#[test]
fn test_e2e_distortion() {
    let code = r#"
tempo: 0.5
out $ saw 110 # distortion 3.0 0.5
"#;

    let buffer = test_code(code, 0.5);
    assert_has_audio(&buffer, "Distortion test");
}

#[test]
fn test_e2e_delay() {
    let code = r#"
tempo: 0.5
out $ sine 440 # delay 0.25 0.5 0.4
"#;

    let buffer = test_code(code, 1.0);
    assert_has_audio(&buffer, "Delay test");
}

#[test]
fn test_e2e_chorus() {
    let code = r#"
tempo: 0.5
out $ sine 220 # chorus 2.0 0.5 0.3
"#;

    let buffer = test_code(code, 1.0);
    assert_has_audio(&buffer, "Chorus test");
}

#[test]
fn test_e2e_bitcrush() {
    let code = r#"
tempo: 0.5
out $ sine 440 # bitcrush 4.0 8000.0
"#;

    let buffer = test_code(code, 0.5);
    assert_has_audio(&buffer, "Bitcrush test");
}

#[test]
fn test_e2e_chained_effects() {
    let code = r#"
tempo: 0.5
~bass $ saw 55 # lpf 400 0.8
out $ ~bass # distortion 2.0 0.3 # reverb 0.5 0.4 0.2
"#;

    let buffer = test_code(code, 1.0);
    assert_has_audio(&buffer, "Chained effects test");
}

// ========== Sample Bank Selection Tests ==========

#[test]
fn test_e2e_sample_banks() {
    let code = r#"
tempo: 0.5
out $ s "bd:0 bd:1 bd:2"
"#;

    let buffer = test_code(code, 1.0);
    // Note: This will be silent without actual sample files
    // The test verifies compilation, not audio output
    assert_eq!(buffer.len(), 44100, "Should render 1 second");
}

#[test]
fn test_e2e_sample_banks_with_effects() {
    let code = r#"
tempo: 0.5
out $ s "bd:0 sn:1 hh:2" # lpf 2000 0.8
"#;

    let buffer = test_code(code, 1.0);
    assert_eq!(buffer.len(), 44100, "Should render 1 second");
}

// ========== Pattern Transform Tests ==========

#[test]
fn test_e2e_fast_transform() {
    let code = r#"
tempo: 0.5
out $ s "bd sn" $ fast 2
"#;

    let buffer = test_code(code, 1.0);
    assert_eq!(buffer.len(), 44100);
}

#[test]
fn test_e2e_slow_transform() {
    let code = r#"
tempo: 0.5
out $ s "bd*4" $ slow 0.5
"#;

    let buffer = test_code(code, 1.0);
    assert_eq!(buffer.len(), 44100);
}

#[test]
fn test_e2e_rev_transform() {
    let code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ rev
"#;

    let buffer = test_code(code, 1.0);
    assert_eq!(buffer.len(), 44100);
}

#[test]
fn test_e2e_degrade_transform() {
    let code = r#"
tempo: 0.5
out $ s "hh*16" $ degrade
"#;

    let buffer = test_code(code, 1.0);
    assert_eq!(buffer.len(), 44100);
}

#[test]
fn test_e2e_stutter_transform() {
    let code = r#"
tempo: 0.5
out $ s "bd sn" $ stutter 4
"#;

    let buffer = test_code(code, 1.0);
    assert_eq!(buffer.len(), 44100);
}

#[test]
fn test_e2e_palindrome_transform() {
    let code = r#"
tempo: 0.5
out $ s "a b c" $ palindrome
"#;

    let buffer = test_code(code, 1.0);
    assert_eq!(buffer.len(), 44100);
}

#[test]
fn test_e2e_stacked_transforms() {
    let code = r#"
tempo: 0.5
out $ s "bd sn" $ fast 2 $ rev
"#;

    let buffer = test_code(code, 1.0);
    assert_eq!(buffer.len(), 44100);
}

// ========== Complex Integration Tests ==========

#[test]
fn test_e2e_full_track() {
    let code = r#"
-- Full track with all features
tempo: 0.5

-- Drums with pattern transforms
~drums $ s "bd sn hh cp" $ fast 2

-- Pattern-controlled filter
~cutoffs $ "<500 1000 2000>" $ fast 3
~filtered_drums $ ~drums # lpf ~cutoffs 0.8

-- Bass with effects
~bass $ saw 55 # lpf 400 0.7 # distortion 2.0 0.3

-- Final mix with reverb
out $ (~filtered_drums * 0.5 + ~bass * 0.3) # reverb 0.5 0.4 0.2
"#;

    let buffer = test_code(code, 2.0);
    assert_eq!(buffer.len(), 88200);
    // Note: May be silent without actual samples, but should compile and render
}

#[test]
fn test_e2e_pattern_controlled_synthesis() {
    let code = r#"
tempo: 0.5

-- Pattern-controlled oscillator frequency
~freqs $ "220 440 330"
~osc $ sine ~freqs

-- Pattern-controlled filter
~cutoffs $ "500 1000 2000"
~filtered $ ~osc # lpf ~cutoffs 0.8

out $ ~filtered * 0.3
"#;

    let buffer = test_code(code, 1.0);
    assert_has_audio(&buffer, "Pattern-controlled synthesis");
}

#[test]
fn test_e2e_arithmetic_operations() {
    let code = r#"
tempo: 0.5

~base $ 220
~octave $ ~base * 2
~fifth $ ~base * 1.5

~chord $ sine ~base + sine ~fifth + sine ~octave
out $ ~chord * 0.2
"#;

    let buffer = test_code(code, 1.0);
    assert_has_audio(&buffer, "Arithmetic operations");
}

#[test]
fn test_e2e_all_waveforms() {
    let code = r#"
tempo: 0.5

~sine_osc $ sine 220
~saw_osc $ saw 220
~square_osc $ square 220
~tri_osc $ tri 220

out $ (~sine_osc + ~saw_osc + ~square_osc + ~tri_osc) * 0.1
"#;

    let buffer = test_code(code, 0.5);
    assert_has_audio(&buffer, "All waveforms");
}

#[test]
fn test_e2e_all_filters() {
    let code = r#"
tempo: 0.5

~source $ saw 110
~lpf_out $ ~source # lpf 1000 0.8
~hpf_out $ ~source # hpf 500 0.8
~bpf_out $ ~source # bpf 800 2.0

out $ (~lpf_out + ~hpf_out + ~bpf_out) * 0.15
"#;

    let buffer = test_code(code, 0.5);
    assert_has_audio(&buffer, "All filters");
}

#[test]
fn test_e2e_complex_modulation() {
    let code = r#"
tempo: 0.5

-- LFO modulating filter cutoff
~lfo $ sine 0.5
~cutoff_mod $ ~lfo * 1000 + 1500

-- Oscillator through modulated filter
~osc $ saw 110
~filtered $ ~osc # lpf ~cutoff_mod 0.7

out $ ~filtered * 0.3
"#;

    let buffer = test_code(code, 2.0);
    assert_has_audio(&buffer, "Complex modulation");
}

#[test]
fn test_e2e_tempo_setting() {
    let code = r#"
tempo: 3.5

~pattern $ "110 220 330 440"
out $ sine ~pattern * 0.3
"#;

    let buffer = test_code(code, 1.0);
    assert_has_audio(&buffer, "Tempo setting");
}

// ========== Syntax Variation Tests ==========

#[test]
fn test_e2e_space_separated_syntax() {
    // This is the PREFERRED syntax style
    let code = r#"
tempo: 0.5
~freq $ 440
out $ sine ~freq # lpf 1000 0.8
"#;

    let buffer = test_code(code, 0.5);
    assert_has_audio(&buffer, "Space-separated syntax");
}

// REMOVED: Parenthesized syntax is no longer supported
// Only space-separated syntax is allowed for live coding optimization

#[test]
fn test_e2e_mixed_syntax() {
    // Both work, but prefer space-separated for consistency
    let code = r#"
tempo: 0.5

~osc1 $ sine 220
~osc2 $ saw 330
~filtered $ ~osc1 # lpf 1000 0.8
~mixed $ ~filtered + ~osc2

out $ ~mixed * 0.2
"#;

    let buffer = test_code(code, 0.5);
    assert_has_audio(&buffer, "Mixed syntax");
}
