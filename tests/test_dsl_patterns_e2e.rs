use std::fs;
/// End-to-end tests for pattern DSL syntax
/// Tests mini-notation, transformations, and pattern operations
use std::process::Command;

fn render_and_verify(dsl_code: &str, test_name: &str) -> (bool, String) {
    let ph_path = format!("/tmp/test_pattern_{}.ph", test_name);
    let wav_path = format!("/tmp/test_pattern_{}.wav", test_name);

    fs::write(&ph_path, dsl_code).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "phonon",
            "--quiet",
            "--",
            "render",
            &ph_path,
            &wav_path,
            "--duration",
            "2",
        ])
        .output()
        .expect("Failed to run phonon render");

    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (success, stderr)
}

// ============================================================================
// BASIC NUMBER PATTERN TESTS
// ============================================================================

#[test]
fn test_two_value_pattern() {
    let dsl = r#"
tempo: 0.5
out: sine "220 440" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "two_values");
    assert!(success, "Failed to render 2-value pattern: {}", stderr);
}

#[test]
fn test_four_value_pattern() {
    let dsl = r#"
tempo: 0.5
out: sine "110 220 330 440" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "four_values");
    assert!(success, "Failed to render 4-value pattern: {}", stderr);
}

#[test]
fn test_eight_value_pattern() {
    let dsl = r#"
tempo: 0.5
out: sine "110 165 220 275 330 385 440 495" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "eight_values");
    assert!(success, "Failed to render 8-value pattern: {}", stderr);
}

#[test]
fn test_single_value_pattern() {
    let dsl = r#"
tempo: 0.5
out: sine "440" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "single_value");
    assert!(success, "Failed to render single-value pattern: {}", stderr);
}

#[test]
fn test_three_value_pattern() {
    let dsl = r#"
tempo: 0.5
out: sine "220 330 440" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "three_values");
    assert!(success, "Failed to render 3-value pattern: {}", stderr);
}

// ============================================================================
// REST AND SILENCE TESTS
// ============================================================================

#[test]
fn test_pattern_with_rest() {
    let dsl = r#"
tempo: 0.5
out: sine "440 ~ 330 ~" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "with_rest");
    assert!(success, "Failed to render pattern with rests: {}", stderr);
}

#[test]
fn test_pattern_alternating_rest() {
    let dsl = r#"
tempo: 0.5
out: sine "220 ~ 440 ~ 660 ~" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "alternating_rest");
    assert!(
        success,
        "Failed to render alternating rest pattern: {}",
        stderr
    );
}

#[test]
fn test_pattern_multiple_consecutive_rests() {
    let dsl = r#"
tempo: 0.5
out: sine "440 ~ ~ 220" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "consecutive_rests");
    assert!(success, "Failed to render consecutive rests: {}", stderr);
}

// ============================================================================
// SUBDIVISION TESTS - Using * operator
// ============================================================================

#[test]
fn test_subdivision_2x() {
    let dsl = r#"
tempo: 0.5
out: sine "220 440*2" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "subdiv_2x");
    assert!(success, "Failed to render 2x subdivision: {}", stderr);
}

#[test]
fn test_subdivision_4x() {
    let dsl = r#"
tempo: 0.5
out: sine "220 330*4" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "subdiv_4x");
    assert!(success, "Failed to render 4x subdivision: {}", stderr);
}

#[test]
fn test_subdivision_8x() {
    let dsl = r#"
tempo: 0.5
out: sine "220 440*8" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "subdiv_8x");
    assert!(success, "Failed to render 8x subdivision: {}", stderr);
}

#[test]
fn test_multiple_subdivisions() {
    let dsl = r#"
tempo: 0.5
out: sine "220*2 330*3 440*4" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "multi_subdiv");
    assert!(
        success,
        "Failed to render multiple subdivisions: {}",
        stderr
    );
}

// ============================================================================
// ALTERNATION TESTS - Using < > brackets
// ============================================================================

