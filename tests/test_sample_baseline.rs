use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

#[test]
fn test_sample_without_transform() {
    let input = r#"
        cps: 2.0
        out $ s "bd bd bd bd" * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Should parse DSL");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Render 2 seconds (4 cycles at 2 CPS)
    let buffer = graph.render(88200);

    // Calculate RMS
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    println!("RMS without degrade: {}", rms);
    assert!(
        rms > 0.01,
        "Sample pattern should produce audio, got RMS: {}",
        rms
    );
}
