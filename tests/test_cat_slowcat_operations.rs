// Test cat and slowcat pattern combinators
//
// These operations concatenate or alternate between patterns:
// - cat: concatenates patterns within each cycle (each gets 1/n of cycle time)
// - slowcat: alternates between patterns on each cycle

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

// ========== Cat Tests ==========

#[test]
fn test_cat_two_patterns() {
    // Test: cat with 2 patterns - each gets half the cycle
    test_compilation(
        r#"
tempo: 0.5
out: cat ["bd", "sn"]
"#,
        "Cat with 2 patterns",
    );
}

#[test]
fn test_cat_three_patterns() {
    // Test: cat with 3 patterns - each gets 1/3 of the cycle
    test_compilation(
        r#"
tempo: 0.5
out: cat ["bd", "sn", "hh"]
"#,
        "Cat with 3 patterns",
    );
}

#[test]
fn test_cat_single_pattern() {
    // Test: cat with single pattern (edge case) - should work like regular pattern
    test_compilation(
        r#"
tempo: 0.5
out: cat ["bd*4"]
"#,
        "Cat with single pattern",
    );
}

#[test]
fn test_cat_complex_patterns() {
    // Test: cat with complex mini-notation patterns
    test_compilation(
        r#"
tempo: 0.5
out: cat ["bd*4", "~ sn ~ sn", "hh*8"]
"#,
        "Cat with complex patterns",
    );
}

#[test]
fn test_cat_with_effects() {
    // Test: cat pattern routed through effects
    test_compilation(
        r#"
tempo: 0.5
out: cat ["bd", "sn", "hh"] # lpf 1000 0.8
"#,
        "Cat with filter effect",
    );
}

// ========== Slowcat Tests ==========

#[test]
fn test_slowcat_two_patterns() {
    // Test: slowcat with 2 patterns - alternates each cycle
    test_compilation(
        r#"
tempo: 0.5
out: slowcat ["bd*4", "sn*4"]
"#,
        "Slowcat with 2 patterns",
    );
}

#[test]
fn test_slowcat_three_patterns() {
    // Test: slowcat with 3 patterns - cycles through 3 different patterns
    test_compilation(
        r#"
tempo: 0.5
out: slowcat ["bd*4", "sn*4", "hh*8"]
"#,
        "Slowcat with 3 patterns",
    );
}

#[test]
fn test_slowcat_single_pattern() {
    // Test: slowcat with single pattern (edge case) - should work like regular pattern
    test_compilation(
        r#"
tempo: 0.5
out: slowcat ["bd ~ sn ~"]
"#,
        "Slowcat with single pattern",
    );
}

#[test]
fn test_slowcat_complex_patterns() {
    // Test: slowcat with complex mini-notation patterns
    test_compilation(
        r#"
tempo: 0.5
out: slowcat ["bd(3,8)", "sn*4", "<hh oh>*8"]
"#,
        "Slowcat with complex patterns",
    );
}

#[test]
fn test_slowcat_with_effects() {
    // Test: slowcat pattern routed through effects
    test_compilation(
        r#"
tempo: 0.5
out: slowcat ["bd*4", "sn*2"] # reverb 0.5 0.3 0.2
"#,
        "Slowcat with reverb",
    );
}

// ========== Combined Tests ==========

#[test]
fn test_cat_and_slowcat_mixed() {
    // Test: using both cat and slowcat in same program
    test_compilation(
        r#"
tempo: 0.5
~drums1: cat ["bd", "sn", "hh"]
~drums2: slowcat ["bd*4", "sn*4"]
out: ~drums1 + ~drums2 * 0.5
"#,
        "Cat and slowcat mixed",
    );
}

// NOTE: Bus references in cat/slowcat don't work yet because buses are compiled to NodeIds
// This is a known limitation documented in KLUDGES_AND_IMPROVEMENTS.md
// To combine patterns from buses, you need to use stack instead:
//   ~combined: stack [~kicks, ~snares]
