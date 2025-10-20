// Test function-taking transforms: superimpose, chunk, sometimes, often, rarely
//
// These transforms take other transforms as arguments, enabling powerful
// compositional patterns. They use the `every`-style syntax:
//   pattern $ transform argument
//
// Examples:
//   "bd sn" $ superimpose rev
//   "bd sn hh cp" $ chunk 2 (fast 2)
//   "bd*4" $ sometimes (fast 2)

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

// ========== Superimpose Tests ==========

#[test]
fn test_superimpose_basic() {
    // Test: superimpose with fast transform
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn" $ superimpose (fast 2)
"#,
        "Superimpose with fast 2",
    );
}

#[test]
fn test_superimpose_rev() {
    // Test: superimpose with rev
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ superimpose rev
"#,
        "Superimpose with rev",
    );
}

#[test]
fn test_superimpose_slow() {
    // Test: superimpose with slow transform
    test_compilation(
        r#"
tempo: 2.0
out: "bd*8" $ superimpose (slow 2)
"#,
        "Superimpose with slow 2",
    );
}

#[test]
fn test_superimpose_double() {
    // Test: superimpose applied twice
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn" $ superimpose (fast 2) $ superimpose rev
"#,
        "Superimpose with fast and then rev",
    );
}

#[test]
fn test_superimpose_with_effects() {
    // Test: superimpose through reverb
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*2" $ superimpose (fast 3) # reverb 0.5 0.3 0.2
"#,
        "Superimpose with effects chain",
    );
}

#[test]
fn test_superimpose_combined() {
    // Test: superimpose combined with other transforms
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh" $ superimpose (fast 2) $ slow 2
"#,
        "Superimpose combined with slow",
    );
}

// ========== Chunk Tests ==========

#[test]
fn test_chunk_basic() {
    // Test: chunk with 2 pieces and fast
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ chunk 2 (fast 2)
"#,
        "Chunk 2 with fast 2",
    );
}

#[test]
fn test_chunk_four() {
    // Test: chunk into 4 pieces
    test_compilation(
        r#"
tempo: 2.0
out: "bd*8" $ chunk 4 rev
"#,
        "Chunk 4 with rev",
    );
}

#[test]
fn test_chunk_slow() {
    // Test: chunk with slow transform
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4" $ chunk 2 (slow 2)
"#,
        "Chunk 2 with slow 2",
    );
}

#[test]
fn test_chunk_double() {
    // Test: chunk applied with fast, then rev
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ chunk 3 (fast 2) $ rev
"#,
        "Chunk with fast, then rev applied",
    );
}

#[test]
fn test_chunk_with_effects() {
    // Test: chunk through delay
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*2" $ chunk 2 (fast 3) # delay 0.25 0.5 0.3
"#,
        "Chunk with delay effect",
    );
}

#[test]
fn test_chunk_combined() {
    // Test: chunk combined with other transforms
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh" $ chunk 2 (fast 2) $ palindrome
"#,
        "Chunk combined with palindrome",
    );
}

// ========== Sometimes Tests ==========

#[test]
fn test_sometimes_basic() {
    // Test: sometimes with fast transform
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ sometimes (fast 2)
"#,
        "Sometimes with fast 2",
    );
}

#[test]
fn test_sometimes_rev() {
    // Test: sometimes with rev
    test_compilation(
        r#"
tempo: 2.0
out: "bd*8" $ sometimes rev
"#,
        "Sometimes with rev",
    );
}

#[test]
fn test_sometimes_slow() {
    // Test: sometimes with slow transform
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4" $ sometimes (slow 2)
"#,
        "Sometimes with slow 2",
    );
}

#[test]
fn test_sometimes_double() {
    // Test: sometimes with fast, then rev
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn" $ sometimes (fast 2) $ rev
"#,
        "Sometimes with fast, then rev",
    );
}

#[test]
fn test_sometimes_with_effects() {
    // Test: sometimes through chorus
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*2" $ sometimes (fast 3) # chorus 0.5 0.3 0.2
"#,
        "Sometimes with chorus effect",
    );
}

#[test]
fn test_sometimes_combined() {
    // Test: sometimes combined with other transforms
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh" $ sometimes (fast 2) $ slow 2
"#,
        "Sometimes combined with slow",
    );
}

// ========== Often Tests ==========

#[test]
fn test_often_basic() {
    // Test: often with fast transform
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ often (fast 2)
"#,
        "Often with fast 2",
    );
}

#[test]
fn test_often_rev() {
    // Test: often with rev
    test_compilation(
        r#"
tempo: 2.0
out: "bd*8" $ often rev
"#,
        "Often with rev",
    );
}

#[test]
fn test_often_slow() {
    // Test: often with slow transform
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4" $ often (slow 2)
"#,
        "Often with slow 2",
    );
}

#[test]
fn test_often_double() {
    // Test: often with fast, then rev
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn" $ often (fast 2) $ rev
"#,
        "Often with fast, then rev",
    );
}

#[test]
fn test_often_with_effects() {
    // Test: often through reverb
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*2" $ often (fast 3) # reverb 0.5 0.3 0.2
"#,
        "Often with reverb effect",
    );
}

#[test]
fn test_often_combined() {
    // Test: often combined with other transforms
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh" $ often (fast 2) $ palindrome
"#,
        "Often combined with palindrome",
    );
}

// ========== Rarely Tests ==========

#[test]
fn test_rarely_basic() {
    // Test: rarely with fast transform
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ rarely (fast 2)
"#,
        "Rarely with fast 2",
    );
}

