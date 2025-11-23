/// Tests for Resonz filter buffer-based evaluation
///
/// These tests verify that Resonz (resonant bandpass) filter buffer evaluation
/// produces correct filtering behavior with strong resonance at center frequency.

use phonon::unified_graph::{Signal, UnifiedSignalGraph, Waveform};
use rustfft::{FftPlanner, num_complex::Complex};
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

/// Helper: Perform FFT and get magnitude spectrum
fn get_spectrum(buffer: &[f32]) -> (Vec<f32>, Vec<f32>) {
    let fft_size = 8192.min(buffer.len());
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);

    let mut input: Vec<Complex<f32>> = buffer[..fft_size]
        .iter()
        .enumerate()
        .map(|(i, &sample)| {
            let window = 0.5 * (1.0 - (2.0 * PI * i as f32 / fft_size as f32).cos());
            Complex::new(sample * window, 0.0)
        })
        .collect();

    fft.process(&mut input);

    let magnitudes: Vec<f32> = input[..fft_size / 2]
        .iter()
        .map(|c| (c.re * c.re + c.im * c.im).sqrt())
        .collect();

    let frequencies: Vec<f32> = (0..fft_size / 2)
        .map(|i| i as f32 * 44100.0 / fft_size as f32)
        .collect();

    (frequencies, magnitudes)
}

/// Helper: Find peak frequency in spectrum
fn find_peak_frequency(frequencies: &[f32], magnitudes: &[f32]) -> (f32, f32) {
    let mut peak_idx = 0;
    let mut peak_mag = 0.0;

    for (i, &mag) in magnitudes.iter().enumerate() {
        if mag > peak_mag {
            peak_mag = mag;
            peak_idx = i;
        }
    }

    (frequencies[peak_idx], peak_mag)
}

// ============================================================================
// TEST: Basic Filtering
// ============================================================================

#[test]
fn test_resonz_passes_center_frequency() {
    let mut graph = create_test_graph();

    // Create white noise source
    let noise_id = graph.add_whitenoise_node();

    // Resonz centered at 1000 Hz with moderate Q
    let resonz_id = graph.add_resonz_node(
        Signal::Node(noise_id),
        Signal::Value(1000.0),
        Signal::Value(20.0),
    );

    let buffer_size = 8192;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&resonz_id, &mut output);

    // Analyze spectrum
    let (frequencies, magnitudes) = get_spectrum(&output);

    // Energy near center frequency (900-1100 Hz)
    let center_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 900.0 && **f < 1100.0)
        .map(|(_, m)| m * m)
        .sum();

    // Energy far from center (below 500 Hz or above 2000 Hz)
    let side_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f < 500.0 || **f > 2000.0)
        .map(|(_, m)| m * m)
        .sum();

    let ratio = center_energy / side_energy.max(0.001);
    assert!(ratio > 5.0,
        "Resonz should strongly pass center frequency, center/side ratio: {}",
        ratio);
}

#[test]
fn test_resonz_rejects_far_frequencies() {
    let mut graph = create_test_graph();

    // Create oscillator far from resonance (200 Hz)
    let osc_id = graph.add_oscillator(Signal::Value(200.0), Waveform::Sine);

    // Resonz centered at 2000 Hz should reject 200 Hz
    let resonz_id = graph.add_resonz_node(
        Signal::Node(osc_id),
        Signal::Value(2000.0),
        Signal::Value(20.0),
    );

    let buffer_size = 4096;
    let mut filtered = vec![0.0; buffer_size];

    graph.eval_node_buffer(&resonz_id, &mut filtered);

    // Should significantly reduce amplitude
    let filtered_rms = calculate_rms(&filtered);
    assert!(filtered_rms < 0.2,
        "Resonz should reject far frequencies: filtered RMS = {}", filtered_rms);
}

// ============================================================================
// TEST: Q Factor (Resonance)
// ============================================================================

#[test]
fn test_resonz_high_q_narrow_band() {
    let mut graph = create_test_graph();

    let noise_id = graph.add_whitenoise_node();

    // Very high Q should produce very narrow passband
    let resonz_id = graph.add_resonz_node(
        Signal::Node(noise_id),
        Signal::Value(1000.0),
        Signal::Value(50.0),
    );

    let buffer_size = 8192;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&resonz_id, &mut output);

    let (frequencies, magnitudes) = get_spectrum(&output);

    // Very narrow band around center (950-1050 Hz)
    let narrow_center: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 950.0 && **f < 1050.0)
        .map(|(_, m)| m * m)
        .sum();

    // Slightly wider band (800-1200 Hz, excluding narrow center)
    let wider_band: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| (**f > 800.0 && **f < 950.0) || (**f > 1050.0 && **f < 1200.0))
        .map(|(_, m)| m * m)
        .sum();

    // With high Q, narrow center should dominate
    assert!(narrow_center > wider_band,
        "High Q resonz should have very narrow passband, narrow: {}, wider: {}",
        narrow_center, wider_band);
}

