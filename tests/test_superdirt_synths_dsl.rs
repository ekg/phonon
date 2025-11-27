//! Test SuperDirt synths accessible from DSL

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

// ========== SuperKick Tests ==========

#[test]
fn test_superkick_basic() {
    let code = r#"
tempo: 2.0
out: superkick 60
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    // Render 100ms
    let buffer = graph.render(4410);

    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    eprintln!("SuperKick RMS: {}", rms);

    assert!(rms > 0.01, "SuperKick should produce audio");
}

#[test]
fn test_superkick_with_params() {
    let code = r#"
tempo: 2.0
out: superkick 60 0.8 0.4 0.2
"#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(4410);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    assert!(rms > 0.01, "SuperKick with params should produce audio");
}

// ========== SuperSaw Tests ==========

#[test]
fn test_supersaw_basic() {
    let code = r#"
tempo: 2.0
out: supersaw 220
"#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    // Render 1 second for stable RMS
    let buffer = graph.render(44100);

    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    eprintln!("SuperSaw RMS: {}", rms);

    assert!(rms > 0.1, "SuperSaw should produce audio, got RMS={}", rms);
}

#[test]
fn test_supersaw_with_params() {
    let code = r#"
tempo: 2.0
out: supersaw 110 0.5 5
"#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(44100);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    assert!(rms > 0.1, "SuperSaw with params should produce audio");
}

// ========== SuperPWM Tests ==========

#[test]
fn test_superpwm_basic() {
    let code = r#"
tempo: 2.0
out: superpwm 110
"#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(44100);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    assert!(rms > 0.3, "SuperPWM should produce strong audio");
}

// ========== SuperChip Tests ==========

#[test]
fn test_superchip_basic() {
    let code = r#"
tempo: 2.0
out: superchip 440
"#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(44100);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    assert!(rms > 0.5, "SuperChip should produce strong audio");
}

// ========== SuperFM Tests ==========

#[test]
fn test_superfm_basic() {
    let code = r#"
tempo: 2.0
out: superfm 440
"#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(4410);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    assert!(rms > 0.1, "SuperFM should produce audio");
}

#[test]
fn test_superfm_with_params() {
    let code = r#"
tempo: 2.0
out: superfm 440 2.0 1.0
"#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(4410);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    assert!(rms > 0.1, "SuperFM with params should produce audio");
}

// ========== SuperSnare Tests ==========

#[test]
fn test_supersnare_basic() {
    let code = r#"
tempo: 2.0
out: supersnare 200
"#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(2205); // 50ms
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    assert!(rms > 0.01, "SuperSnare should produce audio");
}

// ========== SuperHat Tests ==========

#[test]
fn test_superhat_basic() {
    let code = r#"
tempo: 2.0
out: superhat 0.7 0.05
"#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(2205); // 50ms
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    assert!(rms > 0.01, "SuperHat should produce audio");
}

// ========== Combined Tests ==========

#[test]
fn test_synths_in_bus() {
    let code = r#"
tempo: 2.0
~kick: superkick 60
~bass: supersaw 55
out: ~kick * 0.5 + ~bass * 0.3
"#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(44100);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    assert!(rms > 0.01, "Mixed synths should produce audio");
}

#[test]
fn test_synth_through_filter() {
    let code = r#"
tempo: 2.0
out: supersaw 110 # lpf 2000 0.8
"#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(44100);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    assert!(rms > 0.01, "Filtered synth should produce audio");
}

#[test]
fn test_synth_through_effects_chain() {
    let code = r#"
tempo: 2.0
out: supersaw 110 0.5 5 # distortion 2.0 0.3 # reverb 0.5 0.5 0.2
"#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(22050); // 0.5 seconds
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    assert!(rms > 0.01, "Synth with effects chain should produce audio");
}

// ========== Pattern-controlled Synths ==========

#[test]
fn test_synth_with_pattern_freq() {
    let code = r#"
tempo: 2.0
~freq: "110 220 440"
out: supersaw ~freq
"#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(44100);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    assert!(rms > 0.01, "Pattern-controlled synth should produce audio");
}

#[test]
fn test_drum_kit() {
    // Real drum kit using SuperDirt synths
    let code = r#"
tempo: 2.0
~kick: superkick 60 0.5 0.3 0.1
~snare: supersnare 200 0.8 0.15
~hat: superhat 0.7 0.05
out: ~kick * 0.8 + ~snare * 0.6 + ~hat * 0.4
"#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(22050); // 0.5 seconds
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    assert!(rms > 0.01, "Drum kit should produce audio");
}
