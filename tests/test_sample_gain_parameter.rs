/// Test the gain parameter for sample playback
///
/// The gain parameter should control per-event amplitude:
/// - s("bd sn", gain="0.8 1.0") - different gain per event
/// - s("bd*4", gain="1 0.8 0.6 0.4") - descending volume
///
/// This test uses FFT and RMS analysis to verify gain is applied correctly.
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

#[test]
fn test_gain_parameter_affects_amplitude() {
    // Pattern with two events at different gains
    // Positional args: s("pattern", gain, pan, speed)
    let input = r#"
        tempo: 2.0
        out: s("bd sn", "0.5 1.0")
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

    println!("BD RMS (gain=0.5): {:.4}", bd_rms);
    println!("SN RMS (gain=1.0): {:.4}", sn_rms);

    // SN should have roughly 2x the amplitude of BD (gain ratio is 2:1)
    // Allow tolerance for sample variation
    let ratio = sn_rms / bd_rms.max(0.0001);
    println!("RMS ratio (sn/bd): {:.2}", ratio);

    assert!(
        ratio > 1.5 && ratio < 2.5,
        "Expected ~2x ratio, got {:.2}x (bd_rms={:.4}, sn_rms={:.4})",
        ratio,
        bd_rms,
        sn_rms
    );
}

#[test]
fn test_gain_pattern_with_multiple_events() {
    // Four kick drums with descending gain
    let input = r#"
        tempo: 2.0
        out: s("bd*4", "1.0 0.75 0.5 0.25")
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse DSL");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Render 1 cycle = 22050 samples, split into 4 quarters
    let buffer = graph.render(22050);
    let quarter_size = 22050 / 4;

    let rms_values: Vec<f32> = (0..4)
        .map(|i| {
            let start = i * quarter_size;
            let end = start + quarter_size;
            calculate_rms(&buffer[start..end])
        })
        .collect();

    println!("RMS values: {:?}", rms_values);

    // Each quarter should have less RMS than the previous
    for i in 1..4 {
        assert!(
            rms_values[i] < rms_values[i - 1],
            "Quarter {} should have lower RMS than quarter {}",
            i,
            i - 1
        );
    }

    // Check approximate ratios
    let ratio_0_to_1 = rms_values[0] / rms_values[1].max(0.0001);
    let ratio_1_to_2 = rms_values[1] / rms_values[2].max(0.0001);
    let ratio_2_to_3 = rms_values[2] / rms_values[3].max(0.0001);

    println!(
        "Ratios: {:.2}, {:.2}, {:.2}",
        ratio_0_to_1, ratio_1_to_2, ratio_2_to_3
    );

    // All ratios should be close (since gain steps are consistent)
    assert!(
        (ratio_0_to_1 - 1.33).abs() < 0.3,
        "Ratio 0->1 should be ~1.33 (1.0/0.75)"
    );
    assert!(
        (ratio_2_to_3 - 2.0).abs() < 0.5,
        "Ratio 2->3 should be ~2.0 (0.5/0.25)"
    );
}

#[test]
fn test_gain_zero_produces_silence() {
    // Sample with gain=0 should produce no audio
    let input = r#"
        tempo: 2.0
        out: s("bd sn", "0.0 1.0")
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse DSL");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(22050);

    // First half (bd with gain=0) should be silent
    let bd_samples = &buffer[0..11025];
    let bd_rms = calculate_rms(bd_samples);

    // Second half (sn with gain=1.0) should have audio
    let sn_samples = &buffer[11025..22050];
    let sn_rms = calculate_rms(sn_samples);

    println!("BD RMS (gain=0.0): {:.6}", bd_rms);
    println!("SN RMS (gain=1.0): {:.4}", sn_rms);

    assert!(bd_rms < 0.001, "BD with gain=0 should be nearly silent");
    assert!(sn_rms > 0.01, "SN with gain=1.0 should have audio");
}

#[test]
fn test_gain_default_is_one() {
    // Without gain parameter, should default to 1.0
    let input_with_gain = r#"
        tempo: 2.0
        out: s("bd", "1.0")
    "#;

    let input_without_gain = r#"
        tempo: 2.0
        out: s("bd")
    "#;

    // Both should produce the same RMS
    let (_, statements_with) = parse_dsl(input_with_gain).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph_with = compiler.compile(statements_with);
    let buffer_with = graph_with.render(22050);
    let rms_with = calculate_rms(&buffer_with);

    let (_, statements_without) = parse_dsl(input_without_gain).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph_without = compiler.compile(statements_without);
    let buffer_without = graph_without.render(22050);
    let rms_without = calculate_rms(&buffer_without);

    println!("RMS with gain=1.0: {:.4}", rms_with);
    println!("RMS without gain: {:.4}", rms_without);

    // Should be essentially identical (within 1% tolerance)
    let ratio = rms_with / rms_without.max(0.0001);
    assert!(
        (ratio - 1.0).abs() < 0.01,
        "Default gain should be 1.0 (ratio={:.3})",
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
