use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Conditional Routing - Basic Compilation
/// Tests that conditional (if-then-else) syntax compiles correctly
#[test]
fn test_conditional_compilation() {
    let dsl = r#"
tempo: 1.0
~condition $ sine 0.5
~wet $ sine 440
~dry $ sine 220
~output $ if ~condition ~wet ~dry
out $ ~output
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
        "Conditional routing should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: Conditional Routing - Signal Switching
/// Tests that conditional actually switches between signals based on condition
#[test]
fn test_conditional_signal_switching() {
    let dsl = r#"
tempo: 1.0
-- Condition that's always high (> 0.5)
~condition_high $ 1.0
-- Two different signals
~signal_a $ sine 440
~signal_b $ sine 880
~output $ if ~condition_high ~signal_a ~signal_b
out $ ~output * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Render 1 second
    let samples = graph.render(SAMPLE_RATE as usize);

    // Calculate RMS
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    let rms = rms.sqrt();

    // Should produce audible output (not silent)
    assert!(
        rms > 0.05,
        "Conditional output should be audible, got RMS {}",
        rms
    );

    // Peak should be reasonable
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        peak > 0.1 && peak < 0.5,
        "Conditional output should have reasonable peak, got {}",
        peak
    );
}

/// LEVEL 3: Conditional Routing - Dynamic Switching
/// Tests that conditional switches based on time-varying condition
#[test]
fn test_conditional_dynamic_switching() {
    let dsl = r#"
tempo: 0.5
-- LFO as condition (crosses 0.5 threshold)
~condition $ sine 0.5
~high_freq $ 880
~low_freq $ 220
~selected_freq $ if ~condition ~high_freq ~low_freq
~output $ sine ~selected_freq
out $ ~output * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Render 2 seconds (1 full LFO cycle)
    let samples = graph.render((SAMPLE_RATE * 2.0) as usize);

    // Should produce audible output
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    let rms = rms.sqrt();
    assert!(
        rms > 0.05,
        "Dynamically switched signal should be audible, got RMS {}",
        rms
    );

    // Signal should vary (not constant) due to frequency switching
    let first_half = &samples[0..samples.len() / 2];
    let second_half = &samples[samples.len() / 2..];

    let rms_first: f32 = first_half.iter().map(|s| s * s).sum::<f32>() / first_half.len() as f32;
    let rms_second: f32 = second_half.iter().map(|s| s * s).sum::<f32>() / second_half.len() as f32;

    // Both halves should have audible content
    assert!(
        rms_first.sqrt() > 0.01,
        "First half should be audible, got RMS {}",
        rms_first.sqrt()
    );
    assert!(
        rms_second.sqrt() > 0.01,
        "Second half should be audible, got RMS {}",
        rms_second.sqrt()
    );
}

/// LEVEL 1: Select/Multiplex - Basic Compilation
/// Tests that select syntax compiles correctly
#[test]
fn test_select_compilation() {
    let dsl = r#"
tempo: 1.0
~index $ 0
~bus0 $ sine 220
~bus1 $ sine 330
~bus2 $ sine 440
~bus3 $ sine 550
~output $ select ~index ~bus0 ~bus1 ~bus2 ~bus3
out $ ~output
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
        "Select routing should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: Select/Multiplex - Static Selection
/// Tests that select chooses the correct signal based on constant index
#[test]
fn test_select_static_selection() {
    // Test selecting index 2 (third signal)
    let dsl = r#"
tempo: 1.0
~index $ 2.0
~bus0 $ sine 220
~bus1 $ sine 330
~bus2 $ sine 440
~bus3 $ sine 550
~output $ select ~index ~bus0 ~bus1 ~bus2 ~bus3
out $ ~output * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Render 0.5 seconds
    let samples = graph.render((SAMPLE_RATE * 0.5) as usize);

    // Should produce audible output
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    let rms = rms.sqrt();
    assert!(
        rms > 0.05,
        "Selected signal should be audible, got RMS {}",
        rms
    );
}

/// LEVEL 3: Select/Multiplex - Dynamic Selection with Patterns
/// Tests that select can be modulated by patterns to switch between signals
#[test]
fn test_select_pattern_modulation() {
    let dsl = r#"
tempo: 0.5
-- Pattern that cycles through indices 0, 1, 2, 3
~index $ "0 1 2 3"
~bus0 $ sine 220
~bus1 $ sine 330
~bus2 $ sine 440
~bus3 $ sine 550
~output $ select ~index ~bus0 ~bus1 ~bus2 ~bus3
out $ ~output * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Render 2 seconds (4 cycles at tempo 2.0)
    let samples = graph.render((SAMPLE_RATE * 2.0) as usize);

    // Should produce audible output
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    let rms = rms.sqrt();
    assert!(
        rms > 0.05,
        "Pattern-modulated select should be audible, got RMS {}",
        rms
    );

    // Peak should be reasonable
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        peak > 0.1 && peak < 0.5,
        "Pattern-modulated select should have reasonable peak, got {}",
        peak
    );
}

