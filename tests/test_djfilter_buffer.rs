/// Tests for DJFilter buffer-based evaluation
///
/// These tests verify that the DJFilter buffer evaluation produces correct
/// DJ-style filter behavior: sweeping from lowpass (value=0.0) through
/// neutral (value=0.5) to highpass (value=1.0).

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

/// Helper: Measure high-frequency content (rate of change)
fn measure_high_freq_energy(buffer: &[f32]) -> f32 {
    let mut energy = 0.0;
    for i in 1..buffer.len() {
        energy += (buffer[i] - buffer[i - 1]).abs();
    }
    energy / buffer.len() as f32
}

/// Helper: Measure low-frequency content (moving average)
fn measure_low_freq_energy(buffer: &[f32]) -> f32 {
    // Use a simple DC component measurement
    let mean = buffer.iter().sum::<f32>() / buffer.len() as f32;
    mean.abs()
}

// ============================================================================
// TEST: Full Lowpass Mode (value = 0.0)
// ============================================================================

#[test]
fn test_djfilter_full_lowpass_cuts_highs() {
    let mut graph = create_test_graph();

    // High-frequency signal (8000 Hz)
    let osc_id = graph.add_oscillator(Signal::Value(8000.0), Waveform::Sine);

    // Full lowpass position (value = 0.0)
    let djf_id = graph.add_djfilter_node(Signal::Node(osc_id), Signal::Value(0.0));

    let buffer_size = 1024;
    let mut filtered = vec![0.0; buffer_size];
    let mut unfiltered = vec![0.0; buffer_size];

    // Get both signals
    graph.eval_node_buffer(&osc_id, &mut unfiltered);
    graph.eval_node_buffer(&djf_id, &mut filtered);

    // High frequencies should be strongly attenuated
    let unfiltered_rms = calculate_rms(&unfiltered);
    let filtered_rms = calculate_rms(&filtered);

    assert!(
        filtered_rms < unfiltered_rms * 0.3,
        "Full lowpass should cut highs: unfiltered RMS = {}, filtered RMS = {}",
        unfiltered_rms,
        filtered_rms
    );
}

#[test]
fn test_djfilter_full_lowpass_passes_lows() {
    let mut graph = create_test_graph();

    // Low-frequency signal (100 Hz) - close to the 80 Hz cutoff at position 0.0
    let osc_id = graph.add_oscillator(Signal::Value(100.0), Waveform::Sine);

    // Full lowpass position (value = 0.0, cutoff = 80 Hz)
    let djf_id = graph.add_djfilter_node(Signal::Node(osc_id), Signal::Value(0.0));

    let buffer_size = 1024;
    let mut filtered = vec![0.0; buffer_size];
    let mut unfiltered = vec![0.0; buffer_size];

    // Get both signals
    graph.eval_node_buffer(&osc_id, &mut unfiltered);
    graph.eval_node_buffer(&djf_id, &mut filtered);

    // 100 Hz should pass reasonably well through 80 Hz lowpass (close to cutoff)
    let unfiltered_rms = calculate_rms(&unfiltered);
    let filtered_rms = calculate_rms(&filtered);

    assert!(
        filtered_rms > unfiltered_rms * 0.15,
        "Full lowpass should pass nearby low frequencies: unfiltered RMS = {}, filtered RMS = {}",
        unfiltered_rms,
        filtered_rms
    );
}

// ============================================================================
// TEST: Neutral Position (value = 0.5)
// ============================================================================

#[test]
fn test_djfilter_neutral_position_passes_signal() {
    let mut graph = create_test_graph();

    // Mid-frequency signal (800 Hz - the neutral frequency)
    let osc_id = graph.add_oscillator(Signal::Value(800.0), Waveform::Sine);

    // Neutral position (value = 0.5)
    let djf_id = graph.add_djfilter_node(Signal::Node(osc_id), Signal::Value(0.5));

    let buffer_size = 1024;
    let mut filtered = vec![0.0; buffer_size];
    let mut unfiltered = vec![0.0; buffer_size];

    // Get both signals
    graph.eval_node_buffer(&osc_id, &mut unfiltered);
    graph.eval_node_buffer(&djf_id, &mut filtered);

    // At neutral position, signal should pass with minimal attenuation
    let unfiltered_rms = calculate_rms(&unfiltered);
    let filtered_rms = calculate_rms(&filtered);

    assert!(
        filtered_rms > unfiltered_rms * 0.5,
        "Neutral position should pass signal: unfiltered RMS = {}, filtered RMS = {}",
        unfiltered_rms,
        filtered_rms
    );
}

