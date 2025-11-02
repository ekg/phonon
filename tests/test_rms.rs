use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Pattern Query Verification
/// Tests that RMS syntax is parsed and compiled correctly
#[test]
fn test_rms_pattern_query() {
    let dsl = r#"
tempo: 1.0
~input: sine 440
~level: ~input # rms 0.1
out: ~level
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
        "RMS should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: RMS Measures Average Amplitude
/// Tests that RMS correctly measures signal amplitude
#[test]
fn test_rms_measures_amplitude() {
    // Known amplitude sine wave
    let dsl = r#"
tempo: 1.0
~sine: sine 440
~rms: ~sine # rms 0.01
out: ~rms
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE * 0.5) as usize);

    // Skip first samples (window fill-in)
    let stable_samples = &samples[1000..];

    // Calculate average RMS value
    let avg_rms: f32 = stable_samples.iter().sum::<f32>() / stable_samples.len() as f32;

    println!("Average RMS of sine wave: {}", avg_rms);

    // Sine wave RMS should be amplitude / sqrt(2) ≈ 0.707
    // Allow some tolerance due to windowing
    assert!(
        avg_rms > 0.6 && avg_rms < 0.8,
        "Sine wave RMS should be ~0.707, got {}",
        avg_rms
    );
}

/// LEVEL 2: Window Size Affects Smoothness
/// Tests that larger windows produce smoother output
#[test]
fn test_rms_window_size() {
    // Small window (fast response)
    let dsl_small = r#"
tempo: 1.0
~impulse: impulse 10.0
~rms: ~impulse # rms 0.001
out: ~rms
"#;

    let (_, statements) = parse_program(dsl_small).unwrap();
    let mut graph_small = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples_small = graph_small.render(SAMPLE_RATE as usize);

    // Large window (slow response)
    let dsl_large = r#"
tempo: 1.0
~impulse: impulse 10.0
~rms: ~impulse # rms 0.1
out: ~rms
"#;

    let (_, statements) = parse_program(dsl_large).unwrap();
    let mut graph_large = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples_large = graph_large.render(SAMPLE_RATE as usize);

    // Measure variability (standard deviation)
    let mean_small: f32 = samples_small.iter().sum::<f32>() / samples_small.len() as f32;
    let variance_small: f32 = samples_small
        .iter()
        .map(|&s| (s - mean_small).powi(2))
        .sum::<f32>()
        / samples_small.len() as f32;

    let mean_large: f32 = samples_large.iter().sum::<f32>() / samples_large.len() as f32;
    let variance_large: f32 = samples_large
        .iter()
        .map(|&s| (s - mean_large).powi(2))
        .sum::<f32>()
        / samples_large.len() as f32;

    println!(
        "Small window variance: {}, Large window variance: {}",
        variance_small, variance_large
    );

    // Large window should be smoother (less variance)
    assert!(
        variance_large < variance_small,
        "Large window should be smoother than small window"
    );
}

/// LEVEL 2: RMS Tracks Amplitude Changes
/// Tests that RMS follows changes in signal amplitude
#[test]
fn test_rms_tracks_changes() {
    // Compare low amplitude vs high amplitude signals
    let dsl_low = r#"
tempo: 1.0
~signal: sine 440 * 0.2
~rms: ~signal # rms 0.01
out: ~rms
"#;

    let (_, statements) = parse_program(dsl_low).unwrap();
    let mut graph_low = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples_low = graph_low.render(SAMPLE_RATE as usize);

    let dsl_high = r#"
tempo: 1.0
~signal: sine 440 * 1.0
~rms: ~signal # rms 0.01
out: ~rms
"#;

    let (_, statements) = parse_program(dsl_high).unwrap();
    let mut graph_high = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples_high = graph_high.render(SAMPLE_RATE as usize);

    // Sample at midpoint after RMS window has stabilized
    let mid = SAMPLE_RATE as usize / 2;
    let rms_low = samples_low[mid];
    let rms_high = samples_high[mid];

    println!(
        "Low amplitude RMS: {}, High amplitude RMS: {}",
        rms_low, rms_high
    );

    // High amplitude should have significantly higher RMS
    // Expected: 0.2/sqrt(2) ≈ 0.14 vs 1.0/sqrt(2) ≈ 0.71
    assert!(
        rms_high > rms_low * 3.0,
        "High amplitude signal should have much higher RMS than low amplitude: {} vs {}",
        rms_high,
        rms_low
    );
}

/// LEVEL 2: RMS of DC Signal
/// Tests RMS of a constant (DC) signal
#[test]
fn test_rms_dc_signal() {
    let dsl = r#"
tempo: 1.0
~dc: 0.5
~rms: ~dc # rms 0.01
out: ~rms
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // Skip first samples (window fill-in)
    let stable_samples = &samples[1000..];
    let avg: f32 = stable_samples.iter().sum::<f32>() / stable_samples.len() as f32;

    println!("RMS of DC 0.5: {}", avg);

    // RMS of DC signal = abs(DC value)
    assert!(
        (avg - 0.5).abs() < 0.01,
        "RMS of DC 0.5 should be 0.5, got {}",
        avg
    );
}

/// LEVEL 2: RMS Stability
/// Tests that RMS doesn't blow up or produce NaN
#[test]
fn test_rms_stability() {
    let dsl = r#"
tempo: 1.0
~noise: white_noise
~rms: ~noise # rms 0.05
out: ~rms * 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // Check for NaN or Inf
    let has_nan = samples.iter().any(|s| s.is_nan() || s.is_infinite());
    assert!(!has_nan, "RMS should not produce NaN or Inf");

    // Check for reasonable output
    let max_val = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        max_val < 10.0,
        "RMS output should be reasonable, got max {}",
        max_val
    );
}

/// LEVEL 3: Musical Example - Envelope Follower
/// Tests RMS used as envelope follower for sidechain-like effect
#[test]
fn test_rms_envelope_follower() {
    let dsl = r#"
tempo: 2.0
~kick: impulse 2.0
~kick_rms: ~kick # rms 0.01
~pad: saw 110
~ducked: ~pad * (1.0 - ~kick_rms * 2.0)
out: ~ducked * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE * 2.0) as usize);

    // Should produce audible output
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    assert!(
        rms.sqrt() > 0.05,
        "Envelope follower should produce audible output, got RMS {}",
        rms.sqrt()
    );
}

/// LEVEL 3: Pattern-Modulated Window Size
/// Tests that RMS window can be pattern-modulated
#[test]
fn test_rms_pattern_window() {
    let dsl = r#"
tempo: 2.0
~input: sine 440
~windows: "0.01 0.05"
~rms: ~input # rms ~windows
out: ~rms
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let graph = compile_program(statements, SAMPLE_RATE);

    assert!(
        graph.is_ok(),
        "Pattern-modulated RMS window should compile: {:?}",
        graph.err()
    );
}

/// LEVEL 3: RMS as VU Meter
/// Tests RMS for loudness metering
#[test]
fn test_rms_vu_meter() {
    let dsl = r#"
tempo: 1.0
~music: saw 110 + sine 220 * 0.5 + sine 440 * 0.25
~level: ~music # rms 0.1
out: ~level
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // Skip first samples (window fill-in)
    let stable_samples = &samples[5000..];
    let avg_level: f32 = stable_samples.iter().sum::<f32>() / stable_samples.len() as f32;

    println!("Average level (VU meter): {}", avg_level);

    // Should measure something reasonable
    assert!(
        avg_level > 0.3 && avg_level < 1.5,
        "Level should be reasonable, got {}",
        avg_level
    );
}
