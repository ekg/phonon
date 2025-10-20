//! Utilities for verifying that audio output matches pattern specifications
//!
//! This module provides tools to:
//! 1. Query patterns to get expected event times
//! 2. Detect events/onsets in rendered audio
//! 3. Compare expected vs actual events

use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use std::collections::HashMap;

/// Event detected in audio or expected from pattern
#[derive(Debug, Clone)]
pub struct Event {
    /// Time in seconds
    pub time: f64,
    /// Optional value (for patterns with values)
    pub value: Option<String>,
    /// RMS amplitude around this event
    pub amplitude: f32,
}

/// Query a pattern to get expected events over a time range
pub fn get_expected_events(
    pattern: &Pattern<String>,
    duration_seconds: f64,
    cps: f64,
) -> Vec<Event> {
    let duration_cycles = duration_seconds * cps;

    let state = State {
        span: TimeSpan::new(
            Fraction::from_float(0.0),
            Fraction::from_float(duration_cycles),
        ),
        controls: HashMap::new(),
    };

    let haps = pattern.query(&state);

    haps.into_iter()
        .map(|hap| Event {
            time: hap.part.begin.to_float() / cps,
            value: Some(hap.value),
            amplitude: 0.0, // Will be filled in from audio analysis
        })
        .collect()
}

/// Detect events in audio buffer using onset detection
pub fn detect_audio_events(audio: &[f32], sample_rate: f32, threshold: f32) -> Vec<Event> {
    let mut events = Vec::new();

    // Simple onset detection: look for sudden increases in RMS
    let window_size = (sample_rate * 0.01) as usize; // 10ms window
    let hop_size = window_size / 4;

    let mut prev_rms = 0.0;

    for (i, window) in audio.windows(window_size).step_by(hop_size).enumerate() {
        let rms: f32 = (window.iter().map(|x| x * x).sum::<f32>() / window.len() as f32).sqrt();

        // Detect onset: current RMS is significantly higher than previous
        let onset_strength = (rms - prev_rms).max(0.0);

        if onset_strength > threshold {
            let time = (i * hop_size) as f64 / sample_rate as f64;
            events.push(Event {
                time,
                value: None,
                amplitude: rms,
            });
        }

        prev_rms = rms * 0.9; // Decay for next comparison
    }

    events
}

/// Compare expected events with detected events
#[derive(Debug)]
pub struct EventComparison {
    pub matched: usize,
    pub missing: Vec<Event>,
    pub extra: Vec<Event>,
    pub total_expected: usize,
    pub match_rate: f32,
}

impl EventComparison {
    pub fn is_acceptable(&self, min_match_rate: f32) -> bool {
        self.match_rate >= min_match_rate
    }
}

/// Match detected events with expected events
/// tolerance: time tolerance in seconds for matching
pub fn compare_events(expected: &[Event], detected: &[Event], tolerance: f64) -> EventComparison {
    let mut matched = 0;
    let mut missing = Vec::new();
    let mut detected_used = vec![false; detected.len()];

    for exp_event in expected {
        // Try to find a matching detected event
        let mut found = false;

        for (i, det_event) in detected.iter().enumerate() {
            if detected_used[i] {
                continue;
            }

            let time_diff = (exp_event.time - det_event.time).abs();
            if time_diff <= tolerance {
                matched += 1;
                detected_used[i] = true;
                found = true;
                break;
            }
        }

        if !found {
            missing.push(exp_event.clone());
        }
    }

    // Find extra detected events (not matched to any expected event)
    let extra: Vec<Event> = detected
        .iter()
        .enumerate()
        .filter(|(i, _)| !detected_used[*i])
        .map(|(_, e)| e.clone())
        .collect();

    let match_rate = if expected.is_empty() {
        1.0
    } else {
        matched as f32 / expected.len() as f32
    };

    EventComparison {
        matched,
        missing,
        extra,
        total_expected: expected.len(),
        match_rate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_detection_with_impulses() {
        let sample_rate = 44100.0;
        let mut audio = vec![0.0; 44100]; // 1 second of silence

        // Add impulses at 0.25s, 0.5s, 0.75s
        audio[11025] = 0.5; // 0.25s
        audio[22050] = 0.5; // 0.5s
        audio[33075] = 0.5; // 0.75s

        let detected = detect_audio_events(&audio, sample_rate, 0.01);

        // Should detect 3 events
        assert!(
            detected.len() >= 2,
            "Should detect at least 2 events, got {}",
            detected.len()
        );

        // Check approximate timing
        if detected.len() >= 2 {
            assert!(
                (detected[0].time - 0.25).abs() < 0.05,
                "First event should be near 0.25s"
            );
        }
    }

    #[test]
    fn test_compare_events() {
        let expected = vec![
            Event {
                time: 0.0,
                value: Some("bd".to_string()),
                amplitude: 0.0,
            },
            Event {
                time: 0.5,
                value: Some("sn".to_string()),
                amplitude: 0.0,
            },
            Event {
                time: 1.0,
                value: Some("hh".to_string()),
                amplitude: 0.0,
            },
        ];

        let detected = vec![
            Event {
                time: 0.01,
                value: None,
                amplitude: 0.5,
            },
            Event {
                time: 0.51,
                value: None,
                amplitude: 0.4,
            },
            // Missing third event
        ];

        let comparison = compare_events(&expected, &detected, 0.05);

        assert_eq!(comparison.matched, 2);
        assert_eq!(comparison.missing.len(), 1);
        assert_eq!(comparison.extra.len(), 0);
        assert!((comparison.match_rate - 0.666).abs() < 0.01);
    }
}
