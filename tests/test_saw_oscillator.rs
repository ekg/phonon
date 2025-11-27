/// Systematic tests: Sawtooth Oscillator
///
/// Tests sawtooth wave oscillator with spectral analysis and audio verification.
/// Sawtooth waves contain all harmonics (odd and even) with 1/n amplitude falloff.
///
/// Key characteristics:
/// - All harmonics present (rich harmonic content)
/// - 1/n amplitude falloff
/// - Bright, buzzy sound
/// - Linear waveform (ramp from -1 to +1)
/// - Frequency pattern-modulated
/// - Used for leads, bass, strings, brass

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

// ========== Basic Saw Tests ==========

#[test]
fn test_saw_compiles() {
    let code = r#"
        tempo: 0.5
        o1: saw 440
    "#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);
    assert!(result.is_ok(), "Saw should compile: {:?}", result.err());
}

#[test]
fn test_saw_generates_audio() {
    let code = r#"
        tempo: 0.5
        o1: saw 440 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.15, "Saw should produce audio, got RMS: {}", rms);
    println!("Saw RMS: {}", rms);
}

// ========== Spectral Content Tests ==========

#[test]
fn test_saw_has_harmonics() {
    // Sawtooth should have all harmonics present
    let code = r#"
        tempo: 0.5
        o1: saw 440 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    // Find fundamental (440 Hz)
    let mut fundamental_mag = 0.0f32;
    for (i, &freq) in frequencies.iter().enumerate() {
        if (freq - 440.0).abs() < 20.0 {
            fundamental_mag = fundamental_mag.max(magnitudes[i]);
        }
    }

    // Find second harmonic (880 Hz)
    let mut second_harmonic_mag = 0.0f32;
    for (i, &freq) in frequencies.iter().enumerate() {
        if (freq - 880.0).abs() < 20.0 {
            second_harmonic_mag = second_harmonic_mag.max(magnitudes[i]);
        }
    }

    // Find third harmonic (1320 Hz)
    let mut third_harmonic_mag = 0.0f32;
    for (i, &freq) in frequencies.iter().enumerate() {
        if (freq - 1320.0).abs() < 20.0 {
            third_harmonic_mag = third_harmonic_mag.max(magnitudes[i]);
        }
    }

    assert!(fundamental_mag > 0.1,
        "Saw should have strong fundamental, got {}",
        fundamental_mag);

    assert!(second_harmonic_mag > 0.01,
        "Saw should have second harmonic, got {}",
        second_harmonic_mag);

    assert!(third_harmonic_mag > 0.01,
        "Saw should have third harmonic, got {}",
        third_harmonic_mag);

    println!("Harmonics - 1st: {}, 2nd: {}, 3rd: {}",
        fundamental_mag, second_harmonic_mag, third_harmonic_mag);
}

#[test]
fn test_saw_brighter_than_sine() {
    // Sawtooth should have more high-frequency content than sine
    let code_saw = r#"
        tempo: 0.5
        o1: saw 440 * 0.3
    "#;

    let code_sine = r#"
        tempo: 0.5
        o1: sine 440 * 0.3
    "#;

    let buffer_saw = render_dsl(code_saw, 1.0);
    let buffer_sine = render_dsl(code_sine, 1.0);

    let (frequencies, magnitudes_saw) = analyze_spectrum(&buffer_saw, 44100.0);
    let (_, magnitudes_sine) = analyze_spectrum(&buffer_sine, 44100.0);

    // Calculate energy above 1kHz
    let high_energy_saw: f32 = frequencies.iter()
        .zip(magnitudes_saw.iter())
        .filter(|(f, _)| **f > 1000.0 && **f < 10000.0)
        .map(|(_, m)| m * m)
        .sum();

    let high_energy_sine: f32 = frequencies.iter()
        .zip(magnitudes_sine.iter())
        .filter(|(f, _)| **f > 1000.0 && **f < 10000.0)
        .map(|(_, m)| m * m)
        .sum();

    assert!(high_energy_saw > high_energy_sine * 2.0,
        "Saw should have more high-frequency content than sine. Saw: {}, Sine: {}",
        high_energy_saw, high_energy_sine);

    println!("High freq energy - Saw: {}, Sine: {}", high_energy_saw, high_energy_sine);
}

// ========== Frequency Range Tests ==========

#[test]
fn test_saw_bass() {
    let code = r#"
        tempo: 0.5
        o1: saw 55 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.15, "Saw bass should work, RMS: {}", rms);
    println!("Saw bass RMS: {}", rms);
}

#[test]
fn test_saw_mid_range() {
    let code = r#"
        tempo: 0.5
        o1: saw 880 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.15, "Mid-range saw should work, RMS: {}", rms);
    println!("Mid-range saw RMS: {}", rms);
}

#[test]
fn test_saw_high_frequency() {
    let code = r#"
        tempo: 0.5
        o1: saw 4000 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.15, "High frequency saw should work, RMS: {}", rms);
    println!("High frequency saw RMS: {}", rms);
}

// ========== Musical Applications ==========

#[test]
fn test_saw_bass_synth() {
    // Classic analog bass: saw with filter envelope
    let code = r#"
        tempo: 0.5
        ~env: ad 0.01 0.3
        ~cutoff: ~env * 3000 + 200
        ~bass: saw 55 # rlpf ~cutoff 4.0
        o1: ~bass * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.02, "Saw bass synth should work, RMS: {}", rms);
    println!("Saw bass synth RMS: {}", rms);
}

