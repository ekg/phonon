// Test humanize, within, and euclid pattern transforms
//
// These operations provide expressive timing and rhythmic control:
// - humanize: add human timing variation (shuffle-based)
// - within: apply transform within a specific time window
// - euclid: generate euclidean rhythm patterns

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

// ========== Humanize Tests ==========

#[test]
fn test_humanize_basic() {
    // Test: humanize with small variation
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ humanize 0.1 0.2
"#,
        "Humanize with 0.1 time, 0.2 velocity",
    );
}

#[test]
fn test_humanize_subtle() {
    // Test: subtle humanization
    test_compilation(
        r#"
tempo: 0.5
out: "bd*8" $ humanize 0.05 0.1
"#,
        "Subtle humanization",
    );
}

#[test]
fn test_humanize_strong() {
    // Test: strong humanization
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh*4" $ humanize 0.3 0.5
"#,
        "Strong humanization",
    );
}

#[test]
fn test_humanize_time_only() {
    // Test: humanize timing only (no velocity variation)
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ humanize 0.2 0.0
"#,
        "Humanize timing only",
    );
}

#[test]
fn test_humanize_velocity_only() {
    // Test: humanize velocity only (no timing variation)
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ humanize 0.0 0.3
"#,
        "Humanize velocity only",
    );
}

#[test]
fn test_humanize_with_effects() {
    // Test: humanize through reverb
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh*2" $ humanize 0.15 0.2 # reverb 0.5 0.3 0.2
"#,
        "Humanize with reverb",
    );
}

#[test]
fn test_humanize_combined() {
    // Test: humanize combined with other transforms
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh" $ humanize 0.1 0.2 $ fast 2
"#,
        "Humanize combined with fast",
    );
}

// ========== Within Tests ==========

#[test]
fn test_within_basic() {
    // Test: within first half of cycle
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ within 0.0 0.5 (fast 2)
"#,
        "Within 0.0-0.5 with fast 2",
    );
}

#[test]
fn test_within_second_half() {
    // Test: within second half
    test_compilation(
        r#"
tempo: 0.5
out: "bd*8" $ within 0.5 1.0 rev
"#,
        "Within 0.5-1.0 with rev",
    );
}

#[test]
fn test_within_middle() {
    // Test: within middle third
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh*4" $ within 0.33 0.67 (slow 2)
"#,
        "Within 0.33-0.67 with slow 2",
    );
}

#[test]
fn test_within_small_window() {
    // Test: within small window
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ within 0.25 0.35 (fast 3)
"#,
        "Within small window (0.25-0.35)",
    );
}

#[test]
fn test_within_with_effects() {
    // Test: within through delay
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh*2" $ within 0.0 0.5 (fast 2) # delay 0.25 0.5 0.3
"#,
        "Within with delay",
    );
}

#[test]
fn test_within_combined() {
    // Test: within combined with other transforms
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh" $ within 0.25 0.75 (fast 2) $ slow 2
"#,
        "Within combined with slow",
    );
}

#[test]
fn test_within_multiple() {
    // Test: multiple within operations
    test_compilation(
        r#"
tempo: 0.5
~w1: "bd*8" $ within 0.0 0.25 (fast 2)
~w2: "sn*8" $ within 0.25 0.5 rev
~w3: "hh*8" $ within 0.5 0.75 (slow 2)
~w4: "cp*8" $ within 0.75 1.0 (fast 3)
out: ~w1 + ~w2 + ~w3 + ~w4
"#,
        "Multiple within operations",
    );
}

// ========== Euclid Tests ==========

#[test]
fn test_euclid_basic() {
    // Test: euclidean 3 pulses in 8 steps
    test_compilation(
        r#"
tempo: 0.5
out: "bd" $ euclid 3 8
"#,
        "Euclid 3 8 (basic euclidean rhythm)",
    );
}

#[test]
fn test_euclid_four_four() {
    // Test: 4 pulses in 4 steps (every beat)
    test_compilation(
        r#"
tempo: 0.5
out: "bd" $ euclid 4 4
"#,
        "Euclid 4 4 (four on the floor)",
    );
}

#[test]
fn test_euclid_sparse() {
    // Test: sparse euclidean pattern
    test_compilation(
        r#"
tempo: 0.5
out: "bd" $ euclid 3 16
"#,
        "Euclid 3 16 (sparse pattern)",
    );
}

#[test]
fn test_euclid_dense() {
    // Test: dense euclidean pattern
    test_compilation(
        r#"
tempo: 0.5
out: "hh" $ euclid 7 8
"#,
        "Euclid 7 8 (dense pattern)",
    );
}

#[test]
fn test_euclid_clave() {
    // Test: son clave pattern (3-2)
    test_compilation(
        r#"
tempo: 0.5
out: "cp" $ euclid 5 8
"#,
        "Euclid 5 8 (clave pattern)",
    );
}

#[test]
fn test_euclid_with_sample() {
    // Test: euclidean pattern with specific sample
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh" $ euclid 5 13
"#,
        "Euclid with multiple samples",
    );
}

#[test]
fn test_euclid_with_effects() {
    // Test: euclidean through chorus
    test_compilation(
        r#"
tempo: 0.5
out: "bd" $ euclid 3 8 # chorus 0.5 0.3 0.2
"#,
        "Euclid with chorus",
    );
}

#[test]
fn test_euclid_combined() {
    // Test: euclid combined with other transforms
    test_compilation(
        r#"
tempo: 0.5
out: "bd" $ euclid 5 16 $ fast 2
"#,
        "Euclid combined with fast",
    );
}

// ========== Combined Tests ==========

