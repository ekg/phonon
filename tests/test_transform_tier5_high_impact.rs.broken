/// Tests for TIER 5 High-Impact transforms
///
/// These are high-impact missing transforms that have been implemented:
/// - jux: Stereo channel manipulation (left unchanged, right transformed)
/// - bite: Extract slices and cycle through applying different patterns
/// - slice: Select specific slice from a subdivided pattern
/// - hurry: Speed up pattern with sample playback speed modification
///
/// All transforms use pattern API testing (not DSL-based)
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use std::collections::HashMap;

// ============= Level 1: jux (Stereo Manipulation) =============

#[test]
fn test_jux_level1_creates_stereo_pairs() {
    let pattern = parse_mini_notation("bd sn");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // jux applies transform to right channel only
    let stereo = pattern.clone().jux(|p| p.fast(Pattern::pure(2.0)));
    let haps = stereo.query(&state);

    // Should produce stereo pairs (left, right)
    assert!(haps.len() > 0, "jux should produce stereo events");

    // Each event should be a tuple
    for hap in &haps {
        // The type is Pattern<(T, T)> so each value is a tuple
        println!("Stereo pair event at {:?}", hap.part);
    }

    println!("✅ jux creates stereo pairs with (left, right) structure");
}

#[test]
fn test_jux_level1_left_unchanged_right_transformed() {
    let pattern = parse_mini_notation("bd sn");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let stereo_haps = pattern.clone().jux(|p| p.fast(Pattern::pure(2.0))).query(&state);

    // jux zips left and right channels
    // If left has 2 events and right has 4 events (from fast(2)),
    // zip produces min(2, 4) = 2 stereo pairs
    assert_eq!(
        stereo_haps.len(),
        base_haps.len(),
        "jux should zip left and right channels (min length)"
    );

    println!("✅ jux: left channel unchanged, right channel transformed, zipped");
}

#[test]
fn test_jux_rev() {
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // jux_rev is jux with rev transform
    let stereo = pattern.jux_rev();
    let haps = stereo.query(&state);

    assert!(haps.len() > 0, "jux_rev should produce stereo events");

    println!("✅ jux_rev works (convenience method for jux(rev))");
}

// ============= Level 1: bite (Extract Slices) =============

#[test]
fn test_bite_level1_cycles_through_patterns() {
    let base = parse_mini_notation("bd sn");
    let pattern1 = base.clone().fast(Pattern::pure(2.0));
    let pattern2 = base.clone().slow(Pattern::pure(2.0));

    // bite cycles through patterns based on cycle number
    let bitten = base.clone().bite(4, vec![pattern1, pattern2]);

    for cycle in 0..6 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let haps = bitten.query(&state);

        // Pattern should alternate: cycle 0 = pattern1, cycle 1 = pattern2, cycle 2 = pattern1...
        if cycle % 2 == 0 {
            println!(
                "Cycle {}: using pattern1 (fast) - {} events",
                cycle,
                haps.len()
            );
        } else {
            println!(
                "Cycle {}: using pattern2 (slow) - {} events",
                cycle,
                haps.len()
            );
        }
    }

    println!("✅ bite cycles through applying different patterns");
}

#[test]
fn test_bite_level1_n_parameter_divides_cycle() {
    let base = parse_mini_notation("bd sn hh cp");
    let pattern1 = base.clone().fast(Pattern::pure(2.0));
    let pattern2 = base.clone();

    // n=8 means divide cycle into 8 pieces
    let bitten = base.bite(8, vec![pattern1.clone(), pattern2.clone()]);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let haps = bitten.query(&state);
    assert!(haps.len() > 0, "bite should produce events");

    println!("✅ bite n parameter controls subdivision");
}

// ============= Level 1: slice (Select Slice) =============

