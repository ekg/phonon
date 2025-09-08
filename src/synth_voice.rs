//! Synth voice system for triggering synthesis from patterns
//!
//! Provides voice allocation and management for polyphonic synth triggering

use crate::envelope::{ADSREnvelope, PercEnvelope};
use crate::glicol_dsp::{DspChain, DspNode};
use std::collections::HashMap;

/// A single synth voice that can be triggered
#[derive(Clone)]
pub struct SynthVoice {
    /// The DSP chain for this voice
    pub chain: DspChain,
    
    /// Envelope for amplitude control
    pub envelope: PercEnvelope,
    
    /// Current frequency (for oscillators)
    pub frequency: f32,
    
    /// Voice ID for tracking
    pub id: usize,
    
    /// Is this voice currently active?
    pub active: bool,
    
    /// Sample rate
    sample_rate: f32,
}

impl SynthVoice {
    pub fn new(id: usize, sample_rate: f32) -> Self {
        Self {
            chain: DspChain::new(),
            envelope: PercEnvelope::new(sample_rate),
            frequency: 440.0,
            id,
            active: false,
            sample_rate,
        }
    }
    
    /// Set the DSP chain for this voice
    pub fn set_chain(&mut self, chain: DspChain) {
        self.chain = chain;
    }
    
    /// Trigger the voice with optional frequency
    pub fn trigger(&mut self, frequency: Option<f32>) {
        if let Some(freq) = frequency {
            self.frequency = freq;
            // Update oscillator frequency if present
            self.update_frequency(freq);
        }
        
        self.envelope.trigger();
        self.active = true;
    }
    
    /// Update oscillator frequency in the chain
    fn update_frequency(&mut self, freq: f32) {
        // Find and update any oscillator nodes
        for node in &mut self.chain.nodes {
            match node {
                DspNode::Sin { freq: f } |
                DspNode::Saw { freq: f } |
                DspNode::Triangle { freq: f } |
                DspNode::Impulse { freq: f } => {
                    *f = freq;
                },
                DspNode::Square { freq: f, duty: _ } => {
                    *f = freq;
                },
                _ => {}
            }
        }
    }
    
    /// Process one sample
    pub fn process(&mut self, time: f64) -> f32 {
        if !self.active {
            return 0.0;
        }
        
        // Get envelope value
        let env = self.envelope.process();
        
        // Check if voice is done
        if !self.envelope.is_active() {
            self.active = false;
            return 0.0;
        }
        
        // Simple sine wave generation for now
        // TODO: Process the full DSP chain when available
        let sample = if !self.chain.nodes.is_empty() {
            match &self.chain.nodes[0] {
                DspNode::Sin { freq } => {
                    let phase = (time * *freq as f64 * 2.0 * std::f64::consts::PI) % (2.0 * std::f64::consts::PI);
                    phase.sin() as f32
                },
                DspNode::Saw { freq } => {
                    let phase = (time * *freq as f64) % 1.0;
                    (2.0 * phase - 1.0) as f32
                },
                DspNode::Square { freq, duty: _ } => {
                    let phase = (time * *freq as f64) % 1.0;
                    if phase < 0.5 { 1.0 } else { -1.0 }
                },
                DspNode::Noise { seed: _ } => {
                    // Simple white noise
                    (rand::random::<f32>() * 2.0) - 1.0
                },
                _ => 0.0,
            }
        } else {
            // Default sine wave
            (time * self.frequency as f64 * 2.0 * std::f64::consts::PI).sin() as f32
        };
        
        // Apply envelope
        sample * env * 0.5  // Scale down to prevent clipping
    }
    
    /// Generate a buffer of samples
    pub fn generate(&mut self, num_samples: usize, start_time: f64) -> Vec<f32> {
        let mut output = Vec::with_capacity(num_samples);
        let dt = 1.0 / self.sample_rate as f64;
        
        for i in 0..num_samples {
            let time = start_time + (i as f64 * dt);
            output.push(self.process(time));
        }
        
        output
    }
}

/// Voice allocator for managing multiple synth voices
pub struct VoiceAllocator {
    /// Pool of available voices
    voices: Vec<SynthVoice>,
    
    /// Map from channel names to DSP chains
    pub channel_chains: HashMap<String, DspChain>,
    
    /// Next voice to allocate (round-robin)
    next_voice: usize,
    
    /// Maximum polyphony
    max_voices: usize,
    
    sample_rate: f32,
}

impl VoiceAllocator {
    pub fn new(max_voices: usize, sample_rate: f32) -> Self {
        let mut voices = Vec::with_capacity(max_voices);
        for i in 0..max_voices {
            voices.push(SynthVoice::new(i, sample_rate));
        }
        
        Self {
            voices,
            channel_chains: HashMap::new(),
            next_voice: 0,
            max_voices,
            sample_rate,
        }
    }
    
