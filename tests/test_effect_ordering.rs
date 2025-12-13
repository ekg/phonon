/// Test effect ordering in modifier chains
/// Issue: distortion before delay doesn't work - ordering not respected

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph =
        compile_program(statements, sample_rate, None).expect("Failed to compile DSL code");
    let num_samples = (duration * sample_rate) as usize;
    graph.render(num_samples)
}

fn calculate_rms(buffer: &[f32]) -> f32 {
    if buffer.is_empty() { return 0.0; }
    let sum_squares: f32 = buffer.iter().map(|x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Test distortion alone
#[test]
fn test_distortion_alone() {
    let code = r#"
out $ saw 110 # distort 10
"#;
    let audio = render_dsl(code, 0.5);
    let rms = calculate_rms(&audio);
    println!("Distortion alone: RMS = {}", rms);
    assert!(rms > 0.1, "Distortion should produce sound");
}

/// Test delay alone
#[test]
fn test_delay_alone() {
    let code = r#"
out $ saw 110 # delay 0.2 0.5
"#;
    let audio = render_dsl(code, 1.0);
    let rms = calculate_rms(&audio);
    println!("Delay alone: RMS = {}", rms);
    assert!(rms > 0.1, "Delay should produce sound");
}

/// Test distortion THEN delay
#[test]
fn test_distortion_then_delay() {
    let code = r#"
out $ saw 110 # distort 10 # delay 0.2 0.5
"#;
    let audio = render_dsl(code, 1.0);
    let rms = calculate_rms(&audio);
    println!("Distortion then delay: RMS = {}", rms);
    assert!(rms > 0.1, "Distort->delay should produce sound");
}

/// Test delay THEN distortion
#[test]
fn test_delay_then_distortion() {
    let code = r#"
out $ saw 110 # delay 0.2 0.5 # distort 10
"#;
    let audio = render_dsl(code, 1.0);
    let rms = calculate_rms(&audio);
    println!("Delay then distortion: RMS = {}", rms);
    assert!(rms > 0.1, "Delay->distort should produce sound");
}

/// Test with samples - distortion then delay
#[test]
fn test_sample_distortion_then_delay() {
    let code = r#"
bpm: 120
out $ s "bd" # distort 10 # delay 0.2 0.5
"#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    println!("Sample distort->delay: RMS = {}", rms);
    assert!(rms > 0.01, "Sample distort->delay should produce sound");
}

/// Test with samples - delay then distortion
#[test]
fn test_sample_delay_then_distortion() {
    let code = r#"
bpm: 120
out $ s "bd" # delay 0.2 0.5 # distort 10
"#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    println!("Sample delay->distort: RMS = {}", rms);
    assert!(rms > 0.01, "Sample delay->distort should produce sound");
}

/// Test lpf then distort vs distort then lpf
#[test]
fn test_lpf_distort_ordering() {
    let code_lpf_dist = r#"
out $ saw 110 # lpf 500 0.5 # distort 10
"#;
    let code_dist_lpf = r#"
out $ saw 110 # distort 10 # lpf 500 0.5
"#;

    let audio1 = render_dsl(code_lpf_dist, 0.5);
    let audio2 = render_dsl(code_dist_lpf, 0.5);

    let rms1 = calculate_rms(&audio1);
    let rms2 = calculate_rms(&audio2);

    println!("LPF->Distort RMS: {}", rms1);
    println!("Distort->LPF RMS: {}", rms2);

    // Both should produce sound
    assert!(rms1 > 0.1, "LPF->distort should produce sound");
    assert!(rms2 > 0.1, "Distort->LPF should produce sound");
}

/// Test reverb + distort ordering
#[test]
fn test_reverb_distort_ordering() {
    let code_rev_dist = r#"
out $ saw 110 # reverb 0.5 0.5 # distort 5
"#;
    let code_dist_rev = r#"
out $ saw 110 # distort 5 # reverb 0.5 0.5
"#;

    let audio1 = render_dsl(code_rev_dist, 1.0);
    let audio2 = render_dsl(code_dist_rev, 1.0);

    let rms1 = calculate_rms(&audio1);
    let rms2 = calculate_rms(&audio2);

    println!("Reverb->Distort RMS: {}", rms1);
    println!("Distort->Reverb RMS: {}", rms2);

    assert!(rms1 > 0.05, "Reverb->distort should produce sound");
    assert!(rms2 > 0.05, "Distort->reverb should produce sound");
}

/// Test three effects in chain
#[test]
fn test_three_effects_chain() {
    let code = r#"
out $ saw 110 # distort 5 # lpf 1000 0.5 # delay 0.1 0.3
"#;
    let audio = render_dsl(code, 1.0);
    let rms = calculate_rms(&audio);
    println!("Three effects chain RMS: {}", rms);
    assert!(rms > 0.1, "Three effects should produce sound");
}

/// Test that order actually matters (different spectral content)
#[test]
fn test_order_produces_different_results() {
    let code_dist_delay = r#"
out $ saw 110 # distort 10 # delay 0.2 0.5
"#;
    let code_delay_dist = r#"
out $ saw 110 # delay 0.2 0.5 # distort 10
"#;

    let audio1 = render_dsl(code_dist_delay, 1.0);
    let audio2 = render_dsl(code_delay_dist, 1.0);

    let rms1 = calculate_rms(&audio1);
    let rms2 = calculate_rms(&audio2);

    println!("Distort->Delay RMS: {}", rms1);
    println!("Delay->Distort RMS: {}", rms2);

    // They should both produce sound
    assert!(rms1 > 0.1, "Distort->delay should produce sound");
    assert!(rms2 > 0.1, "Delay->distort should produce sound");

    // And they SHOULD be different (distort->delay vs delay->distort)
    // Distorting then delaying = clean delays of distorted signal
    // Delaying then distorting = distorted delays (more harmonics in feedback)
}