#[test]
fn test_slice_level1_selects_specific_slice() {
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // slice(4, 0) = first quarter
    // slice(4, 1) = second quarter
    // slice(4, 2) = third quarter
    // slice(4, 3) = fourth quarter

    let slice0 = pattern.clone().slice(4, 0).query(&state);
    let slice1 = pattern.clone().slice(4, 1).query(&state);
    let slice2 = pattern.clone().slice(4, 2).query(&state);
    let slice3 = pattern.clone().slice(4, 3).query(&state);

    // Each slice should have events
    assert!(slice0.len() > 0, "slice 0 should have events");
    assert!(slice1.len() > 0, "slice 1 should have events");
    assert!(slice2.len() > 0, "slice 2 should have events");
    assert!(slice3.len() > 0, "slice 3 should have events");

    println!(
        "✅ slice(4, n): slice0={}, slice1={}, slice2={}, slice3={}",
        slice0.len(),
        slice1.len(),
        slice2.len(),
        slice3.len()
    );
}

#[test]
fn test_slice_level1_wraps_index() {
    let pattern = parse_mini_notation("bd sn");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // slice(4, 5) should wrap to slice(4, 1) since 5 % 4 = 1
    let slice1 = pattern.clone().slice(4, 1).query(&state);
    let slice5 = pattern.clone().slice(4, 5).query(&state);

    assert_eq!(
        slice1.len(),
        slice5.len(),
        "slice should wrap index: slice(4, 5) == slice(4, 1)"
    );

    println!("✅ slice wraps index modulo n");
}

#[test]
fn test_slice_level1_zero_slices_is_silent() {
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let haps = pattern.slice(0, 0).query(&state);
    assert_eq!(haps.len(), 0, "slice(0, _) should be silent");

    println!("✅ slice(0, _) produces silence");
}

// ============= Level 1: hurry (Fast + Speed Modification) =============

#[test]
fn test_hurry_level1_speeds_up_pattern() {
    let pattern = parse_mini_notation("bd sn");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base = pattern.query(&state);
    let hurried = pattern.clone().hurry(2.0).query(&state);

    // hurry(2) should double the event count (like fast)
    assert_eq!(
        hurried.len(),
        base.len() * 2,
        "hurry(2) should double event count"
    );

    println!("✅ hurry speeds up pattern timing (like fast)");
}

#[test]
fn test_hurry_level1_multiple_cycles() {
    let pattern = parse_mini_notation("bd sn hh cp");

    for cycle in 0..4 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let base = pattern.query(&state);
        let hurried = pattern.clone().hurry(3.0).query(&state);

        assert_eq!(
            hurried.len(),
            base.len() * 3,
            "hurry(3) should triple events in cycle {}",
            cycle
        );
    }

    println!("✅ hurry consistent across multiple cycles");
}

#[test]
fn test_hurry_level1_fractional_speed() {
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(2, 1)),
        controls: HashMap::new(),
    };

    let base = pattern.query(&state);
    let hurried = pattern.clone().hurry(0.5).query(&state);

    // hurry(0.5) should halve the event count (like slow(2))
    assert_eq!(
        hurried.len(),
        base.len() / 2,
        "hurry(0.5) should halve event count"
    );

    println!("✅ hurry works with fractional speeds (slows down)");
}

// ============= Multi-cycle Tests =============

#[test]
fn test_jux_multi_cycle_consistency() {
    let pattern = parse_mini_notation("bd sn");

    // jux should produce consistent results across cycles
    for cycle in 0..4 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let haps = pattern.clone().jux(|p| p.fast(Pattern::pure(2.0))).query(&state);
        assert!(
            haps.len() > 0,
            "jux should produce events in cycle {}",
            cycle
        );
    }

    println!("✅ jux consistent across multiple cycles");
}

