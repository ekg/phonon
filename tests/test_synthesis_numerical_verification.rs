/// Numerical verification tests for synthesis accuracy
///
/// These tests verify that synthesis generates the EXACT expected waveforms
/// by comparing sample-by-sample with mathematically generated reference signals.
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use std::f32::consts::PI;

/// Generate expected sine wave for comparison
fn generate_expected_sine(frequency: f32, sample_rate: f32, num_samples: usize) -> Vec<f32> {
    let mut samples = Vec::with_capacity(num_samples);
    for i in 0..num_samples {
        let phase = (i as f32) / sample_rate * frequency;
        let sample = (2.0 * PI * phase).sin();
        samples.push(sample);
    }
    samples
}

/// Calculate root mean square error between two signals
fn calculate_rmse(signal_a: &[f32], signal_b: &[f32]) -> f32 {
    assert_eq!(
        signal_a.len(),
        signal_b.len(),
        "Signals must be same length"
    );

    let sum_squared_error: f32 = signal_a
        .iter()
        .zip(signal_b.iter())
        .map(|(a, b)| {
            let diff = a - b;
            diff * diff
        })
        .sum();

    (sum_squared_error / signal_a.len() as f32).sqrt()
}

/// Calculate correlation coefficient between two signals
fn calculate_correlation(signal_a: &[f32], signal_b: &[f32]) -> f32 {
    assert_eq!(
        signal_a.len(),
        signal_b.len(),
        "Signals must be same length"
    );

    let mean_a: f32 = signal_a.iter().sum::<f32>() / signal_a.len() as f32;
    let mean_b: f32 = signal_b.iter().sum::<f32>() / signal_b.len() as f32;

    let mut numerator = 0.0_f32;
    let mut sum_sq_a = 0.0_f32;
    let mut sum_sq_b = 0.0_f32;

    for (a, b) in signal_a.iter().zip(signal_b.iter()) {
        let diff_a = a - mean_a;
        let diff_b = b - mean_b;
        numerator += diff_a * diff_b;
        sum_sq_a += diff_a * diff_a;
        sum_sq_b += diff_b * diff_b;
    }

    numerator / (sum_sq_a * sum_sq_b).sqrt()
}

#[test]
fn test_sine_440_numerical_accuracy() {
    let sample_rate = 44100.0;
    let frequency = 440.0;

    // Generate reference sine wave
    let num_samples = 44100; // 1 second
    let expected = generate_expected_sine(frequency, sample_rate, num_samples);

    // Generate using Phonon synthesis
    let code = "out $ sine 440";
    let (_, statements) = parse_program(code).expect("Parse failed");
    let mut graph = compile_program(statements, sample_rate, None).expect("Compilation failed");
    let actual = graph.render(num_samples);

    // Calculate error metrics
    let rmse = calculate_rmse(&expected, &actual);
    let correlation = calculate_correlation(&expected, &actual);

    // RMSE should be very small (< 0.01 for nearly perfect match)
    assert!(
        rmse < 0.01,
        "RMSE too high: {} (expected < 0.01). Synthesis not matching expected waveform!",
        rmse
    );

    // Correlation should be very close to 1.0 (> 0.99 for nearly perfect match)
    assert!(
        correlation > 0.99,
        "Correlation too low: {} (expected > 0.99). Waveform shape doesn't match!",
        correlation
    );

    println!("✓ Sine 440Hz numerical verification:");
    println!("  RMSE: {:.6}", rmse);
    println!("  Correlation: {:.6}", correlation);
}

#[test]
fn test_sine_phase_continuity_numerical() {
    let sample_rate = 44100.0;
    let frequency = 440.0;

    // Render in two separate buffers
    let code = "out $ sine 440";
    let (_, statements) = parse_program(code).expect("Parse failed");
    let mut graph = compile_program(statements, sample_rate, None).expect("Compilation failed");

    let buffer_size = 512;
    let buffer1 = graph.render(buffer_size);
    let buffer2 = graph.render(buffer_size);

    // Generate expected continuous sine wave
    let expected = generate_expected_sine(frequency, sample_rate, buffer_size * 2);

    // Combine actual buffers
    let mut actual = Vec::with_capacity(buffer_size * 2);
    actual.extend_from_slice(&buffer1);
    actual.extend_from_slice(&buffer2);

    // Check boundary continuity
    let last_sample_buf1 = buffer1[buffer_size - 1];
    let first_sample_buf2 = buffer2[0];
    let boundary_diff = (first_sample_buf2 - last_sample_buf1).abs();

    // Boundary should be smooth (phase continuous)
    assert!(
        boundary_diff < 0.1,
        "Buffer boundary discontinuity: {:.6} (last={:.6}, first={:.6})",
        boundary_diff,
        last_sample_buf1,
        first_sample_buf2
    );

    // Overall waveform should match expected
    let rmse = calculate_rmse(&expected, &actual);
    let correlation = calculate_correlation(&expected, &actual);

    assert!(
        rmse < 0.01,
        "RMSE too high across buffers: {} (phase not continuous!)",
        rmse
    );

    assert!(
        correlation > 0.99,
        "Correlation too low across buffers: {} (phase discontinuity!)",
        correlation
    );

    println!("✓ Phase continuity numerical verification:");
    println!("  Boundary diff: {:.6}", boundary_diff);
    println!("  RMSE: {:.6}", rmse);
    println!("  Correlation: {:.6}", correlation);
}

