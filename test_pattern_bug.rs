use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{State, TimeSpan, Fraction};
use std::collections::HashMap;

fn main() {
    println!("=== Debugging Pattern Compression Bug ===\n");
    
    // Test the simplest case
    let pattern = parse_mini_notation("a b c d");
    
    let state = State {
        span: TimeSpan::new(
            Fraction::new(0, 1),
            Fraction::new(1, 1),
        ),
        controls: HashMap::new(),
    };
    
    let events = pattern.query(&state);
    
    println!("Pattern: 'a b c d'");
    println!("Query span: {} to {}", 
             state.span.begin.to_float(),
             state.span.end.to_float());
    println!("Events: {}\n", events.len());
    
    for (i, event) in events.iter().enumerate() {
        let start = event.part.begin.to_float();
        let end = event.part.end.to_float();
        
        println!("Event {}: '{}'", i, event.value);
        println!("  part.begin: {}", start);
        println!("  part.end:   {}", end);
        
        // Check whole if present
        if let Some(whole) = &event.whole {
            println!("  whole.begin: {}", whole.begin.to_float());
            println!("  whole.end:   {}", whole.end.to_float());
        }
        println!();
    }
    
    // Check what the pattern thinks its arc is
    println!("Expected timing:");
    println!("  Event 0: 0.00 - 0.25");
    println!("  Event 1: 0.25 - 0.50");
    println!("  Event 2: 0.50 - 0.75");
    println!("  Event 3: 0.75 - 1.00");
    
    println!("\nActual timing:");
    for (i, event) in events.iter().enumerate() {
        println!("  Event {}: {:.2} - {:.2} {}",
                 i,
                 event.part.begin.to_float(),
                 event.part.end.to_float(),
                 if event.part.end.to_float() <= 0.75 { "❌ WRONG!" } else { "✓" });
    }
}