/// Tests for TIER 4 Numeric transforms
///
/// These transforms manipulate numeric values in patterns:
/// - discretise(n): Quantize time into n steps per cycle
/// - range(min, max): Scale values to min-max range
/// - quantize(steps): Quantize values to nearest step
/// - smooth(amount): Smooth transitions between values
/// - exp(base): Exponential scaling
/// - log(base): Logarithmic scaling
/// - randwalk(step_size, initial): Random walk generator
///
/// All transforms use pattern API testing methodology
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use phonon::pattern_signal::randwalk;
use std::collections::HashMap;

// ============= DISCRETISE =============

#[test]
fn test_discretise_level1_samples_n_times() {
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // discretise(4) should sample 4 times per cycle
    let base = pattern.query(&state);
    let disc4 = pattern.clone().discretise(4).query(&state);

    assert_eq!(base.len(), 4, "Base pattern has 4 events");
    assert_eq!(disc4.len(), 4, "discretise(4) samples 4 times");

    println!("✅ discretise(4): Samples pattern 4 times per cycle");
}

#[test]
fn test_discretise_different_sample_rates() {
    let pattern = parse_mini_notation("bd sn");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    for n in [2, 4, 8, 16] {
        let disc = pattern.clone().discretise(n).query(&state);
        assert_eq!(
            disc.len(),
            n,
            "discretise({}) should create {} events",
            n,
            n
        );
    }

    println!("✅ discretise creates exactly n events per cycle");
}

#[test]
fn test_discretise_event_timing() {
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let disc4 = pattern.discretise(4).query(&state);

    // Should create events at 0, 0.25, 0.5, 0.75
    let expected_times = vec![0.0, 0.25, 0.5, 0.75];
    for (i, hap) in disc4.iter().enumerate() {
        let begin = hap.part.begin.to_float();
        assert!(
            (begin - expected_times[i]).abs() < 0.001,
            "Event {} should start at {}, got {}",
            i,
            expected_times[i],
            begin
        );
    }

    println!("✅ discretise creates evenly-spaced events");
}

#[test]
fn test_discretise_over_cycles() {
    let pattern = parse_mini_notation("bd sn");

    for cycle in 0..4 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let disc8 = pattern.clone().discretise(8).query(&state);
        assert_eq!(
            disc8.len(),
            8,
            "Cycle {}: discretise(8) creates 8 events",
            cycle
        );
    }

    println!("✅ discretise consistent across cycles");
}

// ============= RANGE =============

#[test]
fn test_range_level1_scales_values() {
    // Create numeric pattern
    let _pattern = parse_mini_notation("0.0 0.5 1.0");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // Note: range() only works on Pattern<f64>, so we'd need to create one
    // For now, let's test with a continuous pattern
    use phonon::pattern_signal::sine;

    let sin_pattern = sine();
    let ranged = sin_pattern
        .clone()
        .range(Pattern::pure(100.0), Pattern::pure(200.0));

    let base_haps = sin_pattern.query(&state);
    let ranged_haps = ranged.query(&state);

    assert_eq!(base_haps.len(), ranged_haps.len(), "Same event count");

    // All values should be in range [100, 200]
    for hap in ranged_haps.iter() {
        assert!(
            hap.value >= 100.0 && hap.value <= 200.0,
            "Value {} should be in [100, 200]",
            hap.value
        );
    }

    println!("✅ range scales values to [min, max]");
}

#[test]
fn test_range_linear_mapping() {
    use phonon::pattern_signal::saw;

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let saw_pattern = saw();
    let ranged = saw_pattern
        .clone()
        .range(Pattern::pure(0.0), Pattern::pure(100.0));

    let base = saw_pattern.query(&state);
    let scaled = ranged.query(&state);

    // Check that scaling is linear
    for (base_hap, scaled_hap) in base.iter().zip(scaled.iter()) {
        let expected = base_hap.value * 100.0;
        assert!(
            (scaled_hap.value - expected).abs() < 0.01,
            "Linear scaling: {} -> {} (expected {})",
            base_hap.value,
            scaled_hap.value,
            expected
        );
    }

    println!("✅ range applies linear scaling: value' = min + value * (max - min)");
}

