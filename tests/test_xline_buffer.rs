use phonon::unified_graph::{Signal, UnifiedGraph};

const SAMPLE_RATE: f32 = 44100.0;
const BUFFER_SIZE: usize = 512;

/// Helper to create a test graph
fn create_test_graph() -> UnifiedGraph {
    UnifiedGraph::new(SAMPLE_RATE, 1.0)
}

/// Test 1: Basic exponential ramp (descending)
#[test]
fn test_xline_basic_exponential_descending() {
    let mut graph = create_test_graph();

    let xline_id = graph.add_xline_node(
        Signal::Value(1.0),
        Signal::Value(0.1),
        Signal::Value(1.0),  // 1 second
    );

    let mut output = vec![0.0; 44100];
    graph.eval_node_buffer(&xline_id, &mut output);

    // Check start value (should be near 1.0)
    assert!(
        (output[0] - 1.0).abs() < 0.1,
        "Should start near 1.0, got {}",
        output[0]
    );

    // Check end value (should be near 0.1)
    let end_val = output[44099];
    assert!(
        (end_val - 0.1).abs() < 0.05,
        "Should end near 0.1, got {}",
        end_val
    );

    // Check that it's monotonically decreasing
    for i in 1..output.len() {
        assert!(
            output[i] <= output[i-1] + 0.001, // Allow small tolerance for numerical precision
            "XLine should be monotonically decreasing, but output[{}]={} > output[{}]={}",
            i, output[i], i-1, output[i-1]
        );
    }

    // Check exponential shape: midpoint should be sqrt(1.0 * 0.1) ≈ 0.316
    let mid_val = output[22050];
    let expected_mid = (1.0 * 0.1_f32).sqrt();
    assert!(
        (mid_val - expected_mid).abs() < 0.1,
        "Exponential midpoint should be near {}, got {}",
        expected_mid, mid_val
    );
}

/// Test 2: Basic exponential ramp (ascending)
#[test]
fn test_xline_basic_exponential_ascending() {
    let mut graph = create_test_graph();

    let xline_id = graph.add_xline_node(
        Signal::Value(0.1),
        Signal::Value(1.0),
        Signal::Value(1.0),  // 1 second
    );

    let mut output = vec![0.0; 44100];
    graph.eval_node_buffer(&xline_id, &mut output);

    // Check start value
    assert!(
        (output[0] - 0.1).abs() < 0.05,
        "Should start near 0.1, got {}",
        output[0]
    );

    // Check end value
    let end_val = output[44099];
    assert!(
        (end_val - 1.0).abs() < 0.1,
        "Should end near 1.0, got {}",
        end_val
    );

    // Check that it's monotonically increasing
    for i in 1..output.len() {
        assert!(
            output[i] >= output[i-1] - 0.001,
            "XLine should be monotonically increasing"
        );
    }
}

/// Test 3: Different durations
#[test]
fn test_xline_different_durations() {
    let mut graph = create_test_graph();

    // Short duration (0.5 seconds)
    let xline_short = graph.add_xline_node(
        Signal::Value(1.0),
        Signal::Value(0.1),
        Signal::Value(0.5),
    );

    let mut output_short = vec![0.0; 44100];
    graph.eval_node_buffer(&xline_short, &mut output_short);

    // At 0.5 seconds (sample 22050), should be at end value
    let val_at_half = output_short[22050];
    assert!(
        (val_at_half - 0.1).abs() < 0.05,
        "Should reach end value at 0.5s, got {}",
        val_at_half
    );

    // After 0.5 seconds, should hold at end value
    let val_after = output_short[30000];
    assert!(
        (val_after - 0.1).abs() < 0.05,
        "Should hold end value after duration, got {}",
        val_after
    );
}

/// Test 4: Zero start value (should fall back to linear)
#[test]
fn test_xline_zero_start() {
    let mut graph = create_test_graph();

    let xline_id = graph.add_xline_node(
        Signal::Value(0.0),
        Signal::Value(1.0),
        Signal::Value(1.0),
    );

    let mut output = vec![0.0; 44100];
    graph.eval_node_buffer(&xline_id, &mut output);

    // Should start at 0
    assert!(
        output[0].abs() < 0.01,
        "Should start at 0, got {}",
        output[0]
    );

    // Should end at 1
    assert!(
        (output[44099] - 1.0).abs() < 0.1,
        "Should end at 1, got {}",
        output[44099]
    );

    // Should be approximately linear (not exponential)
    let mid_val = output[22050];
    assert!(
        (mid_val - 0.5).abs() < 0.1,
        "With zero start, should be linear with midpoint near 0.5, got {}",
        mid_val
    );
}

