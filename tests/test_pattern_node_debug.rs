use phonon::mini_notation_v3::parse_mini_notation;
use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph};

#[test]
fn test_pattern_node_value_changes() {
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(2.0); // 2 cycles per second

    // Create a Pattern node with three values
    let pattern = parse_mini_notation("110 220 440");
    let pattern_node = graph.add_node(SignalNode::Pattern {
        pattern_str: "110 220 440".to_string(),
        pattern,
        last_value: 110.0,
        last_trigger_time: -1.0,
    });

    let output = graph.add_node(SignalNode::Output {
        input: Signal::Node(pattern_node),
    });

    graph.set_output(output);

    println!("\nTesting Pattern node with '110 220 440' at 2 CPS");
    println!("One cycle = 0.5 seconds = 22050 samples");
    println!("Expected transitions at samples: 0, 7350, 14700");
    println!();

    let mut last_value = 0.0;
    let mut max_value: f32 = 0.0;
    let mut seen_values = Vec::new();
    let samples_to_test = 30000; // ~0.68 seconds, should see all 3 values

    for i in 0..samples_to_test {
        let value = graph.process_sample();

        // Detect value changes
        if (value - last_value).abs() > 0.1 {
            let time_secs = i as f32 / sample_rate;
            let cycle_pos = time_secs * 2.0; // 2 cps
            println!(
                "Sample {}: value changed {} -> {} (time={:.4}s, cycle={:.3})",
                i, last_value, value, time_secs, cycle_pos
            );
            last_value = value;
            seen_values.push(value);
            max_value = max_value.max(value);
        }
    }

    println!("\nFinal value: {}", last_value);
    println!("Max value seen: {}", max_value);
    println!("All values seen: {:?}", seen_values);

    // We should have seen all three values (110, 220, 440)
    assert!(
        max_value > 400.0,
        "Should have seen 440 Hz, got max {}",
        max_value
    );
    assert!(seen_values.contains(&110.0), "Should have seen 110 Hz");
    assert!(seen_values.contains(&220.0), "Should have seen 220 Hz");
}
