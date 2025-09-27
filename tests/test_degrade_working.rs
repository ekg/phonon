use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use phonon::pattern_ops::*;
use std::collections::HashMap;

#[test]
fn test_degrade_working() {
    // Create pattern directly
    let p1 = Pattern::pure("bd".to_string());
    let p2 = Pattern::pure("sn".to_string());
    let seq = Pattern::cat(vec![p1, p2]);
    let degraded = seq.degrade_by(0.5);

    for cycle in 0..3 {
        let state = State {
            span: TimeSpan::new(
                Fraction::new(cycle as i64, 1),
                Fraction::new((cycle + 1) as i64, 1),
            ),
            controls: HashMap::new(),
        };

        let events = degraded.query(&state);
        println!("Cycle {}: {} events", cycle, events.len());
    }
}
