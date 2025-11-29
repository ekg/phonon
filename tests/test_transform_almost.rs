/// Tests for almostAlways and almostNever transforms
///
/// These are the final two transforms to achieve 100% verification:
/// - almostAlways: Apply transform with 90% probability
/// - almostNever: Apply transform with 10% probability (same as rarely)
///
/// All transforms use pattern API testing (not DSL-based)
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use std::collections::HashMap;

// ============= Level 1: almostAlways (90% Probability) =============

#[test]
fn test_almost_always_level1_high_probability() {
    let pattern = parse_mini_notation("bd sn");

    let mut applied_count = 0;
    let total_cycles = 100;

    for cycle in 0..total_cycles {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let base = pattern.query(&state);
        let transformed = pattern
            .clone()
            .almost_always(|p| p.fast(Pattern::pure(2.0)))
            .query(&state);

        // If transformed, should have 2x events
        if transformed.len() > base.len() {
            applied_count += 1;
        }
    }

    // With prob=0.9 over 100 cycles, expect ~80-95 applications
    let probability = applied_count as f64 / total_cycles as f64;
    assert!(
        probability > 0.80 && probability < 0.95,
        "almostAlways should apply ~90% of time, got {:.1}%",
        probability * 100.0
    );

    println!(
        "✅ almostAlways over 100 cycles: applied {:.1}% (expected ~90%)",
        probability * 100.0
    );
}

#[test]
fn test_almost_always_deterministic() {
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(5, 1), Fraction::new(6, 1)),
        controls: HashMap::new(),
    };

    // Same cycle should give same result
    let result1 = pattern
        .clone()
        .almost_always(|p| p.fast(Pattern::pure(2.0)))
        .query(&state);
    let result2 = pattern
        .clone()
        .almost_always(|p| p.fast(Pattern::pure(2.0)))
        .query(&state);

    assert_eq!(
        result1.len(),
        result2.len(),
        "Same cycle should give same result"
    );

    println!("✅ almostAlways is deterministic per cycle");
}

#[test]
fn test_almost_always_vs_often() {
    let pattern = parse_mini_notation("bd sn");
    let cycles = 1000;

    let mut almost_always_count = 0;
    let mut often_count = 0;

    for cycle in 0..cycles {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let base = pattern.query(&state);

        let almost_always = pattern
            .clone()
            .almost_always(|p| p.fast(Pattern::pure(2.0)))
            .query(&state);
        if almost_always.len() > base.len() {
            almost_always_count += 1;
        }

        let often = pattern
            .clone()
            .often(|p| p.fast(Pattern::pure(2.0)))
            .query(&state);
        if often.len() > base.len() {
            often_count += 1;
        }
    }

    let almost_always_prob = almost_always_count as f64 / cycles as f64;
    let often_prob = often_count as f64 / cycles as f64;

    // almostAlways (90%) should trigger more than often (75%)
    assert!(
        almost_always_prob > often_prob,
        "almostAlways ({:.1}%) should trigger more than often ({:.1}%)",
        almost_always_prob * 100.0,
        often_prob * 100.0
    );

    println!(
        "✅ almostAlways ({:.1}%) triggers more than often ({:.1}%)",
        almost_always_prob * 100.0,
        often_prob * 100.0
    );
}

// ============= Level 1: almostNever (10% Probability) =============

#[test]
fn test_almost_never_level1_low_probability() {
    let pattern = parse_mini_notation("bd sn");

    let mut applied_count = 0;
    let total_cycles = 100;

    for cycle in 0..total_cycles {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let base = pattern.query(&state);
        let transformed = pattern
            .clone()
            .almost_never(|p| p.fast(Pattern::pure(2.0)))
            .query(&state);

        // If transformed, should have 2x events
        if transformed.len() > base.len() {
            applied_count += 1;
        }
    }

    // With prob=0.1 over 100 cycles, expect ~5-20 applications
    let probability = applied_count as f64 / total_cycles as f64;
    assert!(
        probability > 0.05 && probability < 0.20,
        "almostNever should apply ~10% of time, got {:.1}%",
        probability * 100.0
    );

    println!(
        "✅ almostNever over 100 cycles: applied {:.1}% (expected ~10%)",
        probability * 100.0
    );
}

