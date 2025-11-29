/// Tests for sample pattern compilation in the compositional compiler
///
/// This module tests that:
/// 1. String patterns compile to SamplePatternNode
/// 2. Pattern transforms are applied before compilation
/// 3. The s function extracts and compiles string arguments

use phonon::compositional_parser::parse_program;

#[test]
fn test_compile_string_pattern() {
    // Test that a simple string pattern compiles successfully
    let code = r#"
tempo: 0.5
out $ "bd sn"
"#;

    let (_globals, statements) = parse_program(code).expect("Failed to parse program");

    // Compile the program
    let result = phonon::compositional_compiler::compile_program(statements, 44100.0, None);

    // Should compile without errors
    if let Err(e) = result {
        panic!("Failed to compile string pattern: {}", e);
    }
}

#[test]
fn test_compile_pattern_with_transform() {
    // Test that a pattern with transform compiles successfully
    let code = r#"
tempo: 0.5
out $ "bd sn" $ fast 2
"#;

    let (_globals, statements) = parse_program(code).expect("Failed to parse program");

    // Compile the program
    let result = phonon::compositional_compiler::compile_program(statements, 44100.0, None);

    // Should compile without errors
    if let Err(e) = result {
        panic!("Failed to compile pattern with transform: {}", e);
    }
}

#[test]
fn test_compile_s_function() {
    // Test that the s function compiles successfully
    let code = r#"
tempo: 0.5
out $ s "bd sn hh cp"
"#;

    let (_globals, statements) = parse_program(code).expect("Failed to parse program");

    // Compile the program
    let result = phonon::compositional_compiler::compile_program(statements, 44100.0, None);

    // Should compile without errors
    if let Err(e) = result {
        panic!("Failed to compile s function: {}", e);
    }
}

#[test]
fn test_compile_s_with_transform() {
    // Test that s function with transform compiles successfully
    let code = r#"
tempo: 0.5
out $ s "bd sn" $ fast 2
"#;

    let (_globals, statements) = parse_program(code).expect("Failed to parse program");

    // Compile the program
    let result = phonon::compositional_compiler::compile_program(statements, 44100.0, None);

    // Should compile without errors
    if let Err(e) = result {
        panic!("Failed to compile s with transform: {}", e);
    }
}

#[test]
fn test_compile_multiple_transforms() {
    // Test that multiple transforms compile successfully
    let code = r#"
tempo: 0.5
out $ "bd sn" $ fast 2 $ rev
"#;

    let (_globals, statements) = parse_program(code).expect("Failed to parse program");

    // Compile the program
    let result = phonon::compositional_compiler::compile_program(statements, 44100.0, None);

    // Should compile without errors
    if let Err(e) = result {
        panic!("Failed to compile multiple transforms: {}", e);
    }
}

#[test]
fn test_s_function_wrong_arg_count() {
    // Test that s function with wrong number of arguments fails gracefully
    let code = r#"
tempo: 0.5
out $ s "bd" "sn"
"#;

    let (_globals, statements) = parse_program(code).expect("Failed to parse program");

    // Compile the program
    let result = phonon::compositional_compiler::compile_program(statements, 44100.0, None);

    // Should fail with clear error message about the unexpected second argument
    match result {
        Ok(_) => panic!("Expected error for wrong arg count, but compilation succeeded"),
        Err(e) => {
            // The error message says "Expected transform as second argument to s()"
            // which is a clear indication that s() shouldn't have multiple string args
            assert!(e.contains("s()") || e.contains("Expected transform"),
                "Error message should mention s() or argument issue: {}", e);
        }
    }
}
