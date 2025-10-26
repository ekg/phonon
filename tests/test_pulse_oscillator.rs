use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Pattern Query Verification
/// Tests that pulse syntax is parsed and compiled correctly
#[test]
fn test_pulse_pattern_query() {
    let dsl = r#"
tempo: 1.0
~pulse: pulse 440 0.5
out: ~pulse * 0.3
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
        "Pulse should compile successfully: {:?}",
        graph.err()
    );
}

/// Helper function to compute FFT and find peak frequencies
fn find_peak_frequencies(samples: &[f32], sample_rate: f32, num_peaks: usize) -> Vec<f32> {
    use rustfft::{FftPlanner, num_complex::Complex};

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(samples.len());

    let mut buffer: Vec<Complex<f32>> = samples.iter()
        .map(|&s| Complex { re: s, im: 0.0 })
        .collect();

    fft.process(&mut buffer);

    // Compute magnitude spectrum (first half only - nyquist)
    let magnitudes: Vec<f32> = buffer[0..buffer.len()/2]
        .iter()
        .map(|c| c.norm())
        .collect();

    // Find peaks
    let mut peaks: Vec<(usize, f32)> = magnitudes.iter()
        .enumerate()
        .filter(|(i, &mag)| {
            if *i == 0 || *i >= magnitudes.len() - 1 {
                return false;
            }
            mag > magnitudes[i-1] && mag > magnitudes[i+1] && mag > 0.01
        })
        .map(|(i, &mag)| (i, mag))
        .collect();

    // Sort by magnitude descending
    peaks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // Convert bin indices to frequencies
    let bin_to_freq = sample_rate / samples.len() as f32;
    peaks.iter()
        .take(num_peaks)
        .map(|(bin, _)| *bin as f32 * bin_to_freq)
        .collect()
}

/// LEVEL 2: Harmonic Content Analysis
/// Tests that pulse width affects harmonic content
#[test]
fn test_pulse_harmonic_content() {
    // Narrow pulse (10%) - should have more harmonics
    let dsl_narrow = r#"
tempo: 1.0
~pulse: pulse 440 0.1
out: ~pulse * 0.5
"#;
    let (_, statements) = parse_program(dsl_narrow).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples_narrow = graph.render(SAMPLE_RATE as usize);
    let peaks_narrow = find_peak_frequencies(&samples_narrow, SAMPLE_RATE, 15);

    // Wide pulse (90%) - should be similar to 10% (inverted)
    let dsl_wide = r#"
tempo: 1.0
~pulse: pulse 440 0.9
out: ~pulse * 0.5
"#;
    let (_, statements) = parse_program(dsl_wide).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples_wide = graph.render(SAMPLE_RATE as usize);
    let peaks_wide = find_peak_frequencies(&samples_wide, SAMPLE_RATE, 15);

    // 50% duty cycle (square wave) - should have only odd harmonics
    let dsl_square = r#"
tempo: 1.0
~pulse: pulse 440 0.5
out: ~pulse * 0.5
"#;
    let (_, statements) = parse_program(dsl_square).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples_square = graph.render(SAMPLE_RATE as usize);
    let peaks_square = find_peak_frequencies(&samples_square, SAMPLE_RATE, 10);

    println!("Narrow pulse (10%) peaks: {:?}", peaks_narrow);
    println!("Wide pulse (90%) peaks: {:?}", peaks_wide);
    println!("Square wave (50%) peaks: {:?}", peaks_square);

    // All should have fundamental at 440 Hz
    assert!(
        peaks_narrow.iter().any(|&f| (f - 440.0).abs() < 30.0),
        "Narrow pulse should have fundamental at 440 Hz"
    );
    assert!(
        peaks_square.iter().any(|&f| (f - 440.0).abs() < 30.0),
        "Square wave should have fundamental at 440 Hz"
    );

    // Square wave should have odd harmonics (3rd harmonic at 1320 Hz)
    let has_third_harmonic = peaks_square.iter().any(|&f| (f - 1320.0).abs() < 50.0);
    assert!(
        has_third_harmonic,
        "Square wave should have 3rd harmonic at 1320 Hz"
    );

    // Narrow pulse should have more harmonics than square wave
    assert!(
        peaks_narrow.len() >= peaks_square.len(),
        "Narrow pulse should have at least as many harmonics as square wave"
    );
}

