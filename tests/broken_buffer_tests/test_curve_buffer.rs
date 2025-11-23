/// Tests for Curve (curved ramp generator) buffer-based evaluation
///
/// These tests verify that Curve buffer evaluation produces correct
/// exponential/logarithmic curves and responds properly to parameter changes.
///
/// Curve generates smooth ramps with shape control:
/// - curve < 0: Concave (slow start, fast end) - logarithmic
/// - curve = 0: Linear
/// - curve > 0: Convex (fast start, slow end) - exponential

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
// TEST: Basic Curve Shapes
// ============================================================================

#[test]
fn test_curve_linear_shape() {
    let mut graph = create_test_graph();

    // Curve = 0 should be linear
    let curve_id = graph.add_curve_node(
        Signal::Value(0.0),
        Signal::Value(1.0),
        Signal::Value(1.0),   // 1 second
        Signal::Value(0.0),   // Linear (curve = 0)
    );

    let buffer_size = 44100;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&curve_id, &mut output);

    // Should start at 0
    assert!(
        output[0].abs() < 0.05,
        "Linear curve should start at 0, got {}",
        output[0]
    );

    // Should end at 1
    assert!(
        (output[44099] - 1.0).abs() < 0.05,
        "Linear curve should end at 1, got {}",
        output[44099]
    );

    // Midpoint should be approximately 0.5 (linear)
    let mid_val = output[22050];
    assert!(
        (mid_val - 0.5).abs() < 0.1,
        "Linear curve midpoint should be near 0.5, got {}",
        mid_val
    );

    println!("Linear curve: start={}, mid={}, end={}", output[0], mid_val, output[44099]);
}

#[test]
fn test_curve_convex_shape() {
    let mut graph = create_test_graph();

    // Curve < 0 should be convex (fast start, slow end)
    let curve_id = graph.add_curve_node(
        Signal::Value(0.0),
        Signal::Value(1.0),
        Signal::Value(1.0),   // 1 second
        Signal::Value(-5.0),  // Convex (negative curve)
    );

    let buffer_size = 44100;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&curve_id, &mut output);

    // Should start at 0
    assert!(
        output[0].abs() < 0.05,
        "Convex curve should start at 0, got {}",
        output[0]
    );

    // Should end at 1
    assert!(
        (output[44099] - 1.0).abs() < 0.05,
        "Convex curve should end at 1, got {}",
        output[44099]
    );

    // Midpoint should be ABOVE 0.5 (convex = fast start)
    let mid_val = output[22050];
    assert!(
        mid_val > 0.6,
        "Convex curve midpoint should be above 0.6, got {}",
        mid_val
    );

    println!("Convex curve: start={}, mid={}, end={}", output[0], mid_val, output[44099]);
}

#[test]
fn test_curve_concave_shape() {
    let mut graph = create_test_graph();

    // Curve > 0 should be concave (slow start, fast end)
    let curve_id = graph.add_curve_node(
        Signal::Value(0.0),
        Signal::Value(1.0),
        Signal::Value(1.0),    // 1 second
        Signal::Value(5.0),    // Concave (positive curve)
    );

    let buffer_size = 44100;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&curve_id, &mut output);

    // Should start at 0
    assert!(
        output[0].abs() < 0.05,
        "Concave curve should start at 0, got {}",
        output[0]
    );

    // Should end at 1
    assert!(
        (output[44099] - 1.0).abs() < 0.05,
        "Concave curve should end at 1, got {}",
        output[44099]
    );

    // Midpoint should be BELOW 0.5 (concave = slow start)
    let mid_val = output[22050];
    assert!(
        mid_val < 0.4,
        "Concave curve midpoint should be below 0.4, got {}",
        mid_val
    );

    println!("Concave curve: start={}, mid={}, end={}", output[0], mid_val, output[44099]);
}

#[test]
fn test_curve_concave_vs_convex() {
    let mut graph = create_test_graph();

    // Concave (positive curve = slow start)
    let concave_id = graph.add_curve_node(
        Signal::Value(0.0),
        Signal::Value(1.0),
        Signal::Value(1.0),
        Signal::Value(5.0),   // Concave
    );

    // Convex (negative curve = fast start)
    let convex_id = graph.add_curve_node(
        Signal::Value(0.0),
        Signal::Value(1.0),
        Signal::Value(1.0),
        Signal::Value(-5.0),  // Convex
    );

    let buffer_size = 44100;
    let mut concave = vec![0.0; buffer_size];
    let mut convex = vec![0.0; buffer_size];

    graph.eval_node_buffer(&concave_id, &mut concave);
    graph.eval_node_buffer(&convex_id, &mut convex);

    // At midpoint, concave should be lower than convex
    assert!(
        concave[22050] < convex[22050],
        "Concave should be below convex at midpoint: concave={}, convex={}",
        concave[22050], convex[22050]
    );

    // At 1/4 point, difference should be even more pronounced
    assert!(
        concave[11025] < convex[11025],
        "Concave should be below convex at 1/4 point: concave={}, convex={}",
        concave[11025], convex[11025]
    );

    println!("Midpoint comparison: concave={}, convex={}", concave[22050], convex[22050]);
}

