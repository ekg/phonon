/// Comprehensive tests for XFade (Crossfader) UGen
/// Tests crossfading between two signals with pattern-modulated position

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Pattern Query Verification
/// Tests that XFade syntax is parsed and compiled correctly
#[test]
fn test_xfade_pattern_query() {
    let dsl = r#"
tempo: 1.0
~a: sine 440
~b: sine 880
~crossfade: xfade ~a ~b 0.5
out: ~crossfade
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
        "XFade should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: XFade Full Signal A
/// Position = 0.0 should output 100% signal A
#[test]
fn test_xfade_full_signal_a() {
    let dsl = r#"
tempo: 1.0
~a: sine 440
~b: sine 880
~faded: xfade ~a ~b 0.0
out: ~faded
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

    // Signal A should be present (440 Hz sine should have significant energy)
    assert!(
        output_rms.sqrt() > 0.3,
        "Expected signal A output, got RMS {}",
        output_rms.sqrt()
    );
}

/// LEVEL 3: XFade Full Signal B
/// Position = 1.0 should output 100% signal B
#[test]
fn test_xfade_full_signal_b() {
    let dsl = r#"
tempo: 1.0
~a: sine 440
~b: sine 880
~faded: xfade ~a ~b 1.0
out: ~faded
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

    // Signal B should be present (880 Hz sine should have significant energy)
    assert!(
        output_rms.sqrt() > 0.3,
        "Expected signal B output, got RMS {}",
        output_rms.sqrt()
    );
}

/// LEVEL 4: XFade Center Position
/// Position = 0.5 should output 50/50 mix
#[test]
fn test_xfade_center_position() {
    let dsl = r#"
tempo: 1.0
~a: sine 440
~b: sine 440
~faded: xfade ~a ~b 0.5
out: ~faded
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

    // 50/50 mix of two identical signals should equal the original
    assert!(
        output_rms.sqrt() > 0.5,
        "Expected 50/50 mix output, got RMS {}",
        output_rms.sqrt()
    );
}

/// LEVEL 5: Pattern-Modulated Crossfade Position
/// Tests dynamic crossfade with pattern-modulated position
#[test]
fn test_xfade_pattern_modulated() {
    let dsl = r#"
tempo: 1.0
~a: sine 220
~b: sine 440
~position: line 0 1
~faded: xfade ~a ~b ~position
out: ~faded * 0.5
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

    // Should have audible output (crossfading between 220 and 440 Hz)
    assert!(
        output_rms.sqrt() > 0.2,
        "Expected pattern-modulated crossfade, got RMS {}",
        output_rms.sqrt()
    );
}

/// LEVEL 6: Crossfade Between Different Waveforms
/// Tests crossfade between sine and saw waves
#[test]
fn test_xfade_different_waveforms() {
    let dsl = r#"
tempo: 1.0
~sine: sine 220
~saw: saw 220
~faded: xfade ~sine ~saw 0.3
out: ~faded * 0.5
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

    // 70% sine + 30% saw should be audible
    assert!(
        output_rms.sqrt() > 0.15,
        "Expected crossfade between waveforms, got RMS {}",
        output_rms.sqrt()
    );
}

/// LEVEL 7: Crossfade with Enveloped Signals
/// Tests crossfade with ADSR-enveloped signals
#[test]
fn test_xfade_with_envelopes() {
    let dsl = r#"
tempo: 1.0
~env1: adsr 0.1 0.2 0.7 0.3
~env2: adsr 0.05 0.1 0.5 0.2
~a: sine 440 * ~env1
~b: sine 880 * ~env2
~faded: xfade ~a ~b 0.5
out: ~faded * 0.5
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

    // Enveloped crossfade should be audible
    assert!(
        output_rms.sqrt() > 0.05,
        "Expected enveloped crossfade, got RMS {}",
        output_rms.sqrt()
    );
}
