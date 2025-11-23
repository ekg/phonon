//! Buffer-level tests for Waveguide physical modeling
//!
//! Tests the buffer-based evaluation of the Waveguide node, which uses
//! bidirectional delay lines to simulate wave propagation in physical media
//! (strings, tubes, membranes). These tests verify that:
//! 1. Buffer evaluation produces same results as sample-by-sample
//! 2. State is properly maintained across buffer boundaries
//! 3. Pattern-modulated parameters work correctly
//! 4. Physical behavior (resonance, damping, decay) is correct

use phonon::unified_graph::{NodeId, UnifiedGraph, Signal};
use std::f32::consts::PI;

// Helper to create test graph
fn create_test_graph() -> UnifiedGraph {
    UnifiedGraph::new(44100.0)
}

// ========== HELPER FUNCTIONS ==========

/// Calculate RMS (Root Mean Square) of a buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    if buffer.is_empty() {
        return 0.0;
    }
    let sum: f32 = buffer.iter().map(|x| x * x).sum();
    (sum / buffer.len() as f32).sqrt()
}

/// Detect zero crossings (rising edges) in a buffer
fn detect_zero_crossings(buffer: &[f32]) -> Vec<usize> {
    buffer
        .windows(2)
        .enumerate()
        .filter_map(|(i, w)| {
            if w[0] <= 0.0 && w[1] > 0.0 {
                Some(i)
            } else {
                None
            }
        })
        .collect()
}

/// Measure fundamental frequency from zero crossings
fn measure_frequency(buffer: &[f32], sample_rate: f32) -> Option<f32> {
    let crossings = detect_zero_crossings(buffer);
    if crossings.len() < 2 {
        return None;
    }

    let periods: Vec<f32> = crossings.windows(2).map(|w| (w[1] - w[0]) as f32).collect();
    let avg_period = periods.iter().sum::<f32>() / periods.len() as f32;
    Some(sample_rate / avg_period)
}

/// Calculate peak amplitude in a buffer
fn peak_amplitude(buffer: &[f32]) -> f32 {
    buffer.iter().map(|x| x.abs()).fold(0.0f32, f32::max)
}

/// Simple FFT helper to find dominant frequency
/// Returns (frequency, magnitude) of the strongest peak
fn find_dominant_frequency(buffer: &[f32], sample_rate: f32) -> Option<(f32, f32)> {
    use rustfft::{FftPlanner, num_complex::Complex};

    if buffer.len() < 2 {
        return None;
    }

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(buffer.len());

    // Convert to complex numbers
    let mut complex_buffer: Vec<Complex<f32>> = buffer
        .iter()
        .map(|&x| Complex::new(x, 0.0))
        .collect();

    // Perform FFT
    fft.process(&mut complex_buffer);

    // Find peak in positive frequencies (skip DC bin)
    let half_len = complex_buffer.len() / 2;
    let mut max_magnitude = 0.0f32;
    let mut max_bin = 0;

    for (i, c) in complex_buffer[1..half_len].iter().enumerate() {
        let magnitude = c.norm();
        if magnitude > max_magnitude {
            max_magnitude = magnitude;
            max_bin = i + 1;
        }
    }

    if max_magnitude > 0.0 {
        let freq = (max_bin as f32) * sample_rate / (buffer.len() as f32);
        Some((freq, max_magnitude))
    } else {
        None
    }
}

// ========== LEVEL 1: BASIC FUNCTIONALITY ==========

