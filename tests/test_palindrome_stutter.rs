/// Tests for stut (3-arg stutter/echo) transform
///
/// stut n time decay - Creates n echoes with time delay and decay
/// Example: stut 3 0.125 0.7 creates original + 2 echoes at 70%, 49% volume

use phonon::compositional_parser::parse_program;
use phonon::compositional_compiler::compile_program;

/// Test stut (3-arg Tidal-style echo/stutter) with explicit frequency
#[test]
fn test_stut_with_freq() {
    let code = r#"
cps: 2.0
~synth $ stut 2 0.27 0.9 $ sine 261.63
out $ ~synth * 0.3
    "#;

    let (_, statements) = parse_program(code).expect("Should parse");
    let mut graph = compile_program(statements, 44100.0, None).expect("Should compile");

    let buffer = graph.render(44100);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    println!("Stut with freq RMS: {}", rms);
    assert!(rms > 0.0001, "Stut with freq should produce audio, got RMS: {}", rms);
}

/// Test stut with sample playback - the most common use case
#[test]
fn test_stut_with_samples() {
    let code = r#"
cps: 2.0
~drums $ stut 3 0.125 0.7 $ s "bd sn"
out $ ~drums * 0.5
    "#;

    let (_, statements) = parse_program(code).expect("Should parse");
    let mut graph = compile_program(statements, 44100.0, None).expect("Should compile");

    let buffer = graph.render(44100);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    println!("Stut with samples RMS: {}", rms);
    assert!(rms > 0.0001, "Stut with samples should produce audio, got RMS: {}", rms);
}

/// Test stut parsing compiles correctly
#[test]
fn test_stut_parsing() {
    let code = r#"
cps: 2.0
~pattern $ stut 4 0.1 0.8 $ s "bd cp"
out $ ~pattern
    "#;

    let (_, statements) = parse_program(code).expect("Should parse stut");
    let result = compile_program(statements, 44100.0, None);
    assert!(result.is_ok(), "Stut should compile: {:?}", result.err());
}

/// Test stut with saw wave
#[test]
fn test_stut_with_saw() {
    let code = r#"
cps: 2.0
~synth $ stut 3 0.2 0.6 $ saw 110
out $ ~synth * 0.3
    "#;

    let (_, statements) = parse_program(code).expect("Should parse");
    let mut graph = compile_program(statements, 44100.0, None).expect("Should compile");

    let buffer = graph.render(44100);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    println!("Stut with saw RMS: {}", rms);
    assert!(rms > 0.0001, "Stut with saw should produce audio, got RMS: {}", rms);
}
