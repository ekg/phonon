use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{State, TimeSpan, Fraction};
use std::collections::HashMap;

#[test]
#[ignore] // TODO: Fix for new implementation
fn test_alternation_in_euclidean() {
    // Test bd(<3,4>,8) - should alternate between 3 and 4 pulses each cycle
    let pattern = parse_mini_notation("bd(<3,4>,8)");
    
    let mut all_events = Vec::new();
    
    // Check multiple cycles
    for cycle in 0..4 {
        let begin = Fraction::new(cycle as i64, 1);
        let end = Fraction::new((cycle + 1) as i64, 1);
        let state = State {
            span: TimeSpan::new(begin, end),
            controls: HashMap::new(),
        };
        
        let events = pattern.query(&state);
        
        println!("\nCycle {} - bd(<3,4>,8) events:", cycle);
        for event in &events {
            println!("  {:.3} -> {:.3} : {}", 
                     event.part.begin.to_float(), 
                     event.part.end.to_float(), 
                     event.value);
        }
        
        let bd_count = events.iter().filter(|e| e.value == "bd").count();
        println!("Found {} bd events", bd_count);
        
        all_events.push((cycle, bd_count));
    }
    
    // Should alternate between 3 and 4 hits
    // Cycle 0: 3 hits, Cycle 1: 4 hits, Cycle 2: 3 hits, Cycle 3: 4 hits
    assert!(all_events[0].1 == 3 || all_events[0].1 == 4);
    assert!(all_events[1].1 == 3 || all_events[1].1 == 4);
    assert_ne!(all_events[0].1, all_events[1].1, "Should alternate");
}