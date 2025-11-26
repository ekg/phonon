use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Pattern Query Verification
/// Tests that Peak Follower syntax is parsed and compiled correctly
#[test]
fn test_peak_follower_pattern_query() {
    let dsl = r#"
tempo: 1.0
~input: sine 440
~peak: ~input # peak_follower 0.01 0.1
out: ~peak
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
        "Peak Follower should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: Peak Follower Tracks Peaks
/// Tests that peak follower follows the peak amplitude
#[test]
fn test_peak_follower_tracks_peaks() {
    // Slow sine wave - peak follower should track peak values
    let dsl = r#"
tempo: 1.0
~slow_sine: sine 2
~peak: ~slow_sine # peak_follower 0.001 0.1
out: ~peak
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE * 2.0) as usize);

    // Find peak output value - should be close to 1.0 (peak of sine)
    let max_peak = samples.iter().fold(0.0f32, |a, &b| a.max(b));

    println!("Max peak value: {}", max_peak);

    // Peak follower should reach close to the actual peak (1.0)
    assert!(
        max_peak > 0.9,
        "Peak follower should track peak amplitude, got {}",
        max_peak
    );
}

/// LEVEL 2: Peak Follower Decays
/// Tests that peak follower decays when signal drops
#[test]
fn test_peak_follower_decays() {
    // Impulse creates sharp peak then drops to zero
    // Peak follower should decay gradually
    let dsl = r#"
tempo: 1.0
~impulses: impulse 2.0
~peak: ~impulses # peak_follower 0.001 0.05
out: ~peak
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // After first impulse (t=0), check values during decay
    let t_0_05 = samples[(SAMPLE_RATE * 0.05) as usize]; // 50ms after impulse
    let t_0_10 = samples[(SAMPLE_RATE * 0.10) as usize]; // 100ms after impulse
    let t_0_20 = samples[(SAMPLE_RATE * 0.20) as usize]; // 200ms after impulse

    println!(
        "Decay values: t=0.05: {}, t=0.1: {}, t=0.2: {}",
        t_0_05, t_0_10, t_0_20
    );

    // Values should decrease over time (decay)
    assert!(t_0_05 > t_0_10, "Should decay: {} > {}", t_0_05, t_0_10);
    assert!(t_0_10 > t_0_20, "Should decay: {} > {}", t_0_10, t_0_20);
}

/// LEVEL 2: Fast Attack Time
/// Tests that peak follower responds quickly to increases with fast attack
#[test]
fn test_peak_follower_fast_attack() {
    // Square wave alternates between -1 and 1
    // Fast attack should quickly reach peak
    let dsl = r#"
tempo: 1.0
~square: square 10
~peak: ~square # peak_follower 0.0001 0.1
out: ~peak
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE * 0.2) as usize);

    // With very fast attack (0.1ms), should reach near peak quickly
    let max_in_first_100ms = samples[..((SAMPLE_RATE * 0.1) as usize)]
        .iter()
        .fold(0.0f32, |a, &b| a.max(b));

    println!(
        "Max in first 100ms with fast attack: {}",
        max_in_first_100ms
    );

    assert!(
        max_in_first_100ms > 0.8,
        "Fast attack should reach peak quickly, got {}",
        max_in_first_100ms
    );
}

/// LEVEL 2: Slow Release Time
/// Tests that peak follower decays slowly with long release time
#[test]
fn test_peak_follower_slow_release() {
    // Sine wave pulsed by slow LFO - creates peaks with gaps
    let dsl = r#"
tempo: 1.0
~tone: sine 440
~pulse: square 4 * 0.5 + 0.5
~pulsed: ~tone * ~pulse
~peak: ~pulsed # peak_follower 0.01 0.3
out: ~peak
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE * 1.0) as usize);

    // Check values at different times
    // Square wave at 4Hz means period is 0.25s
    // High from 0-0.125, low from 0.125-0.25
    let t_0_10 = samples[(SAMPLE_RATE * 0.10) as usize]; // During high portion
    let t_0_20 = samples[(SAMPLE_RATE * 0.20) as usize]; // During low/decay portion
    let t_0_30 = samples[(SAMPLE_RATE * 0.30) as usize]; // During next high

    println!(
        "Slow release: t=0.1: {}, t=0.2: {}, t=0.3: {}",
        t_0_10, t_0_20, t_0_30
    );

    // During high portion, should reach near peak
    assert!(
        t_0_10 > 0.8,
        "Should reach peak during high portion, got {}",
        t_0_10
    );

    // During decay, should be lower than peak but not zero (slow release)
    assert!(
        t_0_20 < t_0_10 && t_0_20 > 0.3,
        "Should decay slowly: {} should be between 0.3 and {}",
        t_0_20,
        t_0_10
    );
}

/// LEVEL 2: Peak Follower Stability
/// Tests that peak follower doesn't produce NaN or Inf
#[test]
fn test_peak_follower_stability() {
    let dsl = r#"
tempo: 1.0
~noise: white_noise
~peak: ~noise # peak_follower 0.01 0.1
out: ~peak * 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // Check for NaN or Inf
    let has_nan = samples.iter().any(|s| s.is_nan() || s.is_infinite());
    assert!(!has_nan, "Peak follower should not produce NaN or Inf");

    // Check for reasonable output
    let max_val = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        max_val < 10.0,
        "Peak follower output should be reasonable, got max {}",
        max_val
    );
}

/// LEVEL 3: Musical Example - Envelope Follower
/// Tests peak follower for extracting amplitude envelope
#[test]
fn test_peak_follower_envelope() {
    // Sine wave amplitude modulated by slow LFO
    // Peak follower should extract the envelope
    let dsl = r#"
tempo: 1.0
~lfo: sine 2 * 0.5 + 0.5
~carrier: sine 440 * ~lfo
~envelope: ~carrier # peak_follower 0.01 0.05
out: ~envelope
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE * 2.0) as usize);

    // Envelope should produce audible varying signal
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    assert!(
        rms.sqrt() > 0.05,
        "Envelope follower should produce audible output, got RMS {}",
        rms.sqrt()
    );
}

/// LEVEL 3: Musical Example - Dynamics Sidechain
/// Tests peak follower for sidechain compression effect
#[test]
fn test_peak_follower_sidechain() {
    // Use kick pattern to modulate sustained bass
    let dsl = r#"
tempo: 2.0
~kick: impulse 4.0
~kick_env: ~kick # peak_follower 0.001 0.2
~bass: saw 55
~ducked: ~bass * (1.0 - ~kick_env * 0.8)
out: ~ducked * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE * 2.0) as usize);

    // Should produce audible ducking effect
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    assert!(
        rms.sqrt() > 0.05,
        "Sidechain effect should be audible, got RMS {}",
        rms.sqrt()
    );
}

/// LEVEL 3: Peak Follower with Variable Parameters
/// Tests that peak follower parameters can be pattern-modulated
#[test]
fn test_peak_follower_variable_params() {
    let dsl = r#"
tempo: 1.0
~input: sine 440
~attack_mod: sine 0.5 * 0.01 + 0.01
~release_mod: sine 0.3 * 0.1 + 0.1
~peak: ~input # peak_follower ~attack_mod ~release_mod
out: ~peak
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let graph = compile_program(statements, SAMPLE_RATE, None);

    assert!(
        graph.is_ok(),
        "Peak follower with variable params should compile: {:?}",
        graph.err()
    );
}
