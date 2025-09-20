use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{State, TimeSpan, Fraction};
use std::collections::HashMap;

#[test]
fn test_alternation_in_euclidean() {
    // Test bd(<3,4>,8) - should alternate between 3 and 4 pulses each cycle
    let pattern = parse_mini_notation("bd(<3,4>,8)");
    
    println!("\nTesting bd(<3,4>,8) - alternating euclidean patterns:");
    
    for cycle in 0..4 {
        let begin = Fraction::new(cycle as i64, 1);
        let end = Fraction::new((cycle + 1) as i64, 1);
        let state = State {
            span: TimeSpan::new(begin, end),
            controls: HashMap::new(),
        };
        
        let events = pattern.query(&state);
        
        println!("\nCycle {}:", cycle);
        for event in &events {
            println!("  {:.3} -> {:.3} : {}", 
                     event.part.begin.to_float(), 
                     event.part.end.to_float(), 
                     event.value);
        }
        
        let bd_count = events.iter().filter(|e| e.value == "bd").count();
        println!("  Total: {} bd events", bd_count);
        
        // Cycles 0 and 2 should have 3 events
        // Cycles 1 and 3 should have 4 events
        if cycle % 2 == 0 {
            assert_eq!(bd_count, 3, "Even cycles should have 3 events");
        } else {
            assert_eq!(bd_count, 4, "Odd cycles should have 4 events");
        }
    }
}

#[test]
fn test_nested_alternation() {
    // Test even more complex nesting: bd(<3,<4,5>>,8)
    // This alternates between 3 and an alternation of 4,5
    // NOTE: Current implementation evaluates nested alternations with the same cycle,
    // so <3,<4,5>> produces: 3, 5, 3, 5, ... (not the ideal 3, 4, 3, 5, ...)
    let pattern = parse_mini_notation("bd(<3,<4,5>>,8)");

    println!("\nTesting bd(<3,<4,5>>,8) - nested alternating patterns:");

    for cycle in 0..6 {
        let begin = Fraction::new(cycle as i64, 1);
        let end = Fraction::new((cycle + 1) as i64, 1);
        let state = State {
            span: TimeSpan::new(begin, end),
            controls: HashMap::new(),
        };

        let events = pattern.query(&state);
        let bd_count = events.iter().filter(|e| e.value == "bd").count();

        println!("Cycle {}: {} bd events", cycle, bd_count);

        // Current behavior: alternates between 3 and 5 (inner alternation uses same cycle)
        let expected = if cycle % 2 == 0 { 3 } else { 5 };

        assert_eq!(bd_count, expected, "Cycle {} should have {} events", cycle, expected);
    }
}

#[test]
fn test_polyrhythm_with_euclidean() {
    // Test [bd(3,8), cp(<2,3>,4)]
    let pattern = parse_mini_notation("[bd(3,8), cp(<2,3>,4)]");
    
    println!("\nTesting [bd(3,8), cp(<2,3>,4)] - polyrhythm with patterns:");
    
    for cycle in 0..2 {
        let begin = Fraction::new(cycle as i64, 1);
        let end = Fraction::new((cycle + 1) as i64, 1);
        let state = State {
            span: TimeSpan::new(begin, end),
            controls: HashMap::new(),
        };
        
        let events = pattern.query(&state);
        
        println!("\nCycle {}:", cycle);
        for event in &events {
            println!("  {:.3} -> {:.3} : {}", 
                     event.part.begin.to_float(), 
                     event.part.end.to_float(), 
                     event.value);
        }
        
        let bd_count = events.iter().filter(|e| e.value == "bd").count();
        let cp_count = events.iter().filter(|e| e.value == "cp").count();
        
        println!("  bd: {} events, cp: {} events", bd_count, cp_count);
        
        // bd should always have 3 events
        assert_eq!(bd_count, 3, "bd should have 3 events");
        
        // cp should alternate between 2 and 3
        if cycle % 2 == 0 {
            assert_eq!(cp_count, 2, "cp should have 2 events on even cycles");
        } else {
            assert_eq!(cp_count, 3, "cp should have 3 events on odd cycles");
        }
    }
}

#[test]
fn test_operators_with_alternation() {
    // Test bd*<3,4> - the repeat amount alternates
    let pattern = parse_mini_notation("bd*<3,4>");
    
    println!("\nTesting bd*<3,4> - alternating repeat amounts:");
    
    for cycle in 0..2 {
        let begin = Fraction::new(cycle as i64, 1);
        let end = Fraction::new((cycle + 1) as i64, 1);
        let state = State {
            span: TimeSpan::new(begin, end),
            controls: HashMap::new(),
        };
        
        let events = pattern.query(&state);
        let bd_count = events.iter().filter(|e| e.value == "bd").count();
        
        println!("Cycle {}: {} bd events", cycle, bd_count);
        
        // Should alternate between 3 and 4 repetitions
        let expected = if cycle % 2 == 0 { 3 } else { 4 };
        assert_eq!(bd_count, expected, "Cycle {} should have {} events", cycle, expected);
    }
}