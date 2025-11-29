/// Tests for chain operator (#) combined with binary operations (+, -, *, /)
///
/// These tests verify that patterns like:
///   s "bd" # note "c3'maj" + "0 3 7"
/// compile correctly, where the + operator is applied to the modifier argument.
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use phonon::unified_graph::UnifiedSignalGraph;

/// Helper to verify code compiles successfully
fn compile_code(code: &str) -> Result<UnifiedSignalGraph, String> {
    let (rest, stmts) = parse_program(code).map_err(|e| format!("Parse error: {}", e))?;
    if !rest.trim().is_empty() {
        return Err(format!("Parser did not consume all input: {:?}", rest));
    }
    compile_program(stmts, 44100.0, None)
}

#[test]
fn test_note_chord_plus_offset() {
    // This is the original bug case: note "c3'maj" + "0 3 7"
    // The + should apply to the note argument, not create a separate expression
    let code = r#"~synth $ saw 55
o2 $ s "~synth" # note "c3'maj" + "0 3 7" # gain 0.3"#;

    match compile_code(code) {
        Ok(graph) => {
            assert_eq!(graph.get_cps(), 0.5, "Default CPS should be 0.5");
        }
        Err(e) => panic!("Should compile: {}", e),
    }
}

#[test]
fn test_chain_with_simple_add() {
    // s "bd" # n "0" + "1"
    // The n parameter should receive the result of "0" + "1"
    let code = r#"out $ s "bd" # n "0" + "1""#;

    match compile_code(code) {
        Ok(_) => (),
        Err(e) => panic!("Should compile: {}", e),
    }
}

#[test]
fn test_chain_with_multiply() {
    // Test multiplication in chain modifier argument
    let code = r#"out $ s "bd" # gain "0.5" * "2""#;

    match compile_code(code) {
        Ok(_) => (),
        Err(e) => panic!("Should compile: {}", e),
    }
}

#[test]
fn test_chain_lpf_with_add() {
    // Test filter with arithmetic on cutoff
    // lpf takes (cutoff, resonance) - use parentheses to group the arithmetic
    let code = r#"out $ saw 110 # lpf ("1000" + "500") 0.8"#;

    match compile_code(code) {
        Ok(_) => (),
        Err(e) => panic!("Should compile: {}", e),
    }
}

#[test]
fn test_multiple_chains_with_binops() {
    // Test multiple chained modifiers each with binary ops
    let code = r#"out $ s "bd" # n "0" + "1" # gain "0.5" * "1.5""#;

    match compile_code(code) {
        Ok(_) => (),
        Err(e) => panic!("Should compile: {}", e),
    }
}

#[test]
fn test_structure_operator_in_chain() {
    // Test structure-taking operators (|+) in chain
    let code = r#"out $ s "bd" # note "c4" |+ "0 12""#;

    match compile_code(code) {
        Ok(_) => (),
        Err(e) => panic!("Should compile: {}", e),
    }
}

#[test]
fn test_note_with_pattern_arithmetic() {
    // Various note arithmetic patterns
    let codes = [
        r#"out $ s "bd" # note "0 5 7" + "12""#,
        r#"out $ s "bd" # note "c4 e4 g4" + "0""#,
        r#"out $ s "bd" # note "c3'maj" - "12""#,
    ];

    for code in codes {
        match compile_code(code) {
            Ok(_) => (),
            Err(e) => panic!("Should compile '{}': {}", code, e),
        }
    }
}

#[test]
fn test_chain_preserves_signal() {
    // Verify the signal chain compiles and can render
    let code = r#"~synth $ sine 440
out $ ~synth # gain 0.5"#;

    match compile_code(code) {
        Ok(mut graph) => {
            // Use the render method which handles output correctly
            let output = graph.render(4410); // 0.1 second at 44100 Hz

            let rms: f32 = (output.iter().map(|s| s * s).sum::<f32>() / output.len() as f32).sqrt();
            assert!(rms > 0.01, "Output should have signal, got RMS={}", rms);
        }
        Err(e) => panic!("Should compile: {}", e),
    }
}
