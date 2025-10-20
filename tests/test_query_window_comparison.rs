use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use std::collections::HashMap;

#[test]
fn compare_big_vs_small_query_windows() {
    println!("\n=== BIG CHUNK VS SAMPLE-BY-SAMPLE QUERY COMPARISON ===");

    let pattern_str = "bd bd bd bd";
    let base_pattern = parse_mini_notation(pattern_str);
    let degraded_pattern = base_pattern.degrade();

    // Method 1: Query entire 2-cycle span at once
    let state_big = State {
        span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(2.0)),
        controls: HashMap::new(),
    };
    let events_big = degraded_pattern.query(&state_big);

    println!("Method 1 (big query): {} events", events_big.len());
    for event in &events_big {
        println!(
            "  Event at {:.6}: {}",
            event.part.begin.to_float(),
            event.value
        );
    }

    // Method 2: Query sample-by-sample and collect unique event starts
    let sample_rate = 44100.0;
    let cps = 2.0;
    let sample_width = 1.0 / sample_rate as f64 / cps as f64;
    let num_samples = (2.0 * sample_rate / cps) as usize; // 2 cycles

    let mut unique_events = Vec::new();
    let mut last_event_start = -1.0;
    let tolerance = sample_width * 0.001;

    for sample_num in 0..num_samples {
        let cycle_pos = (sample_num as f64 / sample_rate as f64) * cps as f64;

        let state_small = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle_pos),
                Fraction::from_float(cycle_pos + sample_width),
            ),
            controls: HashMap::new(),
        };

        let events_small = degraded_pattern.query(&state_small);

        for event in events_small.iter() {
            if event.value.trim() == "~" || event.value.is_empty() {
                continue;
            }

            let event_start_abs = if let Some(whole) = &event.whole {
                whole.begin.to_float()
            } else {
                event.part.begin.to_float()
            };

            let event_is_new = event_start_abs > last_event_start + tolerance;

            if event_is_new {
                unique_events.push((event_start_abs, event.value.clone()));
                last_event_start = event_start_abs;
            }
        }
    }

    println!(
        "\nMethod 2 (sample-by-sample): {} unique events",
        unique_events.len()
    );
    for (time, value) in &unique_events {
        println!("  Event at {:.6}: {}", time, value);
    }

    println!("\nComparison:");
    println!("  Big query: {} events", events_big.len());
    println!("  Small queries: {} unique events", unique_events.len());

    // They should match!
    assert_eq!(
        events_big.len(),
        unique_events.len(),
        "Query methods should return same number of events"
    );
}
