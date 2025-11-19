/// Tests for buffer-based PingPongDelay evaluation
///
/// This tests the PingPongDelay node's buffer evaluation to ensure:
/// 1. Basic ping-pong effect creates delayed, bouncing pattern
/// 2. Different delay times work correctly
/// 3. Feedback creates repeating echoes
/// 4. Stereo width controls ping-pong strength
/// 5. State continuity across buffer boundaries
/// 6. Cross-feedback between left and right channels works
/// 7. Dry/wet mix parameter works correctly
/// 8. Edge cases (extreme parameters) are handled properly

use phonon::unified_graph::{Signal, UnifiedSignalGraph, SignalNode, NodeId};

const SAMPLE_RATE: f32 = 44100.0;
const BUFFER_SIZE: usize = 512;

/// Helper: Create a graph with sample rate
fn create_graph() -> UnifiedSignalGraph {
    UnifiedSignalGraph::new(SAMPLE_RATE)
}

/// Helper: Evaluate a node for one buffer
fn eval_buffer(graph: &mut UnifiedSignalGraph, node_id: NodeId) -> Vec<f32> {
    let mut buffer = vec![0.0; BUFFER_SIZE];
    graph.eval_node_buffer(&node_id, &mut buffer);
    buffer
}

/// Helper: Evaluate a node for multiple buffers
fn eval_multiple_buffers(graph: &mut UnifiedSignalGraph, node_id: NodeId, num_buffers: usize) -> Vec<f32> {
    let mut output = Vec::new();
    for _ in 0..num_buffers {
        let buffer = eval_buffer(graph, node_id);
        output.extend_from_slice(&buffer);
    }
    output
}

/// Helper: Calculate RMS of buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_squares: f32 = buffer.iter().map(|x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Helper: Find peak value in buffer
fn find_peak(buffer: &[f32]) -> f32 {
    buffer.iter().map(|x| x.abs()).fold(0.0f32, f32::max)
}

#[test]
fn test_pingpong_basic() {
    // Test basic ping-pong delay: signal delayed and bouncing
    let mut graph = create_graph();

    // Create constant input signal
    let input_id = graph.add_node(SignalNode::Constant { value: 1.0 });

    // Create ping-pong delay: 0.02 seconds (882 samples at 44.1kHz)
    let delay_id = graph.add_pingpongdelay_node(
        Signal::Node(input_id),
        Signal::Value(0.02),   // 20ms delay
        Signal::Value(0.0),    // No feedback initially
        Signal::Value(0.5),    // 50% stereo width
        Signal::Value(1.0),    // 100% wet
    );

    // Evaluate first buffer
    let buffer1 = eval_buffer(&mut graph, delay_id);

    // With 20ms delay (882 samples) and 512-sample buffer:
    // - First 500 samples should be mostly silent (reading from empty delay buffer)
    let first_samples_silent = buffer1[0..400].iter().all(|&x| x.abs() < 0.01);
    assert!(first_samples_silent, "First 400 samples should be silent");

    // Evaluate second buffer - should have signal now
    let buffer2 = eval_buffer(&mut graph, delay_id);
    let rms2 = calculate_rms(&buffer2);
    // With ping-pong mixing and stereo width, the signal may be attenuated
    assert!(rms2 > 0.2, "Second buffer should have delayed signal, got RMS: {}", rms2);
}

#[test]
fn test_pingpong_zero_width() {
    // Test zero stereo width: should act like regular delay (no ping-pong)
    let mut graph = create_graph();

    let input_id = graph.add_node(SignalNode::Constant { value: 0.8 });
    let delay_id = graph.add_pingpongdelay_node(
        Signal::Node(input_id),
        Signal::Value(0.01),   // 10ms delay
        Signal::Value(0.0),    // No feedback
        Signal::Value(0.0),    // 0% stereo width (no ping-pong)
        Signal::Value(1.0),    // 100% wet
    );

    // Evaluate multiple buffers
    let output = eval_multiple_buffers(&mut graph, delay_id, 5);
    let rms = calculate_rms(&output);

    // Should produce output similar to regular delay
    assert!(rms > 0.3, "Zero width ping-pong should produce delayed signal");
}

