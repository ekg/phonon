/// Tests for BrownNoise buffer-based evaluation
///
/// These tests verify that BrownNoise buffer evaluation produces correct
/// characteristics: smooth random walk, -6dB/octave rolloff, state continuity, no drift.

use phonon::unified_graph::{BrownNoiseState, Signal, SignalNode, UnifiedSignalGraph};
use std::rc::Rc;

/// Helper: Create a test graph
fn create_test_graph() -> UnifiedSignalGraph {
    UnifiedSignalGraph::new(44100.0) // 44.1kHz
}

/// Helper: Calculate RMS of a buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Helper: Measure rate of change (smoothness metric)
/// Returns average absolute difference between consecutive samples
fn measure_smoothness(buffer: &[f32]) -> f32 {
    let mut total = 0.0;
    for i in 1..buffer.len() {
        total += (buffer[i] - buffer[i - 1]).abs();
    }
    total / buffer.len() as f32
}

/// Helper: Add a brown noise node
fn add_brownnoise_node(graph: &mut UnifiedSignalGraph) -> phonon::unified_graph::NodeId {
    let node = SignalNode::BrownNoise {
        state: BrownNoiseState::new(),
    };
    graph.add_node(node)
}

/// Helper: Add a white noise node
fn add_whitenoise_node(graph: &mut UnifiedSignalGraph) -> phonon::unified_graph::NodeId {
    let node = SignalNode::WhiteNoise;
    graph.add_node(node)
}

// ============================================================================
// TEST 1: Basic Output Properties
// ============================================================================

#[test]
fn test_brownnoise_generates_audio() {
    let mut graph = create_test_graph();

    let brown_id = add_brownnoise_node(&mut graph);

    // Generate one buffer
    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&brown_id, &mut output);

    // Check that we got non-zero audio
    let rms = calculate_rms(&output);
    assert!(
        rms > 0.01,
        "Brown noise should produce audio, got RMS: {}",
        rms
    );

    println!("Brown noise RMS: {}", rms);
}

#[test]
fn test_brownnoise_stays_in_bounds() {
    let mut graph = create_test_graph();

    let brown_id = add_brownnoise_node(&mut graph);

    // Generate multiple buffers to test stability
    for iteration in 0..10 {
        let mut output = vec![0.0; 512];
        graph.eval_node_buffer(&brown_id, &mut output);

        // Check all samples are in reasonable bounds
        for (i, &sample) in output.iter().enumerate() {
            assert!(
                sample.abs() <= 1.5,
                "Sample {} in iteration {} out of bounds: {}",
                i,
                iteration,
                sample
            );
        }
    }
}

// ============================================================================
// TEST 2: Smoothness (Brown noise should be much smoother than white)
// ============================================================================

#[test]
fn test_brownnoise_is_smooth() {
    let mut graph = create_test_graph();

    let brown_id = add_brownnoise_node(&mut graph);
    let white_id = add_whitenoise_node(&mut graph);

    let buffer_size = 512;
    let mut brown_out = vec![0.0; buffer_size];
    let mut white_out = vec![0.0; buffer_size];

    graph.eval_node_buffer(&brown_id, &mut brown_out);
    graph.eval_node_buffer(&white_id, &mut white_out);

    // Measure rate of change (smoothness)
    let brown_smoothness = measure_smoothness(&brown_out);
    let white_smoothness = measure_smoothness(&white_out);

    // Brown noise should be MUCH smoother (lower rate of change)
    assert!(
        brown_smoothness < white_smoothness * 0.5,
        "Brown noise should be smoother: brown={}, white={}",
        brown_smoothness,
        white_smoothness
    );

    println!(
        "Smoothness - Brown: {}, White: {}",
        brown_smoothness, white_smoothness
    );
}

// ============================================================================
// TEST 3: State Continuity
// ============================================================================

#[test]
fn test_brownnoise_state_persists() {
    let mut graph = create_test_graph();

    let brown_id = add_brownnoise_node(&mut graph);

    // Generate first buffer
    let buffer_size = 256;
    let mut buffer1 = vec![0.0; buffer_size];
    graph.eval_node_buffer(&brown_id, &mut buffer1);

    // Generate second buffer
    let mut buffer2 = vec![0.0; buffer_size];
    graph.eval_node_buffer(&brown_id, &mut buffer2);

    // The last sample of buffer1 and first sample of buffer2 should be close
    // (random walk is continuous, not huge jumps)
    let last_val = buffer1[buffer_size - 1];
    let first_val = buffer2[0];
    let jump = (first_val - last_val).abs();

    assert!(
        jump < 0.2,
        "Brown noise state should be continuous across buffers, jump: {}",
        jump
    );

    println!("Jump between buffers: {}", jump);
}

