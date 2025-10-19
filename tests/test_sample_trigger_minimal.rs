use phonon::unified_graph::{UnifiedSignalGraph, SignalNode, Signal};
use phonon::mini_notation_v3::parse_mini_notation;
use std::collections::HashMap;

#[test]
fn test_direct_sample_trigger() {
    eprintln!("\n=== TEST: Direct Sample Trigger ===\n");

    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(0.5); // 0.5 cycles per second = 2 second cycle at 120 BPM

    // Parse pattern "bd"
    let pattern = parse_mini_notation("bd");
    eprintln!("Pattern parsed");

    // Create sample node
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd".to_string(),
        pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(0.8),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        n: Signal::Value(0.0),
        note: Signal::Value(0.0),
        attack: Signal::Value(0.0),
        release: Signal::Value(0.0),
    });

    graph.set_output(sample_node);
    eprintln!("Graph constructed");

    // Render 1 second (44100 samples)
    eprintln!("Rendering...");
    let buffer = graph.render(44100);

    // Check if we have audio
    let peak: f32 = buffer.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    let rms: f32 = (buffer.iter().map(|&x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    eprintln!("Peak: {:.6}", peak);
    eprintln!("RMS: {:.6}", rms);
    eprintln!("Non-zero samples: {}", buffer.iter().filter(|&&x| x.abs() > 0.0001).count());

    assert!(peak > 0.001, "No audio generated! Peak: {}", peak);
    assert!(rms > 0.0001, "No audio generated! RMS: {}", rms);

    eprintln!("✅ Test passed!");
}
