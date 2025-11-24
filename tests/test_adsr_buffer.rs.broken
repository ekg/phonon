/// Tests for ADSR envelope buffer-based evaluation
///
/// These tests verify that ADSR envelope buffer evaluation produces correct
/// envelope shapes and responds properly to triggers and parameter changes.

use phonon::unified_graph::{ADSRState, Signal, SignalNode, UnifiedSignalGraph};

/// Helper: Create a test graph
fn create_test_graph() -> UnifiedSignalGraph {
    UnifiedSignalGraph::new(44100.0) // 44.1kHz
}

/// Helper: Calculate RMS of a buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Helper: Find maximum value in buffer
fn find_max(buffer: &[f32]) -> f32 {
    buffer.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b))
}

/// Helper: Find minimum value in buffer
fn find_min(buffer: &[f32]) -> f32 {
    buffer.iter().fold(f32::INFINITY, |a, &b| a.min(b))
}

// ============================================================================
// TEST: Basic ADSR Shape
// ============================================================================

#[test]
fn test_adsr_basic_envelope_shape() {
    let mut graph = create_test_graph();

    // Create ADSR with known parameters
    // Attack: 0.01s, Decay: 0.05s, Sustain: 0.7, Release: 0.1s
    let adsr_id = graph.add_node(SignalNode::ADSR {

        attack: Signal::Value(0.01),  // 10ms attack
        Signal::Value(0.05),  // 50ms decay
        Signal::Value(0.7),   // 70% sustain level
        Signal::Value(0.1),   // 100ms release
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&adsr_id, &mut output);

    // Envelope should have non-zero content
    let rms = calculate_rms(&output);
    assert!(rms > 0.1, "ADSR envelope should produce non-zero output, got RMS: {}", rms);

    // Peak should be close to 1.0 (during attack phase)
    let max = find_max(&output);
    assert!(max > 0.5, "ADSR should reach significant level, got max: {}", max);
}

#[test]
fn test_adsr_attack_phase() {
    let mut graph = create_test_graph();

    // Fast attack (10ms), long decay/release to isolate attack
    let adsr_id = graph.add_node(SignalNode::ADSR {
        attack: Signal::Value(0.01),   // 10ms attack
        Signal::Value(0.5),    // 500ms decay
        Signal::Value(0.7),    // 70% sustain
        Signal::Value(0.5),    // 500ms release
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&adsr_id, &mut output);

    // During attack, envelope should be rising
    // Check that early samples are lower than later samples
    let early_avg = output[0..100].iter().sum::<f32>() / 100.0;
    let mid_avg = output[200..300].iter().sum::<f32>() / 100.0;

    assert!(mid_avg > early_avg,
        "Envelope should rise during attack phase: early={}, mid={}", early_avg, mid_avg);
}

#[test]
fn test_adsr_sustain_level() {
    let mut graph = create_test_graph();

    // Quick attack/decay, then sustain
    let adsr_id = graph.add_node(SignalNode::ADSR { attack: 
        Signal::Value(0.001),  // 1ms attack (very fast)
        Signal::Value(0.002),  // 2ms decay (very fast)
        Signal::Value(0.5),    // 50% sustain level
        Signal::Value(0.5),    // 500ms release (won't reach in this buffer)
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&adsr_id, &mut output);

    // After attack and decay, should be near sustain level
    // Check last quarter of buffer (should be in sustain phase)
    let sustain_avg = output[384..512].iter().sum::<f32>() / 128.0;

    // Should be reasonably close to sustain level (0.5)
    assert!((sustain_avg - 0.5).abs() < 0.3,
        "Envelope should settle near sustain level 0.5, got {}", sustain_avg);
}

// ============================================================================
// TEST: Parameter Modulation
// ============================================================================

#[test]
fn test_adsr_different_attack_times() {
    let mut graph = create_test_graph();

    // Fast attack
    let fast_id = graph.add_node(SignalNode::ADSR { attack: 
        Signal::Value(0.001),  // 1ms
        Signal::Value(0.05),

        decay: Signal::Value(0.7),

        sustain: Signal::Value(0.1),
    );

    // Slow attack
    let slow_id = graph.add_node(SignalNode::ADSR { attack: 
        Signal::Value(0.05),   // 50ms
        Signal::Value(0.05),

        release: Signal::Value(0.7),
        decay: Signal::Value(0.1),

        state: ADSRState::default(),

    });

    let buffer_size = 512;
    let mut fast_out = vec![0.0; buffer_size];
    let mut slow_out = vec![0.0; buffer_size];

    graph.eval_node_buffer(&fast_id, &mut fast_out);
    graph.eval_node_buffer(&slow_id, &mut slow_out);

    // Fast attack should reach higher level earlier
    let fast_early = fast_out[100];
    let slow_early = slow_out[100];

    assert!(fast_early > slow_early,
        "Fast attack should reach higher level earlier: fast={}, slow={}", fast_early, slow_early);
}

#[test]
fn test_adsr_different_sustain_levels() {
    let mut graph = create_test_graph();

    // High sustain
    let high_id = graph.add_node(SignalNode::ADSR {

        attack: Signal::Value(0.001),

        decay: Signal::Value(0.002),

        sustain: Signal::Value(0.9),   // 90%
        Signal::Value(0.1),
    );

    // Low sustain
    let low_id = graph.add_node(SignalNode::ADSR { attack: 
        Signal::Value(0.001),

        release: Signal::Value(0.002),
        sustain: Signal::Value(0.3),   // 30%
        Signal::Value(0.1),

        state: ADSRState::default(),

    });

    let buffer_size = 512;
    let mut high_out = vec![0.0; buffer_size];
    let mut low_out = vec![0.0; buffer_size];

    graph.eval_node_buffer(&high_id, &mut high_out);
    graph.eval_node_buffer(&low_id, &mut low_out);

    // Check sustain portion (last quarter of buffer)
    let high_sustain = high_out[384..512].iter().sum::<f32>() / 128.0;
    let low_sustain = low_out[384..512].iter().sum::<f32>() / 128.0;

    assert!(high_sustain > low_sustain,
        "High sustain should be greater than low sustain: high={}, low={}",
        high_sustain, low_sustain);
}

// ============================================================================
// TEST: Edge Cases
// ============================================================================

#[test]
fn test_adsr_zero_attack() {
    let mut graph = create_test_graph();

    // Zero attack (should be clamped to minimum)
    let adsr_id = graph.add_node(SignalNode::ADSR {

        attack: Signal::Value(0.0),

        decay: Signal::Value(0.05),

        sustain: Signal::Value(0.7),

        release: Signal::Value(0.1),

        state: ADSRState::default(),

    });

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&adsr_id, &mut output);

    // Should still produce output (attack clamped to min 1ms)
    let rms = calculate_rms(&output);
    assert!(rms > 0.01, "ADSR with zero attack should still work, got RMS: {}", rms);
}

#[test]
fn test_adsr_sustain_clamping() {
    let mut graph = create_test_graph();

    // Sustain > 1.0 (should be clamped)
    let high_id = graph.add_node(SignalNode::ADSR {

        attack: Signal::Value(0.001),

        decay: Signal::Value(0.002),

        sustain: Signal::Value(2.0),   // Should clamp to 1.0
        Signal::Value(0.1),
    );

    // Sustain < 0.0 (should be clamped)
    let low_id = graph.add_node(SignalNode::ADSR { attack: 
        Signal::Value(0.001),

        release: Signal::Value(0.002),
        release: Signal::Value(-0.5),  // Should clamp to 0.0
        Signal::Value(0.1),

        state: ADSRState::default(),

    });

    let buffer_size = 512;
    let mut high_out = vec![0.0; buffer_size];
    let mut low_out = vec![0.0; buffer_size];

    graph.eval_node_buffer(&high_id, &mut high_out);
    graph.eval_node_buffer(&low_id, &mut low_out);

    // High should be clamped to 1.0
    let high_max = find_max(&high_out);
    assert!(high_max <= 1.01, "Sustain should be clamped to 1.0, got max: {}", high_max);

    // Low should be clamped to 0.0
    let low_min = find_min(&low_out);
    assert!(low_min >= -0.01, "Sustain should be clamped to 0.0, got min: {}", low_min);
}

#[test]
fn test_adsr_all_phases() {
    let mut graph = create_test_graph();

    // Balanced ADSR that shows all phases
    let adsr_id = graph.add_node(SignalNode::ADSR {

        attack: Signal::Value(0.002),  // 2ms attack
        Signal::Value(0.003),  // 3ms decay
        Signal::Value(0.6),    // 60% sustain
        Signal::Value(0.003),  // 3ms release
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&adsr_id, &mut output);

    // Should see variation (not flat)
    let max = find_max(&output);
    let min = find_min(&output);
    let range = max - min;

    assert!(range > 0.1, "ADSR should show dynamic range across phases, got range: {}", range);
}

// ============================================================================
// TEST: Multiple Buffers (State Persistence)
// ============================================================================

#[test]
fn test_adsr_multiple_buffers() {
    let mut graph = create_test_graph();

    let adsr_id = graph.add_node(SignalNode::ADSR { attack: 
        Signal::Value(0.01),

        decay: Signal::Value(0.05),

        sustain: Signal::Value(0.7),

        release: Signal::Value(0.1),

        state: ADSRState::default(),

    });

    // Generate multiple consecutive buffers
    let buffer_size = 512;
    let num_buffers = 5;

    for i in 0..num_buffers {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&adsr_id, &mut output);

        // Each buffer should have reasonable audio
        let rms = calculate_rms(&output);
        assert!(rms > 0.01,
            "Buffer {} should have audio content, got RMS: {}", i, rms);
    }
}

#[test]
fn test_adsr_continuity_across_buffers() {
    let mut graph = create_test_graph();

    let adsr_id = graph.add_node(SignalNode::ADSR {


        attack: Signal::Value(0.01),


        decay: Signal::Value(0.05),


        sustain: Signal::Value(0.7),


        release: Signal::Value(0.1),


        state: ADSRState::default(),


    });

    // Generate two consecutive buffers
    let buffer_size = 512;
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&adsr_id, &mut buffer1);
    graph.eval_node_buffer(&adsr_id, &mut buffer2);

    // Check continuity at boundary
    let last_sample = buffer1[buffer_size - 1];
    let first_sample = buffer2[0];

    // Should be reasonably continuous (within 20%)
    let diff = (first_sample - last_sample).abs();
    let avg = (first_sample + last_sample) / 2.0;

    if avg > 0.01 {  // Only check continuity if not near zero
        assert!(diff < avg * 0.2,
            "ADSR should be continuous across buffers: last={}, first={}, diff={}",
            last_sample, first_sample, diff);
    }
}

// ============================================================================
// TEST: Performance
// ============================================================================

#[test]
fn test_adsr_buffer_performance() {
    let mut graph = create_test_graph();

    let adsr_id = graph.add_node(SignalNode::ADSR {


        attack: Signal::Value(0.01),


        decay: Signal::Value(0.05),


        sustain: Signal::Value(0.7),


        release: Signal::Value(0.1),


        state: ADSRState::default(),


    });

    let buffer_size = 512;
    let iterations = 1000;

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&adsr_id, &mut output);
    }
    let duration = start.elapsed();

    println!("ADSR buffer eval: {:?} for {} iterations", duration, iterations);
    println!("Per iteration: {:?}", duration / iterations);

    // Should complete in reasonable time (< 2 seconds for 1000 iterations)
    assert!(duration.as_secs() < 2,
        "ADSR buffer evaluation too slow: {:?}", duration);
}

// ============================================================================
// TEST: Amplitude Verification
// ============================================================================

#[test]
fn test_adsr_amplitude_range() {
    let mut graph = create_test_graph();

    let adsr_id = graph.add_node(SignalNode::ADSR {


        attack: Signal::Value(0.01),


        decay: Signal::Value(0.05),


        sustain: Signal::Value(0.7),


        release: Signal::Value(0.1),


        state: ADSRState::default(),


    });

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&adsr_id, &mut output);

    // All samples should be in valid range [0, 1]
    for (i, &sample) in output.iter().enumerate() {
        assert!(sample >= -0.01 && sample <= 1.01,
            "Sample {} out of range [0, 1]: {}", i, sample);
    }
}

#[test]
fn test_adsr_reaches_peak() {
    let mut graph = create_test_graph();

    // Very fast attack to ensure we reach peak
    let adsr_id = graph.add_node(SignalNode::ADSR { attack: 
        Signal::Value(0.001),  // 1ms attack
        Signal::Value(0.1),
        Signal::Value(0.7),
        Signal::Value(0.1),
        state: ADSRState::default(),
    })

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&adsr_id, &mut output);

    // Should reach close to 1.0 during attack
    let max = find_max(&output);
    assert!(max > 0.8, "ADSR should reach near peak (1.0), got max: {}", max);
}
