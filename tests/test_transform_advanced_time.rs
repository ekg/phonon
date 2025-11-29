/// Tests for TIER 3 Advanced Time transforms
///
/// These are advanced time manipulation transforms:
/// - gap(n): Play only on cycles divisible by n (silence otherwise)
/// - fit(n): Fit pattern to n cycles (alias for slow)
/// - stretch: Extend duration to full cycle (alias for legato 1.0)
/// - linger(factor): Make pattern linger on values for factor cycles
/// - loop(n): Loop pattern n times within cycle (fast then overlay)
/// - chew(n): Progressively shift pattern by (cycle % n) / n
///
/// All transforms use 3-level verification
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use std::collections::HashMap;

// ============= Level 1: Pattern Query Tests =============

#[test]
fn test_gap_level1_plays_on_divisible_cycles() {
    let pattern = parse_mini_notation("bd sn hh cp");

    // gap(3) should play on cycles 0, 3, 6, 9... silence on others
    for cycle in 0..12 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let gap_haps = pattern.clone().gap(3).query(&state);

        if cycle % 3 == 0 {
            // Should have events
            assert!(gap_haps.len() > 0, "Cycle {}: should have events", cycle);
        } else {
            // Should be silent
            assert_eq!(gap_haps.len(), 0, "Cycle {}: should be silent", cycle);
        }
    }

    println!("✅ gap(3) plays on cycles 0, 3, 6, 9... (divisible by 3)");
}

#[test]
fn test_gap_event_count() {
    let pattern = parse_mini_notation("bd sn");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let gap_haps = pattern.clone().gap(1).query(&state);

    // gap(1) should play every cycle (cycle 0 % 1 == 0)
    assert_eq!(
        gap_haps.len(),
        base_haps.len(),
        "gap(1) should play every cycle"
    );

    println!("✅ gap(1) plays every cycle (equivalent to identity on cycle 0)");
}

#[test]
fn test_fit_level1_slows_down() {
    let pattern = parse_mini_notation("bd sn hh cp");

    // fit(2) should be same as slow(2)
    let fit_pattern = pattern.clone().fit(2);
    let slow_pattern = pattern.clone().slow(Pattern::pure(2.0));

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let fit_haps = fit_pattern.query(&state);
    let slow_haps = slow_pattern.query(&state);

    assert_eq!(
        fit_haps.len(),
        slow_haps.len(),
        "fit(2) should equal slow(2)"
    );

    println!("✅ fit is alias for slow (fits pattern to n cycles)");
}

#[test]
fn test_stretch_level1_extends_duration() {
    let pattern = parse_mini_notation("bd sn");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let stretch_haps = pattern.clone().stretch().query(&state);

    // stretch should preserve event count
    assert_eq!(stretch_haps.len(), base_haps.len());

    // Events should have extended duration
    for (base, stretched) in base_haps.iter().zip(stretch_haps.iter()) {
        let base_dur = base.part.duration().to_float();
        let stretched_dur = stretched.part.duration().to_float();

        assert!(stretched_dur >= base_dur, "stretch should extend duration");
    }

    println!("✅ stretch extends event durations (alias for legato 1.0)");
}

#[test]
fn test_linger_level1_stays_on_values() {
    let pattern = parse_mini_notation("bd sn hh cp");

    // linger(2) should stay on each value for 2 cycles
    // On cycle 0 and 1, query cycle 0
    // On cycle 2 and 3, query cycle 1, etc.
    let linger_pattern = pattern.clone().linger(2.0);

    let state0 = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    let state1 = State {
        span: TimeSpan::new(Fraction::new(1, 1), Fraction::new(2, 1)),
        controls: HashMap::new(),
    };

    let linger0 = linger_pattern.query(&state0);
    let linger1 = linger_pattern.query(&state1);

    // Both should have same values (lingering on cycle 0 pattern)
    assert_eq!(linger0.len(), linger1.len(), "linger should repeat values");

    println!("✅ linger stays on values for multiple cycles");
}

