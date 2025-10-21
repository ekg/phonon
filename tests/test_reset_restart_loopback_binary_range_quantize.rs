// Test reset, restart, loopback, binary, range, and quantize pattern transforms
//
// These operations modify pattern timing and structure:
// - reset: restart pattern every n cycles
// - restart: restart pattern every n cycles (alias for reset)
// - loopback: play backwards then forwards
// - binary: bit mask pattern
// - range: scale numeric values to range (numeric patterns only)
// - quantize: quantize numeric values (numeric patterns only)

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

    compile_program(statements, 44100.0)
        .unwrap_or_else(|e| panic!("{} - Compilation failed: {}", description, e));
}

/// Helper to compile code and verify it fails with expected error
fn test_compilation_error(code: &str, description: &str, expected_error_substring: &str) {
    let (rest, statements) =
        parse_program(code).unwrap_or_else(|e| panic!("{} - Parse failed: {:?}", description, e));
    assert_eq!(
        rest.trim(),
        "",
        "{} - Parser didn't consume all input",
        description
    );

    match compile_program(statements, 44100.0) {
        Ok(_) => panic!(
            "{} - Expected compilation to fail but it succeeded",
            description
        ),
        Err(e) => {
            assert!(
                e.contains(expected_error_substring),
                "{} - Error message '{}' does not contain expected substring '{}'",
                description,
                e,
                expected_error_substring
            );
        }
    }
}

// ========== Reset Tests ==========

#[test]
fn test_reset_basic() {
    // Test: reset pattern every 2 cycles
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ reset 2
"#,
        "Reset every 2 cycles",
    );
}

#[test]
fn test_reset_single_cycle() {
    // Test: reset every cycle
    test_compilation(
        r#"
tempo: 2.0
out: "bd*8" $ reset 1
"#,
        "Reset every cycle",
    );
}

#[test]
fn test_reset_many_cycles() {
    // Test: reset every 8 cycles
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4" $ reset 8
"#,
        "Reset every 8 cycles",
    );
}

#[test]
fn test_reset_with_effects() {
    // Test: reset pattern through reverb
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*2" $ reset 4 # reverb 0.5 0.3 0.2
"#,
        "Reset with reverb",
    );
}

#[test]
fn test_reset_combined() {
    // Test: reset combined with other transforms
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh" $ reset 3 $ fast 2
"#,
        "Reset combined with fast",
    );
}

// ========== Restart Tests ==========

#[test]
fn test_restart_basic() {
    // Test: restart pattern every 2 cycles (alias for reset)
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ restart 2
"#,
        "Restart every 2 cycles",
    );
}

#[test]
fn test_restart_with_subdivision() {
    // Test: restart with subdivision pattern
    test_compilation(
        r#"
tempo: 2.0
out: "bd*4 sn*4" $ restart 4
"#,
        "Restart with subdivision",
    );
}

#[test]
fn test_reset_and_restart_equivalence() {
    // Test: reset and restart should be equivalent
    test_compilation(
        r#"
tempo: 2.0
~reset_pat: "bd sn" $ reset 3
~restart_pat: "hh cp" $ restart 3
out: ~reset_pat + ~restart_pat
"#,
        "Reset and restart equivalence",
    );
}

// ========== Loopback Tests ==========

#[test]
fn test_loopback_basic() {
    // Test: play backwards then forwards
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ loopback
"#,
        "Loopback basic",
    );
}

#[test]
fn test_loopback_with_subdivision() {
    // Test: loopback with subdivision
    test_compilation(
        r#"
tempo: 2.0
out: "bd*8" $ loopback
"#,
        "Loopback with subdivision",
    );
}

#[test]
fn test_loopback_with_effects() {
    // Test: loopback pattern through delay
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4" $ loopback # delay 0.25 0.5 0.3
"#,
        "Loopback with delay",
    );
}

#[test]
fn test_loopback_combined() {
    // Test: loopback combined with other transforms
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh" $ loopback $ fast 2
"#,
        "Loopback combined with fast",
    );
}

// ========== Binary Tests ==========

#[test]
fn test_binary_basic() {
    // Test: binary pattern with simple mask
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ binary 5
"#,
        "Binary mask 5 (0b0101)",
    );
}

#[test]
fn test_binary_alternating() {
    // Test: alternating pattern (0b1010 = 10)
    test_compilation(
        r#"
tempo: 2.0
out: "bd*8" $ binary 10
"#,
        "Binary mask 10 (0b1010)",
    );
}

#[test]
fn test_binary_sparse() {
    // Test: sparse pattern (0b0001 = 1)
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ binary 1
"#,
        "Binary mask 1 (0b0001)",
    );
}

#[test]
fn test_binary_dense() {
    // Test: dense pattern (0b1111 = 15)
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ binary 15
"#,
        "Binary mask 15 (0b1111)",
    );
}

#[test]
fn test_binary_with_effects() {
    // Test: binary pattern through chorus
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4 cp*2" $ binary 7 # chorus 0.5 0.3 0.2
"#,
        "Binary with chorus",
    );
}

#[test]
fn test_binary_combined() {
    // Test: binary combined with other transforms
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ binary 9 $ fast 2
"#,
        "Binary combined with fast",
    );
}

// ========== Range Tests (Numeric Patterns Only) ==========

#[test]
fn test_range_on_sample_pattern_fails() {
    // Test: range should fail on sample patterns
    test_compilation_error(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ range 0.0 1.0
"#,
        "Range on sample pattern should fail",
        "range transform only works with numeric patterns",
    );
}

