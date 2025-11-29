/// Tests for buffer-based evaluation framework
///
/// These tests verify that buffer-based evaluation produces correct
/// output, ensuring correctness during the gradual migration from
/// sample-by-sample to buffer-based processing.
use phonon::unified_graph::{Signal, SignalExpr, UnifiedSignalGraph};

/// Helper: Create a test graph with basic setup
fn create_test_graph() -> UnifiedSignalGraph {
    UnifiedSignalGraph::new(44100.0) // 44.1kHz
}

/// Helper: Compare two buffers with floating point tolerance
fn assert_buffers_match(actual: &[f32], expected: &[f32], tolerance: f32) {
    assert_eq!(actual.len(), expected.len(), "Buffer sizes don't match");

    for (i, (&a, &e)) in actual.iter().zip(expected.iter()).enumerate() {
        let diff = (a - e).abs();
        assert!(
            diff < tolerance,
            "Sample {} differs: actual={}, expected={}, diff={}",
            i,
            a,
            e,
            diff
        );
    }
}

// ============================================================================
// TEST: Constant Signals
// ============================================================================

#[test]
fn test_constant_signal_value() {
    let mut graph = create_test_graph();

    let signal = Signal::Value(0.75);
    let buffer_size = 512;

    // Evaluate signal to buffer
    let mut output = vec![0.0; buffer_size];
    graph.eval_signal_buffer(&signal, &mut output);

    // All samples should equal 0.75
    for &sample in &output {
        assert!((sample - 0.75_f32).abs() < 1e-6);
    }
}

// ============================================================================
// TEST: Arithmetic Operations
// ============================================================================

#[test]
fn test_add_expression_buffer() {
    let mut graph = create_test_graph();

    // Create expression: 0.3 + 0.4 = 0.7
    let expr = SignalExpr::Add(Signal::Value(0.3), Signal::Value(0.4));

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_expression_buffer(&expr, &mut output);

    // All samples should equal 0.7
    for &sample in &output {
        assert!(
            (sample - 0.7_f32).abs() < 1e-6,
            "Expected 0.7, got {}",
            sample
        );
    }
}

#[test]
fn test_multiply_expression_buffer() {
    let mut graph = create_test_graph();

    // Create expression: 0.5 * 0.8 = 0.4
    let expr = SignalExpr::Multiply(Signal::Value(0.5), Signal::Value(0.8));

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_expression_buffer(&expr, &mut output);

    // All samples should equal 0.4
    for &sample in &output {
        assert!(
            (sample - 0.4_f32).abs() < 1e-6,
            "Expected 0.4, got {}",
            sample
        );
    }
}

#[test]
fn test_subtract_expression_buffer() {
    let mut graph = create_test_graph();

    // Create expression: 0.8 - 0.3 = 0.5
    let expr = SignalExpr::Subtract(Signal::Value(0.8), Signal::Value(0.3));

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_expression_buffer(&expr, &mut output);

    // All samples should equal 0.5
    for &sample in &output {
        assert!(
            (sample - 0.5_f32).abs() < 1e-6,
            "Expected 0.5, got {}",
            sample
        );
    }
}

#[test]
fn test_divide_expression_buffer() {
    let mut graph = create_test_graph();

    // Create expression: 0.8 / 0.4 = 2.0
    let expr = SignalExpr::Divide(Signal::Value(0.8), Signal::Value(0.4));

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_expression_buffer(&expr, &mut output);

    // All samples should equal 2.0
    for &sample in &output {
        assert!(
            (sample - 2.0_f32).abs() < 1e-6,
            "Expected 2.0, got {}",
            sample
        );
    }
}

#[test]
fn test_divide_by_zero_buffer() {
    let mut graph = create_test_graph();

    // Create expression: 1.0 / 0.0 = 0.0 (safe handling)
    let expr = SignalExpr::Divide(Signal::Value(1.0), Signal::Value(0.0));

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_expression_buffer(&expr, &mut output);

    // All samples should equal 0.0 (divide by zero returns 0)
    for &sample in &output {
        assert!(
            (sample - 0.0_f32).abs() < 1e-6,
            "Expected 0.0, got {}",
            sample
        );
    }
}

#[test]
fn test_scale_expression_buffer() {
    let mut graph = create_test_graph();

    // Create expression: scale 0.5 from [0,1] to [2,6] = 2 + 0.5*4 = 4.0
    let expr = SignalExpr::Scale {
        input: Signal::Value(0.5),
        min: 2.0,
        max: 6.0,
    };

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_expression_buffer(&expr, &mut output);

    // All samples should equal 4.0
    for &sample in &output {
        assert!(
            (sample - 4.0_f32).abs() < 1e-6,
            "Expected 4.0, got {}",
            sample
        );
    }
}

