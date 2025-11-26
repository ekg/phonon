/// Systematic tests: Allpass Filter
///
/// Tests allpass filter with phase analysis and audio quality verification.
/// An allpass filter passes all frequencies with unity gain but shifts their phase.
///
/// Key characteristics:
/// - Flat magnitude response (all frequencies pass through)
/// - Frequency-dependent phase shift
/// - Delay time controls phase shift amount
/// - No amplitude change (unity gain)
/// - Used in reverb, phasers, and delay networks

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
/// Returns (frequency_bins, magnitudes)
fn analyze_spectrum(buffer: &[f32], sample_rate: f32) -> (Vec<f32>, Vec<f32>) {
    use rustfft::{FftPlanner, num_complex::Complex};

    let fft_size = 8192.min(buffer.len());
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);

    // Prepare input with Hann window
    let mut input: Vec<Complex<f32>> = buffer[..fft_size]
        .iter()
        .enumerate()
        .map(|(i, &sample)| {
            let window = 0.5 * (1.0 - (2.0 * PI * i as f32 / fft_size as f32).cos());
            Complex::new(sample * window, 0.0)
        })
        .collect();

    fft.process(&mut input);

    // Calculate magnitudes and frequencies
    let magnitudes: Vec<f32> = input[..fft_size / 2]
        .iter()
        .map(|c| (c.re * c.re + c.im * c.im).sqrt())
        .collect();

    let frequencies: Vec<f32> = (0..fft_size / 2)
        .map(|i| i as f32 * sample_rate / fft_size as f32)
        .collect();

    (frequencies, magnitudes)
}

// ========== Basic Allpass Tests ==========

#[test]
fn test_allpass_compiles() {
    let code = r#"
        tempo: 2.0
        o1: sine 440 # allpass 0.5
    "#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0);
    assert!(result.is_ok(), "Allpass should compile: {:?}", result.err());
}

#[test]
fn test_allpass_generates_audio() {
    let code = r#"
        tempo: 2.0
        o1: sine 440 # allpass 0.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    // Allpass should preserve amplitude (unity gain)
    assert!(rms > 0.2, "Allpass should produce audio, got RMS: {}", rms);

    println!("Allpass RMS: {}", rms);
}

#[test]
fn test_allpass_no_clipping() {
    let code = r#"
        tempo: 2.0
        o1: sine 440 # allpass 0.7
    "#;

    let buffer = render_dsl(code, 2.0);
    let max_amplitude = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    assert!(max_amplitude <= 1.0, "Allpass should not clip, max amplitude: {}", max_amplitude);

    println!("Allpass peak: {}", max_amplitude);
}

#[test]
fn test_allpass_no_dc_offset() {
    let code = r#"
        tempo: 2.0
        o1: sine 440 # allpass 0.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let mean: f32 = buffer.iter().sum::<f32>() / buffer.len() as f32;

    assert!(mean.abs() < 0.02, "Allpass should have no DC offset, got {}", mean);

    println!("Allpass DC offset: {}", mean);
}

// ========== Unity Gain Tests ==========

#[test]
fn test_allpass_unity_gain() {
    // Allpass should not change amplitude, only phase
    let dry_code = r#"
        tempo: 2.0
        o1: sine 440
    "#;

    let wet_code = r#"
        tempo: 2.0
        o1: sine 440 # allpass 0.5
    "#;

    let dry_buffer = render_dsl(dry_code, 2.0);
    let wet_buffer = render_dsl(wet_code, 2.0);

    let dry_rms = calculate_rms(&dry_buffer);
    let wet_rms = calculate_rms(&wet_buffer);

    // RMS should be similar (within 10%)
    let ratio = wet_rms / dry_rms;
    assert!((0.9..=1.1).contains(&ratio),
        "Allpass should preserve amplitude (unity gain), dry RMS: {}, wet RMS: {}, ratio: {}",
        dry_rms, wet_rms, ratio);

    println!("Unity gain test - Dry RMS: {}, Wet RMS: {}, Ratio: {}", dry_rms, wet_rms, ratio);
}

