/// Tests for ASR (Attack-Sustain-Release) envelope buffer-based evaluation
///
/// These tests verify that ASR envelope buffer evaluation produces correct
/// envelope shapes and responds properly to gate signals.
///
/// ASR is a gate-controlled envelope:
/// - Attack: rises to 1.0 when gate goes high
/// - Sustain: holds at 1.0 while gate stays high
/// - Release: falls to 0.0 when gate goes low
/// - Idle: stays at 0.0 when gate is low

use phonon::unified_graph::{Signal, UnifiedSignalGraph};

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
// TEST 1: Gate-On Response (Attack → Sustain)
// ============================================================================

#[test]
fn test_asr_gate_on_response() {
    let mut graph = create_test_graph();

    // Gate starts high (1.0)
    let gate_id = graph.add_constant_node(1.0);
    let asr_id = graph.add_asr_node(
        Signal::Value(0.01),  // 10ms attack
        Signal::Value(0.05),  // 50ms release
        Signal::Node(gate_id),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&asr_id, &mut output);

    // Should see rising envelope (attack) then sustain at 1.0
    // Early samples should be rising
    assert!(output[50] > output[0],
        "Envelope should be rising during attack: output[0]={}, output[50]={}",
        output[0], output[50]);

    // Later samples should be near 1.0 (sustain)
    let sustain_avg = output[400..512].iter().sum::<f32>() / 112.0;
    assert!(sustain_avg > 0.8,
        "Envelope should sustain near 1.0, got average: {}", sustain_avg);
}

// ============================================================================
// TEST 2: Gate-Off Response (Release)
// ============================================================================

#[test]
fn test_asr_gate_off_response() {
    let mut graph = create_test_graph();

    // First, trigger gate-on to reach sustain
    let gate_id = graph.add_constant_node(1.0);
    let asr_id = graph.add_asr_node(
        Signal::Value(0.001),  // 1ms attack (very fast)
        Signal::Value(0.02),   // 20ms release
        Signal::Node(gate_id),
    );

    let buffer_size = 512;
    let mut output1 = vec![0.0; buffer_size];
    graph.eval_node_buffer(&asr_id, &mut output1);

    // Now turn gate off
    graph.set_constant_value(gate_id, 0.0);

    let mut output2 = vec![0.0; buffer_size];
    graph.eval_node_buffer(&asr_id, &mut output2);

    // Should see falling envelope (release)
    // Early samples should be higher than later samples
    assert!(output2[0] > output2[400],
        "Envelope should be falling during release: output2[0]={}, output2[400]={}",
        output2[0], output2[400]);
}

// ============================================================================
// TEST 3: Attack Time Variation
// ============================================================================

#[test]
fn test_asr_different_attack_times() {
    let mut graph = create_test_graph();

    let gate_id = graph.add_constant_node(1.0);

    // Fast attack
    let fast_id = graph.add_asr_node(
        Signal::Value(0.001),  // 1ms
        Signal::Value(0.05),
        Signal::Node(gate_id),
    );

    // Slow attack
    let slow_id = graph.add_asr_node(
        Signal::Value(0.01),   // 10ms
        Signal::Value(0.05),
        Signal::Node(gate_id),
    );

    let buffer_size = 256;
    let mut fast_out = vec![0.0; buffer_size];
    let mut slow_out = vec![0.0; buffer_size];

    graph.eval_node_buffer(&fast_id, &mut fast_out);

    // Reset graph for second envelope
    let mut graph2 = create_test_graph();
    let gate_id2 = graph2.add_constant_node(1.0);
    let slow_id2 = graph2.add_asr_node(
        Signal::Value(0.01),
        Signal::Value(0.05),
        Signal::Node(gate_id2),
    );
    graph2.eval_node_buffer(&slow_id2, &mut slow_out);

    // Fast attack should reach higher level earlier
    let fast_early = fast_out[100];
    let slow_early = slow_out[100];

    assert!(fast_early > slow_early,
        "Fast attack should reach higher level earlier: fast={}, slow={}",
        fast_early, slow_early);
}

// ============================================================================
// TEST 4: Release Time Variation
// ============================================================================

