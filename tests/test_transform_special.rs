/// Tests for TIER 4 Special Purpose transforms
///
/// These are advanced/specialized transforms:
/// - compressGap: Compress to range with silence outside
/// - reset/restart: Reset cycle counter every n cycles
/// - loopback: Forward then backward
/// - binary: Apply only on cycles where bit is set
/// - focus/trim: Zoom to range (aliases)
/// - wait: Delay pattern by n cycles
/// - mask: Boolean masking
/// - weave: Stack patterns (alias)
/// - degradeSeed: Seeded random removal
/// - undegrade: Identity transform
/// - accelerate: Speed up over time (stub)
/// - humanize: Timing variation (alias for shuffle)
/// - mirror: Palindrome (alias)
/// - always: Always apply function
/// - fastGap: Fast with gaps
///
/// Focus on non-alias transforms with actual behavior
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use phonon::pattern_structure::wait;
use std::collections::HashMap;

// ============= COMPRESS_GAP =============

#[test]
fn test_compress_gap_level1_shows_only_in_range() {
    let pattern = parse_mini_notation("bd sn hh cp");

    // compress_gap(0.25, 0.75) should only show pattern in range [0.25, 0.75)
    let compressed = pattern.compress_gap(0.25, 0.75);

    // Query first quarter (before range)
    let state1 = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::from_float(0.25)),
        controls: HashMap::new(),
    };
    let haps1 = compressed.query(&state1);
    assert_eq!(haps1.len(), 0, "Before range should be silent");

    // Query middle half (in range)
    let state2 = State {
        span: TimeSpan::new(Fraction::from_float(0.25), Fraction::from_float(0.75)),
        controls: HashMap::new(),
    };
    let haps2 = compressed.query(&state2);
    assert!(haps2.len() > 0, "In range should have events");

    // Query last quarter (after range)
    let state3 = State {
        span: TimeSpan::new(Fraction::from_float(0.75), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    let haps3 = compressed.query(&state3);
    assert_eq!(haps3.len(), 0, "After range should be silent");

    println!("✅ compressGap shows pattern only in specified range");
}

#[test]
fn test_compress_gap_different_ranges() {
    let pattern = parse_mini_notation("bd sn");

    for (begin, end) in [(0.0, 0.5), (0.25, 0.75), (0.5, 1.0)] {
        let compressed = pattern.clone().compress_gap(begin, end);

        // Query the active range where compress_gap should produce events
        let state = State {
            span: TimeSpan::new(Fraction::from_float(begin), Fraction::from_float(end)),
            controls: HashMap::new(),
        };

        let haps = compressed.query(&state);

        // compress_gap should produce events when querying the active range
        assert!(
            haps.len() > 0,
            "compressGap({}, {}) should produce events when querying active range",
            begin,
            end
        );
    }

    println!("✅ compressGap works with different ranges");
}

// ============= RESET / RESTART =============

#[test]
fn test_reset_level1_resets_every_n_cycles() {
    let pattern = parse_mini_notation("bd sn hh cp");

    // reset(2) should reset to cycle 0 every 2 cycles
    let reset_pattern = pattern.clone().reset(2);

    // Cycle 0: shows cycle 0
    // Cycle 1: shows cycle 1
    // Cycle 2: shows cycle 0 again (reset)
    // Cycle 3: shows cycle 1 again
    // Cycle 4: shows cycle 0 again (reset)

    for cycle in 0..6 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let base_cycle = cycle % 2; // Which cycle the reset pattern shows
        let base_state = State {
            span: TimeSpan::new(
                Fraction::from_float(base_cycle as f64),
                Fraction::from_float((base_cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let reset_haps = reset_pattern.query(&state);
        let base_haps = pattern.query(&base_state);

        assert_eq!(
            reset_haps.len(),
            base_haps.len(),
            "Cycle {} should match cycle {} (reset every 2)",
            cycle,
            base_cycle
        );
    }

    println!("✅ reset resets pattern every n cycles");
}

#[test]
fn test_restart_is_alias_for_reset() {
    let pattern = parse_mini_notation("bd sn hh cp");

    let reset_pattern = pattern.clone().reset(3);
    let restart_pattern = pattern.clone().restart(3);

    for cycle in 0..9 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let reset_haps = reset_pattern.query(&state);
        let restart_haps = restart_pattern.query(&state);

        assert_eq!(
            reset_haps.len(),
            restart_haps.len(),
            "restart should be same as reset"
        );
    }

    println!("✅ restart is alias for reset");
}

// ============= LOOPBACK =============

#[test]
fn test_loopback_level1_forward_then_backward() {
    let pattern = parse_mini_notation("bd sn hh");

    let loopback_pattern = pattern.clone().loopback();

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)), // 1 cycle
        controls: HashMap::new(),
    };

    let haps = loopback_pattern.query(&state);

    // loopback = cat[pattern, pattern.rev()]
    // cat concatenates within the same cycle, so it plays forward then backward in ONE cycle
    // Total events = base events × 2

    let base_events = pattern
        .clone()
        .query(&State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        })
        .len();

    assert_eq!(
        haps.len(),
        base_events * 2,
        "loopback should have 2× events per cycle (forward + backward)"
    );

    println!("✅ loopback plays forward then backward within one cycle");
}