#[test]
fn test_almost_never_same_as_rarely() {
    let pattern = parse_mini_notation("bd sn");
    let cycles = 1000;

    let mut almost_never_count = 0;
    let mut rarely_count = 0;

    for cycle in 0..cycles {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let base = pattern.query(&state);

        let almost_never = pattern
            .clone()
            .almost_never(|p| p.fast(Pattern::pure(2.0)))
            .query(&state);
        if almost_never.len() > base.len() {
            almost_never_count += 1;
        }

        let rarely = pattern
            .clone()
            .rarely(|p| p.fast(Pattern::pure(2.0)))
            .query(&state);
        if rarely.len() > base.len() {
            rarely_count += 1;
        }
    }

    let almost_never_prob = almost_never_count as f64 / cycles as f64;
    let rarely_prob = rarely_count as f64 / cycles as f64;

    // Both should be ~10%, within 3% of each other
    assert!(
        (almost_never_prob - rarely_prob).abs() < 0.03,
        "almostNever ({:.1}%) should match rarely ({:.1}%)",
        almost_never_prob * 100.0,
        rarely_prob * 100.0
    );

    println!(
        "✅ almostNever ({:.1}%) matches rarely ({:.1}%)",
        almost_never_prob * 100.0,
        rarely_prob * 100.0
    );
}

#[test]
fn test_almost_never_deterministic() {
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(7, 1), Fraction::new(8, 1)),
        controls: HashMap::new(),
    };

    // Same cycle should give same result
    let result1 = pattern
        .clone()
        .almost_never(|p| p.fast(Pattern::pure(2.0)))
        .query(&state);
    let result2 = pattern
        .clone()
        .almost_never(|p| p.fast(Pattern::pure(2.0)))
        .query(&state);

    assert_eq!(
        result1.len(),
        result2.len(),
        "Same cycle should give same result"
    );

    println!("✅ almostNever is deterministic per cycle");
}

// ============= Multi-cycle Tests =============

#[test]
fn test_almost_always_multi_cycle_consistency() {
    let pattern = parse_mini_notation("bd sn");

    // Verify consistent behavior over multiple cycles
    for cycle in 0..10 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let haps = pattern
            .clone()
            .almost_always(|p| p.fast(Pattern::pure(2.0)))
            .query(&state);
        // Should produce events (either base or transformed)
        assert!(
            haps.len() >= 2,
            "almostAlways should produce events in cycle {}",
            cycle
        );
    }

    println!("✅ almostAlways consistent across multiple cycles");
}

#[test]
fn test_almost_never_multi_cycle_consistency() {
    let pattern = parse_mini_notation("bd sn");

    // Verify consistent behavior over multiple cycles
    for cycle in 0..10 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let haps = pattern
            .clone()
            .almost_never(|p| p.fast(Pattern::pure(2.0)))
            .query(&state);
        // Should produce events (either base or transformed)
        assert!(
            haps.len() >= 2,
            "almostNever should produce events in cycle {}",
            cycle
        );
    }

    println!("✅ almostNever consistent across multiple cycles");
}

// ============= Composition Tests =============

#[test]
fn test_almost_always_composition() {
    let pattern = parse_mini_notation("bd sn");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // almostAlways composed with fast
    let composed = pattern
        .clone()
        .fast(Pattern::pure(2.0))
        .almost_always(|p| p.rev());
    let haps = composed.query(&state);

    assert!(haps.len() > 0, "Composed almostAlways should work");

    println!("✅ almostAlways composes with other transforms");
}

