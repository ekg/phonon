use phonon::mini_notation_v3::parse_mini_notation;
/// Minimal reproduction of single-event pattern bug
///
/// BUG: Pattern with 1 event at slow tempo (2s cycles) produces silence
/// WORKS: Same pattern at fast tempo (0.5s cycles) works fine
use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph};
use std::collections::HashMap;

#[test]
fn test_single_event_slow_tempo_bug() {
    // This FAILS - produces silence
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(0.5); // 2-second cycles

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
        attack: Signal::Value(0.0),
        release: Signal::Value(0.0),
        envelope_type: None,
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
    });

    graph.set_output(sample_node);

    // Render 2 seconds (1 cycle)
    let buffer = graph.render(88200);

    // Check peak amplitude
    let peak = buffer.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
    println!("Peak (slow tempo): {:.6}", peak);

    // BUG: This should be ~0.012 but is actually ~0.000 (silence)
    assert!(peak > 0.005, "Peak too low: {:.6}", peak);
}

#[test]
fn test_single_event_fast_tempo_works() {
    // This WORKS - produces audio
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0); // 0.5-second cycles

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
        attack: Signal::Value(0.0),
        release: Signal::Value(0.0),
        envelope_type: None,
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
    });

    graph.set_output(sample_node);

    // Render 2 seconds (4 cycles)
    let buffer = graph.render(88200);

    // Check peak amplitude
    let peak = buffer.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
    println!("Peak (fast tempo): {:.6}", peak);

    // This works correctly
    assert!(peak > 0.005, "Peak too low: {:.6}", peak);
}

#[test]
fn test_two_events_slow_tempo_works() {
    // This WORKS - 2 events at slow tempo
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(0.5); // 2-second cycles

    let pattern = parse_mini_notation("bd bd");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd bd".to_string(),
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
        attack: Signal::Value(0.0),
        release: Signal::Value(0.0),
        envelope_type: None,
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
    });

    graph.set_output(sample_node);

    // Render 2 seconds (1 cycle)
    let buffer = graph.render(88200);

    // Check peak amplitude
    let peak = buffer.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
    println!("Peak (2 events, slow tempo): {:.6}", peak);

    // This works correctly
    assert!(peak > 0.005, "Peak too low: {:.6}", peak);
}
