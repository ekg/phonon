/// Tests for RHPF (Resonant Highpass Filter) buffer-based evaluation
///
/// These tests verify that RHPF buffer evaluation produces correct
/// filtering behavior including:
/// - Blocking low frequencies
/// - Passing high frequencies
/// - Resonance peak at cutoff
/// - State continuity across buffers
/// - Stability with extreme parameters
///
/// RHPF is a biquad highpass filter with Q control for resonance.
/// It's the highpass complement to RLPF.

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

/// Helper: Measure high-frequency energy (rate of change)
fn measure_high_freq_energy(buffer: &[f32]) -> f32 {
    let mut energy = 0.0;
    for i in 1..buffer.len() {
        energy += (buffer[i] - buffer[i-1]).abs();
    }
    energy / buffer.len() as f32
}

/// Helper: Measure spectral energy in a frequency band
fn measure_band_energy(buffer: &[f32], low_hz: f32, high_hz: f32, sample_rate: f32) -> f32 {
    // Simple frequency measurement using zero-crossing rate
    // This is approximate but sufficient for testing
    let mut crossings = 0;
    for i in 1..buffer.len() {
        if (buffer[i-1] < 0.0 && buffer[i] >= 0.0) || (buffer[i-1] >= 0.0 && buffer[i] < 0.0) {
            crossings += 1;
        }
    }

    let freq = (crossings as f32 / 2.0) * (sample_rate / buffer.len() as f32);

    if freq >= low_hz && freq <= high_hz {
        calculate_rms(buffer)
    } else {
        0.0
    }
}

// ============================================================================
// TEST: Basic Highpass Filtering
// ============================================================================

#[test]
fn test_rhpf_blocks_low_frequencies() {
    let mut graph = create_test_graph();

    // Create low-frequency oscillator (100 Hz)
    let osc_id = graph.add_oscillator(Signal::Value(100.0), Waveform::Sine);

    // Filter with high cutoff (1000 Hz) should significantly reduce amplitude
    let rhpf_id = graph.add_rhpf_node(
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
    graph.eval_node_buffer(&rhpf_id, &mut filtered);

    // Filtered should have much less energy than unfiltered
    let unfiltered_rms = calculate_rms(&unfiltered);
    let filtered_rms = calculate_rms(&filtered);

    assert!(filtered_rms < unfiltered_rms * 0.3,
        "RHPF should significantly reduce low-frequency content: unfiltered RMS = {}, filtered RMS = {}",
        unfiltered_rms, filtered_rms);

    println!("Low freq (100Hz) attenuation: unfiltered={:.4}, filtered={:.4}, ratio={:.2}%",
        unfiltered_rms, filtered_rms, (filtered_rms / unfiltered_rms) * 100.0);
}

#[test]
fn test_rhpf_passes_high_frequencies() {
    let mut graph = create_test_graph();

    // Create high-frequency oscillator (5000 Hz)
    let osc_id = graph.add_oscillator(Signal::Value(5000.0), Waveform::Sine);

    // Filter with low cutoff (500 Hz) should pass signal mostly unchanged
    let rhpf_id = graph.add_rhpf_node(
        Signal::Node(osc_id),
        Signal::Value(500.0),
        Signal::Value(1.0),
    );

    let buffer_size = 512;
    let mut filtered = vec![0.0; buffer_size];
    let mut unfiltered = vec![0.0; buffer_size];

    // Get unfiltered signal
    graph.eval_node_buffer(&osc_id, &mut unfiltered);

    // Get filtered signal
    graph.eval_node_buffer(&rhpf_id, &mut filtered);

    // Filtered should have similar energy to unfiltered (within 20%)
    let unfiltered_rms = calculate_rms(&unfiltered);
    let filtered_rms = calculate_rms(&filtered);

    assert!(filtered_rms > unfiltered_rms * 0.8,
        "RHPF with low cutoff should pass high frequencies: unfiltered RMS = {}, filtered RMS = {}",
        unfiltered_rms, filtered_rms);

    println!("High freq (5000Hz) passthrough: unfiltered={:.4}, filtered={:.4}, ratio={:.2}%",
        unfiltered_rms, filtered_rms, (filtered_rms / unfiltered_rms) * 100.0);
}

// ============================================================================
// TEST: Resonance Effect
// ============================================================================

#[test]
fn test_rhpf_resonance_low_vs_high() {
    let mut graph = create_test_graph();

    // Create oscillator at cutoff frequency (1000 Hz)
    let osc_id = graph.add_oscillator(Signal::Value(1000.0), Waveform::Sine);

    // Low resonance (Q = 0.5)
    let rhpf_low_q = graph.add_rhpf_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(0.5),
    );

    // High resonance (Q = 10.0)
    let rhpf_high_q = graph.add_rhpf_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(10.0),
    );

    let buffer_size = 512;
    let mut low_q_output = vec![0.0; buffer_size];
    let mut high_q_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&rhpf_low_q, &mut low_q_output);
    graph.eval_node_buffer(&rhpf_high_q, &mut high_q_output);

    // High Q should boost signal at cutoff frequency
    let low_q_rms = calculate_rms(&low_q_output);
    let high_q_rms = calculate_rms(&high_q_output);

    assert!(high_q_rms > low_q_rms * 1.2,
        "Higher Q should boost signal at cutoff: low Q RMS = {}, high Q RMS = {}",
        low_q_rms, high_q_rms);

    println!("Resonance effect: low_q={:.4}, high_q={:.4}, boost={:.2}x",
        low_q_rms, high_q_rms, high_q_rms / low_q_rms);
}

