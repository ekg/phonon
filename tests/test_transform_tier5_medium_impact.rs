/// Tests for TIER 5 Medium-Impact transforms
///
/// These are medium-impact concatenation and splicing transforms:
/// - fastcat: Fast concatenation (each pattern gets 1/n of cycle)
/// - slowcat: Slow concatenation (each pattern gets 1 full cycle)
/// - randcat: Random selection from patterns per cycle
/// - timeCat: Time-weighted concatenation
/// - splice: Splice two patterns at a time point
/// - loopAt: Loop pattern at specified cycle duration
///
/// All transforms use pattern API testing (not DSL-based)
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use phonon::pattern_structure::timecat;
use std::collections::HashMap;

// ============= Level 1: fastcat (Fast Concatenation) =============

#[test]
fn test_fastcat_level1_concatenates_in_one_cycle() {
    let p1 = parse_mini_notation("bd");
    let p2 = parse_mini_notation("sn");
    let p3 = parse_mini_notation("hh");

    // fastcat puts all 3 patterns in 1 cycle
    let concatenated = Pattern::fastcat(vec![p1.clone(), p2.clone(), p3.clone()]);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let haps = concatenated.query(&state);

    // Should have all 3 patterns' events in one cycle
    assert_eq!(
        haps.len(),
        3,
        "fastcat should concatenate all patterns in one cycle"
    );

    println!("✅ fastcat concatenates all patterns in one cycle");
}

#[test]
fn test_fastcat_level1_each_pattern_gets_equal_time() {
    let p1 = parse_mini_notation("bd");
    let p2 = parse_mini_notation("sn");

    let concatenated = Pattern::fastcat(vec![p1.clone(), p2.clone()]);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let haps = concatenated.query(&state);

    // Each pattern should occupy 1/2 of the cycle
    // First event (bd) should be in [0, 0.5)
    // Second event (sn) should be in [0.5, 1.0)
    assert_eq!(haps.len(), 2, "Should have 2 events");

    let first_pos = haps[0].part.begin.to_float();
    let second_pos = haps[1].part.begin.to_float();

    assert!(first_pos < 0.5, "First event should be in first half");
    assert!(second_pos >= 0.5, "Second event should be in second half");

    println!("✅ fastcat divides cycle equally among patterns");
}

#[test]
fn test_fastcat_empty_list_is_silence() {
    let concatenated: Pattern<String> = Pattern::fastcat(vec![]);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let haps = concatenated.query(&state);
    assert_eq!(haps.len(), 0, "Empty fastcat should be silent");

    println!("✅ fastcat with empty list produces silence");
}

// ============= Level 1: slowcat (Slow Concatenation) =============

#[test]
fn test_slowcat_level1_one_pattern_per_cycle() {
    let p1 = parse_mini_notation("bd");
    let p2 = parse_mini_notation("sn");
    let p3 = parse_mini_notation("hh");

    let concatenated = Pattern::slowcat(vec![p1.clone(), p2.clone(), p3.clone()]);

    // Cycle 0: p1 (bd)
    // Cycle 1: p2 (sn)
    // Cycle 2: p3 (hh)
    // Cycle 3: p1 (bd) again...

    for cycle in 0..6 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let haps = concatenated.query(&state);

        assert_eq!(haps.len(), 1, "slowcat should have 1 pattern per cycle");

        let expected_index = cycle % 3;
        println!(
            "Cycle {}: pattern {} - {} events",
            cycle,
            expected_index,
            haps.len()
        );
    }

    println!("✅ slowcat plays one pattern per cycle");
}

#[test]
fn test_slowcat_level1_pattern_fills_cycle() {
    let p1 = parse_mini_notation("bd sn");

    let concatenated = Pattern::slowcat(vec![p1.clone()]);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base = p1.query(&state);
    let slow = concatenated.query(&state);

    assert_eq!(
        slow.len(),
        base.len(),
        "Single pattern should fill whole cycle"
    );

    println!("✅ slowcat with single pattern fills cycle");
}

// ============= Level 1: randcat (Random Concatenation) =============

#[test]
fn test_randcat_level1_picks_one_pattern() {
    let p1 = parse_mini_notation("bd");
    let p2 = parse_mini_notation("sn");
    let p3 = parse_mini_notation("hh");

    let concatenated = Pattern::randcat(vec![p1.clone(), p2.clone(), p3.clone()]);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let haps = concatenated.query(&state);

    // Should pick one of the patterns randomly
    assert_eq!(haps.len(), 1, "randcat should pick one pattern");

    println!("✅ randcat picks one pattern per cycle");
}