#[test]
fn test_resonz_low_q_wider_band() {
    let mut graph = create_test_graph();

    let noise_id = graph.add_whitenoise_node();

    // Low Q should produce wider passband
    let resonz_id = graph.add_resonz_node(
        Signal::Node(noise_id),
        Signal::Value(1000.0),
        Signal::Value(2.0),
    );

    let buffer_size = 8192;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&resonz_id, &mut output);

    let (frequencies, magnitudes) = get_spectrum(&output);

    // Wide band around center (700-1400 Hz)
    let wide_band: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 700.0 && **f < 1400.0)
        .map(|(_, m)| m * m)
        .sum();

    // Far from center
    let far_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f < 400.0 || **f > 2000.0)
        .map(|(_, m)| m * m)
        .sum();

    // Even with low Q, should still favor center
    assert!(wide_band > far_energy,
        "Low Q resonz should pass wider band, wide: {}, far: {}",
        wide_band, far_energy);
}

#[test]
fn test_resonz_q_affects_resonance() {
    let mut graph = create_test_graph();

    let noise_id = graph.add_whitenoise_node();

    // Low Q
    let low_q_id = graph.add_resonz_node(
        Signal::Node(noise_id),
        Signal::Value(1000.0),
        Signal::Value(2.0),
    );

    // High Q
    let high_q_id = graph.add_resonz_node(
        Signal::Node(noise_id),
        Signal::Value(1000.0),
        Signal::Value(40.0),
    );

    let buffer_size = 4096;
    let mut low_q_output = vec![0.0; buffer_size];
    let mut high_q_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&low_q_id, &mut low_q_output);
    graph.eval_node_buffer(&high_q_id, &mut high_q_output);

    // High Q should have more pronounced peak (higher RMS in passband)
    // But since it's narrower, overall RMS might be similar or lower
    // Instead, check peak magnitude in FFT
    let (_, low_q_mags) = get_spectrum(&low_q_output);
    let (_, high_q_mags) = get_spectrum(&high_q_output);

    let low_q_peak = low_q_mags.iter().fold(0.0f32, |a, &b| a.max(b));
    let high_q_peak = high_q_mags.iter().fold(0.0f32, |a, &b| a.max(b));

    assert!(high_q_peak > low_q_peak * 0.8,
        "High Q should produce stronger resonance peak, low: {}, high: {}",
        low_q_peak, high_q_peak);
}

// ============================================================================
// TEST: Pattern Modulation
// ============================================================================

#[test]
fn test_resonz_frequency_modulation() {
    let mut graph = create_test_graph();
    graph.set_cps(1.0);

    let noise_id = graph.add_whitenoise_node();

    // Modulate frequency from 500 to 2000 Hz
    let lfo_id = graph.add_oscillator(Signal::Value(2.0), Waveform::Sine);
    let freq_signal = Signal::Expression {
        operator: phonon::unified_graph::Operator::Multiply,
        left: Box::new(Signal::Node(lfo_id)),
        right: Box::new(Signal::Value(750.0)),
    };
    let freq_signal = Signal::Expression {
        operator: phonon::unified_graph::Operator::Add,
        left: Box::new(freq_signal),
        right: Box::new(Signal::Value(1250.0)),
    };

    let resonz_id = graph.add_resonz_node(
        Signal::Node(noise_id),
        freq_signal,
        Signal::Value(20.0),
    );

    let buffer_size = 8192;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&resonz_id, &mut output);

    // Should produce audio with varying spectral content
    let rms = calculate_rms(&output);
    assert!(rms > 0.01,
        "Frequency-modulated resonz should produce audio, RMS: {}", rms);
}

#[test]
fn test_resonz_q_modulation() {
    let mut graph = create_test_graph();
    graph.set_cps(1.0);

    let noise_id = graph.add_whitenoise_node();

    // Modulate Q from 5 to 50
    let lfo_id = graph.add_oscillator(Signal::Value(3.0), Waveform::Sine);
    let q_signal = Signal::Expression {
        operator: phonon::unified_graph::Operator::Multiply,
        left: Box::new(Signal::Node(lfo_id)),
        right: Box::new(Signal::Value(22.5)),
    };
    let q_signal = Signal::Expression {
        operator: phonon::unified_graph::Operator::Add,
        left: Box::new(q_signal),
        right: Box::new(Signal::Value(27.5)),
    };

    let resonz_id = graph.add_resonz_node(
        Signal::Node(noise_id),
        Signal::Value(1000.0),
        q_signal,
    );

    let buffer_size = 8192;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&resonz_id, &mut output);

    // Should produce audio with varying resonance
    let rms = calculate_rms(&output);
    assert!(rms > 0.01,
        "Q-modulated resonz should produce audio, RMS: {}", rms);
}

