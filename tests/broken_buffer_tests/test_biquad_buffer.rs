/// Tests for Biquad filter buffer-based evaluation
///
/// These tests verify that Biquad filter buffer evaluation produces correct
/// filtering behavior for all modes (lowpass, highpass, bandpass, notch)
/// and maintains proper state continuity.
///
/// Biquad is a versatile second-order IIR filter based on RBJ Audio EQ Cookbook.

use phonon::unified_graph::{Signal, UnifiedSignalGraph, Waveform};
use std::f32::consts::PI;

/// Helper: Create a test graph
fn create_test_graph() -> UnifiedSignalGraph {
    UnifiedSignalGraph::new(44100.0)
}

/// Helper: Calculate RMS of a buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Helper: Measure frequency content (simplified - just checks high-freq energy)
fn measure_high_freq_energy(buffer: &[f32]) -> f32 {
    // Simple high-pass measure: sum of absolute differences (measures rate of change)
    let mut energy = 0.0;
    for i in 1..buffer.len() {
        energy += (buffer[i] - buffer[i-1]).abs();
    }
    energy / buffer.len() as f32
}

// ============================================================================
// TEST: Lowpass Mode (mode=0)
// ============================================================================

#[test]
fn test_biquad_lowpass_reduces_high_frequencies() {
    let mut graph = create_test_graph();

    // Create high-frequency oscillator (10kHz - near Nyquist)
    let osc_id = graph.add_oscillator(Signal::Value(10000.0), Waveform::Sine);

    // Filter with low cutoff (500 Hz) should significantly reduce amplitude
    let biquad_id = graph.add_biquad_node(
        Signal::Node(osc_id),
        Signal::Value(500.0),
        Signal::Value(1.0),
        0, // mode=0 (lowpass)
    );

    let buffer_size = 512;
    let mut filtered = vec![0.0; buffer_size];
    let mut unfiltered = vec![0.0; buffer_size];

    // Get unfiltered signal
    graph.eval_node_buffer(&osc_id, &mut unfiltered);

    // Get filtered signal
    graph.eval_node_buffer(&biquad_id, &mut filtered);

    // Filtered should have much less energy than unfiltered
    let unfiltered_rms = calculate_rms(&unfiltered);
    let filtered_rms = calculate_rms(&filtered);

    assert!(filtered_rms < unfiltered_rms * 0.5,
        "Biquad LP should significantly reduce high-frequency content: unfiltered RMS = {}, filtered RMS = {}",
        unfiltered_rms, filtered_rms);
}

#[test]
fn test_biquad_lowpass_passes_low_frequencies() {
    let mut graph = create_test_graph();

    // Create low-frequency oscillator (100 Hz)
    let osc_id = graph.add_oscillator(Signal::Value(100.0), Waveform::Sine);

    // Filter with high cutoff (5000 Hz) should pass signal mostly unchanged
    let biquad_id = graph.add_biquad_node(
        Signal::Node(osc_id),
        Signal::Value(5000.0),
        Signal::Value(1.0),
        0, // mode=0 (lowpass)
    );

    let buffer_size = 512;
    let mut filtered = vec![0.0; buffer_size];
    let mut unfiltered = vec![0.0; buffer_size];

    // Get unfiltered signal
    graph.eval_node_buffer(&osc_id, &mut unfiltered);

    // Get filtered signal
    graph.eval_node_buffer(&biquad_id, &mut filtered);

    // Filtered should have similar energy to unfiltered (within 20%)
    let unfiltered_rms = calculate_rms(&unfiltered);
    let filtered_rms = calculate_rms(&filtered);

    assert!(filtered_rms > unfiltered_rms * 0.8,
        "Biquad LP with high cutoff should pass low frequencies: unfiltered RMS = {}, filtered RMS = {}",
        unfiltered_rms, filtered_rms);
}

// ============================================================================
// TEST: Highpass Mode (mode=1)
// ============================================================================

