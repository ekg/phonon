/// Test that samples can be routed through effects
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph};
use std::collections::HashMap;

fn calculate_rms(samples: &[f32]) -> f32 {
    let sum_squares: f32 = samples.iter().map(|&x| x * x).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

#[test]
fn test_samples_through_lowpass_filter() {
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(2.0); // 2 cycles per second

    // Create sample node
    let pattern = parse_mini_notation("bd sn cp hh");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd sn cp hh".to_string(),
        pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(1.0),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
    });

    // Route through lowpass filter
    let filtered = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(sample_node),
        cutoff: Signal::Value(1000.0),
        q: Signal::Value(0.7),
        state: Default::default(),
    });

    graph.set_output(filtered);

    // Render 1 second
    let buffer = graph.render(sample_rate as usize);

    let rms = calculate_rms(&buffer);
    let peak = buffer.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("\n✓ Samples through Filter Test");
    println!("  RMS={:.4}, Peak={:.4}", rms, peak);

    // Should have audio (filtered but still present)
    assert!(
        rms > 0.01,
        "Filtered samples should have audio, got RMS={}",
        rms
    );
    assert!(
        peak > 0.1,
        "Filtered samples should have peaks, got peak={}",
        peak
    );
}

#[test]
fn test_samples_through_multiply() {
    // Test that samples can be processed with basic math
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(1.0);

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
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
    });

    // Multiply by 0.5 (reduce volume)
    let scaled = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(sample_node),
        b: Signal::Value(0.5),
    });

    graph.set_output(scaled);

    let buffer = graph.render(sample_rate as usize);
    let peak = buffer.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("\n✓ Samples with Multiply Test");
    println!("  Peak={:.4} (should be ~0.5)", peak);

    // Peak should be around 0.5 (half of original)
    assert!(
        peak > 0.3 && peak < 0.7,
        "Scaled peak should be ~0.5, got {}",
        peak
    );
}