#[test]
fn test_almost_never_composition() {
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // almostNever composed with slow
    let composed = pattern
        .clone()
        .slow(Pattern::pure(2.0))
        .almost_never(|p| p.fast(Pattern::pure(4.0)));
    let haps = composed.query(&state);

    assert!(haps.len() > 0, "Composed almostNever should work");

    println!("✅ almostNever composes with other transforms");
}

#[test]
fn test_nested_almost() {
    let pattern = parse_mini_notation("bd sn");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // Nested: almostAlways(almostNever(fast))
    // Effective probability ≈ 0.9 * 0.1 = 0.09 (9%)
    let _nested = pattern
        .clone()
        .almost_always(|p| p.almost_never(|p2| p2.fast(Pattern::pure(2.0))))
        .query(&state);

    println!("✅ almostAlways and almostNever can be nested");
}

// ============= Edge Cases =============

#[test]
fn test_almost_always_with_identity() {
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // almostAlways with identity should always give base pattern
    let base = pattern.query(&state);
    let transformed = pattern.clone().almost_always(|p| p).query(&state);

    assert_eq!(
        transformed.len(),
        base.len(),
        "Identity transform should not change count"
    );

    println!("✅ almostAlways with identity works correctly");
}

#[test]
fn test_almost_never_with_identity() {
    let pattern = parse_mini_notation("bd sn");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // almostNever with identity should always give base pattern
    let base = pattern.query(&state);
    let transformed = pattern.clone().almost_never(|p| p).query(&state);

    assert_eq!(
        transformed.len(),
        base.len(),
        "Identity transform should not change count"
    );

    println!("✅ almostNever with identity works correctly");
}

#[test]
fn test_probability_spectrum() {
    let pattern = parse_mini_notation("bd sn");
    let cycles = 1000;

    // Test the full spectrum of probability transforms
    let mut rarely_count = 0; // 10%
    let mut almost_never_count = 0; // 10%
    let mut sometimes_count = 0; // 50%
    let mut often_count = 0; // 75%
    let mut almost_always_count = 0; // 90%

    for cycle in 0..cycles {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let base = pattern.query(&state);

        if pattern
            .clone()
            .rarely(|p| p.fast(Pattern::pure(2.0)))
            .query(&state)
            .len()
            > base.len()
        {
            rarely_count += 1;
        }
        if pattern
            .clone()
            .almost_never(|p| p.fast(Pattern::pure(2.0)))
            .query(&state)
            .len()
            > base.len()
        {
            almost_never_count += 1;
        }
        if pattern
            .clone()
            .sometimes(|p| p.fast(Pattern::pure(2.0)))
            .query(&state)
            .len()
            > base.len()
        {
            sometimes_count += 1;
        }
        if pattern
            .clone()
            .often(|p| p.fast(Pattern::pure(2.0)))
            .query(&state)
            .len()
            > base.len()
        {
            often_count += 1;
        }
        if pattern
            .clone()
            .almost_always(|p| p.fast(Pattern::pure(2.0)))
            .query(&state)
            .len()
            > base.len()
        {
            almost_always_count += 1;
        }
    }

    println!("Probability spectrum over {} cycles:", cycles);
    println!(
        "  rarely:        {:.1}% (expected 10%)",
        rarely_count as f64 / cycles as f64 * 100.0
    );
    println!(
        "  almostNever:   {:.1}% (expected 10%)",
        almost_never_count as f64 / cycles as f64 * 100.0
    );
    println!(
        "  sometimes:     {:.1}% (expected 50%)",
        sometimes_count as f64 / cycles as f64 * 100.0
    );
    println!(
        "  often:         {:.1}% (expected 75%)",
        often_count as f64 / cycles as f64 * 100.0
    );
    println!(
        "  almostAlways:  {:.1}% (expected 90%)",
        almost_always_count as f64 / cycles as f64 * 100.0
    );

    // Verify ordering: rarely < sometimes < often < almostAlways
    assert!(rarely_count < sometimes_count);
    assert!(sometimes_count < often_count);
    assert!(often_count < almost_always_count);

    println!("✅ Full probability spectrum works correctly");
}
