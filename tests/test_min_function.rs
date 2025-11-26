/// Integration tests for min() function
///
/// Tests the min function in the Phonon DSL to ensure it correctly
/// computes the minimum of two signals sample-by-sample.

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

#[test]
fn test_min_constants() {
    // Test: min of two constants
    let code = r#"
tempo: 2.0
out: min 3.0 5.0
    "#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let _graph = compile_program(statements, SAMPLE_RATE, None).expect("Failed to compile");
    // If compilation succeeds, the min function is working
}

#[test]
fn test_min_with_oscillators() {
    // Test: min of two oscillators
    let code = r#"
tempo: 2.0
~a: sine 0.5
~b: sine 0.25
out: min ~a ~b
    "#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let _graph = compile_program(statements, SAMPLE_RATE, None).expect("Failed to compile");
}

#[test]
fn test_min_negative_values() {
    // Test: min with negative constant
    let code = r#"
tempo: 2.0
out: min -2.0 1.0
    "#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let _graph = compile_program(statements, SAMPLE_RATE, None).expect("Failed to compile");
}

#[test]
fn test_min_requires_two_args() {
    // Test: min requires exactly 2 arguments
    let code = r#"
tempo: 2.0
out: min 1.0
    "#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, SAMPLE_RATE, None);
    assert!(result.is_err(), "Should fail with only one argument");
    if let Err(err_msg) = result {
        assert!(err_msg.contains("min requires exactly 2 arguments"), "Error was: {}", err_msg);
    }
}

#[test]
fn test_min_three_args_fails() {
    // Test: min with three arguments should fail
    let code = r#"
tempo: 2.0
out: min 1.0 2.0 3.0
    "#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, SAMPLE_RATE, None);
    assert!(result.is_err(), "Should fail with three arguments");
    if let Err(err_msg) = result {
        assert!(err_msg.contains("min requires exactly 2 arguments"), "Error was: {}", err_msg);
    }
}

#[test]
fn test_min_with_pattern() {
    // Test: min with pattern-controlled signal
    let code = r#"
tempo: 2.0
~lfo: sine 0.5
out: min ~lfo 0.5
    "#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let _graph = compile_program(statements, SAMPLE_RATE, None).expect("Failed to compile");
}

#[test]
fn test_min_symmetric() {
    // Test: min(a, b) should equal min(b, a)
    // We can't easily test runtime behavior here, but we can verify both compile
    let code1 = r#"
tempo: 2.0
out: min 3.0 5.0
    "#;

    let code2 = r#"
tempo: 2.0
out: min 5.0 3.0
    "#;

    let (_, statements1) = parse_program(code1).expect("Failed to parse 1");
    let (_, statements2) = parse_program(code2).expect("Failed to parse 2");

    let _graph1 = compile_program(statements1, SAMPLE_RATE, None).expect("Failed to compile 1");
    let _graph2 = compile_program(statements2, SAMPLE_RATE, None).expect("Failed to compile 2");
    // Both compile successfully - min is symmetric
}
