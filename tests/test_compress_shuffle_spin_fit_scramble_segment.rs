// Test compress, shuffle, spin, fit, scramble, and segment pattern transforms
//
// These operations modify pattern timing and structure:
// - compress: compress pattern to time range (begin, end)
// - shuffle: randomly shift events in time (amount)
// - spin: rotate through n different versions
// - fit: fit pattern to n cycles
// - scramble: Fisher-Yates shuffle of events (seed n)
// - segment: divide pattern into n segments

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

// ========== Compress Tests ==========

#[test]
fn test_compress_basic() {
    // Test: compress pattern to middle half of cycle
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh cp" $ compress 0.25 0.75
"#,
        "Compress to 0.25-0.75",
    );
}

#[test]
fn test_compress_first_quarter() {
    // Test: compress to first quarter
    test_compilation(
        r#"
tempo: 0.5
out $ "bd*8" $ compress 0.0 0.25
"#,
        "Compress to first quarter (0.0-0.25)",
    );
}

#[test]
fn test_compress_last_third() {
    // Test: compress to last third
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh*4" $ compress 0.66 1.0
"#,
        "Compress to last third (0.66-1.0)",
    );
}

#[test]
fn test_compress_tiny_window() {
    // Test: compress to tiny window
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh cp" $ compress 0.4 0.6
"#,
        "Compress to tiny window (0.4-0.6)",
    );
}

#[test]
fn test_compress_with_effects() {
    // Test: compressed pattern through reverb
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh*2" $ compress 0.0 0.5 # reverb 0.5 0.3 0.2
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
out $ "bd sn hh" $ compress 0.25 0.75 $ fast 2
"#,
        "Compress combined with fast",
    );
}

// ========== Shuffle Tests ==========

#[test]
fn test_shuffle_basic() {
    // Test: shuffle pattern with default amount
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh cp" $ shuffle 0.5
"#,
        "Shuffle with amount 0.5",
    );
}

#[test]
fn test_shuffle_small_amount() {
    // Test: shuffle with small amount (subtle variation)
    test_compilation(
        r#"
tempo: 0.5
out $ "bd*8" $ shuffle 0.1
"#,
        "Shuffle with small amount (0.1)",
    );
}

#[test]
fn test_shuffle_large_amount() {
    // Test: shuffle with large amount (more randomization)
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh*4" $ shuffle 0.9
"#,
        "Shuffle with large amount (0.9)",
    );
}

#[test]
fn test_shuffle_with_subdivision() {
    // Test: shuffle pattern with subdivision
    test_compilation(
        r#"
tempo: 0.5
out $ "bd*4 sn*4 hh*8" $ shuffle 0.5
"#,
        "Shuffle with subdivision",
    );
}

#[test]
fn test_shuffle_with_effects() {
    // Test: shuffled pattern through delay
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh*2" $ shuffle 0.5 # delay 0.25 0.5 0.3
"#,
        "Shuffle with delay",
    );
}

#[test]
fn test_shuffle_combined() {
    // Test: shuffle combined with other transforms
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh" $ shuffle 0.5 $ fast 2
"#,
        "Shuffle combined with fast",
    );
}

// ========== Spin Tests ==========

#[test]
fn test_spin_basic() {
    // Test: spin pattern (rotate through 4 versions)
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh cp" $ spin 4
"#,
        "Spin 4 versions",
    );
}

#[test]
fn test_spin_double() {
    // Test: spin with 2 versions (simple alternation)
    test_compilation(
        r#"
tempo: 0.5
out $ "bd*8" $ spin 2
"#,
        "Spin 2 versions",
    );
}

#[test]
fn test_spin_many() {
    // Test: spin with many versions
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh*4" $ spin 8
"#,
        "Spin 8 versions",
    );
}

#[test]
fn test_spin_negative() {
    // Test: spin with negative n (reverse direction)
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh cp" $ spin (-2)
"#,
        "Spin with negative direction",
    );
}

#[test]
fn test_spin_with_effects() {
    // Test: spun pattern through chorus
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh*2" $ spin 4 # chorus 0.5 0.3 0.2
"#,
        "Spin with chorus",
    );
}

#[test]
fn test_spin_combined() {
    // Test: spin combined with other transforms
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh" $ spin 3 $ slow 2
"#,
        "Spin combined with slow",
    );
}

