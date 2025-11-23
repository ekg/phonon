/// Tests for Expander buffer-based evaluation
///
/// These tests verify that Expander buffer evaluation produces correct
/// dynamics processing behavior (upward expansion) and maintains proper state continuity.

use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph, Waveform, ExpanderState};

/// Helper: Create a test graph
fn create_test_graph() -> UnifiedSignalGraph {
    UnifiedSignalGraph::new(44100.0)
}

/// Helper: Calculate RMS of a buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Helper: Calculate peak value in a buffer
fn calculate_peak(buffer: &[f32]) -> f32 {
    buffer.iter().map(|&x| x.abs()).fold(0.0, f32::max)
}

/// Helper: Generate loud signal (above threshold)
fn generate_loud_signal(graph: &mut UnifiedSignalGraph, amplitude: f32) -> Signal {
    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let scaled = graph.add_multiply_node(Signal::Node(osc), Signal::Value(amplitude));
    Signal::Node(scaled)
}

/// Helper: Generate quiet signal (below threshold)
fn generate_quiet_signal(graph: &mut UnifiedSignalGraph, amplitude: f32) -> Signal {
    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let scaled = graph.add_multiply_node(Signal::Node(osc), Signal::Value(amplitude));
    Signal::Node(scaled)
}

// ============================================================================
// TEST 1: Basic Expansion - Boosts Loud Signals
// ============================================================================

#[test]
fn test_expander_boosts_loud_signals() {
    let mut graph = create_test_graph();

    // Create loud signal (well above threshold)
    let loud_signal = generate_loud_signal(&mut graph, 0.8);

    // Expander: threshold = -20 dB, ratio = 2:1
    let exp_id = graph.add_node(SignalNode::Expander {
            input: loud_signal.clone(),
            threshold: Signal::Value(-20.0),
            ratio: Signal::Value(2.0),
            attack: Signal::Value(0.001),
            release: Signal::Value(0.1),
            state: ExpanderState::new(),
        });

    let buffer_size = 4410; // 100ms at 44.1kHz
    let mut expanded = vec![0.0; buffer_size];
    let mut unexpanded = vec![0.0; buffer_size];

    // Get unexpanded signal
    if let Signal::Node(sig_id) = loud_signal {
        graph.eval_node_buffer(&sig_id, &mut unexpanded);
    }

    // Get expanded signal
    graph.eval_node_buffer(&exp_id, &mut expanded);

    // Expanded should have HIGHER RMS than unexpanded (opposite of compressor)
    let unexpanded_rms = calculate_rms(&unexpanded);
    let expanded_rms = calculate_rms(&expanded);

    assert!(expanded_rms > unexpanded_rms,
        "Expander should boost loud signals: unexpanded RMS = {}, expanded RMS = {}",
        unexpanded_rms, expanded_rms);
}

// ============================================================================
// TEST 2: Quiet Signals Pass Through Unchanged
// ============================================================================

#[test]
fn test_expander_passes_quiet_signals() {
    let mut graph = create_test_graph();

    // Create quiet signal (below threshold)
    let quiet_signal = generate_quiet_signal(&mut graph, 0.05);

    // Expander: threshold = -20 dB
    let exp_id = graph.add_node(SignalNode::Expander {
            input: quiet_signal.clone(),
            threshold: Signal::Value(-20.0),
            ratio: Signal::Value(2.0),
            attack: Signal::Value(0.001),
            release: Signal::Value(0.1),
            state: ExpanderState::new(),
        });

    let buffer_size = 4410;
    let mut expanded = vec![0.0; buffer_size];
    let mut unexpanded = vec![0.0; buffer_size];

    // Get unexpanded signal
    if let Signal::Node(sig_id) = quiet_signal {
        graph.eval_node_buffer(&sig_id, &mut unexpanded);
    }

    // Get expanded signal
    graph.eval_node_buffer(&exp_id, &mut expanded);

    // Expanded should be similar to unexpanded (below threshold)
    let unexpanded_rms = calculate_rms(&unexpanded);
    let expanded_rms = calculate_rms(&expanded);

    // Within 10% (slight envelope follower lag is OK)
    assert!((expanded_rms - unexpanded_rms).abs() < unexpanded_rms * 0.1,
        "Expander should pass quiet signals: unexpanded RMS = {}, expanded RMS = {}",
        unexpanded_rms, expanded_rms);
}

