#![allow(unused_assignments, unused_mut)]
#![allow(dead_code)]
//! Polyphonic synth voice manager
//!
//! Manages up to 64 simultaneous synthesizer voices with per-voice ADSR envelopes.
//! Each voice can play a different frequency with independent envelope control.

use std::f32::consts::PI;

const DEFAULT_MAX_VOICES: usize = 256;

/// Waveform types for oscillators
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SynthWaveform {
    Sine,
    Saw,
    Square,
    Triangle,
}

/// ADSR envelope parameters
#[derive(Debug, Clone, Copy)]
pub struct ADSRParams {
    pub attack: f32,  // Attack time in seconds
    pub decay: f32,   // Decay time in seconds
    pub sustain: f32, // Sustain level (0.0-1.0)
    pub release: f32, // Release time in seconds
}

impl Default for ADSRParams {
    fn default() -> Self {
        Self {
            attack: 0.01,
            decay: 0.1,
            sustain: 0.7,
            release: 0.2,
        }
    }
}

/// ADSR envelope state
#[derive(Debug, Clone, Copy, PartialEq)]
enum EnvelopePhase {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// A single synthesizer voice
struct SynthVoice {
    // Oscillator state
    phase: f32,
    frequency: f32,
    waveform: SynthWaveform,

    // Envelope state
    envelope_phase: EnvelopePhase,
    envelope_level: f32,
    time_in_phase: f32,
    release_start_level: f32,

    // ADSR parameters
    adsr: ADSRParams,

    // Voice parameters
    gain: f32,
    pan: f32,

    // Lifetime
    age: usize, // How many samples since triggered
    is_active: bool,
}

impl SynthVoice {
    fn new() -> Self {
        Self {
            phase: 0.0,
            frequency: 440.0,
            waveform: SynthWaveform::Sine,
            envelope_phase: EnvelopePhase::Idle,
            envelope_level: 0.0,
            time_in_phase: 0.0,
            release_start_level: 0.0,
            adsr: ADSRParams::default(),
            gain: 1.0,
            pan: 0.0,
            age: 0,
            is_active: false,
        }
    }

    /// Trigger the voice with a new note
    fn trigger(
        &mut self,
        frequency: f32,
        waveform: SynthWaveform,
        adsr: ADSRParams,
        gain: f32,
        pan: f32,
    ) {
        self.frequency = frequency;
        self.waveform = waveform;
        self.adsr = adsr;
        self.gain = gain;
        self.pan = pan;

        // Reset envelope
        self.envelope_phase = EnvelopePhase::Attack;
        self.envelope_level = 0.0;
        self.time_in_phase = 0.0;
        self.release_start_level = 0.0;

        // Reset oscillator phase for consistent sound
        self.phase = 0.0;

        self.age = 0;
        self.is_active = true;
    }

    /// Release the voice (start release phase)
    fn release(&mut self) {
        if matches!(
            self.envelope_phase,
            EnvelopePhase::Attack | EnvelopePhase::Decay | EnvelopePhase::Sustain
        ) {
            self.release_start_level = self.envelope_level;
            self.envelope_phase = EnvelopePhase::Release;
            self.time_in_phase = 0.0;
        }
    }

