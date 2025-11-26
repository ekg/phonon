/// Systematic tests: Resonz Filter
///
/// Tests resonz (resonant bandpass) filter with spectral analysis.
/// Resonz is a highly resonant bandpass filter with sharp peak response.
///
/// Key characteristics:
/// - Strong resonant peak at center frequency
/// - Steep attenuation outside passband
/// - High Q produces ringing/singing quality
/// - Pattern-modulated parameters
/// - Used for formants, vocal synthesis, and resonant effects

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use std::f32::consts::PI;

mod audio_test_utils;
use audio_test_utils::calculate_rms;

/// Helper function to render DSL code to audio buffer
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

// ========== Basic Resonz Tests ==========

#[test]
fn test_resonz_compiles() {
    let code = r#"
        tempo: 2.0
        o1: saw 110 # resonz 1000 10.0
    "#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0);
    assert!(result.is_ok(), "Resonz should compile: {:?}", result.err());
}

#[test]
fn test_resonz_generates_audio() {
    let code = r#"
        tempo: 2.0
        o1: saw 110 # resonz 1000 10.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.01, "Resonz should produce audio, got RMS: {}", rms);
    println!("Resonz RMS: {}", rms);
}

// ========== Spectral Response Tests ==========

#[test]
fn test_resonz_passes_center_frequency() {
    let code = r#"
        tempo: 2.0
        o1: white_noise # resonz 1000 20.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    // Energy near center frequency (900-1100 Hz)
    let center_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 900.0 && **f < 1100.0)
        .map(|(_, m)| m * m)
        .sum();

    // Energy far from center (below 500 Hz or above 2000 Hz)
    let side_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f < 500.0 || **f > 2000.0)
        .map(|(_, m)| m * m)
        .sum();

    let ratio = center_energy / side_energy.max(0.001);
    assert!(ratio > 5.0,
        "Resonz should strongly pass center frequency, center/side ratio: {}",
        ratio);

    println!("Resonz - Center energy: {}, Side energy: {}, Ratio: {}", center_energy, side_energy, ratio);
}

#[test]
fn test_resonz_high_q_narrow_band() {
    // High Q should produce very narrow passband
    let code = r#"
        tempo: 2.0
        o1: white_noise # resonz 1000 50.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    // Very narrow band around center (950-1050 Hz)
    let narrow_center: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 950.0 && **f < 1050.0)
        .map(|(_, m)| m * m)
        .sum();

    // Slightly wider band (800-1200 Hz, excluding narrow center)
    let wider_band: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| (**f > 800.0 && **f < 950.0) || (**f > 1050.0 && **f < 1200.0))
        .map(|(_, m)| m * m)
        .sum();

    // With high Q, narrow center should dominate
    assert!(narrow_center > wider_band,
        "High Q resonz should have very narrow passband, narrow: {}, wider: {}",
        narrow_center, wider_band);

    println!("High Q - Narrow center: {}, Wider band: {}", narrow_center, wider_band);
}

#[test]
fn test_resonz_low_q_wider_band() {
    // Low Q should produce wider passband
    let code = r#"
        tempo: 2.0
        o1: white_noise # resonz 1000 2.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    // Wide band around center (700-1400 Hz)
    let wide_band: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 700.0 && **f < 1400.0)
        .map(|(_, m)| m * m)
        .sum();

    // Far from center
    let far_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f < 400.0 || **f > 2000.0)
        .map(|(_, m)| m * m)
        .sum();

    // Even with low Q, should still favor center
    assert!(wide_band > far_energy,
        "Low Q resonz should pass wider band, wide: {}, far: {}",
        wide_band, far_energy);

    println!("Low Q - Wide band: {}, Far: {}", wide_band, far_energy);
}

// ========== Pattern Modulation Tests ==========

#[test]
fn test_resonz_pattern_frequency() {
    let code = r#"
        tempo: 2.0
        ~lfo: sine 4 * 500 + 1000
        o1: saw 110 # resonz ~lfo 10.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.01,
        "Resonz with pattern-modulated frequency should work, RMS: {}",
        rms);

    println!("Resonz pattern frequency RMS: {}", rms);
}

