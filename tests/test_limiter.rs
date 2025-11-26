use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Pattern Query Verification
/// Tests that limiter syntax is parsed and compiled correctly
#[test]
fn test_limiter_pattern_query() {
    let dsl = r#"
tempo: 1.0
~hot_signal: sine 440 * 2.0
~limited: limiter ~hot_signal 0.8
out: ~limited * 0.5
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
        "Limiter should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: Brick-Wall Limiting Verification
/// Tests that limiter prevents signals from exceeding threshold
#[test]
fn test_limiter_brick_wall() {
    let dsl = r#"
tempo: 1.0
-- Sine wave that would exceed threshold (amplitude 2.0)
~hot: sine 440 * 2.0
~limited: limiter ~hot 0.5
out: ~limited
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Render 1/10 second
    let samples = graph.render((SAMPLE_RATE / 10.0) as usize);

    // Write to file for manual inspection
    let filename = "/tmp/test_limiter_brick_wall.wav";
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE as u32,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(filename, spec).unwrap();
    for sample in &samples {
        let amplitude = (sample * i16::MAX as f32) as i16;
        writer.write_sample(amplitude).unwrap();
    }
    writer.finalize().unwrap();

    // Find peak (should be clamped at threshold)
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    println!("Peak after limiting: {}", peak);
    println!("Threshold: 0.5");

    // Peak should not exceed threshold
    assert!(
        peak <= 0.5 + 0.001, // Small epsilon for floating point
        "Limiter should prevent peaks exceeding threshold (0.5), got {}",
        peak
    );

    // Peak should be at or very near threshold (the sine wave is hot enough)
    assert!(
        peak >= 0.49,
        "Limiter should reach threshold with hot input, got {}",
        peak
    );
}

/// Test that signals below threshold pass unchanged
#[test]
fn test_limiter_below_threshold() {
    let dsl = r#"
tempo: 1.0
-- Quiet sine wave (amplitude 0.3)
~quiet: sine 440 * 0.3
~limited: limiter ~quiet 0.8
out: ~limited
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph_limited = compile_program(statements.clone(), SAMPLE_RATE, None).unwrap();
    let samples_limited = graph_limited.render((SAMPLE_RATE / 10.0) as usize);

    // Compare with unlimited version
    let dsl_unlimited = r#"
tempo: 1.0
~quiet: sine 440 * 0.3
out: ~quiet
"#;
    let (_, statements) = parse_program(dsl_unlimited).unwrap();
    let mut graph_unlimited = compile_program(statements, SAMPLE_RATE, None).unwrap();
    let samples_unlimited = graph_unlimited.render((SAMPLE_RATE / 10.0) as usize);

    // Samples should be nearly identical (below threshold)
    let max_diff = samples_limited
        .iter()
        .zip(samples_unlimited.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);

    println!("Max difference between limited and unlimited: {}", max_diff);

    assert!(
        max_diff < 0.001,
        "Signals below threshold should pass unchanged, got max diff {}",
        max_diff
    );
}

/// Test limiter handles both positive and negative peaks
#[test]
fn test_limiter_bipolar() {
    let dsl = r#"
tempo: 1.0
~hot: sine 440 * 2.0
~limited: limiter ~hot 0.6
out: ~limited
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();
    let samples = graph.render((SAMPLE_RATE / 10.0) as usize);

    // Find positive and negative peaks
    let pos_peak = samples.iter().cloned().fold(0.0f32, f32::max);
    let neg_peak = samples.iter().cloned().fold(0.0f32, f32::min);

    println!("Positive peak: {}, Negative peak: {}", pos_peak, neg_peak);

    // Both should be limited to ±0.6
    assert!(
        pos_peak <= 0.6 + 0.001,
        "Positive peaks should be limited to 0.6, got {}",
        pos_peak
    );

    assert!(
        neg_peak >= -0.6 - 0.001,
        "Negative peaks should be limited to -0.6, got {}",
        neg_peak
    );
}

/// LEVEL 3: Musical Integration Test
#[test]
fn test_limiter_musical_example() {
    let dsl = r#"
tempo: 2.0
-- Prevent distortion from hot signal
~synth: saw 220 * 1.5
~safe: limiter ~synth 0.7
out: ~safe * 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE / 2.0) as usize); // 0.5 seconds

    // Write to file
    let filename = "/tmp/test_limiter_musical.wav";
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE as u32,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(filename, spec).unwrap();
    for sample in &samples {
        let amplitude = (sample * i16::MAX as f32) as i16;
        writer.write_sample(amplitude).unwrap();
    }
    writer.finalize().unwrap();

    // Should produce audible output
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    let rms = rms.sqrt();
    assert!(
        rms > 0.1,
        "Limited signal should be audible (RMS > 0.1), got RMS {}",
        rms
    );

    // Peak should not exceed threshold
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        peak <= 0.7 + 0.001,
        "Limited signal should not exceed threshold, got peak {}",
        peak
    );
}

/// Test pattern-modulated threshold
#[test]
fn test_limiter_pattern_threshold() {
    let dsl = r#"
tempo: 2.0
~threshold_pattern: "0.3 0.5 0.7 0.9"
~hot: sine 440 * 2.0
~limited: limiter ~hot ~threshold_pattern
out: ~limited * 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let graph = compile_program(statements, SAMPLE_RATE, None);

    assert!(
        graph.is_ok(),
        "Limiter with pattern threshold should compile: {:?}",
        graph.err()
    );
}

/// Test limiter prevents clipping on mix
#[test]
fn test_limiter_prevents_clipping() {
    let dsl = r#"
tempo: 1.0
-- Multiple oscillators that would clip when summed
~osc1: sine 220 * 0.8
~osc2: sine 330 * 0.8
~osc3: sine 440 * 0.8
~mix: ~osc1 + ~osc2 + ~osc3
~safe: limiter ~mix 1.0
out: ~safe
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();
    let samples = graph.render((SAMPLE_RATE / 10.0) as usize);

    // Peak should not exceed 1.0
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    println!("Mix peak after limiting: {}", peak);

    assert!(
        peak <= 1.0 + 0.001,
        "Limiter should prevent clipping, got peak {}",
        peak
    );
}

/// Test limiter with envelope (mastering use case)
#[test]
fn test_limiter_with_envelope() {
    let dsl = r#"
tempo: 2.0
~env: ad 0.01 0.3
~synth: saw 440 * 2.5
~hot: ~synth * ~env
~master: limiter ~hot 0.9
out: ~master * 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();
    let samples = graph.render((SAMPLE_RATE / 2.0) as usize);

    // Should produce audible output
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    assert!(rms.sqrt() > 0.05, "Limited signal should be audible");

    // Peak should not exceed threshold even during loud attack
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        peak <= 0.9 + 0.001,
        "Limiter should work throughout envelope, got peak {}",
        peak
    );
}
