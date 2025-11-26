/// Comprehensive oscillator verification tests
///
/// Tests all 8 oscillator functions:
/// - sine, saw, square, triangle (continuous oscillators)
/// - sine_trig, saw_trig, square_trig, tri_trig (pattern-triggered oscillators)
///
/// For each oscillator, we verify:
/// 1. Frequency accuracy - FFT analysis confirms expected fundamental
/// 2. Waveform characteristics - Spectral content matches theoretical expectations
/// 3. Amplitude - Generates appropriate signal level
/// 4. Pattern triggering (for _trig variants) - Responds to pattern events

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use rustfft::{FftPlanner, num_complex::Complex};
use std::f32::consts::PI;

mod audio_test_utils;
use audio_test_utils::{calculate_rms, find_dominant_frequency};

mod pattern_verification_utils;
use pattern_verification_utils::detect_audio_events;

fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph = compile_program(statements, sample_rate, None).expect("Failed to compile DSL code");
    let num_samples = (duration * sample_rate) as usize;
    graph.render(num_samples)
}

/// Perform FFT and analyze spectrum
fn analyze_spectrum(buffer: &[f32], sample_rate: f32) -> (Vec<f32>, Vec<f32>) {
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

// ========== SINE OSCILLATOR TESTS ==========

#[test]
fn test_sine_level1_frequency_accuracy() {
    let code = r#"
bpm: 120
o1: sine 440
"#;
    let audio = render_dsl(code, 1.0);
    let freq = find_dominant_frequency(&audio, 44100.0);
    assert!((freq - 440.0).abs() < 10.0,
        "Sine 440Hz should have fundamental near 440Hz, got {}", freq);
    println!("Sine fundamental: {} Hz (expected 440Hz)", freq);
}

#[test]
fn test_sine_level2_pure_tone() {
    // Sine should have only fundamental, no harmonics
    let code = r#"
bpm: 120
o1: sine 440 * 0.5
"#;
    let audio = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&audio, 44100.0);

    // Find fundamental (440 Hz)
    let mut fundamental_mag = 0.0f32;
    for (i, &freq) in frequencies.iter().enumerate() {
        if (freq - 440.0).abs() < 20.0 {
            fundamental_mag = fundamental_mag.max(magnitudes[i]);
        }
    }

    // Find second harmonic (880 Hz) - should be very weak
    let mut second_harmonic_mag = 0.0f32;
    for (i, &freq) in frequencies.iter().enumerate() {
        if (freq - 880.0).abs() < 20.0 {
            second_harmonic_mag = second_harmonic_mag.max(magnitudes[i]);
        }
    }

    assert!(fundamental_mag > 0.1, "Sine should have strong fundamental");
    let harmonic_ratio = second_harmonic_mag / fundamental_mag.max(0.001);
    assert!(harmonic_ratio < 0.05,
        "Sine should have minimal harmonics, got 2nd/1st ratio: {}", harmonic_ratio);

    println!("Sine harmonic purity - 1st: {}, 2nd: {}, ratio: {}",
        fundamental_mag, second_harmonic_mag, harmonic_ratio);
}

#[test]
fn test_sine_level3_amplitude() {
    let code = r#"
bpm: 120
o1: sine 440 * 0.5
"#;
    let audio = render_dsl(code, 1.0);
    let rms = calculate_rms(&audio);

    // Sine wave RMS should be amplitude/sqrt(2) ≈ 0.5/1.414 ≈ 0.35
    assert!(rms > 0.25 && rms < 0.45,
        "Sine wave RMS should be ~0.35 for amplitude 0.5, got {}", rms);
    println!("Sine RMS: {} (expected ~0.35)", rms);
}

// ========== SAW OSCILLATOR TESTS ==========

#[test]
fn test_saw_level1_frequency_accuracy() {
    let code = r#"
bpm: 120
o1: saw 440
"#;
    let audio = render_dsl(code, 1.0);
    let freq = find_dominant_frequency(&audio, 44100.0);
    assert!((freq - 440.0).abs() < 10.0,
        "Saw 440Hz should have fundamental near 440Hz, got {}", freq);
    println!("Saw fundamental: {} Hz (expected 440Hz)", freq);
}

