#![allow(unused_assignments, unused_mut)]
#![allow(dead_code)]
//! Pattern Sequencer with Voice Management
//!
//! Sequences pattern events and manages polyphonic sample playback
//! using a proper voice allocation system.

use crate::pattern::{Fraction, Pattern, State, TimeSpan};
use crate::sample_loader::SampleBank;
use crate::voice_manager::VoiceManager;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Sequences pattern events with voice-based polyphonic playback
pub struct PatternSequencer {
    /// The pattern to sequence
    pattern: Pattern<String>,

    /// Sample bank for loading samples
    sample_bank: Arc<Mutex<SampleBank>>,

    /// Voice manager for polyphonic playback
    voice_manager: VoiceManager,

    /// Current position in samples since start
    global_sample_pos: usize,

    /// Sample rate
    sample_rate: f32,

    /// Samples per cycle (at 44100 Hz, 1 cycle = 1 second by default)
    samples_per_cycle: usize,

    /// Track which events have been triggered in current cycle
    triggered_events: Vec<String>,
}

impl PatternSequencer {
    pub fn new(pattern: Pattern<String>, sample_rate: f32) -> Self {
        // Default to 1 cycle = 1 second
        let samples_per_cycle = sample_rate as usize;

        Self {
            pattern,
            sample_bank: Arc::new(Mutex::new(SampleBank::new())),
            voice_manager: VoiceManager::new(),
            global_sample_pos: 0,
            sample_rate,
            samples_per_cycle,
            triggered_events: Vec::new(),
        }
    }

    /// Process one sample of audio
    pub fn process_sample(&mut self) -> f32 {
        // Calculate current cycle and position within cycle
        let current_cycle = (self.global_sample_pos / self.samples_per_cycle) as f64;
        let sample_in_cycle = self.global_sample_pos % self.samples_per_cycle;
        let cycle_position = sample_in_cycle as f64 / self.samples_per_cycle as f64;

        // Check if we're starting a new cycle
        if sample_in_cycle == 0 {
            self.triggered_events.clear();
            eprintln!(
                "Cycle {}: {} active voices",
                current_cycle,
                self.voice_manager.active_voice_count()
            );
        }

        // Query pattern for current cycle
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(current_cycle),
                Fraction::from_float(current_cycle + 1.0),
            ),
            controls: HashMap::new(),
        };

        let events = self.pattern.query(&state);

        // Check which events should trigger at this position
        for event in events {
            let event_start = event.part.begin.to_float() - current_cycle;

            // Trigger if we've reached the event start time and haven't triggered it yet
            if event_start <= cycle_position
                && event_start > (cycle_position - 1.0 / self.samples_per_cycle as f64)
            {
                let event_key = format!(
                    "{}_{}_{:.3}",
                    event.value, current_cycle as i32, event_start
                );
                if !self.triggered_events.contains(&event_key) {
                    self.trigger_sample(&event.value);
                    self.triggered_events.push(event_key);
                }
            }
        }

        // Increment global position
        self.global_sample_pos += 1;

        // Process all voices and return mixed output
        self.voice_manager.process()
    }

    /// Trigger a sample by name
    fn trigger_sample(&mut self, sample_name: &str) {
        let mut bank = self.sample_bank.lock().unwrap();
        if let Some(samples) = bank.get_sample(sample_name) {
            eprintln!("Triggering '{}' ({} samples)", sample_name, samples.len());
            // Default gain of 0.8 to prevent clipping with multiple samples
            self.voice_manager.trigger_sample(samples, 0.8);
        } else {
            eprintln!("Warning: Sample '{sample_name}' not found");
        }
    }

    /// Process a block of samples
    pub fn process_block(&mut self, size: usize) -> Vec<f32> {
        let mut output = Vec::with_capacity(size);
        for _ in 0..size {
            output.push(self.process_sample());
        }
        output
    }

    /// Reset the sequencer
    pub fn reset(&mut self) {
        self.global_sample_pos = 0;
        self.triggered_events.clear();
        self.voice_manager.reset();
    }
}

/// Create a pattern sequencer
pub fn create_pattern_sequencer(pattern_str: &str, sample_rate: f32) -> PatternSequencer {
    let pattern = Pattern::from_string(pattern_str);
    PatternSequencer::new(pattern, sample_rate)
}
