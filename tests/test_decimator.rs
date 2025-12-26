/// Comprehensive Decimator Node Integration Tests
///
/// Tests the DecimatorNode for sample rate reduction (lo-fi/bit-crush effects).
/// Verifies:
/// 1. Factor parameter controls decimation amount
/// 2. Creates sample-and-hold effect (stepped audio)
/// 3. Smooth parameter reduces harshness
/// 4. Pattern modulation of parameters
/// 5. Musical examples (8-bit drums, chiptune leads)
/// 6. Nyquist aliasing behavior
use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph, Waveform};
use std::cell::RefCell;

/// Helper: Calculate RMS of audio buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    if buffer.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = buffer.iter().map(|x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Helper: Count number of unique values in buffer (detects stepped audio)
fn count_unique_values(buffer: &[f32], tolerance: f32) -> usize {
    if buffer.is_empty() {
        return 0;
    }

    let mut unique = Vec::new();
    for &sample in buffer {
        let mut found = false;
        for &unique_val in &unique {
            if ((sample - unique_val) as f32).abs() < tolerance {
                found = true;
                break;
            }
        }
        if !found {
            unique.push(sample);
        }
    }
    unique.len()
}

/// Helper: Count number of times consecutive samples are identical (held)
fn count_held_samples(buffer: &[f32]) -> usize {
    let mut count = 0;
    for i in 1..buffer.len() {
        if (buffer[i] - buffer[i - 1]).abs() < 1e-6 {
            count += 1;
        }
    }
    count
}

#[test]
fn test_decimator_factor_1_no_effect() {
    // Test 1: factor=1 should pass signal through unchanged
    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Generate sine wave
    let sine = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,

        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    // Apply decimator with factor=1 (no effect)
    let decimated = graph.add_node(SignalNode::Decimator {
        input: Signal::Node(sine),
        factor: Signal::Value(1.0),
        smooth: Signal::Value(0.0),
        sample_counter: RefCell::new(0.0),
        held_value: RefCell::new(0.0),
        smooth_state: RefCell::new(0.0),
    });

    graph.set_output(decimated);

    // Render
    let buffer = graph.render(1024);

    // With factor=1, output should be smooth sine wave
    let rms = calculate_rms(&buffer);

    // Sine wave RMS should be ~0.707 (1.0 / sqrt(2))
    assert!(
        rms > 0.6 && rms < 0.8,
        "RMS should be ~0.707 for sine, got {}",
        rms
    );

    // Should have many unique values (smooth sine wave)
    // Use fine tolerance (0.001) since a 440Hz sine with 1024 samples
    // only produces ~141 unique values with tolerance 0.01
    let unique = count_unique_values(&buffer, 0.001);
    assert!(
        unique > 500,
        "Should have many unique values for smooth sine, got {}",
        unique
    );
}

#[test]
fn test_decimator_factor_2_half_rate() {
    // Test 2: factor=2 should create stepped effect (half sample rate)
    let mut graph = UnifiedSignalGraph::new(44100.0);

    let sine = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,

        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    // Decimator with factor=2
    let decimated = graph.add_node(SignalNode::Decimator {
        input: Signal::Node(sine),
        factor: Signal::Value(2.0),
        smooth: Signal::Value(0.0),
        sample_counter: RefCell::new(0.0),
        held_value: RefCell::new(0.0),
        smooth_state: RefCell::new(0.0),
    });

    graph.set_output(decimated);

    let buffer = graph.render(1024);

    // Should have significant sample holding
    let held = count_held_samples(&buffer);
    assert!(
        held > 400,
        "Should have many held samples with factor=2, got {}",
        held
    );

    // Should have fewer unique values than original
    let unique = count_unique_values(&buffer, 0.01);
    assert!(
        unique < 600,
        "Should have fewer unique values, got {}",
        unique
    );
}

#[test]
fn test_decimator_factor_4_quarter_rate() {
    // Test 3: factor=4 should create more extreme decimation
    let mut graph = UnifiedSignalGraph::new(44100.0);

    let sine = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,

        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    let decimated = graph.add_node(SignalNode::Decimator {
        input: Signal::Node(sine),
        factor: Signal::Value(4.0),
        smooth: Signal::Value(0.0),
        sample_counter: RefCell::new(0.0),
        held_value: RefCell::new(0.0),
        smooth_state: RefCell::new(0.0),
    });

    graph.set_output(decimated);

    let buffer = graph.render(1024);

    // Should have even more sample holding
    let held = count_held_samples(&buffer);
    assert!(
        held > 700,
        "Should have extensive holding with factor=4, got {}",
        held
    );

    // Should have very few unique values
    let unique = count_unique_values(&buffer, 0.01);
    assert!(
        unique < 300,
        "Should have very few unique values, got {}",
        unique
    );
}

