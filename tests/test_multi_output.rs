//! Tests for multi-output system (out1, out2, etc.)

use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

#[test]
fn test_multi_output_two_channels() {
    let input = r#"
        tempo: 2.0
        out1: s("bd ~ bd ~") * 0.5
        out2: s("~ sn ~ sn") * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse multi-output DSL");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Render 1 second (2 cycles at 2 CPS)
    let buffer = graph.render(44100);

    // Calculate RMS
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    // Should produce audio from both channels
    assert!(
        rms > 0.1,
        "Multi-output should produce audio, got RMS: {}",
        rms
    );
    println!("✅ test_multi_output_two_channels: RMS = {:.6}", rms);
}

#[test]
fn test_multi_output_single_channel() {
    let input = r#"
        tempo: 2.0
        out1: s("bd sn hh cp") * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(22050);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    assert!(
        rms > 0.05,
        "Single numbered output should produce audio, got RMS: {}",
        rms
    );
    println!("✅ test_multi_output_single_channel: RMS = {:.6}", rms);
}

#[test]
fn test_multi_output_three_channels() {
    let input = r#"
        tempo: 2.0
        out1: s("bd ~ bd ~") * 0.3
        out2: s("~ sn ~ sn") * 0.3
        out3: s("hh hh hh hh") * 0.3
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(44100);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    // Three channels should produce more combined output
    assert!(
        rms > 0.15,
        "Three-channel output should produce audio, got RMS: {}",
        rms
    );
    println!("✅ test_multi_output_three_channels: RMS = {:.6}", rms);
}

#[test]
fn test_multi_output_with_plain_out() {
    // Test that plain "out" still works alongside numbered outputs
    let input = r#"
        tempo: 2.0
        out: s("bd ~ bd ~") * 0.3
        out1: s("~ sn ~ sn") * 0.3
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(44100);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    // Both plain "out" and numbered outputs should work together
    assert!(
        rms > 0.1,
        "Mixed plain and numbered output should produce audio, got RMS: {}",
        rms
    );
    println!("✅ test_multi_output_with_plain_out: RMS = {:.6}", rms);
}

#[test]
fn test_multi_output_different_patterns() {
    // Test with different types of patterns
    let input = r#"
        tempo: 2.0
        out1: sine("110 220") * 0.2
        out2: s("bd sn") * 0.3
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(22050);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    assert!(
        rms > 0.05,
        "Multi-output with different pattern types should work, got RMS: {}",
        rms
    );
    println!("✅ test_multi_output_different_patterns: RMS = {:.6}", rms);
}
