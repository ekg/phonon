// Test late, early, dup, fit, stretch pattern transforms
//
// These operations modify pattern timing and structure:
// - late: delay pattern in time
// - early: shift pattern earlier in time
// - dup: duplicate pattern n times (like bd*n)
// - fit: fit pattern to n cycles
// - stretch: sustain notes to fill gaps (legato 1.0)

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

// ========== Late Tests ==========

#[test]
fn test_late_basic() {
    // Test: delay pattern by 0.25 cycles
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ late 0.25
"#,
        "Late by 0.25 cycles",
    );
}

#[test]
fn test_late_small_amount() {
    // Test: subtle delay
    test_compilation(
        r#"
tempo: 0.5
out: "bd*8" $ late 0.125
"#,
        "Late by 0.125 cycles (small delay)",
    );
}

#[test]
fn test_late_large_amount() {
    // Test: large delay (more than one cycle)
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn" $ late 1.5
"#,
        "Late by 1.5 cycles (large delay)",
    );
}

#[test]
fn test_late_with_chain() {
    // Test: delayed pattern through effects
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh*4" $ late 0.5 # lpf 1000 0.8
"#,
        "Late with chained filter",
    );
}

#[test]
fn test_late_combined_with_fast() {
    // Test: late combined with other transforms
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ late 0.25 $ fast 2
"#,
        "Late combined with fast",
    );
}

// ========== Early Tests ==========

#[test]
fn test_early_basic() {
    // Test: shift pattern earlier by 0.25 cycles
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ early 0.25
"#,
        "Early by 0.25 cycles",
    );
}

#[test]
fn test_early_small_amount() {
    // Test: subtle shift
    test_compilation(
        r#"
tempo: 0.5
out: "bd*8" $ early 0.125
"#,
        "Early by 0.125 cycles (small shift)",
    );
}

#[test]
fn test_early_large_amount() {
    // Test: large shift (more than one cycle)
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn" $ early 1.5
"#,
        "Early by 1.5 cycles (large shift)",
    );
}

#[test]
fn test_early_with_effects() {
    // Test: early pattern through reverb
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh*2" $ early 0.5 # reverb 0.5 0.3 0.2
"#,
        "Early with reverb",
    );
}

#[test]
fn test_early_combined() {
    // Test: early combined with other transforms
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh" $ early 0.125 $ rev
"#,
        "Early combined with rev",
    );
}

// ========== Dup Tests ==========

#[test]
fn test_dup_basic() {
    // Test: duplicate pattern 2 times
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn" $ dup 2
"#,
        "Dup 2 times",
    );
}

#[test]
fn test_dup_many_times() {
    // Test: duplicate many times
    test_compilation(
        r#"
tempo: 0.5
out: "bd" $ dup 8
"#,
        "Dup 8 times",
    );
}

#[test]
fn test_dup_with_euclidean() {
    // Test: dup applied to euclidean pattern
    test_compilation(
        r#"
tempo: 0.5
out: "bd(3,8)" $ dup 4
"#,
        "Dup with euclidean pattern",
    );
}

#[test]
fn test_dup_with_effects() {
    // Test: duplicated pattern through distortion
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn" $ dup 3 # distort 2.0 0.5
"#,
        "Dup with distortion",
    );
}

#[test]
fn test_dup_combined() {
    // Test: dup combined with other transforms
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh" $ dup 4 $ rev
"#,
        "Dup combined with rev",
    );
}

// ========== Fit Tests ==========

#[test]
fn test_fit_basic() {
    // Test: fit pattern to 2 cycles
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ fit 2
"#,
        "Fit to 2 cycles",
    );
}

#[test]
fn test_fit_many_cycles() {
    // Test: fit pattern to many cycles
    test_compilation(
        r#"
tempo: 0.5
out: "bd*8" $ fit 8
"#,
        "Fit to 8 cycles",
    );
}

#[test]
fn test_fit_with_alternation() {
    // Test: fit with alternation pattern
    test_compilation(
        r#"
tempo: 0.5
out: "<bd sn hh cp>" $ fit 4
"#,
        "Fit with alternation",
    );
}

#[test]
fn test_fit_with_effects() {
    // Test: fitted pattern through delay
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh*2" $ fit 3 # delay 0.25 0.5 0.3
"#,
        "Fit with delay",
    );
}

#[test]
fn test_fit_combined() {
    // Test: fit combined with other transforms
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn" $ fit 4 $ fast 2
"#,
        "Fit combined with fast",
    );
}

// ========== Stretch Tests ==========

#[test]
fn test_stretch_basic() {
    // Test: stretch pattern (sustain notes)
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ stretch
"#,
        "Stretch basic pattern",
    );
}

#[test]
fn test_stretch_with_fast() {
    // Test: stretched pattern with fast
    test_compilation(
        r#"
tempo: 0.5
out: "bd*4" $ stretch $ fast 2
"#,
        "Stretch with fast",
    );
}

#[test]
fn test_stretch_with_effects() {
    // Test: stretched pattern through chorus
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh" $ stretch # chorus 0.5 0.3 0.2
"#,
        "Stretch with chorus",
    );
}

#[test]
fn test_stretch_with_euclidean() {
    // Test: stretch with euclidean rhythm
    test_compilation(
        r#"
tempo: 0.5
out: "bd(3,8)" $ stretch
"#,
        "Stretch with euclidean pattern",
    );
}

#[test]
fn test_stretch_combined() {
    // Test: stretch combined with other transforms
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh*4" $ stretch $ rev
"#,
        "Stretch combined with rev",
    );
}

// ========== Combined Tests ==========

#[test]
fn test_all_five_operations_in_program() {
    // Test: using all five operations in same program
    test_compilation(
        r#"
tempo: 0.5
~delayed: "bd*8" $ late 0.25
~advanced: "sn*4" $ early 0.125
~duplicated: "hh*2" $ dup 4
~fitted: "cp*4" $ fit 2
~stretched: "bd sn" $ stretch
out: ~delayed + ~advanced + ~duplicated + ~fitted + ~stretched
"#,
        "All five operations in one program",
    );
}

#[test]
fn test_late_and_early_combined() {
    // Test: late and early together (should cancel out if same amount)
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ late 0.5 $ early 0.5
"#,
        "Late and early combined",
    );
}

#[test]
fn test_dup_and_fit_interplay() {
    // Test: dup speeds up, fit slows down - interesting interplay
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn" $ dup 4 $ fit 2
"#,
        "Dup and fit combined",
    );
}

#[test]
fn test_stretch_and_dup() {
    // Test: stretch makes notes longer, dup makes more repetitions
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh" $ stretch $ dup 3
"#,
        "Stretch and dup",
    );
}

#[test]
fn test_timing_manipulations_with_effects_chain() {
    // Test: timing manipulation with effects chain
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh*4" $ late 0.25 $ early 0.125 $ dup 2 # lpf 1000 0.8 # reverb 0.5 0.3 0.2
"#,
        "Timing manipulations with effects chain",
    );
}

#[test]
fn test_fit_with_stretch() {
    // Test: fit to slow down, stretch to sustain
    test_compilation(
        r#"
tempo: 0.5
out: "bd*4 sn*4" $ fit 4 $ stretch
"#,
        "Fit and stretch combined",
    );
}
