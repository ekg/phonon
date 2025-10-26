use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Pattern Query Verification
/// Tests that Tremolo syntax is parsed and compiled correctly
#[test]
fn test_tremolo_pattern_query() {
    let dsl = r#"
tempo: 1.0
~carrier: sine 440
~tremolo: ~carrier # tremolo 5.0 0.5
out: ~tremolo
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
        "Tremolo should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: Tremolo Modulates Amplitude
/// Tests that tremolo creates amplitude modulation
#[test]
fn test_tremolo_modulates_amplitude() {
    let dsl = r#"
tempo: 1.0
~carrier: sine 440
~tremolo: ~carrier # tremolo 4.0 0.5
out: ~tremolo
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE * 0.5) as usize);

    // Calculate amplitude variation over time
    let chunk_size = (SAMPLE_RATE * 0.025) as usize; // 25ms chunks
    let mut amplitudes = Vec::new();

    for chunk in samples.chunks(chunk_size) {
        let rms: f32 = chunk.iter().map(|s| s * s).sum::<f32>() / chunk.len() as f32;
        amplitudes.push(rms.sqrt());
    }

    // Should have varying amplitude (not constant)
    let min_amp = amplitudes.iter().cloned().fold(f32::INFINITY, f32::min);
    let max_amp = amplitudes.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

    println!(
        "Amplitude variation: min={:.4}, max={:.4}, ratio={:.2}",
        min_amp,
        max_amp,
        max_amp / min_amp.max(0.001)
    );

    // With 50% depth, amplitude should vary significantly
    assert!(
        max_amp / min_amp.max(0.001) > 1.5,
        "Tremolo should create amplitude variation, got min={}, max={}",
        min_amp,
        max_amp
    );
}

/// LEVEL 2: Tremolo Rate Controls Speed
/// Tests that rate parameter affects modulation speed
#[test]
fn test_tremolo_rate() {
    // Fast tremolo
    let dsl_fast = r#"
tempo: 1.0
~carrier: sine 440
~tremolo: ~carrier # tremolo 8.0 0.8
out: ~tremolo
"#;

    let (_, statements) = parse_program(dsl_fast).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples_fast = graph.render((SAMPLE_RATE * 0.5) as usize);

    // Count zero crossings in envelope (approximate cycle count)
    let chunk_size = (SAMPLE_RATE * 0.01) as usize;
    let mut crossings_fast = 0;
    for window in samples_fast.windows(chunk_size) {
        let rms1: f32 = window[..chunk_size / 2]
            .iter()
            .map(|s| s * s)
            .sum::<f32>()
            / (chunk_size / 2) as f32;
        let rms2: f32 = window[chunk_size / 2..]
            .iter()
            .map(|s| s * s)
            .sum::<f32>()
            / (chunk_size / 2) as f32;

        if (rms1 - rms2).abs() > 0.05 {
            crossings_fast += 1;
        }
    }

    println!("Fast tremolo crossings: {}", crossings_fast);

    // Should detect multiple cycles
    assert!(
        crossings_fast > 5,
        "Fast tremolo should have many cycles, got {}",
        crossings_fast
    );
}

/// LEVEL 2: Tremolo Depth Controls Amount
/// Tests that depth parameter affects modulation intensity
#[test]
fn test_tremolo_depth() {
    // Shallow tremolo
    let dsl_shallow = r#"
tempo: 1.0
~carrier: sine 440
~tremolo: ~carrier # tremolo 5.0 0.2
out: ~tremolo
"#;

    // Deep tremolo
    let dsl_deep = r#"
tempo: 1.0
~carrier: sine 440
~tremolo: ~carrier # tremolo 5.0 0.9
out: ~tremolo
"#;

    let (_, statements_shallow) = parse_program(dsl_shallow).unwrap();
    let mut graph_shallow = compile_program(statements_shallow, SAMPLE_RATE).unwrap();
    let samples_shallow = graph_shallow.render((SAMPLE_RATE * 0.4) as usize);

    let (_, statements_deep) = parse_program(dsl_deep).unwrap();
    let mut graph_deep = compile_program(statements_deep, SAMPLE_RATE).unwrap();
    let samples_deep = graph_deep.render((SAMPLE_RATE * 0.4) as usize);

    // Calculate amplitude variation for each
    let calc_variation = |samples: &[f32]| {
        let chunk_size = (SAMPLE_RATE * 0.05) as usize;
        let mut amps = Vec::new();
        for chunk in samples.chunks(chunk_size) {
            let rms: f32 = chunk.iter().map(|s| s * s).sum::<f32>() / chunk.len() as f32;
            amps.push(rms.sqrt());
        }
        let min = amps.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = amps.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        max / min.max(0.001)
    };

    let variation_shallow = calc_variation(&samples_shallow);
    let variation_deep = calc_variation(&samples_deep);

    println!(
        "Amplitude variation: shallow={:.2}, deep={:.2}",
        variation_shallow, variation_deep
    );

    // Deep should have more variation than shallow
    assert!(
        variation_deep > variation_shallow,
        "Deep tremolo should vary more than shallow, got shallow={:.2}, deep={:.2}",
        variation_shallow,
        variation_deep
    );
}

