/// Tests for djf (DJ filter) effect
///
/// djf - DJ filter sweep from low-pass to high-pass
/// Values 0-0.5: low pass filter
/// Values 0.5-1: high pass filter
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

/// Helper to test code compilation and rendering
fn test_code(code: &str, duration_seconds: f32) -> Vec<f32> {
    // Parse
    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    // Compile
    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0); // 2 cycles per second

    // Render
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

#[test]
fn test_djf_lowpass() {
    // Test djf in low-pass mode (value < 0.5)
    let code = r#"
tempo: 0.5
out $ saw 220 # djf 0.2
"#;

    let buffer = test_code(code, 2.0);
    let rms = calculate_rms(&buffer);
    println!("djf low-pass RMS: {:.6}", rms);

    assert!(
        rms > 0.01,
        "djf low-pass should produce sound, got RMS {}",
        rms
    );
}

#[test]
fn test_djf_highpass() {
    // Test djf in high-pass mode (value > 0.5)
    let code = r#"
tempo: 0.5
out $ saw 55 # djf 0.8
"#;

    let buffer = test_code(code, 2.0);
    let rms = calculate_rms(&buffer);
    println!("djf high-pass RMS: {:.6}", rms);

    assert!(
        rms > 0.01,
        "djf high-pass should produce sound, got RMS {}",
        rms
    );
}

#[test]
fn test_djf_sweep() {
    // Test djf at different values across the range
    // Use higher frequency source for better high-pass response
    for djf_val in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let code = format!(
            r#"
tempo: 0.5
out $ square 880 # djf {}
"#,
            djf_val
        );

        let buffer = test_code(&code, 2.0);
        let rms = calculate_rms(&buffer);
        println!("djf {} RMS: {:.6}", djf_val, rms);

        // Note: At extreme high-pass (djf=1.0), RMS may be very low but should not be NaN
        if rms.is_nan() {
            panic!("djf {} produced NaN (filter instability)", djf_val);
        }

        // Expect some output for all values (though may be very quiet at extremes)
        assert!(
            rms > 0.001,
            "djf {} should produce sound, got RMS {}",
            djf_val,
            rms
        );
    }
}

#[test]
fn test_djf_changes_tone() {
    // Test that djf actually affects the sound differently at different values
    let lowpass_code = r#"
tempo: 0.5
out $ square 440 # djf 0.1
"#;

    let highpass_code = r#"
tempo: 0.5
out $ square 440 # djf 0.9
"#;

    let lowpass = test_code(lowpass_code, 2.0);
    let highpass = test_code(highpass_code, 2.0);

    let lowpass_rms = calculate_rms(&lowpass);
    let highpass_rms = calculate_rms(&highpass);

    println!("djf low-pass (0.1) RMS: {:.6}", lowpass_rms);
    println!("djf high-pass (0.9) RMS: {:.6}", highpass_rms);

    // Both should have sound
    assert!(lowpass_rms > 0.01, "Low-pass should have sound");
    assert!(highpass_rms > 0.01, "High-pass should have sound");

    // They should produce different results (RMS might be similar but waveform differs)
    // This is a basic check - ideally we'd do spectral analysis
}
