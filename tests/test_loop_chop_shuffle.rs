/// Tests for loop chopping and shuffling workflow
///
/// Demonstrates and verifies the ability to:
/// 1. Take an audio loop (sample pattern)
/// 2. Chop it into pieces (using `chop n`)
/// 3. Shuffle/scramble the pieces (using `shuffle` or `scramble`)
///
/// This is a common technique in:
/// - Breakbeat production
/// - Glitch music
/// - IDM and experimental electronic music
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

mod audio_test_utils;
use audio_test_utils::calculate_rms;

/// Helper to render DSL code
fn render_dsl(code: &str, duration_seconds: f32) -> Vec<f32> {
    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    let num_samples = (duration_seconds * 44100.0) as usize;
    graph.render(num_samples)
}

#[test]
fn test_basic_chop_scramble() {
    // Basic workflow: Take a pattern, chop it, scramble it
    let code = r#"
tempo: 2.0
out: s "bd sn hh cp" $ chop 8 $ scramble 8
"#;

    let buffer = render_dsl(code, 4.0);
    let rms = calculate_rms(&buffer);

    println!("Chop + scramble RMS: {:.6}", rms);
    assert!(rms > 0.05, "Chopped and scrambled pattern should produce sound");
}

#[test]
fn test_chop_shuffle() {
    // Use shuffle instead of scramble (time-based vs order-based)
    let code = r#"
tempo: 2.0
out: s "bd sn hh cp" $ chop 8 $ shuffle 0.2
"#;

    let buffer = render_dsl(code, 4.0);
    let rms = calculate_rms(&buffer);

    println!("Chop + shuffle RMS: {:.6}", rms);
    assert!(rms > 0.05, "Chopped and time-shuffled pattern should produce sound");
}

#[test]
fn test_fine_granular_chopping() {
    // Chop into many pieces for granular-style effect
    let code = r#"
tempo: 2.0
out: s "bd sn" $ chop 32 $ scramble 32
"#;

    let buffer = render_dsl(code, 4.0);
    let rms = calculate_rms(&buffer);

    println!("Granular chopping (32 pieces) RMS: {:.6}", rms);
    assert!(rms > 0.03, "Finely chopped pattern should produce sound");
}

#[test]
fn test_chop_with_effects() {
    // Chop, scramble, then add effects
    let code = r#"
tempo: 2.0
out: s "bd sn hh cp" $ chop 16 $ scramble 16 # lpf 2000 0.8
"#;

    let buffer = render_dsl(code, 4.0);
    let rms = calculate_rms(&buffer);

    println!("Chop + scramble + filter RMS: {:.6}", rms);
    assert!(rms > 0.03, "Chopped pattern with filter should produce sound");
}

#[test]
fn test_chop_euclidean_pattern() {
    // Chop a euclidean rhythm
    let code = r#"
tempo: 2.0
out: s "bd(5,8)" $ chop 16 $ scramble 16
"#;

    let buffer = render_dsl(code, 4.0);
    let rms = calculate_rms(&buffer);

    println!("Chopped euclidean pattern RMS: {:.6}", rms);
    assert!(rms > 0.05, "Chopped euclidean pattern should produce sound");
}

#[test]
fn test_chop_with_fast() {
    // Combine with fast transform
    let code = r#"
tempo: 2.0
out: s "bd sn" $ fast 2 $ chop 16 $ scramble 16
"#;

    let buffer = render_dsl(code, 4.0);
    let rms = calculate_rms(&buffer);

    println!("Fast + chop + scramble RMS: {:.6}", rms);
    assert!(rms > 0.05, "Fast then chopped pattern should produce sound");
}

#[test]
fn test_layered_chop_variations() {
    // Layer multiple differently-chopped versions
    let code = r#"
tempo: 2.0
~layer1: s "bd sn hh cp" $ chop 8 $ scramble 8 # gain 0.5
~layer2: s "bd sn hh cp" $ chop 16 $ scramble 16 # gain 0.3
out: ~layer1 + ~layer2
"#;

    let buffer = render_dsl(code, 4.0);
    let rms = calculate_rms(&buffer);

    println!("Layered chopped patterns RMS: {:.6}", rms);
    assert!(rms > 0.05, "Layered chopped patterns should produce sound");
}

#[test]
fn test_varying_chop_sizes() {
    // Test different chop sizes (not pattern-controlled, chop only takes constants)
    let code = r#"
tempo: 2.0
out: s "bd*8" $ chop 16 $ scramble 16
"#;

    let buffer = render_dsl(code, 4.0);
    let rms = calculate_rms(&buffer);

    println!("Chop size 16 RMS: {:.6}", rms);
    assert!(rms > 0.05, "Chop with size 16 should produce sound");
}

#[test]
fn test_chop_preserves_audio_energy() {
    // Verify chopping doesn't lose significant energy
    let normal_code = r#"
tempo: 2.0
out: s "bd sn hh cp"
"#;

    let chopped_code = r#"
tempo: 2.0
out: s "bd sn hh cp" $ chop 8 $ scramble 8
"#;

    let normal = render_dsl(normal_code, 4.0);
    let chopped = render_dsl(chopped_code, 4.0);

    let normal_rms = calculate_rms(&normal);
    let chopped_rms = calculate_rms(&chopped);

    println!("Normal RMS: {:.6}, Chopped RMS: {:.6}", normal_rms, chopped_rms);

    // Chopped version should have similar energy (within 50%)
    let ratio = chopped_rms / normal_rms;
    assert!(
        ratio > 0.5 && ratio < 2.0,
        "Chopped pattern should preserve audio energy: ratio = {:.2}",
        ratio
    );
}

#[test]
fn test_chop_scramble_different_from_original() {
    // Verify that chop + scramble actually changes the pattern
    // (This is a basic sanity check - scramble should reorder events)
    let original = r#"
tempo: 2.0
out: s "bd sn hh cp"
"#;

    let scrambled = r#"
tempo: 2.0
out: s "bd sn hh cp" $ chop 8 $ scramble 8
"#;

    let orig_buffer = render_dsl(original, 2.0);
    let scram_buffer = render_dsl(scrambled, 2.0);

    // Buffers should not be identical (scramble changes event timing)
    let identical = orig_buffer
        .iter()
        .zip(&scram_buffer)
        .all(|(a, b)| (a - b).abs() < 0.001);

    assert!(
        !identical,
        "Scrambled pattern should be different from original"
    );
}
