/// Tests for HighPass filter buffer-based evaluation
///
/// These tests verify that HighPass filter buffer evaluation produces correct
/// filtering behavior (opposite of LowPass - passes high, rejects low).

use phonon::unified_graph::{Signal, UnifiedSignalGraph, Waveform};

/// Helper: Create a test graph
fn create_test_graph() -> UnifiedSignalGraph {
    UnifiedSignalGraph::new(44100.0)
}

/// Helper: Calculate RMS of a buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Helper: Measure frequency content (simplified - measures rate of change)
fn measure_high_freq_energy(buffer: &[f32]) -> f32 {
    let mut energy = 0.0;
    for i in 1..buffer.len() {
        energy += (buffer[i] - buffer[i-1]).abs();
    }
    energy / buffer.len() as f32
}

// ============================================================================
// TEST: Basic Filtering (Opposite of LowPass)
// ============================================================================

#[test]
fn test_hpf_passes_high_frequencies() {
    let mut graph = create_test_graph();

    // Create high-frequency oscillator (5kHz)
    let osc_id = graph.add_oscillator(Signal::Value(5000.0), Waveform::Sine);

    // Filter with low cutoff (500 Hz) should pass high frequencies
    let hpf_id = graph.add_highpass_node(
        Signal::Node(osc_id),
        Signal::Value(500.0),
        Signal::Value(1.0),
    );

    let buffer_size = 512;
    let mut filtered = vec![0.0; buffer_size];

    graph.eval_node_buffer(&hpf_id, &mut filtered);

    // Filtered should have strong signal (5kHz >> 500Hz cutoff)
    let filtered_rms = calculate_rms(&filtered);
    assert!(filtered_rms > 0.5,
        "HPF should pass high frequencies: filtered RMS = {}", filtered_rms);
}

#[test]
fn test_hpf_reduces_low_frequencies() {
    let mut graph = create_test_graph();

    // Create low-frequency oscillator (100 Hz)
    let osc_id = graph.add_oscillator(Signal::Value(100.0), Waveform::Sine);

    // Filter with high cutoff (2000 Hz) should reject low frequencies
    let hpf_id = graph.add_highpass_node(
        Signal::Node(osc_id),
        Signal::Value(2000.0),
        Signal::Value(1.0),
    );

    let buffer_size = 512;
    let mut filtered = vec![0.0; buffer_size];

    graph.eval_node_buffer(&hpf_id, &mut filtered);

    // Filtered should have significantly reduced amplitude
    let filtered_rms = calculate_rms(&filtered);
    assert!(filtered_rms < 0.4,
        "HPF should reduce low frequencies: filtered RMS = {}", filtered_rms);
}

// ============================================================================
// TEST: Cutoff Frequency Effect
// ============================================================================

#[test]
fn test_hpf_cutoff_effect() {
    let mut graph = create_test_graph();

    // Broadband signal (sawtooth)
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);

    // Low cutoff (200 Hz) - passes most frequencies
    let hpf_low = graph.add_highpass_node(
        Signal::Node(osc_id),
        Signal::Value(200.0),
        Signal::Value(1.0),
    );

    // High cutoff (2000 Hz) - filters more aggressively
    let hpf_high = graph.add_highpass_node(
        Signal::Node(osc_id),
        Signal::Value(2000.0),
        Signal::Value(1.0),
    );

    let buffer_size = 512;
    let mut low_cutoff = vec![0.0; buffer_size];
    let mut high_cutoff = vec![0.0; buffer_size];

    graph.eval_node_buffer(&hpf_low, &mut low_cutoff);
    graph.eval_node_buffer(&hpf_high, &mut high_cutoff);

    // Higher cutoff should have more high-frequency content
    let low_energy = measure_high_freq_energy(&low_cutoff);
    let high_energy = measure_high_freq_energy(&high_cutoff);

    // Higher cutoff = more filtering of lows = more relative high energy
    assert!(high_energy >= low_energy * 0.8,
        "Higher cutoff should preserve high freqs: low = {}, high = {}",
        low_energy, high_energy);
}

// ============================================================================
// TEST: Resonance (Q) Effect
// ============================================================================

#[test]
fn test_hpf_resonance_effect() {
    let mut graph = create_test_graph();

    // Oscillator at cutoff frequency
    let osc_id = graph.add_oscillator(Signal::Value(1000.0), Waveform::Sine);

    // Low Q (no resonance)
    let hpf_low_q = graph.add_highpass_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(0.5),
    );

    // High Q (high resonance)
    let hpf_high_q = graph.add_highpass_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(10.0),
    );

    let buffer_size = 512;
    let mut low_q_output = vec![0.0; buffer_size];
    let mut high_q_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&hpf_low_q, &mut low_q_output);
    graph.eval_node_buffer(&hpf_high_q, &mut high_q_output);

    // High Q should boost signal at cutoff frequency
    let low_q_rms = calculate_rms(&low_q_output);
    let high_q_rms = calculate_rms(&high_q_output);

    assert!(high_q_rms > low_q_rms,
        "Higher Q should boost signal: low Q = {}, high Q = {}",
        low_q_rms, high_q_rms);
}

// ============================================================================
// TEST: State Continuity
// ============================================================================