// ============================================================================
// TEST 4: No Long-Term Drift
// ============================================================================

#[test]
fn test_brownnoise_no_drift() {
    let mut graph = create_test_graph();

    let brown_id = add_brownnoise_node(&mut graph);

    // Generate many buffers and check we don't drift to infinity
    for iteration in 0..100 {
        let mut output = vec![0.0; 512];
        graph.eval_node_buffer(&brown_id, &mut output);

        // Check still in bounds (leaky integrator should prevent drift)
        for &sample in &output {
            assert!(
                sample.abs() <= 1.5,
                "Drift detected at iteration {}: sample={}",
                iteration,
                sample
            );
        }
    }
}

#[test]
fn test_brownnoise_mean_near_zero() {
    let mut graph = create_test_graph();

    let brown_id = add_brownnoise_node(&mut graph);

    // Generate large buffer
    let buffer_size = 8192;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&brown_id, &mut output);

    let mean: f32 = output.iter().sum::<f32>() / buffer_size as f32;

    assert!(
        mean.abs() < 0.15,
        "Brown noise mean should be near 0 (leaky integrator), got {}",
        mean
    );

    println!("Brown noise mean: {}", mean);
}

// ============================================================================
// TEST 5: Multiple Buffer Evaluation
// ============================================================================

#[test]
fn test_brownnoise_multiple_buffers() {
    let mut graph = create_test_graph();

    let brown_id = add_brownnoise_node(&mut graph);

    // Generate 10 consecutive buffers
    let buffer_size = 512;
    let num_buffers = 10;

    for i in 0..num_buffers {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&brown_id, &mut output);

        // Each buffer should have reasonable audio
        let rms = calculate_rms(&output);
        assert!(
            rms > 0.01 && rms < 2.0,
            "Buffer {} has unexpected RMS: {}",
            i,
            rms
        );
    }
}

// ============================================================================
// TEST 6: Spectral Properties (Brown noise has -6dB/octave rolloff)
// ============================================================================

#[test]
fn test_brownnoise_bass_emphasis() {
    use rustfft::{num_complex::Complex, FftPlanner};
    use std::f32::consts::PI;

    let mut graph = create_test_graph();
    let brown_id = add_brownnoise_node(&mut graph);

    // Generate longer buffer for FFT
    let buffer_size = 8192;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&brown_id, &mut output);

    // Perform FFT
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(buffer_size);

    let mut input: Vec<Complex<f32>> = output
        .iter()
        .enumerate()
        .map(|(i, &sample)| {
            // Apply Hann window
            let window = 0.5 * (1.0 - (2.0 * PI * i as f32 / buffer_size as f32).cos());
            Complex::new(sample * window, 0.0)
        })
        .collect();

    fft.process(&mut input);

    // Calculate energy in different frequency bands
    let sample_rate = 44100.0;
    let bin_width = sample_rate / buffer_size as f32;

    let low_energy: f32 = (0..buffer_size / 2)
        .filter(|&i| {
            let freq = i as f32 * bin_width;
            freq > 50.0 && freq < 200.0
        })
        .map(|i| {
            let mag = (input[i].re * input[i].re + input[i].im * input[i].im).sqrt();
            mag * mag
        })
        .sum();

    let mid_energy: f32 = (0..buffer_size / 2)
        .filter(|&i| {
            let freq = i as f32 * bin_width;
            freq > 800.0 && freq < 1600.0
        })
        .map(|i| {
            let mag = (input[i].re * input[i].re + input[i].im * input[i].im).sqrt();
            mag * mag
        })
        .sum();

    let high_energy: f32 = (0..buffer_size / 2)
        .filter(|&i| {
            let freq = i as f32 * bin_width;
            freq > 6400.0 && freq < 12800.0
        })
        .map(|i| {
            let mag = (input[i].re * input[i].re + input[i].im * input[i].im).sqrt();
            mag * mag
        })
        .sum();

    // Brown noise should have strong low-frequency dominance
    assert!(
        low_energy > mid_energy * 1.5,
        "Brown noise should have strong low-frequency energy. Low: {}, Mid: {}",
        low_energy,
        mid_energy
    );

    assert!(
        mid_energy > high_energy,
        "Brown noise mid should have more energy than high. Mid: {}, High: {}",
        mid_energy,
        high_energy
    );

    println!(
        "Energy - Low: {}, Mid: {}, High: {}",
        low_energy, mid_energy, high_energy
    );
}

