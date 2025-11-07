use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

fn render_dsl(code: &str, duration_sec: f32) -> Vec<f32> {
    let num_samples = (duration_sec * SAMPLE_RATE) as usize;
    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert!(rest.trim().is_empty(), "Failed to parse entire program");
    let mut graph = compile_program(statements, SAMPLE_RATE).expect("Failed to compile");
    graph.render(num_samples)
}

fn calculate_rms(samples: &[f32]) -> f32 {
    let sum: f32 = samples.iter().map(|s| s * s).sum();
    (sum / samples.len() as f32).sqrt()
}

/// LEVEL 1: Pattern Query Verification
#[test]
fn test_wavetable_pattern_query() {
    let dsl = r#"
tempo: 1.0
~synth: wavetable 220
out: ~synth
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
        "Wavetable should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: Wavetable Produces Sound
#[test]
fn test_wavetable_produces_sound() {
    let dsl = r#"
tempo: 1.0
~synth: wavetable 220
out: ~synth * 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE * 1.0) as usize);

    let rms = calculate_rms(&samples);
    assert!(
        rms > 0.2,
        "Wavetable should produce audible output, got RMS {}",
        rms
    );
}

/// LEVEL 2: Different Frequencies
#[test]
fn test_wavetable_frequencies() {
    let dsl_low = r#"
tempo: 1.0
~synth: wavetable 110
out: ~synth * 0.5
"#;

    let dsl_high = r#"
tempo: 1.0
~synth: wavetable 440
out: ~synth * 0.5
"#;

    let (_, statements_low) = parse_program(dsl_low).unwrap();
    let mut graph_low = compile_program(statements_low, SAMPLE_RATE).unwrap();
    let samples_low = graph_low.render((SAMPLE_RATE * 0.5) as usize);

    let (_, statements_high) = parse_program(dsl_high).unwrap();
    let mut graph_high = compile_program(statements_high, SAMPLE_RATE).unwrap();
    let samples_high = graph_high.render((SAMPLE_RATE * 0.5) as usize);

    // Both should be audible
    let rms_low = calculate_rms(&samples_low);
    let rms_high = calculate_rms(&samples_high);

    assert!(rms_low > 0.2, "Low frequency should be audible");
    assert!(rms_high > 0.2, "High frequency should be audible");
}

/// LEVEL 2: Wavetable Stability
#[test]
fn test_wavetable_stability() {
    let dsl = r#"
tempo: 1.0
~synth: wavetable 440
out: ~synth * 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE * 2.0) as usize);

    // Check for NaN or Inf
    let has_nan = samples.iter().any(|s| s.is_nan() || s.is_infinite());
    assert!(!has_nan, "Wavetable should not produce NaN or Inf");

    // Check for reasonable output
    let max_val = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        max_val < 10.0,
        "Wavetable output should be reasonable, got max {}",
        max_val
    );
}

/// LEVEL 2: Pattern-Controlled Frequency
#[test]
fn test_wavetable_pattern_frequency() {
    let dsl = r#"
tempo: 2.0
~synth: wavetable "110 220 440"
out: ~synth * 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE * 1.0) as usize);

    // Should produce varying pitch
    let rms = calculate_rms(&samples);
    assert!(
        rms > 0.2,
        "Pattern-controlled wavetable should be audible, got RMS {}",
        rms
    );
}

/// LEVEL 3: Musical Example - Bass
#[test]
fn test_wavetable_bass() {
    let dsl = r#"
tempo: 2.0
~bass: wavetable 55
out: ~bass * 0.4
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE * 1.0) as usize);

    // Should produce bass sound
    let rms = calculate_rms(&samples);
    assert!(
        rms > 0.15,
        "Wavetable bass should be audible, got RMS {}",
        rms
    );
}

/// LEVEL 3: Musical Example - Lead
#[test]
fn test_wavetable_lead() {
    let dsl = r#"
tempo: 1.5
~lead: wavetable 440
out: ~lead * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE * 1.0) as usize);

    // Should produce lead sound
    let rms = calculate_rms(&samples);
    assert!(
        rms > 0.1,
        "Wavetable lead should be audible, got RMS {}",
        rms
    );
}

/// LEVEL 3: Musical Example - Filtered Pad
#[test]
fn test_wavetable_pad() {
    let dsl = r#"
tempo: 1.0
~pad: wavetable 220 # lpf 1000 0.5
out: ~pad * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE * 2.0) as usize);

    // Should produce pad sound
    let rms = calculate_rms(&samples);
    assert!(
        rms > 0.1,
        "Filtered wavetable pad should be audible, got RMS {}",
        rms
    );
}