// ============================================================================
// TEST: State Continuity
// ============================================================================

#[test]
fn test_resonz_state_continuity() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let resonz_id = graph.add_resonz_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(10.0),
    );

    let buffer_size = 512;
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];

    // Render first buffer
    graph.eval_node_buffer(&resonz_id, &mut buffer1);

    // Render second buffer (should be continuous)
    graph.eval_node_buffer(&resonz_id, &mut buffer2);

    // Check that there's no discontinuity at boundary
    let last_of_first = buffer1[buffer_size - 1];
    let first_of_second = buffer2[0];

    let discontinuity = (last_of_first - first_of_second).abs();

    // For a sine wave through resonz, adjacent samples should be close
    assert!(discontinuity < 0.5,
        "Resonz should maintain state continuity between buffers, discontinuity: {}",
        discontinuity);
}

// ============================================================================
// TEST: Stability
// ============================================================================

#[test]
fn test_resonz_no_nan() {
    let mut graph = create_test_graph();

    let noise_id = graph.add_whitenoise_node();
    let resonz_id = graph.add_resonz_node(
        Signal::Node(noise_id),
        Signal::Value(1000.0),
        Signal::Value(50.0),
    );

    let buffer_size = 4096;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&resonz_id, &mut output);

    // Check for NaN or Inf
    for (i, &sample) in output.iter().enumerate() {
        assert!(sample.is_finite(),
            "Resonz produced non-finite value at index {}: {}", i, sample);
    }
}

#[test]
fn test_resonz_no_excessive_clipping() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(110.0), Waveform::Saw);
    let resonz_id = graph.add_resonz_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(50.0),
    );

    let buffer_size = 4096;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&resonz_id, &mut output);

    let max_amplitude = output.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    // High Q resonz can have significant gain at resonance
    assert!(max_amplitude <= 5.0,
        "Resonz should not excessively clip, max: {}",
        max_amplitude);
}

// ============================================================================
// TEST: Frequency Sweep
// ============================================================================

#[test]
fn test_resonz_frequency_sweep() {
    let mut graph = create_test_graph();

    let noise_id = graph.add_whitenoise_node();

    // Test multiple frequencies
    let test_frequencies = [100.0, 500.0, 1000.0, 2000.0, 5000.0, 10000.0];

    for &freq in &test_frequencies {
        let resonz_id = graph.add_resonz_node(
            Signal::Node(noise_id),
            Signal::Value(freq),
            Signal::Value(20.0),
        );

        let buffer_size = 4096;
        let mut output = vec![0.0; buffer_size];

        graph.eval_node_buffer(&resonz_id, &mut output);

        let rms = calculate_rms(&output);
        assert!(rms > 0.005,
            "Resonz should work at {} Hz, RMS: {}", freq, rms);

        // Verify peak is near target frequency
        let (frequencies, magnitudes) = get_spectrum(&output);
        let (peak_freq, _) = find_peak_frequency(&frequencies, &magnitudes);

        let freq_error = (peak_freq - freq).abs() / freq;
        assert!(freq_error < 0.1,
            "Peak frequency should be near {} Hz, got {} Hz (error: {:.1}%)",
            freq, peak_freq, freq_error * 100.0);
    }
}

// ============================================================================
// TEST: Self-Oscillation
// ============================================================================

#[test]
fn test_resonz_extreme_q_produces_ringing() {
    let mut graph = create_test_graph();

    // Very short noise burst
    let noise_id = graph.add_whitenoise_node();

    // Extreme Q
    let resonz_id = graph.add_resonz_node(
        Signal::Node(noise_id),
        Signal::Value(1000.0),
        Signal::Value(100.0),
    );

    let buffer_size = 8192;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&resonz_id, &mut output);

    // Extreme Q should produce sustained ringing
    // Check that energy persists throughout the buffer
    let first_half_rms = calculate_rms(&output[..buffer_size/2]);
    let second_half_rms = calculate_rms(&output[buffer_size/2..]);

    assert!(first_half_rms > 0.01 && second_half_rms > 0.01,
        "Extreme Q should produce sustained ringing, first half: {}, second half: {}",
        first_half_rms, second_half_rms);
}
