use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use std::collections::HashMap;

#[test]
fn test_degrade_direct() {
    println!("\n=== Test Degrade Direct ===");

    // Create a simple pattern
    let pattern = Pattern::pure("bd".to_string()).degrade_by(Pattern::pure(0.5));

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
