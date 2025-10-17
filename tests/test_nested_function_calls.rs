//! Comprehensive tests for nested function calls with space-separated syntax
//!
//! Tests various levels of nesting to ensure parser handles all cases correctly

use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

#[test]
fn test_single_level_nesting() {
    // Single nested function call
    let code = r#"
        bpm 120
        out: reverb (sine 440) 0.7 0.5 0.5
    "#;

    let (remaining, statements) = parse_dsl(code).expect("Should parse single-level nesting");
    assert!(remaining.trim().is_empty(), "Should consume all input, remaining: {:?}", remaining);
    assert_eq!(statements.len(), 2);

    // Verify it compiles
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Verify it produces audio
    let buffer = graph.render(4410);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(rms > 0.001, "Single-level nesting should produce audio, got RMS={}", rms);
}

#[test]
fn test_double_level_nesting() {
    // Two levels of nesting: reverb(lpf(sine 440, ...))
    let code = r#"
        bpm 120
        out: reverb (lpf (sine 440) 2000 0.8) 0.7 0.5 0.5
    "#;

    let (remaining, statements) = parse_dsl(code).expect("Should parse double-level nesting");
    assert!(remaining.trim().is_empty(), "Should consume all input");
    assert_eq!(statements.len(), 2);

    // Verify it compiles
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Verify it produces audio
    let buffer = graph.render(4410);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(rms > 0.001, "Double-level nesting should produce audio, got RMS={}", rms);
}

#[test]
fn test_triple_level_nesting() {
    // Three levels: reverb(lpf(sine(pattern), ...), ...)
    let code = r#"
        bpm 120
        out: reverb (delay (lpf (sine 440) 1000 0.8) 0.25 0.5 0.5) 0.7 0.5 0.5
    "#;

    let (remaining, statements) = parse_dsl(code).expect("Should parse triple-level nesting");
    assert!(remaining.trim().is_empty(), "Should consume all input");
    assert_eq!(statements.len(), 2);

    // Verify it compiles
    let compiler = DslCompiler::new(44100.0);
    let _graph = compiler.compile(statements);
}

#[test]
fn test_nesting_with_arithmetic() {
    // Nesting combined with arithmetic operations
    let code = r#"
        bpm 120
        ~lfo: sine 0.5
        out: lpf (sine 440) (~lfo * 2000 + 500) 0.8 * 0.3
    "#;

    let (remaining, statements) = parse_dsl(code).expect("Should parse nesting with arithmetic");
    assert!(remaining.trim().is_empty(), "Should consume all input");
    assert_eq!(statements.len(), 3);

    // Verify it compiles
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Verify it produces audio
    let buffer = graph.render(4410);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(rms > 0.001, "Nesting with arithmetic should produce audio, got RMS={}", rms);
}

#[test]
fn test_multiple_nested_calls_in_expression() {
    // Multiple separate nested calls in same expression
    let code = r#"
        bpm 120
        out: (lpf (sine 440) 1000 0.8) + (lpf (sine 220) 2000 0.5) * 0.5
    "#;

    let (remaining, statements) = parse_dsl(code).expect("Should parse multiple nested calls");
    assert!(remaining.trim().is_empty(), "Should consume all input");
    assert_eq!(statements.len(), 2);

    // Verify it compiles
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Verify it produces audio
    let buffer = graph.render(4410);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(rms > 0.001, "Multiple nested calls should produce audio, got RMS={}", rms);
}

#[test]
fn test_nesting_with_pattern_strings() {
    // Nesting with pattern strings as arguments
    let code = r#"
        bpm 120
        out: lpf (sine "220 440 330") 1500 0.8 * 0.3
    "#;

    let (remaining, statements) = parse_dsl(code).expect("Should parse nesting with patterns");
    assert!(remaining.trim().is_empty(), "Should consume all input");
    assert_eq!(statements.len(), 2);

    // Verify it compiles
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Verify it produces audio
    let buffer = graph.render(8820); // 2 cycles at 1 CPS
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(rms > 0.001, "Nesting with patterns should produce audio, got RMS={}", rms);
}

#[test]
fn test_nesting_with_sample_patterns() {
    // Nesting with sample patterns
    let code = r#"
        bpm 120
        out: reverb (s "bd sn") 0.5 0.3 0.4 * 50
    "#;

    let (remaining, statements) = parse_dsl(code).expect("Should parse nesting with samples");
    assert!(remaining.trim().is_empty(), "Should consume all input");
    assert_eq!(statements.len(), 2);

    // Verify it compiles
    let compiler = DslCompiler::new(44100.0);
    let _graph = compiler.compile(statements);
}

