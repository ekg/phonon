use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use std::collections::HashMap;

#[test]
fn debug_degrade_pattern_query() {
    // Test that degrade() actually filters events when the pattern is queried
    let pattern_str = "bd bd bd bd";
    let base_pattern = parse_mini_notation(pattern_str);
    let degraded_pattern = base_pattern.clone().degrade();

    // Query both patterns over 2 cycles
    let duration_cycles = 2.0;
    let state = State {
        span: TimeSpan::new(
            Fraction::from_float(0.0),
            Fraction::from_float(duration_cycles),
        ),
        controls: HashMap::new(),
    };

    let normal_events = base_pattern.query(&state);
    let degraded_events = degraded_pattern.query(&state);

    println!("\n=== PATTERN QUERY DEBUG ===");
    println!("Normal pattern: {} events", normal_events.len());
    for (i, event) in normal_events.iter().enumerate() {
        println!(
            "  Event {}: time={:.3}, value={:?}",
            i,
            event.part.begin.to_float(),
            event.value
        );
    }

    println!("\nDegraded pattern: {} events", degraded_events.len());
    for (i, event) in degraded_events.iter().enumerate() {
        println!(
            "  Event {}: time={:.3}, value={:?}",
            i,
            event.part.begin.to_float(),
            event.value
        );
    }

    // The degraded pattern should have fewer events (approximately 50%)
    println!(
        "\nEvent count ratio (degraded/normal): {:.2}",
        degraded_events.len() as f32 / normal_events.len().max(1) as f32
    );

    assert!(
        degraded_events.len() < normal_events.len(),
        "Degraded pattern should have fewer events than normal pattern"
    );
}

#[test]
fn debug_degrade_multiple_queries() {
    // Test that degrade produces different results on each cycle
    let pattern_str = "bd bd bd bd";
    let base_pattern = parse_mini_notation(pattern_str);
    let degraded_pattern = base_pattern.degrade();

    println!("\n=== MULTIPLE QUERY DEBUG ===");

    for cycle in 0..5 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let events = degraded_pattern.query(&state);
        println!("Cycle {}: {} events", cycle, events.len());
        for event in &events {
            println!(
                "  time={:.3}, value={}",
                event.part.begin.to_float(),
                event.value
            );
        }
    }
}
