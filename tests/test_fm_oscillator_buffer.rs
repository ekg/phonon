/// Buffer-level tests for FM Oscillator
///
/// These tests verify that FM oscillator buffer evaluation produces correct
/// frequency modulation synthesis at the UnifiedSignalGraph buffer API level.
///
/// Complements test_fm_oscillator.rs which tests at the DSL/compiler level.

use phonon::unified_graph::{Signal, UnifiedSignalGraph};
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

/// Helper: Count zero crossings
fn count_zero_crossings(buffer: &[f32]) -> usize {
    let mut count = 0;
    for i in 1..buffer.len() {
        if (buffer[i-1] < 0.0 && buffer[i] >= 0.0) || (buffer[i-1] >= 0.0 && buffer[i] < 0.0) {
            count += 1;
        }
    }
    count
}

// ============================================================================
// TEST: Basic Buffer Evaluation
// ============================================================================

#[test]
fn test_fm_buffer_basic_tone() {
    let mut graph = create_test_graph();

    let fm_id = graph.add_fmoscillator_node(
        Signal::Value(440.0),  // Carrier: A4
        Signal::Value(220.0),  // Modulator: A3
        Signal::Value(2.0),    // Mod index
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&fm_id, &mut output);

    // Should produce audible signal
    let rms = calculate_rms(&output);
    assert!(rms > 0.3, "FM should produce significant energy, got RMS {}", rms);

    // Peak amplitude should be ~1.0
    let max_amplitude = output.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    assert!(max_amplitude > 0.9 && max_amplitude <= 1.05,
        "FM peak amplitude should be ~1.0, got {}", max_amplitude);
}

#[test]
fn test_fm_buffer_zero_index_equals_sine() {
    let mut graph = create_test_graph();

    // With mod_index=0, FM should behave like a pure sine wave
    let fm_id = graph.add_fmoscillator_node(
        Signal::Value(440.0),
        Signal::Value(220.0),  // Irrelevant when index=0
        Signal::Value(0.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&fm_id, &mut output);

    // RMS should be close to sine wave RMS (1/sqrt(2) ≈ 0.707)
    let rms = calculate_rms(&output);
    assert!(rms > 0.65 && rms < 0.75,
        "FM with index=0 should match sine RMS (~0.707), got {}", rms);
}

// ============================================================================
// TEST: Phase Continuity
// ============================================================================

#[test]
fn test_fm_buffer_phase_continuity() {
    let mut graph = create_test_graph();

    let fm_id = graph.add_fmoscillator_node(
        Signal::Value(440.0),
        Signal::Value(220.0),
        Signal::Value(2.0),
    );

    let buffer_size = 512;
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&fm_id, &mut buffer1);
    graph.eval_node_buffer(&fm_id, &mut buffer2);

    // Check signal is continuous (no glitches at buffer boundary)
    let last_sample = buffer1[buffer_size - 1];
    let first_sample = buffer2[0];

    // The difference should be reasonable (no huge discontinuity)
    let max_step = 0.5;
    assert!((first_sample - last_sample).abs() < max_step,
        "Phase discontinuity: {} -> {} (diff = {})",
        last_sample, first_sample, (first_sample - last_sample).abs());
}

#[test]
fn test_fm_buffer_multiple_buffers_consistent() {
    let mut graph = create_test_graph();

    let fm_id = graph.add_fmoscillator_node(
        Signal::Value(440.0),
        Signal::Value(220.0),
        Signal::Value(3.0),
    );

    let buffer_size = 512;
    let num_buffers = 10;

    for i in 0..num_buffers {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&fm_id, &mut output);

        // Each buffer should have consistent energy
        let rms = calculate_rms(&output);
        assert!(rms > 0.4 && rms < 0.9,
            "Buffer {} has unexpected RMS: {}", i, rms);
    }
}

// ============================================================================
// TEST: Modulation Index Variation
// ============================================================================

