/// Systematic tests: Sine Oscillator
///
/// Tests sine wave oscillator with spectral analysis and audio verification.
/// Sine waves are the purest waveform containing only the fundamental frequency.
///
/// Key characteristics:
/// - Single harmonic (fundamental only)
/// - Smooth, pure tone
/// - No overtones
/// - Frequency pattern-modulated
/// - Used for sub-bass, pure tones, LFOs, test signals
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use std::f32::consts::PI;

mod audio_test_utils;
use audio_test_utils::calculate_rms;

fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph =
        compile_program(statements, sample_rate, None).expect("Failed to compile DSL code");
    let num_samples = (duration * sample_rate) as usize;
    graph.render(num_samples)
}

/// Perform FFT and analyze spectrum
fn analyze_spectrum(buffer: &[f32], sample_rate: f32) -> (Vec<f32>, Vec<f32>) {
    use rustfft::{num_complex::Complex, FftPlanner};

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

// ========== Basic Sine Tests ==========

#[test]
fn test_sine_compiles() {
    let code = r#"
        tempo: 0.5
        out $ sine 440
    "#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);
    assert!(result.is_ok(), "Sine should compile: {:?}", result.err());
}

#[test]
fn test_sine_generates_audio() {
    let code = r#"
        tempo: 0.5
        out $ sine 440 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.15, "Sine should produce audio, got RMS: {}", rms);
    println!("Sine RMS: {}", rms);
}

// ========== Spectral Purity Tests ==========

#[test]
fn test_sine_single_harmonic() {
    // Sine wave should only have fundamental, no harmonics
    let code = r#"
        tempo: 0.5
        out $ sine 440 * 0.3
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

    assert!(
        fundamental_mag > 0.1,
        "Sine should have strong fundamental, got {}",
        fundamental_mag
    );

    let ratio = second_harmonic_mag / fundamental_mag.max(0.001);
    assert!(
        ratio < 0.05,
        "Sine should have minimal harmonics, 2nd/1st ratio: {}",
        ratio
    );

    println!(
        "Fundamental: {}, 2nd harmonic: {}, ratio: {}",
        fundamental_mag, second_harmonic_mag, ratio
    );
}

#[test]
fn test_sine_frequency_accuracy() {
    // Verify sine generates correct frequency
    let code = r#"
        tempo: 0.5
        out $ sine 440 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&buffer, 44100.0);

    // Find peak frequency
    let mut peak_freq = 0.0f32;
    let mut peak_mag = 0.0f32;
    for (i, &freq) in frequencies.iter().enumerate() {
        if freq > 400.0 && freq < 480.0 {
            if magnitudes[i] > peak_mag {
                peak_mag = magnitudes[i];
                peak_freq = freq;
            }
        }
    }

    assert!(
        (peak_freq - 440.0).abs() < 10.0,
        "Sine should peak at 440Hz, got {}Hz",
        peak_freq
    );

    println!("Peak frequency: {}Hz", peak_freq);
}

// ========== Frequency Range Tests ==========

#[test]
fn test_sine_sub_bass() {
    // Very low frequency sine
    let code = r#"
        tempo: 0.5
        out $ sine 40 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.15, "Sub-bass sine should work, RMS: {}", rms);
    println!("Sub-bass sine RMS: {}", rms);
}

#[test]
fn test_sine_mid_range() {
    let code = r#"
        tempo: 0.5
        out $ sine 1000 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.15, "Mid-range sine should work, RMS: {}", rms);
    println!("Mid-range sine RMS: {}", rms);
}

#[test]
fn test_sine_high_frequency() {
    let code = r#"
        tempo: 0.5
        out $ sine 8000 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.15, "High frequency sine should work, RMS: {}", rms);
    println!("High frequency sine RMS: {}", rms);
}

// ========== Musical Applications ==========

#[test]
fn test_sine_bass() {
    // Deep sine bass with envelope
    let code = r#"
        tempo: 0.5
        ~env $ ad 0.01 0.3
        ~bass $ sine 55
        out $ ~bass * ~env * 0.4
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Sine bass should work, RMS: {}", rms);
    println!("Sine bass RMS: {}", rms);
}

#[test]
fn test_sine_lfo() {
    // Sine as LFO modulating another sine
    let code = r#"
        tempo: 0.5
        ~lfo $ sine 0.5
        ~freq $ ~lfo * 100 + 440
        out $ sine ~freq * 0.3
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "Sine LFO should work, RMS: {}", rms);
    println!("Sine LFO RMS: {}", rms);
}

