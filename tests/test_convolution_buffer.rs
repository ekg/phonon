/// Tests for Convolution buffer-based evaluation
///
/// These tests verify that Convolution buffer evaluation produces correct
/// realistic reverb using impulse response convolution.

use phonon::unified_graph::{Signal, UnifiedSignalGraph, Waveform};

/// Helper: Create a test graph
fn create_test_graph() -> UnifiedSignalGraph {
    UnifiedSignalGraph::new(44100.0)
}

/// Helper: Calculate RMS of a buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Helper: Calculate peak absolute value
fn calculate_peak(buffer: &[f32]) -> f32 {
    buffer.iter().map(|&x| x.abs()).fold(0.0, f32::max)
}

/// Helper: Measure energy in second half vs first half (for decay analysis)
fn measure_decay_ratio(buffer: &[f32]) -> f32 {
    let mid = buffer.len() / 2;
    let first_half = &buffer[0..mid];
    let second_half = &buffer[mid..];

    let first_rms = calculate_rms(first_half);
    let second_rms = calculate_rms(second_half);

    if first_rms > 0.0 {
        second_rms / first_rms
    } else {
        0.0
    }
}

// ============================================================================
// TEST 1: Basic Convolution Produces Sound
// ============================================================================

#[test]
fn test_convolution_produces_sound() {
    let mut graph = create_test_graph();

    // Create sine wave oscillator
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Apply convolution
    let conv_id = graph.add_convolution_node(Signal::Node(osc_id));

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&conv_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(
        rms > 0.01,
        "Convolution should produce audible output, got RMS={}",
        rms
    );

    // Check for NaN or Inf
    for (i, &sample) in output.iter().enumerate() {
        assert!(
            sample.is_finite(),
            "Sample {} should be finite: got {}",
            i,
            sample
        );
    }
}

// ============================================================================
// TEST 2: Convolution Creates Reverb Effect
// ============================================================================

#[test]
fn test_convolution_creates_reverb() {
    let mut graph = create_test_graph();

    // Create sine wave oscillator
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Get clean and convolved signals
    let clean_id = osc_id;
    let conv_id = graph.add_convolution_node(Signal::Node(osc_id));

    let buffer_size = 1024;
    let mut clean = vec![0.0; buffer_size];
    let mut convolved = vec![0.0; buffer_size];

    graph.eval_node_buffer(&clean_id, &mut clean);
    graph.eval_node_buffer(&conv_id, &mut convolved);

    // Signals should be different (convolution adds reflections)
    let mut differences = 0;
    for i in 0..buffer_size {
        if (clean[i] - convolved[i]).abs() > 0.01 {
            differences += 1;
        }
    }

    assert!(
        differences > buffer_size / 4,
        "Convolution should produce different signal from clean: only {} of {} samples differ",
        differences,
        buffer_size
    );

    // Both should have audible energy
    let clean_rms = calculate_rms(&clean);
    let conv_rms = calculate_rms(&convolved);

    assert!(clean_rms > 0.1, "Clean signal should be audible: RMS = {}", clean_rms);
    assert!(conv_rms > 0.01, "Convolved signal should be audible: RMS = {}", conv_rms);
}

// ============================================================================
// TEST 3: Convolution with Different Waveforms
// ============================================================================

#[test]
fn test_convolution_with_saw_wave() {
    let mut graph = create_test_graph();

    // Create saw wave oscillator
    let osc_id = graph.add_oscillator(Signal::Value(220.0), Waveform::Saw);

    // Apply convolution
    let conv_id = graph.add_convolution_node(Signal::Node(osc_id));

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&conv_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(
        rms > 0.01,
        "Convolution with saw wave should produce sound, got RMS={}",
        rms
    );
}

#[test]
fn test_convolution_with_square_wave() {
    let mut graph = create_test_graph();

    // Create square wave oscillator
    let osc_id = graph.add_oscillator(Signal::Value(330.0), Waveform::Square);

    // Apply convolution
    let conv_id = graph.add_convolution_node(Signal::Node(osc_id));

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&conv_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(
        rms > 0.01,
        "Convolution with square wave should produce sound, got RMS={}",
        rms
    );
}

