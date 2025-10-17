use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use std::collections::HashMap;

#[test]
fn debug_euclidean_rotation() {
    println!("\n=== Testing Euclidean Patterns with Rotation ===");

    // Test no rotation
    let patterns = vec![
        ("bd(3,8)", "3 pulses, 8 steps, no rotation"),
        ("bd(3,8,0)", "3 pulses, 8 steps, rotation 0"),
        ("bd(3,8,1)", "3 pulses, 8 steps, rotation 1"),
        ("bd(3,8,2)", "3 pulses, 8 steps, rotation 2"),
        ("bd(3,8,-1)", "3 pulses, 8 steps, rotation -1"),
    ];

    for (pattern_str, desc) in patterns {
        println!("\nTesting {}: {}", pattern_str, desc);
        let pattern = parse_mini_notation(pattern_str);

        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };

        let haps = pattern.query(&state);
        println!("  Produced {} events:", haps.len());

        for (i, hap) in haps.iter().enumerate() {
            println!(
                "    Event {}: {} at {:.3} ({}/{})",
                i,
                hap.value,
                hap.part.begin.to_float(),
                hap.part.begin.numerator,
                hap.part.begin.denominator
            );
        }
    }
}
