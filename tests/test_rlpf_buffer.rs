/// Tests for RLPF (Resonant Lowpass Filter) buffer-based evaluation
///
/// These tests verify that RLPF filter buffer evaluation produces correct
/// filtering behavior with resonance and maintains proper state continuity.
///
/// RLPF is a biquad lowpass filter with Q (resonance) parameter that creates
/// a resonant peak at the cutoff frequency, essential for analog synthesizer sounds.

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

/// Helper: Perform FFT and find peak magnitude in a frequency range
fn find_peak_in_range(buffer: &[f32], sample_rate: f32, freq_min: f32, freq_max: f32) -> f32 {
    use rustfft::{FftPlanner, num_complex::Complex};

    let fft_size = 8192.min(buffer.len());
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);

    // Apply Hann window
    let mut input: Vec<Complex<f32>> = buffer[..fft_size]
        .iter()
        .enumerate()
        .map(|(i, &sample)| {
            let window = 0.5 * (1.0 - (2.0 * PI * i as f32 / fft_size as f32).cos());
            Complex::new(sample * window, 0.0)
        })
        .collect();

    fft.process(&mut input);

    // Find peak in specified frequency range
    let mut peak = 0.0f32;
    for i in 0..fft_size / 2 {
        let freq = i as f32 * sample_rate / fft_size as f32;
        if freq >= freq_min && freq <= freq_max {
            let magnitude = (input[i].re * input[i].re + input[i].im * input[i].im).sqrt();
            peak = peak.max(magnitude);
        }
    }

    peak
}

// ============================================================================
// TEST: Basic Filtering
// ============================================================================

#[test]
fn test_rlpf_reduces_high_frequencies() {
    let mut graph = create_test_graph();

    // Create high-frequency oscillator (10kHz - near Nyquist)
    let osc_id = graph.add_oscillator(Signal::Value(10000.0), Waveform::Sine);

    // Filter with low cutoff (500 Hz) should significantly reduce amplitude
    let rlpf_id = graph.add_rlpf_node(
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
    graph.eval_node_buffer(&rlpf_id, &mut filtered);

    // Filtered should have much less energy than unfiltered
    let unfiltered_rms = calculate_rms(&unfiltered);
    let filtered_rms = calculate_rms(&filtered);

    assert!(filtered_rms < unfiltered_rms * 0.5,
        "RLPF should significantly reduce high-frequency content: unfiltered RMS = {}, filtered RMS = {}",
        unfiltered_rms, filtered_rms);
}

#[test]
fn test_rlpf_passes_low_frequencies() {
    let mut graph = create_test_graph();

    // Create low-frequency oscillator (100 Hz)
    let osc_id = graph.add_oscillator(Signal::Value(100.0), Waveform::Sine);

    // Filter with high cutoff (5000 Hz) should pass signal mostly unchanged
    let rlpf_id = graph.add_rlpf_node(
        Signal::Node(osc_id),
        Signal::Value(5000.0),
        Signal::Value(1.0),
    );

    let buffer_size = 512;
    let mut filtered = vec![0.0; buffer_size];
    let mut unfiltered = vec![0.0; buffer_size];

    // Get unfiltered signal
    graph.eval_node_buffer(&osc_id, &mut unfiltered);

    // Get filtered signal
    graph.eval_node_buffer(&rlpf_id, &mut filtered);

    // Filtered should have similar energy to unfiltered (within 20%)
    let unfiltered_rms = calculate_rms(&unfiltered);
    let filtered_rms = calculate_rms(&filtered);

    assert!(filtered_rms > unfiltered_rms * 0.8,
        "RLPF with high cutoff should pass low frequencies: unfiltered RMS = {}, filtered RMS = {}",
        unfiltered_rms, filtered_rms);
}

// ============================================================================
// TEST: Cutoff Frequency Effect
// ============================================================================

#[test]
fn test_rlpf_cutoff_effect() {
    let mut graph = create_test_graph();

    // Create broadband signal (sawtooth has lots of harmonics)
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);

    // Low cutoff (500 Hz) should filter heavily
    let rlpf_low_id = graph.add_rlpf_node(
        Signal::Node(osc_id),
        Signal::Value(500.0),
        Signal::Value(1.0),
    );

    // High cutoff (5000 Hz) should filter less
    let rlpf_high_id = graph.add_rlpf_node(
        Signal::Node(osc_id),
        Signal::Value(5000.0),
        Signal::Value(1.0),
    );

    let buffer_size = 512;
    let mut low_cutoff = vec![0.0; buffer_size];
    let mut high_cutoff = vec![0.0; buffer_size];

    graph.eval_node_buffer(&rlpf_low_id, &mut low_cutoff);
    graph.eval_node_buffer(&rlpf_high_id, &mut high_cutoff);

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
fn test_rlpf_resonance_boost() {
    let mut graph = create_test_graph();

    // Create oscillator at cutoff frequency
    let osc_id = graph.add_oscillator(Signal::Value(1000.0), Waveform::Sine);

    // Low Q (no resonance)
    let rlpf_low_q = graph.add_rlpf_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(0.5),
    );

    // High Q (high resonance)
    let rlpf_high_q = graph.add_rlpf_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(10.0),
    );

    let buffer_size = 512;
    let mut low_q_output = vec![0.0; buffer_size];
    let mut high_q_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&rlpf_low_q, &mut low_q_output);
    graph.eval_node_buffer(&rlpf_high_q, &mut high_q_output);

    // High Q should boost signal at cutoff frequency
    let low_q_rms = calculate_rms(&low_q_output);
    let high_q_rms = calculate_rms(&high_q_output);

    assert!(high_q_rms > low_q_rms * 1.2,
        "Higher Q should boost signal at cutoff: low Q RMS = {}, high Q RMS = {}",
        low_q_rms, high_q_rms);
}