#[test]
fn test_all_three_operations() {
    // Test: using all three operations in same program
    test_compilation(
        r#"
tempo: 0.5
~human: "bd*8" $ humanize 0.15 0.2
~within_fast: "sn*4" $ within 0.0 0.5 (fast 2)
~eucl: "hh" $ euclid 5 8
out: ~human + ~within_fast + ~eucl
"#,
        "All three operations in one program",
    );
}

#[test]
fn test_humanize_and_within() {
    // Test: humanize and within together
    test_compilation(
        r#"
tempo: 0.5
~h: "bd sn" $ humanize 0.1 0.15
~w: "hh cp" $ within 0.25 0.75 (fast 2)
out: ~h + ~w
"#,
        "Humanize and within together",
    );
}

#[test]
fn test_humanize_and_euclid() {
    // Test: humanize euclidean patterns
    test_compilation(
        r#"
tempo: 0.5
out: "bd" $ euclid 5 8 $ humanize 0.1 0.2
"#,
        "Humanize euclidean pattern",
    );
}

#[test]
fn test_within_and_euclid() {
    // Test: within with euclidean pattern
    test_compilation(
        r#"
tempo: 0.5
out: "bd" $ euclid 3 8 $ within 0.0 0.5 (fast 2)
"#,
        "Within with euclidean pattern",
    );
}

#[test]
fn test_nested_within() {
    // Test: nesting within transforms
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ within 0.0 0.5 (within 0.0 0.5 (fast 2))
"#,
        "Nested within transforms",
    );
}

#[test]
fn test_complex_combination() {
    // Test: complex combination of all three
    test_compilation(
        r#"
tempo: 0.5
out: "bd" $ euclid 5 8 $ humanize 0.1 0.15 $ within 0.25 0.75 (fast 2) $ slow 2
"#,
        "Complex combination: euclid, humanize, within, slow",
    );
}

#[test]
fn test_with_effects_chain() {
    // Test: multiple operations with effects
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh*4" $ humanize 0.1 0.2 $ within 0.0 0.5 (fast 2) # lpf 1000 0.8 # reverb 0.5 0.3 0.2
"#,
        "Multiple operations with effects chain",
    );
}

#[test]
fn test_euclidean_polyrhythm() {
    // Test: polyrhythmic euclidean patterns
    test_compilation(
        r#"
tempo: 0.5
~kick: "bd" $ euclid 4 16
~snare: "sn" $ euclid 3 16
~hats: "hh" $ euclid 7 16
out: ~kick + ~snare + ~hats
"#,
        "Euclidean polyrhythm",
    );
}

#[test]
fn test_in_complex_multi_bus_program() {
    // Test: all operations in complex multi-bus program
    test_compilation(
        r#"
tempo: 0.5
~kick: "bd" $ euclid 5 16 $ humanize 0.05 0.1
~snare: "sn" $ euclid 3 8 $ within 0.0 0.5 (fast 2)
~hats: "hh" $ euclid 11 16 $ humanize 0.1 0.15
~perc: "cp" $ euclid 5 13 $ within 0.25 0.75 (slow 2)
~mixed: (~kick + ~snare) $ humanize 0.08 0.12
out: ~mixed * 0.5 + ~hats * 0.3 + ~perc * 0.2
"#,
        "Complex multi-bus program with all operations",
    );
}

#[test]
fn test_humanize_different_amounts() {
    // Test: different humanization amounts
    test_compilation(
        r#"
tempo: 0.5
~subtle: "bd*4" $ humanize 0.05 0.05
~moderate: "sn*4" $ humanize 0.15 0.15
~strong: "hh*8" $ humanize 0.3 0.3
out: ~subtle + ~moderate + ~strong
"#,
        "Different humanization amounts",
    );
}

#[test]
fn test_within_different_windows() {
    // Test: different time windows
    test_compilation(
        r#"
tempo: 0.5
~w1: "bd*8" $ within 0.0 0.25 (fast 2)
~w2: "sn*8" $ within 0.25 0.5 rev
~w3: "hh*8" $ within 0.5 0.75 (slow 2)
~w4: "cp*8" $ within 0.75 1.0 (fast 3)
out: ~w1 + ~w2 + ~w3 + ~w4
"#,
        "Different time windows",
    );
}

#[test]
fn test_euclidean_variations() {
    // Test: euclidean rhythm variations
    test_compilation(
        r#"
tempo: 0.5
~e1: "bd" $ euclid 3 8
~e2: "sn" $ euclid 5 8
~e3: "hh" $ euclid 7 16
~e4: "cp" $ euclid 5 13
out: ~e1 + ~e2 + ~e3 + ~e4
"#,
        "Euclidean rhythm variations",
    );
}

#[test]
fn test_all_with_reverb() {
    // Test: all operations through reverb
    test_compilation(
        r#"
tempo: 0.5
out: "bd" $ euclid 5 8 $ humanize 0.1 0.15 $ within 0.0 0.5 (fast 2) # reverb 0.5 0.7 0.3
"#,
        "All operations with reverb",
    );
}

#[test]
fn test_humanize_with_stutter() {
    // Test: humanize with stutter
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh" $ stutter 3 $ humanize 0.1 0.2
"#,
        "Humanize with stutter",
    );
}

#[test]
fn test_within_in_every() {
    // Test: within inside every
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ every 2 (within 0.0 0.5 (fast 2))
"#,
        "Within inside every",
    );
}

#[test]
fn test_euclid_with_sometimes() {
    // Test: euclidean with sometimes
    test_compilation(
        r#"
tempo: 0.5
out: "bd" $ euclid 5 8 $ sometimes (fast 2)
"#,
        "Euclid with sometimes",
    );
}
