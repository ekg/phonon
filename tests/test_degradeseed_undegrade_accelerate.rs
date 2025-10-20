// Test degradeSeed, undegrade, and accelerate pattern transforms
//
// These operations provide additional control over randomization and timing:
// - degradeSeed: randomly remove events with specific seed (reproducible)
// - undegrade: return pattern unchanged (identity transform)
// - accelerate: speed up pattern over time

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

// ========== DegradeSeed Tests ==========

#[test]
fn test_degradeseed_basic() {
    // Test: degrade with specific seed
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ degradeSeed 42
"#,
        "DegradeSeed with seed 42",
    );
}

#[test]
fn test_degradeseed_zero_seed() {
    // Test: degrade with zero seed
    test_compilation(
        r#"
tempo: 2.0
out: "bd*8" $ degradeSeed 0
"#,
        "DegradeSeed with seed 0",
    );
}

#[test]
fn test_degradeseed_large_seed() {
    // Test: degrade with large seed value
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4" $ degradeSeed 999999
"#,
        "DegradeSeed with large seed",
    );
}

#[test]
fn test_degradeseed_different_seeds() {
    // Test: different seeds produce different results
    test_compilation(
        r#"
tempo: 2.0
~deg1: "bd*4" $ degradeSeed 1
~deg2: "sn*4" $ degradeSeed 2
~deg3: "hh*8" $ degradeSeed 42
out: ~deg1 + ~deg2 + ~deg3
"#,
        "DegradeSeed with different seeds",
    );
}

#[test]
fn test_degradeseed_reproducible() {
    // Test: same seed produces same result (reproducibility)
    test_compilation(
        r#"
tempo: 2.0
~deg1: "bd sn hh cp" $ degradeSeed 7
~deg2: "bd sn hh cp" $ degradeSeed 7
out: ~deg1 + ~deg2
"#,
        "DegradeSeed reproducibility with same seed",
    );
}

#[test]
fn test_degradeseed_with_subdivision() {
    // Test: degradeSeed with subdivision
    test_compilation(
        r#"
tempo: 2.0
out: "bd*4 sn*4 hh*8" $ degradeSeed 13
"#,
        "DegradeSeed with subdivision",
    );
}

#[test]
fn test_degradeseed_with_effects() {
    // Test: degradeSeed through reverb
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*2" $ degradeSeed 5 # reverb 0.5 0.3 0.2
"#,
        "DegradeSeed with reverb",
    );
}

#[test]
fn test_degradeseed_combined() {
    // Test: degradeSeed combined with other transforms
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh" $ degradeSeed 42 $ fast 2
"#,
        "DegradeSeed combined with fast",
    );
}

// ========== Undegrade Tests ==========

#[test]
fn test_undegrade_basic() {
    // Test: undegrade returns pattern unchanged
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ undegrade
"#,
        "Undegrade basic",
    );
}

#[test]
fn test_undegrade_after_degrade() {
    // Test: undegrade after degrade (though undegrade doesn't undo degrade)
    test_compilation(
        r#"
tempo: 2.0
out: "bd*8" $ degrade $ undegrade
"#,
        "Undegrade after degrade",
    );
}

#[test]
fn test_undegrade_with_subdivision() {
    // Test: undegrade with subdivision
    test_compilation(
        r#"
tempo: 2.0
out: "bd*4 sn*4 hh*8" $ undegrade
"#,
        "Undegrade with subdivision",
    );
}

#[test]
fn test_undegrade_identity() {
    // Test: undegrade is identity (doesn't modify pattern)
    test_compilation(
        r#"
tempo: 2.0
~original: "bd sn hh cp"
~undegraded: "bd sn hh cp" $ undegrade
out: ~original + ~undegraded
"#,
        "Undegrade identity property",
    );
}

#[test]
fn test_undegrade_with_effects() {
    // Test: undegrade through delay
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*2" $ undegrade # delay 0.25 0.5 0.3
"#,
        "Undegrade with delay",
    );
}

#[test]
fn test_undegrade_combined() {
    // Test: undegrade combined with other transforms
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh" $ undegrade $ rev
"#,
        "Undegrade combined with rev",
    );
}

#[test]
fn test_undegrade_multiple() {
    // Test: multiple undegrade operations (still identity)
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ undegrade $ undegrade $ undegrade
"#,
        "Multiple undegrade operations",
    );
}

// ========== Accelerate Tests ==========

#[test]
fn test_accelerate_basic() {
    // Test: accelerate with positive rate
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ accelerate 0.5
"#,
        "Accelerate with rate 0.5",
    );
}

#[test]
fn test_accelerate_slow() {
    // Test: accelerate with small rate (slow acceleration)
    test_compilation(
        r#"
tempo: 2.0
out: "bd*8" $ accelerate 0.1
"#,
        "Accelerate with slow rate (0.1)",
    );
}

#[test]
fn test_accelerate_fast() {
    // Test: accelerate with large rate (fast acceleration)
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4" $ accelerate 2.0
"#,
        "Accelerate with fast rate (2.0)",
    );
}

#[test]
fn test_accelerate_negative() {
    // Test: accelerate with negative rate (deceleration)
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ accelerate (-0.5)
"#,
        "Accelerate with negative rate (deceleration)",
    );
}

#[test]
fn test_accelerate_zero() {
    // Test: accelerate with zero rate (no change)
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4" $ accelerate 0.0
"#,
        "Accelerate with zero rate",
    );
}

#[test]
fn test_accelerate_with_subdivision() {
    // Test: accelerate with subdivision
    test_compilation(
        r#"
tempo: 2.0
out: "bd*4 sn*4 hh*8" $ accelerate 0.3
"#,
        "Accelerate with subdivision",
    );
}

