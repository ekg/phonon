/// Systematic tests: RHPF (Resonant Highpass Filter)
///
/// Tests resonant highpass filter with spectral analysis.
/// RHPF is a resonant highpass filter with peak at cutoff frequency.
///
/// Key characteristics:
/// - Attenuates low frequencies
/// - Passes high frequencies
/// - Resonant peak at cutoff frequency
/// - High Q produces ringing at cutoff
/// - Pattern-modulated parameters
/// - Used for removing low end, creating air, and rhythmic filtering

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use std::f32::consts::PI;

mod audio_test_utils;
use audio_test_utils::calculate_rms;

/// Helper function to render DSL code to audio buffer
fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph = compile_program(statements, sample_rate).expect("Failed to compile DSL code");
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

// ========== Basic RHPF Tests ==========

#[test]
fn test_rhpf_compiles() {
    let code = r#"
        tempo: 2.0
        o1: saw 110 # rhpf 1000 5.0
    "#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0);
    assert!(result.is_ok(), "RHPF should compile: {:?}", result.err());
}

#[test]
fn test_rhpf_generates_audio() {
    let code = r#"
        tempo: 2.0
        o1: saw 110 # rhpf 500 2.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "RHPF should produce audio, got RMS: {}", rms);
    println!("RHPF RMS: {}", rms);
}

// ========== Highpass Response Tests ==========

#[test]
fn test_rhpf_attenuates_lows() {
    let code = r#"
        tempo: 2.0
        o1: white_noise # rhpf 1000 1.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    let low_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f < 500.0)
        .map(|(_, m)| m * m)
        .sum();

    let high_energy: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 2000.0)
        .map(|(_, m)| m * m)
        .sum();

    let ratio = high_energy / low_energy;
    assert!(ratio > 2.0,
        "RHPF should attenuate low frequencies, high/low ratio: {}",
        ratio);

    println!("RHPF - Low energy: {}, High energy: {}, Ratio: {}", low_energy, high_energy, ratio);
}

// ========== Resonance Tests ==========

#[test]
fn test_rhpf_resonance_peak() {
    // High Q should create resonant peak at cutoff
    let code = r#"
        tempo: 2.0
        o1: white_noise # rhpf 1000 10.0
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

    // Compare to average magnitude in passband (3000-5000 Hz)
    let mut passband_sum = 0.0f32;
    let mut passband_count = 0;
    for (i, &freq) in frequencies.iter().enumerate() {
        if freq > 3000.0 && freq < 5000.0 {
            passband_sum += magnitudes[i];
            passband_count += 1;
        }
    }
    let passband_avg = passband_sum / passband_count as f32;

    // Resonant peak should be significantly higher
    assert!(peak_mag > passband_avg * 1.5,
        "RHPF with high Q should have resonant peak, peak: {}, passband avg: {}",
        peak_mag, passband_avg);

    println!("Resonance - Peak: {}, Passband avg: {}, Ratio: {}", peak_mag, passband_avg, peak_mag / passband_avg);
}

#[test]
fn test_rhpf_low_q_smooth() {
    // Low Q should have smooth rolloff without peak
    let code = r#"
        tempo: 2.0
        o1: white_noise # rhpf 1000 0.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    // Just verify it produces audio
    assert!(rms > 0.1, "RHPF with low Q should work, RMS: {}", rms);
    println!("Low Q RHPF RMS: {}", rms);
}

#[test]
fn test_rhpf_extreme_resonance() {
    // Very high Q can self-oscillate
    let code = r#"
        tempo: 2.0
        o1: saw 110 # rhpf 440 20.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.02, "RHPF with extreme Q should work, RMS: {}", rms);
    println!("Extreme Q RHPF RMS: {}", rms);
}

// ========== Pattern Modulation Tests ==========

