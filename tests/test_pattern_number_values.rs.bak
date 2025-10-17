use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use std::collections::HashMap;

#[test]
fn test_number_pattern_parsing() {
    let pattern = parse_mini_notation("110 220 440");

    println!("\nTesting pattern: '110 220 440'");
    println!();

    // Query at different cycle positions in the first cycle
    for i in 0..12 {
        let pos = i as f64 / 12.0;
        let state = State {
            span: TimeSpan::new(Fraction::from_float(pos), Fraction::from_float(pos + 0.01)),
            controls: HashMap::new(),
        };

        let events = pattern.query(&state);
        if !events.is_empty() {
            for event in &events {
                let start = event
                    .whole
                    .as_ref()
                    .map(|w| w.begin.to_float())
                    .unwrap_or(0.0);
                println!(
                    "  Query pos {:.3}: Event value='{}', start={:.3}",
                    pos, event.value, start
                );
            }
        }
    }

    // Also test at the exact trigger points
    println!("\nExact trigger points:");
    for (i, expected_val) in [(0.0, "110"), (0.333, "220"), (0.666, "440")].iter() {
        let state = State {
            span: TimeSpan::new(Fraction::from_float(*i), Fraction::from_float(*i + 0.001)),
            controls: HashMap::new(),
        };

        let events = pattern.query(&state);
        println!("  At {:.3}: {} events", i, events.len());
        for event in &events {
            println!("    value='{}' (expected '{}')", event.value, expected_val);
        }
    }
}