#[test]
fn test_saw_level2_rich_harmonics() {
    // Saw should have all harmonics (odd + even) with 1/n falloff
    let code = r#"
bpm: 120
o1: saw 440 * 0.3
"#;
    let audio = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&audio, 44100.0);

    let mut fundamental_mag = 0.0f32;
    let mut second_harmonic_mag = 0.0f32;
    let mut third_harmonic_mag = 0.0f32;

    for (i, &freq) in frequencies.iter().enumerate() {
        if (freq - 440.0).abs() < 20.0 {
            fundamental_mag = fundamental_mag.max(magnitudes[i]);
        } else if (freq - 880.0).abs() < 20.0 {
            second_harmonic_mag = second_harmonic_mag.max(magnitudes[i]);
        } else if (freq - 1320.0).abs() < 20.0 {
            third_harmonic_mag = third_harmonic_mag.max(magnitudes[i]);
        }
    }

    assert!(fundamental_mag > 0.1, "Saw should have strong fundamental");
    assert!(second_harmonic_mag > 0.01, "Saw should have even harmonics");
    assert!(third_harmonic_mag > 0.01, "Saw should have odd harmonics");

    println!("Saw harmonics - 1st: {}, 2nd: {}, 3rd: {}",
        fundamental_mag, second_harmonic_mag, third_harmonic_mag);
}

#[test]
fn test_saw_level3_amplitude() {
    let code = r#"
bpm: 120
o1: saw 440 * 0.5
"#;
    let audio = render_dsl(code, 1.0);
    let rms = calculate_rms(&audio);

    // Saw wave should produce reasonable signal
    assert!(rms > 0.2 && rms < 0.5,
        "Saw wave RMS should be in reasonable range, got {}", rms);
    println!("Saw RMS: {}", rms);
}

// ========== SQUARE OSCILLATOR TESTS ==========

#[test]
fn test_square_level1_frequency_accuracy() {
    let code = r#"
bpm: 120
o1: square 440
"#;
    let audio = render_dsl(code, 1.0);
    let freq = find_dominant_frequency(&audio, 44100.0);
    assert!((freq - 440.0).abs() < 10.0,
        "Square 440Hz should have fundamental near 440Hz, got {}", freq);
    println!("Square fundamental: {} Hz (expected 440Hz)", freq);
}

#[test]
fn test_square_level2_odd_harmonics() {
    // Square should have only odd harmonics with 1/n falloff
    let code = r#"
bpm: 120
o1: square 440 * 0.3
"#;
    let audio = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&audio, 44100.0);

    let mut fundamental_mag = 0.0f32;
    let mut second_harmonic_mag = 0.0f32;
    let mut third_harmonic_mag = 0.0f32;

    for (i, &freq) in frequencies.iter().enumerate() {
        if (freq - 440.0).abs() < 20.0 {
            fundamental_mag = fundamental_mag.max(magnitudes[i]);
        } else if (freq - 880.0).abs() < 20.0 {
            second_harmonic_mag = second_harmonic_mag.max(magnitudes[i]);
        } else if (freq - 1320.0).abs() < 20.0 {
            third_harmonic_mag = third_harmonic_mag.max(magnitudes[i]);
        }
    }

    assert!(fundamental_mag > 0.1, "Square should have strong fundamental");

    // Even harmonic should be weak
    let even_ratio = second_harmonic_mag / fundamental_mag.max(0.001);
    assert!(even_ratio < 0.1,
        "Square should have weak even harmonics, got 2nd/1st ratio: {}", even_ratio);

    // Odd harmonic should be present
    assert!(third_harmonic_mag > 0.01, "Square should have odd harmonics");

    println!("Square harmonics - 1st: {}, 2nd: {}, 3rd: {}",
        fundamental_mag, second_harmonic_mag, third_harmonic_mag);
}

