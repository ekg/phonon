#![allow(unused_assignments, unused_mut)]
//! Voice Manager - Handles polyphonic sample playback with voice allocation
//!
//! Based on SuperCollider's architecture, this module manages multiple
//! simultaneous sample playback voices with proper position tracking.
//!
//! # Features
//!
//! - **64 simultaneous voices**: Can play up to 64 samples at once
//! - **Automatic voice allocation**: Finds free voices or steals oldest one
//! - **Voice stealing**: When all voices are busy, the oldest voice is reused
//! - **Per-voice control**: Gain, pan, and speed parameters for each voice
//! - **Stereo output**: Equal-power panning for proper stereo imaging
//!
//! # Examples
//!
//! ## Basic sample playback
//!
//! ```
//! use phonon::voice_manager::VoiceManager;
//! use phonon::sample_loader::SampleBank;
//!
//! // Create voice manager and sample bank
//! let mut vm = VoiceManager::new();
//! let mut bank = SampleBank::new();
//!
//! // Load a sample
//! let bd_sample = bank.get_sample("bd").expect("Sample not found");
//!
//! // Trigger the sample
//! vm.trigger_sample(bd_sample, 1.0); // gain=1.0
//!
//! // Process audio (call in your audio callback)
//! for _ in 0..1000 {
//!     let audio_sample = vm.process();
//!     // Send audio_sample to audio output
//! }
//! ```
//!
//! ## Stereo with panning
//!
//! ```
//! use phonon::voice_manager::VoiceManager;
//! use phonon::sample_loader::SampleBank;
//!
//! let mut vm = VoiceManager::new();
//! let mut bank = SampleBank::new();
//!
//! let bd = bank.get_sample("bd").unwrap();
//! let sn = bank.get_sample("sn").unwrap();
//!
//! // Pan kick to left, snare to right
//! vm.trigger_sample_with_pan(bd, 1.0, -1.0); // hard left
//! vm.trigger_sample_with_pan(sn, 0.8, 1.0);  // hard right
//!
//! // Process stereo
//! let (left, right) = vm.process_stereo();
//! ```
//!
//! ## Speed control (pitch shifting)
//!
//! ```
//! use phonon::voice_manager::Voice;
//! use std::sync::Arc;
//!
//! let mut voice = Voice::new();
//! let sample_data = vec![0.5, 0.6, 0.7, 0.8];
//! let sample = Arc::new(sample_data);
//!
//! // Play at different speeds
//! voice.trigger_with_speed(sample.clone(), 1.0, 0.0, 1.0); // normal
//! voice.trigger_with_speed(sample.clone(), 1.0, 0.0, 2.0); // double speed (octave up)
//! voice.trigger_with_speed(sample, 1.0, 0.0, 0.5);         // half speed (octave down)
//! ```

use crate::envelope::VoiceEnvelope;
use std::sync::Arc;

/// Maximum number of simultaneous voices
/// Default number of voices if not specified
const DEFAULT_MAX_VOICES: usize = 256;

/// Sample rate for envelope calculations (will be set per-voice)
const SAMPLE_RATE: f32 = 44100.0;

/// A single voice that plays a sample
#[derive(Clone)]
pub struct Voice {
    /// The sample data to play
    sample_data: Option<Arc<Vec<f32>>>,

    /// Current playback position in the sample (fractional for speed control)
    position: f32,

    /// Whether this voice is currently active
    active: bool,

    /// Gain for this voice
    gain: f32,

    /// Pan position: -1.0 = hard left, 0.0 = center, 1.0 = hard right
    pan: f32,

    /// Playback speed: 1.0 = normal, 2.0 = double speed, 0.5 = half speed
    speed: f32,

    /// Age counter for voice stealing (incremented each sample)
    age: usize,

    /// Cut group: voices in the same cut group stop each other when triggered
    /// None = no cut group, Some(n) = cut group number n
    cut_group: Option<u32>,

    /// Envelope generator for amplitude shaping (supports multiple types)
    envelope: VoiceEnvelope,

    /// Attack time in seconds (for backward compatibility)
    attack: f32,

