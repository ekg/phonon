/// Tests for LowPass filter buffer-based evaluation
///
/// These tests verify that LowPass filter buffer evaluation produces correct
/// filtering behavior and maintains proper state continuity.

use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph, Waveform, FilterState};
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

/// Helper: Generate white noise buffer (for testing filter)
fn generate_noise(seed: u32, size: usize) -> Vec<f32> {
    let mut rng = seed;
    let mut buffer = vec![0.0; size];
    for i in 0..size {
        // Simple LCG random number generator
        rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
        buffer[i] = ((rng >> 16) & 0x7FFF) as f32 / 32768.0 * 2.0 - 1.0;
    }
    buffer
}

// ============================================================================
// TEST: Basic Filtering
// ============================================================================

#[test]
fn test_lpf_reduces_high_frequencies() {
    let mut graph = create_test_graph();

    // Create high-frequency oscillator (10kHz - near Nyquist)
    let osc_id = graph.add_oscillator(Signal::Value(10000.0), Waveform::Sine);

    // Filter with low cutoff (500 Hz) should significantly reduce amplitude
    let lpf_id = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(osc_id),
        cutoff: Signal::Value(500.0),
        q: Signal::Value(1.0),
        state: FilterState::default(),
    });

    let buffer_size = 512;
    let mut filtered = vec![0.0; buffer_size];
    let mut unfiltered = vec![0.0; buffer_size];

    // Get unfiltered signal
    graph.eval_node_buffer(&osc_id, &mut unfiltered);

    // Get filtered signal
    graph.eval_node_buffer(&lpf_id, &mut filtered);

    // Filtered should have much less energy than unfiltered
    let unfiltered_rms = calculate_rms(&unfiltered);
    let filtered_rms = calculate_rms(&filtered);

    assert!(filtered_rms < unfiltered_rms * 0.5,
        "LPF should significantly reduce high-frequency content: unfiltered RMS = {}, filtered RMS = {}",
        unfiltered_rms, filtered_rms);
}

#[test]
fn test_lpf_passes_low_frequencies() {
    let mut graph = create_test_graph();

    // Create low-frequency oscillator (100 Hz)
    let osc_id = graph.add_oscillator(Signal::Value(100.0), Waveform::Sine);

    // Filter with high cutoff (5000 Hz) should pass signal mostly unchanged
    let lpf_id = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(osc_id),
        cutoff: Signal::Value(5000.0),
        q: Signal::Value(1.0),
        state: FilterState::default(),
    });

    let buffer_size = 512;
    let mut filtered = vec![0.0; buffer_size];
    let mut unfiltered = vec![0.0; buffer_size];

    // Get unfiltered signal
    graph.eval_node_buffer(&osc_id, &mut unfiltered);

    // Get filtered signal
    graph.eval_node_buffer(&lpf_id, &mut filtered);

    // Filtered should have similar energy to unfiltered (within 20%)
    let unfiltered_rms = calculate_rms(&unfiltered);
    let filtered_rms = calculate_rms(&filtered);

    assert!(filtered_rms > unfiltered_rms * 0.8,
        "LPF with high cutoff should pass low frequencies: unfiltered RMS = {}, filtered RMS = {}",
        unfiltered_rms, filtered_rms);
}

// ============================================================================
// TEST: Cutoff Frequency Effect
// ============================================================================

#[test]
fn test_lpf_cutoff_effect() {
    let mut graph = create_test_graph();

    // Create broadband signal (sawtooth has lots of harmonics)
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);

    // Low cutoff (500 Hz) should filter heavily
    let lpf_low_id = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(osc_id),
        cutoff: Signal::Value(500.0),
        q: Signal::Value(1.0),
        state: FilterState::default(),
    });

    // High cutoff (5000 Hz) should filter less
    let lpf_high_id = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(osc_id),
        cutoff: Signal::Value(5000.0),
        q: Signal::Value(1.0),
        state: FilterState::default(),
    });

    let buffer_size = 512;
    let mut low_cutoff = vec![0.0; buffer_size];
    let mut high_cutoff = vec![0.0; buffer_size];

    graph.eval_node_buffer(&lpf_low_id, &mut low_cutoff);
    graph.eval_node_buffer(&lpf_high_id, &mut high_cutoff);

    // Measure high-frequency energy (rate of change)
    let low_energy = measure_high_freq_energy(&low_cutoff);
    let high_energy = measure_high_freq_energy(&high_cutoff);

    assert!(low_energy < high_energy,
        "Lower cutoff should result in less high-freq energy: low = {}, high = {}",
        low_energy, high_energy);
}