/// Test that pulse width affects duty cycle correctly
#[test]
fn test_pulse_duty_cycle() {
    let dsl = r#"
tempo: 1.0
~pulse: pulse 100 0.3
out: ~pulse * 0.5
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    // Render one period at 100 Hz (441 samples)
    let period_samples = (SAMPLE_RATE / 100.0) as usize;
    let samples = graph.render(period_samples);

    // Count samples above 0 (high state)
    let high_count = samples.iter().filter(|&&s| s > 0.0).count();
    let duty_cycle = high_count as f32 / samples.len() as f32;

    println!("Measured duty cycle: {}", duty_cycle);
    println!("Expected duty cycle: 0.3");

    // Should be close to 30%
    assert!(
        (duty_cycle - 0.3).abs() < 0.05,
        "Duty cycle should be close to 0.3, got {}",
        duty_cycle
    );
}

/// LEVEL 3: Musical Integration Test
#[test]
fn test_pulse_musical_example() {
    let dsl = r#"
tempo: 2.0
-- Pulse wave with varying width
~pulse: pulse 220 0.25
out: ~pulse * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();

    let samples = graph.render((SAMPLE_RATE / 2.0) as usize); // 0.5 seconds

    // Write to file
    let filename = "/tmp/test_pulse_musical.wav";
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
        "Pulse tone should be audible (RMS > 0.1), got RMS {}",
        rms
    );

    // Peak should be reasonable
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        peak > 0.2 && peak < 0.5,
        "Pulse tone should have reasonable peak (0.2-0.5), got {}",
        peak
    );
}

/// Test pattern-modulated pulse width
#[test]
fn test_pulse_pattern_width() {
    let dsl = r#"
tempo: 2.0
-- Pattern-controlled pulse width
~width_pattern: "0.1 0.5 0.9"
~pulse: pulse 220 ~width_pattern
out: ~pulse * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let graph = compile_program(statements, SAMPLE_RATE);

    assert!(
        graph.is_ok(),
        "Pulse with pattern-controlled width should compile: {:?}",
        graph.err()
    );
}

/// Test pulse with envelope modulation
#[test]
fn test_pulse_with_envelope() {
    let dsl = r#"
tempo: 2.0
~env: ad 0.01 0.2
~pulse: pulse 440 0.3
out: ~pulse * ~env * 0.4
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples = graph.render((SAMPLE_RATE / 2.0) as usize);

    // Should produce percussive pulse tone
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    assert!(rms.sqrt() > 0.02, "Enveloped pulse should be audible");

    // Peak should be near the start (attack phase)
    let first_quarter: f32 = samples[0..samples.len()/4]
        .iter()
        .map(|s| s * s)
        .sum::<f32>() / (samples.len() / 4) as f32;
    let last_quarter: f32 = samples[samples.len()*3/4..]
        .iter()
        .map(|s| s * s)
        .sum::<f32>() / (samples.len() / 4) as f32;

    assert!(
        first_quarter.sqrt() > last_quarter.sqrt() * 2.0,
        "Enveloped pulse should be louder at start than end"
    );
}

/// Test pulse width modulation (PWM) with LFO
#[test]
fn test_pulse_pwm_with_lfo() {
    let dsl = r#"
tempo: 1.0
-- PWM: pulse width modulated by slow LFO
~lfo: sine 0.5
~width: ~lfo * 0.4 + 0.5
~pulse: pulse 220 ~width
out: ~pulse * 0.2
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE).unwrap();
    let samples = graph.render(SAMPLE_RATE as usize);

    // Should produce audible output with varying timbre
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    assert!(rms.sqrt() > 0.05, "PWM should be audible");

    // Should have reasonable peak
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(peak > 0.1 && peak < 0.4, "PWM should have reasonable peak");
}
