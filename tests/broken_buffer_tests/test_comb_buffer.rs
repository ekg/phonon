/// Tests for Comb filter buffer-based evaluation
///
/// These tests verify that Comb buffer evaluation produces correct
/// resonant feedback behavior for physical modeling and metallic sounds.

use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph, Waveform};

/// Helper: Create a test graph
fn create_test_graph() -> UnifiedSignalGraph {
    UnifiedSignalGraph::new(44100.0)
}

/// Calculate RMS of a signal
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_squares: f32 = buffer.iter().map(|x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Calculate RMS of a portion of the signal
fn calculate_rms_range(buffer: &[f32], start: usize, end: usize) -> f32 {
    let sum_squares: f32 = buffer[start..end].iter().map(|x| x * x).sum();
    (sum_squares / (end - start) as f32).sqrt()
}

/// Find peaks in a signal (simple onset detection)
fn find_peaks(buffer: &[f32], threshold: f32) -> Vec<usize> {
    let mut peaks = Vec::new();
    let mut in_peak = false;

    for i in 1..buffer.len() - 1 {
        let val = buffer[i].abs();
        if val > threshold && !in_peak {
            peaks.push(i);
            in_peak = true;
        } else if val < threshold * 0.5 {
            in_peak = false;
        }
    }

    peaks
}

/// Test 1: Comb filter creates resonance (sustains after input)
#[test]
fn test_comb_creates_resonance() {
    let mut graph = create_test_graph();

    // Create a brief impulse-like signal (short burst)
    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Comb filter with strong feedback at 100 Hz
    let comb = graph.add_comb_node(
        Signal::Node(osc),
        Signal::Value(100.0),  // Low frequency = longer delay
        Signal::Value(0.85),   // Strong feedback
    );

    let buffer_size = 8820; // 0.2 seconds
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&comb, &mut output);

    // The late portion should still have significant energy due to feedback
    let early_rms = calculate_rms_range(&output, 0, 2205);
    let late_rms = calculate_rms_range(&output, 6615, 8820);

    assert!(late_rms > 0.05, "Comb should sustain resonance in late portion: RMS={}", late_rms);
    assert!(late_rms > early_rms * 0.3, "Late resonance should be substantial compared to early: early={}, late={}", early_rms, late_rms);
}

/// Test 2: No feedback = no resonance
#[test]
fn test_comb_no_feedback() {
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Comb with zero feedback
    let comb = graph.add_comb_node(
        Signal::Node(osc),
        Signal::Value(100.0),
        Signal::Value(0.0),  // No feedback
    );

    let buffer_size = 8820;
    let mut output = vec![0.0; buffer_size];
    let mut osc_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&comb, &mut output);
    graph.eval_node_buffer(&osc, &mut osc_output);

    // With no feedback, output should be similar to input (just delayed by one sample)
    let rms_output = calculate_rms(&output);
    let rms_osc = calculate_rms(&osc_output);

    // Should be similar amplitude (no buildup)
    assert!((rms_output - rms_osc).abs() / rms_osc < 0.3,
        "No feedback should produce similar RMS: osc={}, comb={}", rms_osc, rms_output);
}

/// Test 3: Higher feedback = stronger resonance
#[test]
fn test_comb_feedback_strength() {
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(220.0), Waveform::Sine);

    // Low feedback
    let comb_low = graph.add_comb_node(
        Signal::Node(osc),
        Signal::Value(110.0),
        Signal::Value(0.3),
    );

    // High feedback
    let comb_high = graph.add_comb_node(
        Signal::Node(osc),
        Signal::Value(110.0),
        Signal::Value(0.85),
    );

    let buffer_size = 8820;
    let mut output_low = vec![0.0; buffer_size];
    let mut output_high = vec![0.0; buffer_size];

    graph.eval_node_buffer(&comb_low, &mut output_low);
    graph.eval_node_buffer(&comb_high, &mut output_high);

    let rms_low = calculate_rms(&output_low);
    let rms_high = calculate_rms(&output_high);

    // Higher feedback should produce louder output
    assert!(rms_high > rms_low * 1.5,
        "Higher feedback should produce stronger signal: low={}, high={}", rms_low, rms_high);
}

