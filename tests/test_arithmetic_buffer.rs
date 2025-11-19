/// Tests for arithmetic node buffer evaluation
///
/// These tests verify that Add and Multiply nodes produce correct
/// results when evaluated in buffer mode.

use phonon::unified_graph::{Signal, UnifiedSignalGraph, Waveform};
use std::f32::consts::PI;

/// Helper: Create a test graph
fn create_test_graph() -> UnifiedSignalGraph {
    UnifiedSignalGraph::new(44100.0)
}

/// Helper: Calculate RMS of a buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

// ============================================================================
// TEST: Add Node - Constants
// ============================================================================

#[test]
fn test_add_two_constants() {
    let mut graph = create_test_graph();

    let add_id = graph.add_add_node(
        Signal::Value(0.3),
        Signal::Value(0.4),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&add_id, &mut output);

    // All samples should equal 0.7
    for &sample in &output {
        assert!((sample - 0.7_f32).abs() < 1e-6,
            "Expected 0.7, got {}", sample);
    }
}

#[test]
fn test_add_positive_and_negative() {
    let mut graph = create_test_graph();

    let add_id = graph.add_add_node(
        Signal::Value(1.0),
        Signal::Value(-0.4),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&add_id, &mut output);

    // All samples should equal 0.6
    for &sample in &output {
        assert!((sample - 0.6_f32).abs() < 1e-6,
            "Expected 0.6, got {}", sample);
    }
}

// ============================================================================
// TEST: Add Node - Signal + Constant
// ============================================================================

#[test]
fn test_add_oscillator_and_constant() {
    let mut graph = create_test_graph();

    // Create sine oscillator at 440 Hz
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Add 0.5 to oscillator (DC offset)
    let add_id = graph.add_add_node(
        Signal::Node(osc_id),
        Signal::Value(0.5),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&add_id, &mut output);

    // Sine wave ranges [-1, 1], adding 0.5 gives [-0.5, 1.5]
    let min = output.iter().fold(f32::INFINITY, |a, &b| a.min(b));
    let max = output.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));

    assert!(min > -0.6 && min < -0.4,
        "Min should be ~-0.5, got {}", min);
    assert!(max > 1.4 && max < 1.6,
        "Max should be ~1.5, got {}", max);
}

// ============================================================================
// TEST: Add Node - Two Oscillators
// ============================================================================

#[test]
fn test_add_two_oscillators() {
    let mut graph = create_test_graph();

    // Create two sine oscillators at different frequencies
    let osc1_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let osc2_id = graph.add_oscillator(Signal::Value(880.0), Waveform::Sine);

    // Add them together
    let add_id = graph.add_add_node(
        Signal::Node(osc1_id),
        Signal::Node(osc2_id),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&add_id, &mut output);

    // Sum of two sine waves can range from -2 to 2
    let max_amplitude = output.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    assert!(max_amplitude > 0.5 && max_amplitude <= 2.0,
        "Max amplitude should be between 0.5 and 2.0, got {}", max_amplitude);

    // RMS should be higher than single oscillator
    let rms = calculate_rms(&output);
    assert!(rms > 0.7,
        "RMS of two added oscillators should be > 0.7, got {}", rms);
}

// ============================================================================
// TEST: Multiply Node - Constants
// ============================================================================

#[test]
fn test_multiply_two_constants() {
    let mut graph = create_test_graph();

    let mul_id = graph.add_multiply_node(
        Signal::Value(0.5),
        Signal::Value(0.8),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&mul_id, &mut output);

    // All samples should equal 0.4
    for &sample in &output {
        assert!((sample - 0.4_f32).abs() < 1e-6,
            "Expected 0.4, got {}", sample);
    }
}

