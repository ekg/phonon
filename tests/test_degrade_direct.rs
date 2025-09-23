use phonon::pattern::{Pattern, State, TimeSpan, Fraction};
use phonon::pattern_ops::*;
use std::collections::HashMap;

#[test]
fn test_degrade_direct() {
    println!("\n=== Test Degrade Direct ===");

    // Create a simple pattern
    let pattern = Pattern::pure("bd".to_string())
        .degrade_by(0.5);

    for cycle in 0..5 {
        let state = State {
            span: TimeSpan::new(
                Fraction::new(cycle as i64, 1),
                Fraction::new((cycle + 1) as i64, 1),
            ),
            controls: HashMap::new(),
        };

        let events = pattern.query(&state);
        println!("Cycle {}: {} events", cycle, events.len());
    }
}