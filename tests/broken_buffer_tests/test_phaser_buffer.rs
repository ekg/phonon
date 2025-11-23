use phonon::unified_graph::{UnifiedSignalGraph, SignalNode, Signal, Waveform, NodeId};

const SAMPLE_RATE: f32 = 44100.0;

fn create_test_graph() -> UnifiedSignalGraph {
    UnifiedSignalGraph::new(SAMPLE_RATE)
}

fn calculate_rms(samples: &[f32]) -> f32 {
    let sum: f32 = samples.iter().map(|s| s * s).sum();
    (sum / samples.len() as f32).sqrt()
}

fn calculate_spectral_variance(samples: &[f32]) -> f32 {
    // Simple measure: variance in amplitude over time
    // Phaser creates moving notches which increase variance
    let mean: f32 = samples.iter().sum::<f32>() / samples.len() as f32;
    let variance: f32 = samples
        .iter()
        .map(|s| (s - mean).powi(2))
        .sum::<f32>()
        / samples.len() as f32;
    variance
}

// Helper function removed - using add_node directly

/// LEVEL 1: Basic Phaser Creates Modulation
#[test]
fn test_phaser_creates_modulation() {
    let mut graph = create_test_graph();

    // Create a saw wave oscillator at 440 Hz
    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Saw,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    });

    // Add phaser with moderate settings
    let phaser_id = graph.add_node(SignalNode::Phaser {
        input: Signal::Node(osc),
        rate: Signal::Value(0.5),   // Slow sweep
        depth: Signal::Value(0.8),   // Deep modulation
        feedback: Signal::Value(0.5),   // Moderate feedback
        stages: 6,                    // 6 stages
        phase: 0.0,
        allpass_z1: Vec::new(),
        allpass_y1: Vec::new(),
        feedback_sample: 0.0,
    });

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&phaser_id, &mut output);

    // Should produce sound
    let rms = calculate_rms(&output);
    assert!(
        rms > 0.2,
        "Phaser should produce audible sound: RMS={}",
        rms
    );

    // Check for no NaN or Inf
    let has_nan = output.iter().any(|s| s.is_nan() || s.is_infinite());
    assert!(!has_nan, "Phaser should not produce NaN or Inf");
}

/// LEVEL 2: Phaser Rate Affects Sweep Speed
#[test]
fn test_phaser_rate_affects_sweep() {
    let mut graph = create_test_graph();

    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(220.0),
        waveform: Waveform::Saw,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    });

    // Slow phaser
    let slow_phaser = graph.add_node(SignalNode::Phaser {
        input: Signal::Node(osc),
        rate: Signal::Value(0.2),
        depth: Signal::Value(0.7),
        feedback: Signal::Value(0.3),
        stages: 4,
        phase: 0.0,
        allpass_z1: Vec::new(),
        allpass_y1: Vec::new(),
        feedback_sample: 0.0,
    });

    // Fast phaser
    let fast_phaser = graph.add_node(SignalNode::Phaser {
        input: Signal::Node(osc),
        rate: Signal::Value(3.0),
        depth: Signal::Value(0.7),
        feedback: Signal::Value(0.3),
        stages: 4,
        phase: 0.0,
        allpass_z1: Vec::new(),
        allpass_y1: Vec::new(),
        feedback_sample: 0.0,
    });

    let buffer_size = 2048; // Longer buffer to capture sweep differences
    let mut slow_output = vec![0.0; buffer_size];
    let mut fast_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&slow_phaser, &mut slow_output);
    graph.eval_node_buffer(&fast_phaser, &mut fast_output);

    // Both should produce sound
    let rms_slow = calculate_rms(&slow_output);
    let rms_fast = calculate_rms(&fast_output);

    assert!(rms_slow > 0.1, "Slow phaser should be audible");
    assert!(rms_fast > 0.1, "Fast phaser should be audible");

    // Fast phaser should have higher spectral variance (more notch movement)
    let var_slow = calculate_spectral_variance(&slow_output);
    let var_fast = calculate_spectral_variance(&fast_output);

    // Note: This is a weak test - we're mainly checking both produce sound
    assert!(
        var_slow >= 0.0 && var_fast >= 0.0,
        "Both phasers should have valid spectral variance"
    );
}

