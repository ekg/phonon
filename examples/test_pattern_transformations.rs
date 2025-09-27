use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use std::collections::HashMap;

fn main() {
    println!("=== Testing Pattern Transformations ===\n");

    // Test 1: Basic pattern
    let pattern = parse_mini_notation("bd sn hh cp");
    println!("Base pattern: \"bd sn hh cp\"");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let events = pattern.query(&state);
    println!("Events in cycle 0-1:");
    for event in &events {
        println!(
            "  {} at {:.3}-{:.3}",
            event.value,
            event.part.begin.to_float(),
            event.part.end.to_float()
        );
    }

    // Test 2: Apply fast transformation
    println!("\nAfter .fast(2):");
    let fast_pattern = pattern.clone().fast(2.0);
    let fast_events = fast_pattern.query(&state);
    for event in &fast_events {
        println!(
            "  {} at {:.3}-{:.3}",
            event.value,
            event.part.begin.to_float(),
            event.part.end.to_float()
        );
    }

    // Test 3: Apply rev transformation
    println!("\nAfter .rev():");
    let rev_pattern = pattern.clone().rev();
    let rev_events = rev_pattern.query(&state);
    for event in &rev_events {
        println!(
            "  {} at {:.3}-{:.3}",
            event.value,
            event.part.begin.to_float(),
            event.part.end.to_float()
        );
    }

    // Test 4: Chain transformations
    println!("\nAfter .fast(2).rev():");
    let chained = pattern.clone().fast(2.0).rev();
    let chained_events = chained.query(&state);
    for event in &chained_events {
        println!(
            "  {} at {:.3}-{:.3}",
            event.value,
            event.part.begin.to_float(),
            event.part.end.to_float()
        );
    }

    // Test 5: Every transformation
    println!("\nAfter .every(2, |p| p.fast(2.0)):");
    let every_pattern = pattern.clone().every(2, |p| p.fast(2.0));

    // Query two cycles to see the effect
    for cycle in 0..2 {
        let cycle_state = State {
            span: TimeSpan::new(Fraction::new(cycle, 1), Fraction::new(cycle + 1, 1)),
            controls: HashMap::new(),
        };
        let cycle_events = every_pattern.query(&cycle_state);
        println!("  Cycle {}:", cycle);
        for event in &cycle_events {
            println!(
                "    {} at {:.3}-{:.3}",
                event.value,
                event.part.begin.to_float(),
                event.part.end.to_float()
            );
        }
    }

    // Test 6: Jux (creates stereo by applying transformation to one channel)
    println!("\nAfter .jux(|p| p.rev()):");
    let jux_pattern = pattern.clone().jux(|p| p.rev());
    let jux_events = jux_pattern.query(&state);
    for event in &jux_events {
        println!(
            "  ({}, {}) at {:.3}-{:.3}",
            event.value.0,
            event.value.1,
            event.part.begin.to_float(),
            event.part.end.to_float()
        );
    }

    println!("\n=== Summary ===");
    println!("Pattern transformations work by method chaining:");
    println!("  pattern.fast(2)         - Speed up by factor of 2");
    println!("  pattern.slow(2)         - Slow down by factor of 2");
    println!("  pattern.rev()           - Reverse pattern");
    println!("  pattern.every(n, f)     - Apply f every n cycles");
    println!("  pattern.jux(f)          - Apply f to right channel");
    println!("  pattern.degrade()       - Randomly drop events");
    println!("  pattern.stutter(n)      - Repeat each event n times");
    println!("  pattern.chunk(n, f)     - Apply f to chunks");
    println!("\nTransformations can be chained:");
    println!("  pattern.fast(2).rev().every(3, |p| p.slow(2))");
}