#[test]
fn test_rhpf_resonance_peak_at_cutoff() {
    let mut graph = create_test_graph();

    // Test at cutoff frequency (1000 Hz)
    let osc_at_cutoff = graph.add_oscillator(Signal::Value(1000.0), Waveform::Sine);

    // Test below cutoff (500 Hz)
    let osc_below = graph.add_oscillator(Signal::Value(500.0), Waveform::Sine);

    // High resonance filter
    let rhpf_at_cutoff = graph.add_rhpf_node(
        Signal::Node(osc_at_cutoff),
        Signal::Value(1000.0),
        Signal::Value(8.0),
    );

    let rhpf_below = graph.add_rhpf_node(
        Signal::Node(osc_below),
        Signal::Value(1000.0),
        Signal::Value(8.0),
    );

    let buffer_size = 512;
    let mut at_cutoff = vec![0.0; buffer_size];
    let mut below = vec![0.0; buffer_size];

    graph.eval_node_buffer(&rhpf_at_cutoff, &mut at_cutoff);
    graph.eval_node_buffer(&rhpf_below, &mut below);

    let at_cutoff_rms = calculate_rms(&at_cutoff);
    let below_rms = calculate_rms(&below);

    // Signal at cutoff should have higher energy than below cutoff
    assert!(at_cutoff_rms > below_rms,
        "Resonant peak should be at cutoff frequency: at_cutoff={}, below={}",
        at_cutoff_rms, below_rms);

    println!("Resonance peak: at_cutoff={:.4}, below={:.4}, ratio={:.2}x",
        at_cutoff_rms, below_rms, at_cutoff_rms / below_rms);
}

// ============================================================================
// TEST: Frequency Sweep
// ============================================================================

#[test]
fn test_rhpf_cutoff_sweep() {
    let mut graph = create_test_graph();

    // Broadband signal (sawtooth has rich harmonics)
    let osc_id = graph.add_oscillator(Signal::Value(220.0), Waveform::Saw);

    // Test different cutoff frequencies
    let cutoffs = [100.0, 500.0, 1000.0, 2000.0, 5000.0];
    let mut prev_energy = 0.0;

    for (i, &cutoff) in cutoffs.iter().enumerate() {
        let rhpf_id = graph.add_rhpf_node(
            Signal::Node(osc_id),
            Signal::Value(cutoff),
            Signal::Value(1.0),
        );

        let buffer_size = 512;
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&rhpf_id, &mut output);

        let energy = measure_high_freq_energy(&output);

        // Higher cutoff should allow more high-frequency content
        if i > 0 {
            assert!(energy >= prev_energy * 0.7,
                "Cutoff {} Hz should have >= high-freq energy than previous: {} vs {}",
                cutoff, energy, prev_energy);
        }

        println!("Cutoff {}Hz: high_freq_energy={:.4}", cutoff, energy);
        prev_energy = energy;
    }
}

// ============================================================================
// TEST: Low Frequency Rejection
// ============================================================================

