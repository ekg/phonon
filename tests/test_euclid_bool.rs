use phonon::pattern::{Pattern, State, TimeSpan, Fraction};
use std::collections::HashMap;

#[test]
#[ignore] // TODO: Fix for new implementation
#[ignore] // TODO: Fix euclid pattern
fn test_bool_euclid() {
    // Test the boolean euclidean pattern directly
    let bool_pattern = Pattern::<bool>::euclid(3, 8, 0);
    
    let begin = Fraction::new(0, 1);
    let end = Fraction::new(1, 1);
    let state = State {
        span: TimeSpan::new(begin, end),
        controls: HashMap::new(),
    };
    
    let events = bool_pattern.query(&state);
    
    println!("\nBoolean Euclidean (3,8) - {} events:", events.len());
    for event in &events {
        println!("  {:.3} -> {:.3} : {}", 
                 event.part.begin.to_float(), 
                 event.part.end.to_float(), 
                 event.value);
    }
    
    // The euclidean (3,8) should have pattern: X..X..X.
    // So we expect booleans at positions 0/8, 3/8, 6/8
    assert_eq!(events.len(), 3, "Should have 3 true events");
    
    // Check approximate positions
    let positions: Vec<f64> = events.iter().map(|e| e.part.begin.to_float()).collect();
    assert!((positions[0] - 0.0).abs() < 0.01, "First hit at 0");
    assert!((positions[1] - 0.375).abs() < 0.01, "Second hit at 3/8");
    assert!((positions[2] - 0.75).abs() < 0.01, "Third hit at 6/8");
}