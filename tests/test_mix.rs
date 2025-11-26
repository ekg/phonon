/// Comprehensive tests for Mix UGen
/// Sums multiple signals together (variable number of inputs)
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Pattern Query Verification
/// Tests that Mix syntax is parsed and compiled correctly
#[test]
fn test_mix_pattern_query() {
    let dsl = r#"
tempo: 1.0
~a: sine 220
~b: sine 440
~c: sine 880
~mixed: mix ~a ~b ~c
out: ~mixed * 0.3
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
        "Mix should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: Mix Two Signals
/// Tests summing two sine waves
#[test]
fn test_mix_two_signals() {
    let dsl = r#"
tempo: 1.0
~a: sine 220
~b: sine 440
~mixed: mix ~a ~b
out: ~mixed * 0.5
"#;

    let (remaining, statements) = parse_program(dsl).unwrap();
    assert!(remaining.trim().is_empty());

    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Generate 1 second of audio
    let duration = 1.0;
    let num_samples = (SAMPLE_RATE * duration) as usize;
    let buffer = graph.render(num_samples);

    // Calculate RMS of output
    let output_rms: f32 = buffer.iter().map(|&x| x * x).sum::<f32>() / buffer.len() as f32;

    // Mixed signal should be audible
    assert!(
        output_rms.sqrt() > 0.3,
        "Expected audible mix, got RMS {}",
        output_rms.sqrt()
    );
}

/// LEVEL 3: Mix Three Signals
/// Tests summing three signals at different frequencies
#[test]
fn test_mix_three_signals() {
    let dsl = r#"
tempo: 1.0
~a: sine 220
~b: sine 440
~c: sine 880
~mixed: mix ~a ~b ~c
out: ~mixed * 0.3
"#;

    let (remaining, statements) = parse_program(dsl).unwrap();
    assert!(remaining.trim().is_empty());

    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Generate 1 second of audio
    let duration = 1.0;
    let num_samples = (SAMPLE_RATE * duration) as usize;
    let buffer = graph.render(num_samples);

    // Calculate RMS of output
    let output_rms: f32 = buffer.iter().map(|&x| x * x).sum::<f32>() / buffer.len() as f32;

    // Three mixed signals should be audible
    assert!(
        output_rms.sqrt() > 0.3,
        "Expected audible mix of 3 signals, got RMS {}",
        output_rms.sqrt()
    );
}

/// LEVEL 4: Mix With Different Waveforms
/// Tests mixing sine, saw, and square waves
#[test]
fn test_mix_different_waveforms() {
    let dsl = r#"
tempo: 1.0
~sine: sine 220
~saw: saw 220
~square: square 220
~mixed: mix ~sine ~saw ~square
out: ~mixed * 0.3
"#;

    let (remaining, statements) = parse_program(dsl).unwrap();
    assert!(remaining.trim().is_empty());

    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Generate 1 second of audio
    let duration = 1.0;
    let num_samples = (SAMPLE_RATE * duration) as usize;
    let buffer = graph.render(num_samples);

    // Calculate RMS of output
    let output_rms: f32 = buffer.iter().map(|&x| x * x).sum::<f32>() / buffer.len() as f32;

    // Mixed waveforms should be audible
    assert!(
        output_rms.sqrt() > 0.2,
        "Expected audible mix of different waveforms, got RMS {}",
        output_rms.sqrt()
    );
}

/// LEVEL 5: Mix With Envelopes
/// Tests mixing enveloped signals
#[test]
fn test_mix_with_envelopes() {
    let dsl = r#"
tempo: 1.0
~env1: adsr 0.1 0.2 0.7 0.3
~env2: ad 0.05 0.2
~a: sine 440 * ~env1
~b: saw 880 * ~env2
~mixed: mix ~a ~b
out: ~mixed * 0.5
"#;

    let (remaining, statements) = parse_program(dsl).unwrap();
    assert!(remaining.trim().is_empty());

    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Generate 2 seconds of audio
    let duration = 2.0;
    let num_samples = (SAMPLE_RATE * duration) as usize;
    let buffer = graph.render(num_samples);

    // Calculate RMS of output
    let output_rms: f32 = buffer.iter().map(|&x| x * x).sum::<f32>() / buffer.len() as f32;

    // Enveloped mix should be audible
    assert!(
        output_rms.sqrt() > 0.05,
        "Expected enveloped mix, got RMS {}",
        output_rms.sqrt()
    );
}

/// LEVEL 6: Mix Four Signals
/// Tests summing four signals
#[test]
fn test_mix_four_signals() {
    let dsl = r#"
tempo: 1.0
~a: sine 220
~b: sine 330
~c: sine 440
~d: sine 550
~mixed: mix ~a ~b ~c ~d
out: ~mixed * 0.25
"#;

    let (remaining, statements) = parse_program(dsl).unwrap();
    assert!(remaining.trim().is_empty());

    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Generate 1 second of audio
    let duration = 1.0;
    let num_samples = (SAMPLE_RATE * duration) as usize;
    let buffer = graph.render(num_samples);

    // Calculate RMS of output
    let output_rms: f32 = buffer.iter().map(|&x| x * x).sum::<f32>() / buffer.len() as f32;

    // Four mixed signals should be audible
    assert!(
        output_rms.sqrt() > 0.3,
        "Expected audible mix of 4 signals, got RMS {}",
        output_rms.sqrt()
    );
}

/// LEVEL 7: Mix With Nested Operations
/// Tests mixing signals that are results of other operations
#[test]
fn test_mix_nested_operations() {
    let dsl = r#"
tempo: 1.0
~osc1: sine 220
~osc2: sine 440
~filtered: ~osc2 # lpf 1000 0.8
~mixed: mix ~osc1 ~filtered
out: ~mixed * 0.5
"#;

    let (remaining, statements) = parse_program(dsl).unwrap();
    assert!(remaining.trim().is_empty());

    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Generate 1 second of audio
    let duration = 1.0;
    let num_samples = (SAMPLE_RATE * duration) as usize;
    let buffer = graph.render(num_samples);

    // Calculate RMS of output
    let output_rms: f32 = buffer.iter().map(|&x| x * x).sum::<f32>() / buffer.len() as f32;

    // Nested operations mix should be audible
    assert!(
        output_rms.sqrt() > 0.3,
        "Expected audible nested mix, got RMS {}",
        output_rms.sqrt()
    );
}