// ============================================================================
// TEST: Resonance (Q) Effect
// ============================================================================

#[test]
fn test_lpf_resonance_effect() {
    let mut graph = create_test_graph();

    // Create oscillator at cutoff frequency
    let osc_id = graph.add_oscillator(Signal::Value(1000.0), Waveform::Sine);

    // Low Q (no resonance)
    let lpf_low_q = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(osc_id),
        cutoff: Signal::Value(1000.0),
        q: Signal::Value(0.5),
        state: FilterState::default(),
    });

    // High Q (high resonance)
    let lpf_high_q = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(osc_id),
        cutoff: Signal::Value(1000.0),
        q: Signal::Value(10.0),
        state: FilterState::default(),
    });

    let buffer_size = 512;
    let mut low_q_output = vec![0.0; buffer_size];
    let mut high_q_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&lpf_low_q, &mut low_q_output);
    graph.eval_node_buffer(&lpf_high_q, &mut high_q_output);

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
fn test_lpf_state_continuity() {
    let mut graph = create_test_graph();

    // Create oscillator
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Filter
    let lpf_id = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(osc_id),
        cutoff: Signal::Value(1000.0),
        q: Signal::Value(1.0),
        state: FilterState::default(),
    });

    // Generate two consecutive buffers
    let buffer_size = 512;
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&lpf_id, &mut buffer1);
    graph.eval_node_buffer(&lpf_id, &mut buffer2);

    // Check continuity at boundary (no huge discontinuity)
    let last_sample = buffer1[buffer_size - 1];
    let first_sample = buffer2[0];
    let discontinuity = (first_sample - last_sample).abs();

    // Should be continuous (small change between samples)
    assert!(discontinuity < 0.1,
        "Filter state should be continuous across buffers, discontinuity = {}",
        discontinuity);
}

// ============================================================================
// TEST: Multiple Buffer Evaluation
// ============================================================================

#[test]
fn test_lpf_multiple_buffers() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);
    let lpf_id = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(osc_id),
        cutoff: Signal::Value(1000.0),
        q: Signal::Value(2.0),
        state: FilterState::default(),
    });

    // Generate 10 consecutive buffers
    let buffer_size = 512;
    let num_buffers = 10;

    for i in 0..num_buffers {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&lpf_id, &mut output);

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
fn test_lpf_modulated_cutoff() {
    let mut graph = create_test_graph();

    // Broadband signal
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);

    // LFO to modulate cutoff (0.5 Hz)
    let lfo_id = graph.add_oscillator(Signal::Value(0.5), Waveform::Sine);

    // Modulated cutoff: 500 + (lfo * 2000) = [500, 2500] Hz range
    let lfo_scaled = graph.add_multiply_node(Signal::Node(lfo_id), Signal::Value(1000.0));
    let cutoff_signal = graph.add_add_node(Signal::Node(lfo_scaled), Signal::Value(1500.0));

    let lpf_id = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(osc_id),
        cutoff: Signal::Node(cutoff_signal),
        q: Signal::Value(1.0),
        state: FilterState::default(),
    });

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&lpf_id, &mut output);

    // Should produce sound (modulated filter)
    let rms = calculate_rms(&output);
    assert!(rms > 0.1,
        "Modulated filter should produce sound, RMS = {}", rms);
}

// ============================================================================
// TEST: Edge Cases
// ============================================================================

#[test]
fn test_lpf_very_low_cutoff() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Very low cutoff (50 Hz) - should heavily attenuate 440 Hz
    let lpf_id = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(osc_id),
        cutoff: Signal::Value(50.0),
        q: Signal::Value(1.0),
        state: FilterState::default(),
    });

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&lpf_id, &mut output);

    // Should significantly reduce amplitude
    let rms = calculate_rms(&output);
    assert!(rms < 0.3,
        "Very low cutoff should heavily attenuate signal, RMS = {}", rms);
}

