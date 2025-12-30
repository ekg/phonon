//! Tests for macro expansion in Phonon DSL
//!
//! Macros allow programmatic generation of DSL code:
//! - for loops to generate multiple buses
//! - indexed buses ~name[i]
//! - sum() to mix indexed buses
//! - arithmetic with loop variables

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::{parse_program, parse_program_with_macros};
use phonon::macro_expander::expand_macros;

// ========== Macro Expansion Tests ==========

#[test]
fn test_for_loop_basic() {
    let code = r#"
tempo: 2.0
for i in 1..3:
    ~osc[i] $ sine (110 * i)
out $ sum(~osc[1..3]) * 0.3
"#;

    let expanded = expand_macros(code);

    // Should expand to:
    // ~osc1 $ sine 110
    // ~osc2 $ sine 220
    // ~osc3 $ sine 330
    // out $ (~osc1 + ~osc2 + ~osc3) * 0.3

    assert!(expanded.contains("~osc1"));
    assert!(expanded.contains("~osc2"));
    assert!(expanded.contains("~osc3"));
    assert!(expanded.contains("sine 110"));
    assert!(expanded.contains("sine 220"));
    assert!(expanded.contains("sine 330"));
}

#[test]
fn test_for_loop_with_effects() {
    let code = r#"
tempo: 2.0
for i in 1..4:
    ~synth[i] $ saw (55 * i) # lpf (200 * i) 0.7
out $ sum(~synth[1..4]) * 0.2
"#;

    let expanded = expand_macros(code);

    assert!(expanded.contains("~synth1"));
    assert!(expanded.contains("~synth4"));
    assert!(expanded.contains("lpf 200"));   // 200 * 1
    assert!(expanded.contains("lpf 800"));   // 200 * 4
}

#[test]
fn test_sum_expansion() {
    let code = r#"
~a1 $ sine 110
~a2 $ sine 220
~a3 $ sine 330
out $ sum(~a[1..3])
"#;

    let expanded = expand_macros(code);

    // sum(~a[1..3]) should become (~a1 + ~a2 + ~a3)
    assert!(expanded.contains("(~a1 + ~a2 + ~a3)"));
}

#[test]
fn test_for_loop_compiles() {
    let code = r#"
tempo: 2.0
for i in 1..3:
    ~osc[i] $ sine (110 * i)
out $ sum(~osc[1..3]) * 0.3
"#;

    let expanded = expand_macros(code);
    let (_, statements) = parse_program(&expanded).expect("Parse failed");
    let result = compile_program(statements, 44100.0, None);

    assert!(result.is_ok(), "Expanded code should compile: {:?}", result.err());
}

#[test]
fn test_for_loop_renders_audio() {
    let code = r#"
tempo: 2.0
for i in 1..5:
    ~harm[i] $ sine (110 * i) * (1.0 / i)
out $ sum(~harm[1..5])
"#;

    let expanded = expand_macros(code);
    let (_, statements) = parse_program(&expanded).expect("Parse failed");
    let mut graph = compile_program(statements, 44100.0, None).expect("Compile failed");

    let buffer = graph.render(44100); // 1 second
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    assert!(rms > 0.01, "Should produce audio, got RMS: {}", rms);
}

#[test]
fn test_arithmetic_in_loop() {
    let code = r#"
for i in 0..3:
    ~v[i] $ sine (220 + i * 110)
out $ sum(~v[0..3])
"#;

    let expanded = expand_macros(code);

    assert!(expanded.contains("sine 220"));  // 220 + 0 * 110
    assert!(expanded.contains("sine 330"));  // 220 + 1 * 110
    assert!(expanded.contains("sine 440"));  // 220 + 2 * 110
}

#[test]
fn test_nested_arithmetic() {
    let code = r#"
for i in 1..3:
    ~s[i] $ sine ((110 * i) + 55)
out $ sum(~s[1..3])
"#;

    let expanded = expand_macros(code);

    assert!(expanded.contains("sine 165"));  // (110 * 1) + 55
    assert!(expanded.contains("sine 275"));  // (110 * 2) + 55
    assert!(expanded.contains("sine 385"));  // (110 * 3) + 55
}

#[test]
fn test_division_in_loop() {
    let code = r#"
for i in 1..4:
    ~h[i] $ sine 440 * (1.0 / i)
out $ sum(~h[1..4])
"#;

    let expanded = expand_macros(code);

    // Should have decreasing amplitudes
    assert!(expanded.contains("* 1"));      // 1.0 / 1
    assert!(expanded.contains("* 0.5"));    // 1.0 / 2
}

// ========== If/Else Tests ==========

#[test]
fn test_if_else_basic() {
    // For now, if/else could be compile-time evaluated
    let code = r#"
@mode: 1
~sound $ if @mode == 1 then saw 110 else sine 110
out $ ~sound
"#;

    let expanded = expand_macros(code);

    // With @mode = 1, should expand to saw 110
    assert!(expanded.contains("saw 110"));
}

// ========== Edge Cases ==========

#[test]
fn test_empty_loop_body() {
    let code = r#"
for i in 1..1:
    ~empty[i] $ sine 440
out $ sine 440
"#;

    let expanded = expand_macros(code);
    // Range 1..1 is empty, should produce no loop iterations
    // But code should still be valid
    let (_, statements) = parse_program(&expanded).expect("Parse failed");
    assert!(compile_program(statements, 44100.0, None).is_ok());
}

#[test]
fn test_passthrough_no_macros() {
    // Code without macros should pass through unchanged
    let code = r#"
tempo: 2.0
~bass $ saw 55 # lpf 400 0.8
out $ ~bass
"#;

    let expanded = expand_macros(code);
    let (_, statements) = parse_program(&expanded).expect("Parse failed");
    assert!(compile_program(statements, 44100.0, None).is_ok());
}

#[test]
fn test_multiple_for_loops() {
    let code = r#"
for i in 1..3:
    ~low[i] $ sine (55 * i)

for j in 1..3:
    ~high[j] $ sine (440 * j)

out $ sum(~low[1..3]) * 0.3 + sum(~high[1..3]) * 0.2
"#;

    let expanded = expand_macros(code);

    assert!(expanded.contains("~low1"));
    assert!(expanded.contains("~low3"));
    assert!(expanded.contains("~high1"));
    assert!(expanded.contains("~high3"));
}

// ========== Integrated Parser Tests ==========

#[test]
fn test_parse_program_with_macros_direct() {
    // Test the integrated parse_program_with_macros function
    let code = r#"
tempo: 2.0
for i in 1..4:
    ~voice[i] $ sine (110 * i) * (1.0 / i)
out $ sum(~voice[1..4])
"#;

    // Use the integrated function - no need to manually expand
    let (_, statements) = parse_program_with_macros(code).expect("Parse failed");
    let mut graph = compile_program(statements, 44100.0, None).expect("Compile failed");

    let buffer = graph.render(44100); // 1 second
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    assert!(rms > 0.01, "Should produce audio, got RMS: {}", rms);
}

#[test]
fn test_harmonic_series_with_macros() {
    // Generate a harmonic series using macros
    let code = r#"
tempo: 2.0
-- Generate first 4 harmonics with decreasing amplitude
for n in 1..4:
    ~h[n] $ sine (220 * n) * (1.0 / n)
out $ sum(~h[1..4]) * 0.3
"#;

    let (_, statements) = parse_program_with_macros(code).expect("Parse failed");
    let mut graph = compile_program(statements, 44100.0, None).expect("Compile failed");

    let buffer = graph.render(44100);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    assert!(rms > 0.01, "Harmonic series should produce audio, got RMS: {}", rms);
}
