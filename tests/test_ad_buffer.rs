/// Tests for AD (Attack-Decay) envelope buffer-based evaluation
///
/// These tests verify that AD envelope buffer evaluation produces correct
/// envelope shapes and responds properly to parameter changes.
///
/// AD envelope is simpler than ADSR: Attack → Decay → Silent (no sustain or release)

use phonon::unified_graph::{ADState, ADSRState, Signal, SignalNode, UnifiedSignalGraph};

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
// TEST: Basic AD Shape
// ============================================================================

#[test]
fn test_ad_basic_envelope_shape() {
    let mut graph = create_test_graph();

    // Create AD with known parameters
    // Attack: 0.01s, Decay: 0.1s
    let ad_id = graph.add_node(SignalNode::AD {
        state: ADState::default(),
        attack: Signal::Value(0.01),  // 10ms attack
        decay: Signal::Value(0.1),    // 100ms decay
    });

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&ad_id, &mut output);

    // Envelope should have non-zero content
    let rms = calculate_rms(&output);
    assert!(rms > 0.1, "AD envelope should produce non-zero output, got RMS: {}", rms);

    // Peak should be close to 1.0 (during attack phase)
    let max = find_max(&output);
    assert!(max > 0.5, "AD should reach significant level, got max: {}", max);

    println!("AD basic shape - RMS: {}, Max: {}", rms, max);
}

#[test]
fn test_ad_attack_phase() {
    let mut graph = create_test_graph();

    // Fast attack (10ms), longer decay to isolate attack
    let ad_id = graph.add_node(SignalNode::AD {
        state: ADState::default(),
        attack: Signal::Value(0.01),   // 10ms attack
        decay: Signal::Value(0.5),     // 500ms decay
    });

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&ad_id, &mut output);

    // During attack, envelope should be rising
    // Check that early samples are lower than later samples
    let early_avg = output[0..100].iter().sum::<f32>() / 100.0;
    let mid_avg = output[200..300].iter().sum::<f32>() / 100.0;

    assert!(mid_avg > early_avg,
        "Envelope should rise during attack phase: early={}, mid={}", early_avg, mid_avg);

    println!("AD attack phase - early: {}, mid: {}", early_avg, mid_avg);
}

#[test]
fn test_ad_decay_phase() {
    let mut graph = create_test_graph();

    // Very fast attack, moderate decay
    let ad_id = graph.add_node(SignalNode::AD {
        state: ADState::default(),
        attack: Signal::Value(0.001),  // 1ms attack (very fast)
        decay: Signal::Value(0.05),    // 50ms decay
    });

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&ad_id, &mut output);

    // After attack, should decay toward zero
    // Check that later samples are lower than mid samples
    let mid_avg = output[100..200].iter().sum::<f32>() / 100.0;
    let late_avg = output[400..512].iter().sum::<f32>() / 112.0;

    assert!(mid_avg > late_avg,
        "Envelope should decay after attack: mid={}, late={}", mid_avg, late_avg);

    println!("AD decay phase - mid: {}, late: {}", mid_avg, late_avg);
}

#[test]
fn test_ad_reaches_zero() {
    let mut graph = create_test_graph();

    // Short attack and decay, buffer should reach zero
    let ad_id = graph.add_node(SignalNode::AD {
        attack: Signal::Value(0.001),  // 1ms attack
        decay: Signal::Value(0.005),   // 5ms decay
        state: ADState::default(),
    });

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&ad_id, &mut output);

    // Last quarter should be near zero (after decay complete)
    let end_avg = output[384..512].iter().sum::<f32>() / 128.0;

    assert!(end_avg < 0.1,
        "AD should reach near zero after decay, got {}", end_avg);

    println!("AD end average: {}", end_avg);
}

// ============================================================================
// TEST: Parameter Modulation
// ============================================================================

