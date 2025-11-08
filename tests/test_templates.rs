//! Tests for template functionality (@name: expression)
//!
//! Templates allow defining reusable transforms and effect chains once and applying them multiple times.

use phonon::compositional_parser::parse_program;
use phonon::compositional_compiler::compile_program;

#[test]
fn test_template_simple_constant() {
    // Define a template that's just a constant, use it in output
    let code = r#"
        @gain: 0.5
        out: sine 440 * @gain
    "#;

    let (_, statements) = parse_program(code).expect("Parse failed");
    let graph = compile_program(statements, 44100.0);

    // Should compile successfully without errors
    assert!(graph.is_ok());
}

#[test]
fn test_template_transform() {
    // Define a transform template and apply it
    let code = r#"
        tempo: 2.0
        @swing: swing 0.6
        out: s "bd sn" $ @swing
    "#;

    let (_, statements) = parse_program(code).expect("Parse failed");
    let result = compile_program(statements, 44100.0);

    // Should compile successfully
    assert!(result.is_ok());
}

#[test]
fn test_template_effect_chain() {
    // Define an effect chain template and apply it
    let code = r#"
        @heavy: lpf 800 0.9 # distortion 0.4
        out: s "bd" # @heavy
    "#;

    let (_, statements) = parse_program(code).expect("Parse failed");
    let result = compile_program(statements, 44100.0);

    // Should compile successfully
    assert!(result.is_ok());
}

#[test]
fn test_template_multiple_uses() {
    // Define a template and use it multiple times
    let code = r#"
        @filt: lpf 1000 0.8
        ~bass: saw 55 # @filt
        ~lead: sine 440 # @filt
        out: ~bass + ~lead
    "#;

    let (_, statements) = parse_program(code).expect("Parse failed");
    let result = compile_program(statements, 44100.0);

    // Should compile successfully
    assert!(result.is_ok());
}

#[test]
fn test_template_undefined_error() {
    // Try to use an undefined template
    let code = r#"
        out: sine 440 * @undefined
    "#;

    let (_, statements) = parse_program(code).expect("Parse failed");
    let result = compile_program(statements, 44100.0);

    // Should fail with undefined template error
    assert!(result.is_err());
    if let Err(err) = result {
        assert!(err.contains("Undefined template: @undefined"));
    }
}

#[test]
fn test_template_chained_transforms() {
    // Template with chained transforms
    let code = r#"
        @crazy: fast 2 $ rev
        out: s "bd sn hh cp" $ @crazy
    "#;

    let (_, statements) = parse_program(code).expect("Parse failed");
    let result = compile_program(statements, 44100.0);

    // Should compile successfully
    assert!(result.is_ok());
}

#[test]
fn test_template_in_bus() {
    // Use template in bus definition
    let code = r#"
        @verb: reverb 0.3 0.5
        ~wet: s "cp" # @verb
        out: ~wet
    "#;

    let (_, statements) = parse_program(code).expect("Parse failed");
    let result = compile_program(statements, 44100.0);

    // Should compile successfully
    assert!(result.is_ok());
}
