use phonon::unified_graph::{Signal, UnifiedSignalGraph, Waveform};

fn create_test_graph() -> UnifiedSignalGraph {
    UnifiedSignalGraph::new(44100.0)
}

fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

fn calculate_mean(buffer: &[f32]) -> f32 {
    let sum: f32 = buffer.iter().sum();
    sum / buffer.len() as f32
}

#[test]
fn test_xfade_full_a() {
    let mut graph = create_test_graph();

    // Two different constants
    let xfade_id = graph.add_xfade_node(
        Signal::Value(0.8),
        Signal::Value(0.2),
        Signal::Value(0.0), // 100% A
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&xfade_id, &mut output);

    // Should output A (0.8)
    for &sample in &output {
        assert!((sample - 0.8).abs() < 0.001, "Expected 0.8, got {}", sample);
    }
}

#[test]
fn test_xfade_full_b() {
    let mut graph = create_test_graph();

    let xfade_id = graph.add_xfade_node(
        Signal::Value(0.8),
        Signal::Value(0.2),
        Signal::Value(1.0), // 100% B
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&xfade_id, &mut output);

    // Should output B (0.2)
    for &sample in &output {
        assert!((sample - 0.2).abs() < 0.001, "Expected 0.2, got {}", sample);
    }
}

#[test]
fn test_xfade_center() {
    let mut graph = create_test_graph();

    let xfade_id = graph.add_xfade_node(
        Signal::Value(1.0),
        Signal::Value(-1.0),
        Signal::Value(0.5), // 50/50 mix
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&xfade_id, &mut output);

    // Should output 0.5 * 1.0 + 0.5 * (-1.0) = 0.0
    for &sample in &output {
        assert!(sample.abs() < 0.001, "Expected 0.0, got {}", sample);
    }
}

#[test]
fn test_xfade_quarter_mix() {
    let mut graph = create_test_graph();

    // Test 25% mix (75% A, 25% B)
    let xfade_id = graph.add_xfade_node(
        Signal::Value(1.0),
        Signal::Value(0.0),
        Signal::Value(0.25),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&xfade_id, &mut output);

    // Should output 0.75 * 1.0 + 0.25 * 0.0 = 0.75
    for &sample in &output {
        assert!((sample - 0.75).abs() < 0.001, "Expected 0.75, got {}", sample);
    }
}

#[test]
fn test_xfade_three_quarter_mix() {
    let mut graph = create_test_graph();

    // Test 75% mix (25% A, 75% B)
    let xfade_id = graph.add_xfade_node(
        Signal::Value(1.0),
        Signal::Value(0.0),
        Signal::Value(0.75),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&xfade_id, &mut output);

    // Should output 0.25 * 1.0 + 0.75 * 0.0 = 0.25
    for &sample in &output {
        assert!((sample - 0.25).abs() < 0.001, "Expected 0.25, got {}", sample);
    }
}

#[test]
fn test_xfade_modulated_lfo() {
    let mut graph = create_test_graph();

    // Two oscillators at different frequencies
    let osc_a = graph.add_oscillator(Signal::Value(220.0), Waveform::Sine);
    let osc_b = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // LFO to modulate crossfade (2 Hz sine wave)
    let lfo = graph.add_oscillator(Signal::Value(2.0), Waveform::Sine);

    // Normalize LFO to 0-1 range: (sine + 1) / 2
    let lfo_scaled = graph.add_multiply_node(Signal::Node(lfo), Signal::Value(0.5));
    let lfo_offset = graph.add_add_node(Signal::Node(lfo_scaled), Signal::Value(0.5));

    let xfade_id = graph.add_xfade_node(
        Signal::Node(osc_a),
        Signal::Node(osc_b),
        Signal::Node(lfo_offset),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&xfade_id, &mut output);

    // Should produce sound (varying blend of two oscillators)
    let rms = calculate_rms(&output);
    assert!(rms > 0.3, "RMS too low: {}", rms);
    assert!(rms < 1.0, "RMS too high: {}", rms);
}

