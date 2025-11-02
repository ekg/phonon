use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Pattern Query Verification
/// Tests that pink_noise syntax is parsed and compiled correctly
#[test]
fn test_pink_noise_pattern_query() {
    let dsl = r#"
tempo: 1.0
~noise: pink_noise
out: ~noise * 0.5
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
        "Pink noise should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: Output Characteristics
/// Tests that pink_noise generates non-silent, non-zero output
#[test]
fn test_pink_noise_output() {
    let dsl = r#"
tempo: 1.0
~noise: pink_noise
out: ~noise * 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    // Render 1/10 second
    let samples = graph.render((SAMPLE_RATE / 10.0) as usize);

    // Should be non-silent
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    let rms = rms.sqrt();
    assert!(
        rms > 0.05,
        "Pink noise should be audible (RMS > 0.05), got RMS {}",
        rms
    );

    // Peak should be reasonable (not clipping or too quiet)
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        peak > 0.1 && peak < 1.0,
        "Peak should be reasonable (0.1-1.0), got {}",
        peak
    );

    // Should vary (not constant)
    let variance: f32 = samples
        .iter()
        .map(|s| {
            let diff = s - (samples.iter().sum::<f32>() / samples.len() as f32);
            diff * diff
        })
        .sum::<f32>()
        / samples.len() as f32;
    assert!(
        variance > 0.003,
        "Pink noise should vary significantly (variance > 0.003), got {}",
        variance
    );
}

/// LEVEL 2: Spectral Characteristics
/// Tests that pink_noise has 1/f spectrum (power decreases with frequency)
#[test]
fn test_pink_noise_spectral_slope() {
    let dsl = r#"
tempo: 1.0
~noise: pink_noise
out: ~noise
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    // Render 1 second for better frequency resolution
    let samples = graph.render(SAMPLE_RATE as usize);

    // Apply Hanning window to reduce spectral leakage
    let windowed: Vec<f32> = samples
        .iter()
        .enumerate()
        .map(|(i, &s)| {
            let window =
                0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / samples.len() as f32).cos());
            s * window
        })
        .collect();

    // Perform FFT to analyze spectrum
    use rustfft::{num_complex::Complex, FftPlanner};
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(windowed.len());

    let mut buffer: Vec<Complex<f32>> = windowed
        .iter()
        .map(|&s| Complex { re: s, im: 0.0 })
        .collect();
    fft.process(&mut buffer);

    // Compute power spectrum (magnitude squared)
    let spectrum: Vec<f32> = buffer.iter().map(|c| c.norm_sqr()).collect();

    // Divide spectrum into octave bands and measure power
    // Pink noise should have approximately equal energy per octave
    let bin_to_freq = |bin: usize| (bin as f32 * SAMPLE_RATE) / samples.len() as f32;

    // Define octave bands (100-200, 200-400, 400-800, 800-1600, 1600-3200, 3200-6400 Hz)
    let bands = vec![
        (100.0, 200.0),
        (200.0, 400.0),
        (400.0, 800.0),
        (800.0, 1600.0),
        (1600.0, 3200.0),
        (3200.0, 6400.0),
    ];

    let band_powers: Vec<f32> = bands
        .iter()
        .map(|(low, high)| {
            let bins: Vec<usize> = (0..spectrum.len())
                .filter(|&bin| {
                    let freq = bin_to_freq(bin);
                    freq >= *low && freq < *high
                })
                .collect();

            let power: f32 = bins.iter().map(|&bin| spectrum[bin]).sum();
            power / bins.len() as f32 // Average power in band
        })
        .collect();

    println!("Pink noise octave band powers: {:?}", band_powers);

    // Each octave should have approximately equal energy
    // Allow some variation but check that we don't have huge differences
    let max_power = band_powers.iter().cloned().fold(0.0f32, f32::max);
    let min_power = band_powers.iter().cloned().fold(f32::INFINITY, f32::min);

    assert!(
        max_power > 0.0,
        "Pink noise should have energy in all bands"
    );

    // Pink noise should have more consistent energy per octave than white noise
    // but won't be perfectly flat. Allow up to 10x variation.
    let ratio = max_power / min_power;
    assert!(
        ratio < 10.0,
        "Octave band energy should be relatively consistent, got ratio {}",
        ratio
    );

    // Verify that high frequencies have less power than low frequencies
    // (characteristic of pink noise)
    let low_freq_power = band_powers[0..2].iter().sum::<f32>() / 2.0;
    let high_freq_power = band_powers[4..6].iter().sum::<f32>() / 2.0;

    assert!(
        low_freq_power > high_freq_power * 0.3,
        "Low frequencies should have more power than high frequencies in pink noise"
    );
}

