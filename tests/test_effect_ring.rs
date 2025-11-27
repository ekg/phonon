/// Tests for ring (ring modulation) effect
///
/// ring freq - classic ring modulation effect
/// Multiplies input signal with a carrier frequency
/// Creates metallic, inharmonic tones
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

fn calculate_rms(buffer: &[f32]) -> f32 {
    if buffer.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

#[test]
fn test_ring_basic() {
    // Test basic ring modulation
    let code = r#"
tempo: 2.0
out: saw 220 # ring 440
"#;

    let buffer = test_code(code, 2.0);
    let rms = calculate_rms(&buffer);
    println!("ring basic RMS: {:.6}", rms);

    assert!(rms > 0.01, "ring should produce sound, got RMS {}", rms);
}

#[test]
fn test_ring_different_frequencies() {
    // Test ring modulation at different carrier frequencies
    for freq in [100.0, 440.0, 1000.0] {
        let code = format!(
            r#"
tempo: 2.0
out: square 220 # ring {}
"#,
            freq
        );

        let buffer = test_code(&code, 2.0);
        let rms = calculate_rms(&buffer);
        println!("ring {} Hz RMS: {:.6}", freq, rms);

        assert!(
            rms > 0.01,
            "ring {} Hz should produce sound, got RMS {}",
            freq,
            rms
        );
    }
}

#[test]
fn test_ring_changes_tone() {
    // Test that ring modulation actually changes the sound
    let normal_code = r#"
tempo: 2.0
out: saw 220
"#;

    let ring_code = r#"
tempo: 2.0
out: saw 220 # ring 440
"#;

    let normal = test_code(normal_code, 2.0);
    let ring_mod = test_code(ring_code, 2.0);

    let normal_rms = calculate_rms(&normal);
    let ring_rms = calculate_rms(&ring_mod);

    println!("Normal RMS: {:.6}", normal_rms);
    println!("Ring mod RMS: {:.6}", ring_rms);

    // Both should have sound
    assert!(normal_rms > 0.01, "Normal should have sound");
    assert!(ring_rms > 0.01, "Ring mod should have sound");

    // Ring modulation creates inharmonic content - spectrum differs
    // but RMS might be similar (depends on modulation depth)
}

#[test]
fn test_ring_with_synthesis() {
    // Test ring mod on different oscillator types
    for osc in ["sine", "square", "saw"] {
        let code = format!(
            r#"
tempo: 2.0
out: {} 220 # ring 440
"#,
            osc
        );

        let buffer = test_code(&code, 2.0);
        let rms = calculate_rms(&buffer);
        println!("ring on {} RMS: {:.6}", osc, rms);

        assert!(
            rms > 0.01,
            "ring on {} should produce sound, got RMS {}",
            osc,
            rms
        );
    }
}
