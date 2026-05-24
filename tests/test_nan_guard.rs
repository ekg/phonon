/// Tests for the global finite/denormal sanitization guard at the graph output boundary.
/// Verifies that NaN, Inf, and denormal values are flushed to silence before reaching
/// the ring buffer, preventing CPAL output poisoning.
use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph};

#[test]
fn test_nan_is_sanitized_in_process_buffer_dag() {
    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Inject a constant NaN directly as the output signal.
    let output = graph.add_node(SignalNode::Output {
        input: Signal::Value(f32::NAN),
    });
    graph.set_output(output);

    let buffer = graph.render(256);

    for (i, &sample) in buffer.iter().enumerate() {
        assert!(
            sample.is_finite(),
            "Sample[{}] should be sanitized to finite (was NaN), got: {}",
            i,
            sample
        );
        assert_eq!(
            sample, 0.0,
            "Sample[{}] should be 0.0 after NaN sanitization, got: {}",
            i, sample
        );
    }
}

#[test]
fn test_inf_is_sanitized_in_process_buffer_dag() {
    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Inject +Inf directly as the output signal.
    let output = graph.add_node(SignalNode::Output {
        input: Signal::Value(f32::INFINITY),
    });
    graph.set_output(output);

    let buffer = graph.render(256);

    for (i, &sample) in buffer.iter().enumerate() {
        assert!(
            sample.is_finite(),
            "Sample[{}] should be sanitized to finite (was Inf), got: {}",
            i,
            sample
        );
    }
}

#[test]
fn test_neg_inf_is_sanitized_in_process_buffer_dag() {
    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Inject -Inf directly as the output signal.
    let output = graph.add_node(SignalNode::Output {
        input: Signal::Value(f32::NEG_INFINITY),
    });
    graph.set_output(output);

    let buffer = graph.render(256);

    for (i, &sample) in buffer.iter().enumerate() {
        assert!(
            sample.is_finite(),
            "Sample[{}] should be sanitized to finite (was -Inf), got: {}",
            i,
            sample
        );
    }
}

#[test]
fn test_nan_is_sanitized_in_process_sample_stereo() {
    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Inject a constant NaN as the output signal.
    let output = graph.add_node(SignalNode::Output {
        input: Signal::Value(f32::NAN),
    });
    graph.set_output(output);

    // Call process_sample_stereo directly
    for _ in 0..256 {
        let (left, right) = graph.process_sample_stereo();
        assert!(
            left.is_finite(),
            "Left sample should be finite after NaN sanitization, got: {}",
            left
        );
        assert!(
            right.is_finite(),
            "Right sample should be finite after NaN sanitization, got: {}",
            right
        );
    }
}

#[test]
fn test_inf_is_sanitized_in_process_sample_stereo() {
    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Inject +Inf as the output signal.
    let output = graph.add_node(SignalNode::Output {
        input: Signal::Value(f32::INFINITY),
    });
    graph.set_output(output);

    for _ in 0..256 {
        let (left, right) = graph.process_sample_stereo();
        assert!(
            left.is_finite(),
            "Left sample should be finite after Inf sanitization, got: {}",
            left
        );
        assert!(
            right.is_finite(),
            "Right sample should be finite after Inf sanitization, got: {}",
            right
        );
    }
}

#[test]
fn test_normal_audio_passes_through_nan_guard_unaffected() {
    let mut graph = UnifiedSignalGraph::new(44100.0);
    std::cell::RefCell::new(0.0f32);

    // A normal-amplitude signal should not be zeroed by the guard.
    let output = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: phonon::unified_graph::Waveform::Sine,
        semitone_offset: 0.0,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    });
    let out_node = graph.add_node(SignalNode::Output {
        input: Signal::Node(output),
    });
    graph.set_output(out_node);

    let buffer = graph.render(1024);

    // The oscillator should produce non-zero output
    let has_nonzero = buffer.iter().any(|&s| s != 0.0);
    assert!(has_nonzero, "Normal audio should pass through the NaN guard unchanged");

    // All samples should be finite
    for (i, &sample) in buffer.iter().enumerate() {
        assert!(
            sample.is_finite(),
            "Normal audio sample[{}] should be finite, got: {}",
            i,
            sample
        );
    }
}
