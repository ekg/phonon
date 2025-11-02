use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use std::process::Command;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Pattern Query Verification
/// Tests that FM syntax is parsed and compiled correctly
#[test]
fn test_fm_pattern_query() {
    let dsl = r#"
tempo: 1.0
~fm: fm 440 110 2
out: ~fm * 0.3
"#;

    let (remaining, statements) = parse_program(dsl).unwrap();
    assert!(
        remaining.trim().is_empty(),
        "Should parse completely, remaining: '{}'",
        remaining
    );

    let graph = compile_program(statements, SAMPLE_RATE);
    assert!(
        graph.is_ok(),
        "FM should compile successfully: {:?}",
        graph.err()
    );
}

/// Helper function to compute FFT and find peak frequencies
fn find_peak_frequencies(samples: &[f32], sample_rate: f32, num_peaks: usize) -> Vec<f32> {
    use rustfft::{num_complex::Complex, FftPlanner};

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(samples.len());

    let mut buffer: Vec<Complex<f32>> = samples
        .iter()
        .map(|&s| Complex { re: s, im: 0.0 })
        .collect();

    fft.process(&mut buffer);

    // Compute magnitude spectrum (first half only - nyquist)
    let magnitudes: Vec<f32> = buffer[0..buffer.len() / 2]
        .iter()
        .map(|c| c.norm())
        .collect();

    // Find peaks
    let mut peaks: Vec<(usize, f32)> = magnitudes
        .iter()
        .enumerate()
        .filter(|(i, &mag)| {
            if *i == 0 || *i >= magnitudes.len() - 1 {
                return false;
            }
            mag > magnitudes[i - 1] && mag > magnitudes[i + 1] && mag > 0.01
        })
        .map(|(i, &mag)| (i, mag))
        .collect();

    // Sort by magnitude descending
    peaks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // Convert bin indices to frequencies
    let bin_to_freq = sample_rate / samples.len() as f32;
    peaks
        .iter()
        .take(num_peaks)
        .map(|(bin, _)| *bin as f32 * bin_to_freq)
        .collect()
}

/// LEVEL 2: Spectral Analysis Verification
/// Tests that FM produces correct harmonic spectrum with sidebands
#[test]
fn test_fm_spectral_sidebands() {
    let dsl = r#"
tempo: 1.0
-- FM: carrier=440Hz, modulator=110Hz, index=1.0
-- Should produce sidebands at 440±110, 440±220, etc.
~fm: fm 440 110 1.0
out: ~fm * 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    // Render 1 full cycle
    let samples = graph.render(SAMPLE_RATE as usize);

    // Write to file for manual inspection if needed
    let filename = "/tmp/test_fm_spectrum.wav";
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE as u32,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(filename, spec).unwrap();
    for sample in &samples {
        let amplitude = (sample * i16::MAX as f32) as i16;
        writer.write_sample(amplitude).unwrap();
    }
    writer.finalize().unwrap();

    // Find peak frequencies
    let peak_freqs = find_peak_frequencies(&samples, SAMPLE_RATE, 5);

    println!("FM Peak frequencies: {:?}", peak_freqs);

    // For FM with carrier=440, modulator=110, index=1.0:
    // Expected peaks: 440 (carrier), 330 (440-110), 550 (440+110)
    // With lower amplitudes: 220 (440-220), 660 (440+220)

    // Check for carrier frequency (440 Hz)
    let has_carrier = peak_freqs.iter().any(|&f| (f - 440.0).abs() < 20.0);
    assert!(
        has_carrier,
        "FM should have carrier frequency near 440 Hz, got peaks: {:?}",
        peak_freqs
    );

    // Check for first-order sidebands (440±110 = 330 or 550 Hz)
    let has_sidebands = peak_freqs.iter().any(|&f| (f - 330.0).abs() < 20.0)
        || peak_freqs.iter().any(|&f| (f - 550.0).abs() < 20.0);
    assert!(
        has_sidebands,
        "FM should have sidebands near 330 or 550 Hz, got peaks: {:?}",
        peak_freqs
    );
}

