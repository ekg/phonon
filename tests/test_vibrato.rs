use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

fn render_dsl(code: &str, duration_sec: f32) -> Vec<f32> {
    let num_samples = (duration_sec * SAMPLE_RATE) as usize;
    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert!(rest.trim().is_empty(), "Failed to parse entire program");
    let mut graph = compile_program(statements, SAMPLE_RATE, None).expect("Failed to compile");
    graph.render(num_samples)
}

fn calculate_rms(samples: &[f32]) -> f32 {
    let sum: f32 = samples.iter().map(|s| s * s).sum();
    (sum / samples.len() as f32).sqrt()
}

/// LEVEL 1: Pattern Query Verification
#[test]
fn test_vibrato_pattern_query() {
    let dsl = r#"
tempo: 1.0
~carrier $ sine 440
~vibrato $ ~carrier # vibrato 5.0 0.5
out $ ~vibrato
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
        "Vibrato should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: Vibrato Modulates Pitch
#[test]
fn test_vibrato_modulates_pitch() {
    let dsl = r#"
tempo: 1.0
~carrier $ sine 440
~vibrato $ ~carrier # vibrato 5.0 1.0
out $ ~vibrato
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE * 1.0) as usize);

    // Vibrato should produce audible pitch modulation
    let rms = calculate_rms(&samples);
    assert!(
        rms > 0.3,
        "Vibrato should produce audible output, got RMS {}",
        rms
    );
}

/// LEVEL 2: Zero Depth Bypasses Effect
#[test]
fn test_vibrato_zero_depth() {
    let dsl_no_vibrato = r#"
tempo: 1.0
~carrier $ sine 440
out $ ~carrier
"#;

    let dsl_zero_vibrato = r#"
tempo: 1.0
~carrier $ sine 440
~vibrato $ ~carrier # vibrato 5.0 0.0
out $ ~vibrato
"#;

    let (_, statements1) = parse_program(dsl_no_vibrato).unwrap();
    let mut graph1 = compile_program(statements1, SAMPLE_RATE, None).unwrap();
    let samples1 = graph1.render((SAMPLE_RATE * 0.5) as usize);

    let (_, statements2) = parse_program(dsl_zero_vibrato).unwrap();
    let mut graph2 = compile_program(statements2, SAMPLE_RATE, None).unwrap();
    let samples2 = graph2.render((SAMPLE_RATE * 0.5) as usize);

    let rms1 = calculate_rms(&samples1);
    let rms2 = calculate_rms(&samples2);

    // Zero depth should be nearly identical to no effect
    assert!(
        (rms1 - rms2).abs() / rms1 < 0.1,
        "Zero depth vibrato should be similar to no effect, got RMS1={}, RMS2={}",
        rms1,
        rms2
    );
}

/// LEVEL 2: Vibrato Rate Affects Speed
#[test]
fn test_vibrato_rate() {
    let dsl = r#"
tempo: 1.0
~carrier $ sine 440
~vibrato $ ~carrier # vibrato 6.0 0.5
out $ ~vibrato
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE * 1.0) as usize);

    // Should produce audible vibrato
    let rms = calculate_rms(&samples);
    assert!(rms > 0.3, "Vibrato should be audible");
}

/// LEVEL 2: Vibrato Depth Affects Amount
#[test]
fn test_vibrato_depth() {
    let dsl_shallow = r#"
tempo: 1.0
~carrier $ sine 440
~vibrato $ ~carrier # vibrato 5.0 0.2
out $ ~vibrato
"#;

    let dsl_deep = r#"
tempo: 1.0
~carrier $ sine 440
~vibrato $ ~carrier # vibrato 5.0 1.0
out $ ~vibrato
"#;

    let (_, statements_shallow) = parse_program(dsl_shallow).unwrap();
    let mut graph_shallow = compile_program(statements_shallow, SAMPLE_RATE, None).unwrap();
    let samples_shallow = graph_shallow.render((SAMPLE_RATE * 0.5) as usize);

    let (_, statements_deep) = parse_program(dsl_deep).unwrap();
    let mut graph_deep = compile_program(statements_deep, SAMPLE_RATE, None).unwrap();
    let samples_deep = graph_deep.render((SAMPLE_RATE * 0.5) as usize);

    // Both should produce audio
    let rms_shallow = calculate_rms(&samples_shallow);
    let rms_deep = calculate_rms(&samples_deep);

    assert!(rms_shallow > 0.3, "Shallow vibrato should be audible");
    assert!(rms_deep > 0.3, "Deep vibrato should be audible");
}

/// LEVEL 2: Vibrato Stability
#[test]
fn test_vibrato_stability() {
    let dsl = r#"
tempo: 1.0
~carrier $ saw 220
~vibrato $ ~carrier # vibrato 7.0 0.8
out $ ~vibrato
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE * 1.0) as usize);

    // Check for NaN or Inf
    let has_nan = samples.iter().any(|s| s.is_nan() || s.is_infinite());
    assert!(!has_nan, "Vibrato should not produce NaN or Inf");

    // Check for reasonable output
    let max_val = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        max_val < 10.0,
        "Vibrato output should be reasonable, got max {}",
        max_val
    );
}

/// LEVEL 2: Pattern Modulation
#[test]
fn test_vibrato_pattern_modulation() {
    let dsl = r#"
tempo: 0.5
~rate_lfo $ sine 0.3 * 2.0 + 5.0
~depth_lfo $ sine 0.2 * 0.3 + 0.5
~carrier $ sine 440
~vibrato $ ~carrier # vibrato ~rate_lfo ~depth_lfo
out $ ~vibrato
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE * 1.0) as usize);

    // Should produce varying vibrato effect
    let rms = calculate_rms(&samples);
    assert!(
        rms > 0.2,
        "Pattern-modulated vibrato should be audible, got RMS {}",
        rms
    );
}

/// LEVEL 3: Musical Example - Classic Vibrato
#[test]
fn test_vibrato_classic() {
    let dsl = r#"
tempo: 0.5
~voice $ sine 330
~vibrato $ ~voice # vibrato 5.5 0.4
out $ ~vibrato * 0.4
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE * 1.0) as usize);

    // Should produce classic vocal-style vibrato
    let rms = calculate_rms(&samples);
    assert!(
        rms > 0.1,
        "Classic vibrato should be audible, got RMS {}",
        rms
    );
}

/// LEVEL 3: Musical Example - Slow Vibrato
#[test]
fn test_vibrato_slow() {
    let dsl = r#"
tempo: 1.0
~pad $ sine 220
~vibrato $ ~pad # vibrato 2.0 0.6
out $ ~vibrato * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE * 2.0) as usize);

    // Should produce slow, expressive vibrato
    let rms = calculate_rms(&samples);
    assert!(rms > 0.1, "Slow vibrato should be audible, got RMS {}", rms);
}
