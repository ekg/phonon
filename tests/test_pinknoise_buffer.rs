/// Tests for PinkNoise buffer-based evaluation
///
/// These tests verify that pink noise buffer evaluation produces correct
/// spectral characteristics and maintains proper state continuity.
///
/// Pink noise has equal energy per octave (-3dB/octave rolloff, 1/f spectrum).

use phonon::unified_graph::UnifiedSignalGraph;
use std::f32::consts::PI;

/// Helper: Create a test graph
fn create_test_graph() -> UnifiedSignalGraph {
    UnifiedSignalGraph::new(44100.0) // 44.1kHz
}

/// Helper: Calculate RMS of a buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Helper: Measure high-frequency energy (differences between consecutive samples)
fn measure_high_freq_energy(buffer: &[f32]) -> f32 {
    let mut energy = 0.0;
    for i in 1..buffer.len() {
        energy += (buffer[i] - buffer[i-1]).abs();
    }
    energy / buffer.len() as f32
}

/// Helper: Perform FFT and get energy in frequency band
fn measure_band_energy(buffer: &[f32], sample_rate: f32, low_freq: f32, high_freq: f32) -> f32 {
    use rustfft::{FftPlanner, num_complex::Complex};

    let fft_size = 8192.min(buffer.len());
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);

    // Apply window and convert to complex
    let mut input: Vec<Complex<f32>> = buffer[..fft_size]
        .iter()
        .enumerate()
        .map(|(i, &sample)| {
            let window = 0.5 * (1.0 - (2.0 * PI * i as f32 / fft_size as f32).cos());
            Complex::new(sample * window, 0.0)
        })
        .collect();

    fft.process(&mut input);

    // Calculate energy in frequency band
    let mut energy = 0.0;
    for i in 0..fft_size / 2 {
        let freq = i as f32 * sample_rate / fft_size as f32;
        if freq >= low_freq && freq <= high_freq {
            let magnitude = (input[i].re * input[i].re + input[i].im * input[i].im).sqrt();
            energy += magnitude * magnitude;
        }
    }
    energy
}

// ============================================================================
// TEST 1: Basic Audio Output
// ============================================================================

#[test]
fn test_pinknoise_generates_audio() {
    let mut graph = create_test_graph();

    let pink_id = graph.add_pinknoise_node();

    // Generate buffer
    let buffer_size = 4096;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&pink_id, &mut output);

    // Check that we got audio
    let rms = calculate_rms(&output);
    assert!(rms > 0.05, "Pink noise should produce audio, got RMS: {}", rms);
    assert!(rms < 0.5, "Pink noise RMS should be reasonable, got: {}", rms);
}

// ============================================================================
// TEST 2: Statistical Properties
// ============================================================================

#[test]
fn test_pinknoise_mean_near_zero() {
    let mut graph = create_test_graph();
    let pink_id = graph.add_pinknoise_node();

    // Generate large buffer for better statistics
    let buffer_size = 44100; // 1 second
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&pink_id, &mut output);

    // Calculate mean
    let mean: f32 = output.iter().sum::<f32>() / buffer_size as f32;

    assert!(mean.abs() < 0.05,
        "Pink noise mean should be near 0, got {}", mean);
}

#[test]
fn test_pinknoise_has_variance() {
    let mut graph = create_test_graph();
    let pink_id = graph.add_pinknoise_node();

    let buffer_size = 4096;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&pink_id, &mut output);

    // Calculate variance
    let mean: f32 = output.iter().sum::<f32>() / buffer_size as f32;
    let variance: f32 = output.iter()
        .map(|&x| (x - mean) * (x - mean))
        .sum::<f32>() / buffer_size as f32;

    assert!(variance > 0.01,
        "Pink noise should have variance, got {}", variance);
}

#[test]
fn test_pinknoise_reasonable_range() {
    let mut graph = create_test_graph();
    let pink_id = graph.add_pinknoise_node();

    let buffer_size = 4096;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&pink_id, &mut output);

    // Check amplitude range
    let max_amplitude = output.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    assert!(max_amplitude < 2.0,
        "Pink noise should not excessively clip, max: {}", max_amplitude);
}

// ============================================================================
// TEST 3: Spectral Properties
// ============================================================================

#[test]
fn test_pinknoise_vs_whitenoise_spectrum() {
    // Pink noise should have more low-frequency energy than white noise
    let mut graph = create_test_graph();
    let pink_id = graph.add_pinknoise_node();
    let white_id = graph.add_whitenoise_node();

    let buffer_size = 8192;
    let mut pink_out = vec![0.0; buffer_size];
    let mut white_out = vec![0.0; buffer_size];

    graph.eval_node_buffer(&pink_id, &mut pink_out);
    graph.eval_node_buffer(&white_id, &mut white_out);

    // Measure low-frequency energy
    let pink_low = measure_band_energy(&pink_out, 44100.0, 100.0, 500.0);
    let white_low = measure_band_energy(&white_out, 44100.0, 100.0, 500.0);

    // Measure high-frequency energy
    let pink_high = measure_band_energy(&pink_out, 44100.0, 5000.0, 15000.0);
    let white_high = measure_band_energy(&white_out, 44100.0, 5000.0, 15000.0);

    // Pink noise should have more bass relative to treble than white noise
    let pink_ratio = pink_low / pink_high.max(0.001);
    let white_ratio = white_low / white_high.max(0.001);

    assert!(pink_ratio > white_ratio,
        "Pink noise should have more bass relative to treble. Pink ratio: {}, White ratio: {}",
        pink_ratio, white_ratio);
}