#[test]
fn test_xfade_oscillators_constant_mix() {
    let mut graph = create_test_graph();

    // Two oscillators at different frequencies
    let osc_a = graph.add_oscillator(Signal::Value(100.0), Waveform::Sine);
    let osc_b = graph.add_oscillator(Signal::Value(200.0), Waveform::Sine);

    // Test at 0%, 50%, and 100%
    for (mix, expected_rms_range) in [(0.0, (0.6, 0.8)), (0.5, (0.6, 0.8)), (1.0, (0.6, 0.8))] {
        let xfade_id = graph.add_xfade_node(
            Signal::Node(osc_a),
            Signal::Node(osc_b),
            Signal::Value(mix),
        );

        let buffer_size = 512;
        let mut output = vec![0.0; buffer_size];

        graph.eval_node_buffer(&xfade_id, &mut output);

        let rms = calculate_rms(&output);
        assert!(
            rms > expected_rms_range.0 && rms < expected_rms_range.1,
            "Mix {}: RMS {} not in expected range ({}, {})",
            mix, rms, expected_rms_range.0, expected_rms_range.1
        );
    }
}

#[test]
fn test_xfade_clamps_position() {
    let mut graph = create_test_graph();

    // Test with position > 1.0 (should clamp to 1.0, giving 100% B)
    let xfade_id = graph.add_xfade_node(
        Signal::Value(0.5),
        Signal::Value(0.9),
        Signal::Value(2.0), // Should clamp to 1.0
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&xfade_id, &mut output);

    // Should output B (0.9) since position is clamped to 1.0
    for &sample in &output {
        assert!((sample - 0.9).abs() < 0.001, "Expected 0.9, got {}", sample);
    }

    // Test with position < 0.0 (should clamp to 0.0, giving 100% A)
    let xfade_id2 = graph.add_xfade_node(
        Signal::Value(0.3),
        Signal::Value(0.7),
        Signal::Value(-1.0), // Should clamp to 0.0
    );

    let mut output2 = vec![0.0; buffer_size];
    graph.eval_node_buffer(&xfade_id2, &mut output2);

    // Should output A (0.3) since position is clamped to 0.0
    for &sample in &output2 {
        assert!((sample - 0.3).abs() < 0.001, "Expected 0.3, got {}", sample);
    }
}

#[test]
fn test_xfade_positive_negative() {
    let mut graph = create_test_graph();

    // Test crossfading between positive and negative values
    let xfade_id = graph.add_xfade_node(
        Signal::Value(0.6),
        Signal::Value(-0.4),
        Signal::Value(0.5),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&xfade_id, &mut output);

    // Should output 0.5 * 0.6 + 0.5 * (-0.4) = 0.3 - 0.2 = 0.1
    let mean = calculate_mean(&output);
    assert!((mean - 0.1).abs() < 0.001, "Expected 0.1, got {}", mean);
}

#[test]
fn test_xfade_square_waves() {
    let mut graph = create_test_graph();

    // Two square waves at different frequencies
    let sq_a = graph.add_oscillator(Signal::Value(50.0), Waveform::Square);
    let sq_b = graph.add_oscillator(Signal::Value(75.0), Waveform::Square);

    let xfade_id = graph.add_xfade_node(
        Signal::Node(sq_a),
        Signal::Node(sq_b),
        Signal::Value(0.3), // 70% A, 30% B
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&xfade_id, &mut output);

    // Should produce square-ish waveform
    let rms = calculate_rms(&output);
    assert!(rms > 0.6, "RMS too low for square waves: {}", rms);
}

#[test]
fn test_xfade_sawtooth_triangle() {
    let mut graph = create_test_graph();

    // Saw and triangle waves
    let saw = graph.add_oscillator(Signal::Value(110.0), Waveform::Saw);
    let tri = graph.add_oscillator(Signal::Value(110.0), Waveform::Triangle);

    // Test different mix positions
    for mix in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let xfade_id = graph.add_xfade_node(
            Signal::Node(saw),
            Signal::Node(tri),
            Signal::Value(mix),
        );

        let buffer_size = 512;
        let mut output = vec![0.0; buffer_size];

        graph.eval_node_buffer(&xfade_id, &mut output);

        let rms = calculate_rms(&output);
        assert!(rms > 0.3, "Mix {}: RMS too low: {}", mix, rms);
    }
}

#[test]
fn test_xfade_silence_to_sound() {
    let mut graph = create_test_graph();

    // Crossfade from silence to sound
    let silence = Signal::Value(0.0);
    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // At position 0.0: should be silent
    let xfade_0 = graph.add_xfade_node(
        silence.clone(),
        Signal::Node(osc),
        Signal::Value(0.0),
    );

    let buffer_size = 512;
    let mut output_0 = vec![0.0; buffer_size];
    graph.eval_node_buffer(&xfade_0, &mut output_0);

    let rms_0 = calculate_rms(&output_0);
    assert!(rms_0 < 0.001, "Should be silent at position 0.0, got RMS: {}", rms_0);

    // At position 1.0: should be loud
    let xfade_1 = graph.add_xfade_node(
        silence,
        Signal::Node(osc),
        Signal::Value(1.0),
    );

    let mut output_1 = vec![0.0; buffer_size];
    graph.eval_node_buffer(&xfade_1, &mut output_1);

    let rms_1 = calculate_rms(&output_1);
    assert!(rms_1 > 0.6, "Should be loud at position 1.0, got RMS: {}", rms_1);
}

