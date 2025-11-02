/// Comprehensive tests for Allpass Filter UGen
/// Tests allpass filter for phase manipulation and reverb effects
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Pattern Query Verification
/// Tests that Allpass syntax is parsed and compiled correctly
#[test]
fn test_allpass_pattern_query() {
    let dsl = r#"
tempo: 1.0
~input: sine 440
~filtered: allpass ~input 0.5
out: ~filtered
"#;

    let (remaining, statements) = parse_program(dsl).unwrap();
    assert!(
        remaining.trim().is_empty(),
        "Should parse completely, remaining: '{}'",
        remaining
    );

    let graph = compile_program(statements, SAMPLE_RATE);
    assert!(
        graph.is_ok(),
        "Allpass should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: Allpass Basic Functionality
/// Tests that allpass filter produces output
#[test]
fn test_allpass_basic() {
    let dsl = r#"
tempo: 1.0
~input: sine 440
~filtered: allpass ~input 0.5
out: ~filtered
"#;

    let (remaining, statements) = parse_program(dsl).unwrap();
    assert!(remaining.trim().is_empty());

    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    // Generate 1 second of audio
    let duration = 1.0;
    let num_samples = (SAMPLE_RATE * duration) as usize;
    let buffer = graph.render(num_samples);

    // Calculate RMS of output
    let output_rms: f32 = buffer.iter().map(|&x| x * x).sum::<f32>() / buffer.len() as f32;

    // Allpass should preserve amplitude (unity gain)
    assert!(
        output_rms.sqrt() > 0.5,
        "Expected allpass to preserve amplitude, got RMS {}",
        output_rms.sqrt()
    );
}

/// LEVEL 3: Allpass Zero Coefficient
/// Coefficient = 0.0 should pass signal unchanged
#[test]
fn test_allpass_zero_coefficient() {
    let dsl = r#"
tempo: 1.0
~input: sine 440
~filtered: allpass ~input 0.0
out: ~filtered
"#;

    let (remaining, statements) = parse_program(dsl).unwrap();
    assert!(remaining.trim().is_empty());

    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    // Generate 1 second of audio
    let duration = 1.0;
    let num_samples = (SAMPLE_RATE * duration) as usize;
    let buffer = graph.render(num_samples);

    // Calculate RMS of output
    let output_rms: f32 = buffer.iter().map(|&x| x * x).sum::<f32>() / buffer.len() as f32;

    // With coefficient 0, should pass signal through
    assert!(
        output_rms.sqrt() > 0.5,
        "Expected signal passthrough with coefficient 0, got RMS {}",
        output_rms.sqrt()
    );
}

/// LEVEL 4: Allpass Pattern-Modulated Coefficient
/// Tests dynamic coefficient modulation
#[test]
fn test_allpass_pattern_modulated() {
    let dsl = r#"
tempo: 1.0
~input: saw 220
~lfo: sine 0.5
~filtered: allpass ~input ~lfo
out: ~filtered * 0.5
"#;

    let (remaining, statements) = parse_program(dsl).unwrap();
    assert!(remaining.trim().is_empty());

    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    // Generate 2 seconds of audio
    let duration = 2.0;
    let num_samples = (SAMPLE_RATE * duration) as usize;
    let buffer = graph.render(num_samples);

    // Calculate RMS of output
    let output_rms: f32 = buffer.iter().map(|&x| x * x).sum::<f32>() / buffer.len() as f32;

    // Pattern-modulated allpass should be audible
    assert!(
        output_rms.sqrt() > 0.1,
        "Expected pattern-modulated allpass, got RMS {}",
        output_rms.sqrt()
    );
}

/// LEVEL 5: Allpass on Noise
/// Tests allpass filter on noise source
#[test]
fn test_allpass_on_noise() {
    let dsl = r#"
tempo: 1.0
~noise: pink_noise
~filtered: allpass ~noise 0.7
out: ~filtered * 0.3
"#;

    let (remaining, statements) = parse_program(dsl).unwrap();
    assert!(remaining.trim().is_empty());

    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    // Generate 1 second of audio
    let duration = 1.0;
    let num_samples = (SAMPLE_RATE * duration) as usize;
    let buffer = graph.render(num_samples);

    // Calculate RMS of output
    let output_rms: f32 = buffer.iter().map(|&x| x * x).sum::<f32>() / buffer.len() as f32;

    // Filtered noise should be audible
    assert!(
        output_rms.sqrt() > 0.04,
        "Expected filtered noise, got RMS {}",
        output_rms.sqrt()
    );
}

/// LEVEL 6: Chained Allpass Filters
/// Tests cascading multiple allpass filters (common in reverb)
#[test]
fn test_allpass_chained() {
    let dsl = r#"
tempo: 1.0
~input: square 220
~ap1: allpass ~input 0.5
~ap2: allpass ~ap1 0.3
~ap3: allpass ~ap2 0.7
out: ~ap3 * 0.5
"#;

    let (remaining, statements) = parse_program(dsl).unwrap();
    assert!(remaining.trim().is_empty());

    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    // Generate 1 second of audio
    let duration = 1.0;
    let num_samples = (SAMPLE_RATE * duration) as usize;
    let buffer = graph.render(num_samples);

    // Calculate RMS of output
    let output_rms: f32 = buffer.iter().map(|&x| x * x).sum::<f32>() / buffer.len() as f32;

    // Chained allpass should preserve energy
    assert!(
        output_rms.sqrt() > 0.2,
        "Expected chained allpass to preserve energy, got RMS {}",
        output_rms.sqrt()
    );
}

/// LEVEL 7: Allpass With Envelope
/// Tests allpass filter with enveloped signal
#[test]
fn test_allpass_with_envelope() {
    let dsl = r#"
tempo: 1.0
~env: adsr 0.1 0.2 0.7 0.3
~osc: saw 440 * ~env
~filtered: allpass ~osc 0.6
out: ~filtered * 0.5
"#;

    let (remaining, statements) = parse_program(dsl).unwrap();
    assert!(remaining.trim().is_empty());

    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    // Generate 2 seconds of audio (full ADSR cycle)
    let duration = 2.0;
    let num_samples = (SAMPLE_RATE * duration) as usize;
    let buffer = graph.render(num_samples);

    // Calculate RMS of output
    let output_rms: f32 = buffer.iter().map(|&x| x * x).sum::<f32>() / buffer.len() as f32;

    // Enveloped allpass should be audible
    assert!(
        output_rms.sqrt() > 0.05,
        "Expected enveloped allpass, got RMS {}",
        output_rms.sqrt()
    );
}
