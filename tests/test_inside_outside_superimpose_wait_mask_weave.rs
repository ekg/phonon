// Test inside, outside, superimpose, wait, mask, and weave pattern transforms
//
// These operations modify pattern timing and structure:
// - inside: apply transform inside time range
// - outside: apply transform outside time range
// - superimpose: layer pattern on itself
// - wait: delay pattern by cycles
// - mask: apply boolean mask to pattern
// - weave: weave pattern

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

/// Helper to compile code and verify it succeeds
fn test_compilation(code: &str, description: &str) {
    let (rest, statements) = parse_program(code).unwrap_or_else(|e| {
        panic!("{} - Parse failed: {:?}", description, e)
    });
    assert_eq!(
        rest.trim(),
        "",
        "{} - Parser didn't consume all input",
        description
    );

    compile_program(statements, 44100.0).unwrap_or_else(|e| {
        panic!("{} - Compilation failed: {}", description, e)
    });
}

/// Helper to compile code and verify it fails with expected error
fn test_compilation_error(code: &str, description: &str, expected_error_substring: &str) {
    let (rest, statements) = parse_program(code).unwrap_or_else(|e| {
        panic!("{} - Parse failed: {:?}", description, e)
    });
    assert_eq!(
        rest.trim(),
        "",
        "{} - Parser didn't consume all input",
        description
    );

    match compile_program(statements, 44100.0) {
        Ok(_) => panic!("{} - Expected compilation to fail but it succeeded", description),
        Err(e) => {
            assert!(
                e.contains(expected_error_substring),
                "{} - Error message '{}' does not contain expected substring '{}'",
                description,
                e,
                expected_error_substring
            );
        }
    }
}

// ========== Inside Tests ==========

#[test]
fn test_inside_basic() {
    // Test: apply fast transform inside first half of cycle
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ inside 0.0 0.5 fast 2
"#,
        "Inside 0.0-0.5 with fast 2",
    );
}

#[test]
fn test_inside_second_half() {
    // Test: apply rev inside second half
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ inside 0.5 1.0 rev
"#,
        "Inside 0.5-1.0 with rev",
    );
}

#[test]
fn test_inside_middle_quarter() {
    // Test: apply transform in middle quarter
    test_compilation(
        r#"
tempo: 2.0
out: "bd*8" $ inside 0.25 0.75 fast 4
"#,
        "Inside 0.25-0.75 with fast 4",
    );
}

#[test]
fn test_inside_with_effects() {
    // Test: inside pattern through reverb
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*2" $ inside 0.0 0.5 rev # reverb 0.5 0.3 0.2
"#,
        "Inside with reverb",
    );
}

#[test]
fn test_inside_nested() {
    // Test: nested inside transforms
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ inside 0.0 0.5 fast 2 $ inside 0.5 1.0 slow 2
"#,
        "Nested inside transforms",
    );
}

// ========== Outside Tests ==========

#[test]
fn test_outside_basic() {
    // Test: apply fast transform outside first half of cycle
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ outside 0.0 0.5 fast 2
"#,
        "Outside 0.0-0.5 with fast 2",
    );
}

#[test]
fn test_outside_middle_half() {
    // Test: apply rev outside middle half
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ outside 0.25 0.75 rev
"#,
        "Outside 0.25-0.75 with rev",
    );
}

#[test]
fn test_outside_with_subdivision() {
    // Test: outside with subdivision pattern
    test_compilation(
        r#"
tempo: 2.0
out: "bd*8" $ outside 0.5 1.0 fast 4
"#,
        "Outside 0.5-1.0 with fast 4",
    );
}

#[test]
fn test_outside_with_effects() {
    // Test: outside pattern through delay
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4" $ outside 0.25 0.75 rev # delay 0.25 0.5 0.3
"#,
        "Outside with delay",
    );
}

#[test]
fn test_inside_and_outside_together() {
    // Test: inside and outside in same program
    test_compilation(
        r#"
tempo: 2.0
~inside_pat: "bd sn" $ inside 0.0 0.5 fast 2
~outside_pat: "hh cp" $ outside 0.0 0.5 rev
out: ~inside_pat + ~outside_pat
"#,
        "Inside and outside in same program",
    );
}

// ========== Superimpose Tests (Not Fully Implemented) ==========

