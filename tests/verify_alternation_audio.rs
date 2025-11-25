use phonon::glicol_parser::parse_glicol;
use phonon::simple_dsp_executor::SimpleDspExecutor;
use std::fs::File;
use std::io::Write;

/// Count zero crossings in a window to estimate frequency
fn estimate_frequency(samples: &[f32], sample_rate: f32) -> f32 {
    let mut zero_crossings = 0;
    let mut last_sign = samples[0] >= 0.0;

    for &sample in samples.iter().skip(1) {
        let current_sign = sample >= 0.0;
        if current_sign != last_sign {
            zero_crossings += 1;
            last_sign = current_sign;
        }
    }

    // Frequency = (zero crossings / 2) / duration
    let duration = samples.len() as f32 / sample_rate;
    (zero_crossings as f32 / 2.0) / duration
}

/// Find peaks in the signal to detect events
fn find_peaks(samples: &[f32], threshold: f32) -> Vec<usize> {
    let mut peaks = Vec::new();
    let mut in_peak = false;
    let mut peak_start = 0;

    for (i, &sample) in samples.iter().enumerate() {
        let magnitude = sample.abs();
        if magnitude > threshold && !in_peak {
            in_peak = true;
            peak_start = i;
        } else if magnitude < threshold * 0.5 && in_peak {
            in_peak = false;
            peaks.push(peak_start);
        }
    }

    peaks
}

#[test]
#[ignore] // TODO: Fix for new implementation
fn test_alternation_generates_different_cycles() {
    let sample_rate = 44100.0;
    let mut executor = SimpleDspExecutor::new(sample_rate);

    // Generate 4 cycles with alternating pattern
    // Each cycle should be 1 second
    let code = r#"o: s "<bd sn>""#;

    let env = parse_glicol(code).expect("Failed to parse");
    let audio = executor.render(&env, 4.0).expect("Failed to render");

    // Save for inspection
    let mut file = File::create("/tmp/alternation_test.raw").unwrap();
    for sample in &audio.data {
        file.write_all(&sample.to_le_bytes()).unwrap();
    }

    println!("\nGenerated {} samples for 4 cycles", audio.data.len());
    println!("Peak amplitude: {:.3}", audio.peak());
    println!("RMS: {:.3}", audio.rms());

    // Analyze each cycle (1 second each)
    let samples_per_cycle = sample_rate as usize;

    for cycle in 0..4 {
        let start = cycle * samples_per_cycle;
        let end = (cycle + 1) * samples_per_cycle;

        if end <= audio.data.len() {
            let cycle_samples = &audio.data[start..end];

            // Find peaks in this cycle
            let peaks = find_peaks(cycle_samples, 0.1);

            // Calculate RMS for this cycle
            let cycle_rms: f32 = (cycle_samples.iter().map(|x| x * x).sum::<f32>()
                / cycle_samples.len() as f32)
                .sqrt();

            println!(
                "\nCycle {}: {} peaks detected, RMS: {:.3}",
                cycle,
                peaks.len(),
                cycle_rms
            );

            // For debugging, print first few samples to see if there's any signal
            if cycle_samples.len() > 100 {
                let first_100_max = cycle_samples[..100]
                    .iter()
                    .map(|x| x.abs())
                    .fold(0.0f32, |a, b| a.max(b));
                println!("  First 100 samples max: {:.6}", first_100_max);
            }
        }
    }

    // We expect to see different patterns in alternating cycles
    // This is a basic check - in reality we'd want to compare spectral content
    assert!(audio.data.len() > 0, "Should generate audio");
    assert!(audio.peak() > 0.01, "Should have non-zero amplitude");
}

#[test]
#[ignore] // TODO: Fix for new implementation
fn test_alternation_with_simple_tones() {
    let sample_rate = 44100.0;
    let _executor = SimpleDspExecutor::new(sample_rate);

    // Use simple sine waves with different frequencies
    // 440Hz should alternate with 880Hz each cycle
    let code = r#"o: sine(<440 880>) >> mul(0.5)"#;

    // This test won't work until we implement proper tone generation
    // For now just check it parses
    let env = parse_glicol(code);
    assert!(env.is_ok(), "Should parse alternation in sine frequency");
}

#[test]
#[ignore] // TODO: Fix for new implementation
fn test_verify_sample_pattern_timing() {
    use phonon::mini_notation_v3::parse_mini_notation;
    use phonon::pattern::{Fraction, State, TimeSpan};
    use std::collections::HashMap;

    // First verify the pattern is parsing correctly
    let pattern = parse_mini_notation("<bd sn>");

    println!("\nVerifying pattern timing for '<bd sn>':");

    for cycle in 0..4 {
        let state = State {
            span: TimeSpan::new(
                Fraction::new(cycle as i64, 1),
                Fraction::new((cycle + 1) as i64, 1),
            ),
            controls: HashMap::new(),
        };

        let events = pattern.query(&state);

        assert_eq!(
            events.len(),
            1,
            "Cycle {} should have exactly 1 event",
            cycle
        );

        let expected = if cycle % 2 == 0 { "bd" } else { "sn" };
        assert_eq!(
            events[0].value, expected,
            "Cycle {} should be '{}'",
            cycle, expected
        );

        println!("  Cycle {}: '{}' ✓", cycle, events[0].value);
    }
}