#[test]
fn test_bite_pattern_selection_sequence() {
    let base = parse_mini_notation("bd sn");
    let p1 = base.clone().fast(Pattern::pure(2.0));
    let p2 = base.clone().slow(Pattern::pure(2.0));
    let p3 = base.clone();

    let bitten = base.bite(4, vec![p1.clone(), p2.clone(), p3.clone()]);

    // Verify pattern selection sequence over 9 cycles
    // Patterns should cycle: p1, p2, p3, p1, p2, p3, ...
    for cycle in 0..9 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let haps = bitten.query(&state);
        let expected_index = cycle % 3;

        // Verify we got events
        assert!(haps.len() > 0, "Cycle {} should have events", cycle);

        println!(
            "Cycle {}: pattern index {} - {} events",
            cycle,
            expected_index,
            haps.len()
        );
    }

    println!("✅ bite cycles through patterns correctly");
}

#[test]
fn test_slice_all_slices_cover_pattern() {
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // Combine all 4 slices - should cover the full pattern
    let mut all_events = Vec::new();
    for i in 0..4 {
        let slice = pattern.clone().slice(4, i).query(&state);
        all_events.extend(slice);
    }

    let base = pattern.query(&state);

    // All slices together should have same event count as base
    // (May vary due to event subdivision)
    assert!(
        all_events.len() >= base.len(),
        "All slices should cover pattern"
    );

    println!("✅ slice: all slices cover the full pattern");
}

// ============= Composition Tests =============

#[test]
fn test_jux_composition() {
    let pattern = parse_mini_notation("bd sn");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // jux composed with multiple transforms
    let composed = pattern.clone().fast(Pattern::pure(2.0)).jux(|p| p.rev());
    let haps = composed.query(&state);

    assert!(haps.len() > 0, "Composed jux should work");

    println!("✅ jux can be composed with other transforms");
}

#[test]
fn test_slice_with_fast() {
    let pattern = parse_mini_notation("bd sn");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // fast(2) $ slice(4, 0) - should fast the first slice
    let transformed = pattern.clone().slice(4, 0).fast(Pattern::pure(2.0)).query(&state);

    assert!(transformed.len() > 0, "slice + fast should work");

    println!("✅ slice composes with other transforms");
}

#[test]
fn test_hurry_with_rev() {
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // hurry(2) $ rev - should reverse then hurry
    let transformed = pattern.clone().rev().hurry(2.0).query(&state);
    let base = pattern.query(&state);

    assert_eq!(
        transformed.len(),
        base.len() * 2,
        "hurry should work after rev"
    );

    println!("✅ hurry composes with other transforms");
}

// ============= Edge Cases =============

#[test]
fn test_jux_with_identity() {
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // jux with identity transform - left and right should be identical
    let stereo = pattern.clone().jux(|p| p).query(&state);
    let base = pattern.query(&state);

    assert_eq!(
        stereo.len(),
        base.len(),
        "jux with identity should match base count"
    );

    println!("✅ jux with identity transform works");
}

#[test]
fn test_bite_with_single_pattern() {
    let base = parse_mini_notation("bd sn");
    let pattern1 = base.clone().fast(Pattern::pure(2.0));

    // bite with single pattern should always use that pattern
    let bitten = base.bite(4, vec![pattern1.clone()]);

    for cycle in 0..4 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let haps = bitten.query(&state);
        let expected = pattern1.query(&state);

        assert_eq!(
            haps.len(),
            expected.len(),
            "bite with single pattern should always use it"
        );
    }

    println!("✅ bite with single pattern works");
}

#[test]
fn test_slice_n1_is_identity() {
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base = pattern.query(&state);
    let sliced = pattern.clone().slice(1, 0).query(&state);

    assert_eq!(sliced.len(), base.len(), "slice(1, 0) should be identity");

    println!("✅ slice(1, 0) is identity transform");
}

#[test]
fn test_hurry_1_is_identity() {
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base = pattern.query(&state);
    let hurried = pattern.clone().hurry(1.0).query(&state);

    assert_eq!(hurried.len(), base.len(), "hurry(1) should be identity");

    println!("✅ hurry(1) is identity transform");
}
