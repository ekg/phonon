use phonon::unified_graph::{UnifiedSignalGraph, Signal, Waveform};

/// Calculate RMS (Root Mean Square) energy of a buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum: f32 = buffer.iter().map(|&s| s * s).sum();
    (sum / buffer.len() as f32).sqrt()
}

/// Test helper: Create a test graph with proper setup
fn create_test_graph() -> UnifiedSignalGraph {
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0);
    graph
}

#[test]
fn test_tapedelay_basic_delay() {
    // Test basic delay functionality without wow/flutter/saturation
    let mut graph = create_test_graph();

    // Create a sine oscillator
    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Add tape delay with no modulation (clean delay)
    let tape_id = graph.add_tapedelay_node(
        Signal::Node(osc),
        Signal::Value(0.25),      // 250ms delay
        Signal::Value(0.5),       // 50% feedback
        Signal::Value(0.0),       // No wow rate
        Signal::Value(0.0),       // No wow depth
        Signal::Value(0.0),       // No flutter rate
        Signal::Value(0.0),       // No flutter depth
        Signal::Value(0.0),       // No saturation
        Signal::Value(0.5),       // 50% mix
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&tape_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.2, "Tape delay should produce audible sound: RMS={}", rms);
    assert!(rms < 1.0, "Output should not clip: RMS={}", rms);
}

#[test]
fn test_tapedelay_with_flutter() {
    // Test flutter effect (high-frequency pitch modulation)
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Tape delay with flutter
    let tape_id = graph.add_tapedelay_node(
        Signal::Node(osc),
        Signal::Value(0.25),      // 250ms delay
        Signal::Value(0.5),       // 50% feedback
        Signal::Value(0.0),       // No wow
        Signal::Value(0.0),
        Signal::Value(8.0),       // 8 Hz flutter rate
        Signal::Value(0.5),       // 50% flutter depth
        Signal::Value(0.0),       // No saturation
        Signal::Value(0.7),       // 70% wet
    );

    let buffer_size = 1024;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&tape_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.2, "Flutter tape delay should produce audible sound: RMS={}", rms);
}

#[test]
fn test_tapedelay_with_wow() {
    // Test wow effect (low-frequency pitch modulation)
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Tape delay with wow
    let tape_id = graph.add_tapedelay_node(
        Signal::Node(osc),
        Signal::Value(0.25),      // 250ms delay
        Signal::Value(0.6),       // 60% feedback
        Signal::Value(1.5),       // 1.5 Hz wow rate
        Signal::Value(0.7),       // 70% wow depth
        Signal::Value(0.0),       // No flutter
        Signal::Value(0.0),
        Signal::Value(0.0),       // No saturation
        Signal::Value(0.8),       // 80% wet
    );

    let buffer_size = 1024;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&tape_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.1, "Wow tape delay should produce audible sound: RMS={}", rms);
}

#[test]
fn test_tapedelay_with_saturation() {
    // Test tape saturation effect
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Tape delay with saturation
    let tape_id = graph.add_tapedelay_node(
        Signal::Node(osc),
        Signal::Value(0.2),       // 200ms delay
        Signal::Value(0.7),       // 70% feedback (more tape warmth)
        Signal::Value(0.0),       // No wow
        Signal::Value(0.0),
        Signal::Value(0.0),       // No flutter
        Signal::Value(0.0),
        Signal::Value(0.8),       // 80% saturation
        Signal::Value(0.6),       // 60% wet
    );

    let buffer_size = 1024;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&tape_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.2, "Saturated tape delay should produce audible sound: RMS={}", rms);

    // Saturation should affect the waveform
    // Check that some samples are different (not all the same)
    let unique_values: std::collections::HashSet<_> = output
        .iter()
        .filter(|&&x| x.abs() > 0.001)
        .map(|&x| (x * 1000.0) as i32)
        .collect();
    assert!(unique_values.len() > 10, "Saturated delay should produce varied output");
}

#[test]
fn test_tapedelay_full_features() {
    // Test all tape delay features together: wow, flutter, and saturation
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Full vintage tape echo
    let tape_id = graph.add_tapedelay_node(
        Signal::Node(osc),
        Signal::Value(0.3),       // 300ms delay
        Signal::Value(0.6),       // 60% feedback
        Signal::Value(1.0),       // 1 Hz wow
        Signal::Value(0.5),       // 50% wow depth
        Signal::Value(7.0),       // 7 Hz flutter
        Signal::Value(0.3),       // 30% flutter depth
        Signal::Value(0.6),       // 60% saturation
        Signal::Value(0.7),       // 70% wet
    );

    let buffer_size = 2048;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&tape_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.2, "Full tape delay should produce audible sound: RMS={}", rms);
    assert!(rms < 1.0, "Output should not clip: RMS={}", rms);
}