#[test]
fn test_ad_different_attack_times() {
    let mut graph = create_test_graph();

    // Fast attack
    let fast_id = graph.add_node(SignalNode::AD {
        attack: Signal::Value(0.001),  // 1ms
        decay: Signal::Value(0.05),
        state: ADState::default(),
    });

    // Slow attack
    let slow_id = graph.add_node(SignalNode::AD {
        attack: Signal::Value(0.05),   // 50ms
        decay: Signal::Value(0.05),
        state: ADState::default(),
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

    println!("AD different attacks - fast@100: {}, slow@100: {}", fast_early, slow_early);
}

#[test]
fn test_ad_different_decay_times() {
    let mut graph = create_test_graph();

    // Fast decay
    let fast_id = graph.add_node(SignalNode::AD {
        attack: Signal::Value(0.001),  // 1ms attack
        decay: Signal::Value(0.01),    // 10ms decay
        state: ADState::default(),
    });

    // Slow decay
    let slow_id = graph.add_node(SignalNode::AD {
        attack: Signal::Value(0.001),  // 1ms attack
        decay: Signal::Value(0.1),     // 100ms decay
        state: ADState::default(),
    });

    let buffer_size = 512;
    let mut fast_out = vec![0.0; buffer_size];
    let mut slow_out = vec![0.0; buffer_size];

    graph.eval_node_buffer(&fast_id, &mut fast_out);
    graph.eval_node_buffer(&slow_id, &mut slow_out);

    // Fast decay should reach zero sooner
    let fast_end = fast_out[400];
    let slow_end = slow_out[400];

    assert!(slow_end > fast_end,
        "Slow decay should be higher later: fast={}, slow={}", fast_end, slow_end);

    println!("AD different decays - fast@400: {}, slow@400: {}", fast_end, slow_end);
}

// ============================================================================
// TEST: Edge Cases
// ============================================================================

#[test]
fn test_ad_zero_attack() {
    let mut graph = create_test_graph();

    // Zero attack (should be clamped to minimum)
    let ad_id = graph.add_node(SignalNode::AD {
        state: ADState::default(),
        attack: Signal::Value(0.0),
        decay: Signal::Value(0.05),
    });

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&ad_id, &mut output);

    // Should still produce output (attack clamped to min 1ms)
    let rms = calculate_rms(&output);
    assert!(rms > 0.01, "AD with zero attack should still work, got RMS: {}", rms);

    println!("AD zero attack RMS: {}", rms);
}

#[test]
fn test_ad_zero_decay() {
    let mut graph = create_test_graph();

    // Zero decay (should be clamped to minimum)
    let ad_id = graph.add_node(SignalNode::AD {
        state: ADState::default(),
        attack: Signal::Value(0.01),
        decay: Signal::Value(0.0),
    });

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&ad_id, &mut output);

    // Should still produce output (decay clamped to min 1ms)
    let rms = calculate_rms(&output);
    assert!(rms > 0.01, "AD with zero decay should still work, got RMS: {}", rms);

    println!("AD zero decay RMS: {}", rms);
}

#[test]
fn test_ad_amplitude_range() {
    let mut graph = create_test_graph();

    let ad_id = graph.add_node(SignalNode::AD {
        state: ADState::default(),
        attack: Signal::Value(0.01),
        decay: Signal::Value(0.05),
    });

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&ad_id, &mut output);

    // All samples should be in valid range [0, 1]
    for (i, &sample) in output.iter().enumerate() {
        assert!(sample >= -0.01 && sample <= 1.01,
            "Sample {} out of range [0, 1]: {}", i, sample);
    }

    println!("AD amplitude range test passed");
}

#[test]
fn test_ad_reaches_peak() {
    let mut graph = create_test_graph();

    // Very fast attack to ensure we reach peak
    let ad_id = graph.add_node(SignalNode::AD {
        state: ADState::default(),
        attack: Signal::Value(0.001),  // 1ms attack
        decay: Signal::Value(0.1),
    });

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&ad_id, &mut output);

    // Should reach close to 1.0 during attack
    let max = find_max(&output);
    assert!(max > 0.8, "AD should reach near peak (1.0), got max: {}", max);

    println!("AD peak: {}", max);
}

// ============================================================================
// TEST: Multiple Buffers (State Persistence)
// ============================================================================

#[test]
fn test_ad_multiple_buffers() {
    let mut graph = create_test_graph();

    let ad_id = graph.add_node(SignalNode::AD {
        state: ADState::default(),
        attack: Signal::Value(0.01),
        decay: Signal::Value(0.05),
    });

    // Generate multiple consecutive buffers
    let buffer_size = 512;
    let num_buffers = 5;

    for i in 0..num_buffers {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&ad_id, &mut output);

        // Each buffer should have reasonable audio
        let rms = calculate_rms(&output);
        assert!(rms > 0.01,
            "Buffer {} should have audio content, got RMS: {}", i, rms);

        println!("Buffer {} RMS: {}", i, rms);
    }
}

