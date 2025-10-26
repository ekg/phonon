use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Pattern Query Verification
/// Tests that pan2 syntax is parsed and compiled correctly
#[test]
fn test_pan2_pattern_query() {
    let dsl = r#"
tempo: 1.0
~input: sine 440 * 0.5
out1: pan2_l ~input 0.0
out2: pan2_r ~input 0.0
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
        "Pan2 should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: Hard Left Panning (pan = -1)
/// Tests that pan2 with position -1 sends all signal to left, none to right
#[test]
fn test_pan2_hard_left() {
    let dsl = r#"
tempo: 1.0
~input: sine 440 * 0.5
out1: pan2_l ~input -1.0
out2: pan2_r ~input -1.0
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let (left, right) = graph.render_stereo(1000);

    // Left should have full signal
    let rms_left: f32 = left.iter().map(|s| s * s).sum::<f32>() / left.len() as f32;
    assert!(
        rms_left.sqrt() > 0.3,
        "Left channel should have signal, got RMS {}",
        rms_left.sqrt()
    );

    // Right should be silent
    let rms_right: f32 = right.iter().map(|s| s * s).sum::<f32>() / right.len() as f32;
    assert!(
        rms_right.sqrt() < 0.01,
        "Right channel should be silent at hard left, got RMS {}",
        rms_right.sqrt()
    );

    // Peak of left should be approximately 0.5 (input amplitude)
    let peak_left = left.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        (peak_left - 0.5).abs() < 0.1,
        "Left peak should be ~0.5, got {}",
        peak_left
    );
}

/// LEVEL 2: Hard Right Panning (pan = 1)
/// Tests that pan2 with position 1 sends all signal to right, none to left
#[test]
fn test_pan2_hard_right() {
    let dsl = r#"
tempo: 1.0
~input: sine 440 * 0.5
out1: pan2_l ~input 1.0
out2: pan2_r ~input 1.0
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let (left, right) = graph.render_stereo(1000);

    // Left should be silent
    let rms_left: f32 = left.iter().map(|s| s * s).sum::<f32>() / left.len() as f32;
    assert!(
        rms_left.sqrt() < 0.01,
        "Left channel should be silent at hard right, got RMS {}",
        rms_left.sqrt()
    );

    // Right should have full signal
    let rms_right: f32 = right.iter().map(|s| s * s).sum::<f32>() / right.len() as f32;
    assert!(
        rms_right.sqrt() > 0.3,
        "Right channel should have signal, got RMS {}",
        rms_right.sqrt()
    );

    // Peak of right should be approximately 0.5 (input amplitude)
    let peak_right = right.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        (peak_right - 0.5).abs() < 0.1,
        "Right peak should be ~0.5, got {}",
        peak_right
    );
}

/// LEVEL 2: Center Panning (pan = 0)
/// Tests that pan2 with position 0 sends equal power to both channels
#[test]
fn test_pan2_center() {
    let dsl = r#"
tempo: 1.0
~input: sine 440 * 0.5
out1: pan2_l ~input 0.0
out2: pan2_r ~input 0.0
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let (left, right) = graph.render_stereo(1000);

    // Both channels should have signal
    let rms_left: f32 = left.iter().map(|s| s * s).sum::<f32>() / left.len() as f32;
    let rms_right: f32 = right.iter().map(|s| s * s).sum::<f32>() / right.len() as f32;

    println!(
        "Center pan - Left RMS: {}, Right RMS: {}",
        rms_left.sqrt(),
        rms_right.sqrt()
    );

    // Equal power panning: both channels should have ~0.707 of original amplitude
    // Original RMS ≈ 0.5/√2 ≈ 0.35
    // After panning: 0.35 * 0.707 ≈ 0.25
    assert!(
        rms_left.sqrt() > 0.2 && rms_left.sqrt() < 0.3,
        "Left channel should have ~0.25 RMS, got {}",
        rms_left.sqrt()
    );
    assert!(
        rms_right.sqrt() > 0.2 && rms_right.sqrt() < 0.3,
        "Right channel should have ~0.25 RMS, got {}",
        rms_right.sqrt()
    );

    // Channels should be nearly identical (same signal, equal power)
    let max_diff = left
        .iter()
        .zip(right.iter())
        .map(|(l, r)| (l - r).abs())
        .fold(0.0f32, f32::max);

    assert!(
        max_diff < 0.01,
        "Center pan should have identical channels, max diff: {}",
        max_diff
    );
}

