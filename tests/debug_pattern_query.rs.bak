/// Debug pattern query to see what events are returned
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use std::collections::HashMap;

#[test]
fn debug_pattern_queries() {
    let pattern = parse_mini_notation("220 330 440 330");

    println!("\n=== Pattern Query Debug ===");
    println!("Pattern: 220 330 440 330");
    println!();

    let cycle_positions = vec![0.0, 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 0.875, 1.0];

    for &pos in &cycle_positions {
        let sample_width = 1.0 / 44100.0 / 2.0; // at 2 CPS
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(pos),
                Fraction::from_float(pos + sample_width),
            ),
            controls: HashMap::new(),
        };

        let events = pattern.query(&state);
        println!("Cycle pos {:.3}:", pos);
        for (i, event) in events.iter().enumerate() {
            let event_start = if let Some(whole) = &event.whole {
                whole.begin.to_float()
            } else {
                event.part.begin.to_float()
            };
            println!(
                "  Event {}: value='{}', start={:.3}",
                i, event.value, event_start
            );
        }
        if events.is_empty() {
            println!("  No events");
        }
        println!();
    }
}