#[test]
fn test_biquad_highpass_reduces_low_frequencies() {
    let mut graph = create_test_graph();

    // Create low-frequency oscillator (100 Hz)
    let osc_id = graph.add_oscillator(Signal::Value(100.0), Waveform::Sine);

    // Filter with high cutoff (2000 Hz) should significantly reduce amplitude
    let biquad_id = graph.add_biquad_node(
        Signal::Node(osc_id),
        Signal::Value(2000.0),
        Signal::Value(1.0),
        1, // mode=1 (highpass)
    );

    let buffer_size = 512;
    let mut filtered = vec![0.0; buffer_size];
    let mut unfiltered = vec![0.0; buffer_size];

    // Get unfiltered signal
    graph.eval_node_buffer(&osc_id, &mut unfiltered);

    // Get filtered signal
    graph.eval_node_buffer(&biquad_id, &mut filtered);

    // Filtered should have much less energy than unfiltered
    let unfiltered_rms = calculate_rms(&unfiltered);
    let filtered_rms = calculate_rms(&filtered);

    assert!(filtered_rms < unfiltered_rms * 0.5,
        "Biquad HP should significantly reduce low-frequency content: unfiltered RMS = {}, filtered RMS = {}",
        unfiltered_rms, filtered_rms);
}

#[test]
fn test_biquad_highpass_passes_high_frequencies() {
    let mut graph = create_test_graph();

    // Create high-frequency oscillator (5000 Hz)
    let osc_id = graph.add_oscillator(Signal::Value(5000.0), Waveform::Sine);

    // Filter with low cutoff (500 Hz) should pass signal mostly unchanged
    let biquad_id = graph.add_biquad_node(
        Signal::Node(osc_id),
        Signal::Value(500.0),
        Signal::Value(1.0),
        1, // mode=1 (highpass)
    );

    let buffer_size = 512;
    let mut filtered = vec![0.0; buffer_size];
    let mut unfiltered = vec![0.0; buffer_size];

    // Get unfiltered signal
    graph.eval_node_buffer(&osc_id, &mut unfiltered);

    // Get filtered signal
    graph.eval_node_buffer(&biquad_id, &mut filtered);

    // Filtered should have similar energy to unfiltered (within 20%)
    let unfiltered_rms = calculate_rms(&unfiltered);
    let filtered_rms = calculate_rms(&filtered);

    assert!(filtered_rms > unfiltered_rms * 0.8,
        "Biquad HP with low cutoff should pass high frequencies: unfiltered RMS = {}, filtered RMS = {}",
        unfiltered_rms, filtered_rms);
}

// ============================================================================
// TEST: Bandpass Mode (mode=2)
// ============================================================================

#[test]
fn test_biquad_bandpass_passes_center_frequency() {
    let mut graph = create_test_graph();

    // Create oscillator at center frequency (1000 Hz)
    let osc_id = graph.add_oscillator(Signal::Value(1000.0), Waveform::Sine);

    // Bandpass filter centered at 1000 Hz
    let biquad_id = graph.add_biquad_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(2.0),
        2, // mode=2 (bandpass)
    );

    let buffer_size = 512;
    let mut filtered = vec![0.0; buffer_size];

    // Get filtered signal
    graph.eval_node_buffer(&biquad_id, &mut filtered);

    // Should pass center frequency with reasonable amplitude
    let rms = calculate_rms(&filtered);
    assert!(rms > 0.1,
        "Biquad BP should pass center frequency, RMS = {}", rms);
}

#[test]
fn test_biquad_bandpass_rejects_off_center_frequencies() {
    let mut graph = create_test_graph();

    // Create oscillator far from center (5000 Hz)
    let osc_far = graph.add_oscillator(Signal::Value(5000.0), Waveform::Sine);

    // Bandpass filter centered at 1000 Hz with narrow bandwidth (high Q)
    let biquad_id = graph.add_biquad_node(
        Signal::Node(osc_far),
        Signal::Value(1000.0),
        Signal::Value(5.0), // High Q = narrow bandwidth
        2, // mode=2 (bandpass)
    );

    let buffer_size = 512;
    let mut filtered = vec![0.0; buffer_size];
    let mut unfiltered = vec![0.0; buffer_size];

    graph.eval_node_buffer(&osc_far, &mut unfiltered);
    graph.eval_node_buffer(&biquad_id, &mut filtered);

    // Should significantly reject off-center frequencies
    let unfiltered_rms = calculate_rms(&unfiltered);
    let filtered_rms = calculate_rms(&filtered);

    assert!(filtered_rms < unfiltered_rms * 0.3,
        "Biquad BP should reject off-center frequencies: unfiltered RMS = {}, filtered RMS = {}",
        unfiltered_rms, filtered_rms);
}

// ============================================================================
// TEST: Notch Mode (mode=3)
// ============================================================================

