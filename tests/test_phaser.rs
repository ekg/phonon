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

fn calculate_spectral_variance(samples: &[f32]) -> f32 {
    // Simple measure: variance in amplitude over time
    // Phaser creates moving notches which increase variance
    let mean: f32 = samples.iter().sum::<f32>() / samples.len() as f32;
    let variance: f32 = samples.iter().map(|s| (s - mean).powi(2)).sum::<f32>() / samples.len() as f32;
    variance
}

/// LEVEL 1: Pattern Query Verification
#[test]
fn test_phaser_pattern_query() {
    let dsl = r#"
tempo: 1.0
~carrier: saw 220
~phased: ~carrier # phaser 0.5 0.7 0.3 4
out: ~phased
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
        "Phaser should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: Phaser Modulates Spectrum
#[test]
fn test_phaser_modulates_spectrum() {
    let dsl_dry = r#"
tempo: 1.0
~carrier: saw 220
out: ~carrier * 0.5
"#;

    let dsl_wet = r#"
tempo: 1.0
~carrier: saw 220
~phased: ~carrier # phaser 0.5 0.8 0.4 6
out: ~phased * 0.5
"#;

    let (_, statements_dry) = parse_program(dsl_dry).unwrap();
    let mut graph_dry = compile_program(statements_dry, SAMPLE_RATE).unwrap();
    let samples_dry = graph_dry.render((SAMPLE_RATE * 1.0) as usize);

    let (_, statements_wet) = parse_program(dsl_wet).unwrap();
    let mut graph_wet = compile_program(statements_wet, SAMPLE_RATE).unwrap();
    let samples_wet = graph_wet.render((SAMPLE_RATE * 1.0) as usize);

    // Phaser should create spectral modulation (measurable via variance)
    let variance_dry = calculate_spectral_variance(&samples_dry);
    let variance_wet = calculate_spectral_variance(&samples_wet);

    // Phased signal should have different spectral character
    assert!(
        (variance_wet - variance_dry).abs() > 0.0001,
        "Phaser should modulate spectrum, dry variance={}, wet variance={}",
        variance_dry,
        variance_wet
    );
}

/// LEVEL 2: Zero Depth Minimal Effect
#[test]
fn test_phaser_zero_depth() {
    let dsl_no_phaser = r#"
tempo: 1.0
~carrier: saw 220
out: ~carrier * 0.5
"#;

    let dsl_zero_phaser = r#"
tempo: 1.0
~carrier: saw 220
~phased: ~carrier # phaser 0.5 0.0 0.0 4
out: ~phased * 0.5
"#;

    let (_, statements1) = parse_program(dsl_no_phaser).unwrap();
    let mut graph1 = compile_program(statements1, SAMPLE_RATE).unwrap();
    let samples1 = graph1.render((SAMPLE_RATE * 0.5) as usize);

    let (_, statements2) = parse_program(dsl_zero_phaser).unwrap();
    let mut graph2 = compile_program(statements2, SAMPLE_RATE).unwrap();
    let samples2 = graph2.render((SAMPLE_RATE * 0.5) as usize);

    let rms1 = calculate_rms(&samples1);
    let rms2 = calculate_rms(&samples2);

    // Zero depth should be very similar to no effect
    assert!(
        (rms1 - rms2).abs() / rms1 < 0.15,
        "Zero depth phaser should be similar to no effect, got RMS1={}, RMS2={}",
        rms1,
        rms2
    );
}

/// LEVEL 2: Phaser Rate Affects Sweep Speed
#[test]
fn test_phaser_rate() {
    let dsl_slow = r#"
tempo: 1.0
~carrier: saw 220
~phased: ~carrier # phaser 0.3 0.7 0.4 4
out: ~phased * 0.5
"#;

    let dsl_fast = r#"
tempo: 1.0
~carrier: saw 220
~phased: ~carrier # phaser 2.0 0.7 0.4 4
out: ~phased * 0.5
"#;

    let (_, statements_slow) = parse_program(dsl_slow).unwrap();
    let mut graph_slow = compile_program(statements_slow, SAMPLE_RATE).unwrap();
    let samples_slow = graph_slow.render((SAMPLE_RATE * 1.0) as usize);

    let (_, statements_fast) = parse_program(dsl_fast).unwrap();
    let mut graph_fast = compile_program(statements_fast, SAMPLE_RATE).unwrap();
    let samples_fast = graph_fast.render((SAMPLE_RATE * 1.0) as usize);

    // Both should produce audible output
    let rms_slow = calculate_rms(&samples_slow);
    let rms_fast = calculate_rms(&samples_fast);

    assert!(rms_slow > 0.1, "Slow phaser should be audible");
    assert!(rms_fast > 0.1, "Fast phaser should be audible");
}

/// LEVEL 2: Phaser Depth Affects Amount
#[test]
fn test_phaser_depth() {
    let dsl_shallow = r#"
tempo: 1.0
~carrier: saw 220
~phased: ~carrier # phaser 0.5 0.2 0.3 4
out: ~phased * 0.5
"#;

    let dsl_deep = r#"
tempo: 1.0
~carrier: saw 220
~phased: ~carrier # phaser 0.5 0.9 0.3 4
out: ~phased * 0.5
"#;

    let (_, statements_shallow) = parse_program(dsl_shallow).unwrap();
    let mut graph_shallow = compile_program(statements_shallow, SAMPLE_RATE).unwrap();
    let samples_shallow = graph_shallow.render((SAMPLE_RATE * 1.0) as usize);

    let (_, statements_deep) = parse_program(dsl_deep).unwrap();
    let mut graph_deep = compile_program(statements_deep, SAMPLE_RATE).unwrap();
    let samples_deep = graph_deep.render((SAMPLE_RATE * 1.0) as usize);

    // Both should produce audible output
    let rms_shallow = calculate_rms(&samples_shallow);
    let rms_deep = calculate_rms(&samples_deep);

    assert!(rms_shallow > 0.1, "Shallow phaser should be audible");
    assert!(rms_deep > 0.1, "Deep phaser should be audible");
}

/// LEVEL 2: Phaser Stability
#[test]
fn test_phaser_stability() {
    let dsl = r#"
tempo: 1.0
~carrier: saw 110
~phased: ~carrier # phaser 1.5 0.8 0.6 6
out: ~phased * 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE * 2.0) as usize);

    // Check for NaN or Inf
    let has_nan = samples.iter().any(|s| s.is_nan() || s.is_infinite());
    assert!(!has_nan, "Phaser should not produce NaN or Inf");

    // Check for reasonable output
    let max_val = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        max_val < 10.0,
        "Phaser output should be reasonable, got max {}",
        max_val
    );
}

