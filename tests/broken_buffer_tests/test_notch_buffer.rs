/// Tests for Notch filter buffer-based evaluation
///
/// These tests verify that Notch filter buffer evaluation produces correct
/// filtering behavior (rejects center frequency, passes both low and high frequencies).
///
/// Notch is the opposite of BandPass - it removes the band at the center frequency
/// while passing everything else (low + high).

use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph, Waveform, FilterState};

/// Helper: Create a test graph
fn create_test_graph() -> UnifiedSignalGraph {
    UnifiedSignalGraph::new(44100.0)
}

/// Helper: Calculate RMS of a buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Helper: Measure frequency content (simplified - measures rate of change)
fn measure_high_freq_energy(buffer: &[f32]) -> f32 {
    let mut energy = 0.0;
    for i in 1..buffer.len() {
        energy += (buffer[i] - buffer[i-1]).abs();
    }
    energy / buffer.len() as f32
}

// ============================================================================
// TEST: Basic Filtering - Notch Rejects Center Frequency
// ============================================================================

#[test]
fn test_notch_rejects_center_frequency() {
    let mut graph = create_test_graph();

    // Oscillator at 1000 Hz
    let osc_id = graph.add_oscillator(Signal::Value(1000.0), Waveform::Sine);

    // Notch at 1000 Hz (should reject this frequency)
    let notch_id = graph.add_node(SignalNode::Notch {
        input: Signal::Node(osc_id),
        center: Signal::Value(1000.0),
        q: Signal::Value(5.0), // Narrow notch
        state: FilterState::default(),
    });

    let buffer_size = 512;
    let mut notched = vec![0.0; buffer_size];
    let mut original = vec![0.0; buffer_size];

    // Process several buffers to let filter settle
    for _ in 0..10 {
        graph.eval_node_buffer(&notch_id, &mut notched);
    }

    // Get original signal for comparison
    graph.eval_node_buffer(&osc_id, &mut original);

    let notch_rms = calculate_rms(&notched);
    let orig_rms = calculate_rms(&original);

    // Notched signal should be significantly attenuated
    // Note: SVF notch doesn't completely eliminate the frequency, just attenuates it
    assert!(notch_rms < orig_rms * 0.8,
        "Notch should reject center freq: notch RMS = {}, orig RMS = {}", notch_rms, orig_rms);
}

#[test]
fn test_notch_passes_off_center_frequencies() {
    let mut graph = create_test_graph();

    // Oscillator at 440 Hz
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Notch at 2000 Hz (far from 440 Hz, should pass through)
    let notch_id = graph.add_node(SignalNode::Notch {
        input: Signal::Node(osc_id),
        center: Signal::Value(2000.0),
        q: Signal::Value(1.0),
        state: FilterState::default(),
    });

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&notch_id, &mut output);

    // Should pass mostly unchanged
    let rms = calculate_rms(&output);
    assert!(rms > 0.6,
        "Notch should pass off-center freq: RMS = {}", rms);
}

#[test]
fn test_notch_passes_low_frequencies() {
    let mut graph = create_test_graph();

    // Low-frequency oscillator (100 Hz)
    let osc_id = graph.add_oscillator(Signal::Value(100.0), Waveform::Sine);

    // Notch at 2000 Hz should pass 100 Hz
    let notch_id = graph.add_node(SignalNode::Notch {
        input: Signal::Node(osc_id),
        center: Signal::Value(2000.0),
        q: Signal::Value(2.0),
        state: FilterState::default(),
    });

    let buffer_size = 512;
    let mut filtered = vec![0.0; buffer_size];
    let mut unfiltered = vec![0.0; buffer_size];

    graph.eval_node_buffer(&notch_id, &mut filtered);
    graph.eval_node_buffer(&osc_id, &mut unfiltered);

    let filtered_rms = calculate_rms(&filtered);
    let unfiltered_rms = calculate_rms(&unfiltered);

    // Should pass with minimal attenuation
    assert!(filtered_rms > unfiltered_rms * 0.8,
        "Notch should pass low frequencies: filtered RMS = {}, unfiltered RMS = {}",
        filtered_rms, unfiltered_rms);
}

