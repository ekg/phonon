/// Systematic tests: RLPF (Resonant Lowpass Filter)
///
/// Tests resonant lowpass filter with spectral analysis.
/// RLPF is a classic analog synthesizer filter with resonant peak at cutoff.
///
/// Key characteristics:
/// - Passes low frequencies
/// - Attenuates high frequencies
/// - Resonant peak at cutoff frequency
/// - High Q produces self-oscillation at cutoff
/// - Pattern-modulated parameters
/// - Used for classic analog synth sounds

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

// ========== Basic RLPF Tests ==========

#[test]
fn test_rlpf_compiles() {
    let code = r#"
        tempo: 0.5
        out $ saw 110 # rlpf 1000 5.0
    "#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);
    assert!(result.is_ok(), "RLPF should compile: {:?}", result.err());
}

#[test]
fn test_rlpf_generates_audio() {
    let code = r#"
        tempo: 0.5
        out $ saw 110 # rlpf 1000 2.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "RLPF should produce audio, got RMS: {}", rms);
    println!("RLPF RMS: {}", rms);
}

// ========== Lowpass Response Tests ==========

#[test]
fn test_rlpf_attenuates_highs() {
    let code = r#"
        tempo: 0.5
        out $ white_noise # rlpf 1000 1.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    let low_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f < 800.0)
        .map(|(_, m)| m * m)
        .sum();

    let high_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 2000.0)
        .map(|(_, m)| m * m)
        .sum();

    let ratio = low_energy / high_energy;
    assert!(ratio > 2.0,
        "RLPF should attenuate high frequencies, low/high ratio: {}",
        ratio);

    println!("RLPF - Low energy: {}, High energy: {}, Ratio: {}", low_energy, high_energy, ratio);
}

// ========== Resonance Tests ==========

#[test]
fn test_rlpf_resonance_peak() {
    // High Q should create resonant peak at cutoff
    let code = r#"
        tempo: 0.5
        out $ white_noise # rlpf 1000 10.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    // Find peak near cutoff (900-1100 Hz)
    let mut peak_mag = 0.0f32;
    for (i, &freq) in frequencies.iter().enumerate() {
        if freq > 900.0 && freq < 1100.0 {
            peak_mag = peak_mag.max(magnitudes[i]);
        }
    }

    // Compare to average magnitude in passband (200-700 Hz)
    let mut passband_sum = 0.0f32;
    let mut passband_count = 0;
    for (i, &freq) in frequencies.iter().enumerate() {
        if freq > 200.0 && freq < 700.0 {
            passband_sum += magnitudes[i];
            passband_count += 1;
        }
    }
    let passband_avg = passband_sum / passband_count as f32;

    // Resonant peak should be significantly higher
    assert!(peak_mag > passband_avg * 1.5,
        "RLPF with high Q should have resonant peak, peak: {}, passband avg: {}",
        peak_mag, passband_avg);

    println!("Resonance - Peak: {}, Passband avg: {}, Ratio: {}", peak_mag, passband_avg, peak_mag / passband_avg);
}

#[test]
fn test_rlpf_low_q_smooth() {
    // Low Q should have smooth rolloff without peak
    let code = r#"
        tempo: 0.5
        out $ white_noise # rlpf 1000 0.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    // Just verify it produces audio
    assert!(rms > 0.1, "RLPF with low Q should work, RMS: {}", rms);
    println!("Low Q RLPF RMS: {}", rms);
}

#[test]
fn test_rlpf_extreme_resonance() {
    // Very high Q can self-oscillate
    let code = r#"
        tempo: 0.5
        out $ saw 110 # rlpf 440 20.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "RLPF with extreme Q should work, RMS: {}", rms);
    println!("Extreme Q RLPF RMS: {}", rms);
}

// ========== Pattern Modulation Tests ==========

#[test]
fn test_rlpf_pattern_cutoff() {
    let code = r#"
        tempo: 0.5
        ~lfo $ sine 4 * 1000 + 1500
        out $ saw 110 # rlpf ~lfo 2.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1,
        "RLPF with pattern-modulated cutoff should work, RMS: {}",
        rms);

    println!("RLPF pattern cutoff RMS: {}", rms);
}

#[test]
fn test_rlpf_pattern_resonance() {
    let code = r#"
        tempo: 0.5
        ~lfo $ sine 2 * 5.0 + 7.0
        out $ saw 110 # rlpf 1000 ~lfo
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1,
        "RLPF with pattern-modulated resonance should work, RMS: {}",
        rms);

    println!("RLPF pattern resonance RMS: {}", rms);
}

// ========== Stability Tests ==========

#[test]
fn test_rlpf_no_clipping() {
    let code = r#"
        tempo: 0.5
        out $ saw 110 # rlpf 1000 15.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let max_amplitude = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    // High Q can boost near cutoff
    assert!(max_amplitude <= 5.0,
        "RLPF should not excessively clip, max: {}",
        max_amplitude);

    println!("RLPF high Q peak: {}", max_amplitude);
}

#[test]
fn test_rlpf_no_dc_offset() {
    let code = r#"
        tempo: 0.5
        out $ saw 110 # rlpf 500 2.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let mean: f32 = buffer.iter().sum::<f32>() / buffer.len() as f32;

    assert!(mean.abs() < 0.02, "RLPF should have no DC offset, got {}", mean);
    println!("RLPF DC offset: {}", mean);
}

// ========== Musical Applications ==========

#[test]
fn test_rlpf_classic_filter_sweep() {
    // Classic analog synth filter sweep
    let code = r#"
        tempo: 0.5
        ~sweep $ line 100 5000
        out $ saw 55 # rlpf ~sweep 5.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "RLPF sweep should work, RMS: {}", rms);
    println!("Classic filter sweep RMS: {}", rms);
}

#[test]
fn test_rlpf_resonant_bass() {
    let code = r#"
        tempo: 0.5
        out $ saw 55 # rlpf 200 8.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "RLPF resonant bass should work, RMS: {}", rms);
    println!("Resonant bass RMS: {}", rms);
}

#[test]
fn test_rlpf_acid_bassline() {
    // Typical acid house filter settings
    let code = r#"
        tempo: 0.5
        ~sweep $ line 50 2000
        out $ saw 55 # rlpf ~sweep 12.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "RLPF acid bassline should work, RMS: {}", rms);
    println!("Acid bassline RMS: {}", rms);
}

// ========== Cascaded Filters ==========

#[test]
fn test_rlpf_cascade() {
    // Two RLPF in series for steeper rolloff
    let code = r#"
        tempo: 0.5
        out $ saw 110 # rlpf 1000 2.0 # rlpf 1200 2.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.08, "Cascaded RLPF should work, RMS: {}", rms);
    println!("Cascaded RLPF RMS: {}", rms);
}

// ========== Edge Cases ==========

#[test]
fn test_rlpf_very_low_cutoff() {
    let code = r#"
        tempo: 0.5
        out $ white_noise # rlpf 50 2.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.01, "RLPF should work at very low cutoff, RMS: {}", rms);
    println!("RLPF very low cutoff RMS: {}", rms);
}

#[test]
fn test_rlpf_very_high_cutoff() {
    let code = r#"
        tempo: 0.5
        out $ white_noise # rlpf 15000 2.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.2, "RLPF should work at very high cutoff, RMS: {}", rms);
    println!("RLPF very high cutoff RMS: {}", rms);
}
