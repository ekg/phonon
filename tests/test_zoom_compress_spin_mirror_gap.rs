// Test zoom, compress, spin, mirror, and gap pattern transforms
//
// These operations modify pattern timing and structure:
// - zoom: focus on specific time range within pattern
// - compress: compress pattern to fit within time range
// - spin: rotate through n different versions across cycles
// - mirror: palindrome within cycle (alias for palindrome)
// - gap: insert silence every n cycles

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

// ========== Zoom Tests ==========

#[test]
fn test_zoom_basic() {
    // Test: zoom to middle half of pattern (0.25 to 0.75)
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh cp" $ zoom 0.25 0.75
"#,
        "Zoom to middle half",
    );
}

#[test]
fn test_zoom_first_quarter() {
    // Test: zoom to first quarter (0 to 0.25)
    test_compilation(
        r#"
tempo: 0.5
out $ "bd*8" $ zoom 0 0.25
"#,
        "Zoom to first quarter",
    );
}

#[test]
fn test_zoom_last_third() {
    // Test: zoom to last third (0.66 to 1.0)
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh cp bd sn" $ zoom 0.66 1.0
"#,
        "Zoom to last third",
    );
}

#[test]
fn test_zoom_with_chain() {
    // Test: zoomed pattern through effects
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh*4" $ zoom 0.5 1.0 # lpf 1000 0.8
"#,
        "Zoom with chained filter",
    );
}

#[test]
fn test_zoom_combined_with_fast() {
    // Test: zoom combined with other transforms
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh cp" $ zoom 0.25 0.75 $ fast 2
"#,
        "Zoom combined with fast",
    );
}

// ========== Compress Tests ==========

#[test]
fn test_compress_basic() {
    // Test: compress pattern to first half (0 to 0.5)
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh cp" $ compress 0 0.5
"#,
        "Compress to first half",
    );
}

#[test]
fn test_compress_to_middle() {
    // Test: compress pattern to middle section (0.25 to 0.75)
    test_compilation(
        r#"
tempo: 0.5
out $ "bd*4" $ compress 0.25 0.75
"#,
        "Compress to middle section",
    );
}

#[test]
fn test_compress_narrow() {
    // Test: compress to very narrow window
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh" $ compress 0.4 0.6
"#,
        "Compress to narrow window",
    );
}

#[test]
fn test_compress_with_effects() {
    // Test: compressed pattern through reverb
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh*2" $ compress 0 0.5 # reverb 0.5 0.3 0.2
"#,
        "Compress with reverb",
    );
}

#[test]
fn test_compress_combined() {
    // Test: compress combined with other transforms
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn" $ compress 0.25 0.5 $ rev
"#,
        "Compress combined with rev",
    );
}

// ========== Spin Tests ==========

#[test]
fn test_spin_basic() {
    // Test: spin with 3 rotations
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh cp" $ spin 3
"#,
        "Spin with 3 rotations",
    );
}

#[test]
fn test_spin_large_n() {
    // Test: spin with many rotations
    test_compilation(
        r#"
tempo: 0.5
out $ "bd*4" $ spin 8
"#,
        "Spin with 8 rotations",
    );
}

#[test]
fn test_spin_with_euclidean() {
    // Test: spin applied to euclidean pattern
    test_compilation(
        r#"
tempo: 0.5
out $ "bd(3,8)" $ spin 4
"#,
        "Spin with euclidean pattern",
    );
}

#[test]
fn test_spin_with_effects() {
    // Test: spun pattern through bitcrush
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh*2" $ spin 4 # bitcrush 8 8000
"#,
        "Spin with bitcrush",
    );
}

#[test]
fn test_spin_combined() {
    // Test: spin combined with other transforms
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn" $ spin 3 $ fast 2
"#,
        "Spin combined with fast",
    );
}

// ========== Mirror Tests ==========

#[test]
fn test_mirror_basic() {
    // Test: mirror pattern (palindrome)
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh cp" $ mirror
"#,
        "Mirror basic pattern",
    );
}

#[test]
fn test_mirror_with_fast() {
    // Test: mirrored pattern with fast
    test_compilation(
        r#"
tempo: 0.5
out $ "bd*4" $ mirror $ fast 2
"#,
        "Mirror with fast",
    );
}

#[test]
fn test_mirror_with_effects() {
    // Test: mirrored pattern through delay
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh" $ mirror # delay 0.25 0.5 0.3
"#,
        "Mirror with delay",
    );
}

#[test]
fn test_mirror_with_alternation() {
    // Test: mirror with alternation pattern
    test_compilation(
        r#"
tempo: 0.5
out $ "<bd sn hh cp>" $ mirror
"#,
        "Mirror with alternation",
    );
}

// ========== Gap Tests ==========

#[test]
fn test_gap_basic() {
    // Test: insert gap every 2 cycles
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh cp" $ gap 2
"#,
        "Gap every 2 cycles",
    );
}

#[test]
fn test_gap_frequent() {
    // Test: gap every 3 cycles
    test_compilation(
        r#"
tempo: 0.5
out $ "bd*4" $ gap 3
"#,
        "Gap every 3 cycles",
    );
}

#[test]
fn test_gap_large_n() {
    // Test: gap every 8 cycles
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh*4" $ gap 8
"#,
        "Gap every 8 cycles",
    );
}

#[test]
fn test_gap_with_effects() {
    // Test: gapped pattern through chorus
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh*2" $ gap 4 # chorus 0.5 0.3 0.2
"#,
        "Gap with chorus",
    );
}

#[test]
fn test_gap_combined() {
    // Test: gap combined with other transforms
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn" $ gap 2 $ rev
"#,
        "Gap combined with rev",
    );
}

// ========== Combined Tests ==========

#[test]
fn test_all_five_operations_in_program() {
    // Test: using all five operations in same program
    test_compilation(
        r#"
tempo: 0.5
~zoomed $ "bd*8" $ zoom 0.25 0.75
~compressed $ "sn*4" $ compress 0 0.5
~spun $ "hh*8" $ spin 4
~mirrored $ "cp*2" $ mirror
~gapped $ "bd sn" $ gap 2
out $ ~zoomed + ~compressed + ~spun + ~mirrored + ~gapped
"#,
        "All five operations in one program",
    );
}

#[test]
fn test_combined_time_manipulations() {
    // Test: combining multiple time manipulation operations
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh cp" $ zoom 0.25 0.75 $ compress 0 0.5 $ spin 4
"#,
        "Combined zoom + compress + spin",
    );
}

#[test]
fn test_zoom_and_mirror() {
    // Test: zoom combined with mirror
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh*4" $ zoom 0.5 1.0 $ mirror
"#,
        "Zoom and mirror",
    );
}

#[test]
fn test_compress_and_gap() {
    // Test: compress with gap for rhythmic breaks
    test_compilation(
        r#"
tempo: 0.5
out $ "bd*4 sn*4 hh*8" $ compress 0.25 0.75 $ gap 3
"#,
        "Compress and gap",
    );
}

#[test]
fn test_with_effects_chain() {
    // Test: time manipulation with effects chain
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh*4" $ zoom 0.25 0.75 $ spin 3 # lpf 1000 0.8 # reverb 0.5 0.3 0.2
"#,
        "Time manipulation with effects chain",
    );
}
