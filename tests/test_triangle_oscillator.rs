/// Systematic tests: Triangle Oscillator
///
/// Tests triangle wave oscillator with spectral analysis and audio verification.
/// Triangle waves contain only odd harmonics with 1/n² amplitude falloff.
///
/// Key characteristics:
/// - Only odd harmonics (like square wave)
/// - 1/n² amplitude falloff (faster than square/saw)
/// - Mellower than square, closer to sine
/// - Smooth, soft sound
/// - Frequency pattern-modulated
/// - Used for flutes, soft leads, less aggressive bass

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use std::f32::consts::PI;

mod audio_test_utils;
use audio_test_utils::calculate_rms;

fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph = compile_program(statements, sample_rate, None).expect("Failed to compile DSL code");
    let num_samples = (duration * sample_rate) as usize;
    graph.render(num_samples)
}

/// Perform FFT and analyze spectrum
fn analyze_spectrum(buffer: &[f32], sample_rate: f32) -> (Vec<f32>, Vec<f32>) {
    use rustfft::{FftPlanner, num_complex::Complex};

    let fft_size = 8192.min(buffer.len());
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);

    let mut input: Vec<Complex<f32>> = buffer[..fft_size]
        .iter()
        .enumerate()
        .map(|(i, &sample)| {
            let window = 0.5 * (1.0 - (2.0 * PI * i as f32 / fft_size as f32).cos());
            Complex::new(sample * window, 0.0)
        })
        .collect();

    fft.process(&mut input);

    let magnitudes: Vec<f32> = input[..fft_size / 2]
        .iter()
        .map(|c| (c.re * c.re + c.im * c.im).sqrt())
        .collect();

    let frequencies: Vec<f32> = (0..fft_size / 2)
        .map(|i| i as f32 * sample_rate / fft_size as f32)
        .collect();

    (frequencies, magnitudes)
}

// ========== Basic Triangle Tests ==========

#[test]
fn test_triangle_compiles() {
    let code = r#"
        tempo: 0.5
        out $ triangle 440
    "#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);
    assert!(result.is_ok(), "Triangle should compile: {:?}", result.err());
}

#[test]
fn test_triangle_generates_audio() {
    let code = r#"
        tempo: 0.5
        out $ triangle 440 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.15, "Triangle should produce audio, got RMS: {}", rms);
    println!("Triangle RMS: {}", rms);
}

// ========== Spectral Content Tests ==========

#[test]
fn test_triangle_odd_harmonics_only() {
    // Triangle wave should have only odd harmonics (like square)
    let code = r#"
        tempo: 0.5
        out $ triangle 440 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    // Find fundamental (440 Hz) - odd harmonic
    let mut fundamental_mag = 0.0f32;
    for (i, &freq) in frequencies.iter().enumerate() {
        if (freq - 440.0).abs() < 20.0 {
            fundamental_mag = fundamental_mag.max(magnitudes[i]);
        }
    }

    // Find second harmonic (880 Hz) - even harmonic, should be weak
    let mut second_harmonic_mag = 0.0f32;
    for (i, &freq) in frequencies.iter().enumerate() {
        if (freq - 880.0).abs() < 20.0 {
            second_harmonic_mag = second_harmonic_mag.max(magnitudes[i]);
        }
    }

    // Find third harmonic (1320 Hz) - odd harmonic, should be present but weaker than square
    let mut third_harmonic_mag = 0.0f32;
    for (i, &freq) in frequencies.iter().enumerate() {
        if (freq - 1320.0).abs() < 20.0 {
            third_harmonic_mag = third_harmonic_mag.max(magnitudes[i]);
        }
    }

    assert!(fundamental_mag > 0.1,
        "Triangle should have strong fundamental, got {}",
        fundamental_mag);

    let even_to_odd_ratio = second_harmonic_mag / fundamental_mag.max(0.001);
    assert!(even_to_odd_ratio < 0.1,
        "Triangle should have weak even harmonics, 2nd/1st ratio: {}",
        even_to_odd_ratio);

    // Triangle should have weaker 3rd harmonic than square (1/9 vs 1/3)
    let third_to_fundamental = third_harmonic_mag / fundamental_mag.max(0.001);
    assert!(third_to_fundamental < 0.2,
        "Triangle should have weak 3rd harmonic, 3rd/1st ratio: {}",
        third_to_fundamental);

    println!("Harmonics - 1st: {}, 2nd: {}, 3rd: {}, 3rd/1st: {}",
        fundamental_mag, second_harmonic_mag, third_harmonic_mag, third_to_fundamental);
}

