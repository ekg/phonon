//! Tests for Tidal-style pattern structure operators (|+, +|, |- etc.)
//!
//! These operators determine which pattern provides the STRUCTURE (timing/rhythm)
//! and which provides VALUES (sampled at structure event times).

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

/// Helper to compile and render Phonon code
fn render_phonon_code(code: &str, num_samples: usize) -> Vec<f32> {
    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(1.0); // 1 cycle per second

    graph.render(num_samples)
}

/// Helper to calculate RMS of a buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt()
}

#[test]
fn test_add_left_compiles_and_renders() {
    // "1 2 3" |+ "10" should produce 3 events per cycle with values 11, 12, 13
    let code = r#"
tempo: 1.0
~result $ "1 2 3" |+ "10"
out $ sine (~result * 10)
"#;

    let buffer = render_phonon_code(code, 44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Structure operator should produce audio, got RMS {}",
        rms
    );

    println!("|+ operator: RMS = {:.4}", rms);
}

#[test]
fn test_add_right_compiles_and_renders() {
    // "1 2 3" +| "10 20" should produce 2 events per cycle with values 11, 22
    let code = r#"
tempo: 1.0
~result $ "1 2 3" +| "10 20"
out $ sine (~result * 10)
"#;

    let buffer = render_phonon_code(code, 44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Structure operator should produce audio, got RMS {}",
        rms
    );

    println!("+| operator: RMS = {:.4}", rms);
}

#[test]
fn test_mul_left_compiles_and_renders() {
    // "2 3 4" |* "10" should produce 3 events with values 20, 30, 40
    let code = r#"
tempo: 1.0
~result $ "2 3 4" |* "10"
out $ sine ~result
"#;

    let buffer = render_phonon_code(code, 44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Structure operator should produce audio, got RMS {}",
        rms
    );

    println!("|* operator: RMS = {:.4}", rms);
}

#[test]
fn test_union_left_compiles_and_renders() {
    // "x x x" |> "100 200" should produce 3 events with values 100, 100, 200
    let code = r#"
tempo: 1.0
~result $ "1 2 3" |> "100 200"
out $ sine ~result
"#;

    let buffer = render_phonon_code(code, 44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Union left operator should produce audio, got RMS {}",
        rms
    );

    println!("|> operator: RMS = {:.4}", rms);
}

#[test]
fn test_union_right_compiles_and_renders() {
    // "100 200" <| "1 2 3" should produce 3 events with values 100, 100, 200
    let code = r#"
tempo: 1.0
~result $ "100 200" <| "1 2 3"
out $ sine ~result
"#;

    let buffer = render_phonon_code(code, 44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Union right operator should produce audio, got RMS {}",
        rms
    );

    println!("<| operator: RMS = {:.4}", rms);
}

#[test]
fn test_structure_affects_event_count() {
    // Left structure with 4 events vs right structure with 2 events
    // Should produce different rhythmic patterns

    // 4-event structure
    let code_left = r#"
tempo: 1.0
~result $ "100 200 300 400" |+ "10 20"
out $ sine ~result * 0.3
"#;

    // 2-event structure
    let code_right = r#"
tempo: 1.0
~result $ "100 200 300 400" +| "10 20"
out $ sine ~result * 0.3
"#;

    let buffer_left = render_phonon_code(code_left, 44100);
    let buffer_right = render_phonon_code(code_right, 44100);

    // Both should produce sound
    let rms_left = calculate_rms(&buffer_left);
    let rms_right = calculate_rms(&buffer_right);

    assert!(rms_left > 0.01, "Left structure should produce audio");
    assert!(rms_right > 0.01, "Right structure should produce audio");

    // The signals should be different because they have different structures
    // (4 events vs 2 events per cycle)
    let diff: f32 = buffer_left
        .iter()
        .zip(buffer_right.iter())
        .map(|(a, b)| (a - b).abs())
        .sum::<f32>()
        / buffer_left.len() as f32;

    assert!(
        diff > 0.001,
        "Different structures should produce different audio, diff = {}",
        diff
    );

    println!(
        "Structure comparison: left RMS = {:.4}, right RMS = {:.4}, avg diff = {:.4}",
        rms_left, rms_right, diff
    );
}

#[test]
fn test_nested_structure_operators() {
    // Test that structure operators can be nested
    let code = r#"
tempo: 1.0
~a $ "1 2" |+ "10"
~b $ ~a |* "2 3"
out $ sine (~b * 10)
"#;

    let buffer = render_phonon_code(code, 44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Nested structure operators should produce audio, got RMS {}",
        rms
    );

    println!("Nested operators: RMS = {:.4}", rms);
}

#[test]
fn test_structure_with_number_literal() {
    // Structure operators should work with number literals
    let code = r#"
tempo: 1.0
~result $ "100 200 300" |+ 10
out $ sine ~result
"#;

    let buffer = render_phonon_code(code, 44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Structure operator with literal should produce audio, got RMS {}",
        rms
    );

    println!("Pattern |+ literal: RMS = {:.4}", rms);
}

#[test]
fn test_all_arithmetic_structure_operators() {
    // Test all 8 arithmetic structure operators compile and run
    let operators = vec![
        ("|+", "add left"),
        ("+|", "add right"),
        ("|-", "sub left"),
        ("-|", "sub right"),
        ("|*", "mul left"),
        ("*|", "mul right"),
        ("|/", "div left"),
        ("/|", "div right"),
    ];

    for (op, name) in operators {
        let code = format!(
            r#"
tempo: 1.0
~result $ "100 200 300" {} "2 3"
out $ sine ~result * 0.3
"#,
            op
        );

        let buffer = render_phonon_code(&code, 22050); // Half second

        let rms = calculate_rms(&buffer);
        assert!(
            rms > 0.001,
            "{} operator ({}) should produce audio, got RMS {}",
            name,
            op,
            rms
        );

        println!("{} ({}): RMS = {:.4}", name, op, rms);
    }
}