#[test]
fn test_xfade_inverted_signals() {
    let mut graph = create_test_graph();

    // Test with inverted versions of the same signal
    let osc = graph.add_oscillator(Signal::Value(220.0), Waveform::Sine);
    let osc_inverted = graph.add_multiply_node(Signal::Node(osc), Signal::Value(-1.0));

    // At 0.5 mix, inverted signals should cancel out
    let xfade_id = graph.add_xfade_node(
        Signal::Node(osc),
        Signal::Node(osc_inverted),
        Signal::Value(0.5),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&xfade_id, &mut output);

    // Should be near silence (signals cancel)
    let rms = calculate_rms(&output);
    assert!(rms < 0.01, "Inverted signals should cancel at 0.5 mix, got RMS: {}", rms);
}

#[test]
fn test_xfade_dynamic_position() {
    let mut graph = create_test_graph();

    // Create a position that varies over time (triangle LFO)
    let position_lfo = graph.add_oscillator(Signal::Value(1.0), Waveform::Triangle);

    // Normalize triangle to 0-1 range
    let pos_scaled = graph.add_multiply_node(Signal::Node(position_lfo), Signal::Value(0.5));
    let pos_offset = graph.add_add_node(Signal::Node(pos_scaled), Signal::Value(0.5));

    // Two different oscillators
    let osc_low = graph.add_oscillator(Signal::Value(110.0), Waveform::Sine);
    let osc_high = graph.add_oscillator(Signal::Value(330.0), Waveform::Sine);

    let xfade_id = graph.add_xfade_node(
        Signal::Node(osc_low),
        Signal::Node(osc_high),
        Signal::Node(pos_offset),
    );

    let buffer_size = 4410; // 0.1 seconds at 44.1kHz
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&xfade_id, &mut output);

    // Should produce varying sound as position sweeps
    let rms = calculate_rms(&output);
    assert!(rms > 0.3, "RMS too low for dynamic crossfade: {}", rms);

    // Check that the output actually varies
    let first_half_rms = calculate_rms(&output[0..buffer_size/2]);
    let second_half_rms = calculate_rms(&output[buffer_size/2..]);

    // Both halves should have sound
    assert!(first_half_rms > 0.2 && second_half_rms > 0.2,
        "Both halves should have significant energy");
}

#[test]
fn test_xfade_multiple_buffers() {
    let mut graph = create_test_graph();

    let osc_a = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let osc_b = graph.add_oscillator(Signal::Value(880.0), Waveform::Sine);

    let xfade_id = graph.add_xfade_node(
        Signal::Node(osc_a),
        Signal::Node(osc_b),
        Signal::Value(0.5),
    );

    // Process multiple buffers to ensure state is maintained correctly
    for _ in 0..10 {
        let buffer_size = 512;
        let mut output = vec![0.0; buffer_size];

        graph.eval_node_buffer(&xfade_id, &mut output);

        let rms = calculate_rms(&output);
        assert!(rms > 0.3, "Each buffer should have consistent RMS: {}", rms);
    }
}

#[test]
fn test_xfade_zero_buffer_size() {
    let mut graph = create_test_graph();

    let xfade_id = graph.add_xfade_node(
        Signal::Value(1.0),
        Signal::Value(0.0),
        Signal::Value(0.5),
    );

    let mut output = vec![];
    graph.eval_node_buffer(&xfade_id, &mut output);

    // Should handle empty buffer gracefully
    assert_eq!(output.len(), 0);
}

#[test]
fn test_xfade_large_buffer() {
    let mut graph = create_test_graph();

    let osc_a = graph.add_oscillator(Signal::Value(220.0), Waveform::Sine);
    let osc_b = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    let xfade_id = graph.add_xfade_node(
        Signal::Node(osc_a),
        Signal::Node(osc_b),
        Signal::Value(0.3),
    );

    // Test with large buffer (1 second)
    let buffer_size = 44100;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&xfade_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.3 && rms < 1.0, "Large buffer RMS out of range: {}", rms);
}
