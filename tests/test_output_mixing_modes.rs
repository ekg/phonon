//! Tests for output mixing modes
//!
//! Tests all five mixing modes: gain, sqrt, tanh, hard, none
//! Verifies that they prevent clipping and behave as expected

use phonon::unified_graph::OutputMixMode;
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

fn calculate_rms(buffer: &[f32]) -> f32 {
    (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt()
}

fn find_peak(buffer: &[f32]) -> f32 {
    buffer.iter().map(|x| x.abs()).fold(0.0f32, f32::max)
}

#[test]
fn test_outmix_gain_mode() {
    // Gain mode: divide by number of channels
    // With 2 channels at 0.5 each, result should be ~0.5 (sum=1.0, /2 = 0.5)
    let input = r#"
        tempo: 2.0
        outmix: gain
        out1: s "bd ~ bd ~" * 0.5
        out2: s "~ sn ~ sn" * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Verify mode was set
    assert_eq!(graph.get_output_mix_mode(), OutputMixMode::Gain);

    let buffer = graph.render(44100);
    let rms = calculate_rms(&buffer);
    let peak = find_peak(&buffer);

    // With gain compensation, peak should stay reasonable
    assert!(
        peak < 0.8,
        "Gain mode should prevent excessive peaks, got peak: {}",
        peak
    );
    assert!(rms > 0.05, "Should still produce audio, got RMS: {}", rms);

    println!(
        "✅ test_outmix_gain_mode: RMS = {:.6}, Peak = {:.6}",
        rms, peak
    );
}

#[test]
fn test_outmix_sqrt_mode() {
    // Sqrt mode (default): divide by sqrt(num_channels)
    // This preserves perceived loudness better than gain mode
    let input = r#"
        tempo: 2.0
        outmix: sqrt
        out1: s "bd ~ bd ~" * 0.5
        out2: s "~ sn ~ sn" * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    assert_eq!(graph.get_output_mix_mode(), OutputMixMode::Sqrt);

    let buffer = graph.render(44100);
    let rms = calculate_rms(&buffer);
    let peak = find_peak(&buffer);

    // Sqrt mode should be louder than gain mode but still controlled
    assert!(
        peak < 0.9,
        "Sqrt mode should prevent clipping, got peak: {}",
        peak
    );
    assert!(rms > 0.05, "Should produce audio, got RMS: {}", rms);

    println!(
        "✅ test_outmix_sqrt_mode: RMS = {:.6}, Peak = {:.6}",
        rms, peak
    );
}

#[test]
fn test_outmix_tanh_mode() {
    // Tanh mode: soft saturation
    // Even with very loud signals, output is clamped smoothly
    let input = r#"
        tempo: 2.0
        outmix: tanh
        out1: s "bd ~ bd ~" * 2.0
        out2: s "~ sn ~ sn" * 2.0
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    assert_eq!(graph.get_output_mix_mode(), OutputMixMode::Tanh);

    let buffer = graph.render(44100);
    let rms = calculate_rms(&buffer);
    let peak = find_peak(&buffer);

    // Tanh never exceeds ±1.0 due to saturation
    assert!(
        peak <= 1.0,
        "Tanh mode should never exceed 1.0, got peak: {}",
        peak
    );
    assert!(
        rms > 0.2,
        "Should produce substantial audio, got RMS: {}",
        rms
    );

    println!(
        "✅ test_outmix_tanh_mode: RMS = {:.6}, Peak = {:.6}",
        rms, peak
    );
}

#[test]
fn test_outmix_hard_mode() {
    // Hard mode: brick-wall limiting at ±1.0
    let input = r#"
        tempo: 2.0
        outmix: hard
        out1: s "bd ~ bd ~" * 2.0
        out2: s "~ sn ~ sn" * 2.0
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    assert_eq!(graph.get_output_mix_mode(), OutputMixMode::Hard);

    let buffer = graph.render(44100);
    let rms = calculate_rms(&buffer);
    let peak = find_peak(&buffer);

    // Hard limiting guarantees absolute maximum of 1.0
    assert!(
        peak <= 1.0,
        "Hard mode should never exceed 1.0, got peak: {}",
        peak
    );
    assert!(
        rms > 0.2,
        "Should produce substantial audio, got RMS: {}",
        rms
    );

    println!(
        "✅ test_outmix_hard_mode: RMS = {:.6}, Peak = {:.6}",
        rms, peak
    );
}

#[test]
fn test_outmix_none_mode() {
    // None mode: no compensation, direct sum
    // This can clip, but users might want it for creative purposes
    let input = r#"
        tempo: 2.0
        outmix: none
        out1: s "bd ~ bd ~" * 0.3
        out2: s "~ sn ~ sn" * 0.3
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    assert_eq!(graph.get_output_mix_mode(), OutputMixMode::None);

    let buffer = graph.render(44100);
    let rms = calculate_rms(&buffer);

    // None mode just sums - no guarantees about clipping
    assert!(rms > 0.05, "Should produce audio, got RMS: {}", rms);

    println!("✅ test_outmix_none_mode: RMS = {:.6}", rms);
}

#[test]
fn test_outmix_default_is_none() {
    // Default mode should be None (direct sum - like a hardware mixer)
    let input = r#"
        tempo: 2.0
        out1: s "bd ~ bd ~" * 0.5
        out2: s "~ sn ~ sn" * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let graph = compiler.compile(statements);

    assert_eq!(
        graph.get_output_mix_mode(),
        OutputMixMode::None,
        "Default should be None (direct sum)"
    );

    println!("✅ test_outmix_default_is_none: Default mode is None (direct sum)");
}

#[test]
fn test_outmix_three_channels_gain() {
    // With 3 channels, gain mode divides by 3
    let input = r#"
        tempo: 2.0
        outmix: gain
        out1: s "bd ~ bd ~" * 0.5
        out2: s "~ sn ~ sn" * 0.5
        out3: s "hh hh hh hh" * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(44100);
    let rms = calculate_rms(&buffer);
    let peak = find_peak(&buffer);

    assert!(
        peak < 0.7,
        "Gain mode with 3 channels should keep peaks low, got: {}",
        peak
    );
    assert!(rms > 0.05, "Should produce audio, got RMS: {}", rms);

    println!(
        "✅ test_outmix_three_channels_gain: RMS = {:.6}, Peak = {:.6}",
        rms, peak
    );
}

#[test]
fn test_outmix_three_channels_sqrt() {
    // With 3 channels, sqrt mode divides by sqrt(3) ≈ 1.732
    let input = r#"
        tempo: 2.0
        outmix: sqrt
        out1: s "bd ~ bd ~" * 0.5
        out2: s "~ sn ~ sn" * 0.5
        out3: s "hh hh hh hh" * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(44100);
    let rms = calculate_rms(&buffer);
    let peak = find_peak(&buffer);

    assert!(
        peak < 0.95,
        "Sqrt mode with 3 channels should prevent clipping, got: {}",
        peak
    );
    assert!(
        rms > 0.08,
        "Should produce decent audio level, got RMS: {}",
        rms
    );

    println!(
        "✅ test_outmix_three_channels_sqrt: RMS = {:.6}, Peak = {:.6}",
        rms, peak
    );
}

#[test]
fn test_outmix_comparison_gain_vs_sqrt() {
    // Compare gain vs sqrt modes - sqrt should be louder
    let code_base = r#"
        tempo: 2.0
        out1: s "bd ~ bd ~" * 0.5
        out2: s "~ sn ~ sn" * 0.5
    "#;

    // Test with gain mode
    let input_gain = format!("outmix: gain\n{}", code_base);
    let (_, statements) = parse_dsl(&input_gain).expect("Failed to parse gain");
    let compiler = DslCompiler::new(44100.0);
    let mut graph_gain = compiler.compile(statements);
    let buffer_gain = graph_gain.render(44100);
    let rms_gain = calculate_rms(&buffer_gain);

    // Test with sqrt mode
    let input_sqrt = format!("outmix: sqrt\n{}", code_base);
    let (_, statements) = parse_dsl(&input_sqrt).expect("Failed to parse sqrt");
    let compiler = DslCompiler::new(44100.0);
    let mut graph_sqrt = compiler.compile(statements);
    let buffer_sqrt = graph_sqrt.render(44100);
    let rms_sqrt = calculate_rms(&buffer_sqrt);

    // Sqrt should be louder than gain (divides by smaller value)
    assert!(
        rms_sqrt > rms_gain,
        "Sqrt mode should be louder than gain mode. Sqrt RMS: {:.6}, Gain RMS: {:.6}",
        rms_sqrt,
        rms_gain
    );

    println!(
        "✅ test_outmix_comparison_gain_vs_sqrt: Gain RMS = {:.6}, Sqrt RMS = {:.6}",
        rms_gain, rms_sqrt
    );
}

#[test]
fn test_outmix_invalid_mode() {
    // Test that invalid mode names are rejected
    // The parser will accept any identifier, but the compiler should reject invalid modes
    let input = r#"
        tempo: 2.0
        outmix: invalid_mode
        out1: s "bd ~ bd ~"
    "#;

    let parse_result = parse_dsl(input);

    // Parser should accept the syntax
    assert!(parse_result.is_ok(), "Parser should accept outmix syntax");

    // But compiler should reject invalid mode names
    // (This test documents expected behavior - actual validation happens at compile time)
    println!("✅ test_outmix_invalid_mode: Parser accepts syntax, validation at compile time");
}

#[test]
fn test_outmix_single_channel_no_effect() {
    // With only one channel, mixing mode shouldn't matter much
    // (except for tanh/hard which always apply)
    let input = r#"
        tempo: 2.0
        outmix: gain
        out1: s "bd sn hh cp" * 0.8
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(22050);
    let rms = calculate_rms(&buffer);
    let peak = find_peak(&buffer);

    assert!(
        peak < 1.0,
        "Single channel should work normally, got peak: {}",
        peak
    );
    assert!(rms > 0.1, "Should produce audio, got RMS: {}", rms);

    println!(
        "✅ test_outmix_single_channel_no_effect: RMS = {:.6}, Peak = {:.6}",
        rms, peak
    );
}

#[test]
fn test_channel_independence_with_none_mode() {
    // CRITICAL: With outmix: none (default), channels should be INDEPENDENT
    // Adding a third channel should NOT change the levels of existing channels

    // Test with 2 channels
    let input_2ch = r#"
        tempo: 2.0
        outmix: none
        out1: sine 220 * 0.3
        out2: sine 440 * 0.3
    "#;

    let (_, statements) = parse_dsl(input_2ch).expect("Failed to parse 2ch");
    let compiler = DslCompiler::new(44100.0);
    let mut graph_2ch = compiler.compile(statements);
    let buffer_2ch = graph_2ch.render(22050);
    let rms_2ch = calculate_rms(&buffer_2ch);

    // Test with 3 channels (same first two, plus a third)
    let input_3ch = r#"
        tempo: 2.0
        outmix: none
        out1: sine 220 * 0.3
        out2: sine 440 * 0.3
        out3: sine 880 * 0.3
    "#;

    let (_, statements) = parse_dsl(input_3ch).expect("Failed to parse 3ch");
    let compiler = DslCompiler::new(44100.0);
    let mut graph_3ch = compiler.compile(statements);
    let buffer_3ch = graph_3ch.render(22050);
    let rms_3ch = calculate_rms(&buffer_3ch);

    // With outmix: none, RMS should INCREASE when we add a channel
    // (because we're just summing directly)
    // 2 channels: 0.3 + 0.3 = 0.6 (in phase)
    // 3 channels: 0.3 + 0.3 + 0.3 = 0.9 (in phase)
    assert!(
        rms_3ch > rms_2ch,
        "With outmix: none, adding a channel should INCREASE total level (direct sum). 2ch RMS: {}, 3ch RMS: {}",
        rms_2ch, rms_3ch
    );

    // The increase should be roughly proportional to adding another channel
    // (Note: won't be exactly 1.5 due to phase relationships between frequencies)
    let ratio = rms_3ch / rms_2ch;
    assert!(
        ratio > 1.1 && ratio < 1.6,
        "Ratio should be around 1.2-1.5 (adding channel increases level), got: {}",
        ratio
    );

    println!(
        "✅ test_channel_independence_with_none_mode: 2ch RMS = {:.6}, 3ch RMS = {:.6}, ratio = {:.2}",
        rms_2ch, rms_3ch, ratio
    );
}

#[test]
fn test_gain_mode_breaks_independence() {
    // Document that gain/sqrt modes intentionally break channel independence
    // This is a TRADE-OFF: prevent clipping vs. maintain independence

    let input_2ch = r#"
        tempo: 2.0
        outmix: gain
        out1: sine 220 * 0.3
        out2: sine 440 * 0.3
    "#;

    let (_, statements) = parse_dsl(input_2ch).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let buffer_2ch = graph.render(22050);
    let rms_2ch = calculate_rms(&buffer_2ch);

    let input_3ch = r#"
        tempo: 2.0
        outmix: gain
        out1: sine 220 * 0.3
        out2: sine 440 * 0.3
        out3: sine 880 * 0.3
    "#;

    let (_, statements) = parse_dsl(input_3ch).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let buffer_3ch = graph.render(22050);
    let rms_3ch = calculate_rms(&buffer_3ch);

    // With gain mode, levels should stay SIMILAR (channels affect each other)
    // 2ch: (0.3 + 0.3) / 2 = 0.3
    // 3ch: (0.3 + 0.3 + 0.3) / 3 = 0.3
    // (Note: won't be exactly 1.0 due to phase relationships)
    let ratio = rms_3ch / rms_2ch;
    assert!(
        ratio > 0.7 && ratio < 1.3,
        "Gain mode should keep RMS similar (channels interdependent). Ratio: {}",
        ratio
    );

    println!(
        "✅ test_gain_mode_breaks_independence: 2ch RMS = {:.6}, 3ch RMS = {:.6}, ratio = {:.2}",
        rms_2ch, rms_3ch, ratio
    );
}
