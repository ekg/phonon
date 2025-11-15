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
use rayon::prelude::*;
use std::sync::Arc;

/// Maximum number of simultaneous voices
/// Default number of voices if not specified
const DEFAULT_MAX_VOICES: usize = 256;

/// Sample rate for envelope calculations (will be set per-voice)
const SAMPLE_RATE: f32 = 44100.0;

/// Voice lifecycle state for proper management
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceState {
    /// Voice is free and available for allocation
    Free,
    /// Voice is actively playing (attack or sustain phase)
    Playing,
    /// Voice is in release phase (envelope releasing)
    Releasing,
}

/// A single voice that plays a sample
#[derive(Clone)]
pub struct Voice {
    /// The sample data to play
    sample_data: Option<Arc<Vec<f32>>>,

    /// Current playback position in the sample (fractional for speed control)
    position: f32,

    /// Current lifecycle state of this voice
    state: VoiceState,

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

    /// Source node ID: identifies which Sample node triggered this voice
    /// Used to separate outputs so each output only hears its own samples
    source_node: usize,

    /// Envelope generator for amplitude shaping (supports multiple types)
    envelope: VoiceEnvelope,

    /// Attack time in seconds (for backward compatibility)
    attack: f32,

    /// Release time in seconds (for backward compatibility)
    release: f32,

    /// Unit mode: "r" (rate) or "c" (cycle-sync)
    /// In rate mode, speed is a multiplier. In cycle mode, speed syncs to cycle duration.
    unit_mode: UnitMode,

    /// Loop mode: whether sample should loop when it reaches the end
    loop_enabled: bool,

    /// Auto-release time: trigger release() when age reaches this value
    /// Used for legato to create sharp note durations
    /// None = no auto-release (envelope controls duration)
    auto_release_at_sample: Option<usize>,
}

