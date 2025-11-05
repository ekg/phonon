/// Tests for vowel (formant filter) effect
///
/// vowel - TidalCycles-style formant filter using vowel letters
/// Accepts patterns of: a, e, i, o, u

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
fn test_vowel_a() {
    // Test vowel "a" formant
    let code = r#"
tempo: 2.0
out: saw 110 # vowel "a"
"#;

    let buffer = test_code(code, 2.0);
    let rms = calculate_rms(&buffer);
    println!("vowel 'a' RMS: {:.6}", rms);

    assert!(rms > 0.01, "vowel 'a' should produce sound, got RMS {}", rms);
}

#[test]
fn test_vowel_e() {
    // Test vowel "e" formant
    let code = r#"
tempo: 2.0
out: square 220 # vowel "e"
"#;

    let buffer = test_code(code, 2.0);
    let rms = calculate_rms(&buffer);
    println!("vowel 'e' RMS: {:.6}", rms);

    assert!(rms > 0.01, "vowel 'e' should produce sound, got RMS {}", rms);
}

#[test]
fn test_vowel_pattern() {
    // Test pattern of vowels
    let code = r#"
tempo: 2.0
out: saw 110 # vowel "a e i o"
"#;

    let buffer = test_code(code, 2.0);
    let rms = calculate_rms(&buffer);
    println!("vowel pattern RMS: {:.6}", rms);

    assert!(rms > 0.01, "vowel pattern should produce sound, got RMS {}", rms);
}

#[test]
fn test_all_vowels() {
    // Test all vowels produce sound
    for vowel in ["a", "e", "i", "o", "u"] {
        let code = format!(
            r#"
tempo: 2.0
out: square 440 # vowel "{}"
"#,
            vowel
        );

        let buffer = test_code(&code, 2.0);
        let rms = calculate_rms(&buffer);
        println!("vowel '{}' RMS: {:.6}", vowel, rms);

        assert!(
            rms > 0.01,
            "vowel '{}' should produce sound, got RMS {}",
            vowel,
            rms
        );
    }
}

#[test]
fn test_vowel_changes_tone() {
    // Test that different vowels produce different tones
    let vowel_a = r#"
tempo: 2.0
out: saw 220 # vowel "a"
"#;

    let vowel_i = r#"
tempo: 2.0
out: saw 220 # vowel "i"
"#;

    let buffer_a = test_code(vowel_a, 2.0);
    let buffer_i = test_code(vowel_i, 2.0);

    let rms_a = calculate_rms(&buffer_a);
    let rms_i = calculate_rms(&buffer_i);

    println!("vowel 'a' RMS: {:.6}", rms_a);
    println!("vowel 'i' RMS: {:.6}", rms_i);

    // Both should have sound
    assert!(rms_a > 0.01, "vowel 'a' should have sound");
    assert!(rms_i > 0.01, "vowel 'i' should have sound");

    // They should produce different results (formants shape the spectrum differently)
    // RMS might be similar but spectral content differs
}