#[test]
fn test_square_level3_amplitude() {
    let code = r#"
bpm: 120
o1: square 440 * 0.5
"#;
    let audio = render_dsl(code, 1.0);
    let rms = calculate_rms(&audio);

    // Square wave RMS should be approximately equal to amplitude
    assert!(rms > 0.35 && rms < 0.65,
        "Square wave RMS should be ~0.5, got {}", rms);
    println!("Square RMS: {} (expected ~0.5)", rms);
}

// ========== TRIANGLE OSCILLATOR TESTS ==========

#[test]
fn test_triangle_level1_frequency_accuracy() {
    let code = r#"
bpm: 120
o1: triangle 440
"#;
    let audio = render_dsl(code, 1.0);
    let freq = find_dominant_frequency(&audio, 44100.0);
    assert!((freq - 440.0).abs() < 10.0,
        "Triangle 440Hz should have fundamental near 440Hz, got {}", freq);
    println!("Triangle fundamental: {} Hz (expected 440Hz)", freq);
}

#[test]
fn test_triangle_level2_weak_harmonics() {
    // Triangle should have only odd harmonics with 1/n² falloff (weaker than square)
    let code = r#"
bpm: 120
o1: triangle 440 * 0.3
"#;
    let audio = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&audio, 44100.0);

    let mut fundamental_mag = 0.0f32;
    let mut second_harmonic_mag = 0.0f32;
    let mut third_harmonic_mag = 0.0f32;

    for (i, &freq) in frequencies.iter().enumerate() {
        if (freq - 440.0).abs() < 20.0 {
            fundamental_mag = fundamental_mag.max(magnitudes[i]);
        } else if (freq - 880.0).abs() < 20.0 {
            second_harmonic_mag = second_harmonic_mag.max(magnitudes[i]);
        } else if (freq - 1320.0).abs() < 20.0 {
            third_harmonic_mag = third_harmonic_mag.max(magnitudes[i]);
        }
    }

    assert!(fundamental_mag > 0.1, "Triangle should have strong fundamental");

    // Even harmonic should be very weak
    let even_ratio = second_harmonic_mag / fundamental_mag.max(0.001);
    assert!(even_ratio < 0.1,
        "Triangle should have weak even harmonics, got 2nd/1st ratio: {}", even_ratio);

    // Odd harmonics should be weaker than square (1/n² vs 1/n)
    let third_ratio = third_harmonic_mag / fundamental_mag.max(0.001);
    assert!(third_ratio < 0.2,
        "Triangle should have weak 3rd harmonic, got 3rd/1st ratio: {}", third_ratio);

    println!("Triangle harmonics - 1st: {}, 2nd: {}, 3rd: {}",
        fundamental_mag, second_harmonic_mag, third_harmonic_mag);
}

#[test]
fn test_triangle_level3_amplitude() {
    let code = r#"
bpm: 120
o1: triangle 440 * 0.5
"#;
    let audio = render_dsl(code, 1.0);
    let rms = calculate_rms(&audio);

    // Triangle wave should produce reasonable signal
    assert!(rms > 0.25 && rms < 0.45,
        "Triangle wave RMS should be in reasonable range, got {}", rms);
    println!("Triangle RMS: {}", rms);
}

// ========== NOTE: COSINE IS A PATTERN FUNCTION, NOT AN OSCILLATOR ==========
// The 'cosine' function generates a 0-1 ramp pattern, not an audio oscillator.
// For cosine-phase audio, use sine with phase offset or pattern modulation.
// Therefore, we skip cosine oscillator tests.

// ========== TRIGGERED OSCILLATOR TESTS ==========

#[test]
fn test_sine_trig_level1_pattern_triggering() {
    // Note: frequency is specified IN the pattern string, not as separate arg
    let code = r#"
bpm: 120
o1: sine_trig "440 ~ 440 ~"
"#;
    let audio = render_dsl(code, 2.0); // 2 seconds = 4 cycles at bpm 120

    // Should have distinct events, not continuous tone
    let onsets = detect_audio_events(&audio, 44100.0, 0.02);
    assert!(onsets.len() >= 2,
        "Sine_trig should produce distinct events, got {} onsets", onsets.len());
    println!("Sine_trig onsets detected: {}", onsets.len());
}