#[test]
fn test_triangle_softer_than_square() {
    // Triangle should have less high-frequency content than square
    let code_triangle = r#"
        tempo: 0.5
        out $ triangle 440 * 0.3
    "#;

    let code_square = r#"
        tempo: 0.5
        out $ square 440 * 0.3
    "#;

    let buffer_triangle = render_dsl(code_triangle, 1.0);
    let buffer_square = render_dsl(code_square, 1.0);

    let (frequencies, magnitudes_triangle) = analyze_spectrum(&buffer_triangle, 44100.0);
    let (_, magnitudes_square) = analyze_spectrum(&buffer_square, 44100.0);

    // Calculate high-frequency energy (above 1kHz)
    let high_energy_triangle: f32 = frequencies.iter()
        .zip(magnitudes_triangle.iter())
        .filter(|(f, _)| **f > 1000.0 && **f < 5000.0)
        .map(|(_, m)| m * m)
        .sum();

    let high_energy_square: f32 = frequencies.iter()
        .zip(magnitudes_square.iter())
        .filter(|(f, _)| **f > 1000.0 && **f < 5000.0)
        .map(|(_, m)| m * m)
        .sum();

    assert!(high_energy_square > high_energy_triangle,
        "Square should have more high-frequency content than triangle. Square: {}, Triangle: {}",
        high_energy_square, high_energy_triangle);

    println!("High freq energy - Triangle: {}, Square: {}", high_energy_triangle, high_energy_square);
}

// ========== Frequency Range Tests ==========

#[test]
fn test_triangle_bass() {
    let code = r#"
        tempo: 0.5
        out $ triangle 55 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.15, "Triangle bass should work, RMS: {}", rms);
    println!("Triangle bass RMS: {}", rms);
}

#[test]
fn test_triangle_mid_range() {
    let code = r#"
        tempo: 0.5
        out $ triangle 880 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.15, "Mid-range triangle should work, RMS: {}", rms);
    println!("Mid-range triangle RMS: {}", rms);
}

#[test]
fn test_triangle_high_frequency() {
    let code = r#"
        tempo: 0.5
        out $ triangle 2000 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.15, "High frequency triangle should work, RMS: {}", rms);
    println!("High frequency triangle RMS: {}", rms);
}

// ========== Musical Applications ==========

#[test]
fn test_triangle_soft_bass() {
    // Softer, mellower bass than square or saw
    let code = r#"
        tempo: 0.5
        ~env $ ad 0.01 0.3
        out $ triangle 55 * ~env * 0.4
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Triangle soft bass should work, RMS: {}", rms);
    println!("Triangle soft bass RMS: {}", rms);
}

#[test]
fn test_triangle_lead_synth() {
    // Smooth lead sound
    let code = r#"
        tempo: 0.5
        ~env $ ad 0.001 0.2
        out $ triangle 440 * ~env * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.03, "Triangle lead synth should work, RMS: {}", rms);
    println!("Triangle lead synth RMS: {}", rms);
}

#[test]
fn test_triangle_flute_simulation() {
    // Flute-like sound (flutes have few harmonics)
    let code = r#"
        tempo: 0.5
        ~env $ ad 0.05 0.3
        ~flute $ triangle 440 # rlpf 2500 1.0
        out $ ~flute * ~env * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.02, "Triangle flute simulation should work, RMS: {}", rms);
    println!("Triangle flute RMS: {}", rms);
}

#[test]
fn test_triangle_pad() {
    // Soft pad sound
    let code = r#"
        tempo: 1.0
        ~env $ ad 0.5 0.5
        ~pad $ triangle 220 # rlpf 1500 1.0
        out $ ~pad * ~env * 0.2
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.02, "Triangle pad should work, RMS: {}", rms);
    println!("Triangle pad RMS: {}", rms);
}

#[test]
fn test_triangle_lfo() {
    // Triangle as LFO (smoother than square LFO)
    let code = r#"
        tempo: 0.5
        ~lfo $ triangle 0.5
        ~freq $ ~lfo * 100 + 440
        out $ sine ~freq * 0.3
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "Triangle LFO should work, RMS: {}", rms);
    println!("Triangle LFO RMS: {}", rms);
}

// ========== Pattern Modulation Tests ==========

#[test]
fn test_triangle_pattern_frequency() {
    let code = r#"
        tempo: 0.5
        ~freq $ sine 1 * 100 + 440
        out $ triangle ~freq * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1,
        "Triangle with pattern-modulated frequency should work, RMS: {}",
        rms);

    println!("Pattern frequency RMS: {}", rms);
}

#[test]
fn test_triangle_pattern_amplitude() {
    let code = r#"
        tempo: 0.5
        ~amp $ sine 2 * 0.2 + 0.3
        out $ triangle 440 * ~amp
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05,
        "Triangle with pattern-modulated amplitude should work, RMS: {}",
        rms);

    println!("Pattern amplitude RMS: {}", rms);
}

// ========== Filtered Triangle Tests ==========

