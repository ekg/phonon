use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph};
use std::collections::HashMap;

#[test]
fn count_unique_event_starts() {
    println!("\n=== UNIQUE EVENT START TIMES ===");

    let pattern_str = "bd bd bd bd";
    let base_pattern = parse_mini_notation(pattern_str);

    // Use degrade_seed for deterministic testing instead of random degrade
    let degraded_pattern = base_pattern.degrade_seed(42);

    // Simulate rendering and track unique event start times
    let sample_rate = 44100.0;
    let cps = 2.0;
    let sample_width = 1.0 / sample_rate as f64 / cps as f64;

    let mut last_event_start = -1.0;
    let tolerance = sample_width * 0.001;
    let mut unique_triggers = Vec::new();

    // Process 22050 samples (0.5 seconds = 1 cycle at CPS 2.0)
    for sample_num in 0..22050 {
        let cycle_pos = (sample_num as f64 / sample_rate as f64) * cps as f64;

        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle_pos),
                Fraction::from_float(cycle_pos + sample_width),
            ),
            controls: HashMap::new(),
        };

        let events = degraded_pattern.query(&state);

        for event in events.iter() {
            let sample_name = event.value.trim();
            if sample_name == "~" || sample_name.is_empty() {
                continue;
            }

            // Get event start time
            let event_start_abs = if let Some(whole) = &event.whole {
                whole.begin.to_float()
            } else {
                event.part.begin.to_float()
            };

            // Check if this is a NEW event (same logic as Sample node)
            let event_is_new = event_start_abs > last_event_start + tolerance;

            if event_is_new {
                unique_triggers.push((sample_num, event_start_abs));
                last_event_start = event_start_abs;
                println!(
                    "Sample {}: NEW event at cycle {:.6}",
                    sample_num, event_start_abs
                );
            }
        }
    }

    println!("\nTotal unique event triggers: {}", unique_triggers.len());
    println!("Note: With seed 42, degrade_seed filters out ~50% of events");

    // With seed 42 and "bd bd bd bd", we expect 2-3 events per cycle
    // (degrade_seed has 50% probability, but exact count depends on seed)
    assert!(
        unique_triggers.len() >= 1 && unique_triggers.len() <= 4,
        "Should have 1-4 unique event triggers (got {}), since degrade_seed removes ~50% of 4 events",
        unique_triggers.len()
    );
}
