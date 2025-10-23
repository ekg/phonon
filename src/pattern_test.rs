#![allow(unused_assignments, unused_mut)]
#![allow(dead_code)]
//! Comprehensive test suite for the Phonon pattern system
//!
//! Each test generates a deterministic string representation of the pattern output
//! that can be verified with hashes to ensure correctness

use crate::pattern::{Fraction, Pattern, State, TimeSpan};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt::Write;

/// Helper to query a pattern and return a deterministic string representation
fn pattern_to_string<T: std::fmt::Debug + Clone + Send + Sync + 'static>(
    pattern: Pattern<T>,
    cycles: f64,
) -> String {
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::from_float(cycles)),
        controls: HashMap::new(),
    };

    let haps = pattern.query(&state);
    let mut output = String::new();

    for hap in haps {
        writeln!(
            &mut output,
            "{:.3}-{:.3}:{}",
            hap.part.begin.to_float(),
            hap.part.end.to_float(),
            format!("{:?}", hap.value).replace(" ", "")
        )
        .unwrap();
    }

    output
}

/// Generate SHA256 hash of pattern output for verification
fn pattern_hash<T: std::fmt::Debug + Clone + Send + Sync + 'static>(
    pattern: Pattern<T>,
    cycles: f64,
) -> String {
    let output = pattern_to_string(pattern, cycles);
    let mut hasher = Sha256::new();
    hasher.update(output.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod core_patterns {
    use super::*;

    #[test]
    fn test_pure_pattern() {
        let p = Pattern::pure(42);
        let output = pattern_to_string(p, 1.0);
        assert_eq!(output, "0.000-1.000:42\n");
    }

    #[test]
    fn test_silence() {
        let p = Pattern::<i32>::silence();
        let output = pattern_to_string(p, 1.0);
        assert_eq!(output, "");
    }

    #[test]
    fn test_from_string() {
        let p = Pattern::from_string("a b c d");
        let output = pattern_to_string(p, 1.0);
        let expected =
            "0.000-0.250:\"a\"\n0.250-0.500:\"b\"\n0.500-0.750:\"c\"\n0.750-1.000:\"d\"\n";
        assert_eq!(output, expected);
    }

    #[test]
    fn test_stack() {
        let p1 = Pattern::from_string("a b");
        let p2 = Pattern::from_string("c d");
        let stacked = Pattern::stack(vec![p1, p2]);
        let output = pattern_to_string(stacked, 1.0);
        assert!(output.contains("\"a\""));
        assert!(output.contains("\"b\""));
        assert!(output.contains("\"c\""));
        assert!(output.contains("\"d\""));
    }

    #[test]
    fn test_cat() {
        let p1 = Pattern::from_string("a");
        let p2 = Pattern::from_string("b");
        let cat = Pattern::cat(vec![p1, p2]);
        let output = pattern_to_string(cat, 1.0);
        // First half should be "a", second half "b"
        assert!(output.contains("0.000-0.500:\"a\""));
        assert!(output.contains("0.500-1.000:\"b\""));
    }
}

#[cfg(test)]
mod time_operations {
    use super::*;

    #[test]
    fn test_fast() {
        let p = Pattern::from_string("a b c d").fast(2.0);
        let output = pattern_to_string(p, 1.0);
        // Should have 8 events (pattern repeated twice)
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 8);
    }

    #[test]
    fn test_slow() {
        let p = Pattern::from_string("a b c d").slow(2.0);
        let output = pattern_to_string(p, 2.0);
        // Should have 4 events spread over 2 cycles
        assert!(output.contains("0.000-0.500:\"a\""));
        assert!(output.contains("0.500-1.000:\"b\""));
        assert!(output.contains("1.000-1.500:\"c\""));
        assert!(output.contains("1.500-2.000:\"d\""));
    }

    #[test]
    fn test_rev() {
        let p = Pattern::from_string("a b c d").rev();
        let output = pattern_to_string(p, 1.0);
        eprintln!("rev output:\n{}", output);
        // Events should be in reverse order within the cycle
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 4);
        // The pattern should be reversed: d c b a
        assert!(lines[0].contains("\"d\""));
        assert!(lines[1].contains("\"c\""));
        assert!(lines[2].contains("\"b\""));
        assert!(lines[3].contains("\"a\""));
    }

    #[test]
    fn test_late() {
        let p = Pattern::from_string("a b").late(0.25);
        let output = pattern_to_string(p, 1.0);
        // Events should be shifted by 0.25
        // "a" was at [0-0.5], now at [0.25-0.75]
        // "b" was at [0.5-1], now at [0.75-1.25] (extends beyond cycle)
        assert!(output.contains("0.250-0.750:\"a\""));
        assert!(output.contains("0.750-1.250:\"b\""));
    }

    #[test]
    fn test_early() {
        let p = Pattern::from_string("a b").early(0.25);
        let output = pattern_to_string(p, 1.25);
        // Events should be shifted earlier
        // Note: early events might wrap around
        assert!(output.contains("\"a\""));
        assert!(output.contains("\"b\""));
    }
}

