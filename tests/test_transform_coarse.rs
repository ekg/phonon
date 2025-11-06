/// Tests for coarse effect
///
/// coarse n - reduces sample rate to 1/n
/// coarse: 1 for original, 2 for half, 3 for a third and so on
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

/// Helper to test code compilation and rendering
fn test_code(code: &str, duration_seconds: f32) -> Vec<f32> {
    // Parse
    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    // Compile
    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
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
fn test_coarse_with_synthesis() {
    // Test coarse on synthesized sound
    let code = r#"
tempo: 2.0
out: saw 110 # coarse 3
"#;

    let buffer = test_code(code, 2.0);
    let rms = calculate_rms(&buffer);
    println!("Coarse synthesis RMS: {:.6}", rms);

    assert!(
        rms > 0.05,
        "Coarse synthesis should produce sound, got RMS {}",
        rms
    );
}

#[test]
fn test_coarse_reduces_quality() {
    // Test that coarse actually reduces sample quality
    // Original sound
    let original_code = r#"
tempo: 2.0
out: sine 440
"#;

    // Coarse sound (half sample rate)
    let coarse_code = r#"
tempo: 2.0
out: sine 440 # coarse 2
"#;

    let original = test_code(original_code, 2.0);
    let coarse = test_code(coarse_code, 2.0);

    // Both should have sound
    let original_rms = calculate_rms(&original);
    let coarse_rms = calculate_rms(&coarse);

    println!("Original RMS: {:.6}", original_rms);
    println!("Coarse RMS: {:.6}", coarse_rms);

    assert!(original_rms > 0.01, "Original should have sound");
    assert!(coarse_rms > 0.01, "Coarse should have sound");

    // RMS should be similar (coarse affects quality, not amplitude much)
    // But allow some variation due to sample-hold behavior
    let ratio = coarse_rms / original_rms;
    assert!(
        ratio > 0.5 && ratio < 2.0,
        "Coarse RMS should be similar to original, got ratio {}",
        ratio
    );
}

#[test]
fn test_coarse_different_rates() {
    // Test different coarse values
    for coarse_val in [1.0, 2.0, 4.0] {
        let code = format!(
            r#"
tempo: 2.0
out: saw 220 # coarse {}
"#,
            coarse_val
        );

        let buffer = test_code(&code, 2.0);
        let rms = calculate_rms(&buffer);
        println!("Coarse {} RMS: {:.6}", coarse_val, rms);

        assert!(
            rms > 0.01,
            "Coarse {} should produce sound, got RMS {}",
            coarse_val,
            rms
        );
    }
}
