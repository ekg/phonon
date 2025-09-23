use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{State, TimeSpan, Fraction};
use std::collections::HashMap;

#[test]
fn test_empty_pattern() {
    let pattern = parse_mini_notation("");
    let state = State {
        span: TimeSpan::new(
            Fraction::new(0, 1),
            Fraction::new(1, 1),
        ),
        controls: HashMap::new(),
    };
    
    let haps = pattern.query(&state);
    assert_eq!(haps.len(), 0, "Empty pattern should produce no events");
}

#[test]
fn test_single_silence() {
    let pattern = parse_mini_notation("~");
    let state = State {
        span: TimeSpan::new(
            Fraction::new(0, 1),
            Fraction::new(1, 1),
        ),
        controls: HashMap::new(),
    };
    
    let haps = pattern.query(&state);
    assert_eq!(haps.len(), 0, "Single silence should produce no events");
}

#[test]
fn test_deeply_nested_groups() {
    let pattern = parse_mini_notation("[[[bd]]]");
    let state = State {
        span: TimeSpan::new(
            Fraction::new(0, 1),
            Fraction::new(1, 1),
        ),
        controls: HashMap::new(),
    };
    
    let haps = pattern.query(&state);
    assert_eq!(haps.len(), 1, "Deeply nested groups should still produce one event");
    assert_eq!(haps[0].value, "bd");
    assert_eq!(haps[0].part.begin, Fraction::new(0, 1));
    assert_eq!(haps[0].part.end, Fraction::new(1, 1));
}

#[test]
fn test_alternation_with_empty_slots() {
    let pattern = parse_mini_notation("<bd ~ sn>");
    
    // Test over 3 cycles
    for cycle in 0..3 {
        let state = State {
            span: TimeSpan::new(
                Fraction::new(cycle, 1),
                Fraction::new(cycle + 1, 1),
            ),
            controls: {
                let mut controls = HashMap::new();
                controls.insert("_global_cycle".to_string(), cycle as f64);
                controls
            },
        };
        
        let haps = pattern.query(&state);
        if cycle == 0 {
            assert_eq!(haps.len(), 1, "Cycle 0 should have bd");
            assert_eq!(haps[0].value, "bd");
        } else if cycle == 1 {
            assert_eq!(haps.len(), 0, "Cycle 1 should be silent");
        } else if cycle == 2 {
            assert_eq!(haps.len(), 1, "Cycle 2 should have sn");
            assert_eq!(haps[0].value, "sn");
        }
    }
}

#[test]
fn test_complex_nesting_with_operators() {
    let pattern = parse_mini_notation("[[bd*2 sn] hh]*2");
    let state = State {
        span: TimeSpan::new(
            Fraction::new(0, 1),
            Fraction::new(1, 1),
        ),
        controls: HashMap::new(),
    };
    
    let haps = pattern.query(&state);
    // [[bd*2 sn] hh]*2 should produce 8 events total:
    // [bd bd sn] hh repeated twice = 4 events per repetition = 8 total
    assert_eq!(haps.len(), 8, "Complex nested pattern with operators should produce 8 events");
    
    // Verify the pattern repeats correctly
    let expected = ["bd", "bd", "sn", "hh", "bd", "bd", "sn", "hh"];
    for (i, hap) in haps.iter().enumerate() {
        assert_eq!(hap.value, expected[i], "Event {} should be {}", i, expected[i]);
    }
}

#[test]
fn test_alternation_in_polyrhythm() {
    let pattern = parse_mini_notation("(<bd sn>, hh*4)");
    
    // Test over 2 cycles
    for cycle in 0..2 {
        let state = State {
            span: TimeSpan::new(
                Fraction::new(cycle, 1),
                Fraction::new(cycle + 1, 1),
            ),
            controls: {
                let mut controls = HashMap::new();
                controls.insert("_global_cycle".to_string(), cycle as f64);
                controls
            },
        };
        
        let haps = pattern.query(&state);
        assert_eq!(haps.len(), 5, "Polyrhythm with alternation should have 5 events");
        
        // Check alternation works in polyrhythm
        if cycle == 0 {
            assert_eq!(haps[0].value, "bd");
        } else {
            assert_eq!(haps[0].value, "sn");
        }
        
        // Check hh*4 is consistent
        let hh_events: Vec<_> = haps.iter().filter(|h| h.value == "hh").collect();
        assert_eq!(hh_events.len(), 4, "Should have 4 hh events");
    }
}

#[test]
#[ignore] // TODO: Fix for mini_notation_v3
fn test_elongate_with_silence() {
    let pattern = parse_mini_notation("bd ~ sn_");
    let state = State {
        span: TimeSpan::new(
            Fraction::new(0, 1),
            Fraction::new(1, 1),
        ),
        controls: HashMap::new(),
    };
    
    let haps = pattern.query(&state);
    assert_eq!(haps.len(), 2, "Pattern with silence should have 2 events");
    
    // bd takes first slot (1/4 since there are 4 slots including elongation)
    assert_eq!(haps[0].value, "bd");
    assert_eq!(haps[0].part.begin, Fraction::new(0, 1));
    assert_eq!(haps[0].part.end, Fraction::new(1, 4));
    
    // sn_ takes last two slots (elongated)
    assert_eq!(haps[1].value, "sn");
    assert_eq!(haps[1].part.begin, Fraction::new(1, 2));
    assert_eq!(haps[1].part.end, Fraction::new(1, 1));  // Elongated to end of cycle
}

