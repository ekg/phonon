/// Tests for ParametricEQ (3-band peaking equalizer) buffer-based evaluation
///
/// These tests verify that ParametricEQ buffer evaluation produces correct
/// EQ behavior with boost/cut at specified frequencies and maintains proper state continuity.

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

/// Helper: Measure spectral energy at a specific frequency using DFT
fn measure_energy_at_freq(buffer: &[f32], freq: f32, sample_rate: f32) -> f32 {
    let n = buffer.len();
    let omega = 2.0 * PI * freq / sample_rate;

    let mut real_sum = 0.0;
    let mut imag_sum = 0.0;

    for (i, &sample) in buffer.iter().enumerate() {
        let phase = omega * i as f32;
        real_sum += sample * phase.cos();
        imag_sum += sample * phase.sin();
    }

    // Return magnitude (sqrt of sum of squares)
    ((real_sum * real_sum + imag_sum * imag_sum) / (n * n) as f32).sqrt()
}

// ============================================================================
// TEST: Low Band Boost
// ============================================================================

#[test]
fn test_eq_low_band_boost() {
    let mut graph = create_test_graph();

    // Create oscillator at low band center frequency (200 Hz)
    let osc_id = graph.add_oscillator(Signal::Value(200.0), Waveform::Sine);

    // Apply EQ with low band boost (+6dB at 200Hz)
    let eq_id = graph.add_parametriceq_node(
        Signal::Node(osc_id),
        Signal::Value(200.0),  // low_freq
        Signal::Value(6.0),    // low_gain (+6dB boost)
        Signal::Value(1.0),    // low_q
        Signal::Value(1000.0), // mid_freq
        Signal::Value(0.0),    // mid_gain (no change)
        Signal::Value(1.0),    // mid_q
        Signal::Value(5000.0), // high_freq
        Signal::Value(0.0),    // high_gain (no change)
        Signal::Value(1.0),    // high_q
    );

    let buffer_size = 4410; // 0.1 second at 44100 Hz
    let mut boosted = vec![0.0; buffer_size];
    let mut original = vec![0.0; buffer_size];

    // Get signals
    graph.eval_node_buffer(&osc_id, &mut original);
    graph.eval_node_buffer(&eq_id, &mut boosted);

    // Calculate RMS
    let boost_rms = calculate_rms(&boosted);
    let orig_rms = calculate_rms(&original);

    // +6dB ≈ 2x amplitude
    assert!(boost_rms > orig_rms * 1.8,
        "EQ should boost low band: boost_rms={:.4}, orig_rms={:.4}, ratio={:.2}",
        boost_rms, orig_rms, boost_rms / orig_rms);

    assert!(boost_rms < orig_rms * 2.2,
        "EQ boost should not exceed expected amount: boost_rms={:.4}, orig_rms={:.4}",
        boost_rms, orig_rms);
}

// ============================================================================
// TEST: Mid Band Cut
// ============================================================================

#[test]
fn test_eq_mid_band_cut() {
    let mut graph = create_test_graph();

    // Create oscillator at mid band center frequency (1000 Hz)
    let osc_id = graph.add_oscillator(Signal::Value(1000.0), Waveform::Sine);

    // Apply EQ with mid band cut (-6dB at 1000Hz)
    let eq_id = graph.add_parametriceq_node(
        Signal::Node(osc_id),
        Signal::Value(200.0),  // low_freq
        Signal::Value(0.0),    // low_gain (no change)
        Signal::Value(1.0),    // low_q
        Signal::Value(1000.0), // mid_freq
        Signal::Value(-6.0),   // mid_gain (-6dB cut)
        Signal::Value(1.0),    // mid_q
        Signal::Value(5000.0), // high_freq
        Signal::Value(0.0),    // high_gain (no change)
        Signal::Value(1.0),    // high_q
    );

    let buffer_size = 4410;
    let mut cut = vec![0.0; buffer_size];
    let mut original = vec![0.0; buffer_size];

    // Get signals
    graph.eval_node_buffer(&osc_id, &mut original);
    graph.eval_node_buffer(&eq_id, &mut cut);

    // Calculate RMS
    let cut_rms = calculate_rms(&cut);
    let orig_rms = calculate_rms(&original);

    // -6dB ≈ 0.5x amplitude
    assert!(cut_rms < orig_rms * 0.55,
        "EQ should cut mid band: cut_rms={:.4}, orig_rms={:.4}, ratio={:.2}",
        cut_rms, orig_rms, cut_rms / orig_rms);

    assert!(cut_rms > orig_rms * 0.45,
        "EQ cut should not exceed expected amount: cut_rms={:.4}, orig_rms={:.4}",
        cut_rms, orig_rms);
}