#[test]
fn test_resonz_pattern_q() {
    let code = r#"
        tempo: 2.0
        ~lfo: sine 2 * 10.0 + 15.0
        o1: saw 110 # resonz 1000 ~lfo
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.01,
        "Resonz with pattern-modulated Q should work, RMS: {}",
        rms);

    println!("Resonz pattern Q RMS: {}", rms);
}

// ========== Stability Tests ==========

#[test]
fn test_resonz_no_clipping() {
    let code = r#"
        tempo: 2.0
        o1: saw 110 # resonz 1000 50.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let max_amplitude = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    // High Q resonz can have significant gain at resonance
    assert!(max_amplitude <= 5.0,
        "Resonz should not excessively clip, max: {}",
        max_amplitude);

    println!("Resonz high Q peak: {}", max_amplitude);
}

#[test]
fn test_resonz_no_dc_offset() {
    let code = r#"
        tempo: 2.0
        o1: saw 110 # resonz 500 10.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let mean: f32 = buffer.iter().sum::<f32>() / buffer.len() as f32;

    assert!(mean.abs() < 0.02, "Resonz should have no DC offset, got {}", mean);
    println!("Resonz DC offset: {}", mean);
}

// ========== Cascaded Filters ==========

#[test]
fn test_resonz_cascade() {
    let code = r#"
        tempo: 2.0
        o1: saw 110 # resonz 800 10.0 # resonz 1200 10.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    // Cascaded bandpass filters naturally attenuate more
    assert!(rms > 0.005, "Cascaded resonz should work, RMS: {}", rms);
    println!("Cascaded resonz RMS: {}", rms);
}

// ========== Musical Applications ==========

#[test]
fn test_resonz_formant_synthesis() {
    // Multiple resonz filters can create vocal formants
    let code = r#"
        tempo: 2.0
        ~src: saw 110
        ~f1: ~src # resonz 800 15.0
        ~f2: ~src # resonz 1200 15.0
        ~f3: ~src # resonz 2600 20.0
        o1: (~f1 + ~f2 + ~f3) * 0.3
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    // Multiple narrow bandpass filters produce lower output
    assert!(rms > 0.02, "Resonz formant synthesis should work, RMS: {}", rms);
    println!("Formant synthesis RMS: {}", rms);
}

#[test]
fn test_resonz_resonant_sweep() {
    let code = r#"
        tempo: 2.0
        ~sweep: line 200 5000
        o1: saw 55 # resonz ~sweep 20.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.01, "Resonz sweep should work, RMS: {}", rms);
    println!("Resonz sweep RMS: {}", rms);
}

#[test]
fn test_resonz_pluck_simulation() {
    // High Q resonz on noise burst simulates plucked string
    let code = r#"
        tempo: 2.0
        ~burst: white_noise * (line 1.0 0.0)
        o1: ~burst # resonz 440 40.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    // Decaying envelope + narrow bandpass = lower RMS
    assert!(rms > 0.005, "Resonz pluck should work, RMS: {}", rms);
    println!("Pluck simulation RMS: {}", rms);
}

// ========== Edge Cases ==========

#[test]
fn test_resonz_very_low_frequency() {
    let code = r#"
        tempo: 2.0
        o1: white_noise # resonz 50 10.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    // Low frequency bandpass on white noise = less energy
    assert!(rms > 0.005, "Resonz should work at very low frequencies, RMS: {}", rms);
    println!("Resonz very low frequency RMS: {}", rms);
}

#[test]
fn test_resonz_very_high_frequency() {
    let code = r#"
        tempo: 2.0
        o1: white_noise # resonz 15000 10.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.01, "Resonz should work at very high frequencies, RMS: {}", rms);
    println!("Resonz very high frequency RMS: {}", rms);
}

#[test]
fn test_resonz_extreme_high_q() {
    let code = r#"
        tempo: 2.0
        o1: saw 110 # resonz 1000 100.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.01, "Resonz should work with extreme Q, RMS: {}", rms);
    println!("Resonz extreme Q RMS: {}", rms);
}

#[test]
fn test_resonz_low_q() {
    let code = r#"
        tempo: 2.0
        o1: saw 110 # resonz 1000 1.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Resonz should work with low Q, RMS: {}", rms);
    println!("Resonz low Q RMS: {}", rms);
}