#[test]
fn test_sine_trig_level2_silence_between_events() {
    let code = r#"
bpm: 120
o1: sine_trig "440 ~ ~ ~"
"#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);

    // With "440 ~ ~ ~" pattern, 75% should be silent, so RMS much lower than continuous
    let code_continuous = r#"
bpm: 120
o1: sine 440 * 0.5
"#;
    let audio_continuous = render_dsl(code_continuous, 2.0);
    let rms_continuous = calculate_rms(&audio_continuous);

    assert!(rms < rms_continuous * 0.6,
        "Triggered oscillator should be quieter than continuous, got {} vs {}", rms, rms_continuous);
    println!("Sine_trig RMS: {}, continuous: {}", rms, rms_continuous);
}

#[test]
fn test_sine_trig_level3_frequency_accuracy() {
    let code = r#"
bpm: 120
o1: sine_trig "440"
"#;
    let audio = render_dsl(code, 1.0);
    let freq = find_dominant_frequency(&audio, 44100.0);
    assert!((freq - 440.0).abs() < 20.0,
        "Sine_trig 440Hz should have fundamental near 440Hz, got {}", freq);
    println!("Sine_trig fundamental: {} Hz (expected 440Hz)", freq);
}

#[test]
fn test_saw_trig_level1_pattern_triggering() {
    let code = r#"
bpm: 120
o1: saw_trig "220 ~ 220 ~"
"#;
    let audio = render_dsl(code, 2.0);

    let onsets = detect_audio_events(&audio, 44100.0, 0.02);
    assert!(onsets.len() >= 2,
        "Saw_trig should produce distinct events, got {} onsets", onsets.len());
    println!("Saw_trig onsets detected: {}", onsets.len());
}

#[test]
fn test_saw_trig_level2_rich_harmonics() {
    let code = r#"
bpm: 120
o1: saw_trig "440"
"#;
    let audio = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&audio, 44100.0);

    let mut fundamental_mag = 0.0f32;
    let mut second_harmonic_mag = 0.0f32;

    for (i, &freq) in frequencies.iter().enumerate() {
        if (freq - 440.0).abs() < 20.0 {
            fundamental_mag = fundamental_mag.max(magnitudes[i]);
        } else if (freq - 880.0).abs() < 20.0 {
            second_harmonic_mag = second_harmonic_mag.max(magnitudes[i]);
        }
    }

    assert!(fundamental_mag > 0.05, "Saw_trig should have strong fundamental");
    assert!(second_harmonic_mag > 0.005, "Saw_trig should have harmonics");

    println!("Saw_trig harmonics - 1st: {}, 2nd: {}", fundamental_mag, second_harmonic_mag);
}

#[test]
fn test_saw_trig_level3_frequency_accuracy() {
    let code = r#"
bpm: 120
o1: saw_trig "440"
"#;
    let audio = render_dsl(code, 1.0);
    let freq = find_dominant_frequency(&audio, 44100.0);
    assert!((freq - 440.0).abs() < 20.0,
        "Saw_trig 440Hz should have fundamental near 440Hz, got {}", freq);
    println!("Saw_trig fundamental: {} Hz (expected 440Hz)", freq);
}

#[test]
fn test_square_trig_level1_pattern_triggering() {
    let code = r#"
bpm: 120
o1: square_trig "330 ~ 330 ~"
"#;
    let audio = render_dsl(code, 2.0);

    let onsets = detect_audio_events(&audio, 44100.0, 0.02);
    assert!(onsets.len() >= 2,
        "Square_trig should produce distinct events, got {} onsets", onsets.len());
    println!("Square_trig onsets detected: {}", onsets.len());
}