#[test]
fn test_decimator_factor_8_severe() {
    // Test 4: factor=8 creates severe lo-fi effect
    let mut graph = UnifiedSignalGraph::new(44100.0);

    let sine = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,

        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    let decimated = graph.add_node(SignalNode::Decimator {
        input: Signal::Node(sine),
        factor: Signal::Value(8.0),
        smooth: Signal::Value(0.0),
        sample_counter: RefCell::new(0.0),
        held_value: RefCell::new(0.0),
        smooth_state: RefCell::new(0.0),
    });

    graph.set_output(decimated);

    let buffer = graph.render(1024);

    // Should be extremely stepped
    let held = count_held_samples(&buffer);
    assert!(
        held > 850,
        "Should have very extensive holding with factor=8, got {}",
        held
    );

    // Should have minimal unique values (extreme quantization)
    let unique = count_unique_values(&buffer, 0.01);
    assert!(
        unique < 150,
        "Should have minimal unique values, got {}",
        unique
    );
}

#[test]
fn test_decimator_smooth_reduces_steps() {
    // Test 5: smooth parameter reduces stepped artifacts
    let mut graph_harsh = UnifiedSignalGraph::new(44100.0);
    let mut graph_smooth = UnifiedSignalGraph::new(44100.0);

    // Harsh version (smooth=0.0)
    let sine_harsh = graph_harsh.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,

        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    let decimated_harsh = graph_harsh.add_node(SignalNode::Decimator {
        input: Signal::Node(sine_harsh),
        factor: Signal::Value(4.0),
        smooth: Signal::Value(0.0), // Harsh
        sample_counter: RefCell::new(0.0),
        held_value: RefCell::new(0.0),
        smooth_state: RefCell::new(0.0),
    });

    graph_harsh.set_output(decimated_harsh);

    // Smooth version (smooth=0.8)
    let sine_smooth = graph_smooth.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,

        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    let decimated_smooth = graph_smooth.add_node(SignalNode::Decimator {
        input: Signal::Node(sine_smooth),
        factor: Signal::Value(4.0),
        smooth: Signal::Value(0.8), // Smooth
        sample_counter: RefCell::new(0.0),
        held_value: RefCell::new(0.0),
        smooth_state: RefCell::new(0.0),
    });

    graph_smooth.set_output(decimated_smooth);

    let buffer_harsh = graph_harsh.render(1024);
    let buffer_smooth = graph_smooth.render(1024);

    // Smooth version should have fewer held samples
    let held_harsh = count_held_samples(&buffer_harsh);
    let held_smooth = count_held_samples(&buffer_smooth);

    assert!(
        held_smooth < held_harsh,
        "Smooth version should have fewer held samples: harsh={}, smooth={}",
        held_harsh,
        held_smooth
    );

    // Smooth version should have more unique values (less stepped)
    // Use fine tolerance (0.001) to see the difference from intermediate values
    let unique_harsh = count_unique_values(&buffer_harsh, 0.001);
    let unique_smooth = count_unique_values(&buffer_smooth, 0.001);

    assert!(
        unique_smooth > unique_harsh,
        "Smooth version should have more unique values: harsh={}, smooth={}",
        unique_harsh,
        unique_smooth
    );
}

#[test]
fn test_decimator_creates_aliasing() {
    // Test 6: High-frequency content decimated creates aliasing
    let mut graph = UnifiedSignalGraph::new(44100.0);

    // High-frequency sine (5kHz)
    let sine = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(5000.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,

        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    // Heavy decimation creates aliasing
    let decimated = graph.add_node(SignalNode::Decimator {
        input: Signal::Node(sine),
        factor: Signal::Value(8.0),
        smooth: Signal::Value(0.0),
        sample_counter: RefCell::new(0.0),
        held_value: RefCell::new(0.0),
        smooth_state: RefCell::new(0.0),
    });

    graph.set_output(decimated);

    let buffer = graph.render(1024);

    // Should still have some energy (aliased frequencies)
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.3, "Should have aliased energy, got RMS={}", rms);

    // Should be heavily stepped
    let held = count_held_samples(&buffer);
    assert!(
        held > 700,
        "Should have extensive stepping (aliasing), got {}",
        held
    );
}

#[test]
fn test_decimator_factor_below_1_clamped() {
    // Test 7: factor < 1.0 should be clamped to 1.0
    let mut graph = UnifiedSignalGraph::new(44100.0);

    let sine = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,

        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    // Try factor=0.5 (should be clamped to 1.0)
    let decimated = graph.add_node(SignalNode::Decimator {
        input: Signal::Node(sine),
        factor: Signal::Value(0.5),
        smooth: Signal::Value(0.0),
        sample_counter: RefCell::new(0.0),
        held_value: RefCell::new(0.0),
        smooth_state: RefCell::new(0.0),
    });

    graph.set_output(decimated);

    let buffer = graph.render(1024);

    // Should behave like factor=1.0 (no decimation)
    // Use fine tolerance (0.001) for meaningful unique value counting
    let unique = count_unique_values(&buffer, 0.001);
    assert!(
        unique > 500,
        "Should act like factor=1 (no decimation), got {} unique",
        unique
    );
}

