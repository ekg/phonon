/// Comprehensive tests for fundsp pink noise integration
///
/// Following the three-level testing methodology:
/// - Level 1: Not applicable (pink noise is a continuous generator)
/// - Level 2: Not applicable (pink noise is continuous, not event-based)
/// - Level 3: Audio characteristics (signal quality verification)
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

/// Render DSL code to audio buffer using compositional compiler
fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph = compile_program(statements, sample_rate, None).expect("Failed to compile DSL code");
    let num_samples = (duration * sample_rate) as usize;
    graph.render(num_samples)
}

/// Calculate RMS (root mean square) of audio buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    if buffer.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = buffer.iter().map(|x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Calculate peak amplitude
fn calculate_peak(buffer: &[f32]) -> f32 {
    buffer.iter().map(|x| x.abs()).fold(0.0f32, f32::max)
}

#[test]
fn test_pink_level3_basic() {
    // Test basic pink noise generator
    let code = "out: pink";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);
    let peak = calculate_peak(&audio);

    // Pink noise should have significant energy
    assert!(rms > 0.1, "RMS too low: {}", rms);
    assert!(peak > rms, "Peak should be higher than RMS");

    println!("Basic pink noise - RMS: {:.4}, Peak: {:.4}", rms, peak);
}

#[test]
fn test_pink_level3_amplitude_control() {
    // Test amplitude scaling
    let code = "out: pink * 0.5";
    let audio = render_dsl(code, 1.0);

    let rms = calculate_rms(&audio);

    // Scaled amplitude should still work
    assert!(rms > 0.05, "Amplitude-scaled pink noise should work");

    println!("Amplitude scaled (0.5x) - RMS: {:.4}", rms);
}

#[test]
fn test_pink_level3_snare() {
    // Snare: pink noise + sine body (more natural than white noise)
    let code = r#"
        ~snare_body: sine 180
        ~snare_noise: pink # hpf 2000 0.5
        out: (~snare_body + ~snare_noise) * 0.3
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Snare (pink noise + sine) should work");

    println!("Snare (pink noise + sine) - RMS: {:.4}", rms);
}

#[test]
fn test_pink_level3_hihat() {
    // Hi-hat: high-pass filtered pink noise
    let code = "out: pink # hpf 5000 0.3";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Hi-hat (filtered pink noise) should work");

    println!("Hi-hat (HPF pink noise) - RMS: {:.4}", rms);
}

#[test]
fn test_pink_level3_rain() {
    // Rain: pink noise with envelope
    let code = r#"
        tempo: 0.5
        ~lfo: sine 0.1
        ~env: ~lfo * 0.5 + 0.5
        ~filtered: pink # lpf 2000 0.3
        out: ~filtered * ~env * 0.3
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Rain (pink noise with envelope) should work");

    println!("Rain (pink noise) - RMS: {:.4}", rms);
}

#[test]
fn test_pink_level3_waterfall() {
    // Waterfall: band-pass filtered pink noise
    let code = "out: pink # bpf 800 0.4";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Waterfall (BPF pink noise) should work");

    println!("Waterfall (BPF pink noise) - RMS: {:.4}", rms);
}

#[test]
fn test_pink_level3_with_envelope() {
    // Pink noise burst with envelope
    let code = r#"
        tempo: 0.5
        ~lfo: sine 0.5
        ~env: ~lfo * 0.4 + 0.6
        out: pink * ~env * 0.3
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Pink noise with envelope should work");

    println!("Pink noise with envelope - RMS: {:.4}", rms);
}

#[test]
fn test_pink_level3_vs_white_noise() {
    // Compare pink to white noise (different spectrum)
    let code_pink = "out: pink * 0.3";
    let code_white = "out: noise * 0.3";

    let audio_pink = render_dsl(code_pink, 1.0);
    let audio_white = render_dsl(code_white, 1.0);

    let rms_pink = calculate_rms(&audio_pink);
    let rms_white = calculate_rms(&audio_white);

    // Both should have energy
    assert!(rms_pink > 0.01);
    assert!(rms_white > 0.01);

    println!("Pink RMS: {:.4}, White RMS: {:.4}", rms_pink, rms_white);
}

#[test]
fn test_pink_level3_textured_pad() {
    // Textured pad (oscillator + filtered pink noise)
    let code = r#"
        ~pad: saw_hz 110
        ~texture: pink # lpf 500 0.2
        out: (~pad * 0.8 + ~texture * 0.2) * 0.3
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Textured pad should work");

    println!("Textured pad (osc + pink) - RMS: {:.4}", rms);
}

#[test]
fn test_pink_level3_rhythmic() {
    // Rhythmic pink noise (amplitude modulation)
    let code = r#"
        tempo: 4.0
        ~lfo: sine 1.0
        ~env: ~lfo * 0.5 + 0.5
        out: pink * ~env * 0.3
    "#;
    let audio = render_dsl(code, 1.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Rhythmic pink noise should work");

    println!("Rhythmic pink noise - RMS: {:.4}", rms);
}
