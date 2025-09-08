//! Envelope generators for triggered synths
//! 
//! Provides ADSR and other envelope types for making synths percussive

use std::f32::consts::PI;

/// Envelope state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EnvelopeState {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
    Finished,
}

/// ADSR Envelope generator
#[derive(Debug, Clone)]
pub struct ADSREnvelope {
    // Parameters (in seconds)
    attack: f32,
    decay: f32,
    sustain: f32,  // Level, not time
    release: f32,
    
    // State
    state: EnvelopeState,
    current_level: f32,
    time_in_state: f32,
    sample_rate: f32,
    
    // Trigger management
    gate: bool,
}

impl ADSREnvelope {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            attack: 0.01,
            decay: 0.1,
            sustain: 0.7,
            release: 0.2,
            state: EnvelopeState::Idle,
            current_level: 0.0,
            time_in_state: 0.0,
            sample_rate,
            gate: false,
        }
    }
    
    /// Set ADSR parameters
    pub fn set_adsr(&mut self, attack: f32, decay: f32, sustain: f32, release: f32) {
        self.attack = attack.max(0.001);  // Minimum 1ms
        self.decay = decay.max(0.001);
        self.sustain = sustain.clamp(0.0, 1.0);
        self.release = release.max(0.001);
    }
    
    /// Trigger the envelope (note on)
    pub fn trigger(&mut self) {
        self.gate = true;
        self.state = EnvelopeState::Attack;
        self.time_in_state = 0.0;
        // Don't reset current_level to allow retriggering
    }
    
    /// Release the envelope (note off)
    pub fn release(&mut self) {
        if self.gate {
            self.gate = false;
            self.state = EnvelopeState::Release;
            self.time_in_state = 0.0;
        }
    }
    
    /// Process one sample
    pub fn process(&mut self) -> f32 {
        let dt = 1.0 / self.sample_rate;
        
        match self.state {
            EnvelopeState::Idle => {
                self.current_level = 0.0;
            },
            EnvelopeState::Attack => {
                self.time_in_state += dt;
                if self.time_in_state >= self.attack {
                    self.state = EnvelopeState::Decay;
                    self.time_in_state = 0.0;
                    self.current_level = 1.0;
                } else {
                    // Linear attack
                    self.current_level = self.time_in_state / self.attack;
                }
            },
            EnvelopeState::Decay => {
                self.time_in_state += dt;
                if self.time_in_state >= self.decay {
                    self.state = EnvelopeState::Sustain;
                    self.time_in_state = 0.0;
                    self.current_level = self.sustain;
                } else {
                    // Exponential decay
                    let progress = self.time_in_state / self.decay;
                    self.current_level = 1.0 + (self.sustain - 1.0) * progress;
                }
            },
            EnvelopeState::Sustain => {
                self.current_level = self.sustain;
                // Stay here until release
            },
            EnvelopeState::Release => {
                self.time_in_state += dt;
                if self.time_in_state >= self.release {
                    self.state = EnvelopeState::Finished;
                    self.current_level = 0.0;
                } else {
                    // Exponential release
                    let progress = self.time_in_state / self.release;
                    let start_level = self.sustain;
                    self.current_level = start_level * (1.0 - progress);
                }
            },
            EnvelopeState::Finished => {
                self.current_level = 0.0;
                self.state = EnvelopeState::Idle;
            },
        }
        
        self.current_level
    }
    
    /// Generate a buffer of envelope values
    pub fn generate(&mut self, num_samples: usize) -> Vec<f32> {
        let mut output = Vec::with_capacity(num_samples);
        for _ in 0..num_samples {
            output.push(self.process());
        }
        output
    }
    
    /// Check if envelope is active
    pub fn is_active(&self) -> bool {
        self.state != EnvelopeState::Idle && self.state != EnvelopeState::Finished
    }
}

/// Simple percussive envelope (attack-decay only)
#[derive(Debug, Clone)]
pub struct PercEnvelope {
    attack: f32,
    decay: f32,
    current_level: f32,
    time: f32,
    sample_rate: f32,
    active: bool,
}

impl PercEnvelope {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            attack: 0.001,  // 1ms default
            decay: 0.1,     // 100ms default
            current_level: 0.0,
            time: 0.0,
            sample_rate,
            active: false,
        }
    }
    
    pub fn set_times(&mut self, attack: f32, decay: f32) {
        self.attack = attack.max(0.0001);
        self.decay = decay.max(0.001);
    }
    
    pub fn trigger(&mut self) {
        self.active = true;
        self.time = 0.0;
    }
    
    pub fn process(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }
        
        let dt = 1.0 / self.sample_rate;
        self.time += dt;
        
        if self.time < self.attack {
            // Attack phase
            self.current_level = self.time / self.attack;
        } else if self.time < (self.attack + self.decay) {
            // Decay phase
            let decay_time = self.time - self.attack;
            let decay_progress = decay_time / self.decay;
            self.current_level = 1.0 * (-5.0 * decay_progress).exp();
        } else {
            // Finished
            self.current_level = 0.0;
            self.active = false;
        }
        
        self.current_level
    }
    
    pub fn is_active(&self) -> bool {
        self.active
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_adsr_envelope() {
        let sample_rate = 44100.0;
        let mut env = ADSREnvelope::new(sample_rate);
        env.set_adsr(0.01, 0.05, 0.5, 0.1);
        
        // Trigger the envelope
        env.trigger();
        
        // Collect samples for attack phase (10ms = 441 samples)
        let attack_samples = 441;
        let mut max_during_attack = 0.0f32;
        for _ in 0..attack_samples {
            let val = env.process();
            max_during_attack = max_during_attack.max(val);
        }
        
        // Should reach close to 1.0 at end of attack
        assert!(max_during_attack > 0.9 && max_during_attack <= 1.0,
                "Attack should reach near 1.0, got {}", max_during_attack);
        
        // Process decay (50ms)
        let decay_samples = 2205;
        for _ in 0..decay_samples {
            env.process();
        }
        
        // Should be at sustain level
        let sustain_val = env.process();
        assert!((sustain_val - 0.5).abs() < 0.1,
                "Should be near sustain level 0.5, got {}", sustain_val);
        
        // Release
        env.release();
        
        // Process release and check it goes to zero
        let release_samples = 4410; // 100ms
        for _ in 0..release_samples {
            env.process();
        }
        
        let final_val = env.process();
        assert!(final_val < 0.01, "Should be near zero after release, got {}", final_val);
    }
    
    #[test]
    fn test_perc_envelope() {
        let sample_rate = 44100.0;
        let mut env = PercEnvelope::new(sample_rate);
        env.set_times(0.001, 0.05);
        
        env.trigger();
        
        // Should start at 0
        assert!(!env.is_active() || env.current_level == 0.0);
        
        // Process attack (1ms = 44 samples)
        let mut peak = 0.0f32;
        for _ in 0..44 {
            peak = peak.max(env.process());
        }
        
        // Should reach peak
        assert!(peak > 0.8, "Should reach near 1.0 during attack, got {}", peak);
        
        // Process full envelope
        for _ in 0..4410 {  // 100ms total
            env.process();
        }
        
        // Should be inactive
        assert!(!env.is_active(), "Envelope should be finished");
    }
}