//! Voice Manager - Handles polyphonic sample playback with voice allocation
//! 
//! Based on SuperCollider's architecture, this module manages multiple
//! simultaneous sample playback voices with proper position tracking.

use std::sync::Arc;

/// Maximum number of simultaneous voices
const MAX_VOICES: usize = 64;

/// A single voice that plays a sample
#[derive(Clone)]
pub struct Voice {
    /// The sample data to play
    sample_data: Option<Arc<Vec<f32>>>,
    
    /// Current playback position in the sample
    position: usize,
    
    /// Whether this voice is currently active
    active: bool,
    
    /// Gain for this voice
    gain: f32,
    
    /// Age counter for voice stealing (incremented each sample)
    age: usize,
}

impl Voice {
    pub fn new() -> Self {
        Self {
            sample_data: None,
            position: 0,
            active: false,
            gain: 1.0,
            age: 0,
        }
    }
    
    /// Start playing a sample
    pub fn trigger(&mut self, sample: Arc<Vec<f32>>, gain: f32) {
        self.sample_data = Some(sample);
        self.position = 0;
        self.active = true;
        self.gain = gain;
        self.age = 0;
    }
    
    /// Process one sample of audio
    pub fn process(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }
        
        if let Some(ref samples) = self.sample_data {
            if self.position < samples.len() {
                let output = samples[self.position] * self.gain;
                self.position += 1;
                self.age += 1;
                output
            } else {
                // Sample finished
                self.active = false;
                self.sample_data = None;
                0.0
            }
        } else {
            self.active = false;
            0.0
        }
    }
    
    /// Check if voice is available for allocation
    pub fn is_available(&self) -> bool {
        !self.active
    }
}

/// Manages a pool of voices for polyphonic playback
pub struct VoiceManager {
    /// Pool of available voices
    voices: Vec<Voice>,
    
    /// Next voice index for round-robin allocation
    next_voice_index: usize,
}

impl VoiceManager {
    pub fn new() -> Self {
        let mut voices = Vec::with_capacity(MAX_VOICES);
        for _ in 0..MAX_VOICES {
            voices.push(Voice::new());
        }
        
        Self {
            voices,
            next_voice_index: 0,
        }
    }
    
    /// Trigger a sample with automatic voice allocation
    pub fn trigger_sample(&mut self, sample: Arc<Vec<f32>>, gain: f32) {
        // Try to find an inactive voice
        for i in 0..MAX_VOICES {
            let idx = (self.next_voice_index + i) % MAX_VOICES;
            if self.voices[idx].is_available() {
                self.voices[idx].trigger(sample, gain);
                self.next_voice_index = (idx + 1) % MAX_VOICES;
                return;
            }
        }
        
        // All voices are active - steal the oldest one
        let mut oldest_idx = 0;
        let mut oldest_age = 0;
        
        for (idx, voice) in self.voices.iter().enumerate() {
            if voice.age > oldest_age {
                oldest_age = voice.age;
                oldest_idx = idx;
            }
        }
        
        // Steal the oldest voice
        self.voices[oldest_idx].trigger(sample, gain);
        self.next_voice_index = (oldest_idx + 1) % MAX_VOICES;
    }
    
    /// Process one sample from all active voices
    pub fn process(&mut self) -> f32 {
        let mut output = 0.0;
        
        for voice in &mut self.voices {
            output += voice.process();
        }
        
        // Apply some limiting to prevent clipping
        if output > 1.0 {
            output = output.tanh();
        } else if output < -1.0 {
            output = output.tanh();
        }
        
        output
    }
    
    /// Process a block of samples
    pub fn process_block(&mut self, size: usize) -> Vec<f32> {
        let mut output = Vec::with_capacity(size);
        
        for _ in 0..size {
            output.push(self.process());
        }
        
        output
    }
    
    /// Get number of active voices
    pub fn active_voice_count(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }
    
    /// Reset all voices
    pub fn reset(&mut self) {
        for voice in &mut self.voices {
            voice.active = false;
            voice.sample_data = None;
            voice.position = 0;
        }
        self.next_voice_index = 0;
    }
}