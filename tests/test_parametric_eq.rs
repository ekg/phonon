use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Pattern Query Verification
/// Tests that parametric_eq syntax is parsed and compiled correctly
#[test]
fn test_parametric_eq_pattern_query() {
    let dsl = r#"
tempo: 1.0
~input $ saw 220
~eq $ parametric_eq ~input 200 3.0 1.0 1000 2.0 1.0 4000 -2.0 1.0
out $ ~eq * 0.5
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
        "Parametric EQ should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: Frequency Response Verification
/// Tests that EQ affects frequency content
#[test]
fn test_parametric_eq_frequency_response() {
    let dsl = r#"
tempo: 1.0
-- Boost mid frequencies, cut highs
~rich $ saw 220
~eq $ parametric_eq ~rich 100 0.0 1.0 1000 6.0 1.0 4000 -6.0 1.0
out $ ~eq
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Render 1/10 second
    let samples = graph.render((SAMPLE_RATE / 10.0) as usize);

    // Write to file for manual inspection
    let filename = "/tmp/test_parametric_eq_response.wav";
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
        "EQ'd signal should be audible (RMS > 0.05), got RMS {}",
        rms
    );

    // Peak may be higher than 1.0 due to boost (that's expected!)
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    println!("Peak level: {}", peak);
    assert!(
        peak > 0.1 && peak < 3.0,
        "EQ'd signal should have reasonable peak (0.1-3.0), got peak {}",
        peak
    );
}

/// Test that positive gain boosts signal level
#[test]
fn test_parametric_eq_boost() {
    // With boost
    let dsl_boost = r#"
tempo: 1.0
~input $ sine 1000
~eq $ parametric_eq ~input 100 0.0 1.0 1000 12.0 1.0 4000 0.0 1.0
out $ ~eq * 0.3
"#;

    let (_, statements) = parse_program(dsl_boost).unwrap();
    let mut graph_boost = compile_program(statements, SAMPLE_RATE, None).unwrap();
    let samples_boost = graph_boost.render((SAMPLE_RATE / 10.0) as usize);

    // Without boost
    let dsl_flat = r#"
tempo: 1.0
~input $ sine 1000
~eq $ parametric_eq ~input 100 0.0 1.0 1000 0.0 1.0 4000 0.0 1.0
out $ ~eq * 0.3
"#;

    let (_, statements) = parse_program(dsl_flat).unwrap();
    let mut graph_flat = compile_program(statements, SAMPLE_RATE, None).unwrap();
    let samples_flat = graph_flat.render((SAMPLE_RATE / 10.0) as usize);

    // Boosted should have higher RMS
    let rms_boost: f32 =
        samples_boost.iter().map(|s| s * s).sum::<f32>() / samples_boost.len() as f32;
    let rms_flat: f32 = samples_flat.iter().map(|s| s * s).sum::<f32>() / samples_flat.len() as f32;

    println!("RMS with +12dB boost: {}", rms_boost.sqrt());
    println!("RMS with flat EQ: {}", rms_flat.sqrt());

    assert!(
        rms_boost > rms_flat * 1.5,
        "Boost should increase RMS, got boost={}, flat={}",
        rms_boost.sqrt(),
        rms_flat.sqrt()
    );
}

/// Test that negative gain cuts signal level
#[test]
fn test_parametric_eq_cut() {
    // With cut
    let dsl_cut = r#"
tempo: 1.0
~input $ sine 1000
~eq $ parametric_eq ~input 100 0.0 1.0 1000 -12.0 1.0 4000 0.0 1.0
out $ ~eq
"#;

    let (_, statements) = parse_program(dsl_cut).unwrap();
    let mut graph_cut = compile_program(statements, SAMPLE_RATE, None).unwrap();
    let samples_cut = graph_cut.render((SAMPLE_RATE / 10.0) as usize);

    // Without cut
    let dsl_flat = r#"
tempo: 1.0
~input $ sine 1000
~eq $ parametric_eq ~input 100 0.0 1.0 1000 0.0 1.0 4000 0.0 1.0
out $ ~eq
"#;

    let (_, statements) = parse_program(dsl_flat).unwrap();
    let mut graph_flat = compile_program(statements, SAMPLE_RATE, None).unwrap();
    let samples_flat = graph_flat.render((SAMPLE_RATE / 10.0) as usize);

    // Cut should have lower RMS
    let rms_cut: f32 = samples_cut.iter().map(|s| s * s).sum::<f32>() / samples_cut.len() as f32;
    let rms_flat: f32 = samples_flat.iter().map(|s| s * s).sum::<f32>() / samples_flat.len() as f32;

    println!("RMS with -12dB cut: {}", rms_cut.sqrt());
    println!("RMS with flat EQ: {}", rms_flat.sqrt());

    assert!(
        rms_cut < rms_flat * 0.7,
        "Cut should decrease RMS, got cut={}, flat={}",
        rms_cut.sqrt(),
        rms_flat.sqrt()
    );
}

