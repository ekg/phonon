use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{State, TimeSpan, Fraction};
use std::collections::HashMap;

#[test]
fn test_single_arg_parsing() {
    println!("\n=== Test Single Argument Euclidean ===");

    // Test what bd(3) actually produces
    let pattern1 = parse_mini_notation("bd(3)");
    let state = State {
        span: TimeSpan::new(
            Fraction::new(0, 1),
            Fraction::new(1, 1),
        ),
        controls: HashMap::new(),
    };

    let haps1 = pattern1.query(&state);
    println!("bd(3) produced {} events", haps1.len());
    if haps1.len() > 0 {
        println!("  Event: {}", haps1[0].value);
    }

    // Compare with bd*3
    let pattern2 = parse_mini_notation("bd*3");
    let haps2 = pattern2.query(&state);
    println!("bd*3 produced {} events", haps2.len());

    // If bd(3) is interpreted as repetition, it should produce 3 events
    // Or it might be parsed as a function call on bd with argument 3
}