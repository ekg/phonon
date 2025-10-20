// Test rotL, rotR, iter, iterBack, ply, and linger pattern transforms
//
// These operations modify pattern timing and structure:
// - rotL: rotate pattern left by n steps
// - rotR: rotate pattern right by n steps
// - iter: iterate pattern shifting by 1/n each cycle
// - iterBack: iterate pattern backwards
// - ply: repeat each event n times
// - linger: linger on values for longer

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

// ========== RotL Tests ==========

#[test]
fn test_rotl_basic() {
    // Test: rotate pattern left by 0.25 steps
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ rotL 0.25
"#,
        "RotL by 0.25 steps",
    );
}

#[test]
fn test_rotl_small_amount() {
    // Test: subtle rotation
    test_compilation(
        r#"
tempo: 2.0
out: "bd*8" $ rotL 0.125
"#,
        "RotL by 0.125 steps (small rotation)",
    );
}

#[test]
fn test_rotl_full_cycle() {
    // Test: rotate by full cycle
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ rotL 1.0
"#,
        "RotL by full cycle",
    );
}

#[test]
fn test_rotl_with_effects() {
    // Test: rotated pattern through reverb
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*2" $ rotL 0.5 # reverb 0.5 0.3 0.2
"#,
        "RotL with reverb",
    );
}

#[test]
fn test_rotl_combined() {
    // Test: rotL combined with other transforms
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh" $ rotL 0.125 $ fast 2
"#,
        "RotL combined with fast",
    );
}

// ========== RotR Tests ==========

#[test]
fn test_rotr_basic() {
    // Test: rotate pattern right by 0.25 steps
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ rotR 0.25
"#,
        "RotR by 0.25 steps",
    );
}

#[test]
fn test_rotr_small_amount() {
    // Test: subtle rotation
    test_compilation(
        r#"
tempo: 2.0
out: "bd*8" $ rotR 0.125
"#,
        "RotR by 0.125 steps (small rotation)",
    );
}

#[test]
fn test_rotr_with_effects() {
    // Test: rotated pattern through delay
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4" $ rotR 0.5 # delay 0.25 0.5 0.3
"#,
        "RotR with delay",
    );
}

#[test]
fn test_rotl_and_rotr_cancel() {
    // Test: rotL and rotR of same amount should cancel
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ rotL 0.25 $ rotR 0.25
"#,
        "RotL and RotR cancel",
    );
}

// ========== Iter Tests ==========

#[test]
fn test_iter_basic() {
    // Test: iterate pattern over 4 cycles
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ iter 4
"#,
        "Iter over 4 cycles",
    );
}

#[test]
fn test_iter_small_n() {
    // Test: iterate over 2 cycles
    test_compilation(
        r#"
tempo: 2.0
out: "bd*4" $ iter 2
"#,
        "Iter over 2 cycles",
    );
}

#[test]
fn test_iter_large_n() {
    // Test: iterate over many cycles
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4 cp*2" $ iter 8
"#,
        "Iter over 8 cycles",
    );
}

#[test]
fn test_iter_with_euclidean() {
    // Test: iter with euclidean pattern
    test_compilation(
        r#"
tempo: 2.0
out: "bd(3,8)" $ iter 3
"#,
        "Iter with euclidean pattern",
    );
}

#[test]
fn test_iter_with_effects() {
    // Test: iterated pattern through chorus
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ iter 4 # chorus 0.5 0.3 0.2
"#,
        "Iter with chorus",
    );
}

// ========== IterBack Tests ==========

#[test]
fn test_iterback_basic() {
    // Test: iterate pattern backwards over 4 cycles
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ iterBack 4
"#,
        "IterBack over 4 cycles",
    );
}

#[test]
fn test_iterback_with_fast() {
    // Test: iterBack combined with fast
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4" $ iterBack 3 $ fast 2
"#,
        "IterBack with fast",
    );
}

#[test]
fn test_iter_and_iterback() {
    // Test: iter and iterBack in same program
    test_compilation(
        r#"
tempo: 2.0
~forward: "bd sn" $ iter 4
~backward: "hh cp" $ iterBack 4
out: ~forward + ~backward
"#,
        "Iter and IterBack together",
    );
}

// ========== Ply Tests ==========

#[test]
fn test_ply_basic() {
    // Test: repeat each event 3 times
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn" $ ply 3
"#,
        "Ply 3 times",
    );
}

#[test]
fn test_ply_many_times() {
    // Test: repeat many times
    test_compilation(
        r#"
tempo: 2.0
out: "bd" $ ply 8
"#,
        "Ply 8 times",
    );
}