#[test]
fn test_asr_different_release_times() {
    // Create two separate graphs to avoid state interference
    let mut graph1 = create_test_graph();
    let mut graph2 = create_test_graph();

    // Fast release envelope
    let gate1_id = graph1.add_constant_node(1.0);
    let fast_rel_id = graph1.add_asr_node(
        Signal::Value(0.001),  // 1ms attack
        Signal::Value(0.01),   // 10ms release (fast)
        Signal::Node(gate1_id),
    );

    // Slow release envelope
    let gate2_id = graph2.add_constant_node(1.0);
    let slow_rel_id = graph2.add_asr_node(
        Signal::Value(0.001),  // 1ms attack
        Signal::Value(0.05),   // 50ms release (slow)
        Signal::Node(gate2_id),
    );

    let buffer_size = 512;

    // Bring both to sustain
    let mut temp = vec![0.0; buffer_size];
    graph1.eval_node_buffer(&fast_rel_id, &mut temp);
    graph2.eval_node_buffer(&slow_rel_id, &mut temp);

    // Turn gates off
    graph1.set_constant_value(gate1_id, 0.0);
    graph2.set_constant_value(gate2_id, 0.0);

    // Measure release
    let mut fast_out = vec![0.0; buffer_size];
    let mut slow_out = vec![0.0; buffer_size];

    graph1.eval_node_buffer(&fast_rel_id, &mut fast_out);
    graph2.eval_node_buffer(&slow_rel_id, &mut slow_out);

    // Fast release should reach lower level sooner
    let fast_mid = fast_out[200];
    let slow_mid = slow_out[200];

    assert!(fast_mid < slow_mid,
        "Fast release should reach lower level sooner: fast={}, slow={}",
        fast_mid, slow_mid);
}

// ============================================================================
// TEST 5: Rapid Gate Changes
// ============================================================================

#[test]
fn test_asr_rapid_gate_changes() {
    let mut graph = create_test_graph();

    let gate_id = graph.add_constant_node(1.0);
    let asr_id = graph.add_asr_node(
        Signal::Value(0.005),  // 5ms attack
        Signal::Value(0.005),  // 5ms release
        Signal::Node(gate_id),
    );

    let buffer_size = 256;
    let mut output = vec![0.0; buffer_size];

    // Gate on
    graph.eval_node_buffer(&asr_id, &mut output);
    let level_after_on = output[buffer_size - 1];

    // Gate off
    graph.set_constant_value(gate_id, 0.0);

    graph.eval_node_buffer(&asr_id, &mut output);
    let level_after_off = output[buffer_size - 1];

    // Gate back on
    graph.set_constant_value(gate_id, 1.0);

    graph.eval_node_buffer(&asr_id, &mut output);
    let level_after_re_on = output[buffer_size - 1];

    // Should respond to each gate change
    assert!(level_after_off < level_after_on,
        "Level should decrease when gate goes off: on={}, off={}",
        level_after_on, level_after_off);

    assert!(level_after_re_on > level_after_off,
        "Level should increase when gate goes back on: off={}, re-on={}",
        level_after_off, level_after_re_on);
}

// ============================================================================
// TEST 6: State Continuity Across Buffers
// ============================================================================

#[test]
fn test_asr_continuity_across_buffers() {
    let mut graph = create_test_graph();

    let gate_id = graph.add_constant_node(1.0);
    let asr_id = graph.add_asr_node(
        Signal::Value(0.02),  // 20ms attack
        Signal::Value(0.02),  // 20ms release
        Signal::Node(gate_id),
    );

    // Generate two consecutive buffers
    let buffer_size = 256;
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&asr_id, &mut buffer1);
    graph.eval_node_buffer(&asr_id, &mut buffer2);

    // Check continuity at boundary
    let last_sample = buffer1[buffer_size - 1];
    let first_sample = buffer2[0];

    // Should be reasonably continuous (within a single increment)
    let diff = (first_sample - last_sample).abs();

    // Max increment per sample at 20ms attack @ 44.1kHz
    let max_increment = 1.0 / (0.02 * 44100.0) * 2.0; // *2 for safety margin

    assert!(diff < max_increment,
        "ASR should be continuous across buffers: last={}, first={}, diff={}, max_incr={}",
        last_sample, first_sample, diff, max_increment);
}