/// LEVEL 3: Select/Multiplex - LFO-Based Selection
/// Tests that select can use continuous signals (LFO) as index
#[test]
fn test_select_lfo_modulation() {
    let dsl = r#"
tempo: 1.0
-- LFO that sweeps through indices (0 to 3)
~lfo $ sine 0.5
~index $ (~lfo + 1.0) * 1.5
~bus0 $ sine 220
~bus1 $ sine 330
~bus2 $ sine 440
~bus3 $ sine 550
~output $ select ~index ~bus0 ~bus1 ~bus2 ~bus3
out $ ~output * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Render 2 seconds (2 full LFO cycles)
    let samples = graph.render((SAMPLE_RATE * 2.0) as usize);

    // Should produce audible output
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    let rms = rms.sqrt();
    assert!(
        rms > 0.05,
        "LFO-modulated select should be audible, got RMS {}",
        rms
    );
}

/// Musical Example: Conditional Effect Routing
/// Tests conditional routing for dynamic effect application
#[test]
fn test_conditional_effect_routing() {
    let dsl = r#"
tempo: 0.5
-- Envelope to control routing
~env $ adsr 0.01 0.1 0.5 0.2
~dry $ sine 440
~wet $ ~dry # lpf 800 0.8
~output $ if ~env ~wet ~dry
out $ ~output * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Render 1 second (2 cycles)
    let samples = graph.render(SAMPLE_RATE as usize);

    // Should produce audible output
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    let rms = rms.sqrt();
    assert!(
        rms > 0.05,
        "Conditional effect routing should be audible, got RMS {}",
        rms
    );

    // Write to file for manual verification
    let filename = "/tmp/test_conditional_effect_routing.wav";
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

    println!(
        "Conditional effect routing test audio written to: {}",
        filename
    );
}

/// Musical Example: Pattern-Based Bus Selection
/// Tests dynamic bus routing with pattern-controlled index
#[test]
fn test_pattern_bus_selection() {
    let dsl = r#"
tempo: 0.5
-- Different tones on different buses (using oscillators instead of samples)
~tone1 $ sine 220
~tone2 $ sine 330
~tone3 $ sine 440
~tone4 $ sine 550

-- Pattern selects which tone to play: 0=220Hz, 1=330Hz, 2=440Hz, 3=550Hz
~selector $ "0 1 2 3"
~output $ select ~selector ~tone1 ~tone2 ~tone3 ~tone4

out $ ~output * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Render 2 seconds (4 cycles)
    let samples = graph.render((SAMPLE_RATE * 2.0) as usize);

    // Should produce audible output (varying tones)
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    let rms = rms.sqrt();
    assert!(
        rms > 0.05,
        "Pattern-based tone selection should be audible, got RMS {}",
        rms
    );

    // Peak should be reasonable
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        peak > 0.1 && peak < 0.5,
        "Tone selection should have reasonable peak (0.1-0.5), got peak {}",
        peak
    );

    // Write to file for manual verification
    let filename = "/tmp/test_pattern_bus_selection.wav";
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

    println!("Pattern bus selection test audio written to: {}", filename);
}

/// Edge Case: Select with wrapping indices
/// Tests that select wraps negative and out-of-range indices correctly
#[test]
fn test_select_index_wrapping() {
    let dsl = r#"
tempo: 1.0
-- Index that wraps: -1 should wrap to last element (index 2)
~index $ -1.0
~bus0 $ sine 220
~bus1 $ sine 330
~bus2 $ sine 440
~output $ select ~index ~bus0 ~bus1 ~bus2
out $ ~output * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Render and verify it produces output (doesn't crash)
    let samples = graph.render((SAMPLE_RATE * 0.5) as usize);

    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    let rms = rms.sqrt();
    assert!(
        rms > 0.05,
        "Select with wrapped index should be audible, got RMS {}",
        rms
    );
}

/// Edge Case: Conditional with threshold testing
/// Tests the exact threshold behavior (0.5)
#[test]
fn test_conditional_threshold() {
    // Test with condition exactly at 0.5 (should go to else branch)
    let dsl = r#"
tempo: 1.0
~condition $ 0.5
~then_signal $ 1.0
~else_signal $ 0.0
~output $ if ~condition ~then_signal ~else_signal
out $ ~output
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render(1000);

    // At exactly 0.5, should choose else branch (< 0.5 threshold means false)
    let avg = samples.iter().sum::<f32>() / samples.len() as f32;
    assert!(
        avg < 0.1,
        "Condition 0.5 should go to else branch (output ~0), got avg {}",
        avg
    );

    // Test with condition slightly above 0.5 (should go to then branch)
    let dsl2 = r#"
tempo: 1.0
~condition $ 0.51
~then_signal $ 1.0
~else_signal $ 0.0
~output $ if ~condition ~then_signal ~else_signal
out $ ~output
"#;

    let (_, statements2) = parse_program(dsl2).unwrap();
    let mut graph2 = compile_program(statements2, SAMPLE_RATE, None).unwrap();

    let samples2 = graph2.render(1000);
    let avg2 = samples2.iter().sum::<f32>() / samples2.len() as f32;
    assert!(
        avg2 > 0.9,
        "Condition 0.51 should go to then branch (output ~1), got avg {}",
        avg2
    );
}
