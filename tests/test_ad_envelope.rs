use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use std::process::Command;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Pattern Query Verification
/// Tests that AD syntax is parsed and compiled correctly
#[test]
fn test_ad_pattern_query() {
    let dsl = r#"
tempo: 1.0
~env: ad 0.1 0.3
out: ~env * sine 440
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
        "AD should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: Audio Characteristic Verification
/// Tests that AD produces correct envelope shape:
/// - Attack: rises from 0 to 1 over attack_time
/// - Decay: falls from 1 to 0 over decay_time
#[test]
fn test_ad_envelope_shape() {
    let dsl = r#"
tempo: 1.0
-- AD: attack=0.2s, decay=0.4s
~env: ad 0.2 0.4
out: ~env
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    // Render 1 full cycle (1 second at tempo 1.0)
    let samples = graph.render(SAMPLE_RATE as usize);

    // Attack phase: first 0.2s (8820 samples) should rise from ~0 to ~1
    let attack_end = (0.2 * SAMPLE_RATE) as usize;
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

    // Decay phase: from 0.2s to 0.6s (8820 to 26460 samples) should fall from ~1 to ~0
    let decay_end = (0.6 * SAMPLE_RATE) as usize;
    assert!(
        samples[decay_end - 100].abs() < 0.2,
        "After decay, envelope should be near 0, got {}",
        samples[decay_end - 100]
    );

    // After decay: should be silent
    let after_decay = (0.7 * SAMPLE_RATE) as usize;
    assert!(
        samples[after_decay].abs() < 0.1,
        "After decay completes, envelope should be silent, got {}",
        samples[after_decay]
    );
}

/// LEVEL 3: Musical Integration Test
/// Tests that AD shapes a percussive tone correctly
#[test]
fn test_ad_musical_example() {
    let dsl = r#"
tempo: 2.0
-- Percussive envelope: quick attack, longer decay
~env: ad 0.01 0.3
~tone: sine 440
out: ~tone * ~env * 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    // Render 2 cycles (1 second at tempo 2.0)
    let samples = graph.render(SAMPLE_RATE as usize);

    // Write to temporary file
    let filename = "/tmp/test_ad_musical.wav";
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
    println!("AD Musical Analysis:\n{}", analysis);

    // Audio should be audible (not silent)
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    let rms = rms.sqrt();
    assert!(
        rms > 0.02,
        "AD-shaped tone should be audible (RMS > 0.02), got RMS {}",
        rms
    );

    // Peak should be reasonable (envelope scales down the tone)
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        peak > 0.2 && peak < 0.8,
        "AD-shaped tone should have reasonable peak (0.2-0.8), got {}",
        peak
    );
}

/// Test that AD can modulate different signals
#[test]
fn test_ad_modulation() {
    let dsl = r#"
tempo: 2.0
~env: ad 0.02 0.2
~saw: saw 220
out: ~saw * ~env * 0.4
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // Should produce audible output
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    let rms = rms.sqrt();
    assert!(
        rms > 0.01,
        "AD-modulated saw should be audible, got RMS {}",
        rms
    );
}

/// Test that AD parameters can be pattern-controlled
#[test]
fn test_ad_pattern_parameters() {
    let dsl = r#"
tempo: 2.0
-- Pattern-controlled decay time
~decay_pattern: "0.1 0.3"
~env: ad 0.01 ~decay_pattern
out: ~env * square 330
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let graph = compile_program(statements, SAMPLE_RATE);

    // This should parse and compile (pattern modulation is a Phonon strength)
    assert!(
        graph.is_ok(),
        "AD with pattern-controlled parameters should compile: {:?}",
        graph.err()
    );
}

/// Test AD with very short envelope (percussive hit)
#[test]
fn test_ad_percussive() {
    let dsl = r#"
tempo: 4.0
~env: ad 0.005 0.05
~kick: sine 55
out: ~kick * ~env
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE * 0.25) as usize); // 1 cycle at tempo 4.0

    // Should have a sharp attack and quick decay
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        peak > 0.5,
        "Percussive AD should have clear peak, got {}",
        peak
    );

    // Most of the envelope should be near zero (quick decay)
    let near_zero_count = samples.iter().filter(|s| s.abs() < 0.1).count();
    let total_samples = samples.len();
    assert!(
        near_zero_count as f32 / total_samples as f32 > 0.7,
        "Percussive AD should be mostly silent after attack/decay"
    );
}