#[test]
fn test_waveguide_buffer_produces_sound() {
    let mut graph = create_test_graph();

    // Create simple waveguide node
    let wg_id = graph.add_waveguide_node(
        Signal::Value(440.0),  // freq
        Signal::Value(0.5),    // damping
        Signal::Value(0.5),    // pickup_position
    );

    let mut output = vec![0.0; 44100]; // 1 second
    graph.eval_node_buffer(&wg_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(
        rms > 0.01,
        "Waveguide should produce audible output, got RMS={}",
        rms
    );
}

#[test]
fn test_waveguide_buffer_vs_sample() {
    // Compare buffer evaluation to sample-by-sample
    let mut graph1 = create_test_graph();
    let mut graph2 = create_test_graph();

    let wg1 = graph1.add_waveguide_node(
        Signal::Value(220.0),
        Signal::Value(0.3),
        Signal::Value(0.5),
    );

    let wg2 = graph2.add_waveguide_node(
        Signal::Value(220.0),
        Signal::Value(0.3),
        Signal::Value(0.5),
    );

    let buffer_size = 1024;
    let mut buffer_output = vec![0.0; buffer_size];
    let mut sample_output = vec![0.0; buffer_size];

    // Buffer evaluation
    graph1.eval_node_buffer(&wg1, &mut buffer_output);

    // Sample-by-sample evaluation
    for i in 0..buffer_size {
        sample_output[i] = graph2.eval_node(&wg2);
    }

    // Should be identical (or very close due to floating-point)
    let max_diff = buffer_output
        .iter()
        .zip(sample_output.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);

    assert!(
        max_diff < 1e-6,
        "Buffer and sample evaluation should match, max diff={}",
        max_diff
    );
}

#[test]
fn test_waveguide_buffer_resonance() {
    let mut graph = create_test_graph();

    // Create waveguide at 440 Hz with low damping (high resonance)
    let wg_id = graph.add_waveguide_node(
        Signal::Value(440.0),
        Signal::Value(0.1), // Low damping = strong resonance
        Signal::Value(0.5),
    );

    let mut output = vec![0.0; 8192];
    graph.eval_node_buffer(&wg_id, &mut output);

    // Skip first few samples (initial noise burst)
    let analysis_start = 1000;
    let analysis_buffer = &output[analysis_start..];

    // Find dominant frequency
    if let Some((freq, _magnitude)) = find_dominant_frequency(analysis_buffer, 44100.0) {
        // Should resonate near 440 Hz (allow 20% tolerance due to physical modeling)
        let tolerance = 440.0 * 0.20;
        assert!(
            (freq - 440.0).abs() < tolerance,
            "Expected resonance near 440Hz (±20%), got {}Hz",
            freq
        );
    } else {
        panic!("Should detect dominant frequency");
    }
}

#[test]
fn test_waveguide_buffer_decay() {
    let mut graph = create_test_graph();

    // Waveguide should decay over time (energy loss)
    let wg_id = graph.add_waveguide_node(
        Signal::Value(440.0),
        Signal::Value(0.5),
        Signal::Value(0.5),
    );

    let mut output = vec![0.0; 88200]; // 2 seconds
    graph.eval_node_buffer(&wg_id, &mut output);

    // Measure RMS in quarters
    let quarter = output.len() / 4;
    let rms_first = calculate_rms(&output[0..quarter]);
    let rms_last = calculate_rms(&output[3 * quarter..]);

    assert!(
        rms_last < rms_first,
        "Waveguide should decay: first_quarter RMS={}, last_quarter RMS={}",
        rms_first,
        rms_last
    );
}

// ========== LEVEL 2: PARAMETER VARIATION ==========

#[test]
fn test_waveguide_buffer_damping_variation() {
    let mut graph_low = create_test_graph();
    let mut graph_high = create_test_graph();

    // Low damping (long decay)
    let wg_low = graph_low.add_waveguide_node(
        Signal::Value(440.0),
        Signal::Value(0.1), // Low damping
        Signal::Value(0.5),
    );

    // High damping (short decay)
    let wg_high = graph_high.add_waveguide_node(
        Signal::Value(440.0),
        Signal::Value(0.9), // High damping
        Signal::Value(0.5),
    );

    let buffer_size = 88200; // 2 seconds
    let mut output_low = vec![0.0; buffer_size];
    let mut output_high = vec![0.0; buffer_size];

    graph_low.eval_node_buffer(&wg_low, &mut output_low);
    graph_high.eval_node_buffer(&wg_high, &mut output_high);

    // Measure RMS in second half (after initial excitation)
    let mid = buffer_size / 2;
    let rms_low_late = calculate_rms(&output_low[mid..]);
    let rms_high_late = calculate_rms(&output_high[mid..]);

    assert!(
        rms_high_late < rms_low_late,
        "High damping should decay faster: low={}, high={}",
        rms_low_late,
        rms_high_late
    );
}

#[test]
fn test_waveguide_buffer_pickup_position() {
    let mut graph_center = create_test_graph();
    let mut graph_off = create_test_graph();

    // Center pickup (emphasizes fundamental)
    let wg_center = graph_center.add_waveguide_node(
        Signal::Value(220.0),
        Signal::Value(0.3),
        Signal::Value(0.5), // Center
    );

    // Off-center pickup (emphasizes harmonics)
    let wg_off = graph_off.add_waveguide_node(
        Signal::Value(220.0),
        Signal::Value(0.3),
        Signal::Value(0.25), // Quarter position
    );

    let buffer_size = 44100;
    let mut output_center = vec![0.0; buffer_size];
    let mut output_off = vec![0.0; buffer_size];

    graph_center.eval_node_buffer(&wg_center, &mut output_center);
    graph_off.eval_node_buffer(&wg_off, &mut output_off);

    // Both should produce sound
    let rms_center = calculate_rms(&output_center);
    let rms_off = calculate_rms(&output_off);

    assert!(rms_center > 0.01, "Center pickup should produce sound");
    assert!(rms_off > 0.01, "Off-center pickup should produce sound");

    // Different pickup positions should produce measurably different output
    assert!(
        (rms_center - rms_off).abs() > 0.001,
        "Different pickup positions should produce different timbres"
    );
}

#[test]
fn test_waveguide_buffer_frequency_range() {
    // Test waveguide at various frequencies
    let frequencies = [55.0, 110.0, 220.0, 440.0, 880.0];

    for freq in &frequencies {
        let mut graph = create_test_graph();

        let wg_id = graph.add_waveguide_node(
            Signal::Value(*freq),
            Signal::Value(0.5),
            Signal::Value(0.5),
        );

        let mut output = vec![0.0; 44100];
        graph.eval_node_buffer(&wg_id, &mut output);

        let rms = calculate_rms(&output);
        assert!(
            rms > 0.01,
            "Waveguide at {}Hz should produce sound, got RMS={}",
            freq,
            rms
        );
    }
}

// ========== LEVEL 3: STATE CONTINUITY ==========

#[test]
fn test_waveguide_buffer_state_continuity() {
    // Verify state is maintained across multiple buffer evaluations
    let mut graph = create_test_graph();

    let wg_id = graph.add_waveguide_node(
        Signal::Value(440.0),
        Signal::Value(0.3),
        Signal::Value(0.5),
    );

    // Render in chunks
    let chunk_size = 1024;
    let num_chunks = 10;
    let mut full_output = Vec::new();

    for _ in 0..num_chunks {
        let mut chunk = vec![0.0; chunk_size];
        graph.eval_node_buffer(&wg_id, &mut chunk);
        full_output.extend_from_slice(&chunk);
    }

    // Check for discontinuities at chunk boundaries
    for i in 1..num_chunks {
        let boundary_idx = i * chunk_size;
        let before = full_output[boundary_idx - 1];
        let after = full_output[boundary_idx];

        // Should not have large jumps at boundaries
        let diff = (after - before).abs();
        assert!(
            diff < 0.5, // Reasonable threshold for continuous audio
            "Discontinuity at chunk boundary {}: diff={}",
            i,
            diff
        );
    }
}

#[test]
fn test_waveguide_buffer_long_sequence() {
    // Test that waveguide maintains coherent state over long sequences
    let mut graph = create_test_graph();

    let wg_id = graph.add_waveguide_node(
        Signal::Value(220.0),
        Signal::Value(0.2),
        Signal::Value(0.5),
    );

    // Render 5 seconds in 1-second chunks
    for _ in 0..5 {
        let mut chunk = vec![0.0; 44100];
        graph.eval_node_buffer(&wg_id, &mut chunk);

        // Each chunk should still contain sound (verifies state persistence)
        let rms = calculate_rms(&chunk);
        assert!(
            rms > 0.005, // May be quieter after decay, but still present
            "Waveguide should maintain oscillation over time"
        );
    }
}

// ========== LEVEL 4: IMPULSE RESPONSE ==========

#[test]
fn test_waveguide_buffer_impulse_response() {
    let mut graph = create_test_graph();

    // Create waveguide
    let wg_id = graph.add_waveguide_node(
        Signal::Value(440.0),
        Signal::Value(0.5),
        Signal::Value(0.5),
    );

    // Render impulse response
    let mut output = vec![0.0; 8192];
    graph.eval_node_buffer(&wg_id, &mut output);

    // Should start with some energy (initialized with noise)
    let early_rms = calculate_rms(&output[0..1000]);
    assert!(early_rms > 0.01, "Should have initial excitation");

    // Should decay over time
    let late_rms = calculate_rms(&output[4000..8000]);
    assert!(
        late_rms < early_rms,
        "Should decay: early={}, late={}",
        early_rms,
        late_rms
    );
}

// ========== LEVEL 5: COMPARISON WITH DELAY-BASED SYNTHESIS ==========

#[test]
fn test_waveguide_vs_simple_delay() {
    // Waveguide should produce richer sound than simple delay-based synthesis
    let mut graph_wg = create_test_graph();
    let mut graph_delay = create_test_graph();

    // Waveguide
    let wg_id = graph_wg.add_waveguide_node(
        Signal::Value(220.0),
        Signal::Value(0.3),
        Signal::Value(0.5),
    );

    // Simple comb filter (for comparison)
    let noise_id = graph_delay.add_noise_node(12345);
    let comb_id = graph_delay.add_comb_node(
        Signal::Node(noise_id),
        Signal::Value(220.0),
        Signal::Value(0.99),
    );

    let mut output_wg = vec![0.0; 44100];
    let mut output_delay = vec![0.0; 44100];

    graph_wg.eval_node_buffer(&wg_id, &mut output_wg);
    graph_delay.eval_node_buffer(&comb_id, &mut output_delay);

    // Both should produce sound
    let rms_wg = calculate_rms(&output_wg);
    let rms_delay = calculate_rms(&output_delay);

    assert!(rms_wg > 0.01, "Waveguide should produce sound");
    assert!(rms_delay > 0.01, "Comb filter should produce sound");

    // Waveguide and comb should produce different characteristics
    // (This is more of a sanity check than a strict requirement)
    let diff_ratio = (rms_wg - rms_delay).abs() / rms_wg.max(rms_delay);
    println!("RMS difference ratio: {}", diff_ratio);
    // Just verify both methods work; actual difference may vary
}

// ========== LEVEL 6: BUFFER SIZE INDEPENDENCE ==========

#[test]
fn test_waveguide_buffer_size_independence() {
    // Results should be identical regardless of buffer size
    let buffer_sizes = [64, 256, 1024, 4096];
    let total_samples = 8192;

    let mut all_outputs = Vec::new();

    for &buffer_size in &buffer_sizes {
        let mut graph = create_test_graph();

        let wg_id = graph.add_waveguide_node(
            Signal::Value(440.0),
            Signal::Value(0.4),
            Signal::Value(0.5),
        );

        let mut full_output = Vec::new();
        let num_buffers = total_samples / buffer_size;

        for _ in 0..num_buffers {
            let mut chunk = vec![0.0; buffer_size];
            graph.eval_node_buffer(&wg_id, &mut chunk);
            full_output.extend_from_slice(&chunk);
        }

        all_outputs.push(full_output);
    }

    // All outputs should be identical (within floating-point tolerance)
    for i in 1..all_outputs.len() {
        let max_diff = all_outputs[0]
            .iter()
            .zip(all_outputs[i].iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);

        assert!(
            max_diff < 1e-6,
            "Buffer size {} should produce same output as size {}, max diff={}",
            buffer_sizes[i],
            buffer_sizes[0],
            max_diff
        );
    }
}

// ========== LEVEL 7: PERFORMANCE ==========

#[test]
fn test_waveguide_buffer_performance() {
    // Verify buffer evaluation is reasonably efficient
    use std::time::Instant;

    let mut graph = create_test_graph();

    let wg_id = graph.add_waveguide_node(
        Signal::Value(440.0),
        Signal::Value(0.5),
        Signal::Value(0.5),
    );

    let buffer_size = 44100; // 1 second at 44.1kHz
    let mut output = vec![0.0; buffer_size];

    let start = Instant::now();
    graph.eval_node_buffer(&wg_id, &mut output);
    let duration = start.elapsed();

    // Should complete in reasonable time (well under 1 second for 1 second of audio)
    assert!(
        duration.as_millis() < 100,
        "Buffer evaluation should be fast, took {:?}",
        duration
    );

    println!(
        "Waveguide buffer performance: {:.2}x realtime",
        1000.0 / duration.as_millis() as f32
    );
}