#[test]
fn test_alternation_two_choices() {
    let dsl = r#"
tempo: 0.5
out: sine "<220 440>" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "alt_two");
    assert!(success, "Failed to render 2-choice alternation: {}", stderr);
}

#[test]
fn test_alternation_three_choices() {
    let dsl = r#"
tempo: 0.5
out: sine "<220 330 440>" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "alt_three");
    assert!(success, "Failed to render 3-choice alternation: {}", stderr);
}

#[test]
fn test_alternation_four_choices() {
    let dsl = r#"
tempo: 0.5
out: sine "<110 220 330 440>" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "alt_four");
    assert!(success, "Failed to render 4-choice alternation: {}", stderr);
}

#[test]
fn test_alternation_mixed_with_values() {
    let dsl = r#"
tempo: 0.5
out: sine "220 <330 440> 550" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "alt_mixed");
    assert!(success, "Failed to render mixed alternation: {}", stderr);
}

// ============================================================================
// EUCLIDEAN RHYTHM TESTS - Using (steps, pulses) syntax
// ============================================================================

#[test]
fn test_euclidean_3_8() {
    let dsl = r#"
tempo: 0.5
out: sine "(3,8,440)" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "euclid_3_8");
    assert!(success, "Failed to render euclidean (3,8): {}", stderr);
}

#[test]
fn test_euclidean_5_8() {
    let dsl = r#"
tempo: 0.5
out: sine "(5,8,440)" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "euclid_5_8");
    assert!(success, "Failed to render euclidean (5,8): {}", stderr);
}

#[test]
fn test_euclidean_3_4() {
    let dsl = r#"
tempo: 0.5
out: sine "(3,4,440)" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "euclid_3_4");
    assert!(success, "Failed to render euclidean (3,4): {}", stderr);
}

#[test]
fn test_euclidean_7_16() {
    let dsl = r#"
tempo: 0.5
out: sine "(7,16,440)" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "euclid_7_16");
    assert!(success, "Failed to render euclidean (7,16): {}", stderr);
}

// ============================================================================
// PATTERN TRANSFORMATION TESTS - Using $ operator
// ============================================================================

#[test]
fn test_fast_transform() {
    let dsl = r#"
tempo: 0.5
~base: sine "220 440"
out: (~base $ fast 2) * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "fast");
    assert!(success, "Failed to apply fast transform: {}", stderr);
}

#[test]
fn test_slow_transform() {
    let dsl = r#"
tempo: 0.5
~base: sine "220 440 330 550"
out: (~base $ slow 2) * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "slow");
    assert!(success, "Failed to apply slow transform: {}", stderr);
}

#[test]
fn test_rev_transform() {
    let dsl = r#"
tempo: 0.5
~base: sine "220 330 440 550"
out: (~base $ rev) * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "rev");
    assert!(success, "Failed to apply rev transform: {}", stderr);
}

#[test]
fn test_every_transform() {
    let dsl = r#"
tempo: 0.5
~base: sine "220 440"
out: (~base $ every 2 rev) * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "every");
    assert!(success, "Failed to apply every transform: {}", stderr);
}

#[test]
fn test_chained_transforms() {
    let dsl = r#"
tempo: 0.5
~base: sine "220 330 440"
out: (~base $ fast 2 $ rev) * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "chained");
    assert!(success, "Failed to chain transforms: {}", stderr);
}

// ============================================================================
// PATTERN ARITHMETIC TESTS
// ============================================================================

#[test]
fn test_pattern_addition() {
    let dsl = r#"
tempo: 0.5
~p1: sine "220 440"
~p2: sine "330 550"
out: (~p1 + ~p2) * 0.1
"#;
    let (success, stderr) = render_and_verify(dsl, "add_patterns");
    assert!(success, "Failed to add patterns: {}", stderr);
}

#[test]
fn test_pattern_multiplication() {
    let dsl = r#"
tempo: 0.5
~p1: sine "220 440"
~amp: "0.1 0.3 0.2 0.4"
out: ~p1 * ~amp
"#;
    let (success, stderr) = render_and_verify(dsl, "mult_patterns");
    assert!(success, "Failed to multiply patterns: {}", stderr);
}