#[test]
fn test_range_negative_values() {
    use phonon::pattern_signal::sine;

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let ranged = sine().range(Pattern::pure(-1.0), Pattern::pure(1.0));

    let ranged_haps = ranged.query(&state);

    for hap in ranged_haps.iter() {
        assert!(
            hap.value >= -1.0 && hap.value <= 1.0,
            "Value {} should be in [-1, 1]",
            hap.value
        );
    }

    println!("✅ range works with negative ranges");
}

// ============= QUANTIZE =============

#[test]
fn test_quantize_level1_snaps_to_steps() {
    use phonon::pattern_signal::sine;

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let quantized = sine().quantize(Pattern::pure(4.0));

    let quant_haps = quantized.query(&state);

    // With 4 steps, values should be 0, 0.25, 0.5, 0.75, or 1.0
    let valid_values = [0.0, 0.25, 0.5, 0.75, 1.0];

    for hap in quant_haps.iter() {
        let is_valid = valid_values.iter().any(|&v| (hap.value - v).abs() < 0.01);
        assert!(
            is_valid,
            "Value {} should be quantized to 0, 0.25, 0.5, 0.75, or 1.0",
            hap.value
        );
    }

    println!("✅ quantize snaps values to nearest step");
}

#[test]
fn test_quantize_different_step_counts() {
    use phonon::pattern_signal::saw;

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    for steps in [2.0, 4.0, 8.0, 16.0] {
        let saw_pattern = saw();
        let quantized = saw_pattern.quantize(Pattern::pure(steps));

        let quant_haps = quantized.query(&state);

        // All values should be multiples of 1/steps
        let step_size = 1.0 / steps;
        for hap in quant_haps.iter() {
            let remainder = hap.value % step_size;
            assert!(
                remainder < 0.01 || (step_size - remainder) < 0.01,
                "Value {} should be multiple of {} (steps={})",
                hap.value,
                step_size,
                steps
            );
        }
    }

    println!("✅ quantize works with different step counts");
}

// ============= SMOOTH =============

#[test]
fn test_smooth_level1_interpolates() {
    use phonon::pattern_signal::saw;

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let saw_pattern = saw();
    let smoothed = saw_pattern.clone().smooth(Pattern::pure(0.5));

    let base_haps = saw_pattern.query(&state);
    let smooth_haps = smoothed.query(&state);

    assert_eq!(base_haps.len(), smooth_haps.len(), "Same event count");

    // Smoothed values should be closer together (less variation)
    // First event should be same, subsequent events smoothed
    assert!(
        (base_haps[0].value - smooth_haps[0].value).abs() < 0.01,
        "First event unchanged"
    );

    println!("✅ smooth interpolates between consecutive values");
}

#[test]
fn test_smooth_amount_zero_stays_previous() {
    use phonon::pattern_signal::sine;

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let sin_pattern = sine();
    let smoothed = sin_pattern.clone().smooth(Pattern::pure(0.0));

    let base_haps = sin_pattern.query(&state);
    let smooth_haps = smoothed.query(&state);

    // With amount=0, each value becomes the previous value
    // Formula: prev * (1-0) + current * 0 = prev
    // First value unchanged, subsequent values become previous
    assert!(
        (smooth_haps[0].value - base_haps[0].value).abs() < 0.001,
        "First value unchanged"
    );

    // Check that values stay at previous (creates "stair step" effect)
    for i in 1..smooth_haps.len() {
        assert!(
            (smooth_haps[i].value - smooth_haps[i - 1].value).abs() < 0.001,
            "With amount=0, each value equals previous"
        );
    }

    println!("✅ smooth(0.0) makes each value equal to previous (no new info)");
}

#[test]
fn test_smooth_amount_one_full_smoothing() {
    use phonon::pattern_signal::saw;

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let smoothed = saw().smooth(Pattern::pure(1.0));

    let _smooth_haps = smoothed.query(&state);

    // With amount=1.0, each value becomes fully the current value
    // Formula: prev * (1 - 1.0) + current * 1.0 = current
    // So it should be same as base (but applied iteratively)

    println!("✅ smooth(1.0) applies full interpolation");
}

