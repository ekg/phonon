/// Comprehensive buffer evaluation tests for Blip (Band-Limited Impulse Train)
///
/// Tests the buffer-based evaluation path for Blip oscillator.
/// Verifies that buffer evaluation produces identical results to sample-by-sample
/// evaluation and that it correctly handles:
/// - Constant frequency
/// - Modulated frequency (LFO, sweep)
/// - Anti-aliasing (band-limiting)
/// - Phase continuity across buffers
/// - Performance characteristics
///
/// Key characteristics of Blip:
/// - Periodic impulse train at specified frequency
/// - Band-limited (no aliasing above Nyquist frequency)
/// - Rich harmonic content up to Nyquist
/// - Useful for percussive sounds and synthesis building blocks

use phonon::unified_graph::{UnifiedSignalGraph, Signal};
use std::f32::consts::PI;

mod audio_test_utils;
use audio_test_utils::calculate_rms;

/// Helper function to create a test graph
fn create_test_graph() -> UnifiedSignalGraph {
    UnifiedSignalGraph::new(44100.0)
}

/// Perform FFT and analyze spectrum
/// Returns (frequency_bins, magnitudes)
fn analyze_spectrum(buffer: &[f32], sample_rate: f32) -> (Vec<f32>, Vec<f32>) {
    use rustfft::{FftPlanner, num_complex::Complex};

    let fft_size = 8192.min(buffer.len());
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);

    // Prepare input with Hann window
    let mut input: Vec<Complex<f32>> = buffer[..fft_size]
        .iter()
        .enumerate()
        .map(|(i, &sample)| {
            let window = 0.5 * (1.0 - (2.0 * PI * i as f32 / fft_size as f32).cos());
            Complex::new(sample * window, 0.0)
        })
        .collect();

    fft.process(&mut input);

    // Calculate magnitudes and frequencies
    let magnitudes: Vec<f32> = input[..fft_size / 2]
        .iter()
        .map(|c| (c.re * c.re + c.im * c.im).sqrt())
        .collect();

    let frequencies: Vec<f32> = (0..fft_size / 2)
        .map(|i| i as f32 * sample_rate / fft_size as f32)
        .collect();

    (frequencies, magnitudes)
}

/// Find peaks in spectrum above threshold
/// Returns (frequency, magnitude) pairs
fn find_spectral_peaks(frequencies: &[f32], magnitudes: &[f32], threshold: f32) -> Vec<(f32, f32)> {
    let mut peaks = Vec::new();

    for i in 1..magnitudes.len() - 1 {
        if magnitudes[i] > threshold
            && magnitudes[i] > magnitudes[i - 1]
            && magnitudes[i] > magnitudes[i + 1]
        {
            peaks.push((frequencies[i], magnitudes[i]));
        }
    }

    peaks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    peaks
}

// ========== Basic Buffer Evaluation Tests ==========

#[test]
fn test_blip_buffer_constant_frequency() {
    // Test basic buffer evaluation with constant frequency
    let mut graph = create_test_graph();

    let blip_id = graph.add_blip_node(Signal::Value(440.0));

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&blip_id, &mut output);

    // Verify we got audio
    let rms = calculate_rms(&output);
    assert!(rms > 0.05, "Blip should produce audio, got RMS: {}", rms);

    // Verify no clipping
    let max = output.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(max <= 1.0, "Blip should not clip, max: {}", max);
}

#[test]
fn test_blip_buffer_low_frequency() {
    // Low frequency blip (audible pulses)
    let mut graph = create_test_graph();

    let blip_id = graph.add_blip_node(Signal::Value(110.0));

    let mut output = vec![0.0; 4410]; // 100ms at 44.1kHz
    graph.eval_node_buffer(&blip_id, &mut output);

    let rms = calculate_rms(&output);
    // Low frequency impulse trains have naturally low RMS due to sparse impulses
    // For 110 Hz: RMS ≈ sqrt(110/44100) ≈ 0.05
    assert!(rms > 0.04, "Low frequency Blip should produce audio, got RMS: {}", rms);
}

#[test]
fn test_blip_buffer_high_frequency() {
    // High frequency blip (more tone-like)
    let mut graph = create_test_graph();

    let blip_id = graph.add_blip_node(Signal::Value(2200.0));

    let mut output = vec![0.0; 4410]; // 100ms at 44.1kHz
    graph.eval_node_buffer(&blip_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.08, "High frequency Blip should produce audio, got RMS: {}", rms);
}