#[test]
fn test_pattern_scaling() {
    let dsl = r#"
tempo: 0.5
~base: "100 200 300"
out: sine (~base * 2) * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "scale_pattern");
    assert!(success, "Failed to scale pattern: {}", stderr);
}

#[test]
fn test_pattern_offset() {
    let dsl = r#"
tempo: 0.5
~base: "100 200 300"
out: sine (~base + 220) * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "offset_pattern");
    assert!(success, "Failed to offset pattern: {}", stderr);
}

// ============================================================================
// COMPLEX MINI-NOTATION TESTS
// ============================================================================

#[test]
fn test_subdivision_with_alternation() {
    let dsl = r#"
tempo: 0.5
out: sine "220 <330 440>*2" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "subdiv_alt");
    assert!(success, "Failed with subdivision + alternation: {}", stderr);
}

#[test]
fn test_rests_with_subdivision() {
    let dsl = r#"
tempo: 0.5
out: sine "220 ~*2 440" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "rest_subdiv");
    assert!(success, "Failed with rests + subdivision: {}", stderr);
}

#[test]
fn test_alternation_with_rests() {
    let dsl = r#"
tempo: 0.5
out: sine "<220 ~ 440>" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "alt_rest");
    assert!(success, "Failed with alternation + rests: {}", stderr);
}

#[test]
fn test_nested_subdivisions() {
    let dsl = r#"
tempo: 0.5
out: sine "220 [330 440]*2" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "nested_subdiv");
    assert!(success, "Failed with nested subdivisions: {}", stderr);
}

// ============================================================================
// PATTERN AS CONTROL SIGNAL TESTS - Unique to Phonon!
// ============================================================================

#[test]
fn test_pattern_controls_filter_cutoff() {
    let dsl = r#"
tempo: 0.5
~cutoff: "500 1000 1500 2000"
~bass: saw 55 # lpf ~cutoff 0.8
out: ~bass * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "pattern_cutoff");
    assert!(success, "Failed pattern-controlled cutoff: {}", stderr);
}

#[test]
fn test_pattern_controls_resonance() {
    let dsl = r#"
tempo: 0.5
~res: "0.3 0.6 0.8 0.5"
~bass: saw 55 # lpf 1500 ~res
out: ~bass * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "pattern_res");
    assert!(success, "Failed pattern-controlled resonance: {}", stderr);
}

#[test]
fn test_pattern_controls_amplitude() {
    let dsl = r#"
tempo: 0.5
~amp: "0.1 0.3 0.2 0.4"
out: sine 440 * ~amp
"#;
    let (success, stderr) = render_and_verify(dsl, "pattern_amp");
    assert!(success, "Failed pattern-controlled amplitude: {}", stderr);
}

#[test]
fn test_multiple_pattern_params() {
    let dsl = r#"
tempo: 0.5
~freq: "220 330 440"
~cutoff: "1000 2000 3000"
~osc: saw ~freq # lpf ~cutoff 0.7
out: ~osc * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "multi_pattern_params");
    assert!(success, "Failed multiple pattern parameters: {}", stderr);
}

// ============================================================================
// DIFFERENT PATTERN LENGTHS - Polyrhythm
// ============================================================================

#[test]
fn test_2_against_3_pattern() {
    let dsl = r#"
tempo: 0.5
~p1: sine "220 440"
~p2: sine "330 550 660"
out: (~p1 + ~p2) * 0.1
"#;
    let (success, stderr) = render_and_verify(dsl, "2_vs_3");
    assert!(success, "Failed 2 vs 3 pattern: {}", stderr);
}

#[test]
fn test_3_against_4_pattern() {
    let dsl = r#"
tempo: 0.5
~p1: sine "220 330 440"
~p2: sine "110 165 220 275"
out: (~p1 + ~p2) * 0.1
"#;
    let (success, stderr) = render_and_verify(dsl, "3_vs_4");
    assert!(success, "Failed 3 vs 4 pattern: {}", stderr);
}

