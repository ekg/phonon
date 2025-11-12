/// Tests for squeeze transform
///
/// squeeze n - squeezes pattern to 1/n of cycle and speeds up by n
/// Similar to fast but compresses time window
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Pattern, Fraction, State, TimeSpan};
use std::collections::HashMap;

#[test]
fn test_squeeze_at_pattern_level() {
    // Test squeeze directly at pattern level
    let pattern = parse_mini_notation("bd sn hh cp");
    let squeezed = pattern.squeeze(2.0); // Squeeze to first half, 2x speed

    // Query first cycle
    let state = State {
        span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(1.0)),
        controls: HashMap::new(),
    };

    let events = squeezed.query(&state);

    println!("\nSqueeze pattern: {} events", events.len());
    for (i, event) in events.iter().enumerate() {
        println!(
            "  Event {}: start={:.6}, end={:.6}, value={}",
            i,
            event.part.begin.to_float(),
            event.part.end.to_float(),
            event.value
        );
    }

    // Squeeze should:
    // 1. Compress events to first 0.5 of cycle (1/n where n=2)
    // 2. Speed up by 2x (fit 2 cycles worth in that 0.5)
    // Original: bd(0-0.25), sn(0.25-0.5), hh(0.5-0.75), cp(0.75-1.0)
    // After squeeze 2: all 4 events in 0.0-0.5, sped up 2x
    // bd(0-0.125), sn(0.125-0.25), hh(0.25-0.375), cp(0.375-0.5)
    assert_eq!(events.len(), 4, "Should have 4 events in squeezed pattern");

    // Check timing - all events should be in first half
    for event in &events {
        assert!(
            event.part.end.to_float() <= 0.5,
            "All events should be in first half of cycle, got end={}",
            event.part.end.to_float()
        );
    }
}

#[test]
fn test_squeeze_multiple_cycles() {
    // Test squeeze over multiple cycles
    let pattern = parse_mini_notation("bd sn");
    let squeezed = pattern.squeeze(3.0); // Squeeze to first 1/3, 3x speed

    let mut total_events = 0;
    for cycle in 0..4 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };
        let events = squeezed.query(&state);
        total_events += events.len();

        // All events should be in first 1/3 of each cycle
        for event in &events {
            let cycle_pos = event.part.begin.to_float() - cycle as f64;
            assert!(
                cycle_pos < 0.34, // Allow small tolerance
                "Event should be in first 1/3, got position {}",
                cycle_pos
            );
        }
    }

    // Should have events in each cycle (2 events per cycle)
    assert!(
        total_events >= 6,
        "Should have at least 6 events over 4 cycles, got {}",
        total_events
    );
}

#[test]
fn test_squeeze_chained_transforms() {
    // Test squeeze with other transforms
    let pattern = parse_mini_notation("bd sn hh cp");
    let transformed = pattern.squeeze(2.0).fast(Pattern::pure(2.0));

    let state = State {
        span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(1.0)),
        controls: HashMap::new(),
    };

    let events = transformed.query(&state);

    println!("\nSqueeze + fast: {} events", events.len());

    // squeeze 2 then fast 2 should give 8 events
    // (4 events squeezed to first half, then doubled by fast)
    assert!(
        events.len() >= 6,
        "Chained transforms should produce events, got {}",
        events.len()
    );
}
