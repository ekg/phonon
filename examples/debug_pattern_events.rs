use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use std::collections::HashMap;

fn main() {
    println!("=== Debugging Pattern Event Generation ===\n");

    let pattern = "~click ~click ~click ~click";
    let parsed_pattern = parse_mini_notation(pattern);

    let duration_secs = 2.0;
    let cycle_duration = 1.0;
    let num_cycles = ((duration_secs / cycle_duration) as f64).ceil() as i64;

    println!("Duration: {}s", duration_secs);
    println!("Cycle duration: {}s", cycle_duration);
    println!("Number of cycles: {}\n", num_cycles);

    let mut all_events = Vec::new();

    for cycle in 0..num_cycles {
        println!("Querying cycle {}:", cycle);

        let state = State {
            span: TimeSpan::new(Fraction::new(cycle, 1), Fraction::new(cycle + 1, 1)),
            controls: HashMap::new(),
        };

        println!(
            "  Query span: {:?} to {:?}",
            state.span.begin, state.span.end
        );

        let cycle_events = parsed_pattern.query(&state);
        println!("  Found {} events in this cycle", cycle_events.len());

        for (i, event) in cycle_events.iter().enumerate() {
            println!(
                "    Event {}: '{}' at {:?}-{:?}",
                i, event.value, event.part.begin, event.part.end
            );
        }

        // Adjust event timing to account for cycle offset
        for mut event in cycle_events {
            let cycle_offset = cycle as f64;
            let orig_begin = event.part.begin.to_float();
            let orig_end = event.part.end.to_float();

            event.part = TimeSpan::new(
                Fraction::from_float(orig_begin + cycle_offset),
                Fraction::from_float(orig_end + cycle_offset),
            );

            println!(
                "    Adjusted: {:.3}-{:.3} -> {:.3}-{:.3}",
                orig_begin,
                orig_end,
                event.part.begin.to_float(),
                event.part.end.to_float()
            );

            all_events.push(event);
        }
        println!();
    }

    println!("Total events across all cycles: {}", all_events.len());
    println!("\nFinal event list:");
    for (i, event) in all_events.iter().enumerate() {
        let start_time = event.part.begin.to_float();
        let end_time = event.part.end.to_float();
        println!(
            "  Event {}: '{}' at cycle position {:.3}-{:.3} ({}s-{}s)",
            i,
            event.value,
            start_time,
            end_time,
            start_time * cycle_duration,
            end_time * cycle_duration
        );
    }
}
