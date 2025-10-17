//! Debug pattern parsing for "1 0 0 0"

use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use std::collections::HashMap;

#[test]
fn test_parse_one_zero_zero_zero() {
    let pattern = parse_mini_notation("1 0 0 0");

    // Query at different points in first cycle
    for i in 0..8 {
        let cycle_pos = i as f64 * 0.125; // 0.0, 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 0.875
        let sample_width = 0.001;

        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle_pos),
                Fraction::from_float(cycle_pos + sample_width),
            ),
            controls: HashMap::new(),
        };

        let events = pattern.query(&state);
        let value = if let Some(event) = events.first() {
            event.value.as_str()
        } else {
            "NO EVENT"
        };

        println!("Position {:.3}: value = '{}'", cycle_pos, value);
    }
}
