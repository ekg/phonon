/// Test the s() function for sample pattern triggering
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

#[test]
fn test_s_function_parses() {
    let input = r#"out: s "bd sn hh cp""#;
    let result = parse_dsl(input);

    println!("Parse result: {:?}", result);
    assert!(result.is_ok(), "s() function should parse");
}

#[test]
fn test_s_function_compiles() {
    let input = r#"out: s "bd ~ sn ~""#;
    let (_, statements) = parse_dsl(input).unwrap();

    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    graph.set_cps(2.0);

    // Render 1 second
    let buffer = graph.render(44100);

    // Should have some audio (kicks and snares)
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    println!("RMS: {}", rms);

    assert!(
        rms > 0.01,
        "s() function should produce audio, got RMS={}",
        rms
    );
    println!("✅ s() function works!");
}

#[test]
fn test_s_function_with_gain_param() {
    let input = r#"out: s("bd*4", 0.5)"#;
    let (_, statements) = parse_dsl(input).unwrap();

    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    graph.set_cps(2.0);

    let buffer = graph.render(44100);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    println!("RMS with gain=0.5: {}", rms);
    assert!(rms > 0.01, "s() with gain should produce audio");
}

#[test]
fn test_s_function_with_pattern_gain() {
    let input = r#"out: s("bd*4", "0.5 1.0 0.7 0.3")"#;
    let (_, statements) = parse_dsl(input).unwrap();

    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    graph.set_cps(2.0);

    let buffer = graph.render(44100);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    println!("RMS with pattern gain: {}", rms);
    assert!(rms > 0.01, "s() with pattern gain should produce audio");
    println!("✅ Pattern gain works!");
}

#[test]
fn test_tidal_workflow_basic() {
    // Test basic Tidal Cycles workflow
    let input = r#"
        out: s "bd sn bd sn"
    "#;

    let (_, statements) = parse_dsl(input).unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    graph.set_cps(2.0);

    let buffer = graph.render(88200); // 2 seconds = 4 cycles
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    println!("Tidal workflow RMS: {}", rms);
    assert!(rms > 0.01, "Basic Tidal workflow should work");
    println!("✅ Basic Tidal Cycles workflow works!");
}
