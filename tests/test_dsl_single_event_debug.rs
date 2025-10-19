/// Debug test for single-event pattern bug using DSL compiler
///
/// This test uses the DslCompiler to reproduce the bug where
/// single-event patterns at slow tempo produce silence

use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

#[test]
fn test_dsl_compiler_single_event_slow() {
    // This should FAIL - produces silence
    let dsl_code = r#"
tempo: 0.5
out: s "bd" * 0.8
"#;

    let (_, statements) = parse_dsl(dsl_code).expect("Failed to parse DSL");

    // Print parsed statements
    println!("Parsed {} statements", statements.len());
    for (i, stmt) in statements.iter().enumerate() {
        println!("  Statement {}: {:?}", i, stmt);
    }

    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Render 2 seconds (1 cycle at 0.5 cps)
    let buffer = graph.render(88200);

    // Check peak amplitude
    let peak = buffer.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
    println!("DSL Peak (slow tempo): {:.6}", peak);

    // Print first 100 samples
    println!("First 100 samples:");
    for (i, &sample) in buffer.iter().take(100).enumerate() {
        if sample.abs() > 0.0001 {
            println!("  Sample {}: {:.6}", i, sample);
        }
    }

    // This should NOT be silent!
    assert!(peak > 0.005, "DSL peak too low: {:.6}", peak);
}

#[test]
fn test_dsl_compiler_single_event_fast() {
    // This should PASS - produces audio
    let dsl_code = r#"
tempo: 2.0
out: s "bd" * 0.8
"#;

    let (_, statements) = parse_dsl(dsl_code).expect("Failed to parse DSL");

    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Render 2 seconds (4 cycles at 2.0 cps)
    let buffer = graph.render(88200);

    // Check peak amplitude
    let peak = buffer.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
    println!("DSL Peak (fast tempo): {:.6}", peak);

    // This should work
    assert!(peak > 0.005, "DSL peak too low: {:.6}", peak);
}

#[test]
fn test_dsl_compiler_two_events_slow() {
    // This should PASS - 2 events work at slow tempo
    let dsl_code = r#"
tempo: 0.5
out: s "bd bd" * 0.8
"#;

    let (_, statements) = parse_dsl(dsl_code).expect("Failed to parse DSL");

    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Render 2 seconds (1 cycle at 0.5 cps)
    let buffer = graph.render(88200);

    // Check peak amplitude
    let peak = buffer.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
    println!("DSL Peak (2 events, slow): {:.6}", peak);

    // This should work
    assert!(peak > 0.005, "DSL peak too low: {:.6}", peak);
}