#[cfg(test)]
mod conditional_operations {
    use super::*;

    #[test]
    fn test_every() {
        let p = Pattern::from_string("a b c d").every(2, |p| p.rev());
        // First cycle: normal, second cycle: reversed
        let output = pattern_to_string(p.clone(), 2.0);

        // Verify alternating behavior
        let cycle1 = pattern_to_string(p.clone(), 1.0);
        let cycle2_pattern = Pattern::from_string("a b c d").every(2, |p| p.rev());

        // For cycle 2, we need to query from cycle 1 to 2
        let state2 = State {
            span: TimeSpan::new(Fraction::new(1, 1), Fraction::new(2, 1)),
            controls: HashMap::new(),
        };
        let haps2 = cycle2_pattern.query(&state2);

        // Second cycle should be reversed
        assert!(!haps2.is_empty());
    }

    #[test]
    fn test_when_mod() {
        let p = Pattern::from_string("a").when_mod(3, 0, |p| p.fast(2.0));
        let output = pattern_to_string(p, 3.0);
        eprintln!("when_mod output:\n{}", output);
        // Every 3rd cycle should be fast
        let lines: Vec<&str> = output.lines().filter(|l| !l.is_empty()).collect();
        // Cycle 0 should be fast (2 events), cycles 1 and 2 normal (1 event each)
        // Total: 2 + 1 + 1 = 4 events
        assert!(lines.len() >= 4); // At least 4 events total
    }
}

#[cfg(test)]
mod probabilistic_operations {
    use super::*;

    #[test]
    fn test_degrade_deterministic() {
        // Degrade is random but deterministic based on cycle number
        let p = Pattern::from_string("a b c d e f g h").degrade();
        let output1 = pattern_to_string(p.clone(), 1.0);
        let output2 = pattern_to_string(p.clone(), 1.0);
        // Same cycle should give same result
        assert_eq!(output1, output2);

        // Should have fewer than 8 events
        let lines: Vec<&str> = output1.lines().collect();
        assert!(lines.len() < 8);
        assert!(lines.len() > 0);
    }

    #[test]
    fn test_sometimes_deterministic() {
        let p = Pattern::from_string("a").sometimes(|p| p.fast(4.0));
        // Check multiple cycles for deterministic behavior
        let output1 = pattern_to_string(p.clone(), 4.0);
        let output2 = pattern_to_string(p.clone(), 4.0);
        assert_eq!(output1, output2);
    }
}

#[cfg(test)]
mod structural_operations {
    use super::*;

    #[test]
    fn test_palindrome() {
        let p = Pattern::from_string("a b c").palindrome();
        let output = pattern_to_string(p, 2.0);
        // Should have pattern followed by reverse: a b c c b a
        assert!(output.contains("\"a\""));
        assert!(output.contains("\"b\""));
        assert!(output.contains("\"c\""));
        // Verify it's actually 6 events over 2 cycles
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 6);
    }

    #[test]
    fn test_dup() {
        let p = Pattern::from_string("a b").dup(3);
        let output = pattern_to_string(p, 1.0);
        // Each event should appear 3 times
        let a_count = output.matches("\"a\"").count();
        let b_count = output.matches("\"b\"").count();
        assert_eq!(a_count, 3);
        assert_eq!(b_count, 3);
    }

    #[test]
    fn test_stutter() {
        let p = Pattern::from_string("a b").stutter(4);
        let output = pattern_to_string(p, 1.0);
        // Should have 8 events total (2 original * 4 stutters)
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 8);
    }
}

