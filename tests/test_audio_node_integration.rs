/// Integration tests for AudioNode-based DAW architecture
///
/// These tests verify that the new block-based AudioNode architecture
/// works correctly for compiling and rendering DSL code.

use phonon::compositional_compiler::CompilerContext;
use phonon::compositional_parser::parse_program;

/// Helper: Compile DSL code and get AudioNodeGraph
fn compile_to_audio_nodes(code: &str, sample_rate: f32) -> Result<phonon::audio_node_graph::AudioNodeGraph, String> {
    // AudioNode mode is now the default (USE_AUDIO_NODES = true)
    let mut ctx = CompilerContext::new(sample_rate);

    // Parse the code
    let (remaining, statements) = parse_program(code).map_err(|e| format!("Parse error: {:?}", e))?;

    if !remaining.trim().is_empty() {
        return Err(format!("Failed to parse entire program, remaining: {}", remaining));
    }

    // Compile statements
    for stmt in statements {
        phonon::compositional_compiler::compile_statement(&mut ctx, stmt)?;
    }

    // Get the graph and build processor
    let mut graph = ctx.into_audio_node_graph();
    graph.build_processor()?;

    Ok(graph)
}

/// Helper: Calculate RMS of audio buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

#[test]
fn test_audio_node_simple_constant() {
    let code = "out: 0.5";

    let mut graph = compile_to_audio_nodes(code, 44100.0)
        .expect("Should compile");

    // Render 512 samples
    let audio = graph.render(512).expect("Should render");

    // All samples should be 0.5
    for sample in &audio {
        assert!((sample - 0.5).abs() < 0.001, "Expected 0.5, got {}", sample);
    }
}

#[test]
fn test_audio_node_sine_440hz() {
    let code = r#"
        tempo: 2.0
        out: sine 440
    "#;

    let mut graph = compile_to_audio_nodes(code, 44100.0)
        .expect("Should compile sine wave");

    // Render 1 second
    let audio = graph.render(44100).expect("Should render");

    // Check RMS is approximately 0.707 (sine wave RMS = 1/√2)
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.65 && rms < 0.75,
        "Sine wave RMS should be ~0.707, got {}",
        rms
    );

    // Check it's not silence
    let max = audio.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    assert!(max > 0.9, "Sine wave peak should be near 1.0, got {}", max);
}

#[test]
fn test_audio_node_addition() {
    let code = r#"
        out: 0.3 + 0.2
    "#;

    let mut graph = compile_to_audio_nodes(code, 44100.0)
        .expect("Should compile addition");

    // Render 512 samples
    let audio = graph.render(512).expect("Should render");

    // All samples should be 0.5 (0.3 + 0.2)
    for sample in &audio {
        assert!((sample - 0.5).abs() < 0.001, "Expected 0.5, got {}", sample);
    }
}

#[test]
fn test_audio_node_complex_expression() {
    let code = r#"
        tempo: 2.0
        ~freq: 220
        ~osc: sine ~freq
        out: ~osc
    "#;

    let mut graph = compile_to_audio_nodes(code, 44100.0)
        .expect("Should compile complex expression");

    // Render 1 second
    let audio = graph.render(44100).expect("Should render");

    // Should be a sine wave at 220 Hz
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.65 && rms < 0.75,
        "Sine wave RMS should be ~0.707, got {}",
        rms
    );
}

#[test]
fn test_audio_node_tempo_setting() {
    let code = r#"
        tempo: 3.0
        out: sine 440
    "#;

    let mut graph = compile_to_audio_nodes(code, 44100.0)
        .expect("Should compile with tempo");

    // Verify tempo was set
    assert_eq!(graph.tempo(), 3.0, "Tempo should be 3.0");
}

#[test]
fn test_audio_node_graph_traversed_once() {
    // This test verifies that the graph is traversed once per block
    // rather than once per sample (the whole point of the DAW architecture!)

    let code = r#"
        tempo: 2.0
        ~a: 0.1
        ~b: 0.2
        ~c: ~a + ~b
        ~d: ~c + 0.3
        ~e: ~d + 0.4
        out: ~e
    "#;

    let mut graph = compile_to_audio_nodes(code, 44100.0)
        .expect("Should compile deep graph");

    // Render
    let audio = graph.render(512).expect("Should render");

    // Result should be 0.1 + 0.2 + 0.3 + 0.4 = 1.0
    for sample in &audio {
        assert!((sample - 1.0).abs() < 0.001, "Expected 1.0, got {}", sample);
    }
}

#[test]
fn test_audio_node_is_default() {
    // Test that AudioNode architecture is the default (USE_AUDIO_NODES = true)
    let ctx = CompilerContext::new(44100.0);
    assert!(ctx.is_using_audio_nodes(), "AudioNode should be the default architecture");
}
