use phonon::unified_graph::{LimiterState, NodeId, Signal, SignalNode, UnifiedSignalGraph, Waveform};
use std::rc::Rc;

/// Helper to create a test graph
fn create_test_graph() -> UnifiedSignalGraph {
    UnifiedSignalGraph::new(44100.0)
}

/// Helper to calculate RMS
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_of_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_of_squares / buffer.len() as f32).sqrt()
}

/// Helper to find peak value
fn find_peak(buffer: &[f32]) -> f32 {
    buffer.iter().map(|&x| x.abs()).fold(0.0, f32::max)
}

#[test]
fn test_limiter_prevents_clipping() {
    let mut graph = create_test_graph();

    // Create loud signal (amplitude = 2.0)
    let osc_id = NodeId(graph.nodes.len());
    let osc_node = SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    };
    graph.nodes.push(Some(Rc::new(osc_node)));

    let loud_id = NodeId(graph.nodes.len());
    let loud_node = SignalNode::Multiply {
        a: Signal::Node(osc_id),
        b: Signal::Value(2.0),
    };
    graph.nodes.push(Some(Rc::new(loud_node)));

    // Limit to 1.0
    let limiter_id = NodeId(graph.nodes.len());
    let limiter_node = SignalNode::Limiter {
        input: Signal::Node(loud_id),
        threshold: Signal::Value(1.0),
        release: Signal::Value(0.01),
        state: LimiterState::new(),
    };
    graph.nodes.push(Some(Rc::new(limiter_node)));

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    // Process multiple buffers to let limiter settle
    for _ in 0..10 {
        graph.eval_node_buffer(&limiter_id, &mut output);
    }

    // All samples should be within threshold
    let peak = find_peak(&output);
    assert!(peak <= 1.0 + 0.01,
        "Peak exceeds threshold: {}", peak);
}

#[test]
fn test_limiter_passthrough_quiet_signal() {
    let mut graph = create_test_graph();

    // Create quiet signal (amplitude = 0.5)
    let osc_id = NodeId(graph.nodes.len());
    let osc_node = SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    };
    graph.nodes.push(Some(Rc::new(osc_node)));

    let quiet_id = NodeId(graph.nodes.len());
    let quiet_node = SignalNode::Multiply {
        a: Signal::Node(osc_id),
        b: Signal::Value(0.5),
    };
    graph.nodes.push(Some(Rc::new(quiet_node)));

    // Limit to 1.0 (should not affect quiet signal)
    let limiter_id = NodeId(graph.nodes.len());
    let limiter_node = SignalNode::Limiter {
        input: Signal::Node(quiet_id),
        threshold: Signal::Value(1.0),
        release: Signal::Value(0.01),
        state: LimiterState::new(),
    };
    graph.nodes.push(Some(Rc::new(limiter_node)));

    let buffer_size = 512;
    let mut limited = vec![0.0; buffer_size];
    let mut original = vec![0.0; buffer_size];

    graph.eval_node_buffer(&limiter_id, &mut limited);
    graph.eval_node_buffer(&quiet_id, &mut original);

    // Should be approximately equal (within 5% since limiter may take time to reach unity gain)
    let rms_limited = calculate_rms(&limited);
    let rms_original = calculate_rms(&original);

    assert!((rms_limited - rms_original).abs() / rms_original < 0.05,
        "Limiter affecting quiet signal: {} vs {}", rms_limited, rms_original);
}

