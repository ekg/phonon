use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use std::collections::HashMap;

#[test]
fn debug_nested_alternation() {
    // Test nested alternation: <3,<4,5>>
    let pattern = parse_mini_notation("<3,<4,5>>");

    println!("\nTesting <3,<4,5>> - nested alternating patterns:");

    for cycle in 0..6 {
        let begin = Fraction::new(cycle as i64, 1);
        let end = Fraction::new((cycle + 1) as i64, 1);
        let state = State {
            span: TimeSpan::new(begin, end),
            controls: HashMap::new(),
        };

        let events = pattern.query(&state);

        println!("\nCycle {}:", cycle);
        for event in &events {
            println!("  Value: {:?}", event.value);
        }

        // Pattern should be: 3, 4, 3, 5, 3, 4, ...
        // This is a 4-cycle pattern
        let expected = match cycle % 4 {
            0 | 2 => "3",
            1 => "4",
            3 => "5",
            _ => "3",
        };

        if let Some(first_event) = events.first() {
            println!("  Got: {:?}, Expected: {}", first_event.value, expected);
        } else {
            println!("  No events!");
        }
    }
}