#[test]
fn test_ad_continuity_across_buffers() {
    let mut graph = create_test_graph();

    let ad_id = graph.add_node(SignalNode::AD {
        state: ADState::default(),
        attack: Signal::Value(0.01),
        decay: Signal::Value(0.05),
    });

    // Generate two consecutive buffers
    let buffer_size = 512;
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&ad_id, &mut buffer1);
    graph.eval_node_buffer(&ad_id, &mut buffer2);

    // Check continuity at boundary
    let last_sample = buffer1[buffer_size - 1];
    let first_sample = buffer2[0];

    // Should be reasonably continuous (within 20%)
    let diff = (first_sample - last_sample).abs();
    let avg = (first_sample + last_sample) / 2.0;

    if avg > 0.01 {  // Only check continuity if not near zero
        assert!(diff < avg * 0.2,
            "AD should be continuous across buffers: last={}, first={}, diff={}",
            last_sample, first_sample, diff);
    }

    println!("Buffer continuity - last: {}, first: {}", last_sample, first_sample);
}

// ============================================================================
// TEST: Performance
// ============================================================================

#[test]
fn test_ad_buffer_performance() {
    let mut graph = create_test_graph();

    let ad_id = graph.add_node(SignalNode::AD {
        state: ADState::default(),
        attack: Signal::Value(0.01),
        decay: Signal::Value(0.05),
    });

    let buffer_size = 512;
    let iterations = 1000;

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&ad_id, &mut output);
    }
    let duration = start.elapsed();

    println!("AD buffer eval: {:?} for {} iterations", duration, iterations);
    println!("Per iteration: {:?}", duration / iterations);

    // Should complete in reasonable time (< 2 seconds for 1000 iterations)
    assert!(duration.as_secs() < 2,
        "AD buffer evaluation too slow: {:?}", duration);
}

// ============================================================================
// TEST: Comparison with ADSR
// ============================================================================

#[test]
fn test_ad_simpler_than_adsr() {
    let mut graph = create_test_graph();

    // AD envelope
    let ad_id = graph.add_node(SignalNode::AD {
        state: ADState::default(),
        attack: Signal::Value(0.01),
        decay: Signal::Value(0.05),
    });

    // Equivalent ADSR (sustain=0, release=0.001 minimal)
    let adsr_id = graph.add_node(SignalNode::ADSR {
        attack: Signal::Value(0.01),   // attack
        decay: Signal::Value(0.05),    // decay
        sustain: Signal::Value(0.0),   // sustain = 0
        release: Signal::Value(0.001), // release (minimal)
        state: ADSRState::default(),
    });

    let buffer_size = 512;
    let mut ad_out = vec![0.0; buffer_size];
    let mut adsr_out = vec![0.0; buffer_size];

    graph.eval_node_buffer(&ad_id, &mut ad_out);
    graph.eval_node_buffer(&adsr_id, &mut adsr_out);

    // Both should produce similar output (AD simpler, no release phase)
    let ad_rms = calculate_rms(&ad_out);
    let adsr_rms = calculate_rms(&adsr_out);

    println!("AD RMS: {}, ADSR RMS: {}", ad_rms, adsr_rms);

    // Both should have significant energy
    assert!(ad_rms > 0.1, "AD should have energy");
    assert!(adsr_rms > 0.1, "ADSR should have energy");
}

#[test]
fn test_ad_typical_percussive() {
    let mut graph = create_test_graph();

    // Typical percussive envelope: fast attack, medium decay
    let ad_id = graph.add_node(SignalNode::AD {
        attack: Signal::Value(0.001),  // 1ms attack (instant)
        decay: Signal::Value(0.15),    // 150ms decay (natural decay)
        state: ADState::default(),
    });

    let buffer_size = 2048;  // Longer buffer to see full envelope
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&ad_id, &mut output);

    // Should have sharp attack
    let very_early = output[0..10].iter().sum::<f32>() / 10.0;
    let early = output[50..60].iter().sum::<f32>() / 10.0;

    assert!(early > very_early * 2.0,
        "Should have sharp attack: very_early={}, early={}", very_early, early);

    // Should have smooth decay
    let mid = output[500..510].iter().sum::<f32>() / 10.0;
    let late = output[1500..1510].iter().sum::<f32>() / 10.0;

    assert!(mid > late,
        "Should decay smoothly: mid={}, late={}", mid, late);

    println!("Percussive AD - very_early: {}, early: {}, mid: {}, late: {}",
        very_early, early, mid, late);
}