#[test]
fn test_range_basic() {
    // Test: range on oscillator pattern (numeric)
    test_compilation(
        r#"
tempo: 2.0
~lfo: sine 0.5
~ranged: ~lfo $ range 200.0 2000.0
out: saw 110 # lpf ~ranged 0.8
"#,
        "Range on oscillator pattern",
    );
}

#[test]
fn test_range_small() {
    // Test: range to small values
    test_compilation(
        r#"
tempo: 2.0
~lfo: sine 1.0
~ranged: ~lfo $ range 0.1 0.9
out: saw 110 * ~ranged
"#,
        "Range to small values",
    );
}

#[test]
fn test_range_negative() {
    // Test: range with negative values
    test_compilation(
        r#"
tempo: 2.0
~lfo: sine 0.25
~ranged: ~lfo $ range (-1.0) 1.0
out: saw 110 * ~ranged
"#,
        "Range with negative values",
    );
}

// ========== Quantize Tests (Numeric Patterns Only) ==========

#[test]
fn test_quantize_on_sample_pattern_fails() {
    // Test: quantize should fail on sample patterns
    test_compilation_error(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ quantize 4
"#,
        "Quantize on sample pattern should fail",
        "quantize transform only works with numeric patterns",
    );
}

#[test]
fn test_quantize_basic() {
    // Test: quantize on oscillator pattern (numeric)
    test_compilation(
        r#"
tempo: 2.0
~lfo: sine 0.5
~quantized: ~lfo $ quantize 4.0
out: saw 110 # lpf (~quantized * 1000 + 500) 0.8
"#,
        "Quantize on oscillator pattern",
    );
}

#[test]
fn test_quantize_fine() {
    // Test: quantize to many steps
    test_compilation(
        r#"
tempo: 2.0
~lfo: sine 1.0
~quantized: ~lfo $ quantize 16.0
out: saw 110 * ~quantized
"#,
        "Quantize to 16 steps",
    );
}

#[test]
fn test_quantize_coarse() {
    // Test: quantize to few steps
    test_compilation(
        r#"
tempo: 2.0
~lfo: sine 0.25
~quantized: ~lfo $ quantize 2.0
out: saw 110 * ~quantized
"#,
        "Quantize to 2 steps",
    );
}

// ========== Combined Tests ==========

#[test]
fn test_all_six_operations_in_program() {
    // Test: using all six operations in same program
    test_compilation(
        r#"
tempo: 2.0
~reset_pat: "bd*8" $ reset 4
~restarted: "sn*2" $ restart 3
~looped_back: "hh*8" $ loopback
~binaried: "cp*4" $ binary 5
~lfo1: sine 0.5 $ range 200.0 2000.0
~lfo2: sine 1.0 $ quantize 8.0
out: ~reset_pat + ~restarted + ~looped_back + ~binaried
"#,
        "All six operations in one program",
    );
}

#[test]
fn test_reset_and_loopback() {
    // Test: reset and loopback together
    test_compilation(
        r#"
tempo: 2.0
~reset_pat: "bd sn" $ reset 2
~looped: "hh cp" $ loopback
out: ~reset_pat + ~looped
"#,
        "Reset and loopback in same program",
    );
}

#[test]
fn test_binary_and_reset() {
    // Test: binary and reset together
    test_compilation(
        r#"
tempo: 2.0
~binaried: "bd*4 sn*4" $ binary 10
~reset_pat: "hh*4 cp*4" $ reset 3
out: ~binaried + ~reset_pat
"#,
        "Binary and reset in same program",
    );
}

#[test]
fn test_range_and_quantize() {
    // Test: range and quantize together
    test_compilation(
        r#"
tempo: 2.0
~lfo1: sine 0.5 $ range 100.0 1000.0
~lfo2: sine 1.0 $ quantize 4.0
~filtered: saw 110 # lpf ~lfo1 0.8
out: ~filtered * ~lfo2
"#,
        "Range and quantize together",
    );
}

#[test]
fn test_complex_combination() {
    // Test: complex combination of operations
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ reset 4 $ loopback $ binary 7 $ fast 2
"#,
        "Complex combination: reset, loopback, binary, fast",
    );
}

#[test]
fn test_with_effects_chain() {
    // Test: multiple operations with effects chain
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh*4" $ reset 3 $ binary 5 # lpf 1000 0.8 # reverb 0.5 0.3 0.2
"#,
        "Multiple operations with effects chain",
    );
}

#[test]
fn test_loopback_and_restart() {
    // Test: loopback and restart combined
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn hh cp" $ loopback $ restart 2
"#,
        "Loopback and restart combined",
    );
}

#[test]
fn test_binary_with_reset() {
    // Test: binary with reset
    test_compilation(
        r#"
tempo: 2.0
out: "bd sn" $ binary 3 $ reset 4
"#,
        "Binary with reset",
    );
}

#[test]
fn test_in_complex_multi_bus_program() {
    // Test: all operations in complex multi-bus program
    test_compilation(
        r#"
tempo: 2.0
~kick: "bd*4" $ reset 2 $ binary 9
~snare: "~ sn ~ sn" $ restart 4
~hats: "hh*8" $ loopback
~lfo: sine 0.5 $ range 500.0 2000.0
~quant_lfo: sine 1.0 $ quantize 4.0
~mixed: (~kick + ~snare) $ reset 3
out: ~mixed * 0.5 + ~hats * 0.3
"#,
        "Complex multi-bus program with all operations",
    );
}
