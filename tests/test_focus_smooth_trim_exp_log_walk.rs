// Test focus, smooth, trim, exp, log, and walk pattern transforms
//
// These operations modify pattern timing and values:
// - focus: focus on specific cycles (cycle_begin to cycle_end)
// - smooth: smooth numeric values (numeric patterns only)
// - trim: trim pattern to time range
// - exp: exponential transformation (numeric patterns only)
// - log: logarithmic transformation (numeric patterns only)
// - walk: random walk (numeric patterns only)

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

    match compile_program(statements, 44100.0, None) {
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

// ========== Focus Tests ==========

#[test]
fn test_focus_basic() {
    // Test: focus on cycles 0 to 2
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ focus 0.0 2.0
"#,
        "Focus on cycles 0-2",
    );
}

#[test]
fn test_focus_middle_cycles() {
    // Test: focus on middle cycles
    test_compilation(
        r#"
tempo: 0.5
out: "bd*8" $ focus 2.0 4.0
"#,
        "Focus on cycles 2-4",
    );
}

#[test]
fn test_focus_single_cycle() {
    // Test: focus on single cycle
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ focus 1.0 2.0
"#,
        "Focus on single cycle 1-2",
    );
}

#[test]
fn test_focus_fractional() {
    // Test: focus on fractional cycle range
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn" $ focus 0.5 1.5
"#,
        "Focus on fractional cycles 0.5-1.5",
    );
}

#[test]
fn test_focus_with_effects() {
    // Test: focused pattern through reverb
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh*2" $ focus 0.0 3.0 # reverb 0.5 0.3 0.2
"#,
        "Focus with reverb",
    );
}

#[test]
fn test_focus_combined() {
    // Test: focus combined with other transforms
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh" $ focus 1.0 3.0 $ fast 2
"#,
        "Focus combined with fast",
    );
}

// ========== Smooth Tests (Numeric Patterns Only) ==========

#[test]
fn test_smooth_on_sample_pattern_fails() {
    // Test: smooth should fail on sample patterns
    test_compilation_error(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ smooth 0.5
"#,
        "Smooth on sample pattern should fail",
        "smooth transform only works with numeric patterns",
    );
}

#[test]
fn test_smooth_basic() {
    // Test: smooth on oscillator pattern (numeric)
    test_compilation(
        r#"
tempo: 0.5
~lfo: sine 0.5
~smoothed: ~lfo $ smooth 0.3
out: saw 110 # lpf (~smoothed * 1000 + 500) 0.8
"#,
        "Smooth on oscillator pattern",
    );
}

#[test]
fn test_smooth_small_amount() {
    // Test: smooth with small amount
    test_compilation(
        r#"
tempo: 0.5
~lfo: sine 1.0
~smoothed: ~lfo $ smooth 0.1
out: saw 110 * ~smoothed
"#,
        "Smooth with small amount",
    );
}

#[test]
fn test_smooth_large_amount() {
    // Test: smooth with large amount
    test_compilation(
        r#"
tempo: 0.5
~lfo: sine 0.25
~smoothed: ~lfo $ smooth 0.9
out: saw 110 * ~smoothed
"#,
        "Smooth with large amount",
    );
}

// ========== Trim Tests ==========

#[test]
fn test_trim_basic() {
    // Test: trim to middle half
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ trim 0.25 0.75
"#,
        "Trim to 0.25-0.75",
    );
}

#[test]
fn test_trim_first_half() {
    // Test: trim to first half
    test_compilation(
        r#"
tempo: 0.5
out: "bd*8" $ trim 0.0 0.5
"#,
        "Trim to first half (0.0-0.5)",
    );
}

#[test]
fn test_trim_last_quarter() {
    // Test: trim to last quarter
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh*4" $ trim 0.75 1.0
"#,
        "Trim to last quarter (0.75-1.0)",
    );
}

#[test]
fn test_trim_small_range() {
    // Test: trim to small range
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ trim 0.4 0.6
"#,
        "Trim to small range (0.4-0.6)",
    );
}

#[test]
fn test_trim_with_effects() {
    // Test: trimmed pattern through delay
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh*2" $ trim 0.0 0.5 # delay 0.25 0.5 0.3
"#,
        "Trim with delay",
    );
}

#[test]
fn test_trim_combined() {
    // Test: trim combined with other transforms
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh" $ trim 0.25 0.75 $ fast 2
"#,
        "Trim combined with fast",
    );
}

// ========== Exp Tests (Numeric Patterns Only) ==========

#[test]
fn test_exp_on_sample_pattern_fails() {
    // Test: exp should fail on sample patterns
    test_compilation_error(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ exp 2.0
"#,
        "Exp on sample pattern should fail",
        "exp transform only works with numeric patterns",
    );
}

#[test]
fn test_exp_basic() {
    // Test: exp on oscillator pattern (numeric)
    test_compilation(
        r#"
tempo: 0.5
~lfo: sine 0.5
~exped: ~lfo $ exp 2.0
out: saw 110 # lpf (~exped * 1000 + 500) 0.8
"#,
        "Exp on oscillator pattern",
    );
}

#[test]
fn test_exp_different_bases() {
    // Test: exp with different bases
    test_compilation(
        r#"
tempo: 0.5
~lfo1: sine 1.0 $ exp 2.0
~lfo2: sine 0.5 $ exp 3.0
out: saw 110 * ~lfo1 # lpf (~lfo2 * 1000 + 500) 0.8
"#,
        "Exp with different bases",
    );
}