/// Unit mode for sample playback speed interpretation
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UnitMode {
    /// Rate mode (default): speed is a rate multiplier (2.0 = double speed)
    Rate,
    /// Cycle mode: speed syncs to cycle duration (1.0 = one cycle)
    Cycle,
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
            state: VoiceState::Free,
            gain: 1.0,
            pan: 0.0,
            speed: 1.0,
            age: 0,
            cut_group: None,
            source_node: 0,            // Default source node (will be set on trigger)
            envelope: VoiceEnvelope::new_percussion(SAMPLE_RATE, 0.001, 0.1),
            attack: 0.001,             // 1ms default attack
            release: 0.1,              // 100ms default release
            unit_mode: UnitMode::Rate, // Default to rate mode
            loop_enabled: false,       // Default to no looping
            auto_release_at_sample: None, // No auto-release by default
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
        // Initialize position based on speed direction
        // For reverse playback (negative speed), start at end of sample
        let initial_position = if speed < 0.0 {
            sample.len() as f32 - 1.0
        } else {
            0.0
        };

        self.sample_data = Some(sample);
        self.position = initial_position;
        self.state = VoiceState::Playing;
        self.gain = gain;
        self.pan = pan.clamp(-1.0, 1.0);
        self.speed = speed; // Allow negative speed for reverse playback
        self.age = 0;
        self.cut_group = cut_group;
        self.attack = attack.max(0.0001); // Minimum 0.1ms
        self.release = release.max(0.001); // Minimum 1ms
        self.auto_release_at_sample = None; // No auto-release for percussion

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
        // Initialize position based on speed direction
        let initial_position = if speed < 0.0 {
            sample.len() as f32 - 1.0
        } else {
            0.0
        };

        self.sample_data = Some(sample);
        self.position = initial_position;
        self.state = VoiceState::Playing;
        self.gain = gain;
        self.pan = pan.clamp(-1.0, 1.0);
        self.speed = speed; // Allow negative speed for reverse playback
        self.age = 0;
        self.cut_group = cut_group;
        self.attack = attack;
        self.release = release;
        self.auto_release_at_sample = None; // Will be set externally for legato

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
        // Initialize position based on speed direction
        let initial_position = if speed < 0.0 {
            sample.len() as f32 - 1.0
        } else {
            0.0
        };

        self.sample_data = Some(sample);
        self.position = initial_position;
        self.state = VoiceState::Playing;
        self.gain = gain;
        self.pan = pan.clamp(-1.0, 1.0);
        self.speed = speed; // Allow negative speed for reverse playback
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
        // Initialize position based on speed direction
        let initial_position = if speed < 0.0 {
            sample.len() as f32 - 1.0
        } else {
            0.0
        };

        self.sample_data = Some(sample);
        self.position = initial_position;
        self.state = VoiceState::Playing;
        self.gain = gain;
        self.pan = pan.clamp(-1.0, 1.0);
        self.speed = speed; // Allow negative speed for reverse playback
        self.age = 0;
        self.cut_group = cut_group;

        // Create and trigger curve envelope
        self.envelope = VoiceEnvelope::new_curve(SAMPLE_RATE, start, end, duration, curve);
        self.envelope.trigger();
    }

    /// Set unit mode (rate or cycle-sync)
    pub fn set_unit_mode(&mut self, mode: UnitMode) {
        self.unit_mode = mode;
    }

    /// Set loop mode (whether sample loops)
    pub fn set_loop_enabled(&mut self, enabled: bool) {
        self.loop_enabled = enabled;
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
        if self.state == VoiceState::Free {
            return (0.0, 0.0);
        }

        // DEBUG: Log voice processing
        if std::env::var("DEBUG_VOICE_PROCESS").is_ok() && self.age < 10 {
            eprintln!(
                "[VOICE] process_stereo called, age={}, position={:.1}, state={:?}",
                self.age, self.position, self.state
            );
        }

        // Process envelope
        // For reverse playback, skip envelope (sample plays backwards, envelope would sound wrong)
        let env_value = if self.speed < 0.0 {
            1.0  // Full gain for reverse playback
        } else {
            self.envelope.process()
        };

        // DEBUG: Log envelope state
        if std::env::var("DEBUG_VOICE_PROCESS").is_ok() && self.age < 10 {
            eprintln!(
                "[VOICE] envelope processed, env_value={:.6}, is_active={}",
                env_value, self.envelope.is_active()
            );
        }

        // Auto-release for legato: trigger release at exact sample count
        if let Some(release_at) = self.auto_release_at_sample {
            if self.age >= release_at {
                self.envelope.release();
                self.auto_release_at_sample = None; // Only trigger once
            }
        }

        // Check if envelope finished
        if !self.envelope.is_active() {
            if std::env::var("DEBUG_VOICE_PROCESS").is_ok() {
                eprintln!("[VOICE] envelope not active, setting state to Free");
            }
            self.state = VoiceState::Free;
            self.sample_data = None;
            return (0.0, 0.0);
        }

        if let Some(ref samples) = self.sample_data {
            let sample_len = samples.len() as f32;

            // Handle looping: wrap position if it exceeds boundaries
            if self.loop_enabled {
                if self.speed >= 0.0 && self.position >= sample_len {
                    self.position = self.position % sample_len;
                } else if self.speed < 0.0 && self.position < 0.0 {
                    // Reverse looping: wrap from end
                    self.position = sample_len - 1.0 + (self.position % sample_len);
                }
            }

            // Check if position is within bounds (handles both forward and reverse)
            let is_in_bounds = self.position >= 0.0 && self.position < sample_len;

            if is_in_bounds {
                // Linear interpolation for fractional positions
                let pos_floor = self.position.floor() as usize;
                let pos_frac = self.position - pos_floor as f32;

                let sample_value = if self.speed >= 0.0 {
                    // Forward playback: interpolate forward
                    if pos_floor + 1 < samples.len() {
                        let current = samples[pos_floor];
                        let next = samples[pos_floor + 1];
                        current * (1.0 - pos_frac) + next * pos_frac
                    } else {
                        samples[pos_floor]
                    }
                } else {
                    // Reverse playback: interpolate backward
                    if pos_floor > 0 {
                        let current = samples[pos_floor];
                        let prev = samples[pos_floor - 1];
                        current * (1.0 - pos_frac) + prev * pos_frac
                    } else {
                        samples[pos_floor]
                    }
                };

                // Apply gain and envelope
                let output_value = sample_value * self.gain * env_value;

                // Advance position by speed (negative speed moves backward)
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
            self.state = VoiceState::Free;
            (0.0, 0.0)
        }
    }

    /// Check if voice is available for allocation
    pub fn is_available(&self) -> bool {
        self.state == VoiceState::Free
    }
}