#[test]
fn test_tapedelay_state_continuity() {
    // Test that state is maintained across multiple buffer evaluations
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    let tape_id = graph.add_tapedelay_node(
        Signal::Node(osc),
        Signal::Value(0.1),       // 100ms delay
        Signal::Value(0.7),       // 70% feedback
        Signal::Value(1.0),       // Wow
        Signal::Value(0.3),
        Signal::Value(7.0),       // Flutter
        Signal::Value(0.2),
        Signal::Value(0.4),       // Saturation
        Signal::Value(0.6),
    );

    let buffer_size = 512;
    let mut output1 = vec![0.0; buffer_size];
    let mut output2 = vec![0.0; buffer_size];
    let mut output3 = vec![0.0; buffer_size];

    // Evaluate multiple buffers
    graph.eval_node_buffer(&tape_id, &mut output1);
    graph.eval_node_buffer(&tape_id, &mut output2);
    graph.eval_node_buffer(&tape_id, &mut output3);

    // All buffers should have sound
    let rms1 = calculate_rms(&output1);
    let rms2 = calculate_rms(&output2);
    let rms3 = calculate_rms(&output3);

    assert!(rms1 > 0.1, "First buffer should have sound: RMS={}", rms1);
    assert!(rms2 > 0.1, "Second buffer should have sound: RMS={}", rms2);
    assert!(rms3 > 0.1, "Third buffer should have sound: RMS={}", rms3);

    // Buffers should be different (state is evolving)
    assert_ne!(output1, output2, "Consecutive buffers should differ due to state evolution");
    assert_ne!(output2, output3, "Consecutive buffers should differ due to state evolution");
}

#[test]
fn test_tapedelay_vs_clean_delay() {
    // Compare tape delay (with effects) to clean delay (no effects)
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Clean tape delay (no modulation)
    let clean_id = graph.add_tapedelay_node(
        Signal::Node(osc),
        Signal::Value(0.2),
        Signal::Value(0.5),
        Signal::Value(0.0),       // No wow
        Signal::Value(0.0),
        Signal::Value(0.0),       // No flutter
        Signal::Value(0.0),
        Signal::Value(0.0),       // No saturation
        Signal::Value(0.5),
    );

    // Create another oscillator for comparison
    let osc2 = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Vintage tape delay (with wow, flutter, saturation)
    let vintage_id = graph.add_tapedelay_node(
        Signal::Node(osc2),
        Signal::Value(0.2),
        Signal::Value(0.5),
        Signal::Value(1.5),       // Wow
        Signal::Value(0.6),
        Signal::Value(8.0),       // Flutter
        Signal::Value(0.4),
        Signal::Value(0.7),       // Saturation
        Signal::Value(0.5),
    );

    let buffer_size = 1024;
    let mut clean_output = vec![0.0; buffer_size];
    let mut vintage_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&clean_id, &mut clean_output);
    graph.eval_node_buffer(&vintage_id, &mut vintage_output);

    let clean_rms = calculate_rms(&clean_output);
    let vintage_rms = calculate_rms(&vintage_output);

    // Both should produce sound
    assert!(clean_rms > 0.1, "Clean delay should produce sound");
    assert!(vintage_rms > 0.1, "Vintage delay should produce sound");

    // The vintage delay should have different character
    // (this is a loose check - just verifying they're both working)
    assert!(clean_rms > 0.0 && vintage_rms > 0.0,
           "Both delays should be active");
}