#[test]
fn test_triangle_lowpass_filter() {
    // Lowpassed triangle becomes even more sine-like
    let code = r#"
        tempo: 0.5
        ~filtered $ triangle 220 # rlpf 600 2.0
        out $ ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Lowpassed triangle should work, RMS: {}", rms);
    println!("Lowpassed triangle RMS: {}", rms);
}

#[test]
fn test_triangle_highpass_filter() {
    // Highpassed triangle
    let code = r#"
        tempo: 0.5
        ~filtered $ triangle 220 # rhpf 300 2.0
        out $ ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.03, "Highpassed triangle should work, RMS: {}", rms);
    println!("Highpassed triangle RMS: {}", rms);
}

#[test]
fn test_triangle_resonant_filter() {
    // Triangle through resonant filter
    let code = r#"
        tempo: 0.5
        ~env $ ad 0.01 0.2
        ~cutoff $ ~env * 2500 + 300
        ~synth $ triangle 110 # rlpf ~cutoff 8.0
        out $ ~synth * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.01, "Triangle with resonant filter should work, RMS: {}", rms);
    println!("Triangle + resonant filter RMS: {}", rms);
}

// ========== Stability Tests ==========

#[test]
fn test_triangle_no_clipping() {
    let code = r#"
        tempo: 0.5
        out $ triangle 440 * 0.5
    "#;

    let buffer = render_dsl(code, 1.0);
    let max_amplitude = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    assert!(max_amplitude <= 0.7,
        "Triangle should not clip, max: {}",
        max_amplitude);

    println!("Triangle max amplitude: {}", max_amplitude);
}

#[test]
fn test_triangle_dc_offset() {
    // Triangle should have no DC offset (symmetric waveform)
    let code = r#"
        tempo: 0.5
        out $ triangle 440 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let mean: f32 = buffer.iter().sum::<f32>() / buffer.len() as f32;

    assert!(mean.abs() < 0.01,
        "Triangle should have no DC offset, mean: {}",
        mean);

    println!("Triangle DC offset: {}", mean);
}

// ========== Phase Continuity Tests ==========

#[test]
fn test_triangle_continuous() {
    // Triangle should be smooth and continuous
    let code = r#"
        tempo: 0.5
        out $ triangle 440 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);

    // Check for discontinuities (triangle should be very smooth)
    let mut max_diff = 0.0f32;
    for i in 1..buffer.len() {
        let diff = (buffer[i] - buffer[i-1]).abs();
        max_diff = max_diff.max(diff);
    }

    // Triangle should be smoother than square or saw
    assert!(max_diff < 0.02,
        "Triangle should be very smooth, max diff: {}",
        max_diff);

    println!("Triangle max sample-to-sample diff: {}", max_diff);
}

// ========== Comparison Tests ==========

#[test]
fn test_triangle_closer_to_sine_than_square() {
    // Triangle should have less harmonic content than square, closer to sine
    let code_triangle = r#"
        tempo: 0.5
        out $ triangle 440 * 0.3
    "#;

    let code_square = r#"
        tempo: 0.5
        out $ square 440 * 0.3
    "#;

    let code_sine = r#"
        tempo: 0.5
        out $ sine 440 * 0.3
    "#;

    let buffer_triangle = render_dsl(code_triangle, 1.0);
    let buffer_square = render_dsl(code_square, 1.0);
    let buffer_sine = render_dsl(code_sine, 1.0);

    let (frequencies, magnitudes_triangle) = analyze_spectrum(&buffer_triangle, 44100.0);
    let (_, magnitudes_square) = analyze_spectrum(&buffer_square, 44100.0);
    let (_, magnitudes_sine) = analyze_spectrum(&buffer_sine, 44100.0);

    // Calculate total harmonic energy (excluding fundamental)
    let harmonic_energy_triangle: f32 = frequencies.iter()
        .zip(magnitudes_triangle.iter())
        .filter(|(f, _)| **f > 500.0 && **f < 5000.0)
        .map(|(_, m)| m * m)
        .sum();

    let harmonic_energy_square: f32 = frequencies.iter()
        .zip(magnitudes_square.iter())
        .filter(|(f, _)| **f > 500.0 && **f < 5000.0)
        .map(|(_, m)| m * m)
        .sum();

    let harmonic_energy_sine: f32 = frequencies.iter()
        .zip(magnitudes_sine.iter())
        .filter(|(f, _)| **f > 500.0 && **f < 5000.0)
        .map(|(_, m)| m * m)
        .sum();

    // Triangle should be between sine and square
    assert!(harmonic_energy_triangle > harmonic_energy_sine,
        "Triangle should have more harmonics than sine");
    assert!(harmonic_energy_triangle < harmonic_energy_square,
        "Triangle should have fewer harmonics than square");

    println!("Harmonic energy - Sine: {}, Triangle: {}, Square: {}",
        harmonic_energy_sine, harmonic_energy_triangle, harmonic_energy_square);
}
