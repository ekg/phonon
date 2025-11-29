use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use std::collections::HashMap;

#[test]
fn test_combined_transform_at_pattern_level() {
    let pattern_str = "bd sn";
    let base_pattern = parse_mini_notation(pattern_str);

    // Apply fast then rev
    let combined = base_pattern.fast(Pattern::pure(2.0)).rev();

    // Query over 1 cycle
    let state = State {
        span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(1.0)),
        controls: HashMap::new(),
    };

    let combined_events = combined.query(&state);

    println!("\nCombined fast(2) $ rev: {} events", combined_events.len());
    for (i, event) in combined_events.iter().enumerate() {
        println!(
            "  Event {}: start={:.6}, end={:.6}, value={}",
            i,
            event.part.begin.to_float(),
            event.part.end.to_float(),
            event.value
        );
    }

    // fast(2) doubles events, so 2 -> 4 events
    assert!(
        combined_events.len() >= 3 && combined_events.len() <= 5,
        "Combined transform should have 3-5 events, got {}",
        combined_events.len()
    );
}
