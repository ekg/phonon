//! Comprehensive tests for mini-notation parsing and playback

use phonon::mini_notation::parse_mini_notation;
use phonon::pattern::{Pattern, State, TimeSpan, Fraction};
use phonon::pattern_debug::{describe_pattern, verify_polyphony};
use std::collections::HashMap;

fn query_pattern(pattern: &Pattern<String>, cycle: usize) -> Vec<String> {
    let begin = Fraction::new(cycle as i64, 1);
    let end = Fraction::new((cycle + 1) as i64, 1);
    let span = TimeSpan::new(begin, end);
    let state = State {
        span,
        controls: HashMap::new(),
    };
    
    pattern.query(&state)
        .into_iter()
        .map(|hap| hap.value)
        .collect()
}

#[test]
fn test_simple_sequence() {
    let pattern = parse_mini_notation("bd sn hh cp");
    let events = query_pattern(&pattern, 0);
    
    assert_eq!(events, vec!["bd", "sn", "hh", "cp"]);
    assert!(!verify_polyphony(&pattern));
}

#[test]
fn test_rests() {
    let pattern = parse_mini_notation("bd ~ sn ~");
    let events = query_pattern(&pattern, 0);
    
    assert_eq!(events, vec!["bd", "sn"]);
    assert!(!verify_polyphony(&pattern));
}

#[test]
fn test_groups_play_faster() {
    let pattern = parse_mini_notation("bd [sn sn] hh");
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    
    let haps = pattern.query(&state);
    
    // Check that [sn sn] takes same time as single bd
    assert_eq!(haps.len(), 4); // bd, sn, sn, hh
    
    // bd should be 0.0-0.333
    assert!(haps[0].value == "bd");
    assert!((haps[0].part.end.to_float() - haps[0].part.begin.to_float() - 0.333).abs() < 0.01);
    
    // Each sn should be half of the group time
    let sn_haps: Vec<_> = haps.iter().filter(|h| h.value == "sn").collect();
    assert_eq!(sn_haps.len(), 2);
    for sn in &sn_haps {
        assert!((sn.part.end.to_float() - sn.part.begin.to_float() - 0.167).abs() < 0.01);
    }
}

#[test]
fn test_chord_polyphony() {
    let pattern = parse_mini_notation("[bd cp, hh hh hh]");
    assert!(verify_polyphony(&pattern));
    
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    
    let haps = pattern.query(&state);
    
    // Should have bd, cp, and 3 hh's
    assert_eq!(haps.len(), 5);
    
    // Check overlapping events
    let bd_hap = haps.iter().find(|h| h.value == "bd").unwrap();
    let hh_haps: Vec<_> = haps.iter().filter(|h| h.value == "hh").collect();
    
    // bd and first hh should overlap
    assert!(bd_hap.part.begin.to_float() < hh_haps[0].part.end.to_float());
}

#[test]
fn test_alternation() {
    let pattern = parse_mini_notation("<bd sn cp>");
    
    // Each cycle should have a different sound
    assert_eq!(query_pattern(&pattern, 0), vec!["bd"]);
    assert_eq!(query_pattern(&pattern, 1), vec!["sn"]);
    assert_eq!(query_pattern(&pattern, 2), vec!["cp"]);
    assert_eq!(query_pattern(&pattern, 3), vec!["bd"]); // Cycles back
}

#[test]
fn test_repeat_operator() {
    let pattern = parse_mini_notation("bd*3 sn");
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    
    let haps = pattern.query(&state);
    
    // Should have 3 bd's and 1 sn
    let bd_count = haps.iter().filter(|h| h.value == "bd").count();
    let sn_count = haps.iter().filter(|h| h.value == "sn").count();
    
    assert_eq!(bd_count, 3);
    assert_eq!(sn_count, 1);
}

#[test]
fn test_polyrhythm_parentheses() {
    let pattern = parse_mini_notation("(bd, sn cp, hh hh hh)");
    assert!(verify_polyphony(&pattern));
    
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    
    let haps = pattern.query(&state);
    
    // Should have 1 bd, 1 sn, 1 cp, and 3 hh's
    assert_eq!(haps.len(), 6);
    
    let bd_count = haps.iter().filter(|h| h.value == "bd").count();
    let sn_count = haps.iter().filter(|h| h.value == "sn").count();
    let cp_count = haps.iter().filter(|h| h.value == "cp").count();
    let hh_count = haps.iter().filter(|h| h.value == "hh").count();
    
    assert_eq!(bd_count, 1);
    assert_eq!(sn_count, 1);
    assert_eq!(cp_count, 1);
    assert_eq!(hh_count, 3);
    
    // bd should span the whole cycle
    let bd_hap = haps.iter().find(|h| h.value == "bd").unwrap();
    assert!((bd_hap.part.end.to_float() - bd_hap.part.begin.to_float() - 1.0).abs() < 0.01);
}

#[test]
fn test_stacking_with_pipe() {
    let pattern = parse_mini_notation("bd sn | hh hh hh hh");
    assert!(verify_polyphony(&pattern));
    
    let events = query_pattern(&pattern, 0);
    
    // Should contain both bd/sn and hh patterns
    assert!(events.contains(&"bd".to_string()));
    assert!(events.contains(&"sn".to_string()));
    assert_eq!(events.iter().filter(|e| *e == "hh").count(), 4);
}

#[test]
fn test_complex_pattern() {
    // Complex pattern combining multiple features
    let pattern = parse_mini_notation("[bd*2 ~, hh hh hh hh] | <sn cp>");
    assert!(verify_polyphony(&pattern));
    
    // Cycle 0 should have bd, hh's, and sn
    let events_0 = query_pattern(&pattern, 0);
    assert!(events_0.contains(&"bd".to_string()));
    assert!(events_0.contains(&"sn".to_string()));
    assert!(events_0.iter().filter(|e| *e == "hh").count() > 0);
    
    // Cycle 1 should have cp instead of sn
    let events_1 = query_pattern(&pattern, 1);
    assert!(events_1.contains(&"cp".to_string()));
    assert!(!events_1.contains(&"sn".to_string()));
}

#[test]
fn test_nested_groups() {
    let pattern = parse_mini_notation("bd [[sn sn] cp] hh");
    
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    
    let haps = pattern.query(&state);
    
    // Should have bd, 2 sn's, cp, and hh
    assert_eq!(haps.len(), 5);
}

#[test]
fn test_slow_operator() {
    let pattern = parse_mini_notation("bd/2 sn");
    
    // bd/2 stretches bd over 2 cycles, so it appears in both
    // but only partially in each cycle
    let events_0 = query_pattern(&pattern, 0);
    let events_1 = query_pattern(&pattern, 1);
    
    // Both cycles should have bd (it's stretched over 2 cycles)
    assert!(events_0.contains(&"bd".to_string()));
    assert!(events_1.contains(&"bd".to_string()));
    
    // sn should be in both
    assert!(events_0.contains(&"sn".to_string()));
    assert!(events_1.contains(&"sn".to_string()));
}

// Test for audio rendering integration
#[test]
fn test_pattern_renders_audio() {
    use phonon::pattern_sequencer_voice::create_pattern_sequencer;
    
    let mut sequencer = create_pattern_sequencer("bd sn hh cp", 44100.0);
    
    // Process a block and verify it produces audio
    let block = sequencer.process_block(512);
    
    // Should have some non-zero samples
    let has_audio = block.iter().any(|&s| s != 0.0);
    assert!(has_audio, "Pattern should produce audio output");
}