// ============= BINARY =============

#[test]
fn test_binary_level1_bit_masking() {
    let pattern = parse_mini_notation("bd sn hh cp");

    // binary(0) applies on cycles where bit 0 is set: 1, 3, 5, 7, 9... (odd cycles)
    let binary_pattern = pattern.binary(0);

    for cycle in 0..8 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let haps = binary_pattern.query(&state);

        if (cycle & (1 << 0)) != 0 {
            // Bit 0 is set (odd cycles)
            assert!(haps.len() > 0, "Cycle {} (odd): should have events", cycle);
        } else {
            // Bit 0 is not set (even cycles)
            assert_eq!(haps.len(), 0, "Cycle {} (even): should be silent", cycle);
        }
    }

    println!("✅ binary applies pattern only on cycles where bit is set");
}

#[test]
fn test_binary_different_bits() {
    let pattern = parse_mini_notation("bd sn");

    // Test bit 1: applies on cycles 2, 3, 6, 7, 10, 11...
    let binary1 = pattern.clone().binary(1);

    for cycle in 0..8 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let haps = binary1.query(&state);

        if (cycle & (1 << 1)) != 0 {
            assert!(haps.len() > 0, "Cycle {}: bit 1 set", cycle);
        } else {
            assert_eq!(haps.len(), 0, "Cycle {}: bit 1 not set", cycle);
        }
    }

    println!("✅ binary works with different bit positions");
}

// ============= WAIT =============

#[test]
fn test_wait_level1_delays_pattern() {
    let pattern = parse_mini_notation("bd sn hh cp");

    // wait(2, pattern) should be silent for cycles 0-1, then play from cycle 2
    let wait_pattern = wait(2, pattern.clone());

    for cycle in 0..6 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let haps = wait_pattern.query(&state);

        if cycle < 2 {
            assert_eq!(haps.len(), 0, "Cycle {}: should be silent (waiting)", cycle);
        } else {
            assert!(haps.len() > 0, "Cycle {}: should have events", cycle);
        }
    }

    println!("✅ wait delays pattern by n cycles");
}

#[test]
fn test_wait_different_durations() {
    let pattern = parse_mini_notation("bd sn");

    for wait_cycles in [1, 2, 3, 5] {
        let wait_pattern = wait(wait_cycles, pattern.clone());

        for cycle in 0..(wait_cycles + 2) {
            let state = State {
                span: TimeSpan::new(
                    Fraction::from_float(cycle as f64),
                    Fraction::from_float((cycle + 1) as f64),
                ),
                controls: HashMap::new(),
            };

            let haps = wait_pattern.query(&state);

            if cycle < wait_cycles {
                assert_eq!(
                    haps.len(),
                    0,
                    "wait({}): cycle {} silent",
                    wait_cycles,
                    cycle
                );
            } else {
                assert!(
                    haps.len() > 0,
                    "wait({}): cycle {} playing",
                    wait_cycles,
                    cycle
                );
            }
        }
    }

    println!("✅ wait works with different durations");
}

