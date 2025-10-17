use phonon::superdirt_synths::SynthLibrary;
use phonon::unified_graph::{Signal, UnifiedSignalGraph};
/// Isolated test for SuperSaw with pattern frequency
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

#[test]
fn test_supersaw_constant_freq_baseline() {
    let input = "out: supersaw(110, 0.5, 5) * 0.2";
    let (_, statements) = parse_dsl(input).unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    graph.set_cps(2.0);

    let buffer = graph.render(4410); // 0.1 seconds
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    println!("SuperSaw constant 110 Hz RMS: {}", rms);
    assert!(rms > 0.01, "SuperSaw should produce audio");
}

#[test]
fn test_supersaw_pattern_freq_from_dsl() {
    let input = r#"out: supersaw("110 220", 0.5, 5) * 0.2"#;
    let (_, statements) = parse_dsl(input).unwrap();

    println!("Parsed statements: {:?}", statements);

    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    graph.set_cps(2.0);

    let buffer = graph.render(4410);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    println!("SuperSaw pattern \"110 220\" RMS: {}", rms);
    println!("First 10 samples: {:?}", &buffer[..10]);

    assert!(
        rms > 0.01,
        "SuperSaw with pattern freq should produce audio, got RMS={}",
        rms
    );
}

#[test]
fn test_supersaw_manually_built_with_pattern() {
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0);

    let library = SynthLibrary::new();

    // Build supersaw with pattern freq directly
    let freq = Signal::Pattern("110 220".to_string());
    let saw_id = library.build_supersaw(&mut graph, freq, Some(0.5), Some(5));

    graph.set_output(saw_id);

    let buffer = graph.render(4410);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    println!("Manually built SuperSaw with Signal::Pattern RMS: {}", rms);

    assert!(rms > 0.01, "Manually built should work");
}