    /// Register a channel with its DSP chain
    pub fn register_channel(&mut self, name: String, chain: DspChain) {
        self.channel_chains.insert(name, chain);
    }
    
    /// Trigger a synth by channel name
    pub fn trigger_channel(&mut self, channel: &str, frequency: Option<f32>) -> Option<usize> {
        // Look up the chain for this channel
        let chain = self.channel_chains.get(channel)?.clone();
        
        // Find an available voice (steal oldest if necessary)
        let voice_idx = self.allocate_voice();
        
        // Set up the voice
        self.voices[voice_idx].set_chain(chain);
        self.voices[voice_idx].trigger(frequency);
        
        Some(voice_idx)
    }
    
    /// Allocate a voice using round-robin with voice stealing
    fn allocate_voice(&mut self) -> usize {
        // First try to find an inactive voice
        for i in 0..self.max_voices {
            let idx = (self.next_voice + i) % self.max_voices;
            if !self.voices[idx].active {
                self.next_voice = (idx + 1) % self.max_voices;
                return idx;
            }
        }
        
        // All voices active, steal the next one
        let idx = self.next_voice;
        self.next_voice = (self.next_voice + 1) % self.max_voices;
        idx
    }
    
    /// Process all active voices and mix
    pub fn process(&mut self, time: f64) -> f32 {
        let mut output = 0.0;
        
        for voice in &mut self.voices {
            if voice.active {
                output += voice.process(time);
            }
        }
        
        // Prevent clipping with simple limiting
        output.clamp(-1.0, 1.0)
    }
    
    /// Generate a buffer of mixed samples
    pub fn generate(&mut self, num_samples: usize, start_time: f64) -> Vec<f32> {
        let mut output = vec![0.0; num_samples];
        let dt = 1.0 / self.sample_rate as f64;
        
        // Process each voice
        for voice in &mut self.voices {
            if voice.active {
                for i in 0..num_samples {
                    let time = start_time + (i as f64 * dt);
                    output[i] += voice.process(time);
                }
            }
        }
        
        // Apply limiting
        for sample in &mut output {
            *sample = sample.clamp(-1.0, 1.0);
        }
        
        output
    }
    
    /// Get the number of active voices
    pub fn active_voice_count(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glicol_dsp::DspNode;
    
    #[test]
    fn test_synth_voice_trigger() {
        let sample_rate = 44100.0;
        let mut voice = SynthVoice::new(0, sample_rate);
        
        // Create a simple sine wave chain
        let mut chain = DspChain::new();
        chain.nodes.push(DspNode::Sin { freq: 440.0 });
        
        voice.set_chain(chain);
        voice.envelope.set_times(0.001, 0.05);
        
        // Trigger the voice
        voice.trigger(Some(440.0));
        assert!(voice.active);
        
        // Generate some samples
        let samples = voice.generate(1000, 0.0);
        
        // Should produce non-zero output
        let max_val = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(max_val > 0.0, "Voice should produce audio");
        
        // Process until envelope finishes
        for _ in 0..10000 {
            voice.process(0.0);
        }
        
        // Should be inactive
        assert!(!voice.active, "Voice should be inactive after envelope");
    }
    
    #[test]
    fn test_voice_allocator() {
        let sample_rate = 44100.0;
        let mut allocator = VoiceAllocator::new(4, sample_rate);
        
        // Create a test chain
        let mut chain = DspChain::new();
        chain.nodes.push(DspNode::Sin { freq: 440.0 });
        
        // Register channels
        allocator.register_channel("bass".to_string(), chain.clone());
        allocator.register_channel("lead".to_string(), chain.clone());
        
        // Trigger some voices
        let v1 = allocator.trigger_channel("bass", Some(110.0));
        assert!(v1.is_some());
        
        let v2 = allocator.trigger_channel("lead", Some(440.0));
        assert!(v2.is_some());
        
        assert_eq!(allocator.active_voice_count(), 2);
        
        // Generate audio
        let samples = allocator.generate(100, 0.0);
        assert_eq!(samples.len(), 100);
        
        // Should have non-zero output
        let max_val = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(max_val > 0.0, "Should produce audio");
    }
    
    #[test]
    fn test_voice_stealing() {
        let sample_rate = 44100.0;
        let mut allocator = VoiceAllocator::new(2, sample_rate);  // Only 2 voices
        
        let mut chain = DspChain::new();
        chain.nodes.push(DspNode::Sin { freq: 440.0 });
        
        allocator.register_channel("test".to_string(), chain);
        
        // Trigger 3 voices (should steal the oldest)
        allocator.trigger_channel("test", Some(220.0));
        allocator.trigger_channel("test", Some(330.0));
        allocator.trigger_channel("test", Some(440.0));  // This should steal the first
        
        // Should still only have 2 active
        assert!(allocator.active_voice_count() <= 2);
    }
}