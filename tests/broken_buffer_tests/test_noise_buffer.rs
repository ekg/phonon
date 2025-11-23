/// Tests for seeded noise (SignalNode::Noise) buffer-based evaluation
///
/// These tests verify that the deterministic seeded noise generator
/// produces correct, reproducible output with proper statistical properties.
///
/// The Noise node uses a Linear Congruential Generator (LCG) for deterministic
/// noise generation. This is different from WhiteNoise which uses rand::thread_rng().

use phonon::unified_graph::{UnifiedSignalGraph, SignalNode};

/// Helper: Create a test graph
fn create_test_graph() -> UnifiedSignalGraph {
    UnifiedSignalGraph::new(44100.0) // 44.1kHz
}

/// Helper: Calculate RMS of a buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Helper: Calculate mean of a buffer
fn calculate_mean(buffer: &[f32]) -> f32 {
    let sum: f32 = buffer.iter().sum();
    sum / buffer.len() as f32
}

/// Helper: Count samples in range
fn count_in_range(buffer: &[f32], min: f32, max: f32) -> usize {
    buffer.iter().filter(|&&x| x >= min && x <= max).count()
}

// ============================================================================
// TEST: Determinism
// ============================================================================

#[test]
fn test_noise_deterministic() {
    let mut graph = create_test_graph();

    // Two noise nodes with same seed
    let noise1_id = graph.add_node(SignalNode::Noise {
        seed: 12345,
    });
    let noise2_id = graph.add_node(SignalNode::Noise {
        seed: 12345,
    });

    let buffer_size = 512;
    let mut output1 = vec![0.0; buffer_size];
    let mut output2 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&noise1_id, &mut output1);
    graph.eval_node_buffer(&noise2_id, &mut output2);

    // Should be identical
    for i in 0..buffer_size {
        assert_eq!(output1[i], output2[i],
            "Samples differ at index {} (same seed should produce identical output)", i);
    }
}

#[test]
fn test_noise_deterministic_multiple_buffers() {
    let mut graph = create_test_graph();

    let noise_id = graph.add_node(SignalNode::Noise {
        seed: 42,
    });

    // Generate first buffer
    let buffer_size = 512;
    let mut buffer1 = vec![0.0; buffer_size];
    graph.eval_node_buffer(&noise_id, &mut buffer1);

    // Reset by creating a new node with same seed
    let noise_id2 = graph.add_node(SignalNode::Noise {
        seed: 42,
    });
    let mut buffer2 = vec![0.0; buffer_size];
    graph.eval_node_buffer(&noise_id2, &mut buffer2);

    // Should produce same output
    for i in 0..buffer_size {
        assert_eq!(buffer1[i], buffer2[i],
            "Fresh noise node with same seed should produce identical output at sample {}", i);
    }
}

// ============================================================================
// TEST: Different Seeds Produce Different Output
// ============================================================================

#[test]
fn test_noise_different_seeds() {
    let mut graph = create_test_graph();

    let noise1_id = graph.add_node(SignalNode::Noise {
        seed: 12345,
    });
    let noise2_id = graph.add_node(SignalNode::Noise {
        seed: 67890,
    });

    let buffer_size = 512;
    let mut output1 = vec![0.0; buffer_size];
    let mut output2 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&noise1_id, &mut output1);
    graph.eval_node_buffer(&noise2_id, &mut output2);

    // Should be different
    let mut diff_count = 0;
    for i in 0..buffer_size {
        if (output1[i] - output2[i]).abs() > 0.001 {
            diff_count += 1;
        }
    }

    // Most samples should differ (at least 95%)
    assert!(diff_count > buffer_size * 95 / 100,
        "Different seeds should produce different output. Only {}/{} samples differ",
        diff_count, buffer_size);
}