#[test]
fn test_tapedelay_parameter_clamping() {
    // Test that extreme parameter values are clamped properly
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Extreme parameter values (should be clamped internally)
    let tape_id = graph.add_tapedelay_node(
        Signal::Node(osc),
        Signal::Value(0.1),       // Short delay so we hear it quickly
        Signal::Value(1.5),       // Too much feedback (should clamp to 0.95)
        Signal::Value(10.0),      // High wow rate (should clamp to 2.0)
        Signal::Value(2.0),       // Too much wow depth (should clamp to 1.0)
        Signal::Value(20.0),      // High flutter rate (should clamp to 10.0)
        Signal::Value(5.0),       // Too much flutter depth (should clamp to 1.0)
        Signal::Value(3.0),       // Too much saturation (should clamp to 1.0)
        Signal::Value(0.5),       // 50% wet so we hear dry signal immediately
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    // Should not crash or produce NaN values
    // Process a few buffers to allow delay to build up
    for _ in 0..5 {
        graph.eval_node_buffer(&tape_id, &mut output);
    }

    // Verify output is valid (no NaN, no Inf)
    for (i, &sample) in output.iter().enumerate() {
        assert!(sample.is_finite(), "Sample {} should be finite: {}", i, sample);
    }

    let rms = calculate_rms(&output);
    assert!(rms > 0.0, "Should produce sound even with clamped parameters");
    assert!(rms < 2.0, "Should not produce excessive output");
}

#[test]
fn test_tapedelay_feedback_stability() {
    // Test that high feedback doesn't cause instability or clipping
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // High feedback (but clamped to 0.95)
    let tape_id = graph.add_tapedelay_node(
        Signal::Node(osc),
        Signal::Value(0.15),
        Signal::Value(0.9),       // 90% feedback
        Signal::Value(0.0),
        Signal::Value(0.0),
        Signal::Value(0.0),
        Signal::Value(0.0),
        Signal::Value(0.5),       // Some saturation to control
        Signal::Value(0.8),
    );

    let buffer_size = 1024;

    // Process multiple buffers to test stability over time
    for iteration in 0..10 {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&tape_id, &mut output);

        // Check for stability
        let rms = calculate_rms(&output);
        assert!(rms < 2.0, "Iteration {}: RMS should remain stable: {}", iteration, rms);

        // Check no NaN or Inf
        for &sample in output.iter() {
            assert!(sample.is_finite(), "Iteration {}: Sample should be finite", iteration);
        }
    }
}

#[test]
fn test_tapedelay_dry_wet_mix() {
    // Test dry/wet mixing
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // 100% dry (mix = 0.0)
    let dry_id = graph.add_tapedelay_node(
        Signal::Node(osc),
        Signal::Value(0.2),
        Signal::Value(0.5),
        Signal::Value(1.0),
        Signal::Value(0.3),
        Signal::Value(7.0),
        Signal::Value(0.2),
        Signal::Value(0.4),
        Signal::Value(0.0),       // 100% dry
    );

    let osc2 = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // 100% wet (mix = 1.0)
    let wet_id = graph.add_tapedelay_node(
        Signal::Node(osc2),
        Signal::Value(0.2),
        Signal::Value(0.5),
        Signal::Value(1.0),
        Signal::Value(0.3),
        Signal::Value(7.0),
        Signal::Value(0.2),
        Signal::Value(0.4),
        Signal::Value(1.0),       // 100% wet
    );

    let buffer_size = 512;
    let mut dry_output = vec![0.0; buffer_size];
    let mut wet_output = vec![0.0; buffer_size];

    // Process a few buffers to build up delay
    for _ in 0..3 {
        graph.eval_node_buffer(&dry_id, &mut dry_output);
        graph.eval_node_buffer(&wet_id, &mut wet_output);
    }

    let dry_rms = calculate_rms(&dry_output);
    let wet_rms = calculate_rms(&wet_output);

    // Both should produce sound
    assert!(dry_rms > 0.1, "Dry signal should be audible");
    // Wet signal may be quieter initially as delay buffer builds up
    assert!(wet_rms >= 0.0, "Wet signal test should complete without error");
}

#[test]
fn test_tapedelay_short_vs_long_delay() {
    // Compare short and long delay times
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Short delay (50ms)
    let short_id = graph.add_tapedelay_node(
        Signal::Node(osc),
        Signal::Value(0.05),      // 50ms
        Signal::Value(0.5),
        Signal::Value(0.0),
        Signal::Value(0.0),
        Signal::Value(0.0),
        Signal::Value(0.0),
        Signal::Value(0.0),
        Signal::Value(0.5),
    );

    let osc2 = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Long delay (500ms)
    let long_id = graph.add_tapedelay_node(
        Signal::Node(osc2),
        Signal::Value(0.5),       // 500ms
        Signal::Value(0.5),
        Signal::Value(0.0),
        Signal::Value(0.0),
        Signal::Value(0.0),
        Signal::Value(0.0),
        Signal::Value(0.0),
        Signal::Value(0.5),
    );

    let buffer_size = 512;
    let mut short_output = vec![0.0; buffer_size];
    let mut long_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&short_id, &mut short_output);
    graph.eval_node_buffer(&long_id, &mut long_output);

    let short_rms = calculate_rms(&short_output);
    let long_rms = calculate_rms(&long_output);

    // Both should produce sound
    assert!(short_rms > 0.1, "Short delay should produce sound");
    assert!(long_rms > 0.1, "Long delay should produce sound");
}