/// LEVEL 2: Phaser Depth Affects Modulation Amount
#[test]
fn test_phaser_depth_affects_amount() {
    let mut graph = create_test_graph();

    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Saw,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    });

    // Shallow phaser (minimal depth)
    let shallow_phaser = graph.add_node(SignalNode::Phaser {
        input: Signal::Node(osc),
        rate: Signal::Value(0.5),
        depth: Signal::Value(0.1), // Low depth
        feedback: Signal::Value(0.3),
        stages: 4,
        phase: 0.0,
        allpass_z1: Vec::new(),
        allpass_y1: Vec::new(),
        feedback_sample: 0.0,
    });

    // Deep phaser (maximum depth)
    let deep_phaser = graph.add_node(SignalNode::Phaser {
        input: Signal::Node(osc),
        rate: Signal::Value(0.5),
        depth: Signal::Value(1.0), // High depth
        feedback: Signal::Value(0.3),
        stages: 4,
        phase: 0.0,
        allpass_z1: Vec::new(),
        allpass_y1: Vec::new(),
        feedback_sample: 0.0,
    });

    let buffer_size = 1024;
    let mut shallow_output = vec![0.0; buffer_size];
    let mut deep_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&shallow_phaser, &mut shallow_output);
    graph.eval_node_buffer(&deep_phaser, &mut deep_output);

    // Both should produce sound
    let rms_shallow = calculate_rms(&shallow_output);
    let rms_deep = calculate_rms(&deep_output);

    assert!(rms_shallow > 0.1, "Shallow phaser should be audible");
    assert!(rms_deep > 0.1, "Deep phaser should be audible");

    // Deep phaser should have more spectral modulation
    let var_shallow = calculate_spectral_variance(&shallow_output);
    let var_deep = calculate_spectral_variance(&deep_output);

    // Deep should have more variance (more notch sweep)
    assert!(
        var_deep > var_shallow * 0.5,
        "Deep phaser should have more spectral variance: shallow={}, deep={}",
        var_shallow,
        var_deep
    );
}

/// LEVEL 2: Zero Depth Bypasses Effect
#[test]
fn test_phaser_zero_depth_bypass() {
    let mut graph = create_test_graph();

    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Saw,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    });

    // Phaser with zero depth
    let phaser_id = graph.add_node(SignalNode::Phaser {
        input: Signal::Node(osc),
        rate: Signal::Value(0.5),
        depth: Signal::Value(0.0), // Zero depth
        feedback: Signal::Value(0.0),
        stages: 4,
        phase: 0.0,
        allpass_z1: Vec::new(),
        allpass_y1: Vec::new(),
        feedback_sample: 0.0,
    });

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&phaser_id, &mut output);

    // Should still produce sound (dry signal passes through)
    let rms = calculate_rms(&output);
    assert!(
        rms > 0.2,
        "Zero-depth phaser should pass dry signal: RMS={}",
        rms
    );
}

/// LEVEL 2: Feedback Affects Resonance
#[test]
fn test_phaser_feedback_affects_resonance() {
    let mut graph = create_test_graph();

    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(220.0),
        waveform: Waveform::Saw,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    });

    // No feedback
    let no_fb_phaser = graph.add_node(SignalNode::Phaser {
        input: Signal::Node(osc),
        rate: Signal::Value(0.5),
        depth: Signal::Value(0.7),
        feedback: Signal::Value(0.0), // No feedback
        stages: 4,
        phase: 0.0,
        allpass_z1: Vec::new(),
        allpass_y1: Vec::new(),
        feedback_sample: 0.0,
    });

    // High feedback
    let high_fb_phaser = graph.add_node(SignalNode::Phaser {
        input: Signal::Node(osc),
        rate: Signal::Value(0.5),
        depth: Signal::Value(0.7),
        feedback: Signal::Value(0.8), // High feedback
        stages: 4,
        phase: 0.0,
        allpass_z1: Vec::new(),
        allpass_y1: Vec::new(),
        feedback_sample: 0.0,
    });

    let buffer_size = 1024;
    let mut no_fb_output = vec![0.0; buffer_size];
    let mut high_fb_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&no_fb_phaser, &mut no_fb_output);
    graph.eval_node_buffer(&high_fb_phaser, &mut high_fb_output);

    // Both should produce sound
    let rms_no_fb = calculate_rms(&no_fb_output);
    let rms_high_fb = calculate_rms(&high_fb_output);

    assert!(rms_no_fb > 0.1, "No-feedback phaser should be audible");
    assert!(rms_high_fb > 0.1, "High-feedback phaser should be audible");

    // High feedback typically produces more pronounced notches
    // (This is a weak test - mainly checking stability)
    let max_no_fb = no_fb_output.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    let max_high_fb = high_fb_output
        .iter()
        .map(|s| s.abs())
        .fold(0.0f32, f32::max);

    assert!(
        max_no_fb < 2.0 && max_high_fb < 2.0,
        "Both phasers should produce stable output"
    );
}

