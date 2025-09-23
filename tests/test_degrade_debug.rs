use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{State, TimeSpan, Fraction};
use std::collections::HashMap;

#[test]
fn debug_degrade() {
    println!("\n=== Debug Degrade Operator ===");

    // Test simple degrade
    let pattern = parse_mini_notation("bd? sn? hh? cp?");

    for cycle in 0..5 {
        let state = State {
            span: TimeSpan::new(
                Fraction::new(cycle as i64, 1),
                Fraction::new((cycle + 1) as i64, 1),
            ),
            controls: HashMap::new(),
        };

        let events = pattern.query(&state);
        println!("Cycle {}: {} events - {:?}",
                 cycle,
                 events.len(),
                 events.iter().map(|e| &e.value).collect::<Vec<_>>());
    }

    // Test without degrade for comparison
    println!("\n=== Without Degrade ===");
    let pattern2 = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let events = pattern2.query(&state);
    println!("Cycle 0: {} events - {:?}",
             events.len(),
             events.iter().map(|e| &e.value).collect::<Vec<_>>());
}