use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use std::collections::HashMap;

#[test]
fn test_degrade_simple() {
    println!("\n=== Test Simple Degrade ===");

    // Test a single degraded element
    let pattern = parse_mini_notation("bd?");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let events = pattern.query(&state);
    println!("bd? produces {} events", events.len());

    // Test without ?
    let pattern2 = parse_mini_notation("bd");
    let events2 = pattern2.query(&state);
    println!("bd produces {} events", events2.len());

    // Test sequence with ?
    let pattern3 = parse_mini_notation("bd? sn?");
    let events3 = pattern3.query(&state);
    println!("'bd? sn?' produces {} events", events3.len());
}
