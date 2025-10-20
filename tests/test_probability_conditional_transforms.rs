// Test probability and conditional transforms: sometimesBy, almostAlways, almostNever, always, whenmod
//
// These transforms provide fine-grained control over when transforms are applied:
// - sometimesBy: apply with specific probability (0.0-1.0)
// - almostAlways: apply with 90% probability
// - almostNever: apply with 10% probability
// - always: always apply (100% probability)
// - whenmod: apply when (cycle - offset) % modulo == 0

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

// ========== SometimesBy Tests ==========

#[test]
fn test_sometimesby_half() {
    // Test: sometimesBy with 50% probability
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ sometimesBy 0.5 (fast 2)
"#,
        "SometimesBy 0.5 with fast 2",
    );
}

#[test]
fn test_sometimesby_low() {
    // Test: sometimesBy with low probability
    test_compilation(
        r#"
tempo: 2.0
out: "bd*8" $ sometimesBy 0.2 rev
"#,
        "SometimesBy 0.2 with rev",
    );
}

#[test]
fn test_sometimesby_high() {
    // Test: sometimesBy with high probability
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4" $ sometimesBy 0.8 (slow 2)
"#,
        "SometimesBy 0.8 with slow 2",
    );
}

#[test]
fn test_sometimesby_zero() {
    // Test: sometimesBy with 0% (never)
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ sometimesBy 0.0 (fast 2)
"#,
        "SometimesBy 0.0 (never)",
    );
}

#[test]
fn test_sometimesby_one() {
    // Test: sometimesBy with 100% (always)
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ sometimesBy 1.0 (fast 2)
"#,
        "SometimesBy 1.0 (always)",
    );
}

#[test]
fn test_sometimesby_with_effects() {
    // Test: sometimesBy through reverb
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*2" $ sometimesBy 0.7 (fast 3) # reverb 0.5 0.3 0.2
"#,
        "SometimesBy with reverb",
    );
}

#[test]
fn test_sometimesby_combined() {
    // Test: sometimesBy combined with other transforms
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh" $ sometimesBy 0.6 (fast 2) $ slow 2
"#,
        "SometimesBy combined with slow",
    );
}

// ========== AlmostAlways Tests ==========

#[test]
fn test_almostalways_basic() {
    // Test: almostAlways with fast
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ almostAlways (fast 2)
"#,
        "AlmostAlways with fast 2",
    );
}

#[test]
fn test_almostalways_rev() {
    // Test: almostAlways with rev
    test_compilation(
        r#"
tempo: 2.0
out: "bd*8" $ almostAlways rev
"#,
        "AlmostAlways with rev",
    );
}

#[test]
fn test_almostalways_slow() {
    // Test: almostAlways with slow
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4" $ almostAlways (slow 2)
"#,
        "AlmostAlways with slow 2",
    );
}

#[test]
fn test_almostalways_with_effects() {
    // Test: almostAlways through chorus
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*2" $ almostAlways (fast 3) # chorus 0.5 0.3 0.2
"#,
        "AlmostAlways with chorus",
    );
}

#[test]
fn test_almostalways_combined() {
    // Test: almostAlways combined with other transforms
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh" $ almostAlways (fast 2) $ palindrome
"#,
        "AlmostAlways combined with palindrome",
    );
}

// ========== AlmostNever Tests ==========

#[test]
fn test_almostnever_basic() {
    // Test: almostNever with fast
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ almostNever (fast 2)
"#,
        "AlmostNever with fast 2",
    );
}

#[test]
fn test_almostnever_rev() {
    // Test: almostNever with rev
    test_compilation(
        r#"
tempo: 2.0
out: "bd*8" $ almostNever rev
"#,
        "AlmostNever with rev",
    );
}

#[test]
fn test_almostnever_slow() {
    // Test: almostNever with slow
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4" $ almostNever (slow 2)
"#,
        "AlmostNever with slow 2",
    );
}

#[test]
fn test_almostnever_with_effects() {
    // Test: almostNever through delay
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*2" $ almostNever (fast 3) # delay 0.25 0.5 0.3
"#,
        "AlmostNever with delay",
    );
}

