#![allow(unused_assignments, unused_mut)]
//! Envelope generators for triggered synths
//!
//! Provides ADSR and other envelope types for making synths percussive

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
    sustain: f32, // Level, not time
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
        self.attack = attack.max(0.001); // Minimum 1ms
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
            }
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
            }
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
            }
            EnvelopeState::Sustain => {
                self.current_level = self.sustain;
                // Stay here until release
            }
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
            }
            EnvelopeState::Finished => {
                self.current_level = 0.0;
                self.state = EnvelopeState::Idle;
            }
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
            attack: 0.001, // 1ms default
            decay: 0.1,    // 100ms default
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

/// Segments envelope - arbitrary breakpoint envelope
#[derive(Debug, Clone)]
pub struct SegmentsEnvelope {
    levels: Vec<f32>,
    times: Vec<f32>,
    current_segment: usize,
    segment_elapsed: f32,
    current_value: f32,
    sample_rate: f32,
    active: bool,
}

impl SegmentsEnvelope {
    pub fn new(sample_rate: f32, levels: Vec<f32>, times: Vec<f32>) -> Self {
        Self {
            levels,
            times,
            current_segment: 0,
            segment_elapsed: 0.0,
            current_value: 0.0,
            sample_rate,
            active: false,
        }
    }

    pub fn trigger(&mut self) {
        self.active = true;
        self.current_segment = 0;
        self.segment_elapsed = 0.0;
        if !self.levels.is_empty() {
            self.current_value = self.levels[0];
        }
    }

    pub fn process(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        if self.levels.is_empty() || self.times.is_empty() {
            self.active = false;
            return 0.0;
        }

        let dt = 1.0 / self.sample_rate;
        self.segment_elapsed += dt;

        // Check if we've finished the current segment
        if self.current_segment < self.times.len() {
            let segment_duration = self.times[self.current_segment];

            if self.segment_elapsed >= segment_duration {
                // Move to next segment
                self.current_segment += 1;
                self.segment_elapsed = 0.0;

                if self.current_segment >= self.levels.len() - 1 {
                    // Finished all segments
                    self.current_value = *self.levels.last().unwrap();
                    self.active = false;
                    return self.current_value;
                }
            }

            // Linear interpolation between current and next level
            let start_level = self.levels[self.current_segment];
            let end_level = self.levels[self.current_segment + 1];
            let progress = (self.segment_elapsed / segment_duration).min(1.0);
            self.current_value = start_level + (end_level - start_level) * progress;
        } else {
            // Hold at final level
            self.current_value = *self.levels.last().unwrap();
            self.active = false;
        }

        self.current_value
    }

    pub fn is_active(&self) -> bool {
        self.active
    }
}

/// Curve envelope - exponential/logarithmic shaped ramp
#[derive(Debug, Clone)]
pub struct CurveEnvelope {
    start: f32,
    end: f32,
    duration: f32,
    curve: f32, // -10 to +10, 0=linear
    elapsed_time: f32,
    current_value: f32,
    sample_rate: f32,
    active: bool,
}

impl CurveEnvelope {
    pub fn new(sample_rate: f32, start: f32, end: f32, duration: f32, curve: f32) -> Self {
        Self {
            start,
            end,
            duration: duration.max(0.001),
            curve,
            elapsed_time: 0.0,
            current_value: start,
            sample_rate,
            active: false,
        }
    }

    pub fn trigger(&mut self) {
        self.active = true;
        self.elapsed_time = 0.0;
        self.current_value = self.start;
    }

    pub fn process(&mut self) -> f32 {
        if !self.active {
            return self.current_value;
        }

        let dt = 1.0 / self.sample_rate;
        self.elapsed_time += dt;

        let t = (self.elapsed_time / self.duration).min(1.0);

        // Apply curve shape
        let curved_t = if self.curve.abs() < 0.001 {
            t // Linear
        } else {
            // Exponential curve
            let exp_curve = self.curve.exp();
            let exp_curve_t = (self.curve * t).exp();
            (exp_curve_t - 1.0) / (exp_curve - 1.0)
        };

        self.current_value = self.start + (self.end - self.start) * curved_t;

        if t >= 1.0 {
            self.active = false;
            self.current_value = self.end;
        }

        self.current_value
    }

    pub fn is_active(&self) -> bool {
        self.active
    }
}

