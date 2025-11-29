/// Debug test for cut groups
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

#[test]
fn test_cut_group_debug_simple() {
    // Two hi-hats with cut group 1
    let input = r#"
        tempo: 0.5
        out $ s("hh hh", "1.0 1.0", "0 0", "1 1", "1 1")
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse DSL");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    println!("\n=== RENDERING CYCLE 0 ===");

    // Render sample by sample for first 1000 samples, tracking voices
    for i in 0..1000 {
        let _sample = graph.process_sample();
        let voices = graph.active_voice_count();

        if voices > 0 {
            println!("Sample {}: {} voices active", i, voices);
        }
    }
}
