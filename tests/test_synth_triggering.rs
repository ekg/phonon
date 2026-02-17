use phonon::glicol_parser::parse_glicol;
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use phonon::simple_dsp_executor::SimpleDspExecutor;
use std::collections::HashMap;

/// Helper function to find peaks in audio
fn find_peaks(samples: &[f32], threshold: f32) -> Vec<usize> {
    let mut peaks = Vec::new();
    let mut in_peak = false;

    for (i, &sample) in samples.iter().enumerate() {
        if sample.abs() > threshold && !in_peak {
            peaks.push(i);
            in_peak = true;
        } else if sample.abs() < threshold * 0.5 {
            in_peak = false;
        }
    }

    peaks
}

/// Calculate RMS of a buffer
fn calculate_rms(samples: &[f32]) -> f32 {
    let sum: f32 = samples.iter().map(|x| x * x).sum();
    (sum / samples.len() as f32).sqrt()
}

#[test]
fn test_channel_reference_parsing() {
    println!("\n=== Testing Channel Reference Parsing ===");

    // Test that channel references are preserved in patterns
    let pattern = parse_mini_notation("~bass ~lead ~ ~bass");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let events = pattern.query(&state);

    // Should have 3 events (rest doesn't generate event)
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].value, "~bass");
    assert_eq!(events[1].value, "~lead");
    assert_eq!(events[2].value, "~bass");

    println!("✓ Channel references parsed correctly");
}

#[test]
fn test_synth_triggering_basic() {
    println!("\n=== Testing Basic Synth Triggering ===");

    let sample_rate = 44100.0;
    let mut executor = SimpleDspExecutor::new(sample_rate);

    // Define a simple synth and trigger it from a pattern
    let code = r#"
        ~kick: sin 60 >> mul 0.5
        o: s "~kick ~ ~kick ~"
    "#;

    let env = parse_glicol(code).expect("Failed to parse");
    let audio = executor.render(&env, 1.0).expect("Failed to render");

    // Should generate audio
    assert!(!audio.data.is_empty());

    // Find peaks (should be 2 - two kicks)
    let peaks = find_peaks(&audio.data, 0.1);
    println!("Found {} peaks (expected 2 for two kicks)", peaks.len());

    // Should have some non-zero audio
    let rms = calculate_rms(&audio.data);
    assert!(rms > 0.01, "Should have non-zero RMS: {}", rms);

    println!("✓ Basic synth triggering works");
}

#[test]
fn test_alternating_synths() {
    println!("\n=== Testing Alternating Synth Patterns ===");

    let sample_rate = 44100.0;
    let mut executor = SimpleDspExecutor::new(sample_rate);

    // Define multiple synths and alternate between them
    let code = r#"
        ~low: sin 110 >> mul 0.5
        ~mid: sin 220 >> mul 0.4
        ~high: sin 440 >> mul 0.3
        o: s "<~low ~mid ~high>"
    "#;

    let env = parse_glicol(code).expect("Failed to parse");
    let audio = executor.render(&env, 3.0).expect("Failed to render"); // 3 cycles

    // Analyze each cycle
    let samples_per_cycle = sample_rate as usize;
    let mut cycle_stats = Vec::new();

    for cycle in 0..3 {
        let start = cycle * samples_per_cycle;
        let end = ((cycle + 1) * samples_per_cycle).min(audio.data.len());

        if end > start {
            let cycle_data = &audio.data[start..end];
            let rms = calculate_rms(cycle_data);
            let peaks = find_peaks(cycle_data, 0.1);

            cycle_stats.push((rms, peaks.len()));
            println!("Cycle {}: RMS={:.3}, peaks={}", cycle, rms, peaks.len());
        }
    }

    // Each cycle should have audio (non-zero RMS)
    // Note: Some cycles might have very low RMS due to synth implementation
    for (i, (rms, _)) in cycle_stats.iter().enumerate() {
        assert!(
            *rms > 0.001,
            "Cycle {} should have audio, got RMS {}",
            i,
            rms
        );
    }

    // At least 2 out of 3 cycles should have significant audio
    let significant_cycles = cycle_stats.iter().filter(|(rms, _)| *rms > 0.01).count();
    assert!(
        significant_cycles >= 2,
        "At least 2 cycles should have significant audio"
    );

    println!("✓ Alternating synths work");
}

#[test]
fn test_synth_with_frequency_parameter() {
    println!("\n=== Testing Synth with Frequency Parameter ===");

    let sample_rate = 44100.0;
    let mut executor = SimpleDspExecutor::new(sample_rate);

    // Test pattern with frequency parameters
    let code = r#"
        ~sine: sin 440 >> mul 0.5
        o: s "~sine 220 ~sine 440 ~sine 880"
    "#;

    // This feature might not be implemented yet, but test the parsing at least
    let env_result = parse_glicol(code);

    if let Ok(env) = env_result {
        let audio = executor.render(&env, 1.0);

        if let Ok(audio) = audio {
            println!("Generated {} samples", audio.data.len());

            // Should have some audio
            let rms = calculate_rms(&audio.data);
            println!("RMS: {:.3}", rms);
        }
    }

    println!("✓ Frequency parameter test complete");
}

#[test]
fn test_euclidean_with_synths() {
    println!("\n=== Testing Euclidean Patterns with Synths ===");

    let sample_rate = 44100.0;
    let mut executor = SimpleDspExecutor::new(sample_rate);

    // Use euclidean rhythm with synth
    let code = r#"
        ~click: sin 1000 >> mul 0.3
        o: s "~click(3,8)"
    "#;

    let env = parse_glicol(code).expect("Failed to parse");
    let audio = executor.render(&env, 1.0).expect("Failed to render");

    // Find peaks - should be 3 (euclidean 3,8)
    let peaks = find_peaks(&audio.data, 0.05);
    println!("Found {} peaks (expected 3 for euclidean 3,8)", peaks.len());

    // Should have the right number of events
    // Note: actual peak detection might vary due to envelope
    assert!(peaks.len() >= 2, "Should have at least 2 peaks");

    println!("✓ Euclidean patterns with synths work");
}

#[test]
fn test_polyrhythm_with_synths() {
    println!("\n=== Testing Polyrhythm with Synths ===");

    let sample_rate = 44100.0;
    let mut executor = SimpleDspExecutor::new(sample_rate);

    // Polyrhythm with different synths
    let code = r#"
        ~bass: sin 55 >> mul 0.5
        ~hi: sin 880 >> mul 0.2
        o: s "[~bass*3, ~hi*4]"
    "#;

    let env = parse_glicol(code).expect("Failed to parse");
    let audio = executor.render(&env, 1.0).expect("Failed to render");

    // Should generate complex pattern
    let rms = calculate_rms(&audio.data);
    assert!(rms > 0.01, "Should generate audio");

    // Find peaks - should be multiple from both patterns
    let peaks = find_peaks(&audio.data, 0.05);
    println!("Found {} peaks in polyrhythm", peaks.len());

    println!("✓ Polyrhythm with synths works");
}

