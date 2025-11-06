use phonon::mini_notation_v3::parse_mini_notation;
use phonon::unified_graph::{SignalNode, UnifiedSignalGraph};
use std::collections::HashMap;

#[test]
fn test_voice_gain_parameter() {
    // Test that voices can be triggered with custom gain values
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(1.0); // 1 cycle per second

    // Create a pattern that triggers a sample
    let pattern = parse_mini_notation("bd");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd".to_string(),
        pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: phonon::unified_graph::Signal::Value(1.0),
        pan: phonon::unified_graph::Signal::Value(0.0),
        speed: phonon::unified_graph::Signal::Value(1.0),
        cut_group: phonon::unified_graph::Signal::Value(0.0),
        n: phonon::unified_graph::Signal::Value(0.0),
        note: phonon::unified_graph::Signal::Value(0.0),
        attack: phonon::unified_graph::Signal::Value(0.001),
        release: phonon::unified_graph::Signal::Value(0.1),
        envelope_type: None,
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
    });

    graph.set_output(sample_node);

    // Process enough samples to trigger the pattern (bd happens at cycle start)
    let mut max_amplitude = 0.0_f32;
    for _ in 0..1000 {
        let sample = graph.process_sample();
        max_amplitude = max_amplitude.max(sample.abs());
    }

    // With default gain=1.0, amplitude should be close to 1.0
    // (actual peak depends on sample content, but should be > 0.5)
    assert!(
        max_amplitude > 0.5,
        "Expected significant amplitude with gain=1.0, got {}",
        max_amplitude
    );

    println!("Max amplitude with default gain: {}", max_amplitude);
}

#[test]
#[ignore] // Ignore until gain parameter is implemented
fn test_voice_gain_reduction() {
    // Test that voices respect reduced gain values
    // This test will be implemented after we add gain parameter support
    // Expected syntax: s("bd", gain=0.5) or similar
}

#[test]
#[ignore] // Ignore until per-voice gain is implemented
fn test_voice_per_event_gain() {
    // Test that different events can have different gain values
    // Expected: each trigger can have its own gain
    // E.g., s("bd sn", gain="0.5 1.0") where bd gets 0.5 and sn gets 1.0
}
