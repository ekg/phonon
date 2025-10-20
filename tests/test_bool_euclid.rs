#[cfg(test)]
mod bool_euclid_test {
    use phonon::pattern::{Pattern, State, Fraction, TimeSpan};
    use std::collections::HashMap;

    #[test]
    fn test_bool_euclid_pattern() {
        // Generate boolean euclidean pattern
        let euclid = Pattern::<bool>::euclid(3, 8, 0);
        
        // Query for 1 full cycle
        let state = State {
            span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(1.0)),
            controls: HashMap::new(),
        };
        
        let events = euclid.query(&state);
        
        println!("Bool euclid(3, 8) generated {} events:", events.len());
        for (i, event) in events.iter().enumerate() {
            println!("  Event {}: value={} at {}-{}", 
                i, event.value,
                event.whole.as_ref().map(|w| w.begin.to_float()).unwrap_or(0.0),
                event.whole.as_ref().map(|w| w.end.to_float()).unwrap_or(0.0)
            );
        }
    }
}