#[test]
fn test_accelerate_with_effects() {
    // Test: accelerate through chorus
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*2" $ accelerate 0.7 # chorus 0.5 0.3 0.2
"#,
        "Accelerate with chorus",
    );
}

#[test]
fn test_accelerate_combined() {
    // Test: accelerate combined with other transforms
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh" $ accelerate 0.5 $ fast 2
"#,
        "Accelerate combined with fast",
    );
}

// ========== Combined Tests ==========

#[test]
fn test_all_three_operations() {
    // Test: using all three operations in same program
    test_compilation(
        r#"
tempo: 2.0
~degraded: "bd*8" $ degradeSeed 42
~undegraded: "sn*4" $ undegrade
~accelerated: "hh*8" $ accelerate 0.5
out: ~degraded + ~undegraded + ~accelerated
"#,
        "All three operations in one program",
    );
}

#[test]
fn test_degradeseed_and_undegrade() {
    // Test: degradeSeed and undegrade together
    test_compilation(
        r#"
tempo: 2.0
~deg: "bd sn" $ degradeSeed 7
~undeg: "hh cp" $ undegrade
out: ~deg + ~undeg
"#,
        "DegradeSeed and undegrade together",
    );
}

#[test]
fn test_degradeseed_and_accelerate() {
    // Test: degradeSeed and accelerate together
    test_compilation(
        r#"
tempo: 2.0
~deg: "bd*4 sn*4" $ degradeSeed 13
~acc: "hh*4 cp*4" $ accelerate 0.3
out: ~deg + ~acc
"#,
        "DegradeSeed and accelerate together",
    );
}

#[test]
fn test_undegrade_and_accelerate() {
    // Test: undegrade and accelerate together
    test_compilation(
        r#"
tempo: 2.0
~undeg: "bd sn hh" $ undegrade
~acc: "cp bd sn" $ accelerate 0.5
out: ~undeg + ~acc
"#,
        "Undegrade and accelerate together",
    );
}

#[test]
fn test_complex_combination() {
    // Test: complex combination of operations
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ degradeSeed 42 $ undegrade $ accelerate 0.7 $ fast 2
"#,
        "Complex combination: degradeSeed, undegrade, accelerate, fast",
    );
}

#[test]
fn test_with_effects_chain() {
    // Test: multiple operations with effects chain
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4" $ degradeSeed 7 $ accelerate 0.5 # lpf 1000 0.8 # reverb 0.5 0.3 0.2
"#,
        "Multiple operations with effects chain",
    );
}

#[test]
fn test_degradeseed_with_other_transforms() {
    // Test: degradeSeed with various transforms
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ degradeSeed 9 $ rev $ slow 2
"#,
        "DegradeSeed with rev and slow",
    );
}

#[test]
fn test_accelerate_with_palindrome() {
    // Test: accelerate with palindrome
    test_compilation(
        r#"
tempo: 2.0
out: "bd*8" $ accelerate 0.5 $ palindrome
"#,
        "Accelerate with palindrome",
    );
}

#[test]
fn test_undegrade_in_every() {
    // Test: undegrade inside every transform
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ every 2 undegrade
"#,
        "Undegrade inside every",
    );
}

#[test]
fn test_in_complex_multi_bus_program() {
    // Test: all operations in complex multi-bus program
    test_compilation(
        r#"
tempo: 2.0
~kick: "bd*4" $ degradeSeed 7 $ accelerate 0.2
~snare: "~ sn ~ sn" $ undegrade
~hats: "hh*8" $ accelerate 0.5
~perc: "cp*4" $ degradeSeed 13 $ undegrade
~mixed: (~kick + ~snare) $ accelerate 0.3
out: ~mixed * 0.5 + ~hats * 0.3 + ~perc * 0.2
"#,
        "Complex multi-bus program with all operations",
    );
}

#[test]
fn test_nested_operations() {
    // Test: nested operations on same pattern
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ degradeSeed 42 $ undegrade $ accelerate 0.5 $ fast 2 $ rev
"#,
        "Nested operations on same pattern",
    );
}

#[test]
fn test_multiple_degradeseed_different_seeds() {
    // Test: multiple patterns with different seeds
    test_compilation(
        r#"
tempo: 2.0
~d1: "bd*8" $ degradeSeed 1
~d2: "sn*8" $ degradeSeed 2
~d3: "hh*8" $ degradeSeed 3
~d4: "cp*8" $ degradeSeed 4
out: ~d1 + ~d2 + ~d3 + ~d4
"#,
        "Multiple degradeSeed with different seeds",
    );
}

#[test]
fn test_accelerate_with_stutter() {
    // Test: accelerate and stutter working together
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh" $ accelerate 0.5 $ stutter 3
"#,
        "Accelerate with stutter",
    );
}

#[test]
fn test_all_operations_with_reverb() {
    // Test: all operations through reverb
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4" $ degradeSeed 7 $ undegrade $ accelerate 0.5 # reverb 0.5 0.7 0.3
"#,
        "All operations with reverb",
    );
}

#[test]
fn test_degradeseed_reproducibility_verification() {
    // Test: verify seed reproducibility across buses
    test_compilation(
        r#"
tempo: 2.0
~a: "bd sn hh cp" $ degradeSeed 42
~b: "bd sn hh cp" $ degradeSeed 42
~c: "bd sn hh cp" $ degradeSeed 99
out: ~a + ~b + ~c
"#,
        "DegradeSeed reproducibility verification",
    );
}
