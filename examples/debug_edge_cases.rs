use phonon::mini_notation::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use std::collections::HashMap;

fn main() {
    println!("Testing edge cases:\n");

    // Test elongate with silence
    println!("1. Testing 'bd ~ sn_':");
    let pattern = parse_mini_notation("bd ~ sn_");
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let haps = pattern.query(&state);
    println!("   Events: {}", haps.len());
    for hap in &haps {
        println!(
            "   ({:.3} -> {:.3}): {}",
            hap.part.begin.to_float(),
            hap.part.end.to_float(),
            hap.value
        );
    }
    println!();

    // Test complex nesting
    println!("2. Testing '[[bd*2 sn] hh]*2':");
    let pattern = parse_mini_notation("[[bd*2 sn] hh]*2");
    let haps = pattern.query(&state);
    println!("   Events: {}", haps.len());
    for hap in &haps {
        println!(
            "   ({:.3} -> {:.3}): {}",
            hap.part.begin.to_float(),
            hap.part.end.to_float(),
            hap.value
        );
    }
    println!();

    // Test euclidean with 0
    println!("3. Testing '{{bd}}%0':");
    let pattern = parse_mini_notation("{bd}%0");
    let haps = pattern.query(&state);
    println!("   Events: {}", haps.len());
    println!();

    // Test euclidean with 1
    println!("4. Testing '{{bd}}%1':");
    let pattern = parse_mini_notation("{bd}%1");
    let haps = pattern.query(&state);
    println!("   Events: {}", haps.len());
    for hap in &haps {
        println!(
            "   ({:.3} -> {:.3}): {}",
            hap.part.begin.to_float(),
            hap.part.end.to_float(),
            hap.value
        );
    }
    println!();

    // Test multiple operators
    println!("5. Testing 'bd*2/2':");
    let pattern = parse_mini_notation("bd*2/2");
    let haps = pattern.query(&state);
    println!("   Events: {}", haps.len());
    for hap in &haps {
        println!(
            "   ({:.3} -> {:.3}): {}",
            hap.part.begin.to_float(),
            hap.part.end.to_float(),
            hap.value
        );
    }
    println!();

    // Test alternation in polyrhythm
    println!("6. Testing '(<bd sn>, hh*4)' over 2 cycles:");
    let pattern = parse_mini_notation("(<bd sn>, hh*4)");
    for cycle in 0..2 {
        let state = State {
            span: TimeSpan::new(Fraction::new(cycle, 1), Fraction::new(cycle + 1, 1)),
            controls: {
                let mut controls = HashMap::new();
                controls.insert("_global_cycle".to_string(), cycle as f64);
                controls
            },
        };

        let haps = pattern.query(&state);
        println!("   Cycle {}: {} events", cycle, haps.len());
        for hap in &haps {
            println!(
                "     ({:.3} -> {:.3}): {}",
                hap.part.begin.to_float(),
                hap.part.end.to_float(),
                hap.value
            );
        }
    }
}