#[test]
fn test_sine_vibrato() {
    // Vibrato effect (fast frequency modulation)
    let code = r#"
        tempo: 0.5
        ~vibrato $ sine 6 * 10
        out $ sine (440 + ~vibrato) * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "Sine vibrato should work, RMS: {}", rms);
    println!("Sine vibrato RMS: {}", rms);
}

#[test]
fn test_sine_tremolo() {
    // Tremolo effect (amplitude modulation)
    let code = r#"
        tempo: 0.5
        ~trem $ sine 5 * 0.3 + 0.7
        out $ sine 440 * ~trem * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "Sine tremolo should work, RMS: {}", rms);
    println!("Sine tremolo RMS: {}", rms);
}

#[test]
fn test_sine_chord() {
    // Multiple sines for chord
    let code = r#"
        tempo: 0.5
        ~root $ sine 220
        ~third $ sine 277
        ~fifth $ sine 330
        out $ (~root + ~third + ~fifth) * 0.1
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.10, "Sine chord should work, RMS: {}", rms);
    println!("Sine chord RMS: {}", rms);
}

// ========== Pattern Modulation Tests ==========

#[test]
fn test_sine_pattern_frequency() {
    let code = r#"
        tempo: 0.5
        ~freq $ sine 1 * 100 + 440
        out $ sine ~freq * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.1,
        "Sine with pattern-modulated frequency should work, RMS: {}",
        rms
    );

    println!("Pattern frequency RMS: {}", rms);
}

#[test]
fn test_sine_pattern_amplitude() {
    let code = r#"
        tempo: 0.5
        ~amp $ sine 2 * 0.2 + 0.3
        out $ sine 440 * ~amp
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.05,
        "Sine with pattern-modulated amplitude should work, RMS: {}",
        rms
    );

    println!("Pattern amplitude RMS: {}", rms);
}

// ========== Filtered Sine Tests ==========

#[test]
fn test_sine_through_lowpass() {
    // Sine through lowpass (should be mostly unchanged)
    let code = r#"
        tempo: 0.5
        ~filtered $ sine 440 # rlpf 2000 2.0
        out $ ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "Filtered sine should work, RMS: {}", rms);
    println!("Lowpassed sine RMS: {}", rms);
}

#[test]
fn test_sine_through_highpass() {
    // Sine through highpass (removes fundamental if cutoff too high)
    let code = r#"
        tempo: 0.5
        ~filtered $ sine 1000 # rhpf 500 2.0
        out $ ~filtered * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Highpassed sine should work, RMS: {}", rms);
    println!("Highpassed sine RMS: {}", rms);
}

// ========== Stability Tests ==========

#[test]
fn test_sine_no_clipping() {
    let code = r#"
        tempo: 0.5
        out $ sine 440 * 0.5
    "#;

    let buffer = render_dsl(code, 1.0);
    let max_amplitude = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    assert!(
        max_amplitude <= 0.7,
        "Sine should not clip, max: {}",
        max_amplitude
    );

    println!("Sine max amplitude: {}", max_amplitude);
}

#[test]
fn test_sine_dc_offset() {
    // Sine should have no DC offset (mean near zero)
    let code = r#"
        tempo: 0.5
        out $ sine 440 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let mean: f32 = buffer.iter().sum::<f32>() / buffer.len() as f32;

    assert!(
        mean.abs() < 0.01,
        "Sine should have no DC offset, mean: {}",
        mean
    );

    println!("Sine DC offset: {}", mean);
}

// ========== Phase Continuity Tests ==========

#[test]
fn test_sine_continuous() {
    // Verify sine is continuous (no pops/clicks)
    let code = r#"
        tempo: 0.5
        out $ sine 440 * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);

    // Check for discontinuities (sudden jumps)
    let mut max_diff = 0.0f32;
    for i in 1..buffer.len() {
        let diff = (buffer[i] - buffer[i - 1]).abs();
        max_diff = max_diff.max(diff);
    }

    // At 440Hz and 44100 samples/sec, max change per sample should be small
    assert!(
        max_diff < 0.1,
        "Sine should be continuous, max diff: {}",
        max_diff
    );

    println!("Sine max sample-to-sample diff: {}", max_diff);
}