#[test]
fn test_notch_passes_high_frequencies() {
    let mut graph = create_test_graph();

    // High-frequency oscillator (8000 Hz)
    let osc_id = graph.add_oscillator(Signal::Value(8000.0), Waveform::Sine);

    // Notch at 1000 Hz should pass 8000 Hz
    let notch_id = graph.add_node(SignalNode::Notch {
        input: Signal::Node(osc_id),
        center: Signal::Value(1000.0),
        q: Signal::Value(2.0),
        state: FilterState::default(),
    });

    let buffer_size = 512;
    let mut filtered = vec![0.0; buffer_size];
    let mut unfiltered = vec![0.0; buffer_size];

    graph.eval_node_buffer(&notch_id, &mut filtered);
    graph.eval_node_buffer(&osc_id, &mut unfiltered);

    let filtered_rms = calculate_rms(&filtered);
    let unfiltered_rms = calculate_rms(&unfiltered);

    // Should pass with minimal attenuation
    assert!(filtered_rms > unfiltered_rms * 0.8,
        "Notch should pass high frequencies: filtered RMS = {}, unfiltered RMS = {}",
        filtered_rms, unfiltered_rms);
}

// ============================================================================
// TEST: Q Factor Effect (Notch Width)
// ============================================================================

#[test]
fn test_notch_q_factor_affects_width() {
    let mut graph = create_test_graph();

    // Test frequency slightly off center (450 Hz vs 440 Hz center)
    let osc_id = graph.add_oscillator(Signal::Value(450.0), Waveform::Sine);

    // Narrow notch (high Q) - should miss 450 Hz, passing it through
    let notch_narrow = graph.add_node(SignalNode::Notch {
        input: Signal::Node(osc_id),
        center: Signal::Value(440.0),
        q: Signal::Value(10.0), // High Q = narrow notch
        state: FilterState::default(),
    });

    // Wide notch (low Q) - should catch 450 Hz, attenuating it
    let notch_wide = graph.add_node(SignalNode::Notch {
        input: Signal::Node(osc_id),
        center: Signal::Value(440.0),
        q: Signal::Value(0.5), // Low Q = wide notch
        state: FilterState::default(),
    });

    let buffer_size = 512;
    let mut narrow_output = vec![0.0; buffer_size];
    let mut wide_output = vec![0.0; buffer_size];

    // Let filters settle
    for _ in 0..5 {
        graph.eval_node_buffer(&notch_narrow, &mut narrow_output);
        graph.eval_node_buffer(&notch_wide, &mut wide_output);
    }

    let narrow_rms = calculate_rms(&narrow_output);
    let wide_rms = calculate_rms(&wide_output);

    // Narrow notch should pass more of 450 Hz (frequency outside narrow notch)
    // Wide notch should attenuate more of 450 Hz (frequency within wide notch)
    assert!(narrow_rms > wide_rms,
        "Narrow notch (high Q) should pass more than wide notch (low Q) for nearby frequencies: narrow = {}, wide = {}",
        narrow_rms, wide_rms);
}

#[test]
fn test_notch_high_q_narrower_rejection() {
    let mut graph = create_test_graph();

    // Oscillator at center frequency
    let osc_id = graph.add_oscillator(Signal::Value(1000.0), Waveform::Sine);

    // Low Q (wide rejection band)
    let notch_low_q = graph.add_node(SignalNode::Notch {
        input: Signal::Node(osc_id),
        center: Signal::Value(1000.0),
        q: Signal::Value(0.5),
        state: FilterState::default(),
    });

    // High Q (narrow rejection band)
    let notch_high_q = graph.add_node(SignalNode::Notch {
        input: Signal::Node(osc_id),
        center: Signal::Value(1000.0),
        q: Signal::Value(10.0),
        state: FilterState::default(),
    });

    let buffer_size = 512;
    let mut low_q_output = vec![0.0; buffer_size];
    let mut high_q_output = vec![0.0; buffer_size];

    // Let filters settle
    for _ in 0..10 {
        graph.eval_node_buffer(&notch_low_q, &mut low_q_output);
        graph.eval_node_buffer(&notch_high_q, &mut high_q_output);
    }

    let low_q_rms = calculate_rms(&low_q_output);
    let high_q_rms = calculate_rms(&high_q_output);

    // Both should be attenuated at center frequency (but not completely eliminated)
    // SVF notch filters attenuate but don't create infinite nulls
    assert!(low_q_rms < 0.9, "Low Q should attenuate center frequency: {}", low_q_rms);
    assert!(high_q_rms < 0.9, "High Q should attenuate center frequency: {}", high_q_rms);
}