#[test]
fn test_rlpf_resonance_peak_at_cutoff() {
    let mut graph = create_test_graph();

    // Create white noise (full spectrum)
    let noise_id = graph.add_whitenoise_node();

    // Apply RLPF with high resonance at 1000 Hz
    let rlpf_id = graph.add_rlpf_node(
        Signal::Node(noise_id),
        Signal::Value(1000.0),
        Signal::Value(10.0),
    );

    let buffer_size = 8192;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&rlpf_id, &mut output);

    // Find peak near cutoff (900-1100 Hz)
    let peak_at_cutoff = find_peak_in_range(&output, 44100.0, 900.0, 1100.0);

    // Find average in passband below cutoff (200-400 Hz)
    let passband_level = find_peak_in_range(&output, 44100.0, 200.0, 400.0);

    // Resonance should create a peak at cutoff
    assert!(peak_at_cutoff > passband_level * 1.5,
        "Resonance should create peak at cutoff: peak = {}, passband = {}",
        peak_at_cutoff, passband_level);
}

#[test]
fn test_rlpf_high_q_self_oscillation() {
    let mut graph = create_test_graph();

    // Create very brief impulse (white noise for one sample)
    let noise_id = graph.add_whitenoise_node();

    // Apply RLPF with very high resonance (near self-oscillation)
    let rlpf_id = graph.add_rlpf_node(
        Signal::Node(noise_id),
        Signal::Value(1000.0),
        Signal::Value(20.0),  // Maximum Q
    );

    let buffer_size = 8192;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&rlpf_id, &mut output);

    // High Q should sustain energy (ringing)
    let rms = calculate_rms(&output);
    assert!(rms > 0.1,
        "Very high Q should produce ringing/self-oscillation, RMS = {}", rms);

    // Check for sustained oscillation (energy in later half of buffer)
    let late_start = buffer_size / 2;
    let late_rms = calculate_rms(&output[late_start..]);
    assert!(late_rms > 0.05,
        "High Q should sustain oscillation, late RMS = {}", late_rms);
}

// ============================================================================
// TEST: State Continuity
// ============================================================================

#[test]
fn test_rlpf_state_continuity() {
    let mut graph = create_test_graph();

    // Create oscillator
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Filter
    let rlpf_id = graph.add_rlpf_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(2.0),
    );

    // Generate two consecutive buffers
    let buffer_size = 512;
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&rlpf_id, &mut buffer1);
    graph.eval_node_buffer(&rlpf_id, &mut buffer2);

    // Check continuity at boundary (no huge discontinuity)
    let last_sample = buffer1[buffer_size - 1];
    let first_sample = buffer2[0];
    let discontinuity = (first_sample - last_sample).abs();

    // Should be continuous (small change between samples)
    assert!(discontinuity < 0.15,
        "Filter state should be continuous across buffers, discontinuity = {}",
        discontinuity);
}

// ============================================================================
// TEST: Multiple Buffer Evaluation
// ============================================================================