#[test]
fn test_rhpf_strong_low_freq_rejection() {
    let mut graph = create_test_graph();

    // Very low frequency (50 Hz)
    let osc_id = graph.add_oscillator(Signal::Value(50.0), Waveform::Sine);

    // High cutoff (2000 Hz) should heavily attenuate
    let rhpf_id = graph.add_rhpf_node(
        Signal::Node(osc_id),
        Signal::Value(2000.0),
        Signal::Value(1.0),
    );

    let buffer_size = 512;
    let mut filtered = vec![0.0; buffer_size];
    let mut unfiltered = vec![0.0; buffer_size];

    graph.eval_node_buffer(&osc_id, &mut unfiltered);
    graph.eval_node_buffer(&rhpf_id, &mut filtered);

    let unfiltered_rms = calculate_rms(&unfiltered);
    let filtered_rms = calculate_rms(&filtered);

    // Should have >90% attenuation
    assert!(filtered_rms < unfiltered_rms * 0.1,
        "RHPF should heavily attenuate sub-cutoff frequencies: unfiltered={}, filtered={}",
        unfiltered_rms, filtered_rms);

    println!("Sub-cutoff rejection: unfiltered={:.4}, filtered={:.4}, attenuation={:.1}dB",
        unfiltered_rms, filtered_rms, 20.0 * (filtered_rms / unfiltered_rms).log10());
}

// ============================================================================
// TEST: State Continuity
// ============================================================================

#[test]
fn test_rhpf_state_continuity() {
    let mut graph = create_test_graph();

    // Create oscillator
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Filter
    let rhpf_id = graph.add_rhpf_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(2.0),
    );

    // Generate two consecutive buffers
    let buffer_size = 512;
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&rhpf_id, &mut buffer1);
    graph.eval_node_buffer(&rhpf_id, &mut buffer2);

    // Check continuity at boundary (no huge discontinuity)
    let last_sample = buffer1[buffer_size - 1];
    let first_sample = buffer2[0];
    let discontinuity = (first_sample - last_sample).abs();

    // Should be continuous (small change between samples)
    assert!(discontinuity < 0.1,
        "RHPF filter state should be continuous across buffers, discontinuity = {}",
        discontinuity);

    println!("Buffer boundary discontinuity: {:.6}", discontinuity);
}

#[test]
fn test_rhpf_multiple_buffers() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);
    let rhpf_id = graph.add_rhpf_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(2.0),
    );

    // Generate 10 consecutive buffers
    let buffer_size = 512;
    let num_buffers = 10;

    for i in 0..num_buffers {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&rhpf_id, &mut output);

        // Each buffer should have reasonable audio
        let rms = calculate_rms(&output);
        assert!(rms > 0.01 && rms < 2.0,
            "Buffer {} has unexpected RMS: {}", i, rms);

        if i < 3 {
            println!("Buffer {}: RMS={:.4}", i, rms);
        }
    }
}

// ============================================================================
// TEST: Stability
// ============================================================================

#[test]
fn test_rhpf_stability_extreme_parameters() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(1000.0), Waveform::Sine);

    // Very high Q (testing stability)
    let rhpf_id = graph.add_rhpf_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(20.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    // Should not crash or produce NaN
    graph.eval_node_buffer(&rhpf_id, &mut output);

    // Check no NaN/Inf values
    for (i, &sample) in output.iter().enumerate() {
        assert!(sample.is_finite(),
            "Sample {} is non-finite: {}", i, sample);
    }

    let rms = calculate_rms(&output);
    println!("Extreme Q stability test: RMS={:.4}", rms);
}

#[test]
fn test_rhpf_stability_very_low_cutoff() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(1000.0), Waveform::Sine);

    // Very low cutoff (20 Hz) - testing edge case
    let rhpf_id = graph.add_rhpf_node(
        Signal::Node(osc_id),
        Signal::Value(20.0),
        Signal::Value(1.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&rhpf_id, &mut output);

    // Should pass 1000 Hz signal (well above 20 Hz cutoff)
    let rms = calculate_rms(&output);
    assert!(rms > 0.5,
        "Very low cutoff should pass high frequencies, RMS = {}", rms);

    println!("Very low cutoff (20Hz) for 1000Hz signal: RMS={:.4}", rms);
}

#[test]
fn test_rhpf_stability_very_high_cutoff() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(1000.0), Waveform::Sine);

    // Very high cutoff (15000 Hz) - testing edge case
    let rhpf_id = graph.add_rhpf_node(
        Signal::Node(osc_id),
        Signal::Value(15000.0),
        Signal::Value(1.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&rhpf_id, &mut output);

    // Should attenuate 1000 Hz signal (below 15000 Hz cutoff)
    let rms = calculate_rms(&output);
    assert!(rms < 0.5,
        "Very high cutoff should attenuate lower frequencies, RMS = {}", rms);

    println!("Very high cutoff (15000Hz) for 1000Hz signal: RMS={:.4}", rms);
}