/// Test that modulation index affects spectral content
#[test]
fn test_fm_modulation_index_effect() {
    // Low modulation index - mostly carrier
    let dsl_low = r#"
tempo: 1.0
~fm: fm 440 110 0.5
out: ~fm * 0.5
"#;
    let (_, statements) = parse_program(dsl_low).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples_low = graph.render(SAMPLE_RATE as usize);
    let peaks_low = find_peak_frequencies(&samples_low, SAMPLE_RATE, 10);

    // High modulation index - more sidebands
    let dsl_high = r#"
tempo: 1.0
~fm: fm 440 110 3.0
out: ~fm * 0.5
"#;
    let (_, statements) = parse_program(dsl_high).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples_high = graph.render(SAMPLE_RATE as usize);
    let peaks_high = find_peak_frequencies(&samples_high, SAMPLE_RATE, 10);

    println!("Low index peaks: {:?}", peaks_low);
    println!("High index peaks: {:?}", peaks_high);

    // High index should have more frequency components
    assert!(
        peaks_high.len() >= peaks_low.len(),
        "High modulation index should create more sidebands"
    );
}

/// LEVEL 3: Musical Integration Test
#[test]
fn test_fm_musical_example() {
    let dsl = r#"
tempo: 2.0
-- Bell-like FM tone
~fm: fm 440 880 2.5
out: ~fm * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE / 2.0) as usize); // 0.5 seconds

    // Write to file
    let filename = "/tmp/test_fm_musical.wav";
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE as u32,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(filename, spec).unwrap();
    for sample in &samples {
        let amplitude = (sample * i16::MAX as f32) as i16;
        writer.write_sample(amplitude).unwrap();
    }
    writer.finalize().unwrap();

    // Should produce audible output
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    let rms = rms.sqrt();
    assert!(
        rms > 0.1,
        "FM tone should be audible (RMS > 0.1), got RMS {}",
        rms
    );

    // Peak should be reasonable
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        peak > 0.2 && peak < 0.5,
        "FM tone should have reasonable peak (0.2-0.5), got {}",
        peak
    );
}

/// Test pattern-modulated FM parameters
#[test]
fn test_fm_pattern_parameters() {
    let dsl = r#"
tempo: 2.0
-- Pattern-controlled modulation index
~index_pattern: "1.0 3.0"
~fm: fm 440 110 ~index_pattern
out: ~fm * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let graph = compile_program(statements, SAMPLE_RATE);

    assert!(
        graph.is_ok(),
        "FM with pattern-controlled parameters should compile: {:?}",
        graph.err()
    );
}

/// Test FM with different carrier/modulator ratios
#[test]
fn test_fm_ratio_variations() {
    // Integer ratio (harmonic)
    let dsl_harmonic = r#"
tempo: 1.0
~fm: fm 440 220 1.5
out: ~fm * 0.3
"#;
    let (_, statements) = parse_program(dsl_harmonic).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples = graph.render(SAMPLE_RATE as usize);

    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    assert!(rms.sqrt() > 0.05, "Harmonic FM should be audible");

    // Non-integer ratio (inharmonic/bell-like)
    let dsl_inharmonic = r#"
tempo: 1.0
~fm: fm 440 337 2.0
out: ~fm * 0.3
"#;
    let (_, statements) = parse_program(dsl_inharmonic).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples = graph.render(SAMPLE_RATE as usize);

    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    assert!(rms.sqrt() > 0.05, "Inharmonic FM should be audible");
}

/// Test FM with envelope modulation
#[test]
fn test_fm_with_envelope() {
    let dsl = r#"
tempo: 2.0
~env: ad 0.01 0.2
~fm: fm 440 880 2.0
out: ~fm * ~env * 0.4
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples = graph.render((SAMPLE_RATE / 2.0) as usize);

    // Should produce percussive FM tone
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    assert!(rms.sqrt() > 0.02, "Enveloped FM should be audible");

    // Peak should be near the start (attack phase)
    let first_quarter: f32 = samples[0..samples.len() / 4]
        .iter()
        .map(|s| s * s)
        .sum::<f32>()
        / (samples.len() / 4) as f32;
    let last_quarter: f32 = samples[samples.len() * 3 / 4..]
        .iter()
        .map(|s| s * s)
        .sum::<f32>()
        / (samples.len() / 4) as f32;

    assert!(
        first_quarter.sqrt() > last_quarter.sqrt() * 2.0,
        "Enveloped FM should be louder at start than end"
    );
}
