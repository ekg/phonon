use phonon::unified_graph::{NodeId, Signal, SignalGraph};

const SAMPLE_RATE: f32 = 44100.0;

/// Helper: Create a test graph with sample rate
fn create_test_graph() -> SignalGraph {
    SignalGraph::new(SAMPLE_RATE)
}

/// LEVEL 1: Test single impulse (frequency = 0)
#[test]
fn test_impulse_single() {
    let mut graph = create_test_graph();

    let impulse_id = graph.add_impulse_node(Signal::Value(0.0));

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&impulse_id, &mut output);

    // First sample should be 1.0 (impulse fired), rest should be 0.0
    assert_eq!(output[0], 1.0, "First sample should be impulse");
    assert!(output[1..].iter().all(|&x| x == 0.0),
        "All samples after first should be 0.0");
}

/// LEVEL 1: Test periodic impulses (10 Hz)
#[test]
fn test_impulse_periodic_10hz() {
    let mut graph = create_test_graph();

    let impulse_id = graph.add_impulse_node(Signal::Value(10.0)); // 10 Hz

    let mut output = vec![0.0; 44100]; // 1 second
    graph.eval_node_buffer(&impulse_id, &mut output);

    // Should have approximately 10 impulses in 1 second
    let impulse_count = output.iter().filter(|&&x| x == 1.0).count();
    assert!(impulse_count >= 9 && impulse_count <= 11,
        "Expected ~10 impulses, got {}", impulse_count);
}

/// LEVEL 1: Test impulse timing accuracy (440 Hz)
#[test]
fn test_impulse_timing_accuracy() {
    let mut graph = create_test_graph();

    let impulse_id = graph.add_impulse_node(Signal::Value(440.0)); // 440 Hz (A4)

    let mut output = vec![0.0; 44100]; // 1 second
    graph.eval_node_buffer(&impulse_id, &mut output);

    // Should have 440 impulses in 1 second
    let impulse_count = output.iter().filter(|&&x| x == 1.0).count();
    assert!(impulse_count >= 438 && impulse_count <= 442,
        "Expected ~440 impulses, got {}", impulse_count);

    // Calculate average interval between impulses
    let impulse_indices: Vec<usize> = output.iter()
        .enumerate()
        .filter(|(_, &x)| x == 1.0)
        .map(|(i, _)| i)
        .collect();

    if impulse_indices.len() >= 2 {
        let expected_interval = SAMPLE_RATE / 440.0; // ~100.227 samples
        let actual_interval = (impulse_indices[1] - impulse_indices[0]) as f32;
        assert!((actual_interval - expected_interval).abs() < 1.0,
            "Interval should be ~{}, got {}", expected_interval, actual_interval);
    }
}

/// LEVEL 2: Test state continuity across buffers
#[test]
fn test_impulse_state_continuity() {
    let mut graph = create_test_graph();

    let impulse_id = graph.add_impulse_node(Signal::Value(10.0)); // 10 Hz

    // Process multiple buffers
    let mut buffer1 = vec![0.0; 2205]; // ~50ms
    let mut buffer2 = vec![0.0; 2205];
    let mut buffer3 = vec![0.0; 2205];

    graph.eval_node_buffer(&impulse_id, &mut buffer1);
    graph.eval_node_buffer(&impulse_id, &mut buffer2);
    graph.eval_node_buffer(&impulse_id, &mut buffer3);

    // Count impulses across all buffers
    let count1 = buffer1.iter().filter(|&&x| x == 1.0).count();
    let count2 = buffer2.iter().filter(|&&x| x == 1.0).count();
    let count3 = buffer3.iter().filter(|&&x| x == 1.0).count();

    let total = count1 + count2 + count3;

    // 10 Hz over ~150ms (3 buffers) should give ~1-2 impulses
    // But due to phase continuity, timing should be correct
    assert!(total >= 1 && total <= 3,
        "Expected 1-3 impulses over 3 buffers, got {}", total);
}

/// LEVEL 2: Test pattern-modulated frequency
#[test]
fn test_impulse_modulated_frequency() {
    let mut graph = create_test_graph();

    // Create oscillating frequency (5 Hz to 15 Hz)
    let lfo_id = graph.add_oscillator(Signal::Value(0.5), phonon::unified_graph::Waveform::Sine); // 0.5 Hz LFO
    let scaled_lfo = graph.add_multiply_node(
        Signal::Node(lfo_id),
        Signal::Value(5.0), // ±5 Hz
    );
    let offset_lfo = graph.add_add_node(
        Signal::Node(scaled_lfo),
        Signal::Value(10.0), // Center at 10 Hz
    );

    let impulse_id = graph.add_impulse_node(Signal::Node(offset_lfo));

    let mut output = vec![0.0; 44100]; // 1 second
    graph.eval_node_buffer(&impulse_id, &mut output);

    // Should have impulses (frequency varies, so count should be in range)
    let impulse_count = output.iter().filter(|&&x| x == 1.0).count();
    assert!(impulse_count >= 5 && impulse_count <= 20,
        "Expected 5-20 impulses with modulated frequency, got {}", impulse_count);
}

