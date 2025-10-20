use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph};
use std::collections::HashMap;

#[test]
fn count_unique_event_starts() {
    println!("\n=== UNIQUE EVENT START TIMES ===");

    let pattern_str = "bd bd bd bd";
    let base_pattern = parse_mini_notation(pattern_str);
    let degraded_pattern = base_pattern.degrade();

    // Simulate rendering and track unique event start times
    let sample_rate = 44100.0;
    let cps = 2.0;
    let sample_width = 1.0 / sample_rate as f64 / cps as f64;

    let mut last_event_start = -1.0;
    let tolerance = sample_width * 0.001;
    let mut unique_triggers = Vec::new();

    // Process 88200 samples (2 seconds)
    for sample_num in 0..88200 {
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
    println!("Expected: 4 (since degraded pattern has 4 events)");

    assert_eq!(
        unique_triggers.len(),
        4,
        "Should have exactly 4 unique event triggers"
    );
}