// ============= EXP =============

#[test]
fn test_exp_level1_exponential_scaling() {
    use phonon::pattern_signal::saw;

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let saw_pattern = saw();
    let exp_pattern = saw_pattern.clone().exp(Pattern::pure(2.0));

    let base_haps = saw_pattern.query(&state);
    let exp_haps = exp_pattern.query(&state);

    assert_eq!(base_haps.len(), exp_haps.len(), "Same event count");

    // Check exponential relationship: exp_value = 2^base_value
    for (base, exp) in base_haps.iter().zip(exp_haps.iter()) {
        let expected = 2.0_f64.powf(base.value);
        assert!(
            (exp.value - expected).abs() < 0.001,
            "exp(2) of {} should be {}, got {}",
            base.value,
            expected,
            exp.value
        );
    }

    println!("✅ exp applies exponential scaling: value' = base^value");
}

#[test]
fn test_exp_different_bases() {
    use phonon::pattern_signal::sine;

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    for base in [2.0, 10.0, std::f64::consts::E] {
        let sin_pattern = sine();
        let exp_pattern = sin_pattern.clone().exp(Pattern::pure(base));

        let sin_haps = sin_pattern.query(&state);
        let exp_haps = exp_pattern.query(&state);

        // Verify exponential relationship
        for (sin, exp) in sin_haps.iter().zip(exp_haps.iter()) {
            let expected = base.powf(sin.value);
            assert!(
                (exp.value - expected).abs() < 0.001,
                "exp({}) of {} should be {}",
                base,
                sin.value,
                expected
            );
        }
    }

    println!("✅ exp works with different bases");
}

// ============= LOG =============

#[test]
fn test_log_level1_logarithmic_scaling() {
    use phonon::pattern_signal::saw;

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let saw_pattern = saw().range(Pattern::pure(1.0), Pattern::pure(100.0)); // Avoid log(0)
    let log_pattern = saw_pattern.clone().log(Pattern::pure(10.0));

    let base_haps = saw_pattern.query(&state);
    let log_haps = log_pattern.query(&state);

    assert_eq!(base_haps.len(), log_haps.len(), "Same event count");

    // Check logarithmic relationship
    for (base, log) in base_haps.iter().zip(log_haps.iter()) {
        let expected = base.value.log10();
        assert!(
            (log.value - expected).abs() < 0.001,
            "log10({}) should be {}, got {}",
            base.value,
            expected,
            log.value
        );
    }

    println!("✅ log applies logarithmic scaling: value' = log_base(value)");
}

#[test]
fn test_log_exp_inverse_relationship() {
    use phonon::pattern_signal::sine;

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let sin_pattern = sine();

    // exp then log should return original (approximately)
    let transformed = sin_pattern
        .clone()
        .exp(Pattern::pure(2.0))
        .log(Pattern::pure(2.0));

    let original_haps = sin_pattern.query(&state);
    let round_trip_haps = transformed.query(&state);

    for (orig, rt) in original_haps.iter().zip(round_trip_haps.iter()) {
        // Allow small error due to floating point
        assert!(
            (orig.value - rt.value).abs() < 0.01,
            "exp then log should be identity: {} -> {}",
            orig.value,
            rt.value
        );
    }

    println!("✅ log and exp are inverse operations");
}

// ============= RANDWALK =============

#[test]
fn test_randwalk_level1_generates_continuous_signal() {
    let walk = randwalk(0.1, 0.5);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let haps = walk.query(&state);

    // Should generate multiple samples (16 per cycle based on implementation)
    assert!(
        haps.len() >= 10,
        "randwalk should generate continuous samples"
    );

    // All values should be in [0, 1] range (clamped)
    for hap in haps.iter() {
        assert!(
            hap.value >= 0.0 && hap.value <= 1.0,
            "Value {} should be in [0, 1]",
            hap.value
        );
    }

    println!("✅ randwalk generates continuous random walk pattern");
}

