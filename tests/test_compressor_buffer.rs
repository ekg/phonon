/// Tests for Compressor buffer-based evaluation
///
/// These tests verify that Compressor buffer evaluation produces correct
/// dynamics processing behavior and maintains proper state continuity.

use phonon::unified_graph::{Signal, UnifiedSignalGraph, Waveform, SignalNode, CompressorState};

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
// TEST: Basic Compression
// ============================================================================

#[test]
fn test_compressor_reduces_loud_signals() {
    let mut graph = create_test_graph();

    // Create loud signal (well above threshold)
    let loud_signal = generate_loud_signal(&mut graph, 0.8);

    // Compressor: threshold = -20 dB, ratio = 4:1
    let comp_id = graph.add_node(SignalNode::Compressor {
        input: loud_signal.clone(),
        threshold: Signal::Value(-20.0),  // threshold (dB)
        ratio: Signal::Value(4.0),     // ratio
        attack: Signal::Value(0.001),   // attack
        release: Signal::Value(0.1),     // release
        makeup_gain: Signal::Value(0.0),     // makeup gain
        state: CompressorState::new(),
    });

    let buffer_size = 4410; // 100ms at 44.1kHz
    let mut compressed = vec![0.0; buffer_size];
    let mut uncompressed = vec![0.0; buffer_size];

    // Get uncompressed signal
    if let Signal::Node(sig_id) = loud_signal {
        graph.eval_node_buffer(&sig_id, &mut uncompressed);
    }

    // Get compressed signal
    graph.eval_node_buffer(&comp_id, &mut compressed);

    // Compressed should have lower RMS than uncompressed
    let uncompressed_rms = calculate_rms(&uncompressed);
    let compressed_rms = calculate_rms(&compressed);

    assert!(compressed_rms < uncompressed_rms,
        "Compressor should reduce loud signals: uncompressed RMS = {}, compressed RMS = {}",
        uncompressed_rms, compressed_rms);
}

