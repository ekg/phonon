/// Tests for colon syntax in mini-notation: s "bd:0 bd:1 bd:2"
///
/// This allows direct sample selection in the pattern string without using the n parameter.
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

/// Test that colon syntax parses correctly
#[test]
fn test_colon_syntax_parses() {
    let dsl = r#"
tempo: 2.0
~drums: s "bd:0 bd:1"
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
        "Colon syntax should compile successfully: {:?}",
        graph.err()
    );
}

/// Test that colon syntax produces audio
#[test]
fn test_colon_syntax_produces_audio() {
    let dsl = r#"
tempo: 2.0
~drums: s "bd:0 bd:1 bd:2"
out: ~drums
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Render 2 seconds
    let buffer = graph.render((SAMPLE_RATE * 2.0) as usize);

    let rms: f32 = buffer.iter().map(|&x| x * x).sum::<f32>() / buffer.len() as f32;

    assert!(
        rms.sqrt() > 0.01,
        "Colon syntax should produce audio, got RMS {}",
        rms.sqrt()
    );
}

/// Test combining colon syntax with euclidean rhythms
#[test]
fn test_colon_with_euclidean() {
    let dsl = r#"
tempo: 2.0
~drums: s "bd:0(3,8) bd:1(5,8,2)"
out: ~drums
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let buffer = graph.render((SAMPLE_RATE * 2.0) as usize);

    let rms: f32 = buffer.iter().map(|&x| x * x).sum::<f32>() / buffer.len() as f32;

    assert!(
        rms.sqrt() > 0.01,
        "Colon with euclidean should work, got RMS {}",
        rms.sqrt()
    );
}

/// Test alternation with colon syntax
#[test]
fn test_colon_with_alternation() {
    let dsl = r#"
tempo: 2.0
~drums: s "<bd:0 bd:1 bd:2>"
out: ~drums
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let buffer = graph.render((SAMPLE_RATE * 3.0) as usize);

    let rms: f32 = buffer.iter().map(|&x| x * x).sum::<f32>() / buffer.len() as f32;

    assert!(
        rms.sqrt() > 0.01,
        "Colon with alternation should work, got RMS {}",
        rms.sqrt()
    );
}

/// Test musical example - drum variations
#[test]
fn test_colon_drum_variations() {
    let dsl = r#"
tempo: 2.0
~kick: s "bd:0*4"
~snare: s "~ sn:1 ~ sn:2"
~hats: s "hh:0*8"
out: (~kick + ~snare + ~hats) * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let buffer = graph.render((SAMPLE_RATE * 2.0) as usize);

    let rms: f32 = buffer.iter().map(|&x| x * x).sum::<f32>() / buffer.len() as f32;

    assert!(
        rms.sqrt() > 0.05,
        "Drum pattern with variations should be audible, got RMS {}",
        rms.sqrt()
    );
}
