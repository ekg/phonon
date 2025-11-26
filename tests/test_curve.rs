use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Pattern Query Verification
/// Tests that Curve syntax is parsed and compiled correctly
#[test]
fn test_curve_pattern_query() {
    let dsl = r#"
tempo: 1.0
~ramp: curve 0.0 1.0 1.0 2.0
out: ~ramp
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
        "Curve should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: Curve Ramps Upward
/// Tests that curve increases from start to end
#[test]
fn test_curve_upward() {
    let dsl = r#"
tempo: 1.0
~ramp: curve 0.0 1.0 1.0 0.0
out: ~ramp
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    let t_0_0 = samples[0];
    let t_0_5 = samples[(SAMPLE_RATE * 0.5) as usize];
    let t_1_0 = samples[(SAMPLE_RATE * 0.99) as usize];

    println!(
        "Curve values: t=0: {}, t=0.5: {}, t=1: {}",
        t_0_0, t_0_5, t_1_0
    );

    // Should start near 0
    assert!(t_0_0 < 0.1, "Should start at 0, got {}", t_0_0);

    // Should increase over time
    assert!(t_0_5 > t_0_0, "Should increase: {} > {}", t_0_5, t_0_0);
    assert!(t_1_0 > t_0_5, "Should increase: {} > {}", t_1_0, t_0_5);

    // Should end near 1
    assert!(t_1_0 > 0.9, "Should end near 1, got {}", t_1_0);
}

/// LEVEL 2: Curve Ramps Downward
/// Tests that curve decreases from start to end
#[test]
fn test_curve_downward() {
    let dsl = r#"
tempo: 1.0
~ramp: curve 1.0 0.0 1.0 0.0
out: ~ramp
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    let t_0_0 = samples[0];
    let t_0_5 = samples[(SAMPLE_RATE * 0.5) as usize];
    let t_1_0 = samples[(SAMPLE_RATE * 0.99) as usize];

    println!(
        "Curve values: t=0: {}, t=0.5: {}, t=1: {}",
        t_0_0, t_0_5, t_1_0
    );

    // Should start near 1
    assert!(t_0_0 > 0.9, "Should start at 1, got {}", t_0_0);

    // Should decrease over time
    assert!(t_0_5 < t_0_0, "Should decrease: {} < {}", t_0_5, t_0_0);
    assert!(t_1_0 < t_0_5, "Should decrease: {} < {}", t_1_0, t_0_5);

    // Should end near 0
    assert!(t_1_0 < 0.1, "Should end near 0, got {}", t_1_0);
}

/// LEVEL 2: Exponential Curve (Positive Curve Value)
/// Tests that positive curve creates exponential shape
#[test]
fn test_curve_exponential() {
    let dsl = r#"
tempo: 1.0
~exp_curve: curve 0.0 1.0 1.0 5.0
out: ~exp_curve
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    let t_0_25 = samples[(SAMPLE_RATE * 0.25) as usize];
    let t_0_50 = samples[(SAMPLE_RATE * 0.50) as usize];
    let t_0_75 = samples[(SAMPLE_RATE * 0.75) as usize];

    println!(
        "Exponential curve: t=0.25: {}, t=0.5: {}, t=0.75: {}",
        t_0_25, t_0_50, t_0_75
    );

    // Exponential: slow start, fast end
    // At t=0.25, should still be relatively low (< 0.1)
    assert!(
        t_0_25 < 0.1,
        "Exponential should start slow, got {} at t=0.25",
        t_0_25
    );

    // Growth should accelerate - difference between later intervals should be larger
    let delta_early = t_0_50 - t_0_25;
    let delta_late = t_0_75 - t_0_50;

    assert!(
        delta_late > delta_early,
        "Exponential should accelerate: late growth {} > early growth {}",
        delta_late,
        delta_early
    );
}

/// LEVEL 2: Logarithmic Curve (Negative Curve Value)
/// Tests that negative curve creates logarithmic shape
#[test]
fn test_curve_logarithmic() {
    let dsl = r#"
tempo: 1.0
~log_curve: curve 0.0 1.0 1.0 -5.0
out: ~log_curve
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    let t_0_25 = samples[(SAMPLE_RATE * 0.25) as usize];
    let t_0_50 = samples[(SAMPLE_RATE * 0.50) as usize];
    let t_0_75 = samples[(SAMPLE_RATE * 0.75) as usize];

    println!(
        "Logarithmic curve: t=0.25: {}, t=0.5: {}, t=0.75: {}",
        t_0_25, t_0_50, t_0_75
    );

    // Logarithmic: fast start, slow end
    // At t=0.25, should already be relatively high (> 0.3)
    assert!(
        t_0_25 > 0.3,
        "Logarithmic should start fast, got {} at t=0.25",
        t_0_25
    );

    // Growth should slow down toward end
    let delta_early = t_0_25;
    let delta_late = t_0_75 - t_0_50;

    assert!(
        delta_early > delta_late,
        "Should slow down: early growth {} > late growth {}",
        delta_early,
        delta_late
    );
}

/// LEVEL 2: Linear Curve (Zero Curve Value)
/// Tests that zero curve value creates linear ramp
#[test]
fn test_curve_linear() {
    let dsl = r#"
tempo: 1.0
~linear: curve 0.0 1.0 1.0 0.0
out: ~linear
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    let t_0_25 = samples[(SAMPLE_RATE * 0.25) as usize];
    let t_0_50 = samples[(SAMPLE_RATE * 0.50) as usize];
    let t_0_75 = samples[(SAMPLE_RATE * 0.75) as usize];

    println!(
        "Linear curve: t=0.25: {}, t=0.5: {}, t=0.75: {}",
        t_0_25, t_0_50, t_0_75
    );

    // Linear should be approximately proportional
    assert!(
        (t_0_25 - 0.25).abs() < 0.1,
        "Should be ~0.25 at t=0.25, got {}",
        t_0_25
    );
    assert!(
        (t_0_50 - 0.50).abs() < 0.1,
        "Should be ~0.50 at t=0.50, got {}",
        t_0_50
    );
    assert!(
        (t_0_75 - 0.75).abs() < 0.1,
        "Should be ~0.75 at t=0.75, got {}",
        t_0_75
    );
}

/// LEVEL 2: Curve Stability
/// Tests that curve doesn't produce NaN or Inf
#[test]
fn test_curve_stability() {
    let dsl = r#"
tempo: 1.0
~ramp: curve 0.0 1.0 2.0 3.0
out: ~ramp
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE * 2.0) as usize);

    // Check for NaN or Inf
    let has_nan = samples.iter().any(|s| s.is_nan() || s.is_infinite());
    assert!(!has_nan, "Curve should not produce NaN or Inf");

    // Check for reasonable output
    let max_val = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        max_val < 10.0,
        "Curve output should be reasonable, got max {}",
        max_val
    );
}

/// LEVEL 3: Musical Example - Filter Sweep
/// Tests curve for creating filter sweeps
#[test]
fn test_curve_filter_sweep() {
    let dsl = r#"
tempo: 1.0
~sweep: curve 200.0 4000.0 2.0 2.0
~carrier: saw 110
~filtered: ~carrier # lpf ~sweep 0.8
out: ~filtered * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE * 2.0) as usize);

    // Should produce audible swept sound
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    assert!(
        rms.sqrt() > 0.05,
        "Filter sweep should be audible, got RMS {}",
        rms.sqrt()
    );
}

/// LEVEL 3: Musical Example - Amplitude Fade
/// Tests curve for creating fades
#[test]
fn test_curve_fade() {
    let dsl = r#"
tempo: 1.0
~fade: curve 1.0 0.0 3.0 -2.0
~tone: sine 440
~faded: ~tone * ~fade
out: ~faded
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE * 3.0) as usize);

    // Should produce audible fade
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    assert!(
        rms.sqrt() > 0.05,
        "Fade should be audible, got RMS {}",
        rms.sqrt()
    );
}