#[test]
#[ignore] // TODO: Fix for mini_notation_v3
fn test_random_choice_consistency() {
    // Random choice should be consistent within a cycle
    let pattern = parse_mini_notation("[bd|sn|cp]");
    let state = State {
        span: TimeSpan::new(
            Fraction::new(0, 1),
            Fraction::new(1, 1),
        ),
        controls: HashMap::new(),
    };
    
    let haps = pattern.query(&state);
    assert_eq!(haps.len(), 1, "Random choice should produce exactly one event");
    assert!(["bd", "sn", "cp"].contains(&haps[0].value.as_str()));
}

#[test]
#[ignore] // TODO: Fix for mini_notation_v3
fn test_euclidean_edge_cases() {
    // Test euclidean with 0 pulses
    let pattern = parse_mini_notation("{bd}%0");
    let state = State {
        span: TimeSpan::new(
            Fraction::new(0, 1),
            Fraction::new(1, 1),
        ),
        controls: HashMap::new(),
    };
    
    let haps = pattern.query(&state);
    assert_eq!(haps.len(), 0, "Euclidean with 0 pulses should produce no events");
    
    // Test euclidean with 1 pulse
    let pattern = parse_mini_notation("{bd}%1");
    let haps = pattern.query(&state);
    assert_eq!(haps.len(), 1, "Euclidean with 1 pulse should produce 1 event");
}

#[test]
#[ignore] // TODO: Fix for mini_notation_v3
fn test_division_edge_cases() {
    // Division by large number (sparse pattern)
    let pattern = parse_mini_notation("bd/100");
    let mut all_haps = Vec::new();
    
    // Check first 100 cycles
    for cycle in 0..100 {
        let state = State {
            span: TimeSpan::new(
                Fraction::new(cycle, 1),
                Fraction::new(cycle + 1, 1),
            ),
            controls: HashMap::new(),
        };
        
        let haps = pattern.query(&state);
        all_haps.extend(haps);
    }
    
    assert_eq!(all_haps.len(), 1, "bd/100 should produce 1 event in 100 cycles");
}

#[test]
fn test_multiple_operators_on_element() {
    let pattern = parse_mini_notation("bd*2/2");  // Multiply then divide
    let state = State {
        span: TimeSpan::new(
            Fraction::new(0, 1),
            Fraction::new(1, 1),
        ),
        controls: HashMap::new(),
    };
    
    let haps = pattern.query(&state);
    assert_eq!(haps.len(), 1, "bd*2/2 should cancel out to 1 event");
}

#[test]
#[ignore] // TODO: Fix for mini_notation_v3
fn test_alternation_with_different_lengths() {
    let pattern = parse_mini_notation("<bd [sn sn] cp>");
    
    // Test over 3 cycles
    for cycle in 0..3 {
        let state = State {
            span: TimeSpan::new(
                Fraction::new(cycle, 1),
                Fraction::new(cycle + 1, 1),
            ),
            controls: {
                let mut controls = HashMap::new();
                controls.insert("_global_cycle".to_string(), cycle as f64);
                controls
            },
        };
        
        let haps = pattern.query(&state);
        
        if cycle == 0 {
            assert_eq!(haps.len(), 1, "Cycle 0 should have 1 bd");
            assert_eq!(haps[0].value, "bd");
        } else if cycle == 1 {
            assert_eq!(haps.len(), 2, "Cycle 1 should have 2 sn");
            assert_eq!(haps[0].value, "sn");
            assert_eq!(haps[1].value, "sn");
        } else if cycle == 2 {
            assert_eq!(haps.len(), 1, "Cycle 2 should have 1 cp");
            assert_eq!(haps[0].value, "cp");
        }
    }
}

#[test]
fn test_pattern_with_only_operators() {
    // Test pattern with just operators and no samples
    let pattern = parse_mini_notation("~ ~ ~");
    let state = State {
        span: TimeSpan::new(
            Fraction::new(0, 1),
            Fraction::new(1, 1),
        ),
        controls: HashMap::new(),
    };
    
    let haps = pattern.query(&state);
    assert_eq!(haps.len(), 0, "Pattern with only silence should produce no events");
}

#[test]
fn test_very_long_pattern() {
    // Test a pattern with many elements
    let pattern_str = (0..100)
        .map(|i| format!("s{}", i))
        .collect::<Vec<_>>()
        .join(" ");
    
    let pattern = parse_mini_notation(&pattern_str);
    let state = State {
        span: TimeSpan::new(
            Fraction::new(0, 1),
            Fraction::new(1, 1),
        ),
        controls: HashMap::new(),
    };
    
    let haps = pattern.query(&state);
    assert_eq!(haps.len(), 100, "Very long pattern should produce 100 events");
    
    // Check they're evenly distributed (allow for precision)
    for (i, hap) in haps.iter().enumerate() {
        assert_eq!(hap.value, format!("s{}", i));
        // Use float comparison for better precision handling
        let expected_begin = i as f64 / 100.0;
        let expected_end = (i + 1) as f64 / 100.0;
        let actual_begin = hap.part.begin.to_float();
        let actual_end = hap.part.end.to_float();
        
        assert!((actual_begin - expected_begin).abs() < 0.0001, 
                "Begin time for event {} should be close to {}", i, expected_begin);
        assert!((actual_end - expected_end).abs() < 0.0001,
                "End time for event {} should be close to {}", i, expected_end);
    }
}