#[test]
fn test_saw_lead_synth() {
    // Lead synth: saw with fast envelope
    let code = r#"
        tempo: 0.5
        ~env: ad 0.001 0.2
        o1: saw 440 * ~env * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.03, "Saw lead synth should work, RMS: {}", rms);
    println!("Saw lead synth RMS: {}", rms);
}

#[test]
fn test_saw_pad() {
    // Pad sound: filtered saw with slow envelope
    let code = r#"
        tempo: 1.0
        ~env: ad 0.5 0.5
        ~pad: saw 220 # rlpf 1500 1.0
        o1: ~pad * ~env * 0.2
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.02, "Saw pad should work, RMS: {}", rms);
    println!("Saw pad RMS: {}", rms);
}

#[test]
fn test_saw_string_section() {
    // String section: multiple detuned saws
    let code = r#"
        tempo: 0.5
        ~saw1: saw 220
        ~saw2: saw 221
        ~saw3: saw 219
        o1: (~saw1 + ~saw2 + ~saw3) * 0.1
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.08, "Saw string section should work, RMS: {}", rms);
    println!("Saw string section RMS: {}", rms);
}

#[test]
fn test_saw_supersaw() {
    // Supersaw: multiple saws with slight detuning
    let code = r#"
        tempo: 0.5
        ~s1: saw 440
        ~s2: saw 441
        ~s3: saw 439
        ~s4: saw 442
        ~s5: saw 438
        o1: (~s1 + ~s2 + ~s3 + ~s4 + ~s5) * 0.06
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.07, "Supersaw should work, RMS: {}", rms);
    println!("Supersaw RMS: {}", rms);
}

// ========== Pattern Modulation Tests ==========

#[test]
fn test_saw_pattern_frequency() {
    let code = r#"
        tempo: 0.5
        ~freq: sine 1 * 100 + 440
        o1: saw ~freq * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1,
        "Saw with pattern-modulated frequency should work, RMS: {}",
        rms);

    println!("Pattern frequency RMS: {}", rms);
}

#[test]
fn test_saw_pattern_amplitude() {
    let code = r#"
        tempo: 0.5
        ~amp: sine 2 * 0.2 + 0.3
        o1: saw 440 * ~amp
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05,
        "Saw with pattern-modulated amplitude should work, RMS: {}",
        rms);

    println!("Pattern amplitude RMS: {}", rms);
}

// ========== Filtered Saw Tests ==========

#[test]
fn test_saw_lowpass_filter() {
    // Lowpassed saw becomes mellower
    let code = r#"
        tempo: 0.5
        ~filtered: saw 220 # rlpf 800 2.0
        o1: ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Lowpassed saw should work, RMS: {}", rms);
    println!("Lowpassed saw RMS: {}", rms);
}

#[test]
fn test_saw_highpass_filter() {
    // Highpassed saw removes bass
    let code = r#"
        tempo: 0.5
        ~filtered: saw 220 # rhpf 400 2.0
        o1: ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Highpassed saw should work, RMS: {}", rms);
    println!("Highpassed saw RMS: {}", rms);
}

#[test]
fn test_saw_resonant_filter() {
    // Saw through resonant filter
    let code = r#"
        tempo: 0.5
        ~env: ad 0.01 0.2
        ~cutoff: ~env * 3000 + 300
        ~synth: saw 110 # rlpf ~cutoff 8.0
        o1: ~synth * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.01, "Saw with resonant filter should work, RMS: {}", rms);
    println!("Saw + resonant filter RMS: {}", rms);
}

// ========== Stability Tests ==========

#[test]
fn test_saw_no_excessive_clipping() {
    let code = r#"
        tempo: 0.5
        o1: saw 440 * 0.5
    "#;

    let buffer = render_dsl(code, 1.0);
    let max_amplitude = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    assert!(max_amplitude <= 0.8,
        "Saw should not excessively clip, max: {}",
        max_amplitude);

    println!("Saw max amplitude: {}", max_amplitude);
}

#[test]
fn test_saw_dc_offset() {
    // Saw should have no significant DC offset
    let code = r#"
        tempo: 0.5
        o1: saw 440 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let mean: f32 = buffer.iter().sum::<f32>() / buffer.len() as f32;

    assert!(mean.abs() < 0.05,
        "Saw should have minimal DC offset, mean: {}",
        mean);

    println!("Saw DC offset: {}", mean);
}

// ========== Phase Continuity Tests ==========

#[test]
fn test_saw_continuous() {
    // Verify saw is mostly continuous (allow for intentional sawtooth discontinuity)
    let code = r#"
        tempo: 0.5
        o1: saw 440 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);

    // Check for excessive discontinuities
    let mut large_jumps = 0;
    for i in 1..buffer.len() {
        let diff = (buffer[i] - buffer[i-1]).abs();
        if diff > 0.3 {
            large_jumps += 1;
        }
    }

    // Sawtooth has intentional discontinuities at phase resets
    // At 440Hz and 44100 samples/sec, we expect ~440 phase resets per second
    let expected_resets = 440; // approximately
    let tolerance_factor = 2.0;

    assert!((large_jumps as f32) < (expected_resets as f32 * tolerance_factor),
        "Saw should not have excessive discontinuities, found {}",
        large_jumps);

    println!("Saw phase resets: {}", large_jumps);
}