#[test]
fn test_noise_zero_vs_nonzero_seed() {
    let mut graph = create_test_graph();

    let noise1_id = graph.add_node(SignalNode::Noise {
        seed: 0,
    });
    let noise2_id = graph.add_node(SignalNode::Noise {
        seed: 1,
    });

    let buffer_size = 512;
    let mut output1 = vec![0.0; buffer_size];
    let mut output2 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&noise1_id, &mut output1);
    graph.eval_node_buffer(&noise2_id, &mut output2);

    // Even seed 0 should produce valid noise different from seed 1
    let mut diff_count = 0;
    for i in 0..buffer_size {
        if (output1[i] - output2[i]).abs() > 0.001 {
            diff_count += 1;
        }
    }

    assert!(diff_count > buffer_size * 95 / 100,
        "Seed 0 should produce valid different output from seed 1");
}

// ============================================================================
// TEST: Output Range
// ============================================================================

#[test]
fn test_noise_output_range() {
    let mut graph = create_test_graph();

    // Test multiple seeds to ensure range is consistent
    for seed in [1, 42, 12345, 0xDEADBEEF] {
        let noise_id = graph.add_noise_node(seed);

        let buffer_size = 512;
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&noise_id, &mut output);

        // All samples should be in [-1, 1] range
        for (i, &sample) in output.iter().enumerate() {
            assert!(sample >= -1.0 && sample <= 1.0,
                "Sample {} from seed {} out of range: {} (should be in [-1, 1])",
                i, seed, sample);
        }
    }
}

#[test]
fn test_noise_uses_full_range() {
    let mut graph = create_test_graph();

    let noise_id = graph.add_node(SignalNode::Noise {
        seed: 12345,
    });

    // Generate multiple buffers to get good coverage
    let buffer_size = 512;
    let mut all_samples = Vec::new();

    for _ in 0..10 {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&noise_id, &mut output);
        all_samples.extend_from_slice(&output);
    }

    let max_val = all_samples.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
    let min_val = all_samples.iter().fold(f32::INFINITY, |a, &b| a.min(b));

    // Should use most of the [-1, 1] range (at least 80%)
    assert!(max_val > 0.8,
        "Max value {} should be close to 1.0 (using full positive range)", max_val);
    assert!(min_val < -0.8,
        "Min value {} should be close to -1.0 (using full negative range)", min_val);
}

// ============================================================================
// TEST: Statistical Properties
// ============================================================================

#[test]
fn test_noise_mean_near_zero() {
    let mut graph = create_test_graph();

    let noise_id = graph.add_node(SignalNode::Noise {
        seed: 42,
    });

    // Generate large sample to get accurate mean
    let buffer_size = 4096;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&noise_id, &mut output);

    let mean = calculate_mean(&output);

    // Mean should be near 0 for uniform noise in [-1, 1]
    // Allow generous tolerance due to finite sample size
    assert!(mean.abs() < 0.1,
        "Noise mean should be near 0, got {}", mean);
}

#[test]
fn test_noise_rms_appropriate() {
    let mut graph = create_test_graph();

    let noise_id = graph.add_node(SignalNode::Noise {
        seed: 54321,
    });

    let buffer_size = 4096;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&noise_id, &mut output);

    let rms = calculate_rms(&output);

    // For uniform distribution in [-1, 1], RMS should be ~0.577
    // (sqrt(1/3) for uniform distribution)
    assert!(rms > 0.45 && rms < 0.7,
        "Noise RMS should be ~0.577 (uniform distribution), got {}", rms);
}

#[test]
fn test_noise_distribution_uniformity() {
    let mut graph = create_test_graph();

    let noise_id = graph.add_node(SignalNode::Noise {
        seed: 99999,
    });

    // Generate large sample
    let buffer_size = 4096;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&noise_id, &mut output);

    // Divide [-1, 1] into 4 bins and count samples in each
    let count_neg_1_to_neg_half = count_in_range(&output, -1.0, -0.5);
    let count_neg_half_to_0 = count_in_range(&output, -0.5, 0.0);
    let count_0_to_half = count_in_range(&output, 0.0, 0.5);
    let count_half_to_1 = count_in_range(&output, 0.5, 1.0);

    // Each bin should have roughly 25% of samples (allow ±10% deviation)
    let expected = buffer_size / 4;
    let tolerance = expected / 4; // 25% tolerance

    assert!((count_neg_1_to_neg_half as i32 - expected as i32).abs() < tolerance as i32,
        "Bin [-1, -0.5] has {} samples, expected ~{}", count_neg_1_to_neg_half, expected);
    assert!((count_neg_half_to_0 as i32 - expected as i32).abs() < tolerance as i32,
        "Bin [-0.5, 0] has {} samples, expected ~{}", count_neg_half_to_0, expected);
    assert!((count_0_to_half as i32 - expected as i32).abs() < tolerance as i32,
        "Bin [0, 0.5] has {} samples, expected ~{}", count_0_to_half, expected);
    assert!((count_half_to_1 as i32 - expected as i32).abs() < tolerance as i32,
        "Bin [0.5, 1] has {} samples, expected ~{}", count_half_to_1, expected);
}

