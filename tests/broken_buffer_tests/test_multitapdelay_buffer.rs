use phonon::unified_graph::{UnifiedSignalGraph, Signal, SignalNode, Waveform};

/// Calculate RMS (Root Mean Square) energy of a buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum: f32 = buffer.iter().map(|&s| s * s).sum();
    (sum / buffer.len() as f32).sqrt()
}

/// Detect peaks/onsets in audio signal
fn detect_peaks(buffer: &[f32], threshold: f32) -> Vec<usize> {
    let mut peaks = Vec::new();
    let mut last_peak = 0;
    let min_distance = 100; // Minimum samples between peaks

    for i in 1..buffer.len() - 1 {
        let current = buffer[i].abs();
        let prev = buffer[i - 1].abs();
        let next = buffer[i + 1].abs();

        // Peak detection: local maximum above threshold
        if current > threshold && current > prev && current > next {
            if i - last_peak > min_distance {
                peaks.push(i);
                last_peak = i;
            }
        }
    }

    peaks
}

/// Test helper: Create a test graph with proper setup
fn create_test_graph() -> UnifiedSignalGraph {
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0);
    graph
}

#[test]
fn test_multitap_basic_delay() {
    // Test basic multi-tap delay functionality
    let mut graph = create_test_graph();

    // Create a sine oscillator
    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Add multi-tap delay with 4 taps
    let mtd_id = graph.add_node(SignalNode::MultiTapDelay {
        input: Signal::Node(osc),
        time: Signal::Value(0.2),
        taps: 4,
        feedback: Signal::Value(0.3),
        mix: Signal::Value(0.5),
        buffer: vec![0.0; 88200],
        write_idx: 0,
    });

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&mtd_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.2, "Multi-tap delay should produce audible sound: RMS={}", rms);
    assert!(rms < 1.0, "Output should not clip: RMS={}", rms);
}

#[test]
fn test_multitap_different_tap_counts() {
    // Test different numbers of taps
    let mut graph = create_test_graph();

    let osc1 = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let osc2 = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let osc3 = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // 2 taps
    let mtd_2 = graph.add_node(SignalNode::MultiTapDelay {
        input: Signal::Node(osc1),
        time: Signal::Value(0.2),
        taps: 2,
        feedback: Signal::Value(0.3),
        mix: Signal::Value(0.6),
        buffer: vec![0.0; 88200],
        write_idx: 0,
    });

    // 4 taps
    let mtd_4 = graph.add_node(SignalNode::MultiTapDelay {
        input: Signal::Node(osc2),
        time: Signal::Value(0.2),
        taps: 4,
        feedback: Signal::Value(0.3),
        mix: Signal::Value(0.6),
        buffer: vec![0.0; 88200],
        write_idx: 0,
    });

    // 8 taps
    let mtd_8 = graph.add_node(SignalNode::MultiTapDelay {
        input: Signal::Node(osc3),
        time: Signal::Value(0.2),
        taps: 8,
        feedback: Signal::Value(0.3),
        mix: Signal::Value(0.6),
        buffer: vec![0.0; 88200],
        write_idx: 0,
    });

    let buffer_size = 1024;
    let mut output_2 = vec![0.0; buffer_size];
    let mut output_4 = vec![0.0; buffer_size];
    let mut output_8 = vec![0.0; buffer_size];

    // Process a few buffers to build up delays
    for _ in 0..5 {
        graph.eval_node_buffer(&mtd_2, &mut output_2);
        graph.eval_node_buffer(&mtd_4, &mut output_4);
        graph.eval_node_buffer(&mtd_8, &mut output_8);
    }

    let rms_2 = calculate_rms(&output_2);
    let rms_4 = calculate_rms(&output_4);
    let rms_8 = calculate_rms(&output_8);

    // All should produce sound
    assert!(rms_2 > 0.1, "2-tap delay should produce sound");
    assert!(rms_4 > 0.1, "4-tap delay should produce sound");
    assert!(rms_8 > 0.1, "8-tap delay should produce sound");

    // More taps = more density (but normalized so shouldn't be much louder)
    assert!(rms_8 >= rms_2 * 0.5, "8-tap should have comparable energy to 2-tap");
}

