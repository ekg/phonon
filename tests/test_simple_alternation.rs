use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{State, TimeSpan, Fraction};
use std::collections::HashMap;

#[test]
fn test_simple_alternation() {
    println!("\n=== Testing Simple Alternation ===");

    // Test just alternation
    let pattern = parse_mini_notation("<sn cp>");

    for cycle in 0..4 {
        let state = State {
            span: TimeSpan::new(
                Fraction::new(cycle as i64, 1),
                Fraction::new((cycle + 1) as i64, 1),
            ),
            controls: HashMap::new(),
        };

        let events = pattern.query(&state);
        println!("Cycle {}: {:?}", cycle,
                 events.iter().map(|e| &e.value).collect::<Vec<_>>());

        assert_eq!(events.len(), 1);
        let expected = if cycle % 2 == 0 { "sn" } else { "cp" };
        assert_eq!(events[0].value, expected);
    }

    println!("\n=== Testing Alternation with Stack ===");

    // Test in a stack
    let pattern2 = parse_mini_notation("[bd, <sn cp>]");

    for cycle in 0..4 {
        let state = State {
            span: TimeSpan::new(
                Fraction::new(cycle as i64, 1),
                Fraction::new((cycle + 1) as i64, 1),
            ),
            controls: HashMap::new(),
        };

        let events = pattern2.query(&state);
        let bd_count = events.iter().filter(|e| e.value == "bd").count();
        let sn_count = events.iter().filter(|e| e.value == "sn").count();
        let cp_count = events.iter().filter(|e| e.value == "cp").count();

        println!("Cycle {}: bd={}, sn={}, cp={}", cycle, bd_count, sn_count, cp_count);

        assert_eq!(bd_count, 1);
        if cycle % 2 == 0 {
            assert_eq!(sn_count, 1);
            assert_eq!(cp_count, 0);
        } else {
            assert_eq!(sn_count, 0);
            assert_eq!(cp_count, 1);
        }
    }
}