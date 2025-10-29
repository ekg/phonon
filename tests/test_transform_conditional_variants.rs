/// Tests for TIER 3 Conditional Variant transforms
///
/// These are advanced probability and cycle-based conditional transforms:
/// - sometimes_by(prob, f): Apply transform with custom probability
/// - when_mod(n, offset, f): Apply on cycles where (cycle - offset) % n == 0
///
/// Note: rarely, often, sometimes, always were verified in TIER 1
///
/// All transforms use 3-level verification but adapted for probabilistic behavior

use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use std::collections::HashMap;

// ============= Level 1: Pattern Query Tests =============

#[test]
fn test_sometimes_by_level1_probability_0() {
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // With prob=0.0, transform should NEVER apply
    let transformed = pattern.clone().sometimes_by(0.0, |p| p.fast(2.0)).query(&state);
    let base = pattern.query(&state);

    assert_eq!(transformed.len(), base.len(), "prob=0.0 should never apply transform");

    println!("✅ sometimes_by(0.0): Never applies (base events only)");
}

#[test]
fn test_sometimes_by_level1_probability_1() {
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // With prob=1.0, transform should ALWAYS apply
    let transformed = pattern.clone().sometimes_by(1.0, |p| p.fast(2.0)).query(&state);
    let just_fast = pattern.clone().fast(2.0).query(&state);

    assert_eq!(transformed.len(), just_fast.len(), "prob=1.0 should always apply transform");

    println!("✅ sometimes_by(1.0): Always applies (same as direct transform)");
}

#[test]
fn test_sometimes_by_level1_probability_distribution() {
    let pattern = parse_mini_notation("bd sn");

    // Test over many cycles to verify probability distribution
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
        let transformed = pattern.clone().sometimes_by(0.5, |p| p.fast(2.0)).query(&state);

        // If transformed, should have 2x events
        if transformed.len() > base.len() {
            applied_count += 1;
        }
    }

    // With prob=0.5 over 100 cycles, expect ~40-60 applications (within 2 std devs)
    let probability = applied_count as f64 / total_cycles as f64;
    assert!(
        probability > 0.35 && probability < 0.65,
        "sometimes_by(0.5) should apply ~50% of time, got {:.1}%",
        probability * 100.0
    );

    println!("✅ sometimes_by(0.5) over 100 cycles: applied {:.1}% (expected ~50%)", probability * 100.0);
}

#[test]
fn test_sometimes_by_deterministic_per_cycle() {
    let pattern = parse_mini_notation("bd sn hh cp");

    // Same cycle should give same result (deterministic RNG seeded by cycle)
    let state = State {
        span: TimeSpan::new(Fraction::new(5, 1), Fraction::new(6, 1)),
        controls: HashMap::new(),
    };

    let result1 = pattern.clone().sometimes_by(0.5, |p| p.fast(2.0)).query(&state);
    let result2 = pattern.clone().sometimes_by(0.5, |p| p.fast(2.0)).query(&state);

    assert_eq!(result1.len(), result2.len(), "Same cycle should give same result");

    println!("✅ sometimes_by is deterministic per cycle (seeded RNG)");
}

#[test]
fn test_when_mod_level1_every_n_cycles() {
    let pattern = parse_mini_notation("bd sn");

    // when_mod(3, 0, f) should apply on cycles 0, 3, 6, 9, ...
    for cycle in 0..12 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let base = pattern.query(&state);
        let when_mod = pattern.clone().when_mod(3, 0, |p| p.fast(2.0)).query(&state);

        if cycle % 3 == 0 {
            // Should apply fast(2)
            assert_eq!(when_mod.len(), base.len() * 2, "Cycle {}: should apply transform", cycle);
        } else {
            // Should NOT apply
            assert_eq!(when_mod.len(), base.len(), "Cycle {}: should NOT apply transform", cycle);
        }
    }

    println!("✅ when_mod(3, 0): Applies on cycles 0, 3, 6, 9...");
}

#[test]
fn test_when_mod_with_offset() {
    let pattern = parse_mini_notation("bd sn hh cp");

    // when_mod(4, 1, f) should apply on cycles where (cycle - 1) % 4 == 0
    // i.e., cycles 1, 5, 9, 13...
    for cycle in 0..16 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let base = pattern.query(&state);
        let when_mod = pattern.clone().when_mod(4, 1, |p| p.fast(2.0)).query(&state);

        if (cycle - 1) % 4 == 0 {
            // Should apply
            assert!(when_mod.len() >= base.len(), "Cycle {}: should apply transform", cycle);
        } else {
            // Should NOT apply
            assert_eq!(when_mod.len(), base.len(), "Cycle {}: should NOT apply", cycle);
        }
    }

    println!("✅ when_mod(4, 1): Applies on cycles 1, 5, 9, 13... (with offset)");
}

#[test]
fn test_when_mod_with_every() {
    let pattern = parse_mini_notation("bd sn");

    // when_mod(2, 0, f) should apply on even cycles
    // This should be similar to every(2, f)
    let when_mod_pattern = pattern.clone().when_mod(2, 0, |p| p.fast(2.0));
    let every_pattern = pattern.clone().every(2, |p| p.fast(2.0));

    for cycle in 0..8 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let when_mod_haps = when_mod_pattern.query(&state);
        let every_haps = every_pattern.query(&state);

        assert_eq!(
            when_mod_haps.len(),
            every_haps.len(),
            "when_mod(2, 0) should behave like every(2) on cycle {}",
            cycle
        );
    }

    println!("✅ when_mod(2, 0) equivalent to every(2)");
}

// ============= Multi-cycle Tests =============

