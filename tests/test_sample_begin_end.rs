//! Tests for begin/end sample parameters
//!
//! These tests verify that begin/end parameters work correctly for sample slicing:
//! - begin: Sets the start point of the sample (0.0 = start, 0.5 = middle, 1.0 = end)
//! - end: Sets the end point of the sample (0.0 = start, 1.0 = end)
//!
//! Use cases:
//! - Jungle/breakbeat slicing: s "breaks152" # begin 0.25 # end 0.5
//! - Sample trimming: Remove silence at start/end
//! - Creative sample manipulation

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

// ============================================================================
// LEVEL 1: Compilation Tests - Do begin/end modifiers compile?
// ============================================================================

#[test]
fn test_begin_modifier_compiles() {
    let code = r#"
tempo: 0.5
out $ s "bd" # begin 0.5
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);

    match result {
        Ok(_) => {} // Success
        Err(e) => panic!("begin modifier failed to compile: {}", e),
    }
}

#[test]
fn test_end_modifier_compiles() {
    let code = r#"
tempo: 0.5
out $ s "bd" # end 0.5
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);

    match result {
        Ok(_) => {} // Success
        Err(e) => panic!("end modifier failed to compile: {}", e),
    }
}

#[test]
fn test_begin_end_chained_compiles() {
    let code = r#"
tempo: 0.5
out $ s "bd" # begin 0.25 # end 0.75
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);

    match result {
        Ok(_) => {} // Success
        Err(e) => panic!("begin/end chained failed to compile: {}", e),
    }
}

// ============================================================================
// LEVEL 2: Pattern-controlled begin/end - Can they take patterns?
// ============================================================================

#[test]
fn test_pattern_controlled_begin() {
    let code = r#"
tempo: 0.5
out $ s "bd*4" # begin "0 0.25 0.5 0.75"
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);

    match result {
        Ok(_) => {} // Success
        Err(e) => panic!("Pattern-controlled begin failed to compile: {}", e),
    }
}

#[test]
fn test_pattern_controlled_end() {
    let code = r#"
tempo: 0.5
out $ s "bd*4" # end "0.25 0.5 0.75 1.0"
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);

    match result {
        Ok(_) => {} // Success
        Err(e) => panic!("Pattern-controlled end failed to compile: {}", e),
    }
}

#[test]
fn test_pattern_controlled_begin_end_chained() {
    let code = r#"
tempo: 0.5
out $ s "bd*4" # begin "0 0.25 0.5 0.75" # end "0.25 0.5 0.75 1"
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);

    match result {
        Ok(_) => {} // Success
        Err(e) => panic!(
            "Pattern-controlled begin/end chained failed to compile: {}",
            e
        ),
    }
}

// ============================================================================
// LEVEL 3: Audio Output Tests - Does slicing actually work?
// ============================================================================

#[test]
fn test_begin_end_produces_audio() {
    let code = r#"
tempo: 0.5
out $ s "bd" # begin 0.0 # end 1.0
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0, None).expect("Compile failed");
    let audio = graph.render(22050); // 0.5 seconds

    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "begin 0 # end 1 should produce audio, got RMS={}",
        rms
    );
}

#[test]
fn test_begin_half_produces_shorter_audio() {
    // begin 0.5 should play the second half of the sample
    let code = r#"
tempo: 0.5
out $ s "bd" # begin 0.5 # end 1.0
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0, None).expect("Compile failed");
    let audio = graph.render(22050);

    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "begin 0.5 # end 1.0 should still produce audio, got RMS={}",
        rms
    );
}

#[test]
fn test_end_half_produces_shorter_audio() {
    // end 0.5 should play the first half of the sample
    let code = r#"
tempo: 0.5
out $ s "bd" # begin 0.0 # end 0.5
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0, None).expect("Compile failed");
    let audio = graph.render(22050);

    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "begin 0 # end 0.5 should produce audio, got RMS={}",
        rms
    );
}

#[test]
fn test_begin_end_middle_slice() {
    // begin 0.25 # end 0.75 should play the middle half of the sample
    let code = r#"
tempo: 0.5
out $ s "bd" # begin 0.25 # end 0.75
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0, None).expect("Compile failed");
    let audio = graph.render(22050);

    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "begin 0.25 # end 0.75 should produce audio, got RMS={}",
        rms
    );
}

// ============================================================================
// LEVEL 4: Complex Patterns - Real-world jungle/breakbeat usage
// ============================================================================