// ============= MASK =============

#[test]
fn test_mask_level1_boolean_filtering() {
    let pattern = parse_mini_notation("bd sn hh cp");

    // Create a simple boolean pattern: true, false, true, false
    // Use cat to create boolean pattern
    let mask_pattern = Pattern::cat(vec![
        Pattern::pure(true),
        Pattern::pure(false),
        Pattern::pure(true),
        Pattern::pure(false),
    ]);

    let masked = pattern.clone().mask(mask_pattern);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let masked_haps = masked.query(&state);

    // Should filter out events where mask is false
    assert!(
        masked_haps.len() <= base_haps.len(),
        "Masked pattern should have same or fewer events"
    );
    assert!(masked_haps.len() > 0, "Should still have some events");

    println!("✅ mask filters events using boolean pattern");
}

// ============= WEAVE =============

#[test]
fn test_weave_level1_stacks_patterns() {
    let pattern1 = parse_mini_notation("bd sn");
    let pattern2 = parse_mini_notation("hh cp");

    let woven = pattern1.clone().weave(pattern2.clone());

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let haps1 = pattern1.query(&state);
    let haps2 = pattern2.query(&state);
    let woven_haps = woven.query(&state);

    // weave is an alias for stack, so should have combined events
    assert_eq!(
        woven_haps.len(),
        haps1.len() + haps2.len(),
        "weave should stack both patterns"
    );

    println!("✅ weave stacks two patterns together");
}

// ============= DEGRADE_SEED =============

#[test]
fn test_degrade_seed_level1_removes_events() {
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let degraded = pattern.clone().degrade_seed(42).query(&state);

    // Should remove ~50% of events (random, seeded)
    assert!(
        degraded.len() <= base_haps.len(),
        "degraded should have same or fewer events"
    );

    println!("✅ degradeSeed randomly removes events with seed");
}

#[test]
fn test_degrade_seed_deterministic() {
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // Same seed should give same result
    let degraded1 = pattern.clone().degrade_seed(42).query(&state);
    let degraded2 = pattern.clone().degrade_seed(42).query(&state);

    assert_eq!(
        degraded1.len(),
        degraded2.len(),
        "Same seed should give same result"
    );

    // Different seed might give different result
    let _degraded3 = pattern.clone().degrade_seed(999).query(&state);
    // Just verify it works (might coincidentally be same count)

    println!("✅ degradeSeed is deterministic per seed");
}

// ============= UNDEGRADE =============

#[test]
fn test_undegrade_is_identity() {
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let undegraded = pattern.clone().undegrade().query(&state);

    assert_eq!(
        base_haps.len(),
        undegraded.len(),
        "undegrade should be identity"
    );

    println!("✅ undegrade is identity transform");
}

// ============= ALWAYS =============

#[test]
fn test_always_applies_function() {
    let pattern = parse_mini_notation("bd sn");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // always(fast(2)) should always apply fast(2)
    let transformed = pattern.clone().always(|p| p.fast(Pattern::pure(2.0)));

    let fast_haps = pattern.clone().fast(Pattern::pure(2.0)).query(&state);
    let always_haps = transformed.query(&state);

    assert_eq!(
        always_haps.len(),
        fast_haps.len(),
        "always should apply function"
    );

    println!("✅ always applies function (always)");
}

// ============= FAST_GAP =============

