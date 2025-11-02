use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Pattern Query Verification
/// Tests that Timer syntax is parsed and compiled correctly
#[test]
fn test_timer_pattern_query() {
    let dsl = r#"
tempo: 1.0
~trigger: impulse 2.0
~time: ~trigger # timer
out: ~time
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
        "Timer should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: Timer Counts Upward
/// Tests that timer counts up over time
#[test]
fn test_timer_counts_up() {
    let dsl = r#"
tempo: 1.0
~trigger: impulse 1.0
~time: ~trigger # timer
out: ~time
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // Sample at different points in time
    let t_0_1 = samples[(SAMPLE_RATE * 0.1) as usize];
    let t_0_5 = samples[(SAMPLE_RATE * 0.5) as usize];
    let t_0_9 = samples[(SAMPLE_RATE * 0.9) as usize];

    println!(
        "Time values: t=0.1: {}, t=0.5: {}, t=0.9: {}",
        t_0_1, t_0_5, t_0_9
    );

    // Timer should increase over time
    assert!(
        t_0_5 > t_0_1,
        "Timer should increase: {} > {}",
        t_0_5,
        t_0_1
    );
    assert!(
        t_0_9 > t_0_5,
        "Timer should increase: {} > {}",
        t_0_9,
        t_0_5
    );

    // Values should be approximately correct (within 10%)
    assert!(
        (t_0_1 - 0.1).abs() < 0.02,
        "At t=0.1s, timer should read ~0.1, got {}",
        t_0_1
    );
    assert!(
        (t_0_5 - 0.5).abs() < 0.02,
        "At t=0.5s, timer should read ~0.5, got {}",
        t_0_5
    );
    assert!(
        (t_0_9 - 0.9).abs() < 0.02,
        "At t=0.9s, timer should read ~0.9, got {}",
        t_0_9
    );
}

/// LEVEL 2: Timer Resets on Trigger
/// Tests that timer resets to 0 when triggered
#[test]
fn test_timer_resets() {
    let dsl = r#"
tempo: 1.0
~trigger: impulse 2.0
~time: ~trigger # timer
out: ~time
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // After first trigger (t=0), timer counts up to t=0.4
    // Then trigger at t=0.5 resets to 0, counts up to t=0.4 again
    let before_first_reset = samples[(SAMPLE_RATE * 0.4) as usize];
    let after_reset = samples[(SAMPLE_RATE * 0.52) as usize]; // Just after t=0.5 trigger
    let before_second_reset = samples[(SAMPLE_RATE * 0.9) as usize];

    println!(
        "Timer values: t=0.4: {}, t=0.52 (after reset): {}, t=0.9: {}",
        before_first_reset, after_reset, before_second_reset
    );

    // Before first reset, should be ~0.4s
    assert!(
        (before_first_reset - 0.4).abs() < 0.05,
        "Before reset should be ~0.4s, got {}",
        before_first_reset
    );

    // After reset, should be close to 0
    assert!(
        after_reset < 0.1,
        "After reset should be near 0, got {}",
        after_reset
    );

    // Before second reset, should be ~0.4s again
    assert!(
        (before_second_reset - 0.4).abs() < 0.05,
        "Before second reset should be ~0.4s, got {}",
        before_second_reset
    );
}

/// LEVEL 2: Timer Without Trigger
/// Tests that timer counts continuously without trigger
#[test]
fn test_timer_continuous() {
    let dsl = r#"
tempo: 1.0
~no_trigger: 0.0
~time: ~no_trigger # timer
out: ~time
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // With no trigger (always low), timer should count continuously from 0
    let t_0_25 = samples[(SAMPLE_RATE * 0.25) as usize];
    let t_0_50 = samples[(SAMPLE_RATE * 0.50) as usize];
    let t_0_75 = samples[(SAMPLE_RATE * 0.75) as usize];

    println!(
        "Continuous timer: t=0.25: {}, t=0.5: {}, t=0.75: {}",
        t_0_25, t_0_50, t_0_75
    );

    // Should continuously increase
    assert!(t_0_50 > t_0_25, "Should increase continuously");
    assert!(t_0_75 > t_0_50, "Should increase continuously");

    // Values should be approximately correct
    assert!(
        (t_0_25 - 0.25).abs() < 0.05,
        "Should read ~0.25s, got {}",
        t_0_25
    );
    assert!(
        (t_0_50 - 0.50).abs() < 0.05,
        "Should read ~0.50s, got {}",
        t_0_50
    );
    assert!(
        (t_0_75 - 0.75).abs() < 0.05,
        "Should read ~0.75s, got {}",
        t_0_75
    );
}

/// LEVEL 2: Timer with Fast Triggers
/// Tests timer behavior with rapid triggers
#[test]
fn test_timer_fast_triggers() {
    let dsl = r#"
tempo: 1.0
~fast_trigger: impulse 10.0
~time: ~fast_trigger # timer
out: ~time
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // With 10 Hz triggers (every 0.1s), timer should reset frequently
    // Max value should be close to 0.1s
    let max_value = samples.iter().fold(0.0f32, |a, &b| a.max(b));

    println!("Max timer value with 10 Hz triggers: {}", max_value);

    // Should not exceed much more than 0.1s
    assert!(
        max_value < 0.15,
        "With 10 Hz triggers, max should be ~0.1s, got {}",
        max_value
    );
}

/// LEVEL 2: Timer Stability
/// Tests that timer doesn't produce NaN or Inf
#[test]
fn test_timer_stability() {
    let dsl = r#"
tempo: 1.0
~trigger: impulse 4.0
~time: ~trigger # timer
out: ~time * 0.1
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // Check for NaN or Inf
    let has_nan = samples.iter().any(|s| s.is_nan() || s.is_infinite());
    assert!(!has_nan, "Timer should not produce NaN or Inf");

    // Check for reasonable output
    let max_val = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        max_val < 10.0,
        "Timer output should be reasonable, got max {}",
        max_val
    );
}

/// LEVEL 3: Musical Example - Time-based Filter Sweep
/// Tests timer for creating time-based modulation
#[test]
fn test_timer_filter_sweep() {
    let dsl = r#"
tempo: 1.0
~trigger: impulse 2.0
~time: ~trigger # timer
~carrier: saw 110
~cutoff: ~time * 4000.0 + 200.0
~swept: ~carrier # lpf ~cutoff 0.8
out: ~swept * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE * 2.0) as usize);

    // Should produce audible swept sound
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    assert!(
        rms.sqrt() > 0.05,
        "Timer-based filter sweep should be audible, got RMS {}",
        rms.sqrt()
    );
}

/// LEVEL 3: Timing Gates
/// Tests timer for measuring gate durations
#[test]
fn test_timer_gate_measurement() {
    let dsl = r#"
tempo: 1.0
~gate: impulse 1.0
~duration: ~gate # timer
~tone: sine 440 * ~duration
out: ~tone * 0.2
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // Should produce audible output with increasing amplitude
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    assert!(
        rms.sqrt() > 0.05,
        "Timer-modulated tone should be audible, got RMS {}",
        rms.sqrt()
    );
}

/// LEVEL 3: Pattern-Modulated Trigger
/// Tests that timer trigger can be pattern-modulated
#[test]
fn test_timer_pattern_trigger() {
    let dsl = r#"
tempo: 2.0
~triggers: "0.0 1.0 0.0 1.0"
~time: ~triggers # timer
out: ~time
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let graph = compile_program(statements, SAMPLE_RATE);

    assert!(
        graph.is_ok(),
        "Pattern-modulated timer should compile: {:?}",
        graph.err()
    );
}
