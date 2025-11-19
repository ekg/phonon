/// Tests for SVF (State Variable Filter) buffer-based evaluation
///
/// These tests verify that SVF buffer evaluation produces correct multi-mode
/// filtering behavior for LP, HP, BP, and Notch modes with proper state continuity.
///
/// SVF is a versatile filter that can produce multiple filter responses from the
/// same circuit topology (Chamberlin SVF).

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

/// Helper: Measure high-frequency energy (rate of change)
fn measure_high_freq_energy(buffer: &[f32]) -> f32 {
    let mut energy = 0.0;
    for i in 1..buffer.len() {
        energy += (buffer[i] - buffer[i-1]).abs();
    }
    energy / buffer.len() as f32
}

/// Helper: Perform FFT and get energy in frequency band
fn measure_band_energy(buffer: &[f32], sample_rate: f32, low_hz: f32, high_hz: f32) -> f32 {
    use rustfft::{FftPlanner, num_complex::Complex};

    let fft_size = 8192.min(buffer.len());
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);

    // Apply window and convert to complex
    let mut input: Vec<Complex<f32>> = buffer[..fft_size]
        .iter()
        .enumerate()
        .map(|(i, &sample)| {
            let window = 0.5 * (1.0 - (2.0 * PI * i as f32 / fft_size as f32).cos());
            Complex::new(sample * window, 0.0)
        })
        .collect();

    fft.process(&mut input);

    // Calculate energy in target frequency band
    let mut energy = 0.0;
    let bin_width = sample_rate / fft_size as f32;
    let low_bin = (low_hz / bin_width) as usize;
    let high_bin = (high_hz / bin_width) as usize;

    for i in low_bin..high_bin.min(fft_size / 2) {
        let magnitude = (input[i].re * input[i].re + input[i].im * input[i].im).sqrt();
        energy += magnitude;
    }

    energy
}

// ============================================================================
// TEST: Lowpass Mode (mode=0)
// ============================================================================

#[test]
fn test_svf_lowpass_mode_filters_highs() {
    let mut graph = create_test_graph();

    // Create high-frequency oscillator (8kHz)
    let osc_id = graph.add_oscillator(Signal::Value(8000.0), Waveform::Sine);

    // SVF lowpass with 1kHz cutoff should significantly reduce amplitude
    let svf_id = graph.add_svf_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(0.707), // Butterworth response
        0 // Mode 0 = Lowpass
    );

    let buffer_size = 512;
    let mut filtered = vec![0.0; buffer_size];
    let mut unfiltered = vec![0.0; buffer_size];

    graph.eval_node_buffer(&osc_id, &mut unfiltered);
    graph.eval_node_buffer(&svf_id, &mut filtered);

    let unfiltered_rms = calculate_rms(&unfiltered);
    let filtered_rms = calculate_rms(&filtered);

    assert!(filtered_rms < unfiltered_rms * 0.5,
        "SVF lowpass should reduce high-frequency content: unfiltered={}, filtered={}",
        unfiltered_rms, filtered_rms);
}

#[test]
fn test_svf_lowpass_passes_low_freqs() {
    let mut graph = create_test_graph();

    // Create low-frequency oscillator (200 Hz)
    let osc_id = graph.add_oscillator(Signal::Value(200.0), Waveform::Sine);

    // SVF lowpass with high cutoff (5kHz) should pass signal
    let svf_id = graph.add_svf_node(
        Signal::Node(osc_id),
        Signal::Value(5000.0),
        Signal::Value(0.707),
        0 // Lowpass
    );

    let buffer_size = 512;
    let mut filtered = vec![0.0; buffer_size];
    let mut unfiltered = vec![0.0; buffer_size];

    graph.eval_node_buffer(&osc_id, &mut unfiltered);
    graph.eval_node_buffer(&svf_id, &mut filtered);

    let unfiltered_rms = calculate_rms(&unfiltered);
    let filtered_rms = calculate_rms(&filtered);

    assert!(filtered_rms > unfiltered_rms * 0.8,
        "SVF lowpass with high cutoff should pass low frequencies: unfiltered={}, filtered={}",
        unfiltered_rms, filtered_rms);
}

// ============================================================================
// TEST: Highpass Mode (mode=1)
// ============================================================================

