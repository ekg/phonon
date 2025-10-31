use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use std::collections::HashMap;

#[test]
fn test_alternation_parsing() {
    let pattern = parse_mini_notation("<bd sn>");

    println!("\n=== Testing Alternation Parsing ===");
    println!("Pattern: <bd sn>");

    // Query cycles 0-3 to see if alternation works
    for cycle in 0..4 {
        let state = State {
            span: TimeSpan::new(Fraction::new(cycle, 1), Fraction::new(cycle + 1, 1)),
            controls: HashMap::new(),
        };

        let events = pattern.query(&state);
        println!("\nCycle {}: {} events", cycle, events.len());
        for (i, event) in events.iter().enumerate() {
            println!(
                "  Event {}: value='{}' at {:.3}-{:.3}",
                i,
                event.value,
                event.part.begin.to_float(),
                event.part.end.to_float()
            );
        }
    }
}

#[test]
fn test_simple_pattern_parsing() {
    let pattern = parse_mini_notation("bd sn cp hh");

    println!("\n=== Testing Simple Concatenation Parsing ===");
    println!("Pattern: bd sn cp hh");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let events = pattern.query(&state);
    println!("\nCycle 0: {} events", events.len());
    for (i, event) in events.iter().enumerate() {
        println!(
            "  Event {}: value='{}' at {:.3}-{:.3}",
            i,
            event.value,
            event.part.begin.to_float(),
            event.part.end.to_float()
        );
    }
}

#[test]
fn test_subdivision_parsing() {
    let pattern = parse_mini_notation("bd*16");

    println!("\n=== Testing Subdivision Parsing ===");
    println!("Pattern: bd*16");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let events = pattern.query(&state);
    println!("\nCycle 0: {} events", events.len());
    for (i, event) in events.iter().take(5).enumerate() {
        println!(
            "  Event {}: value='{}' at {:.3}-{:.3}",
            i,
            event.value,
            event.part.begin.to_float(),
            event.part.end.to_float()
        );
    }
    if events.len() > 5 {
        println!("  ... {} more events", events.len() - 5);
    }
}
