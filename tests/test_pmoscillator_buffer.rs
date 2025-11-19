/// Buffer-level tests for PM (Phase Modulation) Oscillator
///
/// These tests verify that PM oscillator buffer evaluation produces correct
/// phase modulation synthesis at the UnifiedSignalGraph buffer API level.
///
/// PM vs FM: PM modulates phase directly, FM modulates frequency.
/// PM: output = sin(2π * carrier_phase + mod_index * modulation_signal)
/// Used in Yamaha DX7 (marketed as FM but actually PM).

use phonon::unified_graph::{Signal, UnifiedSignalGraph};
use std::f32::consts::PI;

/// Helper: Create a test graph
fn create_test_graph() -> UnifiedSignalGraph {
    UnifiedSignalGraph::new(44100.0) // 44.1kHz
}

/// Helper: Calculate RMS of a buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Helper: Count zero crossings
fn count_zero_crossings(buffer: &[f32]) -> usize {
    let mut count = 0;
    for i in 1..buffer.len() {
        if (buffer[i-1] < 0.0 && buffer[i] >= 0.0) || (buffer[i-1] >= 0.0 && buffer[i] < 0.0) {
            count += 1;
        }
    }
    count
}

// ============================================================================
// TEST: Basic Buffer Evaluation
// ============================================================================

#[test]
fn test_pm_buffer_basic_tone() {
    let mut graph = create_test_graph();

    // Create a simple sine wave as modulation source
    let mod_osc = graph.add_node(phonon::unified_graph::SignalNode::Oscillator {
        freq: Signal::Value(220.0),
        waveform: phonon::unified_graph::Waveform::Sine,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    });

    let pm_id = graph.add_pmoscillator_node(
        Signal::Value(440.0),     // Carrier: A4
        Signal::Node(mod_osc),    // Modulation: 220Hz sine
        Signal::Value(2.0),       // Mod index
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&pm_id, &mut output);

    // Should produce audible signal
    let rms = calculate_rms(&output);
    assert!(rms > 0.3, "PM should produce significant energy, got RMS {}", rms);

    // Peak amplitude should be ~1.0
    let max_amplitude = output.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    assert!(max_amplitude > 0.9 && max_amplitude <= 1.05,
        "PM peak amplitude should be ~1.0, got {}", max_amplitude);
}

#[test]
fn test_pm_buffer_zero_index_equals_sine() {
    let mut graph = create_test_graph();

    // Create a modulation source (irrelevant when index=0)
    let mod_osc = graph.add_node(phonon::unified_graph::SignalNode::Oscillator {
        freq: Signal::Value(220.0),
        waveform: phonon::unified_graph::Waveform::Sine,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    });

    // With mod_index=0, PM should behave like a pure sine wave
    let pm_id = graph.add_pmoscillator_node(
        Signal::Value(440.0),
        Signal::Node(mod_osc),
        Signal::Value(0.0),       // Zero modulation
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&pm_id, &mut output);

    // RMS should be close to sine wave RMS (1/sqrt(2) ≈ 0.707)
    let rms = calculate_rms(&output);
    assert!(rms > 0.65 && rms < 0.75,
        "PM with index=0 should match sine RMS (~0.707), got {}", rms);
}

// ============================================================================
// TEST: Phase Continuity
// ============================================================================

#[test]
fn test_pm_buffer_phase_continuity() {
    let mut graph = create_test_graph();

    let mod_osc = graph.add_node(phonon::unified_graph::SignalNode::Oscillator {
        freq: Signal::Value(110.0),
        waveform: phonon::unified_graph::Waveform::Sine,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    });

    let pm_id = graph.add_pmoscillator_node(
        Signal::Value(440.0),
        Signal::Node(mod_osc),
        Signal::Value(2.0),
    );

    let buffer_size = 512;
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&pm_id, &mut buffer1);
    graph.eval_node_buffer(&pm_id, &mut buffer2);

    // Check signal is continuous (no glitches at buffer boundary)
    let last_sample = buffer1[buffer_size - 1];
    let first_sample = buffer2[0];

    // The difference should be reasonable (no huge discontinuity)
    let max_step = 0.5;
    assert!((first_sample - last_sample).abs() < max_step,
        "Phase discontinuity: {} -> {} (diff = {})",
        last_sample, first_sample, (first_sample - last_sample).abs());
}

#[test]
fn test_pm_buffer_multiple_buffers_consistent() {
    let mut graph = create_test_graph();

    let mod_osc = graph.add_node(phonon::unified_graph::SignalNode::Oscillator {
        freq: Signal::Value(220.0),
        waveform: phonon::unified_graph::Waveform::Sine,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    });

    let pm_id = graph.add_pmoscillator_node(
        Signal::Value(440.0),
        Signal::Node(mod_osc),
        Signal::Value(3.0),
    );

    let buffer_size = 512;
    let num_buffers = 10;

    for i in 0..num_buffers {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&pm_id, &mut output);

        // Each buffer should have consistent energy
        let rms = calculate_rms(&output);
        assert!(rms > 0.4 && rms < 0.9,
            "Buffer {} has unexpected RMS: {}", i, rms);
    }
}

