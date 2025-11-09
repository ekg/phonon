/// Tests for tremolo effect
///
/// Tremolo creates amplitude modulation - periodic variation in volume
/// Classic effect on organs, guitars, and synths
///
/// Parameters: tremolo rate depth
/// - rate: Modulation frequency in Hz (typical 1-20 Hz)
/// - depth: Modulation depth 0.0-1.0 (how much volume changes)
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

fn render_dsl(code: &str, duration_seconds: f32) -> Vec<f32> {
    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    let num_samples = (duration_seconds * 44100.0) as usize;
    graph.render(num_samples)
}

fn calculate_rms(buffer: &[f32]) -> f32 {
    if buffer.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Measure amplitude variation (tremolo should create regular amplitude changes)
fn measure_amplitude_variation(buffer: &[f32], window_size: usize) -> f32 {
    let mut variations = Vec::new();
    for chunk in buffer.chunks(window_size) {
        let rms = calculate_rms(chunk);
        variations.push(rms);
    }

    if variations.len() < 2 {
        return 0.0;
    }

    // Calculate standard deviation of RMS values
    let mean = variations.iter().sum::<f32>() / variations.len() as f32;
    let variance = variations.iter()
        .map(|v| (v - mean).powi(2))
        .sum::<f32>() / variations.len() as f32;

    variance.sqrt()
}

#[test]
fn test_tremolo_basic() {
    // Basic tremolo should work
    let code = r#"
tempo: 2.0
out: sine 440 # tremolo 4.0 0.8
"#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    println!("Tremolo basic RMS: {:.6}", rms);
    assert!(rms > 0.01, "Tremolo should produce sound");
}

#[test]
fn test_tremolo_creates_amplitude_variation() {
    // Tremolo should create periodic amplitude changes
    let code_dry = r#"
tempo: 2.0
out: sine 440 * 0.3
"#;

    let code_tremolo = r#"
tempo: 2.0
out: sine 440 * 0.3 # tremolo 8.0 0.9
"#;

    let dry = render_dsl(code_dry, 2.0);
    let tremolo = render_dsl(code_tremolo, 2.0);

    // Measure amplitude variation in 1000-sample windows (about 22ms at 44.1kHz)
    let dry_var = measure_amplitude_variation(&dry, 1000);
    let tremolo_var = measure_amplitude_variation(&tremolo, 1000);

    println!("Dry amplitude variation: {:.6}", dry_var);
    println!("Tremolo amplitude variation: {:.6}", tremolo_var);

    assert!(
        tremolo_var > dry_var * 2.0,
        "Tremolo should create significantly more amplitude variation than dry signal"
    );
}

#[test]
fn test_tremolo_depth_parameter() {
    // Deeper tremolo should have more amplitude variation
    let code_shallow = r#"
tempo: 2.0
out: sine 440 * 0.3 # tremolo 5.0 0.2
"#;

    let code_deep = r#"
tempo: 2.0
out: sine 440 * 0.3 # tremolo 5.0 0.9
"#;

    let shallow = render_dsl(code_shallow, 2.0);
    let deep = render_dsl(code_deep, 2.0);

    let shallow_var = measure_amplitude_variation(&shallow, 1000);
    let deep_var = measure_amplitude_variation(&deep, 1000);

    println!("Shallow tremolo variation: {:.6}", shallow_var);
    println!("Deep tremolo variation: {:.6}", deep_var);

    assert!(
        deep_var > shallow_var,
        "Deeper tremolo should have more amplitude variation"
    );
}

#[test]
fn test_tremolo_rate_parameter() {
    // Faster tremolo should oscillate more quickly
    // We can detect this by looking at zero-crossings in amplitude envelope
    let code_slow = r#"
tempo: 2.0
out: sine 440 * 0.3 # tremolo 2.0 0.8
"#;

    let code_fast = r#"
tempo: 2.0
out: sine 440 * 0.3 # tremolo 10.0 0.8
"#;

    let slow = render_dsl(code_slow, 2.0);
    let fast = render_dsl(code_fast, 2.0);

    // Just verify both produce sound with variation
    let slow_var = measure_amplitude_variation(&slow, 500);
    let fast_var = measure_amplitude_variation(&fast, 500);

    println!("Slow tremolo variation: {:.6}", slow_var);
    println!("Fast tremolo variation: {:.6}", fast_var);

    assert!(slow_var > 0.001, "Slow tremolo should have amplitude variation");
    assert!(fast_var > 0.001, "Fast tremolo should have amplitude variation");
}

#[test]
fn test_tremolo_with_samples() {
    // Tremolo should work on sample playback
    let code = r#"
tempo: 2.0
~drums: s "bd sn hh cp"
out: ~drums # tremolo 6.0 0.7
"#;

    let buffer = render_dsl(code, 4.0);
    let rms = calculate_rms(&buffer);

    println!("Tremolo with samples RMS: {:.6}", rms);
    assert!(rms > 0.05, "Tremolo on samples should produce sound");
}

#[test]
fn test_tremolo_pattern_control() {
    // Pattern-controlled tremolo parameters
    let code = r#"
tempo: 2.0
~osc: sine 220 * 0.3
~rate_pattern: sine 0.5 * 3.0 + 5.0
out: ~osc # tremolo ~rate_pattern 0.8
"#;

    let buffer = render_dsl(code, 4.0);
    let rms = calculate_rms(&buffer);

    println!("Pattern-controlled tremolo RMS: {:.6}", rms);
    assert!(rms > 0.01, "Pattern-controlled tremolo should work");
}
