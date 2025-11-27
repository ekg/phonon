use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use std::process::Command;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Pattern Query Verification
/// Tests that ADSR syntax is parsed and compiled correctly
#[test]
fn test_adsr_pattern_query() {
    let dsl = r#"
tempo: 1.0
~env: adsr 0.01 0.1 0.7 0.2
out: ~env * sine 440
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
        "ADSR should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: Audio Characteristic Verification
/// Tests that ADSR produces correct envelope shape:
/// - Attack: rises from 0 to 1 over attack_time
/// - Decay: falls from 1 to sustain_level over decay_time
/// - Sustain: holds at sustain_level
/// - Release: falls from sustain_level to 0 over release_time
#[test]
fn test_adsr_envelope_shape() {
    let dsl = r#"
tempo: 1.0
-- ADSR: attack=0.1s, decay=0.1s, sustain=0.5, release=0.2s
~env: adsr 0.1 0.1 0.5 0.2
out: ~env
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Render 1 full cycle (1 second at tempo 1.0)
    let samples = graph.render(SAMPLE_RATE as usize);

    // Attack phase: first 0.1s (4410 samples) should rise from ~0 to ~1
    let attack_end = (0.1 * SAMPLE_RATE) as usize;
    assert!(
        samples[0].abs() < 0.1,
        "Envelope should start near 0, got {}",
        samples[0]
    );
    assert!(
        samples[attack_end - 1] > 0.9,
        "After attack, envelope should be near 1, got {}",
        samples[attack_end - 1]
    );

    // Decay phase: next 0.1s (4410 samples) should fall from ~1 to ~0.5
    let decay_end = (0.2 * SAMPLE_RATE) as usize;
    assert!(
        (samples[decay_end - 1] - 0.5).abs() < 0.15,
        "After decay, envelope should be near sustain level (0.5), got {}",
        samples[decay_end - 1]
    );

    // Sustain phase: from 0.2s to 0.8s should hold near 0.5
    let sustain_start = decay_end;
    let sustain_end = (0.8 * SAMPLE_RATE) as usize;
    let sustain_avg: f32 = samples[sustain_start..sustain_end].iter().sum::<f32>()
        / (sustain_end - sustain_start) as f32;
    assert!(
        (sustain_avg - 0.5).abs() < 0.1,
        "During sustain, envelope should hold near 0.5, got avg {}",
        sustain_avg
    );

    // Release phase: last 0.2s should fall from ~0.5 to ~0
    let release_start = (0.8 * SAMPLE_RATE) as usize;
    let release_end = samples.len();
    assert!(
        samples[release_start] > 0.4,
        "Release should start from sustain level, got {}",
        samples[release_start]
    );
    assert!(
        samples[release_end - 100].abs() < 0.2,
        "Release should end near 0, got {}",
        samples[release_end - 100]
    );
}

/// LEVEL 3: Musical Integration Test
/// Tests that ADSR shapes a musical tone correctly
#[test]
fn test_adsr_musical_example() {
    let dsl = r#"
tempo: 0.5
-- Fast attack, slow release envelope
~env: adsr 0.01 0.05 0.6 0.3
~tone: sine 440
out: ~tone * ~env * 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Render 2 cycles (1 second at tempo 2.0)
    let samples = graph.render(SAMPLE_RATE as usize);

    // Write to temporary file
    let filename = "/tmp/test_adsr_musical.wav";
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
    println!("ADSR Musical Analysis:\n{}", analysis);

    // Audio should be audible (not silent)
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    let rms = rms.sqrt();
    assert!(
        rms > 0.05,
        "ADSR-shaped tone should be audible (RMS > 0.05), got RMS {}",
        rms
    );

    // Peak should be reasonable (envelope scales down the tone)
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        peak > 0.2 && peak < 0.8,
        "ADSR-shaped tone should have reasonable peak (0.2-0.8), got {}",
        peak
    );
}

/// Test that ADSR can modulate different signals
#[test]
fn test_adsr_modulation() {
    let dsl = r#"
tempo: 1.0
~env: adsr 0.05 0.1 0.3 0.4
~saw: saw 110
out: ~saw * ~env * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // Should produce audible output
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    let rms = rms.sqrt();
    assert!(
        rms > 0.01,
        "ADSR-modulated saw should be audible, got RMS {}",
        rms
    );
}

/// Test that ADSR parameters can be pattern-controlled
#[test]
fn test_adsr_pattern_parameters() {
    let dsl = r#"
tempo: 1.0
-- Pattern-controlled attack time
~attack_pattern: "0.01 0.1"
~env: adsr ~attack_pattern 0.1 0.5 0.2
out: ~env * sine 220
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let graph = compile_program(statements, SAMPLE_RATE, None);

    // This should parse and compile (pattern modulation is a Phonon strength)
    assert!(
        graph.is_ok(),
        "ADSR with pattern-controlled parameters should compile: {:?}",
        graph.err()
    );
}
