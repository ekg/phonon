use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{State, TimeSpan, Fraction};
use std::collections::HashMap;

fn main() {
    // Test different patterns
    let patterns = vec![
        ("bd", "single event"),
        ("bd sn", "two events"),
        ("hh*4", "subdivision"),
        ("bd sn hh*4 cp", "mixed"),
    ];

    for (pattern_str, desc) in patterns {
        println!("\n=== {} ({}) ===", pattern_str, desc);
        let pattern = parse_mini_notation(pattern_str);

        // Query for one cycle
        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };

        let events = pattern.query(&state);
        println!("Total events in cycle: {}", events.len());

        for (i, event) in events.iter().enumerate() {
            let start = if let Some(whole) = &event.whole {
                whole.begin.to_float()
            } else {
                event.part.begin.to_float()
            };
            println!("  Event {}: '{}' at {:.4}", i, event.value, start);
        }
    }
}