/// LEVEL 2: Zero Depth Bypasses Effect
/// Tests that depth=0 passes signal with minimal modulation
#[test]
fn test_tremolo_zero_depth() {
    let dsl = r#"
tempo: 1.0
~carrier: sine 440
~tremolo: ~carrier # tremolo 5.0 0.0
out: ~tremolo
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE * 1.0) as usize);

    // With zero depth, overall RMS should be relatively constant
    // Split into two halves and compare RMS
    let mid = samples.len() / 2;
    let rms1: f32 = samples[..mid]
        .iter()
        .map(|s| s * s)
        .sum::<f32>()
        / mid as f32;
    let rms2: f32 = samples[mid..]
        .iter()
        .map(|s| s * s)
        .sum::<f32>()
        / (samples.len() - mid) as f32;

    let ratio = rms1.sqrt() / rms2.sqrt().max(0.001);

    // Should be very close (within 20%)
    assert!(
        (ratio - 1.0).abs() < 0.2,
        "Zero depth should produce consistent amplitude, got ratio={:.2}",
        ratio
    );
}

/// LEVEL 2: Tremolo Stability
/// Tests that tremolo doesn't produce NaN or Inf
#[test]
fn test_tremolo_stability() {
    let dsl = r#"
tempo: 1.0
~carrier: saw 110
~tremolo: ~carrier # tremolo 6.0 0.7
out: ~tremolo
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE * 1.0) as usize);

    // Check for NaN or Inf
    let has_nan = samples.iter().any(|s| s.is_nan() || s.is_infinite());
    assert!(!has_nan, "Tremolo should not produce NaN or Inf");

    // Check for reasonable output
    let max_val = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        max_val < 10.0,
        "Tremolo output should be reasonable, got max {}",
        max_val
    );
}

/// LEVEL 2: Pattern Modulation
/// Tests that rate and depth can be pattern-modulated
#[test]
fn test_tremolo_pattern_modulation() {
    let dsl = r#"
tempo: 2.0
~rate_lfo: sine 0.5 * 2.0 + 5.0
~depth_lfo: sine 0.3 * 0.3 + 0.5
~carrier: sine 440
~tremolo: ~carrier # tremolo ~rate_lfo ~depth_lfo
out: ~tremolo
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE * 1.0) as usize);

    // Should produce varying tremolo effect
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    assert!(
        rms.sqrt() > 0.05,
        "Pattern-modulated tremolo should be audible, got RMS {}",
        rms.sqrt()
    );
}

/// LEVEL 3: Musical Example - Classic Tremolo
/// Tests classic electric guitar tremolo effect
#[test]
fn test_tremolo_classic() {
    let dsl = r#"
tempo: 2.0
~guitar: saw 220
~tremolo: ~guitar # tremolo 6.0 0.6
out: ~tremolo * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE * 1.0) as usize);

    // Should produce audible tremolo
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    assert!(
        rms.sqrt() > 0.05,
        "Classic tremolo should be audible, got RMS {}",
        rms.sqrt()
    );
}

/// LEVEL 3: Musical Example - Slow Fade Tremolo
/// Tests slow tremolo for swelling effects
#[test]
fn test_tremolo_slow_swell() {
    let dsl = r#"
tempo: 1.0
~pad: sine 220
~tremolo: ~pad # tremolo 0.5 0.8
out: ~tremolo * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE * 3.0) as usize);

    // Should produce slow amplitude swell
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    assert!(
        rms.sqrt() > 0.05,
        "Slow tremolo should be audible, got RMS {}",
        rms.sqrt()
    );
}
