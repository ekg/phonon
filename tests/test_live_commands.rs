use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph, Waveform};

#[test]
fn test_hush_command_silences_outputs() {
    // Setup graph with multiple outputs
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);

    let osc1 = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        phase: 0.0,
        pending_freq: None,
        last_sample: 0.0,
    });

    let osc2 = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(880.0),
        waveform: Waveform::Sine,
        phase: 0.0,
        pending_freq: None,
        last_sample: 0.0,
    });

    graph.set_output_channel(1, osc1);
    graph.set_output_channel(2, osc2);

    // Process some samples - should have audio
    graph.process_sample_multi(); // Skip first sample (phase=0)
    let outputs_before = graph.process_sample_multi();
    assert!(
        outputs_before[0].abs() > 0.0,
        "Channel 1 should have audio before hush"
    );
    assert!(
        outputs_before[1].abs() > 0.0,
        "Channel 2 should have audio before hush"
    );

    // Call hush
    graph.hush_all();

    // Process samples - should be silent
    let outputs_after = graph.process_sample_multi();
    assert_eq!(
        outputs_after[0], 0.0,
        "Channel 1 should be silent after hush"
    );
    assert_eq!(
        outputs_after[1], 0.0,
        "Channel 2 should be silent after hush"
    );
}

#[test]
fn test_panic_command_kills_voices() {
    // Setup graph with sample playback
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(2.0);

    use phonon::mini_notation_v3::parse_mini_notation;
    use std::collections::HashMap;

    let pattern = parse_mini_notation("bd sn hh*8");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd sn hh*8".to_string(),
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
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
        envelope_type: None,
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
    });

    graph.set_output_channel(1, sample_node);

    // Trigger some samples
    for _ in 0..1000 {
        graph.process_sample_multi();
    }

    // Call panic - should kill all voices AND silence outputs
    graph.panic();

    // Should be completely silent
    for _ in 0..100 {
        let outputs = graph.process_sample_multi();
        assert_eq!(outputs[0], 0.0, "Should be silent after panic");
    }
}

#[test]
fn test_hush_specific_channel() {
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);

    let osc1 = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        phase: 0.0,
        pending_freq: None,
        last_sample: 0.0,
    });

    let osc2 = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(880.0),
        waveform: Waveform::Sine,
        phase: 0.0,
        pending_freq: None,
        last_sample: 0.0,
    });

    graph.set_output_channel(1, osc1);
    graph.set_output_channel(2, osc2);

    // Hush only channel 1
    graph.hush_channel(1);

    // Skip first sample
    graph.process_sample_multi();
    let outputs = graph.process_sample_multi();

    assert_eq!(outputs[0], 0.0, "Channel 1 should be hushed");
    assert!(outputs[1].abs() > 0.0, "Channel 2 should still play");
}