// ============================================================================
// TEST: Comparison with Non-Resonant HighPass
// ============================================================================

#[test]
fn test_rhpf_vs_highpass_at_cutoff() {
    let mut graph = create_test_graph();

    // Oscillator at cutoff frequency (1000 Hz)
    let osc_id = graph.add_oscillator(Signal::Value(1000.0), Waveform::Sine);

    // Standard HighPass (SVF-based, Q=0.707)
    let hp_id = graph.add_highpass_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(0.707),
    );

    // RHPF with high resonance (Q=5.0)
    let rhpf_id = graph.add_rhpf_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(5.0),
    );

    let buffer_size = 512;
    let mut hp_output = vec![0.0; buffer_size];
    let mut rhpf_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&hp_id, &mut hp_output);
    graph.eval_node_buffer(&rhpf_id, &mut rhpf_output);

    let hp_rms = calculate_rms(&hp_output);
    let rhpf_rms = calculate_rms(&rhpf_output);

    // RHPF with resonance should boost signal more at cutoff
    assert!(rhpf_rms > hp_rms,
        "RHPF with resonance should boost more than standard HP at cutoff: HP={}, RHPF={}",
        hp_rms, rhpf_rms);

    println!("HP vs RHPF at cutoff: HP={:.4}, RHPF={:.4}, boost={:.2}x",
        hp_rms, rhpf_rms, rhpf_rms / hp_rms);
}

// ============================================================================
// TEST: Modulated Parameters
// ============================================================================

#[test]
fn test_rhpf_modulated_cutoff() {
    let mut graph = create_test_graph();

    // Broadband signal
    let osc_id = graph.add_oscillator(Signal::Value(220.0), Waveform::Saw);

    // LFO to modulate cutoff (0.5 Hz)
    let lfo_id = graph.add_oscillator(Signal::Value(0.5), Waveform::Sine);

    // Modulated cutoff: 1000 + (lfo * 2000) = [500, 3000] Hz range
    let lfo_scaled = graph.add_multiply_node(Signal::Node(lfo_id), Signal::Value(1000.0));
    let cutoff_signal = graph.add_add_node(Signal::Node(lfo_scaled), Signal::Value(1500.0));

    let rhpf_id = graph.add_rhpf_node(
        Signal::Node(osc_id),
        Signal::Node(cutoff_signal),
        Signal::Value(2.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&rhpf_id, &mut output);

    // Should produce sound (modulated filter)
    let rms = calculate_rms(&output);
    assert!(rms > 0.05,
        "Modulated RHPF filter should produce sound, RMS = {}", rms);

    println!("Modulated cutoff: RMS={:.4}", rms);
}

#[test]
fn test_rhpf_modulated_resonance() {
    let mut graph = create_test_graph();

    // Signal at cutoff
    let osc_id = graph.add_oscillator(Signal::Value(1000.0), Waveform::Sine);

    // LFO to modulate resonance (1 Hz)
    let lfo_id = graph.add_oscillator(Signal::Value(1.0), Waveform::Sine);

    // Modulated resonance: 2 + (lfo * 5) = [1, 7] range (since lfo is [-1, 1], scaled to [0, 10] effectively)
    let lfo_scaled = graph.add_multiply_node(Signal::Node(lfo_id), Signal::Value(2.5));
    let res_signal = graph.add_add_node(Signal::Node(lfo_scaled), Signal::Value(4.0));

    let rhpf_id = graph.add_rhpf_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Node(res_signal),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&rhpf_id, &mut output);

    // Should produce sound with varying resonance
    let rms = calculate_rms(&output);
    assert!(rms > 0.1,
        "Modulated resonance RHPF should produce sound, RMS = {}", rms);

    println!("Modulated resonance: RMS={:.4}", rms);
}

// ============================================================================
// TEST: Performance
// ============================================================================

#[test]
fn test_rhpf_buffer_performance() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);
    let rhpf_id = graph.add_rhpf_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(2.0),
    );

    let buffer_size = 512;
    let iterations = 1000;

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&rhpf_id, &mut output);
    }
    let duration = start.elapsed();

    println!("RHPF buffer eval: {:?} for {} iterations", duration, iterations);
    println!("Per iteration: {:?}", duration / iterations);
    println!("Samples per second: {:.0}",
        (buffer_size as f64 * iterations as f64) / duration.as_secs_f64());

    // Should complete in reasonable time (< 1 second for 1000 iterations)
    assert!(duration.as_secs() < 1,
        "RHPF buffer evaluation too slow: {:?}", duration);
}
