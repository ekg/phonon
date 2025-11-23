use phonon::unified_graph::{Signal, UnifiedSignalGraph};

/// Helper to create test graph
fn create_test_graph() -> UnifiedSignalGraph {
    UnifiedSignalGraph::new(44100.0)
}

/// Calculate RMS (root mean square) of buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Calculate mean (average) of buffer
fn calculate_mean(buffer: &[f32]) -> f32 {
    buffer.iter().sum::<f32>() / buffer.len() as f32
}

/// Calculate standard deviation of buffer
fn calculate_std_dev(buffer: &[f32], mean: f32) -> f32 {
    let sum_squared_diff: f32 = buffer.iter().map(|&x| (x - mean).powi(2)).sum();
    (sum_squared_diff / buffer.len() as f32).sqrt()
}

#[test]
fn test_whitenoise_range() {
    let mut graph = create_test_graph();

    let noise_id = graph.add_whitenoise_node();

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&noise_id, &mut output);

    // All samples should be in [-1, 1]
    for &sample in &output {
        assert!(sample >= -1.0 && sample <= 1.0,
            "Sample out of range: {}", sample);
    }
}

#[test]
fn test_whitenoise_distribution() {
    let mut graph = create_test_graph();

    let noise_id = graph.add_whitenoise_node();

    // Generate large buffer for statistical test
    let buffer_size = 4096;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&noise_id, &mut output);

    // Mean should be close to 0
    let mean = calculate_mean(&output);
    assert!(mean.abs() < 0.1, "Mean too far from 0: {}", mean);

    // RMS should be around 1/√3 ≈ 0.577 for uniform distribution in [-1, 1]
    let rms = calculate_rms(&output);
    assert!(rms > 0.5 && rms < 0.7, "RMS unexpected: {}", rms);
}

#[test]
fn test_whitenoise_randomness() {
    let mut graph = create_test_graph();

    let noise_id = graph.add_whitenoise_node();

    let buffer_size = 512;
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&noise_id, &mut buffer1);
    graph.eval_node_buffer(&noise_id, &mut buffer2);

    // Buffers should be different
    let mut same_count = 0;
    for i in 0..buffer_size {
        if (buffer1[i] - buffer2[i]).abs() < 0.001 {
            same_count += 1;
        }
    }

    // Very unlikely to have many identical samples
    assert!(same_count < buffer_size / 10,
        "Too many identical samples: {}", same_count);
}

#[test]
fn test_whitenoise_no_nan() {
    let mut graph = create_test_graph();

    let noise_id = graph.add_whitenoise_node();

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&noise_id, &mut output);

    for &sample in &output {
        assert!(sample.is_finite(), "Non-finite value: {}", sample);
    }
}

#[test]
fn test_whitenoise_independence() {
    let mut graph = create_test_graph();

    let noise_id = graph.add_whitenoise_node();

    let buffer_size = 1024;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&noise_id, &mut output);

    // Test for independence: consecutive samples shouldn't be highly correlated
    // Calculate autocorrelation at lag 1
    let mut sum_products = 0.0;
    let mut sum_squares_1 = 0.0;
    let mut sum_squares_2 = 0.0;

    for i in 0..(buffer_size - 1) {
        sum_products += output[i] * output[i + 1];
        sum_squares_1 += output[i] * output[i];
        sum_squares_2 += output[i + 1] * output[i + 1];
    }

    let correlation = sum_products / ((sum_squares_1 * sum_squares_2).sqrt());

    // Correlation should be close to 0 for independent samples
    assert!(correlation.abs() < 0.15,
        "Autocorrelation too high: {}", correlation);
}

#[test]
fn test_whitenoise_not_all_same() {
    let mut graph = create_test_graph();

    let noise_id = graph.add_whitenoise_node();

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&noise_id, &mut output);

    // Not all samples should be identical
    let first_sample = output[0];
    let all_same = output.iter().all(|&x| (x - first_sample).abs() < 0.0001);

    assert!(!all_same, "All samples are identical!");
}

#[test]
fn test_whitenoise_multiple_buffers() {
    let mut graph = create_test_graph();

    let noise_id = graph.add_whitenoise_node();

    let buffer_size = 256;
    let num_buffers = 4;
    let mut buffers: Vec<Vec<f32>> = vec![vec![0.0; buffer_size]; num_buffers];

    // Generate multiple buffers
    for buffer in &mut buffers {
        graph.eval_node_buffer(&noise_id, buffer);
    }

    // Each buffer should be different
    for i in 0..num_buffers {
        for j in (i + 1)..num_buffers {
            let mut same_count = 0;
            for k in 0..buffer_size {
                if (buffers[i][k] - buffers[j][k]).abs() < 0.001 {
                    same_count += 1;
                }
            }
            assert!(same_count < buffer_size / 10,
                "Buffers {} and {} too similar: {} same samples", i, j, same_count);
        }
    }
}

#[test]
fn test_whitenoise_long_term_stats() {
    let mut graph = create_test_graph();

    let noise_id = graph.add_whitenoise_node();

    // Generate very large buffer for better statistical accuracy
    let buffer_size = 8192;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&noise_id, &mut output);

    let mean = calculate_mean(&output);
    let std_dev = calculate_std_dev(&output, mean);

    // Mean should be very close to 0 with large sample
    assert!(mean.abs() < 0.05, "Mean too far from 0: {}", mean);

    // Standard deviation for uniform distribution on [-1, 1] is √(1/3) ≈ 0.577
    let expected_std_dev = (1.0 / 3.0_f32).sqrt();
    assert!((std_dev - expected_std_dev).abs() < 0.05,
        "Std dev {} too far from expected {}", std_dev, expected_std_dev);
}