// ============================================================================
// TEST 4: State Continuity Across Multiple Buffers
// ============================================================================

#[test]
fn test_convolution_state_continuity() {
    let mut graph = create_test_graph();

    // Create sine wave oscillator
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Apply convolution
    let conv_id = graph.add_convolution_node(Signal::Node(osc_id));

    let buffer_size = 512;
    let num_buffers = 4;
    let mut buffers = vec![vec![0.0; buffer_size]; num_buffers];

    // Render multiple buffers
    for i in 0..num_buffers {
        graph.eval_node_buffer(&conv_id, &mut buffers[i]);
    }

    // Each buffer should have audible output
    for (i, buffer) in buffers.iter().enumerate() {
        let rms = calculate_rms(buffer);
        assert!(
            rms > 0.01,
            "Buffer {} should have audible output: RMS = {}",
            i,
            rms
        );

        // Check for NaN or Inf
        for (j, &sample) in buffer.iter().enumerate() {
            assert!(
                sample.is_finite(),
                "Buffer {} sample {} should be finite: got {}",
                i,
                j,
                sample
            );
        }
    }

    // RMS values should be consistent across buffers (since oscillator is continuous)
    // Note: First few buffers may have lower energy as convolution state builds up
    let rms_values: Vec<f32> = buffers.iter().map(|b| calculate_rms(b)).collect();

    // Skip first 2 buffers (warm-up period) and check consistency of remaining buffers
    let steady_state_rms: Vec<f32> = rms_values.iter().skip(2).copied().collect();
    let avg_rms = steady_state_rms.iter().sum::<f32>() / steady_state_rms.len() as f32;

    // Use larger tolerance due to convolution's dynamic nature
    for (i, &rms) in steady_state_rms.iter().enumerate() {
        assert!(
            (rms - avg_rms).abs() < 0.2,
            "Buffer {} (after warm-up) RMS should be consistent: got {}, avg {}",
            i + 2,
            rms,
            avg_rms
        );
    }
}

// ============================================================================
// TEST 5: Convolution Adds Spatial Depth
// ============================================================================

#[test]
fn test_convolution_adds_depth() {
    let mut graph = create_test_graph();

    // Create sine wave oscillator
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Apply convolution
    let conv_id = graph.add_convolution_node(Signal::Node(osc_id));

    // Use longer buffer to capture reverb tail
    let buffer_size = 4410; // 0.1 seconds at 44.1kHz
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&conv_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(
        rms > 0.01,
        "Convolution should produce audible output with depth, got RMS={}",
        rms
    );

    // With convolution, the signal should have some decay characteristics
    // (though with continuous sine input, it's steady-state)
    // Just verify we have consistent energy
    let decay_ratio = measure_decay_ratio(&output);
    assert!(
        decay_ratio > 0.5,
        "With continuous input, convolution should maintain energy: decay_ratio = {}",
        decay_ratio
    );
}

// ============================================================================
// TEST 6: Multiple Buffers Stress Test
// ============================================================================

#[test]
fn test_convolution_multiple_buffers_stress() {
    let mut graph = create_test_graph();

    // Create sine wave oscillator
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Apply convolution
    let conv_id = graph.add_convolution_node(Signal::Node(osc_id));

    let buffer_size = 256;
    let num_buffers = 16;
    let mut buffers = vec![vec![0.0; buffer_size]; num_buffers];

    // Render many buffers
    for i in 0..num_buffers {
        graph.eval_node_buffer(&conv_id, &mut buffers[i]);
    }

    // All buffers should produce valid audio
    for (i, buffer) in buffers.iter().enumerate() {
        let rms = calculate_rms(buffer);
        assert!(
            rms > 0.01,
            "Buffer {} should have audible output: RMS = {}",
            i,
            rms
        );

        // Check for NaN or Inf
        for (j, &sample) in buffer.iter().enumerate() {
            assert!(
                sample.is_finite(),
                "Buffer {} sample {} should be finite: got {}",
                i,
                j,
                sample
            );
        }
    }
}