// ============================================================================
// TEST 7: Comparison with WhiteNoise and PinkNoise
// ============================================================================

#[test]
fn test_brownnoise_smoother_than_white_and_pink() {
    let mut graph = create_test_graph();

    let brown_id = add_brownnoise_node(&mut graph);
    let white_id = add_whitenoise_node(&mut graph);

    // We can't test pink noise comparison without a pink noise node
    // but we can compare brown vs white

    let buffer_size = 1024;
    let mut brown_out = vec![0.0; buffer_size];
    let mut white_out = vec![0.0; buffer_size];

    graph.eval_node_buffer(&brown_id, &mut brown_out);
    graph.eval_node_buffer(&white_id, &mut white_out);

    let brown_smooth = measure_smoothness(&brown_out);
    let white_smooth = measure_smoothness(&white_out);

    // Brown should be much smoother
    assert!(
        brown_smooth < white_smooth * 0.5,
        "Brown should be smoother than white: brown={}, white={}",
        brown_smooth,
        white_smooth
    );

    println!(
        "Smoothness comparison - Brown: {}, White: {}",
        brown_smooth, white_smooth
    );
}

// ============================================================================
// TEST 8: Edge Cases
// ============================================================================

#[test]
fn test_brownnoise_small_buffer() {
    let mut graph = create_test_graph();

    let brown_id = add_brownnoise_node(&mut graph);

    // Very small buffer
    let mut output = vec![0.0; 8];
    graph.eval_node_buffer(&brown_id, &mut output);

    // Should still produce reasonable output
    let rms = calculate_rms(&output);
    assert!(rms > 0.0, "Small buffer should still produce audio");
}

#[test]
fn test_brownnoise_large_buffer() {
    let mut graph = create_test_graph();

    let brown_id = add_brownnoise_node(&mut graph);

    // Large buffer
    let buffer_size = 16384;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&brown_id, &mut output);

    // Should not drift despite long buffer
    for (i, &sample) in output.iter().enumerate() {
        assert!(
            sample.abs() <= 1.5,
            "Sample {} in large buffer out of bounds: {}",
            i,
            sample
        );
    }

    let rms = calculate_rms(&output);
    println!("Large buffer RMS: {}", rms);
}

// ============================================================================
// TEST 9: Determinism/Randomness
// ============================================================================

#[test]
fn test_brownnoise_is_random() {
    let mut graph1 = create_test_graph();
    let mut graph2 = create_test_graph();

    let brown_id1 = add_brownnoise_node(&mut graph1);
    let brown_id2 = add_brownnoise_node(&mut graph2);

    let buffer_size = 512;
    let mut output1 = vec![0.0; buffer_size];
    let mut output2 = vec![0.0; buffer_size];

    graph1.eval_node_buffer(&brown_id1, &mut output1);
    graph2.eval_node_buffer(&brown_id2, &mut output2);

    // Different graph instances should produce different noise
    let mut differences = 0;
    for i in 0..buffer_size {
        if (output1[i] - output2[i]).abs() > 0.01 {
            differences += 1;
        }
    }

    let diff_ratio = differences as f32 / buffer_size as f32;
    assert!(
        diff_ratio > 0.8,
        "Brown noise should be random (different each time), similarity: {}",
        1.0 - diff_ratio
    );

    println!("Difference ratio: {}", diff_ratio);
}

// ============================================================================
// TEST 10: Performance
// ============================================================================

#[test]
fn test_brownnoise_buffer_performance() {
    let mut graph = create_test_graph();

    let brown_id = add_brownnoise_node(&mut graph);

    let buffer_size = 512;
    let iterations = 1000;

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&brown_id, &mut output);
    }
    let duration = start.elapsed();

    println!(
        "Brown noise buffer eval: {:?} for {} iterations",
        duration, iterations
    );
    println!("Per iteration: {:?}", duration / iterations);

    // Should complete in reasonable time (< 2 seconds for 1000 iterations)
    assert!(
        duration.as_secs() < 2,
        "Brown noise buffer evaluation too slow: {:?}",
        duration
    );
}