// ============================================================================
// TEST: Different Curve Values
// ============================================================================

#[test]
fn test_curve_different_values() {
    let mut graph = create_test_graph();

    // Test various curve values
    let curve_vals = vec![-10.0, -5.0, 0.0, 5.0, 10.0];
    let mut midpoints = Vec::new();

    for curve_val in &curve_vals {
        let curve_id = graph.add_curve_node(
            Signal::Value(0.0),
            Signal::Value(1.0),
            Signal::Value(1.0),
            Signal::Value(*curve_val),
        );

        let mut output = vec![0.0; 44100];
        graph.eval_node_buffer(&curve_id, &mut output);

        midpoints.push(output[22050]);
    }

    // Midpoints should DECREASE with curve value
    // (more negative = higher midpoint [convex], more positive = lower midpoint [concave])
    for i in 1..midpoints.len() {
        assert!(
            midpoints[i] < midpoints[i-1],
            "Midpoints should decrease with curve value: {} vs {}",
            midpoints[i-1], midpoints[i]
        );
    }

    println!("Midpoints for different curves: {:?}", midpoints);
}

// ============================================================================
// TEST: Duration
// ============================================================================

#[test]
fn test_curve_different_durations() {
    let mut graph = create_test_graph();

    // Short duration (0.5 seconds)
    let curve_short = graph.add_curve_node(
        Signal::Value(0.0),
        Signal::Value(1.0),
        Signal::Value(0.5),
        Signal::Value(5.0),
    );

    let mut output_short = vec![0.0; 44100];
    graph.eval_node_buffer(&curve_short, &mut output_short);

    // At 0.5 seconds (sample 22050), should be at end value
    let val_at_half = output_short[22050];
    assert!(
        (val_at_half - 1.0).abs() < 0.05,
        "Should reach end value at 0.5s, got {}",
        val_at_half
    );

    // After 0.5 seconds, should hold at end value
    let val_after = output_short[30000];
    assert!(
        (val_after - 1.0).abs() < 0.05,
        "Should hold end value after duration, got {}",
        val_after
    );

    println!("Short duration: val@0.5s={}, val@0.68s={}", val_at_half, val_after);
}

#[test]
fn test_curve_holds_at_end() {
    let mut graph = create_test_graph();

    let curve_id = graph.add_curve_node(
        Signal::Value(0.0),
        Signal::Value(1.0),
        Signal::Value(0.5),  // Only 0.5 seconds
        Signal::Value(5.0),
    );

    let mut output = vec![0.0; 44100]; // Render 1 second
    graph.eval_node_buffer(&curve_id, &mut output);

    // After 0.5s, should hold at end value
    let val_at_0_6s = output[26460]; // 0.6 seconds
    let val_at_0_8s = output[35280]; // 0.8 seconds
    let val_at_1_0s = output[44099]; // 1.0 second

    // All should be very close to end value
    for (time, val) in &[(0.6, val_at_0_6s), (0.8, val_at_0_8s), (1.0, val_at_1_0s)] {
        assert!(
            (val - 1.0).abs() < 0.05,
            "Should hold at end value after duration, got {} at {}s",
            val, time
        );
    }

    println!("Hold test: 0.6s={}, 0.8s={}, 1.0s={}", val_at_0_6s, val_at_0_8s, val_at_1_0s);
}

// ============================================================================
// TEST: Comparison with Line (linear)
// ============================================================================

#[test]
fn test_curve_zero_matches_line() {
    let mut graph = create_test_graph();

    // Curve with curve=0 should match Line
    let curve_id = graph.add_curve_node(
        Signal::Value(0.0),
        Signal::Value(1.0),
        Signal::Value(1.0),
        Signal::Value(0.0),  // Linear
    );

    let mut curve_output = vec![0.0; 44100];
    graph.eval_node_buffer(&curve_id, &mut curve_output);

    // Compare various points to expected linear values
    let points_to_check = vec![
        (0, 0.0),
        (11025, 0.25),
        (22050, 0.5),
        (33075, 0.75),
        (44099, 1.0),
    ];

    for (idx, expected) in points_to_check {
        let actual = curve_output[idx];
        assert!(
            (actual - expected).abs() < 0.05,
            "Linear curve should match Line at sample {}: expected {}, got {}",
            idx, expected, actual
        );
    }

    println!("Linear curve matches Line: passed");
}

