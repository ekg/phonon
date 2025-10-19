use phonon::unified_graph::{UnifiedSignalGraph, SignalNode, Signal};
use phonon::mini_notation_v3::parse_mini_notation;
use std::collections::HashMap;

#[test]
fn test_pattern_gain_debug() {
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0); // 2 cycles per second

    // Create sample pattern
    let sample_pattern = parse_mini_notation("bd bd");

    // Create gain pattern
    let gain_pattern = parse_mini_notation("1.0 0.5");
    let gain_node = graph.add_node(SignalNode::Pattern {
        pattern_str: "1.0 0.5".to_string(),
        pattern: gain_pattern,
        last_value: 1.0,
        last_trigger_time: -1.0,
    });

    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd bd".to_string(),
        pattern: sample_pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Node(gain_node), // Pattern-valued gain
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        n: Signal::Value(0.0),
        note: Signal::Value(0.0),
        attack: Signal::Value(0.0),
        release: Signal::Value(0.0),
    });

    graph.set_output(sample_node);

    // Render and analyze
    let buffer = graph.render(22050); // 0.5 seconds at 44.1kHz

    // Find peaks
    let first_half = &buffer[0..11025];
    let second_half = &buffer[11025..22050];

    let first_peak = first_half.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    let second_peak = second_half.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("First BD (should be gain=1.0):  peak = {:.6}", first_peak);
    println!("Second BD (should be gain=0.5): peak = {:.6}", second_peak);
    println!("Ratio: {:.3} (expected ~2.0)", first_peak / second_peak);

    // The first BD should be roughly 2x louder than the second
    assert!((first_peak / second_peak - 2.0).abs() < 0.2,
            "Pattern gain not working: ratio = {:.3}, expected 2.0",
            first_peak / second_peak);
}