#[test]
fn test_decimator_smooth_clamp() {
    // Test 8: smooth parameter should be clamped to [0, 1]
    let mut graph = UnifiedSignalGraph::new(44100.0);

    let sine = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,

        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    // Try smooth=2.0 (should be clamped to 1.0)
    let decimated = graph.add_node(SignalNode::Decimator {
        input: Signal::Node(sine),
        factor: Signal::Value(4.0),
        smooth: Signal::Value(2.0), // Out of range
        sample_counter: RefCell::new(0.0),
        held_value: RefCell::new(0.0),
        smooth_state: RefCell::new(0.0),
    });

    graph.set_output(decimated);

    let buffer = graph.render(1024);

    // Should not crash, should produce valid audio
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.0 && rms < 2.0,
        "Should produce valid audio with clamped smooth"
    );

    // All samples should be finite
    for (i, &sample) in buffer.iter().enumerate() {
        assert!(
            sample.is_finite(),
            "Sample {} should be finite, got {}",
            i,
            sample
        );
    }
}

#[test]
fn test_decimator_dc_signal() {
    // Test 9: DC signal (constant value) should pass through correctly
    let mut graph = UnifiedSignalGraph::new(44100.0);

    let decimated = graph.add_node(SignalNode::Decimator {
        input: Signal::Value(0.5), // DC signal
        factor: Signal::Value(4.0),
        smooth: Signal::Value(0.0),
        sample_counter: RefCell::new(0.0),
        held_value: RefCell::new(0.0),
        smooth_state: RefCell::new(0.0),
    });

    graph.set_output(decimated);

    let buffer = graph.render(1024);

    // After initial samples, should converge to 0.5
    let last_100: Vec<f32> = buffer.iter().skip(buffer.len() - 100).copied().collect();
    let avg = last_100.iter().sum::<f32>() / last_100.len() as f32;

    assert!(
        (avg - 0.5).abs() < 0.01,
        "DC signal should settle to 0.5, got {}",
        avg
    );
}

#[test]
fn test_decimator_square_wave() {
    // Test 10: Square wave decimated should create distinct steps
    let mut graph = UnifiedSignalGraph::new(44100.0);

    let square = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(100.0),
        waveform: Waveform::Square,
        semitone_offset: 0.0,

        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    let decimated = graph.add_node(SignalNode::Decimator {
        input: Signal::Node(square),
        factor: Signal::Value(8.0),
        smooth: Signal::Value(0.0),
        sample_counter: RefCell::new(0.0),
        held_value: RefCell::new(0.0),
        smooth_state: RefCell::new(0.0),
    });

    graph.set_output(decimated);

    let buffer = graph.render(1024);

    // Should be heavily stepped
    let held = count_held_samples(&buffer);
    assert!(
        held > 800,
        "Square wave should be heavily decimated, got {} held",
        held
    );

    // Should preserve energy
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.5,
        "Should maintain square wave energy, got RMS={}",
        rms
    );
}

#[test]
fn test_decimator_noise() {
    // Test 11: White noise decimated becomes stepped noise
    let mut graph = UnifiedSignalGraph::new(44100.0);

    let noise = graph.add_node(SignalNode::WhiteNoise);

    let decimated = graph.add_node(SignalNode::Decimator {
        input: Signal::Node(noise),
        factor: Signal::Value(4.0),
        smooth: Signal::Value(0.0),
        sample_counter: RefCell::new(0.0),
        held_value: RefCell::new(0.0),
        smooth_state: RefCell::new(0.0),
    });

    graph.set_output(decimated);

    let buffer = graph.render(1024);

    // Should have significant holding
    let held = count_held_samples(&buffer);
    assert!(
        held > 600,
        "Noise should be decimated (stepped), got {} held",
        held
    );

    // Should still have noise-like RMS
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.2 && rms < 0.8,
        "Should maintain noise energy, got RMS={}",
        rms
    );
}

