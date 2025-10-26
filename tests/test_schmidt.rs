use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Pattern Query Verification
/// Tests that Schmidt trigger syntax is parsed and compiled correctly
#[test]
fn test_schmidt_pattern_query() {
    let dsl = r#"
tempo: 1.0
~input: sine 2
~gate: ~input # schmidt 0.5 -0.5
out: ~gate
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
        "Schmidt trigger should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: Schmidt Creates Gate from Sine
/// Tests that Schmidt trigger converts continuous signal to gate
#[test]
fn test_schmidt_creates_gate() {
    let dsl = r#"
tempo: 1.0
~sine: sine 2
~gate: ~sine # schmidt 0.5 -0.5
out: ~gate
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE * 2.0) as usize);

    // Schmidt trigger should output only 0.0 or 1.0
    let has_low = samples.iter().any(|&s| s < 0.1);
    let has_high = samples.iter().any(|&s| s > 0.9);

    println!("Has low state: {}, Has high state: {}", has_low, has_high);

    // Should have both 0.0 and 1.0 states
    assert!(has_low, "Should have low state (0.0)");
    assert!(has_high, "Should have high state (1.0)");
}

/// LEVEL 2: Hysteresis Prevents Rapid Oscillation
/// Tests that Schmidt trigger has hysteresis (different on/off thresholds)
#[test]
fn test_schmidt_hysteresis() {
    // Create a signal that oscillates around 0.0
    // With hysteresis, it should NOT trigger rapidly
    let dsl = r#"
tempo: 1.0
~noisy: sine 20 * 0.3
~gate: ~noisy # schmidt 0.5 -0.5
out: ~gate
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // Count transitions (0→1 or 1→0)
    let mut transitions = 0;
    for i in 1..samples.len() {
        if (samples[i] - samples[i - 1]).abs() > 0.5 {
            transitions += 1;
        }
    }

    println!("Transitions in 1 second: {}", transitions);

    // With hysteresis, transitions should be relatively few
    // A sine at 20 Hz has 20 cycles, so max 40 transitions (20 up, 20 down)
    // But with amplitude 0.3, it never reaches ±0.5, so should be 0 transitions
    assert!(
        transitions == 0,
        "With hysteresis and low amplitude, should have 0 transitions, got {}",
        transitions
    );
}

/// LEVEL 2: High Threshold Triggers On
/// Tests that signal must exceed high threshold to turn on
#[test]
fn test_schmidt_high_threshold() {
    // Use a slow sine wave that goes from 0.0 to 1.0 over 1 second
    let dsl = r#"
tempo: 1.0
~sine: sine 0.5 * 0.5 + 0.5
~gate: ~sine # schmidt 0.7 0.3
out: ~gate
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE * 2.0) as usize);

    // At t=0.0s: sine = 0.5 (middle), gate should be LOW (starts at 0)
    // At t=0.25s: sine ≈ 1.0 (peak), gate should be HIGH (exceeded 0.7)
    // At t=0.5s: sine = 0.5 (middle), gate should still be HIGH (above low threshold 0.3)
    // At t=1.0s: sine = 0.5 (middle returning), gate should still be HIGH
    let t0 = samples[0];
    let t_quarter = samples[(SAMPLE_RATE * 0.25) as usize];
    let t_half = samples[(SAMPLE_RATE * 0.5) as usize];

    println!("Samples at t=0: {}, t=0.25: {}, t=0.5: {}", t0, t_quarter, t_half);

    // Initially LOW (sine starts at 0.5, below high threshold 0.7)
    assert!(t0 < 0.5, "Should start LOW");

    // After quarter cycle, should be HIGH (sine peaked above 0.7)
    assert!(t_quarter > 0.5, "Should be HIGH after exceeding high threshold");

    // At half cycle, should still be HIGH (hysteresis - hasn't fallen below 0.3)
    assert!(t_half > 0.5, "Should stay HIGH (hysteresis)");
}