    /// Process one sample
    fn process(&mut self, sample_rate: f32) -> f32 {
        if !self.is_active {
            return 0.0;
        }

        // Update envelope
        let dt = 1.0 / sample_rate;
        self.time_in_phase += dt;

        match self.envelope_phase {
            EnvelopePhase::Attack => {
                if self.adsr.attack > 0.0 {
                    self.envelope_level = self.time_in_phase / self.adsr.attack;
                    if self.envelope_level >= 1.0 {
                        self.envelope_level = 1.0;
                        self.envelope_phase = EnvelopePhase::Decay;
                        self.time_in_phase = 0.0;
                    }
                } else {
                    self.envelope_level = 1.0;
                    self.envelope_phase = EnvelopePhase::Decay;
                    self.time_in_phase = 0.0;
                }
            }
            EnvelopePhase::Decay => {
                if self.adsr.decay > 0.0 {
                    self.envelope_level =
                        1.0 - (1.0 - self.adsr.sustain) * (self.time_in_phase / self.adsr.decay);
                    if self.envelope_level <= self.adsr.sustain {
                        self.envelope_level = self.adsr.sustain;
                        self.envelope_phase = EnvelopePhase::Sustain;
                        self.time_in_phase = 0.0;
                    }
                } else {
                    self.envelope_level = self.adsr.sustain;
                    self.envelope_phase = EnvelopePhase::Sustain;
                    self.time_in_phase = 0.0;
                }
            }
            EnvelopePhase::Sustain => {
                self.envelope_level = self.adsr.sustain;
            }
            EnvelopePhase::Release => {
                if self.adsr.release > 0.0 {
                    let progress = (self.time_in_phase / self.adsr.release).min(1.0);
                    self.envelope_level = self.release_start_level * (1.0 - progress);

                    if progress >= 1.0 {
                        self.envelope_level = 0.0;
                        self.envelope_phase = EnvelopePhase::Idle;
                        self.is_active = false;
                        return 0.0;
                    }
                } else {
                    self.envelope_level = 0.0;
                    self.envelope_phase = EnvelopePhase::Idle;
                    self.is_active = false;
                    return 0.0;
                }
            }
            EnvelopePhase::Idle => {
                self.is_active = false;
                return 0.0;
            }
        }

        // Generate oscillator sample
        let osc_sample = match self.waveform {
            SynthWaveform::Sine => (2.0 * PI * self.phase).sin(),
            SynthWaveform::Saw => 2.0 * self.phase - 1.0,
            SynthWaveform::Square => {
                if self.phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            SynthWaveform::Triangle => {
                if self.phase < 0.5 {
                    4.0 * self.phase - 1.0
                } else {
                    3.0 - 4.0 * self.phase
                }
            }
        };

        // Update phase
        self.phase += self.frequency / sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        // Increment age
        self.age += 1;

        // Apply envelope and gain
        osc_sample * self.envelope_level * self.gain
    }
}

/// Manager for polyphonic synthesizer voices
pub struct SynthVoiceManager {
    voices: Vec<SynthVoice>,
    sample_rate: f32,
    next_voice_idx: usize,
}

impl SynthVoiceManager {
    pub fn new(sample_rate: f32) -> Self {
        Self::with_max_voices(sample_rate, DEFAULT_MAX_VOICES)
    }

    pub fn with_max_voices(sample_rate: f32, max_voices: usize) -> Self {
        let max_voices = max_voices.max(1).min(4096); // Clamp to reasonable range
        let voices = (0..max_voices).map(|_| SynthVoice::new()).collect();

        Self {
            voices,
            sample_rate,
            next_voice_idx: 0,
        }
    }

    /// Trigger a new note
    pub fn trigger_note(
        &mut self,
        frequency: f32,
        waveform: SynthWaveform,
        adsr: ADSRParams,
        gain: f32,
        pan: f32,
    ) {
        // Find a free voice or steal the oldest
        let voice_idx = self.find_free_voice();
        self.voices[voice_idx].trigger(frequency, waveform, adsr, gain, pan);
    }

    /// Find a free voice or steal the oldest one
    fn find_free_voice(&mut self) -> usize {
        // First, try to find an inactive voice
        for (i, voice) in self.voices.iter().enumerate() {
            if !voice.is_active {
                return i;
            }
        }

        // All voices active - steal the oldest one
        let mut oldest_idx = 0;
        let mut oldest_age = 0;

        for (i, voice) in self.voices.iter().enumerate() {
            if voice.age > oldest_age {
                oldest_age = voice.age;
                oldest_idx = i;
            }
        }

        oldest_idx
    }

    /// Release a specific voice (if we ever need direct control)
    pub fn release_voice(&mut self, voice_idx: usize) {
        if voice_idx < self.voices.len() {
            self.voices[voice_idx].release();
        }
    }

    /// Release all active voices
    pub fn release_all(&mut self) {
        for voice in &mut self.voices {
            if voice.is_active {
                voice.release();
            }
        }
    }

