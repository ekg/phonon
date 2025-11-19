use phonon::unified_graph::{Signal, UnifiedSignalGraph, Waveform};

fn create_test_graph() -> UnifiedSignalGraph {
    UnifiedSignalGraph::new(44100.0)
}

fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

fn calculate_peak(buffer: &[f32]) -> f32 {
    buffer.iter().map(|&x| x.abs()).fold(0.0f32, f32::max)
}

fn is_valid_audio(buffer: &[f32]) -> bool {
    buffer.iter().all(|&x| x.is_finite())
}

#[test]
fn test_mix_two_constants() {
    let mut graph = create_test_graph();

    // Create mix of two constants: 0.5 + 0.3 = 0.8, normalized to 0.8/2 = 0.4
    let mix_id = graph.add_mix_node(vec![
        Signal::Value(0.5),
        Signal::Value(0.3),
    ]);

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&mix_id, &mut output);

    // With normalization: (0.5 + 0.3) / 2 = 0.4
    for &sample in &output {
        assert!((sample - 0.4).abs() < 0.001, "Expected 0.4, got {}", sample);
    }
}

#[test]
fn test_mix_three_constants() {
    let mut graph = create_test_graph();

    // Mix three constants: (0.6 + 0.3 + 0.9) / 3 = 0.6
    let mix_id = graph.add_mix_node(vec![
        Signal::Value(0.6),
        Signal::Value(0.3),
        Signal::Value(0.9),
    ]);

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&mix_id, &mut output);

    // Expected: (0.6 + 0.3 + 0.9) / 3 = 1.8 / 3 = 0.6
    for &sample in &output {
        assert!((sample - 0.6).abs() < 0.001, "Expected 0.6, got {}", sample);
    }
}

#[test]
fn test_mix_empty_signals() {
    let mut graph = create_test_graph();

    // Mix with no signals should produce silence
    let mix_id = graph.add_mix_node(vec![]);

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&mix_id, &mut output);

    for &sample in &output {
        assert_eq!(sample, 0.0, "Empty mix should produce silence");
    }
}

#[test]
fn test_mix_single_signal() {
    let mut graph = create_test_graph();

    // Mix with single signal should equal that signal / 1 (unchanged)
    let mix_id = graph.add_mix_node(vec![
        Signal::Value(0.75),
    ]);

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&mix_id, &mut output);

    for &sample in &output {
        assert!((sample - 0.75).abs() < 0.001, "Single signal mix should equal input");
    }
}

#[test]
fn test_mix_two_oscillators() {
    let mut graph = create_test_graph();

    // Create two sine oscillators at different frequencies
    let osc1 = graph.add_oscillator(Signal::Value(220.0), Waveform::Sine);
    let osc2 = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Mix them
    let mix_id = graph.add_mix_node(vec![
        Signal::Node(osc1),
        Signal::Node(osc2),
    ]);

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&mix_id, &mut output);

    // Verify audio is valid
    assert!(is_valid_audio(&output), "Output should contain valid audio");

    let rms = calculate_rms(&output);

    // Two sine waves mixed with normalization
    // Individual sine RMS ≈ 0.707, sum ≈ sqrt(2) * 0.707 ≈ 1.0, normalized: 1.0/2 = 0.5
    assert!(rms > 0.4 && rms < 0.6, "RMS should be around 0.5, got {}", rms);
}

#[test]
fn test_mix_three_oscillators() {
    let mut graph = create_test_graph();

    // Create three oscillators at different frequencies
    let osc1 = graph.add_oscillator(Signal::Value(220.0), Waveform::Sine);
    let osc2 = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let osc3 = graph.add_oscillator(Signal::Value(880.0), Waveform::Sine);

    // Mix them
    let mix_id = graph.add_mix_node(vec![
        Signal::Node(osc1),
        Signal::Node(osc2),
        Signal::Node(osc3),
    ]);

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&mix_id, &mut output);

    // Verify audio is valid
    assert!(is_valid_audio(&output), "Output should contain valid audio");

    let rms = calculate_rms(&output);

    // Three uncorrelated sine waves: sqrt(3) * 0.707 ≈ 1.22, normalized: 1.22/3 ≈ 0.41
    assert!(rms > 0.35 && rms < 0.5, "RMS should be around 0.41, got {}", rms);
}

#[test]
fn test_mix_many_oscillators() {
    let mut graph = create_test_graph();

    // Create 10 oscillators at different frequencies
    let mut signals = Vec::new();
    for i in 0..10 {
        let freq = 220.0 * (i + 1) as f32;
        let osc = graph.add_oscillator(Signal::Value(freq), Waveform::Sine);
        signals.push(Signal::Node(osc));
    }

    // Mix them
    let mix_id = graph.add_mix_node(signals);

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&mix_id, &mut output);

    // Verify audio is valid
    assert!(is_valid_audio(&output), "Output should contain valid audio");

    let rms = calculate_rms(&output);

    // 10 uncorrelated sine waves: sqrt(10) * 0.707 ≈ 2.24, normalized: 2.24/10 ≈ 0.224
    assert!(rms > 0.15 && rms < 0.3, "RMS should be around 0.22, got {}", rms);
}

#[test]
fn test_mix_different_waveforms() {
    let mut graph = create_test_graph();

    // Mix sine, saw, square
    let sine = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let saw = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);
    let square = graph.add_oscillator(Signal::Value(440.0), Waveform::Square);

    let mix_id = graph.add_mix_node(vec![
        Signal::Node(sine),
        Signal::Node(saw),
        Signal::Node(square),
    ]);

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&mix_id, &mut output);

    // Verify audio is valid
    assert!(is_valid_audio(&output), "Output should contain valid audio");

    let rms = calculate_rms(&output);

    // Should have reasonable energy (not silence, not clipping)
    assert!(rms > 0.2 && rms < 0.8, "RMS should be reasonable, got {}", rms);
}

