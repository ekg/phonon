// Test offset, loop, chew, fastGap, discretise, and compressGap pattern transforms
//
// These operations modify pattern timing and structure:
// - offset: shift pattern in time (alias for late)
// - loop: loop pattern n times within cycle
// - chew: chew through pattern
// - fastGap: fast with gaps
// - discretise: quantize time
// - compressGap: compress to range with gaps

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

// ========== Offset Tests ==========

#[test]
fn test_offset_basic() {
    // Test: offset pattern by 0.25
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ offset 0.25
"#,
        "Offset by 0.25",
    );
}

#[test]
fn test_offset_negative() {
    // Test: negative offset (shift earlier)
    test_compilation(
        r#"
tempo: 0.5
out: "bd*4" $ offset (-0.125)
"#,
        "Offset by -0.125 (negative)",
    );
}

#[test]
fn test_offset_with_effects() {
    // Test: offset pattern through reverb
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh*2" $ offset 0.5 # reverb 0.5 0.3 0.2
"#,
        "Offset with reverb",
    );
}

#[test]
fn test_offset_combined() {
    // Test: offset combined with other transforms
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh" $ offset 0.125 $ fast 2
"#,
        "Offset combined with fast",
    );
}

// ========== Loop Tests ==========

#[test]
fn test_loop_basic() {
    // Test: loop pattern 4 times within cycle
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn" $ loop 4
"#,
        "Loop 4 times",
    );
}

#[test]
fn test_loop_small_n() {
    // Test: loop 2 times
    test_compilation(
        r#"
tempo: 0.5
out: "bd" $ loop 2
"#,
        "Loop 2 times",
    );
}

#[test]
fn test_loop_large_n() {
    // Test: loop many times
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh" $ loop 8
"#,
        "Loop 8 times",
    );
}

#[test]
fn test_loop_with_effects() {
    // Test: looped pattern through delay
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn" $ loop 3 # delay 0.25 0.5 0.3
"#,
        "Loop with delay",
    );
}

#[test]
fn test_loop_with_subdivision() {
    // Test: loop with subdivision pattern
    test_compilation(
        r#"
tempo: 0.5
out: "bd*2 sn*2" $ loop 4
"#,
        "Loop with subdivision",
    );
}

// ========== Chew Tests ==========

#[test]
fn test_chew_basic() {
    // Test: chew through pattern
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ chew 4
"#,
        "Chew 4 steps",
    );
}

#[test]
fn test_chew_small_n() {
    // Test: chew with small n
    test_compilation(
        r#"
tempo: 0.5
out: "bd*8" $ chew 2
"#,
        "Chew 2 steps",
    );
}

#[test]
fn test_chew_large_n() {
    // Test: chew with large n
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh*4 cp*2" $ chew 16
"#,
        "Chew 16 steps",
    );
}

#[test]
fn test_chew_with_effects() {
    // Test: chewed pattern through chorus
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ chew 8 # chorus 0.5 0.3 0.2
"#,
        "Chew with chorus",
    );
}

#[test]
fn test_chew_combined() {
    // Test: chew combined with other transforms
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh*4" $ chew 4 $ fast 2
"#,
        "Chew combined with fast",
    );
}

// ========== FastGap Tests ==========

#[test]
fn test_fastgap_basic() {
    // Test: fast with gaps
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ fastGap 2.0
"#,
        "FastGap factor 2.0",
    );
}

#[test]
fn test_fastgap_small_factor() {
    // Test: fastGap with small factor
    test_compilation(
        r#"
tempo: 0.5
out: "bd*4" $ fastGap 1.5
"#,
        "FastGap factor 1.5",
    );
}

#[test]
fn test_fastgap_large_factor() {
    // Test: fastGap with large factor
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn" $ fastGap 4.0
"#,
        "FastGap factor 4.0",
    );
}

#[test]
fn test_fastgap_with_effects() {
    // Test: fastGap pattern through distortion
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh*2" $ fastGap 3.0 # distort 2.0 0.5
"#,
        "FastGap with distortion",
    );
}

#[test]
fn test_fastgap_combined() {
    // Test: fastGap combined with other transforms
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh" $ fastGap 2.0 $ rev
"#,
        "FastGap combined with rev",
    );
}

// ========== Discretise Tests ==========

#[test]
fn test_discretise_basic() {
    // Test: quantize time to 4 steps
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ discretise 4
"#,
        "Discretise 4 steps",
    );
}

#[test]
fn test_discretise_small_n() {
    // Test: discretise to 2 steps
    test_compilation(
        r#"
tempo: 0.5
out: "bd*8" $ discretise 2
"#,
        "Discretise 2 steps",
    );
}