#[test]
fn test_almostnever_combined() {
    // Test: almostNever combined with other transforms
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh" $ almostNever (fast 2) $ slow 2
"#,
        "AlmostNever combined with slow",
    );
}

// ========== Always Tests ==========

#[test]
fn test_always_basic() {
    // Test: always with fast
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ always (fast 2)
"#,
        "Always with fast 2",
    );
}

#[test]
fn test_always_rev() {
    // Test: always with rev
    test_compilation(
        r#"
tempo: 2.0
out: "bd*8" $ always rev
"#,
        "Always with rev",
    );
}

#[test]
fn test_always_slow() {
    // Test: always with slow
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4" $ always (slow 2)
"#,
        "Always with slow 2",
    );
}

#[test]
fn test_always_identity() {
    // Test: always should behave same as direct application
    test_compilation(
        r#"
tempo: 2.0
~direct: "bd sn" $ fast 2
~through_always: "bd sn" $ always (fast 2)
out: ~direct + ~through_always
"#,
        "Always behaves like direct application",
    );
}

#[test]
fn test_always_with_effects() {
    // Test: always through reverb
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*2" $ always (fast 3) # reverb 0.5 0.3 0.2
"#,
        "Always with reverb",
    );
}

#[test]
fn test_always_combined() {
    // Test: always combined with other transforms
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh" $ always (fast 2) $ palindrome
"#,
        "Always combined with palindrome",
    );
}

// ========== Whenmod Tests ==========

#[test]
fn test_whenmod_basic() {
    // Test: whenmod every 4 cycles
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ whenmod 4 0 (fast 2)
"#,
        "Whenmod 4 0 with fast 2",
    );
}

#[test]
fn test_whenmod_with_offset() {
    // Test: whenmod with offset
    test_compilation(
        r#"
tempo: 2.0
out: "bd*8" $ whenmod 3 1 rev
"#,
        "Whenmod 3 1 with rev",
    );
}

#[test]
fn test_whenmod_every_two() {
    // Test: whenmod every 2 cycles
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4" $ whenmod 2 0 (slow 2)
"#,
        "Whenmod 2 0 with slow 2",
    );
}

#[test]
fn test_whenmod_large_modulo() {
    // Test: whenmod with large modulo
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ whenmod 8 2 (fast 3)
"#,
        "Whenmod 8 2 with fast 3",
    );
}

#[test]
fn test_whenmod_with_effects() {
    // Test: whenmod through chorus
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*2" $ whenmod 4 1 (fast 2) # chorus 0.5 0.3 0.2
"#,
        "Whenmod with chorus",
    );
}

#[test]
fn test_whenmod_combined() {
    // Test: whenmod combined with other transforms
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh" $ whenmod 3 0 (fast 2) $ slow 2
"#,
        "Whenmod combined with slow",
    );
}

// ========== Combined Tests ==========

#[test]
fn test_all_five_operations() {
    // Test: using all five operations in same program
    test_compilation(
        r#"
tempo: 2.0
~sby: "bd*4" $ sometimesBy 0.6 (fast 2)
~aalw: "sn*4" $ almostAlways rev
~anev: "hh*8" $ almostNever (fast 3)
~alw: "cp*4" $ always (slow 2)
~wmod: "bd sn" $ whenmod 4 0 rev
out: ~sby + ~aalw + ~anev + ~alw + ~wmod
"#,
        "All five operations in one program",
    );
}

#[test]
fn test_probability_spectrum() {
    // Test: probability spectrum from never to always
    test_compilation(
        r#"
tempo: 2.0
~never: "bd*4" $ sometimesBy 0.0 (fast 2)
~rarely: "bd*4" $ sometimesBy 0.1 (fast 2)
~sometimes: "bd*4" $ sometimesBy 0.5 (fast 2)
~often: "bd*4" $ sometimesBy 0.75 (fast 2)
~almost: "bd*4" $ sometimesBy 0.9 (fast 2)
~always: "bd*4" $ sometimesBy 1.0 (fast 2)
out: ~never + ~rarely + ~sometimes + ~often + ~almost + ~always
"#,
        "Probability spectrum",
    );
}

