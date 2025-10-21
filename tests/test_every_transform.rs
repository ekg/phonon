// Test every pattern transform
//
// The every operation applies a transform every n cycles:
// - every 2 fast 2: apply fast 2 on cycles 0, 2, 4, 6, ...
// - every 3 rev: reverse pattern on cycles 0, 3, 6, 9, ...
// - Can be chained with other transforms and effects

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

    compile_program(statements, 44100.0)
        .unwrap_or_else(|e| panic!("{} - Compilation failed: {}", description, e));
}

// ========== Basic Every Tests ==========

#[test]
fn test_every_basic() {
    // Test: apply fast 2 every 2 cycles
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ every 2 fast 2
"#,
        "Every 2 cycles apply fast 2",
    );
}

#[test]
fn test_every_with_rev() {
    // Test: reverse every 3 cycles
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ every 3 rev
"#,
        "Every 3 cycles apply rev",
    );
}

#[test]
fn test_every_with_slow() {
    // Test: slow down every 4 cycles
    test_compilation(
        r#"
tempo: 2.0
out: "bd*8" $ every 4 slow 2
"#,
        "Every 4 cycles apply slow 2",
    );
}

#[test]
fn test_every_large_interval() {
    // Test: large cycle interval
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4" $ every 8 fast 4
"#,
        "Every 8 cycles apply fast 4",
    );
}

#[test]
fn test_every_with_degrade() {
    // Test: degrade every 2 cycles
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ every 2 degrade
"#,
        "Every 2 cycles apply degrade",
    );
}

// ========== Every with Time Manipulation ==========

#[test]
fn test_every_with_late() {
    // Test: delay pattern every 3 cycles
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ every 3 late 0.25
"#,
        "Every 3 cycles delay by 0.25",
    );
}

#[test]
fn test_every_with_early() {
    // Test: shift pattern earlier every 2 cycles
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4" $ every 2 early 0.125
"#,
        "Every 2 cycles shift early by 0.125",
    );
}

#[test]
fn test_every_with_dup() {
    // Test: duplicate pattern every 4 cycles
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn" $ every 4 dup 2
"#,
        "Every 4 cycles duplicate 2 times",
    );
}

#[test]
fn test_every_with_fit() {
    // Test: fit pattern to 2 cycles every 3 cycles
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ every 3 fit 2
"#,
        "Every 3 cycles fit to 2",
    );
}

#[test]
fn test_every_with_stretch() {
    // Test: stretch pattern every 2 cycles
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4" $ every 2 stretch
"#,
        "Every 2 cycles stretch",
    );
}

// ========== Every with Structural Operations ==========

#[test]
fn test_every_with_palindrome() {
    // Test: palindrome every 3 cycles
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ every 3 palindrome
"#,
        "Every 3 cycles palindrome",
    );
}

#[test]
fn test_every_with_stutter() {
    // Test: stutter every 2 cycles
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4" $ every 2 stutter 4
"#,
        "Every 2 cycles stutter 4 times",
    );
}

#[test]
fn test_every_with_chop() {
    // Test: chop every 4 cycles
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn" $ every 4 chop 8
"#,
        "Every 4 cycles chop into 8",
    );
}

#[test]
fn test_every_with_zoom() {
    // Test: zoom every 3 cycles
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ every 3 zoom 0.25 0.75
"#,
        "Every 3 cycles zoom to middle half",
    );
}

// ========== Every with Effects ==========

#[test]
fn test_every_with_effects_chain() {
    // Test: every combined with effects
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4" $ every 2 fast 2 # lpf 1000 0.8
"#,
        "Every 2 cycles fast 2, with lpf",
    );
}

#[test]
fn test_every_before_effects() {
    // Test: every applied before effects chain
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ every 3 rev # reverb 0.5 0.3 0.2 # lpf 2000 0.7
"#,
        "Every 3 cycles rev, then effects",
    );
}

// ========== Chained Every Tests ==========

#[test]
fn test_every_chained_with_fast() {
    // Test: every followed by fast
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ every 2 rev $ fast 2
"#,
        "Every 2 cycles rev, then fast 2",
    );
}

#[test]
fn test_every_chained_with_slow() {
    // Test: every followed by slow
    test_compilation(
        r#"
tempo: 2.0
out: "bd*8" $ every 3 fast 4 $ slow 2
"#,
        "Every 3 cycles fast 4, then slow 2",
    );
}

#[test]
fn test_multiple_every_operations() {
    // Test: multiple every operations in same program
    test_compilation(
        r#"
tempo: 2.0
~drums1: "bd sn" $ every 2 fast 2
~drums2: "hh*4 cp" $ every 3 rev
out: ~drums1 + ~drums2
"#,
        "Multiple every operations in program",
    );
}

// ========== Every with Pattern Types ==========

#[test]
fn test_every_with_euclidean() {
    // Test: every with euclidean pattern
    test_compilation(
        r#"
tempo: 2.0
out: "bd(3,8)" $ every 2 fast 2
"#,
        "Every 2 cycles on euclidean pattern",
    );
}

#[test]
fn test_every_with_alternation() {
    // Test: every with alternation pattern
    test_compilation(
        r#"
tempo: 2.0
out: "<bd sn hh cp>" $ every 3 rev
"#,
        "Every 3 cycles on alternation pattern",
    );
}

#[test]
fn test_every_with_subdivision() {
    // Test: every with subdivision
    test_compilation(
        r#"
tempo: 2.0
out: "bd*4 sn*2 hh*8" $ every 2 slow 2
"#,
        "Every 2 cycles on subdivision pattern",
    );
}

// ========== Edge Cases ==========

#[test]
fn test_every_1() {
    // Test: every 1 cycle (always applies transform)
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ every 1 fast 2
"#,
        "Every 1 cycle (always applies)",
    );
}

#[test]
fn test_every_with_degradeBy() {
    // Test: every with probabilistic transform
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4 cp" $ every 2 degradeBy 0.5
"#,
        "Every 2 cycles degrade by 50%",
    );
}

// ========== Complex Combinations ==========

#[test]
fn test_every_complex_chain() {
    // Test: every with multiple chained transforms
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ every 2 fast 2 $ every 3 rev $ slow 1.5
"#,
        "Multiple every operations chained",
    );
}

#[test]
fn test_every_with_all_timing_ops() {
    // Test: every combined with timing operations
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4" $ every 2 late 0.25 $ every 3 early 0.125 $ every 4 dup 2
"#,
        "Every with multiple timing operations",
    );
}

#[test]
fn test_every_in_complex_program() {
    // Test: every used in complex multi-bus program
    test_compilation(
        r#"
tempo: 2.0
~kick: "bd*4" $ every 4 fast 2
~snare: "~ sn ~ sn" $ every 3 rev
~hats: "hh*8" $ every 2 degrade
~perc: "cp*4" $ every 2 late 0.125
out: ~kick * 0.4 + ~snare * 0.3 + ~hats * 0.2 + ~perc * 0.1
"#,
        "Every in complex multi-bus program",
    );
}