#[test]
fn test_discretise_large_n() {
    // Test: discretise to many steps
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh*4" $ discretise 16
"#,
        "Discretise 16 steps",
    );
}

#[test]
fn test_discretise_with_effects() {
    // Test: discretised pattern through bitcrush
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ discretise 8 # bitcrush 4 8000
"#,
        "Discretise with bitcrush",
    );
}

#[test]
fn test_discretise_combined() {
    // Test: discretise combined with other transforms
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh" $ discretise 8 $ fast 2
"#,
        "Discretise combined with fast",
    );
}

// ========== CompressGap Tests ==========

#[test]
fn test_compressgap_basic() {
    // Test: compress to middle half with gaps
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ compressGap 0.25 0.75
"#,
        "CompressGap 0.25-0.75",
    );
}

#[test]
fn test_compressgap_first_quarter() {
    // Test: compress to first quarter
    test_compilation(
        r#"
tempo: 0.5
out: "bd*8" $ compressGap 0.0 0.25
"#,
        "CompressGap 0.0-0.25 (first quarter)",
    );
}

#[test]
fn test_compressgap_last_quarter() {
    // Test: compress to last quarter
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh*4" $ compressGap 0.75 1.0
"#,
        "CompressGap 0.75-1.0 (last quarter)",
    );
}

#[test]
fn test_compressgap_small_range() {
    // Test: compress to small range
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ compressGap 0.4 0.6
"#,
        "CompressGap 0.4-0.6 (small range)",
    );
}

#[test]
fn test_compressgap_with_effects() {
    // Test: compressGap pattern through reverb
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh*2" $ compressGap 0.25 0.75 # reverb 0.5 0.3 0.2
"#,
        "CompressGap with reverb",
    );
}

#[test]
fn test_compressgap_combined() {
    // Test: compressGap combined with other transforms
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh" $ compressGap 0.0 0.5 $ fast 2
"#,
        "CompressGap combined with fast",
    );
}

// ========== Combined Tests ==========

#[test]
fn test_all_six_operations_in_program() {
    // Test: using all six operations in same program
    test_compilation(
        r#"
tempo: 0.5
~offset_pat: "bd*8" $ offset 0.125
~looped: "sn*2" $ loop 4
~chewed: "hh*8" $ chew 4
~fastgapped: "cp*4" $ fastGap 2.0
~discretised: "bd sn" $ discretise 8
~compressed: "hh cp" $ compressGap 0.25 0.75
out: ~offset_pat + ~looped + ~chewed + ~fastgapped + ~discretised + ~compressed
"#,
        "All six operations in one program",
    );
}

#[test]
fn test_offset_and_loop() {
    // Test: offset and loop together
    test_compilation(
        r#"
tempo: 0.5
~offset_pat: "bd sn" $ offset 0.25
~looped: "hh cp" $ loop 3
out: ~offset_pat + ~looped
"#,
        "Offset and loop in same program",
    );
}

#[test]
fn test_chew_and_fastgap() {
    // Test: chew and fastGap together
    test_compilation(
        r#"
tempo: 0.5
~chewed: "bd*4 sn*4" $ chew 8
~gapped: "hh*4 cp*4" $ fastGap 2.0
out: ~chewed + ~gapped
"#,
        "Chew and fastGap in same program",
    );
}

#[test]
fn test_discretise_and_compressgap() {
    // Test: discretise and compressGap together
    test_compilation(
        r#"
tempo: 0.5
~discretised: "bd sn" $ discretise 4
~compressed: "hh cp" $ compressGap 0.0 0.5
out: ~discretised + ~compressed
"#,
        "Discretise and compressGap in same program",
    );
}

#[test]
fn test_complex_combination() {
    // Test: complex combination of operations
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ offset 0.125 $ loop 2 $ chew 4 $ fastGap 2.0
"#,
        "Complex combination: offset, loop, chew, fastGap",
    );
}

#[test]
fn test_with_effects_chain() {
    // Test: multiple operations with effects chain
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh*4" $ offset 0.25 $ discretise 8 # lpf 1000 0.8 # reverb 0.5 0.3 0.2
"#,
        "Multiple operations with effects chain",
    );
}

#[test]
fn test_in_complex_multi_bus_program() {
    // Test: all operations in complex multi-bus program
    test_compilation(
        r#"
tempo: 0.5
~kick: "bd*4" $ loop 2 $ offset 0.125
~snare: "~ sn ~ sn" $ chew 4
~hats: "hh*8" $ fastGap 2.0
~perc: "cp*4" $ discretise 8
~compressed: (~kick + ~snare) $ compressGap 0.25 0.75
out: ~compressed * 0.5 + ~hats * 0.3 + ~perc * 0.2
"#,
        "Complex multi-bus program with all operations",
    );
}