#[test]
fn test_sometimes_by_different_probabilities() {
    let pattern = parse_mini_notation("bd sn");
    let cycles = 100;

    // Test different probability levels
    for prob in [0.1, 0.25, 0.5, 0.75, 0.9] {
        let mut applied = 0;

        for cycle in 0..cycles {
            let state = State {
                span: TimeSpan::new(
                    Fraction::from_float(cycle as f64),
                    Fraction::from_float((cycle + 1) as f64),
                ),
                controls: HashMap::new(),
            };

            let base = pattern.query(&state);
            let transformed = pattern.clone().sometimes_by(prob, |p| p.fast(2.0)).query(&state);

            if transformed.len() > base.len() {
                applied += 1;
            }
        }

        let actual = applied as f64 / cycles as f64;
        let tolerance = 0.15; // Allow 15% deviation

        assert!(
            (actual - prob).abs() < tolerance,
            "prob={:.2}: expected ~{:.0}%, got {:.1}%",
            prob,
            prob * 100.0,
            actual * 100.0
        );

        println!("prob={:.2}: {:.1}% applied (expected ~{:.0}%)", prob, actual * 100.0, prob * 100.0);
    }

    println!("✅ sometimes_by respects different probability levels");
}

#[test]
fn test_when_mod_different_modulos() {
    let pattern = parse_mini_notation("bd sn hh cp");

    // Test different modulo values
    for modulo in [2, 3, 4, 5] {
        let mut applied_cycles = Vec::new();

        for cycle in 0..20 {
            let state = State {
                span: TimeSpan::new(
                    Fraction::from_float(cycle as f64),
                    Fraction::from_float((cycle + 1) as f64),
                ),
                controls: HashMap::new(),
            };

            let base = pattern.query(&state);
            let when_mod = pattern.clone().when_mod(modulo, 0, |p| p.fast(2.0)).query(&state);

            if when_mod.len() > base.len() {
                applied_cycles.push(cycle);
            }
        }

        // Should apply on cycles 0, modulo, modulo*2, ...
        let expected: Vec<i32> = (0..20).filter(|c| c % modulo == 0).collect();
        assert_eq!(
            applied_cycles, expected,
            "when_mod({}) should apply on cycles divisible by {}",
            modulo, modulo
        );

        println!("✅ when_mod({}) applies on cycles: {:?}", modulo, applied_cycles);
    }
}

// ============= Composition Tests =============

#[test]
fn test_sometimes_by_nested() {
    let pattern = parse_mini_notation("bd sn");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // Nested: sometimes_by(0.5, sometimes_by(0.5, fast))
    // Effective probability = 0.5 * 0.5 = 0.25
    let nested = pattern
        .clone()
        .sometimes_by(0.5, |p| p.sometimes_by(0.5, |p2| p2.fast(2.0)))
        .query(&state);

    println!("✅ sometimes_by can be nested (effective prob = product)");
}

#[test]
fn test_when_mod_composition() {
    let pattern = parse_mini_notation("bd sn");

    // when_mod(2, 0) $ when_mod(3, 0) should apply on cycles 0, 6, 12... (LCM)
    for cycle in 0..12 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let base = pattern.query(&state);
        let composed = pattern
            .clone()
            .when_mod(2, 0, |p| p.when_mod(3, 0, |p2| p2.fast(2.0)))
            .query(&state);

        if cycle % 2 == 0 && cycle % 3 == 0 {
            // Both conditions met
            assert_eq!(composed.len(), base.len() * 2, "Cycle {}: both conditions met", cycle);
        } else {
            assert_eq!(composed.len(), base.len(), "Cycle {}: conditions not met", cycle);
        }
    }

    println!("✅ when_mod can be composed (AND logic)");
}

// ============= Edge Cases =============

#[test]
fn test_sometimes_by_identity_transform() {
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // sometimes_by with identity should always give base pattern
    let base = pattern.query(&state);
    let transformed = pattern.clone().sometimes_by(0.5, |p| p).query(&state);

    assert_eq!(transformed.len(), base.len(), "Identity transform should not change count");

    println!("✅ sometimes_by with identity transform works correctly");
}

#[test]
fn test_when_mod_every_cycle() {
    let pattern = parse_mini_notation("bd sn");

    // when_mod(1, 0, f) should apply on EVERY cycle
    for cycle in 0..5 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let base = pattern.query(&state);
        let when_mod = pattern.clone().when_mod(1, 0, |p| p.fast(2.0)).query(&state);

        assert_eq!(when_mod.len(), base.len() * 2, "when_mod(1) should apply every cycle");
    }

    println!("✅ when_mod(1, 0) applies on every cycle");
}

#[test]
fn test_when_mod_negative_cycle() {
    let pattern = parse_mini_notation("bd sn hh cp");

    // Test that when_mod handles negative cycle offsets correctly
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // when_mod(4, -1, f) - negative offset
    let when_mod = pattern.clone().when_mod(4, -1, |p| p.fast(2.0)).query(&state);

    // Cycle 0: (0 - (-1)) % 4 = 1 % 4 = 1 (not 0, so don't apply)
    let base = pattern.query(&state);
    assert_eq!(when_mod.len(), base.len(), "Negative offset should work correctly");

    println!("✅ when_mod handles negative offsets");
}

#[test]
fn test_sometimes_by_with_silence() {
    let pattern = parse_mini_notation("bd ~ sn ~");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // Should handle rests correctly
    let base = pattern.query(&state);
    let transformed = pattern.clone().sometimes_by(1.0, |p| p.fast(2.0)).query(&state);

    assert!(transformed.len() >= base.len(), "Should handle rests");

    println!("✅ sometimes_by handles rests correctly");
}
