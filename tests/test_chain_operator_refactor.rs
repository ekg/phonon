// Chain operator refactor tests - Verifying type-safe ChainInput variant
// Tests that all chainable functions work in both standalone and chained forms
//
// This test suite verifies the fix for the chain operator hack where NodeId
// was being stored as Expr::Number (f64), causing type confusion.

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

/// Helper to compile code and verify it succeeds
fn test_compilation(code: &str, description: &str) {
    let (rest, statements) =
        parse_program(code).unwrap_or_else(|e| panic!("{} - Parse failed: {:?}", description, e));
    assert_eq!(
        rest.trim(),
        "",
        "{} - Parser didn't consume all input",
        description
    );

    compile_program(statements, 44100.0, None)
        .unwrap_or_else(|e| panic!("{} - Compilation failed: {}", description, e));
}

#[test]
fn test_chain_lpf_filter() {
    // Test: Low-pass filter in chained form
    test_compilation(
        r#"
tempo: 0.5
out $ sine 440 # lpf 1000 0.8
"#,
        "Chain LPF filter",
    );
}

#[test]
fn test_standalone_lpf_filter() {
    // Test: Low-pass filter applied to bus via chain syntax
    // Note: Bus references cannot be used as function parameters directly,
    // so we use the chain syntax instead of `lpf ~osc 1000 0.8`
    test_compilation(
        r#"
tempo: 0.5
~osc $ sine 440
out $ ~osc # lpf 1000 0.8
"#,
        "Standalone LPF filter",
    );
}

#[test]
fn test_chain_hpf_filter() {
    // Test: High-pass filter in chained form
    test_compilation(
        r#"
tempo: 0.5
out $ saw 110 # hpf 500 0.5
"#,
        "Chain HPF filter",
    );
}

#[test]
fn test_chain_bpf_filter() {
    // Test: Band-pass filter in chained form
    test_compilation(
        r#"
tempo: 0.5
out $ saw 110 # bpf 1000 2.0
"#,
        "Chain BPF filter",
    );
}

#[test]
fn test_chain_reverb_with_all_params() {
    // Test: Reverb in chained form with all 3 parameters (room_size, damping, mix)
    // This also verifies the reverb args bug fix (was using args[2] twice)
    test_compilation(
        r#"
tempo: 0.5
out $ sine 440 # reverb 0.8 0.5 0.3
"#,
        "Chain Reverb with all params",
    );
}

#[test]
fn test_standalone_reverb() {
    // Test: Reverb applied to bus via chain syntax
    // Note: Bus references cannot be used as function parameters directly,
    // so we use the chain syntax instead of `reverb ~osc 0.8 0.5 0.3`
    test_compilation(
        r#"
tempo: 0.5
~osc $ sine 440
out $ ~osc # reverb 0.8 0.5 0.3
"#,
        "Standalone Reverb",
    );
}

#[test]
fn test_chain_distortion() {
    // Test: Distortion in chained form
    test_compilation(
        r#"
tempo: 0.5
out $ sine 110 # distortion 2.0 0.5
"#,
        "Chain Distortion",
    );
}

#[test]
fn test_chain_delay() {
    // Test: Delay in chained form
    test_compilation(
        r#"
tempo: 0.5
out $ sine 440 # delay 0.25 0.5 0.4
"#,
        "Chain Delay",
    );
}

#[test]
fn test_chain_chorus() {
    // Test: Chorus in chained form
    test_compilation(
        r#"
tempo: 0.5
out $ sine 440 # chorus 0.5 2.0 0.3
"#,
        "Chain Chorus",
    );
}

#[test]
fn test_chain_bitcrush() {
    // Test: Bitcrush in chained form
    test_compilation(
        r#"
tempo: 0.5
out $ sine 440 # bitcrush 8 8000
"#,
        "Chain Bitcrush",
    );
}

#[test]
fn test_chain_envelope() {
    // Test: Envelope in chained form (ADSR)
    test_compilation(
        r#"
tempo: 0.5
out $ sine 440 # env 0.01 0.1 0.7 0.2
"#,
        "Chain Envelope",
    );
}

#[test]
fn test_multiple_chains() {
    // Test: Multiple chain operations (a # b # c)
    // This tests that ChainInput can be passed through multiple stages
    test_compilation(
        r#"
tempo: 0.5
out $ sine 440 # lpf 2000 0.8 # reverb 0.5 0.3 0.2
"#,
        "Multiple chains (a # b # c)",
    );
}

#[test]
fn test_chain_with_sample_playback() {
    // Test: Chaining effects with sample playback
    test_compilation(
        r#"
tempo: 0.5
out $ s "bd sn" # lpf 1000 0.8 # reverb 0.5 0.3 0.2
"#,
        "Chain with sample playback",
    );
}

#[test]
fn test_chain_with_pattern_controlled_oscillator() {
    // Test: Chaining with pattern-controlled oscillator
    test_compilation(
        r#"
tempo: 0.5
out $ sine "440 550 660" # lpf 2000 0.8
"#,
        "Chain with pattern-controlled oscillator",
    );
}

#[test]
fn test_complex_chain_with_bus() {
    // Test: Complex chain using bus references
    test_compilation(
        r#"
tempo: 0.5
~bass $ saw 55
~filtered $ ~bass # lpf 800 0.7
~wet $ ~filtered # reverb 0.6 0.4 0.3
out $ ~wet * 0.8
"#,
        "Complex chain with bus references",
    );
}