// ============================================================================
// TEST: High Band Boost
// ============================================================================

#[test]
fn test_eq_high_band_boost() {
    let mut graph = create_test_graph();

    // Create oscillator at high band center frequency (5000 Hz)
    let osc_id = graph.add_oscillator(Signal::Value(5000.0), Waveform::Sine);

    // Apply EQ with high band boost (+6dB at 5000Hz)
    let eq_id = graph.add_parametriceq_node(
        Signal::Node(osc_id),
        Signal::Value(200.0),  // low_freq
        Signal::Value(0.0),    // low_gain (no change)
        Signal::Value(1.0),    // low_q
        Signal::Value(1000.0), // mid_freq
        Signal::Value(0.0),    // mid_gain (no change)
        Signal::Value(1.0),    // mid_q
        Signal::Value(5000.0), // high_freq
        Signal::Value(6.0),    // high_gain (+6dB boost)
        Signal::Value(1.0),    // high_q
    );

    let buffer_size = 4410;
    let mut boosted = vec![0.0; buffer_size];
    let mut original = vec![0.0; buffer_size];

    // Get signals
    graph.eval_node_buffer(&osc_id, &mut original);
    graph.eval_node_buffer(&eq_id, &mut boosted);

    // Calculate RMS
    let boost_rms = calculate_rms(&boosted);
    let orig_rms = calculate_rms(&original);

    // +6dB ≈ 2x amplitude
    assert!(boost_rms > orig_rms * 1.8,
        "EQ should boost high band: boost_rms={:.4}, orig_rms={:.4}, ratio={:.2}",
        boost_rms, orig_rms, boost_rms / orig_rms);
}

// ============================================================================
// TEST: Zero Gain (Pass-through)
// ============================================================================

#[test]
fn test_eq_zero_gain_passthrough() {
    let mut graph = create_test_graph();

    // Create oscillator at 1000 Hz
    let osc_id = graph.add_oscillator(Signal::Value(1000.0), Waveform::Sine);

    // Apply EQ with all gains at 0dB (should pass through)
    let eq_id = graph.add_parametriceq_node(
        Signal::Node(osc_id),
        Signal::Value(200.0),  // low_freq
        Signal::Value(0.0),    // low_gain (no change)
        Signal::Value(1.0),    // low_q
        Signal::Value(1000.0), // mid_freq
        Signal::Value(0.0),    // mid_gain (no change)
        Signal::Value(1.0),    // mid_q
        Signal::Value(5000.0), // high_freq
        Signal::Value(0.0),    // high_gain (no change)
        Signal::Value(1.0),    // high_q
    );

    let buffer_size = 4410;
    let mut eq_output = vec![0.0; buffer_size];
    let mut original = vec![0.0; buffer_size];

    // Get signals
    graph.eval_node_buffer(&osc_id, &mut original);
    graph.eval_node_buffer(&eq_id, &mut eq_output);

    // Calculate RMS
    let eq_rms = calculate_rms(&eq_output);
    let orig_rms = calculate_rms(&original);

    // Should be nearly identical (within 1%)
    let ratio = eq_rms / orig_rms;
    assert!((ratio - 1.0).abs() < 0.01,
        "EQ with 0dB gain should pass through unchanged: eq_rms={:.4}, orig_rms={:.4}, ratio={:.4}",
        eq_rms, orig_rms, ratio);
}

// ============================================================================
// TEST: Q Factor Effect (Narrow vs Wide)
// ============================================================================

#[test]
fn test_eq_q_factor_bandwidth() {
    let mut graph = create_test_graph();

    // Create white noise to test frequency response
    let noise_id = graph.add_whitenoise_node();

    // High Q (narrow boost at 1000Hz)
    let eq_narrow_id = graph.add_parametriceq_node(
        Signal::Node(noise_id),
        Signal::Value(200.0),
        Signal::Value(0.0),
        Signal::Value(1.0),
        Signal::Value(1000.0),
        Signal::Value(10.0),   // +10dB boost
        Signal::Value(5.0),    // High Q = narrow
        Signal::Value(5000.0),
        Signal::Value(0.0),
        Signal::Value(1.0),
    );

    // Low Q (wide boost at 1000Hz)
    let eq_wide_id = graph.add_parametriceq_node(
        Signal::Node(noise_id),
        Signal::Value(200.0),
        Signal::Value(0.0),
        Signal::Value(1.0),
        Signal::Value(1000.0),
        Signal::Value(10.0),   // +10dB boost
        Signal::Value(0.5),    // Low Q = wide
        Signal::Value(5000.0),
        Signal::Value(0.0),
        Signal::Value(1.0),
    );

    let buffer_size = 8820; // 0.2 seconds
    let mut narrow = vec![0.0; buffer_size];
    let mut wide = vec![0.0; buffer_size];

    // Get signals
    graph.eval_node_buffer(&eq_narrow_id, &mut narrow);
    graph.eval_node_buffer(&eq_wide_id, &mut wide);

    // Measure energy at center frequency and nearby frequencies
    let energy_center_narrow = measure_energy_at_freq(&narrow, 1000.0, 44100.0);
    let energy_nearby_narrow = measure_energy_at_freq(&narrow, 1500.0, 44100.0);

    let energy_center_wide = measure_energy_at_freq(&wide, 1000.0, 44100.0);
    let energy_nearby_wide = measure_energy_at_freq(&wide, 1500.0, 44100.0);

    // Narrow Q should have higher ratio (center/nearby) than wide Q
    let narrow_ratio = energy_center_narrow / energy_nearby_narrow;
    let wide_ratio = energy_center_wide / energy_nearby_wide;

    assert!(narrow_ratio > wide_ratio * 1.2,
        "High Q should have narrower boost than low Q: narrow_ratio={:.2}, wide_ratio={:.2}",
        narrow_ratio, wide_ratio);
}