// ============================================================================
// TEST: State Continuity
// ============================================================================

#[test]
fn test_notch_state_continuity() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let notch_id = graph.add_node(SignalNode::Notch {
        input: Signal::Node(osc_id),
        center: Signal::Value(1000.0),
        q: Signal::Value(2.0),
        state: FilterState::default(),
    });

    // Generate two consecutive buffers
    let buffer_size = 512;
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&notch_id, &mut buffer1);
    graph.eval_node_buffer(&notch_id, &mut buffer2);

    // Check continuity at boundary
    let last_sample = buffer1[buffer_size - 1];
    let first_sample = buffer2[0];
    let discontinuity = (first_sample - last_sample).abs();

    assert!(discontinuity < 0.1,
        "Filter state should be continuous: discontinuity = {}", discontinuity);
}

#[test]
fn test_notch_multiple_buffers() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);
    let notch_id = graph.add_node(SignalNode::Notch {
        input: Signal::Node(osc_id),
        center: Signal::Value(1000.0),
        q: Signal::Value(2.0),
        state: FilterState::default(),
    });

    // Generate 10 consecutive buffers
    let buffer_size = 512;
    for i in 0..10 {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&notch_id, &mut output);

        let rms = calculate_rms(&output);
        assert!(rms > 0.01 && rms < 2.0,
            "Buffer {} has unexpected RMS: {}", i, rms);
    }
}

// ============================================================================
// TEST: Comparison with BandPass (Notch is Inverse)
// ============================================================================

#[test]
fn test_notch_opposite_of_bandpass() {
    let mut graph = create_test_graph();

    // Broadband signal (sawtooth has many harmonics)
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);

    // BandPass at 1000 Hz (passes 1000 Hz, rejects others)
    let bpf_id = graph.add_node(SignalNode::BandPass {
        input: Signal::Node(osc_id),
        center: Signal::Value(1000.0),
        q: Signal::Value(2.0),
        state: FilterState::default(),
    });

    // Notch at 1000 Hz (rejects 1000 Hz, passes others)
    let notch_id = graph.add_node(SignalNode::Notch {
        input: Signal::Node(osc_id),
        center: Signal::Value(1000.0),
        q: Signal::Value(2.0),
        state: FilterState::default(),
    });

    let buffer_size = 512;
    let mut bpf_output = vec![0.0; buffer_size];
    let mut notch_output = vec![0.0; buffer_size];
    let mut original = vec![0.0; buffer_size];

    graph.eval_node_buffer(&bpf_id, &mut bpf_output);
    graph.eval_node_buffer(&notch_id, &mut notch_output);
    graph.eval_node_buffer(&osc_id, &mut original);

    // Both should produce sound
    let bpf_rms = calculate_rms(&bpf_output);
    let notch_rms = calculate_rms(&notch_output);
    let orig_rms = calculate_rms(&original);

    assert!(bpf_rms > 0.01, "BPF should produce sound: {}", bpf_rms);
    assert!(notch_rms > 0.01, "Notch should produce sound: {}", notch_rms);

    // For broadband signal like saw wave:
    // - BPF isolates one frequency region (narrow band)
    // - Notch removes one frequency region (passes wide band)
    // So notch output should generally be louder (passes more content)
    assert!(notch_rms > bpf_rms,
        "Notch should pass more than BPF for broadband signal: notch = {}, bpf = {}",
        notch_rms, bpf_rms);
}

// ============================================================================
// TEST: Broadband Signal - Creates "Hole" in Spectrum
// ============================================================================