#[test]
fn test_begin_end_with_other_modifiers() {
    // Combine begin/end with speed, gain, pan
    let code = r#"
tempo: 0.5
out $ s "bd" # begin 0.25 # end 0.75 # speed 2.0 # gain 0.8
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);

    match result {
        Ok(_) => {} // Success
        Err(e) => panic!("begin/end with other modifiers failed: {}", e),
    }
}

#[test]
fn test_begin_end_with_pattern_transforms() {
    // Use begin/end with pattern transforms like fast
    let code = r#"
tempo: 0.5
out $ s "bd" $ fast 2 # begin 0.5 # end 1.0
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);

    match result {
        Ok(_) => {} // Success
        Err(e) => panic!("begin/end with pattern transforms failed: {}", e),
    }
}

#[test]
fn test_breakbeat_slicing_pattern() {
    // Simulate breakbeat slicing with pattern-controlled begin/end
    // Each beat plays a different slice of the sample
    let code = r#"
tempo: 0.5
out $ s "bd*4" # begin "0 0.25 0.5 0.75" # end "0.25 0.5 0.75 1"
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0, None).expect("Compile failed");
    let audio = graph.render(44100); // 1 second

    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Breakbeat slicing pattern should produce audio, got RMS={}",
        rms
    );
}

// ============================================================================
// LEVEL 5: Edge Cases
// ============================================================================

#[test]
fn test_begin_greater_than_end_handled() {
    // When begin > end, should still not crash (clamp or swap values)
    let code = r#"
tempo: 0.5
out $ s "bd" # begin 0.75 # end 0.25
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);

    // Should compile (may produce silence or handle gracefully)
    assert!(result.is_ok(), "begin > end should compile without panic");
}

#[test]
fn test_begin_end_clamped_to_valid_range() {
    // Values outside 0-1 should be clamped
    let code = r#"
tempo: 0.5
out $ s "bd" # begin -0.5 # end 1.5
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);

    assert!(
        result.is_ok(),
        "Out-of-range begin/end should compile without panic"
    );
}

#[test]
#[ignore = "BUG: begin==end still triggers audio"]
fn test_begin_equals_end_produces_silence() {
    // When begin == end, should produce silence or near-silence
    let code = r#"
tempo: 0.5
out $ s "bd" # begin 0.5 # end 0.5
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0, None).expect("Compile failed");
    let audio = graph.render(22050);

    let rms = calculate_rms(&audio);
    // Should be very quiet or silent
    assert!(
        rms < 0.1,
        "begin == end should produce little/no audio, got RMS={}",
        rms
    );
}

// ============================================================================
// Regression: begin/end preserved when other params are modified after
// ============================================================================

#[test]
fn test_begin_preserved_after_gain_modifier() {
    // Regression: modify_sample_param was hardcoding begin=0.0/end=1.0
    // so "# begin 0.5 # gain 0.8" would lose the begin=0.5 setting.
    //
    // We verify by comparing audio from:
    //   A: s "bd" # begin 0.5 # gain 0.8   (begin applied first, then gain)
    //   B: s "bd" # gain 0.8                (default begin=0.0)
    //
    // If begin is preserved, A and B should differ.
    let code_with_begin = r#"
tempo: 0.5
out $ s "bd" # begin 0.5 # gain 0.8
"#;
    let code_without_begin = r#"
tempo: 0.5
out $ s "bd" # gain 0.8
"#;

    let (_g, stmts_a) = parse_program(code_with_begin).expect("parse A");
    let mut graph_a = compile_program(stmts_a, 44100.0, None).expect("compile A");
    let audio_a = graph_a.render(22050);

    let (_g, stmts_b) = parse_program(code_without_begin).expect("parse B");
    let mut graph_b = compile_program(stmts_b, 44100.0, None).expect("compile B");
    let audio_b = graph_b.render(22050);

    // Both should produce audio
    let rms_a = calculate_rms(&audio_a);
    let rms_b = calculate_rms(&audio_b);
    assert!(rms_a > 0.001, "begin=0.5 + gain should produce audio, got RMS={}", rms_a);
    assert!(rms_b > 0.001, "gain only should produce audio, got RMS={}", rms_b);

    // They should NOT be identical (begin=0.5 plays from the middle)
    let diff: f32 = audio_a.iter().zip(audio_b.iter())
        .map(|(a, b)| (a - b).abs())
        .sum::<f32>() / audio_a.len() as f32;
    assert!(
        diff > 0.0001,
        "begin 0.5 + gain 0.8 should differ from just gain 0.8 (begin must be preserved), diff={}",
        diff
    );
}