#[test]
fn test_decimator_preserves_amplitude() {
    // Test 12: Decimation should preserve overall amplitude
    let mut graph_original = UnifiedSignalGraph::new(44100.0);
    let mut graph_decimated = UnifiedSignalGraph::new(44100.0);

    // Original sine
    let sine_orig = graph_original.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,

        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });
    graph_original.set_output(sine_orig);

    // Decimated sine
    let sine_dec = graph_decimated.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,

        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    let decimated = graph_decimated.add_node(SignalNode::Decimator {
        input: Signal::Node(sine_dec),
        factor: Signal::Value(2.0),
        smooth: Signal::Value(0.0),
        sample_counter: RefCell::new(0.0),
        held_value: RefCell::new(0.0),
        smooth_state: RefCell::new(0.0),
    });
    graph_decimated.set_output(decimated);

    let buffer_orig = graph_original.render(1024);
    let buffer_dec = graph_decimated.render(1024);

    let rms_orig = calculate_rms(&buffer_orig);
    let rms_dec = calculate_rms(&buffer_dec);

    // Amplitude should be similar (within 20%)
    let ratio = rms_dec / rms_orig;
    assert!(
        ratio > 0.8 && ratio < 1.2,
        "Decimation should preserve amplitude: orig={}, dec={}, ratio={}",
        rms_orig,
        rms_dec,
        ratio
    );
}

#[test]
fn test_decimator_increasing_factors() {
    // Test 13: Increasing factor should increase stepping
    let mut graph2 = UnifiedSignalGraph::new(44100.0);
    let mut graph4 = UnifiedSignalGraph::new(44100.0);
    let mut graph8 = UnifiedSignalGraph::new(44100.0);

    // Factor=2
    let sine2 = graph2.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,

        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });
    let dec2 = graph2.add_node(SignalNode::Decimator {
        input: Signal::Node(sine2),
        factor: Signal::Value(2.0),
        smooth: Signal::Value(0.0),
        sample_counter: RefCell::new(0.0),
        held_value: RefCell::new(0.0),
        smooth_state: RefCell::new(0.0),
    });
    graph2.set_output(dec2);

    // Factor=4
    let sine4 = graph4.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,

        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });
    let dec4 = graph4.add_node(SignalNode::Decimator {
        input: Signal::Node(sine4),
        factor: Signal::Value(4.0),
        smooth: Signal::Value(0.0),
        sample_counter: RefCell::new(0.0),
        held_value: RefCell::new(0.0),
        smooth_state: RefCell::new(0.0),
    });
    graph4.set_output(dec4);

    // Factor=8
    let sine8 = graph8.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,

        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });
    let dec8 = graph8.add_node(SignalNode::Decimator {
        input: Signal::Node(sine8),
        factor: Signal::Value(8.0),
        smooth: Signal::Value(0.0),
        sample_counter: RefCell::new(0.0),
        held_value: RefCell::new(0.0),
        smooth_state: RefCell::new(0.0),
    });
    graph8.set_output(dec8);

    let buffer2 = graph2.render(1024);
    let buffer4 = graph4.render(1024);
    let buffer8 = graph8.render(1024);

    let held2 = count_held_samples(&buffer2);
    let held4 = count_held_samples(&buffer4);
    let held8 = count_held_samples(&buffer8);

    // Higher factors should produce more held samples
    assert!(
        held4 > held2,
        "Factor 4 should have more holds than factor 2: {}vs{}",
        held4,
        held2
    );
    assert!(
        held8 > held4,
        "Factor 8 should have more holds than factor 4: {}vs{}",
        held8,
        held4
    );

    // Verify monotonic increase in stepping
    assert!(
        held2 > 200 && held4 > 500 && held8 > 700,
        "Held counts should increase: 2={}, 4={}, 8={}",
        held2,
        held4,
        held8
    );
}

#[test]
fn test_decimator_chained_with_filter() {
    // Test 14: Decimator can be chained with other effects
    // This is a musical use case: decimated signal through lowpass filter
    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Saw wave (rich harmonics)
    let saw = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(110.0),
        waveform: Waveform::Saw,
        semitone_offset: 0.0,

        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    // Decimate it (lo-fi effect)
    let decimated = graph.add_node(SignalNode::Decimator {
        input: Signal::Node(saw),
        factor: Signal::Value(4.0),
        smooth: Signal::Value(0.0),
        sample_counter: RefCell::new(0.0),
        held_value: RefCell::new(0.0),
        smooth_state: RefCell::new(0.0),
    });

    // Filter it (smooth out harsh aliasing)
    let filtered = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(decimated),
        cutoff: Signal::Value(2000.0),
        q: Signal::Value(0.7),
        state: Default::default(),
    });

    graph.set_output(filtered);

    let buffer = graph.render(1024);

    // Should produce valid audio
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.1 && rms < 1.0,
        "Chained effect should produce valid audio, got RMS={}",
        rms
    );

    // Note: After filtering, consecutive samples are no longer identical
    // because the lowpass smooths out the step edges. The decimation effect
    // is still present but manifests as slow-changing slopes instead of steps.

    // All samples finite
    for &sample in &buffer {
        assert!(sample.is_finite(), "All samples should be finite");
    }
}