/// Test 4: Frequency determines delay time
#[test]
fn test_comb_frequency_effect() {
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(220.0), Waveform::Sine);

    // Low frequency = longer delay
    let comb_low = graph.add_comb_node(
        Signal::Node(osc),
        Signal::Value(55.0),   // Low freq = long delay
        Signal::Value(0.7),
    );

    // High frequency = shorter delay
    let comb_high = graph.add_comb_node(
        Signal::Node(osc),
        Signal::Value(880.0),  // High freq = short delay
        Signal::Value(0.7),
    );

    let buffer_size = 4410;
    let mut output_low = vec![0.0; buffer_size];
    let mut output_high = vec![0.0; buffer_size];

    graph.eval_node_buffer(&comb_low, &mut output_low);
    graph.eval_node_buffer(&comb_high, &mut output_high);

    // Both should produce output
    let rms_low = calculate_rms(&output_low);
    let rms_high = calculate_rms(&output_high);

    assert!(rms_low > 0.1, "Low frequency comb should produce output: {}", rms_low);
    assert!(rms_high > 0.1, "High frequency comb should produce output: {}", rms_high);
}

/// Test 5: State continuity (delay buffer persists across calls)
#[test]
fn test_comb_state_continuity() {
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(110.0), Waveform::Sine);
    let comb = graph.add_comb_node(
        Signal::Node(osc),
        Signal::Value(110.0),
        Signal::Value(0.8),
    );

    let buffer_size = 2205; // 0.05 seconds
    let mut output1 = vec![0.0; buffer_size];
    let mut output2 = vec![0.0; buffer_size];
    let mut output3 = vec![0.0; buffer_size];

    // Render three consecutive buffers
    graph.eval_node_buffer(&comb, &mut output1);
    graph.eval_node_buffer(&comb, &mut output2);
    graph.eval_node_buffer(&comb, &mut output3);

    // All should have similar energy (state persists)
    let rms1 = calculate_rms(&output1);
    let rms2 = calculate_rms(&output2);
    let rms3 = calculate_rms(&output3);

    assert!(rms2 > 0.1, "Second buffer should have energy: {}", rms2);
    assert!(rms3 > 0.1, "Third buffer should have energy: {}", rms3);

    // RMS should be relatively stable (not decaying to zero)
    assert!(rms3 > rms1 * 0.5,
        "State should persist: rms1={}, rms3={}", rms1, rms3);
}

/// Test 6: Impulse response shows ringing
#[test]
fn test_comb_impulse_response() {
    let mut graph = create_test_graph();

    // Create a very brief impulse (single sample burst approximation)
    // Using a high-frequency oscillator for a short time
    let osc = graph.add_oscillator(Signal::Value(4000.0), Waveform::Sine);

    let comb = graph.add_comb_node(
        Signal::Node(osc),
        Signal::Value(220.0),  // Resonant at 220 Hz
        Signal::Value(0.9),    // Very strong feedback
    );

    let buffer_size = 8820; // 0.2 seconds
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&comb, &mut output);

    // Should see multiple peaks from the ringing
    let peaks = find_peaks(&output, 0.1);

    assert!(peaks.len() >= 3,
        "Comb impulse response should show multiple peaks (ringing): found {}", peaks.len());
}

/// Test 7: Very low feedback approaches unity gain
#[test]
fn test_comb_low_feedback_unity() {
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    let comb = graph.add_comb_node(
        Signal::Node(osc),
        Signal::Value(440.0),
        Signal::Value(0.01),  // Very low feedback
    );

    let buffer_size = 4410;
    let mut output = vec![0.0; buffer_size];
    let mut osc_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&comb, &mut output);
    graph.eval_node_buffer(&osc, &mut osc_output);

    let rms_comb = calculate_rms(&output);
    let rms_osc = calculate_rms(&osc_output);

    // With very low feedback, should be close to input
    assert!((rms_comb - rms_osc).abs() / rms_osc < 0.2,
        "Low feedback should preserve input level: osc={}, comb={}", rms_osc, rms_comb);
}

/// Test 8: Produces non-zero output
#[test]
fn test_comb_produces_output() {
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let comb = graph.add_comb_node(
        Signal::Node(osc),
        Signal::Value(220.0),
        Signal::Value(0.7),
    );

    let buffer_size = 4410;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&comb, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.1, "Comb should produce audible output: RMS={}", rms);
}