// ============================================================================
// TEST 3: Threshold Effect
// ============================================================================

#[test]
fn test_expander_threshold_effect() {
    let mut graph = create_test_graph();

    // Medium level signal
    let signal = generate_loud_signal(&mut graph, 0.4);

    // High threshold (-10 dB) - should expand less
    let exp_high_thresh = graph.add_node(SignalNode::Expander {
            input: signal.clone(),
            threshold: Signal::Value(-10.0),
            ratio: Signal::Value(2.0),
            attack: Signal::Value(0.001),
            release: Signal::Value(0.1),
            state: ExpanderState::new(),
        });

    // Low threshold (-30 dB) - should expand more
    let exp_low_thresh = graph.add_node(SignalNode::Expander {
            input: signal,
            threshold: Signal::Value(-30.0),
            ratio: Signal::Value(2.0),
            attack: Signal::Value(0.001),
            release: Signal::Value(0.1),
            state: ExpanderState::new(),
        });

    let buffer_size = 4410;
    let mut high_thresh_output = vec![0.0; buffer_size];
    let mut low_thresh_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&exp_high_thresh, &mut high_thresh_output);
    graph.eval_node_buffer(&exp_low_thresh, &mut low_thresh_output);

    let high_thresh_rms = calculate_rms(&high_thresh_output);
    let low_thresh_rms = calculate_rms(&low_thresh_output);

    // Lower threshold should expand more (higher RMS)
    assert!(low_thresh_rms > high_thresh_rms,
        "Lower threshold should expand more: high threshold RMS = {}, low threshold RMS = {}",
        high_thresh_rms, low_thresh_rms);
}

// ============================================================================
// TEST 4: Ratio Effect
// ============================================================================

#[test]
fn test_expander_ratio_effect() {
    let mut graph = create_test_graph();

    let loud_signal = generate_loud_signal(&mut graph, 0.8);

    // Low ratio (2:1) - gentle expansion
    let exp_low_ratio = graph.add_node(SignalNode::Expander {
            input: loud_signal.clone(),
            threshold: Signal::Value(-20.0),
            ratio: Signal::Value(2.0),
            attack: Signal::Value(0.001),
            release: Signal::Value(0.1),
            state: ExpanderState::new(),
        });

    // High ratio (5:1) - aggressive expansion
    let exp_high_ratio = graph.add_node(SignalNode::Expander {
            input: loud_signal,
            threshold: Signal::Value(-20.0),
            ratio: Signal::Value(5.0),
            attack: Signal::Value(0.001),
            release: Signal::Value(0.1),
            state: ExpanderState::new(),
        });

    let buffer_size = 4410;
    let mut low_ratio_output = vec![0.0; buffer_size];
    let mut high_ratio_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&exp_low_ratio, &mut low_ratio_output);
    graph.eval_node_buffer(&exp_high_ratio, &mut high_ratio_output);

    let low_ratio_rms = calculate_rms(&low_ratio_output);
    let high_ratio_rms = calculate_rms(&high_ratio_output);

    // Higher ratio should expand more (higher RMS)
    assert!(high_ratio_rms > low_ratio_rms,
        "Higher ratio should expand more: low ratio RMS = {}, high ratio RMS = {}",
        low_ratio_rms, high_ratio_rms);
}

// ============================================================================
// TEST 5: Attack Time
// ============================================================================

