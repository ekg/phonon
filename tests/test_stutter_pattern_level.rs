use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use std::collections::HashMap;

#[test]
fn test_stutter_pattern_query() {
    let pattern_str = "bd sn";
    let base_pattern = parse_mini_notation(pattern_str);
    let stutter_pattern = base_pattern.clone().stutter(3);

    // Query over 1 cycle
    let state = State {
        span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(1.0)),
        controls: HashMap::new(),
    };

    let base_events = base_pattern.query(&state);
    let stutter_events = stutter_pattern.query(&state);

    println!("\nBase pattern: {} events", base_events.len());
    for (i, event) in base_events.iter().enumerate() {
        println!(
            "  Event {}: start={:.6}, end={:.6}, value={}",
            i,
            event.part.begin.to_float(),
            event.part.end.to_float(),
            event.value
        );
    }

    println!("\nStutter(3) pattern: {} events", stutter_events.len());
    for (i, event) in stutter_events.iter().enumerate() {
        println!(
            "  Event {}: start={:.6}, end={:.6}, value={}",
            i,
            event.part.begin.to_float(),
            event.part.end.to_float(),
            event.value
        );
    }

    let ratio = stutter_events.len() as f32 / base_events.len() as f32;
    println!("\nRatio: {:.2}", ratio);

    // Stutter(3) should triple the event count: 2 events -> 6 events
    assert_eq!(
        stutter_events.len(),
        base_events.len() * 3,
        "Stutter(3) should have 3x events"
    );
}
