use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Pattern Query Verification
/// Tests that segments syntax is parsed and compiled correctly
#[test]
fn test_env_pattern_query() {
    let dsl = r#"
tempo: 1.0
~envelope: segments "0 1 0" "0.5 0.5"
out: ~envelope
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
        "Segments should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: Segments Reaches Target Levels
/// Tests that segments progresses through specified breakpoints
#[test]
fn test_env_reaches_targets() {
    let dsl = r#"
tempo: 1.0
~envelope: segments "0 1 0.5" "0.25 0.25"
out: ~envelope
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE * 0.6) as usize);

    // At t=0, should be near 0
    let t_0_00 = samples[0];
    // At t=0.25, should be near 1.0 (peak)
    let t_0_25 = samples[(SAMPLE_RATE * 0.24) as usize];
    // At t=0.5, should be near 0.5 (second target)
    let t_0_50 = samples[(SAMPLE_RATE * 0.49) as usize];

    println!(
        "Env values: t=0: {}, t=0.25: {}, t=0.5: {}",
        t_0_00, t_0_25, t_0_50
    );

    assert!(t_0_00 < 0.1, "Should start at 0, got {}", t_0_00);
    assert!(
        (t_0_25 - 1.0).abs() < 0.1,
        "Should reach 1.0 at t=0.25, got {}",
        t_0_25
    );
    assert!(
        (t_0_50 - 0.5).abs() < 0.15,
        "Should reach 0.5 at t=0.5, got {}",
        t_0_50
    );
}

/// LEVEL 2: Segments Holds Final Value
/// Tests that envelope holds final level after completion
#[test]
fn test_env_holds_final() {
    let dsl = r#"
tempo: 1.0
~envelope: segments "0 1 0" "0.2 0.2"
out: ~envelope
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE * 0.6) as usize);

    // After envelope completes (t > 0.4), should hold at 0
    let t_0_50 = samples[(SAMPLE_RATE * 0.50) as usize];
    let t_0_55 = samples[(SAMPLE_RATE * 0.55) as usize];

    println!("Final values: t=0.5: {}, t=0.55: {}", t_0_50, t_0_55);

    assert!(
        (t_0_50 - 0.0).abs() < 0.1,
        "Should hold final value 0, got {} at t=0.5",
        t_0_50
    );
    assert!(
        (t_0_55 - 0.0).abs() < 0.1,
        "Should hold final value 0, got {} at t=0.55",
        t_0_55
    );
}

/// LEVEL 2: Single Segment Envelope
/// Tests simplest case: two levels, one time
#[test]
fn test_env_single_segment() {
    let dsl = r#"
tempo: 1.0
~ramp: segments "0 1" "0.5"
out: ~ramp
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE * 0.6) as usize);

    let t_0_0 = samples[0];
    let t_0_25 = samples[(SAMPLE_RATE * 0.25) as usize];
    let t_0_5 = samples[(SAMPLE_RATE * 0.49) as usize];

    println!(
        "Single segment: t=0: {}, t=0.25: {}, t=0.5: {}",
        t_0_0, t_0_25, t_0_5
    );

    assert!(t_0_0 < 0.1, "Should start at 0");
    assert!(t_0_25 > t_0_0, "Should increase");
    assert!(t_0_5 > 0.9, "Should reach 1.0");
}

/// LEVEL 2: Multi-Segment Envelope
/// Tests complex envelope with 4 levels
#[test]
fn test_env_multi_segment() {
    let dsl = r#"
tempo: 1.0
~env: segments "0 1 0.5 0.8 0" "0.1 0.1 0.1 0.1"
out: ~env
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE * 0.5) as usize);

    let t_0_00 = samples[0];
    let t_0_09 = samples[(SAMPLE_RATE * 0.09) as usize];
    let t_0_19 = samples[(SAMPLE_RATE * 0.19) as usize];
    let t_0_29 = samples[(SAMPLE_RATE * 0.29) as usize];
    let t_0_39 = samples[(SAMPLE_RATE * 0.39) as usize];

    println!(
        "Multi-segment: t=0: {}, t=0.1: {}, t=0.2: {}, t=0.3: {}, t=0.4: {}",
        t_0_00, t_0_09, t_0_19, t_0_29, t_0_39
    );

    // Should progress through all stages
    assert!(t_0_00 < 0.1, "Should start at 0");
    assert!(
        (t_0_09 - 1.0).abs() < 0.15,
        "Should reach 1.0 at t=0.1, got {}",
        t_0_09
    );
    assert!(
        (t_0_19 - 0.5).abs() < 0.15,
        "Should reach 0.5 at t=0.2, got {}",
        t_0_19
    );
    assert!(
        (t_0_29 - 0.8).abs() < 0.15,
        "Should reach 0.8 at t=0.3, got {}",
        t_0_29
    );
    assert!(
        (t_0_39 - 0.0).abs() < 0.15,
        "Should reach 0.0 at t=0.4, got {}",
        t_0_39
    );
}

/// LEVEL 2: Env Stability
/// Tests that env doesn't produce NaN or Inf
#[test]
fn test_env_stability() {
    let dsl = r#"
tempo: 1.0
~envelope: segments "0 1 0.5 0" "0.1 0.2 0.1"
out: ~envelope
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE * 1.0) as usize);

    // Check for NaN or Inf
    let has_nan = samples.iter().any(|s| s.is_nan() || s.is_infinite());
    assert!(!has_nan, "Segments should not produce NaN or Inf");

    // Check for reasonable output
    let max_val = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        max_val < 10.0,
        "Segments output should be reasonable, got max {}",
        max_val
    );
}

/// LEVEL 3: Musical Example - ADSR-like Envelope
/// Tests creating an ADSR-style envelope with segments
#[test]
fn test_env_adsr_style() {
    let dsl = r#"
tempo: 1.0
~adsr: segments "0 1 0.7 0" "0.1 0.2 0.3"
~carrier: sine 440
~shaped: ~carrier * ~adsr
out: ~shaped * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE * 0.7) as usize);

    // Should produce audible shaped tone
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    assert!(
        rms.sqrt() > 0.05,
        "ADSR-style env should be audible, got RMS {}",
        rms.sqrt()
    );
}

/// LEVEL 3: Musical Example - Percussion Envelope
/// Tests creating percussion-style envelope
#[test]
fn test_env_percussion() {
    let dsl = r#"
tempo: 0.5
~perc: segments "0 1 0" "0.01 0.3"
~osc: sine 110
~drum: ~osc * ~perc
out: ~drum * 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE * 0.5) as usize);

    // Should produce audible percussion hit
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    assert!(
        rms.sqrt() > 0.05,
        "Percussion env should be audible, got RMS {}",
        rms.sqrt()
    );

    // Should have sharp attack
    let first_100_rms: f32 = samples[..441].iter().map(|s| s * s).sum::<f32>() / 441.0;

    assert!(
        first_100_rms.sqrt() > 0.1,
        "Should have sharp attack, got first 10ms RMS {}",
        first_100_rms.sqrt()
    );
}

/// LEVEL 3: Musical Example - Complex Modulation Envelope
/// Tests using segments for filter modulation
#[test]
fn test_env_filter_modulation() {
    let dsl = r#"
tempo: 1.0
~filter_env: segments "200 3000 800 200" "0.2 0.3 0.2"
~carrier: saw 110
~filtered: ~carrier # lpf ~filter_env 0.8
out: ~filtered * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE * 0.8) as usize);

    // Should produce audible filtered sweep
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    assert!(
        rms.sqrt() > 0.05,
        "Filter modulation should be audible, got RMS {}",
        rms.sqrt()
    );
}
