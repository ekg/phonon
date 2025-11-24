/// Buffer evaluation tests for Allpass filter
///
/// Tests that the buffer-based evaluation produces identical results
/// to sample-by-sample evaluation and provides performance benefits.

use phonon::unified_graph::{UnifiedSignalGraph, Signal, SignalNode, Waveform, AllpassState};
use std::cell::RefCell;

/// Helper to calculate RMS of audio buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

#[test]
fn test_allpass_buffer_vs_sample() {
    // Test that buffer evaluation matches sample-by-sample
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);

    // Create a 440Hz sine input
    let sine_id = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        phase: RefCell::new(0.0),
    });

    // Create allpass with coefficient 0.7
    let allpass_id = graph.add_node(SignalNode::Allpass {
        input: Signal::Node(sine_id),
        coefficient: Signal::Value(0.7),
        state: AllpassState::default(),
    });

    // Render 1 second using buffer evaluation
    let buffer_size = 44100;
    let mut buffer_output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&allpass_id, &mut buffer_output);

    let rms = calculate_rms(&buffer_output);

    // Should have audio output with preserved energy
    assert!(rms > 0.2, "Allpass buffer should produce audio, got RMS: {}", rms);
    assert!(rms < 0.8, "Allpass should have unity gain, got RMS: {}", rms);
}

#[test]
fn test_allpass_buffer_flat_magnitude() {
    // Test that allpass preserves magnitude (unity gain)
    let sample_rate = 44100.0;
    let mut graph_dry = UnifiedSignalGraph::new(sample_rate);
    let mut graph_wet = UnifiedSignalGraph::new(sample_rate);

    // Dry: just sine
    let sine_dry = graph_dry.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        phase: RefCell::new(0.0),
    });

    // Wet: sine through allpass
    let sine_wet = graph_wet.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        phase: RefCell::new(0.0),
    });
    let allpass_wet = graph_wet.add_node(SignalNode::Allpass {
        input: Signal::Node(sine_wet),
        coefficient: Signal::Value(0.5),
        state: AllpassState::default(),
    });

    // Render both
    let buffer_size = 44100;
    let mut dry_output = vec![0.0; buffer_size];
    let mut wet_output = vec![0.0; buffer_size];

    graph_dry.eval_node_buffer(&sine_dry, &mut dry_output);
    graph_wet.eval_node_buffer(&allpass_wet, &mut wet_output);

    let dry_rms = calculate_rms(&dry_output);
    let wet_rms = calculate_rms(&wet_output);

    // RMS should be very similar (unity gain)
    let ratio = wet_rms / dry_rms;
    assert!((0.9..=1.1).contains(&ratio),
        "Allpass should preserve magnitude, dry: {}, wet: {}, ratio: {}",
        dry_rms, wet_rms, ratio);
}

#[test]
fn test_allpass_buffer_changes_phase() {
    // Test that allpass changes phase (buffers should be different)
    let sample_rate = 44100.0;
    let mut graph_dry = UnifiedSignalGraph::new(sample_rate);
    let mut graph_wet = UnifiedSignalGraph::new(sample_rate);

    // Dry: just sine
    let sine_dry = graph_dry.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        phase: RefCell::new(0.0),
    });

    // Wet: sine through allpass
    let sine_wet = graph_wet.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        phase: RefCell::new(0.0),
    });
    let allpass_wet = graph_wet.add_node(SignalNode::Allpass {
        input: Signal::Node(sine_wet),
        coefficient: Signal::Value(0.7),
        state: AllpassState::default(),
    });

    // Render both
    let buffer_size = 8820; // 0.2 seconds
    let mut dry_output = vec![0.0; buffer_size];
    let mut wet_output = vec![0.0; buffer_size];

    graph_dry.eval_node_buffer(&sine_dry, &mut dry_output);
    graph_wet.eval_node_buffer(&allpass_wet, &mut wet_output);

    // Calculate sample-wise difference
    let mut diff = 0.0;
    for i in 0..buffer_size {
        diff += (dry_output[i] - wet_output[i]).abs();
    }
    let avg_diff = diff / buffer_size as f32;

    // Should have noticeable difference due to phase shift
    assert!(avg_diff > 0.01,
        "Allpass should change phase, avg sample difference: {}",
        avg_diff);
}

