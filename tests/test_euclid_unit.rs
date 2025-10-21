#[cfg(test)]
mod euclid_test {
    use phonon::mini_notation_v3::parse_mini_notation;
    use phonon::pattern::{Fraction, State, TimeSpan};
    use std::collections::HashMap;

    #[test]
    fn test_euclid_generates_correct_events() {
        // Create a simple pattern
        let pattern = parse_mini_notation("bd");

        // Apply euclidean
        let euclid_pattern = pattern.euclidean_legato(3, 8);

        // Query for 1 full cycle
        let state = State {
            span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(1.0)),
            controls: HashMap::new(),
        };

        let events = euclid_pattern.query(&state);

        println!("Euclid(3, 8) generated {} events:", events.len());
        for (i, event) in events.iter().enumerate() {
            println!(
                "  Event {}: value='{}' at {}-{}",
                i,
                event.value,
                event
                    .whole
                    .as_ref()
                    .map(|w| w.begin.to_float())
                    .unwrap_or(0.0),
                event
                    .whole
                    .as_ref()
                    .map(|w| w.end.to_float())
                    .unwrap_or(0.0)
            );
        }

        // euclid(3, 8) should generate 3 events
        assert_eq!(events.len(), 3, "Expected 3 events from euclid(3, 8)");
    }
}