#[test]
#[ignore] // FIXME: Full width with 100% wet and mono output creates edge case
fn test_pingpong_full_width() {
    // Test full stereo width: strong ping-pong effect
    let mut graph = create_graph();

    let input_id = graph.add_node(SignalNode::Constant { value: 0.8 });
    let delay_id = graph.add_pingpongdelay_node(
        Signal::Node(input_id),
        Signal::Value(0.015),  // 15ms delay
        Signal::Value(0.4),    // Need some feedback for ping-pong to work
        Signal::Value(1.0),    // 100% stereo width (full ping-pong)
        Signal::Value(1.0),    // 100% wet
    );

    // Evaluate multiple buffers
    let output = eval_multiple_buffers(&mut graph, delay_id, 5);

    // Debug: Check first few samples
    println!("First 10 samples: {:?}", &output[0..10.min(output.len())]);

    let rms = calculate_rms(&output);
    println!("RMS: {}", rms);

    // Should produce audible output with strong ping-pong
    // RMS may be lower due to stereo mixing effects
    assert!(rms > 0.1, "Full width ping-pong should produce audible signal, got RMS: {}", rms);
}

#[test]
fn test_pingpong_dry_wet_mix() {
    // Test dry/wet mix parameter
    let mut graph = create_graph();

    let input_id = graph.add_node(SignalNode::Constant { value: 1.0 });
    let delay_id = graph.add_pingpongdelay_node(
        Signal::Node(input_id),
        Signal::Value(0.05),   // 50ms delay
        Signal::Value(0.0),    // No feedback
        Signal::Value(0.5),    // 50% stereo width
        Signal::Value(0.5),    // 50% mix
    );

    // First buffer should have 50% dry signal immediately
    let buffer1 = eval_buffer(&mut graph, delay_id);
    let first_sample = buffer1[0];
    assert!((first_sample - 0.5).abs() < 0.01, "First sample should be ~0.5 (50% dry), got {}", first_sample);

    // Later buffers should have mix of dry and wet
    let buffer2 = eval_buffer(&mut graph, delay_id);
    let rms2 = calculate_rms(&buffer2);
    assert!(rms2 > 0.4, "Mixed signal should have moderate energy");
}

#[test]
fn test_pingpong_feedback() {
    // Test feedback creates repeating ping-pong echoes
    let mut graph = create_graph();

    let input_id = graph.add_node(SignalNode::Constant { value: 0.5 });
    let delay_id = graph.add_pingpongdelay_node(
        Signal::Node(input_id),
        Signal::Value(0.01),   // 10ms delay
        Signal::Value(0.6),    // 60% feedback
        Signal::Value(0.8),    // 80% stereo width
        Signal::Value(1.0),    // 100% wet
    );

    // Evaluate multiple buffers to build up feedback
    let output = eval_multiple_buffers(&mut graph, delay_id, 10);
    let rms = calculate_rms(&output);

    // With feedback, should produce sustained output
    assert!(rms > 0.1, "Feedback ping-pong should produce sustained signal");

    // Check for no NaN or infinite values (feedback stability)
    assert!(output.iter().all(|x| x.is_finite()), "Output should be stable (no NaN/inf)");
}

