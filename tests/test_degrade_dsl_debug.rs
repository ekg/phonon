use phonon::pattern::{Fraction, State, TimeSpan};
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};
use std::collections::HashMap;

#[test]
fn debug_degrade_dsl_compilation() {
    // Test what pattern actually ends up in the graph when compiled from DSL
    let input_degraded = r#"
        cps: 2.0
        out: s("bd bd bd bd" $ degrade) * 0.5
    "#;

    // Parse the DSL
    let (_, statements) = parse_dsl(input_degraded).expect("Should parse");

    println!("\n=== DSL COMPILATION DEBUG ===");
    println!("Parsed {} statements", statements.len());

    // Compile to graph
    let compiler = DslCompiler::new(44100.0);
    let graph = compiler.compile(statements);

    println!("Graph compiled successfully");

    // Try to inspect the graph nodes (if we can access them)
    // For now, just render and analyze

    println!("\nAttempting to render audio to verify transform was applied...");
}

#[test]
fn debug_direct_pattern_in_sample_node() {
    // Manually create what the DSL compiler SHOULD create
    use phonon::mini_notation_v3::parse_mini_notation;
    use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph};
    use std::collections::HashMap as StdHashMap;

    println!("\n=== DIRECT SAMPLE NODE TEST ===");

    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0);

    // Create a degraded pattern manually
    let pattern_str = "bd bd bd bd";
    let base_pattern = parse_mini_notation(pattern_str);
    let degraded_pattern = base_pattern.degrade();

    // Query the degraded pattern to verify it has fewer events
    let state = State {
        span: TimeSpan::new(
            Fraction::from_float(0.0),
            Fraction::from_float(2.0), // 2 cycles
        ),
        controls: HashMap::new(),
    };
    let events = degraded_pattern.query(&state);
    println!("Degraded pattern has {} events over 2 cycles", events.len());

    // Create a Sample node with the degraded pattern
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: pattern_str.to_string(),
        pattern: degraded_pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: StdHashMap::new(),
        gain: Signal::Value(0.5),
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

    // Render audio
    let audio = graph.render(88200); // 2 seconds at 44.1kHz

    // Calculate RMS
    let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();

    // Count non-zero samples
    let non_zero = audio.iter().filter(|&&x| x.abs() > 0.0001).count();

    println!("Direct Sample node with degraded pattern:");
    println!("  RMS: {:.6}", rms);
    println!("  Non-zero samples: {}", non_zero);

    assert!(rms > 0.0001, "Should produce audio");
}
