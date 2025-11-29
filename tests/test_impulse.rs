use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Pattern Query Verification
/// Tests that impulse syntax is parsed and compiled correctly
#[test]
fn test_impulse_pattern_query() {
    let dsl = r#"
tempo: 1.0
~pulse $ impulse 2.0
out $ ~pulse
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
        "Impulse should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: Basic Impulse Generation
/// Tests that impulse generates single-sample spikes at correct frequency
#[test]
fn test_impulse_basic() {
    let dsl = r#"
tempo: 1.0
out $ impulse 2.0
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Render 1 second (should have 2 impulses at 2 Hz)
    let samples = graph.render(SAMPLE_RATE as usize);

    // Count impulses (values significantly above 0.0)
    let impulse_count = samples.iter().filter(|&&s| s > 0.5).count();

    println!(
        "Found {} impulses in {} samples",
        impulse_count,
        samples.len()
    );

    // Should have approximately 2 impulses (at 2 Hz for 1 second)
    assert!(
        impulse_count >= 1 && impulse_count <= 3,
        "Expected ~2 impulses at 2 Hz, got {}",
        impulse_count
    );

    // Impulses should be single samples (not sustained)
    let mut in_impulse = false;
    let mut max_impulse_width = 0;
    let mut current_width = 0;

    for &sample in &samples {
        if sample > 0.5 {
            if !in_impulse {
                in_impulse = true;
                current_width = 1;
            } else {
                current_width += 1;
            }
        } else {
            if in_impulse {
                max_impulse_width = max_impulse_width.max(current_width);
                in_impulse = false;
            }
        }
    }

    assert!(
        max_impulse_width <= 2,
        "Impulses should be single samples, got width {}",
        max_impulse_width
    );
}

/// LEVEL 2: Impulse Frequency Accuracy
/// Tests that impulse frequency matches expected rate
#[test]
fn test_impulse_frequency() {
    let dsl = r#"
tempo: 1.0
out $ impulse 10.0
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Render 1 second (should have 10 impulses at 10 Hz)
    let samples = graph.render(SAMPLE_RATE as usize);

    let impulse_count = samples.iter().filter(|&&s| s > 0.5).count();

    println!("10 Hz: Found {} impulses", impulse_count);

    // Should have approximately 10 impulses (±1 for edge effects)
    assert!(
        impulse_count >= 9 && impulse_count <= 11,
        "Expected ~10 impulses at 10 Hz, got {}",
        impulse_count
    );
}

/// LEVEL 2: Low Frequency Impulse
/// Tests that low frequency impulses work correctly
#[test]
fn test_impulse_low_frequency() {
    let dsl = r#"
tempo: 1.0
out $ impulse 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Render 4 seconds (should have 2 impulses at 0.5 Hz)
    let samples = graph.render((SAMPLE_RATE * 4.0) as usize);

    let impulse_count = samples.iter().filter(|&&s| s > 0.5).count();

    println!("0.5 Hz over 4s: Found {} impulses", impulse_count);

    // Should have approximately 2 impulses
    assert!(
        impulse_count >= 1 && impulse_count <= 3,
        "Expected ~2 impulses at 0.5 Hz over 4s, got {}",
        impulse_count
    );
}

/// LEVEL 2: Impulse Spacing Verification
/// Tests that impulses are evenly spaced
#[test]
fn test_impulse_spacing() {
    let dsl = r#"
tempo: 1.0
out $ impulse 4.0
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Render 1 second
    let samples = graph.render(SAMPLE_RATE as usize);

    // Find impulse positions
    let mut impulse_positions = Vec::new();
    for (i, &sample) in samples.iter().enumerate() {
        if sample > 0.5 {
            impulse_positions.push(i);
        }
    }

    println!("Impulse positions: {:?}", impulse_positions);

    // Calculate spacing between impulses
    if impulse_positions.len() >= 2 {
        let spacings: Vec<_> = impulse_positions.windows(2).map(|w| w[1] - w[0]).collect();

        println!("Spacings: {:?}", spacings);

        // At 4 Hz and 44100 Hz sample rate, spacing should be 44100/4 = 11025 samples
        let expected_spacing = SAMPLE_RATE / 4.0;

        for spacing in spacings {
            let spacing_f = spacing as f32;
            let error = (spacing_f - expected_spacing).abs() / expected_spacing;
            assert!(
                error < 0.01,
                "Spacing error too large: expected {}, got {}, error {}%",
                expected_spacing,
                spacing,
                error * 100.0
            );
        }
    }
}

/// LEVEL 2: Impulse Amplitude
/// Tests that impulse amplitude is 1.0
#[test]
fn test_impulse_amplitude() {
    let dsl = r#"
tempo: 1.0
out $ impulse 5.0
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render(SAMPLE_RATE as usize);

    // Find peak value (should be 1.0)
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    println!("Impulse peak amplitude: {}", peak);

    assert!(
        (peak - 1.0).abs() < 0.01,
        "Impulse amplitude should be 1.0, got {}",
        peak
    );
}

/// LEVEL 3: Impulse Multiplied with Tone
/// Tests musical use case: impulse gating a continuous tone
#[test]
fn test_impulse_with_tone() {
    let dsl = r#"
tempo: 0.5
~pulse $ impulse 2.0
~tone $ sine 440
out $ ~tone * ~pulse
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE * 1.0) as usize);

    // Should have audible impulses (tone gated by impulses)
    let nonzero_count = samples.iter().filter(|&&s| s.abs() > 0.001).count();

    println!("Tone test: {} non-zero samples", nonzero_count);

    assert!(
        nonzero_count > 0,
        "Should have audible signal, got {} non-zero samples",
        nonzero_count
    );

    // Verify we have at least 2 impulses worth of samples (at 2 Hz over 1 second)
    assert!(
        nonzero_count >= 2,
        "Should have at least 2 impulse events, got {}",
        nonzero_count
    );
}

/// LEVEL 3: Pattern-Modulated Impulse Frequency
/// Tests that impulse frequency can be modulated by patterns
#[test]
fn test_impulse_pattern_frequency() {
    let dsl = r#"
tempo: 1.0
~freq_pattern $ "2.0 4.0"
out $ impulse ~freq_pattern
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let graph = compile_program(statements, SAMPLE_RATE, None);

    assert!(
        graph.is_ok(),
        "Pattern-modulated impulse should compile: {:?}",
        graph.err()
    );
}

/// LEVEL 3: Impulse as Clock Signal
/// Tests impulse used as a rhythmic clock
#[test]
fn test_impulse_clock() {
    let dsl = r#"
tempo: 0.5
~clock $ impulse 8.0
~bass $ sine 55
~kick $ ~bass * ~clock
out $ ~kick
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE * 0.5) as usize);

    // Should have rhythmic pulses (8 Hz = fast kick pattern)
    // RMS will be low because impulses are sparse (only 4 samples out of 22050)
    let nonzero_count = samples.iter().filter(|&&s| s.abs() > 0.001).count();

    println!(
        "Clock test: {} non-zero samples out of {}",
        nonzero_count,
        samples.len()
    );

    assert!(
        nonzero_count > 0,
        "Clock should produce audible rhythm, got {} non-zero samples",
        nonzero_count
    );
}
