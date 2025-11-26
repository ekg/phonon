use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

/// Test that render_stereo returns separate left and right channels
#[test]
fn test_render_stereo_basic() {
    let dsl = r#"
tempo: 1.0
out1: sine 440 * 0.5
out2: sine 880 * 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Render stereo
    let (left, right) = graph.render_stereo(1000);

    // Both channels should have samples
    assert_eq!(left.len(), 1000);
    assert_eq!(right.len(), 1000);

    // Left should have 440 Hz, right should have 880 Hz
    let rms_left: f32 = left.iter().map(|s| s * s).sum::<f32>() / left.len() as f32;
    let rms_right: f32 = right.iter().map(|s| s * s).sum::<f32>() / right.len() as f32;

    assert!(rms_left.sqrt() > 0.3, "Left channel should have signal");
    assert!(rms_right.sqrt() > 0.3, "Right channel should have signal");

    // Channels should be different (different frequencies)
    let correlation: f32 = left
        .iter()
        .zip(right.iter())
        .map(|(l, r)| l * r)
        .sum::<f32>()
        / left.len() as f32;

    println!("Correlation: {}", correlation);
    // Different frequencies should have low correlation
    assert!(
        correlation.abs() < 0.2,
        "Different frequencies should have low correlation"
    );
}

/// Test stereo rendering with only left channel
#[test]
fn test_render_stereo_left_only() {
    let dsl = r#"
tempo: 1.0
out1: sine 440 * 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let (left, right) = graph.render_stereo(1000);

    // Left should have signal
    let rms_left: f32 = left.iter().map(|s| s * s).sum::<f32>() / left.len() as f32;
    assert!(rms_left.sqrt() > 0.3, "Left channel should have signal");

    // Right should be silent
    let rms_right: f32 = right.iter().map(|s| s * s).sum::<f32>() / right.len() as f32;
    assert!(
        rms_right.sqrt() < 0.01,
        "Right channel should be silent, got {}",
        rms_right.sqrt()
    );
}

/// Test stereo rendering with only right channel
#[test]
fn test_render_stereo_right_only() {
    let dsl = r#"
tempo: 1.0
out2: sine 440 * 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let (left, right) = graph.render_stereo(1000);

    // Left should be silent
    let rms_left: f32 = left.iter().map(|s| s * s).sum::<f32>() / left.len() as f32;
    assert!(
        rms_left.sqrt() < 0.01,
        "Left channel should be silent, got {}",
        rms_left.sqrt()
    );

    // Right should have signal
    let rms_right: f32 = right.iter().map(|s| s * s).sum::<f32>() / right.len() as f32;
    assert!(rms_right.sqrt() > 0.3, "Right channel should have signal");
}

/// Test that mono render still works (backward compatibility)
#[test]
fn test_mono_render_backward_compat() {
    let dsl = r#"
tempo: 1.0
out: sine 440 * 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Mono render should still work
    let mono = graph.render(1000);
    assert_eq!(mono.len(), 1000);

    let rms: f32 = mono.iter().map(|s| s * s).sum::<f32>() / mono.len() as f32;
    assert!(rms.sqrt() > 0.3, "Mono output should work");
}

/// Test stereo with both channels having same signal (mono-compatible)
#[test]
fn test_stereo_mono_compatible() {
    let dsl = r#"
tempo: 1.0
~sig: sine 440 * 0.5
out1: ~sig
out2: ~sig
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let (left, right) = graph.render_stereo(1000);

    // Both channels should be identical
    let max_diff = left
        .iter()
        .zip(right.iter())
        .map(|(l, r)| (l - r).abs())
        .fold(0.0f32, f32::max);

    assert!(
        max_diff < 0.001,
        "Identical signals should be the same in both channels, max diff: {}",
        max_diff
    );
}

/// Test writing stereo WAV file
#[test]
fn test_stereo_wav_output() {
    let dsl = r#"
tempo: 1.0
out1: sine 440 * 0.3
out2: sine 880 * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let (left, right) = graph.render_stereo((SAMPLE_RATE * 0.5) as usize);

    // Write stereo WAV file
    let filename = "/tmp/test_stereo_output.wav";
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: SAMPLE_RATE as u32,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(filename, spec).unwrap();

    // Interleave left and right samples
    for (l, r) in left.iter().zip(right.iter()) {
        writer.write_sample((l * i16::MAX as f32) as i16).unwrap();
        writer.write_sample((r * i16::MAX as f32) as i16).unwrap();
    }

    writer.finalize().unwrap();

    // Verify file was created
    assert!(std::path::Path::new(filename).exists());
}
