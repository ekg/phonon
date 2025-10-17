use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{State, TimeSpan, Fraction};
use std::collections::HashMap;

fn main() {
    let sample_rate = 44100.0;
    let cps = 0.5;
    let sample_width = 1.0 / sample_rate as f64 / cps as f64;

    println!("Simulating Sample node triggering logic");
    println!("Sample rate: {}, CPS: {}, Sample width: {:.10}\n", sample_rate, cps, sample_width);

    let pattern = parse_mini_notation("bd sn hh*4 cp");
    let mut last_event_start = -1.0;
    let tolerance = sample_width * 0.001;

    // Simulate processing a few samples where events should trigger
    let sample_positions = vec![0, 5512, 11025, 12403, 13781, 15159, 16537];

    for sample_num in sample_positions {
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
            println!("\n=== Sample {} (cycle_pos {:.6}) ===", sample_num, cycle_pos);
            println!("last_event_start: {:.6}", last_event_start);
            println!("Query returned {} events:", events.len());

            let mut latest_triggered_start = last_event_start;

            for event in events.iter() {
                let sample_name = event.value.trim();
                if sample_name == "~" || sample_name.is_empty() {
                    continue;
                }

                let event_start_abs = if let Some(whole) = &event.whole {
                    whole.begin.to_float()
                } else {
                    event.part.begin.to_float()
                };

                let event_is_new = event_start_abs > last_event_start + tolerance;

                println!("  '{}' at {:.6}", sample_name, event_start_abs);
                println!("    Check: {:.6} > {:.6} + {:.10} = {}",
                    event_start_abs, last_event_start, tolerance, event_is_new);

                if event_is_new {
                    println!("    -> TRIGGER!");
                    if event_start_abs > latest_triggered_start {
                        latest_triggered_start = event_start_abs;
                    }
                } else {
                    println!("    -> skip (already triggered)");
                }
            }

            if latest_triggered_start > last_event_start {
                last_event_start = latest_triggered_start;
                println!("Updated last_event_start to: {:.6}", last_event_start);
            }
        }
    }
}
