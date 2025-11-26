/// Comprehensive tests for fundsp square_hz integration
///
/// Following the three-level testing methodology:
/// - Level 1: Not applicable (square_hz is a continuous generator)
/// - Level 2: Not applicable (square_hz is continuous, not event-based)
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
fn test_square_hz_level3_basic() {
    // Test basic square oscillator
    let code = "out: square_hz 220";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);
    let peak = calculate_peak(&audio);

    // Should have energy
    assert!(rms > 0.01, "RMS too low: {}", rms);
    assert!(peak > rms, "Peak should be higher than RMS");

    println!(
        "Basic square_hz 220 Hz - RMS: {:.4}, Peak: {:.4}",
        rms, peak
    );
}

#[test]
fn test_square_hz_level3_frequency_sweep() {
    // Test different frequencies
    let frequencies = vec![55.0, 110.0, 220.0, 440.0, 880.0];

    for freq in &frequencies {
        let code = format!("out: square_hz {}", freq);
        let audio = render_dsl(&code, 1.0);
        let rms = calculate_rms(&audio);

        assert!(rms > 0.01, "Frequency {} should produce output", freq);
        println!("Frequency {} Hz: RMS {:.4}", freq, rms);
    }
}

#[test]
fn test_square_hz_level3_pattern_control() {
    // Test pattern-controlled frequency (Phonon's killer feature!)
    let code = r#"
        tempo: 2.0
        out: square_hz "110 165 220 330"
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Pattern-controlled square should work: {}", rms);

    println!("Pattern control - RMS: {:.4}", rms);
}

#[test]
fn test_square_hz_level3_lfo_modulation() {
    // Test LFO modulation of frequency
    let code = r#"
        tempo: 2.0
        ~lfo: sine 0.5
        ~freq: ~lfo * 100 + 220
        out: square_hz ~freq
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "LFO modulated square should work: {}", rms);

    println!("LFO modulation - RMS: {:.4}", rms);
}

#[test]
fn test_square_hz_level3_through_filter() {
    // Test square through filter (classic bass sound)
    let code = "out: square_hz 55 # lpf 200 0.8";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Filtered square should work");

    println!("Square through LPF - RMS: {:.4}", rms);
}

#[test]
fn test_square_hz_level3_bass() {
    // Test bass frequency
    let code = "out: square_hz 55";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Bass square should work");

    println!("Bass square (55 Hz) - RMS: {:.4}", rms);
}

#[test]
fn test_square_hz_level3_high() {
    // Test high frequency
    let code = "out: square_hz 2000";
    let audio = render_dsl(code, 1.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "High square should work");

    println!("High square (2000 Hz) - RMS: {:.4}", rms);
}

#[test]
fn test_square_hz_level3_amplitude_control() {
    // Test amplitude scaling
    let code = "out: square_hz 220 * 0.5";
    let audio = render_dsl(code, 1.0);

    let rms = calculate_rms(&audio);

    // Scaled amplitude should still work
    assert!(rms > 0.01, "Amplitude-scaled square should work");

    println!("Amplitude scaled (0.5x) - RMS: {:.4}", rms);
}

#[test]
fn test_square_hz_level3_multiple_oscillators() {
    // Test multiple square oscillators mixed
    let code = r#"
        ~sq1: square_hz 220
        ~sq2: square_hz 221.5
        out: (~sq1 + ~sq2) * 0.5
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Mixed squares should work");

    println!("Mixed squares - RMS: {:.4}", rms);
}

#[test]
fn test_square_hz_level3_detuned_stack() {
    // Test detuned square stack (pseudo-PWM)
    let code = r#"
        ~sq1: square_hz 220
        ~sq2: square_hz 220.5
        ~sq3: square_hz 219.5
        out: (~sq1 + ~sq2 + ~sq3) * 0.33
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Detuned stack should work");

    println!("Detuned stack - RMS: {:.4}", rms);
}

#[test]
fn test_square_hz_level3_with_envelope() {
    // Test square with amplitude envelope
    let code = r#"
        tempo: 2.0
        ~lfo: sine 0.5
        ~env: ~lfo * 0.4 + 0.6
        out: square_hz 220 * ~env
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Square with envelope should work");

    println!("Square with envelope - RMS: {:.4}", rms);
}

#[test]
fn test_square_hz_level3_vs_saw_hz() {
    // Compare square to saw (different harmonic content)
    let code_square = "out: square_hz 220";
    let code_saw = "out: saw_hz 220";

    let audio_square = render_dsl(code_square, 1.0);
    let audio_saw = render_dsl(code_saw, 1.0);

    let rms_square = calculate_rms(&audio_square);
    let rms_saw = calculate_rms(&audio_saw);

    // Both should have energy
    assert!(rms_square > 0.01);
    assert!(rms_saw > 0.01);

    println!("Square RMS: {:.4}, Saw RMS: {:.4}", rms_square, rms_saw);
}
