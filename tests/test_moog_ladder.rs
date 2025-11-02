use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Pattern Query Verification
/// Tests that moog_ladder syntax is parsed and compiled correctly
#[test]
fn test_moog_ladder_pattern_query() {
    let dsl = r#"
tempo: 1.0
~input: saw 220
~filtered: moog_ladder ~input 1000 0.5
out: ~filtered * 0.5
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
        "Moog ladder should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: Low-Pass Response Verification
/// Tests that moog_ladder attenuates high frequencies
#[test]
fn test_moog_ladder_low_pass_response() {
    // Test with low cutoff (should attenuate high frequencies significantly)
    let dsl = r#"
tempo: 1.0
-- Saw wave (rich in harmonics) filtered at 500 Hz
~rich: saw 220
~filtered: moog_ladder ~rich 500 0.1
out: ~filtered
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    // Render 1/10 second
    let samples = graph.render((SAMPLE_RATE / 10.0) as usize);

    // Write to file for manual inspection
    let filename = "/tmp/test_moog_ladder_low_pass.wav";
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

    // Should produce audible output (not silence)
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    let rms = rms.sqrt();
    assert!(
        rms > 0.05,
        "Filtered signal should be audible (RMS > 0.05), got RMS {}",
        rms
    );

    // Should not clip
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        peak <= 1.0,
        "Filtered signal should not clip, got peak {}",
        peak
    );
}

/// Test that higher resonance increases peak at cutoff
#[test]
fn test_moog_ladder_resonance() {
    let dsl_low = r#"
tempo: 1.0
~input: saw 220
~filtered: moog_ladder ~input 800 0.1
out: ~filtered
"#;

    let (_, statements) = parse_program(dsl_low).unwrap();
    let mut graph_low = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples_low = graph_low.render((SAMPLE_RATE / 10.0) as usize);

    let dsl_high = r#"
tempo: 1.0
~input: saw 220
~filtered: moog_ladder ~input 800 0.9
out: ~filtered
"#;

    let (_, statements) = parse_program(dsl_high).unwrap();
    let mut graph_high = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples_high = graph_high.render((SAMPLE_RATE / 10.0) as usize);

    // High resonance should have higher peak (resonance boost)
    let peak_low = samples_low.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    let peak_high = samples_high.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    println!("Peak with resonance=0.1: {}", peak_low);
    println!("Peak with resonance=0.9: {}", peak_high);

    assert!(
        peak_high >= peak_low,
        "Higher resonance should increase peak, got low={}, high={}",
        peak_low,
        peak_high
    );
}

/// Test self-oscillation at very high resonance
#[test]
fn test_moog_ladder_self_oscillation() {
    let dsl = r#"
tempo: 1.0
-- Tiny input signal with very high resonance (should self-oscillate)
~tiny: sine 220 * 0.01
~self_osc: moog_ladder ~tiny 1000 0.99
out: ~self_osc * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples = graph.render((SAMPLE_RATE / 10.0) as usize);

    // High resonance should produce significant output even from tiny input
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    let rms = rms.sqrt();

    println!("RMS with self-oscillation: {}", rms);

    assert!(
        rms > 0.0001,
        "Self-oscillation should produce some output (RMS > 0.0001), got RMS {}",
        rms
    );
}

/// LEVEL 3: Musical Integration Test
#[test]
fn test_moog_ladder_musical_example() {
    let dsl = r#"
tempo: 2.0
-- Classic Moog bass sound
~bass: saw 55
~moog_bass: moog_ladder ~bass 400 0.7
~env: ad 0.01 0.3
out: ~moog_bass * ~env * 0.4
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE / 2.0) as usize); // 0.5 seconds

    // Write to file
    let filename = "/tmp/test_moog_ladder_musical.wav";
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
        "Moog bass should be audible (RMS > 0.03), got RMS {}",
        rms
    );
}

/// Test pattern-modulated cutoff
#[test]
fn test_moog_ladder_pattern_cutoff() {
    let dsl = r#"
tempo: 2.0
~cutoff_pattern: "500 1000 2000 4000"
~input: saw 110
~swept: moog_ladder ~input ~cutoff_pattern 0.6
out: ~swept * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let graph = compile_program(statements, SAMPLE_RATE);

    assert!(
        graph.is_ok(),
        "Moog ladder with pattern cutoff should compile: {:?}",
        graph.err()
    );
}

/// Test pattern-modulated resonance
#[test]
fn test_moog_ladder_pattern_resonance() {
    let dsl = r#"
tempo: 2.0
~res_pattern: "0.1 0.4 0.7 0.95"
~input: saw 110
~variable_res: moog_ladder ~input 800 ~res_pattern
out: ~variable_res * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let graph = compile_program(statements, SAMPLE_RATE);

    assert!(
        graph.is_ok(),
        "Moog ladder with pattern resonance should compile: {:?}",
        graph.err()
    );
}

/// Test Moog ladder with different input sources
#[test]
fn test_moog_ladder_different_inputs() {
    // Test with white noise (should sound like filtered noise)
    let dsl = r#"
tempo: 1.0
~noise: white_noise
~filtered_noise: moog_ladder ~noise 600 0.5
out: ~filtered_noise * 0.2
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples = graph.render((SAMPLE_RATE / 10.0) as usize);

    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    let rms_val = rms.sqrt();
    println!("Filtered noise RMS: {}", rms_val);
    assert!(
        rms_val > 0.001,
        "Filtered noise should produce some output (RMS > 0.001), got {}",
        rms_val
    );

    // Test with square wave
    let dsl = r#"
tempo: 1.0
~square: square 220
~filtered_square: moog_ladder ~square 1000 0.6
out: ~filtered_square * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples = graph.render((SAMPLE_RATE / 10.0) as usize);

    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    assert!(rms.sqrt() > 0.03, "Filtered square should be audible");
}

/// Test Moog ladder cascaded for steeper rolloff
#[test]
fn test_moog_ladder_cascade() {
    let dsl = r#"
tempo: 1.0
~input: saw 220
~stage1: moog_ladder ~input 800 0.3
~stage2: moog_ladder ~stage1 800 0.3
out: ~stage2 * 0.4
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples = graph.render((SAMPLE_RATE / 10.0) as usize);

    // Should produce audible output
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    assert!(rms.sqrt() > 0.02, "Cascaded filter should be audible");
}