/// Test 5: Different signs (should fall back to linear)
#[test]
fn test_xline_different_signs() {
    let mut graph = create_test_graph();

    let xline_id = graph.add_xline_node(
        Signal::Value(-1.0),
        Signal::Value(1.0),
        Signal::Value(1.0),
    );

    let mut output = vec![0.0; 44100];
    graph.eval_node_buffer(&xline_id, &mut output);

    // Should start at -1
    assert!(
        (output[0] + 1.0).abs() < 0.1,
        "Should start at -1, got {}",
        output[0]
    );

    // Should end at 1
    assert!(
        (output[44099] - 1.0).abs() < 0.1,
        "Should end at 1, got {}",
        output[44099]
    );

    // Should pass through 0
    let mid_val = output[22050];
    assert!(
        mid_val.abs() < 0.1,
        "With different signs, should be linear with midpoint near 0, got {}",
        mid_val
    );
}

/// Test 6: Very short duration (should jump immediately)
#[test]
fn test_xline_very_short_duration() {
    let mut graph = create_test_graph();

    let xline_id = graph.add_xline_node(
        Signal::Value(1.0),
        Signal::Value(0.1),
        Signal::Value(0.00001),  // Very short
    );

    let mut output = vec![0.0; 1000];
    graph.eval_node_buffer(&xline_id, &mut output);

    // Should jump to end value almost immediately
    assert!(
        (output[10] - 0.1).abs() < 0.1,
        "With very short duration, should jump to end quickly, got {}",
        output[10]
    );
}

/// Test 7: Multiple buffers (state continuity)
#[test]
fn test_xline_state_continuity() {
    let mut graph = create_test_graph();

    let xline_id = graph.add_xline_node(
        Signal::Value(1.0),
        Signal::Value(0.1),
        Signal::Value(2.0),  // 2 seconds
    );

    // First buffer (0.5s)
    let mut buffer1 = vec![0.0; 22050];
    graph.eval_node_buffer(&xline_id, &mut buffer1);

    // Second buffer (0.5s)
    let mut buffer2 = vec![0.0; 22050];
    graph.eval_node_buffer(&xline_id, &mut buffer2);

    // Values should continue smoothly
    let last_of_first = buffer1[22049];
    let first_of_second = buffer2[0];

    // Second buffer should continue from where first left off
    assert!(
        first_of_second <= last_of_first,
        "Second buffer should continue decreasing from first, got {} -> {}",
        last_of_first, first_of_second
    );

    // Both should be monotonically decreasing
    assert!(buffer1[0] > buffer1[22049], "First buffer should decrease");
    assert!(buffer2[0] > buffer2[22049], "Second buffer should decrease");
}

/// Test 8: Compare to linear (Line) - should be different
#[test]
fn test_xline_vs_line_shape() {
    let mut graph = create_test_graph();

    // XLine (exponential)
    let xline_id = graph.add_xline_node(
        Signal::Value(1.0),
        Signal::Value(0.1),
        Signal::Value(1.0),
    );

    let mut xline_output = vec![0.0; 44100];
    graph.eval_node_buffer(&xline_id, &mut xline_output);

    // Compare shapes at various points
    // For exponential decay from 1.0 to 0.1, curve should be steeper at start
    let quarter_point = xline_output[11025];
    let three_quarter_point = xline_output[33075];

    // Exponential should drop faster initially
    assert!(
        quarter_point < 0.7,
        "Exponential should drop below 0.7 at 1/4 point, got {}",
        quarter_point
    );

    // But slow down later
    assert!(
        three_quarter_point > 0.15,
        "Exponential should still be above 0.15 at 3/4 point, got {}",
        three_quarter_point
    );
}