/// LEVEL 3: Musical Integration Test
#[test]
fn test_parametric_eq_musical_example() {
    let dsl = r#"
tempo: 0.5
-- Classic bass boost, mid scoop, treble boost
~bass $ saw 110
~scooped $ parametric_eq ~bass 80 6.0 0.7 500 -4.0 1.0 3000 3.0 1.0
~env $ ad 0.01 0.3
out $ ~scooped * ~env * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE / 2.0) as usize); // 0.5 seconds

    // Write to file
    let filename = "/tmp/test_parametric_eq_musical.wav";
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
        "EQ'd bass should be audible (RMS > 0.05), got RMS {}",
        rms
    );
}

/// Test pattern-modulated gain
#[test]
fn test_parametric_eq_pattern_gain() {
    let dsl = r#"
tempo: 0.5
~gain_pattern $ "0.0 3.0 6.0 -3.0"
~input $ saw 220
~dynamic_eq $ parametric_eq ~input 100 0.0 1.0 1000 ~gain_pattern 1.0 4000 0.0 1.0
out $ ~dynamic_eq * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let graph = compile_program(statements, SAMPLE_RATE, None);

    assert!(
        graph.is_ok(),
        "Parametric EQ with pattern gain should compile: {:?}",
        graph.err()
    );
}

/// Test with white noise (full spectrum)
#[test]
fn test_parametric_eq_on_noise() {
    let dsl = r#"
tempo: 1.0
~noise $ white_noise
~shaped $ parametric_eq ~noise 200 6.0 1.0 1000 -3.0 1.0 4000 6.0 1.0
out $ ~shaped * 0.15
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();
    let samples = graph.render((SAMPLE_RATE / 10.0) as usize);

    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    assert!(
        rms.sqrt() > 0.03,
        "EQ'd noise should be audible, got RMS {}",
        rms.sqrt()
    );
}

/// Test flat EQ (all gains = 0) passes signal unchanged
#[test]
fn test_parametric_eq_flat() {
    let dsl_eq = r#"
tempo: 1.0
~input $ sine 440 * 0.5
~flat_eq $ parametric_eq ~input 100 0.0 1.0 1000 0.0 1.0 4000 0.0 1.0
out $ ~flat_eq
"#;

    let (_, statements) = parse_program(dsl_eq).unwrap();
    let mut graph_eq = compile_program(statements, SAMPLE_RATE, None).unwrap();
    let samples_eq = graph_eq.render((SAMPLE_RATE / 10.0) as usize);

    // Compare with no EQ
    let dsl_plain = r#"
tempo: 1.0
~input $ sine 440 * 0.5
out $ ~input
"#;

    let (_, statements) = parse_program(dsl_plain).unwrap();
    let mut graph_plain = compile_program(statements, SAMPLE_RATE, None).unwrap();
    let samples_plain = graph_plain.render((SAMPLE_RATE / 10.0) as usize);

    // Signals should be very similar
    let max_diff = samples_eq
        .iter()
        .zip(samples_plain.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);

    println!("Max difference between flat EQ and plain: {}", max_diff);

    assert!(
        max_diff < 0.1,
        "Flat EQ should pass signal nearly unchanged, got max diff {}",
        max_diff
    );
}

/// Test extreme boost doesn't cause instability
#[test]
fn test_parametric_eq_extreme_boost() {
    let dsl = r#"
tempo: 1.0
~input $ sine 1000 * 0.1
~boosted $ parametric_eq ~input 100 0.0 1.0 1000 18.0 1.0 4000 0.0 1.0
out $ ~boosted * 0.05
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();
    let samples = graph.render((SAMPLE_RATE / 10.0) as usize);

    // Should not produce NaN or Inf
    assert!(
        samples.iter().all(|s| s.is_finite()),
        "Extreme boost should not cause instability"
    );

    // Should produce audible output
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    assert!(rms.sqrt() > 0.01, "Extreme boost should be audible");
}