#[test]
fn test_limiter_release_time() {
    let mut graph = create_test_graph();

    // Create loud signal
    let osc_id = NodeId(graph.nodes.len());
    let osc_node = SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    };
    graph.nodes.push(Some(Rc::new(osc_node)));

    let loud_id = NodeId(graph.nodes.len());
    let loud_node = SignalNode::Multiply {
        a: Signal::Node(osc_id),
        b: Signal::Value(2.0),
    };
    graph.nodes.push(Some(Rc::new(loud_node)));

    // Fast release (0.001 seconds)
    let fast_limiter_id = NodeId(graph.nodes.len());
    let fast_limiter_node = SignalNode::Limiter {
        input: Signal::Node(loud_id),
        threshold: Signal::Value(1.0),
        release: Signal::Value(0.001),
        state: LimiterState::new(),
    };
    graph.nodes.push(Some(Rc::new(fast_limiter_node)));

    // Slow release (0.1 seconds)
    let slow_limiter_id = NodeId(graph.nodes.len());
    let slow_limiter_node = SignalNode::Limiter {
        input: Signal::Node(loud_id),
        threshold: Signal::Value(1.0),
        release: Signal::Value(0.1),
        state: LimiterState::new(),
    };
    graph.nodes.push(Some(Rc::new(slow_limiter_node)));

    let buffer_size = 512;
    let mut fast_output = vec![0.0; buffer_size];
    let mut slow_output = vec![0.0; buffer_size];

    // Process buffers
    for _ in 0..5 {
        graph.eval_node_buffer(&fast_limiter_id, &mut fast_output);
        graph.eval_node_buffer(&slow_limiter_id, &mut slow_output);
    }

    // Fast release should have higher RMS (recovers faster)
    let fast_rms = calculate_rms(&fast_output);
    let slow_rms = calculate_rms(&slow_output);

    assert!(fast_rms > slow_rms,
        "Fast release should have higher RMS: {} vs {}", fast_rms, slow_rms);
}

#[test]
fn test_limiter_state_persistence() {
    let mut graph = create_test_graph();

    // Create loud signal
    let osc_id = NodeId(graph.nodes.len());
    let osc_node = SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    };
    graph.nodes.push(Some(Rc::new(osc_node)));

    let loud_id = NodeId(graph.nodes.len());
    let loud_node = SignalNode::Multiply {
        a: Signal::Node(osc_id),
        b: Signal::Value(2.0),
    };
    graph.nodes.push(Some(Rc::new(loud_node)));

    // Limiter with slow release
    let limiter_id = NodeId(graph.nodes.len());
    let limiter_node = SignalNode::Limiter {
        input: Signal::Node(loud_id),
        threshold: Signal::Value(1.0),
        release: Signal::Value(0.1),
        state: LimiterState::new(),
    };
    graph.nodes.push(Some(Rc::new(limiter_node)));

    let buffer_size = 512;
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];
    let mut buffer3 = vec![0.0; buffer_size];

    // Process multiple buffers
    graph.eval_node_buffer(&limiter_id, &mut buffer1);
    graph.eval_node_buffer(&limiter_id, &mut buffer2);
    graph.eval_node_buffer(&limiter_id, &mut buffer3);

    // RMS should gradually increase as limiter releases
    let rms1 = calculate_rms(&buffer1);
    let rms2 = calculate_rms(&buffer2);
    let rms3 = calculate_rms(&buffer3);

    assert!(rms1 < rms2 && rms2 < rms3,
        "RMS should increase as limiter releases: {} < {} < {}", rms1, rms2, rms3);
}

#[test]
fn test_limiter_instant_attack() {
    let mut graph = create_test_graph();

    // Create signal that goes from quiet to loud
    let osc_id = NodeId(graph.nodes.len());
    let osc_node = SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    };
    graph.nodes.push(Some(Rc::new(osc_node)));

    let loud_id = NodeId(graph.nodes.len());
    let loud_node = SignalNode::Multiply {
        a: Signal::Node(osc_id),
        b: Signal::Value(3.0),
    };
    graph.nodes.push(Some(Rc::new(loud_node)));

    // Limiter
    let limiter_id = NodeId(graph.nodes.len());
    let limiter_node = SignalNode::Limiter {
        input: Signal::Node(loud_id),
        threshold: Signal::Value(1.0),
        release: Signal::Value(0.01),
        state: LimiterState::new(),
    };
    graph.nodes.push(Some(Rc::new(limiter_node)));

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    // Process one buffer
    graph.eval_node_buffer(&limiter_id, &mut output);

    // Even the first buffer should be limited (instant attack)
    let peak = find_peak(&output);
    assert!(peak <= 1.0 + 0.01,
        "Peak should be limited immediately: {}", peak);
}