/// Test 9: Large range (100 to 1)
#[test]
fn test_xline_large_range() {
    let mut graph = create_test_graph();

    let xline_id = graph.add_xline_node(
        Signal::Value(100.0),
        Signal::Value(1.0),
        Signal::Value(1.0),
    );

    let mut output = vec![0.0; 44100];
    graph.eval_node_buffer(&xline_id, &mut output);

    // Should start near 100
    assert!(
        (output[0] - 100.0).abs() < 10.0,
        "Should start near 100, got {}",
        output[0]
    );

    // Should end near 1
    assert!(
        (output[44099] - 1.0).abs() < 0.5,
        "Should end near 1, got {}",
        output[44099]
    );

    // Midpoint should be geometric mean: sqrt(100 * 1) = 10
    let mid_val = output[22050];
    assert!(
        (mid_val - 10.0).abs() < 3.0,
        "Exponential midpoint should be near 10, got {}",
        mid_val
    );
}

/// Test 10: Small range (1.0 to 0.9)
#[test]
fn test_xline_small_range() {
    let mut graph = create_test_graph();

    let xline_id = graph.add_xline_node(
        Signal::Value(1.0),
        Signal::Value(0.9),
        Signal::Value(1.0),
    );

    let mut output = vec![0.0; 44100];
    graph.eval_node_buffer(&xline_id, &mut output);

    // Should start near 1.0
    assert!(
        (output[0] - 1.0).abs() < 0.05,
        "Should start near 1.0, got {}",
        output[0]
    );

    // Should end near 0.9
    assert!(
        (output[44099] - 0.9).abs() < 0.05,
        "Should end near 0.9, got {}",
        output[44099]
    );

    // Should be smooth (small range)
    for i in 1..output.len() {
        let delta = (output[i] - output[i-1]).abs();
        assert!(
            delta < 0.01,
            "Changes should be small and smooth, got delta={}",
            delta
        );
    }
}

/// Test 11: Multiple small buffers (realistic audio processing)
#[test]
fn test_xline_multiple_small_buffers() {
    let mut graph = create_test_graph();

    let xline_id = graph.add_xline_node(
        Signal::Value(1.0),
        Signal::Value(0.1),
        Signal::Value(1.0),
    );

    // Process in small buffers (typical audio engine behavior)
    let num_buffers = 86; // 44100 samples / 512 samples per buffer ≈ 86
    let mut all_samples = Vec::new();

    for _ in 0..num_buffers {
        let mut buffer = vec![0.0; BUFFER_SIZE];
        graph.eval_node_buffer(&xline_id, &mut buffer);
        all_samples.extend_from_slice(&buffer);
    }

    // Should still form smooth exponential curve
    let start_val = all_samples[0];
    let end_val = all_samples[all_samples.len() - 1];

    assert!(
        (start_val - 1.0).abs() < 0.1,
        "Should start near 1.0 even with small buffers, got {}",
        start_val
    );

    assert!(
        (end_val - 0.1).abs() < 0.1,
        "Should end near 0.1 even with small buffers, got {}",
        end_val
    );

    // Check for smooth transitions between buffers
    for i in 1..all_samples.len() {
        assert!(
            all_samples[i] <= all_samples[i-1] + 0.001,
            "Should be smooth across buffer boundaries"
        );
    }
}

/// Test 12: Hold at end value after duration completes
#[test]
fn test_xline_holds_at_end() {
    let mut graph = create_test_graph();

    let xline_id = graph.add_xline_node(
        Signal::Value(1.0),
        Signal::Value(0.1),
        Signal::Value(0.5),  // Only 0.5 seconds
    );

    let mut output = vec![0.0; 44100]; // Render 1 second
    graph.eval_node_buffer(&xline_id, &mut output);

    // After 0.5s, should hold at end value
    let val_at_0_6s = output[26460]; // 0.6 seconds
    let val_at_0_8s = output[35280]; // 0.8 seconds
    let val_at_1_0s = output[44099]; // 1.0 second

    // All should be very close to end value
    assert!(
        (val_at_0_6s - 0.1).abs() < 0.05,
        "Should hold at end value after duration, got {} at 0.6s",
        val_at_0_6s
    );

    assert!(
        (val_at_0_8s - 0.1).abs() < 0.05,
        "Should hold at end value after duration, got {} at 0.8s",
        val_at_0_8s
    );

    assert!(
        (val_at_1_0s - 0.1).abs() < 0.05,
        "Should hold at end value after duration, got {} at 1.0s",
        val_at_1_0s
    );
}