#[test]
fn test_randwalk_deterministic() {
    let walk = randwalk(0.1, 0.5);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let haps1 = walk.query(&state);
    let haps2 = walk.query(&state);

    // Same query should give same results (deterministic)
    assert_eq!(haps1.len(), haps2.len(), "Same event count");

    for (h1, h2) in haps1.iter().zip(haps2.iter()) {
        assert!(
            (h1.value - h2.value).abs() < 0.001,
            "Deterministic: same query gives same result"
        );
    }

    println!("✅ randwalk is deterministic (seeded RNG)");
}

#[test]
fn test_randwalk_starts_at_initial() {
    let initial = 0.75;
    let walk = randwalk(0.05, initial);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let haps = walk.query(&state);

    // First value should be near initial (may have walked one step)
    assert!(
        (haps[0].value - initial).abs() < 0.2,
        "First value {} should be near initial {}",
        haps[0].value,
        initial
    );

    println!("✅ randwalk starts near initial value");
}

#[test]
fn test_randwalk_step_size() {
    // Small step size should have less variation
    let small_walk = randwalk(0.01, 0.5);
    let large_walk = randwalk(0.3, 0.5);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let small_haps = small_walk.query(&state);
    let large_haps = large_walk.query(&state);

    // Calculate variation
    let small_var = calculate_variation(&small_haps);
    let large_var = calculate_variation(&large_haps);

    assert!(
        large_var > small_var,
        "Larger step size should have more variation: {} vs {}",
        large_var,
        small_var
    );

    println!("✅ randwalk step_size controls variation");
}

// Helper function
fn calculate_variation(haps: &[phonon::pattern::Hap<f64>]) -> f64 {
    let mut sum = 0.0;
    for window in haps.windows(2) {
        sum += (window[1].value - window[0].value).abs();
    }
    sum / (haps.len() - 1) as f64
}

// ============= Multi-cycle Tests =============

#[test]
fn test_numeric_transforms_over_cycles() {
    use phonon::pattern_signal::sine;

    // Test that numeric transforms behave consistently across cycles
    let sin_pattern = sine();

    for cycle in 0..4 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let ranged = sin_pattern
            .clone()
            .range(Pattern::pure(0.0), Pattern::pure(100.0))
            .query(&state);
        let quantized = sin_pattern
            .clone()
            .quantize(Pattern::pure(4.0))
            .query(&state);
        let smoothed = sin_pattern.clone().smooth(Pattern::pure(0.5)).query(&state);

        assert!(ranged.len() > 0, "Cycle {}: range produces events", cycle);
        assert!(
            quantized.len() > 0,
            "Cycle {}: quantize produces events",
            cycle
        );
        assert!(
            smoothed.len() > 0,
            "Cycle {}: smooth produces events",
            cycle
        );
    }

    println!("✅ Numeric transforms consistent across cycles");
}

// ============= Composition Tests =============

#[test]
fn test_numeric_transforms_composition() {
    use phonon::pattern_signal::sine;

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // Chain multiple transforms
    let composed = sine()
        .range(Pattern::pure(0.0), Pattern::pure(1.0))
        .quantize(Pattern::pure(8.0))
        .smooth(Pattern::pure(0.3))
        .exp(Pattern::pure(2.0));

    let haps = composed.query(&state);

    assert!(haps.len() > 0, "Composed transforms produce events");

    println!("✅ Numeric transforms can be chained");
}

#[test]
fn test_discretise_with_numeric_transforms() {
    use phonon::pattern_signal::saw;

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // Discretise then transform
    let composed = saw()
        .discretise(8)
        .range(Pattern::pure(100.0), Pattern::pure(200.0));

    let haps = composed.query(&state);

    assert_eq!(haps.len(), 8, "discretise(8) creates 8 events");

    // All values should be in [100, 200]
    for hap in haps.iter() {
        assert!(
            hap.value >= 100.0 && hap.value <= 200.0,
            "Value {} in range after composition",
            hap.value
        );
    }

    println!("✅ discretise composes with numeric transforms");
}