#[test]
fn test_bus_triggered_synthesis_numerical() {
    let sample_rate = 44100.0;

    // Simple bus-triggered synthesis
    let code = r#"
tempo: 0.5
~synth $ sine 440
~trig $ s "~synth"
out $ ~trig
"#;

    let (_, statements) = parse_program(code).expect("Parse failed");
    let mut graph = compile_program(statements, sample_rate, None).expect("Compilation failed");

    // Render 2 seconds (4 cycles at 2 cps)
    let buffer_size = 128;
    let num_buffers = (sample_rate * 2.0) as usize / buffer_size;

    let mut full_audio = Vec::new();
    for _ in 0..num_buffers {
        let buffer = graph.render(buffer_size);
        full_audio.extend_from_slice(&buffer);
    }

    // Verify audio was generated
    let rms: f32 = full_audio.iter().map(|s| s * s).sum::<f32>() / full_audio.len() as f32;
    let rms = rms.sqrt();

    assert!(rms > 0.01, "No audio generated (RMS = {})", rms);

    // Check for discontinuities at buffer boundaries
    let mut max_discontinuity = 0.0_f32;
    let mut max_location = 0;
    for i in (buffer_size..full_audio.len()).step_by(buffer_size) {
        if i > 0 && i < full_audio.len() {
            let diff = (full_audio[i] - full_audio[i - 1]).abs();
            if diff > max_discontinuity {
                max_discontinuity = diff;
                max_location = i;
            }
        }
    }

    // Buffer boundaries should be smooth (no clicks)
    assert!(
        max_discontinuity < 0.1,
        "Discontinuity at sample {}: {} (indicates clicking)",
        max_location,
        max_discontinuity
    );

    println!("✓ Bus-triggered synthesis numerical verification:");
    println!("  RMS: {:.6}", rms);
    println!(
        "  Max discontinuity: {:.6} at sample {}",
        max_discontinuity, max_location
    );
}

#[test]
fn test_static_vs_live_synthesis() {
    let sample_rate = 44100.0;
    let _frequency = 440.0;

    // STATIC: Direct sine oscillator (no pattern triggering)
    let code_static = "out $ sine 440";
    let (_, statements_static) = parse_program(code_static).expect("Parse failed");
    let mut graph_static =
        compile_program(statements_static, sample_rate, None).expect("Compilation failed");
    let static_audio = graph_static.render(44100); // 1 second

    // LIVE: Bus-triggered continuous synthesis
    let code_live = r#"
tempo: 10.0
~synth $ sine 440
~trig $ s "~synth"
out $ ~trig
"#;
    let (_, statements_live) = parse_program(code_live).expect("Parse failed");
    let mut graph_live =
        compile_program(statements_live, sample_rate, None).expect("Compilation failed");

    // Render in multiple buffers to test live mode
    let buffer_size = 512;
    let num_buffers = 44100 / buffer_size;
    let mut live_audio = Vec::new();
    for _ in 0..num_buffers {
        let buffer = graph_live.render(buffer_size);
        live_audio.extend_from_slice(&buffer);
    }

    // Trim to same length
    let min_len = static_audio.len().min(live_audio.len());
    let static_audio = &static_audio[0..min_len];
    let live_audio = &live_audio[0..min_len];

    // Both should have similar RMS (accounting for envelope)
    let rms_static: f32 =
        static_audio.iter().map(|s| s * s).sum::<f32>() / static_audio.len() as f32;
    let rms_static = rms_static.sqrt();

    let rms_live: f32 = live_audio.iter().map(|s| s * s).sum::<f32>() / live_audio.len() as f32;
    let rms_live = rms_live.sqrt();

    // Live synthesis should generate audio
    assert!(
        rms_live > 0.01,
        "Live synthesis generated no audio (RMS = {})",
        rms_live
    );

    // Static should be louder (no envelope), but both should have significant energy
    assert!(
        rms_static > 0.1,
        "Static synthesis too quiet (RMS = {})",
        rms_static
    );

    println!("✓ Static vs Live synthesis comparison:");
    println!("  Static RMS: {:.6}", rms_static);
    println!("  Live RMS: {:.6}", rms_live);
    println!("  Ratio: {:.2}", rms_static / rms_live);
}

#[test]
fn test_multiple_frequencies_numerical() {
    let sample_rate = 44100.0;

    // Test multiple frequencies for accuracy
    let frequencies = vec![110.0, 220.0, 440.0, 880.0];

    for frequency in frequencies {
        let code = format!("out $ sine {}", frequency);
        let (_, statements) = parse_program(&code).expect("Parse failed");
        let mut graph = compile_program(statements, sample_rate, None).expect("Compilation failed");

        let num_samples = 4410; // 0.1 second
        let expected = generate_expected_sine(frequency, sample_rate, num_samples);
        let actual = graph.render(num_samples);

        let rmse = calculate_rmse(&expected, &actual);
        let correlation = calculate_correlation(&expected, &actual);

        assert!(
            rmse < 0.01,
            "{}Hz: RMSE too high: {} (expected < 0.01)",
            frequency,
            rmse
        );

        assert!(
            correlation > 0.99,
            "{}Hz: Correlation too low: {} (expected > 0.99)",
            frequency,
            correlation
        );

        println!(
            "✓ {}Hz: RMSE={:.6}, Correlation={:.6}",
            frequency, rmse, correlation
        );
    }
}