// ========== Fit Tests ==========

#[test]
fn test_fit_basic() {
    // Test: fit pattern to 2 cycles
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh cp" $ fit 2
"#,
        "Fit to 2 cycles",
    );
}

#[test]
fn test_fit_single_cycle() {
    // Test: fit to single cycle (no change)
    test_compilation(
        r#"
tempo: 0.5
out $ "bd*8" $ fit 1
"#,
        "Fit to 1 cycle",
    );
}

#[test]
fn test_fit_many_cycles() {
    // Test: fit to many cycles
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh*4" $ fit 8
"#,
        "Fit to 8 cycles",
    );
}

#[test]
fn test_fit_negative() {
    // Test: fit with negative n (reverse)
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh cp" $ fit (-3)
"#,
        "Fit with negative cycles",
    );
}

#[test]
fn test_fit_with_effects() {
    // Test: fitted pattern through reverb
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh*2" $ fit 4 # reverb 0.5 0.3 0.2
"#,
        "Fit with reverb",
    );
}

#[test]
fn test_fit_combined() {
    // Test: fit combined with other transforms
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh" $ fit 3 $ rev
"#,
        "Fit combined with rev",
    );
}

// ========== Scramble Tests ==========

#[test]
fn test_scramble_basic() {
    // Test: scramble pattern with seed
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh cp" $ scramble 42
"#,
        "Scramble with seed 42",
    );
}

#[test]
fn test_scramble_different_seeds() {
    // Test: scramble with different seeds
    test_compilation(
        r#"
tempo: 0.5
~scr1 $ "bd*4" $ scramble 1
~scr2 $ "sn*4" $ scramble 2
out $ ~scr1 + ~scr2
"#,
        "Scramble with different seeds",
    );
}

#[test]
fn test_scramble_zero_seed() {
    // Test: scramble with zero seed
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh*4" $ scramble 0
"#,
        "Scramble with seed 0",
    );
}

#[test]
fn test_scramble_large_seed() {
    // Test: scramble with large seed
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh cp" $ scramble 999999
"#,
        "Scramble with large seed",
    );
}

#[test]
fn test_scramble_with_effects() {
    // Test: scrambled pattern through distortion
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh*2" $ scramble 7 # distort 2.0 0.5
"#,
        "Scramble with distortion",
    );
}

#[test]
fn test_scramble_combined() {
    // Test: scramble combined with other transforms
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh" $ scramble 5 $ fast 2
"#,
        "Scramble combined with fast",
    );
}

// ========== Segment Tests ==========

#[test]
fn test_segment_basic() {
    // Test: segment pattern into 4 pieces
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh cp" $ segment 4
"#,
        "Segment into 4 pieces",
    );
}

#[test]
fn test_segment_two() {
    // Test: segment into 2 pieces
    test_compilation(
        r#"
tempo: 0.5
out $ "bd*8" $ segment 2
"#,
        "Segment into 2 pieces",
    );
}

#[test]
fn test_segment_many() {
    // Test: segment into many pieces
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh*4" $ segment 16
"#,
        "Segment into 16 pieces",
    );
}

#[test]
fn test_segment_single() {
    // Test: segment into 1 piece (no change)
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh cp" $ segment 1
"#,
        "Segment into 1 piece",
    );
}

#[test]
fn test_segment_with_effects() {
    // Test: segmented pattern through lpf
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh*2" $ segment 8 # lpf 1000 0.8
"#,
        "Segment with lpf",
    );
}

#[test]
fn test_segment_combined() {
    // Test: segment combined with other transforms
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh" $ segment 4 $ rev
"#,
        "Segment combined with rev",
    );
}

// ========== Combined Tests ==========

#[test]
fn test_all_six_operations_in_program() {
    // Test: using all six operations in same program
    test_compilation(
        r#"
tempo: 0.5
~compressed $ "bd*8" $ compress 0.25 0.75
~shuffled $ "sn*4" $ shuffle 0.5
~spun $ "hh*8" $ spin 4
~fitted $ "cp*2" $ fit 3
~scrambled $ "bd sn" $ scramble 42
~segmented $ "hh cp" $ segment 8
out $ ~compressed + ~shuffled + ~spun + ~fitted
"#,
        "All six operations in one program",
    );
}