#[test]
fn test_expander_attack_time() {
    let mut graph = create_test_graph();

    let loud_signal = generate_loud_signal(&mut graph, 0.8);

    // Fast attack (0.001s) - quick response
    let exp_fast = graph.add_node(SignalNode::Expander {
            input: loud_signal.clone(),
            threshold: Signal::Value(-20.0),
            ratio: Signal::Value(2.0),
            attack: Signal::Value(0.001),
            release: Signal::Value(0.1),
            state: ExpanderState::new(),
        });

    // Slow attack (0.1s) - slower response
    let exp_slow = graph.add_node(SignalNode::Expander {
            input: loud_signal,
            threshold: Signal::Value(-20.0),
            ratio: Signal::Value(2.0),
            attack: Signal::Value(0.1),
            release: Signal::Value(0.1),
            state: ExpanderState::new(),
        });

    let buffer_size = 4410; // 100ms
    let mut fast_output = vec![0.0; buffer_size];
    let mut slow_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&exp_fast, &mut fast_output);
    graph.eval_node_buffer(&exp_slow, &mut slow_output);

    // Measure RMS in first 10ms (attack phase)
    let early_samples = 441; // 10ms
    let fast_early_rms = calculate_rms(&fast_output[..early_samples]);
    let slow_early_rms = calculate_rms(&slow_output[..early_samples]);

    // Fast attack should expand more in the early phase
    assert!(fast_early_rms > slow_early_rms,
        "Fast attack should expand more quickly: fast = {}, slow = {}",
        fast_early_rms, slow_early_rms);
}

// ============================================================================
// TEST 6: Release Time
// ============================================================================

#[test]
fn test_expander_release_time() {
    let mut graph = create_test_graph();

    let loud_signal = generate_loud_signal(&mut graph, 0.8);

    // Fast release
    let exp_fast_rel = graph.add_node(SignalNode::Expander {
            input: loud_signal.clone(),
            threshold: Signal::Value(-20.0),
            ratio: Signal::Value(2.0),
            attack: Signal::Value(0.001),
            release: Signal::Value(0.01),
            state: ExpanderState::new(),
        });

    // Slow release
    let exp_slow_rel = graph.add_node(SignalNode::Expander {
            input: loud_signal,
            threshold: Signal::Value(-20.0),
            ratio: Signal::Value(2.0),
            attack: Signal::Value(0.001),
            release: Signal::Value(0.5),
            state: ExpanderState::new(),
        });

    let buffer_size = 4410;
    let mut fast_output = vec![0.0; buffer_size];
    let mut slow_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&exp_fast_rel, &mut fast_output);
    graph.eval_node_buffer(&exp_slow_rel, &mut slow_output);

    // Both should produce sound
    let fast_rms = calculate_rms(&fast_output);
    let slow_rms = calculate_rms(&slow_output);

    assert!(fast_rms > 0.1 && slow_rms > 0.1,
        "Both fast and slow release should produce sound: fast = {}, slow = {}",
        fast_rms, slow_rms);
}

// ============================================================================
// TEST 7: State Continuity
// ============================================================================

#[test]
fn test_expander_state_continuity() {
    let mut graph = create_test_graph();

    let loud_signal = generate_loud_signal(&mut graph, 0.8);

    let exp_id = graph.add_node(SignalNode::Expander {
            input: loud_signal,
            threshold: Signal::Value(-20.0),
            ratio: Signal::Value(2.0),
            attack: Signal::Value(0.01),
            release: Signal::Value(0.1),
            state: ExpanderState::new(),
        });

    // Generate two consecutive buffers
    let buffer_size = 512;
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&exp_id, &mut buffer1);
    graph.eval_node_buffer(&exp_id, &mut buffer2);

    // Check continuity at boundary
    let last_sample = buffer1[buffer_size - 1];
    let first_sample = buffer2[0];
    let discontinuity = (first_sample - last_sample).abs();

    // Should be continuous (envelope follower maintains state)
    assert!(discontinuity < 0.2,
        "Expander state should be continuous across buffers, discontinuity = {}",
        discontinuity);
}