#[test]
fn test_biquad_notch_rejects_center_frequency() {
    let mut graph = create_test_graph();

    // Create oscillator at notch frequency (1000 Hz)
    let osc_id = graph.add_oscillator(Signal::Value(1000.0), Waveform::Sine);

    // Notch filter centered at 1000 Hz
    let biquad_id = graph.add_biquad_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(5.0), // High Q = narrow notch
        3, // mode=3 (notch)
    );

    let buffer_size = 512;
    let mut filtered = vec![0.0; buffer_size];
    let mut unfiltered = vec![0.0; buffer_size];

    graph.eval_node_buffer(&osc_id, &mut unfiltered);
    graph.eval_node_buffer(&biquad_id, &mut filtered);

    // Should significantly reject center frequency
    let unfiltered_rms = calculate_rms(&unfiltered);
    let filtered_rms = calculate_rms(&filtered);

    assert!(filtered_rms < unfiltered_rms * 0.2,
        "Biquad notch should reject center frequency: unfiltered RMS = {}, filtered RMS = {}",
        unfiltered_rms, filtered_rms);
}

#[test]
fn test_biquad_notch_passes_off_center_frequencies() {
    let mut graph = create_test_graph();

    // Create oscillator far from notch (3000 Hz)
    let osc_id = graph.add_oscillator(Signal::Value(3000.0), Waveform::Sine);

    // Notch filter centered at 1000 Hz
    let biquad_id = graph.add_biquad_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(5.0),
        3, // mode=3 (notch)
    );

    let buffer_size = 512;
    let mut filtered = vec![0.0; buffer_size];
    let mut unfiltered = vec![0.0; buffer_size];

    graph.eval_node_buffer(&osc_id, &mut unfiltered);
    graph.eval_node_buffer(&biquad_id, &mut filtered);

    // Should pass off-center frequencies mostly unchanged
    let unfiltered_rms = calculate_rms(&unfiltered);
    let filtered_rms = calculate_rms(&filtered);

    assert!(filtered_rms > unfiltered_rms * 0.8,
        "Biquad notch should pass off-center frequencies: unfiltered RMS = {}, filtered RMS = {}",
        unfiltered_rms, filtered_rms);
}

// ============================================================================
// TEST: Q Factor (Resonance) Effect
// ============================================================================

#[test]
fn test_biquad_q_factor_effect() {
    let mut graph = create_test_graph();

    // Create oscillator at cutoff frequency
    let osc_id = graph.add_oscillator(Signal::Value(1000.0), Waveform::Sine);

    // Low Q (no resonance)
    let biquad_low_q = graph.add_biquad_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(0.5),
        0, // lowpass
    );

    // High Q (high resonance)
    let biquad_high_q = graph.add_biquad_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(10.0),
        0, // lowpass
    );

    let buffer_size = 512;
    let mut low_q_output = vec![0.0; buffer_size];
    let mut high_q_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&biquad_low_q, &mut low_q_output);
    graph.eval_node_buffer(&biquad_high_q, &mut high_q_output);

    // High Q should boost signal at cutoff frequency
    let low_q_rms = calculate_rms(&low_q_output);
    let high_q_rms = calculate_rms(&high_q_output);

    assert!(high_q_rms > low_q_rms,
        "Higher Q should boost signal at cutoff: low Q RMS = {}, high Q RMS = {}",
        low_q_rms, high_q_rms);
}

// ============================================================================
// TEST: State Continuity
// ============================================================================

#[test]
fn test_biquad_state_continuity() {
    let mut graph = create_test_graph();

    // Create oscillator
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Filter
    let biquad_id = graph.add_biquad_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(1.0),
        0, // lowpass
    );

    // Generate two consecutive buffers
    let buffer_size = 512;
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&biquad_id, &mut buffer1);
    graph.eval_node_buffer(&biquad_id, &mut buffer2);

    // Check continuity at boundary (no huge discontinuity)
    let last_sample = buffer1[buffer_size - 1];
    let first_sample = buffer2[0];
    let discontinuity = (first_sample - last_sample).abs();

    // Should be continuous (small change between samples)
    assert!(discontinuity < 0.1,
        "Biquad filter state should be continuous across buffers, discontinuity = {}",
        discontinuity);
}

// ============================================================================
// TEST: Multiple Buffer Evaluation
// ============================================================================