// ============================================================================
// TEST: State Continuity (Stateful Behavior)
// ============================================================================

#[test]
fn test_noise_state_continues_across_buffers() {
    let mut graph = create_test_graph();

    let noise_id = graph.add_node(SignalNode::Noise {
        seed: 777,
    });

    // Generate three consecutive buffers
    let buffer_size = 512;
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];
    let mut buffer3 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&noise_id, &mut buffer1);
    graph.eval_node_buffer(&noise_id, &mut buffer2);
    graph.eval_node_buffer(&noise_id, &mut buffer3);

    // Buffers should be different from each other (state advances)
    let mut diff_1_2 = 0;
    let mut diff_2_3 = 0;

    for i in 0..buffer_size {
        if (buffer1[i] - buffer2[i]).abs() > 0.001 {
            diff_1_2 += 1;
        }
        if (buffer2[i] - buffer3[i]).abs() > 0.001 {
            diff_2_3 += 1;
        }
    }

    assert!(diff_1_2 > buffer_size * 95 / 100,
        "Consecutive buffers should differ (stateful), only {}/{} samples differ",
        diff_1_2, buffer_size);
    assert!(diff_2_3 > buffer_size * 95 / 100,
        "Consecutive buffers should differ (stateful), only {}/{} samples differ",
        diff_2_3, buffer_size);
}

#[test]
fn test_noise_no_repetition_within_buffer() {
    let mut graph = create_test_graph();

    let noise_id = graph.add_node(SignalNode::Noise {
        seed: 555,
    });

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&noise_id, &mut output);

    // Check that samples don't repeat (LCG should have long period)
    // This is a weak test but checks for obvious failures
    let mut repeat_count = 0;
    for i in 1..buffer_size {
        if (output[i] - output[i-1]).abs() < 0.0001 {
            repeat_count += 1;
        }
    }

    // Should have very few exact repeats (< 5% by chance)
    assert!(repeat_count < buffer_size / 20,
        "Too many repeated samples: {}/{} (possible LCG failure)",
        repeat_count, buffer_size);
}

// ============================================================================
// TEST: Multiple Nodes (Independence)
// ============================================================================

#[test]
fn test_noise_multiple_nodes_independent() {
    let mut graph = create_test_graph();

    // Create multiple noise nodes with different seeds
    let noise1_id = graph.add_node(SignalNode::Noise {
        seed: 111,
    });
    let noise2_id = graph.add_node(SignalNode::Noise {
        seed: 222,
    });
    let noise3_id = graph.add_node(SignalNode::Noise {
        seed: 333,
    });

    let buffer_size = 512;
    let mut output1 = vec![0.0; buffer_size];
    let mut output2 = vec![0.0; buffer_size];
    let mut output3 = vec![0.0; buffer_size];

    // Evaluate in sequence (simulating multiple voices)
    graph.eval_node_buffer(&noise1_id, &mut output1);
    graph.eval_node_buffer(&noise2_id, &mut output2);
    graph.eval_node_buffer(&noise3_id, &mut output3);

    // Each should be different
    let mut diff_1_2 = 0;
    let mut diff_2_3 = 0;
    let mut diff_1_3 = 0;

    for i in 0..buffer_size {
        if (output1[i] - output2[i]).abs() > 0.001 { diff_1_2 += 1; }
        if (output2[i] - output3[i]).abs() > 0.001 { diff_2_3 += 1; }
        if (output1[i] - output3[i]).abs() > 0.001 { diff_1_3 += 1; }
    }

    assert!(diff_1_2 > buffer_size * 95 / 100,
        "Nodes 1 and 2 should be independent");
    assert!(diff_2_3 > buffer_size * 95 / 100,
        "Nodes 2 and 3 should be independent");
    assert!(diff_1_3 > buffer_size * 95 / 100,
        "Nodes 1 and 3 should be independent");
}

