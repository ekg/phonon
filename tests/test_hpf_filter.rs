/// Systematic tests: Highpass Filter (HPF)
///
/// Tests highpass filter with frequency response analysis and audio verification.
/// Highpass filters attenuate frequencies below the cutoff frequency.
///
/// Key characteristics:
/// - Passes high frequencies, attenuates low frequencies
/// - Cutoff frequency (-3dB point)
/// - Rolloff slope (dB per octave)
/// - Used for removing rumble, thinning bass, brightening sounds

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use std::f32::consts::PI;

mod audio_test_utils;
use audio_test_utils::calculate_rms;

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

// ========== Basic HPF Tests ==========

#[test]
fn test_hpf_compiles() {
    let code = r#"
        tempo: 2.0
        ~filtered: sine 440 # hpf 200
        o1: ~filtered
    "#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0);
    assert!(result.is_ok(), "HPF should compile: {:?}", result.err());
}

#[test]
fn test_hpf_generates_audio() {
    let code = r#"
        tempo: 2.0
        ~filtered: sine 440 # hpf 200
        o1: ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "HPF should produce audio, got RMS: {}", rms);
    println!("HPF RMS: {}", rms);
}

// ========== Frequency Response Tests ==========

#[test]
fn test_hpf_passes_high_frequencies() {
    // Sine at 2000Hz through 500Hz HPF should pass mostly unaffected
    let code_filtered = r#"
        tempo: 2.0
        ~filtered: sine 2000 # hpf 500
        o1: ~filtered * 0.3
    "#;

    let code_unfiltered = r#"
        tempo: 2.0
        o1: sine 2000 * 0.3
    "#;

    let buffer_filtered = render_dsl(code_filtered, 1.0);
    let buffer_unfiltered = render_dsl(code_unfiltered, 1.0);

    let rms_filtered = calculate_rms(&buffer_filtered);
    let rms_unfiltered = calculate_rms(&buffer_unfiltered);

    let attenuation = rms_filtered / rms_unfiltered;
    
    assert!(attenuation > 0.7,
        "HPF should pass high frequencies mostly unaffected, attenuation: {}",
        attenuation);

    println!("High frequency attenuation: {}", attenuation);
}

#[test]
fn test_hpf_attenuates_low_frequencies() {
    // Sine at 100Hz through 1000Hz HPF should be heavily attenuated
    let code_filtered = r#"
        tempo: 2.0
        ~filtered: sine 100 # hpf 1000
        o1: ~filtered * 0.3
    "#;

    let code_unfiltered = r#"
        tempo: 2.0
        o1: sine 100 * 0.3
    "#;

    let buffer_filtered = render_dsl(code_filtered, 1.0);
    let buffer_unfiltered = render_dsl(code_unfiltered, 1.0);

    let rms_filtered = calculate_rms(&buffer_filtered);
    let rms_unfiltered = calculate_rms(&buffer_unfiltered);

    let attenuation = rms_filtered / rms_unfiltered;
    
    assert!(attenuation < 0.5,
        "HPF should attenuate low frequencies, attenuation: {}",
        attenuation);

    println!("Low frequency attenuation: {}", attenuation);
}

#[test]
fn test_hpf_frequency_response_curve() {
    // Test HPF response across frequency spectrum using white noise
    let code_filtered = r#"
        tempo: 2.0
        ~filtered: white_noise # hpf 2000
        o1: ~filtered * 0.3
    "#;

    let code_unfiltered = r#"
        tempo: 2.0
        o1: white_noise * 0.3
    "#;

    let buffer_filtered = render_dsl(code_filtered, 2.0);
    let buffer_unfiltered = render_dsl(code_unfiltered, 2.0);

    let (frequencies, magnitudes_filtered) = analyze_spectrum(&buffer_filtered, 44100.0);
    let (_, _magnitudes_unfiltered) = analyze_spectrum(&buffer_unfiltered, 44100.0);

    // Calculate energy in frequency bands
    let below_cutoff: f32 = frequencies.iter()
        .zip(magnitudes_filtered.iter())
        .filter(|(f, _)| **f < 1000.0)
        .map(|(_, m)| m * m)
        .sum();

    let above_cutoff: f32 = frequencies.iter()
        .zip(magnitudes_filtered.iter())
        .filter(|(f, _)| **f > 4000.0 && **f < 12000.0)
        .map(|(_, m)| m * m)
        .sum();

    let ratio = above_cutoff / below_cutoff.max(0.001);
    
    assert!(ratio > 2.0,
        "HPF should favor frequencies above cutoff, ratio: {}",
        ratio);

    println!("Above/below cutoff energy ratio: {}", ratio);
}