#[test]
fn test_mix_constant_and_oscillator() {
    let mut graph = create_test_graph();

    // Mix a DC offset with an oscillator
    let osc = graph.add_oscillator(Signal::Value(220.0), Waveform::Sine);

    let mix_id = graph.add_mix_node(vec![
        Signal::Value(0.5),
        Signal::Node(osc),
    ]);

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&mix_id, &mut output);

    // Verify audio is valid
    assert!(is_valid_audio(&output), "Output should contain valid audio");

    // Should have DC offset from constant
    let mean: f32 = output.iter().sum::<f32>() / output.len() as f32;

    // DC component: 0.5/2 = 0.25
    assert!(mean > 0.2 && mean < 0.3, "Should have DC offset around 0.25, got {}", mean);
}

#[test]
fn test_mix_nested() {
    let mut graph = create_test_graph();

    // Create nested mix: mix(osc1, osc2), then mix that with osc3
    let osc1 = graph.add_oscillator(Signal::Value(220.0), Waveform::Sine);
    let osc2 = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let osc3 = graph.add_oscillator(Signal::Value(880.0), Waveform::Sine);

    let inner_mix = graph.add_mix_node(vec![
        Signal::Node(osc1),
        Signal::Node(osc2),
    ]);

    let outer_mix = graph.add_mix_node(vec![
        Signal::Node(inner_mix),
        Signal::Node(osc3),
    ]);

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&outer_mix, &mut output);

    // Verify audio is valid
    assert!(is_valid_audio(&output), "Output should contain valid audio");

    let rms = calculate_rms(&output);

    // Should produce reasonable audio
    assert!(rms > 0.2 && rms < 0.6, "RMS should be reasonable, got {}", rms);
}

#[test]
fn test_mix_with_gain() {
    let mut graph = create_test_graph();

    // Mix signals with different gains
    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let gain1 = graph.add_gain_node(Signal::Node(osc), Signal::Value(0.5));
    let gain2 = graph.add_gain_node(Signal::Node(osc), Signal::Value(1.0));

    let mix_id = graph.add_mix_node(vec![
        Signal::Node(gain1),
        Signal::Node(gain2),
    ]);

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&mix_id, &mut output);

    // Verify audio is valid
    assert!(is_valid_audio(&output), "Output should contain valid audio");

    let rms = calculate_rms(&output);

    // Should have energy
    assert!(rms > 0.1, "Should have audio energy, got {}", rms);
}

#[test]
fn test_mix_no_clipping() {
    let mut graph = create_test_graph();

    // Mix many loud signals - normalization should prevent clipping
    let mut signals = Vec::new();
    for i in 0..20 {
        let freq = 220.0 * (i + 1) as f32;
        let osc = graph.add_oscillator(Signal::Value(freq), Waveform::Sine);
        signals.push(Signal::Node(osc));
    }

    let mix_id = graph.add_mix_node(signals);

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&mix_id, &mut output);

    // Verify audio is valid
    assert!(is_valid_audio(&output), "Output should contain valid audio");

    let peak = calculate_peak(&output);

    // With normalization, peak should stay reasonable (well below 1.0 for many signals)
    assert!(peak < 1.0, "Peak should not clip, got {}", peak);
}

#[test]
fn test_mix_zero_and_nonzero() {
    let mut graph = create_test_graph();

    // Mix silence with oscillator
    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    let mix_id = graph.add_mix_node(vec![
        Signal::Value(0.0),
        Signal::Node(osc),
    ]);

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&mix_id, &mut output);

    // Verify audio is valid
    assert!(is_valid_audio(&output), "Output should contain valid audio");

    let rms = calculate_rms(&output);

    // Sine wave RMS ≈ 0.707, normalized by 2: 0.707/2 ≈ 0.35
    assert!(rms > 0.3 && rms < 0.4, "RMS should be around 0.35, got {}", rms);
}

#[test]
fn test_mix_multiple_buffers() {
    let mut graph = create_test_graph();

    // Create mix
    let osc1 = graph.add_oscillator(Signal::Value(220.0), Waveform::Sine);
    let osc2 = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    let mix_id = graph.add_mix_node(vec![
        Signal::Node(osc1),
        Signal::Node(osc2),
    ]);

    let buffer_size = 512;

    // Render multiple buffers to check consistency
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&mix_id, &mut buffer1);
    graph.eval_node_buffer(&mix_id, &mut buffer2);

    let rms1 = calculate_rms(&buffer1);
    let rms2 = calculate_rms(&buffer2);

    // Both buffers should have similar energy
    assert!((rms1 - rms2).abs() < 0.1, "RMS should be consistent across buffers");
}

#[test]
fn test_mix_buffer_size_independence() {
    let mut graph = create_test_graph();

    // Create mix
    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    let mix_id = graph.add_mix_node(vec![
        Signal::Value(0.5),
        Signal::Node(osc),
    ]);

    // Test different buffer sizes
    for size in [64, 128, 256, 512, 1024, 2048] {
        let mut output = vec![0.0; size];
        graph.eval_node_buffer(&mix_id, &mut output);

        assert!(is_valid_audio(&output), "Should work with buffer size {}", size);

        let rms = calculate_rms(&output);
        assert!(rms > 0.2 && rms < 0.5, "RMS reasonable for size {}, got {}", size, rms);
    }
}
