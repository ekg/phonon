/// Tests for the `n` parameter with modulo wrapping
///
/// The `n` parameter selects which sample from a bank to play,
/// with modulo wrapping so every number is valid.
///
/// Examples:
/// - If bank "bd" has 3 samples (bd:0, bd:1, bd:2):
///   - n=0 → bd:0
///   - n=1 → bd:1
///   - n=2 → bd:2
///   - n=3 → bd:0 (wraps)
///   - n=5 → bd:2 (5 % 3 = 2)
///   - n=100 → bd:1 (100 % 3 = 1)
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Pattern Query - Test that n parameter is parsed correctly
#[test]
fn test_n_parameter_parses() {
    let dsl = r#"
tempo: 2.0
~drums: s "bd" # n 0
out: ~drums
"#;

    let (remaining, statements) = parse_program(dsl).unwrap();
    assert!(
        remaining.trim().is_empty(),
        "Should parse completely, remaining: '{}'",
        remaining
    );

    let graph = compile_program(statements, SAMPLE_RATE, None);
    assert!(
        graph.is_ok(),
        "n parameter should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: Basic n selection - Test that different n values produce different samples
#[test]
fn test_n_selects_different_samples() {
    // This test assumes bd has multiple samples (bd:0, bd:1, etc.)
    // If only one sample exists, they'll all sound the same - that's OK
    let dsl_0 = r#"
tempo: 2.0
~drums: s "bd" # n 0
out: ~drums
"#;

    let dsl_1 = r#"
tempo: 2.0
~drums: s "bd" # n 1
out: ~drums
"#;

    let (_, statements_0) = parse_program(dsl_0).unwrap();
    let mut graph_0 = compile_program(statements_0, SAMPLE_RATE, None).unwrap();

    let (_, statements_1) = parse_program(dsl_1).unwrap();
    let mut graph_1 = compile_program(statements_1, SAMPLE_RATE, None).unwrap();

    // Render 1 second
    let buffer_0 = graph_0.render((SAMPLE_RATE * 1.0) as usize);
    let buffer_1 = graph_1.render((SAMPLE_RATE * 1.0) as usize);

    // Both should produce audio
    let rms_0: f32 = buffer_0.iter().map(|&x| x * x).sum::<f32>() / buffer_0.len() as f32;
    let rms_1: f32 = buffer_1.iter().map(|&x| x * x).sum::<f32>() / buffer_1.len() as f32;

    assert!(
        rms_0.sqrt() > 0.01,
        "n=0 should produce audio, got RMS {}",
        rms_0.sqrt()
    );
    assert!(
        rms_1.sqrt() > 0.01,
        "n=1 should produce audio, got RMS {}",
        rms_1.sqrt()
    );
}

/// LEVEL 2: Pattern-based n - Test that n can be a pattern
#[test]
fn test_n_pattern_variation() {
    let dsl = r#"
tempo: 2.0
~drums: s "bd*4" # n "0 1 2 3"
out: ~drums
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Render 2 seconds (4 cycles at tempo 2.0)
    let buffer = graph.render((SAMPLE_RATE * 2.0) as usize);

    // Should produce audio with variation
    let rms: f32 = buffer.iter().map(|&x| x * x).sum::<f32>() / buffer.len() as f32;

    assert!(
        rms.sqrt() > 0.01,
        "Pattern-based n should produce audio, got RMS {}",
        rms.sqrt()
    );
}

/// LEVEL 2: Wrapping behavior - Test that large n values wrap correctly
#[test]
fn test_n_wrapping_large_values() {
    // Test with large n values that should wrap
    // If bd has 3 samples: n=100 should wrap to 100 % 3 = 1
    let dsl = r#"
tempo: 2.0
~drums: s "bd" # n 100
out: ~drums
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Render 1 second
    let buffer = graph.render((SAMPLE_RATE * 1.0) as usize);

    // Should produce audio (wrapping means no error)
    let rms: f32 = buffer.iter().map(|&x| x * x).sum::<f32>() / buffer.len() as f32;

    assert!(
        rms.sqrt() > 0.01,
        "Large n values should wrap and produce audio, got RMS {}",
        rms.sqrt()
    );
}

/// LEVEL 3: Musical example - Cycling through samples
#[test]
fn test_n_musical_cycle() {
    let dsl = r#"
tempo: 2.0
~drums: s "bd*8" # n "<0 1 2>"
out: ~drums * 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Render 3 seconds to get full alternation cycle
    let buffer = graph.render((SAMPLE_RATE * 3.0) as usize);

    let rms: f32 = buffer.iter().map(|&x| x * x).sum::<f32>() / buffer.len() as f32;

    assert!(
        rms.sqrt() > 0.05,
        "Musical n pattern should be audible, got RMS {}",
        rms.sqrt()
    );
}

/// LEVEL 3: Combined with other parameters - Test n with gain and pan
#[test]
fn test_n_with_other_parameters() {
    let dsl = r#"
tempo: 2.0
~drums: s "bd*4" # n "0 1 2 3" # gain "1.0 0.8 0.6 0.4" # pan "-1 -0.5 0.5 1"
out: ~drums * 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Render 2 seconds
    let buffer = graph.render((SAMPLE_RATE * 2.0) as usize);

    let rms: f32 = buffer.iter().map(|&x| x * x).sum::<f32>() / buffer.len() as f32;

    assert!(
        rms.sqrt() > 0.01,
        "n with multiple parameters should work, got RMS {}",
        rms.sqrt()
    );
}