// ============================================================================
// TEST 7: Zero Input Produces Zero Output
// ============================================================================

#[test]
fn test_convolution_zero_input() {
    let mut graph = create_test_graph();

    // Convolution with zero input (constant 0.0)
    let conv_id = graph.add_convolution_node(Signal::Value(0.0));

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&conv_id, &mut output);

    // Output should be very close to zero
    let rms = calculate_rms(&output);
    assert!(
        rms < 0.0001,
        "Zero input should produce near-zero output: RMS = {}",
        rms
    );
}

// ============================================================================
// TEST 8: Large Buffer Performance
// ============================================================================

#[test]
fn test_convolution_large_buffer() {
    let mut graph = create_test_graph();

    // Create sine wave oscillator
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Apply convolution
    let conv_id = graph.add_convolution_node(Signal::Node(osc_id));

    // Process a large buffer (1 second at 44.1kHz)
    let buffer_size = 44100;
    let mut output = vec![0.0; buffer_size];

    // This should complete without issues
    graph.eval_node_buffer(&conv_id, &mut output);

    // Verify output is valid
    let rms = calculate_rms(&output);
    assert!(
        rms > 0.1,
        "Large buffer should produce audible output: RMS = {}",
        rms
    );
    assert!(
        rms < 2.0,
        "Large buffer output should be reasonable: RMS = {}",
        rms
    );

    // Check for NaN or Inf
    for (i, &sample) in output.iter().enumerate() {
        assert!(
            sample.is_finite(),
            "Sample {} should be finite: got {}",
            i,
            sample
        );
    }
}

// ============================================================================
// TEST 9: Convolution with Different Frequencies
// ============================================================================

#[test]
fn test_convolution_different_frequencies() {
    let mut graph = create_test_graph();

    // Test with low frequency
    let low_osc_id = graph.add_oscillator(Signal::Value(110.0), Waveform::Sine);
    let low_conv_id = graph.add_convolution_node(Signal::Node(low_osc_id));

    // Test with high frequency
    let high_osc_id = graph.add_oscillator(Signal::Value(880.0), Waveform::Sine);
    let high_conv_id = graph.add_convolution_node(Signal::Node(high_osc_id));

    let buffer_size = 512;
    let mut low_output = vec![0.0; buffer_size];
    let mut high_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&low_conv_id, &mut low_output);
    graph.eval_node_buffer(&high_conv_id, &mut high_output);

    // Both should produce sound
    let low_rms = calculate_rms(&low_output);
    let high_rms = calculate_rms(&high_output);

    assert!(
        low_rms > 0.01,
        "Low frequency convolution should produce sound: RMS = {}",
        low_rms
    );
    assert!(
        high_rms > 0.01,
        "High frequency convolution should produce sound: RMS = {}",
        high_rms
    );
}

// ============================================================================
// TEST 10: Buffer Boundary Continuity
// ============================================================================

#[test]
fn test_convolution_buffer_boundary_continuity() {
    let mut graph = create_test_graph();

    // Create sine wave oscillator
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Apply convolution
    let conv_id = graph.add_convolution_node(Signal::Node(osc_id));

    let buffer_size = 512;
    let num_buffers = 4;
    let mut buffers = vec![vec![0.0; buffer_size]; num_buffers];

    // Render multiple buffers
    for i in 0..num_buffers {
        graph.eval_node_buffer(&conv_id, &mut buffers[i]);
    }

    // Check continuity at buffer boundaries
    for i in 0..num_buffers - 1 {
        let last_sample = buffers[i][buffer_size - 1];
        let first_sample = buffers[i + 1][0];

        // They shouldn't be identical but should be reasonably close
        // (convolution adds reflections, so some variation is expected)
        assert!(
            (last_sample - first_sample).abs() < 0.5,
            "Buffer boundary {} -> {} should be continuous: {} vs {}",
            i,
            i + 1,
            last_sample,
            first_sample
        );
    }
}
