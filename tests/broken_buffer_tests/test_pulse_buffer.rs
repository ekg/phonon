/// Tests for Pulse oscillator buffer-based evaluation
///
/// Verifies buffer evaluation produces correct waveforms with PolyBLEP anti-aliasing.
/// Tests pulse width modulation (PWM), phase continuity, and anti-aliasing effectiveness.

use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph};
use std::rc::Rc;

/// Helper: Create a test graph
fn create_test_graph() -> UnifiedSignalGraph {
    UnifiedSignalGraph::new(44100.0) // 44.1kHz
}

/// Helper: Calculate RMS of a buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

// ============================================================================
// TEST 1: Basic Pulse Generation
// ============================================================================

#[test]
fn test_pulse_generates_audio() {
    let mut graph = create_test_graph();

    let pulse_id = graph.add_pulse_node(
        Signal::Value(220.0),
        Signal::Value(0.5),  // Square wave
    );

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&pulse_id, &mut output);

    // Should have reasonable audio
    let rms = calculate_rms(&output);
    assert!(rms > 0.5, "Pulse should produce audio, got RMS: {}", rms);

    println!("Pulse RMS: {}", rms);
}

#[test]
fn test_pulse_square_wave_symmetry() {
    let mut graph = create_test_graph();

    let pulse_id = graph.add_pulse_node(
        Signal::Value(220.0),
        Signal::Value(0.5),  // Square wave (50% duty cycle)
    );

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&pulse_id, &mut output);

    // Count positive vs negative samples (should be roughly equal for square wave)
    let pos_count = output.iter().filter(|&&x| x > 0.0).count();
    let neg_count = output.iter().filter(|&&x| x < 0.0).count();

    let ratio = pos_count as f32 / neg_count as f32;
    assert!((ratio - 1.0).abs() < 0.2,
        "Square wave should have equal positive/negative time, got ratio: {}",
        ratio);

    println!("Pos/Neg ratio: {} ({}/{})", ratio, pos_count, neg_count);
}

// ============================================================================
// TEST 2: Pulse Width Modulation
// ============================================================================

#[test]
fn test_pulse_narrow_width() {
    let mut graph = create_test_graph();

    let pulse_id = graph.add_pulse_node(
        Signal::Value(220.0),
        Signal::Value(0.1),  // Narrow pulse (10% duty cycle)
    );

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&pulse_id, &mut output);

    // Narrow pulse should have fewer positive samples
    let pos_count = output.iter().filter(|&&x| x > 0.0).count();
    let ratio = pos_count as f32 / 512.0;

    assert!((ratio - 0.1).abs() < 0.15,
        "Narrow pulse should have ~10% positive time, got: {}",
        ratio);

    println!("Narrow pulse pos ratio: {}", ratio);
}

#[test]
fn test_pulse_wide_width() {
    let mut graph = create_test_graph();

    let pulse_id = graph.add_pulse_node(
        Signal::Value(220.0),
        Signal::Value(0.9),  // Wide pulse (90% duty cycle)
    );

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&pulse_id, &mut output);

    // Wide pulse should have more positive samples
    let pos_count = output.iter().filter(|&&x| x > 0.0).count();
    let ratio = pos_count as f32 / 512.0;

    assert!((ratio - 0.9).abs() < 0.15,
        "Wide pulse should have ~90% positive time, got: {}",
        ratio);

    println!("Wide pulse pos ratio: {}", ratio);
}

// ============================================================================
// TEST 3: Frequency Sweep
// ============================================================================

#[test]
fn test_pulse_low_frequency() {
    let mut graph = create_test_graph();

    let pulse_id = graph.add_pulse_node(
        Signal::Value(55.0),  // Low frequency
        Signal::Value(0.5),
    );

    let mut output = vec![0.0; 8192];  // Longer buffer for low frequency
    graph.eval_node_buffer(&pulse_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.5, "Low frequency pulse should work, RMS: {}", rms);
}

#[test]
fn test_pulse_high_frequency() {
    let mut graph = create_test_graph();

    let pulse_id = graph.add_pulse_node(
        Signal::Value(4000.0),  // High frequency
        Signal::Value(0.5),
    );

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&pulse_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.3, "High frequency pulse should work, RMS: {}", rms);

    // Check for transitions (should have many at high frequency)
    let mut transitions = 0;
    for i in 1..output.len() {
        if (output[i] > 0.0) != (output[i - 1] > 0.0) {
            transitions += 1;
        }
    }

    assert!(transitions > 10,
        "High frequency pulse should have many transitions, got: {}",
        transitions);
}

// ============================================================================
// TEST 4: Phase Continuity
// ============================================================================