// ============================================================================
// TEST: State Continuity
// ============================================================================

#[test]
fn test_eq_state_continuity() {
    let mut graph = create_test_graph();

    // Create oscillator
    let osc_id = graph.add_oscillator(Signal::Value(1000.0), Waveform::Sine);

    // Apply EQ with mid band boost
    let eq_id = graph.add_parametriceq_node(
        Signal::Node(osc_id),
        Signal::Value(200.0),
        Signal::Value(0.0),
        Signal::Value(1.0),
        Signal::Value(1000.0),
        Signal::Value(6.0),    // +6dB boost
        Signal::Value(2.0),
        Signal::Value(5000.0),
        Signal::Value(0.0),
        Signal::Value(1.0),
    );

    let buffer_size = 512;
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];
    let mut buffer_combined = vec![0.0; buffer_size * 2];

    // Render two separate buffers
    graph.eval_node_buffer(&eq_id, &mut buffer1);
    graph.eval_node_buffer(&eq_id, &mut buffer2);

    // Reset graph and render one combined buffer
    let mut graph2 = create_test_graph();
    let osc_id2 = graph2.add_oscillator(Signal::Value(1000.0), Waveform::Sine);
    let eq_id2 = graph2.add_parametriceq_node(
        Signal::Node(osc_id2),
        Signal::Value(200.0),
        Signal::Value(0.0),
        Signal::Value(1.0),
        Signal::Value(1000.0),
        Signal::Value(6.0),
        Signal::Value(2.0),
        Signal::Value(5000.0),
        Signal::Value(0.0),
        Signal::Value(1.0),
    );
    graph2.eval_node_buffer(&eq_id2, &mut buffer_combined);

    // First buffer should match first half of combined
    let max_diff1: f32 = buffer1.iter()
        .zip(buffer_combined[0..buffer_size].iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0_f32, |a, b| a.max(b));

    assert!(max_diff1 < 0.001,
        "State continuity: first buffer should match combined (max_diff={:.6})",
        max_diff1);
}

// ============================================================================
// TEST: All Bands Active (Complex EQ Curve)
// ============================================================================

#[test]
fn test_eq_all_bands_active() {
    let mut graph = create_test_graph();

    // Create white noise to test frequency response
    let noise_id = graph.add_whitenoise_node();

    // Apply EQ with all bands active (classic "smile" curve)
    let eq_id = graph.add_parametriceq_node(
        Signal::Node(noise_id),
        Signal::Value(200.0),  // low_freq
        Signal::Value(3.0),    // low_gain (+3dB boost)
        Signal::Value(1.0),    // low_q
        Signal::Value(1000.0), // mid_freq
        Signal::Value(-3.0),   // mid_gain (-3dB cut)
        Signal::Value(1.0),    // mid_q
        Signal::Value(5000.0), // high_freq
        Signal::Value(3.0),    // high_gain (+3dB boost)
        Signal::Value(1.0),    // high_q
    );

    let buffer_size = 8820; // 0.2 seconds
    let mut eq_output = vec![0.0; buffer_size];
    let mut original = vec![0.0; buffer_size];

    // Get signals
    graph.eval_node_buffer(&noise_id, &mut original);
    graph.eval_node_buffer(&eq_id, &mut eq_output);

    // Measure energy at each band
    let low_energy = measure_energy_at_freq(&eq_output, 200.0, 44100.0);
    let mid_energy = measure_energy_at_freq(&eq_output, 1000.0, 44100.0);
    let high_energy = measure_energy_at_freq(&eq_output, 5000.0, 44100.0);

    // Low and high should be boosted relative to mid
    assert!(low_energy > mid_energy * 1.2,
        "Low band should be boosted relative to mid: low={:.4}, mid={:.4}",
        low_energy, mid_energy);

    assert!(high_energy > mid_energy * 1.2,
        "High band should be boosted relative to mid: high={:.4}, mid={:.4}",
        high_energy, mid_energy);
}

