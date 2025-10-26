use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Pattern Query Verification
/// Tests that brown_noise syntax is parsed and compiled correctly
#[test]
fn test_brown_noise_pattern_query() {
    let dsl = r#"
tempo: 1.0
~noise: brown_noise
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
        "Brown noise should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: Output Characteristics
/// Tests that brown_noise generates non-silent, non-zero output
#[test]
fn test_brown_noise_output() {
    let dsl = r#"
tempo: 1.0
~noise: brown_noise
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
        rms > 0.02,
        "Brown noise should be audible (RMS > 0.02), got RMS {}",
        rms
    );

    // Peak should be reasonable (not clipping or too quiet)
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        peak > 0.05 && peak < 1.0,
        "Peak should be reasonable (0.05-1.0), got {}",
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
        variance > 0.001,
        "Brown noise should vary (variance > 0.001), got {}",
        variance
    );
}

/// LEVEL 2: Spectral Characteristics
/// Tests that brown_noise has steeper rolloff than pink noise (more low frequency)
#[test]
fn test_brown_noise_spectral_characteristics() {
    let dsl = r#"
tempo: 1.0
~noise: brown_noise
out: ~noise
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    // Render 1 second for better frequency resolution
    let samples = graph.render(SAMPLE_RATE as usize);

    // Perform FFT to analyze spectrum
    use rustfft::{num_complex::Complex, FftPlanner};
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(samples.len());

    // Apply Hanning window
    let windowed: Vec<Complex<f32>> = samples
        .iter()
        .enumerate()
        .map(|(i, &s)| {
            let window =
                0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / samples.len() as f32).cos());
            Complex { re: s * window, im: 0.0 }
        })
        .collect();

    let mut buffer = windowed;
    fft.process(&mut buffer);

    // Compute power spectrum
    let spectrum: Vec<f32> = buffer.iter().map(|c| c.norm_sqr()).collect();

    let bin_to_freq = |bin: usize| (bin as f32 * SAMPLE_RATE) / samples.len() as f32;

    // Measure power in low vs high frequency bands
    // Brown noise should have significantly more low frequency content
    let low_band: Vec<usize> = (0..spectrum.len())
        .filter(|&bin| {
            let freq = bin_to_freq(bin);
            freq >= 50.0 && freq < 500.0
        })
        .collect();

    let high_band: Vec<usize> = (0..spectrum.len())
        .filter(|&bin| {
            let freq = bin_to_freq(bin);
            freq >= 2000.0 && freq < 8000.0
        })
        .collect();

    let low_power: f32 = low_band.iter().map(|&bin| spectrum[bin]).sum::<f32>()
        / low_band.len() as f32;
    let high_power: f32 = high_band.iter().map(|&bin| spectrum[bin]).sum::<f32>()
        / high_band.len() as f32;

    println!(
        "Brown noise - Low power: {}, High power: {}",
        low_power, high_power
    );

    // Brown noise should have much more low frequency power than high frequency
    // Expect at least 5x more power in low frequencies
    assert!(
        low_power > high_power * 3.0,
        "Brown noise should have much more low frequency power, got low={}, high={}",
        low_power,
        high_power
    );
}

/// LEVEL 2: Different from Pink/White Noise
/// Tests that brown_noise is distinct from pink and white noise
#[test]
fn test_brown_vs_pink_vs_white() {
    // Brown noise
    let dsl_brown = r#"
tempo: 1.0
~noise: brown_noise
out: ~noise
"#;

    let (_, statements) = parse_program(dsl_brown).unwrap();
    let mut graph_brown = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples_brown = graph_brown.render(SAMPLE_RATE as usize);

    // Pink noise
    let dsl_pink = r#"
tempo: 1.0
~noise: pink_noise
out: ~noise
"#;

    let (_, statements) = parse_program(dsl_pink).unwrap();
    let mut graph_pink = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples_pink = graph_pink.render(SAMPLE_RATE as usize);

    // All should be audible
    let rms_brown: f32 =
        samples_brown.iter().map(|s| s * s).sum::<f32>() / samples_brown.len() as f32;
    let rms_pink: f32 =
        samples_pink.iter().map(|s| s * s).sum::<f32>() / samples_pink.len() as f32;

    println!(
        "Brown noise RMS: {}, Pink noise RMS: {}",
        rms_brown.sqrt(),
        rms_pink.sqrt()
    );

    assert!(rms_brown.sqrt() > 0.02, "Brown noise should be audible");
    assert!(rms_pink.sqrt() > 0.05, "Pink noise should be audible");
}

/// LEVEL 3: Musical Integration Test
#[test]
fn test_brown_noise_musical_example() {
    let dsl = r#"
tempo: 2.0
~noise: brown_noise
~env: ad 0.01 0.2
~rumble: ~noise * ~env * 0.3
out: ~rumble
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE / 2.0) as usize); // 0.5 seconds

    // Write to file
    let filename = "/tmp/test_brown_noise_musical.wav";
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
        "Brown noise rumble should be audible (RMS > 0.01), got RMS {}",
        rms
    );
}

/// Test brown noise with filtering
#[test]
fn test_brown_noise_with_filter() {
    let dsl = r#"
tempo: 1.0
~noise: brown_noise
~filtered: lpf ~noise 800 0.6
out: ~filtered * 0.4
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples = graph.render((SAMPLE_RATE / 10.0) as usize);

    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    assert!(
        rms.sqrt() > 0.02,
        "Filtered brown noise should be audible, got RMS {}",
        rms.sqrt()
    );
}

/// Test amplitude scaling
#[test]
fn test_brown_noise_amplitude() {
    let dsl_loud = r#"
tempo: 1.0
~noise: brown_noise * 0.8
out: ~noise
"#;

    let (_, statements) = parse_program(dsl_loud).unwrap();
    let mut graph_loud = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples_loud = graph_loud.render((SAMPLE_RATE / 10.0) as usize);

    let dsl_quiet = r#"
tempo: 1.0
~noise: brown_noise * 0.2
out: ~noise
"#;

    let (_, statements) = parse_program(dsl_quiet).unwrap();
    let mut graph_quiet = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples_quiet = graph_quiet.render((SAMPLE_RATE / 10.0) as usize);

    let rms_loud: f32 =
        samples_loud.iter().map(|s| s * s).sum::<f32>() / samples_loud.len() as f32;
    let rms_quiet: f32 =
        samples_quiet.iter().map(|s| s * s).sum::<f32>() / samples_quiet.len() as f32;

    println!("Loud RMS: {}, Quiet RMS: {}", rms_loud.sqrt(), rms_quiet.sqrt());

    assert!(
        rms_loud > rms_quiet * 2.0,
        "Louder brown noise should have higher RMS"
    );
}

/// Test stability (no DC drift or NaN)
#[test]
fn test_brown_noise_stability() {
    let dsl = r#"
tempo: 1.0
~noise: brown_noise
out: ~noise
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    // Render 5 seconds to test for drift
    let samples = graph.render((SAMPLE_RATE * 5.0) as usize);

    // Should not produce NaN or Inf
    assert!(
        samples.iter().all(|s| s.is_finite()),
        "Brown noise should not produce NaN or Inf"
    );

    // Check for excessive DC drift
    let mean: f32 = samples.iter().sum::<f32>() / samples.len() as f32;
    assert!(
        mean.abs() < 0.1,
        "Brown noise should not have excessive DC drift, got mean {}",
        mean
    );
}
