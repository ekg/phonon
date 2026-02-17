//! Integration tests for polished error messages with musical context.
//!
//! Validates that compiler and parser errors are helpful, include suggestions,
//! and provide musical context for live coders.

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

/// Helper: compile code and return the error string
fn compile_err(code: &str) -> String {
    let (_, statements) = parse_program(code).expect("parse should succeed");
    match compile_program(statements, 44100.0, None) {
        Err(e) => e,
        Ok(_) => panic!("Expected compile error for code: {}", code),
    }
}

/// Helper: attempt to parse code and return remaining-input diagnostic
fn parse_remaining(code: &str) -> Option<String> {
    match parse_program(code) {
        Ok((remaining, _)) if !remaining.trim().is_empty() => {
            use phonon::error_diagnostics::diagnose_parse_failure;
            let diag = diagnose_parse_failure(code, remaining);
            Some(format!("{}", diag))
        }
        _ => None,
    }
}

// ========== Unknown Function Suggestions ==========

#[test]
fn test_unknown_function_suggests_similar() {
    // Typo: "revrb" should suggest "reverb"
    let err = compile_err("out $ revrb 0.5");
    assert!(
        err.contains("Unknown function"),
        "Expected 'Unknown function' in: {}",
        err
    );
    assert!(
        err.contains("Did you mean"),
        "Expected 'Did you mean' suggestion in: {}",
        err
    );
    assert!(
        err.contains("reverb"),
        "Expected 'reverb' suggestion in: {}",
        err
    );
}

#[test]
fn test_unknown_function_suggests_lpf_typo() {
    let err = compile_err("out $ sine 440 # lp 1000 0.8");
    assert!(err.contains("Unknown function"), "got: {}", err);
    // "lp" is close to "lpf"
    assert!(err.contains("lpf"), "Expected 'lpf' suggestion in: {}", err);
}

#[test]
fn test_unknown_function_no_suggestion_for_gibberish() {
    let err = compile_err("out $ xyzfoobar 42");
    assert!(err.contains("Unknown function"), "got: {}", err);
    // Should NOT contain "Did you mean" for something totally unrelated
    assert!(
        !err.contains("Did you mean"),
        "Should not suggest for gibberish: {}",
        err
    );
}

#[test]
fn test_parameter_modifier_arg_count_error() {
    // Calling gain with wrong arg count gives a clear message
    let err = compile_err("out $ gain 0.5");
    assert!(
        err.contains("gain") && err.contains("argument"),
        "Expected argument count error for gain in: {}",
        err
    );
}

// ========== Undefined Bus Suggestions ==========

#[test]
fn test_undefined_bus_suggests_similar() {
    let code = r#"
~drums $ s "bd sn"
out $ ~drum
"#;
    let err = compile_err(code);
    assert!(
        err.contains("Undefined bus"),
        "Expected 'Undefined bus' in: {}",
        err
    );
    assert!(
        err.contains("Did you mean") || err.contains("~drums"),
        "Expected suggestion for ~drums in: {}",
        err
    );
}

#[test]
fn test_undefined_bus_shows_available_buses() {
    let code = r#"
~kick $ s "bd*4"
~snare $ s "sn*2"
out $ ~hihat
"#;
    let err = compile_err(code);
    assert!(
        err.contains("Undefined bus"),
        "Expected 'Undefined bus' in: {}",
        err
    );
    assert!(
        err.contains("Available buses") || err.contains("~kick") || err.contains("~snare"),
        "Expected available buses in: {}",
        err
    );
}

// ========== Unknown Transform Suggestions ==========

#[test]
fn test_unknown_transform_suggests_similar() {
    let code = r#"out $ s "bd sn" $ fst 2"#;
    let err = compile_err(code);
    assert!(
        err.contains("Unknown transform"),
        "Expected 'Unknown transform' in: {}",
        err
    );
    assert!(
        err.contains("fast"),
        "Expected 'fast' suggestion in: {}",
        err
    );
}

#[test]
fn test_unknown_transform_rev_typo() {
    // Use a transform syntax that gets parsed correctly as a transform call
    let code = r#"out $ s "bd sn" $ rve 1"#;
    let err = compile_err(code);
    assert!(
        err.contains("Unknown transform") || err.contains("Unknown function"),
        "Expected error for 'rve' in: {}",
        err
    );
    // "rve" should suggest "rev"
    assert!(
        err.contains("rev"),
        "Expected 'rev' suggestion in: {}",
        err
    );
}

// ========== Unknown Sample Parameter Suggestions ==========

#[test]
fn test_unknown_sample_param_suggests_similar() {
    // Test the unknown sample parameter via the # chain modifier path
    // s "bd" # gain 0.5 works; test that a typo like "gian" is caught
    // In the function call path, "gian" as a standalone function should suggest "gain"
    let code = r#"out $ sine 440 # gian 0.5"#;
    let err = compile_err(code);
    assert!(
        err.contains("Unknown function") || err.contains("Unknown"),
        "Expected unknown function error in: {}",
        err
    );
    assert!(
        err.contains("gain"),
        "Expected 'gain' suggestion in: {}",
        err
    );
}

// ========== Parse Error Diagnostics ==========

#[test]
fn test_parse_error_hash_comment() {
    let diag = parse_remaining("tempo: 0.5\n# This is a comment\nout: sine 440");
    assert!(diag.is_some(), "Expected diagnostic for # comment");
    let msg = diag.unwrap();
    assert!(
        msg.contains("chain operator"),
        "Expected # hint in: {}",
        msg
    );
    assert!(msg.contains("--"), "Expected -- suggestion in: {}", msg);
}

#[test]
fn test_parse_error_parentheses_syntax() {
    let diag = parse_remaining("~kick: s(\"bd*4\")");
    assert!(diag.is_some(), "Expected diagnostic for s() syntax");
    let msg = diag.unwrap();
    assert!(
        msg.contains("space-separated"),
        "Expected space-separated hint in: {}",
        msg
    );
}

// ========== Error Message Formatting ==========

#[test]
fn test_error_message_is_not_debug_format() {
    // Ensure error messages are human-readable, not Rust debug output
    let err = compile_err("out $ unknownfunc 42");
    assert!(
        !err.contains("Err("),
        "Error should not contain Rust debug format: {}",
        err
    );
    assert!(
        !err.contains("Ok("),
        "Error should not contain Rust debug format: {}",
        err
    );
}

#[test]
fn test_arg_count_error_shows_usage() {
    // lpf needs input + cutoff + resonance, but we only provide cutoff (chained input counts as 1)
    // When chained: input # lpf needs cutoff and resonance
    // Standalone: lpf needs input, cutoff, resonance
    let err = compile_err("out $ lpf");
    assert!(
        err.contains("argument") || err.contains("requires"),
        "Expected argument count info in: {}",
        err
    );
}