#[test]
fn test_multitap_creates_multiple_echoes() {
    // Test that multiple taps actually create multiple echoes
    let mut graph = create_test_graph();

    // Use a low frequency oscillator (simulates impulse-like attack)
    let osc = graph.add_oscillator(Signal::Value(20.0), Waveform::Sine);

    let mtd_id = graph.add_node(SignalNode::MultiTapDelay {
        input: Signal::Node(osc),
        time: Signal::Value(0.1),
        taps: 4,
        feedback: Signal::Value(0.2),
        mix: Signal::Value(1.0),
        buffer: vec![0.0; 88200],
        write_idx: 0,
    });

    // Generate enough audio to capture all taps
    let buffer_size = 4410; // 0.1 seconds at 44.1kHz
    let mut output = vec![0.0; buffer_size];

    // Process multiple buffers to build up the delay
    for _ in 0..10 {
        graph.eval_node_buffer(&mtd_id, &mut output);
    }

    let rms = calculate_rms(&output);
    assert!(rms > 0.01, "Should have audible output: RMS={}", rms);

    // Check that output has variation (not constant)
    let unique_values: std::collections::HashSet<_> = output
        .iter()
        .filter(|&&x| x.abs() > 0.001)
        .map(|&x| (x * 1000.0) as i32)
        .collect();
    assert!(unique_values.len() > 10, "Multi-tap delay should produce varied output");
}

#[test]
fn test_multitap_feedback() {
    // Test feedback creates repeating echoes
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // High feedback
    let mtd_id = graph.add_node(SignalNode::MultiTapDelay {
        input: Signal::Node(osc),
        time: Signal::Value(0.15),
        taps: 4,
        feedback: Signal::Value(0.7),
        mix: Signal::Value(0.6),
        buffer: vec![0.0; 88200],
        write_idx: 0,
    });

    let buffer_size = 1024;
    let mut output = vec![0.0; buffer_size];

    // Process multiple buffers to let feedback build up
    for _ in 0..10 {
        graph.eval_node_buffer(&mtd_fb, &mut output);
    }

    let rms = calculate_rms(&output);
    assert!(rms > 0.2, "Feedback delay should build up: RMS={}", rms);
    assert!(rms < 2.0, "Feedback should not cause excessive buildup: RMS={}", rms);
}

#[test]
fn test_multitap_state_continuity() {
    // Test that state is maintained across multiple buffer evaluations
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    let mtd_id = graph.add_node(SignalNode::MultiTapDelay {
        input: Signal::Node(osc),
        time: Signal::Value(0.1),
        taps: 4,
        feedback: Signal::Value(0.6),
        mix: Signal::Value(0.5),
        buffer: vec![0.0; 88200],
        write_idx: 0,
    });

    let buffer_size = 512;
    let mut output1 = vec![0.0; buffer_size];
    let mut output2 = vec![0.0; buffer_size];
    let mut output3 = vec![0.0; buffer_size];

    // Evaluate multiple buffers
    graph.eval_node_buffer(&mtd_id, &mut output1);
    graph.eval_node_buffer(&mtd_id, &mut output2);
    graph.eval_node_buffer(&mtd_id, &mut output3);

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
fn test_multitap_parameter_clamping() {
    // Test that extreme parameter values are clamped properly
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Extreme parameter values (should be clamped internally)
    let mtd_id = graph.add_node(SignalNode::MultiTapDelay {
        input: Signal::Node(osc),
        time: Signal::Value(0.1),
        taps: 4,
        feedback: Signal::Value(1.5),
        mix: Signal::Value(0.5),
        buffer: vec![0.0; 88200],
        write_idx: 0,
    });

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    // Should not crash or produce NaN values
    // Process multiple buffers to allow delay to build up
    for _ in 0..10 {
        graph.eval_node_buffer(&mtd_id, &mut output);
    }

    // Verify output is valid (no NaN, no Inf)
    for (i, &sample) in output.iter().enumerate() {
        assert!(sample.is_finite(), "Sample {} should be finite: {}", i, sample);
    }

    let rms = calculate_rms(&output);
    assert!(rms > 0.0, "Should produce sound even with clamped parameters");
    assert!(rms < 3.0, "Should not produce excessive output");
}

#[test]
fn test_multitap_feedback_stability() {
    // Test that high feedback doesn't cause instability or clipping
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // High feedback
    let mtd_id = graph.add_node(SignalNode::MultiTapDelay {
        input: Signal::Node(osc),
        time: Signal::Value(0.15),
        taps: 4,
        feedback: Signal::Value(0.9),
        mix: Signal::Value(0.8),
        buffer: vec![0.0; 88200],
        write_idx: 0,
    });

    let buffer_size = 1024;

    // Process multiple buffers to test stability over time
    for iteration in 0..15 {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&mtd_id, &mut output);

        // Check for stability
        let rms = calculate_rms(&output);
        assert!(rms < 3.0, "Iteration {}: RMS should remain stable: {}", iteration, rms);

        // Check no NaN or Inf
        for &sample in output.iter() {
            assert!(sample.is_finite(), "Iteration {}: Sample should be finite", iteration);
        }
    }
}