#[test]
fn test_nesting_with_bus_refs() {
    // Nesting with bus references
    let code = r#"
        bpm 120
        ~osc: sine 440
        out: lpf ~osc 1000 0.8 * 0.3
    "#;

    let (remaining, statements) = parse_dsl(code).expect("Should parse nesting with bus refs");
    assert!(remaining.trim().is_empty(), "Should consume all input");
    assert_eq!(statements.len(), 3);

    // Verify it compiles
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Verify it produces audio
    let buffer = graph.render(4410);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(rms > 0.001, "Nesting with bus refs should produce audio, got RMS={}", rms);
}

#[test]
fn test_deeply_nested_with_mixed_types() {
    // Deep nesting with various argument types
    let code = r#"
        bpm 120
        ~lfo: sine 0.25
        out: reverb (delay (lpf (saw "110 220") (~lfo * 1000 + 500) 0.8) 0.25 0.5 0.3) 0.7 0.5 0.4 * 0.2
    "#;

    let (remaining, statements) = parse_dsl(code).expect("Should parse deeply nested mixed types");
    assert!(remaining.trim().is_empty(), "Should consume all input");
    assert_eq!(statements.len(), 3);

    // Verify it compiles
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Verify it produces audio
    let buffer = graph.render(8820);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(rms > 0.001, "Deep nesting with mixed types should produce audio, got RMS={}", rms);
}

#[test]
fn test_nesting_with_chaining() {
    // Nesting combined with # operator chaining
    let code = r#"
        bpm 120
        out: lpf (s "bd sn" # gain 0.8) 2000 0.5 * 50
    "#;

    let (remaining, statements) = parse_dsl(code).expect("Should parse nesting with chaining");
    assert!(remaining.trim().is_empty(), "Should consume all input");
    assert_eq!(statements.len(), 2);

    // Verify it compiles
    let compiler = DslCompiler::new(44100.0);
    let _graph = compiler.compile(statements);
}

#[test]
fn test_quadruple_level_nesting() {
    // Four levels of nesting - stress test
    let code = r#"
        bpm 120
        out: reverb (delay (lpf (sine 440) 1000 0.8) 0.25 0.5 0.3) 0.7 0.5 0.5 * 0.2
    "#;

    let (remaining, statements) = parse_dsl(code).expect("Should parse quadruple-level nesting");
    assert!(remaining.trim().is_empty(), "Should consume all input");
    assert_eq!(statements.len(), 2);

    // Verify it compiles without panicking
    let compiler = DslCompiler::new(44100.0);
    let _graph = compiler.compile(statements);
}

#[test]
fn test_parallel_nested_calls() {
    // Multiple nested calls at same level (in addition)
    let code = r#"
        bpm 120
        out: (reverb (sine 440) 0.5 0.3 0.4) + (delay (saw 220) 0.25 0.5 0.3) * 0.3
    "#;

    let (remaining, statements) = parse_dsl(code).expect("Should parse parallel nested calls");
    assert!(remaining.trim().is_empty(), "Should consume all input");
    assert_eq!(statements.len(), 2);

    // Verify it compiles
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Verify it produces audio
    let buffer = graph.render(4410);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(rms > 0.001, "Parallel nested calls should produce audio, got RMS={}", rms);
}

#[test]
fn test_asymmetric_nesting() {
    // Different nesting depths in same expression
    let code = r#"
        bpm 120
        out: (lpf (sine 440) 1000 0.8) + (sine 220) * 0.3
    "#;

    let (remaining, statements) = parse_dsl(code).expect("Should parse asymmetric nesting");
    assert!(remaining.trim().is_empty(), "Should consume all input");
    assert_eq!(statements.len(), 2);

    // Verify it compiles
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Verify it produces audio
    let buffer = graph.render(4410);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(rms > 0.001, "Asymmetric nesting should produce audio, got RMS={}", rms);
}

#[test]
fn test_nesting_with_all_numeric_args() {
    // Ensure numeric args are parsed correctly in nested context
    let code = r#"
        bpm 120
        out: delay (sine 440) 0.25 0.5 0.3 * 0.2
    "#;

    let (remaining, statements) = parse_dsl(code).expect("Should parse nesting with numeric args");
    assert!(remaining.trim().is_empty(), "Should consume all input");
    assert_eq!(statements.len(), 2);

    // Verify it compiles
    let compiler = DslCompiler::new(44100.0);
    let _graph = compiler.compile(statements);
}

#[test]
fn test_nesting_stops_at_operators() {
    // Ensure parser correctly stops at operators
    let code = r#"
        bpm 120
        out: (sine 440) * 0.5 + (saw 220) * 0.3
    "#;

    let (remaining, statements) = parse_dsl(code).expect("Should parse with operator boundaries");
    assert!(remaining.trim().is_empty(), "Should consume all input");
    assert_eq!(statements.len(), 2);

    // Verify it compiles
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Verify it produces audio
    let buffer = graph.render(4410);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(rms > 0.001, "Operator boundaries should work correctly, got RMS={}", rms);
}