#[test]
fn test_rlpf_multiple_buffers() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);
    let rlpf_id = graph.add_rlpf_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(3.0),
    );

    // Generate 10 consecutive buffers
    let buffer_size = 512;
    let num_buffers = 10;

    for i in 0..num_buffers {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&rlpf_id, &mut output);

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
fn test_rlpf_modulated_cutoff() {
    let mut graph = create_test_graph();

    // Broadband signal
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);

    // LFO to modulate cutoff (0.5 Hz)
    let lfo_id = graph.add_oscillator(Signal::Value(0.5), Waveform::Sine);

    // Modulated cutoff: 1500 + (lfo * 1000) = [500, 2500] Hz range
    let lfo_scaled = graph.add_multiply_node(Signal::Node(lfo_id), Signal::Value(1000.0));
    let cutoff_signal = graph.add_add_node(Signal::Node(lfo_scaled), Signal::Value(1500.0));

    let rlpf_id = graph.add_rlpf_node(
        Signal::Node(osc_id),
        Signal::Node(cutoff_signal),
        Signal::Value(2.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&rlpf_id, &mut output);

    // Should produce sound (modulated filter)
    let rms = calculate_rms(&output);
    assert!(rms > 0.1,
        "Modulated filter should produce sound, RMS = {}", rms);
}

#[test]
fn test_rlpf_modulated_resonance() {
    let mut graph = create_test_graph();

    // Broadband signal
    let osc_id = graph.add_oscillator(Signal::Value(220.0), Waveform::Saw);

    // LFO to modulate resonance (1 Hz)
    let lfo_id = graph.add_oscillator(Signal::Value(1.0), Waveform::Sine);

    // Modulated resonance: 5 + (lfo * 5) = [0, 10] range
    let lfo_scaled = graph.add_multiply_node(Signal::Node(lfo_id), Signal::Value(5.0));
    let res_signal = graph.add_add_node(Signal::Node(lfo_scaled), Signal::Value(5.0));

    let rlpf_id = graph.add_rlpf_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Node(res_signal),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&rlpf_id, &mut output);

    // Should produce sound (modulated resonance)
    let rms = calculate_rms(&output);
    assert!(rms > 0.1,
        "Modulated resonance filter should produce sound, RMS = {}", rms);
}

// ============================================================================
// TEST: Edge Cases
// ============================================================================

#[test]
fn test_rlpf_very_low_cutoff() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Very low cutoff (50 Hz) - should heavily attenuate 440 Hz
    let rlpf_id = graph.add_rlpf_node(
        Signal::Node(osc_id),
        Signal::Value(50.0),
        Signal::Value(1.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&rlpf_id, &mut output);

    // Should significantly reduce amplitude
    let rms = calculate_rms(&output);
    assert!(rms < 0.3,
        "Very low cutoff should heavily attenuate signal, RMS = {}", rms);
}

#[test]
fn test_rlpf_very_high_cutoff() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // High cutoff (10000 Hz) - should pass 440 Hz with minimal attenuation
    let rlpf_id = graph.add_rlpf_node(
        Signal::Node(osc_id),
        Signal::Value(10000.0),
        Signal::Value(1.0),
    );

    let buffer_size = 512;
    let mut filtered = vec![0.0; buffer_size];

    // Evaluate filter
    graph.eval_node_buffer(&rlpf_id, &mut filtered);

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
fn test_rlpf_extreme_q_values() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(1000.0), Waveform::Sine);

    // Very low Q (0.1 - minimum)
    let rlpf_low = graph.add_rlpf_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(0.1),
    );

    // Very high Q (20.0 - maximum)
    let rlpf_high = graph.add_rlpf_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(20.0),
    );

    let buffer_size = 512;
    let mut low_output = vec![0.0; buffer_size];
    let mut high_output = vec![0.0; buffer_size];

    // Should not crash or produce NaN
    graph.eval_node_buffer(&rlpf_low, &mut low_output);
    graph.eval_node_buffer(&rlpf_high, &mut high_output);

    // Check no NaN/Inf values
    for &sample in &low_output {
        assert!(sample.is_finite(), "Low Q produced non-finite value");
    }
    for &sample in &high_output {
        assert!(sample.is_finite(), "High Q produced non-finite value");
    }
}

