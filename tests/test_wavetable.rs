//! Tests for Wavetable Oscillator
//!
//! Wavetable synthesis reads through a stored waveform at variable speeds
//! to generate different pitches. Classic technique used in PPG Wave, Waldorf, Serum.

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use std::f32::consts::PI;

/// Helper: Calculate RMS of a buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum: f32 = buffer.iter().map(|x| x * x).sum();
    (sum / buffer.len() as f32).sqrt()
}

/// Helper: Detect zero crossings for frequency measurement
fn detect_zero_crossings(buffer: &[f32]) -> Vec<usize> {
    buffer
        .windows(2)
        .enumerate()
        .filter_map(|(i, w)| {
            if w[0] <= 0.0 && w[1] > 0.0 {
                Some(i)
            } else {
                None
            }
        })
        .collect()
}

/// Helper: Measure fundamental frequency from zero crossings
fn measure_frequency(buffer: &[f32], sample_rate: f32) -> Option<f32> {
    let crossings = detect_zero_crossings(buffer);
    if crossings.len() < 2 {
        return None;
    }

    let periods: Vec<f32> = crossings.windows(2).map(|w| (w[1] - w[0]) as f32).collect();

    let avg_period = periods.iter().sum::<f32>() / periods.len() as f32;
    Some(sample_rate / avg_period)
}

// ========== LEVEL 1: Basic Functionality ==========

#[test]
fn test_wavetable_produces_sound() {
    // Simple test: wavetable oscillator produces non-zero output
    let code = r#"
tempo: 1.0
out: wavetable 440
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    let buffer = graph.render(44100); // 1 second

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Wavetable should produce audible output, got RMS={}",
        rms
    );
}

#[test]
fn test_wavetable_frequency_accuracy() {
    // Verify wavetable plays at the correct frequency
    let code = r#"
tempo: 1.0
out: wavetable 440
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    let buffer = graph.render(44100); // 1 second

    let measured_freq = measure_frequency(&buffer, 44100.0).expect("Should detect frequency");

    // Should be within 1% of 440Hz
    assert!(
        (measured_freq - 440.0).abs() < 4.4,
        "Expected ~440Hz, measured {}Hz",
        measured_freq
    );
}

#[test]
fn test_wavetable_different_frequencies() {
    // Test multiple frequencies
    let frequencies = [110.0, 220.0, 440.0, 880.0];

    for freq in &frequencies {
        let code = format!(
            r#"
tempo: 1.0
out: wavetable {}
"#,
            freq
        );

        let (rest, statements) = parse_program(&code).expect("Failed to parse");
        assert_eq!(rest.trim(), "", "Parser should consume all input");

        let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
        let buffer = graph.render(44100);

        let measured_freq = measure_frequency(&buffer, 44100.0).expect("Should detect frequency");

        // Within 2% tolerance
        let tolerance = freq * 0.02;
        assert!(
            (measured_freq - freq).abs() < tolerance,
            "Expected ~{}Hz, measured {}Hz",
            freq,
            measured_freq
        );
    }
}

// ========== LEVEL 2: Pattern Modulation ==========

#[test]
fn test_wavetable_pattern_frequency() {
    // Wavetable with pattern-modulated frequency
    let code = r#"
tempo: 2.0
out: wavetable "220 440 330 550"
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(44100); // 1 second = 2 cycles

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Pattern-modulated wavetable should produce sound, got RMS={}",
        rms
    );
}

// ========== LEVEL 3: Custom Waveforms ==========

#[test]
fn test_wavetable_custom_waveform() {
    // In the future, support loading custom waveforms
    // For now, we'll use a built-in waveform
    let code = r#"
tempo: 1.0
out: wavetable 440
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    let buffer = graph.render(44100);

    // Should produce clean periodic output
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.01 && rms < 1.0, "RMS should be reasonable: {}", rms);
}

// ========== LEVEL 4: With Effects ==========

#[test]
fn test_wavetable_with_filter() {
    // Wavetable through a low-pass filter
    let code = r#"
tempo: 1.0
~osc: wavetable 440
out: ~osc # lpf 1000 1.0
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    let buffer = graph.render(44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Filtered wavetable should produce sound, got RMS={}",
        rms
    );
}

#[test]
fn test_wavetable_with_envelope() {
    // Wavetable with ADSR envelope
    let code = r#"
tempo: 1.0
~osc: wavetable 440
~envelope: adsr 0.01 0.1 0.7 0.2
out: ~osc * ~envelope
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    let buffer = graph.render(44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.005,
        "Enveloped wavetable should produce sound, got RMS={}",
        rms
    );
}

// ========== LEVEL 5: Musical Examples ==========

#[test]
fn test_wavetable_melody() {
    // Play a simple melody with wavetable
    let code = r#"
tempo: 2.0
~melody: wavetable "220 330 440 330"
~env: adsr 0.01 0.1 0.0 0.1
out: ~melody * ~env * 0.3
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(44100); // 1 second = 2 cycles

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.005,
        "Wavetable melody should produce sound, got RMS={}",
        rms
    );
}
