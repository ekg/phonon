use phonon::mini_notation_v3::parse_mini_notation;
use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph};
use std::collections::HashMap;

/// Test that attack parameter creates gradual fade-in
#[test]
fn test_attack_parameter_shapes_onset() {
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(2.0); // 2 cycles per second = 500ms per cycle

    // Test with 100ms attack time (position 5)
    let pattern = parse_mini_notation("bd");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd".to_string(),
        pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(1.0),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        n: Signal::Value(0.0),
        note: Signal::Value(0.0),
        attack: Signal::Value(0.1),   // 100ms attack
        release: Signal::Value(0.05), // 50ms release
        envelope_type: None,
    });

    graph.set_output_channel(1, sample_node);

    // Collect first 8820 samples (200ms at 44.1kHz)
    let mut samples = Vec::new();
    for _ in 0..8820 {
        let outputs = graph.process_sample_multi();
        samples.push(outputs[0]);
    }

    // Find the first non-zero sample (attack start)
    let mut attack_start = 0;
    for (i, &sample) in samples.iter().enumerate() {
        if sample.abs() > 0.001 {
            attack_start = i;
            break;
        }
    }

    // Verify gradual attack: amplitude should increase over attack period
    // Sample at 25ms after attack start should be less than at 75ms
    let sample_25ms = attack_start + (sample_rate * 0.025) as usize;
    let sample_75ms = attack_start + (sample_rate * 0.075) as usize;

    if sample_25ms < samples.len() && sample_75ms < samples.len() {
        let amp_25ms = samples[sample_25ms].abs();
        let amp_75ms = samples[sample_75ms].abs();

        assert!(
            amp_25ms < amp_75ms,
            "Attack envelope should increase over time: {}ms={:.4} should be < {}ms={:.4}",
            25,
            amp_25ms,
            75,
            amp_75ms
        );
    }
}

/// Test that release parameter controls tail length
#[test]
fn test_release_parameter_controls_tail() {
    let sample_rate = 44100.0;

    // Test 1: Short release (10ms)
    let mut graph1 = UnifiedSignalGraph::new(sample_rate);
    graph1.set_cps(2.0);

    let pattern1 = parse_mini_notation("bd");
    let sample_node1 = graph1.add_node(SignalNode::Sample {
        pattern_str: "bd".to_string(),
        pattern: pattern1,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(1.0),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        n: Signal::Value(0.0),
        note: Signal::Value(0.0),
        attack: Signal::Value(0.001), // 1ms attack
        release: Signal::Value(0.01), // 10ms release (short)
        envelope_type: None,
    });
    graph1.set_output_channel(1, sample_node1);

    // Test 2: Long release (200ms)
    let mut graph2 = UnifiedSignalGraph::new(sample_rate);
    graph2.set_cps(2.0);

    let pattern2 = parse_mini_notation("bd");
    let sample_node2 = graph2.add_node(SignalNode::Sample {
        pattern_str: "bd".to_string(),
        pattern: pattern2,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(1.0),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        n: Signal::Value(0.0),
        note: Signal::Value(0.0),
        attack: Signal::Value(0.001), // 1ms attack
        release: Signal::Value(0.2),  // 200ms release (long)
        envelope_type: None,
    });
    graph2.set_output_channel(1, sample_node2);

    // Process enough samples to capture release tails
    let num_samples = (sample_rate * 0.3) as usize; // 300ms
    let mut samples1 = Vec::new();
    let mut samples2 = Vec::new();

    for _ in 0..num_samples {
        samples1.push(graph1.process_sample_multi()[0]);
        samples2.push(graph2.process_sample_multi()[0]);
    }

    // Find last non-zero sample for each (end of release)
    let mut tail_end1 = 0;
    let mut tail_end2 = 0;

    for i in (0..samples1.len()).rev() {
        if samples1[i].abs() > 0.0001 {
            tail_end1 = i;
            break;
        }
    }

    for i in (0..samples2.len()).rev() {
        if samples2[i].abs() > 0.0001 {
            tail_end2 = i;
            break;
        }
    }

    // Convert to milliseconds
    let tail1_ms = (tail_end1 as f32 / sample_rate) * 1000.0;
    let tail2_ms = (tail_end2 as f32 / sample_rate) * 1000.0;

    // Long release should have significantly longer tail
    assert!(
        tail2_ms > tail1_ms + 50.0,
        "Long release (200ms) should produce longer tail than short release (10ms): tail1={:.1}ms, tail2={:.1}ms",
        tail1_ms, tail2_ms
    );
}

