// Test shuffle, chop/striate, and scramble pattern transforms
//
// These operations modify event timing and ordering:
// - shuffle: randomly shifts events in time by a given amount
// - chop/striate: slices pattern into n equal parts and plays them in order
// - scramble: randomly reorders events using Fisher-Yates shuffle

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

// ========== Shuffle Tests ==========

#[test]
fn test_shuffle_basic() {
    // Test: shuffle with small amount - randomly shifts events in time
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh cp" $ shuffle 0.1
"#,
        "Shuffle with 0.1 amount",
    );
}

#[test]
fn test_shuffle_large_amount() {
    // Test: shuffle with large amount - more dramatic time shifts
    test_compilation(
        r#"
tempo: 0.5
out $ "bd*8" $ shuffle 0.5
"#,
        "Shuffle with 0.5 amount",
    );
}

#[test]
fn test_shuffle_with_chain() {
    // Test: shuffle routed through effects
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh" $ shuffle 0.2 # lpf 1000 0.8
"#,
        "Shuffle with chained filter",
    );
}

#[test]
fn test_shuffle_combined_with_other_transforms() {
    // Test: shuffle combined with fast/slow
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn" $ fast 2 $ shuffle 0.15
"#,
        "Shuffle combined with fast",
    );
}

// ========== Chop Tests ==========

#[test]
fn test_chop_basic() {
    // Test: chop pattern into 4 parts
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh cp" $ chop 4
"#,
        "Chop into 4 parts",
    );
}

#[test]
fn test_chop_power_of_two() {
    // Test: chop into 8 parts (common for sample slicing)
    test_compilation(
        r#"
tempo: 0.5
out $ "bd*4" $ chop 8
"#,
        "Chop into 8 parts",
    );
}

#[test]
fn test_chop_with_euclidean() {
    // Test: chop complex pattern with euclidean rhythm
    test_compilation(
        r#"
tempo: 0.5
out $ "bd(3,8)" $ chop 16
"#,
        "Chop euclidean pattern",
    );
}

#[test]
fn test_chop_with_effects() {
    // Test: chopped pattern through reverb
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh*2" $ chop 4 # reverb 0.5 0.3 0.2
"#,
        "Chop with reverb",
    );
}

// ========== Striate Tests (alias for chop) ==========

#[test]
fn test_striate_basic() {
    // Test: striate is an alias for chop
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh cp" $ striate 4
"#,
        "Striate into 4 parts",
    );
}

#[test]
fn test_striate_large_n() {
    // Test: striate with many slices (common in granular synthesis)
    test_compilation(
        r#"
tempo: 0.5
out $ "bd" $ striate 32
"#,
        "Striate into 32 parts (granular)",
    );
}

#[test]
fn test_striate_combined() {
    // Test: striate combined with other transforms
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn" $ striate 8 $ fast 2
"#,
        "Striate combined with fast",
    );
}

// ========== Scramble Tests ==========

#[test]
fn test_scramble_basic() {
    // Test: scramble events (Fisher-Yates shuffle)
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh cp" $ scramble 4
"#,
        "Scramble 4 events",
    );
}

#[test]
fn test_scramble_complex_pattern() {
    // Test: scramble with complex mini-notation
    test_compilation(
        r#"
tempo: 0.5
out $ "bd*4 sn*2 hh*8" $ scramble 8
"#,
        "Scramble complex pattern",
    );
}

#[test]
fn test_scramble_with_alternation() {
    // Test: scramble pattern with alternation
    test_compilation(
        r#"
tempo: 0.5
out $ "<bd sn hh cp>" $ scramble 4
"#,
        "Scramble with alternation",
    );
}

#[test]
fn test_scramble_with_effects() {
    // Test: scrambled pattern through bitcrush
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh*4" $ scramble 8 # bitcrush 8 8000
"#,
        "Scramble with bitcrush",
    );
}

// ========== Combined Tests ==========

#[test]
fn test_all_three_operations_in_program() {
    // Test: using shuffle, chop, and scramble in same program
    test_compilation(
        r#"
tempo: 0.5
~shuffled $ "bd*4" $ shuffle 0.2
~chopped $ "sn*4" $ chop 8
~scrambled $ "hh*8" $ scramble 8
out $ ~shuffled + ~chopped + ~scrambled
"#,
        "All three operations in one program",
    );
}

#[test]
fn test_chained_reordering_transforms() {
    // Test: combining multiple reordering operations
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh cp" $ chop 4 $ shuffle 0.1 $ scramble 4
"#,
        "Chained reordering transforms",
    );
}

#[test]
fn test_with_pattern_params() {
    // Test: reordering with pattern-controlled DSP parameters
    test_compilation(
        r#"
tempo: 0.5
out $ s "bd sn hh*4" $ shuffle 0.15 # lpf 1000 0.8
"#,
        "Shuffle with sample playback and filter",
    );
}