#[test]
fn test_compressor_passes_quiet_signals() {
    let mut graph = create_test_graph();

    // Create quiet signal (below threshold)
    let quiet_signal = generate_quiet_signal(&mut graph, 0.05);

    // Compressor: threshold = -20 dB
    let comp_id = graph.add_node(SignalNode::Compressor {
        input: quiet_signal.clone(),
        threshold: Signal::Value(-20.0),
        ratio: Signal::Value(4.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
        makeup_gain: Signal::Value(0.0),
        state: CompressorState::new(),
    });

    let buffer_size = 4410;
    let mut compressed = vec![0.0; buffer_size];
    let mut uncompressed = vec![0.0; buffer_size];

    // Get uncompressed signal
    if let Signal::Node(sig_id) = quiet_signal {
        graph.eval_node_buffer(&sig_id, &mut uncompressed);
    }

    // Get compressed signal
    graph.eval_node_buffer(&comp_id, &mut compressed);

    // Compressed should be similar to uncompressed (below threshold)
    let uncompressed_rms = calculate_rms(&uncompressed);
    let compressed_rms = calculate_rms(&compressed);

    // Within 10% (slight envelope follower lag is OK)
    assert!((compressed_rms - uncompressed_rms).abs() < uncompressed_rms * 0.1,
        "Compressor should pass quiet signals: uncompressed RMS = {}, compressed RMS = {}",
        uncompressed_rms, compressed_rms);
}

// ============================================================================
// TEST: Threshold Effect
// ============================================================================

#[test]
fn test_compressor_threshold_effect() {
    let mut graph = create_test_graph();

    // Medium level signal
    let signal = generate_loud_signal(&mut graph, 0.4);

    // High threshold (-10 dB) - should compress less
    let comp_high_thresh = graph.add_node(SignalNode::Compressor {
        input: signal.clone(),
        threshold: Signal::Value(-10.0),
        ratio: Signal::Value(4.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
        makeup_gain: Signal::Value(0.0),
        state: CompressorState::new(),
    });

    // Low threshold (-30 dB) - should compress more
    let comp_low_thresh = graph.add_node(SignalNode::Compressor {
        input: signal,
        threshold: Signal::Value(-30.0),
        ratio: Signal::Value(4.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
        makeup_gain: Signal::Value(0.0),
        state: CompressorState::new(),
    });

    let buffer_size = 4410;
    let mut high_thresh_output = vec![0.0; buffer_size];
    let mut low_thresh_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&comp_high_thresh, &mut high_thresh_output);
    graph.eval_node_buffer(&comp_low_thresh, &mut low_thresh_output);

    let high_thresh_rms = calculate_rms(&high_thresh_output);
    let low_thresh_rms = calculate_rms(&low_thresh_output);

    // Lower threshold should compress more (lower RMS)
    assert!(low_thresh_rms < high_thresh_rms,
        "Lower threshold should compress more: high threshold RMS = {}, low threshold RMS = {}",
        high_thresh_rms, low_thresh_rms);
}

// ============================================================================
// TEST: Ratio Effect
// ============================================================================

#[test]
fn test_compressor_ratio_effect() {
    let mut graph = create_test_graph();

    let loud_signal = generate_loud_signal(&mut graph, 0.8);

    // Low ratio (2:1) - gentle compression
    let comp_low_ratio = graph.add_node(SignalNode::Compressor {
        input: loud_signal.clone(),
        threshold: Signal::Value(-20.0),
        ratio: Signal::Value(2.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
        makeup_gain: Signal::Value(0.0),
        state: CompressorState::new(),
    });

    // High ratio (10:1) - aggressive compression
    let comp_high_ratio = graph.add_node(SignalNode::Compressor {
        input: loud_signal,
        threshold: Signal::Value(-20.0),
        ratio: Signal::Value(10.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
        makeup_gain: Signal::Value(0.0),
        state: CompressorState::new(),
    });

    let buffer_size = 4410;
    let mut low_ratio_output = vec![0.0; buffer_size];
    let mut high_ratio_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&comp_low_ratio, &mut low_ratio_output);
    graph.eval_node_buffer(&comp_high_ratio, &mut high_ratio_output);

    let low_ratio_rms = calculate_rms(&low_ratio_output);
    let high_ratio_rms = calculate_rms(&high_ratio_output);

    // Higher ratio should compress more (lower RMS)
    assert!(high_ratio_rms < low_ratio_rms,
        "Higher ratio should compress more: low ratio RMS = {}, high ratio RMS = {}",
        low_ratio_rms, high_ratio_rms);
}

// ============================================================================
// TEST: Attack/Release
// ============================================================================

#[test]
fn test_compressor_attack_time() {
    let mut graph = create_test_graph();

    let loud_signal = generate_loud_signal(&mut graph, 0.8);

    // Fast attack (0.001s) - quick response
    let comp_fast = graph.add_node(SignalNode::Compressor {
        input: loud_signal.clone(),
        threshold: Signal::Value(-20.0),
        ratio: Signal::Value(4.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
        makeup_gain: Signal::Value(0.0),
        state: CompressorState::new(),
    });

    // Slow attack (0.1s) - slower response
    let comp_slow = graph.add_node(SignalNode::Compressor {
        input: loud_signal,
        threshold: Signal::Value(-20.0),
        ratio: Signal::Value(4.0),
        attack: Signal::Value(0.1),
        release: Signal::Value(0.1),
        makeup_gain: Signal::Value(0.0),
        state: CompressorState::new(),
    });

    let buffer_size = 4410; // 100ms
    let mut fast_output = vec![0.0; buffer_size];
    let mut slow_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&comp_fast, &mut fast_output);
    graph.eval_node_buffer(&comp_slow, &mut slow_output);

    // Measure RMS in first 10ms (attack phase)
    let early_samples = 441; // 10ms
    let fast_early_rms = calculate_rms(&fast_output[..early_samples]);
    let slow_early_rms = calculate_rms(&slow_output[..early_samples]);

    // Fast attack should compress more in the early phase
    assert!(fast_early_rms < slow_early_rms,
        "Fast attack should compress more quickly: fast = {}, slow = {}",
        fast_early_rms, slow_early_rms);
}

#[test]
fn test_compressor_release_time() {
    let mut graph = create_test_graph();

    let loud_signal = generate_loud_signal(&mut graph, 0.8);

    // Fast release
    let comp_fast_rel = graph.add_node(SignalNode::Compressor {
        input: loud_signal.clone(),
        threshold: Signal::Value(-20.0),
        ratio: Signal::Value(4.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.01),
        makeup_gain: Signal::Value(0.0),
        state: CompressorState::new(),
    });

    // Slow release
    let comp_slow_rel = graph.add_node(SignalNode::Compressor {
        input: loud_signal,
        threshold: Signal::Value(-20.0),
        ratio: Signal::Value(4.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.5),
        makeup_gain: Signal::Value(0.0),
        state: CompressorState::new(),
    });

    let buffer_size = 4410;
    let mut fast_output = vec![0.0; buffer_size];
    let mut slow_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&comp_fast_rel, &mut fast_output);
    graph.eval_node_buffer(&comp_slow_rel, &mut slow_output);

    // Both should produce sound
    let fast_rms = calculate_rms(&fast_output);
    let slow_rms = calculate_rms(&slow_output);

    assert!(fast_rms > 0.01 && slow_rms > 0.01,
        "Both fast and slow release should produce sound: fast = {}, slow = {}",
        fast_rms, slow_rms);
}

// ============================================================================
// TEST: Makeup Gain
// ============================================================================

#[test]
fn test_compressor_makeup_gain() {
    let mut graph = create_test_graph();

    let loud_signal = generate_loud_signal(&mut graph, 0.8);

    // No makeup gain
    let comp_no_makeup = graph.add_node(SignalNode::Compressor {
        input: loud_signal.clone(),
        threshold: Signal::Value(-20.0),
        ratio: Signal::Value(4.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
        makeup_gain: Signal::Value(0.0),
        state: CompressorState::new(),
    });

    // With makeup gain (10 dB)
    let comp_with_makeup = graph.add_node(SignalNode::Compressor {
        input: loud_signal,
        threshold: Signal::Value(-20.0),
        ratio: Signal::Value(4.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
        makeup_gain: Signal::Value(10.0),
        state: CompressorState::new(),
    });

    let buffer_size = 4410;
    let mut no_makeup = vec![0.0; buffer_size];
    let mut with_makeup = vec![0.0; buffer_size];

    graph.eval_node_buffer(&comp_no_makeup, &mut no_makeup);
    graph.eval_node_buffer(&comp_with_makeup, &mut with_makeup);

    let no_makeup_rms = calculate_rms(&no_makeup);
    let with_makeup_rms = calculate_rms(&with_makeup);

    // Makeup gain should boost the signal
    assert!(with_makeup_rms > no_makeup_rms * 2.0,
        "Makeup gain should boost signal: no makeup = {}, with makeup = {}",
        no_makeup_rms, with_makeup_rms);
}

// ============================================================================
// TEST: State Continuity
// ============================================================================

#[test]
fn test_compressor_state_continuity() {
    let mut graph = create_test_graph();

    let loud_signal = generate_loud_signal(&mut graph, 0.8);

    let comp_id = graph.add_node(SignalNode::Compressor {
        input: loud_signal,
        threshold: Signal::Value(-20.0),
        ratio: Signal::Value(4.0),
        attack: Signal::Value(0.01),
        release: Signal::Value(0.1),
        makeup_gain: Signal::Value(0.0),
        state: CompressorState::new(),
    });

    // Generate two consecutive buffers
    let buffer_size = 512;
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&comp_id, &mut buffer1);
    graph.eval_node_buffer(&comp_id, &mut buffer2);

    // Check continuity at boundary
    let last_sample = buffer1[buffer_size - 1];
    let first_sample = buffer2[0];
    let discontinuity = (first_sample - last_sample).abs();

    // Should be continuous (envelope follower maintains state)
    assert!(discontinuity < 0.2,
        "Compressor state should be continuous across buffers, discontinuity = {}",
        discontinuity);
}

// ============================================================================
// TEST: Multiple Buffer Evaluation
// ============================================================================

#[test]
fn test_compressor_multiple_buffers() {
    let mut graph = create_test_graph();

    let loud_signal = generate_loud_signal(&mut graph, 0.8);
    let comp_id = graph.add_node(SignalNode::Compressor {
        input: loud_signal,
        threshold: Signal::Value(-20.0),
        ratio: Signal::Value(4.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
        makeup_gain: Signal::Value(0.0),
        state: CompressorState::new(),
    });

    // Generate 10 consecutive buffers
    let buffer_size = 512;
    let num_buffers = 10;

    for i in 0..num_buffers {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&comp_id, &mut output);

        // Each buffer should have reasonable audio
        let rms = calculate_rms(&output);
        assert!(rms > 0.01 && rms < 1.0,
            "Buffer {} has unexpected RMS: {}", i, rms);

        // Check for no NaN/Inf
        for (j, &sample) in output.iter().enumerate() {
            assert!(sample.is_finite(),
                "Buffer {} sample {} is non-finite: {}", i, j, sample);
        }
    }
}

// ============================================================================
// TEST: Modulated Parameters
// ============================================================================

#[test]
fn test_compressor_modulated_threshold() {
    let mut graph = create_test_graph();

    let loud_signal = generate_loud_signal(&mut graph, 0.8);

    // LFO to modulate threshold
    let lfo_id = graph.add_oscillator(Signal::Value(0.5), Waveform::Sine);
    let lfo_scaled = graph.add_multiply_node(Signal::Node(lfo_id), Signal::Value(10.0));
    let threshold_signal = graph.add_add_node(Signal::Node(lfo_scaled), Signal::Value(-20.0));

    let comp_id = graph.add_node(SignalNode::Compressor {
        input: loud_signal,
        threshold: Signal::Node(threshold_signal),
        ratio: Signal::Value(4.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
        makeup_gain: Signal::Value(0.0),
        state: CompressorState::new(),
    });

    let buffer_size = 4410;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&comp_id, &mut output);

    // Should produce sound (modulated threshold)
    let rms = calculate_rms(&output);
    assert!(rms > 0.1,
        "Modulated threshold compressor should produce sound, RMS = {}", rms);
}

// ============================================================================
// TEST: Edge Cases
// ============================================================================

#[test]
fn test_compressor_extreme_ratios() {
    let mut graph = create_test_graph();

    let loud_signal = generate_loud_signal(&mut graph, 0.8);

    // Minimum ratio (1:1 - no compression)
    let comp_min = graph.add_node(SignalNode::Compressor {
        input: loud_signal.clone(),
        threshold: Signal::Value(-20.0),
        ratio: Signal::Value(1.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
        makeup_gain: Signal::Value(0.0),
        state: CompressorState::new(),
    });

    // Maximum ratio (20:1 - near limiting)
    let comp_max = graph.add_node(SignalNode::Compressor {
        input: loud_signal,
        threshold: Signal::Value(-20.0),
        ratio: Signal::Value(20.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
        makeup_gain: Signal::Value(0.0),
        state: CompressorState::new(),
    });

    let buffer_size = 4410;
    let mut min_output = vec![0.0; buffer_size];
    let mut max_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&comp_min, &mut min_output);
    graph.eval_node_buffer(&comp_max, &mut max_output);

    // Check no NaN/Inf values
    for &sample in &min_output {
        assert!(sample.is_finite(), "Min ratio produced non-finite value");
    }
    for &sample in &max_output {
        assert!(sample.is_finite(), "Max ratio produced non-finite value");
    }

    let min_rms = calculate_rms(&min_output);
    let max_rms = calculate_rms(&max_output);

    // Max ratio should compress more
    assert!(max_rms < min_rms,
        "Higher ratio should compress more: 1:1 RMS = {}, 20:1 RMS = {}",
        min_rms, max_rms);
}

#[test]
fn test_compressor_no_input() {
    let mut graph = create_test_graph();

    // Silent input
    let comp_id = graph.add_node(SignalNode::Compressor {
        input: Signal::Value(0.0),
        threshold: Signal::Value(-20.0),
        ratio: Signal::Value(4.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
        makeup_gain: Signal::Value(0.0),
        state: CompressorState::new(),
    });

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&comp_id, &mut output);

    // Should produce silence (no division by zero, no NaN)
    for (i, &sample) in output.iter().enumerate() {
        assert!(sample.is_finite(), "Sample {} is non-finite: {}", i, sample);
        assert_eq!(sample, 0.0, "Sample {} should be silent", i);
    }
}

// ============================================================================
// TEST: Performance
// ============================================================================

#[test]
fn test_compressor_buffer_performance() {
    let mut graph = create_test_graph();

    let loud_signal = generate_loud_signal(&mut graph, 0.8);
    let comp_id = graph.add_node(SignalNode::Compressor {
        input: loud_signal,
        threshold: Signal::Value(-20.0),
        ratio: Signal::Value(4.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
        makeup_gain: Signal::Value(0.0),
        state: CompressorState::new(),
    });

    let buffer_size = 512;
    let iterations = 1000;

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&comp_id, &mut output);
    }
    let duration = start.elapsed();

    println!("Compressor buffer eval: {:?} for {} iterations", duration, iterations);
    println!("Per iteration: {:?}", duration / iterations);

    // Should complete in reasonable time (< 2 seconds for 1000 iterations)
    assert!(duration.as_secs() < 2,
        "Compressor buffer evaluation too slow: {:?}", duration);
}

// ============================================================================
// TEST: Constant vs Signal Parameters
// ============================================================================

#[test]
fn test_compressor_constant_parameters() {
    let mut graph = create_test_graph();

    let loud_signal = generate_loud_signal(&mut graph, 0.8);

    // All constant parameters
    let comp_id = graph.add_node(SignalNode::Compressor {
        input: loud_signal,
        threshold: Signal::Value(-20.0),
        ratio: Signal::Value(4.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
        makeup_gain: Signal::Value(0.0),
        state: CompressorState::new(),
    });

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&comp_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.1,
        "Compressor with constant parameters should work, RMS = {}", rms);
}