/// Manages a pool of voices for polyphonic playback
pub struct VoiceManager {
    /// Pool of available voices (grows dynamically)
    voices: Vec<Voice>,

    /// Next voice index for round-robin allocation
    next_voice_index: usize,

    /// Index of the last triggered voice (for post-trigger configuration)
    last_triggered_voice_index: Option<usize>,

    /// Default source node ID to assign to newly triggered voices
    /// This is set before triggering and applied automatically
    default_source_node: usize,

    /// Maximum voices allowed (None = unlimited)
    max_voices: Option<usize>,

    /// Initial voice pool size
    initial_voices: usize,

    /// Sample counter for periodic pool shrinking
    shrink_counter: usize,

    /// Performance monitoring: peak voice count
    peak_voice_count: usize,

    /// Performance monitoring: total samples processed
    total_samples_processed: u64,

    /// Adaptive parallelism: current threshold (dynamically adjusted)
    parallel_threshold: usize,

    /// Performance tracking: processing time history (in microseconds per sample)
    processing_times: Vec<f32>,

    /// Performance tracking: number of times we exceeded time budget
    underrun_count: u64,

    /// Performance tracking: samples processed since last adjustment
    samples_since_adjustment: usize,
}

impl Default for VoiceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl VoiceManager {
    /// Create a new VoiceManager with unlimited voices (grows dynamically from 16)
    pub fn new() -> Self {
        Self::with_config(16, None)
    }

    /// Create a new VoiceManager with specified max voices (deprecated, use with_config)
    ///
    /// # Arguments
    /// * `max_voices` - Maximum number of simultaneous voices
    ///
    /// # Example
    /// ```
    /// let vm = VoiceManager::with_max_voices(512); // Max 512 voices
    /// ```
    pub fn with_max_voices(max_voices: usize) -> Self {
        Self::with_config(16.min(max_voices), Some(max_voices))
    }

    /// Create a new VoiceManager with custom configuration
    ///
    /// # Arguments
    /// * `initial_voices` - Initial voice pool size (grows from here)
    /// * `max_voices` - Optional maximum voices (None = unlimited)
    ///
    /// # Example
    /// ```
    /// // Start with 32 voices, grow to 500 max
    /// let vm = VoiceManager::with_config(32, Some(500));
    ///
    /// // Start with 16 voices, unlimited growth
    /// let vm = VoiceManager::with_config(16, None);
    /// ```
    pub fn with_config(initial_voices: usize, max_voices: Option<usize>) -> Self {
        let initial_voices = initial_voices.max(1).min(4096);
        let mut voices = Vec::with_capacity(initial_voices * 2); // Reserve 2x for growth
        for _ in 0..initial_voices {
            voices.push(Voice::new());
        }

        Self {
            voices,
            next_voice_index: 0,
            last_triggered_voice_index: None,
            default_source_node: 0,
            max_voices,
            initial_voices,
            shrink_counter: 0,
            peak_voice_count: initial_voices,
            total_samples_processed: 0,
            parallel_threshold: 32, // Aggressive threshold - enable parallelism earlier on multi-core systems
            processing_times: Vec::with_capacity(1000), // Track last 1000 samples
            underrun_count: 0,
            samples_since_adjustment: 0,
        }
    }