    /// Release time in seconds (for backward compatibility)
    release: f32,
}

impl Default for Voice {
    fn default() -> Self {
        Self::new()
    }
}

impl Voice {
    pub fn new() -> Self {
        Self {
            sample_data: None,
            position: 0.0,
            active: false,
            gain: 1.0,
            pan: 0.0,
            speed: 1.0,
            age: 0,
            cut_group: None,
            envelope: VoiceEnvelope::new_percussion(SAMPLE_RATE, 0.001, 0.1),
            attack: 0.001, // 1ms default attack
            release: 0.1,  // 100ms default release
        }
    }

    /// Start playing a sample with pan (backward compatibility, speed=1.0, no cut group)
    pub fn trigger(&mut self, sample: Arc<Vec<f32>>, gain: f32, pan: f32) {
        self.trigger_with_speed(sample, gain, pan, 1.0);
    }

    /// Start playing a sample with gain, pan, and speed control (no cut group)
    pub fn trigger_with_speed(&mut self, sample: Arc<Vec<f32>>, gain: f32, pan: f32, speed: f32) {
        self.trigger_with_cut_group(sample, gain, pan, speed, None);
    }

    /// Start playing a sample with full control including cut group
    pub fn trigger_with_cut_group(
        &mut self,
        sample: Arc<Vec<f32>>,
        gain: f32,
        pan: f32,
        speed: f32,
        cut_group: Option<u32>,
    ) {
        self.trigger_with_envelope(sample, gain, pan, speed, cut_group, 0.001, 0.1);
    }

    /// Start playing a sample with full control including envelope parameters
    pub fn trigger_with_envelope(
        &mut self,
        sample: Arc<Vec<f32>>,
        gain: f32,
        pan: f32,
        speed: f32,
        cut_group: Option<u32>,
        attack: f32,
        release: f32,
    ) {
        self.sample_data = Some(sample);
        self.position = 0.0;
        self.active = true;
        self.gain = gain;
        self.pan = pan.clamp(-1.0, 1.0);
        self.speed = speed.max(0.01); // Prevent zero or negative speed for now
        self.age = 0;
        self.cut_group = cut_group;
        self.attack = attack.max(0.0001); // Minimum 0.1ms
        self.release = release.max(0.001); // Minimum 1ms

        // Configure and trigger envelope (recreate as percussion type)
        self.envelope = VoiceEnvelope::new_percussion(SAMPLE_RATE, self.attack, self.release);
        self.envelope.trigger();
    }

    /// Start playing a sample with ADSR envelope
    pub fn trigger_with_adsr(
        &mut self,
        sample: Arc<Vec<f32>>,
        gain: f32,
        pan: f32,
        speed: f32,
        cut_group: Option<u32>,
        attack: f32,
        decay: f32,
        sustain: f32,
        release: f32,
    ) {
        self.sample_data = Some(sample);
        self.position = 0.0;
        self.active = true;
        self.gain = gain;
        self.pan = pan.clamp(-1.0, 1.0);
        self.speed = speed.max(0.01);
        self.age = 0;
        self.cut_group = cut_group;
        self.attack = attack;
        self.release = release;

        // Create and trigger ADSR envelope
        self.envelope = VoiceEnvelope::new_adsr(SAMPLE_RATE, attack, decay, sustain, release);
        self.envelope.trigger();
    }

    /// Start playing a sample with segments envelope
    pub fn trigger_with_segments(
        &mut self,
        sample: Arc<Vec<f32>>,
        gain: f32,
        pan: f32,
        speed: f32,
        cut_group: Option<u32>,
        levels: Vec<f32>,
        times: Vec<f32>,
    ) {
        self.sample_data = Some(sample);
        self.position = 0.0;
        self.active = true;
        self.gain = gain;
        self.pan = pan.clamp(-1.0, 1.0);
        self.speed = speed.max(0.01);
        self.age = 0;
        self.cut_group = cut_group;

        // Create and trigger segments envelope
        self.envelope = VoiceEnvelope::new_segments(SAMPLE_RATE, levels, times);
        self.envelope.trigger();
    }