#[test]
fn test_ply_with_subdivision() {
    // Test: ply with subdivision pattern
    test_compilation(
        r#"
tempo: 2.0
out: "bd*4 sn*4" $ ply 2
"#,
        "Ply with subdivision",
    );
}

#[test]
fn test_ply_with_effects() {
    // Test: plied pattern through distortion
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh" $ ply 4 # distort 2.0 0.5
"#,
        "Ply with distortion",
    );
}

#[test]
fn test_ply_combined() {
    // Test: ply combined with other transforms
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn" $ ply 3 $ fast 2
"#,
        "Ply combined with fast",
    );
}

// ========== Linger Tests ==========

#[test]
fn test_linger_basic() {
    // Test: linger on values for 2x longer
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ linger 2.0
"#,
        "Linger factor 2.0",
    );
}

#[test]
fn test_linger_small_factor() {
    // Test: linger with small factor
    test_compilation(
        r#"
tempo: 2.0
out: "bd*8" $ linger 1.5
"#,
        "Linger factor 1.5",
    );
}

#[test]
fn test_linger_large_factor() {
    // Test: linger with large factor
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ linger 4.0
"#,
        "Linger factor 4.0",
    );
}

#[test]
fn test_linger_with_effects() {
    // Test: lingered pattern through reverb
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*2" $ linger 3.0 # reverb 0.5 0.3 0.2
"#,
        "Linger with reverb",
    );
}

#[test]
fn test_linger_combined() {
    // Test: linger combined with other transforms
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh" $ linger 2.0 $ rev
"#,
        "Linger combined with rev",
    );
}

// ========== Combined Tests ==========

#[test]
fn test_all_six_operations_in_program() {
    // Test: using all six operations in same program
    test_compilation(
        r#"
tempo: 2.0
~rotated_left: "bd*8" $ rotL 0.25
~rotated_right: "sn*4" $ rotR 0.125
~iterated: "hh*8" $ iter 4
~iterated_back: "cp*2" $ iterBack 3
~plied: "bd sn" $ ply 4
~lingered: "hh cp" $ linger 2.0
out: ~rotated_left + ~rotated_right + ~iterated + ~iterated_back + ~plied + ~lingered
"#,
        "All six operations in one program",
    );
}

#[test]
fn test_rotation_operations() {
    // Test: both rotation operations together
    test_compilation(
        r#"
tempo: 2.0
~left: "bd sn hh cp" $ rotL 0.5
~right: "bd sn hh cp" $ rotR 0.5
out: ~left + ~right
"#,
        "RotL and RotR in same program",
    );
}

#[test]
fn test_iteration_operations() {
    // Test: both iteration operations together
    test_compilation(
        r#"
tempo: 2.0
~forward: "bd*4 sn*4" $ iter 4
~backward: "hh*4 cp*4" $ iterBack 4
out: ~forward + ~backward
"#,
        "Iter and IterBack in same program",
    );
}

#[test]
fn test_ply_and_linger() {
    // Test: ply and linger together
    test_compilation(
        r#"
tempo: 2.0
~plied: "bd sn" $ ply 4
~lingered: "hh cp" $ linger 2.0
out: ~plied + ~lingered
"#,
        "Ply and linger in same program",
    );
}

#[test]
fn test_complex_combination() {
    // Test: complex combination of operations
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ rotL 0.25 $ iter 4 $ ply 2 $ fast 2
"#,
        "Complex combination: rotL, iter, ply, fast",
    );
}

#[test]
fn test_with_effects_chain() {
    // Test: multiple operations with effects chain
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4" $ rotL 0.125 $ iter 3 # lpf 1000 0.8 # reverb 0.5 0.3 0.2
"#,
        "Multiple operations with effects chain",
    );
}

#[test]
fn test_linger_and_iter() {
    // Test: linger and iter combined
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ linger 2.0 $ iter 4
"#,
        "Linger and iter combined",
    );
}

#[test]
fn test_ply_with_rotations() {
    // Test: ply with rotations
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn" $ ply 3 $ rotL 0.25 $ rotR 0.125
"#,
        "Ply with rotations",
    );
}

#[test]
fn test_in_complex_multi_bus_program() {
    // Test: all operations in complex multi-bus program
    test_compilation(
        r#"
tempo: 2.0
~kick: "bd*4" $ ply 2 $ rotL 0.125
~snare: "~ sn ~ sn" $ iter 4
~hats: "hh*8" $ iterBack 3
~perc: "cp*4" $ linger 1.5
~mixed: (~kick + ~snare) $ rotR 0.25
out: ~mixed * 0.5 + ~hats * 0.3 + ~perc * 0.2
"#,
        "Complex multi-bus program with all operations",
    );
}
