/// Test new bus syntax: ~name $ expr or ~name # expr
///
/// The new syntax eliminates the colon and uses operators directly:
/// - ~bass $ saw 55 # lpf 1000 0.8    ($ assigns source, # chains effects)
/// - ~drums $ s "bd sn" $ fast 2      ($ for pattern source and transforms)
/// - out $ ~bass + ~drums             ($ for output assignment)

use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

#[test]
fn test_new_bus_syntax_dollar() {
    // Test: ~bass $ saw 55
    let input = r#"
        cps: 1.0
        ~bass $ saw 55
        out $ ~bass * 0.3
    "#;

    let (_, statements) = parse_dsl(input).expect("Should parse new $ syntax");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Render and verify audio output
    let buffer = graph.render(4410); // 0.1 seconds

    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(rms > 0.01, "New $ syntax should produce audio, got RMS={}", rms);
}

#[test]
fn test_new_bus_syntax_hash_chain() {
    // Test: ~bass $ saw 55 # lpf 1000 0.8
    let input = r#"
        cps: 1.0
        ~bass $ saw 55 # lpf 1000 0.8
        out $ ~bass * 0.3
    "#;

    let (_, statements) = parse_dsl(input).expect("Should parse $ with # chain");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(4410);

    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(rms > 0.01, "$ with # chain should produce audio, got RMS={}", rms);
}

#[test]
fn test_new_bus_syntax_pattern_transform() {
    // Test: ~drums $ s "bd sn" $ fast 2
    let input = r#"
        cps: 2.0
        ~drums $ s "bd sn" $ fast 2
        out $ ~drums
    "#;

    let (_, statements) = parse_dsl(input).expect("Should parse pattern transforms with $");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Render 1 second (2 cycles at 2 CPS)
    let buffer = graph.render(44100);

    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(rms > 0.01, "Pattern transforms with $ should produce audio, got RMS={}", rms);
}

#[test]
fn test_new_bus_syntax_multiple_buses() {
    // Test multiple buses with new syntax
    let input = r#"
        cps: 2.0
        ~bass $ saw 55 # lpf 800 0.8
        ~drums $ s "bd sn"
        out $ ~bass * 0.3 + ~drums * 0.7
    "#;

    let (_, statements) = parse_dsl(input).expect("Should parse multiple buses");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(44100);

    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(rms > 0.01, "Multiple buses should produce audio, got RMS={}", rms);
}

#[test]
fn test_new_output_syntax_dollar() {
    // Test: out $ expression
    let input = r#"
        cps: 1.0
        out $ saw 110 * 0.2
    "#;

    let (_, statements) = parse_dsl(input).expect("Should parse out $ syntax");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(4410);

    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(rms > 0.01, "out $ syntax should produce audio, got RMS={}", rms);
}

#[test]
fn test_backward_compatibility_colon() {
    // Test that old colon syntax still works
    let input = r#"
        cps: 1.0
        ~bass: saw 55
        out: ~bass * 0.3
    "#;

    let (_, statements) = parse_dsl(input).expect("Should still parse colon syntax");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(4410);

    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(rms > 0.01, "Colon syntax should still work, got RMS={}", rms);
}

#[test]
fn test_parentheses_in_new_syntax() {
    // Test that parentheses work correctly with new syntax
    let input = r#"
        cps: 2.0
        ~mod $ sine 0.5
        ~bass $ saw (110 + ~mod * 50) # lpf 1000 0.8
        out $ ~bass * 0.3
    "#;

    let (_, statements) = parse_dsl(input).expect("Should parse parentheses");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(44100);

    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(rms > 0.01, "Parenthesized expressions should work, got RMS={}", rms);
}

#[test]
fn test_complex_pattern_with_new_syntax() {
    // Test complex pattern operations with new syntax
    let input = r#"
        cps: 2.0
        ~drums $ s "bd sn hh cp" $ fast 2 $ rev
        ~bass $ saw 55 # lpf 800 0.9
        out $ ~drums * 0.6 + ~bass * 0.3
    "#;

    let (_, statements) = parse_dsl(input).expect("Should parse complex patterns");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(44100);

    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(rms > 0.01, "Complex patterns should work, got RMS={}", rms);
}