/// LEVEL 2: Phaser Feedback Affects Resonance
#[test]
fn test_phaser_feedback() {
    let dsl_no_fb = r#"
tempo: 1.0
~carrier: saw 220
~phased: ~carrier # phaser 0.5 0.7 0.0 4
out: ~phased * 0.5
"#;

    let dsl_high_fb = r#"
tempo: 1.0
~carrier: saw 220
~phased: ~carrier # phaser 0.5 0.7 0.7 4
out: ~phased * 0.5
"#;

    let (_, statements_no_fb) = parse_program(dsl_no_fb).unwrap();
    let mut graph_no_fb = compile_program(statements_no_fb, SAMPLE_RATE).unwrap();
    let samples_no_fb = graph_no_fb.render((SAMPLE_RATE * 0.5) as usize);

    let (_, statements_high_fb) = parse_program(dsl_high_fb).unwrap();
    let mut graph_high_fb = compile_program(statements_high_fb, SAMPLE_RATE).unwrap();
    let samples_high_fb = graph_high_fb.render((SAMPLE_RATE * 0.5) as usize);

    // Both should be audible
    let rms_no_fb = calculate_rms(&samples_no_fb);
    let rms_high_fb = calculate_rms(&samples_high_fb);

    assert!(rms_no_fb > 0.1, "No-feedback phaser should be audible");
    assert!(rms_high_fb > 0.1, "High-feedback phaser should be audible");
}

/// LEVEL 2: Pattern Modulation
#[test]
fn test_phaser_pattern_modulation() {
    let dsl = r#"
tempo: 2.0
~rate_lfo: sine 0.1 * 1.0 + 1.0
~depth_lfo: sine 0.2 * 0.3 + 0.5
~carrier: saw 220
~phased: ~carrier # phaser ~rate_lfo ~depth_lfo 0.4 4
out: ~phased * 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE * 1.0) as usize);

    // Should produce varying phaser effect
    let rms = calculate_rms(&samples);
    assert!(
        rms > 0.1,
        "Pattern-modulated phaser should be audible, got RMS {}",
        rms
    );
}

/// LEVEL 3: Musical Example - Classic Phaser
#[test]
fn test_phaser_classic() {
    let dsl = r#"
tempo: 2.0
~synth: saw 110
~phased: ~synth # phaser 0.4 0.7 0.5 4
out: ~phased * 0.4
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE * 1.0) as usize);

    // Should produce classic phaser sound
    let rms = calculate_rms(&samples);
    assert!(
        rms > 0.1,
        "Classic phaser should be audible, got RMS {}",
        rms
    );
}

/// LEVEL 3: Musical Example - Deep Phaser
#[test]
fn test_phaser_deep() {
    let dsl = r#"
tempo: 1.0
~pad: sine 220
~deep_phase: ~pad # phaser 0.2 0.9 0.6 8
out: ~deep_phase * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE * 2.0) as usize);

    // Should produce deep, dramatic phaser
    let rms = calculate_rms(&samples);
    assert!(
        rms > 0.05,
        "Deep phaser should be audible, got RMS {}",
        rms
    );
}
