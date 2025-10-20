use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph};
use std::collections::HashMap;

#[test]
fn log_events_during_rendering() {
    println!("\n=== EVENT LOGGING DURING RENDERING ===");

    let pattern_str = "bd bd bd bd";
    let base_pattern = parse_mini_notation(pattern_str);
    let degraded_pattern = base_pattern.degrade();

    // Query to see what events exist over 2 cycles
    let state_full = State {
        span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(2.0)),
        controls: HashMap::new(),
    };
    let all_events = degraded_pattern.query(&state_full);
    println!("Degraded pattern has {} total events:", all_events.len());
    for event in &all_events {
        println!("  Event at time {:.6}", event.part.begin.to_float());
    }

    // Now let's simulate what happens during rendering
    // We'll manually query the pattern at each sample position
    let sample_rate = 44100.0;
    let cps = 2.0;
    let sample_width = 1.0 / sample_rate as f64 / cps as f64;

    println!("\nSample width: {:.10} cycles", sample_width);
    println!("\nQuerying pattern at each sample position (first 1000 samples):");

    let mut trigger_count = 0;
    for sample_num in 0..1000 {
        let cycle_pos = (sample_num as f64 / sample_rate as f64) * cps as f64;

        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle_pos),
                Fraction::from_float(cycle_pos + sample_width),
            ),
            controls: HashMap::new(),
        };

        let events = degraded_pattern.query(&state);
        if !events.is_empty() {
            println!(
                "  Sample {}: cycle_pos={:.10}, {} events",
                sample_num,
                cycle_pos,
                events.len()
            );
            for event in &events {
                println!("    Event: {}", event.value);
            }
            trigger_count += events.len();
        }
    }

    println!(
        "\nTotal events detected in first 1000 samples: {}",
        trigger_count
    );
}