#[test]
fn test_allpass_flat_magnitude_response() {
    // Allpass should pass all frequencies equally
    let code = r#"
        tempo: 2.0
        o1: white_noise # allpass 0.3
    "#;

    let buffer = render_dsl(code, 2.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    // Compare with dry white noise
    let dry_code = r#"
        tempo: 2.0
        o1: white_noise
    "#;
    let dry_buffer = render_dsl(dry_code, 2.0);
    let (_, dry_magnitudes) = analyze_spectrum(&dry_buffer, 44100.0);

    // Calculate average magnitude ratio across all frequencies
    let mut total_ratio = 0.0;
    let mut count = 0;

    for (i, &freq) in frequencies.iter().enumerate() {
        if freq > 100.0 && freq < 18000.0 && dry_magnitudes[i] > 0.01 {
            let ratio = magnitudes[i] / dry_magnitudes[i];
            total_ratio += ratio;
            count += 1;
        }
    }

    let avg_ratio = total_ratio / count as f32;

    // Average ratio should be close to 1.0 (unity gain)
    // Allow wider range due to white noise randomness and filter ringing
    assert!((0.7..=1.6).contains(&avg_ratio),
        "Allpass should have flat magnitude response, avg ratio: {}",
        avg_ratio);

    println!("Flat magnitude response - Avg ratio: {}", avg_ratio);
}

// ========== Phase Shift Tests ==========

#[test]
fn test_allpass_changes_phase() {
    // Allpass should change phase but not amplitude
    let dry_code = r#"
        tempo: 2.0
        o1: sine 440
    "#;

    let wet_code = r#"
        tempo: 2.0
        o1: sine 440 # allpass 0.5
    "#;

    let dry_buffer = render_dsl(dry_code, 1.0);
    let wet_buffer = render_dsl(wet_code, 1.0);

    // Buffers should be different (phase shifted)
    let mut diff = 0.0;
    for i in 0..dry_buffer.len().min(wet_buffer.len()) {
        diff += (dry_buffer[i] - wet_buffer[i]).abs();
    }
    let avg_diff = diff / dry_buffer.len() as f32;

    // Should have noticeable difference due to phase shift
    assert!(avg_diff > 0.01,
        "Allpass should change phase, avg difference: {}",
        avg_diff);

    println!("Phase shift - Avg sample difference: {}", avg_diff);
}

#[test]
fn test_allpass_different_coefficients() {
    // Different coefficients should produce different phase shifts
    let code1 = r#"
        tempo: 2.0
        o1: sine 440 # allpass 0.3
    "#;

    let code2 = r#"
        tempo: 2.0
        o1: sine 440 # allpass 0.7
    "#;

    let buffer1 = render_dsl(code1, 1.0);
    let buffer2 = render_dsl(code2, 1.0);

    // Buffers should be different
    let mut diff = 0.0;
    for i in 0..buffer1.len().min(buffer2.len()) {
        diff += (buffer1[i] - buffer2[i]).abs();
    }
    let avg_diff = diff / buffer1.len() as f32;

    assert!(avg_diff > 0.01,
        "Different coefficients should produce different phase shifts, avg diff: {}",
        avg_diff);

    println!("Different coefficients - Avg difference: {}", avg_diff);
}

// ========== Pattern Modulation Tests ==========

#[test]
fn test_allpass_pattern_coefficient() {
    // Allpass should work with pattern-modulated coefficient
    let code = r#"
        tempo: 2.0
        ~lfo: sine 2 * 0.3 + 0.5
        o1: sine 440 # allpass ~lfo
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    // Should produce audio
    assert!(rms > 0.1,
        "Allpass with pattern coefficient should work, RMS: {}",
        rms);

    println!("Pattern-modulated allpass RMS: {}", rms);
}

// ========== Cascaded Allpass Tests ==========

#[test]
fn test_allpass_cascade() {
    // Multiple allpass filters in series
    let code = r#"
        tempo: 2.0
        o1: sine 440 # allpass 0.3 # allpass 0.5 # allpass 0.7
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    // Should still preserve amplitude (unity gain)
    assert!(rms > 0.2,
        "Cascaded allpass should preserve amplitude, RMS: {}",
        rms);

    println!("Cascaded allpass RMS: {}", rms);
}

// ========== Musical Applications ==========

#[test]
fn test_allpass_for_reverb() {
    // Allpass filters are key building blocks of reverb
    let code = r#"
        tempo: 2.0
        ~dry: saw 110
        ~ap1: ~dry # allpass 0.131
        ~ap2: ~ap1 # allpass 0.359
        ~ap3: ~ap2 # allpass 0.677
        o1: ~ap3 * 0.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    // Should produce smooth output
    assert!(rms > 0.1,
        "Allpass reverb chain should work, RMS: {}",
        rms);

    println!("Allpass reverb chain RMS: {}", rms);
}

#[test]
fn test_allpass_for_phaser() {
    // Allpass + dry signal creates phaser effect
    let code = r#"
        tempo: 2.0
        ~dry: saw 220
        ~wet: ~dry # allpass 0.5
        o1: (~dry + ~wet) * 0.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    // Should create phasing effect
    assert!(rms > 0.1,
        "Allpass phaser should work, RMS: {}",
        rms);

    println!("Allpass phaser RMS: {}", rms);
}

// ========== Edge Cases ==========

#[test]
fn test_allpass_zero_coefficient() {
    // Coefficient of 0 should pass signal through unchanged
    let dry_code = r#"
        tempo: 2.0
        o1: sine 440
    "#;

    let wet_code = r#"
        tempo: 2.0
        o1: sine 440 # allpass 0.0
    "#;

    let dry_buffer = render_dsl(dry_code, 1.0);
    let wet_buffer = render_dsl(wet_code, 1.0);

    let dry_rms = calculate_rms(&dry_buffer);
    let wet_rms = calculate_rms(&wet_buffer);

    // Should be very similar
    assert!((wet_rms / dry_rms - 1.0).abs() < 0.1,
        "Allpass(0) should be nearly transparent");

    println!("Zero coefficient - Dry: {}, Wet: {}", dry_rms, wet_rms);
}

#[test]
fn test_allpass_with_noise() {
    let code = r#"
        tempo: 2.0
        o1: white_noise # allpass 0.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    // Noise should pass through with preserved energy
    assert!(rms > 0.2,
        "Allpass should preserve noise energy, RMS: {}",
        rms);

    println!("Allpass with noise RMS: {}", rms);
}