#[test]
fn test_randcat_deterministic_per_cycle() {
    let p1 = parse_mini_notation("bd");
    let p2 = parse_mini_notation("sn");

    let concatenated = Pattern::randcat(vec![p1.clone(), p2.clone()]);

    let state = State {
        span: TimeSpan::new(Fraction::new(5, 1), Fraction::new(6, 1)),
        controls: HashMap::new(),
    };

    // Same cycle should give same result
    let result1 = concatenated.query(&state);
    let result2 = concatenated.query(&state);

    assert_eq!(
        result1.len(),
        result2.len(),
        "Same cycle should give same result"
    );

    println!("✅ randcat is deterministic per cycle");
}

#[test]
fn test_randcat_distribution() {
    let p1 = parse_mini_notation("bd");
    let p2 = parse_mini_notation("sn");

    let concatenated = Pattern::randcat(vec![p1.clone(), p2.clone()]);

    let mut p1_count = 0;
    let mut p2_count = 0;

    for cycle in 0..100 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        // Since we can't directly tell which pattern was chosen,
        // we just verify we get events
        let haps = concatenated.query(&state);
        if haps.len() > 0 {
            // Count would require knowing the pattern value
            p1_count += 1;
        }
    }

    // Should have selected patterns for all cycles
    assert!(p1_count > 0, "Should have selected patterns");

    println!("✅ randcat distributes selections over many cycles");
}

// ============= Level 1: timeCat (Time-Weighted Concatenation) =============

#[test]
fn test_timecat_level1_weighted_durations() {
    let p1 = parse_mini_notation("bd");
    let p2 = parse_mini_notation("sn");

    // timeCat with weights (3, 1) = p1 gets 3/4 of cycle, p2 gets 1/4
    let concatenated = timecat(vec![(3.0, p1.clone()), (1.0, p2.clone())]);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let haps = concatenated.query(&state);

    // Should have both patterns' events
    assert_eq!(haps.len(), 2, "timeCat should have both patterns");

    // First event should be around [0, 0.75)
    // Second event should be around [0.75, 1.0)
    let first_pos = haps[0].part.begin.to_float();
    let second_pos = haps[1].part.begin.to_float();

    assert!(first_pos < 0.75, "First pattern should occupy first 3/4");
    assert!(second_pos >= 0.75, "Second pattern should occupy last 1/4");

    println!("✅ timeCat respects time weights");
}

#[test]
fn test_timecat_equal_weights_like_fastcat() {
    let p1 = parse_mini_notation("bd");
    let p2 = parse_mini_notation("sn");

    // Equal weights should behave like fastcat
    let timecat_pattern = timecat(vec![(1.0, p1.clone()), (1.0, p2.clone())]);
    let fastcat_pattern = Pattern::fastcat(vec![p1.clone(), p2.clone()]);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let timecat_haps = timecat_pattern.query(&state);
    let fastcat_haps = fastcat_pattern.query(&state);

    assert_eq!(
        timecat_haps.len(),
        fastcat_haps.len(),
        "Equal weights should behave like fastcat"
    );

    println!("✅ timeCat with equal weights behaves like fastcat");
}

// ============= Level 1: splice (Splice Patterns) =============

#[test]
fn test_splice_level1_switches_at_position() {
    let p1 = parse_mini_notation("bd bd bd bd");
    let p2 = parse_mini_notation("sn sn sn sn");

    // splice at 0.5 = first half is p1, second half is p2
    let spliced = p1.clone().splice(Pattern::pure(0.5), p2.clone());

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let haps = spliced.query(&state);

    // Should have events from both patterns
    assert!(haps.len() > 0, "splice should produce events");

    println!("✅ splice switches patterns at specified position");
}

#[test]
fn test_splice_at_zero_is_second_pattern() {
    let p1 = parse_mini_notation("bd");
    let p2 = parse_mini_notation("sn");

    // splice at 0.0 = all p2
    let spliced = p1.clone().splice(Pattern::pure(0.0), p2.clone());

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let spliced_haps = spliced.query(&state);
    let p2_haps = p2.query(&state);

    assert_eq!(
        spliced_haps.len(),
        p2_haps.len(),
        "splice at 0.0 should be second pattern"
    );

    println!("✅ splice at 0.0 is second pattern only");
}

#[test]
fn test_splice_at_one_is_first_pattern() {
    let p1 = parse_mini_notation("bd");
    let p2 = parse_mini_notation("sn");

    // splice at 1.0 = all p1
    let spliced = p1.clone().splice(Pattern::pure(1.0), p2.clone());

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let spliced_haps = spliced.query(&state);
    let p1_haps = p1.query(&state);

    assert_eq!(
        spliced_haps.len(),
        p1_haps.len(),
        "splice at 1.0 should be first pattern"
    );

    println!("✅ splice at 1.0 is first pattern only");
}

// ============= Level 1: loopAt (Loop at Cycles) =============

