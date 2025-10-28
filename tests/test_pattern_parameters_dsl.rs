/// Test DSL syntax for pattern-based DSP parameters
/// These tests verify that the high-level Phonon DSL can specify
/// per-voice parameters like gain, pan, speed, etc.

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

mod audio_test_utils;
use audio_test_utils::calculate_rms;

/// Render a Phonon DSL program and return the audio buffer
fn render_dsl(code: &str, cycles: usize) -> Vec<f32> {
    let (_, statements) = parse_program(code).expect("Parse failed");
    let sample_rate = 44100.0;
    let mut graph = compile_program(statements, sample_rate).expect("Compile failed");

    let samples_per_cycle = (sample_rate as f64 / 0.5) as usize; // tempo = 0.5 cps
    let total_samples = samples_per_cycle * cycles;

    graph.render(total_samples)
}

#[test]
fn test_gain_parameter_pattern() {
    let code = r#"
tempo: 0.5
d1: s "bd bd" gain="0.5 1.0"
"#;

    let audio = render_dsl(code, 2);
    let rms = calculate_rms(&audio);

    // Should have audible output
    assert!(rms > 0.01, "Should produce audible sound with gain parameter");

    println!("✅ gain parameter: RMS = {:.3}", rms);
}

#[test]
fn test_pan_parameter_pattern() {
    let code = r#"
tempo: 0.5
d1: s "bd bd" pan="-1.0 1.0"
"#;

    let audio = render_dsl(code, 2);
    let rms = calculate_rms(&audio);

    // Should have audible output
    assert!(rms > 0.01, "Should produce audible sound with pan parameter");

    println!("✅ pan parameter: RMS = {:.3}", rms);
}

#[test]
fn test_speed_parameter_pattern() {
    let code = r#"
tempo: 0.5
d1: s "bd bd" speed="1.0 2.0"
"#;

    let audio = render_dsl(code, 2);
    let rms = calculate_rms(&audio);

    // Should have audible output
    assert!(rms > 0.01, "Should produce audible sound with speed parameter");

    println!("✅ speed parameter: RMS = {:.3}", rms);
}

#[test]
fn test_multiple_parameters() {
    let code = r#"
tempo: 0.5
d1: s "bd sn" gain="0.8 1.0" pan="-1.0 1.0" speed="1.0 0.5"
"#;

    let audio = render_dsl(code, 2);
    let rms = calculate_rms(&audio);

    // Should have audible output
    assert!(rms > 0.01, "Should produce audible sound with multiple parameters");

    println!("✅ multiple parameters: RMS = {:.3}", rms);
}

#[test]
fn test_continuous_modulation() {
    let code = r#"
tempo: 0.5
~lfo: sine 0.25
d1: s "hh*8" pan=~lfo
"#;

    let audio = render_dsl(code, 4);
    let rms = calculate_rms(&audio);

    // Should have audible output
    assert!(rms > 0.01, "Should produce audible sound with continuous modulation");

    println!("✅ continuous modulation: RMS = {:.3}", rms);
}