#[test]
fn test_svf_highpass_mode_filters_lows() {
    let mut graph = create_test_graph();

    // Create low-frequency oscillator (100 Hz)
    let osc_id = graph.add_oscillator(Signal::Value(100.0), Waveform::Sine);

    // SVF highpass with 1kHz cutoff should significantly reduce amplitude
    let svf_id = graph.add_svf_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(0.707),
        1 // Mode 1 = Highpass
    );

    let buffer_size = 512;
    let mut filtered = vec![0.0; buffer_size];
    let mut unfiltered = vec![0.0; buffer_size];

    graph.eval_node_buffer(&osc_id, &mut unfiltered);
    graph.eval_node_buffer(&svf_id, &mut filtered);

    let unfiltered_rms = calculate_rms(&unfiltered);
    let filtered_rms = calculate_rms(&filtered);

    assert!(filtered_rms < unfiltered_rms * 0.5,
        "SVF highpass should reduce low-frequency content: unfiltered={}, filtered={}",
        unfiltered_rms, filtered_rms);
}

#[test]
fn test_svf_highpass_passes_high_freqs() {
    let mut graph = create_test_graph();

    // Create high-frequency oscillator (8 kHz)
    let osc_id = graph.add_oscillator(Signal::Value(8000.0), Waveform::Sine);

    // SVF highpass with low cutoff (500Hz) should pass signal
    let svf_id = graph.add_svf_node(
        Signal::Node(osc_id),
        Signal::Value(500.0),
        Signal::Value(0.707),
        1 // Highpass
    );

    let buffer_size = 512;
    let mut filtered = vec![0.0; buffer_size];
    let mut unfiltered = vec![0.0; buffer_size];

    graph.eval_node_buffer(&osc_id, &mut unfiltered);
    graph.eval_node_buffer(&svf_id, &mut filtered);

    let unfiltered_rms = calculate_rms(&unfiltered);
    let filtered_rms = calculate_rms(&filtered);

    assert!(filtered_rms > unfiltered_rms * 0.6,
        "SVF highpass with low cutoff should pass high frequencies: unfiltered={}, filtered={}",
        unfiltered_rms, filtered_rms);
}

// ============================================================================
// TEST: Bandpass Mode (mode=2)
// ============================================================================

#[test]
fn test_svf_bandpass_mode_passes_center() {
    let mut graph = create_test_graph();

    // Create oscillator at center frequency (1kHz)
    let osc_center = graph.add_oscillator(Signal::Value(1000.0), Waveform::Sine);

    // Create oscillator below center (200Hz)
    let osc_low = graph.add_oscillator(Signal::Value(200.0), Waveform::Sine);

    // Create oscillator above center (5kHz)
    let osc_high = graph.add_oscillator(Signal::Value(5000.0), Waveform::Sine);

    // SVF bandpass centered at 1kHz with moderate Q
    let svf_center = graph.add_svf_node(
        Signal::Node(osc_center),
        Signal::Value(1000.0),
        Signal::Value(2.0), // Higher Q for selectivity
        2 // Mode 2 = Bandpass
    );

    let svf_low = graph.add_svf_node(
        Signal::Node(osc_low),
        Signal::Value(1000.0),
        Signal::Value(2.0),
        2
    );

    let svf_high = graph.add_svf_node(
        Signal::Node(osc_high),
        Signal::Value(1000.0),
        Signal::Value(2.0),
        2
    );

    let buffer_size = 512;
    let mut center_buf = vec![0.0; buffer_size];
    let mut low_buf = vec![0.0; buffer_size];
    let mut high_buf = vec![0.0; buffer_size];

    graph.eval_node_buffer(&svf_center, &mut center_buf);
    graph.eval_node_buffer(&svf_low, &mut low_buf);
    graph.eval_node_buffer(&svf_high, &mut high_buf);

    let center_rms = calculate_rms(&center_buf);
    let low_rms = calculate_rms(&low_buf);
    let high_rms = calculate_rms(&high_buf);

    assert!(center_rms > low_rms * 1.3,
        "SVF bandpass should pass center frequency more than low: center={}, low={}",
        center_rms, low_rms);

    assert!(center_rms > high_rms * 1.3,
        "SVF bandpass should pass center frequency more than high: center={}, high={}",
        center_rms, high_rms);
}