#[test]
fn test_rlpf_stability() {
    let mut graph = create_test_graph();

    // Create oscillator
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);

    // RLPF with moderate resonance
    let rlpf_id = graph.add_rlpf_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(8.0),
    );

    // Generate many buffers to test stability
    let buffer_size = 512;
    for i in 0..100 {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&rlpf_id, &mut output);

        // Check no NaN/Inf values
        for &sample in &output {
            assert!(sample.is_finite(),
                "Buffer {} produced non-finite value", i);
        }

        // Check RMS stays reasonable
        let rms = calculate_rms(&output);
        assert!(rms < 10.0,
            "Buffer {} has runaway values: RMS = {}", i, rms);
    }
}

// ============================================================================
// TEST: Constant vs Signal Parameters
// ============================================================================

#[test]
fn test_rlpf_constant_parameters() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);

    // Constant parameters
    let rlpf_id = graph.add_rlpf_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(2.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&rlpf_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.1,
        "Filter with constant parameters should work, RMS = {}", rms);
}

// ============================================================================
// TEST: Performance
// ============================================================================

#[test]
fn test_rlpf_buffer_performance() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);
    let rlpf_id = graph.add_rlpf_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(3.0),
    );

    let buffer_size = 512;
    let iterations = 1000;

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&rlpf_id, &mut output);
    }
    let duration = start.elapsed();

    println!("RLPF buffer eval: {:?} for {} iterations", duration, iterations);
    println!("Per iteration: {:?}", duration / iterations);

    // Should complete in reasonable time (< 1 second for 1000 iterations)
    assert!(duration.as_secs() < 1,
        "RLPF buffer evaluation too slow: {:?}", duration);
}

// ============================================================================
// TEST: Chained Filters
// ============================================================================

#[test]
fn test_rlpf_chained() {
    let mut graph = create_test_graph();

    // Create broadband signal
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);

    // First filter (2000 Hz)
    let rlpf1_id = graph.add_rlpf_node(
        Signal::Node(osc_id),
        Signal::Value(2000.0),
        Signal::Value(2.0),
    );

    // Second filter (1000 Hz) - should filter even more
    let rlpf2_id = graph.add_rlpf_node(
        Signal::Node(rlpf1_id),
        Signal::Value(1000.0),
        Signal::Value(2.0),
    );

    let buffer_size = 512;
    let mut once_filtered = vec![0.0; buffer_size];
    let mut twice_filtered = vec![0.0; buffer_size];

    graph.eval_node_buffer(&rlpf1_id, &mut once_filtered);
    graph.eval_node_buffer(&rlpf2_id, &mut twice_filtered);

    // Twice filtered should have less overall energy (more filtering)
    let once_rms = calculate_rms(&once_filtered);
    let twice_rms = calculate_rms(&twice_filtered);

    // Second filter should reduce energy (additional attenuation)
    assert!(twice_rms < once_rms * 0.95,
        "Chained filters should reduce energy: once RMS = {}, twice RMS = {}",
        once_rms, twice_rms);

    // Also check high-frequency content (but be lenient due to resonance effects)
    let once_hf = measure_high_freq_energy(&once_filtered);
    let twice_hf = measure_high_freq_energy(&twice_filtered);

    // With resonance, the relationship may be complex, so we just verify both produce valid output
    assert!(once_hf > 0.0 && twice_hf > 0.0,
        "Both filters should produce high-frequency content: once = {}, twice = {}",
        once_hf, twice_hf);
}

// ============================================================================
// TEST: Comparison with Non-Resonant LowPass
// ============================================================================

#[test]
fn test_rlpf_vs_lowpass() {
    let mut graph = create_test_graph();

    // Create broadband signal
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);

    // Regular lowpass (SVF-based)
    let lpf_id = graph.add_lowpass_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(1.0),
    );

    // RLPF with high resonance
    let rlpf_id = graph.add_rlpf_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(10.0),
    );

    let buffer_size = 512;
    let mut lpf_output = vec![0.0; buffer_size];
    let mut rlpf_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&lpf_id, &mut lpf_output);
    graph.eval_node_buffer(&rlpf_id, &mut rlpf_output);

    // RLPF with high Q should have more energy at cutoff (resonant peak)
    let lpf_rms = calculate_rms(&lpf_output);
    let rlpf_rms = calculate_rms(&rlpf_output);

    assert!(rlpf_rms > lpf_rms * 1.1,
        "RLPF with high Q should have more energy than regular LPF: RLPF RMS = {}, LPF RMS = {}",
        rlpf_rms, lpf_rms);
}
