//! Debug envelope behavior

use phonon::unified_graph::{EnvState, Signal, SignalNode, UnifiedSignalGraph};

#[test]
fn test_envelope_trigger_basic() {
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);

    // Constant high trigger
    let trigger = graph.add_node(SignalNode::Constant { value: 1.0 });
    let source = graph.add_node(SignalNode::Constant { value: 1.0 });

    let envelope = graph.add_node(SignalNode::Envelope {
        input: Signal::Node(source),
        trigger: Signal::Node(trigger),
        attack: Signal::Value(0.01),
        decay: Signal::Value(0.0),
        sustain: Signal::Value(1.0),
        release: Signal::Value(0.0),
        state: EnvState::default(),
    });

    graph.set_output(envelope);

    // Process 1000 samples and print
    for i in 0..1000 {
        let sample = graph.process_sample();
        if i % 100 == 0 {
            println!("Sample {}: {}", i, sample);
        }
    }
}

#[test]
fn test_envelope_trigger_pattern() {
    use phonon::mini_notation_v3::parse_mini_notation;

    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(4.0);

    // Pattern trigger: "1 0 0 0"
    let trigger_pattern = parse_mini_notation("1 0 0 0");
    let trigger = graph.add_node(SignalNode::Pattern {
        pattern_str: "1 0 0 0".to_string(),
        pattern: trigger_pattern,
        last_value: 0.0,
        last_trigger_time: -1.0,
    });

    let source = graph.add_node(SignalNode::Constant { value: 1.0 });

    let envelope = graph.add_node(SignalNode::Envelope {
        input: Signal::Node(source),
        trigger: Signal::Node(trigger),
        attack: Signal::Value(0.001),
        decay: Signal::Value(0.0),
        sustain: Signal::Value(1.0),
        release: Signal::Value(0.1),
        state: EnvState::default(),
    });

    graph.set_output(envelope);

    // Render 1 second and print key points
    let buffer = graph.render(sample_rate as usize);

    println!("Sample at 0.05s (in trigger): {}", buffer[2205]);
    println!("Sample at 0.30s (after trigger): {}", buffer[13230]);
    println!("Sample at 0.40s (after release): {}", buffer[17640]);
}
