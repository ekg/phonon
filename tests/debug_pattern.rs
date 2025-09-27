use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use std::collections::HashMap;

#[test]
fn debug_euclidean() {
    // Test just euclidean pattern alone
    let pattern = parse_mini_notation("bd(3,8)");

    let begin = Fraction::new(0, 1);
    let end = Fraction::new(1, 1);
    let state = State {
        span: TimeSpan::new(begin, end),
        controls: HashMap::new(),
    };

    let events = pattern.query(&state);

    println!("\nEuclidean bd(3,8) alone - {} events:", events.len());
    for event in &events {
        println!(
            "  {:.3} -> {:.3} : {}",
            event.part.begin.to_float(),
            event.part.end.to_float(),
            event.value
        );
    }

    // Test euclidean with other pattern
    let pattern2 = parse_mini_notation("bd(3,8) sn");
    let events2 = pattern2.query(&state);

    println!("\nEuclidean 'bd(3,8) sn' - {} events:", events2.len());
    for event in &events2 {
        println!(
            "  {:.3} -> {:.3} : {}",
            event.part.begin.to_float(),
            event.part.end.to_float(),
            event.value
        );
    }

    // Check if it's parsed as euclidean or as regular pattern
    assert!(events.len() > 0, "Should have events");
}