#[test]
fn test_multiply_by_negative() {
    let mut graph = create_test_graph();

    let mul_id = graph.add_multiply_node(
        Signal::Value(0.5),
        Signal::Value(-2.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&mul_id, &mut output);

    // All samples should equal -1.0
    for &sample in &output {
        assert!((sample - (-1.0_f32)).abs() < 1e-6,
            "Expected -1.0, got {}", sample);
    }
}

#[test]
fn test_multiply_by_zero() {
    let mut graph = create_test_graph();

    let mul_id = graph.add_multiply_node(
        Signal::Value(0.5),
        Signal::Value(0.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&mul_id, &mut output);

    // All samples should equal 0.0
    for &sample in &output {
        assert!(sample.abs() < 1e-6,
            "Expected 0.0, got {}", sample);
    }
}

// ============================================================================
// TEST: Multiply Node - Signal Scaling
// ============================================================================

#[test]
fn test_multiply_oscillator_scale_down() {
    let mut graph = create_test_graph();

    // Create sine oscillator at 440 Hz
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Scale by 0.5 (amplitude reduction)
    let mul_id = graph.add_multiply_node(
        Signal::Node(osc_id),
        Signal::Value(0.5),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&mul_id, &mut output);

    // Sine wave amplitude should be ~0.5
    let max_amplitude = output.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    assert!(max_amplitude > 0.45 && max_amplitude <= 0.55,
        "Max amplitude should be ~0.5, got {}", max_amplitude);

    // RMS should be ~0.5 * 0.707 = 0.3535
    let rms = calculate_rms(&output);
    assert!(rms > 0.3 && rms < 0.4,
        "RMS should be ~0.35, got {}", rms);
}

#[test]
fn test_multiply_oscillator_scale_up() {
    let mut graph = create_test_graph();

    // Create sine oscillator at 440 Hz
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Scale by 2.0 (amplitude increase)
    let mul_id = graph.add_multiply_node(
        Signal::Node(osc_id),
        Signal::Value(2.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&mul_id, &mut output);

    // Sine wave amplitude should be ~2.0
    let max_amplitude = output.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    assert!(max_amplitude > 1.9 && max_amplitude <= 2.1,
        "Max amplitude should be ~2.0, got {}", max_amplitude);

    // RMS should be ~2.0 * 0.707 = 1.414
    let rms = calculate_rms(&output);
    assert!(rms > 1.3 && rms < 1.5,
        "RMS should be ~1.4, got {}", rms);
}

// ============================================================================
// TEST: Multiply Node - Ring Modulation
// ============================================================================

#[test]
fn test_multiply_two_oscillators_ring_mod() {
    let mut graph = create_test_graph();

    // Create two sine oscillators
    let osc1_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let osc2_id = graph.add_oscillator(Signal::Value(100.0), Waveform::Sine);

    // Multiply them (ring modulation)
    let mul_id = graph.add_multiply_node(
        Signal::Node(osc1_id),
        Signal::Node(osc2_id),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&mul_id, &mut output);

    // Ring modulation of two sine waves
    // Maximum amplitude is 1.0 (when both are at peak)
    let max_amplitude = output.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    assert!(max_amplitude > 0.3 && max_amplitude <= 1.0,
        "Max amplitude should be between 0.3 and 1.0, got {}", max_amplitude);

    // Should have sound (not silent)
    let rms = calculate_rms(&output);
    assert!(rms > 0.1,
        "RMS should be > 0.1 for ring modulation, got {}", rms);
}

// ============================================================================
// TEST: Complex Combinations
// ============================================================================

#[test]
fn test_add_then_multiply() {
    let mut graph = create_test_graph();

    // (0.5 + 0.3) * 2.0 = 0.8 * 2.0 = 1.6
    let add_id = graph.add_add_node(
        Signal::Value(0.5),
        Signal::Value(0.3),
    );

    let mul_id = graph.add_multiply_node(
        Signal::Node(add_id),
        Signal::Value(2.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&mul_id, &mut output);

    for &sample in &output {
        assert!((sample - 1.6_f32).abs() < 1e-5,
            "Expected 1.6, got {}", sample);
    }
}

#[test]
fn test_multiply_then_add() {
    let mut graph = create_test_graph();

    // (0.5 * 0.4) + 0.1 = 0.2 + 0.1 = 0.3
    let mul_id = graph.add_multiply_node(
        Signal::Value(0.5),
        Signal::Value(0.4),
    );

    let add_id = graph.add_add_node(
        Signal::Node(mul_id),
        Signal::Value(0.1),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&add_id, &mut output);

    for &sample in &output {
        assert!((sample - 0.3_f32).abs() < 1e-5,
            "Expected 0.3, got {}", sample);
    }
}

#[test]
fn test_oscillator_with_add_and_multiply() {
    let mut graph = create_test_graph();

    // Create oscillator
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Scale by 0.5
    let mul_id = graph.add_multiply_node(
        Signal::Node(osc_id),
        Signal::Value(0.5),
    );

    // Add DC offset of 0.5 (should range [0, 1])
    let add_id = graph.add_add_node(
        Signal::Node(mul_id),
        Signal::Value(0.5),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&add_id, &mut output);

    // Sine scaled to [-0.5, 0.5] then offset to [0, 1]
    let min = output.iter().fold(f32::INFINITY, |a, &b| a.min(b));
    let max = output.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));

    assert!(min >= -0.1 && min <= 0.1,
        "Min should be ~0, got {}", min);
    assert!(max >= 0.9 && max <= 1.1,
        "Max should be ~1, got {}", max);
}

// ============================================================================
// TEST: Multiple Buffer Evaluation (State Persistence)
// ============================================================================

#[test]
fn test_add_oscillators_multiple_buffers() {
    let mut graph = create_test_graph();

    let osc1_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let osc2_id = graph.add_oscillator(Signal::Value(880.0), Waveform::Sine);

    let add_id = graph.add_add_node(
        Signal::Node(osc1_id),
        Signal::Node(osc2_id),
    );

    // Generate 5 consecutive buffers
    let buffer_size = 512;
    for i in 0..5 {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&add_id, &mut output);

        // Each buffer should have reasonable audio
        let rms = calculate_rms(&output);
        assert!(rms > 0.7,
            "Buffer {} has unexpected RMS: {}", i, rms);
    }
}

#[test]
fn test_multiply_oscillators_multiple_buffers() {
    let mut graph = create_test_graph();

    let osc1_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let osc2_id = graph.add_oscillator(Signal::Value(100.0), Waveform::Sine);

    let mul_id = graph.add_multiply_node(
        Signal::Node(osc1_id),
        Signal::Node(osc2_id),
    );

    // Generate 5 consecutive buffers
    let buffer_size = 512;
    for i in 0..5 {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&mul_id, &mut output);

        // Each buffer should have reasonable audio
        let rms = calculate_rms(&output);
        assert!(rms > 0.1,
            "Buffer {} has unexpected RMS: {}", i, rms);
    }
}