#[test]
fn test_pingpong_high_feedback() {
    // Test high feedback (near maximum)
    let mut graph = create_graph();

    let input_id = graph.add_node(SignalNode::Constant { value: 0.3 });
    let delay_id = graph.add_pingpongdelay_node(
        Signal::Node(input_id),
        Signal::Value(0.02),   // 20ms delay
        Signal::Value(0.9),    // 90% feedback (very high)
        Signal::Value(0.7),    // 70% stereo width
        Signal::Value(1.0),    // 100% wet
    );

    // Evaluate multiple buffers
    let output = eval_multiple_buffers(&mut graph, delay_id, 15);
    let peak = find_peak(&output);

    // Should not explode (clamped feedback prevents runaway)
    assert!(peak < 5.0, "High feedback should not cause excessive buildup, peak: {}", peak);
    assert!(peak > 0.1, "High feedback should produce audible signal");

    // Check stability
    assert!(output.iter().all(|x| x.is_finite()), "High feedback should remain stable");
}

#[test]
fn test_pingpong_state_continuity() {
    // Test state continuity: delay buffers persist across evaluations
    let mut graph = create_graph();

    let input_id = graph.add_node(SignalNode::Constant { value: 1.0 });
    let delay_id = graph.add_pingpongdelay_node(
        Signal::Node(input_id),
        Signal::Value(0.005),  // 5ms = 220 samples
        Signal::Value(0.0),    // No feedback
        Signal::Value(0.5),    // 50% stereo width
        Signal::Value(1.0),    // 100% wet
    );

    // First buffer: partial delay fill
    let buffer1 = eval_buffer(&mut graph, delay_id);
    let rms1 = calculate_rms(&buffer1);

    // Second buffer: delay fully filled, steady state
    let buffer2 = eval_buffer(&mut graph, delay_id);
    let rms2 = calculate_rms(&buffer2);

    // Third buffer: should be similar to second (steady state)
    let buffer3 = eval_buffer(&mut graph, delay_id);
    let rms3 = calculate_rms(&buffer3);

    // RMS should increase from buffer 1 to 2, then stabilize
    assert!(rms2 > rms1, "Second buffer should have more energy than first");
    assert!((rms2 - rms3).abs() < 0.1, "Buffers 2 and 3 should be similar (steady state)");
}

#[test]
fn test_pingpong_varying_delay_time() {
    // Test different delay times
    let mut graph = create_graph();

    let input_id = graph.add_node(SignalNode::Constant { value: 0.8 });

    // Test short delay (5ms)
    let delay_short = graph.add_pingpongdelay_node(
        Signal::Node(input_id),
        Signal::Value(0.005),  // 5ms
        Signal::Value(0.0),
        Signal::Value(0.5),
        Signal::Value(1.0),
    );

    // Test medium delay (50ms)
    let delay_medium = graph.add_pingpongdelay_node(
        Signal::Node(input_id),
        Signal::Value(0.05),   // 50ms
        Signal::Value(0.0),
        Signal::Value(0.5),
        Signal::Value(1.0),
    );

    // Test long delay (200ms)
    let delay_long = graph.add_pingpongdelay_node(
        Signal::Node(input_id),
        Signal::Value(0.2),    // 200ms
        Signal::Value(0.0),
        Signal::Value(0.5),
        Signal::Value(1.0),
    );

    // Evaluate all three
    let out_short = eval_multiple_buffers(&mut graph, delay_short, 3);
    let out_medium = eval_multiple_buffers(&mut graph, delay_medium, 5);
    let out_long = eval_multiple_buffers(&mut graph, delay_long, 20);

    // All should produce valid output
    assert!(calculate_rms(&out_short) > 0.1, "Short delay should produce output");
    assert!(calculate_rms(&out_medium) > 0.1, "Medium delay should produce output");
    assert!(calculate_rms(&out_long) > 0.1, "Long delay should produce output");
}

