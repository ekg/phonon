//! Test scale quantization with audio verification
//!
//! Verifies that scale() quantizes scale degrees to musical frequencies.

use phonon::unified_graph_parser::{parse_dsl, DslCompiler};
use rustfft::{num_complex::Complex, FftPlanner};

/// Find the dominant frequency in an audio buffer using FFT
fn find_dominant_frequency(buffer: &[f32], sample_rate: f32) -> f32 {
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(buffer.len());

    let mut complex_input: Vec<Complex<f32>> =
        buffer.iter().map(|&x| Complex { re: x, im: 0.0 }).collect();

    fft.process(&mut complex_input);

    // Find peak in FFT (skip DC bin)
    let magnitudes: Vec<f32> = complex_input[1..complex_input.len() / 2]
        .iter()
        .map(|c| (c.re * c.re + c.im * c.im).sqrt())
        .collect();

    let max_idx = magnitudes
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(i, _)| i)
        .unwrap_or(0);

    (max_idx + 1) as f32 * sample_rate / buffer.len() as f32
}

#[test]
fn test_scale_major_c4() {
    // Test C major scale starting at C4 (MIDI 60)
    // Scale degrees: 0, 1, 2, 3, 4 = C, D, E, F, G
    // Expected frequencies: 261.63, 293.66, 329.63, 349.23, 392.00 Hz
    // Using <> alternation to get one note per cycle
    let input = r#"
        cps: 4.0
        out $ sine(scale("<0 1 2 3 4>", "major", "c4")) * 0.5
    "#;

    let (_, statements) = parse_dsl(input).unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Render 5 cycles (1.25 seconds at 4 cps) - each note plays for 0.25s
    let samples_per_cycle = (44100.0 / 4.0) as usize;
    let buffer = graph.render(samples_per_cycle * 5);

    // Analyze each cycle separately
    let expected_freqs = [261.63, 293.66, 329.63, 349.23, 392.00];

    for (i, expected) in expected_freqs.iter().enumerate() {
        let start = i * samples_per_cycle + samples_per_cycle / 4; // Skip transient
        let end = start + samples_per_cycle / 2;
        let segment = &buffer[start..end];

        let detected_freq = find_dominant_frequency(segment, 44100.0);
        let error = (detected_freq - expected).abs();
        let tolerance = 5.0; // 5 Hz tolerance

        assert!(
            error < tolerance,
            "Cycle {}: Expected {}Hz, got {}Hz (error: {}Hz)",
            i,
            expected,
            detected_freq,
            error
        );
    }
}

#[test]
fn test_scale_minor_a4() {
    // Test A minor scale starting at A4 (MIDI 69)
    // Scale degrees: 0, 1, 2 = A, B, C
    // Expected frequencies: 440.00, 493.88, 523.25 Hz
    // Using <> alternation to get one note per cycle
    let input = r#"
        cps: 3.0
        out $ sine(scale("<0 1 2>", "minor", "a4")) * 0.5
    "#;

    let (_, statements) = parse_dsl(input).unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let samples_per_cycle = (44100.0 / 3.0) as usize;
    let buffer = graph.render(samples_per_cycle * 3);

    let expected_freqs = [440.00, 493.88, 523.25];

    for (i, expected) in expected_freqs.iter().enumerate() {
        let start = i * samples_per_cycle + samples_per_cycle / 4;
        let end = start + samples_per_cycle / 2;
        let segment = &buffer[start..end];

        let detected_freq = find_dominant_frequency(segment, 44100.0);
        let error = (detected_freq - expected).abs();

        assert!(
            error < 5.0,
            "Expected {}Hz, got {}Hz",
            expected,
            detected_freq
        );
    }
}

#[test]
fn test_scale_pentatonic() {
    // Test pentatonic scale
    // Pentatonic scale: [0, 2, 4, 7, 9] semitones
    // Degrees: 0, 1, 2 = C, D, E (from C major pentatonic)
    // Using <> alternation to get one note per cycle
    let input = r#"
        cps: 3.0
        out $ sine(scale("<0 1 2>", "pentatonic", "60")) * 0.5
    "#;

    let (_, statements) = parse_dsl(input).unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let samples_per_cycle = (44100.0 / 3.0) as usize;
    let buffer = graph.render(samples_per_cycle * 3);

    // C, D, E pentatonic = C, D, E (same as major for these degrees)
    let expected_freqs = [261.63, 293.66, 329.63];

    for (i, expected) in expected_freqs.iter().enumerate() {
        let start = i * samples_per_cycle + samples_per_cycle / 4;
        let end = start + samples_per_cycle / 2;
        let segment = &buffer[start..end];

        let detected_freq = find_dominant_frequency(segment, 44100.0);
        let error = (detected_freq - expected).abs();

        assert!(
            error < 5.0,
            "Expected {}Hz, got {}Hz",
            expected,
            detected_freq
        );
    }
}

#[test]
fn test_scale_octave_wrapping() {
    // Test that scale degrees wrap to higher octaves
    // Degrees: 0, 7 (0 = C4, 7 = C5 in 7-note scale)
    // Using <> alternation to get one note per cycle
    let input = r#"
        cps: 2.0
        out $ sine(scale("<0 7>", "major", "60")) * 0.5
    "#;

    let (_, statements) = parse_dsl(input).unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let samples_per_cycle = (44100.0 / 2.0) as usize;
    let buffer = graph.render(samples_per_cycle * 2);

    // C4 = 261.63 Hz, C5 = 523.25 Hz
    let expected_freqs = [261.63, 523.25];

    for (i, expected) in expected_freqs.iter().enumerate() {
        let start = i * samples_per_cycle + samples_per_cycle / 4;
        let end = start + samples_per_cycle / 2;
        let segment = &buffer[start..end];

        let detected_freq = find_dominant_frequency(segment, 44100.0);
        let error = (detected_freq - expected).abs();

        assert!(
            error < 5.0,
            "Expected {}Hz (degree {}), got {}Hz",
            expected,
            if i == 0 { 0 } else { 7 },
            detected_freq
        );
    }
}

#[test]
fn test_scale_produces_audio() {
    // Basic sanity check that scale() produces audio
    let input = r#"
        cps: 2.0
        out $ sine(scale("0 1 2 3", "major", "c4")) * 0.3
    "#;

    let (_, statements) = parse_dsl(input).unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(22050);

    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(
        rms > 0.1,
        "Scale should produce audible output, got RMS={}",
        rms
    );
}

#[test]
fn test_scale_with_fast_pattern() {
    // Test scale with fast subdivision (arpeggio effect)
    let input = r#"
        cps: 1.0
        out $ sine(scale("0*4", "major", "60")) * 0.5
    "#;

    let (_, statements) = parse_dsl(input).unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(44100); // 1 second at 1 cps

    // Should produce 4 identical notes (C4) at 261.63 Hz
    let detected_freq = find_dominant_frequency(&buffer, 44100.0);
    assert!(
        (detected_freq - 261.63).abs() < 5.0,
        "Expected 261.63Hz, got {}Hz",
        detected_freq
    );
}