// ============================================================================
// TEST: Nested Expressions
// ============================================================================

#[test]
fn test_nested_arithmetic_buffer() {
    let mut graph = create_test_graph();

    // Create expression: (0.5 + 0.3) * 2.0 = 0.8 * 2.0 = 1.6
    let add_expr = SignalExpr::Add(Signal::Value(0.5), Signal::Value(0.3));

    let multiply_expr =
        SignalExpr::Multiply(Signal::Expression(Box::new(add_expr)), Signal::Value(2.0));

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_expression_buffer(&multiply_expr, &mut output);

    // All samples should equal 1.6
    for &sample in &output {
        assert!(
            (sample - 1.6_f32).abs() < 1e-5,
            "Expected 1.6, got {}",
            sample
        );
    }
}

#[test]
fn test_complex_nested_expression() {
    let mut graph = create_test_graph();

    // Create expression: ((2 + 3) * 4) - 1 = (5 * 4) - 1 = 20 - 1 = 19
    let add = SignalExpr::Add(Signal::Value(2.0), Signal::Value(3.0));

    let multiply = SignalExpr::Multiply(Signal::Expression(Box::new(add)), Signal::Value(4.0));

    let subtract = SignalExpr::Subtract(Signal::Expression(Box::new(multiply)), Signal::Value(1.0));

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_expression_buffer(&subtract, &mut output);

    // All samples should equal 19.0
    for &sample in &output {
        assert!(
            (sample - 19.0_f32).abs() < 1e-5,
            "Expected 19.0, got {}",
            sample
        );
    }
}

// ============================================================================
// TEST: Buffer Size Variations
// ============================================================================

#[test]
fn test_various_buffer_sizes() {
    let mut graph = create_test_graph();

    let signal = Signal::Value(0.123);

    // Test different buffer sizes
    for size in [1, 16, 64, 128, 256, 512, 1024, 2048] {
        let mut output = vec![0.0; size];
        graph.eval_signal_buffer(&signal, &mut output);

        for &sample in &output {
            assert!(
                (sample - 0.123_f32).abs() < 1e-6,
                "Failed for buffer size {}",
                size
            );
        }
    }
}

// ============================================================================
// TEST: Performance (Sanity Check)
// ============================================================================

#[test]
fn test_buffer_eval_performance_sanity() {
    let mut graph = create_test_graph();

    // Create a moderately complex expression
    let expr = SignalExpr::Multiply(
        Signal::Expression(Box::new(SignalExpr::Add(
            Signal::Value(0.5),
            Signal::Value(0.3),
        ))),
        Signal::Value(2.0),
    );

    let buffer_size = 512;
    let iterations = 1000;

    // Time buffer evaluation
    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let mut output = vec![0.0; buffer_size];
        graph.eval_expression_buffer(&expr, &mut output);
    }
    let buffer_duration = start.elapsed();

    println!(
        "Buffer evaluation: {:?} for {} iterations",
        buffer_duration, iterations
    );
    println!("Per iteration: {:?}", buffer_duration / iterations);

    // Sanity check: Should complete in reasonable time (< 1 second for 1000 iterations)
    assert!(
        buffer_duration.as_secs() < 1,
        "Buffer evaluation too slow: {:?}",
        buffer_duration
    );
}

// ============================================================================
// TEST: Edge Cases
// ============================================================================

#[test]
fn test_empty_buffer() {
    let mut graph = create_test_graph();

    let signal = Signal::Value(0.5);
    let mut output = vec![];

    // Should not panic with empty buffer
    graph.eval_signal_buffer(&signal, &mut output);
    assert_eq!(output.len(), 0);
}

#[test]
fn test_large_values() {
    let mut graph = create_test_graph();

    // Test with very large values
    let expr = SignalExpr::Multiply(Signal::Value(1000.0), Signal::Value(1000.0));

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_expression_buffer(&expr, &mut output);

    for &sample in &output {
        assert!((sample - 1000000.0_f32).abs() < 1.0);
    }
}

#[test]
fn test_very_small_values() {
    let mut graph = create_test_graph();

    // Test with very small values
    let expr = SignalExpr::Multiply(Signal::Value(0.0001), Signal::Value(0.0001));

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_expression_buffer(&expr, &mut output);

    for &sample in &output {
        assert!((sample - 0.00000001_f32).abs() < 1e-9);
    }
}