// ============================================================================
// TEST 8: Multiple Buffer Evaluation
// ============================================================================

#[test]
fn test_expander_multiple_buffers() {
    let mut graph = create_test_graph();

    let loud_signal = generate_loud_signal(&mut graph, 0.8);
    let exp_id = graph.add_node(SignalNode::Expander {
            input: loud_signal,
            threshold: Signal::Value(-20.0),
            ratio: Signal::Value(2.0),
            attack: Signal::Value(0.001),
            release: Signal::Value(0.1),
            state: ExpanderState::new(),
        });

    // Generate 10 consecutive buffers
    let buffer_size = 512;
    let num_buffers = 10;

    for i in 0..num_buffers {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&exp_id, &mut output);

        // Each buffer should have reasonable audio
        let rms = calculate_rms(&output);
        assert!(rms > 0.1 && rms < 5.0,
            "Buffer {} has unexpected RMS: {}", i, rms);

        // Check for no NaN/Inf
        for (j, &sample) in output.iter().enumerate() {
            assert!(sample.is_finite(),
                "Buffer {} sample {} is non-finite: {}", i, j, sample);
        }
    }
}

// ============================================================================
// TEST 9: Modulated Threshold Parameter
// ============================================================================

#[test]
fn test_expander_modulated_threshold() {
    let mut graph = create_test_graph();

    let loud_signal = generate_loud_signal(&mut graph, 0.8);

    // LFO to modulate threshold
    let lfo_id = graph.add_oscillator(Signal::Value(0.5), Waveform::Sine);
    let lfo_scaled = graph.add_multiply_node(Signal::Node(lfo_id), Signal::Value(10.0));
    let threshold_signal = graph.add_add_node(Signal::Node(lfo_scaled), Signal::Value(-20.0));

    let exp_id = graph.add_node(SignalNode::Expander {
            input: loud_signal,
            threshold: Signal::Node(threshold_signal),
            ratio: Signal::Value(2.0),
            attack: Signal::Value(0.001),
            release: Signal::Value(0.1),
            state: ExpanderState::new(),
        });

    let buffer_size = 4410;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&exp_id, &mut output);

    // Should produce sound (modulated threshold)
    let rms = calculate_rms(&output);
    assert!(rms > 0.3,
        "Modulated threshold expander should produce sound, RMS = {}", rms);
}

// ============================================================================
// TEST 10: Extreme Ratios
// ============================================================================

#[test]
fn test_expander_extreme_ratios() {
    let mut graph = create_test_graph();

    let loud_signal = generate_loud_signal(&mut graph, 0.8);

    // Minimum ratio (1:1 - no expansion)
    let exp_min = graph.add_node(SignalNode::Expander {
            input: loud_signal.clone(),
            threshold: Signal::Value(-20.0),
            ratio: Signal::Value(1.0),
            attack: Signal::Value(0.001),
            release: Signal::Value(0.1),
            state: ExpanderState::new(),
        });

    // Maximum ratio (10:1 - aggressive expansion)
    let exp_max = graph.add_node(SignalNode::Expander {
            input: loud_signal,
            threshold: Signal::Value(-20.0),
            ratio: Signal::Value(10.0),
            attack: Signal::Value(0.001),
            release: Signal::Value(0.1),
            state: ExpanderState::new(),
        });

    let buffer_size = 4410;
    let mut min_output = vec![0.0; buffer_size];
    let mut max_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&exp_min, &mut min_output);
    graph.eval_node_buffer(&exp_max, &mut max_output);

    // Check no NaN/Inf values
    for &sample in &min_output {
        assert!(sample.is_finite(), "Min ratio produced non-finite value");
    }
    for &sample in &max_output {
        assert!(sample.is_finite(), "Max ratio produced non-finite value");
    }

    let min_rms = calculate_rms(&min_output);
    let max_rms = calculate_rms(&max_output);

    // Max ratio should expand more
    assert!(max_rms > min_rms,
        "Higher ratio should expand more: 1:1 RMS = {}, 10:1 RMS = {}",
        min_rms, max_rms);
}

