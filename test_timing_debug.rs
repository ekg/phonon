use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{State, TimeSpan, Fraction};
use std::collections::HashMap;

fn main() {
    println!("=== Testing Pattern Timing ===\n");
    
    // Test a simple 4/4 pattern
    let pattern = parse_mini_notation("bd sn bd sn");
    
    let state = State {
        span: TimeSpan::new(
            Fraction::new(0, 1),
            Fraction::new(1, 1),
        ),
        controls: HashMap::new(),
    };
    
    let events = pattern.query(&state);
    
    println!("Pattern: 'bd sn bd sn' (should be evenly spaced)");
    println!("Events: {}\n", events.len());
    
    for (i, event) in events.iter().enumerate() {
        let start = event.part.begin.to_float();
        let end = event.part.end.to_float();
        let duration = end - start;
        
        println!("Event {}: '{}'\n  Start: {:.4}\n  End:   {:.4}\n  Duration: {:.4}",
                 i, event.value, start, end, duration);
    }
    
    // Check if timing is correct
    println!("\n=== Expected vs Actual ===");
    println!("Expected: Each event should take 0.25 of the cycle");
    
    for i in 0..4 {
        let expected_start = i as f64 * 0.25;
        let expected_end = (i + 1) as f64 * 0.25;
        
        if i < events.len() {
            let actual_start = events[i].part.begin.to_float();
            let actual_end = events[i].part.end.to_float();
            
            println!("Event {}: Expected {:.3}-{:.3}, Got {:.3}-{:.3} {}",
                     i, expected_start, expected_end, actual_start, actual_end,
                     if (actual_start - expected_start).abs() < 0.01 { "✓" } else { "✗" });
        }
    }
    
    println!("\n=== Testing pattern with rest ===");
    let pattern2 = parse_mini_notation("bd ~ sn ~");
    let events2 = pattern2.query(&state);
    
    println!("Pattern: 'bd ~ sn ~'");
    println!("Events: {} (should be 2, rests don't generate events)\n", events2.len());
    
    for (i, event) in events2.iter().enumerate() {
        let start = event.part.begin.to_float();
        let end = event.part.end.to_float();
        
        println!("Event {}: '{}' at {:.3}-{:.3}",
                 i, event.value, start, end);
    }
}