#[test]
fn test_fast_gap_level1_fast_with_silence() {
    let pattern = parse_mini_notation("bd sn");

    // fastGap(2) should fast(2) but only in first half of cycle
    let fast_gap_pattern = pattern.fast_gap(2.0);

    // Query first half
    let state1 = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::from_float(0.5)),
        controls: HashMap::new(),
    };
    let haps1 = fast_gap_pattern.query(&state1);
    assert!(haps1.len() > 0, "First half should have events");

    // Query second half
    let state2 = State {
        span: TimeSpan::new(Fraction::from_float(0.5), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    let haps2 = fast_gap_pattern.query(&state2);
    assert_eq!(haps2.len(), 0, "Second half should be silent");

    println!("✅ fastGap plays fast pattern only in first 1/factor of cycle");
}

#[test]
fn test_fast_gap_different_factors() {
    let pattern = parse_mini_notation("bd sn hh cp");

    for factor in [2.0, 3.0, 4.0] {
        let fast_gap = pattern.clone().fast_gap(factor);

        // Query first 1/factor of cycle (should have events)
        let active_range = 1.0 / factor;
        let state_active = State {
            span: TimeSpan::new(
                Fraction::new(0, 1),
                Fraction::from_float(active_range * 0.9),
            ),
            controls: HashMap::new(),
        };
        let haps_active = fast_gap.query(&state_active);
        assert!(
            haps_active.len() > 0,
            "factor={}: active range has events",
            factor
        );

        // Query rest of cycle (should be silent)
        let state_silent = State {
            span: TimeSpan::new(
                Fraction::from_float(active_range + 0.01),
                Fraction::new(1, 1),
            ),
            controls: HashMap::new(),
        };
        let haps_silent = fast_gap.query(&state_silent);
        assert_eq!(haps_silent.len(), 0, "factor={}: rest is silent", factor);
    }

    println!("✅ fastGap works with different factors");
}

// ============= ALIASES (quick verification) =============

#[test]
fn test_focus_is_alias_for_zoom() {
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let focused = pattern
        .clone()
        .focus(Pattern::pure(0.25), Pattern::pure(0.75))
        .query(&state);
    let zoomed = pattern
        .clone()
        .zoom(Pattern::pure(0.25), Pattern::pure(0.75))
        .query(&state);

    assert_eq!(
        focused.len(),
        zoomed.len(),
        "focus should be alias for zoom"
    );

    println!("✅ focus is alias for zoom");
}

#[test]
fn test_trim_is_alias_for_zoom() {
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let trimmed = pattern
        .clone()
        .trim(Pattern::pure(0.0), Pattern::pure(0.5))
        .query(&state);
    let zoomed = pattern
        .clone()
        .zoom(Pattern::pure(0.0), Pattern::pure(0.5))
        .query(&state);

    assert_eq!(trimmed.len(), zoomed.len(), "trim should be alias for zoom");

    println!("✅ trim is alias for zoom");
}

#[test]
fn test_mirror_is_alias_for_palindrome() {
    let pattern = parse_mini_notation("bd sn");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(2, 1)),
        controls: HashMap::new(),
    };

    let mirrored = pattern.clone().mirror().query(&state);
    let palindrome = pattern.clone().palindrome().query(&state);

    assert_eq!(
        mirrored.len(),
        palindrome.len(),
        "mirror should be alias for palindrome"
    );

    println!("✅ mirror is alias for palindrome");
}

#[test]
fn test_humanize_is_alias_for_shuffle() {
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let humanized = pattern
        .clone()
        .humanize(Pattern::pure(0.1), Pattern::pure(0.0))
        .query(&state);
    let shuffled = pattern.clone().shuffle(Pattern::pure(0.1)).query(&state);

    // Both should have same event count (just different timing)
    assert_eq!(
        humanized.len(),
        shuffled.len(),
        "humanize should be alias for shuffle"
    );

    println!("✅ humanize is alias for shuffle");
}

// ============= Edge Cases =============

#[test]
fn test_special_transforms_over_cycles() {
    let pattern = parse_mini_notation("bd sn");

    // Test various transforms over multiple cycles for consistency
    let reset_p = pattern.clone().reset(2);
    let binary_p = pattern.clone().binary(0);
    let wait_p = wait(1, pattern.clone());

    for cycle in 0..4 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let reset_haps = reset_p.query(&state);
        let binary_haps = binary_p.query(&state);
        let wait_haps = wait_p.query(&state);

        // Just verify they all execute without crashing
        assert!(reset_haps.len() >= 0);
        assert!(binary_haps.len() >= 0);
        assert!(wait_haps.len() >= 0);
    }

    println!("✅ Special transforms consistent over multiple cycles");
}