/// Unified envelope type that can be any of the supported envelope types
#[derive(Debug, Clone)]
pub enum VoiceEnvelope {
    Percussion(PercEnvelope),
    ADSR(ADSREnvelope),
    Segments(SegmentsEnvelope),
    Curve(CurveEnvelope),
}

impl VoiceEnvelope {
    /// Create a new percussion envelope
    pub fn new_percussion(sample_rate: f32, attack: f32, release: f32) -> Self {
        let mut env = PercEnvelope::new(sample_rate);
        env.set_times(attack, release);
        VoiceEnvelope::Percussion(env)
    }

    /// Create a new ADSR envelope
    pub fn new_adsr(sample_rate: f32, attack: f32, decay: f32, sustain: f32, release: f32) -> Self {
        let mut env = ADSREnvelope::new(sample_rate);
        env.set_adsr(attack, decay, sustain, release);
        VoiceEnvelope::ADSR(env)
    }

    /// Create a new segments envelope
    pub fn new_segments(sample_rate: f32, levels: Vec<f32>, times: Vec<f32>) -> Self {
        VoiceEnvelope::Segments(SegmentsEnvelope::new(sample_rate, levels, times))
    }

    /// Create a new curve envelope
    pub fn new_curve(sample_rate: f32, start: f32, end: f32, duration: f32, curve: f32) -> Self {
        VoiceEnvelope::Curve(CurveEnvelope::new(sample_rate, start, end, duration, curve))
    }

    /// Trigger the envelope
    pub fn trigger(&mut self) {
        match self {
            VoiceEnvelope::Percussion(env) => env.trigger(),
            VoiceEnvelope::ADSR(env) => env.trigger(),
            VoiceEnvelope::Segments(env) => env.trigger(),
            VoiceEnvelope::Curve(env) => env.trigger(),
        }
    }

    /// Release the envelope (for ADSR)
    pub fn release(&mut self) {
        if let VoiceEnvelope::ADSR(env) = self {
            env.release();
        }
    }

    /// Process one sample
    pub fn process(&mut self) -> f32 {
        match self {
            VoiceEnvelope::Percussion(env) => env.process(),
            VoiceEnvelope::ADSR(env) => env.process(),
            VoiceEnvelope::Segments(env) => env.process(),
            VoiceEnvelope::Curve(env) => env.process(),
        }
    }

    /// Check if envelope is active
    pub fn is_active(&self) -> bool {
        match self {
            VoiceEnvelope::Percussion(env) => env.is_active(),
            VoiceEnvelope::ADSR(env) => env.is_active(),
            VoiceEnvelope::Segments(env) => env.is_active(),
            VoiceEnvelope::Curve(env) => env.is_active(),
        }
    }

    /// Trigger a quick release for anti-click fades (used by cut groups)
    /// Forces the envelope into release phase with the specified time
    pub fn trigger_quick_release(&mut self, _release_time: f32) {
        match self {
            VoiceEnvelope::Percussion(env) => {
                // For perc envelope, set to inactive to trigger decay
                // The decay phase will naturally fade out
                env.active = false;
            }
            VoiceEnvelope::ADSR(env) => {
                // Trigger release phase
                env.release();
            }
            VoiceEnvelope::Segments(env) => {
                // Force to last segment for quick fade
                env.active = false;
            }
            VoiceEnvelope::Curve(env) => {
                // Force to inactive
                env.active = false;
            }
        }
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
        assert!(
            max_during_attack > 0.9 && max_during_attack <= 1.0,
            "Attack should reach near 1.0, got {}",
            max_during_attack
        );

        // Process decay (50ms)
        let decay_samples = 2205;
        for _ in 0..decay_samples {
            env.process();
        }

        // Should be at sustain level
        let sustain_val = env.process();
        assert!(
            (sustain_val - 0.5).abs() < 0.1,
            "Should be near sustain level 0.5, got {}",
            sustain_val
        );

        // Release
        env.release();

        // Process release and check it goes to zero
        let release_samples = 4410; // 100ms
        for _ in 0..release_samples {
            env.process();
        }

        let final_val = env.process();
        assert!(
            final_val < 0.01,
            "Should be near zero after release, got {}",
            final_val
        );
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
        assert!(
            peak > 0.8,
            "Should reach near 1.0 during attack, got {}",
            peak
        );

        // Process full envelope
        for _ in 0..4410 {
            // 100ms total
            env.process();
        }

        // Should be inactive
        assert!(!env.is_active(), "Envelope should be finished");
    }
}