#[test]
fn test_superimpose_not_implemented() {
    // Test: superimpose requires function argument - not yet exposed
    test_compilation_error(
        r#"
tempo: 2.0
out: "bd sn" $ superimpose
"#,
        "Superimpose should not be fully implemented",
        "superimpose transform requires a function argument",
    );
}

// ========== Wait Tests ==========

#[test]
fn test_wait_basic() {
    // Test: wait/delay pattern by 1 cycle
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ wait 1.0
"#,
        "Wait 1 cycle",
    );
}

#[test]
fn test_wait_half_cycle() {
    // Test: wait half cycle
    test_compilation(
        r#"
tempo: 2.0
out: "bd*8" $ wait 0.5
"#,
        "Wait 0.5 cycles",
    );
}

#[test]
fn test_wait_multiple_cycles() {
    // Test: wait multiple cycles
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4" $ wait 4.0
"#,
        "Wait 4 cycles",
    );
}

#[test]
fn test_wait_with_effects() {
    // Test: waited pattern through reverb
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*2" $ wait 2.0 # reverb 0.5 0.3 0.2
"#,
        "Wait with reverb",
    );
}

#[test]
fn test_wait_combined() {
    // Test: wait combined with other transforms
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh" $ wait 1.0 $ fast 2
"#,
        "Wait combined with fast",
    );
}

// ========== Mask Tests (Not Fully Implemented) ==========

#[test]
fn test_mask_not_implemented() {
    // Test: mask should fail with not implemented error
    test_compilation_error(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ mask "1 0 1 0"
"#,
        "Mask should not be fully implemented",
        "mask transform is not yet fully implemented",
    );
}

// ========== Weave Tests (Not Fully Implemented) ==========

#[test]
fn test_weave_not_implemented() {
    // Test: weave requires pattern argument - not yet exposed
    test_compilation_error(
        r#"
tempo: 2.0
out: "bd sn" $ weave 4
"#,
        "Weave should not be fully implemented",
        "weave transform requires a pattern argument",
    );
}

// ========== Combined Tests ==========

#[test]
fn test_working_operations_in_program() {
    // Test: using working operations in same program (inside, outside, wait)
    test_compilation(
        r#"
tempo: 2.0
~inside_pat: "bd*8" $ inside 0.0 0.5 fast 2
~outside_pat: "sn*4" $ outside 0.25 0.75 rev
~waited: "cp*2" $ wait 1.0
out: ~inside_pat + ~outside_pat + ~waited
"#,
        "Multiple working operations in one program",
    );
}

#[test]
fn test_outside_and_wait() {
    // Test: outside and wait together
    test_compilation(
        r#"
tempo: 2.0
~outside_pat: "bd*4 sn*4" $ outside 0.25 0.75 rev
~waited: "hh*4 cp*4" $ wait 2.0
out: ~outside_pat + ~waited
"#,
        "Outside and wait together",
    );
}

#[test]
fn test_complex_combination() {
    // Test: complex combination of working operations
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ inside 0.0 0.5 fast 2 $ wait 1.0
"#,
        "Complex combination: inside, wait",
    );
}

#[test]
fn test_with_effects_chain() {
    // Test: multiple operations with effects chain
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4" $ inside 0.25 0.75 fast 2 # lpf 1000 0.8 # reverb 0.5 0.3 0.2
"#,
        "Multiple operations with effects chain",
    );
}

#[test]
fn test_in_complex_multi_bus_program() {
    // Test: working operations in complex multi-bus program
    test_compilation(
        r#"
tempo: 2.0
~kick: "bd*4" $ inside 0.0 0.5 fast 2 $ wait 0.5
~snare: "~ sn ~ sn" $ outside 0.25 0.75 rev
~hats: "hh*8" $ wait 1.0
~perc: "cp*4" $ wait 1.0
~mixed: (~kick + ~snare) $ inside 0.0 0.25 fast 4
out: ~mixed * 0.5 + ~hats * 0.3 + ~perc * 0.2
"#,
        "Complex multi-bus program with working operations",
    );
}

#[test]
fn test_nested_inside_outside() {
    // Test: nested inside and outside
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ inside 0.0 0.5 fast 2 $ outside 0.25 0.75 slow 2
"#,
        "Nested inside and outside",
    );
}