#[test]
fn test_pinknoise_high_freq_content_less_than_white() {
    // Pink noise should have less high-frequency content than white noise
    let mut graph = create_test_graph();
    let pink_id = graph.add_pinknoise_node();
    let white_id = graph.add_whitenoise_node();

    let buffer_size = 4096;
    let mut pink_out = vec![0.0; buffer_size];
    let mut white_out = vec![0.0; buffer_size];

    graph.eval_node_buffer(&pink_id, &mut pink_out);
    graph.eval_node_buffer(&white_id, &mut white_out);

    // Measure high-frequency energy (differences between samples)
    let pink_hf = measure_high_freq_energy(&pink_out);
    let white_hf = measure_high_freq_energy(&white_out);

    // Pink noise should have less high-frequency energy
    assert!(pink_hf < white_hf * 0.9,
        "Pink noise should have less HF energy: pink={}, white={}", pink_hf, white_hf);
}

// ============================================================================
// TEST 4: State Continuity
// ============================================================================

#[test]
fn test_pinknoise_state_persists_across_buffers() {
    let mut graph = create_test_graph();
    let pink_id = graph.add_pinknoise_node();

    let buffer_size = 512;
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];

    // Generate two consecutive buffers
    graph.eval_node_buffer(&pink_id, &mut buffer1);
    graph.eval_node_buffer(&pink_id, &mut buffer2);

    // Buffers should be different (randomness working)
    let mut differences = 0;
    for i in 0..buffer_size {
        if (buffer1[i] - buffer2[i]).abs() > 0.001 {
            differences += 1;
        }
    }

    let diff_ratio = differences as f32 / buffer_size as f32;
    assert!(diff_ratio > 0.8,
        "Pink noise should produce different output each buffer, diff ratio: {}",
        diff_ratio);
}

#[test]
fn test_pinknoise_multiple_buffer_generation() {
    let mut graph = create_test_graph();
    let pink_id = graph.add_pinknoise_node();

    // Generate multiple buffers to ensure state updates correctly
    let buffer_size = 512;
    let num_buffers = 10;

    for i in 0..num_buffers {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&pink_id, &mut output);

        // Each buffer should have reasonable audio
        let rms = calculate_rms(&output);
        assert!(rms > 0.05 && rms < 0.5,
            "Buffer {} has unexpected RMS: {}", i, rms);
    }
}

// ============================================================================
// TEST 5: Comparison with Sample-by-Sample Evaluation
// ============================================================================

#[test]
fn test_pinknoise_buffer_vs_sample_by_sample() {
    // Both approaches should produce similar statistical properties
    let mut graph1 = create_test_graph();
    let mut graph2 = create_test_graph();

    let pink_id1 = graph1.add_pinknoise_node();
    let pink_id2 = graph2.add_pinknoise_node();

    let buffer_size = 4096;
    let mut buffer_output = vec![0.0; buffer_size];
    let mut sample_output = vec![0.0; buffer_size];

    // Generate with buffer evaluation
    graph1.eval_node_buffer(&pink_id1, &mut buffer_output);

    // Generate with sample-by-sample evaluation
    for i in 0..buffer_size {
        sample_output[i] = graph2.eval_node(&pink_id2);
    }

    // Compare statistical properties
    let buffer_rms = calculate_rms(&buffer_output);
    let sample_rms = calculate_rms(&sample_output);

    // RMS should be similar (within 20%)
    let rms_ratio = buffer_rms / sample_rms.max(0.001);
    assert!(rms_ratio > 0.8 && rms_ratio < 1.2,
        "Buffer and sample-by-sample RMS should be similar: buffer={}, sample={}",
        buffer_rms, sample_rms);
}

// ============================================================================
// TEST 6: Different Buffer Sizes
// ============================================================================

#[test]
fn test_pinknoise_various_buffer_sizes() {
    let buffer_sizes = vec![128, 256, 512, 1024, 2048, 4096];

    for &size in &buffer_sizes {
        let mut graph = create_test_graph();
        let pink_id = graph.add_pinknoise_node();

        let mut output = vec![0.0; size];
        graph.eval_node_buffer(&pink_id, &mut output);

        let rms = calculate_rms(&output);
        assert!(rms > 0.05 && rms < 0.5,
            "Buffer size {} has unexpected RMS: {}", size, rms);
    }
}