#[test]
fn test_4_against_8_pattern() {
    let dsl = r#"
tempo: 0.5
~p1: sine "220 440 330 550"
~p2: sine "110 165 220 275 330 385 440 495"
out: (~p1 + ~p2) * 0.1
"#;
    let (success, stderr) = render_and_verify(dsl, "4_vs_8");
    assert!(success, "Failed 4 vs 8 pattern: {}", stderr);
}

// ============================================================================
// TEMPO VARIATION TESTS
// ============================================================================

#[test]
fn test_pattern_slow_tempo() {
    let dsl = r#"
tempo: 0.5
out: sine "220 440 330 550" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "slow_tempo");
    assert!(success, "Failed pattern at slow tempo: {}", stderr);
}

#[test]
fn test_pattern_fast_tempo() {
    let dsl = r#"
tempo: 4.0
out: sine "220 440" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "fast_tempo");
    assert!(success, "Failed pattern at fast tempo: {}", stderr);
}

// ============================================================================
// NUMERIC RANGE TESTS
// ============================================================================

#[test]
fn test_pattern_low_values() {
    let dsl = r#"
tempo: 0.5
out: sine "55 82.5 110" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "low_values");
    assert!(success, "Failed pattern with low values: {}", stderr);
}

#[test]
fn test_pattern_high_values() {
    let dsl = r#"
tempo: 0.5
out: sine "1760 2200 2640" * 0.1
"#;
    let (success, stderr) = render_and_verify(dsl, "high_values");
    assert!(success, "Failed pattern with high values: {}", stderr);
}

#[test]
fn test_pattern_decimal_values() {
    let dsl = r#"
tempo: 0.5
out: sine "220.5 440.25 330.75" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "decimal_values");
    assert!(success, "Failed pattern with decimal values: {}", stderr);
}

#[test]
fn test_pattern_zero_value() {
    let dsl = r#"
tempo: 0.5
out: sine "0 220 440" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "zero_value");
    assert!(success, "Failed pattern with zero: {}", stderr);
}

// ============================================================================
// PATTERN IN DIFFERENT CONTEXTS
// ============================================================================

#[test]
fn test_pattern_as_lfo_freq() {
    let dsl = r#"
tempo: 0.5
~lfo: sine "0.5 1 2" * 0.5 + 0.5
~carrier: sine 440
out: ~carrier * ~lfo * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "pattern_lfo_freq");
    assert!(success, "Failed pattern as LFO frequency: {}", stderr);
}

#[test]
fn test_pattern_drives_fm_modulator() {
    let dsl = r#"
tempo: 0.5
~mod_freq: "55 110 165"
~mod: sine ~mod_freq * 200
~carrier: sine (440 + ~mod)
out: ~carrier * 0.1
"#;
    let (success, stderr) = render_and_verify(dsl, "pattern_fm");
    assert!(success, "Failed pattern-driven FM: {}", stderr);
}

#[test]
fn test_pattern_in_arithmetic() {
    let dsl = r#"
tempo: 0.5
~base: "100 200 300"
out: sine (~base * 2 + 100) * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "pattern_math");
    assert!(success, "Failed pattern in arithmetic: {}", stderr);
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

#[test]
fn test_very_long_pattern() {
    let dsl = r#"
tempo: 0.5
out: sine "110 165 220 275 330 385 440 495 550 605 660 715 770 825 880 935" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "long_pattern");
    assert!(success, "Failed with very long pattern: {}", stderr);
}

#[test]
fn test_pattern_all_same_value() {
    let dsl = r#"
tempo: 0.5
out: sine "440 440 440 440" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "all_same");
    assert!(success, "Failed pattern with all same values: {}", stderr);
}

#[test]
fn test_pattern_with_very_fast_subdivision() {
    let dsl = r#"
tempo: 0.5
out: sine "220 440*16" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "fast_subdiv");
    assert!(success, "Failed with very fast subdivision: {}", stderr);
}