#[test]
fn test_whitenoise_amplitude_scaling() {
    let mut graph = create_test_graph();

    let noise_id = graph.add_whitenoise_node();
    let scale = 0.5;
    let scaled_id = graph.add_multiply_node(
        Signal::Node(noise_id),
        Signal::Value(scale)
    );

    let buffer_size = 1024;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&scaled_id, &mut output);

    // Scaled samples should be in [-0.5, 0.5]
    for &sample in &output {
        assert!(sample >= -0.5 && sample <= 0.5,
            "Scaled sample out of range: {}", sample);
    }

    // RMS should scale proportionally
    let rms = calculate_rms(&output);
    let expected_rms = 0.577 * scale;
    assert!((rms - expected_rms).abs() < 0.05,
        "Scaled RMS {} not close to expected {}", rms, expected_rms);
}

#[test]
fn test_whitenoise_through_lowpass() {
    let mut graph = create_test_graph();

    let noise_id = graph.add_whitenoise_node();
    let filtered_id = graph.add_lowpass_node(
        Signal::Node(noise_id),
        Signal::Value(1000.0),  // 1kHz cutoff
        Signal::Value(0.707)     // Q = 1/√2
    );

    let buffer_size = 1024;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&filtered_id, &mut output);

    // Should still be in valid range
    for &sample in &output {
        assert!(sample.is_finite(), "Non-finite filtered value: {}", sample);
        assert!(sample >= -2.0 && sample <= 2.0,
            "Filtered sample out of reasonable range: {}", sample);
    }

    // Should have some energy
    let rms = calculate_rms(&output);
    assert!(rms > 0.1, "Filtered noise RMS too low: {}", rms);
}

#[test]
fn test_whitenoise_through_highpass() {
    let mut graph = create_test_graph();

    let noise_id = graph.add_whitenoise_node();
    let filtered_id = graph.add_highpass_node(
        Signal::Node(noise_id),
        Signal::Value(100.0),    // 100Hz cutoff
        Signal::Value(0.707)     // Q = 1/√2
    );

    let buffer_size = 1024;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&filtered_id, &mut output);

    // Should still be in valid range
    for &sample in &output {
        assert!(sample.is_finite(), "Non-finite filtered value: {}", sample);
        assert!(sample >= -2.0 && sample <= 2.0,
            "Filtered sample out of reasonable range: {}", sample);
    }

    // Should have some energy
    let rms = calculate_rms(&output);
    assert!(rms > 0.1, "Filtered noise RMS too low: {}", rms);
}

#[test]
fn test_whitenoise_buffer_sizes() {
    let mut graph = create_test_graph();

    let noise_id = graph.add_whitenoise_node();

    // Test various buffer sizes
    let sizes = [64, 128, 256, 512, 1024, 2048];

    for &size in &sizes {
        let mut output = vec![0.0; size];
        graph.eval_node_buffer(&noise_id, &mut output);

        // Verify range for all buffer sizes
        for &sample in &output {
            assert!(sample >= -1.0 && sample <= 1.0,
                "Sample out of range for buffer size {}: {}", size, sample);
        }

        // Verify some randomness
        let mean = calculate_mean(&output);
        assert!(mean.abs() < 0.2, "Mean too far from 0 for size {}: {}", size, mean);
    }
}

#[test]
fn test_whitenoise_mixed_with_silence() {
    let mut graph = create_test_graph();

    let noise_id = graph.add_whitenoise_node();
    let silence_id = graph.add_node(phonon::unified_graph::SignalNode::Constant { value: 0.0 });
    let mixed_id = graph.add_add_node(
        Signal::Node(noise_id),
        Signal::Node(silence_id)
    );

    let buffer_size = 512;
    let mut noise_buffer = vec![0.0; buffer_size];
    let mut mixed_buffer = vec![0.0; buffer_size];

    graph.eval_node_buffer(&noise_id, &mut noise_buffer);
    graph.eval_node_buffer(&mixed_id, &mut mixed_buffer);

    // Mixed should be same as noise (adding 0 doesn't change it)
    for i in 0..buffer_size {
        assert!((noise_buffer[i] - mixed_buffer[i]).abs() < 0.0001,
            "Mixed signal differs from noise at index {}", i);
    }
}

#[test]
fn test_whitenoise_two_instances() {
    let mut graph = create_test_graph();

    // Create two separate noise nodes
    let noise1_id = graph.add_whitenoise_node();
    let noise2_id = graph.add_whitenoise_node();

    let buffer_size = 512;
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&noise1_id, &mut buffer1);
    graph.eval_node_buffer(&noise2_id, &mut buffer2);

    // Two separate noise nodes should produce different outputs
    let mut same_count = 0;
    for i in 0..buffer_size {
        if (buffer1[i] - buffer2[i]).abs() < 0.001 {
            same_count += 1;
        }
    }

    assert!(same_count < buffer_size / 10,
        "Two noise instances too similar: {} same samples", same_count);
}

#[test]
fn test_whitenoise_coverage_positive_negative() {
    let mut graph = create_test_graph();

    let noise_id = graph.add_whitenoise_node();

    let buffer_size = 1024;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&noise_id, &mut output);

    // Count positive and negative samples
    let positive_count = output.iter().filter(|&&x| x > 0.0).count();
    let negative_count = output.iter().filter(|&&x| x < 0.0).count();

    // Should have roughly equal positive and negative samples
    let ratio = positive_count as f32 / buffer_size as f32;
    assert!(ratio > 0.3 && ratio < 0.7,
        "Imbalanced positive/negative ratio: {}", ratio);

    // Should have both positive and negative samples
    assert!(positive_count > 0, "No positive samples!");
    assert!(negative_count > 0, "No negative samples!");
}
