//! Debug test to see what pattern queries return

use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use std::collections::HashMap;

#[test]
fn test_pattern_query_alternating() {
    // Parse pattern "0 1"
    let pattern = parse_mini_notation("0 1");
    let cps = 2.0;
    let sample_rate = 44100.0;

    // Query the pattern at different cycle positions
    for cycle_num in 0..4 {
        let cycle_pos = cycle_num as f64 * 0.5; // At 2 cps, each cycle is 0.5 seconds
        let sample_width = 1.0 / sample_rate as f64 / cps as f64;

        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle_pos),
                Fraction::from_float(cycle_pos + sample_width),
            ),
            controls: HashMap::new(),
        };

        let events = pattern.query(&state);
        println!(
            "Cycle {}: cycle_pos={:.6}, events={}",
            cycle_num,
            cycle_pos,
            events.len()
        );

        if let Some(event) = events.first() {
            println!(
                "  Event value: '{}', whole: {:?}, part: {:?}",
                event.value,
                event.whole.as_ref().map(|ts| format!(
                    "{:.3}..{:.3}",
                    ts.begin.to_float(),
                    ts.end.to_float()
                )),
                format!(
                    "{:.3}..{:.3}",
                    event.part.begin.to_float(),
                    event.part.end.to_float()
                )
            );
        }
    }
}

#[test]
fn test_pattern_query_throughout_cycle() {
    // Parse pattern "0 1"
    let pattern = parse_mini_notation("0 1");
    let cps = 1.0;
    let sample_rate = 44100.0;
    let sample_width = 1.0 / sample_rate as f64 / cps as f64;

    // Query at different points in the first cycle
    println!("\nQuerying throughout cycle 0:");
    for i in 0..10 {
        let cycle_pos = i as f64 * 0.1; // 0.0, 0.1, 0.2, ..., 0.9

        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle_pos),
                Fraction::from_float(cycle_pos + sample_width),
            ),
            controls: HashMap::new(),
        };

        let events = pattern.query(&state);
        let value = events.first().map(|e| e.value.as_str()).unwrap_or("none");
        println!("  pos={:.1}: value='{}'", cycle_pos, value);
    }

    // Query at different points in the second cycle
    println!("\nQuerying throughout cycle 1:");
    for i in 0..10 {
        let cycle_pos = 1.0 + i as f64 * 0.1; // 1.0, 1.1, 1.2, ..., 1.9

        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle_pos),
                Fraction::from_float(cycle_pos + sample_width),
            ),
            controls: HashMap::new(),
        };

        let events = pattern.query(&state);
        let value = events.first().map(|e| e.value.as_str()).unwrap_or("none");
        println!("  pos={:.1}: value='{}'", cycle_pos, value);
    }
}
