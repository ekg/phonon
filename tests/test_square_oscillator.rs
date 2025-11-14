/// Systematic tests: Square Oscillator
///
/// Tests square wave oscillator with spectral analysis and audio verification.
/// Square waves contain only odd harmonics with 1/n amplitude falloff.
///
/// Key characteristics:
/// - Only odd harmonics (1st, 3rd, 5th, 7th...)
/// - 1/n amplitude falloff
/// - Hollow, woody sound
/// - 50% duty cycle pulse wave
/// - Frequency pattern-modulated
/// - Used for leads, bass, retro game sounds, clarinet-like tones

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

// ========== Basic Square Tests ==========

#[test]
fn test_square_compiles() {
    let code = r#"
        tempo: 2.0
        o1: square 440
    "#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0);
    assert!(result.is_ok(), "Square should compile: {:?}", result.err());
}

#[test]
fn test_square_generates_audio() {
    let code = r#"
        tempo: 2.0
        o1: square 440 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.15, "Square should produce audio, got RMS: {}", rms);
    println!("Square RMS: {}", rms);
}

// ========== Spectral Content Tests ==========

#[test]
fn test_square_odd_harmonics_only() {
    // Square wave should have only odd harmonics
    let code = r#"
        tempo: 2.0
        o1: square 440 * 0.3
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

    // Find third harmonic (1320 Hz) - odd harmonic, should be present
    let mut third_harmonic_mag = 0.0f32;
    for (i, &freq) in frequencies.iter().enumerate() {
        if (freq - 1320.0).abs() < 20.0 {
            third_harmonic_mag = third_harmonic_mag.max(magnitudes[i]);
        }
    }

    assert!(fundamental_mag > 0.1,
        "Square should have strong fundamental, got {}",
        fundamental_mag);

    let even_to_odd_ratio = second_harmonic_mag / fundamental_mag.max(0.001);
    assert!(even_to_odd_ratio < 0.1,
        "Square should have weak even harmonics, 2nd/1st ratio: {}",
        even_to_odd_ratio);

    assert!(third_harmonic_mag > 0.01,
        "Square should have third harmonic, got {}",
        third_harmonic_mag);

    println!("Harmonics - 1st: {}, 2nd: {}, 3rd: {}, even/odd ratio: {}",
        fundamental_mag, second_harmonic_mag, third_harmonic_mag, even_to_odd_ratio);
}

#[test]
fn test_square_vs_saw_spectrum() {
    // Square has fewer harmonics than saw (only odd vs all)
    let code_square = r#"
        tempo: 2.0
        o1: square 440 * 0.3
    "#;

    let code_saw = r#"
        tempo: 2.0
        o1: saw 440 * 0.3
    "#;

    let buffer_square = render_dsl(code_square, 1.0);
    let buffer_saw = render_dsl(code_saw, 1.0);

    let (frequencies, magnitudes_square) = analyze_spectrum(&buffer_square, 44100.0);
    let (_, magnitudes_saw) = analyze_spectrum(&buffer_saw, 44100.0);

    // Calculate even harmonic energy (around 880, 1760, 2640 Hz)
    let even_energy_square: f32 = frequencies.iter()
        .zip(magnitudes_square.iter())
        .filter(|(f, _)| {
            let harmonic = **f / 440.0;
            harmonic > 1.8 && harmonic < 2.2 || // 2nd
            harmonic > 3.8 && harmonic < 4.2 || // 4th
            harmonic > 5.8 && harmonic < 6.2    // 6th
        })
        .map(|(_, m)| m * m)
        .sum();

    let even_energy_saw: f32 = frequencies.iter()
        .zip(magnitudes_saw.iter())
        .filter(|(f, _)| {
            let harmonic = **f / 440.0;
            harmonic > 1.8 && harmonic < 2.2 || // 2nd
            harmonic > 3.8 && harmonic < 4.2 || // 4th
            harmonic > 5.8 && harmonic < 6.2    // 6th
        })
        .map(|(_, m)| m * m)
        .sum();

    assert!(even_energy_saw > even_energy_square * 2.0,
        "Saw should have more even harmonic content than square. Saw: {}, Square: {}",
        even_energy_saw, even_energy_square);

    println!("Even harmonic energy - Square: {}, Saw: {}", even_energy_square, even_energy_saw);
}

// ========== Frequency Range Tests ==========

#[test]
fn test_square_bass() {
    let code = r#"
        tempo: 2.0
        o1: square 55 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.15, "Square bass should work, RMS: {}", rms);
    println!("Square bass RMS: {}", rms);
}

#[test]
fn test_square_mid_range() {
    let code = r#"
        tempo: 2.0
        o1: square 880 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.15, "Mid-range square should work, RMS: {}", rms);
    println!("Mid-range square RMS: {}", rms);
}

#[test]
fn test_square_high_frequency() {
    let code = r#"
        tempo: 2.0
        o1: square 2000 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.15, "High frequency square should work, RMS: {}", rms);
    println!("High frequency square RMS: {}", rms);
}

// ========== Musical Applications ==========