#[test]
fn test_compress_and_shuffle() {
    // Test: compress and shuffle together
    test_compilation(
        r#"
tempo: 0.5
~compressed $ "bd sn" $ compress 0.0 0.5
~shuffled $ "hh cp" $ shuffle 0.5
out $ ~compressed + ~shuffled
"#,
        "Compress and shuffle together",
    );
}

#[test]
fn test_spin_and_fit() {
    // Test: spin and fit together
    test_compilation(
        r#"
tempo: 0.5
~spun $ "bd*4 sn*4" $ spin 4
~fitted $ "hh*4 cp*4" $ fit 2
out $ ~spun + ~fitted
"#,
        "Spin and fit together",
    );
}

#[test]
fn test_scramble_and_segment() {
    // Test: scramble and segment together
    test_compilation(
        r#"
tempo: 0.5
~scrambled $ "bd sn hh" $ scramble 7
~segmented $ "cp bd sn" $ segment 4
out $ ~scrambled + ~segmented
"#,
        "Scramble and segment together",
    );
}

#[test]
fn test_complex_combination() {
    // Test: complex combination of operations
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh cp" $ compress 0.25 0.75 $ shuffle 0.3 $ spin 4 $ fast 2
"#,
        "Complex combination: compress, shuffle, spin, fast",
    );
}

#[test]
fn test_with_effects_chain() {
    // Test: multiple operations with effects chain
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh*4" $ compress 0.0 0.5 $ shuffle 0.5 # lpf 1000 0.8 # reverb 0.5 0.3 0.2
"#,
        "Multiple operations with effects chain",
    );
}

#[test]
fn test_scramble_with_fit() {
    // Test: scramble with fit
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh cp" $ scramble 9 $ fit 4
"#,
        "Scramble with fit",
    );
}

#[test]
fn test_compress_with_segment() {
    // Test: compress with segment
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh cp" $ compress 0.25 0.75 $ segment 8
"#,
        "Compress with segment",
    );
}

#[test]
fn test_shuffle_with_spin() {
    // Test: shuffle with spin
    test_compilation(
        r#"
tempo: 0.5
out $ "bd*8" $ shuffle 0.5 $ spin 4
"#,
        "Shuffle with spin",
    );
}

#[test]
fn test_in_complex_multi_bus_program() {
    // Test: all operations in complex multi-bus program
    test_compilation(
        r#"
tempo: 0.5
~kick $ "bd*4" $ compress 0.25 0.75 $ shuffle 0.2
~snare $ "~ sn ~ sn" $ spin 4 $ scramble 7
~hats $ "hh*8" $ fit 2 $ segment 4
~perc $ "cp*4" $ compress 0.0 0.5 $ fit 3
~mixed $ (~kick + ~snare) $ shuffle 0.3
out $ ~mixed * 0.5 + ~hats * 0.3 + ~perc * 0.2
"#,
        "Complex multi-bus program with all operations",
    );
}

#[test]
fn test_nested_operations() {
    // Test: nested operations on same pattern
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh cp" $ compress 0.25 0.75 $ shuffle 0.5 $ spin 2 $ fit 4 $ segment 8
"#,
        "Nested operations on same pattern",
    );
}

#[test]
fn test_multiple_compress_operations() {
    // Test: multiple patterns with compress at different ranges
    test_compilation(
        r#"
tempo: 0.5
~c1 $ "bd*8" $ compress 0.0 0.25
~c2 $ "sn*8" $ compress 0.25 0.5
~c3 $ "hh*8" $ compress 0.5 0.75
~c4 $ "cp*8" $ compress 0.75 1.0
out $ ~c1 + ~c2 + ~c3 + ~c4
"#,
        "Multiple compress operations at different ranges",
    );
}

#[test]
fn test_fit_with_segment() {
    // Test: fit and segment working together
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh" $ fit 3 $ segment 12
"#,
        "Fit with segment",
    );
}

#[test]
fn test_all_operations_with_reverb() {
    // Test: all operations through reverb
    test_compilation(
        r#"
tempo: 0.5
out $ "bd sn hh*4" $ compress 0.25 0.75 $ shuffle 0.3 $ spin 4 $ fit 2 $ scramble 5 $ segment 8 # reverb 0.5 0.7 0.3
"#,
        "All operations with reverb",
    );
}