// ============================================================================
// TEST 11: Silent Input
// ============================================================================

#[test]
fn test_expander_no_input() {
    let mut graph = create_test_graph();

    // Silent input
    let exp_id = graph.add_node(SignalNode::Expander {
            input: Signal::Value(0.0),
            threshold: Signal::Value(-20.0),
            ratio: Signal::Value(2.0),
            attack: Signal::Value(0.001),
            release: Signal::Value(0.1),
            state: ExpanderState::new(),
        });

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&exp_id, &mut output);

    // Should produce silence (no division by zero, no NaN)
    for (i, &sample) in output.iter().enumerate() {
        assert!(sample.is_finite(), "Sample {} is non-finite: {}", i, sample);
        assert_eq!(sample, 0.0, "Sample {} should be silent", i);
    }
}

// ============================================================================
// TEST 12: Performance Benchmark
// ============================================================================

#[test]
fn test_expander_buffer_performance() {
    let mut graph = create_test_graph();

    let loud_signal = generate_loud_signal(&mut graph, 0.8);
    let exp_id = graph.add_node(SignalNode::Expander {
            input: loud_signal,
            threshold: Signal::Value(-20.0),
            ratio: Signal::Value(2.0),
            attack: Signal::Value(0.001),
            release: Signal::Value(0.1),
            state: ExpanderState::new(),
        });

    let buffer_size = 512;
    let iterations = 1000;

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&exp_id, &mut output);
    }
    let duration = start.elapsed();

    println!("Expander buffer eval: {:?} for {} iterations", duration, iterations);
    println!("Per iteration: {:?}", duration / iterations);

    // Should complete in reasonable time (< 2 seconds for 1000 iterations)
    assert!(duration.as_secs() < 2,
        "Expander buffer evaluation too slow: {:?}", duration);
}

// ============================================================================
// TEST 13: Constant Parameters
// ============================================================================

#[test]
fn test_expander_constant_parameters() {
    let mut graph = create_test_graph();

    let loud_signal = generate_loud_signal(&mut graph, 0.8);

    // All constant parameters
    let exp_id = graph.add_node(SignalNode::Expander {
            input: loud_signal,
            threshold: Signal::Value(-20.0),
            ratio: Signal::Value(2.0),
            attack: Signal::Value(0.001),
            release: Signal::Value(0.1),
            state: ExpanderState::new(),
        });

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&exp_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.3,
        "Expander with constant parameters should work, RMS = {}", rms);
}

// ============================================================================
// TEST 14: Modulated Ratio Parameter
// ============================================================================

#[test]
fn test_expander_modulated_ratio() {
    let mut graph = create_test_graph();

    let loud_signal = generate_loud_signal(&mut graph, 0.8);

    // LFO to modulate ratio (2:1 to 4:1)
    let lfo_id = graph.add_oscillator(Signal::Value(0.5), Waveform::Sine);
    let lfo_scaled = graph.add_multiply_node(Signal::Node(lfo_id), Signal::Value(1.0));
    let ratio_signal = graph.add_add_node(Signal::Node(lfo_scaled), Signal::Value(3.0));

    let exp_id = graph.add_node(SignalNode::Expander {
            input: loud_signal,
            threshold: Signal::Value(-20.0),
            ratio: Signal::Node(ratio_signal),
            attack: Signal::Value(0.001),
            release: Signal::Value(0.1),
            state: ExpanderState::new(),
        });

    let buffer_size = 4410;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&exp_id, &mut output);

    // Should produce sound (modulated ratio)
    let rms = calculate_rms(&output);
    assert!(rms > 0.3,
        "Modulated ratio expander should produce sound, RMS = {}", rms);
}
