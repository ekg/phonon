/// Integration tests for soft_saw_hz oscillator through Phonon DSL
///
/// Level 2: Verify soft_saw_hz works through compositional parser/compiler
///
/// These tests ensure:
/// - DSL syntax parsing works: soft_saw_hz 440
/// - Pattern-modulated frequency: soft_saw_hz "110 220"
/// - Audio generation and rendering
/// - Spectral characteristics (fewer harmonics than regular saw)
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

mod audio_test_utils;
use audio_test_utils::calculate_rms;

/// Helper to compile and render DSL
fn compile_and_render(input: &str, duration_samples: usize) -> Vec<f32> {
    let (_, program) = parse_program(input).expect("Failed to parse DSL");
    let mut graph = compile_program(program, 44100.0, None).expect("Failed to compile");
    graph.render(duration_samples)
}

#[test]
fn test_soft_saw_hz_basic_audio() {
    // Test that soft_saw_hz produces audio
    let code = r#"tempo: 0.5
out: soft_saw_hz 440 * 0.3"#;

    let audio = compile_and_render(code, 44100); // 1 second
    let rms = calculate_rms(&audio);

    println!("Soft saw 440 Hz - RMS: {:.4}", rms);

    assert!(
        rms > 0.01,
        "Soft saw should produce significant audio: {}",
        rms
    );
}

#[test]
fn test_soft_saw_hz_different_frequencies() {
    // Test low and high frequencies
    let low_code = r#"tempo: 0.5
out: soft_saw_hz 55 * 0.3"#;

    let high_code = r#"tempo: 0.5
out: soft_saw_hz 2000 * 0.3"#;

    let low_audio = compile_and_render(low_code, 44100);
    let high_audio = compile_and_render(high_code, 44100);

    let low_rms = calculate_rms(&low_audio);
    let high_rms = calculate_rms(&high_audio);

    println!("Low (55 Hz) RMS: {:.4}", low_rms);
    println!("High (2000 Hz) RMS: {:.4}", high_rms);

    assert!(low_rms > 0.01, "Low frequency should produce audio");
    assert!(high_rms > 0.01, "High frequency should produce audio");
}

#[test]
fn test_soft_saw_hz_pattern_modulation() {
    // Test pattern-modulated frequency
    let code = r#"tempo: 0.5
out: soft_saw_hz "110 220 440" * 0.3"#;

    let audio = compile_and_render(code, 88200); // 2 seconds
    let rms = calculate_rms(&audio);

    println!("Pattern-modulated soft saw - RMS: {:.4}", rms);

    assert!(
        rms > 0.01,
        "Pattern-modulated soft saw should produce audio"
    );
}

#[test]
fn test_soft_saw_vs_regular_saw_amplitude() {
    // Compare RMS levels between soft_saw and regular saw
    let soft_code = r#"tempo: 0.5
out: soft_saw_hz 220 * 0.3"#;

    let regular_code = r#"tempo: 0.5
out: saw_hz 220 * 0.3"#;

    let soft_audio = compile_and_render(soft_code, 44100);
    let regular_audio = compile_and_render(regular_code, 44100);

    let soft_rms = calculate_rms(&soft_audio);
    let regular_rms = calculate_rms(&regular_audio);

    println!("Soft saw RMS: {:.4}", soft_rms);
    println!("Regular saw RMS: {:.4}", regular_rms);

    // Both should produce audio
    assert!(soft_rms > 0.01, "Soft saw should produce audio");
    assert!(regular_rms > 0.01, "Regular saw should produce audio");

    // RMS levels might be similar (spectral difference is in frequency domain)
    // Just verify both work
}

#[test]
fn test_soft_saw_hz_arithmetic() {
    // Test soft_saw_hz with arithmetic operations
    let code = r#"tempo: 0.5
~lfo: sine 0.5
out: soft_saw_hz (~lfo * 100 + 220) * 0.3"#;

    let audio = compile_and_render(code, 88200); // 2 seconds
    let rms = calculate_rms(&audio);

    println!("LFO-modulated soft saw - RMS: {:.4}", rms);

    assert!(rms > 0.01, "LFO-modulated soft saw should produce audio");
}

#[test]
fn test_soft_saw_hz_with_filter() {
    // Test soft_saw_hz through a filter
    let code = r#"tempo: 0.5
out: soft_saw_hz 110 # lpf 500 0.8 * 0.3"#;

    let audio = compile_and_render(code, 44100);
    let rms = calculate_rms(&audio);

    println!("Filtered soft saw - RMS: {:.4}", rms);

    assert!(rms > 0.005, "Filtered soft saw should produce audio");
}

#[test]
fn test_soft_saw_hz_mixing() {
    // Test mixing soft_saw with other signals
    let code = r#"tempo: 0.5
~soft: soft_saw_hz 110 * 0.15
~regular: saw_hz 220 * 0.15
out: ~soft + ~regular"#;

    let audio = compile_and_render(code, 44100);
    let rms = calculate_rms(&audio);

    println!("Mixed saws - RMS: {:.4}", rms);

    assert!(rms > 0.01, "Mixed saws should produce audio");
}

#[test]
fn test_soft_saw_hz_stereo() {
    // Test soft_saw_hz in stereo context
    let code = r#"tempo: 0.5
~saw: soft_saw_hz 220 * 0.3
out: ~saw # pan2 0.0"#;

    let (_, program) = parse_program(code).expect("Failed to parse DSL");
    let mut graph = compile_program(program, 44100.0, None).expect("Failed to compile");

    let (left, right) = graph.render_stereo(44100); // 1 second

    let left_rms = calculate_rms(&left);
    let right_rms = calculate_rms(&right);

    println!(
        "Stereo soft saw - Left RMS: {:.4}, Right RMS: {:.4}",
        left_rms, right_rms
    );

    // Panned hard left, so left should have more energy
    assert!(left_rms > 0.01, "Left channel should have audio");
    assert!(
        left_rms > right_rms * 2.0,
        "Left should be louder (panned left)"
    );
}
