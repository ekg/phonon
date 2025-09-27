use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use std::collections::HashMap;

#[test]
fn test_alternation_with_replicate() {
    println!("\n=== Testing Alternation with Replicate ===");

    // First test plain alternation
    let alt = parse_mini_notation("<sn cp>");
    println!("\nPlain <sn cp>:");
    for cycle in 0..3 {
        let state = State {
            span: TimeSpan::new(
                Fraction::new(cycle as i64, 1),
                Fraction::new((cycle + 1) as i64, 1),
            ),
            controls: HashMap::new(),
        };
        let events = alt.query(&state);
        println!(
            "  Cycle {}: {:?}",
            cycle,
            events.iter().map(|e| &e.value).collect::<Vec<_>>()
        );
    }

    // Now test with static replicate
    let rep2 = parse_mini_notation("<sn cp>*2");
    println!("\n<sn cp>*2:");
    for cycle in 0..3 {
        let state = State {
            span: TimeSpan::new(
                Fraction::new(cycle as i64, 1),
                Fraction::new((cycle + 1) as i64, 1),
            ),
            controls: HashMap::new(),
        };
        let events = rep2.query(&state);
        println!(
            "  Cycle {}: {:?}",
            cycle,
            events.iter().map(|e| &e.value).collect::<Vec<_>>()
        );
    }

    // Test the parsed structure
    println!("\n=== Checking if issue is in parsing or evaluation ===");

    // Manually create what we expect
    let manual = parse_mini_notation("sn sn"); // What cycle 0 should look like
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    let manual_events = manual.query(&state);
    println!(
        "Manual 'sn sn': {:?}",
        manual_events.iter().map(|e| &e.value).collect::<Vec<_>>()
    );

    let manual2 = parse_mini_notation("cp cp"); // What cycle 1 should look like
    let state2 = State {
        span: TimeSpan::new(Fraction::new(1, 1), Fraction::new(2, 1)),
        controls: HashMap::new(),
    };
    let manual_events2 = manual2.query(&state2);
    println!(
        "Manual 'cp cp': {:?}",
        manual_events2.iter().map(|e| &e.value).collect::<Vec<_>>()
    );
}