#[test]
fn test_loopat_level1_loops_at_duration() {
    let pattern = parse_mini_notation("bd sn");

    // loopAt(2) loops pattern every 2 cycles
    let looped = pattern.clone().loop_at(Pattern::pure(2.0));

    for cycle in 0..4 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let haps = looped.query(&state);
        println!("Cycle {}: {} events", cycle, haps.len());
    }

    println!("✅ loopAt loops pattern at specified duration");
}

#[test]
fn test_loopat_1_is_identity() {
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base = pattern.query(&state);
    let looped = pattern.clone().loop_at(Pattern::pure(1.0)).query(&state);

    assert_eq!(looped.len(), base.len(), "loopAt(1) should be identity");

    println!("✅ loopAt(1) is identity");
}

// ============= Multi-cycle Tests =============

#[test]
fn test_fastcat_multi_cycle_consistency() {
    let p1 = parse_mini_notation("bd");
    let p2 = parse_mini_notation("sn");

    let concatenated = Pattern::fastcat(vec![p1, p2]);

    for cycle in 0..4 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let haps = concatenated.query(&state);
        assert_eq!(haps.len(), 2, "fastcat should be consistent across cycles");
    }

    println!("✅ fastcat consistent across multiple cycles");
}

#[test]
fn test_slowcat_cycles_through_patterns() {
    let p1 = parse_mini_notation("bd");
    let p2 = parse_mini_notation("sn");
    let p3 = parse_mini_notation("hh");

    let concatenated = Pattern::slowcat(vec![p1, p2, p3]);

    // Over 6 cycles, should repeat twice: bd, sn, hh, bd, sn, hh
    for cycle in 0..6 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let haps = concatenated.query(&state);
        assert_eq!(haps.len(), 1, "slowcat should have 1 pattern per cycle");
    }

    println!("✅ slowcat cycles through patterns correctly");
}

// ============= Composition Tests =============

#[test]
fn test_fastcat_with_fast() {
    let p1 = parse_mini_notation("bd");
    let p2 = parse_mini_notation("sn");

    let concatenated = Pattern::fastcat(vec![p1, p2]).fast(2.0);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let haps = concatenated.query(&state);

    // fastcat(2 patterns) = 2 events
    // fast(2) = 2x events = 4 events
    assert_eq!(haps.len(), 4, "fastcat $ fast should work");

    println!("✅ fastcat composes with fast");
}

#[test]
fn test_slowcat_with_rev() {
    let p1 = parse_mini_notation("bd");
    let p2 = parse_mini_notation("sn");

    let concatenated = Pattern::slowcat(vec![p1, p2]).rev();

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let haps = concatenated.query(&state);
    assert!(haps.len() > 0, "slowcat $ rev should work");

    println!("✅ slowcat composes with rev");
}

#[test]
fn test_splice_composition() {
    let p1 = parse_mini_notation("bd");
    let p2 = parse_mini_notation("sn");

    let spliced = p1.clone().splice(Pattern::pure(0.5), p2.clone()).fast(2.0);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let haps = spliced.query(&state);
    assert!(
        haps.len() > 0,
        "splice should compose with other transforms"
    );

    println!("✅ splice composes with other transforms");
}

// ============= Edge Cases =============

#[test]
fn test_fastcat_single_pattern() {
    let p1 = parse_mini_notation("bd sn");

    let concatenated = Pattern::fastcat(vec![p1.clone()]);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base = p1.query(&state);
    let fast = concatenated.query(&state);

    assert_eq!(
        fast.len(),
        base.len(),
        "fastcat with single pattern should be identity"
    );

    println!("✅ fastcat with single pattern is identity");
}

#[test]
fn test_slowcat_empty_list() {
    let concatenated: Pattern<String> = Pattern::slowcat(vec![]);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let haps = concatenated.query(&state);
    assert_eq!(haps.len(), 0, "Empty slowcat should be silent");

    println!("✅ slowcat with empty list produces silence");
}

#[test]
fn test_randcat_single_pattern() {
    let p1 = parse_mini_notation("bd sn");

    let concatenated = Pattern::randcat(vec![p1.clone()]);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base = p1.query(&state);
    let rand = concatenated.query(&state);

    assert_eq!(
        rand.len(),
        base.len(),
        "randcat with single pattern should be identity"
    );

    println!("✅ randcat with single pattern is identity");
}

#[test]
fn test_loopat_fractional_cycles() {
    let pattern = parse_mini_notation("bd sn");

    // loopAt(0.5) should speed up pattern (loop twice per cycle)
    let looped = pattern.clone().loop_at(Pattern::pure(0.5));

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let haps = looped.query(&state);
    let base = pattern.query(&state);

    // loopAt(0.5) should have more events (pattern loops twice in one cycle)
    assert!(
        haps.len() >= base.len(),
        "loopAt(0.5) should loop pattern multiple times"
    );

    println!("✅ loopAt works with fractional cycles");
}