// ============================================================================
// TEST: Descending Curves
// ============================================================================

#[test]
fn test_curve_descending() {
    let mut graph = create_test_graph();

    // Descending curve (1.0 to 0.0)
    let curve_id = graph.add_curve_node(
        Signal::Value(1.0),
        Signal::Value(0.0),
        Signal::Value(1.0),
        Signal::Value(5.0),
    );

    let mut output = vec![0.0; 44100];
    graph.eval_node_buffer(&curve_id, &mut output);

    // Should start at 1
    assert!(
        (output[0] - 1.0).abs() < 0.05,
        "Descending curve should start at 1, got {}",
        output[0]
    );

    // Should end at 0
    assert!(
        output[44099].abs() < 0.05,
        "Descending curve should end at 0, got {}",
        output[44099]
    );

    // Should be monotonically decreasing
    for i in 1..output.len() {
        assert!(
            output[i] <= output[i-1] + 0.001,
            "Descending curve should be monotonic at sample {}",
            i
        );
    }

    println!("Descending curve: start={}, end={}", output[0], output[44099]);
}

// ============================================================================
// TEST: State Continuity
// ============================================================================

#[test]
fn test_curve_state_continuity() {
    let mut graph = create_test_graph();

    let curve_id = graph.add_curve_node(
        Signal::Value(0.0),
        Signal::Value(1.0),
        Signal::Value(2.0),  // 2 seconds
        Signal::Value(5.0),
    );

    // First buffer (0.5s)
    let mut buffer1 = vec![0.0; 22050];
    graph.eval_node_buffer(&curve_id, &mut buffer1);

    // Second buffer (0.5s)
    let mut buffer2 = vec![0.0; 22050];
    graph.eval_node_buffer(&curve_id, &mut buffer2);

    // Values should continue smoothly
    let last_of_first = buffer1[22049];
    let first_of_second = buffer2[0];

    // Second buffer should continue from where first left off
    // Allow small discontinuity due to buffer boundaries
    let diff = (first_of_second - last_of_first).abs();
    assert!(
        diff < 0.01,
        "Buffers should be continuous: {} -> {}, diff={}",
        last_of_first, first_of_second, diff
    );

    // Both should be monotonically increasing
    assert!(buffer1[0] < buffer1[22049], "First buffer should increase");
    assert!(buffer2[0] < buffer2[22049], "Second buffer should increase");

    println!("Continuity: buffer1_end={}, buffer2_start={}, diff={}",
        last_of_first, first_of_second, diff);
}

#[test]
fn test_curve_multiple_buffers() {
    let mut graph = create_test_graph();

    let curve_id = graph.add_curve_node(
        Signal::Value(0.0),
        Signal::Value(1.0),
        Signal::Value(0.05),  // Short duration (0.05s = 2205 samples, ~5 buffers)
        Signal::Value(5.0),
    );

    // Generate multiple consecutive buffers
    let buffer_size = 512;
    let num_buffers = 10;
    let mut all_rms = Vec::new();

    for i in 0..num_buffers {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&curve_id, &mut output);

        let rms = calculate_rms(&output);
        all_rms.push(rms);
        println!("Buffer {} RMS: {}", i, rms);
    }

    // First few buffers should have reasonable content (ramping up)
    assert!(all_rms[0] > 0.001, "First buffer should have some content");

    // Later buffers should hold at end value (constant 1.0, so RMS = 1.0)
    let last_rms = all_rms[num_buffers - 1];
    assert!((last_rms - 1.0).abs() < 0.05, "Last buffer should hold at end value 1.0, RMS should be 1.0, got {}", last_rms);
}

// ============================================================================
// TEST: Edge Cases
// ============================================================================

#[test]
fn test_curve_very_short_duration() {
    let mut graph = create_test_graph();

    let curve_id = graph.add_curve_node(
        Signal::Value(0.0),
        Signal::Value(1.0),
        Signal::Value(0.001),  // Very short (1ms = 44 samples at 44.1kHz)
        Signal::Value(5.0),
    );

    let mut output = vec![0.0; 1000];
    graph.eval_node_buffer(&curve_id, &mut output);

    // Should reach end value by sample 50 (after 1ms)
    assert!(
        (output[50] - 1.0).abs() < 0.05,
        "With very short duration, should reach end value quickly, got {}",
        output[50]
    );

    // Should hold at end value
    assert!(
        (output[500] - 1.0).abs() < 0.05,
        "Should hold at end value, got {}",
        output[500]
    );

    println!("Very short duration: sample[50]={}, sample[500]={}", output[50], output[500]);
}