// ========== Cutoff Frequency Tests ==========

#[test]
fn test_hpf_cutoff_200() {
    // Low cutoff - removes sub-bass
    let code = r#"
        tempo: 2.0
        ~filtered: white_noise # hpf 200
        o1: ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    let low: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f < 150.0)
        .map(|(_, m)| m * m)
        .sum();

    let mid: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 500.0 && **f < 2000.0)
        .map(|(_, m)| m * m)
        .sum();

    assert!(mid > low * 2.0,
        "HPF 200Hz should favor mid over low frequencies, mid: {}, low: {}",
        mid, low);

    println!("HPF 200Hz - Mid: {}, Low: {}", mid, low);
}

#[test]
fn test_hpf_cutoff_1000() {
    let code = r#"
        tempo: 2.0
        ~filtered: white_noise # hpf 1000
        o1: ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    let low: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f < 800.0)
        .map(|(_, m)| m * m)
        .sum();

    let high: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 2000.0 && **f < 8000.0)
        .map(|(_, m)| m * m)
        .sum();

    assert!(high > low,
        "HPF 1000Hz should favor high over low frequencies, high: {}, low: {}",
        high, low);

    println!("HPF 1000Hz - High: {}, Low: {}", high, low);
}

#[test]
fn test_hpf_cutoff_5000() {
    // Very high cutoff - only brightest frequencies pass
    let code = r#"
        tempo: 2.0
        ~filtered: white_noise # hpf 5000
        o1: ~filtered * 0.5
    "#;

    let buffer = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    let below: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f < 4000.0)
        .map(|(_, m)| m * m)
        .sum();

    let above: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 6000.0 && **f < 15000.0)
        .map(|(_, m)| m * m)
        .sum();

    assert!(above > below,
        "HPF 5000Hz should strongly favor high frequencies, above: {}, below: {}",
        above, below);

    println!("HPF 5000Hz - Above: {}, Below: {}", above, below);
}

// ========== Musical Applications ==========

#[test]
fn test_hpf_removing_rumble() {
    // Remove low-frequency rumble from bass
    let code = r#"
        tempo: 2.0
        ~env: ad 0.01 0.3
        ~bass: saw 55 # hpf 40
        o1: ~bass * ~env * 0.4
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "HPF removing rumble should work, RMS: {}", rms);
    println!("Rumble removal RMS: {}", rms);
}

#[test]
fn test_hpf_thin_bass() {
    // Thin out bass for more clarity
    let code = r#"
        tempo: 2.0
        ~env: ad 0.01 0.3
        ~bass: saw 110 # hpf 150
        o1: ~bass * ~env * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.03, "HPF thinning bass should work, RMS: {}", rms);
    println!("Thin bass RMS: {}", rms);
}

#[test]
fn test_hpf_air_and_sparkle() {
    // High HPF adds air and sparkle
    let code = r#"
        tempo: 2.0
        ~sparkle: white_noise # hpf 8000
        o1: ~sparkle * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    let very_high: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 8000.0 && **f < 16000.0)
        .map(|(_, m)| m * m)
        .sum();

    assert!(very_high > 0.01,
        "HPF air effect should have high-frequency content: {}",
        very_high);

    println!("Very high frequency content: {}", very_high);
}

#[test]
fn test_hpf_telephone_voice() {
    // Telephone voice effect (HPF + LPF)
    let code = r#"
        tempo: 2.0
        ~voice: sine 250 # hpf 300
        o1: ~voice * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    // Should be attenuated since 250Hz is below 300Hz cutoff
    assert!(rms < 0.25,
        "HPF telephone effect should attenuate, RMS: {}",
        rms);

    println!("Telephone voice RMS: {}", rms);
}