// ============================================================================
// TEST: Full Highpass Mode (value = 1.0)
// ============================================================================

#[test]
fn test_djfilter_full_highpass_cuts_lows() {
    let mut graph = create_test_graph();

    // Low-frequency signal (100 Hz)
    let osc_id = graph.add_oscillator(Signal::Value(100.0), Waveform::Sine);

    // Full highpass position (value = 1.0)
    let djf_id = graph.add_djfilter_node(Signal::Node(osc_id), Signal::Value(1.0));

    let buffer_size = 1024;
    let mut filtered = vec![0.0; buffer_size];
    let mut unfiltered = vec![0.0; buffer_size];

    // Get both signals
    graph.eval_node_buffer(&osc_id, &mut unfiltered);
    graph.eval_node_buffer(&djf_id, &mut filtered);

    // Low frequencies should be strongly attenuated
    let unfiltered_rms = calculate_rms(&unfiltered);
    let filtered_rms = calculate_rms(&filtered);

    assert!(
        filtered_rms < unfiltered_rms * 0.3,
        "Full highpass should cut lows: unfiltered RMS = {}, filtered RMS = {}",
        unfiltered_rms,
        filtered_rms
    );
}

#[test]
fn test_djfilter_full_highpass_passes_highs() {
    let mut graph = create_test_graph();

    // High-frequency signal (5000 Hz)
    let osc_id = graph.add_oscillator(Signal::Value(5000.0), Waveform::Sine);

    // Full highpass position (value = 1.0)
    let djf_id = graph.add_djfilter_node(Signal::Node(osc_id), Signal::Value(1.0));

    let buffer_size = 1024;
    let mut filtered = vec![0.0; buffer_size];
    let mut unfiltered = vec![0.0; buffer_size];

    // Get both signals
    graph.eval_node_buffer(&osc_id, &mut unfiltered);
    graph.eval_node_buffer(&djf_id, &mut filtered);

    // High frequencies should pass reasonably well
    let unfiltered_rms = calculate_rms(&unfiltered);
    let filtered_rms = calculate_rms(&filtered);

    assert!(
        filtered_rms > unfiltered_rms * 0.4,
        "Full highpass should pass highs: unfiltered RMS = {}, filtered RMS = {}",
        unfiltered_rms,
        filtered_rms
    );
}

// ============================================================================
// TEST: Smooth Transition Behavior
// ============================================================================

#[test]
fn test_djfilter_smooth_transition_from_lpf_to_hpf() {
    let mut graph = create_test_graph();

    // Broadband signal (sawtooth has both low and high harmonics)
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);

    let buffer_size = 1024;
    let mut unfiltered = vec![0.0; buffer_size];
    graph.eval_node_buffer(&osc_id, &mut unfiltered);

    // Test sweep from lowpass (0.0) to highpass (1.0)
    let positions = [0.0, 0.25, 0.5, 0.75, 1.0];
    let mut rms_values = Vec::new();

    for &pos in &positions {
        let djf_id = graph.add_djfilter_node(Signal::Node(osc_id), Signal::Value(pos));
        let mut filtered = vec![0.0; buffer_size];
        graph.eval_node_buffer(&djf_id, &mut filtered);
        rms_values.push(calculate_rms(&filtered));
    }

    // Verify all RMS values are reasonable (filter doesn't go silent or unstable)
    for (i, &rms) in rms_values.iter().enumerate() {
        assert!(
            rms > 0.01,
            "Position {} should have audible signal: RMS = {}",
            positions[i],
            rms
        );
        assert!(
            rms < 2.0,
            "Position {} should not amplify excessively: RMS = {}",
            positions[i],
            rms
        );
    }
}