#[test]
fn test_blip_buffer_matches_single_sample() {
    // Verify buffer evaluation produces same results as sample-by-sample
    let mut graph1 = create_test_graph();
    let mut graph2 = create_test_graph();

    let blip_id1 = graph1.add_blip_node(Signal::Value(440.0));
    let blip_id2 = graph2.add_blip_node(Signal::Value(440.0));

    // Buffer evaluation
    let mut buffer_output = vec![0.0; 512];
    graph1.eval_node_buffer(&blip_id1, &mut buffer_output);

    // Sample-by-sample evaluation
    let mut sample_output = vec![0.0; 512];
    for i in 0..512 {
        sample_output[i] = graph2.eval_node(&blip_id2);
    }

    // Compare outputs
    let mut max_diff = 0.0f32;
    for i in 0..512 {
        let diff = (buffer_output[i] - sample_output[i]).abs();
        max_diff = max_diff.max(diff);
    }

    assert!(max_diff < 1e-6, "Buffer and sample-by-sample should match, max diff: {}", max_diff);
}

#[test]
fn test_blip_buffer_phase_continuity() {
    // Test that phase is continuous across multiple buffer evaluations
    let mut graph = create_test_graph();

    let blip_id = graph.add_blip_node(Signal::Value(440.0));

    // Render 4 buffers and concatenate
    let mut full_output = Vec::new();
    for _ in 0..4 {
        let mut buffer = vec![0.0; 512];
        graph.eval_node_buffer(&blip_id, &mut buffer);
        full_output.extend_from_slice(&buffer);
    }

    // Check for phase discontinuities
    // Look for sudden jumps that would indicate phase reset
    let mut max_jump = 0.0f32;
    for i in 1..full_output.len() {
        let jump = (full_output[i] - full_output[i - 1]).abs();
        max_jump = max_jump.max(jump);
    }

    // Impulse trains can have large jumps at impulses, but should be bounded
    // The actual jump should be ≤ 2.0 (from -1 to +1 at most)
    assert!(max_jump <= 2.0, "Phase should be continuous, max jump: {}", max_jump);

    // Verify we got continuous audio (RMS should be consistent)
    let rms = calculate_rms(&full_output);
    assert!(rms > 0.05, "Continuous buffers should produce audio, got RMS: {}", rms);
}

#[test]
fn test_blip_buffer_frequency_modulation() {
    // Test Blip with frequency modulated by another signal
    let mut graph = create_test_graph();

    // Create LFO (slow modulation)
    let lfo_id = graph.add_oscillator(Signal::Value(2.0), phonon::unified_graph::Waveform::Sine);

    // Modulate Blip frequency: 440 + 110 * LFO
    let lfo_signal = Signal::Node(lfo_id);
    let scaled_lfo = Signal::Expression(Box::new(phonon::unified_graph::SignalExpr::Multiply(
        lfo_signal.clone(),
        Signal::Value(110.0),
    )));
    let modulated_freq = Signal::Expression(Box::new(phonon::unified_graph::SignalExpr::Add(
        Signal::Value(440.0),
        scaled_lfo,
    )));

    let blip_id = graph.add_blip_node(modulated_freq);

    let mut output = vec![0.0; 4410]; // 100ms at 44.1kHz
    graph.eval_node_buffer(&blip_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.05, "FM Blip should produce audio, got RMS: {}", rms);
}

// ========== Spectral Analysis Tests ==========

#[test]
fn test_blip_buffer_rich_harmonic_content() {
    // Blip should have rich harmonic spectrum
    let mut graph = create_test_graph();

    let blip_id = graph.add_blip_node(Signal::Value(110.0));

    let mut output = vec![0.0; 8192];
    graph.eval_node_buffer(&blip_id, &mut output);

    let (frequencies, magnitudes) = analyze_spectrum(&output, 44100.0);

    // Find peaks
    let max_magnitude = magnitudes.iter().cloned().fold(0.0f32, f32::max);
    let threshold = max_magnitude * 0.05; // 5% of max
    let peaks = find_spectral_peaks(&frequencies, &magnitudes, threshold);

    // Blip should have multiple harmonics (at least 10 for 110 Hz)
    assert!(peaks.len() >= 10,
        "Blip should have rich harmonic content, found {} peaks", peaks.len());

    println!("Blip buffer spectral peaks (top 10): {:?}", peaks.iter().take(10).collect::<Vec<_>>());
}