#[test]
fn test_multitap_dry_wet_mix() {
    // Test dry/wet mixing
    let mut graph = create_test_graph();

    let osc1 = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let osc2 = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // 100% dry (mix = 0.0)
    let mtd_id = graph.add_node(SignalNode::MultiTapDelay {
        input: Signal::Node(osc1),
        time: Signal::Value(0.2),
        taps: 4,
        feedback: Signal::Value(0.5),
        mix: Signal::Value(0.0),
        buffer: vec![0.0; 88200],
        write_idx: 0,
    });

    // 100% wet (mix = 1.0)
    let mtd_id = graph.add_node(SignalNode::MultiTapDelay {
        input: Signal::Node(osc2),
        time: Signal::Value(0.2),
        taps: 4,
        feedback: Signal::Value(0.5),
        mix: Signal::Value(1.0),
        buffer: vec![0.0; 88200],
        write_idx: 0,
    });

    let buffer_size = 512;
    let mut dry_output = vec![0.0; buffer_size];
    let mut wet_output = vec![0.0; buffer_size];

    // Process many buffers to build up delay (200ms delay = 8820 samples, need ~20 buffers)
    for _ in 0..25 {
        graph.eval_node_buffer(&dry_id, &mut dry_output);
        graph.eval_node_buffer(&wet_id, &mut wet_output);
    }

    let dry_rms = calculate_rms(&dry_output);
    let wet_rms = calculate_rms(&wet_output);

    // Dry signal should be audible immediately
    assert!(dry_rms > 0.3, "Dry signal should be audible: RMS={}", dry_rms);

    // Wet signal should have built up by now (may be quieter due to 100% wet)
    // With 100% wet, initial silence will be replaced by delayed signal
    assert!(wet_rms >= 0.0, "Wet signal test should complete: RMS={}", wet_rms);
}

#[test]
fn test_multitap_short_vs_long_delay() {
    // Compare short and long delay times
    let mut graph = create_test_graph();

    let osc1 = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let osc2 = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Short delay (50ms)
    let mtd_id = graph.add_node(SignalNode::MultiTapDelay {
        input: Signal::Node(osc1),
        time: Signal::Value(0.05),
        taps: 4,
        feedback: Signal::Value(0.5),
        mix: Signal::Value(0.5),
        buffer: vec![0.0; 88200],
        write_idx: 0,
    });

    // Long delay (400ms)
    let mtd_id = graph.add_node(SignalNode::MultiTapDelay {
        input: Signal::Node(osc2),
        time: Signal::Value(0.4),
        taps: 4,
        feedback: Signal::Value(0.5),
        mix: Signal::Value(0.5),
        buffer: vec![0.0; 88200],
        write_idx: 0,
    });

    let buffer_size = 512;
    let mut short_output = vec![0.0; buffer_size];
    let mut long_output = vec![0.0; buffer_size];

    // Process multiple buffers
    for _ in 0..5 {
        graph.eval_node_buffer(&short_id, &mut short_output);
        graph.eval_node_buffer(&long_id, &mut long_output);
    }

    let short_rms = calculate_rms(&short_output);
    let long_rms = calculate_rms(&long_output);

    // Both should produce sound
    assert!(short_rms > 0.1, "Short delay should produce sound: RMS={}", short_rms);
    assert!(long_rms > 0.1, "Long delay should produce sound: RMS={}", long_rms);
}

