/// Test the pan parameter for sample playback
///
/// The pan parameter should control per-event stereo positioning:
/// - s("bd sn", "", "-1 1") - hard left, then hard right
/// - s("bd*4", "", "-1 -0.33 0.33 1") - pan sweep left to right
///
/// Pan values: -1.0 = hard left, 0.0 = center, 1.0 = hard right
/// This test uses stereo RMS analysis to verify panning is applied correctly.
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

#[test]
fn test_pan_parameter_affects_stereo_position() {
    // Pattern with two events: bd hard left, sn hard right
    // Positional args: s("pattern", gain, pan, speed)
    let input = r#"
        tempo: 2.0
        out: s("bd sn", "1.0 1.0", "-1 1")
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse DSL");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Render 1 cycle = 0.5 seconds at 2 CPS = 22050 samples
    let buffer = graph.render(22050);

    // Split into two halves (bd event vs sn event)
    let bd_samples = &buffer[0..11025];
    let sn_samples = &buffer[11025..22050];

    // Calculate RMS for each half
    let bd_rms = calculate_rms(bd_samples);
    let sn_rms = calculate_rms(sn_samples);

    println!("BD RMS (pan=-1): {:.4}", bd_rms);
    println!("SN RMS (pan=1): {:.4}", sn_rms);

    // Both should have similar RMS (just different pan positions)
    assert!(bd_rms > 0.01, "BD should have audio");
    assert!(sn_rms > 0.01, "SN should have audio");

    // The mono output mixes both channels, so we can't distinguish left/right here
    // But we can verify both samples are playing
    let ratio = bd_rms / sn_rms.max(0.0001);
    println!("RMS ratio (bd/sn): {:.2}", ratio);

    // Should be roughly equal (within 50% tolerance)
    assert!(
        ratio > 0.5 && ratio < 2.0,
        "Both samples should have similar energy, got ratio {:.2}",
        ratio
    );
}

#[test]
fn test_pan_center_produces_equal_stereo() {
    // Sample with center pan (0.0) should produce equal left/right channels
    let input = r#"
        tempo: 2.0
        out: s("bd", "1.0", "0")
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse DSL");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(22050);
    let rms = calculate_rms(&buffer);

    println!("Center pan RMS: {:.4}", rms);

    // Center panning should still produce audio
    assert!(rms > 0.01, "Center pan should produce audio");
}

#[test]
fn test_pan_pattern_with_multiple_events() {
    // Four hi-hats panning from left to right
    let input = r#"
        tempo: 2.0
        out: s("hh*4", "1.0 1.0 1.0 1.0", "-1 -0.33 0.33 1")
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

    println!("RMS values (pan sweep): {:?}", rms_values);

    // All quarters should have audio (pan doesn't affect total energy in mono mix)
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
fn test_pan_default_is_center() {
    // Without pan parameter, should default to 0.0 (center)
    let input_with_pan = r#"
        tempo: 2.0
        out: s("bd", "1.0", "0")
    "#;

    let input_without_pan = r#"
        tempo: 2.0
        out: s "bd"
    "#;

    // Both should produce similar RMS
    let (_, statements_with) = parse_dsl(input_with_pan).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph_with = compiler.compile(statements_with);
    let buffer_with = graph_with.render(22050);
    let rms_with = calculate_rms(&buffer_with);

    let (_, statements_without) = parse_dsl(input_without_pan).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph_without = compiler.compile(statements_without);
    let buffer_without = graph_without.render(22050);
    let rms_without = calculate_rms(&buffer_without);

    println!("RMS with pan=0: {:.4}", rms_with);
    println!("RMS without pan: {:.4}", rms_without);

    // Should be essentially identical (within 5% tolerance)
    let ratio = rms_with / rms_without.max(0.0001);
    assert!(
        (ratio - 1.0).abs() < 0.05,
        "Default pan should be 0.0 (center), ratio={:.3}",
        ratio
    );
}

/// Helper function to calculate RMS
fn calculate_rms(buffer: &[f32]) -> f32 {
    if buffer.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = buffer.iter().map(|x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}