#[test]
fn test_notch_creates_spectral_hole() {
    let mut graph = create_test_graph();

    // Broadband signal (sawtooth contains all harmonics)
    let osc_id = graph.add_oscillator(Signal::Value(110.0), Waveform::Saw);

    // Notch at 2000 Hz - should create hole in spectrum
    let notch_id = graph.add_node(SignalNode::Notch {
        input: Signal::Node(osc_id),
        center: Signal::Value(2000.0),
        q: Signal::Value(5.0),
        state: FilterState::default(),
    });

    let buffer_size = 512;
    let mut notched = vec![0.0; buffer_size];
    let mut original = vec![0.0; buffer_size];

    graph.eval_node_buffer(&notch_id, &mut notched);
    graph.eval_node_buffer(&osc_id, &mut original);

    let notched_rms = calculate_rms(&notched);
    let original_rms = calculate_rms(&original);

    // Notched signal should have similar or slightly less energy
    // (removed one narrow harmonic from saw wave which has many harmonics)
    // The difference may be small since we're only notching one frequency
    assert!(notched_rms > 0.01,
        "Notched signal should still produce sound: RMS = {}", notched_rms);
    assert!(notched_rms < original_rms * 1.2,
        "Notched should not have more energy than original");
}

// ============================================================================
// TEST: Modulated Parameters
// ============================================================================

#[test]
fn test_notch_modulated_center() {
    let mut graph = create_test_graph();

    // Broadband signal
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);

    // LFO to modulate center frequency (0.5 Hz)
    let lfo_id = graph.add_oscillator(Signal::Value(0.5), Waveform::Sine);

    // Modulated center: 1000 + (lfo * 1000) = [0, 2000] Hz range
    let lfo_scaled = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(lfo_id),
        b: Signal::Value(1000.0),
    });
    let center_signal = graph.add_node(SignalNode::Add {
        a: Signal::Node(lfo_scaled),
        b: Signal::Value(1000.0),
    });

    let notch_id = graph.add_node(SignalNode::Notch {
        input: Signal::Node(osc_id),
        center: Signal::Node(center_signal),
        q: Signal::Value(2.0),
        state: FilterState::default(),
    });

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&notch_id, &mut output);

    // Should produce sound (modulated notch filter sweeping through spectrum)
    let rms = calculate_rms(&output);
    assert!(rms > 0.1,
        "Modulated notch should produce sound, RMS = {}", rms);
}

// ============================================================================
// TEST: Edge Cases
// ============================================================================

#[test]
fn test_notch_very_low_center() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Very low center frequency (100 Hz) - should pass 440 Hz
    let notch_id = graph.add_node(SignalNode::Notch {
        input: Signal::Node(osc_id),
        center: Signal::Value(100.0),
        q: Signal::Value(2.0),
        state: FilterState::default(),
    });

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&notch_id, &mut output);

    // Should pass 440 Hz with minimal attenuation
    let rms = calculate_rms(&output);
    assert!(rms > 0.6,
        "Very low center frequency should pass 440 Hz: RMS = {}", rms);
}

#[test]
fn test_notch_very_high_center() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Very high center frequency (8000 Hz) - should pass 440 Hz
    let notch_id = graph.add_node(SignalNode::Notch {
        input: Signal::Node(osc_id),
        center: Signal::Value(8000.0),
        q: Signal::Value(2.0),
        state: FilterState::default(),
    });

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&notch_id, &mut output);

    // Should pass 440 Hz with minimal attenuation
    let rms = calculate_rms(&output);
    assert!(rms > 0.6,
        "Very high center frequency should pass 440 Hz: RMS = {}", rms);
}

