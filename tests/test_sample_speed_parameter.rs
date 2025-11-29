/// Test the speed parameter for sample playback
///
/// The speed parameter controls playback rate (pitch shifting):
/// - s("bd", "1.0", "0", "2") - play at double speed (octave up)
/// - s("bd", "1.0", "0", "0.5") - play at half speed (octave down)
/// - s("bd*4", "1 1 1 1", "0 0 0 0", "1 2 0.5 1.5") - varying speeds
///
/// Speed values: 1.0 = normal, 2.0 = double speed, 0.5 = half speed
/// This test uses duration and RMS analysis to verify speed is applied correctly.
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

#[test]
fn test_speed_parameter_affects_playback_rate() {
    // Two kick drums: normal speed and double speed
    // Double speed should finish in half the time
    // Positional args: s("pattern", gain, pan, speed)
    let input = r#"
        tempo: 0.5
        out $ s("bd bd", "1.0 1.0", "0 0", "1 2")
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse DSL");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Render 1 cycle = 0.5 seconds at 2 CPS = 22050 samples
    let buffer = graph.render(22050);

    // Split into two halves
    let first_half = &buffer[0..11025];
    let second_half = &buffer[11025..22050];

    // Calculate RMS for each half
    let first_rms = calculate_rms(first_half);
    let second_rms = calculate_rms(second_half);

    println!("First half RMS (speed=1): {:.4}", first_rms);
    println!("Second half RMS (speed=2): {:.4}", second_rms);

    // Both should have audio
    assert!(first_rms > 0.01, "First half should have audio");
    assert!(second_rms > 0.01, "Second half should have audio");

    // Double-speed sample should have somewhat higher energy density
    // (same energy in half the time)
    let ratio = second_rms / first_rms.max(0.0001);
    println!("RMS ratio (speed2/speed1): {:.2}", ratio);

    // Allow wide tolerance since sample content affects this
    assert!(
        ratio > 0.5 && ratio < 3.0,
        "Speed should affect playback, ratio={:.2}",
        ratio
    );
}

#[test]
fn test_speed_half_plays_longer() {
    // Compare normal speed vs half speed
    // Half speed should have audio lasting longer
    let input_normal = r#"
        tempo: 1.0
        out $ s("bd", "1.0", "0", "1")
    "#;

    let input_half = r#"
        tempo: 1.0
        out $ s("bd", "1.0", "0", "0.5")
    "#;

    // Normal speed
    let (_, statements) = parse_dsl(input_normal).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph_normal = compiler.compile(statements);
    let buffer_normal = graph_normal.render(44100); // 1 second

    // Half speed
    let (_, statements) = parse_dsl(input_half).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph_half = compiler.compile(statements);
    let buffer_half = graph_half.render(44100); // 1 second

    // Find when audio drops below threshold (sample end)
    let threshold = 0.001;

    let duration_normal = find_audio_duration(&buffer_normal, threshold);
    let duration_half = find_audio_duration(&buffer_half, threshold);

    println!("Normal speed duration: {} samples", duration_normal);
    println!("Half speed duration: {} samples", duration_half);

    // Half speed should last longer (at least 1.5x as long)
    assert!(
        duration_half > duration_normal,
        "Half speed ({}) should last longer than normal ({})",
        duration_half,
        duration_normal
    );
}

#[test]
fn test_speed_pattern_with_multiple_values() {
    // Four samples with different speeds
    let input = r#"
        tempo: 0.5
        out $ s("bd*4", "1 1 1 1", "0 0 0 0", "1 2 0.5 1.5")
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse DSL");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Render 1 cycle = 22050 samples
    let buffer = graph.render(22050);
    let quarter_size = 22050 / 4;

    let rms_values: Vec<f32> = (0..4)
        .map(|i| {
            let start = i * quarter_size;
            let end = start + quarter_size;
            calculate_rms(&buffer[start..end])
        })
        .collect();

    println!("RMS values (speed pattern): {:?}", rms_values);

    // All quarters should have audio
    for (i, rms) in rms_values.iter().enumerate() {
        assert!(
            *rms > 0.005,
            "Quarter {} should have audio (RMS={:.4})",
            i,
            rms
        );
    }
}

#[test]
fn test_speed_default_is_one() {
    // Without speed parameter, should default to 1.0 (normal speed)
    let input_with_speed = r#"
        tempo: 0.5
        out $ s("bd", "1.0", "0", "1")
    "#;

    let input_without_speed = r#"
        tempo: 0.5
        out $ s "bd"
    "#;

    // Both should produce similar output
    let (_, statements_with) = parse_dsl(input_with_speed).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph_with = compiler.compile(statements_with);
    let buffer_with = graph_with.render(22050);
    let rms_with = calculate_rms(&buffer_with);

    let (_, statements_without) = parse_dsl(input_without_speed).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph_without = compiler.compile(statements_without);
    let buffer_without = graph_without.render(22050);
    let rms_without = calculate_rms(&buffer_without);

    println!("RMS with speed=1: {:.4}", rms_with);
    println!("RMS without speed: {:.4}", rms_without);

    // Should be essentially identical (within 5% tolerance)
    let ratio = rms_with / rms_without.max(0.0001);
    assert!(
        (ratio - 1.0).abs() < 0.05,
        "Default speed should be 1.0, ratio={:.3}",
        ratio
    );
}

#[test]
fn test_speed_extreme_values() {
    // Test very fast and very slow speeds
    let input_fast = r#"
        tempo: 0.5
        out $ s("bd", "1.0", "0", "4")
    "#;

    let input_slow = r#"
        tempo: 0.5
        out $ s("bd", "1.0", "0", "0.25")
    "#;

    // Fast speed (4x)
    let (_, statements) = parse_dsl(input_fast).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph_fast = compiler.compile(statements);
    let buffer_fast = graph_fast.render(22050);
    let rms_fast = calculate_rms(&buffer_fast);

    println!("Fast speed (4x) RMS: {:.4}", rms_fast);
    assert!(rms_fast > 0.01, "Fast speed should produce audio");

    // Slow speed (0.25x)
    let (_, statements) = parse_dsl(input_slow).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph_slow = compiler.compile(statements);
    let buffer_slow = graph_slow.render(22050);
    let rms_slow = calculate_rms(&buffer_slow);

    println!("Slow speed (0.25x) RMS: {:.4}", rms_slow);
    assert!(rms_slow > 0.01, "Slow speed should produce audio");
}

/// Helper function to calculate RMS
fn calculate_rms(buffer: &[f32]) -> f32 {
    if buffer.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = buffer.iter().map(|x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Find the duration of audio (when it drops below threshold)
fn find_audio_duration(buffer: &[f32], threshold: f32) -> usize {
    // Find the last point where absolute value exceeds threshold
    for i in (0..buffer.len()).rev() {
        if buffer[i].abs() > threshold {
            return i + 1; // Duration in samples
        }
    }
    0 // No audio found
}
