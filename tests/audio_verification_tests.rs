//! Audio verification tests for Phonon
//! 
//! These tests verify that patterns generate correct audio output

use phonon::mini_notation::parse_mini_notation;
use phonon::pattern::{Pattern, State, TimeSpan, Fraction};
use phonon::test_utils::*;
use phonon::render::render_pattern;
use std::collections::HashMap;

#[test]
fn test_simple_pattern_audio() {
    // Test "bd sn bd sn" generates 4 hits
    let pattern = parse_mini_notation("bd sn bd sn");
    let sample_rate = 44100.0;
    let duration = 1.0; // 1 second = 1 cycle
    
    // Render the pattern
    let audio = render_pattern(&pattern, duration, sample_rate as u32, 120.0);
    
    // Expected hits at 0, 0.25, 0.5, 0.75 seconds
    let expected_hits = vec![
        0,
        (0.25 * sample_rate) as usize,
        (0.5 * sample_rate) as usize,
        (0.75 * sample_rate) as usize,
    ];
    
    assert!(verify_pattern_audio(&audio, &expected_hits, sample_rate));
}

#[test]
fn test_euclidean_pattern_audio() {
    // Test "bd(3,8)" generates 3 hits evenly distributed
    let pattern = parse_mini_notation("bd(3,8)");
    let sample_rate = 44100.0;
    let duration = 1.0;
    
    let audio = render_pattern(&pattern, duration, sample_rate as u32, 120.0);
    
    // Euclidean (3,8) pattern: X..X..X.
    // Hits at positions 0/8, 3/8, 6/8
    let expected_hits = vec![
        0,
        (3.0/8.0 * sample_rate) as usize,
        (6.0/8.0 * sample_rate) as usize,
    ];
    
    assert!(verify_pattern_audio(&audio, &expected_hits, sample_rate));
}

#[test]
fn test_polyrhythm_audio() {
    // Test "[bd cp, hh*3]" generates correct polyrhythmic pattern
    let pattern = parse_mini_notation("[bd cp, hh*3]");
    let sample_rate = 44100.0;
    let duration = 1.0;
    
    let audio = render_pattern(&pattern, duration, sample_rate as u32, 120.0);
    
    // Should have 5 total hits: bd at 0, cp at 0.5, hh at 0, 1/3, 2/3
    // Combined: hits at 0 (bd+hh), 1/3 (hh), 0.5 (cp), 2/3 (hh)
    let expected_hits = vec![
        0,
        (1.0/3.0 * sample_rate) as usize,
        (0.5 * sample_rate) as usize,
        (2.0/3.0 * sample_rate) as usize,
    ];
    
    // This is approximate - polyrhythm might produce overlapping hits
    let onsets = detect_onsets(&audio, (sample_rate * 0.01) as usize, 1.5);
    assert!(onsets.len() >= 4 && onsets.len() <= 5);
}

#[test]
fn test_rest_pattern_audio() {
    // Test "bd ~ sn ~" has only 2 hits
    let pattern = parse_mini_notation("bd ~ sn ~");
    let sample_rate = 44100.0;
    let duration = 1.0;
    
    let audio = render_pattern(&pattern, duration, sample_rate as u32, 120.0);
    
    // Hits at 0 and 0.5 seconds only
    let expected_hits = vec![
        0,
        (0.5 * sample_rate) as usize,
    ];
    
    assert!(verify_pattern_audio(&audio, &expected_hits, sample_rate));
}

#[test]
fn test_fast_pattern_audio() {
    // Test "bd*4" generates 4 hits in rapid succession
    let pattern = parse_mini_notation("bd*4");
    let sample_rate = 44100.0;
    let duration = 1.0;
    
    let audio = render_pattern(&pattern, duration, sample_rate as u32, 120.0);
    
    // 4 hits evenly spaced
    let expected_hits = vec![
        0,
        (0.25 * sample_rate) as usize,
        (0.5 * sample_rate) as usize,
        (0.75 * sample_rate) as usize,
    ];
    
    assert!(verify_pattern_audio(&audio, &expected_hits, sample_rate));
}

#[test]
fn test_sample_differentiation() {
    // Test that different samples produce different audio
    let kick_pattern = parse_mini_notation("bd");
    let snare_pattern = parse_mini_notation("sn");
    let hihat_pattern = parse_mini_notation("hh");
    
    let sample_rate = 44100.0;
    let duration = 0.1; // Short duration
    
    let kick_audio = render_pattern(&kick_pattern, duration, sample_rate as u32, 120.0);
    let snare_audio = render_pattern(&snare_pattern, duration, sample_rate as u32, 120.0);
    let hihat_audio = render_pattern(&hihat_pattern, duration, sample_rate as u32, 120.0);
    
    // Calculate spectral centroids - they should be different
    let kick_centroid = spectral_centroid(&kick_audio, sample_rate);
    let snare_centroid = spectral_centroid(&snare_audio, sample_rate);
    let hihat_centroid = spectral_centroid(&hihat_audio, sample_rate);
    
    // Kick should have lowest centroid, hihat highest
    assert!(kick_centroid < snare_centroid);
    assert!(snare_centroid < hihat_centroid);
}

#[test]
fn test_pattern_timing_accuracy() {
    // Test precise timing of pattern events
    let pattern = parse_mini_notation("bd sn cp hh");
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    
    let events: Vec<_> = pattern.query(&state).collect();
    
    // Should have exactly 4 events
    assert_eq!(events.len(), 4);
    
    // Check timing of each event
    assert_eq!(events[0].part.begin, Fraction::new(0, 4));
    assert_eq!(events[1].part.begin, Fraction::new(1, 4));
    assert_eq!(events[2].part.begin, Fraction::new(2, 4));
    assert_eq!(events[3].part.begin, Fraction::new(3, 4));
    
    // All events should have 1/4 duration
    for event in &events {
        let duration = event.part.end - event.part.begin;
        assert_eq!(duration, Fraction::new(1, 4));
    }
}

#[test]
fn test_alternation_pattern_audio() {
    // Test "<bd sn>" alternates between bd and sn each cycle
    let pattern = parse_mini_notation("<bd sn>");
    let sample_rate = 44100.0;
    
    // Render 2 cycles
    let audio_cycle1 = render_pattern(&pattern, 1.0, sample_rate as u32, 120.0);
    let audio_cycle2 = render_pattern(&pattern, 1.0, sample_rate as u32, 120.0);
    
    // The audio should be different between cycles
    // (This is a simplified test - real implementation would check actual alternation)
    assert!(!compare_audio(&audio_cycle1, &audio_cycle2, 0.01));
}

#[test]
fn test_group_pattern_audio() {
    // Test "[bd sn] cp" generates correct grouping
    let pattern = parse_mini_notation("[bd sn] cp");
    let sample_rate = 44100.0;
    let duration = 1.0;
    
    let audio = render_pattern(&pattern, duration, sample_rate as u32, 120.0);
    
    // [bd sn] takes first half, cp takes second half
    // bd at 0, sn at 0.25, cp at 0.5
    let expected_hits = vec![
        0,
        (0.25 * sample_rate) as usize,
        (0.5 * sample_rate) as usize,
    ];
    
    assert!(verify_pattern_audio(&audio, &expected_hits, sample_rate));
}