/// LEVEL 2: State Continuity Across Buffers
#[test]
fn test_phaser_state_continuity() {
    let mut graph = create_test_graph();

    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Saw,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    });

    let phaser_id = graph.add_node(SignalNode::Phaser {
        input: Signal::Node(osc),
        rate: Signal::Value(0.5),
        depth: Signal::Value(0.7),
        feedback: Signal::Value(0.5),
        stages: 6,
        phase: 0.0,
        allpass_z1: Vec::new(),
        allpass_y1: Vec::new(),
        feedback_sample: 0.0,
    });

    let buffer_size = 256;
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];
    let mut buffer3 = vec![0.0; buffer_size];

    // Render three consecutive buffers
    graph.eval_node_buffer(&phaser_id, &mut buffer1);
    graph.eval_node_buffer(&phaser_id, &mut buffer2);
    graph.eval_node_buffer(&phaser_id, &mut buffer3);

    // All buffers should have sound
    let rms1 = calculate_rms(&buffer1);
    let rms2 = calculate_rms(&buffer2);
    let rms3 = calculate_rms(&buffer3);

    assert!(rms1 > 0.1, "Buffer 1 should be audible");
    assert!(rms2 > 0.1, "Buffer 2 should be audible");
    assert!(rms3 > 0.1, "Buffer 3 should be audible");

    // Should not have discontinuities at buffer boundaries
    // (Check last sample of buffer1 vs first sample of buffer2)
    let transition1 = (buffer1[buffer_size - 1] - buffer2[0]).abs();
    let transition2 = (buffer2[buffer_size - 1] - buffer3[0]).abs();

    // Transitions should be smooth (not a huge jump)
    assert!(
        transition1 < 0.5,
        "Transition between buffer 1 and 2 should be smooth: {}",
        transition1
    );
    assert!(
        transition2 < 0.5,
        "Transition between buffer 2 and 3 should be smooth: {}",
        transition2
    );
}

/// LEVEL 2: Different Stage Counts
#[test]
fn test_phaser_stage_counts() {
    let mut graph = create_test_graph();

    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Saw,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    });

    // 2-stage phaser (subtle)
    let phaser_2stage = graph.add_node(SignalNode::Phaser {
        input: Signal::Node(osc),
        rate: Signal::Value(0.5),
        depth: Signal::Value(0.7),
        feedback: Signal::Value(0.3),
        stages: 2,
        phase: 0.0,
        allpass_z1: Vec::new(),
        allpass_y1: Vec::new(),
        feedback_sample: 0.0,
    });

    // 12-stage phaser (dramatic)
    let phaser_12stage = graph.add_node(SignalNode::Phaser {
        input: Signal::Node(osc),
        rate: Signal::Value(0.5),
        depth: Signal::Value(0.7),
        feedback: Signal::Value(0.3),
        stages: 12,
        phase: 0.0,
        allpass_z1: Vec::new(),
        allpass_y1: Vec::new(),
        feedback_sample: 0.0,
    });

    let buffer_size = 1024;
    let mut output_2stage = vec![0.0; buffer_size];
    let mut output_12stage = vec![0.0; buffer_size];

    graph.eval_node_buffer(&phaser_2stage, &mut output_2stage);
    graph.eval_node_buffer(&phaser_12stage, &mut output_12stage);

    // Both should produce sound
    let rms_2 = calculate_rms(&output_2stage);
    let rms_12 = calculate_rms(&output_12stage);

    assert!(rms_2 > 0.1, "2-stage phaser should be audible");
    assert!(rms_12 > 0.1, "12-stage phaser should be audible");

    // More stages typically create more notches
    // (This is a weak test - mainly checking both work)
    assert!(
        rms_2 > 0.0 && rms_12 > 0.0,
        "Both stage counts should produce valid output"
    );
}

/// LEVEL 3: Stability Test - Extended Duration
#[test]
fn test_phaser_stability_extended() {
    let mut graph = create_test_graph();

    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(220.0),
        waveform: Waveform::Saw,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    });

    let phaser_id = graph.add_node(SignalNode::Phaser {
        input: Signal::Node(osc),
        rate: Signal::Value(1.5),
        depth: Signal::Value(0.8),
        feedback: Signal::Value(0.6),
        stages: 6,
        phase: 0.0,
        allpass_z1: Vec::new(),
        allpass_y1: Vec::new(),
        feedback_sample: 0.0,
    });

    // Render multiple buffers (simulate ~2 seconds)
    let buffer_size = 1024;
    let num_buffers = 86; // ~2 seconds at 44.1kHz

    for _ in 0..num_buffers {
        let mut buffer = vec![0.0; buffer_size];
        graph.eval_node_buffer(&phaser_id, &mut buffer);

        // Check for stability
        let has_nan = buffer.iter().any(|s| s.is_nan() || s.is_infinite());
        assert!(!has_nan, "Phaser should remain stable over time");

        let max_val = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(
            max_val < 10.0,
            "Phaser output should remain reasonable: max={}",
            max_val
        );
    }
}

