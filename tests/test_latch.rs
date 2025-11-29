use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Pattern Query Verification
/// Tests that Latch syntax is parsed and compiled correctly
#[test]
fn test_latch_pattern_query() {
    let dsl = r#"
tempo: 1.0
~input $ sine 440
~gate $ sine 8 # schmidt 0.5 -0.5
~held $ ~input # latch ~gate
out $ ~held
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
        "Latch should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: Latch Holds Values
/// Tests that latch holds input value when gate is low
#[test]
fn test_latch_holds_value() {
    // Input ramps from 0 to 1, gate triggers once at start
    let dsl = r#"
tempo: 1.0
~input $ line 0.0 1.0
~gate $ impulse 1.0
~held $ ~input # latch ~gate
out $ ~held
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // Sample at three points
    let early = samples[(SAMPLE_RATE * 0.1) as usize];
    let mid = samples[(SAMPLE_RATE * 0.5) as usize];
    let late = samples[(SAMPLE_RATE * 0.9) as usize];

    println!(
        "Latched values: early={}, mid={}, late={}",
        early, mid, late
    );

    // With only one impulse at the start, all values should be similar (near 0)
    // because the latch samples the input once and holds it
    assert!(
        (early - mid).abs() < 0.2,
        "Values should be held constant (early={}, mid={})",
        early,
        mid
    );
    assert!(
        (mid - late).abs() < 0.2,
        "Values should be held constant (mid={}, late={})",
        mid,
        late
    );
}

/// LEVEL 2: Latch Updates on Trigger
/// Tests that latch samples new value when gate goes high
#[test]
fn test_latch_updates_on_trigger() {
    // Use a slow sine wave as input and regular impulse triggers
    // This will show stepped sampling of the sine
    let dsl = r#"
tempo: 1.0
~input $ sine 1 * 0.5 + 0.5
~gate $ impulse 4.0
~held $ ~input # latch ~gate
out $ ~held
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // Impulse at 4 Hz gives triggers at t=0, 0.25, 0.5, 0.75
    // Sample shortly after each trigger to see the held values
    let s1 = samples[(SAMPLE_RATE * 0.02) as usize]; // Just after 1st trigger (t=0)
    let s2 = samples[(SAMPLE_RATE * 0.27) as usize]; // Just after 2nd trigger (t=0.25)
    let s3 = samples[(SAMPLE_RATE * 0.52) as usize]; // Just after 3rd trigger (t=0.5)
    let s4 = samples[(SAMPLE_RATE * 0.77) as usize]; // Just after 4th trigger (t=0.75)

    println!("Sampled values: {} {} {} {}", s1, s2, s3, s4);

    // Check that values form a sequence (sampling different points of the sine)
    // They should be different from each other
    assert!(
        (s2 - s1).abs() > 0.1,
        "2nd sample should differ from 1st: {} vs {}",
        s2,
        s1
    );
    assert!(
        (s3 - s2).abs() > 0.1,
        "3rd sample should differ from 2nd: {} vs {}",
        s3,
        s2
    );
}

/// LEVEL 2: Latch Creates Stepped Output
/// Tests that latch produces stepped (quantized) output
#[test]
fn test_latch_creates_steps() {
    let dsl = r#"
tempo: 1.0
~smooth $ sine 2
~clock $ impulse 10.0
~stepped $ ~smooth # latch ~clock
out $ ~stepped
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // Measure how much the signal varies
    // A smooth sine would have high variance, stepped output should have plateaus
    let windows = 10;
    let window_size = samples.len() / windows;
    let mut variances = Vec::new();

    for w in 0..windows {
        let start = w * window_size;
        let end = start + window_size;
        let window = &samples[start..end];

        let mean: f32 = window.iter().sum::<f32>() / window.len() as f32;
        let variance: f32 =
            window.iter().map(|&s| (s - mean).powi(2)).sum::<f32>() / window.len() as f32;

        variances.push(variance);
    }

    let avg_variance = variances.iter().sum::<f32>() / variances.len() as f32;

    println!("Average variance within windows: {}", avg_variance);

    // Stepped output should have low variance within windows
    assert!(
        avg_variance < 0.01,
        "Stepped output should have low variance, got {}",
        avg_variance
    );
}

/// LEVEL 2: Latch with Slow Gate
/// Tests that latch holds value between slow triggers
#[test]
fn test_latch_slow_gate() {
    let dsl = r#"
tempo: 1.0
~noise $ white_noise
~slow_gate $ impulse 2.0
~held $ ~noise # latch ~slow_gate
out $ ~held * 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // Check that consecutive samples are identical (held)
    // Between triggers, all samples should be the same
    let start = (SAMPLE_RATE * 0.1) as usize;
    let end = (SAMPLE_RATE * 0.4) as usize;
    let first_val = samples[start];

    let all_same = samples[start..end]
        .iter()
        .all(|&s| (s - first_val).abs() < 0.0001);

    println!("All samples identical between triggers: {}", all_same);
    assert!(all_same, "Values should be held constant between triggers");
}

/// LEVEL 2: Latch Stability
/// Tests that latch doesn't produce NaN or Inf
#[test]
fn test_latch_stability() {
    let dsl = r#"
tempo: 1.0
~noise $ white_noise
~gate $ impulse 20.0
~held $ ~noise # latch ~gate
out $ ~held * 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // Check for NaN or Inf
    let has_nan = samples.iter().any(|s| s.is_nan() || s.is_infinite());
    assert!(!has_nan, "Latch should not produce NaN or Inf");

    // Check for reasonable output
    let max_val = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        max_val < 10.0,
        "Latch output should be reasonable, got max {}",
        max_val
    );
}

/// LEVEL 3: Musical Example - Random Melody
/// Tests latch for creating random stepped melodies
#[test]
fn test_latch_random_melody() {
    let dsl = r#"
tempo: 0.5
~noise $ white_noise * 200.0 + 440.0
~notes $ impulse 8.0
~freq $ ~noise # latch ~notes
~melody $ sine ~freq
out $ ~melody * 0.2
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE * 2.0) as usize);

    // Should produce audible stepped melody
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    assert!(
        rms.sqrt() > 0.05,
        "Random melody should be audible, got RMS {}",
        rms.sqrt()
    );
}

/// LEVEL 3: Sample & Hold Effect
/// Tests latch for classic sample & hold effect
#[test]
fn test_latch_sample_hold_effect() {
    let dsl = r#"
tempo: 1.0
~carrier $ saw 110
~modulator $ white_noise
~clock $ impulse 16.0
~held $ ~modulator # latch ~clock
~filtered $ ~carrier # lpf ((~held * 0.5 + 0.5) * 2000.0 + 200.0) 0.8
out $ ~filtered * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // Should produce audible sample & hold effect
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    assert!(
        rms.sqrt() > 0.05,
        "Sample & hold effect should be audible, got RMS {}",
        rms.sqrt()
    );
}

/// LEVEL 3: Pattern-Modulated Gate
/// Tests that latch gate can be pattern-modulated
#[test]
fn test_latch_pattern_gate() {
    let dsl = r#"
tempo: 0.5
~input $ sine 1
~gates $ "0.0 1.0 0.0 1.0"
~held $ ~input # latch ~gates
out $ ~held
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let graph = compile_program(statements, SAMPLE_RATE, None);

    assert!(
        graph.is_ok(),
        "Pattern-modulated latch should compile: {:?}",
        graph.err()
    );
}