#[test]
fn test_rarely_rev() {
    // Test: rarely with rev
    test_compilation(
        r#"
tempo: 2.0
out: "bd*8" $ rarely rev
"#,
        "Rarely with rev",
    );
}

#[test]
fn test_rarely_slow() {
    // Test: rarely with slow transform
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4" $ rarely (slow 2)
"#,
        "Rarely with slow 2",
    );
}

#[test]
fn test_rarely_double() {
    // Test: rarely with fast, then rev
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn" $ rarely (fast 2) $ rev
"#,
        "Rarely with fast, then rev",
    );
}

#[test]
fn test_rarely_with_effects() {
    // Test: rarely through delay
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*2" $ rarely (fast 3) # delay 0.25 0.5 0.3
"#,
        "Rarely with delay effect",
    );
}

#[test]
fn test_rarely_combined() {
    // Test: rarely combined with other transforms
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh" $ rarely (fast 2) $ slow 2
"#,
        "Rarely combined with slow",
    );
}

// ========== Combined Tests ==========

#[test]
fn test_all_five_operations() {
    // Test: using all five operations in same program
    test_compilation(
        r#"
tempo: 2.0
~sup: "bd*4" $ superimpose (fast 2)
~chk: "sn*4" $ chunk 2 rev
~some: "hh*8" $ sometimes (fast 3)
~oft: "cp*4" $ often (slow 2)
~rare: "bd sn" $ rarely rev
out: ~sup + ~chk + ~some + ~oft + ~rare
"#,
        "All five operations in one program",
    );
}

#[test]
fn test_superimpose_and_chunk() {
    // Test: superimpose and chunk together
    test_compilation(
        r#"
tempo: 2.0
~sup: "bd sn" $ superimpose (fast 2)
~chk: "hh cp" $ chunk 2 rev
out: ~sup + ~chk
"#,
        "Superimpose and chunk together",
    );
}

#[test]
fn test_probabilistic_transforms() {
    // Test: sometimes, often, rarely together
    test_compilation(
        r#"
tempo: 2.0
~some: "bd*4" $ sometimes (fast 2)
~oft: "sn*4" $ often (fast 2)
~rare: "hh*8" $ rarely (fast 2)
out: ~some + ~oft + ~rare
"#,
        "Probabilistic transforms together",
    );
}

#[test]
fn test_nested_function_transforms() {
    // Test: nesting function-taking transforms
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ superimpose (sometimes (fast 2))
"#,
        "Nested function transforms",
    );
}

#[test]
fn test_with_every() {
    // Test: combining with every transform
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ every 2 (superimpose (fast 2))
"#,
        "Function transform inside every",
    );
}

#[test]
fn test_complex_combination() {
    // Test: complex combination of function transforms
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ superimpose (fast 2) $ sometimes rev $ often (slow 2)
"#,
        "Complex combination of function transforms",
    );
}

#[test]
fn test_with_effects_chain() {
    // Test: multiple function transforms with effects
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4" $ superimpose (fast 2) $ sometimes rev # lpf 1000 0.8 # reverb 0.5 0.3 0.2
"#,
        "Function transforms with effects chain",
    );
}

#[test]
fn test_chunk_with_stutter() {
    // Test: chunk with stutter transform
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ chunk 2 (stutter 3)
"#,
        "Chunk with stutter",
    );
}

#[test]
fn test_in_complex_multi_bus_program() {
    // Test: all function transforms in complex multi-bus program
    test_compilation(
        r#"
tempo: 2.0
~kick: "bd*4" $ superimpose (fast 2) $ sometimes (fast 3)
~snare: "~ sn ~ sn" $ chunk 2 rev $ often (slow 2)
~hats: "hh*8" $ rarely rev $ superimpose (fast 4)
~perc: "cp*4" $ chunk 3 (fast 2) $ sometimes palindrome
~mixed: (~kick + ~snare) $ sometimes (fast 2)
out: ~mixed * 0.5 + ~hats * 0.3 + ~perc * 0.2
"#,
        "Complex multi-bus program with function transforms",
    );
}

#[test]
fn test_superimpose_multiple_times() {
    // Test: superimpose applied multiple times
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn" $ superimpose (fast 2) $ superimpose (slow 2)
"#,
        "Multiple superimpose applications",
    );
}

#[test]
fn test_chunk_different_sizes() {
    // Test: different chunk sizes
    test_compilation(
        r#"
tempo: 2.0
~c2: "bd*8" $ chunk 2 (fast 2)
~c3: "sn*8" $ chunk 3 rev
~c4: "hh*8" $ chunk 4 (slow 2)
out: ~c2 + ~c3 + ~c4
"#,
        "Different chunk sizes",
    );
}

#[test]
fn test_probabilistic_with_degrade() {
    // Test: probabilistic transforms with degrade
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ sometimes (fast 2) $ degrade
"#,
        "Probabilistic with degrade",
    );
}

#[test]
fn test_all_with_reverb() {
    // Test: all function transforms through reverb
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4" $ superimpose (fast 2) $ chunk 2 rev $ sometimes (fast 3) # reverb 0.5 0.7 0.3
"#,
        "All function transforms with reverb",
    );
}

#[test]
fn test_superimpose_with_stutter() {
    // Test: superimpose with stutter
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh" $ superimpose (stutter 3)
"#,
        "Superimpose with stutter",
    );
}

#[test]
fn test_chunk_with_every() {
    // Test: chunk inside every
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ every 2 (chunk 2 (fast 2))
"#,
        "Chunk inside every",
    );
}
