/// Comprehensive tests for fundsp saw_hz integration
///
/// Following the three-level testing methodology:
/// - Level 1: Not applicable (saw_hz is a continuous generator)
/// - Level 2: Not applicable (saw_hz is continuous, not event-based)
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
fn test_saw_hz_level3_basic() {
    // Test basic saw oscillator
    let code = "out $ saw_hz 220";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);
    let peak = calculate_peak(&audio);

    // Should have energy
    assert!(rms > 0.01, "RMS too low: {}", rms);
    assert!(peak > rms, "Peak should be higher than RMS");

    println!("Basic saw_hz 220 Hz - RMS: {:.4}, Peak: {:.4}", rms, peak);
}

#[test]
fn test_saw_hz_level3_frequency_sweep() {
    // Test different frequencies
    let frequencies = vec![55.0, 110.0, 220.0, 440.0, 880.0];

    for freq in &frequencies {
        let code = format!("out $ saw_hz {}", freq);
        let audio = render_dsl(&code, 1.0);
        let rms = calculate_rms(&audio);

        assert!(rms > 0.01, "Frequency {} should produce output", freq);
        println!("Frequency {} Hz: RMS {:.4}", freq, rms);
    }
}

#[test]
fn test_saw_hz_level3_pattern_control() {
    // Test pattern-controlled frequency (Phonon's killer feature!)
    let code = r#"
        tempo: 0.5
        out $ saw_hz "110 165 220 330"
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Pattern-controlled saw should work: {}", rms);

    println!("Pattern control - RMS: {:.4}", rms);
}

#[test]
fn test_saw_hz_level3_lfo_modulation() {
    // Test LFO modulation of frequency
    let code = r#"
        tempo: 0.5
        ~lfo $ sine 0.5
        ~freq $ ~lfo * 100 + 220
        out $ saw_hz ~freq
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "LFO modulated saw should work: {}", rms);

    println!("LFO modulation - RMS: {:.4}", rms);
}

#[test]
fn test_saw_hz_level3_through_filter() {
    // Test saw through filter
    let code = "out $ saw_hz 110 # lpf 500 0.8";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Filtered saw should work");

    println!("Saw through LPF - RMS: {:.4}", rms);
}

#[test]
fn test_saw_hz_level3_bass() {
    // Test bass frequency
    let code = "out $ saw_hz 55";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Bass saw should work");

    println!("Bass saw (55 Hz) - RMS: {:.4}", rms);
}

#[test]
fn test_saw_hz_level3_high() {
    // Test high frequency
    let code = "out $ saw_hz 2000";
    let audio = render_dsl(code, 1.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "High saw should work");

    println!("High saw (2000 Hz) - RMS: {:.4}", rms);
}

#[test]
fn test_saw_hz_level3_amplitude_control() {
    // Test amplitude scaling
    let code = "out $ saw_hz 220 * 0.5";
    let audio = render_dsl(code, 1.0);

    let rms = calculate_rms(&audio);

    // Scaled amplitude should still work
    assert!(rms > 0.01, "Amplitude-scaled saw should work");

    println!("Amplitude scaled (0.5x) - RMS: {:.4}", rms);
}

#[test]
fn test_saw_hz_level3_multiple_oscillators() {
    // Test multiple saw oscillators mixed
    let code = r#"
        ~saw1 $ saw_hz 220
        ~saw2 $ saw_hz 221.5
        out $ (~saw1 + ~saw2) * 0.5
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Mixed saws should work");

    println!("Mixed saws - RMS: {:.4}", rms);
}

#[test]
fn test_saw_hz_level3_detuned_stack() {
    // Test detuned saw stack (pseudo-supersaw)
    let code = r#"
        ~saw1 $ saw_hz 220
        ~saw2 $ saw_hz 221.5
        ~saw3 $ saw_hz 218.5
        out $ (~saw1 + ~saw2 + ~saw3) * 0.33
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Detuned stack should work");

    println!("Detuned stack - RMS: {:.4}", rms);
}

#[test]
fn test_saw_hz_level3_with_envelope() {
    // Test saw with amplitude envelope
    let code = r#"
        tempo: 0.5
        ~lfo $ sine 0.5
        ~env $ ~lfo * 0.4 + 0.6
        out $ saw_hz 220 * ~env
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Saw with envelope should work");

    println!("Saw with envelope - RMS: {:.4}", rms);
}
