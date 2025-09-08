use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{State, TimeSpan, Fraction};
use std::collections::HashMap;

#[test]
fn test_polyrhythm_brackets() {
    // Test [bd(3,8), cp(2,4,2)] - should play both patterns simultaneously
    let pattern = parse_mini_notation("[bd(3,8), cp(2,4,2)]");
    
    let begin = Fraction::new(0, 1);
    let end = Fraction::new(1, 1);
    let state = State {
        span: TimeSpan::new(begin, end),
        controls: HashMap::new(),
    };
    
    let events = pattern.query(&state);
    
    println!("\n[bd(3,8), cp(2,4,2)] events:");
    for event in &events {
        println!("  {:.3} -> {:.3} : {}", 
                 event.part.begin.to_float(), 
                 event.part.end.to_float(), 
                 event.value);
    }
    
    // Count events
    let bd_count = events.iter().filter(|e| e.value == "bd").count();
    let cp_count = events.iter().filter(|e| e.value == "cp").count();
    
    println!("\nFound {} bd events and {} cp events", bd_count, cp_count);
    
    // Should have both bd and cp events
    assert!(bd_count > 0, "Should have bd events");
    assert!(cp_count > 0, "Should have cp events");
}