#[test]
fn test_biquad_multiple_buffers() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);
    let biquad_id = graph.add_biquad_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(2.0),
        0, // lowpass
    );

    // Generate 10 consecutive buffers
    let buffer_size = 512;
    let num_buffers = 10;

    for i in 0..num_buffers {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&biquad_id, &mut output);

        // Each buffer should have reasonable audio
        let rms = calculate_rms(&output);
        assert!(rms > 0.01 && rms < 2.0,
            "Buffer {} has unexpected RMS: {}", i, rms);
    }
}

// ============================================================================
// TEST: Modulated Parameters
// ============================================================================

#[test]
fn test_biquad_modulated_frequency() {
    let mut graph = create_test_graph();

    // Broadband signal
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);

    // LFO to modulate cutoff (0.5 Hz)
    let lfo_id = graph.add_oscillator(Signal::Value(0.5), Waveform::Sine);

    // Modulated cutoff: 500 + (lfo * 2000) = [500, 2500] Hz range
    let lfo_scaled = graph.add_multiply_node(Signal::Node(lfo_id), Signal::Value(1000.0));
    let cutoff_signal = graph.add_add_node(Signal::Node(lfo_scaled), Signal::Value(1500.0));

    let biquad_id = graph.add_biquad_node(
        Signal::Node(osc_id),
        Signal::Node(cutoff_signal),
        Signal::Value(1.0),
        0, // lowpass
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&biquad_id, &mut output);

    // Should produce sound (modulated filter)
    let rms = calculate_rms(&output);
    assert!(rms > 0.1,
        "Modulated Biquad filter should produce sound, RMS = {}", rms);
}

// ============================================================================
// TEST: Edge Cases
// ============================================================================

#[test]
fn test_biquad_stability_extreme_parameters() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(1000.0), Waveform::Sine);

    // Very high Q (testing stability)
    let biquad_id = graph.add_biquad_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(20.0),
        0, // lowpass
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    // Should not crash or produce NaN
    graph.eval_node_buffer(&biquad_id, &mut output);

    // Check no NaN/Inf values
    for (i, &sample) in output.iter().enumerate() {
        assert!(sample.is_finite(),
            "Sample {} is non-finite: {}", i, sample);
    }
}

#[test]
fn test_biquad_very_low_frequency() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Very low cutoff (20 Hz)
    let biquad_id = graph.add_biquad_node(
        Signal::Node(osc_id),
        Signal::Value(20.0),
        Signal::Value(1.0),
        0, // lowpass
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&biquad_id, &mut output);

    // Should significantly reduce amplitude
    let rms = calculate_rms(&output);
    assert!(rms < 0.3,
        "Very low cutoff should heavily attenuate signal, RMS = {}", rms);
}

#[test]
fn test_biquad_coefficient_sweep() {
    let mut graph = create_test_graph();

    // Create sawtooth (rich harmonics)
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);

    // Test sweep from low to high cutoff
    let cutoffs = [100.0, 500.0, 1000.0, 2000.0, 5000.0];
    let mut prev_high_energy = 0.0;

    for &cutoff in &cutoffs {
        let biquad_id = graph.add_biquad_node(
            Signal::Node(osc_id),
            Signal::Value(cutoff),
            Signal::Value(1.0),
            0, // lowpass
        );

        let buffer_size = 512;
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&biquad_id, &mut output);

        let high_energy = measure_high_freq_energy(&output);

        // Higher cutoff should allow more high-frequency content
        if cutoff > 100.0 {
            assert!(high_energy >= prev_high_energy * 0.8,
                "Cutoff {} Hz should have >= high-freq energy than previous: {} vs {}",
                cutoff, high_energy, prev_high_energy);
        }

        prev_high_energy = high_energy;
    }
}

// ============================================================================
// TEST: Performance
// ============================================================================

#[test]
fn test_biquad_buffer_performance() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);
    let biquad_id = graph.add_biquad_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(2.0),
        0, // lowpass
    );

    let buffer_size = 512;
    let iterations = 1000;

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&biquad_id, &mut output);
    }
    let duration = start.elapsed();

    println!("Biquad buffer eval: {:?} for {} iterations", duration, iterations);
    println!("Per iteration: {:?}", duration / iterations);

    // Should complete in reasonable time (< 1 second for 1000 iterations)
    assert!(duration.as_secs() < 1,
        "Biquad buffer evaluation too slow: {:?}", duration);
}
