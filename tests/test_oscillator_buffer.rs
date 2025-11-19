/// Tests for oscillator buffer-based evaluation
///
/// These tests verify that oscillator buffer evaluation produces correct
/// waveforms and maintains proper phase continuity.

use phonon::unified_graph::{Signal, UnifiedSignalGraph, Waveform};
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

/// Helper: Find zero crossings (count sign changes)
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
// TEST: Sine Wave Properties
// ============================================================================

#[test]
fn test_sine_wave_440hz_amplitude() {
    let mut graph = create_test_graph();

    // Add sine oscillator at 440 Hz
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Generate one buffer (512 samples at 44100 Hz = ~11.6 ms)
    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&osc_id, &mut output);

    // Check amplitude is reasonable (sine wave peak is 1.0)
    let max_amplitude = output.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    assert!(max_amplitude > 0.9 && max_amplitude <= 1.0,
        "Sine wave max amplitude should be ~1.0, got {}", max_amplitude);

    // Check RMS is reasonable (sine wave RMS = 1/sqrt(2) ≈ 0.707)
    let rms = calculate_rms(&output);
    assert!(rms > 0.6 && rms < 0.8,
        "Sine wave RMS should be ~0.707, got {}", rms);
}

#[test]
fn test_sine_wave_frequency_accuracy() {
    let mut graph = create_test_graph();
    let sample_rate = 44100.0;
    let frequency = 440.0;

    let osc_id = graph.add_oscillator(Signal::Value(frequency), Waveform::Sine);

    // Generate enough samples to capture multiple cycles
    let duration_seconds = 0.1; // 100ms
    let buffer_size = (sample_rate * duration_seconds) as usize;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&osc_id, &mut output);

    // Count zero crossings (each cycle has 2 zero crossings)
    let zero_crossings = count_zero_crossings(&output);
    let cycles = zero_crossings as f32 / 2.0;
    let measured_freq = cycles / duration_seconds;

    // Allow 5% tolerance
    let tolerance = frequency * 0.05;
    assert!((measured_freq - frequency).abs() < tolerance,
        "Expected ~{} Hz, measured {} Hz (from {} zero crossings)",
        frequency, measured_freq, zero_crossings);
}

#[test]
fn test_sine_wave_phase_continuity() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Generate two consecutive buffers
    let buffer_size = 512;
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&osc_id, &mut buffer1);
    graph.eval_node_buffer(&osc_id, &mut buffer2);

    // Check phase continuity at boundary
    // The last sample of buffer1 and first sample of buffer2 should be continuous
    let last_sample = buffer1[buffer_size - 1];
    let first_sample = buffer2[0];

    // Calculate expected phase increment
    let freq = 440.0;
    let sample_rate = 44100.0;
    let phase_increment = freq / sample_rate;

    // Calculate what the next sample should approximately be
    // (This is a rough check, not exact due to phase wrapping)
    let phase_diff = (first_sample.asin() / (2.0 * PI) - last_sample.asin() / (2.0 * PI)).abs();

    // Phase should change by approximately phase_increment per sample
    // Allow generous tolerance due to wrapping and approximation
    assert!(phase_diff < 0.1,
        "Phase should be continuous across buffers, diff = {}", phase_diff);
}

// ============================================================================
// TEST: Different Waveforms
// ============================================================================

#[test]
fn test_saw_wave_properties() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&osc_id, &mut output);

    // Saw wave ranges from -1 to 1
    let max_val = output.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
    let min_val = output.iter().fold(f32::INFINITY, |a, &b| a.min(b));

    assert!(max_val > 0.9 && max_val <= 1.0, "Saw wave max should be ~1.0");
    assert!(min_val < -0.9 && min_val >= -1.0, "Saw wave min should be ~-1.0");
}

#[test]
fn test_square_wave_properties() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Square);

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&osc_id, &mut output);

    // Square wave should be mostly 1.0 or -1.0
    let near_one = output.iter().filter(|&&x| (x - 1.0).abs() < 0.01).count();
    let near_neg_one = output.iter().filter(|&&x| (x + 1.0).abs() < 0.01).count();
    let total_near_extremes = near_one + near_neg_one;

    // Most samples should be at extremes (allow for transitions)
    assert!(total_near_extremes > buffer_size * 90 / 100,
        "Square wave should mostly be at ±1.0, got {}/{} samples",
        total_near_extremes, buffer_size);
}

#[test]
fn test_triangle_wave_properties() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Triangle);

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&osc_id, &mut output);

    // Triangle wave ranges from -1 to 1
    let max_val = output.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
    let min_val = output.iter().fold(f32::INFINITY, |a, &b| a.min(b));

    assert!(max_val > 0.9 && max_val <= 1.0, "Triangle max should be ~1.0");
    assert!(min_val < -0.9 && min_val >= -1.0, "Triangle min should be ~-1.0");
}

// ============================================================================
// TEST: Multiple Buffers (State Persistence)
// ============================================================================

#[test]
fn test_multiple_buffer_evaluation() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Generate 10 consecutive buffers
    let buffer_size = 512;
    let num_buffers = 10;

    for i in 0..num_buffers {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&osc_id, &mut output);

        // Each buffer should have reasonable audio
        let rms = calculate_rms(&output);
        assert!(rms > 0.6 && rms < 0.8,
            "Buffer {} has unexpected RMS: {}", i, rms);
    }
}

// ============================================================================
// TEST: Performance
// ============================================================================

#[test]
fn test_oscillator_buffer_performance() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    let buffer_size = 512;
    let iterations = 1000;

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&osc_id, &mut output);
    }
    let duration = start.elapsed();

    println!("Oscillator buffer eval: {:?} for {} iterations", duration, iterations);
    println!("Per iteration: {:?}", duration / iterations);

    // Should complete in reasonable time (< 1 second for 1000 iterations)
    assert!(duration.as_secs() < 1,
        "Oscillator buffer evaluation too slow: {:?}", duration);
}

// ============================================================================
// TEST: Edge Cases
// ============================================================================

#[test]
fn test_oscillator_zero_frequency() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(0.0), Waveform::Sine);

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&osc_id, &mut output);

    // Zero frequency should produce constant value (DC)
    let first_val = output[0];
    for &sample in &output {
        assert!((sample - first_val).abs() < 0.01,
            "Zero frequency should produce constant output");
    }
}

#[test]
fn test_oscillator_very_high_frequency() {
    let mut graph = create_test_graph();

    // High frequency near Nyquist (22050 Hz)
    let osc_id = graph.add_oscillator(Signal::Value(20000.0), Waveform::Sine);

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&osc_id, &mut output);

    // Should still produce output (though aliased)
    let rms = calculate_rms(&output);
    assert!(rms > 0.01, "High frequency oscillator should produce output");
}
