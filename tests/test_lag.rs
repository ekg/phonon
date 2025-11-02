use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Pattern Query Verification
/// Tests that lag syntax is parsed and compiled correctly
#[test]
fn test_lag_pattern_query() {
    let dsl = r#"
tempo: 1.0
~input: sine 1.0
~smoothed: lag ~input 0.1
out: ~smoothed
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
        "Lag should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: Lag Smooths Abrupt Changes
/// Tests that lag smooths a step function
#[test]
fn test_lag_smooth_step() {
    let dsl = r#"
tempo: 1.0
~step: "0.0 1.0"
~smoothed: lag ~step 0.01
out: ~smoothed
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    // Render enough samples to see the lag effect
    let samples = graph.render(1000);

    // Check that we don't have instant jumps
    let mut max_diff = 0.0f32;
    for i in 1..samples.len() {
        let diff = (samples[i] - samples[i - 1]).abs();
        max_diff = max_diff.max(diff);
    }

    println!("Max sample-to-sample difference: {}", max_diff);

    // With lag, changes should be gradual (not instant jumps from 0 to 1)
    assert!(
        max_diff < 0.5,
        "Lag should smooth changes, max diff should be < 0.5, got {}",
        max_diff
    );
}

/// LEVEL 2: Faster Lag Time = Faster Response
/// Tests that smaller lag values respond faster
#[test]
fn test_lag_time_response() {
    let dsl_fast = r#"
tempo: 1.0
~step: line 0.0 1.0
~smoothed: lag ~step 0.001
out: ~smoothed
"#;

    let dsl_slow = r#"
tempo: 1.0
~step: line 0.0 1.0
~smoothed: lag ~step 0.1
out: ~smoothed
"#;

    let (_, statements_fast) = parse_program(dsl_fast).unwrap();
    let mut graph_fast = compile_program(statements_fast, SAMPLE_RATE).unwrap();

    let (_, statements_slow) = parse_program(dsl_slow).unwrap();
    let mut graph_slow = compile_program(statements_slow, SAMPLE_RATE).unwrap();

    let samples_fast = graph_fast.render(1000);
    let samples_slow = graph_slow.render(1000);

    // Check how quickly they respond - fast lag should track input more closely
    // Calculate how close each is to the input (line going from 0 to 1)
    let mut fast_total_error = 0.0f32;
    let mut slow_total_error = 0.0f32;

    for i in 0..1000 {
        let expected = i as f32 / 1000.0; // Line goes from 0 to 1
        fast_total_error += (samples_fast[i] - expected).abs();
        slow_total_error += (samples_slow[i] - expected).abs();
    }

    println!("Fast lag total error: {}", fast_total_error);
    println!("Slow lag total error: {}", slow_total_error);

    // Fast lag should have less error (track input more closely)
    assert!(
        fast_total_error < slow_total_error,
        "Fast lag should track input more closely (less error)"
    );
}

/// LEVEL 2: Lag Preserves DC Level
/// Tests that constant input produces constant output
#[test]
fn test_lag_dc_preservation() {
    let dsl = r#"
tempo: 1.0
~constant: sine 0.0
~lagged: lag ~constant 0.1
out: ~lagged
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render(1000);

    // All samples should be approximately the same (constant)
    let mean = samples.iter().sum::<f32>() / samples.len() as f32;
    let variance = samples.iter().map(|s| (s - mean).powi(2)).sum::<f32>() / samples.len() as f32;

    println!("Mean: {}, Variance: {}", mean, variance);

    assert!(
        variance < 0.001,
        "Constant input should produce constant output, got variance {}",
        variance
    );
}

/// LEVEL 2: Lag with Low Frequency Sine
/// Tests that lag smooths a slow-moving signal
#[test]
fn test_lag_with_lfo() {
    let dsl = r#"
tempo: 1.0
~lfo: sine 0.5
~lagged: lag ~lfo 0.05
out: ~lagged
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE * 2.0) as usize);

    // Should produce audible signal
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    assert!(
        rms.sqrt() > 0.1,
        "Lagged LFO should be audible, got RMS {}",
        rms.sqrt()
    );
}

/// LEVEL 3: Musical Example - Portamento
/// Tests lag used for pitch glide effect
#[test]
fn test_lag_portamento() {
    let dsl = r#"
tempo: 2.0
~notes: "220 330 440 550"
~smooth_freq: lag ~notes 0.05
~tone: sine ~smooth_freq
out: ~tone * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE * 0.5) as usize);

    // Should produce smooth gliding tones
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    assert!(
        rms.sqrt() > 0.1,
        "Portamento should be audible, got RMS {}",
        rms.sqrt()
    );
}

/// LEVEL 3: Lag Removes Clicks
/// Tests that lag smooths out abrupt amplitude changes
#[test]
fn test_lag_click_removal() {
    let dsl = r#"
tempo: 4.0
~gate: "0.0 1.0 0.0 1.0"
~smooth_gate: lag ~gate 0.01
~tone: sine 440
out: ~tone * ~smooth_gate * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE * 0.5) as usize);

    // Calculate maximum sample-to-sample difference (should be small with lag)
    let mut max_diff = 0.0f32;
    for i in 1..samples.len() {
        let diff = (samples[i] - samples[i - 1]).abs();
        max_diff = max_diff.max(diff);
    }

    println!("Max difference with lag: {}", max_diff);

    // With lag, there should be no sudden jumps (which cause clicks)
    assert!(
        max_diff < 0.1,
        "Lag should prevent clicks, max diff should be small, got {}",
        max_diff
    );
}

/// LEVEL 3: Pattern-Modulated Lag Time
/// Tests that lag time can be modulated
#[test]
fn test_lag_pattern_time() {
    let dsl = r#"
tempo: 1.0
~input: sine 1.0
~lag_times: "0.001 0.1"
~smoothed: lag ~input ~lag_times
out: ~smoothed
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let graph = compile_program(statements, SAMPLE_RATE);

    assert!(
        graph.is_ok(),
        "Pattern-modulated lag time should compile: {:?}",
        graph.err()
    );
}

/// LEVEL 3: Zero Lag Time Bypass
/// Tests that lag time of 0 passes signal through unchanged
#[test]
fn test_lag_zero_bypass() {
    let dsl = r#"
tempo: 1.0
~input: sine 440
~lagged: lag ~input 0.0
out: ~lagged
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render(1000);

    // Should have signal (not silence)
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    assert!(
        rms.sqrt() > 0.3,
        "Zero lag should pass signal through, got RMS {}",
        rms.sqrt()
    );
}
