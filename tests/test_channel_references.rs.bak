use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use std::collections::HashMap;

#[test]
fn test_channel_references_in_patterns() {
    println!("\n=== Testing Channel References in Patterns ===");

    // Test that ~ references are preserved in pattern parsing
    let pattern = parse_mini_notation("~bass ~drums ~ ~bass");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let events = pattern.query(&state);

    println!("Pattern: '~bass ~drums ~ ~bass'");
    println!("Events generated:");
    for (i, event) in events.iter().enumerate() {
        println!(
            "  {}: {:.3} -> {:.3}: '{}'",
            i,
            event.part.begin.to_float(),
            event.part.end.to_float(),
            event.value
        );
    }

    // Verify the pattern parsed correctly
    // Note: rests (~) don't generate events, so we should have 3 events, not 4
    assert_eq!(
        events.len(),
        3,
        "Should have 3 events (rest doesn't generate event)"
    );
    assert_eq!(events[0].value, "~bass", "First should be ~bass");
    assert_eq!(events[1].value, "~drums", "Second should be ~drums");
    assert_eq!(events[2].value, "~bass", "Third should be ~bass");
}

#[test]
fn test_alternating_channel_references() {
    println!("\n=== Testing Alternating Channel References ===");

    // Test alternation with channel references
    let pattern = parse_mini_notation("<~sine ~saw ~square>");

    for cycle in 0..3 {
        let state = State {
            span: TimeSpan::new(
                Fraction::new(cycle as i64, 1),
                Fraction::new((cycle + 1) as i64, 1),
            ),
            controls: HashMap::new(),
        };

        let events = pattern.query(&state);

        assert_eq!(events.len(), 1, "Should have 1 event per cycle");

        let expected = match cycle % 3 {
            0 => "~sine",
            1 => "~saw",
            2 => "~square",
            _ => unreachable!(),
        };

        assert_eq!(events[0].value, expected);
        println!("  Cycle {}: '{}' ✓", cycle, events[0].value);
    }
}

#[test]
fn test_channel_ref_with_euclidean() {
    println!("\n=== Testing Channel References with Euclidean Rhythms ===");

    // Test that channel refs work in euclidean patterns
    let pattern = parse_mini_notation("~kick(3,8)");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let events = pattern.query(&state);

    // Should have 3 events (3 pulses in 8 steps)
    assert_eq!(events.len(), 3, "Should have 3 events");

    // All should be ~kick
    for event in &events {
        assert_eq!(event.value, "~kick");
    }

    println!(
        "  Generated {} ~kick events in euclidean pattern ✓",
        events.len()
    );
}

/// This test documents what WOULD need to be implemented
/// for full synth triggering from patterns
#[test]
fn test_synth_triggering_requirements() {
    println!("\n=== Synth Triggering Requirements ===");
    println!("To properly trigger synths from patterns, we need:");
    println!("1. DSP executor to recognize ~channel references");
    println!("2. Envelope generation for triggered synths");
    println!("3. Voice allocation for polyphony");
    println!("4. Parameter control from patterns (e.g., ~bass(440) for frequency)");
    println!("");
    println!("Current status:");
    println!("✓ Patterns can parse ~channel references");
    println!("✓ Glicol can define synth chains on channels");
    println!("✗ DSP executor doesn't trigger synths from pattern events");
    println!("✗ No envelope system for triggered synths");

    // This would be the ideal syntax:
    let ideal_code = r#"
        // Define synths
        ~kick: sin 60 >> mul env(0.01, 0.1, 0, 0.1)
        ~snare: noise >> hpf 2000 >> mul env(0.01, 0.05, 0, 0.05)
        
        // Trigger them in patterns
        ~drums: s "~kick ~snare ~kick ~kick"
        
        // With parameters
        ~melody: s "~sine(440) ~sine(550) ~sine(660)"
        
        // Mix everything
        o: ~drums >> mul 0.8
    "#;

    println!("\nIdeal syntax example:");
    for line in ideal_code.lines().filter(|l| !l.trim().is_empty()) {
        println!("{}", line);
    }
}