#[test]
fn test_rhpf_pattern_cutoff() {
    let code = r#"
        tempo: 2.0
        ~lfo: sine 4 * 500 + 1000
        o1: saw 110 # rhpf ~lfo 2.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05,
        "RHPF with pattern-modulated cutoff should work, RMS: {}",
        rms);

    println!("RHPF pattern cutoff RMS: {}", rms);
}

#[test]
fn test_rhpf_pattern_resonance() {
    let code = r#"
        tempo: 2.0
        ~lfo: sine 2 * 5.0 + 7.0
        o1: saw 110 # rhpf 500 ~lfo
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05,
        "RHPF with pattern-modulated resonance should work, RMS: {}",
        rms);

    println!("RHPF pattern resonance RMS: {}", rms);
}

// ========== Stability Tests ==========

#[test]
fn test_rhpf_no_clipping() {
    let code = r#"
        tempo: 2.0
        o1: saw 110 # rhpf 500 15.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let max_amplitude = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    // High Q can boost near cutoff
    assert!(max_amplitude <= 5.0,
        "RHPF should not excessively clip, max: {}",
        max_amplitude);

    println!("RHPF high Q peak: {}", max_amplitude);
}

#[test]
fn test_rhpf_no_dc_offset() {
    let code = r#"
        tempo: 2.0
        o1: saw 110 # rhpf 500 2.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let mean: f32 = buffer.iter().sum::<f32>() / buffer.len() as f32;

    assert!(mean.abs() < 0.02, "RHPF should have no DC offset, got {}", mean);
    println!("RHPF DC offset: {}", mean);
}

// ========== Musical Applications ==========

#[test]
fn test_rhpf_remove_low_end() {
    // Clean up muddy low end
    let code = r#"
        tempo: 2.0
        o1: saw 220 # rhpf 150 1.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "RHPF removing low end should work, RMS: {}", rms);
    println!("Remove low end RMS: {}", rms);
}

#[test]
fn test_rhpf_create_air() {
    // High cutoff for airy sound
    let code = r#"
        tempo: 2.0
        o1: white_noise # rhpf 8000 2.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "RHPF creating air should work, RMS: {}", rms);
    println!("Create air RMS: {}", rms);
}

#[test]
fn test_rhpf_rhythmic_filtering() {
    // Sweeping highpass for rhythmic effect
    let code = r#"
        tempo: 2.0
        ~sweep: line 100 2000
        o1: white_noise # rhpf ~sweep 5.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "RHPF rhythmic filtering should work, RMS: {}", rms);
    println!("Rhythmic filtering RMS: {}", rms);
}

// ========== Cascaded Filters ==========

#[test]
fn test_rhpf_cascade() {
    // Two RHPF in series for steeper rolloff
    let code = r#"
        tempo: 2.0
        o1: saw 110 # rhpf 800 2.0 # rhpf 1000 2.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.03, "Cascaded RHPF should work, RMS: {}", rms);
    println!("Cascaded RHPF RMS: {}", rms);
}

// ========== Complement to RLPF ==========

#[test]
fn test_rhpf_rlpf_combo() {
    // RHPF + RLPF creates bandpass
    let code = r#"
        tempo: 2.0
        o1: white_noise # rhpf 500 1.0 # rlpf 2000 1.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "RHPF + RLPF combo should work, RMS: {}", rms);
    println!("RHPF + RLPF combo RMS: {}", rms);
}

// ========== Edge Cases ==========

#[test]
fn test_rhpf_very_low_cutoff() {
    let code = r#"
        tempo: 2.0
        o1: white_noise # rhpf 50 2.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.2, "RHPF should work at very low cutoff, RMS: {}", rms);
    println!("RHPF very low cutoff RMS: {}", rms);
}

#[test]
fn test_rhpf_very_high_cutoff() {
    let code = r#"
        tempo: 2.0
        o1: white_noise # rhpf 15000 2.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    // Very high cutoff removes almost everything
    assert!(rms > 0.01, "RHPF should work at very high cutoff, RMS: {}", rms);
    println!("RHPF very high cutoff RMS: {}", rms);
}