// ============================================================================
// TEST: Edge Cases
// ============================================================================

#[test]
fn test_add_large_values() {
    let mut graph = create_test_graph();

    let add_id = graph.add_add_node(
        Signal::Value(1000.0),
        Signal::Value(2000.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&add_id, &mut output);

    for &sample in &output {
        assert!((sample - 3000.0_f32).abs() < 0.01,
            "Expected 3000.0, got {}", sample);
    }
}

#[test]
fn test_multiply_large_values() {
    let mut graph = create_test_graph();

    let mul_id = graph.add_multiply_node(
        Signal::Value(100.0),
        Signal::Value(50.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&mul_id, &mut output);

    for &sample in &output {
        assert!((sample - 5000.0_f32).abs() < 0.01,
            "Expected 5000.0, got {}", sample);
    }
}

#[test]
fn test_add_very_small_values() {
    let mut graph = create_test_graph();

    let add_id = graph.add_add_node(
        Signal::Value(0.0001),
        Signal::Value(0.0002),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&add_id, &mut output);

    for &sample in &output {
        assert!((sample - 0.0003_f32).abs() < 1e-7,
            "Expected 0.0003, got {}", sample);
    }
}

#[test]
fn test_multiply_very_small_values() {
    let mut graph = create_test_graph();

    let mul_id = graph.add_multiply_node(
        Signal::Value(0.0001),
        Signal::Value(0.0002),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&mul_id, &mut output);

    for &sample in &output {
        assert!((sample - 0.00000002_f32).abs() < 1e-10,
            "Expected 0.00000002, got {}", sample);
    }
}

// ============================================================================
// TEST: Buffer Size Variations
// ============================================================================

#[test]
fn test_arithmetic_various_buffer_sizes() {
    let mut graph = create_test_graph();

    let add_id = graph.add_add_node(
        Signal::Value(0.3),
        Signal::Value(0.4),
    );

    let mul_id = graph.add_multiply_node(
        Signal::Value(0.5),
        Signal::Value(0.8),
    );

    // Test different buffer sizes
    for size in [1, 16, 64, 128, 256, 512, 1024, 2048] {
        let mut add_output = vec![0.0; size];
        graph.eval_node_buffer(&add_id, &mut add_output);

        for &sample in &add_output {
            assert!((sample - 0.7_f32).abs() < 1e-6,
                "Add failed for buffer size {}", size);
        }

        let mut mul_output = vec![0.0; size];
        graph.eval_node_buffer(&mul_id, &mut mul_output);

        for &sample in &mul_output {
            assert!((sample - 0.4_f32).abs() < 1e-6,
                "Multiply failed for buffer size {}", size);
        }
    }
}

// ============================================================================
// TEST: Performance
// ============================================================================

#[test]
fn test_arithmetic_buffer_performance() {
    let mut graph = create_test_graph();

    // Create complex chain: osc1 + osc2, then multiply by constant
    let osc1_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let osc2_id = graph.add_oscillator(Signal::Value(880.0), Waveform::Sine);

    let add_id = graph.add_add_node(
        Signal::Node(osc1_id),
        Signal::Node(osc2_id),
    );

    let mul_id = graph.add_multiply_node(
        Signal::Node(add_id),
        Signal::Value(0.5),
    );

    let buffer_size = 512;
    let iterations = 1000;

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&mul_id, &mut output);
    }
    let duration = start.elapsed();

    println!("Arithmetic buffer eval: {:?} for {} iterations", duration, iterations);
    println!("Per iteration: {:?}", duration / iterations);

    // Should complete in reasonable time (< 1 second for 1000 iterations)
    assert!(duration.as_secs() < 1,
        "Arithmetic buffer evaluation too slow: {:?}", duration);
}