#[test]
fn test_pulse_phase_continuity_across_buffers() {
    let mut graph = create_test_graph();

    let pulse_id = graph.add_pulse_node(
        Signal::Value(220.0),
        Signal::Value(0.5),
    );

    // Generate two consecutive buffers
    let mut buffer1 = vec![0.0; 512];
    let mut buffer2 = vec![0.0; 512];

    graph.eval_node_buffer(&pulse_id, &mut buffer1);
    graph.eval_node_buffer(&pulse_id, &mut buffer2);

    // Phase should be continuous (no sudden jump)
    let last_val = buffer1[511];
    let first_val = buffer2[0];

    // Values should be similar (both positive or both negative)
    assert!((last_val > 0.0) == (first_val > 0.0),
        "Phase should be continuous, last: {}, first: {}",
        last_val, first_val);
}

#[test]
fn test_pulse_multiple_buffer_consistency() {
    let mut graph = create_test_graph();

    let pulse_id = graph.add_pulse_node(
        Signal::Value(220.0),
        Signal::Value(0.5),
    );

    // Generate 10 consecutive buffers
    for i in 0..10 {
        let mut output = vec![0.0; 512];
        graph.eval_node_buffer(&pulse_id, &mut output);

        // Each buffer should have reasonable RMS
        let rms = calculate_rms(&output);
        assert!(rms > 0.5 && rms < 1.5,
            "Buffer {} has unexpected RMS: {}",
            i, rms);
    }
}

// ============================================================================
// TEST 5: Anti-Aliasing (PolyBLEP Verification)
// ============================================================================

#[test]
fn test_pulse_antialiasing_high_freq() {
    // At high frequencies, naive pulse generates aliasing
    // PolyBLEP should reduce high-frequency artifacts

    let mut graph = create_test_graph();

    let pulse_id = graph.add_pulse_node(
        Signal::Value(8000.0),  // Very high frequency (near Nyquist/2)
        Signal::Value(0.5),
    );

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&pulse_id, &mut output);

    // Should not clip or have extreme values
    for (i, &sample) in output.iter().enumerate() {
        assert!(sample.abs() <= 1.5,
            "Sample {} has extreme value (aliasing?): {}",
            i, sample);
    }

    let rms = calculate_rms(&output);
    assert!(rms > 0.1, "High freq pulse should produce audio, RMS: {}", rms);
}

#[test]
fn test_pulse_no_clipping() {
    let mut graph = create_test_graph();

    let pulse_id = graph.add_pulse_node(
        Signal::Value(440.0),
        Signal::Value(0.5),
    );

    let mut output = vec![0.0; 1024];
    graph.eval_node_buffer(&pulse_id, &mut output);

    // PolyBLEP should keep values near ±1.0
    // Allow small overshoot for PolyBLEP correction
    for (i, &sample) in output.iter().enumerate() {
        assert!(sample.abs() <= 1.5,
            "Sample {} out of range: {}",
            i, sample);
    }
}

// ============================================================================
// TEST 6: Edge Cases
// ============================================================================

#[test]
fn test_pulse_zero_frequency() {
    let mut graph = create_test_graph();

    let pulse_id = graph.add_pulse_node(
        Signal::Value(0.0),  // DC (no oscillation)
        Signal::Value(0.5),
    );

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&pulse_id, &mut output);

    // Should produce constant DC value
    let first_val = output[0];
    for &sample in &output {
        assert!((sample - first_val).abs() < 0.01,
            "DC should be constant");
    }
}

#[test]
fn test_pulse_small_buffer() {
    let mut graph = create_test_graph();

    let pulse_id = graph.add_pulse_node(
        Signal::Value(220.0),
        Signal::Value(0.5),
    );

    // Very small buffer
    let mut output = vec![0.0; 8];
    graph.eval_node_buffer(&pulse_id, &mut output);

    // Should still work
    let rms = calculate_rms(&output);
    assert!(rms > 0.0, "Small buffer should still produce audio");
}

#[test]
fn test_pulse_large_buffer() {
    let mut graph = create_test_graph();

    let pulse_id = graph.add_pulse_node(
        Signal::Value(220.0),
        Signal::Value(0.5),
    );

    // Large buffer
    let mut output = vec![0.0; 16384];
    graph.eval_node_buffer(&pulse_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.5, "Large buffer should produce audio, RMS: {}", rms);
}

// ============================================================================
// TEST 7: Pattern-Modulatable Parameters
// ============================================================================