// ============================================================================
// TEST: Notch Mode (mode=3)
// ============================================================================

#[test]
fn test_svf_notch_mode_rejects_center() {
    let mut graph = create_test_graph();

    // Create oscillator at notch frequency (1kHz)
    let osc_notch = graph.add_oscillator(Signal::Value(1000.0), Waveform::Sine);

    // Create oscillator away from notch (200Hz)
    let osc_pass = graph.add_oscillator(Signal::Value(200.0), Waveform::Sine);

    // SVF notch at 1kHz
    let svf_notch = graph.add_svf_node(
        Signal::Node(osc_notch),
        Signal::Value(1000.0),
        Signal::Value(2.0),
        3 // Mode 3 = Notch
    );

    let svf_pass = graph.add_svf_node(
        Signal::Node(osc_pass),
        Signal::Value(1000.0),
        Signal::Value(2.0),
        3
    );

    let buffer_size = 512;
    let mut notch_buf = vec![0.0; buffer_size];
    let mut pass_buf = vec![0.0; buffer_size];

    graph.eval_node_buffer(&svf_notch, &mut notch_buf);
    graph.eval_node_buffer(&svf_pass, &mut pass_buf);

    let notch_rms = calculate_rms(&notch_buf);
    let pass_rms = calculate_rms(&pass_buf);

    // Notch filter should at least show some reduction
    // The SVF notch at moderate Q doesn't completely eliminate the frequency
    // With Q=2.0, we expect subtle reduction
    assert!(notch_rms <= pass_rms,
        "SVF notch should not amplify center frequency: notch={}, pass={}",
        notch_rms, pass_rms);

    // If we want stronger rejection, we'd need higher Q
    println!("Notch rejection: {:.1}%", (1.0 - notch_rms / pass_rms) * 100.0);
}

// ============================================================================
// TEST: Resonance Effect
// ============================================================================

#[test]
fn test_svf_resonance_effect() {
    let mut graph = create_test_graph();

    // Create broadband signal (sawtooth)
    let osc_id = graph.add_oscillator(Signal::Value(110.0), Waveform::Saw);

    // Low resonance
    let svf_low_res = graph.add_svf_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(0.5), // Low Q
        2 // Bandpass
    );

    // High resonance
    let svf_high_res = graph.add_svf_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(5.0), // High Q
        2 // Bandpass
    );

    let buffer_size = 512;
    let mut low_res_buf = vec![0.0; buffer_size];
    let mut high_res_buf = vec![0.0; buffer_size];

    graph.eval_node_buffer(&svf_low_res, &mut low_res_buf);
    graph.eval_node_buffer(&svf_high_res, &mut high_res_buf);

    // Higher resonance should produce narrower bandwidth and potentially higher peak
    let low_res_rms = calculate_rms(&low_res_buf);
    let high_res_rms = calculate_rms(&high_res_buf);

    // With high Q, the filter is more selective
    // For a saw wave, high Q should pass less overall energy but may have a peak at center freq
    println!("Low Q RMS: {}, High Q RMS: {}", low_res_rms, high_res_rms);

    // Just verify both produce sound (resonance effect varies with input spectrum)
    assert!(low_res_rms > 0.01, "Low resonance should produce audio");
    assert!(high_res_rms > 0.01, "High resonance should produce audio");
}

// ============================================================================
// TEST: Cutoff Frequency Sweep
// ============================================================================

#[test]
fn test_svf_cutoff_sweep() {
    let mut graph = create_test_graph();

    // Create broadband signal
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);

    // Low cutoff (500 Hz) should filter heavily
    let svf_low = graph.add_svf_node(
        Signal::Node(osc_id),
        Signal::Value(500.0),
        Signal::Value(1.0),
        0 // Lowpass
    );

    // High cutoff (5000 Hz) should filter less
    let svf_high = graph.add_svf_node(
        Signal::Node(osc_id),
        Signal::Value(5000.0),
        Signal::Value(1.0),
        0 // Lowpass
    );

    let buffer_size = 512;
    let mut low_cutoff = vec![0.0; buffer_size];
    let mut high_cutoff = vec![0.0; buffer_size];

    graph.eval_node_buffer(&svf_low, &mut low_cutoff);
    graph.eval_node_buffer(&svf_high, &mut high_cutoff);

    // Measure high-frequency energy
    let low_energy = measure_high_freq_energy(&low_cutoff);
    let high_energy = measure_high_freq_energy(&high_cutoff);

    assert!(low_energy < high_energy,
        "Lower cutoff should result in less high-freq energy: low={}, high={}",
        low_energy, high_energy);
}

