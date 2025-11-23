use phonon::unified_graph::{NodeId, Signal, SignalGraph};

const SAMPLE_RATE: f32 = 44100.0;

/// Helper: Create a test graph with sample rate
fn create_test_graph() -> SignalGraph {
    SignalGraph::new(SAMPLE_RATE)
}

/// Helper: Create a step signal (0 to 1 transition)
fn create_step_signal(graph: &mut SignalGraph) -> NodeId {
    // Start at 0, then jump to 1
    graph.add_constant_node(1.0)
}

/// LEVEL 1: Test that lag smooths a step function
#[test]
fn test_lag_smooths_steps() {
    let mut graph = create_test_graph();

    // Step from 0 to 1
    let step_id = create_step_signal(&mut graph);
    let lag_id = graph.add_lag_node(
        Signal::Node(step_id),
        Signal::Value(0.01),  // 10ms lag time
    );

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&lag_id, &mut output);

    // Should gradually approach 1.0, not instant
    assert!(output[10] < 0.9, "Should not reach 0.9 after 10 samples, got {}", output[10]);
    assert!(output[400] > 0.95, "Should be near 1.0 after 400 samples, got {}", output[400]);
    assert!(output[500] > 0.98, "Should be very close to 1.0 after 500 samples, got {}", output[500]);
}

/// LEVEL 1: Test different lag times (faster vs slower)
#[test]
fn test_lag_different_times() {
    let mut graph = create_test_graph();

    let step_id = graph.add_constant_node(1.0);

    // Fast lag (1ms)
    let fast_lag_id = graph.add_lag_node(
        Signal::Node(step_id),
        Signal::Value(0.001),
    );

    // Slow lag (100ms)
    let slow_lag_id = graph.add_lag_node(
        Signal::Node(step_id),
        Signal::Value(0.1),
    );

    let mut fast_output = vec![0.0; 512];
    let mut slow_output = vec![0.0; 512];

    graph.eval_node_buffer(&fast_lag_id, &mut fast_output);
    graph.eval_node_buffer(&slow_lag_id, &mut slow_output);

    // Fast lag should reach target much quicker
    assert!(fast_output[100] > slow_output[100],
        "Fast lag should be ahead of slow lag: fast={}, slow={}",
        fast_output[100], slow_output[100]);

    // Both should be moving toward 1.0
    assert!(fast_output[10] > 0.1, "Fast lag should be moving, got {}", fast_output[10]);
    assert!(slow_output[10] > 0.01, "Slow lag should be moving, got {}", slow_output[10]);
}

/// LEVEL 1: Test zero lag time (should pass through almost instantly)
#[test]
fn test_lag_zero_bypass() {
    let mut graph = create_test_graph();

    let input_id = graph.add_constant_node(1.0);
    let lag_id = graph.add_lag_node(
        Signal::Node(input_id),
        Signal::Value(0.0),  // Zero lag time
    );

    let mut output = vec![0.0; 64];
    graph.eval_node_buffer(&lag_id, &mut output);

    // With zero lag, should reach target very quickly
    assert!(output[5] > 0.95, "Zero lag should respond almost instantly, got {}", output[5]);
}

/// LEVEL 2: Test state continuity across buffer calls
#[test]
fn test_lag_state_continuity() {
    let mut graph = create_test_graph();

    let step_id = graph.add_constant_node(1.0);
    let lag_id = graph.add_lag_node(
        Signal::Node(step_id),
        Signal::Value(0.05),  // 50ms lag time
    );

    // Process multiple buffers
    let mut buffer1 = vec![0.0; 256];
    let mut buffer2 = vec![0.0; 256];
    let mut buffer3 = vec![0.0; 256];

    graph.eval_node_buffer(&lag_id, &mut buffer1);
    graph.eval_node_buffer(&lag_id, &mut buffer2);
    graph.eval_node_buffer(&lag_id, &mut buffer3);

    // Each buffer should continue from where the last left off
    // Allow small tolerance for floating point
    let diff1 = (buffer2[0] - buffer1[255]).abs();
    assert!(diff1 < 0.01,
        "State should continue across buffers: buf1_end={}, buf2_start={}, diff={}",
        buffer1[255], buffer2[0], diff1);

    // Should be monotonically increasing toward 1.0
    assert!(buffer1[100] < buffer1[255],
        "Buffer 1 should be increasing");
    assert!(buffer2[100] < buffer2[255],
        "Buffer 2 should be increasing");
}

/// LEVEL 2: Test modulated lag time
#[test]
fn test_lag_modulated_time() {
    let mut graph = create_test_graph();

    let input_id = graph.add_constant_node(1.0);

    // Create oscillating lag time (0.001 to 0.1 seconds)
    let lfo_id = graph.add_oscillator_node(Signal::Value(0.5));
    let scaled_lfo = graph.add_multiply_node(
        Signal::Node(lfo_id),
        Signal::Value(0.05), // Scale to ±0.05
    );
    let offset_lfo = graph.add_add_node(
        Signal::Node(scaled_lfo),
        Signal::Value(0.055), // Offset to [0.005, 0.105]
    );

    let lag_id = graph.add_lag_node(
        Signal::Node(input_id),
        Signal::Node(offset_lfo),
    );

    let mut output = vec![0.0; 1024];
    graph.eval_node_buffer(&lag_id, &mut output);

    // Should be making progress toward 1.0
    assert!(output[100] > 0.01, "Should be moving toward target, got {}", output[100]);
    assert!(output[500] > output[100], "Should keep approaching target");
}