// ============================================================================
// TEST: Performance
// ============================================================================

#[test]
fn test_noise_buffer_performance() {
    let mut graph = create_test_graph();

    let noise_id = graph.add_node(SignalNode::Noise {
        seed: 42,
    });

    let buffer_size = 512;
    let iterations = 1000;

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&noise_id, &mut output);
    }
    let duration = start.elapsed();

    println!("Noise buffer eval: {:?} for {} iterations", duration, iterations);
    println!("Per iteration: {:?}", duration / iterations);

    // Should complete in reasonable time (< 0.5 second for 1000 iterations)
    // Noise is simpler than oscillators so should be faster
    assert!(duration.as_millis() < 500,
        "Noise buffer evaluation too slow: {:?}", duration);
}

// ============================================================================
// TEST: Edge Cases
// ============================================================================

#[test]
fn test_noise_max_seed() {
    let mut graph = create_test_graph();

    // Test with maximum u32 seed
    let noise_id = graph.add_noise_node(u32::MAX);

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&noise_id, &mut output);

    // Should still produce valid output
    let rms = calculate_rms(&output);
    assert!(rms > 0.4 && rms < 0.8,
        "Max seed should produce valid noise, RMS = {}", rms);

    // Check range
    for (i, &sample) in output.iter().enumerate() {
        assert!(sample >= -1.0 && sample <= 1.0,
            "Sample {} out of range with max seed: {}", i, sample);
    }
}

#[test]
fn test_noise_small_buffer() {
    let mut graph = create_test_graph();

    let noise_id = graph.add_node(SignalNode::Noise {
        seed: 888,
    });

    // Test with very small buffer
    let buffer_size = 4;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&noise_id, &mut output);

    // Should work with small buffers
    for &sample in &output {
        assert!(sample >= -1.0 && sample <= 1.0,
            "Small buffer sample out of range: {}", sample);
    }

    // All samples should be different
    assert_ne!(output[0], output[1]);
    assert_ne!(output[1], output[2]);
    assert_ne!(output[2], output[3]);
}

#[test]
fn test_noise_large_buffer() {
    let mut graph = create_test_graph();

    let noise_id = graph.add_node(SignalNode::Noise {
        seed: 999,
    });

    // Test with large buffer (1 second at 44.1kHz)
    let buffer_size = 44100;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&noise_id, &mut output);

    // Should work with large buffers
    let rms = calculate_rms(&output);
    assert!(rms > 0.5 && rms < 0.65,
        "Large buffer should have appropriate RMS: {}", rms);

    // Check for long-term statistical properties
    let mean = calculate_mean(&output);
    assert!(mean.abs() < 0.05,
        "Large buffer mean should be very close to 0: {}", mean);
}

// ============================================================================
// TEST: Comparison with Sample-by-Sample Evaluation
// ============================================================================

#[test]
fn test_noise_buffer_matches_sample_by_sample() {
    let mut graph1 = create_test_graph();
    let mut graph2 = create_test_graph();

    let seed = 314159;
    let noise_buffer_id = graph1.add_noise_node(seed);
    let noise_sample_id = graph2.add_noise_node(seed);

    // Evaluate with buffer method
    let buffer_size = 64; // Use smaller size for sample-by-sample comparison
    let mut buffer_output = vec![0.0; buffer_size];
    graph1.eval_node_buffer(&noise_buffer_id, &mut buffer_output);

    // Evaluate with sample-by-sample method
    let mut sample_output = vec![0.0; buffer_size];
    for i in 0..buffer_size {
        sample_output[i] = graph2.eval_node(&noise_sample_id);
    }

    // Should produce identical output
    for i in 0..buffer_size {
        assert_eq!(buffer_output[i], sample_output[i],
            "Buffer and sample-by-sample evaluation differ at sample {}: {} vs {}",
            i, buffer_output[i], sample_output[i]);
    }
}
