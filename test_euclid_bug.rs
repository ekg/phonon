use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{State, TimeSpan, Fraction};
use std::collections::HashMap;

fn main() {
    let pattern = parse_mini_notation("cp(2,4)");

    println!("Testing cp(2,4) - should be X . X . every cycle");
    println!("Expected: 2 events per cycle at positions 0 and 0.5\n");

    for cycle in 0..4 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let events = pattern.query(&state);
        println!("Cycle {}: {} events", cycle, events.len());
        for (i, event) in events.iter().enumerate() {
            println!("  Event {}: value='{}', whole={:?}",
                i, event.value,
                event.whole.as_ref().map(|ts| (ts.begin.to_float(), ts.end.to_float()))
            );
        }
    }
}
