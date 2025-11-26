use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use std::process::Command;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Pattern Query Verification
/// Tests that Line syntax is parsed and compiled correctly
#[test]
fn test_line_pattern_query() {
    let dsl = r#"
tempo: 1.0
~ramp: line 0 1
out: ~ramp * sine 440
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
        "Line should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: Audio Characteristic Verification
/// Tests that Line produces correct linear ramp:
/// - Starts at start value
/// - Ends at end value
/// - Changes linearly
#[test]
fn test_line_ramp_shape() {
    let dsl = r#"
tempo: 1.0
-- Line: from 0 to 1 over one cycle
~ramp: line 0 1
out: ~ramp
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Render 1 full cycle (1 second at tempo 1.0)
    let samples = graph.render(SAMPLE_RATE as usize);

    // Should start near 0
    assert!(
        samples[0].abs() < 0.05,
        "Line should start near 0, got {}",
        samples[0]
    );

    // Should end near 1
    let end_sample = samples.len() - 100;
    assert!(
        (samples[end_sample] - 1.0).abs() < 0.05,
        "Line should end near 1, got {}",
        samples[end_sample]
    );

    // Should be roughly linear (check midpoint)
    let midpoint = samples.len() / 2;
    let mid_value = samples[midpoint];
    assert!(
        (mid_value - 0.5).abs() < 0.1,
        "Line midpoint should be near 0.5, got {}",
        mid_value
    );

    // Should be monotonically increasing
    let mut prev = samples[0];
    let mut monotonic = true;
    for (i, &sample) in samples.iter().enumerate().skip(1) {
        if sample < prev - 0.01 {
            // Allow small numerical error
            monotonic = false;
            println!(
                "Non-monotonic at sample {}: prev={}, current={}",
                i, prev, sample
            );
            break;
        }
        prev = sample;
    }
    assert!(monotonic, "Line should be monotonically increasing");
}

/// Test descending line
#[test]
fn test_line_descending() {
    let dsl = r#"
tempo: 2.0
~ramp: line 1 0
out: ~ramp
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE / 2.0) as usize); // 0.5 second at tempo 2.0

    // Should start near 1
    assert!(
        (samples[0] - 1.0).abs() < 0.05,
        "Line should start near 1, got {}",
        samples[0]
    );

    // Should end near 0
    let end_sample = samples.len() - 100;
    assert!(
        samples[end_sample].abs() < 0.05,
        "Line should end near 0, got {}",
        samples[end_sample]
    );
}

/// LEVEL 3: Musical Integration Test
/// Tests that Line can create fades and sweeps
#[test]
fn test_line_musical_fade() {
    let dsl = r#"
tempo: 1.0
-- Fade in from 0 to 1
~fade: line 0 1
~tone: sine 440
out: ~tone * ~fade * 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // Write to temporary file
    let filename = "/tmp/test_line_fade.wav";
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

    // Analyze with wav_analyze
    let output = Command::new("cargo")
        .args(&["run", "--bin", "wav_analyze", "--", filename])
        .output()
        .expect("Failed to run wav_analyze");

    let analysis = String::from_utf8_lossy(&output.stdout);
    println!("Line Fade Analysis:\n{}", analysis);

    // Audio should be audible (not silent)
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    let rms = rms.sqrt();
    assert!(
        rms > 0.1,
        "Line-faded tone should be audible (RMS > 0.1), got RMS {}",
        rms
    );

    // Peak should be reasonable
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        peak > 0.2 && peak < 0.8,
        "Line-faded tone should have reasonable peak (0.2-0.8), got {}",
        peak
    );

    // Beginning should be quieter than end (fade in)
    let start_rms: f32 = samples[0..4410].iter().map(|s| s * s).sum::<f32>() / 4410.0;
    let end_rms: f32 = samples[39690..44100].iter().map(|s| s * s).sum::<f32>() / 4410.0;
    assert!(
        end_rms > start_rms * 5.0,
        "End should be much louder than start (fade in), start_rms={}, end_rms={}",
        start_rms.sqrt(),
        end_rms.sqrt()
    );
}

/// Test frequency sweep using Line
#[test]
fn test_line_frequency_sweep() {
    let dsl = r#"
tempo: 2.0
-- Sweep from 100Hz to 1000Hz
~freq_ramp: line 100 1000
~sweep: sine ~freq_ramp
out: ~sweep * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE / 2.0) as usize); // 0.5 second

    // Should produce audible output
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    let rms = rms.sqrt();
    assert!(
        rms > 0.1,
        "Frequency sweep should be audible, got RMS {}",
        rms
    );
}

/// Test that Line parameters can be pattern-controlled
#[test]
fn test_line_pattern_parameters() {
    let dsl = r#"
tempo: 2.0
-- Pattern-controlled end value
~end_pattern: "0.5 1.0"
~ramp: line 0 ~end_pattern
out: ~ramp * sine 440
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let graph = compile_program(statements, SAMPLE_RATE, None);

    // This should parse and compile (pattern modulation is a Phonon strength)
    assert!(
        graph.is_ok(),
        "Line with pattern-controlled parameters should compile: {:?}",
        graph.err()
    );
}

/// Test negative values
/// TODO: Fix negative number parsing in DSL
#[test]
#[ignore]
fn test_line_negative_range() {
    let dsl = r#"
tempo: 1.0
~ramp: line -1 1
out: ~ramp * sine 440
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // Should start near -1
    assert!(
        (samples[0] + 1.0).abs() < 0.1,
        "Line should start near -1, got {}",
        samples[0]
    );

    // Should end near 1
    let end_sample = samples.len() - 100;
    assert!(
        (samples[end_sample] - 1.0).abs() < 0.1,
        "Line should end near 1, got {}",
        samples[end_sample]
    );

    // Midpoint should be near 0
    let midpoint = samples.len() / 2;
    assert!(
        samples[midpoint].abs() < 0.15,
        "Line midpoint should be near 0, got {}",
        samples[midpoint]
    );
}
