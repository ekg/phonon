use phonon::mini_notation::parse_mini_notation;
use phonon::pattern::{State, TimeSpan, Fraction, bjorklund};
use std::collections::HashMap;

#[test]
fn test_euclidean_no_rotation() {
    let pattern = parse_mini_notation("bd(3,8)");
    let state = State {
        span: TimeSpan::new(
            Fraction::new(0, 1),
            Fraction::new(1, 1),
        ),
        controls: HashMap::new(),
    };
    
    let haps = pattern.query(&state);
    assert_eq!(haps.len(), 3, "bd(3,8) should produce 3 events");
    
    // Check positions (1st, 4th, 7th steps out of 8)
    assert_eq!(haps[0].part.begin, Fraction::new(0, 8));  // Step 0
    assert_eq!(haps[1].part.begin, Fraction::new(3, 8));  // Step 3
    assert_eq!(haps[2].part.begin, Fraction::new(6, 8));  // Step 6
}

#[test]
fn test_euclidean_with_rotation_1() {
    let pattern = parse_mini_notation("bd(3,8,1)");
    let state = State {
        span: TimeSpan::new(
            Fraction::new(0, 1),
            Fraction::new(1, 1),
        ),
        controls: HashMap::new(),
    };
    
    let haps = pattern.query(&state);
    assert_eq!(haps.len(), 3, "bd(3,8,1) should produce 3 events");
    
    // Rotated by 1 position
    assert_eq!(haps[0].part.begin, Fraction::new(2, 8));  // Was step 3, now step 2
    assert_eq!(haps[1].part.begin, Fraction::new(5, 8));  // Was step 6, now step 5
    assert_eq!(haps[2].part.begin, Fraction::new(7, 8));  // Was step 0, now step 7
}

#[test]
fn test_euclidean_with_rotation_2() {
    let pattern = parse_mini_notation("bd(3,8,2)");
    let state = State {
        span: TimeSpan::new(
            Fraction::new(0, 1),
            Fraction::new(1, 1),
        ),
        controls: HashMap::new(),
    };
    
    let haps = pattern.query(&state);
    assert_eq!(haps.len(), 3, "bd(3,8,2) should produce 3 events");
    
    // Rotated by 2 positions
    assert_eq!(haps[0].part.begin, Fraction::new(1, 8));  
    assert_eq!(haps[1].part.begin, Fraction::new(4, 8));  
    assert_eq!(haps[2].part.begin, Fraction::new(6, 8));  
}

#[test]
fn test_euclidean_negative_rotation() {
    // Negative rotation should work (rotating backwards)
    let pattern = parse_mini_notation("bd(3,8,-1)");
    let state = State {
        span: TimeSpan::new(
            Fraction::new(0, 1),
            Fraction::new(1, 1),
        ),
        controls: HashMap::new(),
    };
    
    let haps = pattern.query(&state);
    assert_eq!(haps.len(), 3, "bd(3,8,-1) should produce 3 events");
    
    // Rotated by -1 (equivalent to rotating by 7)
    assert_eq!(haps[0].part.begin, Fraction::new(1, 8));
    assert_eq!(haps[1].part.begin, Fraction::new(4, 8));
    assert_eq!(haps[2].part.begin, Fraction::new(7, 8));
}

#[test]
fn test_bjorklund_algorithm_values() {
    // Test the underlying algorithm produces correct patterns
    
    // Cuban tresillo (3,8)
    let tresillo = bjorklund(3, 8);
    assert_eq!(tresillo, vec![true, false, false, true, false, false, true, false]);
    
    // Cuban cinquillo (5,8)
    let cinquillo = bjorklund(5, 8);
    assert_eq!(cinquillo, vec![true, false, true, true, false, true, true, false]);
    
    // Bossa Nova (5,16)
    let bossa = bjorklund(5, 16);
    let expected_bossa = vec![
        true, false, false, true, false, false, true, false,
        false, true, false, false, true, false, false, false
    ];
    assert_eq!(bossa, expected_bossa);
}

#[test]
fn test_euclidean_default_steps() {
    // bd(3) should default to 8 steps
    let pattern = parse_mini_notation("bd(3)");
    let state = State {
        span: TimeSpan::new(
            Fraction::new(0, 1),
            Fraction::new(1, 1),
        ),
        controls: HashMap::new(),
    };
    
    let haps = pattern.query(&state);
    assert_eq!(haps.len(), 3, "bd(3) should produce 3 events");
    
    // Should be same as bd(3,8)
    assert_eq!(haps[0].part.begin, Fraction::new(0, 8));
    assert_eq!(haps[1].part.begin, Fraction::new(3, 8));
    assert_eq!(haps[2].part.begin, Fraction::new(6, 8));
}

#[test]
fn test_euclidean_samba_pattern() {
    // Famous Samba rhythm: (7,16,14)
    let pattern = parse_mini_notation("bd(7,16,14)");
    let state = State {
        span: TimeSpan::new(
            Fraction::new(0, 1),
            Fraction::new(1, 1),
        ),
        controls: HashMap::new(),
    };
    
    let haps = pattern.query(&state);
    assert_eq!(haps.len(), 7, "Samba pattern should have 7 beats");
    
    // Verify it starts at position 0 (after rotation by 14)
    assert_eq!(haps[0].part.begin, Fraction::new(0, 16));
}

#[test]
fn test_euclidean_multiple_patterns() {
    // Test multiple euclidean patterns in sequence
    let pattern = parse_mini_notation("bd(3,8) sn(5,8)");
    let state = State {
        span: TimeSpan::new(
            Fraction::new(0, 1),
            Fraction::new(1, 1),
        ),
        controls: HashMap::new(),
    };
    
    let haps = pattern.query(&state);
    
    // Count bd and sn events
    let bd_count = haps.iter().filter(|h| h.value == "bd").count();
    let sn_count = haps.iter().filter(|h| h.value == "sn").count();
    
    assert_eq!(bd_count, 3, "Should have 3 bd events");
    assert_eq!(sn_count, 5, "Should have 5 sn events");
}

#[test]
fn test_euclidean_edge_cases() {
    // Test (0,8) - no beats
    let pattern = parse_mini_notation("bd(0,8)");
    let state = State {
        span: TimeSpan::new(
            Fraction::new(0, 1),
            Fraction::new(1, 1),
        ),
        controls: HashMap::new(),
    };
    
    let haps = pattern.query(&state);
    assert_eq!(haps.len(), 0, "bd(0,8) should produce no events");
    
    // Test (8,8) - all beats
    let pattern = parse_mini_notation("bd(8,8)");
    let haps = pattern.query(&state);
    assert_eq!(haps.len(), 8, "bd(8,8) should produce 8 events");
    
    // Test (1,1) - single beat
    let pattern = parse_mini_notation("bd(1,1)");
    let haps = pattern.query(&state);
    assert_eq!(haps.len(), 1, "bd(1,1) should produce 1 event");
}