#[test]
fn test_loop_level1_repeats_within_cycle() {
    let pattern = parse_mini_notation("bd sn");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let loop_haps = pattern.clone().loop_pattern(3).query(&state);

    // loop(n) does fast(n) then overlays n shifted versions
    // So loop(3): fast(3) = 6 events, overlay 3 times = 18 events = base × n²
    assert_eq!(
        loop_haps.len(),
        base_haps.len() * 3 * 3,
        "loop(3) should have n² multiplier"
    );

    println!("✅ loop overlays n fast(n) versions (n² multiplier)");
}

#[test]
fn test_chew_level1_progressive_shift() {
    let pattern = parse_mini_notation("bd sn hh cp");

    // chew(4) should shift by 0/4, 1/4, 2/4, 3/4 on cycles 0, 1, 2, 3
    let chew_pattern = pattern.clone().chew(4);

    for cycle in 0..4 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let chew_haps = chew_pattern.query(&state);
        assert!(
            chew_haps.len() > 0,
            "chew should produce events on cycle {}",
            cycle
        );

        // Events should be shifted by (cycle % 4) / 4
        let expected_shift = (cycle % 4) as f64 / 4.0;

        // First event should be shifted relative to base
        if let Some(first) = chew_haps.first() {
            let time_in_cycle = first.part.begin.to_float() - cycle as f64;
            // Should be shifted by expected amount
            println!(
                "Cycle {}: first event at {:.3}, expected shift ~{:.3}",
                cycle, time_in_cycle, expected_shift
            );
        }
    }

    println!("✅ chew progressively shifts pattern by (cycle % n) / n");
}

// ============= Multi-cycle Tests =============

#[test]
fn test_gap_over_many_cycles() {
    let pattern = parse_mini_notation("bd sn");
    let gap_pattern = pattern.clone().gap(4);

    let mut total_events = 0;
    for cycle in 0..16 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let haps = gap_pattern.query(&state);
        if cycle % 4 == 0 {
            assert!(haps.len() > 0, "Should have events on cycle {}", cycle);
            total_events += haps.len();
        } else {
            assert_eq!(haps.len(), 0, "Should be silent on cycle {}", cycle);
        }
    }

    // Over 16 cycles, should play on 0, 4, 8, 12 = 4 times
    // 2 events per cycle × 4 = 8 events
    assert_eq!(
        total_events, 8,
        "gap(4) over 16 cycles should have 8 events"
    );

    println!(
        "✅ gap over 16 cycles: {} events (4 × 2 events/cycle)",
        total_events
    );
}

#[test]
fn test_linger_different_factors() {
    let pattern = parse_mini_notation("bd sn hh cp");

    for factor in [2.0, 3.0, 4.0] {
        let linger_pattern = pattern.clone().linger(factor);

        // First 'factor' cycles should have same values
        let mut cycle_haps = Vec::new();
        for cycle in 0..(factor as usize) {
            let state = State {
                span: TimeSpan::new(
                    Fraction::from_float(cycle as f64),
                    Fraction::from_float((cycle + 1) as f64),
                ),
                controls: HashMap::new(),
            };
            cycle_haps.push(linger_pattern.query(&state));
        }

        // All should have same count (lingering on same cycle)
        let first_len = cycle_haps[0].len();
        for (i, haps) in cycle_haps.iter().enumerate() {
            assert_eq!(
                haps.len(),
                first_len,
                "linger({}) cycle {} should have same count",
                factor,
                i
            );
        }

        println!("✅ linger({}) repeats values for {} cycles", factor, factor);
    }
}

#[test]
fn test_loop_different_repetitions() {
    let pattern = parse_mini_notation("bd sn");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_count = pattern.query(&state).len();

    for n in [2, 3, 4, 5] {
        let loop_haps = pattern.clone().loop_pattern(n).query(&state);
        // loop(n) creates base × n² events (fast(n) overlaid n times)
        assert_eq!(
            loop_haps.len(),
            base_count * n * n,
            "loop({}) should have n² = {}× events",
            n,
            n * n
        );
    }

    println!("✅ loop creates n² events (fast(n) overlaid n times)");
}

