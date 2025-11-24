/// Tests for BandPass filter buffer-based evaluation
///
/// These tests verify that BandPass filter buffer evaluation produces correct
/// filtering behavior (passes center frequency, rejects both low and high frequencies).

use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph, Waveform};

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
// TEST: Basic Filtering
// ============================================================================

#[test]
fn test_bpf_passes_center_frequency() {
    let mut graph = create_test_graph();

    // Create oscillator at cutoff frequency (1000 Hz)
    let osc_id = graph.add_oscillator_new(Signal::Value(1000.0), Waveform::Sine);

    // Filter centered at 1000 Hz should pass this frequency
    let bpf_id = graph.add_node(SignalNode::BandPass { input: 
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(2.0),
    );

    let buffer_size = 512;
    let mut filtered = vec![0.0; buffer_size];
    let mut unfiltered = vec![0.0; buffer_size];

    // Get unfiltered signal
    graph.eval_node_buffer(&osc_id, &mut unfiltered);

    // Get filtered signal
    graph.eval_node_buffer(&bpf_id, &mut filtered);

    // Filtered should have strong signal (at center frequency)
    let unfiltered_rms = calculate_rms(&unfiltered);
    let filtered_rms = calculate_rms(&filtered);

    assert!(filtered_rms > unfiltered_rms * 0.3,
        "BPF should pass center frequency: unfiltered RMS = {}, filtered RMS = {}",
        unfiltered_rms, filtered_rms);
}

#[test]
fn test_bpf_rejects_low_frequencies() {
    let mut graph = create_test_graph();

    // Create low-frequency oscillator (100 Hz)
    let osc_id = graph.add_oscillator_new(Signal::Value(100.0), Waveform::Sine);

    // Filter centered at 2000 Hz should reject 100 Hz
    let bpf_id = graph.add_node(SignalNode::BandPass { input: 
        Signal::Node(osc_id),
        Signal::Value(2000.0),
        Signal::Value(2.0),
    );

    let buffer_size = 512;
    let mut filtered = vec![0.0; buffer_size];

    graph.eval_node_buffer(&bpf_id, &mut filtered);

    // Should significantly reduce amplitude (far below center frequency)
    let filtered_rms = calculate_rms(&filtered);
    assert!(filtered_rms < 0.3,
        "BPF should reject low frequencies: filtered RMS = {}", filtered_rms);
}

#[test]
fn test_bpf_rejects_high_frequencies() {
    let mut graph = create_test_graph();

    // Create high-frequency oscillator (8000 Hz)
    let osc_id = graph.add_oscillator_new(Signal::Value(8000.0), Waveform::Sine);

    // Filter centered at 1000 Hz should reject 8000 Hz
    let bpf_id = graph.add_node(SignalNode::BandPass { input: 
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(2.0),
    );

    let buffer_size = 512;
    let mut filtered = vec![0.0; buffer_size];

    graph.eval_node_buffer(&bpf_id, &mut filtered);

    // Should significantly reduce amplitude (far above center frequency)
    let filtered_rms = calculate_rms(&filtered);
    assert!(filtered_rms < 0.3,
        "BPF should reject high frequencies: filtered RMS = {}", filtered_rms);
}

// ============================================================================
// TEST: Cutoff Frequency Effect
// ============================================================================

#[test]
fn test_bpf_cutoff_effect() {
    let mut graph = create_test_graph();

    // Broadband signal (sawtooth has lots of harmonics)
    let osc_id = graph.add_oscillator_new(Signal::Value(440.0), Waveform::Saw);

    // Low center frequency (500 Hz)
    let bpf_low = graph.add_node(SignalNode::BandPass { input: 
        Signal::Node(osc_id),
        Signal::Value(500.0),
        Signal::Value(2.0),
    );

    // High center frequency (2000 Hz)
    let bpf_high = graph.add_node(SignalNode::BandPass { input: 
        Signal::Node(osc_id),
        Signal::Value(2000.0),
        Signal::Value(2.0),
    );

    let buffer_size = 512;
    let mut low_cutoff = vec![0.0; buffer_size];
    let mut high_cutoff = vec![0.0; buffer_size];

    graph.eval_node_buffer(&bpf_low, &mut low_cutoff);
    graph.eval_node_buffer(&bpf_high, &mut high_cutoff);

    // Higher center frequency should have more high-frequency content
    let low_energy = measure_high_freq_energy(&low_cutoff);
    let high_energy = measure_high_freq_energy(&high_cutoff);

    assert!(high_energy > low_energy,
        "Higher center frequency should have more high-freq content: low = {}, high = {}",
        low_energy, high_energy);
}

// ============================================================================
// TEST: Resonance (Q) Effect
// ============================================================================

#[test]
fn test_bpf_resonance_narrows_band() {
    let mut graph = create_test_graph();

    // Create oscillator at center frequency
    let osc_id = graph.add_oscillator_new(Signal::Value(1000.0), Waveform::Sine);

    // Low Q (wider passband)
    let bpf_low_q = graph.add_node(SignalNode::BandPass { input: 
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(0.5),
    );

    // High Q (narrow passband)
    let bpf_high_q = graph.add_node(SignalNode::BandPass { input: 
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(10.0),
    );

    let buffer_size = 512;
    let mut low_q_output = vec![0.0; buffer_size];
    let mut high_q_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&bpf_low_q, &mut low_q_output);
    graph.eval_node_buffer(&bpf_high_q, &mut high_q_output);

    // High Q should boost signal at center frequency
    let low_q_rms = calculate_rms(&low_q_output);
    let high_q_rms = calculate_rms(&high_q_output);

    assert!(high_q_rms > low_q_rms,
        "Higher Q should boost center frequency: low Q RMS = {}, high Q RMS = {}",
        low_q_rms, high_q_rms);
}

#[test]
fn test_bpf_high_q_narrower_passband() {
    let mut graph = create_test_graph();

    // Test that high Q creates different filtering characteristics
    // High Q = narrow passband with resonance boost at center frequency

    // Broadband signal containing many frequencies
    let osc_id = graph.add_oscillator_new(Signal::Value(1000.0), Waveform::Saw);

    // Low Q (wide passband, no resonance boost)
    let bpf_low_q = graph.add_node(SignalNode::BandPass { input: 
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(0.5),
    );

    // High Q (narrow passband with resonance boost)
    let bpf_high_q = graph.add_node(SignalNode::BandPass { input: 
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(10.0),
    );

    let buffer_size = 512;
    let mut low_q_output = vec![0.0; buffer_size];
    let mut high_q_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&bpf_low_q, &mut low_q_output);
    graph.eval_node_buffer(&bpf_high_q, &mut high_q_output);

    let low_q_rms = calculate_rms(&low_q_output);
    let high_q_rms = calculate_rms(&high_q_output);

    // Both should produce output
    assert!(low_q_rms > 0.01, "Low Q should produce sound: {}", low_q_rms);
    assert!(high_q_rms > 0.01, "High Q should produce sound: {}", high_q_rms);

    // High Q boosts at resonant frequency (1000 Hz fundamental of saw wave)
    // So high Q output is typically LOUDER than low Q at center frequency
    // This is correct SVF behavior - high Q = narrow band + resonance boost
    assert!(high_q_rms > low_q_rms,
        "High Q should boost at resonant frequency: low Q = {}, high Q = {}",
        low_q_rms, high_q_rms);
}

// ============================================================================
// TEST: State Continuity
// ============================================================================

#[test]
fn test_bpf_state_continuity() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator_new(Signal::Value(440.0), Waveform::Sine);
    let bpf_id = graph.add_node(SignalNode::BandPass { input: 
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(2.0),
    );

    // Generate two consecutive buffers
    let buffer_size = 512;
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&bpf_id, &mut buffer1);
    graph.eval_node_buffer(&bpf_id, &mut buffer2);

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
fn test_bpf_multiple_buffers() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator_new(Signal::Value(440.0), Waveform::Saw);
    let bpf_id = graph.add_node(SignalNode::BandPass { input: 
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(2.0),
    );

    // Generate 10 consecutive buffers
    let buffer_size = 512;
    for i in 0..10 {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&bpf_id, &mut output);

        let rms = calculate_rms(&output);
        assert!(rms > 0.01 && rms < 2.0,
            "Buffer {} has unexpected RMS: {}", i, rms);
    }
}

// ============================================================================
// TEST: Modulated Parameters
// ============================================================================

#[test]
fn test_bpf_modulated_cutoff() {
    let mut graph = create_test_graph();

    // Broadband signal
    let osc_id = graph.add_oscillator_new(Signal::Value(440.0), Waveform::Saw);

    // LFO to modulate center frequency (0.5 Hz)
    let lfo_id = graph.add_oscillator_new(Signal::Value(0.5), Waveform::Sine);

    // Modulated cutoff: 1000 + (lfo * 1000) = [0, 2000] Hz range
    let lfo_scaled = graph.add_multiply_node(Signal::Node(lfo_id), Signal::Value(1000.0));
    let cutoff_signal = graph.add_add_node(Signal::Node(lfo_scaled), Signal::Value(1000.0));

    let bpf_id = graph.add_node(SignalNode::BandPass { input: 
        Signal::Node(osc_id),
        Signal::Node(cutoff_signal),
        Signal::Value(2.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&bpf_id, &mut output);

    // Should produce sound (modulated filter)
    let rms = calculate_rms(&output);
    assert!(rms > 0.05,
        "Modulated filter should produce sound, RMS = {}", rms);
}

// ============================================================================
// TEST: Edge Cases
// ============================================================================

#[test]
fn test_bpf_very_low_cutoff() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator_new(Signal::Value(440.0), Waveform::Sine);

    // Very low center frequency (100 Hz) - should reject 440 Hz
    let bpf_id = graph.add_node(SignalNode::BandPass { input: 
        Signal::Node(osc_id),
        Signal::Value(100.0),
        Signal::Value(2.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&bpf_id, &mut output);

    // Should significantly reduce amplitude
    let rms = calculate_rms(&output);
    assert!(rms < 0.3,
        "Very low center frequency should reject 440 Hz: RMS = {}", rms);
}

#[test]
fn test_bpf_very_high_cutoff() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator_new(Signal::Value(440.0), Waveform::Sine);

    // Very high center frequency (8000 Hz) - should reject 440 Hz
    let bpf_id = graph.add_node(SignalNode::BandPass { input: 
        Signal::Node(osc_id),
        Signal::Value(8000.0),
        Signal::Value(2.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&bpf_id, &mut output);

    // Should significantly reduce amplitude
    let rms = calculate_rms(&output);
    assert!(rms < 0.3,
        "Very high center frequency should reject 440 Hz: RMS = {}", rms);
}

#[test]
fn test_bpf_extreme_q_values() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator_new(Signal::Value(1000.0), Waveform::Sine);

    // Very low Q (wide passband)
    let bpf_low = graph.add_node(SignalNode::BandPass { input: 
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(0.5),
    );

    // Very high Q (narrow passband)
    let bpf_high = graph.add_node(SignalNode::BandPass { input: 
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(20.0),
    );

    let buffer_size = 512;
    let mut low_output = vec![0.0; buffer_size];
    let mut high_output = vec![0.0; buffer_size];

    // Should not crash or produce NaN
    graph.eval_node_buffer(&bpf_low, &mut low_output);
    graph.eval_node_buffer(&bpf_high, &mut high_output);

    // Check no NaN/Inf values
    for &sample in &low_output {
        assert!(sample.is_finite(), "Low Q produced non-finite value");
    }
    for &sample in &high_output {
        assert!(sample.is_finite(), "High Q produced non-finite value");
    }
}

#[test]
fn test_bpf_at_high_frequency() {
    let mut graph = create_test_graph();

    // Oscillator at moderately high frequency (5 kHz at 44.1 kHz sample rate)
    let osc_id = graph.add_oscillator_new(Signal::Value(5000.0), Waveform::Sine);

    // Filter centered at high but safe frequency (8 kHz)
    // Note: SVF filters can become unstable near Nyquist (22.05 kHz)
    let bpf_id = graph.add_node(SignalNode::BandPass { input: 
        Signal::Node(osc_id),
        Signal::Value(8000.0),
        Signal::Value(2.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    // Should not crash
    graph.eval_node_buffer(&bpf_id, &mut output);

    // Check no NaN/Inf values
    for (i, &sample) in output.iter().enumerate() {
        assert!(sample.is_finite(),
            "Sample {} is non-finite: {}", i, sample);
    }
}

// ============================================================================
// TEST: Comparison with LPF/HPF
// ============================================================================

#[test]
fn test_bpf_between_lpf_and_hpf() {
    let mut graph = create_test_graph();

    // Broadband signal
    let osc_id = graph.add_oscillator_new(Signal::Value(440.0), Waveform::Saw);

    // All filters at same cutoff
    let lpf_id = graph.add_node(SignalNode::LowPass { input: 
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(2.0),
    );

    let hpf_id = graph.add_node(SignalNode::HighPass { input: 
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(2.0),
    );

    let bpf_id = graph.add_node(SignalNode::BandPass { input: 
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(2.0),
    );

    let buffer_size = 512;
    let mut lpf_output = vec![0.0; buffer_size];
    let mut hpf_output = vec![0.0; buffer_size];
    let mut bpf_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&lpf_id, &mut lpf_output);
    graph.eval_node_buffer(&hpf_id, &mut hpf_output);
    graph.eval_node_buffer(&bpf_id, &mut bpf_output);

    let lpf_rms = calculate_rms(&lpf_output);
    let hpf_rms = calculate_rms(&hpf_output);
    let bpf_rms = calculate_rms(&bpf_output);

    // All three filters should produce some output
    assert!(lpf_rms > 0.01, "LPF should produce sound: {}", lpf_rms);
    assert!(hpf_rms > 0.01, "HPF should produce sound: {}", hpf_rms);
    assert!(bpf_rms > 0.01, "BPF should produce sound: {}", bpf_rms);

    // BPF filters more aggressively than LPF or HPF alone
    // (it rejects both low AND high frequencies)
    assert!(bpf_rms < lpf_rms || bpf_rms < hpf_rms,
        "BPF should filter more than LPF or HPF: lpf = {}, bpf = {}, hpf = {}",
        lpf_rms, bpf_rms, hpf_rms);
}

// ============================================================================
// TEST: Performance
// ============================================================================

#[test]
fn test_bpf_buffer_performance() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator_new(Signal::Value(440.0), Waveform::Saw);
    let bpf_id = graph.add_node(SignalNode::BandPass { input: 
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(2.0),
    );

    let buffer_size = 512;
    let iterations = 1000;

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&bpf_id, &mut output);
    }
    let duration = start.elapsed();

    println!("BPF buffer eval: {:?} for {} iterations", duration, iterations);
    println!("Per iteration: {:?}", duration / iterations);

    // Should complete in reasonable time (< 1 second for 1000 iterations)
    assert!(duration.as_secs() < 1,
        "BPF buffer evaluation too slow: {:?}", duration);
}

// ============================================================================
// TEST: Chained Filters
// ============================================================================

#[test]
fn test_bpf_chained() {
    let mut graph = create_test_graph();

    // Broadband signal
    let osc_id = graph.add_oscillator_new(Signal::Value(440.0), Waveform::Saw);

    // First filter (1000 Hz)
    let bpf1_id = graph.add_node(SignalNode::BandPass { input: 
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(2.0),
    );

    // Second filter (1000 Hz) - should narrow the band further
    let bpf2_id = graph.add_node(SignalNode::BandPass { input: 
        Signal::Node(bpf1_id),
        Signal::Value(1000.0),
        Signal::Value(2.0),
    );

    let buffer_size = 512;
    let mut once_filtered = vec![0.0; buffer_size];
    let mut twice_filtered = vec![0.0; buffer_size];

    graph.eval_node_buffer(&bpf1_id, &mut once_filtered);
    graph.eval_node_buffer(&bpf2_id, &mut twice_filtered);

    // Both should produce sound
    let once_rms = calculate_rms(&once_filtered);
    let twice_rms = calculate_rms(&twice_filtered);

    assert!(once_rms > 0.01,
        "Once filtered should have sound: RMS = {}", once_rms);
    assert!(twice_rms > 0.01,
        "Twice filtered should have sound: RMS = {}", twice_rms);
}

#[test]
fn test_bpf_different_centers_chained() {
    let mut graph = create_test_graph();

    // Broadband signal
    let osc_id = graph.add_oscillator_new(Signal::Value(440.0), Waveform::Saw);

    // First filter (1000 Hz)
    let bpf1_id = graph.add_node(SignalNode::BandPass { input: 
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(2.0),
    );

    // Second filter (500 Hz) - different center, should reduce signal more
    let bpf2_id = graph.add_node(SignalNode::BandPass { input: 
        Signal::Node(bpf1_id),
        Signal::Value(500.0),
        Signal::Value(2.0),
    );

    let buffer_size = 512;
    let mut once_filtered = vec![0.0; buffer_size];
    let mut twice_filtered = vec![0.0; buffer_size];

    graph.eval_node_buffer(&bpf1_id, &mut once_filtered);
    graph.eval_node_buffer(&bpf2_id, &mut twice_filtered);

    let once_rms = calculate_rms(&once_filtered);
    let twice_rms = calculate_rms(&twice_filtered);

    // Second filter with different center should reduce energy
    assert!(twice_rms < once_rms,
        "Chained filters with different centers should filter more: once = {}, twice = {}",
        once_rms, twice_rms);
}

// ============================================================================
// TEST: Constant vs Signal Parameters
// ============================================================================

#[test]
fn test_bpf_constant_cutoff() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator_new(Signal::Value(440.0), Waveform::Saw);

    // Constant cutoff
    let bpf_id = graph.add_node(SignalNode::BandPass { input: 
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(2.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&bpf_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.05,
        "Filter with constant parameters should work, RMS = {}", rms);
}