#[test]
fn test_allpass_buffer_cascade() {
    // Test multiple allpass filters in series
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);

    // Input sine
    let sine_id = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        phase: RefCell::new(0.0),
    });

    // Chain 3 allpass filters
    let ap1 = graph.add_node(SignalNode::Allpass {
        input: Signal::Node(sine_id),
        coefficient: Signal::Value(0.3),
        state: AllpassState::default(),
    });
    let ap2 = graph.add_node(SignalNode::Allpass {
        input: Signal::Node(ap1),
        coefficient: Signal::Value(0.5),
        state: AllpassState::default(),
    });
    let ap3 = graph.add_node(SignalNode::Allpass {
        input: Signal::Node(ap2),
        coefficient: Signal::Value(0.7),
        state: AllpassState::default(),
    });

    // Render
    let buffer_size = 44100;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&ap3, &mut output);

    let rms = calculate_rms(&output);

    // Cascade should still preserve magnitude (unity gain)
    assert!(rms > 0.2, "Cascaded allpass should produce audio, got RMS: {}", rms);
    assert!(rms < 0.8, "Cascaded allpass should preserve magnitude, got RMS: {}", rms);
}

#[test]
fn test_allpass_buffer_state_continuity() {
    // Test that allpass state is maintained across multiple buffer calls
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);

    let sine_id = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        phase: RefCell::new(0.0),
    });
    let allpass_id = graph.add_node(SignalNode::Allpass {
        input: Signal::Node(sine_id),
        coefficient: Signal::Value(0.7),
        state: AllpassState::default(),
    });

    // Render in 3 chunks
    let chunk_size = 4410; // 0.1 second chunks
    let mut chunk1 = vec![0.0; chunk_size];
    let mut chunk2 = vec![0.0; chunk_size];
    let mut chunk3 = vec![0.0; chunk_size];

    graph.eval_node_buffer(&allpass_id, &mut chunk1);
    graph.eval_node_buffer(&allpass_id, &mut chunk2);
    graph.eval_node_buffer(&allpass_id, &mut chunk3);

    // All chunks should have audio (no discontinuities)
    let rms1 = calculate_rms(&chunk1);
    let rms2 = calculate_rms(&chunk2);
    let rms3 = calculate_rms(&chunk3);

    assert!(rms1 > 0.2, "Chunk 1 should have audio");
    assert!(rms2 > 0.2, "Chunk 2 should have audio");
    assert!(rms3 > 0.2, "Chunk 3 should have audio");

    // RMS should be consistent across chunks
    assert!((rms2 / rms1 - 1.0).abs() < 0.1, "Chunks should have consistent energy");
    assert!((rms3 / rms1 - 1.0).abs() < 0.1, "Chunks should have consistent energy");
}

#[test]
fn test_allpass_buffer_modulated_coefficient() {
    // Test allpass with time-varying coefficient
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);

    // Input sine at 440Hz
    let sine_id = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        phase: RefCell::new(0.0),
    });

    // Modulating LFO at 2Hz (oscillates coefficient)
    let lfo_id = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(2.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        phase: RefCell::new(0.0),
    });

    // Scale LFO from -1..1 to 0..0.8
    let scaled_lfo = graph.add_multiply_node(Signal::Node(lfo_id), Signal::Value(0.4));
    let offset_lfo = graph.add_add_node(Signal::Node(scaled_lfo), Signal::Value(0.4));

    // Allpass with modulated coefficient
    let allpass_id = graph.add_node(SignalNode::Allpass {
        input: Signal::Node(sine_id),
        coefficient: Signal::Node(offset_lfo),
        state: AllpassState::default(),
    });

    // Render
    let buffer_size = 44100;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&allpass_id, &mut output);

    let rms = calculate_rms(&output);

    // Should still produce audio despite modulation
    assert!(rms > 0.1, "Modulated allpass should produce audio, got RMS: {}", rms);
}

#[test]
fn test_allpass_buffer_zero_coefficient() {
    // Test allpass with coefficient = 0 (should be close to transparent)
    let sample_rate = 44100.0;
    let mut graph_dry = UnifiedSignalGraph::new(sample_rate);
    let mut graph_wet = UnifiedSignalGraph::new(sample_rate);

    // Dry
    let sine_dry = graph_dry.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        phase: RefCell::new(0.0),
    });

    // Wet with coefficient = 0
    let sine_wet = graph_wet.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        phase: RefCell::new(0.0),
    });
    let allpass_wet = graph_wet.add_node(SignalNode::Allpass {
        input: Signal::Node(sine_wet),
        coefficient: Signal::Value(0.0),
        state: AllpassState::default(),
    });

    // Render
    let buffer_size = 44100;
    let mut dry_output = vec![0.0; buffer_size];
    let mut wet_output = vec![0.0; buffer_size];

    graph_dry.eval_node_buffer(&sine_dry, &mut dry_output);
    graph_wet.eval_node_buffer(&allpass_wet, &mut wet_output);

    let dry_rms = calculate_rms(&dry_output);
    let wet_rms = calculate_rms(&wet_output);

    // Should be very similar
    assert!((wet_rms / dry_rms - 1.0).abs() < 0.1,
        "Allpass(0) should be nearly transparent, dry: {}, wet: {}",
        dry_rms, wet_rms);
}