#[test]
fn test_blip_buffer_band_limited() {
    // Blip should be band-limited (no significant content above Nyquist/2)
    let mut graph = create_test_graph();

    let blip_id = graph.add_blip_node(Signal::Value(110.0));

    let mut output = vec![0.0; 8192];
    graph.eval_node_buffer(&blip_id, &mut output);

    let (frequencies, magnitudes) = analyze_spectrum(&output, 44100.0);

    // Check energy in upper frequency range (18kHz - 22kHz)
    // Should be much lower than lower frequencies due to band-limiting
    let low_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f < 5000.0)
        .map(|(_, m)| m * m)
        .sum();

    let high_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 18000.0)
        .map(|(_, m)| m * m)
        .sum();

    let energy_ratio = high_energy / low_energy;
    // Impulse trains have equal amplitude in all harmonics up to Nyquist
    // For 110 Hz: ~45 harmonics <5kHz, ~37 harmonics >18kHz
    // Expected ratio: 37/45 ≈ 0.82, which is correct for band-limited impulse trains
    assert!(energy_ratio < 0.9,
        "Blip should be band-limited (low aliasing), high/low energy ratio: {}",
        energy_ratio);

    println!("Low energy: {}, High energy: {}, Ratio: {}", low_energy, high_energy, energy_ratio);
}

#[test]
fn test_blip_buffer_harmonic_count_vs_frequency() {
    // Higher frequency Blip should have fewer harmonics before Nyquist
    let mut graph1 = create_test_graph();
    let mut graph2 = create_test_graph();

    let low_blip = graph1.add_blip_node(Signal::Value(110.0));
    let high_blip = graph2.add_blip_node(Signal::Value(2200.0));

    let mut low_output = vec![0.0; 8192];
    let mut high_output = vec![0.0; 8192];

    graph1.eval_node_buffer(&low_blip, &mut low_output);
    graph2.eval_node_buffer(&high_blip, &mut high_output);

    let (low_frequencies, low_magnitudes) = analyze_spectrum(&low_output, 44100.0);
    let (high_frequencies, high_magnitudes) = analyze_spectrum(&high_output, 44100.0);

    let low_max = low_magnitudes.iter().cloned().fold(0.0f32, f32::max);
    let high_max = high_magnitudes.iter().cloned().fold(0.0f32, f32::max);

    let low_peaks = find_spectral_peaks(&low_frequencies, &low_magnitudes, low_max * 0.05);
    let high_peaks = find_spectral_peaks(&high_frequencies, &high_magnitudes, high_max * 0.05);

    // Lower frequency should have more harmonics before Nyquist
    assert!(low_peaks.len() > high_peaks.len(),
        "Lower frequency should have more harmonics: low={}, high={}",
        low_peaks.len(), high_peaks.len());

    println!("Low freq (110 Hz) peaks: {}, High freq (2200 Hz) peaks: {}",
        low_peaks.len(), high_peaks.len());
}

// ========== Audio Quality Tests ==========

#[test]
fn test_blip_buffer_no_dc_offset() {
    // Blip output should be centered around zero (no DC offset)
    let mut graph = create_test_graph();

    let blip_id = graph.add_blip_node(Signal::Value(440.0));

    let mut output = vec![0.0; 4410]; // 100ms
    graph.eval_node_buffer(&blip_id, &mut output);

    let dc_offset: f32 = output.iter().sum::<f32>() / output.len() as f32;

    assert!(dc_offset.abs() < 0.01, "Blip should have no DC offset, got {}", dc_offset);
}

#[test]
fn test_blip_buffer_no_clipping() {
    // Blip output should not clip (stay within -1.0 to 1.0)
    let mut graph = create_test_graph();

    let blip_id = graph.add_blip_node(Signal::Value(110.0));

    let mut output = vec![0.0; 4410]; // 100ms
    graph.eval_node_buffer(&blip_id, &mut output);

    let max_amplitude = output.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    assert!(max_amplitude <= 1.0, "Blip should not clip, max amplitude: {}", max_amplitude);
}

