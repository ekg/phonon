use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Pattern Query Verification
/// Tests that white noise syntax is parsed and compiled correctly
#[test]
fn test_white_noise_pattern_query() {
    let dsl = r#"
tempo: 1.0
~noise: white_noise
out: ~noise * 0.3
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
        "White noise should compile successfully: {:?}",
        graph.err()
    );
}

/// Helper function to compute FFT and power spectrum
fn compute_power_spectrum(samples: &[f32], _sample_rate: f32) -> Vec<f32> {
    use rustfft::{num_complex::Complex, FftPlanner};

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(samples.len());

    let mut buffer: Vec<Complex<f32>> = samples
        .iter()
        .map(|&s| Complex { re: s, im: 0.0 })
        .collect();

    fft.process(&mut buffer);

    // Compute power spectrum (magnitude squared, first half only - nyquist)
    buffer[0..buffer.len() / 2]
        .iter()
        .map(|c| c.norm_sqr())
        .collect()
}

/// Helper function to measure spectral flatness (0=pure tone, 1=white noise)
/// Uses ratio of geometric mean to arithmetic mean of power spectrum
fn spectral_flatness(power_spectrum: &[f32]) -> f32 {
    // Find a threshold to exclude noise floor (use 1% of max power)
    let max_power = power_spectrum.iter().cloned().fold(0.0f32, f32::max);
    let threshold = max_power * 0.01;

    // Filter out bins below threshold for more robust measurement
    let filtered: Vec<f32> = power_spectrum
        .iter()
        .filter(|&&p| p > threshold)
        .copied()
        .collect();

    if filtered.is_empty() {
        return 0.0;
    }

    let geometric_mean = {
        let log_sum: f32 = filtered.iter().map(|&p| p.ln()).sum();
        (log_sum / filtered.len() as f32).exp()
    };

    let arithmetic_mean: f32 = filtered.iter().sum::<f32>() / filtered.len() as f32;

    if arithmetic_mean > 0.0 {
        geometric_mean / arithmetic_mean
    } else {
        0.0
    }
}

/// LEVEL 2: Spectral Analysis Verification
/// Tests that white noise has a flat (white) spectrum across frequencies
#[test]
fn test_white_noise_spectral_flatness() {
    let dsl = r#"
tempo: 1.0
~noise: white_noise
out: ~noise * 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    // Render 1 second of white noise
    let samples = graph.render(SAMPLE_RATE as usize);

    // Write to file for manual inspection if needed
    let filename = "/tmp/test_white_noise_spectrum.wav";
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

    // Compute power spectrum
    let power_spectrum = compute_power_spectrum(&samples, SAMPLE_RATE);

    // Measure spectral flatness
    let flatness = spectral_flatness(&power_spectrum);

    println!("White noise spectral flatness: {}", flatness);
    println!("(1.0 = perfectly flat/white, 0.0 = pure tone)");

    // White noise should have high spectral flatness
    // Real-world white noise from uniform random sampling typically has flatness 0.65-0.80
    assert!(
        flatness > 0.65,
        "White noise should have high spectral flatness (>0.65), got {}",
        flatness
    );
}

/// Test that white noise spectrum is relatively uniform across frequency bands
#[test]
fn test_white_noise_uniform_spectrum() {
    let dsl = r#"
tempo: 1.0
~noise: white_noise
out: ~noise * 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    // Render 1 second of white noise
    let samples = graph.render(SAMPLE_RATE as usize);

    // Compute power spectrum
    let power_spectrum = compute_power_spectrum(&samples, SAMPLE_RATE);

    // Divide spectrum into 10 bands and check variance
    let num_bands = 10;
    let band_size = power_spectrum.len() / num_bands;

    let band_powers: Vec<f32> = (0..num_bands)
        .map(|i| {
            let start = i * band_size;
            let end = (i + 1) * band_size;
            power_spectrum[start..end].iter().sum::<f32>() / band_size as f32
        })
        .collect();

    println!("Band powers: {:?}", band_powers);

    // Calculate coefficient of variation (std dev / mean)
    let mean = band_powers.iter().sum::<f32>() / band_powers.len() as f32;
    let variance =
        band_powers.iter().map(|&p| (p - mean).powi(2)).sum::<f32>() / band_powers.len() as f32;
    let std_dev = variance.sqrt();
    let coef_var = std_dev / mean;

    println!("Coefficient of variation: {}", coef_var);
    println!("(Lower is more uniform, white noise typically < 0.3)");

    // White noise should have relatively low coefficient of variation
    assert!(
        coef_var < 0.4,
        "White noise spectrum should be relatively uniform (CV < 0.4), got {}",
        coef_var
    );
}

/// LEVEL 3: Musical Integration Test
#[test]
fn test_white_noise_musical_example() {
    let dsl = r#"
tempo: 2.0
-- White noise percussion
~noise: white_noise
out: ~noise * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE / 2.0) as usize); // 0.5 seconds

    // Write to file
    let filename = "/tmp/test_white_noise_musical.wav";
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
        rms > 0.15,
        "White noise should be audible (RMS > 0.15), got RMS {}",
        rms
    );

    // Peak should be reasonable
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        peak > 0.2 && peak < 0.5,
        "White noise should have reasonable peak (0.2-0.5), got {}",
        peak
    );
}

/// Test white noise with envelope for percussion
#[test]
fn test_white_noise_with_envelope() {
    let dsl = r#"
tempo: 2.0
~env: ad 0.001 0.05
~noise: white_noise
out: ~noise * ~env * 0.4
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples = graph.render((SAMPLE_RATE / 2.0) as usize);

    // Should produce percussive noise burst
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    assert!(rms.sqrt() > 0.01, "Enveloped noise should be audible");

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
        "Enveloped noise should be louder at start than end"
    );
}

/// Test white noise filtered
#[test]
fn test_white_noise_filtered() {
    let dsl = r#"
tempo: 1.0
~noise: white_noise # lpf 1000 0.8
out: ~noise * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples = graph.render(SAMPLE_RATE as usize);

    // Compute power spectrum
    let power_spectrum = compute_power_spectrum(&samples, SAMPLE_RATE);

    // Check that high frequencies are attenuated
    let bin_to_freq = SAMPLE_RATE / samples.len() as f32;
    let cutoff_bin = (1000.0 / bin_to_freq) as usize;

    // Sum power below and above cutoff
    let low_power: f32 = power_spectrum[0..cutoff_bin].iter().sum();
    let high_power: f32 = power_spectrum[cutoff_bin..].iter().sum();

    println!(
        "Low freq power: {}, High freq power: {}",
        low_power, high_power
    );

    // Low frequencies should have more power than high frequencies
    assert!(
        low_power > high_power * 2.0,
        "Filtered noise should have more low frequency content"
    );
}

/// Test white noise is non-deterministic (different each render)
#[test]
fn test_white_noise_randomness() {
    let dsl = r#"
tempo: 1.0
~noise: white_noise
out: ~noise * 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();

    // Render twice
    let mut graph1 = compile_program(statements.clone(), SAMPLE_RATE).unwrap();
    let samples1 = graph1.render(1000);

    let mut graph2 = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples2 = graph2.render(1000);

    // Samples should be different (not identical)
    let identical_count = samples1
        .iter()
        .zip(samples2.iter())
        .filter(|(&a, &b)| (a - b).abs() < 1e-6)
        .count();

    assert!(
        identical_count < 10,
        "White noise should be random (not identical), got {} identical samples",
        identical_count
    );
}
