/// Test that jux produces correct event patterns with transform chains
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use std::collections::HashMap;

#[test]
fn test_jux_fast2_rev_pattern_structure() {
    // Test: jux (fast 2 $ rev)
    // Expected:
    // - Left channel: 4 events (bd sn hh cp) panned left
    // - Right channel: 8 events (fast 2 $ rev = doubled then reversed) panned right

    let pattern = parse_mini_notation("bd sn hh cp");

    // Manual composition: first fast 2, then rev
    let fast2 = pattern.clone().fast(2.0);
    let fast2_rev = fast2.rev();

    // Manual jux: stack original (left) with transformed (right)
    let left = Pattern::new({
        let p = pattern.clone();
        move |state: &State| {
            let mut haps = p.query(state);
            for hap in &mut haps {
                hap.context.insert("pan".to_string(), "-1".to_string());
            }
            haps
        }
    });

    let right = Pattern::new({
        let p = fast2_rev.clone();
        move |state: &State| {
            let mut haps = p.query(state);
            for hap in &mut haps {
                hap.context.insert("pan".to_string(), "1".to_string());
            }
            haps
        }
    });

    let manual_jux = Pattern::stack(vec![left, right]);

    // Query one cycle
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let events = manual_jux.query(&state);

    // Separate by pan
    let mut left_events = vec![];
    let mut right_events = vec![];

    for event in &events {
        let pan = event.context.get("pan").map(|s| s.as_str()).unwrap_or("0");
        let value = &event.value;
        let start = event.whole.as_ref().map(|w| w.begin.to_float()).unwrap_or(0.0);

        if pan == "-1" {
            left_events.push((start, value.clone()));
        } else if pan == "1" {
            right_events.push((start, value.clone()));
        }
    }

    left_events.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    right_events.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    // Verify counts
    assert_eq!(
        left_events.len(),
        4,
        "Left channel should have 4 events (original pattern)"
    );
    assert_eq!(
        right_events.len(),
        8,
        "Right channel should have 8 events (fast 2)"
    );

    // Verify left channel order (should be bd sn hh cp)
    assert_eq!(left_events[0].1, "bd", "Left event 0 should be bd");
    assert_eq!(left_events[1].1, "sn", "Left event 1 should be sn");
    assert_eq!(left_events[2].1, "hh", "Left event 2 should be hh");
    assert_eq!(left_events[3].1, "cp", "Left event 3 should be cp");

    // Verify right channel order (fast 2 then rev)
    // Original: bd sn hh cp (at 0, 0.25, 0.5, 0.75)
    // fast 2: compresses to 0.5 cycle and repeats twice
    //   First half:  bd sn hh cp (at 0, 0.125, 0.25, 0.375)
    //   Second half: bd sn hh cp (at 0.5, 0.625, 0.75, 0.875)
    // rev: reverses each repetition
    //   First half:  cp hh sn bd (at 0, 0.125, 0.25, 0.375)
    //   Second half: cp hh sn bd (at 0.5, 0.625, 0.75, 0.875)
    println!("\nRight channel events:");
    for (i, (start, value)) in right_events.iter().enumerate() {
        println!("  {}: {:.3} = {}", i, start, value);
    }

    assert_eq!(right_events[0].1, "cp", "Right event 0 should be cp");
    assert_eq!(right_events[1].1, "hh", "Right event 1 should be hh");
    assert_eq!(right_events[2].1, "sn", "Right event 2 should be sn");
    assert_eq!(right_events[3].1, "bd", "Right event 3 should be bd");
    assert_eq!(right_events[4].1, "cp", "Right event 4 should be cp");
    assert_eq!(right_events[5].1, "hh", "Right event 5 should be hh");
    assert_eq!(right_events[6].1, "sn", "Right event 6 should be sn");
    assert_eq!(right_events[7].1, "bd", "Right event 7 should be bd");
}

#[test]
fn test_jux_rev_fast2_pattern_structure() {
    // Test: jux (rev $ fast 2) - DIFFERENT ORDER
    // Expected:
    // - Left channel: 4 events (bd sn hh cp) panned left
    // - Right channel: 8 events (rev $ fast 2 = reversed then doubled) panned right

    let pattern = parse_mini_notation("bd sn hh cp");

    // Manual composition: first rev, then fast 2
    let rev_pattern = pattern.clone().rev();
    let rev_fast2 = rev_pattern.fast(2.0);

    // Manual jux
    let left = Pattern::new({
        let p = pattern.clone();
        move |state: &State| {
            let mut haps = p.query(state);
            for hap in &mut haps {
                hap.context.insert("pan".to_string(), "-1".to_string());
            }
            haps
        }
    });

    let right = Pattern::new({
        let p = rev_fast2.clone();
        move |state: &State| {
            let mut haps = p.query(state);
            for hap in &mut haps {
                hap.context.insert("pan".to_string(), "1".to_string());
            }
            haps
        }
    });

    let manual_jux = Pattern::stack(vec![left, right]);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let events = manual_jux.query(&state);

    let mut right_events = vec![];
    for event in &events {
        let pan = event.context.get("pan").map(|s| s.as_str()).unwrap_or("0");
        if pan == "1" {
            let value = &event.value;
            let start = event.whole.as_ref().map(|w| w.begin.to_float()).unwrap_or(0.0);
            right_events.push((start, value.clone()));
        }
    }

    right_events.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    assert_eq!(
        right_events.len(),
        8,
        "Right channel should have 8 events"
    );

    // Verify right channel order (rev then fast 2)
    // Original: bd sn hh cp
    // rev: cp hh sn bd
    // fast 2: compresses and repeats
    //   cp hh sn bd | cp hh sn bd
    println!("\nRight channel events (rev $ fast 2):");
    for (i, (start, value)) in right_events.iter().enumerate() {
        println!("  {}: {:.3} = {}", i, start, value);
    }

    assert_eq!(right_events[0].1, "cp", "Right event 0 should be cp");
    assert_eq!(right_events[1].1, "hh", "Right event 1 should be hh");
    assert_eq!(right_events[2].1, "sn", "Right event 2 should be sn");
    assert_eq!(right_events[3].1, "bd", "Right event 3 should be bd");
    assert_eq!(right_events[4].1, "cp", "Right event 4 should be cp");
    assert_eq!(right_events[5].1, "hh", "Right event 5 should be hh");
    assert_eq!(right_events[6].1, "sn", "Right event 6 should be sn");
    assert_eq!(right_events[7].1, "bd", "Right event 7 should be bd");
}