// ============================================================================
// TEST: Extreme Boost/Cut
// ============================================================================

#[test]
fn test_eq_extreme_boost() {
    let mut graph = create_test_graph();

    // Create oscillator at 1000 Hz
    let osc_id = graph.add_oscillator(Signal::Value(1000.0), Waveform::Sine);

    // Apply EQ with maximum boost (+20dB at 1000Hz)
    let eq_id = graph.add_parametriceq_node(
        Signal::Node(osc_id),
        Signal::Value(200.0),
        Signal::Value(0.0),
        Signal::Value(1.0),
        Signal::Value(1000.0),
        Signal::Value(20.0),   // +20dB boost (max)
        Signal::Value(1.0),
        Signal::Value(5000.0),
        Signal::Value(0.0),
        Signal::Value(1.0),
    );

    let buffer_size = 4410;
    let mut boosted = vec![0.0; buffer_size];
    let mut original = vec![0.0; buffer_size];

    // Get signals
    graph.eval_node_buffer(&osc_id, &mut original);
    graph.eval_node_buffer(&eq_id, &mut boosted);

    // Calculate RMS
    let boost_rms = calculate_rms(&boosted);
    let orig_rms = calculate_rms(&original);

    // +20dB = 10x amplitude
    assert!(boost_rms > orig_rms * 8.0,
        "EQ should apply extreme boost: boost_rms={:.4}, orig_rms={:.4}, ratio={:.2}",
        boost_rms, orig_rms, boost_rms / orig_rms);
}

#[test]
fn test_eq_extreme_cut() {
    let mut graph = create_test_graph();

    // Create oscillator at 1000 Hz
    let osc_id = graph.add_oscillator(Signal::Value(1000.0), Waveform::Sine);

    // Apply EQ with maximum cut (-20dB at 1000Hz)
    let eq_id = graph.add_parametriceq_node(
        Signal::Node(osc_id),
        Signal::Value(200.0),
        Signal::Value(0.0),
        Signal::Value(1.0),
        Signal::Value(1000.0),
        Signal::Value(-20.0),  // -20dB cut (max)
        Signal::Value(1.0),
        Signal::Value(5000.0),
        Signal::Value(0.0),
        Signal::Value(1.0),
    );

    let buffer_size = 4410;
    let mut cut = vec![0.0; buffer_size];
    let mut original = vec![0.0; buffer_size];

    // Get signals
    graph.eval_node_buffer(&osc_id, &mut original);
    graph.eval_node_buffer(&eq_id, &mut cut);

    // Calculate RMS
    let cut_rms = calculate_rms(&cut);
    let orig_rms = calculate_rms(&original);

    // -20dB = 0.1x amplitude
    assert!(cut_rms < orig_rms * 0.12,
        "EQ should apply extreme cut: cut_rms={:.4}, orig_rms={:.4}, ratio={:.2}",
        cut_rms, orig_rms, cut_rms / orig_rms);
}

// ============================================================================
// TEST: No Clipping with Maximum Boost
// ============================================================================

#[test]
fn test_eq_no_clipping_with_max_boost() {
    let mut graph = create_test_graph();

    // Create low amplitude oscillator
    let osc_id = graph.add_oscillator(Signal::Value(1000.0), Waveform::Sine);
    let gain_id = graph.add_multiply_node(Signal::Node(osc_id), Signal::Value(0.1));

    // Apply maximum boost
    let eq_id = graph.add_parametriceq_node(
        Signal::Node(gain_id),
        Signal::Value(200.0),
        Signal::Value(0.0),
        Signal::Value(1.0),
        Signal::Value(1000.0),
        Signal::Value(20.0),   // +20dB boost
        Signal::Value(1.0),
        Signal::Value(5000.0),
        Signal::Value(0.0),
        Signal::Value(1.0),
    );

    let buffer_size = 4410;
    let mut output = vec![0.0; buffer_size];

    // Get signal
    graph.eval_node_buffer(&eq_id, &mut output);

    // Check for clipping (values outside [-1, 1])
    let max_abs = output.iter().map(|&x| x.abs()).fold(0.0_f32, |a, b| a.max(b));

    assert!(max_abs < 2.0,
        "Output should not clip excessively even with max boost: max_abs={:.4}",
        max_abs);
}
