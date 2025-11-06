use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph};
use std::collections::HashMap;

#[test]
fn compare_normal_vs_degraded_sample_nodes() {
    println!("\n=== NORMAL VS DEGRADED SAMPLE NODE COMPARISON ===");

    let pattern_str = "bd bd bd bd";

    // Test 1: Normal pattern
    let mut graph_normal = UnifiedSignalGraph::new(44100.0);
    graph_normal.set_cps(2.0);

    let normal_pattern = parse_mini_notation(pattern_str);

    // Query to see events
    let state = State {
        span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(2.0)),
        controls: HashMap::new(),
    };
    let normal_events = normal_pattern.query(&state);
    println!("Normal pattern: {} events", normal_events.len());

    let sample_node_normal = graph_normal.add_node(SignalNode::Sample {
        pattern_str: pattern_str.to_string(),
        pattern: normal_pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(0.5),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        n: phonon::unified_graph::Signal::Value(0.0),
        note: phonon::unified_graph::Signal::Value(0.0),
        attack: Signal::Value(0.0),
        release: Signal::Value(0.0),
        envelope_type: None,
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
    });
    graph_normal.set_output(sample_node_normal);

    let audio_normal = graph_normal.render(88200); // 2 seconds
    let rms_normal: f32 =
        (audio_normal.iter().map(|x| x * x).sum::<f32>() / audio_normal.len() as f32).sqrt();
    let non_zero_normal = audio_normal.iter().filter(|&&x| x.abs() > 0.0001).count();

    println!("Normal audio:");
    println!("  RMS: {:.6}", rms_normal);
    println!("  Non-zero samples: {}", non_zero_normal);

    // Test 2: Degraded pattern
    let mut graph_degraded = UnifiedSignalGraph::new(44100.0);
    graph_degraded.set_cps(2.0);

    let base_pattern = parse_mini_notation(pattern_str);
    let degraded_pattern = base_pattern.degrade();

    let degraded_events = degraded_pattern.query(&state);
    println!("\nDegraded pattern: {} events", degraded_events.len());

    let sample_node_degraded = graph_degraded.add_node(SignalNode::Sample {
        pattern_str: pattern_str.to_string(),
        pattern: degraded_pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(0.5),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        n: phonon::unified_graph::Signal::Value(0.0),
        note: phonon::unified_graph::Signal::Value(0.0),
        attack: Signal::Value(0.0),
        release: Signal::Value(0.0),
        envelope_type: None,
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
    });
    graph_degraded.set_output(sample_node_degraded);

    let audio_degraded = graph_degraded.render(88200);
    let rms_degraded: f32 =
        (audio_degraded.iter().map(|x| x * x).sum::<f32>() / audio_degraded.len() as f32).sqrt();
    let non_zero_degraded = audio_degraded.iter().filter(|&&x| x.abs() > 0.0001).count();

    println!("Degraded audio:");
    println!("  RMS: {:.6}", rms_degraded);
    println!("  Non-zero samples: {}", non_zero_degraded);

    println!("\nComparison:");
    println!(
        "  Event ratio: {:.2}",
        degraded_events.len() as f32 / normal_events.len() as f32
    );
    println!("  RMS ratio: {:.2}", rms_degraded / rms_normal);
    println!(
        "  Non-zero ratio: {:.2}",
        non_zero_degraded as f32 / non_zero_normal as f32
    );

    // The degraded version should have lower RMS since it has fewer events
    assert!(
        rms_degraded < rms_normal,
        "Degraded RMS ({:.6}) should be less than normal ({:.6})",
        rms_degraded,
        rms_normal
    );
}
