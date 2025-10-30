/// Comprehensive tests for fundsp triangle_hz integration
///
/// Following the three-level testing methodology:
/// - Level 1: Not applicable (triangle_hz is a continuous generator)
/// - Level 2: Not applicable (triangle_hz is continuous, not event-based)
/// - Level 3: Audio characteristics (signal quality verification)

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

/// Render DSL code to audio buffer using compositional compiler
fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph = compile_program(statements, sample_rate).expect("Failed to compile DSL code");
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
fn test_triangle_hz_level3_basic() {
    // Test basic triangle oscillator
    let code = "out: triangle_hz 220";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);
    let peak = calculate_peak(&audio);

    // Should have energy
    assert!(rms > 0.01, "RMS too low: {}", rms);
    assert!(peak > rms, "Peak should be higher than RMS");

    println!(
        "Basic triangle_hz 220 Hz - RMS: {:.4}, Peak: {:.4}",
        rms, peak
    );
}

#[test]
fn test_triangle_hz_level3_frequency_sweep() {
    // Test different frequencies
    let frequencies = vec![55.0, 110.0, 220.0, 440.0, 880.0];

    for freq in &frequencies {
        let code = format!("out: triangle_hz {}", freq);
        let audio = render_dsl(&code, 1.0);
        let rms = calculate_rms(&audio);

        assert!(rms > 0.01, "Frequency {} should produce output", freq);
        println!("Frequency {} Hz: RMS {:.4}", freq, rms);
    }
}

#[test]
fn test_triangle_hz_level3_pattern_control() {
    // Test pattern-controlled frequency (Phonon's killer feature!)
    let code = r#"
        tempo: 2.0
        out: triangle_hz "110 165 220 330"
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Pattern-controlled triangle should work: {}", rms);

    println!("Pattern control - RMS: {:.4}", rms);
}

#[test]
fn test_triangle_hz_level3_lfo_modulation() {
    // Test LFO modulation of frequency
    let code = r#"
        tempo: 2.0
        ~lfo: sine 0.5
        ~freq: ~lfo * 100 + 220
        out: triangle_hz ~freq
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "LFO modulated triangle should work: {}", rms);

    println!("LFO modulation - RMS: {:.4}", rms);
}

#[test]
fn test_triangle_hz_level3_soft_bass() {
    // Triangle bass is softer/mellower than square/saw
    let code = "out: triangle_hz 55";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Triangle bass should work");

    println!("Triangle bass (55 Hz) - RMS: {:.4}", rms);
}

#[test]
fn test_triangle_hz_level3_high() {
    // Test high frequency (flute-like)
    let code = "out: triangle_hz 880";
    let audio = render_dsl(code, 1.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "High triangle should work");

    println!("High triangle (880 Hz) - RMS: {:.4}", rms);
}

#[test]
fn test_triangle_hz_level3_amplitude_control() {
    // Test amplitude scaling
    let code = "out: triangle_hz 220 * 0.5";
    let audio = render_dsl(code, 1.0);

    let rms = calculate_rms(&audio);

    // Scaled amplitude should still work
    assert!(rms > 0.01, "Amplitude-scaled triangle should work");

    println!("Amplitude scaled (0.5x) - RMS: {:.4}", rms);
}

#[test]
fn test_triangle_hz_level3_multiple_oscillators() {
    // Test multiple triangle oscillators mixed
    let code = r#"
        ~tri1: triangle_hz 220
        ~tri2: triangle_hz 221.5
        out: (~tri1 + ~tri2) * 0.5
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Mixed triangles should work");

    println!("Mixed triangles - RMS: {:.4}", rms);
}

#[test]
fn test_triangle_hz_level3_octave_stack() {
    // Test octave stacking (rich but mellow)
    let code = r#"
        ~tri1: triangle_hz 110
        ~tri2: triangle_hz 220
        ~tri3: triangle_hz 440
        out: (~tri1 * 0.5 + ~tri2 * 0.3 + ~tri3 * 0.2)
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Octave stack should work");

    println!("Octave stack - RMS: {:.4}", rms);
}

#[test]
fn test_triangle_hz_level3_with_envelope() {
    // Test triangle with amplitude envelope
    let code = r#"
        tempo: 2.0
        ~lfo: sine 0.5
        ~env: ~lfo * 0.4 + 0.6
        out: triangle_hz 220 * ~env
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Triangle with envelope should work");

    println!("Triangle with envelope - RMS: {:.4}", rms);
}

#[test]
fn test_triangle_hz_level3_vs_oscillators() {
    // Compare triangle to square and saw (different harmonic content)
    let code_triangle = "out: triangle_hz 220";
    let code_square = "out: square_hz 220";
    let code_saw = "out: saw_hz 220";

    let audio_triangle = render_dsl(code_triangle, 1.0);
    let audio_square = render_dsl(code_square, 1.0);
    let audio_saw = render_dsl(code_saw, 1.0);

    let rms_triangle = calculate_rms(&audio_triangle);
    let rms_square = calculate_rms(&audio_square);
    let rms_saw = calculate_rms(&audio_saw);

    // All should have energy
    assert!(rms_triangle > 0.01);
    assert!(rms_square > 0.01);
    assert!(rms_saw > 0.01);

    println!(
        "Triangle RMS: {:.4}, Square RMS: {:.4}, Saw RMS: {:.4}",
        rms_triangle, rms_square, rms_saw
    );
}

#[test]
fn test_triangle_hz_level3_with_filter() {
    // Test triangle through filter (flute-like)
    let code = "out: triangle_hz 880 # lpf 1500 0.3";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Filtered triangle should work");

    println!("Triangle through LPF (flute-like) - RMS: {:.4}", rms);
}