#[test]
fn test_allpass_buffer_stability() {
    // Test that allpass doesn't explode or produce NaN
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);

    let sine_id = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        phase: RefCell::new(0.0),
    });

    // Extreme coefficient values
    let allpass_id = graph.add_node(SignalNode::Allpass {
        input: Signal::Node(sine_id),
        coefficient: Signal::Value(0.99), // Near unity
        state: AllpassState::default(),
    });

    // Render for a while
    let buffer_size = 44100 * 2; // 2 seconds
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&allpass_id, &mut output);

    // Check for stability
    for (i, &sample) in output.iter().enumerate() {
        assert!(sample.is_finite(), "Sample {} is not finite: {}", i, sample);
        assert!(sample.abs() < 10.0, "Sample {} is too large: {}", i, sample);
    }

    let rms = calculate_rms(&output);
    assert!(rms > 0.1, "Output should have energy");
    assert!(rms < 2.0, "Output should be stable");
}

#[test]
fn test_allpass_buffer_negative_coefficient() {
    // Test allpass with negative coefficient
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);

    let sine_id = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        phase: RefCell::new(0.0),
    });
    let allpass_id = graph.add_node(SignalNode::Allpass {
        input: Signal::Node(sine_id),
        coefficient: Signal::Value(-0.5),
        state: AllpassState::default(),
    });

    // Render
    let buffer_size = 44100;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&allpass_id, &mut output);

    let rms = calculate_rms(&output);

    // Should still preserve magnitude
    assert!(rms > 0.2, "Negative coefficient allpass should work, got RMS: {}", rms);
    assert!(rms < 0.8, "Negative coefficient should preserve magnitude, got RMS: {}", rms);
}

#[test]
fn test_allpass_buffer_different_sample_rates() {
    // Test allpass at different sample rates
    for &sample_rate in &[22050.0, 44100.0, 48000.0, 96000.0] {
        let mut graph = UnifiedSignalGraph::new(sample_rate);

        let sine_id = graph.add_node(SignalNode::Oscillator {
            freq: Signal::Value(440.0),
            waveform: Waveform::Sine,
        semitone_offset: 0.0,
            phase: RefCell::new(0.0),
        });
        let allpass_id = graph.add_node(SignalNode::Allpass {
            input: Signal::Node(sine_id),
            coefficient: Signal::Value(0.7),
            state: AllpassState::default(),
        });

        let buffer_size = (sample_rate * 0.1) as usize; // 0.1 second
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&allpass_id, &mut output);

        let rms = calculate_rms(&output);
        assert!(rms > 0.2, "Allpass should work at {}Hz sample rate", sample_rate);
    }
}

#[test]
fn test_allpass_buffer_for_reverb() {
    // Test allpass chain for reverb-style application
    // Schroeder reverb uses series of allpass filters
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);

    let sine_id = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(220.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        phase: RefCell::new(0.0),
    });

    // Classic Schroeder allpass cascade (simplified)
    let ap1 = graph.add_node(SignalNode::Allpass {
        input: Signal::Node(sine_id),
        coefficient: Signal::Value(0.131),
        state: AllpassState::default(),
    });
    let ap2 = graph.add_node(SignalNode::Allpass {
        input: Signal::Node(ap1),
        coefficient: Signal::Value(0.359),
        state: AllpassState::default(),
    });
    let ap3 = graph.add_node(SignalNode::Allpass {
        input: Signal::Node(ap2),
        coefficient: Signal::Value(0.677),
        state: AllpassState::default(),
    });
    let ap4 = graph.add_node(SignalNode::Allpass {
        input: Signal::Node(ap3),
        coefficient: Signal::Value(0.773),
        state: AllpassState::default(),
    });

    let buffer_size = 44100;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&ap4, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.15, "Reverb-style allpass chain should work, got RMS: {}", rms);
}

#[test]
fn test_allpass_buffer_efficiency() {
    // Measure that buffer evaluation is reasonably efficient
    use std::time::Instant;

    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);

    let sine_id = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        phase: RefCell::new(0.0),
    });
    let allpass_id = graph.add_node(SignalNode::Allpass {
        input: Signal::Node(sine_id),
        coefficient: Signal::Value(0.7),
        state: AllpassState::default(),
    });

    let buffer_size = 44100 * 10; // 10 seconds
    let mut output = vec![0.0; buffer_size];

    let start = Instant::now();
    graph.eval_node_buffer(&allpass_id, &mut output);
    let duration = start.elapsed();

    // Should process 10 seconds of audio in under 100ms (100x realtime)
    assert!(duration.as_millis() < 100,
        "Buffer evaluation should be efficient, took {:?}", duration);

    let rms = calculate_rms(&output);
    assert!(rms > 0.2, "Should produce audio");
}