/// LEVEL 2: Test lag with oscillating input
#[test]
fn test_lag_with_oscillator() {
    let mut graph = create_test_graph();

    // Slow oscillator (1 Hz)
    let osc_id = graph.add_oscillator_node(Signal::Value(1.0));

    // Lag it with 20ms time
    let lag_id = graph.add_lag_node(
        Signal::Node(osc_id),
        Signal::Value(0.02),
    );

    let mut osc_output = vec![0.0; 4410];
    let mut lag_output = vec![0.0; 4410];

    graph.eval_node_buffer(&osc_id, &mut osc_output);
    graph.eval_node_buffer(&lag_id, &mut lag_output);

    // Lag output should be smoother (less high-frequency content)
    let calc_rms_of_diffs = |buffer: &[f32]| -> f32 {
        let mut sum = 0.0;
        for i in 1..buffer.len() {
            let diff = buffer[i] - buffer[i-1];
            sum += diff * diff;
        }
        (sum / (buffer.len() - 1) as f32).sqrt()
    };

    let osc_diff_rms = calc_rms_of_diffs(&osc_output);
    let lag_diff_rms = calc_rms_of_diffs(&lag_output);

    // Lagged signal should have smaller sample-to-sample changes
    assert!(lag_diff_rms < osc_diff_rms * 1.1,
        "Lag should smooth changes: osc_rms={}, lag_rms={}",
        osc_diff_rms, lag_diff_rms);
}

/// LEVEL 3: Test multiple lags in series (cascading smoothing)
#[test]
fn test_lag_cascade() {
    let mut graph = create_test_graph();

    let step_id = graph.add_constant_node(1.0);

    // Two lags in series
    let lag1_id = graph.add_lag_node(
        Signal::Node(step_id),
        Signal::Value(0.01),
    );
    let lag2_id = graph.add_lag_node(
        Signal::Node(lag1_id),
        Signal::Value(0.01),
    );

    let mut output1 = vec![0.0; 512];
    let mut output2 = vec![0.0; 512];

    graph.eval_node_buffer(&lag1_id, &mut output1);
    graph.eval_node_buffer(&lag2_id, &mut output2);

    // Second lag should be even smoother (behind the first)
    assert!(output2[100] < output1[100],
        "Cascaded lag should be behind: lag1={}, lag2={}",
        output1[100], output2[100]);
    assert!(output2[300] < output1[300],
        "Cascaded lag should remain behind");
}

/// LEVEL 3: Test lag removes discontinuities
#[test]
fn test_lag_removes_clicks() {
    let mut graph = create_test_graph();

    let step_id = graph.add_constant_node(1.0);
    let lag_id = graph.add_lag_node(
        Signal::Node(step_id),
        Signal::Value(0.005),  // 5ms lag time
    );

    let mut output = vec![0.0; 256];
    graph.eval_node_buffer(&lag_id, &mut output);

    // Calculate maximum sample-to-sample difference
    let mut max_diff = 0.0f32;
    for i in 1..output.len() {
        let diff = (output[i] - output[i - 1]).abs();
        max_diff = max_diff.max(diff);
    }

    // With lag, there should be no sudden jumps
    assert!(max_diff < 0.1,
        "Lag should prevent large jumps (max_diff={})", max_diff);
}

/// LEVEL 3: Test very small lag times approach bypass
#[test]
fn test_lag_very_small_time() {
    let mut graph = create_test_graph();

    let input_id = graph.add_constant_node(0.5);
    let lag_id = graph.add_lag_node(
        Signal::Node(input_id),
        Signal::Value(0.00001),  // 10 microseconds
    );

    let mut output = vec![0.0; 128];
    graph.eval_node_buffer(&lag_id, &mut output);

    // Should reach target almost immediately
    assert!(output[10] > 0.45, "Very small lag should respond fast");
    assert!(output[50] > 0.49, "Should be very close to target");
}

/// LEVEL 3: Test lag preserves DC value
#[test]
fn test_lag_dc_preservation() {
    let mut graph = create_test_graph();

    let dc_id = graph.add_constant_node(0.7);
    let lag_id = graph.add_lag_node(
        Signal::Node(dc_id),
        Signal::Value(0.05),
    );

    // Process multiple buffers to let it settle
    let mut buffer = vec![0.0; 512];
    for _ in 0..10 {
        graph.eval_node_buffer(&lag_id, &mut buffer);
    }

    // After settling, should be at the DC value
    let mean: f32 = buffer.iter().sum::<f32>() / buffer.len() as f32;
    assert!((mean - 0.7).abs() < 0.01,
        "After settling, lag should preserve DC value: mean={}", mean);
}

/// LEVEL 3: Test negative input values
#[test]
fn test_lag_negative_values() {
    let mut graph = create_test_graph();

    let neg_id = graph.add_constant_node(-1.0);
    let lag_id = graph.add_lag_node(
        Signal::Node(neg_id),
        Signal::Value(0.01),
    );

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&lag_id, &mut output);

    // Should smoothly approach -1.0
    assert!(output[100] < 0.0, "Should be moving toward negative value");
    assert!(output[100] > -0.9, "Should not reach target instantly");
    assert!(output[400] < -0.95, "Should be near target after 400 samples");
}

/// LEVEL 3: Test buffer-aligned processing (powers of 2)
#[test]
fn test_lag_buffer_sizes() {
    let mut graph = create_test_graph();

    let input_id = graph.add_constant_node(1.0);
    let lag_id = graph.add_lag_node(
        Signal::Node(input_id),
        Signal::Value(0.01),
    );

    // Test various buffer sizes
    for size in [64, 128, 256, 512, 1024] {
        let mut output = vec![0.0; size];
        graph.eval_node_buffer(&lag_id, &mut output);

        // Should work for any buffer size
        assert!(output[size - 1] > 0.0,
            "Should produce output for buffer size {}", size);
    }
}
