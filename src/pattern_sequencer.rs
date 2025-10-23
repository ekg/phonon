#![allow(unused_assignments, unused_mut)]
//! Fixed Pattern Sequencer - Handles time-based triggering of pattern events correctly
//! 
//! This module sequences pattern events over time, triggering the appropriate
//! samples at the right moments during audio rendering.

use crate::pattern::{Pattern, Hap, State, TimeSpan, Fraction};
use crate::sample_loader::SampleBank;
use crate::sample_player_real::create_sample_player;
use fundsp::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Sequences pattern events and triggers samples
pub struct PatternSequencer {
    /// The pattern to sequence
    pattern: Pattern<String>,
    
    /// Sample bank for loading samples
    sample_bank: Arc<Mutex<SampleBank>>,
    
    /// Current position in samples since start
    global_sample_pos: usize,
    
    /// Sample rate
    sample_rate: f32,
    
    /// Samples per cycle (at 44100 Hz, 1 cycle = 1 second by default)
    samples_per_cycle: usize,
    
    /// Currently playing samples (unique key -> audio unit)
    active_samples: HashMap<String, Box<dyn AudioUnit>>,
    
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
            global_sample_pos: 0,
            sample_rate,
            samples_per_cycle,
            active_samples: HashMap::new(),
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
            eprintln!("Starting cycle {}", current_cycle);
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
            if event_start <= cycle_position && event_start > (cycle_position - 1.0/self.samples_per_cycle as f64) {
                let event_key = format!("{}_{}_{}", event.value, current_cycle as i32, event_start);
                if !self.triggered_events.contains(&event_key) {
                    self.trigger_sample(&event.value, current_cycle, event_start);
                    self.triggered_events.push(event_key);
                }
            }
        }
        
        // Mix all active samples
        let mut output = 0.0;
        let mut finished_samples = Vec::new();
        
        for (key, sample_player) in &mut self.active_samples {
            let mut sample_out = [0.0];
            sample_player.tick(&[], &mut sample_out);
            output += sample_out[0];
            
            // Check if sample is finished
            if sample_out[0].abs() < 0.0001 {
                finished_samples.push(key.clone());
            }
        }
        
        // Remove finished samples
        for key in finished_samples {
            self.active_samples.remove(&key);
        }
        
        // Increment global position
        self.global_sample_pos += 1;
        
        output
    }
    
    /// Trigger a sample by name
    fn trigger_sample(&mut self, sample_name: &str, cycle: f64, position: f64) {
        let mut bank = self.sample_bank.lock().unwrap();
        if let Some(samples) = bank.get_sample(sample_name) {
            eprintln!("Triggering '{}' at cycle {} pos {:.3}", sample_name, cycle, position);
            let player = create_sample_player(samples);
            
            // Use a unique key for this trigger
            let key = format!("{}_{:.0}_{:.3}", sample_name, cycle, position);
            self.active_samples.insert(key, player);
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
}

/// Create a pattern sequencer audio unit
pub fn create_pattern_sequencer(pattern_str: &str, sample_rate: f32) -> PatternSequencer {
    let pattern = Pattern::from_string(pattern_str);
    PatternSequencer::new(pattern, sample_rate)
}