#[test]
fn test_curve_extreme_values() {
    let mut graph = create_test_graph();

    // Test with extreme curve values
    let curve_id_neg = graph.add_curve_node(
        Signal::Value(0.0),
        Signal::Value(1.0),
        Signal::Value(1.0),
        Signal::Value(-20.0),  // Extreme convex (fast start)
    );

    let curve_id_pos = graph.add_curve_node(
        Signal::Value(0.0),
        Signal::Value(1.0),
        Signal::Value(1.0),
        Signal::Value(20.0),   // Extreme concave (slow start)
    );

    let mut output_neg = vec![0.0; 44100];
    let mut output_pos = vec![0.0; 44100];

    graph.eval_node_buffer(&curve_id_neg, &mut output_neg);
    graph.eval_node_buffer(&curve_id_pos, &mut output_pos);

    // Both should still reach start and end values
    assert!(output_neg[0].abs() < 0.05, "Extreme convex curve should start at 0");
    assert!((output_neg[44099] - 1.0).abs() < 0.05, "Extreme convex curve should end at 1");
    assert!(output_pos[0].abs() < 0.05, "Extreme concave curve should start at 0");
    assert!((output_pos[44099] - 1.0).abs() < 0.05, "Extreme concave curve should end at 1");

    // Midpoints should be very different (convex higher than concave)
    assert!(
        output_neg[22050] > output_pos[22050],
        "Extreme curves should have very different midpoints: convex={} should be > concave={}",
        output_neg[22050], output_pos[22050]
    );

    println!("Extreme curves: neg_mid={}, pos_mid={}", output_neg[22050], output_pos[22050]);
}

#[test]
fn test_curve_amplitude_range() {
    let mut graph = create_test_graph();

    let curve_id = graph.add_curve_node(
        Signal::Value(0.0),
        Signal::Value(1.0),
        Signal::Value(1.0),
        Signal::Value(5.0),
    );

    let mut output = vec![0.0; 44100];
    graph.eval_node_buffer(&curve_id, &mut output);

    // All samples should be in valid range [0, 1]
    for (i, &sample) in output.iter().enumerate() {
        assert!(
            sample >= -0.01 && sample <= 1.01,
            "Sample {} out of range [0, 1]: {}",
            i, sample
        );
    }

    println!("Amplitude range test passed");
}

// ============================================================================
// TEST: Musical Use Cases
// ============================================================================

#[test]
fn test_curve_envelope_shapes() {
    let mut graph = create_test_graph();

    // Natural percussive decay (fast start = convex with negative curve)
    let percussive_id = graph.add_curve_node(
        Signal::Value(1.0),
        Signal::Value(0.0),
        Signal::Value(0.5),
        Signal::Value(8.0),   // Positive curve = concave descent (fast end)
    );

    // Smooth rise (slow start = concave with positive curve)
    let rise_id = graph.add_curve_node(
        Signal::Value(0.0),
        Signal::Value(1.0),
        Signal::Value(0.3),
        Signal::Value(5.0),   // Positive curve = concave rise (slow start)
    );

    let mut percussive = vec![0.0; 22050];
    let mut rise = vec![0.0; 13230];

    graph.eval_node_buffer(&percussive_id, &mut percussive);
    graph.eval_node_buffer(&rise_id, &mut rise);

    // Percussive should start high and decay naturally
    assert!(percussive[0] > 0.9, "Percussive should start high");
    assert!(percussive[22049] < 0.1, "Percussive should decay to low");

    // Rise should start low and increase
    assert!(rise[0] < 0.1, "Rise should start low");
    assert!(rise[13229] > 0.9, "Rise should reach high");

    println!("Musical envelopes: percussive_start={}, rise_end={}",
        percussive[0], rise[13229]);
}

// ============================================================================
// TEST: Performance
// ============================================================================

#[test]
fn test_curve_buffer_performance() {
    let mut graph = create_test_graph();

    let curve_id = graph.add_curve_node(
        Signal::Value(0.0),
        Signal::Value(1.0),
        Signal::Value(1.0),
        Signal::Value(5.0),
    );

    let buffer_size = 512;
    let iterations = 1000;

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&curve_id, &mut output);
    }
    let duration = start.elapsed();

    println!("Curve buffer eval: {:?} for {} iterations", duration, iterations);
    println!("Per iteration: {:?}", duration / iterations);

    // Should complete in reasonable time (< 2 seconds for 1000 iterations)
    assert!(
        duration.as_secs() < 2,
        "Curve buffer evaluation too slow: {:?}",
        duration
    );
}