// ============================================================================
// TEST 7: Edge Cases - Zero/Negative Times
// ============================================================================

#[test]
fn test_asr_zero_attack() {
    let mut graph = create_test_graph();

    let gate_id = graph.add_constant_node(1.0);

    // Zero attack (should be clamped to minimum 0.1ms)
    let asr_id = graph.add_asr_node(
        Signal::Value(0.0),
        Signal::Value(0.05),
        Signal::Node(gate_id),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&asr_id, &mut output);

    // Should still produce output (attack clamped to min 0.1ms)
    let rms = calculate_rms(&output);
    assert!(rms > 0.01, "ASR with zero attack should still work, got RMS: {}", rms);

    // Should reach sustain level
    let max = find_max(&output);
    assert!(max > 0.5, "ASR should reach high level, got max: {}", max);
}

#[test]
fn test_asr_negative_times() {
    let mut graph = create_test_graph();

    let gate_id = graph.add_constant_node(1.0);

    // Negative times (should be clamped to minimum)
    let asr_id = graph.add_asr_node(
        Signal::Value(-0.5),
        Signal::Value(-0.5),
        Signal::Node(gate_id),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&asr_id, &mut output);

    // Should still produce valid output
    let rms = calculate_rms(&output);
    assert!(rms > 0.01, "ASR with negative times should still work, got RMS: {}", rms);
}

// ============================================================================
// TEST 8: Amplitude Range
// ============================================================================

#[test]
fn test_asr_amplitude_range() {
    let mut graph = create_test_graph();

    let gate_id = graph.add_constant_node(1.0);
    let asr_id = graph.add_asr_node(
        Signal::Value(0.01),
        Signal::Value(0.05),
        Signal::Node(gate_id),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&asr_id, &mut output);

    // All samples should be in valid range [0, 1]
    for (i, &sample) in output.iter().enumerate() {
        assert!(sample >= -0.01 && sample <= 1.01,
            "Sample {} out of range [0, 1]: {}", i, sample);
    }
}

// ============================================================================
// TEST 9: Sustain at 1.0
// ============================================================================

#[test]
fn test_asr_sustain_level() {
    let mut graph = create_test_graph();

    let gate_id = graph.add_constant_node(1.0);

    // Very fast attack to quickly reach sustain
    let asr_id = graph.add_asr_node(
        Signal::Value(0.001),  // 1ms attack
        Signal::Value(0.05),
        Signal::Node(gate_id),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&asr_id, &mut output);

    // After attack completes, should be sustaining at 1.0
    // Check last quarter of buffer
    let sustain_avg = output[384..512].iter().sum::<f32>() / 128.0;

    assert!((sustain_avg - 1.0).abs() < 0.1,
        "Envelope should sustain at 1.0, got average: {}", sustain_avg);
}

// ============================================================================
// TEST 10: Multiple Buffers (State Persistence)
// ============================================================================

#[test]
fn test_asr_multiple_buffers() {
    let mut graph = create_test_graph();

    let gate_id = graph.add_constant_node(1.0);
    let asr_id = graph.add_asr_node(
        Signal::Value(0.01),
        Signal::Value(0.05),
        Signal::Node(gate_id),
    );

    // Generate multiple consecutive buffers
    let buffer_size = 256;
    let num_buffers = 5;

    for i in 0..num_buffers {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&asr_id, &mut output);

        // Each buffer should have reasonable audio
        let rms = calculate_rms(&output);
        assert!(rms > 0.01,
            "Buffer {} should have audio content, got RMS: {}", i, rms);
    }
}

// ============================================================================
// TEST 11: Idle State (Gate Low)
// ============================================================================

#[test]
fn test_asr_idle_state() {
    let mut graph = create_test_graph();

    // Gate starts low (0.0)
    let gate_id = graph.add_constant_node(0.0);
    let asr_id = graph.add_asr_node(
        Signal::Value(0.01),
        Signal::Value(0.05),
        Signal::Node(gate_id),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&asr_id, &mut output);

    // Should remain at 0.0 (idle)
    let max = find_max(&output);
    assert!(max < 0.01, "ASR should remain at 0.0 when gate is low, got max: {}", max);
}

