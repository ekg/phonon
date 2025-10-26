use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Pattern Query Verification
/// Tests that xline syntax is parsed and compiled correctly
#[test]
fn test_xline_pattern_query() {
    let dsl = r#"
tempo: 1.0
~envelope: xline 1.0 0.01 1.0
out: ~envelope
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
        "XLine should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: XLine Generates Exponential Curve
/// Tests that xline creates exponential (not linear) ramp
#[test]
fn test_xline_exponential_curve() {
    let dsl = r#"
tempo: 1.0
out: xline 1.0 0.001 1.0
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    // Render 1 second (duration of xline)
    let samples = graph.render(SAMPLE_RATE as usize);

    // Check start, middle, and end values
    let start_val = samples[0];
    let mid_val = samples[samples.len() / 2];
    let end_val = samples[samples.len() - 1];

    println!("Start: {}, Mid: {}, End: {}", start_val, mid_val, end_val);

    // Start should be near 1.0
    assert!(
        (start_val - 1.0).abs() < 0.1,
        "Should start near 1.0, got {}",
        start_val
    );

    // End should be near 0.001
    assert!(
        (end_val - 0.001).abs() < 0.01,
        "Should end near 0.001, got {}",
        end_val
    );

    // For exponential decay from 1.0 to 0.001:
    // Middle value should be closer to start than linear midpoint
    // Linear midpoint would be (1.0 + 0.001) / 2 = 0.5005
    // Exponential midpoint should be sqrt(1.0 * 0.001) ≈ 0.0316
    let linear_mid = (1.0 + 0.001) / 2.0;

    assert!(
        mid_val < linear_mid,
        "Exponential curve should have mid value < linear mid, got {} vs {}",
        mid_val,
        linear_mid
    );
}

/// LEVEL 2: XLine Duration Accuracy
/// Tests that xline completes in specified time
#[test]
fn test_xline_duration() {
    let dsl = r#"
tempo: 1.0
out: xline 1.0 0.1 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    // Render 0.75 seconds (longer than 0.5s duration)
    let samples = graph.render((SAMPLE_RATE * 0.75) as usize);

    // Check value at 0.5 seconds (should be near end value 0.1)
    let idx_at_duration = (SAMPLE_RATE * 0.5) as usize;
    let val_at_duration = samples[idx_at_duration];

    println!("Value at duration (0.5s): {}", val_at_duration);

    // Should be near 0.1 (within 10% error)
    assert!(
        (val_at_duration - 0.1).abs() < 0.02,
        "Should reach 0.1 at 0.5s, got {}",
        val_at_duration
    );

    // After duration, should hold at end value
    let val_after = samples[samples.len() - 1];
    assert!(
        (val_after - 0.1).abs() < 0.02,
        "Should hold end value after duration, got {}",
        val_after
    );
}

/// LEVEL 2: XLine Ascending and Descending
/// Tests that xline works in both directions
#[test]
fn test_xline_bidirectional() {
    // Descending (1.0 -> 0.01)
    let dsl_desc = r#"
tempo: 1.0
out: xline 1.0 0.01 1.0
"#;

    let (_, statements) = parse_program(dsl_desc).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples_desc = graph.render(1000);

    // Ascending (0.01 -> 1.0)
    let dsl_asc = r#"
tempo: 1.0
out: xline 0.01 1.0 1.0
"#;

    let (_, statements) = parse_program(dsl_asc).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples_asc = graph.render(1000);

    // Descending should decrease
    assert!(
        samples_desc[0] > samples_desc[999],
        "Descending xline should decrease"
    );

    // Ascending should increase
    assert!(
        samples_asc[0] < samples_asc[999],
        "Ascending xline should increase"
    );
}

/// LEVEL 2: XLine Range Coverage
/// Tests that xline covers full range from start to end
#[test]
fn test_xline_range() {
    let dsl = r#"
tempo: 1.0
out: xline 100.0 10.0 1.0
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    let max_val = samples.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
    let min_val = samples.iter().fold(f32::INFINITY, |a, &b| a.min(b));

    println!("Range: {} to {}", min_val, max_val);

    // Should cover approximately 100.0 to 10.0
    assert!(
        max_val >= 90.0,
        "Should reach near start value 100.0, got max {}",
        max_val
    );
    assert!(
        min_val <= 15.0,
        "Should reach near end value 10.0, got min {}",
        min_val
    );
}

/// LEVEL 3: Musical Example - Exponential Pitch Glide
/// Tests xline used for frequency sweep (sounds more natural than linear)
#[test]
fn test_xline_pitch_glide() {
    let dsl = r#"
tempo: 2.0
~pitch: xline 880.0 110.0 1.0
~tone: sine ~pitch
out: ~tone * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE * 0.5) as usize);

    // Should produce audible tone with changing pitch
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    assert!(
        rms.sqrt() > 0.1,
        "Pitch glide should be audible, got RMS {}",
        rms.sqrt()
    );
}

/// LEVEL 3: Musical Example - Exponential Fade Out
/// Tests xline used for natural-sounding volume decay
#[test]
fn test_xline_fade_out() {
    let dsl = r#"
tempo: 2.0
~amplitude: xline 1.0 0.001 1.0
~tone: sine 440
out: ~tone * ~amplitude
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE * 0.5) as usize);

    // Check that amplitude decreases over time
    let first_quarter_rms: f32 = samples[0..samples.len() / 4]
        .iter()
        .map(|s| s * s)
        .sum::<f32>()
        / (samples.len() / 4) as f32;

    let last_quarter_rms: f32 = samples[samples.len() * 3 / 4..]
        .iter()
        .map(|s| s * s)
        .sum::<f32>()
        / (samples.len() / 4) as f32;

    println!(
        "First quarter RMS: {}, Last quarter RMS: {}",
        first_quarter_rms.sqrt(),
        last_quarter_rms.sqrt()
    );

    assert!(
        first_quarter_rms > last_quarter_rms * 10.0,
        "Fade out should have much louder start than end"
    );
}

/// LEVEL 3: Pattern-Modulated XLine Parameters
/// Tests that xline parameters can be patterns
#[test]
fn test_xline_pattern_params() {
    let dsl = r#"
tempo: 1.0
~start_vals: "1.0 0.5"
~end_vals: "0.1 0.01"
~envelope: xline ~start_vals ~end_vals 0.5
out: ~envelope
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let graph = compile_program(statements, SAMPLE_RATE);

    assert!(
        graph.is_ok(),
        "Pattern-modulated xline should compile: {:?}",
        graph.err()
    );
}

/// LEVEL 3: XLine with Filter Cutoff Sweep
/// Tests xline modulating filter for natural sweep
#[test]
fn test_xline_filter_sweep() {
    let dsl = r#"
tempo: 2.0
~cutoff: xline 8000.0 200.0 1.0
~source: saw 110
~filtered: ~source # lpf ~cutoff 0.8
out: ~filtered * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE * 0.5) as usize);

    // Should produce audible filtered sweep
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    assert!(
        rms.sqrt() > 0.05,
        "Filter sweep should be audible, got RMS {}",
        rms.sqrt()
    );
}
