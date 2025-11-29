/// Test multi-output bus assignments (out $, o2:, etc.) in AudioNode mode
///
/// Verifies that:
/// 1. out $, o2:, etc. compile correctly
/// 2. Multiple outputs are mixed together automatically
/// 3. Mixed output produces correct audio

use phonon::compositional_compiler::CompilerContext;
use phonon::compositional_parser::parse_program;

/// Helper: Compile DSL code and get AudioNodeGraph
fn compile_to_audio_nodes(code: &str, sample_rate: f32) -> Result<phonon::audio_node_graph::AudioNodeGraph, String> {
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

    // Get the graph (finalize will mix outputs automatically)
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
fn test_single_output_o1() {
    // Single out $ output should work
    let code = r#"
        out $ 0.5
    "#;

    let mut graph = compile_to_audio_nodes(code, 44100.0)
        .expect("Should compile out $");

    // Render 512 samples
    let audio = graph.render(512).expect("Should render");

    // All samples should be 0.5
    for sample in &audio {
        assert!((sample - 0.5).abs() < 0.001, "Expected 0.5, got {}", sample);
    }
}

#[test]
fn test_two_outputs_o1_o2() {
    // Two outputs should be mixed together
    let code = r#"
        out $ 0.3
        o2: 0.2
    "#;

    let mut graph = compile_to_audio_nodes(code, 44100.0)
        .expect("Should compile out $ and o2:");

    // Render 512 samples
    let audio = graph.render(512).expect("Should render");

    // All samples should be 0.5 (0.3 + 0.2)
    for sample in &audio {
        assert!((sample - 0.5).abs() < 0.001,
            "Expected 0.5 (0.3 + 0.2), got {}", sample);
    }
}

#[test]
fn test_three_outputs_mixed() {
    // Three outputs should all be mixed together
    let code = r#"
        out $ 0.1
        o2: 0.2
        o3: 0.3
    "#;

    let mut graph = compile_to_audio_nodes(code, 44100.0)
        .expect("Should compile out $, o2:, o3:");

    // Render 512 samples
    let audio = graph.render(512).expect("Should render");

    // All samples should be 0.6 (0.1 + 0.2 + 0.3)
    for sample in &audio {
        assert!((sample - 0.6).abs() < 0.001,
            "Expected 0.6 (0.1 + 0.2 + 0.3), got {}", sample);
    }
}

#[test]
fn test_output_with_samples() {
    // Multi-output with sample playback
    let code = r#"
        tempo: 0.5
        out $ s "bd"
        o2: s "sn"
    "#;

    let mut graph = compile_to_audio_nodes(code, 44100.0)
        .expect("Should compile sample outputs");

    // Render 1 second
    let audio = graph.render(44100).expect("Should render");

    // Check that audio is not silence
    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "RMS should be > 0.01 (not silence), got {}", rms);

    // Check peak is reasonable
    let peak = audio.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    assert!(peak > 0.1, "Peak should be > 0.1, got {}", peak);
}

#[test]
fn test_output_with_synthesis() {
    // Multi-output with synthesizers
    let code = r#"
        tempo: 0.5
        out $ sine 220
        o2: sine 440
    "#;

    let mut graph = compile_to_audio_nodes(code, 44100.0)
        .expect("Should compile synthesis outputs");

    // Render 1 second
    let audio = graph.render(44100).expect("Should render");

    // Check RMS (two sine waves mixed)
    let rms = calculate_rms(&audio);
    // Two sine waves: each has RMS of ~0.707, mixed together should be higher
    assert!(rms > 0.9, "RMS should be > 0.9 (two sine waves), got {}", rms);
}

#[test]
fn test_explicit_out_overrides_numbered() {
    // If out $ is specified, it should override numbered outputs
    let code = r#"
        out $ 0.3
        o2: 0.2
        out $ 1.0
    "#;

    let mut graph = compile_to_audio_nodes(code, 44100.0)
        .expect("Should compile with explicit out:");

    // Render 512 samples
    let audio = graph.render(512).expect("Should render");

    // All samples should be 1.0 (out $ overrides out $ and o2:)
    for sample in &audio {
        assert!((sample - 1.0).abs() < 0.001,
            "Expected 1.0 (out $ overrides), got {}", sample);
    }
}

#[test]
fn test_bus_references_in_outputs() {
    // Outputs can reference buses
    let code = r#"
        ~bass $ 0.4
        ~drums $ 0.3
        out $ ~bass
        o2: ~drums
    "#;

    let mut graph = compile_to_audio_nodes(code, 44100.0)
        .expect("Should compile with bus references");

    // Render 512 samples
    let audio = graph.render(512).expect("Should render");

    // All samples should be 0.7 (0.4 + 0.3)
    for sample in &audio {
        assert!((sample - 0.7).abs() < 0.001,
            "Expected 0.7 (0.4 + 0.3), got {}", sample);
    }
}

#[test]
fn test_complex_multi_output() {
    // Complex example with multiple buses and outputs
    let code = r#"
        tempo: 0.5
        ~lfo $ sine 0.5
        ~bass $ sine 55
        ~lead $ sine 220

        out $ ~bass * 0.5
        o2: ~lead * 0.3
        o3: ~lfo * 0.1
    "#;

    let mut graph = compile_to_audio_nodes(code, 44100.0)
        .expect("Should compile complex multi-output");

    // Render 1 second
    let audio = graph.render(44100).expect("Should render");

    // Check that audio is not silence
    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "RMS should be > 0.01, got {}", rms);
}