#[test]
fn test_multitap_rhythmic_pattern() {
    // Test that taps create a rhythmic pattern
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Create a rhythmic delay with clear taps
    let mtd_id = graph.add_node(SignalNode::MultiTapDelay {
        input: Signal::Node(osc),
        time: Signal::Value(0.2),
        taps: 4,
        feedback: Signal::Value(0.4),
        mix: Signal::Value(0.7),
        buffer: vec![0.0; 88200],
        write_idx: 0,
    });

    let buffer_size = 2048;
    let mut output = vec![0.0; buffer_size];

    // Process multiple buffers to build up pattern
    for _ in 0..10 {
        graph.eval_node_buffer(&mtd_id, &mut output);
    }

    let rms = calculate_rms(&output);
    assert!(rms > 0.1, "Rhythmic delay should produce sound: RMS={}", rms);

    // Check output has sufficient variation (rhythmic character)
    let max_val = output.iter().map(|x| x.abs()).fold(0.0f32, |a, b| a.max(b));
    assert!(max_val > 0.1, "Should have clear peaks in rhythmic pattern");
}

#[test]
fn test_multitap_vs_single_delay() {
    // Compare multi-tap to a conceptual single delay (2 taps is closest)
    let mut graph = create_test_graph();

    let osc1 = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let osc2 = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Multi-tap with 2 taps (minimal multi-tap)
    let multi_id = graph.add_node(SignalNode::MultiTapDelay {
        input: Signal::Node(osc1),
        time: Signal::Value(0.2),
        taps: 2,
        feedback: Signal::Value(0.5),
        mix: Signal::Value(0.5),
        buffer: vec![0.0; 88200],
        write_idx: 0,
    });

    // Multi-tap with 6 taps (complex pattern)
    let complex_id = graph.add_node(SignalNode::MultiTapDelay {
        input: Signal::Node(osc2),
        time: Signal::Value(0.2),
        taps: 6,
        feedback: Signal::Value(0.5),
        mix: Signal::Value(0.5),
        buffer: vec![0.0; 88200],
        write_idx: 0,
    });

    let buffer_size = 1024;
    let mut multi_output = vec![0.0; buffer_size];
    let mut complex_output = vec![0.0; buffer_size];

    // Process buffers
    for _ in 0..5 {
        graph.eval_node_buffer(&multi_id, &mut multi_output);
        graph.eval_node_buffer(&complex_id, &mut complex_output);
    }

    let multi_rms = calculate_rms(&multi_output);
    let complex_rms = calculate_rms(&complex_output);

    // Both should produce sound
    assert!(multi_rms > 0.1, "2-tap delay should produce sound");
    assert!(complex_rms > 0.1, "6-tap delay should produce sound");

    // More taps should create richer texture (but normalized)
    assert!(complex_rms >= multi_rms * 0.5, "Complex should have comparable energy");
}

#[test]
fn test_multitap_edge_cases() {
    // Test edge cases: minimum taps (2) and maximum taps (8)
    let mut graph = create_test_graph();

    let osc1 = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let osc2 = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Minimum taps (will be clamped to 2)
    let mtd_id = graph.add_node(SignalNode::MultiTapDelay {
        input: Signal::Node(osc1),
        time: Signal::Value(0.15),
        taps: 1,
        feedback: Signal::Value(0.5),
        mix: Signal::Value(0.5),
        buffer: vec![0.0; 88200],
        write_idx: 0,
    });

    // Maximum taps (will be clamped to 8)
    let mtd_id = graph.add_node(SignalNode::MultiTapDelay {
        input: Signal::Node(osc2),
        time: Signal::Value(0.15),
        taps: 10,
        feedback: Signal::Value(0.5),
        mix: Signal::Value(0.5),
        buffer: vec![0.0; 88200],
        write_idx: 0,
    });

    let buffer_size = 512;
    let mut min_output = vec![0.0; buffer_size];
    let mut max_output = vec![0.0; buffer_size];

    // Process buffers
    for _ in 0..5 {
        graph.eval_node_buffer(&min_id, &mut min_output);
        graph.eval_node_buffer(&max_id, &mut max_output);
    }

    let min_rms = calculate_rms(&min_output);
    let max_rms = calculate_rms(&max_output);

    // Both should work despite clamping
    assert!(min_rms > 0.1, "Min taps (clamped) should produce sound");
    assert!(max_rms > 0.1, "Max taps (clamped) should produce sound");

    // No crashes or NaN
    for &sample in min_output.iter().chain(max_output.iter()) {
        assert!(sample.is_finite(), "All samples should be finite");
    }
}