    /// Start playing a sample with curve envelope
    pub fn trigger_with_curve(
        &mut self,
        sample: Arc<Vec<f32>>,
        gain: f32,
        pan: f32,
        speed: f32,
        cut_group: Option<u32>,
        start: f32,
        end: f32,
        duration: f32,
        curve: f32,
    ) {
        self.sample_data = Some(sample);
        self.position = 0.0;
        self.active = true;
        self.gain = gain;
        self.pan = pan.clamp(-1.0, 1.0);
        self.speed = speed.max(0.01);
        self.age = 0;
        self.cut_group = cut_group;

        // Create and trigger curve envelope
        self.envelope = VoiceEnvelope::new_curve(SAMPLE_RATE, start, end, duration, curve);
        self.envelope.trigger();
    }

    /// Process one sample of audio (mono)
    pub fn process(&mut self) -> f32 {
        let (left, right) = self.process_stereo();
        // Mix down to mono with compensation for equal-power panning
        // At center pan, left=right=value*sqrt(0.5), so (left+right)=value*sqrt(2)
        // Divide by sqrt(2) to restore original amplitude: value*sqrt(2)/sqrt(2) = value
        (left + right) / std::f32::consts::SQRT_2
    }

    /// Process one sample of audio (stereo with panning)
    pub fn process_stereo(&mut self) -> (f32, f32) {
        if !self.active {
            return (0.0, 0.0);
        }

        // Process envelope
        let env_value = self.envelope.process();

        // Check if envelope finished
        if !self.envelope.is_active() {
            self.active = false;
            self.sample_data = None;
            return (0.0, 0.0);
        }

        if let Some(ref samples) = self.sample_data {
            let sample_len = samples.len() as f32;

            if self.position < sample_len {
                // Linear interpolation for fractional positions
                let pos_floor = self.position.floor() as usize;
                let pos_frac = self.position - pos_floor as f32;

                let sample_value = if pos_floor + 1 < samples.len() {
                    // Interpolate between current and next sample
                    let current = samples[pos_floor];
                    let next = samples[pos_floor + 1];
                    current * (1.0 - pos_frac) + next * pos_frac
                } else {
                    // Last sample, no interpolation
                    samples[pos_floor]
                };

                // Apply gain and envelope
                let output_value = sample_value * self.gain * env_value;

                // Advance position by speed
                self.position += self.speed;
                self.age += 1;

                // Equal-power panning
                // pan: -1.0 = hard left, 0.0 = center, 1.0 = hard right
                let pan_radians = (self.pan + 1.0) * std::f32::consts::FRAC_PI_4; // Map -1..1 to 0..PI/2
                let left_gain = pan_radians.cos();
                let right_gain = pan_radians.sin();

                let left = output_value * left_gain;
                let right = output_value * right_gain;

                (left, right)
            } else {
                // Sample finished, but envelope might still be ringing
                // Continue processing envelope until it finishes
                (0.0, 0.0)
            }
        } else {
            self.active = false;
            (0.0, 0.0)
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

impl Default for VoiceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl VoiceManager {
    /// Create a new VoiceManager with default voice count (256)
    pub fn new() -> Self {
        Self::with_max_voices(DEFAULT_MAX_VOICES)
    }

    /// Create a new VoiceManager with specified max voices
    ///
    /// # Arguments
    /// * `max_voices` - Maximum number of simultaneous voices (recommended: 64-1024)
    ///
    /// # Example
    /// ```
    /// let vm = VoiceManager::with_max_voices(512); // 512 voices
    /// ```
    pub fn with_max_voices(max_voices: usize) -> Self {
        let max_voices = max_voices.max(1).min(4096); // Clamp to reasonable range
        let mut voices = Vec::with_capacity(max_voices);
        for _ in 0..max_voices {
            voices.push(Voice::new());
        }

        Self {
            voices,
            next_voice_index: 0,
        }
    }

    /// Trigger a sample with automatic voice allocation (center pan)
    pub fn trigger_sample(&mut self, sample: Arc<Vec<f32>>, gain: f32) {
        self.trigger_sample_with_pan(sample, gain, 0.0);
    }

    /// Trigger a sample with automatic voice allocation and pan control
    pub fn trigger_sample_with_pan(&mut self, sample: Arc<Vec<f32>>, gain: f32, pan: f32) {
        self.trigger_sample_with_params(sample, gain, pan, 1.0);
    }

    /// Trigger a sample with full DSP parameter control (gain, pan, speed, no cut group)
    pub fn trigger_sample_with_params(
        &mut self,
        sample: Arc<Vec<f32>>,
        gain: f32,
        pan: f32,
        speed: f32,
    ) {
        self.trigger_sample_with_cut_group(sample, gain, pan, speed, None);
    }

    /// Trigger a sample with full control including cut group
    /// If cut_group is Some(n), all other voices in cut group n will be stopped
    pub fn trigger_sample_with_cut_group(
        &mut self,
        sample: Arc<Vec<f32>>,
        gain: f32,
        pan: f32,
        speed: f32,
        cut_group: Option<u32>,
    ) {
        self.trigger_sample_with_envelope(sample, gain, pan, speed, cut_group, 0.001, 0.1);
    }

    /// Trigger a sample with full control including envelope parameters (percussion envelope)
    /// This is the most complete trigger method with all DSP parameters
    pub fn trigger_sample_with_envelope(
        &mut self,
        sample: Arc<Vec<f32>>,
        gain: f32,
        pan: f32,
        speed: f32,
        cut_group: Option<u32>,
        attack: f32,
        release: f32,
    ) {
        // If this has a cut group, fade out all other voices in the same cut group
        // Use a quick 10ms release to avoid clicks
        if let Some(group) = cut_group {
            for voice in &mut self.voices {
                if voice.cut_group == Some(group) && voice.active {
                    // Trigger quick release instead of hard-stop to avoid clicks
                    voice.envelope.trigger_quick_release(0.01); // 10ms fade-out
                }
            }
        }

        // Try to find an inactive voice
        let max_voices = self.voices.len();
        for i in 0..max_voices {
            let idx = (self.next_voice_index + i) % max_voices;
            if self.voices[idx].is_available() {
                self.voices[idx]
                    .trigger_with_envelope(sample, gain, pan, speed, cut_group, attack, release);
                self.next_voice_index = (idx + 1) % max_voices;
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
        self.voices[oldest_idx]
            .trigger_with_envelope(sample, gain, pan, speed, cut_group, attack, release);
        let max_voices = self.voices.len();
        self.next_voice_index = (oldest_idx + 1) % max_voices;
    }

    /// Trigger a sample with ADSR envelope
    pub fn trigger_sample_with_adsr(
        &mut self,
        sample: Arc<Vec<f32>>,
        gain: f32,
        pan: f32,
        speed: f32,
        cut_group: Option<u32>,
        attack: f32,
        decay: f32,
        sustain: f32,
        release: f32,
    ) {
        // Handle cut groups
        if let Some(group) = cut_group {
            for voice in &mut self.voices {
                if voice.cut_group == Some(group) && voice.active {
                    voice.active = false;
                    voice.sample_data = None;
                }
            }
        }

        // Try to find an inactive voice
        let max_voices = self.voices.len();
        for i in 0..max_voices {
            let idx = (self.next_voice_index + i) % max_voices;
            if self.voices[idx].is_available() {
                self.voices[idx].trigger_with_adsr(
                    sample, gain, pan, speed, cut_group, attack, decay, sustain, release,
                );
                self.next_voice_index = (idx + 1) % max_voices;
                return;
            }
        }

        // Steal oldest voice
        let mut oldest_idx = 0;
        let mut oldest_age = 0;
        for (idx, voice) in self.voices.iter().enumerate() {
            if voice.age > oldest_age {
                oldest_age = voice.age;
                oldest_idx = idx;
            }
        }
        self.voices[oldest_idx].trigger_with_adsr(
            sample, gain, pan, speed, cut_group, attack, decay, sustain, release,
        );
        self.next_voice_index = (oldest_idx + 1) % max_voices;
    }

    /// Trigger a sample with segments envelope
    pub fn trigger_sample_with_segments(
        &mut self,
        sample: Arc<Vec<f32>>,
        gain: f32,
        pan: f32,
        speed: f32,
        cut_group: Option<u32>,
        levels: Vec<f32>,
        times: Vec<f32>,
    ) {
        // Handle cut groups
        if let Some(group) = cut_group {
            for voice in &mut self.voices {
                if voice.cut_group == Some(group) && voice.active {
                    voice.active = false;
                    voice.sample_data = None;
                }
            }
        }

        // Try to find an inactive voice
        let max_voices = self.voices.len();
        for i in 0..max_voices {
            let idx = (self.next_voice_index + i) % max_voices;
            if self.voices[idx].is_available() {
                self.voices[idx]
                    .trigger_with_segments(sample, gain, pan, speed, cut_group, levels, times);
                self.next_voice_index = (idx + 1) % max_voices;
                return;
            }
        }

        // Steal oldest voice
        let mut oldest_idx = 0;
        let mut oldest_age = 0;
        for (idx, voice) in self.voices.iter().enumerate() {
            if voice.age > oldest_age {
                oldest_age = voice.age;
                oldest_idx = idx;
            }
        }
        self.voices[oldest_idx]
            .trigger_with_segments(sample, gain, pan, speed, cut_group, levels, times);
        self.next_voice_index = (oldest_idx + 1) % max_voices;
    }

    /// Trigger a sample with curve envelope
    pub fn trigger_sample_with_curve(
        &mut self,
        sample: Arc<Vec<f32>>,
        gain: f32,
        pan: f32,
        speed: f32,
        cut_group: Option<u32>,
        start: f32,
        end: f32,
        duration: f32,
        curve: f32,
    ) {
        // Handle cut groups
        if let Some(group) = cut_group {
            for voice in &mut self.voices {
                if voice.cut_group == Some(group) && voice.active {
                    voice.active = false;
                    voice.sample_data = None;
                }
            }
        }

        // Try to find an inactive voice
        let max_voices = self.voices.len();
        for i in 0..max_voices {
            let idx = (self.next_voice_index + i) % max_voices;
            if self.voices[idx].is_available() {
                self.voices[idx].trigger_with_curve(
                    sample, gain, pan, speed, cut_group, start, end, duration, curve,
                );
                self.next_voice_index = (idx + 1) % max_voices;
                return;
            }
        }

        // Steal oldest voice
        let mut oldest_idx = 0;
        let mut oldest_age = 0;
        for (idx, voice) in self.voices.iter().enumerate() {
            if voice.age > oldest_age {
                oldest_age = voice.age;
                oldest_idx = idx;
            }
        }
        self.voices[oldest_idx].trigger_with_curve(
            sample, gain, pan, speed, cut_group, start, end, duration, curve,
        );
        self.next_voice_index = (oldest_idx + 1) % max_voices;
    }

    /// Process one sample from all active voices (mono)
    pub fn process(&mut self) -> f32 {
        let (left, right) = self.process_stereo();
        // Mix down to mono with compensation for equal-power panning
        // At center pan, left=right=value*sqrt(0.5), so (left+right)=value*sqrt(2)
        // Divide by sqrt(2) to restore original amplitude
        (left + right) / std::f32::consts::SQRT_2
    }

    /// Process one sample from all active voices (stereo)
    pub fn process_stereo(&mut self) -> (f32, f32) {
        let mut left = 0.0;
        let mut right = 0.0;

        for voice in &mut self.voices {
            let (voice_left, voice_right) = voice.process_stereo();
            left += voice_left;
            right += voice_right;
        }

        // Apply some limiting to prevent clipping
        if left > 1.0 {
            left = left.tanh();
        } else if left < -1.0 {
            left = left.tanh();
        }

        if right > 1.0 {
            right = right.tanh();
        } else if right < -1.0 {
            right = right.tanh();
        }

        (left, right)
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
            voice.position = 0.0;
        }
        self.next_voice_index = 0;
    }

    /// Kill all active voices (alias for reset)
    pub fn kill_all(&mut self) {
        self.reset();
    }
}