#[test]
fn test_limiter_threshold_variation() {
    let mut graph = create_test_graph();

    // Create loud signal
    let osc_id = NodeId(graph.nodes.len());
    let osc_node = SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    };
    graph.nodes.push(Some(Rc::new(osc_node)));

    let loud_id = NodeId(graph.nodes.len());
    let loud_node = SignalNode::Multiply {
        a: Signal::Node(osc_id),
        b: Signal::Value(2.0),
    };
    graph.nodes.push(Some(Rc::new(loud_node)));

    // Low threshold
    let low_thresh_id = NodeId(graph.nodes.len());
    let low_thresh_node = SignalNode::Limiter {
        input: Signal::Node(loud_id),
        threshold: Signal::Value(0.5),
        release: Signal::Value(0.01),
        state: LimiterState::new(),
    };
    graph.nodes.push(Some(Rc::new(low_thresh_node)));

    // High threshold
    let high_thresh_id = NodeId(graph.nodes.len());
    let high_thresh_node = SignalNode::Limiter {
        input: Signal::Node(loud_id),
        threshold: Signal::Value(1.5),
        release: Signal::Value(0.01),
        state: LimiterState::new(),
    };
    graph.nodes.push(Some(Rc::new(high_thresh_node)));

    let buffer_size = 512;
    let mut low_output = vec![0.0; buffer_size];
    let mut high_output = vec![0.0; buffer_size];

    // Process buffers
    for _ in 0..5 {
        graph.eval_node_buffer(&low_thresh_id, &mut low_output);
        graph.eval_node_buffer(&high_thresh_id, &mut high_output);
    }

    // Low threshold should have lower RMS
    let low_rms = calculate_rms(&low_output);
    let high_rms = calculate_rms(&high_output);

    assert!(low_rms < high_rms,
        "Lower threshold should produce lower RMS: {} vs {}", low_rms, high_rms);

    // Check peaks match thresholds
    let low_peak = find_peak(&low_output);
    let high_peak = find_peak(&high_output);

    assert!(low_peak <= 0.5 + 0.01, "Low threshold peak too high: {}", low_peak);
    assert!(high_peak <= 1.5 + 0.01, "High threshold peak too high: {}", high_peak);
}

#[test]
fn test_limiter_handles_silence() {
    let mut graph = create_test_graph();

    // Create constant silence
    let silence_id = NodeId(graph.nodes.len());
    let silence_node = SignalNode::Constant { value: 0.0 };
    graph.nodes.push(Some(Rc::new(silence_node)));

    // Limiter
    let limiter_id = NodeId(graph.nodes.len());
    let limiter_node = SignalNode::Limiter {
        input: Signal::Node(silence_id),
        threshold: Signal::Value(1.0),
        release: Signal::Value(0.01),
        state: LimiterState::new(),
    };
    graph.nodes.push(Some(Rc::new(limiter_node)));

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    // Process buffers
    graph.eval_node_buffer(&limiter_id, &mut output);

    // Should remain silent
    let rms = calculate_rms(&output);
    assert_eq!(rms, 0.0, "Silence should remain silent");
}

#[test]
fn test_limiter_symmetric_limiting() {
    let mut graph = create_test_graph();

    // Create loud signal
    let osc_id = NodeId(graph.nodes.len());
    let osc_node = SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    };
    graph.nodes.push(Some(Rc::new(osc_node)));

    let loud_id = NodeId(graph.nodes.len());
    let loud_node = SignalNode::Multiply {
        a: Signal::Node(osc_id),
        b: Signal::Value(2.0),
    };
    graph.nodes.push(Some(Rc::new(loud_node)));

    // Limiter
    let limiter_id = NodeId(graph.nodes.len());
    let limiter_node = SignalNode::Limiter {
        input: Signal::Node(loud_id),
        threshold: Signal::Value(1.0),
        release: Signal::Value(0.01),
        state: LimiterState::new(),
    };
    graph.nodes.push(Some(Rc::new(limiter_node)));

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    // Process buffers
    for _ in 0..10 {
        graph.eval_node_buffer(&limiter_id, &mut output);
    }

    // Find maximum positive and negative values
    let max_positive = output.iter().cloned().fold(0.0, f32::max);
    let max_negative = output.iter().cloned().fold(0.0, f32::min);

    // Should be symmetric (both limited to ±1.0)
    assert!((max_positive - max_negative.abs()).abs() < 0.1,
        "Limiting should be symmetric: +{} vs -{}", max_positive, max_negative.abs());
}