/// LEVEL 2: Test very low frequency (1 Hz)
#[test]
fn test_impulse_low_frequency() {
    let mut graph = create_test_graph();

    let impulse_id = graph.add_impulse_node(Signal::Value(1.0)); // 1 Hz

    let mut output = vec![0.0; 44100]; // 1 second
    graph.eval_node_buffer(&impulse_id, &mut output);

    // Should have exactly 1 impulse
    let impulse_count = output.iter().filter(|&&x| x == 1.0).count();
    assert_eq!(impulse_count, 1, "Expected 1 impulse at 1 Hz");

    // First impulse should be at the start
    assert_eq!(output[0], 1.0, "Impulse should fire at start");
}

/// LEVEL 2: Test high frequency impulses
#[test]
fn test_impulse_high_frequency() {
    let mut graph = create_test_graph();

    let impulse_id = graph.add_impulse_node(Signal::Value(1000.0)); // 1 kHz

    let mut output = vec![0.0; 4410]; // 100ms
    graph.eval_node_buffer(&impulse_id, &mut output);

    // Should have approximately 100 impulses in 100ms
    let impulse_count = output.iter().filter(|&&x| x == 1.0).count();
    assert!(impulse_count >= 98 && impulse_count <= 102,
        "Expected ~100 impulses, got {}", impulse_count);
}

/// LEVEL 3: Test impulse through filter (integration)
#[test]
fn test_impulse_through_filter() {
    let mut graph = create_test_graph();

    let impulse_id = graph.add_impulse_node(Signal::Value(10.0)); // 10 Hz
    let filtered_id = graph.add_lowpass_node(
        Signal::Node(impulse_id),
        Signal::Value(1000.0), // 1 kHz cutoff
        Signal::Value(0.7),    // Q
    );

    let mut impulse_output = vec![0.0; 4410]; // 100ms
    let mut filtered_output = vec![0.0; 4410];

    graph.eval_node_buffer(&impulse_id, &mut impulse_output);
    graph.eval_node_buffer(&filtered_id, &mut filtered_output);

    // Filter should smooth the impulses
    // Calculate RMS to verify filtering effect
    let impulse_rms: f32 = (impulse_output.iter().map(|x| x * x).sum::<f32>()
        / impulse_output.len() as f32).sqrt();
    let filtered_rms: f32 = (filtered_output.iter().map(|x| x * x).sum::<f32>()
        / filtered_output.len() as f32).sqrt();

    // Filtered signal should have non-zero energy
    assert!(filtered_rms > 0.001, "Filtered signal should have energy");

    // Original impulses are sparse, so filtered version should be smoother
    assert!(filtered_rms > 0.0, "Filter should produce output");
}

/// LEVEL 3: Test zero frequency (should produce single impulse at start only)
#[test]
fn test_impulse_zero_frequency() {
    let mut graph = create_test_graph();

    let impulse_id = graph.add_impulse_node(Signal::Value(0.0)); // 0 Hz (single)

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&impulse_id, &mut output);

    // Should have exactly one impulse at the start
    assert_eq!(output[0], 1.0, "First sample should be impulse");
    assert!(output[1..].iter().all(|&x| x == 0.0),
        "Only first sample should be impulse");
}

/// LEVEL 3: Test negative frequency (should clamp to 0)
#[test]
fn test_impulse_negative_frequency() {
    let mut graph = create_test_graph();

    let impulse_id = graph.add_impulse_node(Signal::Value(-10.0)); // Negative (invalid)

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&impulse_id, &mut output);

    // Should behave like 0 Hz (single impulse at start)
    assert_eq!(output[0], 1.0, "First sample should be impulse");
    assert!(output[1..].iter().all(|&x| x == 0.0),
        "Negative frequency should clamp to 0 Hz behavior");
}

/// LEVEL 3: Test multiple buffers with different sizes
#[test]
fn test_impulse_variable_buffer_sizes() {
    let mut graph = create_test_graph();

    let impulse_id = graph.add_impulse_node(Signal::Value(10.0)); // 10 Hz

    // Test various buffer sizes
    for size in [64, 128, 256, 512, 1024, 2048] {
        let mut output = vec![0.0; size];
        graph.eval_node_buffer(&impulse_id, &mut output);

        // Each buffer should potentially have impulses
        // (depends on phase, so just verify no crashes and output is valid)
        assert!(output.iter().all(|&x| x == 0.0 || x == 1.0),
            "All values should be 0.0 or 1.0 for buffer size {}", size);
    }
}

/// LEVEL 3: Test impulse multiplication (envelope trigger pattern)
#[test]
fn test_impulse_as_trigger() {
    let mut graph = create_test_graph();

    let impulse_id = graph.add_impulse_node(Signal::Value(10.0)); // 10 Hz
    let carrier_id = graph.add_oscillator(Signal::Value(440.0), phonon::unified_graph::Waveform::Sine); // 440 Hz sine
    let gated_id = graph.add_multiply_node(
        Signal::Node(carrier_id),
        Signal::Node(impulse_id),
    );

    let mut output = vec![0.0; 4410]; // 100ms
    graph.eval_node_buffer(&gated_id, &mut output);

    // Output should be non-zero only when impulses fire
    let non_zero_count = output.iter().filter(|&&x| x.abs() > 0.001).count();

    // Should have some non-zero samples (where impulses fire)
    assert!(non_zero_count > 0, "Gated signal should have non-zero samples");

    // But not too many (only at impulse times)
    let total_samples = output.len();
    assert!(non_zero_count < total_samples / 2,
        "Most samples should be zero (gated)");
}
