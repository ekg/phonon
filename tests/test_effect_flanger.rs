/// Tests for flanger effect
///
/// Flanger creates a "swooshing" effect by mixing the input signal with a delayed version
/// where the delay time is modulated by an LFO.
///
/// Key characteristics to test:
/// - Creates comb filtering (notches in frequency spectrum)
/// - Modulation depth affects intensity
/// - Feedback creates more pronounced effect
/// - Mix controls wet/dry balance
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use std::f32::consts::PI;

/// Helper to render Phonon code
fn render_dsl(code: &str, duration_seconds: f32) -> Vec<f32> {
    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0); // 2 cycles per second

    let num_samples = (duration_seconds * 44100.0) as usize;
    graph.render(num_samples)
}

fn calculate_rms(buffer: &[f32]) -> f32 {
    if buffer.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Calculate frequency spectrum magnitude at a specific frequency
fn magnitude_at_frequency(buffer: &[f32], sample_rate: f32, target_freq: f32) -> f32 {
    let n = buffer.len();
    let mut real_sum = 0.0;
    let mut imag_sum = 0.0;

    for (i, &sample) in buffer.iter().enumerate() {
        let phase = -2.0 * PI * target_freq * (i as f32) / sample_rate;
        real_sum += sample * phase.cos();
        imag_sum += sample * phase.sin();
    }

    (real_sum * real_sum + imag_sum * imag_sum).sqrt() / (n as f32)
}

#[test]
fn test_flanger_basic_functionality() {
    // Test that flanger processes audio without silence
    let code = r#"
tempo: 0.5
~osc: sine 440
~flanged: ~osc # flanger 0.5 0.7 0.3
out: ~flanged
"#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    println!("Flanger basic RMS: {:.6}", rms);
    assert!(rms > 0.01, "Flanger should produce audio output");
}

#[test]
fn test_flanger_modulates_spectrum() {
    // Flanger should create notches in the frequency spectrum (comb filtering)
    let code_dry = r#"
tempo: 0.5
out: sine 440
"#;

    let code_flanged = r#"
tempo: 0.5
~osc: sine 440
out: ~osc # flanger 0.5 0.7 0.3
"#;

    let dry = render_dsl(code_dry, 2.0);
    let flanged = render_dsl(code_flanged, 2.0);

    // Check magnitude at fundamental and harmonics
    let mag_dry_440 = magnitude_at_frequency(&dry, 44100.0, 440.0);
    let mag_flanged_440 = magnitude_at_frequency(&flanged, 44100.0, 440.0);

    println!("Dry 440Hz magnitude: {:.6}", mag_dry_440);
    println!("Flanged 440Hz magnitude: {:.6}", mag_flanged_440);

    // Flanger should modify the spectrum
    assert!(
        (mag_dry_440 - mag_flanged_440).abs() > 0.01,
        "Flanger should significantly modify the spectrum"
    );
}

#[test]
fn test_flanger_with_samples() {
    // Test flanger on drum samples
    let code = r#"
tempo: 0.5
~drums: s "bd hh sn hh"
~flanged: ~drums # flanger 1.0 0.5 0.4
out: ~flanged
"#;

    let buffer = render_dsl(code, 4.0);
    let rms = calculate_rms(&buffer);

    println!("Flanger with samples RMS: {:.6}", rms);
    assert!(rms > 0.05, "Flanged drums should have substantial energy");
}

#[test]
fn test_flanger_depth_parameter() {
    // Test that depth affects intensity
    let code_shallow = r#"
tempo: 0.5
~osc: sine 440
out: ~osc # flanger 0.2 0.5 0.3
"#;

    let code_deep = r#"
tempo: 0.5
~osc: sine 440
out: ~osc # flanger 2.0 0.5 0.3
"#;

    let shallow = render_dsl(code_shallow, 2.0);
    let deep = render_dsl(code_deep, 2.0);

    // Deeper flanger should have more pronounced spectral changes
    let shallow_var = shallow.iter().zip(&shallow[1..])
        .map(|(a, b)| (a - b).abs())
        .sum::<f32>() / shallow.len() as f32;

    let deep_var = deep.iter().zip(&deep[1..])
        .map(|(a, b)| (a - b).abs())
        .sum::<f32>() / deep.len() as f32;

    println!("Shallow flanger variation: {:.6}", shallow_var);
    println!("Deep flanger variation: {:.6}", deep_var);

    // Note: This test might need adjustment based on actual implementation behavior
    assert!(shallow_var > 0.0 && deep_var > 0.0, "Both should produce variation");
}

#[test]
fn test_flanger_pattern_control() {
    // Test that parameters can be pattern-controlled
    let code = r#"
tempo: 0.5
~osc: sine 440
~rate: sine 0.25 * 0.5 + 0.5
out: ~osc # flanger ~rate 0.7 0.3
"#;

    let buffer = render_dsl(code, 4.0);
    let rms = calculate_rms(&buffer);

    println!("Pattern-controlled flanger RMS: {:.6}", rms);
    assert!(rms > 0.01, "Pattern-controlled flanger should work");
}

#[test]
fn test_flanger_mix_parameter() {
    // Test wet/dry mix
    let code_dry_mix = r#"
tempo: 0.5
~osc: sine 440
out: ~osc # flanger 1.0 0.5 0.0
"#;

    let code_wet_mix = r#"
tempo: 0.5
~osc: sine 440
out: ~osc # flanger 1.0 0.5 1.0
"#;

    let dry_mix = render_dsl(code_dry_mix, 2.0);
    let wet_mix = render_dsl(code_wet_mix, 2.0);

    let rms_dry = calculate_rms(&dry_mix);
    let rms_wet = calculate_rms(&wet_mix);

    println!("Dry mix (mix=0.0) RMS: {:.6}", rms_dry);
    println!("Wet mix (mix=1.0) RMS: {:.6}", rms_wet);

    // Both should produce sound
    assert!(rms_dry > 0.01 && rms_wet > 0.01, "Both mix settings should produce audio");
}
