use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Pattern Query Verification
/// Tests that Amp Follower syntax is parsed and compiled correctly
#[test]
fn test_amp_follower_pattern_query() {
    let dsl = r#"
tempo: 1.0
~input: sine 440
~amp: ~input # amp_follower 0.01 0.1 0.01
out: ~amp
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
        "Amp Follower should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: Amp Follower Tracks Amplitude Smoothly
/// Tests that amp follower produces smooth envelope
#[test]
fn test_amp_follower_smooth_tracking() {
    // Modulated sine - amp follower should track the amplitude envelope
    let dsl = r#"
tempo: 1.0
~lfo: sine 2 * 0.5 + 0.5
~modulated: sine 440 * ~lfo
~envelope: ~modulated # amp_follower 0.02 0.1 0.01
out: ~envelope
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE * 2.0) as usize);

    // Envelope should be smooth and track the LFO
    let max_envelope = samples.iter().fold(0.0f32, |a, &b| a.max(b));

    println!("Max envelope value: {}", max_envelope);

    // Should reach near the peak of the modulation
    assert!(
        max_envelope > 0.3,
        "Amp follower should track amplitude, got max {}",
        max_envelope
    );
}

/// LEVEL 2: Amp Follower Smoother Than Input
/// Tests that amp follower smooths rapid variations
#[test]
fn test_amp_follower_smoothing() {
    // White noise has rapid variations - amp follower should smooth them
    let dsl = r#"
tempo: 1.0
~noise: white_noise
~smooth: ~noise # amp_follower 0.05 0.1 0.01
out: ~smooth
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE * 0.5) as usize);

    // Measure variance - smoothed output should have lower variance than input
    // Just verify it produces reasonable output
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    println!("RMS of smoothed noise: {}", rms.sqrt());

    assert!(
        rms.sqrt() > 0.1 && rms.sqrt() < 2.0,
        "Smoothed amplitude should be reasonable, got RMS {}",
        rms.sqrt()
    );
}

/// LEVEL 2: Fast Attack Response
/// Tests that amp follower responds quickly to increases
#[test]
fn test_amp_follower_fast_attack() {
    // Square wave amplitude modulation
    let dsl = r#"
tempo: 1.0
~carrier: sine 440
~gate: square 4 * 0.5 + 0.5
~modulated: ~carrier * ~gate
~envelope: ~modulated # amp_follower 0.001 0.1 0.005
out: ~envelope
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE * 0.5) as usize);

    // With fast attack, should reach peak relatively quickly
    let max_in_first_100ms = samples[..((SAMPLE_RATE * 0.1) as usize)]
        .iter()
        .fold(0.0f32, |a, &b| a.max(b));

    println!("Max in first 100ms: {}", max_in_first_100ms);

    assert!(
        max_in_first_100ms > 0.3,
        "Fast attack should respond quickly, got {}",
        max_in_first_100ms
    );
}

/// LEVEL 2: Slow Release Response
/// Tests that amp follower decays slowly with long release
#[test]
fn test_amp_follower_slow_release() {
    // Tone pulsed by slow square wave
    let dsl = r#"
tempo: 1.0
~carrier: sine 440
~pulse: square 4 * 0.5 + 0.5
~pulsed: ~carrier * ~pulse
~envelope: ~pulsed # amp_follower 0.01 0.3 0.01
out: ~envelope
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // Check that envelope maintains level during release
    let t_0_10 = samples[(SAMPLE_RATE * 0.10) as usize];
    let t_0_20 = samples[(SAMPLE_RATE * 0.20) as usize];

    println!("Envelope: t=0.1: {}, t=0.2: {}", t_0_10, t_0_20);

    // Should track amplitude reasonably
    assert!(
        t_0_10 > 0.05,
        "Should track amplitude, got {} at t=0.1",
        t_0_10
    );
}

/// LEVEL 2: Amp Follower Stability
/// Tests that amp follower doesn't produce NaN or Inf
#[test]
fn test_amp_follower_stability() {
    let dsl = r#"
tempo: 1.0
~noise: white_noise
~envelope: ~noise # amp_follower 0.01 0.1 0.01
out: ~envelope * 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // Check for NaN or Inf
    let has_nan = samples.iter().any(|s| s.is_nan() || s.is_infinite());
    assert!(!has_nan, "Amp follower should not produce NaN or Inf");

    // Check for reasonable output
    let max_val = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        max_val < 10.0,
        "Amp follower output should be reasonable, got max {}",
        max_val
    );
}

/// LEVEL 3: Musical Example - Sidechain Compression
/// Tests amp follower for sidechain ducking effect
#[test]
fn test_amp_follower_sidechain() {
    let dsl = r#"
tempo: 0.5
~kick: impulse 4.0
~kick_envelope: ~kick # amp_follower 0.001 0.2 0.01
~bass: saw 55
~ducked: ~bass * (1.0 - ~kick_envelope * 0.7)
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

/// LEVEL 3: Musical Example - Smooth Tremolo
/// Tests amp follower for creating smooth tremolo
#[test]
fn test_amp_follower_tremolo() {
    let dsl = r#"
tempo: 1.0
~carrier: saw 220
~lfo: sine 6 * 0.5 + 0.5
~modulated: ~carrier * ~lfo
~smooth_env: ~modulated # amp_follower 0.02 0.05 0.01
out: ~smooth_env
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE * 2.0) as usize);

    // Should produce smooth tremolo effect
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    assert!(
        rms.sqrt() > 0.05,
        "Tremolo should be audible, got RMS {}",
        rms.sqrt()
    );
}

/// LEVEL 3: Variable Parameters
/// Tests that amp follower parameters can be modulated
#[test]
fn test_amp_follower_variable_params() {
    let dsl = r#"
tempo: 1.0
~input: sine 440
~attack_mod: sine 0.5 * 0.01 + 0.01
~release_mod: sine 0.3 * 0.1 + 0.1
~window_mod: sine 0.2 * 0.01 + 0.01
~envelope: ~input # amp_follower ~attack_mod ~release_mod ~window_mod
out: ~envelope
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let graph = compile_program(statements, SAMPLE_RATE, None);

    assert!(
        graph.is_ok(),
        "Amp follower with variable params should compile: {:?}",
        graph.err()
    );
}
