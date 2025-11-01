//! Simple debug test to see what's happening with channels

use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

#[test]
fn test_single_sample_trigger() {
    let code = r#"
        tempo: 2.0
        o1: s "bd"
    "#;

    let (_, statements) = parse_dsl(code).unwrap();
    let mut graph = DslCompiler::new(44100.0).compile(statements);

    println!("\n=== Single channel ===");
    println!("Has output: {}", graph.has_output());
    println!("Outputs count: {}", graph.get_all_bus_names().len());

    let buffer = graph.render(22050);
    let peak = buffer.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
    println!("Peak: {:.6}", peak);

    assert!(peak > 0.5, "Should have audio");
}

#[test]
fn test_three_sample_triggers() {
    let code = r#"
        tempo: 2.0
        o1: s "bd"
        o2: s "bd"
        o3: s "bd"
    "#;

    let (_, statements) = parse_dsl(code).unwrap();
    let mut graph = DslCompiler::new(44100.0).compile(statements);

    println!("\n=== Three channels (same sample) ===");
    println!("Has output: {}", graph.has_output());

    let buffer = graph.render(22050);
    let peak = buffer.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
    println!("Peak: {:.6}", peak);

    // If samples are independent, peak should be ~3x a single sample
    // If they're being triggered 3 times each, peak could be 9x!
    assert!(peak < 4.0, "Peak way too high: {}", peak);
}
