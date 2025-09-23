use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{State, TimeSpan, Fraction};
use std::collections::HashMap;

#[test]
fn debug_replicate_alternation() {
    // Test bd*<3,4> - the repeat amount alternates
    let pattern = parse_mini_notation("bd*<3,4>");

    println!("\nTesting bd*<3,4> - alternating repeat amounts:");

    for cycle in 0..4 {
        let begin = Fraction::new(cycle as i64, 1);
        let end = Fraction::new((cycle + 1) as i64, 1);
        let state = State {
            span: TimeSpan::new(begin, end),
            controls: HashMap::new(),
        };

        let events = pattern.query(&state);

        println!("\nCycle {}:", cycle);
        for event in &events {
            println!("  {:.3} -> {:.3} : {}",
                     event.part.begin.to_float(),
                     event.part.end.to_float(),
                     event.value);
        }

        let bd_count = events.iter().filter(|e| e.value == "bd").count();
        println!("  Total: {} bd events", bd_count);
    }
}