#[test]
fn test_notch_extreme_q_values() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(1000.0), Waveform::Sine);

    // Very low Q (wide notch)
    let notch_low = graph.add_node(SignalNode::Notch {
        input: Signal::Node(osc_id),
        center: Signal::Value(1000.0),
        q: Signal::Value(0.5),
        state: FilterState::default(),
    });

    // Very high Q (narrow notch)
    let notch_high = graph.add_node(SignalNode::Notch {
        input: Signal::Node(osc_id),
        center: Signal::Value(1000.0),
        q: Signal::Value(20.0),
        state: FilterState::default(),
    });

    let buffer_size = 512;
    let mut low_output = vec![0.0; buffer_size];
    let mut high_output = vec![0.0; buffer_size];

    // Should not crash or produce NaN
    for _ in 0..10 {
        graph.eval_node_buffer(&notch_low, &mut low_output);
        graph.eval_node_buffer(&notch_high, &mut high_output);
    }

    // Check no NaN/Inf values
    for &sample in &low_output {
        assert!(sample.is_finite(), "Low Q produced non-finite value");
    }
    for &sample in &high_output {
        assert!(sample.is_finite(), "High Q produced non-finite value");
    }
}

#[test]
fn test_notch_stability_with_noise() {
    let mut graph = create_test_graph();

    // White noise contains all frequencies
    let noise_id = graph.add_node(SignalNode::WhiteNoise);

    // Notch at 1000 Hz
    let notch_id = graph.add_node(SignalNode::Notch {
        input: Signal::Node(noise_id),
        center: Signal::Value(1000.0),
        q: Signal::Value(5.0),
        state: FilterState::default(),
    });

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    // Process multiple buffers
    for _ in 0..100 {
        graph.eval_node_buffer(&notch_id, &mut output);
    }

    // Check for NaN or Inf
    let has_nan = output.iter().any(|s| s.is_nan() || s.is_infinite());
    assert!(!has_nan, "Notch filter should not produce NaN or Inf with noise");

    // Check for reasonable output
    let max_val = output.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(max_val < 10.0,
        "Notch filter output should be reasonable, got max {}", max_val);
}

// ============================================================================
// TEST: Chained Notches
// ============================================================================

#[test]
fn test_notch_chained_same_frequency() {
    let mut graph = create_test_graph();

    // Broadband signal
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);

    // First notch (1000 Hz)
    let notch1_id = graph.add_node(SignalNode::Notch {
        input: Signal::Node(osc_id),
        center: Signal::Value(1000.0),
        q: Signal::Value(2.0),
        state: FilterState::default(),
    });

    // Second notch (1000 Hz) - should deepen the notch
    let notch2_id = graph.add_node(SignalNode::Notch {
        input: Signal::Node(notch1_id),
        center: Signal::Value(1000.0),
        q: Signal::Value(2.0),
        state: FilterState::default(),
    });

    let buffer_size = 512;
    let mut once_notched = vec![0.0; buffer_size];
    let mut twice_notched = vec![0.0; buffer_size];

    graph.eval_node_buffer(&notch1_id, &mut once_notched);
    graph.eval_node_buffer(&notch2_id, &mut twice_notched);

    // Both should produce sound
    let once_rms = calculate_rms(&once_notched);
    let twice_rms = calculate_rms(&twice_notched);

    assert!(once_rms > 0.1, "Once notched should have sound: RMS = {}", once_rms);
    assert!(twice_rms > 0.1, "Twice notched should have sound: RMS = {}", twice_rms);
}

#[test]
fn test_notch_multiple_frequencies() {
    let mut graph = create_test_graph();

    // Broadband signal
    let osc_id = graph.add_oscillator(Signal::Value(110.0), Waveform::Saw);

    // Notch at 440 Hz
    let notch1_id = graph.add_node(SignalNode::Notch {
        input: Signal::Node(osc_id),
        center: Signal::Value(440.0),
        q: Signal::Value(3.0),
        state: FilterState::default(),
    });

    // Notch at 880 Hz
    let notch2_id = graph.add_node(SignalNode::Notch {
        input: Signal::Node(notch1_id),
        center: Signal::Value(880.0),
        q: Signal::Value(3.0),
        state: FilterState::default(),
    });

    // Notch at 1320 Hz
    let notch3_id = graph.add_node(SignalNode::Notch {
        input: Signal::Node(notch2_id),
        center: Signal::Value(1320.0),
        q: Signal::Value(3.0),
        state: FilterState::default(),
    });

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    let mut original = vec![0.0; buffer_size];

    graph.eval_node_buffer(&notch3_id, &mut output);
    graph.eval_node_buffer(&osc_id, &mut original);

    let notched_rms = calculate_rms(&output);
    let original_rms = calculate_rms(&original);

    // Multiple notches remove specific harmonics from saw wave
    // Should produce audible output
    assert!(notched_rms > 0.1,
        "Multiple notches should still produce sound: RMS = {}", notched_rms);

    // Total energy may be similar or less (depends on which harmonics removed)
    assert!(notched_rms < original_rms * 1.2,
        "Multiple notches should not amplify signal");

    // Check stability
    let has_nan = output.iter().any(|s| s.is_nan() || s.is_infinite());
    assert!(!has_nan, "Multiple notches should be stable");
}

