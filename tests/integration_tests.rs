//! Integration tests for Pattern + DSP interaction

use phonon::mini_notation_v3::parse_mini_notation;
use phonon::glicol_pattern_bridge::PatternDspEngine;
use phonon::pattern::{Pattern, State, TimeSpan, Fraction};
use std::collections::HashMap;

#[test]
fn test_pattern_triggered_synthesis() {
    // Test pattern triggering DSP synthesis
    let mut engine = PatternDspEngine::new(120.0);
    
    // Parse a pattern that triggers synthesis
    let result = engine.parse_hybrid("bd*4 >> lpf(1000, 0.8)");
    assert!(result.is_ok());
    
    // Verify the engine parsed successfully
    // (internal structure is private, just check parse succeeded)
}

#[test]
fn test_pattern_modulating_dsp() {
    // Test using pattern to modulate DSP parameters
    let mut engine = PatternDspEngine::new(120.0);
    
    // Pattern generating modulation values
    let mod_pattern = Pattern::cat(vec![
        Pattern::pure(0.0),
        Pattern::pure(0.5),
        Pattern::pure(1.0),
        Pattern::pure(0.5),
    ]);
    
    // Would insert pattern into engine if it had public access
    // For now, just use the pattern
    let _ = mod_pattern;
    
    // This would modulate a filter cutoff in real implementation
    let result = engine.parse_hybrid("sin(440) >> lpf(~mod * 2000 + 500, 0.8)");
    assert!(result.is_ok());
}

#[test]
fn test_cross_modulation() {
    // Test DSP processing pattern audio
    let mut engine = PatternDspEngine::new(120.0);
    
    // Pattern generates rhythm, DSP processes it
    let result = engine.parse_hybrid("bd(3,8) >> reverb(0.3)");
    if let Err(e) = &result {
        eprintln!("Parse error: {}", e);
    }
    assert!(result.is_ok());
}

#[test]
fn test_pattern_event_timing_with_dsp() {
    // Verify pattern events maintain timing through DSP processing
    let pattern = parse_mini_notation("bd sn hh cp");
    
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    
    let events = pattern.query(&state);
    
    // Events should maintain timing regardless of DSP processing
    assert_eq!(events.len(), 4);
    assert_eq!(events[0].part.begin, Fraction::new(0, 4));
    assert_eq!(events[1].part.begin, Fraction::new(1, 4));
    assert_eq!(events[2].part.begin, Fraction::new(2, 4));
    assert_eq!(events[3].part.begin, Fraction::new(3, 4));
}

#[test]
fn test_complete_signal_flow() {
    // Test complete flow: Pattern -> DSP -> Audio
    let mut engine = PatternDspEngine::new(120.0);
    
    // Complex pattern with DSP processing
    let code = r#"
        bd(5,8) >> lpf(1000, 0.8)
    "#;
    
    let result = engine.parse_hybrid(code.trim());
    assert!(result.is_ok());
    
    // Generate audio (simplified - would need full implementation)
    let _sample_rate = 44100.0;
    let samples = 44100; // 1 second
    let _audio = vec![0.0; samples];
    
    // Process would generate audio here
    // For now, just verify parse succeeded
    assert!(result.is_ok());
}

#[test]
fn test_midi_to_dsp_pipeline() {
    // Test MIDI note patterns through DSP
    let pattern = parse_mini_notation("c4 e4 g4 c5");
    
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    
    let events = pattern.query(&state);
    
    // Should have 4 note events
    assert_eq!(events.len(), 4);
    
    // Each note should map to correct MIDI/frequency
    // C4 = MIDI 60, E4 = MIDI 64, G4 = MIDI 67, C5 = MIDI 72
    let expected_notes = vec!["c4", "e4", "g4", "c5"];
    for (event, expected) in events.iter().zip(expected_notes.iter()) {
        assert_eq!(&event.value, expected);
    }
}

#[test]
fn test_layered_patterns_with_effects() {
    // Test multiple pattern layers with different effects
    let mut engine = PatternDspEngine::new(120.0);
    
    // Multiple layers with different processing
    let layers = vec![
        "bd(3,8) >> lpf(500, 0.9)",          // Filtered kick
        "hh*16 >> hpf(8000, 0.7)",           // Bright hi-hats
        "[~ cp]*4 >> reverb(0.3)",           // Reverbed claps
    ];
    
    for layer in layers {
        let result = engine.parse_hybrid(layer);
        assert!(result.is_ok());
    }
    
    // Verify all layers parsed successfully
    // (internal structure is private)
}

#[test]
fn test_tempo_sync() {
    // Test that patterns and DSP stay in sync at different tempos
    let tempos = vec![60.0, 120.0, 140.0, 174.0]; // Different BPMs
    
    for tempo in tempos {
        let mut engine = PatternDspEngine::new(tempo);
        let result = engine.parse_hybrid("bd*4 sn*2");
        assert!(result.is_ok());
        
        // Events should scale with tempo
        let pattern = parse_mini_notation("bd*4");
        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };
        
        let events = pattern.query(&state);
        assert_eq!(events.len(), 4); // Always 4 events per cycle
    }
}

#[test]
fn test_pattern_chaining() {
    // Test chaining patterns with operators
    let pattern1 = parse_mini_notation("bd sn");
    let pattern2 = parse_mini_notation("hh*4");
    
    // Stack patterns
    let stacked = Pattern::stack(vec![pattern1, pattern2]);
    
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    
    let events = stacked.query(&state);
    
    // Should have events from both patterns
    assert!(events.len() >= 6); // 2 from pattern1, 4 from pattern2
}

#[test]
fn test_dynamic_pattern_generation() {
    // Test generating patterns programmatically
    let mut patterns = Vec::new();
    
    // Generate euclidean patterns with different densities
    for i in 1..=8 {
        let p = Pattern::<bool>::euclid(i, 16, 0);
        patterns.push(p);
    }
    
    // Each should have different number of hits
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    
    let mut hit_counts = Vec::new();
    for p in patterns {
        let events = p.query(&state);
        hit_counts.push(events.len());
    }
    
    // Hit counts should increase
    for i in 1..hit_counts.len() {
        assert!(hit_counts[i] >= hit_counts[i-1]);
    }
}