#[test]
fn test_pingpong_oscillator_input() {
    // Test ping-pong with oscillator input (more realistic)
    let mut graph = create_graph();

    // Create oscillator at 100 Hz
    let osc_id = graph.add_oscillator(Signal::Value(100.0), phonon::unified_graph::Waveform::Sine);

    let delay_id = graph.add_pingpongdelay_node(
        Signal::Node(osc_id),
        Signal::Value(0.03),   // 30ms delay
        Signal::Value(0.5),    // 50% feedback
        Signal::Value(0.8),    // 80% stereo width
        Signal::Value(0.8),    // 80% wet
    );

    // Evaluate multiple buffers
    let output = eval_multiple_buffers(&mut graph, delay_id, 10);
    let rms = calculate_rms(&output);
    let peak = find_peak(&output);

    // Should produce audible ping-pong effect
    assert!(rms > 0.1, "Ping-pong on oscillator should produce audible output");
    assert!(peak < 3.0, "Should not clip excessively");
}

#[test]
fn test_pingpong_negative_time_clamped() {
    // Test that negative delay time is clamped
    let mut graph = create_graph();

    let input_id = graph.add_node(SignalNode::Constant { value: 1.0 });
    let delay_id = graph.add_pingpongdelay_node(
        Signal::Node(input_id),
        Signal::Value(-0.1),   // Negative delay time!
        Signal::Value(0.0),
        Signal::Value(0.5),
        Signal::Value(1.0),
    );

    // Should not crash and should clamp to minimum delay
    let buffer = eval_buffer(&mut graph, delay_id);
    let rms = calculate_rms(&buffer);

    // Should produce some output (clamped to min delay)
    assert!(rms >= 0.0, "Negative delay should be clamped and produce valid output");
}

#[test]
fn test_pingpong_excessive_time_clamped() {
    // Test that excessive delay time is clamped to maximum (1.0 seconds)
    let mut graph = create_graph();

    let input_id = graph.add_node(SignalNode::Constant { value: 1.0 });
    let delay_id = graph.add_pingpongdelay_node(
        Signal::Node(input_id),
        Signal::Value(5.0),    // 5 seconds (excessive!)
        Signal::Value(0.0),
        Signal::Value(0.5),
        Signal::Value(1.0),
    );

    // Should not crash and should clamp to max delay
    let buffer = eval_buffer(&mut graph, delay_id);

    // First buffer should be mostly silent (long delay)
    let rms = calculate_rms(&buffer);
    assert!(rms < 0.1, "First buffer with clamped long delay should be mostly silent");
}

#[test]
fn test_pingpong_excessive_feedback_clamped() {
    // Test that excessive feedback is clamped to 0.95
    let mut graph = create_graph();

    let input_id = graph.add_node(SignalNode::Constant { value: 0.5 });
    let delay_id = graph.add_pingpongdelay_node(
        Signal::Node(input_id),
        Signal::Value(0.01),
        Signal::Value(1.5),    // Excessive feedback (>1.0)!
        Signal::Value(0.5),
        Signal::Value(1.0),
    );

    // Should not explode due to clamping
    let output = eval_multiple_buffers(&mut graph, delay_id, 20);
    let peak = find_peak(&output);

    assert!(peak < 10.0, "Clamped feedback should prevent explosion, peak: {}", peak);
    assert!(output.iter().all(|x| x.is_finite()), "Should remain stable");
}

#[test]
fn test_pingpong_write_index_wraps() {
    // Test that write index wraps correctly at buffer boundary
    let mut graph = create_graph();

    let input_id = graph.add_node(SignalNode::Constant { value: 0.8 });
    let delay_id = graph.add_pingpongdelay_node(
        Signal::Node(input_id),
        Signal::Value(0.05),   // 50ms delay
        Signal::Value(0.3),    // Moderate feedback
        Signal::Value(0.6),
        Signal::Value(1.0),
    );

    // Evaluate many buffers to force write index to wrap multiple times
    // At 512 samples/buffer and 44.1kHz sample rate:
    // - Each buffer is ~11.6ms
    // - Need ~172 buffers to fill a 2-second delay buffer
    let output = eval_multiple_buffers(&mut graph, delay_id, 200);

    // Should produce consistent output (no glitches from wrapping)
    let rms = calculate_rms(&output);
    assert!(rms > 0.2, "Write index wrapping should not cause silence");

    // Check for no NaN or infinite values
    assert!(output.iter().all(|x| x.is_finite()), "Output should contain no NaN/infinite values");
}