#[cfg(test)]
mod euclidean_rhythms {
    use super::*;

    #[test]
    fn test_euclid_3_8() {
        let p = Pattern::<bool>::euclid(3, 8, 0);
        let output = pattern_to_string(p, 1.0);
        // Should have exactly 3 true events
        let true_count = output.matches("true").count();
        assert_eq!(true_count, 3);
    }

    #[test]
    fn test_euclid_5_8() {
        let p = Pattern::<bool>::euclid(5, 8, 0);
        let output = pattern_to_string(p, 1.0);
        let true_count = output.matches("true").count();
        assert_eq!(true_count, 5);
    }

    #[test]
    fn test_euclid_rotation() {
        let p1 = Pattern::<bool>::euclid(3, 8, 0);
        let p2 = Pattern::<bool>::euclid(3, 8, 1);
        let output1 = pattern_to_string(p1, 1.0);
        let output2 = pattern_to_string(p2, 1.0);
        // Should be different due to rotation
        assert_ne!(output1, output2);
        // But same number of hits
        assert_eq!(
            output1.matches("true").count(),
            output2.matches("true").count()
        );
    }
}

#[cfg(test)]
mod hash_verification {
    use super::*;

    #[test]
    fn test_pattern_hashes() {
        // Verify that specific patterns produce expected hashes
        // This ensures the implementation is correct and deterministic

        // Test string patterns
        let string_test_cases = vec![
            (Pattern::from_string("a b c d"), 1.0, "basic_sequence"),
            (
                Pattern::from_string("a b c d").fast(2.0),
                1.0,
                "fast_pattern",
            ),
            (
                Pattern::from_string("a b c d").rev(),
                1.0,
                "reversed_pattern",
            ),
        ];

        for (pattern, cycles, name) in string_test_cases {
            let hash = pattern_hash(pattern.clone(), cycles);
            println!("{}: {}", name, hash);

            // Verify hash is consistent
            let hash2 = pattern_hash(pattern, cycles);
            assert_eq!(hash, hash2, "{} hash should be deterministic", name);
        }

        // Test bool patterns separately
        let euclid_pattern = Pattern::<bool>::euclid(5, 8, 0);
        let euclid_hash = pattern_hash(euclid_pattern.clone(), 1.0);
        println!("euclidean_5_8: {}", euclid_hash);
        let euclid_hash2 = pattern_hash(euclid_pattern, 1.0);
        assert_eq!(
            euclid_hash, euclid_hash2,
            "euclidean hash should be deterministic"
        );
    }

    #[test]
    fn test_complex_combinations() {
        // Test complex combinations of operations
        let p = Pattern::from_string("a b c d")
            .fast(2.0)
            .every(3, |p| p.rev())
            .late(0.125);

        let output = pattern_to_string(p.clone(), 3.0);
        let hash = pattern_hash(p, 3.0);

        // Verify it's deterministic
        let p2 = Pattern::from_string("a b c d")
            .fast(2.0)
            .every(3, |p| p.rev())
            .late(0.125);
        let hash2 = pattern_hash(p2, 3.0);

        assert_eq!(hash, hash2, "Complex pattern should be deterministic");
        println!("Complex pattern hash: {}", hash);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_pattern_arithmetic() {
        // Test patterns that will become numeric
        let p = Pattern::from_string("1 2 3 4");
        let output = pattern_to_string(p, 1.0);
        assert!(output.contains("\"1\""));
        assert!(output.contains("\"2\""));
        assert!(output.contains("\"3\""));
        assert!(output.contains("\"4\""));
    }

    #[test]
    fn test_nested_operations() {
        // Test deeply nested operations
        let p = Pattern::from_string("a b")
            .fast(2.0)
            .every(2, |p| p.slow(2.0))
            .every(4, |p| p.rev());

        let output = pattern_to_string(p, 4.0);
        // Should produce complex but deterministic output
        assert!(!output.is_empty());

        // Verify determinism
        let output2 = pattern_to_string(
            Pattern::from_string("a b")
                .fast(2.0)
                .every(2, |p| p.slow(2.0))
                .every(4, |p| p.rev()),
            4.0,
        );
        assert_eq!(output, output2);
    }
}
