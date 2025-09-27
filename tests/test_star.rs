use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use std::collections::HashMap;

#[test]
fn test_star_operator() {
    // Test bd*4 - should repeat bd 4 times fast
    let pattern = parse_mini_notation("bd*4");

    let begin = Fraction::new(0, 1);
    let end = Fraction::new(1, 1);
    let state = State {
        span: TimeSpan::new(begin, end),
        controls: HashMap::new(),
    };

    let events = pattern.query(&state);

    println!("\nbd*4 events:");
    for event in &events {
        println!(
            "  {:.3} -> {:.3} : {}",
            event.part.begin.to_float(),
            event.part.end.to_float(),
            event.value
        );
    }

    // Also test a Pattern directly with dup
    let direct_pattern = Pattern::pure("test".to_string()).dup(4);
    let direct_events = direct_pattern.query(&state);

    println!("\nDirect pattern.dup(4) events:");
    for event in &direct_events {
        println!(
            "  {:.3} -> {:.3} : {}",
            event.part.begin.to_float(),
            event.part.end.to_float(),
            event.value
        );
    }

    // Test fast directly
    let fast_pattern = Pattern::pure("fast".to_string()).fast(4.0);
    let fast_events = fast_pattern.query(&state);

    println!("\nDirect pattern.fast(4) events:");
    for event in &fast_events {
        println!(
            "  {:.3} -> {:.3} : {}",
            event.part.begin.to_float(),
            event.part.end.to_float(),
            event.value
        );
    }

    // Test with a sequence pattern
    let seq_pattern = parse_mini_notation("a b");
    let seq_fast = parse_mini_notation("a b").fast(4.0);

    let seq_events = seq_pattern.query(&state);
    println!("\n'a b' events:");
    for event in &seq_events {
        println!(
            "  {:.3} -> {:.3} : {}",
            event.part.begin.to_float(),
            event.part.end.to_float(),
            event.value
        );
    }

    let seq_fast_events = seq_fast.query(&state);
    println!("\n'a b'.fast(4) events:");
    for event in &seq_fast_events {
        println!(
            "  {:.3} -> {:.3} : {}",
            event.part.begin.to_float(),
            event.part.end.to_float(),
            event.value
        );
    }

    assert!(events.len() > 0, "Should have events");
}