#[test]
fn test_pulse_dynamic_frequency() {
    let mut graph = create_test_graph();

    // Create a low-frequency oscillator to modulate pulse frequency
    let lfo_id = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(0.5),
        waveform: phonon::unified_graph::Waveform::Sine,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    });

    // Modulate frequency: 220 + lfo * 110 (range: 110-330 Hz)
    let freq_offset = graph.add_multiply_node(
        Signal::Node(lfo_id),
        Signal::Value(110.0),
    );
    let freq_sig = graph.add_add_node(
        Signal::Value(220.0),
        Signal::Node(freq_offset),
    );

    let pulse_id = graph.add_pulse_node(
        Signal::Node(freq_sig),
        Signal::Value(0.5),
    );

    let mut output = vec![0.0; 2048];
    graph.eval_node_buffer(&pulse_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.3, "Dynamic frequency pulse should work, RMS: {}", rms);
}

#[test]
fn test_pulse_dynamic_width() {
    let mut graph = create_test_graph();

    // Create LFO to modulate pulse width
    let lfo_id = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(1.0),
        waveform: phonon::unified_graph::Waveform::Sine,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    });

    // Modulate width: 0.5 + lfo * 0.3 (range: 0.2-0.8)
    let width_offset = graph.add_multiply_node(
        Signal::Node(lfo_id),
        Signal::Value(0.3),
    );
    let width_sig = graph.add_add_node(
        Signal::Value(0.5),
        Signal::Node(width_offset),
    );

    let pulse_id = graph.add_pulse_node(
        Signal::Value(220.0),
        Signal::Node(width_sig),
    );

    let mut output = vec![0.0; 2048];
    graph.eval_node_buffer(&pulse_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.3, "Dynamic width pulse (PWM) should work, RMS: {}", rms);
}

// ============================================================================
// TEST 8: Performance
// ============================================================================

#[test]
fn test_pulse_buffer_performance() {
    let mut graph = create_test_graph();

    let pulse_id = graph.add_pulse_node(
        Signal::Value(440.0),
        Signal::Value(0.5),
    );

    let buffer_size = 512;
    let iterations = 1000;

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&pulse_id, &mut output);
    }
    let duration = start.elapsed();

    println!("Pulse buffer eval: {:?} for {} iterations", duration, iterations);
    println!("Per iteration: {:?}", duration / iterations);

    // Should complete in reasonable time (< 2 seconds for 1000 iterations)
    assert!(duration.as_secs() < 2,
        "Pulse buffer evaluation too slow: {:?}",
        duration);
}

// ============================================================================
// TEST 9: Comparison with Expected Values
// ============================================================================

#[test]
fn test_pulse_amplitude() {
    let mut graph = create_test_graph();

    let pulse_id = graph.add_pulse_node(
        Signal::Value(220.0),
        Signal::Value(0.5),
    );

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&pulse_id, &mut output);

    // Pulse should oscillate between approximately +1 and -1
    let max_val = output.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let min_val = output.iter().cloned().fold(f32::INFINITY, f32::min);

    assert!(max_val > 0.8 && max_val < 1.2,
        "Max value should be near +1.0, got: {}",
        max_val);
    assert!(min_val < -0.8 && min_val > -1.2,
        "Min value should be near -1.0, got: {}",
        min_val);
}

#[test]
fn test_pulse_dc_offset() {
    let mut graph = create_test_graph();

    let pulse_id = graph.add_pulse_node(
        Signal::Value(220.0),
        Signal::Value(0.5),  // Symmetric pulse (no DC offset expected)
    );

    let mut output = vec![0.0; 2048];
    graph.eval_node_buffer(&pulse_id, &mut output);

    let mean: f32 = output.iter().sum::<f32>() / output.len() as f32;

    assert!(mean.abs() < 0.2,
        "Square wave should have minimal DC offset, got mean: {}",
        mean);
}

// ============================================================================
// TEST 10: Width Clamping
// ============================================================================

#[test]
fn test_pulse_width_clamping_low() {
    let mut graph = create_test_graph();

    // Width below minimum (should be clamped to 0.01)
    let pulse_id = graph.add_pulse_node(
        Signal::Value(220.0),
        Signal::Value(-0.5),  // Invalid (negative)
    );

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&pulse_id, &mut output);

    // Should still produce audio (clamped to valid range)
    let rms = calculate_rms(&output);
    assert!(rms > 0.01, "Clamped width should still work, RMS: {}", rms);
}

#[test]
fn test_pulse_width_clamping_high() {
    let mut graph = create_test_graph();

    // Width above maximum (should be clamped to 0.99)
    let pulse_id = graph.add_pulse_node(
        Signal::Value(220.0),
        Signal::Value(1.5),  // Invalid (> 1.0)
    );

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&pulse_id, &mut output);

    // Should still produce audio (clamped to valid range)
    let rms = calculate_rms(&output);
    assert!(rms > 0.01, "Clamped width should still work, RMS: {}", rms);
}
