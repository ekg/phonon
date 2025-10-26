use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Pattern Query Verification
/// Tests that notch filter syntax is parsed and compiled correctly
#[test]
fn test_notch_pattern_query() {
    let dsl = r#"
tempo: 1.0
~input: sine 440
~filtered: ~input # notch 440 1.0
out: ~filtered
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
        "Notch filter should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: Notch Attenuates Center Frequency
/// Tests that notch filter removes frequencies at center frequency
#[test]
fn test_notch_attenuates_center() {
    let dsl = r#"
tempo: 1.0
~input: sine 440
~notched: ~input # notch 440 2.0
out: ~notched
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // Calculate RMS
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    println!("Notched 440Hz signal RMS: {}", rms.sqrt());

    // Should be significantly attenuated (notch removes 440Hz)
    assert!(
        rms.sqrt() < 0.1,
        "440Hz should be notched out (RMS should be very low), got {}",
        rms.sqrt()
    );
}

/// LEVEL 2: Notch Passes Other Frequencies
/// Tests that notch filter passes frequencies away from center
#[test]
fn test_notch_passes_other_frequencies() {
    let dsl = r#"
tempo: 1.0
~input: sine 880
~notched: ~input # notch 440 2.0
out: ~notched
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // Calculate RMS
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    println!("880Hz through 440Hz notch RMS: {}", rms.sqrt());

    // Should pass through with minimal attenuation
    assert!(
        rms.sqrt() > 0.5,
        "880Hz should pass through 440Hz notch, got RMS {}",
        rms.sqrt()
    );
}

/// LEVEL 2: Notch Q Factor (Width)
/// Tests that Q affects notch width
#[test]
fn test_notch_q_factor() {
    // Narrow notch (high Q)
    let dsl_narrow = r#"
tempo: 1.0
~input: sine 450
~notched: ~input # notch 440 10.0
out: ~notched
"#;

    let (_, statements) = parse_program(dsl_narrow).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples_narrow = graph.render(SAMPLE_RATE as usize);
    let rms_narrow: f32 = samples_narrow.iter().map(|s| s * s).sum::<f32>() / samples_narrow.len() as f32;

    // Wide notch (low Q)
    let dsl_wide = r#"
tempo: 1.0
~input: sine 450
~notched: ~input # notch 440 0.5
out: ~notched
"#;

    let (_, statements) = parse_program(dsl_wide).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples_wide = graph.render(SAMPLE_RATE as usize);
    let rms_wide: f32 = samples_wide.iter().map(|s| s * s).sum::<f32>() / samples_wide.len() as f32;

    println!("450Hz through 440Hz notch - Narrow Q RMS: {}, Wide Q RMS: {}", rms_narrow.sqrt(), rms_wide.sqrt());

    // Narrow notch should pass more of 450Hz (frequency is outside narrow notch)
    // Wide notch should attenuate more of 450Hz (frequency is within wide notch)
    assert!(
        rms_narrow.sqrt() > rms_wide.sqrt(),
        "Narrow notch (high Q) should pass more than wide notch (low Q) for nearby frequencies"
    );
}

/// LEVEL 2: Notch Stability
/// Tests that notch filter doesn't blow up or produce NaN
#[test]
fn test_notch_stability() {
    let dsl = r#"
tempo: 1.0
~input: white_noise
~notched: ~input # notch 1000 5.0
out: ~notched * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // Check for NaN or Inf
    let has_nan = samples.iter().any(|s| s.is_nan() || s.is_infinite());
    assert!(!has_nan, "Notch filter should not produce NaN or Inf");

    // Check for reasonable output
    let max_val = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        max_val < 10.0,
        "Notch filter output should be reasonable, got max {}",
        max_val
    );
}

/// LEVEL 3: Musical Example - Remove Hum
/// Tests notch used to remove 60Hz hum
#[test]
fn test_notch_remove_hum() {
    let dsl = r#"
tempo: 1.0
~clean_signal: sine 440
~hum: sine 60 * 0.3
~noisy: ~clean_signal + ~hum
~dehum: ~noisy # notch 60 5.0
out: ~dehum * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // Should have audible signal (440Hz should pass through)
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    assert!(
        rms.sqrt() > 0.1,
        "De-hummed signal should be audible, got RMS {}",
        rms.sqrt()
    );
}

/// LEVEL 3: Pattern-Modulated Notch Frequency
/// Tests that notch frequency can be patterns
#[test]
fn test_notch_pattern_frequency() {
    let dsl = r#"
tempo: 1.0
~input: saw 110
~notch_freqs: "440 880"
~notched: ~input # notch ~notch_freqs 2.0
out: ~notched * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let graph = compile_program(statements, SAMPLE_RATE);

    assert!(
        graph.is_ok(),
        "Pattern-modulated notch should compile: {:?}",
        graph.err()
    );
}

/// LEVEL 3: Notch Removes Resonance
/// Tests notch used to remove resonant peak
#[test]
fn test_notch_remove_resonance() {
    let dsl = r#"
tempo: 1.0
~source: saw 110
~resonant: ~source # lpf 2000 8.0
~clean: ~resonant # notch 2000 3.0
out: ~clean * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // Should produce audible output
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    assert!(
        rms.sqrt() > 0.05,
        "Resonance-removed signal should be audible, got RMS {}",
        rms.sqrt()
    );
}

/// LEVEL 3: Multiple Notches
/// Tests cascading multiple notch filters
#[test]
fn test_multiple_notches() {
    let dsl = r#"
tempo: 1.0
~input: white_noise
~notch1: ~input # notch 440 3.0
~notch2: ~notch1 # notch 880 3.0
~notch3: ~notch2 # notch 1320 3.0
out: ~notch3 * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // Should produce filtered noise
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    assert!(
        rms.sqrt() > 0.05,
        "Multiple notches should still pass signal, got RMS {}",
        rms.sqrt()
    );

    // Check stability
    let has_nan = samples.iter().any(|s| s.is_nan() || s.is_infinite());
    assert!(!has_nan, "Multiple notches should be stable");
}