/// LEVEL 2: Low Threshold Triggers Off
/// Tests that signal must fall below low threshold to turn off
#[test]
fn test_schmidt_low_threshold() {
    // Sine that goes: 0.5 → 1.0 (peak) → 0.5 → 0.0 (trough)
    // sine(1 Hz) * 0.5 + 0.5 gives range [0.0, 1.0]
    let dsl = r#"
tempo: 1.0
~sine: sine 1 * 0.5 + 0.5
~gate: ~sine # schmidt 0.7 0.3
out: ~gate
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // At t=0.0s: sine(0) = 0, so 0*0.5+0.5 = 0.5, gate starts LOW
    // At t=0.25s: sine(π/2) = 1, so 1*0.5+0.5 = 1.0, gate HIGH (exceeded 0.7)
    // At t=0.75s: sine(3π/2) = -1, so -1*0.5+0.5 = 0.0, gate LOW (fell below 0.3)
    let t0 = samples[0];
    let t_peak = samples[(SAMPLE_RATE * 0.30) as usize];    // After peak (t=0.25s)
    let t_trough = samples[(SAMPLE_RATE * 0.75) as usize];  // At trough

    println!(
        "Samples at t=0: {}, t=0.30: {}, t=0.75: {}",
        t0, t_peak, t_trough
    );

    // Initially LOW (0.5 < 0.7 high threshold)
    assert!(t0 < 0.5, "Should start LOW");

    // After rising above 0.7, should be HIGH
    assert!(t_peak > 0.5, "Should be HIGH after rising above high threshold");

    // At trough (signal = 0.0, below low threshold 0.3), should be LOW
    assert!(t_trough < 0.5, "Should be LOW after falling below low threshold");
}

/// LEVEL 2: Schmidt Stability
/// Tests that Schmidt doesn't produce NaN or Inf
#[test]
fn test_schmidt_stability() {
    let dsl = r#"
tempo: 1.0
~noise: white_noise
~gate: ~noise # schmidt 0.3 -0.3
out: ~gate * 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // Check for NaN or Inf
    let has_nan = samples.iter().any(|s| s.is_nan() || s.is_infinite());
    assert!(!has_nan, "Schmidt should not produce NaN or Inf");

    // Output should be 0.0 or 1.0 (or 0.5 after multiplication)
    let all_valid = samples.iter().all(|&s| s >= 0.0 && s <= 0.5);
    assert!(all_valid, "Schmidt output should be in range [0, 0.5]");
}

/// LEVEL 3: Musical Example - Gate from LFO
/// Tests Schmidt used to create rhythmic gates from LFO
#[test]
fn test_schmidt_gate_from_lfo() {
    let dsl = r#"
tempo: 2.0
~lfo: sine 4
~gate: ~lfo # schmidt 0.3 -0.3
~pulse: saw 220 * ~gate
out: ~pulse * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE * 2.0) as usize);

    // Should produce audible pulsing
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    assert!(
        rms.sqrt() > 0.05,
        "Gate from LFO should produce audible output, got RMS {}",
        rms.sqrt()
    );
}

/// LEVEL 3: Pattern-Modulated Thresholds
/// Tests that Schmidt thresholds can be pattern-modulated
#[test]
fn test_schmidt_pattern_thresholds() {
    let dsl = r#"
tempo: 2.0
~input: sine 2
~highs: "0.5 0.7"
~lows: "-0.5 -0.7"
~gate: ~input # schmidt ~highs ~lows
out: ~gate
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let graph = compile_program(statements, SAMPLE_RATE);

    assert!(
        graph.is_ok(),
        "Pattern-modulated Schmidt should compile: {:?}",
        graph.err()
    );
}

/// LEVEL 3: Envelope Gate Detection
/// Tests Schmidt for converting envelopes to gates
#[test]
fn test_schmidt_envelope_gate() {
    let dsl = r#"
tempo: 1.0
~env: line 1.0 0.0
~gate: ~env # schmidt 0.5 0.4
~tone: sine 440 * ~gate
out: ~tone * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // Should produce audible tone while gate is high
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    assert!(
        rms.sqrt() > 0.05,
        "Envelope gate should produce audible output, got RMS {}",
        rms.sqrt()
    );
}