#[test]
fn test_hpf_state_continuity() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let hpf_id = graph.add_highpass_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(1.0),
    );

    // Generate two consecutive buffers
    let buffer_size = 512;
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&hpf_id, &mut buffer1);
    graph.eval_node_buffer(&hpf_id, &mut buffer2);

    // Check continuity at boundary
    let last_sample = buffer1[buffer_size - 1];
    let first_sample = buffer2[0];
    let discontinuity = (first_sample - last_sample).abs();

    assert!(discontinuity < 0.1,
        "Filter state should be continuous: discontinuity = {}", discontinuity);
}

// ============================================================================
// TEST: Multiple Buffer Evaluation
// ============================================================================

#[test]
fn test_hpf_multiple_buffers() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);
    let hpf_id = graph.add_highpass_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(2.0),
    );

    // Generate 10 consecutive buffers
    let buffer_size = 512;
    for i in 0..10 {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&hpf_id, &mut output);

        let rms = calculate_rms(&output);
        assert!(rms > 0.01 && rms < 2.0,
            "Buffer {} has unexpected RMS: {}", i, rms);
    }
}

// ============================================================================
// TEST: Modulated Parameters
// ============================================================================

#[test]
fn test_hpf_modulated_cutoff() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);

    // LFO to modulate cutoff
    let lfo_id = graph.add_oscillator(Signal::Value(0.5), Waveform::Sine);
    let lfo_scaled = graph.add_multiply_node(Signal::Node(lfo_id), Signal::Value(500.0));
    let cutoff_signal = graph.add_add_node(Signal::Node(lfo_scaled), Signal::Value(1000.0));

    let hpf_id = graph.add_highpass_node(
        Signal::Node(osc_id),
        Signal::Node(cutoff_signal),
        Signal::Value(1.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&hpf_id, &mut output);

    // Should produce sound
    let rms = calculate_rms(&output);
    assert!(rms > 0.05,
        "Modulated filter should produce sound, RMS = {}", rms);
}

// ============================================================================
// TEST: Edge Cases
// ============================================================================

#[test]
fn test_hpf_very_low_cutoff() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Very low cutoff (50 Hz) - should pass 440 Hz almost unchanged
    let hpf_id = graph.add_highpass_node(
        Signal::Node(osc_id),
        Signal::Value(50.0),
        Signal::Value(1.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&hpf_id, &mut output);

    // Should pass signal mostly unchanged
    let rms = calculate_rms(&output);
    assert!(rms > 0.6,
        "Very low cutoff should pass signal: RMS = {}", rms);
}

#[test]
fn test_hpf_very_high_cutoff() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Very high cutoff (5000 Hz) - should reject 440 Hz
    let hpf_id = graph.add_highpass_node(
        Signal::Node(osc_id),
        Signal::Value(5000.0),
        Signal::Value(1.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&hpf_id, &mut output);

    // Should significantly reduce amplitude
    let rms = calculate_rms(&output);
    assert!(rms < 0.3,
        "Very high cutoff should reject low frequencies: RMS = {}", rms);
}

#[test]
fn test_hpf_extreme_q_values() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(1000.0), Waveform::Sine);

    // Very low Q
    let hpf_low = graph.add_highpass_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(0.5),
    );

    // Very high Q
    let hpf_high = graph.add_highpass_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(20.0),
    );

    let buffer_size = 512;
    let mut low_output = vec![0.0; buffer_size];
    let mut high_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&hpf_low, &mut low_output);
    graph.eval_node_buffer(&hpf_high, &mut high_output);

    // Check no NaN/Inf values
    for &sample in &low_output {
        assert!(sample.is_finite(), "Low Q produced non-finite value");
    }
    for &sample in &high_output {
        assert!(sample.is_finite(), "High Q produced non-finite value");
    }
}

// ============================================================================
// TEST: Performance
// ============================================================================

#[test]
fn test_hpf_buffer_performance() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);
    let hpf_id = graph.add_highpass_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(2.0),
    );

    let buffer_size = 512;
    let iterations = 1000;

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&hpf_id, &mut output);
    }
    let duration = start.elapsed();

    println!("HPF buffer eval: {:?} for {} iterations", duration, iterations);
    println!("Per iteration: {:?}", duration / iterations);

    assert!(duration.as_secs() < 1,
        "HPF buffer evaluation too slow: {:?}", duration);
}

// ============================================================================
// TEST: Chained Filters
// ============================================================================

#[test]
fn test_hpf_chained() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);

    // First filter (500 Hz)
    let hpf1_id = graph.add_highpass_node(
        Signal::Node(osc_id),
        Signal::Value(500.0),
        Signal::Value(1.0),
    );

    // Second filter (1000 Hz) - should filter even more lows
    let hpf2_id = graph.add_highpass_node(
        Signal::Node(hpf1_id),
        Signal::Value(1000.0),
        Signal::Value(1.0),
    );

    let buffer_size = 512;
    let mut once_filtered = vec![0.0; buffer_size];
    let mut twice_filtered = vec![0.0; buffer_size];

    graph.eval_node_buffer(&hpf1_id, &mut once_filtered);
    graph.eval_node_buffer(&hpf2_id, &mut twice_filtered);

    // Both should have significant high-frequency content
    let once_energy = measure_high_freq_energy(&once_filtered);
    let twice_energy = measure_high_freq_energy(&twice_filtered);

    // Twice filtered should have more relative high-freq energy
    assert!(twice_energy >= once_energy * 0.8,
        "Chained HPFs should preserve highs: once = {}, twice = {}",
        once_energy, twice_energy);
}