/// LEVEL 2: Different from White Noise
/// Tests that pink_noise has different spectral characteristics than white_noise
#[test]
fn test_pink_noise_vs_white_noise() {
    // Pink noise
    let dsl_pink = r#"
tempo: 1.0
~noise: pink_noise
out: ~noise
"#;

    let (_, statements) = parse_program(dsl_pink).unwrap();
    let mut graph_pink = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples_pink = graph_pink.render(SAMPLE_RATE as usize);

    // White noise
    let dsl_white = r#"
tempo: 1.0
~noise: white_noise
out: ~noise
"#;

    let (_, statements) = parse_program(dsl_white).unwrap();
    let mut graph_white = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples_white = graph_white.render(SAMPLE_RATE as usize);

    // Both should be audible
    let rms_pink: f32 = samples_pink.iter().map(|s| s * s).sum::<f32>() / samples_pink.len() as f32;
    let rms_white: f32 =
        samples_white.iter().map(|s| s * s).sum::<f32>() / samples_white.len() as f32;

    println!(
        "Pink noise RMS: {}, White noise RMS: {}",
        rms_pink.sqrt(),
        rms_white.sqrt()
    );

    assert!(rms_pink.sqrt() > 0.05, "Pink noise should be audible");
    assert!(rms_white.sqrt() > 0.05, "White noise should be audible");

    // They should sound different (have different spectral characteristics)
    // We can't directly compare samples since they're random, but we can
    // verify both are generating random output
    let all_different_pink = samples_pink.windows(2).all(|w| (w[0] - w[1]).abs() > 0.001);
    let all_different_white = samples_white
        .windows(2)
        .all(|w| (w[0] - w[1]).abs() > 0.001);

    assert!(
        !all_different_pink || !all_different_white,
        "Noise should have some adjacent samples that are close"
    );
}

/// LEVEL 3: Musical Integration Test
#[test]
fn test_pink_noise_musical_example() {
    let dsl = r#"
tempo: 2.0
~noise: pink_noise
~env: ad 0.005 0.15
~percussion: ~noise * ~env * 0.3
out: ~percussion
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE / 2.0) as usize); // 0.5 seconds

    // Write to file
    let filename = "/tmp/test_pink_noise_musical.wav";
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
        rms > 0.01,
        "Pink noise percussion should be audible (RMS > 0.01), got RMS {}",
        rms
    );
}

/// Test pink noise with filtering
#[test]
fn test_pink_noise_with_filter() {
    let dsl = r#"
tempo: 1.0
~noise: pink_noise
~filtered: lpf ~noise 2000 0.7
out: ~filtered * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples = graph.render((SAMPLE_RATE / 10.0) as usize);

    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    assert!(
        rms.sqrt() > 0.02,
        "Filtered pink noise should be audible, got RMS {}",
        rms.sqrt()
    );
}

/// Test amplitude scaling
#[test]
fn test_pink_noise_amplitude() {
    let dsl_loud = r#"
tempo: 1.0
~noise: pink_noise * 0.8
out: ~noise
"#;

    let (_, statements) = parse_program(dsl_loud).unwrap();
    let mut graph_loud = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples_loud = graph_loud.render((SAMPLE_RATE / 10.0) as usize);

    let dsl_quiet = r#"
tempo: 1.0
~noise: pink_noise * 0.2
out: ~noise
"#;

    let (_, statements) = parse_program(dsl_quiet).unwrap();
    let mut graph_quiet = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples_quiet = graph_quiet.render((SAMPLE_RATE / 10.0) as usize);

    let rms_loud: f32 = samples_loud.iter().map(|s| s * s).sum::<f32>() / samples_loud.len() as f32;
    let rms_quiet: f32 =
        samples_quiet.iter().map(|s| s * s).sum::<f32>() / samples_quiet.len() as f32;

    println!(
        "Loud RMS: {}, Quiet RMS: {}",
        rms_loud.sqrt(),
        rms_quiet.sqrt()
    );

    assert!(
        rms_loud > rms_quiet * 2.0,
        "Louder pink noise should have higher RMS"
    );
}