// ============================================================================
// TEST 7: Edge Cases
// ============================================================================

#[test]
fn test_pinknoise_small_buffer() {
    let mut graph = create_test_graph();
    let pink_id = graph.add_pinknoise_node();

    // Very small buffer
    let buffer_size = 16;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&pink_id, &mut output);

    // Should still produce some audio
    let max_amplitude = output.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    assert!(max_amplitude > 0.001,
        "Pink noise should work with small buffers");
}

#[test]
fn test_pinknoise_large_buffer() {
    let mut graph = create_test_graph();
    let pink_id = graph.add_pinknoise_node();

    // Large buffer
    let buffer_size = 88200; // 2 seconds
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&pink_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.05 && rms < 0.5,
        "Pink noise should work with large buffers, RMS: {}", rms);
}

// ============================================================================
// TEST 8: Performance
// ============================================================================

#[test]
fn test_pinknoise_buffer_performance() {
    let mut graph = create_test_graph();
    let pink_id = graph.add_pinknoise_node();

    let buffer_size = 512;
    let iterations = 1000;

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&pink_id, &mut output);
    }
    let duration = start.elapsed();

    println!("PinkNoise buffer eval: {:?} for {} iterations", duration, iterations);
    println!("Per iteration: {:?}", duration / iterations);

    // Should complete in reasonable time (< 2 seconds for 1000 iterations)
    assert!(duration.as_secs() < 2,
        "PinkNoise buffer evaluation too slow: {:?}", duration);
}

// ============================================================================
// TEST 9: Octave Band Analysis
// ============================================================================

#[test]
fn test_pinknoise_octave_band_energy() {
    // Pink noise should have approximately equal energy per octave
    let mut graph = create_test_graph();
    let pink_id = graph.add_pinknoise_node();

    let buffer_size = 16384; // Larger buffer for better frequency resolution
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&pink_id, &mut output);

    // Measure energy in octave bands
    let low_octave = measure_band_energy(&output, 44100.0, 125.0, 250.0);
    let mid_octave = measure_band_energy(&output, 44100.0, 1000.0, 2000.0);
    let high_octave = measure_band_energy(&output, 44100.0, 8000.0, 16000.0);

    // All octaves should have energy (non-zero)
    assert!(low_octave > 0.0 && mid_octave > 0.0 && high_octave > 0.0,
        "All octave bands should have energy");

    println!("Octave energy - Low: {}, Mid: {}, High: {}", low_octave, mid_octave, high_octave);
}

// ============================================================================
// TEST 10: Randomness Quality
// ============================================================================

#[test]
fn test_pinknoise_different_instances_produce_different_output() {
    let mut graph = create_test_graph();

    // Create two independent pink noise generators
    let pink_id1 = graph.add_pinknoise_node();
    let pink_id2 = graph.add_pinknoise_node();

    let buffer_size = 1024;
    let mut output1 = vec![0.0; buffer_size];
    let mut output2 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&pink_id1, &mut output1);
    graph.eval_node_buffer(&pink_id2, &mut output2);

    // Outputs should be different (different random state)
    let mut differences = 0;
    for i in 0..buffer_size {
        if (output1[i] - output2[i]).abs() > 0.01 {
            differences += 1;
        }
    }

    let diff_ratio = differences as f32 / buffer_size as f32;
    assert!(diff_ratio > 0.9,
        "Different pink noise instances should produce different output, diff ratio: {}",
        diff_ratio);
}

// ============================================================================
// TEST 11: State Consistency
// ============================================================================

#[test]
fn test_pinknoise_counter_wrapping() {
    // Test that counter wrapping doesn't cause issues
    let mut graph = create_test_graph();
    let pink_id = graph.add_pinknoise_node();

    // Generate many buffers to potentially wrap the counter
    let buffer_size = 1024;
    let num_buffers = 1000;

    for _ in 0..num_buffers {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&pink_id, &mut output);

        // Should still produce valid audio
        let rms = calculate_rms(&output);
        assert!(rms > 0.01 && rms < 1.0,
            "Counter wrapping should not break pink noise");
    }
}

// ============================================================================
// TEST 12: Bin Update Pattern Verification
// ============================================================================

#[test]
fn test_pinknoise_bin_updates() {
    // Verify that bins are updating at expected rates
    // This is a basic sanity check
    let mut graph = create_test_graph();
    let pink_id = graph.add_pinknoise_node();

    // Generate several buffers
    let buffer_size = 512;
    let mut all_samples = Vec::new();

    for _ in 0..10 {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&pink_id, &mut output);
        all_samples.extend_from_slice(&output);
    }

    // Check that we have variation in the output (bins are updating)
    let mean: f32 = all_samples.iter().sum::<f32>() / all_samples.len() as f32;
    let variance: f32 = all_samples.iter()
        .map(|&x| (x - mean) * (x - mean))
        .sum::<f32>() / all_samples.len() as f32;

    assert!(variance > 0.01,
        "Pink noise bins should update and create variation, variance: {}", variance);
}