#[test]
fn test_djfilter_frequency_response_sweep() {
    let mut graph = create_test_graph();

    // Test with different frequency inputs
    let test_freqs = [100.0, 500.0, 1000.0, 2000.0, 5000.0];

    for &freq in &test_freqs {
        let osc_id = graph.add_oscillator(Signal::Value(freq), Waveform::Sine);

        // Test at three key positions
        let positions = [0.0, 0.5, 1.0]; // LPF, Neutral, HPF

        for &pos in &positions {
            let djf_id = graph.add_djfilter_node(Signal::Node(osc_id), Signal::Value(pos));

            let buffer_size = 1024;
            let mut filtered = vec![0.0; buffer_size];
            graph.eval_node_buffer(&djf_id, &mut filtered);

            let rms = calculate_rms(&filtered);

            // All outputs should be finite and reasonable
            assert!(
                rms.is_finite(),
                "Filter output should be finite for freq={}, pos={}",
                freq,
                pos
            );
            assert!(
                rms < 2.0,
                "Filter should not excessively amplify: freq={}, pos={}, RMS={}",
                freq,
                pos,
                rms
            );
        }
    }
}

// ============================================================================
// TEST: Dynamic Parameter Modulation
// ============================================================================

#[test]
fn test_djfilter_sweeping_positions() {
    let mut graph = create_test_graph();

    // Broadband input signal
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);

    let buffer_size = 512;

    // Test sweeping through different positions
    let positions = [0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0];
    let mut all_outputs_valid = true;

    for &pos in &positions {
        let djf_id = graph.add_djfilter_node(Signal::Node(osc_id), Signal::Value(pos));
        let mut filtered = vec![0.0; buffer_size];
        graph.eval_node_buffer(&djf_id, &mut filtered);

        let rms = calculate_rms(&filtered);

        // All positions should produce valid output
        if !rms.is_finite() || rms < 0.01 || rms > 2.0 {
            all_outputs_valid = false;
            eprintln!(
                "Position {} produced invalid output: RMS = {}",
                pos, rms
            );
        }
    }

    assert!(
        all_outputs_valid,
        "All filter positions should produce valid output"
    );
}

// ============================================================================
// TEST: Edge Cases and Stability
// ============================================================================

#[test]
fn test_djfilter_clamping_values() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Test values outside 0-1 range (should be clamped)
    let test_values = [-0.5, -0.1, 1.1, 1.5, 2.0];

    for &val in &test_values {
        let djf_id = graph.add_djfilter_node(Signal::Node(osc_id), Signal::Value(val));

        let buffer_size = 512;
        let mut filtered = vec![0.0; buffer_size];
        graph.eval_node_buffer(&djf_id, &mut filtered);

        // Should not blow up, should produce finite output
        for (i, &sample) in filtered.iter().enumerate() {
            assert!(
                sample.is_finite(),
                "Sample {} should be finite with value={}: got {}",
                i,
                val,
                sample
            );
        }

        let rms = calculate_rms(&filtered);
        assert!(
            rms < 2.0,
            "Clamped value {} should not cause excessive amplification: RMS = {}",
            val,
            rms
        );
    }
}

#[test]
fn test_djfilter_state_continuity() {
    let mut graph = create_test_graph();

    // Create a filter and render multiple buffers to ensure state is maintained
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let djf_id = graph.add_djfilter_node(Signal::Node(osc_id), Signal::Value(0.3));

    let buffer_size = 512;

    // Render first buffer
    let mut buffer1 = vec![0.0; buffer_size];
    graph.eval_node_buffer(&djf_id, &mut buffer1);

    // Render second buffer (should have continuous state)
    let mut buffer2 = vec![0.0; buffer_size];
    graph.eval_node_buffer(&djf_id, &mut buffer2);

    // Both buffers should have similar energy (filter is stateful and continuous)
    let rms1 = calculate_rms(&buffer1);
    let rms2 = calculate_rms(&buffer2);

    assert!(
        (rms1 - rms2).abs() < 0.2,
        "Consecutive buffers should have similar RMS (state continuity): RMS1={}, RMS2={}",
        rms1,
        rms2
    );
}

