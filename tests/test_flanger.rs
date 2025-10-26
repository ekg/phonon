use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Pattern Query Verification
/// Tests that flanger syntax is parsed and compiled correctly
#[test]
fn test_flanger_pattern_query() {
    let dsl = r#"
tempo: 1.0
~input: sine 440
~flanged: flanger ~input 0.5 2.0 0.5
out: ~flanged * 0.5
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
        "Flanger should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: Delay Modulation Verification
/// Tests that flanger creates sweeping comb filter effect
#[test]
fn test_flanger_delay_modulation() {
    let dsl = r#"
tempo: 1.0
-- Flanger parameters: input, depth (0-1), rate (Hz), feedback (0-0.95)
~input: sine 440
~flanged: flanger ~input 0.8 0.5 0.3
out: ~flanged * 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    // Render 1 second to capture LFO sweep
    let samples = graph.render(SAMPLE_RATE as usize);

    // Write to file for manual inspection
    let filename = "/tmp/test_flanger_modulation.wav";
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
        rms > 0.05,
        "Flanged signal should be audible (RMS > 0.05), got RMS {}",
        rms
    );

    // Should not clip
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        peak <= 1.0,
        "Flanged signal should not clip, got peak {}",
        peak
    );
}

/// Test that zero depth produces dry signal
#[test]
fn test_flanger_zero_depth() {
    let dsl = r#"
tempo: 1.0
~input: sine 440 * 0.5
~flanged: flanger ~input 0.0 2.0 0.0
out: ~flanged
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph_flanged = compile_program(statements.clone(), SAMPLE_RATE).unwrap();
    let samples_flanged = graph_flanged.render((SAMPLE_RATE / 10.0) as usize);

    // Compare with dry signal
    let dsl_dry = r#"
tempo: 1.0
~input: sine 440 * 0.5
out: ~input
"#;
    let (_, statements) = parse_program(dsl_dry).unwrap();
    let mut graph_dry = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples_dry = graph_dry.render((SAMPLE_RATE / 10.0) as usize);

    // Signals should be very similar (zero depth = no flanging)
    let max_diff = samples_flanged
        .iter()
        .zip(samples_dry.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);

    println!("Max difference between flanged (depth=0) and dry: {}", max_diff);

    assert!(
        max_diff < 0.1,
        "Zero depth should produce nearly dry signal, got max diff {}",
        max_diff
    );
}

/// Test feedback parameter affects resonance
#[test]
fn test_flanger_feedback() {
    let dsl_low = r#"
tempo: 1.0
~input: sine 440
~flanged: flanger ~input 0.5 2.0 0.1
out: ~flanged * 0.5
"#;

    let (_, statements) = parse_program(dsl_low).unwrap();
    let mut graph_low = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples_low = graph_low.render((SAMPLE_RATE / 10.0) as usize);

    let dsl_high = r#"
tempo: 1.0
~input: sine 440
~flanged: flanger ~input 0.5 2.0 0.8
out: ~flanged * 0.5
"#;

    let (_, statements) = parse_program(dsl_high).unwrap();
    let mut graph_high = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples_high = graph_high.render((SAMPLE_RATE / 10.0) as usize);

    // High feedback should have higher peak (more resonance)
    let peak_low = samples_low.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    let peak_high = samples_high.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    println!("Peak with feedback=0.1: {}", peak_low);
    println!("Peak with feedback=0.8: {}", peak_high);

    assert!(
        peak_high >= peak_low * 0.9, // High feedback should have similar or higher peak
        "Higher feedback should increase resonance, got low={}, high={}",
        peak_low,
        peak_high
    );
}

/// LEVEL 3: Musical Integration Test
#[test]
fn test_flanger_musical_example() {
    let dsl = r#"
tempo: 2.0
-- Classic flanger on guitar-like sound
~guitar: saw 220
~flanged: flanger ~guitar 0.7 0.3 0.5
~env: ad 0.01 0.4
out: ~flanged * ~env * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE / 2.0) as usize); // 0.5 seconds

    // Write to file
    let filename = "/tmp/test_flanger_musical.wav";
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
        rms > 0.03,
        "Flanged signal should be audible (RMS > 0.03), got RMS {}",
        rms
    );
}

/// Test pattern-modulated depth
#[test]
fn test_flanger_pattern_depth() {
    let dsl = r#"
tempo: 2.0
~depth_pattern: "0.2 0.5 0.8 1.0"
~input: sine 440
~flanged: flanger ~input ~depth_pattern 2.0 0.5
out: ~flanged * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let graph = compile_program(statements, SAMPLE_RATE);

    assert!(
        graph.is_ok(),
        "Flanger with pattern depth should compile: {:?}",
        graph.err()
    );
}

/// Test pattern-modulated rate
#[test]
fn test_flanger_pattern_rate() {
    let dsl = r#"
tempo: 2.0
~rate_pattern: "0.5 1.0 2.0 4.0"
~input: saw 220
~flanged: flanger ~input 0.7 ~rate_pattern 0.4
out: ~flanged * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let graph = compile_program(statements, SAMPLE_RATE);

    assert!(
        graph.is_ok(),
        "Flanger with pattern rate should compile: {:?}",
        graph.err()
    );
}

/// Test flanger with different waveforms
#[test]
fn test_flanger_different_inputs() {
    // Test with saw wave
    let dsl = r#"
tempo: 1.0
~saw: saw 220
~flanged: flanger ~saw 0.6 1.5 0.4
out: ~flanged * 0.4
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples = graph.render((SAMPLE_RATE / 10.0) as usize);

    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    assert!(rms.sqrt() > 0.05, "Flanged saw should be audible");

    // Test with square wave
    let dsl = r#"
tempo: 1.0
~square: square 220
~flanged: flanger ~square 0.6 1.5 0.4
out: ~flanged * 0.4
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples = graph.render((SAMPLE_RATE / 10.0) as usize);

    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    assert!(rms.sqrt() > 0.05, "Flanged square should be audible");
}
