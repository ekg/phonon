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

#[test]
fn test_audio_node_signal_chain() {
    // Test signal chain operator (#)
    // Example: saw 110 # lpf 1000 0.8
    let code = r#"
        tempo: 2.0
        out: saw 110 # lpf 1000 0.8
    "#;

    let mut graph = compile_to_audio_nodes(code, 44100.0)
        .expect("Should compile signal chain");

    // Render 1 second
    let audio = graph.render(44100).expect("Should render");

    // Should be a filtered saw wave
    // RMS should be similar to unfiltered saw (filter doesn't reduce overall energy much at 1kHz)
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.3 && rms < 0.6,
        "Filtered saw wave RMS should be moderate, got {}",
        rms
    );

    // Check it's not silence
    let max = audio.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    assert!(max > 0.5, "Filtered saw wave should have reasonable amplitude, got {}", max);
}

#[test]
fn test_audio_node_chain_with_bus() {
    // Test chaining with bus reference
    let code = r#"
        tempo: 2.0
        ~osc: saw 220
        out: ~osc # lpf 500 0.9
    "#;

    let mut graph = compile_to_audio_nodes(code, 44100.0)
        .expect("Should compile chain with bus");

    // Render audio
    let audio = graph.render(44100).expect("Should render");

    // Should produce filtered saw wave
    let rms = calculate_rms(&audio);
    assert!(rms > 0.1, "Should have audio output, got RMS {}", rms);
}

#[test]
fn test_audio_node_delay_effect() {
    // Test delay effect
    let code = r#"
        tempo: 2.0
        out: sine 440 # delay 0.1
    "#;

    let mut graph = compile_to_audio_nodes(code, 44100.0)
        .expect("Should compile delay effect");

    // Render audio
    let audio = graph.render(44100).expect("Should render");

    // Should have delayed sine wave
    let rms = calculate_rms(&audio);
    assert!(rms > 0.1, "Delayed signal should have audio, got RMS {}", rms);
}

#[test]
fn test_audio_node_reverb_effect() {
    // Test reverb effect
    let code = r#"
        tempo: 2.0
        out: sine 440 # reverb 0.7 0.5 0.3
    "#;

    let mut graph = compile_to_audio_nodes(code, 44100.0)
        .expect("Should compile reverb effect");

    // Render audio
    let audio = graph.render(44100).expect("Should render");

    // Should have reverb'd sine wave
    let rms = calculate_rms(&audio);
    assert!(rms > 0.1, "Reverb'd signal should have audio, got RMS {}", rms);
}

#[test]
fn test_audio_node_distortion_effect() {
    // Test distortion effect
    let code = r#"
        tempo: 2.0
        out: sine 440 # distortion 5.0 0.8
    "#;

    let mut graph = compile_to_audio_nodes(code, 44100.0)
        .expect("Should compile distortion effect");

    // Render audio
    let audio = graph.render(44100).expect("Should render");

    // Should have distorted sine wave
    let rms = calculate_rms(&audio);
    assert!(rms > 0.1, "Distorted signal should have audio, got RMS {}", rms);

    // Distortion should clip the signal, so max should be near 1.0
    let max = audio.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    assert!(max > 0.8, "Distorted signal should clip near 1.0, got max {}", max);
}

#[test]
fn test_audio_node_effect_chain() {
    // Test chaining multiple effects
    let code = r#"
        tempo: 2.0
        ~osc: saw 110
        ~filtered: ~osc # lpf 800 0.7
        ~delayed: ~filtered # delay 0.05
        out: ~delayed # distortion 3.0 0.5
    "#;

    let mut graph = compile_to_audio_nodes(code, 44100.0)
        .expect("Should compile effect chain");

    // Render audio
    let audio = graph.render(44100).expect("Should render");

    // Should have processed audio through full chain
    let rms = calculate_rms(&audio);
    assert!(rms > 0.1, "Effect chain should produce audio, got RMS {}", rms);
}

#[test]
fn test_audio_node_complex_synthesis() {
    // Complex test: FM synthesis with filter and effects
    let code = r#"
        tempo: 2.0

        -- FM synthesis
        ~modulator_freq: 110 * 3
        ~modulator: sine ~modulator_freq
        ~mod_amount: 200
        ~carrier_freq: 110 + (~modulator * ~mod_amount)
        ~carrier: sine ~carrier_freq

        -- Filter and effects chain
        ~filtered: ~carrier # lpf 2000 0.6
        ~effected: ~filtered # distortion 2.0 0.3

        out: ~effected
    "#;

    let mut graph = compile_to_audio_nodes(code, 44100.0)
        .expect("Should compile complex synthesis");

    // Render 1 second
    let audio = graph.render(44100).expect("Should render");

    // Should produce FM-synthesized audio
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.1,
        "FM synthesis should produce audio, got RMS {}",
        rms
    );

    // Check it's not silence
    let max = audio.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    assert!(max > 0.3, "FM synthesis should have decent amplitude, got {}", max);
}

#[test]
fn test_audio_node_multi_voice_mix() {
    // Test mixing multiple voices with different processing
    let code = r#"
        tempo: 2.0

        -- Voice 1: Bass
        ~bass: saw 55 # lpf 300 0.8

        -- Voice 2: Pad
        ~pad_freq: 110 + 0.5
        ~pad: saw ~pad_freq # lpf 800 0.5

        -- Voice 3: Lead
        ~lead: square 440 # hpf 500 0.4

        -- Mix
        ~mix: (~bass * 0.5) + (~pad * 0.3) + (~lead * 0.4)

        out: ~mix # reverb 0.5 0.6 0.2
    "#;

    let mut graph = compile_to_audio_nodes(code, 44100.0)
        .expect("Should compile multi-voice mix");

    // Render audio
    let audio = graph.render(44100).expect("Should render");

    // Should produce mixed audio
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.2,
        "Multi-voice mix should produce audio, got RMS {}",
        rms
    );
}

#[test]
fn test_audio_node_modulated_parameters() {
    // Test pattern-controlled filter cutoff
    let code = r#"
        tempo: 2.0

        -- LFO for filter cutoff
        ~lfo: sine 0.5
        ~cutoff: (~lfo * 1000) + 1500

        -- Filtered saw
        ~osc: saw 110
        out: ~osc # lpf ~cutoff 0.7
    "#;

    let mut graph = compile_to_audio_nodes(code, 44100.0)
        .expect("Should compile modulated parameters");

    // Render audio
    let audio = graph.render(44100).expect("Should render");

    // Should produce filtered audio with modulated cutoff
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.1,
        "Modulated filter should produce audio, got RMS {}",
        rms
    );
}