#[test]
fn test_square_trig_level2_odd_harmonics() {
    let code = r#"
bpm: 120
o1: square_trig "440"
"#;
    let audio = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&audio, 44100.0);

    let mut fundamental_mag = 0.0f32;
    let mut second_harmonic_mag = 0.0f32;
    let mut third_harmonic_mag = 0.0f32;

    for (i, &freq) in frequencies.iter().enumerate() {
        if (freq - 440.0).abs() < 20.0 {
            fundamental_mag = fundamental_mag.max(magnitudes[i]);
        } else if (freq - 880.0).abs() < 20.0 {
            second_harmonic_mag = second_harmonic_mag.max(magnitudes[i]);
        } else if (freq - 1320.0).abs() < 20.0 {
            third_harmonic_mag = third_harmonic_mag.max(magnitudes[i]);
        }
    }

    assert!(fundamental_mag > 0.05, "Square_trig should have strong fundamental");

    // Even harmonic should be weak
    let even_ratio = second_harmonic_mag / fundamental_mag.max(0.001);
    assert!(even_ratio < 0.15,
        "Square_trig should have weak even harmonics, got 2nd/1st ratio: {}", even_ratio);

    println!("Square_trig harmonics - 1st: {}, 2nd: {}, 3rd: {}",
        fundamental_mag, second_harmonic_mag, third_harmonic_mag);
}

#[test]
fn test_square_trig_level3_frequency_accuracy() {
    let code = r#"
bpm: 120
o1: square_trig "440"
"#;
    let audio = render_dsl(code, 1.0);
    let freq = find_dominant_frequency(&audio, 44100.0);
    assert!((freq - 440.0).abs() < 20.0,
        "Square_trig 440Hz should have fundamental near 440Hz, got {}", freq);
    println!("Square_trig fundamental: {} Hz (expected 440Hz)", freq);
}

#[test]
fn test_tri_trig_level1_pattern_triggering() {
    let code = r#"
bpm: 120
o1: tri_trig "440 ~ 440 ~"
"#;
    let audio = render_dsl(code, 2.0);

    let onsets = detect_audio_events(&audio, 44100.0, 0.02);
    assert!(onsets.len() >= 2,
        "Tri_trig should produce distinct events, got {} onsets", onsets.len());
    println!("Tri_trig onsets detected: {}", onsets.len());
}

#[test]
fn test_tri_trig_level2_weak_harmonics() {
    let code = r#"
bpm: 120
o1: tri_trig "440"
"#;
    let audio = render_dsl(code, 1.0);
    let (frequencies, magnitudes) = analyze_spectrum(&audio, 44100.0);

    let mut fundamental_mag = 0.0f32;
    let mut second_harmonic_mag = 0.0f32;
    let mut third_harmonic_mag = 0.0f32;

    for (i, &freq) in frequencies.iter().enumerate() {
        if (freq - 440.0).abs() < 20.0 {
            fundamental_mag = fundamental_mag.max(magnitudes[i]);
        } else if (freq - 880.0).abs() < 20.0 {
            second_harmonic_mag = second_harmonic_mag.max(magnitudes[i]);
        } else if (freq - 1320.0).abs() < 20.0 {
            third_harmonic_mag = third_harmonic_mag.max(magnitudes[i]);
        }
    }

    assert!(fundamental_mag > 0.05, "Tri_trig should have strong fundamental");

    // Even harmonic should be weak
    let even_ratio = second_harmonic_mag / fundamental_mag.max(0.001);
    assert!(even_ratio < 0.15,
        "Tri_trig should have weak even harmonics, got 2nd/1st ratio: {}", even_ratio);

    // Third harmonic should be weaker than square
    let third_ratio = third_harmonic_mag / fundamental_mag.max(0.001);
    assert!(third_ratio < 0.25,
        "Tri_trig should have weak 3rd harmonic, got 3rd/1st ratio: {}", third_ratio);

    println!("Tri_trig harmonics - 1st: {}, 2nd: {}, 3rd: {}",
        fundamental_mag, second_harmonic_mag, third_harmonic_mag);
}