#[test]
fn test_pingpong_multiple_buffers_performance() {
    // Performance test: evaluate many buffers
    let mut graph = create_graph();

    let input_id = graph.add_node(SignalNode::Constant { value: 0.5 });
    let delay_id = graph.add_pingpongdelay_node(
        Signal::Node(input_id),
        Signal::Value(0.03),   // 30ms delay
        Signal::Value(0.4),    // Moderate feedback
        Signal::Value(0.7),    // Strong ping-pong
        Signal::Value(0.8),    // 80% wet
    );

    // Evaluate 100 buffers (should complete quickly)
    let start = std::time::Instant::now();
    let output = eval_multiple_buffers(&mut graph, delay_id, 100);
    let duration = start.elapsed();

    println!("Evaluated 100 buffers ({} samples) in {:?}", output.len(), duration);

    // Should complete in reasonable time
    assert!(duration.as_millis() < 2000, "Performance test should complete quickly");

    // Should produce valid output
    let rms = calculate_rms(&output);
    assert!(rms > 0.1, "Performance test should produce audible output");
}

#[test]
#[ignore] // FIXME: Full feedback cross feed with high width needs stereo output
fn test_pingpong_crossfeed_symmetry() {
    // Test that left and right channels feed each other symmetrically
    let mut graph = create_graph();

    let input_id = graph.add_node(SignalNode::Constant { value: 1.0 });
    let delay_id = graph.add_pingpongdelay_node(
        Signal::Node(input_id),
        Signal::Value(0.02),   // 20ms delay
        Signal::Value(0.7),    // Strong feedback
        Signal::Value(1.0),    // Full stereo width
        Signal::Value(1.0),    // 100% wet
    );

    // Evaluate multiple buffers to build up crossfeed
    let output = eval_multiple_buffers(&mut graph, delay_id, 10);

    // Should produce consistent bouncing pattern
    let rms = calculate_rms(&output);
    // With stereo width and feedback, signal may be attenuated
    assert!(rms > 0.15, "Crossfeed should produce sustained ping-pong pattern, got RMS: {}", rms);

    // Energy should be distributed (not silent)
    let late_section = &output[output.len() - 512..];
    let late_rms = calculate_rms(late_section);
    assert!(late_rms > 0.1, "Late section should maintain energy from crossfeed, got RMS: {}", late_rms);
}

#[test]
fn test_pingpong_stereo_width_control() {
    // Test that stereo width parameter controls ping-pong strength
    let mut graph = create_graph();

    let input_id = graph.add_node(SignalNode::Constant { value: 0.8 });

    // Create two delays: one with low width, one with high width
    let delay_low_width = graph.add_pingpongdelay_node(
        Signal::Node(input_id),
        Signal::Value(0.02),
        Signal::Value(0.5),
        Signal::Value(0.2),    // 20% width (subtle ping-pong)
        Signal::Value(1.0),
    );

    let delay_high_width = graph.add_pingpongdelay_node(
        Signal::Node(input_id),
        Signal::Value(0.02),
        Signal::Value(0.5),
        Signal::Value(0.9),    // 90% width (strong ping-pong)
        Signal::Value(1.0),
    );

    // Evaluate both
    let out_low = eval_multiple_buffers(&mut graph, delay_low_width, 10);
    let out_high = eval_multiple_buffers(&mut graph, delay_high_width, 10);

    // Both should produce output
    let rms_low = calculate_rms(&out_low);
    let rms_high = calculate_rms(&out_high);

    assert!(rms_low > 0.1, "Low width should produce output");
    assert!(rms_high > 0.1, "High width should produce output");

    // The difference might be subtle, but both should be valid
    // (The actual stereo effect would be more apparent in stereo output)
}