#[test]
fn test_square_bass_synth() {
    // Classic square wave bass
    let code = r#"
        tempo: 2.0
        ~env: ad 0.01 0.3
        o1: square 55 * ~env * 0.4
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Square bass synth should work, RMS: {}", rms);
    println!("Square bass synth RMS: {}", rms);
}

#[test]
fn test_square_lead_synth() {
    // Lead synth with square wave
    let code = r#"
        tempo: 2.0
        ~env: ad 0.001 0.2
        o1: square 440 * ~env * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.03, "Square lead synth should work, RMS: {}", rms);
    println!("Square lead synth RMS: {}", rms);
}

#[test]
fn test_square_retro_game() {
    // Retro game sound with square wave
    let code = r#"
        tempo: 2.0
        ~env: ad 0.001 0.1
        ~pitch: line 880 440
        o1: square ~pitch * ~env * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.03, "Square retro game sound should work, RMS: {}", rms);
    println!("Square retro game RMS: {}", rms);
}

#[test]
fn test_square_clarinet_simulation() {
    // Clarinet-like sound (clarinet has strong odd harmonics)
    let code = r#"
        tempo: 2.0
        ~env: ad 0.05 0.3
        ~clarinet: square 220 # rlpf 2000 1.5
        o1: ~clarinet * ~env * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.02, "Square clarinet simulation should work, RMS: {}", rms);
    println!("Square clarinet RMS: {}", rms);
}

#[test]
fn test_square_pad() {
    // Pad sound with square wave
    let code = r#"
        tempo: 1.0
        ~env: ad 0.5 0.5
        ~pad: square 220 # rlpf 1200 1.0
        o1: ~pad * ~env * 0.2
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.02, "Square pad should work, RMS: {}", rms);
    println!("Square pad RMS: {}", rms);
}

// ========== Pattern Modulation Tests ==========

#[test]
fn test_square_pattern_frequency() {
    let code = r#"
        tempo: 2.0
        ~freq: sine 1 * 100 + 440
        o1: square ~freq * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1,
        "Square with pattern-modulated frequency should work, RMS: {}",
        rms);

    println!("Pattern frequency RMS: {}", rms);
}

#[test]
fn test_square_pattern_amplitude() {
    let code = r#"
        tempo: 2.0
        ~amp: sine 2 * 0.2 + 0.3
        o1: square 440 * ~amp
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05,
        "Square with pattern-modulated amplitude should work, RMS: {}",
        rms);

    println!("Pattern amplitude RMS: {}", rms);
}

// ========== Filtered Square Tests ==========

#[test]
fn test_square_lowpass_filter() {
    // Lowpassed square becomes more sine-like
    let code = r#"
        tempo: 2.0
        ~filtered: square 220 # rlpf 600 2.0
        o1: ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Lowpassed square should work, RMS: {}", rms);
    println!("Lowpassed square RMS: {}", rms);
}

#[test]
fn test_square_highpass_filter() {
    // Highpassed square removes bass
    let code = r#"
        tempo: 2.0
        ~filtered: square 220 # rhpf 300 2.0
        o1: ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Highpassed square should work, RMS: {}", rms);
    println!("Highpassed square RMS: {}", rms);
}

#[test]
fn test_square_resonant_filter() {
    // Square through resonant filter
    let code = r#"
        tempo: 2.0
        ~env: ad 0.01 0.2
        ~cutoff: ~env * 2500 + 300
        ~synth: square 110 # rlpf ~cutoff 8.0
        o1: ~synth * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.01, "Square with resonant filter should work, RMS: {}", rms);
    println!("Square + resonant filter RMS: {}", rms);
}

// ========== Stability Tests ==========

#[test]
fn test_square_no_excessive_clipping() {
    let code = r#"
        tempo: 2.0
        o1: square 440 * 0.5
    "#;

    let buffer = render_dsl(code, 1.0);
    let max_amplitude = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    assert!(max_amplitude <= 0.8,
        "Square should not excessively clip, max: {}",
        max_amplitude);

    println!("Square max amplitude: {}", max_amplitude);
}

#[test]
fn test_square_dc_offset() {
    // Square should have no DC offset (symmetric waveform)
    let code = r#"
        tempo: 2.0
        o1: square 440 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let mean: f32 = buffer.iter().sum::<f32>() / buffer.len() as f32;

    assert!(mean.abs() < 0.05,
        "Square should have no DC offset, mean: {}",
        mean);

    println!("Square DC offset: {}", mean);
}

// ========== Phase Continuity Tests ==========

#[test]
fn test_square_transitions() {
    // Square wave should have clean transitions
    let code = r#"
        tempo: 2.0
        o1: square 440 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);

    // Count transitions (large sample-to-sample changes)
    let mut transitions = 0;
    for i in 1..buffer.len() {
        let diff = (buffer[i] - buffer[i-1]).abs();
        if diff > 0.3 {
            transitions += 1;
        }
    }

    // At 440Hz and 44100 samples/sec, we expect ~880 transitions per second
    // (two per cycle: high to low, low to high)
    let expected_transitions = 880;
    let tolerance_factor = 2.0;

    assert!((transitions as f32) < (expected_transitions as f32 * tolerance_factor),
        "Square should have expected number of transitions, found {}",
        transitions);

    println!("Square transitions: {}", transitions);
}