// ============================================================================
// TEST: Modulation Index Variation
// ============================================================================

#[test]
fn test_pm_buffer_varying_mod_index() {
    let mut graph = create_test_graph();
    let buffer_size = 512;

    let indices = vec![0.0, 1.0, 2.0, 5.0, 10.0];

    for &index in &indices {
        let mod_osc = graph.add_node(phonon::unified_graph::SignalNode::Oscillator {
            freq: Signal::Value(110.0),
            waveform: phonon::unified_graph::Waveform::Sine,
            phase: std::cell::RefCell::new(0.0),
            pending_freq: std::cell::RefCell::new(None),
            last_sample: std::cell::RefCell::new(0.0),
        });

        let pm_id = graph.add_pmoscillator_node(
            Signal::Value(440.0),
            Signal::Node(mod_osc),
            Signal::Value(index),
        );

        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&pm_id, &mut output);

        // Should have reasonable number of zero crossings
        // For 440Hz carrier over 512 samples @ 44.1kHz: ~10 crossings baseline
        let crossings = count_zero_crossings(&output);

        assert!(crossings >= 8,
            "PM with index {} should have multiple zero crossings, got {}",
            index, crossings);
    }
}

// ============================================================================
// TEST: Different Modulation Sources
// ============================================================================

#[test]
fn test_pm_buffer_different_mod_waveforms() {
    let mut graph = create_test_graph();
    let buffer_size = 512;

    // Test different modulation waveforms
    let waveforms = vec![
        phonon::unified_graph::Waveform::Sine,
        phonon::unified_graph::Waveform::Saw,
        phonon::unified_graph::Waveform::Square,
        phonon::unified_graph::Waveform::Triangle,
    ];

    for waveform in waveforms {
        let mod_osc = graph.add_node(phonon::unified_graph::SignalNode::Oscillator {
            freq: Signal::Value(110.0),
            waveform,
            phase: std::cell::RefCell::new(0.0),
            pending_freq: std::cell::RefCell::new(None),
            last_sample: std::cell::RefCell::new(0.0),
        });

        let pm_id = graph.add_pmoscillator_node(
            Signal::Value(440.0),
            Signal::Node(mod_osc),
            Signal::Value(2.0),
        );

        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&pm_id, &mut output);

        // Should produce audio regardless of modulation waveform
        let rms = calculate_rms(&output);
        assert!(rms > 0.3,
            "PM with {:?} modulation should produce energy, got RMS {}",
            waveform, rms);
    }
}

#[test]
fn test_pm_buffer_constant_modulation() {
    let mut graph = create_test_graph();

    // PM with constant modulation is just phase offset
    let pm_id = graph.add_pmoscillator_node(
        Signal::Value(440.0),
        Signal::Value(PI / 4.0),  // Constant phase offset
        Signal::Value(1.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&pm_id, &mut output);

    // Should still be a sine wave, just phase-shifted
    let rms = calculate_rms(&output);
    assert!(rms > 0.65 && rms < 0.75,
        "PM with constant mod should be like sine, got RMS {}", rms);
}

// ============================================================================
// TEST: Frequency Parameters
// ============================================================================

#[test]
fn test_pm_buffer_different_carriers() {
    let mut graph = create_test_graph();
    let buffer_size = 512;

    let mod_osc_a3 = graph.add_node(phonon::unified_graph::SignalNode::Oscillator {
        freq: Signal::Value(110.0),
        waveform: phonon::unified_graph::Waveform::Sine,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    });

    let mod_osc_a4 = graph.add_node(phonon::unified_graph::SignalNode::Oscillator {
        freq: Signal::Value(110.0),
        waveform: phonon::unified_graph::Waveform::Sine,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    });

    // Two different carrier frequencies
    let pm_a3 = graph.add_pmoscillator_node(
        Signal::Value(220.0),  // A3
        Signal::Node(mod_osc_a3),
        Signal::Value(2.0),
    );

    let pm_a4 = graph.add_pmoscillator_node(
        Signal::Value(440.0),  // A4
        Signal::Node(mod_osc_a4),
        Signal::Value(2.0),
    );

    let mut output_a3 = vec![0.0; buffer_size];
    let mut output_a4 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&pm_a3, &mut output_a3);
    graph.eval_node_buffer(&pm_a4, &mut output_a4);

    // Higher frequency should have more zero crossings
    let crossings_a3 = count_zero_crossings(&output_a3);
    let crossings_a4 = count_zero_crossings(&output_a4);

    assert!(crossings_a4 > crossings_a3,
        "A4 (440Hz) should have more crossings than A3 (220Hz): {} vs {}",
        crossings_a4, crossings_a3);
}

// ============================================================================
// TEST: PM vs FM Comparison
// ============================================================================

