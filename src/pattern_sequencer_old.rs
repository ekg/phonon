//! Pattern Sequencer - Handles time-based triggering of pattern events
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
    
    /// Current time in cycles
    current_cycle: f64,
    
    /// Samples per cycle (at 44100 Hz, 1 cycle = 1 second by default)
    samples_per_cycle: usize,
    
    /// Current sample position within the cycle
    current_sample: usize,
    
    /// Currently playing samples (sample name -> audio unit)
    active_samples: HashMap<String, Box<dyn AudioUnit>>,
    
    /// Events for the current cycle
    current_events: Vec<Hap<String>>,
    
    /// Next event index to trigger
    next_event_index: usize,
}

impl PatternSequencer {
    pub fn new(pattern: Pattern<String>, sample_rate: f32) -> Self {
        // Default to 1 cycle = 1 second
        let samples_per_cycle = sample_rate as usize;
        
        Self {
            pattern,
            sample_bank: Arc::new(Mutex::new(SampleBank::new())),
            current_cycle: 0.0,
            samples_per_cycle,
            current_sample: 0,
            active_samples: HashMap::new(),
            current_events: Vec::new(),
            next_event_index: 0,
        }
    }
    
    /// Query events for the current cycle
    fn update_cycle_events(&mut self) {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(self.current_cycle),
                Fraction::from_float(self.current_cycle + 1.0),
            ),
            controls: HashMap::new(),
        };
        
        self.current_events = self.pattern.query(&state);
        eprintln!("Cycle {}: {} events", self.current_cycle, self.current_events.len());
        for event in &self.current_events {
            eprintln!("  Event: {} at {}-{}", event.value, 
                event.part.begin.to_float(), event.part.end.to_float());
        }
        self.current_events.sort_by(|a, b| {
            a.part.begin.partial_cmp(&b.part.begin).unwrap()
        });
        self.next_event_index = 0;
    }
    
    /// Process one sample of audio
    pub fn process_sample(&mut self) -> f32 {
        // Check if we need to move to the next cycle
        if self.current_sample >= self.samples_per_cycle {
            self.current_sample = 0;
            self.current_cycle += 1.0;
            self.update_cycle_events();
        }
        
        // Check if we need to trigger any events
        let cycle_position = self.current_sample as f64 / self.samples_per_cycle as f64;
        
        // Collect events to trigger
        let mut events_to_trigger = Vec::new();
        while self.next_event_index < self.current_events.len() {
            let event = &self.current_events[self.next_event_index];
            let event_time = event.part.begin.to_float() - self.current_cycle.floor();
            
            eprintln!("Sample {}/{}: pos={:.3}, event_time={:.3}, event={}", 
                self.current_sample, self.samples_per_cycle, cycle_position, event_time, event.value);
            
            if event_time <= cycle_position {
                events_to_trigger.push(event.value.clone());
                self.next_event_index += 1;
            } else {
                break;
            }
        }
        
        // Trigger collected events
        for sample_name in events_to_trigger {
            self.trigger_sample(&sample_name);
        }
        
        // Mix all active samples
        let mut output = 0.0;
        let mut finished_samples = Vec::new();
        
        for (name, sample_player) in &mut self.active_samples {
            let mut sample_out = [0.0];
            sample_player.tick(&[], &mut sample_out);
            output += sample_out[0];
            
            // Check if sample is finished (simplified - in reality we'd track duration)
            // For now, samples play once through
            if sample_out[0].abs() < 0.0001 {
                finished_samples.push(name.clone());
            }
        }
        
        // Remove finished samples
        for name in finished_samples {
            self.active_samples.remove(&name);
        }
        
        self.current_sample += 1;
        output
    }
    
    /// Trigger a sample by name
    fn trigger_sample(&mut self, sample_name: &str) {
        let mut bank = self.sample_bank.lock().unwrap();
        if let Some(samples) = bank.get_sample(sample_name) {
            let preview_len = std::cmp::min(5, samples.len());
            eprintln!("Triggering sample: '{}' ({} samples, first: {:?})", 
                sample_name, samples.len(), &samples[..preview_len]);
            let player = create_sample_player(samples);
            
            // Use a unique key for overlapping samples
            let key = format!("{}_{}_{}", sample_name, self.current_cycle as i32, self.current_sample);
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
    let mut sequencer = PatternSequencer::new(pattern, sample_rate);
    sequencer.update_cycle_events();
    sequencer
}