use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Pattern Query Verification
/// Tests that comb filter syntax is parsed and compiled correctly
#[test]
fn test_comb_pattern_query() {
    let dsl = r#"
tempo: 1.0
~input: impulse 1.0
~resonant: ~input # comb 440 0.9
out: ~resonant
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
        "Comb filter should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: Comb Creates Resonance
/// Tests that comb filter creates sustained resonance from impulse
#[test]
fn test_comb_creates_resonance() {
    let dsl = r#"
tempo: 1.0
~impulse: impulse 1.0
~resonant: ~impulse # comb 440 0.99
out: ~resonant
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // Calculate RMS over time to see if resonance sustains
    let early_rms: f32 = samples[0..1000]
        .iter()
        .map(|s| s * s)
        .sum::<f32>()
        / 1000.0;

    let late_rms: f32 = samples[SAMPLE_RATE as usize / 2..]
        .iter()
        .map(|s| s * s)
        .sum::<f32>()
        / (SAMPLE_RATE as usize / 2) as f32;

    println!("Early RMS: {}, Late RMS: {}", early_rms.sqrt(), late_rms.sqrt());

    // With high feedback, resonance should sustain
    assert!(
        late_rms.sqrt() > 0.005,
        "Comb should sustain resonance with high feedback, got late RMS {}",
        late_rms.sqrt()
    );
}

/// LEVEL 2: Feedback Amount Affects Decay
/// Tests that higher feedback = longer sustain
#[test]
fn test_comb_feedback_decay() {
    // Low feedback (short decay)
    let dsl_low = r#"
tempo: 1.0
~impulse: impulse 1.0
~resonant: ~impulse # comb 440 0.5
out: ~resonant
"#;

    let (_, statements) = parse_program(dsl_low).unwrap();
    let mut graph_low = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples_low = graph_low.render(SAMPLE_RATE as usize);

    // High feedback (long decay)
    let dsl_high = r#"
tempo: 1.0
~impulse: impulse 1.0
~resonant: ~impulse # comb 440 0.95
out: ~resonant
"#;

    let (_, statements) = parse_program(dsl_high).unwrap();
    let mut graph_high = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples_high = graph_high.render(SAMPLE_RATE as usize);

    // Measure tail energy (last half of buffer)
    let tail_start = SAMPLE_RATE as usize / 2;
    let low_tail: f32 = samples_low[tail_start..]
        .iter()
        .map(|s| s * s)
        .sum::<f32>();

    let high_tail: f32 = samples_high[tail_start..]
        .iter()
        .map(|s| s * s)
        .sum::<f32>();

    println!("Low feedback tail energy: {}, High feedback tail energy: {}", low_tail, high_tail);

    assert!(
        high_tail > low_tail * 2.0,
        "High feedback should have more tail energy than low feedback"
    );
}

/// LEVEL 2: Comb Tuning (Frequency)
/// Tests that comb can be tuned to specific frequencies
#[test]
fn test_comb_tuning() {
    let dsl = r#"
tempo: 1.0
~impulse: impulse 2.0
~comb_a: ~impulse # comb 220 0.9
~comb_b: ~impulse # comb 440 0.9
out: ~comb_a * 0.5 + ~comb_b * 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // Should produce audible resonance
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    assert!(
        rms.sqrt() > 0.01,
        "Tuned comb filters should be audible, got RMS {}",
        rms.sqrt()
    );
}

/// LEVEL 2: Comb Stability
/// Tests that comb doesn't blow up even with high feedback
#[test]
fn test_comb_stability() {
    let dsl = r#"
tempo: 1.0
~noise: white_noise
~combed: ~noise # comb 1000 0.99
out: ~combed * 0.1
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // Check for NaN or Inf
    let has_nan = samples.iter().any(|s| s.is_nan() || s.is_infinite());
    assert!(!has_nan, "Comb filter should not produce NaN or Inf");

    // Check for reasonable output (shouldn't blow up)
    let max_val = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        max_val < 10.0,
        "Comb filter should not blow up, got max {}",
        max_val
    );
}

/// LEVEL 3: Musical Example - Bell/Metallic Sound
/// Tests comb used to create bell-like resonance
#[test]
fn test_comb_bell_sound() {
    let dsl = r#"
tempo: 1.0
~strike: impulse 1.0
~bell: ~strike # comb 440 0.98
out: ~bell * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE * 2.0) as usize);

    // Should have audible tail (bell-like decay)
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    assert!(
        rms.sqrt() > 0.005,
        "Bell sound should be audible, got RMS {}",
        rms.sqrt()
    );
}

/// LEVEL 3: Pattern-Modulated Comb Frequency
/// Tests that comb frequency can be patterns
#[test]
fn test_comb_pattern_frequency() {
    let dsl = r#"
tempo: 1.0
~impulse: impulse 2.0
~freqs: "220 330 440 550"
~combed: ~impulse # comb ~freqs 0.9
out: ~combed
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let graph = compile_program(statements, SAMPLE_RATE);

    assert!(
        graph.is_ok(),
        "Pattern-modulated comb should compile: {:?}",
        graph.err()
    );
}

/// LEVEL 3: Comb on Continuous Tone
/// Tests comb used to add resonance to ongoing signal
#[test]
fn test_comb_on_tone() {
    let dsl = r#"
tempo: 1.0
~tone: saw 55
~resonant: ~tone # comb 440 0.8
out: ~resonant * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // Should produce audible resonant tone
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    assert!(
        rms.sqrt() > 0.1,
        "Resonant tone should be audible, got RMS {}",
        rms.sqrt()
    );
}

/// LEVEL 3: Multiple Combs (Physical Modeling)
/// Tests cascading comb filters for complex resonance
#[test]
fn test_multiple_combs() {
    let dsl = r#"
tempo: 1.0
~strike: impulse 0.5
~body1: ~strike # comb 220 0.95
~body2: ~body1 # comb 330 0.93
~body3: ~body2 # comb 440 0.91
out: ~body3 * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE * 2.0) as usize);

    // Should create complex resonant decay
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    assert!(
        rms.sqrt() > 0.01,
        "Multiple combs should create audible resonance, got RMS {}",
        rms.sqrt()
    );

    // Check stability
    let has_nan = samples.iter().any(|s| s.is_nan() || s.is_infinite());
    assert!(!has_nan, "Multiple combs should be stable");
}