/// Test attack with fast vs slow attack times
#[test]
fn test_fast_vs_slow_attack() {
    let sample_rate = 44100.0;

    // Fast attack (1ms)
    let mut graph_fast = UnifiedSignalGraph::new(sample_rate);
    graph_fast.set_cps(2.0);

    let pattern_fast = parse_mini_notation("bd");
    let sample_node_fast = graph_fast.add_node(SignalNode::Sample {
        pattern_str: "bd".to_string(),
        pattern: pattern_fast,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(1.0),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        n: Signal::Value(0.0),
        note: Signal::Value(0.0),
        attack: Signal::Value(0.001), // 1ms attack (fast)
        release: Signal::Value(0.05),
        envelope_type: None,
    });
    graph_fast.set_output_channel(1, sample_node_fast);

    // Slow attack (150ms)
    let mut graph_slow = UnifiedSignalGraph::new(sample_rate);
    graph_slow.set_cps(2.0);

    let pattern_slow = parse_mini_notation("bd");
    let sample_node_slow = graph_slow.add_node(SignalNode::Sample {
        pattern_str: "bd".to_string(),
        pattern: pattern_slow,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(1.0),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        n: Signal::Value(0.0),
        note: Signal::Value(0.0),
        attack: Signal::Value(0.15), // 150ms attack (slow)
        release: Signal::Value(0.05),
        envelope_type: None,
    });
    graph_slow.set_output_channel(1, sample_node_slow);

    // Collect samples
    let num_samples = (sample_rate * 0.05) as usize; // 50ms
    let mut samples_fast = Vec::new();
    let mut samples_slow = Vec::new();

    for _ in 0..num_samples {
        samples_fast.push(graph_fast.process_sample_multi()[0]);
        samples_slow.push(graph_slow.process_sample_multi()[0]);
    }

    // Calculate RMS for first 10ms after trigger
    let window = (sample_rate * 0.01) as usize;
    let mut rms_fast = 0.0;
    let mut rms_slow = 0.0;

    for i in 0..window.min(samples_fast.len()) {
        rms_fast += samples_fast[i] * samples_fast[i];
        rms_slow += samples_slow[i] * samples_slow[i];
    }

    rms_fast = (rms_fast / window as f32).sqrt();
    rms_slow = (rms_slow / window as f32).sqrt();

    // Fast attack should have higher RMS in first 10ms
    assert!(
        rms_fast > rms_slow * 1.5,
        "Fast attack (1ms) should have higher initial RMS than slow attack (150ms): fast={:.4}, slow={:.4}",
        rms_fast, rms_slow
    );
}

/// Test default envelope behavior (0.0 values)
#[test]
fn test_default_envelope_values() {
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(2.0);

    // Use 0.0 for attack and release (should use defaults)
    let pattern = parse_mini_notation("bd");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd".to_string(),
        pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(1.0),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        n: Signal::Value(0.0),
        note: Signal::Value(0.0),
        attack: Signal::Value(0.0),  // Default attack
        release: Signal::Value(0.0), // Default release
        envelope_type: None,
    });

    graph.set_output_channel(1, sample_node);

    // Process samples - should not crash and should produce audio
    let mut has_audio = false;
    for _ in 0..4410 {
        // 100ms
        let outputs = graph.process_sample_multi();
        if outputs[0].abs() > 0.01 {
            has_audio = true;
        }
    }

    assert!(has_audio, "Default envelope values should produce audio");
}

