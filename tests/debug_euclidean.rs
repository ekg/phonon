use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use std::collections::HashMap;

fn main() {
    // Test euclidean rhythm
    let pattern = parse_mini_notation("bd(3,8)");

    let begin = Fraction::new(0, 1);
    let end = Fraction::new(1, 1);
    let state = State {
        span: TimeSpan::new(begin, end),
        controls: HashMap::new(),
    };

    let events = pattern.query(&state);

    println!("Euclidean bd(3,8) events:");
    for event in events {
        println!(
            "  {} -> {} : {}",
            event.part.begin.to_float(),
            event.part.end.to_float(),
            event.value
        );
    }

    // Also test a simple pattern for comparison
    let simple = parse_mini_notation("bd sn hh");
    let simple_events = simple.query(&state);

    println!("\nSimple 'bd sn hh' events:");
    for event in simple_events {
        println!(
            "  {} -> {} : {}",
            event.part.begin.to_float(),
            event.part.end.to_float(),
            event.value
        );
    }
}