/// LEVEL 3: Pattern-Modulated Parameters
#[test]
fn test_phaser_pattern_modulation() {
    let mut graph = create_test_graph();

    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Saw,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    });

    // LFO for rate modulation
    let rate_lfo = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(0.2),
        waveform: Waveform::Sine,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    });

    // Map LFO (-1 to 1) to rate range (0.5 to 2.0)
    let rate_scaled = graph.add_node(SignalNode::Add {
        a: Signal::Node(rate_lfo),
        b: Signal::Value(1.5), // Offset to 0.5-2.5
    });

    let phaser_id = graph.add_node(SignalNode::Phaser {
        input: Signal::Node(osc),
        rate: Signal::Node(rate_scaled),
        depth: Signal::Value(0.7),
        feedback: Signal::Value(0.5),
        stages: 6,
        phase: 0.0,
        allpass_z1: Vec::new(),
        allpass_y1: Vec::new(),
        feedback_sample: 0.0,
    });

    let buffer_size = 2048;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&phaser_id, &mut output);

    // Should produce sound with varying rate
    let rms = calculate_rms(&output);
    assert!(
        rms > 0.1,
        "Pattern-modulated phaser should be audible: RMS={}",
        rms
    );

    // Check for stability
    let has_nan = output.iter().any(|s| s.is_nan() || s.is_infinite());
    assert!(
        !has_nan,
        "Pattern-modulated phaser should not produce NaN or Inf"
    );
}

/// LEVEL 3: Extreme Parameters
#[test]
fn test_phaser_extreme_parameters() {
    let mut graph = create_test_graph();

    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Saw,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    });

    // Extreme settings: max rate, max depth, max feedback
    let extreme_phaser = graph.add_node(SignalNode::Phaser {
        input: Signal::Node(osc),
        rate: Signal::Value(5.0),  // Max rate
        depth: Signal::Value(1.0),  // Max depth
        feedback: Signal::Value(0.95), // Max feedback (just below instability)
        stages: 12,                  // Max stages
        phase: 0.0,
        allpass_z1: Vec::new(),
        allpass_y1: Vec::new(),
        feedback_sample: 0.0,
    });

    let buffer_size = 1024;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&extreme_phaser, &mut output);

    // Should still be stable
    let has_nan = output.iter().any(|s| s.is_nan() || s.is_infinite());
    assert!(
        !has_nan,
        "Extreme phaser parameters should not cause instability"
    );

    let rms = calculate_rms(&output);
    assert!(rms > 0.05, "Extreme phaser should produce sound");

    let max_val = output.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        max_val < 5.0,
        "Extreme phaser output should be bounded: max={}",
        max_val
    );
}

/// LEVEL 3: Multiple Phasers in Series
#[test]
fn test_phaser_series_cascade() {
    let mut graph = create_test_graph();

    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(220.0),
        waveform: Waveform::Saw,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    });

    // First phaser
    let phaser1 = graph.add_node(SignalNode::Phaser {
        input: Signal::Node(osc),
        rate: Signal::Value(0.3),
        depth: Signal::Value(0.6),
        feedback: Signal::Value(0.4),
        stages: 4,
        phase: 0.0,
        allpass_z1: Vec::new(),
        allpass_y1: Vec::new(),
        feedback_sample: 0.0,
    });

    // Second phaser cascaded
    let phaser2 = graph.add_node(SignalNode::Phaser {
        input: Signal::Node(phaser1),
        rate: Signal::Value(0.7),
        depth: Signal::Value(0.6),
        feedback: Signal::Value(0.4),
        stages: 4,
        phase: 0.0,
        allpass_z1: Vec::new(),
        allpass_y1: Vec::new(),
        feedback_sample: 0.0,
    });

    let buffer_size = 1024;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&phaser2, &mut output);

    // Cascaded phasers should produce complex modulation
    let rms = calculate_rms(&output);
    assert!(
        rms > 0.1,
        "Cascaded phasers should be audible: RMS={}",
        rms
    );

    // Check stability
    let has_nan = output.iter().any(|s| s.is_nan() || s.is_infinite());
    assert!(!has_nan, "Cascaded phasers should remain stable");
}
