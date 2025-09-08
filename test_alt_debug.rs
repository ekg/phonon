use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{State, TimeSpan, Fraction};
use std::collections::HashMap;

fn main() {
    // Test simple alternation
    let pattern = parse_mini_notation("<bd sn>");
    
    for cycle in 0..2 {
        let state = State {
            span: TimeSpan::new(
                Fraction::new(cycle as i64, 1),
                Fraction::new((cycle + 1) as i64, 1),
            ),
            controls: HashMap::new(),
        };
        
        let events = pattern.query(&state);
        println!("Cycle {}: {:?}", cycle, events.iter().map(|e| &e.value).collect::<Vec<_>>());
    }
    
    // Test with parentheses - this is being incorrectly parsed
    println!("\n--- Testing <sine(440) sine(880)> ---");
    let pattern2 = parse_mini_notation("<sine(440) sine(880)>");
    
    for cycle in 0..2 {
        let state = State {
            span: TimeSpan::new(
                Fraction::new(cycle as i64, 1),
                Fraction::new((cycle + 1) as i64, 1),
            ),
            controls: HashMap::new(),
        };
        
        let events = pattern2.query(&state);
        println!("Cycle {}: {:?}", cycle, events.iter().map(|e| &e.value).collect::<Vec<_>>());
    }
}