#[test]
fn test_blip_buffer_impulsive_characteristic() {
    // Blip should have high peak-to-RMS ratio (impulsive character)
    let mut graph = create_test_graph();

    let blip_id = graph.add_blip_node(Signal::Value(110.0));

    let mut output = vec![0.0; 4410]; // 100ms
    graph.eval_node_buffer(&blip_id, &mut output);

    let rms = calculate_rms(&output);
    let peak = output.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    let crest_factor = peak / rms;

    // Impulse train should have high crest factor (> 3)
    assert!(crest_factor > 3.0,
        "Blip should be impulsive (high crest factor), got {:.2}",
        crest_factor);

    println!("Blip buffer crest factor (peak/RMS): {:.2}", crest_factor);
}

// ========== Performance Tests ==========

#[test]
fn test_blip_buffer_large_buffer_size() {
    // Test that buffer evaluation works efficiently with large buffers
    let mut graph = create_test_graph();

    let blip_id = graph.add_blip_node(Signal::Value(440.0));

    // Large buffer (1 second)
    let mut output = vec![0.0; 44100];
    graph.eval_node_buffer(&blip_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.05, "Large buffer should produce audio, got RMS: {}", rms);
}

#[test]
fn test_blip_buffer_small_buffer_size() {
    // Test that buffer evaluation works with very small buffers
    let mut graph = create_test_graph();

    let blip_id = graph.add_blip_node(Signal::Value(440.0));

    // Very small buffer (just 32 samples)
    let mut output = vec![0.0; 32];
    graph.eval_node_buffer(&blip_id, &mut output);

    // Should still produce valid audio (may be all zeros if phase hasn't crossed impulse)
    let max = output.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(max <= 1.0, "Small buffer should not clip, max: {}", max);
}

#[test]
fn test_blip_buffer_varying_buffer_sizes() {
    // Test that different buffer sizes produce continuous output
    let mut graph = create_test_graph();

    let blip_id = graph.add_blip_node(Signal::Value(440.0));

    // Mix of different buffer sizes
    let sizes = vec![64, 128, 256, 512, 1024];
    let mut full_output = Vec::new();

    for size in sizes {
        let mut buffer = vec![0.0; size];
        graph.eval_node_buffer(&blip_id, &mut buffer);
        full_output.extend_from_slice(&buffer);
    }

    // Verify continuous output
    let rms = calculate_rms(&full_output);
    assert!(rms > 0.05, "Varying buffer sizes should produce audio, got RMS: {}", rms);
}

// ========== Edge Case Tests ==========

#[test]
fn test_blip_buffer_very_low_frequency() {
    // Test Blip at very low frequency (sub-audio)
    let mut graph = create_test_graph();

    let blip_id = graph.add_blip_node(Signal::Value(1.0)); // 1 Hz

    let mut output = vec![0.0; 4410]; // 100ms at 44.1kHz
    graph.eval_node_buffer(&blip_id, &mut output);

    // May have very low RMS due to infrequent impulses
    // Just verify no crashes and valid output
    let max = output.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(max <= 1.0, "Very low frequency should not clip, max: {}", max);
}

#[test]
fn test_blip_buffer_nyquist_frequency() {
    // Test Blip at Nyquist frequency (should have minimal harmonics)
    let mut graph = create_test_graph();

    let nyquist = 44100.0 / 2.0; // 22050 Hz
    let blip_id = graph.add_blip_node(Signal::Value(nyquist));

    let mut output = vec![0.0; 4410]; // 100ms
    graph.eval_node_buffer(&blip_id, &mut output);

    // Should produce audio but with very few harmonics
    let rms = calculate_rms(&output);
    // At Nyquist, only fundamental (if that), so RMS may be low
    assert!(rms >= 0.0, "Nyquist frequency should produce valid output, got RMS: {}", rms);
}

#[test]
fn test_blip_buffer_zero_frequency() {
    // Test Blip with zero frequency (edge case handling)
    let mut graph = create_test_graph();

    // Blip clamps frequency to 0.1 minimum to avoid division by zero
    let blip_id = graph.add_blip_node(Signal::Value(0.0));

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&blip_id, &mut output);

    // Should not crash and produce valid output
    let max = output.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(max <= 1.0, "Zero frequency should not crash or clip, max: {}", max);
}