/// Test envelope interaction with gain parameter
#[test]
fn test_envelope_gain_interaction() {
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(2.0);

    // Use gain=0.5 with envelope
    let pattern = parse_mini_notation("bd");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd".to_string(),
        pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(0.5), // Half gain
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        n: Signal::Value(0.0),
        note: Signal::Value(0.0),
        attack: Signal::Value(0.01),  // 10ms attack
        release: Signal::Value(0.05), // 50ms release
        envelope_type: None,
    });

    graph.set_output_channel(1, sample_node);

    // Collect samples
    let mut samples = Vec::new();
    for _ in 0..8820 {
        // 200ms
        let outputs = graph.process_sample_multi();
        samples.push(outputs[0]);
    }

    // Find peak amplitude
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    // Peak should be reduced by gain (approximately 0.5, allowing for sample data variation)
    assert!(
        peak < 0.7,
        "Peak amplitude with gain=0.5 should be reduced: peak={:.4}",
        peak
    );
    assert!(
        peak > 0.1,
        "Should still have significant audio with gain=0.5: peak={:.4}",
        peak
    );
}

/// Test multiple events with different envelope times
#[test]
fn test_multiple_events_different_envelopes() {
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(4.0); // 4 cycles/sec = 250ms per cycle

    // Pattern with 2 events per cycle
    let pattern = parse_mini_notation("bd sn");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd sn".to_string(),
        pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(1.0),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        n: Signal::Value(0.0),
        note: Signal::Value(0.0),
        attack: Signal::Value(0.01), // 10ms attack
        release: Signal::Value(0.1), // 100ms release
        envelope_type: None,
    });

    graph.set_output_channel(1, sample_node);

    // Process 2 full cycles (500ms)
    let num_samples = (sample_rate * 0.5) as usize;
    let mut samples = Vec::new();

    for _ in 0..num_samples {
        let outputs = graph.process_sample_multi();
        samples.push(outputs[0]);
    }

    // Count number of events (peaks)
    let mut event_count = 0;
    let mut in_event = false;
    let threshold = 0.05;

    for &sample in &samples {
        if sample.abs() > threshold && !in_event {
            event_count += 1;
            in_event = true;
        } else if sample.abs() <= threshold {
            in_event = false;
        }
    }

    // Should have at least 2 events in 2 cycles (bd and sn)
    assert!(
        event_count >= 2,
        "Should detect at least 2 sample events: found {}",
        event_count
    );
}

/// Test extreme envelope values are clamped
#[test]
fn test_extreme_envelope_values_clamped() {
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(2.0);

    // Use very large attack/release values (should be clamped to 10s max)
    let pattern = parse_mini_notation("bd");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd".to_string(),
        pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(1.0),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        n: Signal::Value(0.0),
        note: Signal::Value(0.0),
        attack: Signal::Value(100.0),  // 100s attack (extreme)
        release: Signal::Value(100.0), // 100s release (extreme)
        envelope_type: None,
    });

    graph.set_output_channel(1, sample_node);

    // Should not crash and should eventually produce audio
    // (even if clamped to 10s, it should start ramping up immediately)
    let mut has_audio = false;
    for _ in 0..44100 {
        // 1 second
        let outputs = graph.process_sample_multi();
        if outputs[0].abs() > 0.0 {
            has_audio = true;
            break;
        }
    }

    assert!(
        has_audio,
        "Extreme envelope values should still produce audio (clamped)"
    );
}

/// Test very short attack and release
#[test]
fn test_very_short_envelope() {
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(2.0);

    // Use very short attack/release (should be clamped to minimums)
    let pattern = parse_mini_notation("bd");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd".to_string(),
        pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(1.0),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        n: Signal::Value(0.0),
        note: Signal::Value(0.0),
        attack: Signal::Value(0.00001), // 0.01ms attack (very short)
        release: Signal::Value(0.0001), // 0.1ms release (very short)
        envelope_type: None,
    });

    graph.set_output_channel(1, sample_node);

    // Should produce audio immediately (fast attack)
    let mut samples = Vec::new();
    for _ in 0..1000 {
        // ~23ms
        let outputs = graph.process_sample_multi();
        samples.push(outputs[0]);
    }

    // Find first non-zero sample
    let mut first_audio = 0;
    for (i, &sample) in samples.iter().enumerate() {
        if sample.abs() > 0.001 {
            first_audio = i;
            break;
        }
    }

    // Should start very quickly (within 2ms = 88 samples)
    assert!(
        first_audio < 88,
        "Very short attack should start audio quickly: first_audio at sample {}",
        first_audio
    );
}