#[test]
fn test_sometimesby_and_whenmod() {
    // Test: combining probability with cycle-based conditions
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ sometimesBy 0.7 (fast 2) $ whenmod 4 0 rev
"#,
        "SometimesBy and whenmod together",
    );
}

#[test]
fn test_nested_conditionals() {
    // Test: nesting conditional transforms
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ almostAlways (sometimes (fast 2))
"#,
        "Nested conditional transforms",
    );
}

#[test]
fn test_with_every() {
    // Test: combining with every
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ every 2 (almostAlways (fast 2))
"#,
        "Conditional transform inside every",
    );
}

#[test]
fn test_complex_combination() {
    // Test: complex combination of conditionals
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ sometimesBy 0.8 (fast 2) $ almostNever rev $ whenmod 3 0 (slow 2)
"#,
        "Complex combination of conditionals",
    );
}

#[test]
fn test_with_effects_chain() {
    // Test: multiple conditionals with effects
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4" $ sometimesBy 0.6 (fast 2) $ whenmod 4 0 rev # lpf 1000 0.8 # reverb 0.5 0.3 0.2
"#,
        "Conditionals with effects chain",
    );
}

#[test]
fn test_whenmod_different_modulos() {
    // Test: different whenmod patterns
    test_compilation(
        r#"
tempo: 2.0
~w2: "bd*8" $ whenmod 2 0 (fast 2)
~w3: "sn*8" $ whenmod 3 1 rev
~w4: "hh*8" $ whenmod 4 2 (slow 2)
out: ~w2 + ~w3 + ~w4
"#,
        "Different whenmod patterns",
    );
}

#[test]
fn test_in_complex_multi_bus_program() {
    // Test: all operations in complex multi-bus program
    test_compilation(
        r#"
tempo: 2.0
~kick: "bd*4" $ sometimesBy 0.8 (fast 2) $ whenmod 4 0 rev
~snare: "~ sn ~ sn" $ almostAlways (fast 3) $ almostNever (slow 2)
~hats: "hh*8" $ always (fast 2) $ sometimesBy 0.5 rev
~perc: "cp*4" $ whenmod 3 1 (fast 2) $ almostNever palindrome
~mixed: (~kick + ~snare) $ sometimesBy 0.7 (fast 2)
out: ~mixed * 0.5 + ~hats * 0.3 + ~perc * 0.2
"#,
        "Complex multi-bus program with all operations",
    );
}

#[test]
fn test_sometimesby_fine_control() {
    // Test: fine-grained probability control
    test_compilation(
        r#"
tempo: 2.0
~p10: "bd*4" $ sometimesBy 0.1 (fast 2)
~p25: "sn*4" $ sometimesBy 0.25 (fast 2)
~p50: "hh*4" $ sometimesBy 0.5 (fast 2)
~p75: "cp*4" $ sometimesBy 0.75 (fast 2)
~p90: "bd sn" $ sometimesBy 0.9 (fast 2)
out: ~p10 + ~p25 + ~p50 + ~p75 + ~p90
"#,
        "Fine-grained probability control",
    );
}

#[test]
fn test_whenmod_polyrhythm() {
    // Test: polyrhythmic patterns with whenmod
    test_compilation(
        r#"
tempo: 2.0
~three: "bd sn hh" $ whenmod 3 0 (fast 2)
~four: "bd sn hh cp" $ whenmod 4 0 rev
~five: "bd*5" $ whenmod 5 1 (slow 2)
out: ~three + ~four + ~five
"#,
        "Polyrhythmic patterns with whenmod",
    );
}

#[test]
fn test_all_with_reverb() {
    // Test: all operations through reverb
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4" $ sometimesBy 0.7 (fast 2) $ almostAlways rev $ whenmod 4 0 (fast 3) # reverb 0.5 0.7 0.3
"#,
        "All operations with reverb",
    );
}

#[test]
fn test_sometimesby_with_stutter() {
    // Test: sometimesBy with stutter
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh" $ sometimesBy 0.6 (stutter 3)
"#,
        "SometimesBy with stutter",
    );
}

#[test]
fn test_whenmod_in_every() {
    // Test: whenmod inside every
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ every 2 (whenmod 3 0 (fast 2))
"#,
        "Whenmod inside every",
    );
}