#[test]
fn test_pm_vs_fm_similar_but_different() {
    let mut graph = create_test_graph();
    let buffer_size = 512;

    // PM oscillator
    let mod_osc_pm = graph.add_node(phonon::unified_graph::SignalNode::Oscillator {
        freq: Signal::Value(110.0),
        waveform: phonon::unified_graph::Waveform::Sine,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    });

    let pm_id = graph.add_pmoscillator_node(
        Signal::Value(440.0),
        Signal::Node(mod_osc_pm),
        Signal::Value(2.0),
    );

    // FM oscillator
    let fm_id = graph.add_fmoscillator_node(
        Signal::Value(440.0),
        Signal::Value(110.0),
        Signal::Value(2.0),
    );

    let mut pm_output = vec![0.0; buffer_size];
    let mut fm_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&pm_id, &mut pm_output);
    graph.eval_node_buffer(&fm_id, &mut fm_output);

    let pm_rms = calculate_rms(&pm_output);
    let fm_rms = calculate_rms(&fm_output);

    // Both should produce similar energy
    assert!(pm_rms > 0.3 && fm_rms > 0.3,
        "Both PM and FM should produce energy: PM={}, FM={}", pm_rms, fm_rms);

    // Note: PM with sinusoidal modulation is mathematically equivalent to FM!
    // PM: sin(2πft + I*sin(2πfm*t))
    // FM: sin(2πft + I*sin(2πfm*t))
    // They should produce very similar (if not identical) results
    let mut difference_count = 0;
    for i in 0..buffer_size {
        if (pm_output[i] - fm_output[i]).abs() > 0.001 {
            difference_count += 1;
        }
    }

    // They may be very similar or identical depending on implementation details
    // This test just verifies both work correctly
    println!("PM vs FM: {} of {} samples differ (PM RMS={:.3}, FM RMS={:.3})",
        difference_count, buffer_size, pm_rms, fm_rms);
}

// ============================================================================
// TEST: Edge Cases
// ============================================================================

#[test]
fn test_pm_buffer_zero_carrier() {
    let mut graph = create_test_graph();

    let mod_osc = graph.add_node(phonon::unified_graph::SignalNode::Oscillator {
        freq: Signal::Value(220.0),
        waveform: phonon::unified_graph::Waveform::Sine,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    });

    let pm_id = graph.add_pmoscillator_node(
        Signal::Value(0.0),
        Signal::Node(mod_osc),
        Signal::Value(2.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&pm_id, &mut output);

    // Should handle gracefully (no crash/NaN)
    assert!(output.iter().all(|&x| x.is_finite()),
        "Zero carrier should be handled gracefully");
}

#[test]
fn test_pm_buffer_negative_frequencies_clamped() {
    let mut graph = create_test_graph();

    let mod_osc = graph.add_node(phonon::unified_graph::SignalNode::Oscillator {
        freq: Signal::Value(110.0),
        waveform: phonon::unified_graph::Waveform::Sine,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    });

    let pm_id = graph.add_pmoscillator_node(
        Signal::Value(-100.0),  // Negative
        Signal::Node(mod_osc),
        Signal::Value(2.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&pm_id, &mut output);

    // Should handle gracefully (clamped to 0)
    assert!(output.iter().all(|&x| x.is_finite()),
        "Negative frequencies should be handled gracefully");
}

#[test]
fn test_pm_buffer_high_mod_index() {
    let mut graph = create_test_graph();

    let mod_osc = graph.add_node(phonon::unified_graph::SignalNode::Oscillator {
        freq: Signal::Value(110.0),
        waveform: phonon::unified_graph::Waveform::Sine,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    });

    let pm_id = graph.add_pmoscillator_node(
        Signal::Value(440.0),
        Signal::Node(mod_osc),
        Signal::Value(20.0),   // Very high index
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&pm_id, &mut output);

    // Should not cause excessive amplitude
    let max_amplitude = output.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    assert!(max_amplitude <= 1.05,
        "High index should not cause excessive amplitude: {}", max_amplitude);

    let rms = calculate_rms(&output);
    assert!(rms > 0.3, "High index should produce significant energy");
}

// ============================================================================
// TEST: Performance
// ============================================================================

#[test]
fn test_pm_buffer_performance() {
    let mut graph = create_test_graph();

    let mod_osc = graph.add_node(phonon::unified_graph::SignalNode::Oscillator {
        freq: Signal::Value(220.0),
        waveform: phonon::unified_graph::Waveform::Sine,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    });

    let pm_id = graph.add_pmoscillator_node(
        Signal::Value(440.0),
        Signal::Node(mod_osc),
        Signal::Value(3.0),
    );

    let buffer_size = 512;
    let iterations = 1000;

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&pm_id, &mut output);
    }
    let duration = start.elapsed();

    println!("PM buffer eval: {:?} for {} iterations", duration, iterations);
    println!("Per iteration: {:?}", duration / iterations);

    // Should complete in reasonable time (< 2 seconds)
    assert!(duration.as_secs() < 2,
        "PM buffer evaluation too slow: {:?}", duration);
}