// ========== Log Tests (Numeric Patterns Only) ==========

#[test]
fn test_log_on_sample_pattern_fails() {
    // Test: log should fail on sample patterns
    test_compilation_error(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ log 2.0
"#,
        "Log on sample pattern should fail",
        "log transform only works with numeric patterns",
    );
}

#[test]
fn test_log_basic() {
    // Test: log on oscillator pattern (numeric)
    test_compilation(
        r#"
tempo: 0.5
~lfo: sine 0.5
~logged: ~lfo $ log 2.0
out: saw 110 # lpf (~logged * 1000 + 500) 0.8
"#,
        "Log on oscillator pattern",
    );
}

#[test]
fn test_log_different_bases() {
    // Test: log with different bases
    test_compilation(
        r#"
tempo: 0.5
~lfo1: sine 1.0 $ log 2.0
~lfo2: sine 0.5 $ log 10.0
out: saw 110 * ~lfo1 # lpf (~lfo2 * 1000 + 500) 0.8
"#,
        "Log with different bases",
    );
}

// ========== Walk Tests (Numeric Patterns Only) ==========

#[test]
fn test_walk_on_sample_pattern_fails() {
    // Test: walk should fail on sample patterns
    test_compilation_error(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ walk 0.5
"#,
        "Walk on sample pattern should fail",
        "walk transform only works with numeric patterns",
    );
}

#[test]
fn test_walk_basic() {
    // Test: walk on oscillator pattern (numeric)
    test_compilation(
        r#"
tempo: 0.5
~lfo: sine 0.5
~walked: ~lfo $ walk 0.1
out: saw 110 # lpf (~walked * 1000 + 500) 0.8
"#,
        "Walk on oscillator pattern",
    );
}

#[test]
fn test_walk_small_step() {
    // Test: walk with small step size
    test_compilation(
        r#"
tempo: 0.5
~lfo: sine 1.0
~walked: ~lfo $ walk 0.05
out: saw 110 * ~walked
"#,
        "Walk with small step size",
    );
}

#[test]
fn test_walk_large_step() {
    // Test: walk with large step size
    test_compilation(
        r#"
tempo: 0.5
~lfo: sine 0.25
~walked: ~lfo $ walk 0.3
out: saw 110 * ~walked
"#,
        "Walk with large step size",
    );
}

// ========== Combined Tests ==========

#[test]
fn test_all_six_operations_in_program() {
    // Test: using all six operations in same program
    test_compilation(
        r#"
tempo: 0.5
~focused: "bd*8" $ focus 0.0 2.0
~trimmed: "sn*4" $ trim 0.25 0.75
~lfo1: sine 0.5 $ smooth 0.3
~lfo2: sine 1.0 $ exp 2.0
~lfo3: sine 0.25 $ log 2.0
~lfo4: sine 0.5 $ walk 0.1
out: ~focused + ~trimmed
"#,
        "All six operations in one program",
    );
}

#[test]
fn test_focus_and_trim() {
    // Test: focus and trim together
    test_compilation(
        r#"
tempo: 0.5
~focused: "bd sn" $ focus 0.0 3.0
~trimmed: "hh cp" $ trim 0.0 0.5
out: ~focused + ~trimmed
"#,
        "Focus and trim in same program",
    );
}

#[test]
fn test_numeric_transforms_together() {
    // Test: smooth, exp, log, walk together
    test_compilation(
        r#"
tempo: 0.5
~lfo1: sine 0.5 $ smooth 0.2
~lfo2: sine 1.0 $ exp 2.0
~lfo3: sine 0.25 $ log 2.0
~lfo4: sine 0.5 $ walk 0.1
~saw_osc: saw 110 # lpf (~lfo1 * 1000 + 500) 0.8
out: ~saw_osc * (~lfo2 + ~lfo3 + ~lfo4)
"#,
        "Numeric transforms together",
    );
}

#[test]
fn test_focus_with_trim() {
    // Test: focus and trim on same pattern
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ focus 1.0 4.0 $ trim 0.25 0.75
"#,
        "Focus and trim on same pattern",
    );
}

#[test]
fn test_complex_combination() {
    // Test: complex combination of operations
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh cp" $ focus 0.0 4.0 $ trim 0.25 0.75 $ fast 2
"#,
        "Complex combination: focus, trim, fast",
    );
}

#[test]
fn test_with_effects_chain() {
    // Test: multiple operations with effects chain
    test_compilation(
        r#"
tempo: 0.5
out: "bd sn hh*4" $ focus 0.0 3.0 $ trim 0.0 0.5 # lpf 1000 0.8 # reverb 0.5 0.3 0.2
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
~kick: "bd*4" $ focus 0.0 2.0
~snare: "~ sn ~ sn" $ trim 0.25 0.75
~hats: "hh*8" $ focus 1.0 3.0 $ trim 0.0 0.5
~lfo_smooth: sine 0.5 $ smooth 0.3
~lfo_exp: sine 1.0 $ exp 2.0
~filtered: saw 110 # lpf (~lfo_smooth * 1000 + 500) 0.8
out: (~kick + ~snare) * 0.5 + ~hats * 0.3 + ~filtered * (~lfo_exp * 0.2)
"#,
        "Complex multi-bus program with all operations",
    );
}