    /// Shrink the voice pool if too many voices are unused
    /// Only shrinks down to initial_voices, never below
    /// Returns number of voices removed
    pub fn shrink_voice_pool(&mut self) -> usize {
        let current_count = self.voices.len();
        if current_count <= self.initial_voices {
            return 0; // Don't shrink below initial size
        }

        // Count active voices
        let active_count = self
            .voices
            .iter()
            .filter(|v| v.state != VoiceState::Free)
            .count();

        // Only shrink if less than 25% of voices are in use
        let usage_ratio = active_count as f32 / current_count as f32;
        if usage_ratio > 0.25 {
            return 0; // Still using too many voices, don't shrink
        }

        // Shrink to 150% of active count or initial_voices, whichever is larger
        let target_size = ((active_count as f32 * 1.5) as usize).max(self.initial_voices);

        if target_size < current_count {
            // Truncate to target size (removes from end)
            let remove_count = current_count - target_size;
            self.voices.truncate(target_size);

            eprintln!(
                "🔻 Voice pool shrunk: {} → {} voices ({}% usage)",
                current_count,
                target_size,
                (usage_ratio * 100.0) as u32
            );
            remove_count
        } else {
            0
        }
    }

    /// Grow the voice pool by adding more voices
    /// Returns true if growth succeeded, false if at max_voices limit
    fn grow_voice_pool(&mut self) -> bool {
        let current_count = self.voices.len();

        // Hard cap at 128 voices to prevent excessive memory/CPU usage
        // Even on 16-core systems, 128 voices is more than enough for most patterns
        const ABSOLUTE_MAX_VOICES: usize = 128;

        // Check if we've hit the max limit
        if let Some(max) = self.max_voices {
            if current_count >= max {
                eprintln!(
                    "⚠️  Voice limit reached: {} voices (max: {})",
                    current_count, max
                );
                return false;
            }
        }

        // Check absolute maximum
        if current_count >= ABSOLUTE_MAX_VOICES {
            eprintln!(
                "⚠️  Absolute voice limit reached: {} voices (hard cap: {})",
                current_count, ABSOLUTE_MAX_VOICES
            );
            eprintln!("    Consider using 'cut groups' or longer envelopes to reduce overlapping samples");
            return false;
        }

        // Grow by 50% or 16 voices, whichever is larger
        let growth = (current_count / 2).max(16);
        let new_count = if let Some(max) = self.max_voices {
            (current_count + growth).min(max).min(ABSOLUTE_MAX_VOICES)
        } else {
            (current_count + growth).min(ABSOLUTE_MAX_VOICES)
        };

        let voices_to_add = new_count - current_count;
        if voices_to_add > 0 {
            for _ in 0..voices_to_add {
                self.voices.push(Voice::new());
            }
            eprintln!(
                "🎵 Voice pool grown: {} → {} voices",
                current_count, new_count
            );
            true
        } else {
            false
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
        // DEBUG: Log voice triggers to detect duplication
        if std::env::var("DEBUG_VOICE_TRIGGERS").is_ok() {
            eprintln!("[VOICE_MGR] trigger_sample_with_envelope called: sample_len={}, gain={:.3}, pan={:.3}, speed={:.3}",
                sample.len(), gain, pan, speed);
        }

        // If this has a cut group, fade out all other voices in the same cut group
        // Use a quick 10ms release to avoid clicks
        if let Some(group) = cut_group {
            for voice in &mut self.voices {
                if voice.cut_group == Some(group) && voice.state != VoiceState::Free {
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
                self.voices[idx].source_node = self.default_source_node; // Set source node
                self.next_voice_index = (idx + 1) % max_voices;
                self.last_triggered_voice_index = Some(idx); // Track for post-trigger config

                // DEBUG: Verify voice state after triggering
                if std::env::var("DEBUG_VOICE_TRIGGERS").is_ok() {
                    eprintln!("[VOICE_MGR] Voice {} triggered, state={:?}, envelope_active={}",
                        idx, self.voices[idx].state, self.voices[idx].envelope.is_active());
                }
                return;
            }
        }

        // All voices are active - try to grow the pool first
        if self.grow_voice_pool() {
            // Growth succeeded, allocate from the newly added voices
            let idx = self.voices.len() - 1; // Use the last (newest) voice
            self.voices[idx]
                .trigger_with_envelope(sample, gain, pan, speed, cut_group, attack, release);
            self.voices[idx].source_node = self.default_source_node; // Set source node
            self.next_voice_index = 0; // Reset to beginning
            self.last_triggered_voice_index = Some(idx);
            return;
        }

        // Growth failed or at limit - steal the oldest one
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
        self.voices[oldest_idx].source_node = self.default_source_node; // Set source node
        let max_voices = self.voices.len();
        self.next_voice_index = (oldest_idx + 1) % max_voices;
        self.last_triggered_voice_index = Some(oldest_idx); // Track for post-trigger config
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
                if voice.cut_group == Some(group) && voice.state != VoiceState::Free {
                    voice.state = VoiceState::Free;
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
                self.last_triggered_voice_index = Some(idx); // Track for post-trigger config
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
        self.last_triggered_voice_index = Some(oldest_idx); // Track for post-trigger config
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
                if voice.cut_group == Some(group) && voice.state != VoiceState::Free {
                    voice.state = VoiceState::Free;
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
                if voice.cut_group == Some(group) && voice.state != VoiceState::Free {
                    voice.state = VoiceState::Free;
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

    /// Process all voices and return per-node mixes (HashMap: source_node -> mono_output)
    /// This allows multiple outputs to have independent sample streams
    /// Each voice is processed ONCE, then outputs are grouped by source_node
    pub fn process_per_node(&mut self) -> std::collections::HashMap<usize, f32> {
        use std::collections::HashMap;

        // PERFORMANCE: Use parallel processing for high voice counts
        let voice_outputs: Vec<((f32, f32), usize)> = if self.voices.len() >= self.parallel_threshold {
            // Parallel voice processing (huge win for 243 voices on 16 cores!)
            self.voices
                .par_iter_mut()
                .map(|voice| {
                    let (l, r) = voice.process_stereo();
                    ((l, r), voice.source_node)
                })
                .collect()
        } else {
            // Sequential for low voice counts (avoid Rayon overhead)
            self.voices
                .iter_mut()
                .map(|voice| {
                    let (l, r) = voice.process_stereo();
                    ((l, r), voice.source_node)
                })
                .collect()
        };

        // Accumulate by source_node (sequential - fast HashMap ops)
        let mut node_sums: HashMap<usize, (f32, f32)> = HashMap::new();
        for ((l, r), source_node) in voice_outputs {
            node_sums
                .entry(source_node)
                .and_modify(|(left, right)| {
                    *left += l;
                    *right += r;
                })
                .or_insert((l, r));
        }

        // Convert stereo sums to mono with proper equal-power conversion
        node_sums
            .into_iter()
            .map(|(node, (left, right))| {
                let mono = (left + right) / std::f32::consts::SQRT_2;
                (node, mono)
            })
            .collect()
    }

    /// Process one sample from all active voices (stereo)
    pub fn process_stereo(&mut self) -> (f32, f32) {
        let mut left = 0.0;
        let mut right = 0.0;

        // DEBUG: Count active voices
        if std::env::var("DEBUG_VOICE_COUNT").is_ok() {
            let active_count = self
                .voices
                .iter()
                .filter(|v| v.state != VoiceState::Free)
                .count();
            eprintln!("[VOICE_MGR] process_stereo: total_voices={}, active_voices={}",
                self.voices.len(), active_count);
            if active_count > 0 {
                eprintln!("[VOICE_MGR] {} active voices", active_count);
            }
        }

        // Periodic voice pool shrinking (once per second at 44.1kHz)
        self.shrink_counter += 1;
        if self.shrink_counter >= 44100 {
            self.shrink_counter = 0;
            self.shrink_voice_pool();
        }

        // Performance monitoring: track peak voice count and samples
        let active_count = self
            .voices
            .iter()
            .filter(|v| v.state != VoiceState::Free)
            .count();
        if active_count > self.peak_voice_count {
            self.peak_voice_count = active_count;
        }
        self.total_samples_processed += 1;

        // Adaptive parallel voice processing with performance tracking
        // Measure processing time to detect underruns
        let start_time = std::time::Instant::now();

        // Use dynamic threshold instead of const
        if self.voices.len() >= self.parallel_threshold {
            // Parallel processing for high voice counts
            let voice_outputs: Vec<(f32, f32)> = self
                .voices
                .par_iter_mut()
                .map(|voice| voice.process_stereo())
                .collect();

            // Sequential sum
            for (voice_left, voice_right) in voice_outputs {
                left += voice_left;
                right += voice_right;
            }
        } else {
            // Sequential processing for low voice counts (avoids Rayon overhead)
            for voice in &mut self.voices {
                let (voice_left, voice_right) = voice.process_stereo();
                left += voice_left;
                right += voice_right;
            }
        }

        // Track processing time (in microseconds)
        let elapsed_us = start_time.elapsed().as_micros() as f32;
        self.processing_times.push(elapsed_us);

        // Keep only last 1000 samples for rolling average
        if self.processing_times.len() > 1000 {
            self.processing_times.remove(0);
        }

        // Detect underrun: at 44.1kHz, we have ~22.7 microseconds per sample
        // If we're taking longer than 20 microseconds consistently, we're falling behind
        const TIME_BUDGET_US: f32 = 20.0;
        if elapsed_us > TIME_BUDGET_US {
            self.underrun_count += 1;
        }

        // Adaptive threshold adjustment every 10000 samples (~0.22 seconds at 44.1kHz)
        self.samples_since_adjustment += 1;
        if self.samples_since_adjustment >= 10000 {
            self.adjust_parallel_threshold();
            self.samples_since_adjustment = 0;
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

    /// Configure unit mode for the last triggered voice
    /// Must be called immediately after a trigger_sample_* method
    pub fn set_last_voice_unit_mode(&mut self, mode: UnitMode) {
        if let Some(idx) = self.last_triggered_voice_index {
            self.voices[idx].set_unit_mode(mode);
        }
    }

    /// Configure loop mode for the last triggered voice
    /// Must be called immediately after a trigger_sample_* method
    pub fn set_last_voice_loop_enabled(&mut self, enabled: bool) {
        if let Some(idx) = self.last_triggered_voice_index {
            self.voices[idx].set_loop_enabled(enabled);
        }
    }

    /// Configure auto-release time for the last triggered voice (for legato)
    /// Must be called immediately after a trigger_sample_* method
    /// The voice will trigger envelope release when it reaches the specified sample count
    pub fn set_last_voice_auto_release(&mut self, sample_count: usize) {
        if let Some(idx) = self.last_triggered_voice_index {
            self.voices[idx].auto_release_at_sample = Some(sample_count);
        }
    }

    /// Set the source node ID for the last triggered voice
    /// This is used to separate outputs so each output only hears its own samples
    /// Must be called immediately after a trigger_sample_* method
    pub fn set_last_voice_source_node(&mut self, source_node: usize) {
        if let Some(idx) = self.last_triggered_voice_index {
            self.voices[idx].source_node = source_node;
        }
    }

    /// Set the default source node ID for all future trigger calls
    /// This is applied automatically when voices are triggered
    /// More convenient than calling set_last_voice_source_node after each trigger
    pub fn set_default_source_node(&mut self, source_node: usize) {
        self.default_source_node = source_node;
    }

    /// Get number of active voices
    pub fn active_voice_count(&self) -> usize {
        self.voices
            .iter()
            .filter(|v| v.state != VoiceState::Free)
            .count()
    }

    /// Reset all voices
    pub fn reset(&mut self) {
        for voice in &mut self.voices {
            voice.state = VoiceState::Free;
            voice.sample_data = None;
            voice.position = 0.0;
        }
        self.next_voice_index = 0;
    }

    /// Kill all active voices (alias for reset)
    pub fn kill_all(&mut self) {
        self.reset();
    }

    /// Get peak voice count since startup
    pub fn peak_voice_count(&self) -> usize {
        self.peak_voice_count
    }

    /// Get total samples processed
    pub fn total_samples_processed(&self) -> u64 {
        self.total_samples_processed
    }

    /// Get current pool size
    pub fn pool_size(&self) -> usize {
        self.voices.len()
    }

    /// Adjust parallelism threshold based on recent performance
    /// This is called periodically to adapt to workload
    fn adjust_parallel_threshold(&mut self) {
        if self.processing_times.is_empty() {
            return;
        }

        // Calculate average processing time over last period
        let avg_time_us: f32 = self.processing_times.iter().sum::<f32>()
            / self.processing_times.len() as f32;

        // Calculate underrun rate (% of samples that exceeded budget)
        let total_samples = self.processing_times.len() as f32;
        let underrun_rate = self.underrun_count as f32 / total_samples;

        const TIME_BUDGET_US: f32 = 20.0;

        // Decision logic:
        // 1. If underrun rate > 5% AND avg_time > 15us → Lower threshold (enable parallelism earlier)
        // 2. If underrun rate < 1% AND avg_time < 10us → Raise threshold (avoid unnecessary overhead)
        // 3. Otherwise → Keep current threshold

        let old_threshold = self.parallel_threshold;

        if underrun_rate > 0.05 && avg_time_us > 15.0 {
            // We're struggling - enable parallelism earlier
            self.parallel_threshold = (self.parallel_threshold / 2).max(16);
            if std::env::var("DEBUG_ADAPTIVE_PARALLEL").is_ok() {
                eprintln!(
                    "[ADAPTIVE] Lowering threshold {} → {} (underrun rate: {:.1}%, avg time: {:.1}µs)",
                    old_threshold, self.parallel_threshold, underrun_rate * 100.0, avg_time_us
                );
            }
        } else if underrun_rate < 0.01 && avg_time_us < 10.0 {
            // We have headroom - avoid parallelism overhead
            self.parallel_threshold = (self.parallel_threshold * 3 / 2).min(128);
            if std::env::var("DEBUG_ADAPTIVE_PARALLEL").is_ok() {
                eprintln!(
                    "[ADAPTIVE] Raising threshold {} → {} (underrun rate: {:.1}%, avg time: {:.1}µs)",
                    old_threshold, self.parallel_threshold, underrun_rate * 100.0, avg_time_us
                );
            }
        }

        // Reset counters for next period
        self.underrun_count = 0;
        self.processing_times.clear();
    }

    /// Get current adaptive parallelism threshold
    pub fn get_parallel_threshold(&self) -> usize {
        self.parallel_threshold
    }

    /// Get underrun statistics
    pub fn get_underrun_count(&self) -> u64 {
        self.underrun_count
    }

    /// Get average processing time (in microseconds) over recent samples
    pub fn get_avg_processing_time_us(&self) -> f32 {
        if self.processing_times.is_empty() {
            return 0.0;
        }
        self.processing_times.iter().sum::<f32>() / self.processing_times.len() as f32
    }

    /// Get performance statistics as a formatted string
    pub fn performance_summary(&self) -> String {
        let active = self.active_voice_count();
        let pool_size = self.voices.len();
        let usage_pct = if pool_size > 0 {
            (active as f32 / pool_size as f32 * 100.0) as usize
        } else {
            0
        };

        let avg_time = self.get_avg_processing_time_us();
        format!(
            "Voices: {}/{} ({}% usage) | Peak: {} | Samples: {} | Parallel threshold: {} | Avg time: {:.1}µs | Underruns: {}",
            active, pool_size, usage_pct, self.peak_voice_count, self.total_samples_processed,
            self.parallel_threshold, avg_time, self.underrun_count
        )
    }
}
