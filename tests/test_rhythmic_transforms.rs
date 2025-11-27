// Test rhythmic transforms: swing, legato, staccato, echo, segment
//
// These operations modify event timing and duration:
// - swing: adds swing feel to events
// - legato: lengthens event duration
// - staccato: shortens event duration
// - echo: creates echo/delay effect on pattern
// - segment: divides pattern into n segments

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

// ========== Swing Tests ==========

#[test]
fn test_swing_basic() {
    // Test: swing with small amount - adds subtle swing
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ swing 0.1
"#,
        "Swing with 0.1 amount",
    );
}

#[test]
fn test_swing_large_amount() {
    // Test: swing with large amount - dramatic swing feel
    test_compilation(
        r#"
tempo: 0.5
out: "hh*8" $ swing 0.3
"#,
        "Swing with 0.3 amount (larger)",
    );
}

#[test]
fn test_swing_with_chain() {
    // Test: swing routed through effects
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh" $ swing 0.2 # lpf 1000 0.8
"#,
        "Swing with chained filter",
    );
}

#[test]
fn test_swing_combined_with_fast() {
    // Test: swing combined with other transforms
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn" $ fast 2 $ swing 0.15
"#,
        "Swing combined with fast",
    );
}

// ========== Legato Tests ==========

#[test]
fn test_legato_basic() {
    // Test: legato makes events longer
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ legato 2.0
"#,
        "Legato with 2.0 factor",
    );
}

#[test]
fn test_legato_extreme() {
    // Test: legato with large factor - very long notes
    test_compilation(
        r#"
tempo: 0.5
out: "bd*4" $ legato 5.0
"#,
        "Legato with 5.0 factor (extreme)",
    );
}

#[test]
fn test_legato_with_sample_playback() {
    // Test: legato on sample playback
    test_compilation(
        r#"
tempo: 0.5
out: s "bd sn hh*4" $ legato 1.5
"#,
        "Legato with sample playback",
    );
}

#[test]
fn test_legato_with_reverb() {
    // Test: legato pattern through reverb
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn" $ legato 3.0 # reverb 0.5 0.3 0.2
"#,
        "Legato with reverb",
    );
}

// ========== Staccato Tests ==========

#[test]
fn test_staccato_basic() {
    // Test: staccato makes events shorter
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ staccato 0.5
"#,
        "Staccato with 0.5 factor",
    );
}

#[test]
fn test_staccato_extreme() {
    // Test: staccato with very small factor - very short notes
    test_compilation(
        r#"
tempo: 0.5
out: "bd*8" $ staccato 0.1
"#,
        "Staccato with 0.1 factor (extreme)",
    );
}

#[test]
fn test_staccato_with_euclidean() {
    // Test: staccato on euclidean rhythm
    test_compilation(
        r#"
tempo: 0.5
out: "bd(3,8)" $ staccato 0.3
"#,
        "Staccato with euclidean pattern",
    );
}

#[test]
fn test_staccato_with_bitcrush() {
    // Test: staccato pattern through bitcrush
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh*4" $ staccato 0.2 # bitcrush 8 8000
"#,
        "Staccato with bitcrush",
    );
}

// ========== Echo Tests ==========

#[test]
fn test_echo_basic() {
    // Test: echo with 3 repeats
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn" $ echo 3 0.25 0.5
"#,
        "Echo with 3 repeats, 0.25 time, 0.5 feedback",
    );
}

#[test]
fn test_echo_long_decay() {
    // Test: echo with many repeats and slow decay
    test_compilation(
        r#"
tempo: 0.5
out: "bd" $ echo 8 0.125 0.7
"#,
        "Echo with long decay (8 repeats)",
    );
}

#[test]
fn test_echo_fast_slapback() {
    // Test: fast slapback echo
    test_compilation(
        r#"
tempo: 0.5
out: "sn*2" $ echo 2 0.0625 0.3
"#,
        "Fast slapback echo",
    );
}

#[test]
fn test_echo_with_effects() {
    // Test: echo pattern through effects
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh" $ echo 4 0.25 0.6 # lpf 2000 0.5
"#,
        "Echo with filter",
    );
}

// ========== Segment Tests ==========

#[test]
fn test_segment_basic() {
    // Test: divide pattern into 4 segments
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ segment 4
"#,
        "Segment into 4 parts",
    );
}

#[test]
fn test_segment_many() {
    // Test: divide pattern into many segments
    test_compilation(
        r#"
tempo: 0.5
out: "bd*4" $ segment 16
"#,
        "Segment into 16 parts",
    );
}

#[test]
fn test_segment_with_alternation() {
    // Test: segment pattern with alternation
    test_compilation(
        r#"
tempo: 0.5
out: "<bd sn hh cp>" $ segment 8
"#,
        "Segment with alternation",
    );
}

#[test]
fn test_segment_with_reverb() {
    // Test: segmented pattern through reverb
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh*2" $ segment 8 # reverb 0.5 0.3 0.2
"#,
        "Segment with reverb",
    );
}

// ========== Combined Tests ==========

#[test]
fn test_all_five_operations_in_program() {
    // Test: using all five operations in same program
    test_compilation(
        r#"
tempo: 0.5
~swung: "bd*4" $ swing 0.2
~long: "sn*4" $ legato 2.0
~short: "hh*8" $ staccato 0.3
~echoed: "cp" $ echo 4 0.25 0.5
~segmented: "bd sn hh cp" $ segment 8
out: ~swung + ~long + ~short + ~echoed + ~segmented
"#,
        "All five operations in one program",
    );
}

#[test]
fn test_combined_rhythmic_transforms() {
    // Test: combining multiple rhythmic transforms
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh" $ swing 0.15 $ legato 1.5 $ echo 3 0.25 0.4
"#,
        "Combined swing + legato + echo",
    );
}

#[test]
fn test_segment_with_other_transforms() {
    // Test: segment combined with other operations
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn" $ segment 4 $ fast 2 $ staccato 0.2
"#,
        "Segment + fast + staccato",
    );
}

#[test]
fn test_rhythmic_transforms_with_effects_chain() {
    // Test: rhythmic transforms with effects chain
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh*4" $ swing 0.2 $ echo 3 0.25 0.5 # lpf 1000 0.8 # reverb 0.5 0.3 0.2
"#,
        "Rhythmic transforms with effects chain",
    );
}