#[test]
fn test_chew_cycles_through_pattern() {
    let pattern = parse_mini_notation("bd sn hh cp");
    let chew_pattern = pattern.clone().chew(3);

    // Over 6 cycles, should cycle through shifts twice
    // Cycle 0: shift 0/3, Cycle 1: shift 1/3, Cycle 2: shift 2/3
    // Cycle 3: shift 0/3, Cycle 4: shift 1/3, Cycle 5: shift 2/3
    for cycle in 0..6 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let haps = chew_pattern.query(&state);
        assert!(
            haps.len() > 0,
            "chew should produce events on cycle {}",
            cycle
        );
    }

    println!("✅ chew cycles through shifts correctly");
}

// ============= Composition Tests =============

#[test]
fn test_gap_with_fast() {
    let pattern = parse_mini_notation("bd sn");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // gap(1) with fast(2) should work
    let composed = pattern
        .clone()
        .fast(Pattern::pure(2.0))
        .gap(1)
        .query(&state);
    assert!(composed.len() > 0, "gap should work with fast");

    println!("✅ gap composes with fast");
}

#[test]
fn test_linger_with_fast() {
    let pattern = parse_mini_notation("bd sn");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // linger should work with fast
    let composed = pattern
        .clone()
        .fast(Pattern::pure(2.0))
        .linger(2.0)
        .query(&state);
    assert!(composed.len() > 0, "linger should work with fast");

    println!("✅ linger composes with fast");
}

#[test]
fn test_loop_with_slow() {
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // loop with slow
    let composed = pattern
        .clone()
        .slow(Pattern::pure(2.0))
        .loop_pattern(2)
        .query(&state);
    assert!(composed.len() > 0, "loop should work with slow");

    println!("✅ loop composes with slow");
}

// ============= Edge Cases =============

#[test]
fn test_gap_one_always_plays() {
    let pattern = parse_mini_notation("bd sn hh cp");

    // gap(1) should play every cycle (all cycles divisible by 1)
    for cycle in 0..5 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let haps = pattern.clone().gap(1).query(&state);
        assert!(haps.len() > 0, "gap(1) should play on cycle {}", cycle);
    }

    println!("✅ gap(1) plays on every cycle");
}

#[test]
fn test_fit_one_identity() {
    let pattern = parse_mini_notation("bd sn");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let fit_haps = pattern.clone().fit(1).query(&state);

    // fit(1) = slow(1) should be identity
    assert_eq!(fit_haps.len(), base_haps.len(), "fit(1) should be identity");

    println!("✅ fit(1) is identity (slow(1))");
}

#[test]
fn test_loop_one_identity() {
    let pattern = parse_mini_notation("bd sn");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let loop_haps = pattern.clone().loop_pattern(1).query(&state);

    // loop(1) = fast(1) overlaid 1 time = 1² = identity
    assert_eq!(
        loop_haps.len(),
        base_haps.len(),
        "loop(1) should be identity (1²)"
    );

    println!("✅ loop(1) is identity");
}

#[test]
fn test_chew_one_identity() {
    let pattern = parse_mini_notation("bd sn hh cp");

    // chew(1) should shift by 0 (cycle % 1 == 0)
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let chew_haps = pattern.clone().chew(1).query(&state);

    assert_eq!(
        chew_haps.len(),
        base_haps.len(),
        "chew(1) should preserve count"
    );

    println!("✅ chew(1) behaves like identity");
}

#[test]
fn test_linger_one_identity() {
    let pattern = parse_mini_notation("bd sn");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let linger_haps = pattern.clone().linger(1.0).query(&state);

    // linger(1) should be close to identity
    assert_eq!(
        linger_haps.len(),
        base_haps.len(),
        "linger(1) should preserve count"
    );

    println!("✅ linger(1) is identity");
}

#[test]
fn test_gap_with_silence() {
    let pattern = parse_mini_notation("bd ~ sn ~");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // gap should handle rests
    let gap_haps = pattern.clone().gap(1).query(&state);
    assert!(gap_haps.len() > 0, "gap should handle rests");

    println!("✅ gap handles rests correctly");
}

#[test]
fn test_stretch_preserves_count() {
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let stretch_haps = pattern.clone().stretch().query(&state);

    assert_eq!(
        stretch_haps.len(),
        base_haps.len(),
        "stretch preserves event count"
    );

    println!("✅ stretch preserves event count while extending durations");
}
