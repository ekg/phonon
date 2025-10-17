use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{State, TimeSpan, Fraction};
use std::collections::HashMap;

fn main() {
    let sample_rate = 44100.0;
    let cps = 0.5;
    let sample_width = 1.0 / sample_rate as f64 / cps as f64;

    let pattern = parse_mini_notation("bd sn hh*4 cp");

    println!("Testing narrow queries:");
    println!("Sample width: {:.10}\n", sample_width);

    // Check specific samples where events should trigger
    let event_positions = vec![0.0, 0.25, 0.5, 0.5625, 0.625, 0.6875, 0.75];

    for event_pos in event_positions {
        // Find the sample number for this event
        let sample_num = (event_pos / sample_width) as usize;
        let cycle_pos = sample_num as f64 * sample_width;
        let prev_pos = cycle_pos - sample_width;

        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle_pos),
                Fraction::from_float(cycle_pos + sample_width),
            ),
            controls: HashMap::new(),
        };

        let events = pattern.query(&state);

        println!("Event should be at: {:.6}", event_pos);
        println!("Sample {}: cycle_pos={:.6}, range=({:.6}, {:.6}]",
            sample_num, cycle_pos, prev_pos, cycle_pos);
        println!("Query returned {} events:", events.len());

        for event in events.iter() {
            let start = if let Some(whole) = &event.whole {
                whole.begin.to_float()
            } else {
                event.part.begin.to_float()
            };
            let in_range = start > prev_pos && start <= cycle_pos;
            println!("  '{}' at {:.6} - in range: {}", event.value, start, in_range);
        }
        println!();
    }
}
