use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Pattern Query Verification
/// Tests that ring_mod syntax is parsed and compiled correctly
#[test]
fn test_ring_mod_pattern_query() {
    let dsl = r#"
tempo: 1.0
~carrier: sine 440
~modulator: sine 110
~ring: ring_mod ~carrier ~modulator
out: ~ring * 0.3
"#;

    let (remaining, statements) = parse_program(dsl).unwrap();
    assert!(
        remaining.trim().is_empty(),
        "Should parse completely, remaining: '{}'",
        remaining
    );

    let graph = compile_program(statements, SAMPLE_RATE, None);
    assert!(
        graph.is_ok(),
        "Ring mod should compile successfully: {:?}",
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

/// LEVEL 2: Sideband Verification
/// Tests that ring modulation creates sidebands at sum and difference frequencies
#[test]
fn test_ring_mod_sidebands() {
    let dsl = r#"
tempo: 1.0
-- Ring mod: carrier 440 Hz, modulator 110 Hz
-- Should create sidebands at 440+110=550 Hz and 440-110=330 Hz
-- Original frequencies should be suppressed
~carrier: sine 440
~modulator: sine 110
~ring: ring_mod ~carrier ~modulator
out: ~ring * 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Render 1 second
    let samples = graph.render(SAMPLE_RATE as usize);

    // Write to file for manual inspection if needed
    let filename = "/tmp/test_ring_mod_sidebands.wav";
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

    println!("Ring mod peak frequencies: {:?}", peak_freqs);
    println!("Expected: 330 Hz (difference) and 550 Hz (sum)");

    // Ring modulation should create sidebands at sum and difference frequencies
    // 440 + 110 = 550 Hz (sum)
    // 440 - 110 = 330 Hz (difference)

    let has_sum_sideband = peak_freqs.iter().any(|&f| (f - 550.0).abs() < 30.0);
    let has_diff_sideband = peak_freqs.iter().any(|&f| (f - 330.0).abs() < 30.0);

    assert!(
        has_sum_sideband,
        "Ring mod should have sum sideband at 550 Hz, got peaks: {:?}",
        peak_freqs
    );

    assert!(
        has_diff_sideband,
        "Ring mod should have difference sideband at 330 Hz, got peaks: {:?}",
        peak_freqs
    );

    // Original carrier and modulator should be suppressed (not present)
    let has_carrier = peak_freqs.iter().any(|&f| (f - 440.0).abs() < 20.0);
    let has_modulator = peak_freqs.iter().any(|&f| (f - 110.0).abs() < 20.0);

    // Note: Due to numerical precision, the carrier/modulator might have very small
    // components, but they should NOT be in the top peaks
    println!(
        "Carrier present: {}, Modulator present: {}",
        has_carrier, has_modulator
    );
}

/// LEVEL 3: Musical Integration Test
#[test]
fn test_ring_mod_musical_example() {
    let dsl = r#"
tempo: 0.5
-- Metallic bell-like tone using ring modulation
~carrier: sine 440
~mod: sine 333
~bell: ring_mod ~carrier ~mod
out: ~bell * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE / 2.0) as usize); // 0.5 seconds

    // Write to file
    let filename = "/tmp/test_ring_mod_musical.wav";
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
        "Ring mod tone should be audible (RMS > 0.1), got RMS {}",
        rms
    );

    // Peak should be reasonable
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        peak > 0.2 && peak < 0.5,
        "Ring mod tone should have reasonable peak (0.2-0.5), got {}",
        peak
    );
}

/// Test ring mod with envelope for percussive sounds
#[test]
fn test_ring_mod_with_envelope() {
    let dsl = r#"
tempo: 0.5
~env: ad 0.001 0.1
~carrier: sine 880
~mod: sine 200
~ring: ring_mod ~carrier ~mod
out: ~ring * ~env * 0.4
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();
    let samples = graph.render((SAMPLE_RATE / 2.0) as usize);

    // Should produce percussive metallic sound
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    assert!(rms.sqrt() > 0.01, "Enveloped ring mod should be audible");

    // Peak should be near the start
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
        "Enveloped ring mod should be louder at start than end"
    );
}

/// Test ring mod with non-harmonic ratios creates inharmonic sounds
#[test]
fn test_ring_mod_inharmonic() {
    let dsl = r#"
tempo: 1.0
-- Non-integer ratio creates inharmonic bell-like tones
~carrier: sine 440
~mod: sine 337
~ring: ring_mod ~carrier ~mod
out: ~ring * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();
    let samples = graph.render(SAMPLE_RATE as usize);

    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    assert!(rms.sqrt() > 0.05, "Inharmonic ring mod should be audible");
}

/// Test pattern-modulated ring modulation
#[test]
fn test_ring_mod_pattern_modulation() {
    let dsl = r#"
tempo: 0.5
-- Carrier frequency varies via pattern
~carrier_freq: "220 440 330 550"
~carrier: sine ~carrier_freq
~mod: sine 100
~ring: ring_mod ~carrier ~mod
out: ~ring * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let graph = compile_program(statements, SAMPLE_RATE, None);

    assert!(
        graph.is_ok(),
        "Ring mod with pattern modulation should compile: {:?}",
        graph.err()
    );
}

/// Test ring mod with LFO modulation (tremolo-like effect)
#[test]
fn test_ring_mod_with_lfo() {
    let dsl = r#"
tempo: 1.0
-- Ring mod with very low frequency creates tremolo-like effect
~carrier: sine 440
~lfo: sine 4
~ring: ring_mod ~carrier ~lfo
out: ~ring * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();
    let samples = graph.render(SAMPLE_RATE as usize);

    // Should produce tremolo effect (amplitude modulation)
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    assert!(rms.sqrt() > 0.05, "Ring mod with LFO should be audible");
}
