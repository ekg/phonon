use phonon::mini_notation_v3::parse_mini_notation;
/// Debug why pattern parameters produce zero audio
use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph, Waveform};

#[test]
fn test_pattern_signal_directly() {
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0);

    // Create a Pattern node directly
    let pattern = parse_mini_notation("110 220");
    let pattern_node = graph.add_node(SignalNode::Pattern {
        pattern_str: "110 220".to_string(),
        pattern,
        last_value: 110.0,
        last_trigger_time: -1.0,
    });

    graph.set_output(pattern_node);

    // Render 1 full cycle = 0.5 seconds at CPS=2.0 = 22050 samples
    let buffer = graph.render(22050);

    println!("First 10 samples: {:?}", &buffer[..10]);
    println!(
        "Middle 10 samples (at 0.5 cycles): {:?}",
        &buffer[11020..11030]
    );

    // First half should be ~110, second half should be ~220
    let first_half_avg: f32 = buffer[..11025].iter().sum::<f32>() / 11025.0;
    let second_half_avg: f32 = buffer[11025..].iter().sum::<f32>() / 11025.0;

    println!("First half average: {}", first_half_avg);
    println!("Second half average: {}", second_half_avg);

    assert!(
        (first_half_avg - 110.0).abs() < 5.0,
        "First half should be ~110"
    );
    assert!(
        (second_half_avg - 220.0).abs() < 5.0,
        "Second half should be ~220"
    );
    println!("✅ Pattern cycles correctly!");
}

#[test]
fn test_oscillator_with_pattern_signal() {
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0);

    // Create Pattern node
    let pattern = parse_mini_notation("110 220");
    let pattern_node = graph.add_node(SignalNode::Pattern {
        pattern_str: "110 220".to_string(),
        pattern,
        last_value: 110.0,
        last_trigger_time: -1.0,
    });

    // Create oscillator using pattern as freq
    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Node(pattern_node),
        waveform: Waveform::Sine,
        phase: 0.0,
        pending_freq: None,
        last_sample: 0.0, 
    });

    graph.set_output(osc);

    let buffer = graph.render(44100);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    println!("Oscillator with pattern freq RMS: {}", rms);

    assert!(rms > 0.01, "Should produce audio");
}

#[test]
fn test_how_dsl_compiles_pattern() {
    use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

    // Parse DSL with pattern
    let input = r#"out: sine "110 220" * 0.2"#;
    let result = parse_dsl(input);

    println!("Parse result: {:?}", result);

    if let Ok((_, statements)) = result {
        let compiler = DslCompiler::new(44100.0);
        let mut graph = compiler.compile(statements);
        graph.set_cps(2.0);

        // Render
        let buffer = graph.render(100);
        println!("First 10 samples: {:?}", &buffer[..10]);

        let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
        println!("DSL compiled pattern RMS: {}", rms);
    } else {
        panic!("Failed to parse DSL");
    }
}
