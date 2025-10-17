use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{State, TimeSpan, Fraction};
use std::collections::HashMap;

fn main() {
    let sample_rate = 44100.0;
    let cps = 0.5;
    let sample_width = 1.0 / sample_rate as f64 / cps as f64;

    println!("Tempo: {}, Sample width: {:.10}", cps, sample_width);
    println!("Samples per cycle: {:.0}\n", 1.0 / sample_width);

    let pattern = parse_mini_notation("bd sn hh*4 cp");

    // Simulate a few samples
    for sample_num in [0, 5500, 11000, 11500, 12000, 12500, 13000] {
        let cycle_pos = sample_num as f64 * sample_width;
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle_pos),
                Fraction::from_float(cycle_pos + sample_width),
            ),
            controls: HashMap::new(),
        };

        let events = pattern.query(&state);
        if !events.is_empty() {
            println!("Sample {}: cycle_pos {:.6}, events: {}",
                sample_num, cycle_pos, events.len());
            for event in events.iter() {
                let start = if let Some(whole) = &event.whole {
                    whole.begin.to_float()
                } else {
                    event.part.begin.to_float()
                };
                println!("  '{}' at {:.6}", event.value, start);
            }
        }
    }
}