#[test]
fn test_lpf_very_high_cutoff() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // High cutoff (5000 Hz) - should pass 440 Hz with minimal attenuation
    let lpf_id = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(osc_id),
        cutoff: Signal::Value(5000.0),
        q: Signal::Value(1.0),
        state: FilterState::default(),
    });

    let buffer_size = 512;
    let mut filtered = vec![0.0; buffer_size];

    // Evaluate filter
    graph.eval_node_buffer(&lpf_id, &mut filtered);

    // Check no NaN or Inf values
    for (i, &sample) in filtered.iter().enumerate() {
        assert!(sample.is_finite(),
            "Sample {} is non-finite: {}", i, sample);
    }

    // Should have reasonable signal (sine wave through high-cutoff filter)
    let filtered_rms = calculate_rms(&filtered);
    assert!(filtered_rms > 0.5,
        "High cutoff filter should pass low frequencies, RMS = {}", filtered_rms);
}

#[test]
fn test_lpf_extreme_q_values() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(1000.0), Waveform::Sine);

    // Very low Q (0.5 - minimum)
    let lpf_low = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(osc_id),
        cutoff: Signal::Value(1000.0),
        q: Signal::Value(0.5),
        state: FilterState::default(),
    });

    // Very high Q (20.0 - maximum)
    let lpf_high = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(osc_id),
        cutoff: Signal::Value(1000.0),
        q: Signal::Value(20.0),
        state: FilterState::default(),
    });

    let buffer_size = 512;
    let mut low_output = vec![0.0; buffer_size];
    let mut high_output = vec![0.0; buffer_size];

    // Should not crash or produce NaN
    graph.eval_node_buffer(&lpf_low, &mut low_output);
    graph.eval_node_buffer(&lpf_high, &mut high_output);

    // Check no NaN/Inf values
    for &sample in &low_output {
        assert!(sample.is_finite(), "Low Q produced non-finite value");
    }
    for &sample in &high_output {
        assert!(sample.is_finite(), "High Q produced non-finite value");
    }
}

// ============================================================================
// TEST: Constant vs Signal Parameters
// ============================================================================

#[test]
fn test_lpf_constant_cutoff() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);

    // Constant cutoff
    let lpf_id = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(osc_id),
        cutoff: Signal::Value(1000.0),
        q: Signal::Value(1.0),
        state: FilterState::default(),
    });

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&lpf_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.1,
        "Filter with constant parameters should work, RMS = {}", rms);
}

// ============================================================================
// TEST: Performance
// ============================================================================

#[test]
fn test_lpf_buffer_performance() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);
    let lpf_id = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(osc_id),
        cutoff: Signal::Value(1000.0),
        q: Signal::Value(2.0),
        state: FilterState::default(),
    });

    let buffer_size = 512;
    let iterations = 1000;

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&lpf_id, &mut output);
    }
    let duration = start.elapsed();

    println!("LPF buffer eval: {:?} for {} iterations", duration, iterations);
    println!("Per iteration: {:?}", duration / iterations);

    // Should complete in reasonable time (< 1 second for 1000 iterations)
    assert!(duration.as_secs() < 1,
        "LPF buffer evaluation too slow: {:?}", duration);
}

// ============================================================================
// TEST: Chained Filters
// ============================================================================

#[test]
fn test_lpf_chained() {
    let mut graph = create_test_graph();

    // Create broadband signal
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);

    // First filter (2000 Hz)
    let lpf1_id = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(osc_id),
        cutoff: Signal::Value(2000.0),
        q: Signal::Value(1.0),
        state: FilterState::default(),
    });

    // Second filter (1000 Hz) - should filter even more
    let lpf2_id = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(lpf1_id),
        cutoff: Signal::Value(1000.0),
        q: Signal::Value(1.0),
        state: FilterState::default(),
    });

    let buffer_size = 512;
    let mut once_filtered = vec![0.0; buffer_size];
    let mut twice_filtered = vec![0.0; buffer_size];

    graph.eval_node_buffer(&lpf1_id, &mut once_filtered);
    graph.eval_node_buffer(&lpf2_id, &mut twice_filtered);

    // Twice filtered should have less high-frequency content
    let once_energy = measure_high_freq_energy(&once_filtered);
    let twice_energy = measure_high_freq_energy(&twice_filtered);

    assert!(twice_energy < once_energy,
        "Chained filters should filter more: once = {}, twice = {}",
        once_energy, twice_energy);
}