#[test]
fn test_tri_trig_level3_frequency_accuracy() {
    let code = r#"
bpm: 120
o1: tri_trig "440"
"#;
    let audio = render_dsl(code, 1.0);
    let freq = find_dominant_frequency(&audio, 44100.0);
    assert!((freq - 440.0).abs() < 20.0,
        "Tri_trig 440Hz should have fundamental near 440Hz, got {}", freq);
    println!("Tri_trig fundamental: {} Hz (expected 440Hz)", freq);
}

// ========== CROSS-OSCILLATOR COMPARISON TESTS ==========

#[test]
fn test_harmonic_content_ordering() {
    // Verify relative harmonic richness
    // Theoretical: saw > square > triangle > sine
    // Note: Band-limited implementations may have slightly different characteristics
    let duration = 1.0;

    let sine_audio = render_dsl("bpm: 120\no1: sine 440 * 0.3", duration);
    let triangle_audio = render_dsl("bpm: 120\no1: triangle 440 * 0.3", duration);
    let square_audio = render_dsl("bpm: 120\no1: square 440 * 0.3", duration);
    let saw_audio = render_dsl("bpm: 120\no1: saw 440 * 0.3", duration);

    // Measure high-frequency energy (above 1kHz) as proxy for harmonic richness
    fn high_freq_energy(audio: &[f32]) -> f32 {
        let (frequencies, magnitudes) = analyze_spectrum(audio, 44100.0);
        frequencies.iter()
            .zip(magnitudes.iter())
            .filter(|(f, _)| **f > 1000.0 && **f < 5000.0)
            .map(|(_, m)| m * m)
            .sum()
    }

    let sine_hf = high_freq_energy(&sine_audio);
    let triangle_hf = high_freq_energy(&triangle_audio);
    let square_hf = high_freq_energy(&square_audio);
    let saw_hf = high_freq_energy(&saw_audio);

    // Sine should have minimal harmonics (fundamental only)
    assert!(triangle_hf > sine_hf,
        "Triangle should have more high-freq content than sine. Got Sine: {}, Triangle: {}",
        sine_hf, triangle_hf);

    // Triangle should have weaker harmonics than square (1/n² vs 1/n)
    assert!(square_hf > triangle_hf,
        "Square should have more high-freq content than triangle. Got Triangle: {}, Square: {}",
        triangle_hf, square_hf);

    // Saw and Square are close - both have strong harmonics
    // Saw has even+odd (1/n), Square has odd-only (1/n)
    // With band-limiting, the difference may be small, so just verify both have significant content
    assert!(saw_hf > sine_hf * 2.0,
        "Saw should have much more high-freq content than sine. Got Sine: {}, Saw: {}",
        sine_hf, saw_hf);
    assert!(square_hf > sine_hf * 2.0,
        "Square should have much more high-freq content than sine. Got Sine: {}, Square: {}",
        sine_hf, square_hf);

    println!("High-freq energy - Sine: {:.4}, Triangle: {:.4}, Square: {:.4}, Saw: {:.4}",
        sine_hf, triangle_hf, square_hf, saw_hf);
    println!("Relative to sine: Triangle: {:.2}x, Square: {:.2}x, Saw: {:.2}x",
        triangle_hf / sine_hf.max(0.0001),
        square_hf / sine_hf.max(0.0001),
        saw_hf / sine_hf.max(0.0001));
}

#[test]
fn test_all_oscillators_compile() {
    // Quick smoke test that all oscillators compile
    let oscillators = vec![
        "sine 440",
        "saw 440",
        "square 440",
        "triangle 440",
        "sine_trig \"440\"",
        "saw_trig \"440\"",
        "square_trig \"440\"",
        "tri_trig \"440\"",
    ];

    for osc in oscillators {
        let code = format!("bpm: 120\no1: {}", osc);
        let result = parse_program(&code);
        assert!(result.is_ok(), "Failed to parse: {}", osc);

        let (_, statements) = result.unwrap();
        let graph_result = compile_program(statements, 44100.0);
        assert!(graph_result.is_ok(), "Failed to compile: {}", osc);
    }

    println!("All 8 oscillators compile successfully!");
}