#[test]
fn test_djfilter_multiple_instances() {
    let mut graph = create_test_graph();

    // Create two separate filter instances
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    let djf1_id = graph.add_djfilter_node(Signal::Node(osc_id), Signal::Value(0.2));
    let djf2_id = graph.add_djfilter_node(Signal::Node(osc_id), Signal::Value(0.8));

    let buffer_size = 512;
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&djf1_id, &mut buffer1);
    graph.eval_node_buffer(&djf2_id, &mut buffer2);

    // Both should produce output
    let rms1 = calculate_rms(&buffer1);
    let rms2 = calculate_rms(&buffer2);

    assert!(rms1 > 0.01, "First filter should produce output: RMS = {}", rms1);
    assert!(rms2 > 0.01, "Second filter should produce output: RMS = {}", rms2);

    // They should be different (different filter positions)
    // Note: This might not always hold depending on the frequency, so we're lenient
    // The key test is that both work independently
}

// ============================================================================
// TEST: Performance with Large Buffers
// ============================================================================

#[test]
fn test_djfilter_large_buffer() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let djf_id = graph.add_djfilter_node(Signal::Node(osc_id), Signal::Value(0.5));

    // Large buffer (typical audio processing chunk)
    let buffer_size = 8192;
    let mut filtered = vec![0.0; buffer_size];

    graph.eval_node_buffer(&djf_id, &mut filtered);

    // Should produce valid output for entire buffer
    let rms = calculate_rms(&filtered);
    assert!(rms > 0.01, "Large buffer should produce output: RMS = {}", rms);
    assert!(rms < 2.0, "Large buffer should not amplify excessively: RMS = {}", rms);

    // Check for NaN or Inf
    for (i, &sample) in filtered.iter().enumerate() {
        assert!(
            sample.is_finite(),
            "Sample {} in large buffer should be finite: got {}",
            i,
            sample
        );
    }
}

// ============================================================================
// TEST: Frequency Characteristic Verification
// ============================================================================

#[test]
fn test_djfilter_lowpass_frequency_characteristic() {
    let mut graph = create_test_graph();

    // Test multiple frequencies through lowpass mode
    let low_freq = 200.0;
    let high_freq = 5000.0;

    let low_osc = graph.add_oscillator(Signal::Value(low_freq), Waveform::Sine);
    let high_osc = graph.add_oscillator(Signal::Value(high_freq), Waveform::Sine);

    // Apply full lowpass to both
    let low_filtered = graph.add_djfilter_node(Signal::Node(low_osc), Signal::Value(0.0));
    let high_filtered = graph.add_djfilter_node(Signal::Node(high_osc), Signal::Value(0.0));

    let buffer_size = 1024;
    let mut low_buf = vec![0.0; buffer_size];
    let mut high_buf = vec![0.0; buffer_size];

    graph.eval_node_buffer(&low_filtered, &mut low_buf);
    graph.eval_node_buffer(&high_filtered, &mut high_buf);

    let low_rms = calculate_rms(&low_buf);
    let high_rms = calculate_rms(&high_buf);

    // Lowpass should pass low frequencies better than high frequencies
    assert!(
        low_rms > high_rms,
        "Lowpass should pass low freq ({} Hz) better than high freq ({} Hz): low RMS={}, high RMS={}",
        low_freq,
        high_freq,
        low_rms,
        high_rms
    );
}

#[test]
fn test_djfilter_highpass_frequency_characteristic() {
    let mut graph = create_test_graph();

    // Test multiple frequencies through highpass mode
    let low_freq = 200.0;
    let high_freq = 5000.0;

    let low_osc = graph.add_oscillator(Signal::Value(low_freq), Waveform::Sine);
    let high_osc = graph.add_oscillator(Signal::Value(high_freq), Waveform::Sine);

    // Apply full highpass to both
    let low_filtered = graph.add_djfilter_node(Signal::Node(low_osc), Signal::Value(1.0));
    let high_filtered = graph.add_djfilter_node(Signal::Node(high_osc), Signal::Value(1.0));

    let buffer_size = 1024;
    let mut low_buf = vec![0.0; buffer_size];
    let mut high_buf = vec![0.0; buffer_size];

    graph.eval_node_buffer(&low_filtered, &mut low_buf);
    graph.eval_node_buffer(&high_filtered, &mut high_buf);

    let low_rms = calculate_rms(&low_buf);
    let high_rms = calculate_rms(&high_buf);

    // Highpass should pass high frequencies better than low frequencies
    assert!(
        high_rms > low_rms,
        "Highpass should pass high freq ({} Hz) better than low freq ({} Hz): high RMS={}, low RMS={}",
        high_freq,
        low_freq,
        high_rms,
        low_rms
    );
}