    /// Kill all voices immediately (panic/hush)
    pub fn kill_all(&mut self) {
        for voice in &mut self.voices {
            voice.is_active = false;
            voice.envelope_phase = EnvelopePhase::Idle;
            voice.envelope_level = 0.0;
        }
    }

    /// Process one sample and return mixed output
    pub fn process(&mut self) -> f32 {
        let mut mix = 0.0;

        for voice in &mut self.voices {
            if voice.is_active {
                let sample = voice.process(self.sample_rate);
                mix += sample;
            }
        }

        // Soft clipping to prevent clipping with many voices
        mix.tanh()
    }

    /// Get number of active voices
    pub fn active_voice_count(&self) -> usize {
        self.voices.iter().filter(|v| v.is_active).count()
    }

    /// Reset all voices
    pub fn reset(&mut self) {
        for voice in &mut self.voices {
            *voice = SynthVoice::new();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trigger_single_note() {
        let mut manager = SynthVoiceManager::new(44100.0);

        // Trigger a note
        manager.trigger_note(440.0, SynthWaveform::Sine, ADSRParams::default(), 1.0, 0.0);

        assert_eq!(manager.active_voice_count(), 1);

        // Process attack phase (default attack is 0.01s = 441 samples)
        // Let's process 500 samples to be sure we're past attack
        let mut has_audio = false;
        for i in 0..500 {
            let sample = manager.process();
            if sample.abs() > 0.001 {
                has_audio = true;
            }
            // After a few samples into attack, we should have audio
            if i > 50 {
                assert!(
                    sample.abs() > 0.0,
                    "Voice should produce sound after {} samples, got {}",
                    i,
                    sample
                );
            }
        }
        assert!(has_audio, "Voice should have produced some audible sound");
    }

    #[test]
    fn test_polyphonic_triggering() {
        let mut manager = SynthVoiceManager::new(44100.0);

        // Trigger 4 notes simultaneously (C major chord)
        manager.trigger_note(261.63, SynthWaveform::Sine, ADSRParams::default(), 0.5, 0.0); // C4
        manager.trigger_note(329.63, SynthWaveform::Sine, ADSRParams::default(), 0.5, 0.0); // E4
        manager.trigger_note(392.00, SynthWaveform::Sine, ADSRParams::default(), 0.5, 0.0); // G4
        manager.trigger_note(523.25, SynthWaveform::Sine, ADSRParams::default(), 0.5, 0.0); // C5

        assert_eq!(manager.active_voice_count(), 4);
    }

    #[test]
    fn test_envelope_release() {
        let mut manager = SynthVoiceManager::new(44100.0);

        let adsr = ADSRParams {
            attack: 0.01,
            decay: 0.0,
            sustain: 1.0,
            release: 0.1, // 100ms release
        };

        manager.trigger_note(440.0, SynthWaveform::Sine, adsr, 1.0, 0.0);

        // Let attack finish
        for _ in 0..(44100.0 * 0.01) as usize {
            manager.process();
        }

        assert_eq!(manager.active_voice_count(), 1);

        // Release the voice
        manager.release_voice(0);

        // Process release phase
        for _ in 0..(44100.0 * 0.15) as usize {
            manager.process();
        }

        // Voice should be inactive after release
        assert_eq!(manager.active_voice_count(), 0);
    }

    #[test]
    fn test_voice_stealing() {
        let mut manager = SynthVoiceManager::with_max_voices(44100.0, 64);

        // Trigger 64 notes (max capacity)
        for i in 0..64 {
            let freq = 220.0 * (1.0 + i as f32 * 0.01);
            manager.trigger_note(freq, SynthWaveform::Sine, ADSRParams::default(), 0.5, 0.0);
        }

        assert_eq!(manager.active_voice_count(), 64);

        // Age the voices
        for _ in 0..100 {
            manager.process();
        }

        // Trigger 65th note (should steal oldest)
        manager.trigger_note(880.0, SynthWaveform::Sine, ADSRParams::default(), 0.5, 0.0);

        assert_eq!(
            manager.active_voice_count(),
            64,
            "Should still have 64 voices after stealing"
        );
    }
}
