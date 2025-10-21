#[cfg(test)]
mod timing_test {
    use phonon::mini_notation_v3::parse_mini_notation;
    use phonon::pattern::{Fraction, State, TimeSpan};
    use std::collections::HashMap;

    #[test]
    fn test_euclid_event_spacing() {
        let pattern = parse_mini_notation("bd");
        let euclid_pattern = pattern.euclidean_legato(3, 8);

        let state = State {
            span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(1.0)),
            controls: HashMap::new(),
        };

        let events = euclid_pattern.query(&state);

        println!("\nEuclid(3, 8) event timing (1 second at tempo=2.0, so 0.5s per cycle):");
        for (i, event) in events.iter().enumerate() {
            let start = event
                .whole
                .as_ref()
                .map(|w| w.begin.to_float())
                .unwrap_or(0.0);
            let end = event
                .whole
                .as_ref()
                .map(|w| w.end.to_float())
                .unwrap_or(0.0);
            let duration = end - start;
            println!(
                "  Event {}: at {:.3}-{:.3} (duration: {:.3} cycles = {:.3}s)",
                i,
                start,
                end,
                duration,
                duration * 0.5
            );
        }
    }
}
