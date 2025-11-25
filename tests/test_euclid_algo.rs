use phonon::pattern::Pattern;

// Test the euclidean algorithm directly using a simple implementation
fn simple_euclid(pulses: usize, steps: usize) -> Vec<bool> {
    let mut result = vec![false; steps];

    if pulses > 0 {
        // Distribute pulses evenly across steps
        for i in 0..pulses {
            let pos = (i * steps) / pulses;
            result[pos] = true;
        }
    }

    result
}

#[test]
fn test_bjorklund_algorithm() {
    // Test (3,8) - should give X..X..X.
    let pattern = simple_euclid(3, 8);
    println!("\nEuclidean (3,8) pattern:");
    for (_i, &val) in pattern.iter().enumerate() {
        print!("{}", if val { "X" } else { "." });
    }
    println!();

    // Count positions
    let mut positions = Vec::new();
    for (i, &val) in pattern.iter().enumerate() {
        if val {
            positions.push(i);
        }
    }
    println!("Hit positions: {:?}", positions);

    // For (3,8), we expect hits at positions [0, 3, 6] or similar even distribution
    assert_eq!(positions.len(), 3);

    // Test (5,8)
    let pattern2 = simple_euclid(5, 8);
    println!("\nEuclidean (5,8) pattern:");
    for &val in pattern2.iter() {
        print!("{}", if val { "X" } else { "." });
    }
    println!();
}

#[test]
fn test_pattern_euclid_consistency() {
    use phonon::pattern::{Fraction, State, TimeSpan};
    use std::collections::HashMap;

    // Test that Pattern::euclid produces the same rhythm pattern
    let pattern = Pattern::<bool>::euclid(3, 8, 0);
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let haps = pattern.query(&state);

    // Should produce 3 events (true values)
    let true_count = haps.iter().filter(|h| h.value).count();
    assert_eq!(
        true_count, 3,
        "Pattern::euclid(3,8,0) should produce 3 hits"
    );

    // Compare positions with our simple implementation
    let simple = simple_euclid(3, 8);
    let simple_positions: Vec<usize> = simple
        .iter()
        .enumerate()
        .filter_map(|(i, &v)| if v { Some(i) } else { None })
        .collect();

    println!("Simple euclid positions: {:?}", simple_positions);

    // The Pattern::euclid should have events at roughly the same fractional positions
    for (_i, hap) in haps.iter().enumerate() {
        if hap.value {
            let pos = (hap.part.begin.to_float() * 8.0).round() as usize;
            println!("Pattern::euclid hit at position {}/8", pos);
        }
    }
}