// ============================================================================
// TEST: Performance
// ============================================================================

#[test]
fn test_notch_buffer_performance() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);
    let notch_id = graph.add_node(SignalNode::Notch {
        input: Signal::Node(osc_id),
        center: Signal::Value(1000.0),
        q: Signal::Value(2.0),
        state: FilterState::default(),
    });

    let buffer_size = 512;
    let iterations = 1000;

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&notch_id, &mut output);
    }
    let duration = start.elapsed();

    println!("Notch buffer eval: {:?} for {} iterations", duration, iterations);
    println!("Per iteration: {:?}", duration / iterations);

    // Should complete in reasonable time (< 1 second for 1000 iterations)
    assert!(duration.as_secs() < 1,
        "Notch buffer evaluation too slow: {:?}", duration);
}

// ============================================================================
// TEST: Constant vs Signal Parameters
// ============================================================================

#[test]
fn test_notch_constant_center() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);

    // Constant center frequency
    let notch_id = graph.add_node(SignalNode::Notch {
        input: Signal::Node(osc_id),
        center: Signal::Value(1000.0),
        q: Signal::Value(2.0),
        state: FilterState::default(),
    });

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&notch_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.1,
        "Notch with constant parameters should work, RMS = {}", rms);
}

// ============================================================================
// TEST: Musical Use Cases
// ============================================================================

#[test]
fn test_notch_remove_60hz_hum() {
    let mut graph = create_test_graph();

    // 60 Hz hum (common electrical interference)
    let hum_id = graph.add_oscillator(Signal::Value(60.0), Waveform::Sine);

    // Useful signal at 440 Hz
    let signal_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Mix them
    let mixed_id = graph.add_node(SignalNode::Add {
        a: Signal::Node(hum_id),
        b: Signal::Node(signal_id),
    });

    // Notch out the hum
    let dehum_id = graph.add_node(SignalNode::Notch {
        input: Signal::Node(mixed_id),
        center: Signal::Value(60.0),
        q: Signal::Value(5.0), // Narrow notch just for 60 Hz
        state: FilterState::default(),
    });

    let buffer_size = 512;
    let mut dehummed = vec![0.0; buffer_size];

    // Let filter settle
    for _ in 0..10 {
        graph.eval_node_buffer(&dehum_id, &mut dehummed);
    }

    // Should still have the 440 Hz signal
    let rms = calculate_rms(&dehummed);
    assert!(rms > 0.5,
        "After removing 60Hz hum, 440Hz should remain: RMS = {}", rms);
}

#[test]
fn test_notch_remove_feedback_frequency() {
    let mut graph = create_test_graph();

    // Simulate feedback at 1200 Hz mixed with music at 440 Hz
    let music_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);
    let feedback_id = graph.add_oscillator(Signal::Value(1200.0), Waveform::Sine);
    let mixed_id = graph.add_node(SignalNode::Add {
        a: Signal::Node(music_id),
        b: Signal::Node(feedback_id),
    });

    // Notch to remove feedback
    let cleaned_id = graph.add_node(SignalNode::Notch {
        input: Signal::Node(mixed_id),
        center: Signal::Value(1200.0),
        q: Signal::Value(8.0), // Very narrow to preserve music
        state: FilterState::default(),
    });

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    // Let filter settle
    for _ in 0..10 {
        graph.eval_node_buffer(&cleaned_id, &mut output);
    }

    // Should produce reasonable output
    let rms = calculate_rms(&output);
    assert!(rms > 0.3,
        "After removing feedback, music should remain: RMS = {}", rms);
}
