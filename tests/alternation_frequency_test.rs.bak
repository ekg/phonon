use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use std::collections::HashMap;

/// Detect the dominant frequency in a signal window using zero-crossing
fn detect_frequency_simple(samples: &[f32], sample_rate: f32) -> f32 {
    // Count zero crossings
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

#[test]
fn test_alternation_with_frequencies() {
    // Create pattern that alternates between 440Hz and 880Hz
    let pattern_str = "<sine(440) sine(880)>";
    let pattern = parse_mini_notation(pattern_str);

    let _sample_rate = 44100.0;
    let _cycle_duration = 1.0; // 1 second per cycle
    let num_cycles = 4;

    println!("\nTesting alternation pattern: {}", pattern_str);
    println!("Expected: 440Hz, 880Hz, 440Hz, 880Hz\n");

    for cycle in 0..num_cycles {
        let state = State {
            span: TimeSpan::new(
                Fraction::new(cycle as i64, 1),
                Fraction::new((cycle + 1) as i64, 1),
            ),
            controls: HashMap::new(),
        };

        let events = pattern.query(&state);

        println!("Cycle {}: {} events", cycle, events.len());
        for event in &events {
            println!(
                "  {:.3} -> {:.3}: {}",
                event.part.begin.to_float(),
                event.part.end.to_float(),
                event.value
            );
        }

        // Check that we get the expected alternation
        assert_eq!(events.len(), 1, "Should have exactly 1 event per cycle");

        let expected = if cycle % 2 == 0 {
            "sine(440)"
        } else {
            "sine(880)"
        };

        assert_eq!(
            events[0].value, expected,
            "Cycle {} should have {}",
            cycle, expected
        );
    }
}

#[test]
fn test_alternation_in_euclidean_patterns() {
    // Test that alternation works in euclidean patterns
    // Should alternate between 3 and 4 pulses per cycle
    // Note: "sine" is treated as a sample name, not a function call
    let pattern_str = "bd(<3 4>,8)";
    let pattern = parse_mini_notation(pattern_str);

    println!("\nTesting nested alternation: {}", pattern_str);
    println!("Expected pulses: 3, 4, 3, 4\n");

    for cycle in 0..4 {
        let state = State {
            span: TimeSpan::new(
                Fraction::new(cycle as i64, 1),
                Fraction::new((cycle + 1) as i64, 1),
            ),
            controls: HashMap::new(),
        };

        let events = pattern.query(&state);

        // Filter out rest events
        let non_rest_events: Vec<_> = events.iter().filter(|e| e.value != "~").collect();

        println!(
            "Cycle {}: {} events (excluding rests)",
            cycle,
            non_rest_events.len()
        );

        // Check pulse count alternation (3, 4, 3, 4)
        let expected_pulses = if cycle % 2 == 0 { 3 } else { 4 };

        assert_eq!(
            non_rest_events.len(),
            expected_pulses,
            "Cycle {} should have {} pulses",
            cycle,
            expected_pulses
        );

        // All non-rest events should be "bd"
        for event in &non_rest_events {
            assert_eq!(event.value, "bd", "All events should be 'bd'");
        }
    }
}

#[test]
fn test_simple_pattern_rendering() {
    // Test that we can actually render audio from alternating patterns
    use phonon::glicol_parser::parse_glicol;
    use phonon::simple_dsp_executor::SimpleDspExecutor;

    let sample_rate = 44100.0;
    let mut executor = SimpleDspExecutor::new(sample_rate);

    // Create a simple sine wave pattern
    let code = r#"o: sin 440 >> mul 0.5"#;

    let env = parse_glicol(code).expect("Failed to parse");
    let samples = executor.render(&env, 1.0).expect("Failed to render");

    // Verify we got audio
    assert!(!samples.data.is_empty(), "Should generate audio samples");

    // Check that the signal has the expected frequency
    let window_size = 4096;
    if samples.data.len() >= window_size {
        let freq = detect_frequency_simple(&samples.data[0..window_size], sample_rate);
        println!("Detected frequency: {:.1} Hz", freq);
        assert!(
            (freq - 440.0).abs() < 50.0,
            "Should detect ~440Hz, got {}",
            freq
        );
    }
}
