//! Test if sample playback works through DSL syntax

use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

#[test]
fn test_dsl_sample_playback_simple() {
    // Does out s "bd" produce audio through DslCompiler?
    // NOTE: No leading whitespace - parser is sensitive to formatting
    let input = "tempo: 1.0\nout: s(\"bd\")";

    println!("\n=== DSL Sample Playback Test ===");

    let parse_result = parse_dsl(input);
    match parse_result {
        Ok((_, statements)) => {
            println!("✅ Parse succeeded");
            println!("Statements: {:?}", statements);

            if statements.is_empty() {
                println!("⚠️  WARNING: Statements is empty!");
                println!("   Parser recognized syntax but didn't generate any statements");
                println!("   This is why DslCompiler produces silence");
                return;
            }

            let compiler = DslCompiler::new(44100.0);
            let mut graph = compiler.compile(statements);

            let buffer = graph.render(44100);
            let rms = calculate_rms(&buffer);
            let peak = buffer.iter().map(|&x| x.abs()).fold(0.0, f32::max);

            println!("Audio output:");
            println!("  RMS: {:.4}", rms);
            println!("  Peak: {:.4}", peak);

            if rms > 0.01 {
                println!("✅ DSL sample playback WORKS");
            } else {
                println!("❌ DSL sample playback produces SILENCE");
            }
        }
        Err(e) => {
            println!("❌ Parse failed: {:?}", e);
        }
    }
}

#[test]
fn test_dsl_vs_direct_api() {
    // Compare DSL path vs direct API path
    use phonon::mini_notation_v3::parse_mini_notation;
    use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph};
    use std::collections::HashMap;

    println!("\n=== DSL vs Direct API Comparison ===");

    // Method 1: Direct API (known to work)
    let mut graph_direct = UnifiedSignalGraph::new(44100.0);
    graph_direct.set_cps(1.0);
    let pattern = parse_mini_notation("bd");
    let sample_node = graph_direct.add_node(SignalNode::Sample {
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
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
        envelope_type: None,
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
    });
    graph_direct.set_output(sample_node);
    let buffer_direct = graph_direct.render(44100);
    let rms_direct = calculate_rms(&buffer_direct);

    println!("Direct API:");
    println!("  RMS: {:.4}", rms_direct);

    // Method 2: DSL
    let input = r#"
        tempo 1.0
        out s "bd"
    "#;

    let (_, statements) = parse_dsl(input).expect("Parse failed");
    let compiler = DslCompiler::new(44100.0);
    let mut graph_dsl = compiler.compile(statements);
    let buffer_dsl = graph_dsl.render(44100);
    let rms_dsl = calculate_rms(&buffer_dsl);

    println!("DSL:");
    println!("  RMS: {:.4}", rms_dsl);

    println!("\nComparison:");
    if rms_direct > 0.01 && rms_dsl < 0.01 {
        println!("❌ Direct API works but DSL is broken");
        println!("   Issue is in parse_dsl or DslCompiler");
    } else if rms_direct > 0.01 && rms_dsl > 0.01 {
        println!("✅ Both paths work!");
    } else {
        println!("⚠️  Both paths are silent (unexpected)");
    }
}

fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = samples.iter().map(|&x| x * x).sum();
    (sum_squares / samples.len() as f32).sqrt()
}