// ========== Pattern Modulation Tests ==========

#[test]
fn test_hpf_swept_cutoff() {
    // Cutoff frequency modulated by LFO
    let code = r#"
        tempo: 2.0
        ~lfo: sine 0.5 * 500 + 1000
        ~swept: white_noise # hpf ~lfo
        o1: ~swept * 0.3
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.03,
        "HPF with swept cutoff should work, RMS: {}",
        rms);

    println!("Swept cutoff RMS: {}", rms);
}

#[test]
fn test_hpf_envelope_controlled_cutoff() {
    // Envelope opens HPF cutoff
    let code = r#"
        tempo: 2.0
        ~env: ad 0.02 0.3
        ~cutoff: ~env * 2000 + 100
        ~synth: saw 110 # hpf ~cutoff
        o1: ~synth * ~env * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.02,
        "HPF with envelope-controlled cutoff should work, RMS: {}",
        rms);

    println!("Envelope-controlled cutoff RMS: {}", rms);
}

// ========== Cascaded Filters ==========

#[test]
fn test_hpf_cascade() {
    // Two HPFs in series create steeper rolloff
    let code = r#"
        tempo: 2.0
        ~filtered: white_noise # hpf 2000 # hpf 2000
        o1: ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    let below: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f < 1000.0)
        .map(|(_, m)| m * m)
        .sum();

    let above: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 5000.0 && **f < 12000.0)
        .map(|(_, m)| m * m)
        .sum();

    let ratio = above / below.max(0.001);
    
    assert!(ratio > 5.0,
        "Cascaded HPFs should have steeper rolloff, ratio: {}",
        ratio);

    println!("Cascaded HPF above/below ratio: {}", ratio);
}

#[test]
fn test_hpf_lpf_bandpass() {
    // HPF + LPF creates bandpass effect
    let code = r#"
        tempo: 2.0
        ~bandpass: white_noise # hpf 500 # lpf 2000
        o1: ~bandpass * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    let passband: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f > 800.0 && **f < 1800.0)
        .map(|(_, m)| m * m)
        .sum();

    let stopband: f32 = frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f < 300.0 || (**f > 3000.0 && **f < 10000.0))
        .map(|(_, m)| m * m)
        .sum();

    let ratio = passband / stopband.max(0.001);
    
    assert!(ratio > 1.5,
        "HPF+LPF should favor passband, ratio: {}",
        ratio);

    println!("HPF+LPF passband/stopband ratio: {}", ratio);
}

// ========== Stability Tests ==========

#[test]
fn test_hpf_no_clipping() {
    let code = r#"
        tempo: 2.0
        ~filtered: sine 2000 # hpf 500
        o1: ~filtered * 0.5
    "#;

    let buffer = render_dsl(code, 1.0);
    let max_amplitude = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    assert!(max_amplitude <= 0.7,
        "HPF should not cause clipping, max: {}",
        max_amplitude);

    println!("HPF max amplitude: {}", max_amplitude);
}

#[test]
fn test_hpf_consistent_output() {
    let code = r#"
        tempo: 2.0
        ~filtered: sine 1000 # hpf 500
        o1: ~filtered * 0.3
    "#;

    let buffer1 = render_dsl(code, 0.1);
    let buffer2 = render_dsl(code, 0.1);

    // Buffers should be identical (deterministic)
    let mut identical = 0;
    for i in 0..buffer1.len().min(buffer2.len()) {
        if (buffer1[i] - buffer2[i]).abs() < 0.0001 {
            identical += 1;
        }
    }

    let identity_ratio = identical as f32 / buffer1.len() as f32;
    assert!(identity_ratio > 0.99,
        "HPF should produce consistent output, identity: {}",
        identity_ratio);

    println!("HPF identity ratio: {}", identity_ratio);
}

// ========== Edge Cases ==========

#[test]
fn test_hpf_very_low_cutoff() {
    // Very low cutoff passes almost everything
    let code = r#"
        tempo: 2.0
        ~filtered: white_noise # hpf 20
        o1: ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05,
        "HPF 20Hz should pass most audio, RMS: {}",
        rms);

    println!("HPF 20Hz RMS: {}", rms);
}