/// LEVEL 2: Partial Left Panning (pan = -0.5)
#[test]
fn test_pan2_partial_left() {
    let dsl = r#"
tempo: 1.0
~input: sine 440 * 0.5
out1: pan2_l ~input -0.5
out2: pan2_r ~input -0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let (left, right) = graph.render_stereo(1000);

    let rms_left: f32 = left.iter().map(|s| s * s).sum::<f32>() / left.len() as f32;
    let rms_right: f32 = right.iter().map(|s| s * s).sum::<f32>() / right.len() as f32;

    println!(
        "Partial left - Left RMS: {}, Right RMS: {}",
        rms_left.sqrt(),
        rms_right.sqrt()
    );

    // Left should be louder than right
    assert!(
        rms_left > rms_right,
        "Left should be louder than right, got left={}, right={}",
        rms_left.sqrt(),
        rms_right.sqrt()
    );

    // Both should have some signal
    assert!(rms_left.sqrt() > 0.2, "Left should have signal");
    assert!(rms_right.sqrt() > 0.1, "Right should have some signal");
}

/// LEVEL 3: Musical Integration Test
#[test]
fn test_pan2_musical_example() {
    let dsl = r#"
tempo: 2.0
~bass: saw 55 * 0.3
~bass_left: pan2_l ~bass -0.8
~bass_right: pan2_r ~bass -0.8

~lead: saw 440 * 0.2
~lead_left: pan2_l ~lead 0.8
~lead_right: pan2_r ~lead 0.8

out1: ~bass_left + ~lead_left
out2: ~bass_right + ~lead_right
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let (left, right) = graph.render_stereo((SAMPLE_RATE / 2.0) as usize);

    // Write stereo file
    let filename = "/tmp/test_pan2_musical.wav";
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: SAMPLE_RATE as u32,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(filename, spec).unwrap();
    for (l, r) in left.iter().zip(right.iter()) {
        writer.write_sample((l * i16::MAX as f32) as i16).unwrap();
        writer.write_sample((r * i16::MAX as f32) as i16).unwrap();
    }
    writer.finalize().unwrap();

    // Both channels should be audible
    let rms_left: f32 = left.iter().map(|s| s * s).sum::<f32>() / left.len() as f32;
    let rms_right: f32 = right.iter().map(|s| s * s).sum::<f32>() / right.len() as f32;

    assert!(rms_left.sqrt() > 0.1, "Left should be audible");
    assert!(rms_right.sqrt() > 0.1, "Right should be audible");
}

/// Test pattern-modulated pan position
#[test]
fn test_pan2_pattern_modulation() {
    let dsl = r#"
tempo: 2.0
~pan_pattern: "-1.0 0.0 1.0"
~input: sine 440 * 0.5
out1: pan2_l ~input ~pan_pattern
out2: pan2_r ~input ~pan_pattern
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let graph = compile_program(statements, SAMPLE_RATE);

    assert!(
        graph.is_ok(),
        "Pan2 with pattern modulation should compile: {:?}",
        graph.err()
    );
}

/// Test pan2 with different waveforms
#[test]
fn test_pan2_different_waveforms() {
    let dsl = r#"
tempo: 1.0
~saw: saw 220 * 0.3
~square: square 330 * 0.3
~noise: white_noise * 0.2

~saw_l: pan2_l ~saw -0.7
~saw_r: pan2_r ~saw -0.7

~square_l: pan2_l ~square 0.0
~square_r: pan2_r ~square 0.0

~noise_l: pan2_l ~noise 0.7
~noise_r: pan2_r ~noise 0.7

out1: ~saw_l + ~square_l + ~noise_l
out2: ~saw_r + ~square_r + ~noise_r
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let (left, right) = graph.render_stereo(1000);

    // Both channels should be audible
    let rms_left: f32 = left.iter().map(|s| s * s).sum::<f32>() / left.len() as f32;
    let rms_right: f32 = right.iter().map(|s| s * s).sum::<f32>() / right.len() as f32;

    assert!(rms_left.sqrt() > 0.1, "Left should be audible");
    assert!(rms_right.sqrt() > 0.1, "Right should be audible");
}

/// Test that pan position is clamped to [-1, 1]
#[test]
fn test_pan2_position_clamping() {
    let dsl = r#"
tempo: 1.0
~input: sine 440 * 0.5
out1: pan2_l ~input 5.0
out2: pan2_r ~input 5.0
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let (left, right) = graph.render_stereo(1000);

    // Should behave like pan = 1.0 (hard right)
    let rms_left: f32 = left.iter().map(|s| s * s).sum::<f32>() / left.len() as f32;
    let rms_right: f32 = right.iter().map(|s| s * s).sum::<f32>() / right.len() as f32;

    assert!(
        rms_left.sqrt() < 0.01,
        "Extreme pan should be clamped to hard right (left silent)"
    );
    assert!(
        rms_right.sqrt() > 0.3,
        "Extreme pan should be clamped to hard right (right loud)"
    );
}