/// Test 9: Different frequencies produce different resonances
#[test]
fn test_comb_frequency_specificity() {
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Comb tuned to 440 Hz (matches input)
    let comb_match = graph.add_comb_node(
        Signal::Node(osc),
        Signal::Value(440.0),
        Signal::Value(0.8),
    );

    // Comb tuned to 220 Hz (octave below)
    let comb_octave = graph.add_comb_node(
        Signal::Node(osc),
        Signal::Value(220.0),
        Signal::Value(0.8),
    );

    let buffer_size = 4410;
    let mut output_match = vec![0.0; buffer_size];
    let mut output_octave = vec![0.0; buffer_size];

    graph.eval_node_buffer(&comb_match, &mut output_match);
    graph.eval_node_buffer(&comb_octave, &mut output_octave);

    // Both should produce output, but potentially different characteristics
    let rms_match = calculate_rms(&output_match);
    let rms_octave = calculate_rms(&output_octave);

    assert!(rms_match > 0.1, "Matched frequency comb should resonate: {}", rms_match);
    assert!(rms_octave > 0.1, "Octave frequency comb should resonate: {}", rms_octave);
}

/// Test 10: High feedback near limit (0.99)
#[test]
fn test_comb_high_feedback_limit() {
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(110.0), Waveform::Sine);
    let comb = graph.add_comb_node(
        Signal::Node(osc),
        Signal::Value(110.0),
        Signal::Value(0.98),  // Near maximum
    );

    let buffer_size = 8820;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&comb, &mut output);

    let rms = calculate_rms(&output);

    // Should produce strong resonance but not explode
    assert!(rms > 0.2, "High feedback should produce strong resonance: {}", rms);
    assert!(rms < 15.0, "High feedback should not explode excessively: {}", rms);

    // Check no NaN or Inf
    for &sample in output.iter() {
        assert!(sample.is_finite(), "Output should be finite, got: {}", sample);
    }
}

/// Test 11: Metallic character (multiple harmonics)
#[test]
fn test_comb_metallic_sound() {
    let mut graph = create_test_graph();

    // Use white noise as input for more interesting resonance
    let noise = graph.add_node(SignalNode::Noise { seed: 12345 });

    let comb = graph.add_comb_node(
        Signal::Node(noise),
        Signal::Value(440.0),
        Signal::Value(0.85),
    );

    let buffer_size = 8820;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&comb, &mut output);

    let rms = calculate_rms(&output);

    // Should produce audible metallic resonance
    assert!(rms > 0.05, "Comb filtering noise should produce resonance: {}", rms);
}

/// Test 12: Low frequency resonance (physical modeling range)
#[test]
fn test_comb_low_frequency_resonance() {
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(55.0), Waveform::Sine);

    // Very low resonant frequency (like a large object)
    let comb = graph.add_comb_node(
        Signal::Node(osc),
        Signal::Value(27.5),  // Very low (A0)
        Signal::Value(0.75),
    );

    let buffer_size = 8820;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&comb, &mut output);

    let rms = calculate_rms(&output);

    assert!(rms > 0.1, "Low frequency comb should produce output: {}", rms);
}

/// Test 13: Multiple consecutive buffers maintain resonance
#[test]
fn test_comb_sustained_resonance() {
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(220.0), Waveform::Sine);
    let comb = graph.add_comb_node(
        Signal::Node(osc),
        Signal::Value(220.0),
        Signal::Value(0.85),
    );

    let buffer_size = 2205;

    // Render 5 consecutive buffers
    for _ in 0..5 {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&comb, &mut output);

        let rms = calculate_rms(&output);
        assert!(rms > 0.1, "Each buffer should maintain resonance: {}", rms);
    }
}

/// Test 14: Feedback clamping (values > 0.99 should be clamped)
#[test]
fn test_comb_feedback_clamping() {
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Try to set feedback > 0.99 (should be clamped internally)
    let comb = graph.add_comb_node(
        Signal::Node(osc),
        Signal::Value(440.0),
        Signal::Value(1.5),  // Should be clamped to 0.99
    );

    let buffer_size = 4410;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&comb, &mut output);

    // Should not explode to infinity (some amplification is expected with high feedback)
    // Comb filters with feedback at 0.99 can amplify 50-100x but should remain finite
    let max_abs = output.iter().map(|x| x.abs()).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();

    assert!(max_abs.is_finite(), "Output should remain finite with high feedback");
    assert!(max_abs < 100.0, "Output should not explode to extreme values: max={}", max_abs);
}