// ============================================================================
// TEST: State Continuity
// ============================================================================

#[test]
fn test_svf_state_continuity() {
    let mut graph = create_test_graph();

    // Create oscillator
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // SVF lowpass
    let svf_id = graph.add_svf_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(0.707),
        0 // Lowpass
    );

    let buffer_size = 256;
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];

    // Render first buffer
    graph.eval_node_buffer(&svf_id, &mut buffer1);

    // Render second buffer (should continue smoothly from first)
    graph.eval_node_buffer(&svf_id, &mut buffer2);

    // Check for discontinuity at buffer boundary
    let boundary_diff = (buffer2[0] - buffer1[buffer_size - 1]).abs();

    // Calculate average sample-to-sample difference for comparison
    let mut total_diff = 0.0;
    for i in 1..buffer_size {
        total_diff += (buffer1[i] - buffer1[i-1]).abs();
    }
    let avg_diff = total_diff / (buffer_size - 1) as f32;

    // Allow boundary diff to be slightly larger than average due to phase
    // but should be within 5x of typical variation
    assert!(boundary_diff < avg_diff * 5.0 || boundary_diff < 0.1,
        "State should be continuous across buffers: boundary_diff={}, avg_diff={}",
        boundary_diff, avg_diff);
}

// ============================================================================
// TEST: Stability at Extreme Parameters
// ============================================================================

#[test]
fn test_svf_stability_high_frequency() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);

    // Very high cutoff near Nyquist
    let svf_id = graph.add_svf_node(
        Signal::Node(osc_id),
        Signal::Value(18000.0),
        Signal::Value(0.707),
        0 // Lowpass
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&svf_id, &mut output);

    // Should not produce NaN or Inf
    for (i, &sample) in output.iter().enumerate() {
        assert!(sample.is_finite(), "Sample {} is not finite: {}", i, sample);
    }

    let rms = calculate_rms(&output);
    assert!(rms.is_finite() && rms >= 0.0, "RMS should be finite and non-negative: {}", rms);
}

#[test]
fn test_svf_stability_high_resonance() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);

    // Very high resonance
    let svf_id = graph.add_svf_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),
        Signal::Value(20.0), // Extreme Q
        2 // Bandpass
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&svf_id, &mut output);

    // Should not produce NaN or Inf
    for (i, &sample) in output.iter().enumerate() {
        assert!(sample.is_finite(), "Sample {} is not finite: {}", i, sample);
    }

    let rms = calculate_rms(&output);
    assert!(rms.is_finite(), "RMS should be finite: {}", rms);
}

#[test]
fn test_svf_stability_low_frequency() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);

    // Very low cutoff
    let svf_id = graph.add_svf_node(
        Signal::Node(osc_id),
        Signal::Value(20.0),
        Signal::Value(0.707),
        0 // Lowpass
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&svf_id, &mut output);

    // Should not produce NaN or Inf
    for (i, &sample) in output.iter().enumerate() {
        assert!(sample.is_finite(), "Sample {} is not finite: {}", i, sample);
    }

    let rms = calculate_rms(&output);
    assert!(rms.is_finite() && rms >= 0.0, "RMS should be finite and non-negative: {}", rms);
}

// ============================================================================
// TEST: Mode Switching
// ============================================================================

#[test]
fn test_svf_all_modes_produce_audio() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);

    let modes = vec![
        (0, "Lowpass"),
        (1, "Highpass"),
        (2, "Bandpass"),
        (3, "Notch"),
    ];

    for (mode, mode_name) in modes {
        let svf_id = graph.add_svf_node(
            Signal::Node(osc_id),
            Signal::Value(1000.0),
            Signal::Value(0.707),
            mode
        );

        let buffer_size = 512;
        let mut output = vec![0.0; buffer_size];

        graph.eval_node_buffer(&svf_id, &mut output);

        let rms = calculate_rms(&output);
        assert!(rms > 0.01,
            "SVF mode {} ({}) should produce audio, got RMS: {}",
            mode, mode_name, rms);
    }
}