#[test]
fn test_fm_buffer_varying_mod_index() {
    let mut graph = create_test_graph();
    let buffer_size = 512;

    let indices = vec![0.0, 1.0, 2.0, 5.0, 10.0];

    for &index in &indices {
        let fm_id = graph.add_fmoscillator_node(
            Signal::Value(440.0),
            Signal::Value(110.0),
            Signal::Value(index),
        );

        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&fm_id, &mut output);

        // Higher index creates more harmonics/complexity
        let crossings = count_zero_crossings(&output);

        assert!(crossings > 10,
            "FM with index {} should have multiple zero crossings, got {}",
            index, crossings);
    }
}

// ============================================================================
// TEST: Frequency Parameters
// ============================================================================

#[test]
fn test_fm_buffer_different_carriers() {
    let mut graph = create_test_graph();
    let buffer_size = 512;

    // Two different carrier frequencies
    let fm_a3 = graph.add_fmoscillator_node(
        Signal::Value(220.0),  // A3
        Signal::Value(110.0),
        Signal::Value(2.0),
    );

    let fm_a4 = graph.add_fmoscillator_node(
        Signal::Value(440.0),  // A4
        Signal::Value(110.0),
        Signal::Value(2.0),
    );

    let mut output_a3 = vec![0.0; buffer_size];
    let mut output_a4 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&fm_a3, &mut output_a3);
    graph.eval_node_buffer(&fm_a4, &mut output_a4);

    // Higher frequency should have more zero crossings
    let crossings_a3 = count_zero_crossings(&output_a3);
    let crossings_a4 = count_zero_crossings(&output_a4);

    assert!(crossings_a4 > crossings_a3,
        "A4 (440Hz) should have more crossings than A3 (220Hz): {} vs {}",
        crossings_a4, crossings_a3);
}

// ============================================================================
// TEST: Edge Cases
// ============================================================================

#[test]
fn test_fm_buffer_zero_carrier() {
    let mut graph = create_test_graph();

    let fm_id = graph.add_fmoscillator_node(
        Signal::Value(0.0),
        Signal::Value(220.0),
        Signal::Value(2.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&fm_id, &mut output);

    // Should handle gracefully (produces low-frequency modulation)
    let _rms = calculate_rms(&output);
    // Just verify no crash/NaN
    assert!(output.iter().all(|&x| x.is_finite()),
        "Zero carrier should be handled gracefully");
}

#[test]
fn test_fm_buffer_negative_frequencies_clamped() {
    let mut graph = create_test_graph();

    let fm_id = graph.add_fmoscillator_node(
        Signal::Value(-100.0),  // Negative
        Signal::Value(-50.0),   // Negative
        Signal::Value(2.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&fm_id, &mut output);

    // Should handle gracefully (clamped to 0)
    assert!(output.iter().all(|&x| x.is_finite()),
        "Negative frequencies should be handled gracefully");
}

#[test]
fn test_fm_buffer_high_mod_index() {
    let mut graph = create_test_graph();

    let fm_id = graph.add_fmoscillator_node(
        Signal::Value(440.0),
        Signal::Value(220.0),
        Signal::Value(20.0),   // Very high index
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&fm_id, &mut output);

    // Should not cause excessive amplitude
    let max_amplitude = output.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    assert!(max_amplitude <= 1.05,
        "High index should not cause excessive amplitude: {}", max_amplitude);

    let rms = calculate_rms(&output);
    assert!(rms > 0.3, "High index should produce significant energy");
}

// ============================================================================
// TEST: Performance
// ============================================================================

#[test]
fn test_fm_buffer_performance() {
    let mut graph = create_test_graph();

    let fm_id = graph.add_fmoscillator_node(
        Signal::Value(440.0),
        Signal::Value(220.0),
        Signal::Value(3.0),
    );

    let buffer_size = 512;
    let iterations = 1000;

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&fm_id, &mut output);
    }
    let duration = start.elapsed();

    println!("FM buffer eval: {:?} for {} iterations", duration, iterations);
    println!("Per iteration: {:?}", duration / iterations);

    // Should complete in reasonable time (< 2 seconds)
    assert!(duration.as_secs() < 2,
        "FM buffer evaluation too slow: {:?}", duration);
}