// ============================================================================
// TEST 12: Gate Threshold (0.5)
// ============================================================================

#[test]
fn test_asr_gate_threshold() {
    let mut graph1 = create_test_graph();
    let mut graph2 = create_test_graph();

    // Gate just below threshold (0.4)
    let low_gate = graph1.add_constant_node(0.4);
    let low_id = graph1.add_asr_node(
        Signal::Value(0.01),
        Signal::Value(0.05),
        Signal::Node(low_gate),
    );

    // Gate just above threshold (0.6)
    let high_gate = graph2.add_constant_node(0.6);
    let high_id = graph2.add_asr_node(
        Signal::Value(0.01),
        Signal::Value(0.05),
        Signal::Node(high_gate),
    );

    let buffer_size = 512;
    let mut low_out = vec![0.0; buffer_size];
    let mut high_out = vec![0.0; buffer_size];

    graph1.eval_node_buffer(&low_id, &mut low_out);
    graph2.eval_node_buffer(&high_id, &mut high_out);

    // Low gate (0.4) should stay at 0
    let low_max = find_max(&low_out);
    assert!(low_max < 0.1, "Gate=0.4 should not trigger, got max: {}", low_max);

    // High gate (0.6) should attack
    let high_max = find_max(&high_out);
    assert!(high_max > 0.5, "Gate=0.6 should trigger, got max: {}", high_max);
}

// ============================================================================
// TEST 13: Performance
// ============================================================================

#[test]
fn test_asr_buffer_performance() {
    let mut graph = create_test_graph();

    let gate_id = graph.add_constant_node(1.0);
    let asr_id = graph.add_asr_node(
        Signal::Value(0.01),
        Signal::Value(0.05),
        Signal::Node(gate_id),
    );

    let buffer_size = 512;
    let iterations = 1000;

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&asr_id, &mut output);
    }
    let duration = start.elapsed();

    println!("ASR buffer eval: {:?} for {} iterations", duration, iterations);
    println!("Per iteration: {:?}", duration / iterations);

    // Should complete in reasonable time (< 2 seconds for 1000 iterations)
    assert!(duration.as_secs() < 2,
        "ASR buffer evaluation too slow: {:?}", duration);
}

// ============================================================================
// TEST 14: Full Cycle (Attack → Sustain → Release → Idle)
// ============================================================================

#[test]
fn test_asr_full_cycle() {
    let mut graph = create_test_graph();

    let gate_id = graph.add_constant_node(1.0);
    let asr_id = graph.add_asr_node(
        Signal::Value(0.005),  // 5ms attack
        Signal::Value(0.005),  // 5ms release
        Signal::Node(gate_id),
    );

    let buffer_size = 512;

    // Phase 1: Attack → Sustain
    let mut output1 = vec![0.0; buffer_size];
    graph.eval_node_buffer(&asr_id, &mut output1);
    let peak1 = find_max(&output1);

    // Phase 2: Sustain continues
    let mut output2 = vec![0.0; buffer_size];
    graph.eval_node_buffer(&asr_id, &mut output2);
    let peak2 = find_max(&output2);

    // Should have reached and sustained at high level
    assert!(peak1 > 0.5, "Should reach high level during attack, got: {}", peak1);
    assert!(peak2 > 0.9, "Should sustain at ~1.0, got: {}", peak2);

    // Phase 3: Gate off → Release
    graph.set_constant_value(gate_id, 0.0);

    let mut output3 = vec![0.0; buffer_size];
    graph.eval_node_buffer(&asr_id, &mut output3);
    let end3 = output3[buffer_size - 1];

    // Should be falling
    assert!(end3 < peak2 * 0.8, "Should be releasing, got end3={}, peak2={}", end3, peak2);

    // Phase 4: Continue release → Idle
    let mut output4 = vec![0.0; buffer_size];
    graph.eval_node_buffer(&asr_id, &mut output4);
    let end4 = output4[buffer_size - 1];

    // Should reach idle (near 0)
    assert!(end4 < 0.1, "Should reach idle state, got: {}", end4);
}
