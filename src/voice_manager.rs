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
use crate::sample_loader::StereoSample;
use rayon::prelude::*;
use std::sync::Arc;

#[cfg(target_arch = "x86_64")]
use crate::voice_simd::{apply_panning_simd_x8, interpolate_samples_simd_x8, is_avx2_supported};

/// Default initial voice pool size (preallocated to avoid growth underruns)
const DEFAULT_INITIAL_VOICES: usize = 256;

/// Absolute maximum voices (hard cap)
/// With parallel SIMD, we can handle 1000-2000 voices in real-time
/// Memory: 4096 × 140 bytes = ~0.56 MB (negligible)
const ABSOLUTE_MAX_VOICES: usize = 4096;

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

/// Vec-based voice buffer storage for O(1) lookup in hot loop
///
/// This replaces HashMap<usize, Vec<f32>> for performance:
/// - HashMap lookup: ~20-50ns per access (hash + probe)
/// - Vec index: ~1ns per access (direct memory access)
///
/// In a 4096-sample buffer with 10 source nodes queried per sample,
/// this saves ~800μs per buffer (40,960 HashMap lookups eliminated).
#[derive(Clone, Debug)]
pub struct VoiceBuffers {
    /// Buffers indexed by source_node_id, each containing buffer_size samples
    /// buffers[node_id][sample_idx] gives O(1) access
    /// Nodes without active voices have empty Vec (zero allocation)
    pub buffers: Vec<Vec<f32>>,

    /// Buffer size (number of samples per node buffer)
    pub buffer_size: usize,

    /// Maximum node ID with active data (for bounds checking)
    pub max_active_node: usize,
}

impl VoiceBuffers {
    /// Create empty VoiceBuffers with capacity for max_node_id
    pub fn new(max_node_id: usize, buffer_size: usize) -> Self {
        let mut buffers = Vec::with_capacity(max_node_id + 1);
        for _ in 0..=max_node_id {
            buffers.push(Vec::new()); // Empty vecs for unused nodes
        }
        Self {
            buffers,
            buffer_size,
            max_active_node: 0,
        }
    }

    /// Get sample value for a node at a specific sample index
    /// Returns 0.0 for nodes without active voices or out-of-bounds access
    #[inline(always)]
    pub fn get(&self, node_id: usize, sample_idx: usize) -> f32 {
        if node_id < self.buffers.len() {
            let buf = &self.buffers[node_id];
            if sample_idx < buf.len() {
                return buf[sample_idx];
            }
        }
        0.0
    }

    /// Check if a node has any active samples
    #[inline(always)]
    pub fn has_data(&self, node_id: usize) -> bool {
        node_id < self.buffers.len() && !self.buffers[node_id].is_empty()
    }

    /// Add samples to a node's buffer (accumulates if buffer exists)
    pub fn add_to_node(&mut self, node_id: usize, samples: &[f32]) {
        // Grow buffers vector if needed
        while self.buffers.len() <= node_id {
            self.buffers.push(Vec::new());
        }

        let buf = &mut self.buffers[node_id];
        if buf.is_empty() {
            // First voice for this node: clone the samples
            *buf = samples.to_vec();
        } else {
            // Accumulate into existing buffer
            for (i, &val) in samples.iter().enumerate() {
                if i < buf.len() {
                    buf[i] += val;
                }
            }
        }

        if node_id > self.max_active_node {
            self.max_active_node = node_id;
        }
    }
}

impl Default for VoiceBuffers {
    fn default() -> Self {
        Self {
            buffers: Vec::new(),
            buffer_size: 0,
            max_active_node: 0,
        }
    }
}

/// A single voice that plays a sample OR generates continuous synthesis
#[derive(Clone)]
pub struct Voice {
    /// The sample data to play (for sample playback mode)
    /// Now supports native stereo samples
    sample_data: Option<Arc<StereoSample>>,

    /// Synthesis node ID (for continuous synthesis mode)
    /// When Some, voice evaluates this node continuously instead of playing sample_data
    /// This enables bus-triggered synthesis without pre-rendering to fixed buffers
    synthesis_node_id: Option<usize>,

    /// Cached synthesis sample (updated each sample by caller before process_stereo)
    /// This is filled by VoiceManager before calling process_stereo()
    synthesis_sample_cache: f32,

    /// Semitone offset for synthesis pitch control (0 = no change, 12 = +1 octave)
    /// Used to implement note parameter: `s "~synth" # note "c4 e4 g4"`
    synthesis_semitone_offset: f32,

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

    /// Buffer trigger offset: which sample in the current buffer this voice was triggered
    /// Used for hybrid architecture to produce zeros before trigger point
    /// None = voice was triggered in previous buffer (render normally)
    /// Some(n) = voice was triggered at sample n in current buffer (produce zeros before n)
    buffer_trigger_offset: Option<usize>,
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
            synthesis_node_id: None,        // No synthesis node by default
            synthesis_sample_cache: 0.0,    // No cached sample yet
            synthesis_semitone_offset: 0.0, // No pitch offset by default
            position: 0.0,
            state: VoiceState::Free,
            gain: 1.0,
            pan: 0.0,
            speed: 1.0,
            age: 0,
            cut_group: None,
            source_node: 0, // Default source node (will be set on trigger)
            envelope: VoiceEnvelope::new_percussion(SAMPLE_RATE, 0.001, 0.1),
            buffer_trigger_offset: None,  // No offset by default
            attack: 0.001,                // 1ms default attack
            release: 0.1,                 // 100ms default release
            unit_mode: UnitMode::Rate,    // Default to rate mode
            loop_enabled: false,          // Default to no looping
            auto_release_at_sample: None, // No auto-release by default
        }
    }

    /// Start playing a sample with pan (backward compatibility, speed=1.0, no cut group)
    pub fn trigger(&mut self, sample: Arc<StereoSample>, gain: f32, pan: f32) {
        self.trigger_with_speed(sample, gain, pan, 1.0);
    }

    /// Start playing a sample with gain, pan, and speed control (no cut group)
    pub fn trigger_with_speed(
        &mut self,
        sample: Arc<StereoSample>,
        gain: f32,
        pan: f32,
        speed: f32,
    ) {
        self.trigger_with_cut_group(sample, gain, pan, speed, None);
    }

    /// Start playing a sample with full control including cut group
    pub fn trigger_with_cut_group(
        &mut self,
        sample: Arc<StereoSample>,
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
        sample: Arc<StereoSample>,
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
        self.buffer_trigger_offset = None; // Will be set by VoiceManager if needed

        // Configure and trigger envelope (recreate as percussion type)
        self.envelope = VoiceEnvelope::new_percussion(SAMPLE_RATE, self.attack, self.release);
        self.envelope.trigger();
    }

    /// Start playing a sample with ADSR envelope
    pub fn trigger_with_adsr(
        &mut self,
        sample: Arc<StereoSample>,
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
        self.buffer_trigger_offset = None; // Will be set by VoiceManager if needed

        // Create and trigger ADSR envelope
        self.envelope = VoiceEnvelope::new_adsr(SAMPLE_RATE, attack, decay, sustain, release);
        self.envelope.trigger();
    }

    /// Start playing a sample with segments envelope
    pub fn trigger_with_segments(
        &mut self,
        sample: Arc<StereoSample>,
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
        self.buffer_trigger_offset = None; // Will be set by VoiceManager if needed

        // Create and trigger segments envelope
        self.envelope = VoiceEnvelope::new_segments(SAMPLE_RATE, levels, times);
        self.envelope.trigger();
    }

    /// Start playing a sample with curve envelope
    pub fn trigger_with_curve(
        &mut self,
        sample: Arc<StereoSample>,
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
        self.buffer_trigger_offset = None; // Will be set by VoiceManager if needed

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
            1.0 // Full gain for reverse playback
        } else {
            self.envelope.process()
        };

        // DEBUG: Log envelope state
        if std::env::var("DEBUG_VOICE_PROCESS").is_ok() && self.age < 10 {
            eprintln!(
                "[VOICE] envelope processed, env_value={:.6}, is_active={}",
                env_value,
                self.envelope.is_active()
            );
        }

        // Auto-release for legato: trigger release at exact sample count
        if let Some(release_at) = self.auto_release_at_sample {
            if self.age >= release_at {
                self.envelope.release();
                self.auto_release_at_sample = None; // Only trigger once
            }
        }

        // Handle continuous synthesis mode
        if let Some(_synthesis_node_id) = self.synthesis_node_id {
            // Check if envelope finished for synthesis voice
            if !self.envelope.is_active() {
                if std::env::var("DEBUG_VOICE_PROCESS").is_ok() {
                    eprintln!("[VOICE] synthesis envelope finished, freeing voice");
                }
                self.state = VoiceState::Free;
                self.synthesis_node_id = None;
                return (0.0, 0.0);
            }
            // Use cached synthesis sample (populated by VoiceManager before calling process_stereo)
            let sample_value = self.synthesis_sample_cache;

            // Apply gain and envelope
            let output_value = sample_value * self.gain * env_value;

            // Increment age (synthesis continues indefinitely until envelope finishes)
            self.age += 1;

            // Equal-power panning
            let pan_radians = (self.pan + 1.0) * std::f32::consts::FRAC_PI_4;
            let left_gain = pan_radians.cos();
            let right_gain = pan_radians.sin();

            let left = output_value * left_gain;
            let right = output_value * right_gain;

            return (left, right);
        }

        // Handle sample playback mode
        if let Some(ref sample) = self.sample_data {
            let sample_len = sample.len() as f32;

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
                // Get interpolated stereo sample (handles both mono and stereo)
                let (sample_left, sample_right) = sample.get_interpolated(self.position);

                // Apply gain and envelope
                let gained_left = sample_left * self.gain * env_value;
                let gained_right = sample_right * self.gain * env_value;

                // Advance position by speed (negative speed moves backward)
                self.position += self.speed;
                self.age += 1;

                // Apply panning
                // For stereo samples: pan adjusts stereo width (0 = full stereo, -1/+1 = collapse to mono on that side)
                // For mono samples: traditional equal-power panning
                let (left, right) = if sample.is_stereo() {
                    // For stereo samples, pan controls balance/position
                    // pan = 0: full stereo (left output = sample left, right output = sample right)
                    // pan = -1: left only (mono collapse to left)
                    // pan = +1: right only (mono collapse to right)
                    let pan_radians = (self.pan + 1.0) * std::f32::consts::FRAC_PI_4;
                    let pan_left = pan_radians.cos();
                    let pan_right = pan_radians.sin();

                    // Blend: at center (pan=0), output stereo directly
                    // At extremes, collapse to mono on that side
                    let center_factor = 1.0 - self.pan.abs();
                    let mono_sum = (gained_left + gained_right) * 0.5;

                    let left_out = gained_left * center_factor * pan_left
                        + mono_sum * (1.0 - center_factor) * pan_left;
                    let right_out = gained_right * center_factor * pan_right
                        + mono_sum * (1.0 - center_factor) * pan_right;

                    (left_out, right_out)
                } else {
                    // Mono sample: traditional equal-power panning
                    let pan_radians = (self.pan + 1.0) * std::f32::consts::FRAC_PI_4;
                    let left_gain = pan_radians.cos();
                    let right_gain = pan_radians.sin();
                    (gained_left * left_gain, gained_right * right_gain)
                };

                // Check if envelope finished for sample voice
                if !self.envelope.is_active() {
                    self.state = VoiceState::Free;
                    self.sample_data = None;
                }

                (left, right)
            } else {
                // Sample finished - trigger envelope release if not already releasing
                // This ensures ADSR envelopes properly fade out instead of staying in Sustain
                self.envelope.release();

                // Check if envelope also finished
                if !self.envelope.is_active() {
                    self.state = VoiceState::Free;
                    self.sample_data = None;
                }
                (0.0, 0.0)
            }
        } else {
            // No sample data and no synthesis node - free the voice
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
    /// Create a new VoiceManager with unlimited voices (grows dynamically from 256)
    /// Preallocates 256 voices to avoid underruns during startup
    pub fn new() -> Self {
        Self::with_config(DEFAULT_INITIAL_VOICES, Some(ABSOLUTE_MAX_VOICES))
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
        let initial_voices = initial_voices.max(1).min(ABSOLUTE_MAX_VOICES);
        let max_voices = max_voices.map(|m| m.min(ABSOLUTE_MAX_VOICES));

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
            parallel_threshold: 8, // Very aggressive threshold - enable parallelism with just 8 voices (optimized for 16-core systems)
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

        // NOTE: Use module-level ABSOLUTE_MAX_VOICES constant (4096)
        // Do NOT shadow it with a local constant!

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

        // Check absolute maximum (uses module-level constant = 4096)
        if current_count >= ABSOLUTE_MAX_VOICES {
            eprintln!(
                "⚠️  Absolute voice limit reached: {} voices (hard cap: {})",
                current_count, ABSOLUTE_MAX_VOICES
            );
            eprintln!(
                "    Consider using 'cut groups' or longer envelopes to reduce overlapping samples"
            );
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
    pub fn trigger_sample(&mut self, sample: Arc<StereoSample>, gain: f32) {
        self.trigger_sample_with_pan(sample, gain, 0.0);
    }

    /// Trigger a sample with automatic voice allocation and pan control
    pub fn trigger_sample_with_pan(&mut self, sample: Arc<StereoSample>, gain: f32, pan: f32) {
        self.trigger_sample_with_params(sample, gain, pan, 1.0);
    }

    /// Trigger a sample with full DSP parameter control (gain, pan, speed, no cut group)
    pub fn trigger_sample_with_params(
        &mut self,
        sample: Arc<StereoSample>,
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
        sample: Arc<StereoSample>,
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
        sample: Arc<StereoSample>,
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
                    eprintln!(
                        "[VOICE_MGR] Voice {} triggered, state={:?}, envelope_active={}",
                        idx,
                        self.voices[idx].state,
                        self.voices[idx].envelope.is_active()
                    );
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
        sample: Arc<StereoSample>,
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
        sample: Arc<StereoSample>,
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
        sample: Arc<StereoSample>,
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

    /// Trigger a continuous synthesis voice (no pre-rendered buffer)
    /// The synthesis_node_id will be evaluated continuously for each sample
    /// This enables bus-triggered synthesis without pre-rendering to fixed buffers
    /// semitone_offset: pitch offset in semitones (0 = no change, 12 = +1 octave, -12 = -1 octave)
    pub fn trigger_synthesis_voice(
        &mut self,
        synthesis_node_id: usize,
        gain: f32,
        pan: f32,
        cut_group: Option<u32>,
        attack: f32,
        release: f32,
        semitone_offset: f32,
    ) {
        // DEBUG: Log synthesis voice triggers
        if std::env::var("DEBUG_VOICE_TRIGGERS").is_ok() {
            eprintln!(
                "[VOICE_MGR] trigger_synthesis_voice: node_id={}, semitone_offset={:.1}",
                synthesis_node_id, semitone_offset
            );
        }

        // If this has a cut group, fade out all other voices in the same cut group
        if let Some(group) = cut_group {
            for voice in &mut self.voices {
                if voice.cut_group == Some(group) && voice.state != VoiceState::Free {
                    voice.envelope.trigger_quick_release(0.01); // 10ms fade-out
                }
            }
        }

        // Try to find an inactive voice
        let max_voices = self.voices.len();
        for i in 0..max_voices {
            let idx = (self.next_voice_index + i) % max_voices;
            if self.voices[idx].is_available() {
                // Configure voice for continuous synthesis
                self.voices[idx].synthesis_node_id = Some(synthesis_node_id);
                self.voices[idx].sample_data = None; // Clear any sample data
                self.voices[idx].synthesis_sample_cache = 0.0; // Will be filled during processing
                self.voices[idx].synthesis_semitone_offset = semitone_offset; // Pitch offset for note parameter
                self.voices[idx].state = VoiceState::Playing;
                self.voices[idx].gain = gain;
                self.voices[idx].pan = pan;
                self.voices[idx].speed = 1.0; // Speed doesn't apply to synthesis
                self.voices[idx].position = 0.0;
                self.voices[idx].age = 0;
                self.voices[idx].cut_group = cut_group;
                self.voices[idx].source_node = self.default_source_node;
                self.voices[idx].envelope =
                    VoiceEnvelope::new_percussion(SAMPLE_RATE, attack, release);
                self.voices[idx].envelope.trigger(); // CRITICAL: Start the envelope!
                self.voices[idx].attack = attack;
                self.voices[idx].release = release;

                self.next_voice_index = (idx + 1) % max_voices;
                self.last_triggered_voice_index = Some(idx);

                // DEBUG: Verify voice state
                if std::env::var("DEBUG_VOICE_TRIGGERS").is_ok() {
                    eprintln!(
                        "[VOICE_MGR] Synthesis voice {} triggered, state={:?}",
                        idx, self.voices[idx].state
                    );
                }
                return;
            }
        }

        // All voices active - try to grow the pool
        if self.grow_voice_pool() {
            let idx = self.voices.len() - 1;
            self.voices[idx].synthesis_node_id = Some(synthesis_node_id);
            self.voices[idx].sample_data = None;
            self.voices[idx].synthesis_sample_cache = 0.0;
            self.voices[idx].synthesis_semitone_offset = semitone_offset; // Pitch offset for note parameter
            self.voices[idx].state = VoiceState::Playing;
            self.voices[idx].gain = gain;
            self.voices[idx].pan = pan;
            self.voices[idx].speed = 1.0;
            self.voices[idx].position = 0.0;
            self.voices[idx].age = 0;
            self.voices[idx].cut_group = cut_group;
            self.voices[idx].source_node = self.default_source_node;
            self.voices[idx].envelope = VoiceEnvelope::new_percussion(SAMPLE_RATE, attack, release);
            self.voices[idx].envelope.trigger(); // CRITICAL: Start the envelope!
            self.voices[idx].attack = attack;
            self.voices[idx].release = release;

            self.next_voice_index = 0;
            self.last_triggered_voice_index = Some(idx);
            return;
        }

        // Growth failed - steal oldest voice
        let mut oldest_idx = 0;
        let mut oldest_age = 0;

        for (idx, voice) in self.voices.iter().enumerate() {
            if voice.age > oldest_age {
                oldest_age = voice.age;
                oldest_idx = idx;
            }
        }

        self.voices[oldest_idx].synthesis_node_id = Some(synthesis_node_id);
        self.voices[oldest_idx].sample_data = None;
        self.voices[oldest_idx].synthesis_sample_cache = 0.0;
        self.voices[oldest_idx].state = VoiceState::Playing;
        self.voices[oldest_idx].gain = gain;
        self.voices[oldest_idx].pan = pan;
        self.voices[oldest_idx].speed = 1.0;
        self.voices[oldest_idx].position = 0.0;
        self.voices[oldest_idx].age = 0;
        self.voices[oldest_idx].cut_group = cut_group;
        self.voices[oldest_idx].source_node = self.default_source_node;
        self.voices[oldest_idx].envelope =
            VoiceEnvelope::new_percussion(SAMPLE_RATE, attack, release);
        self.voices[oldest_idx].envelope.trigger(); // CRITICAL: Start the envelope!
        self.voices[oldest_idx].attack = attack;
        self.voices[oldest_idx].release = release;

        self.next_voice_index = (oldest_idx + 1) % max_voices;
        self.last_triggered_voice_index = Some(oldest_idx);
    }

    /// Get synthesis node IDs and semitone offsets for all active synthesis voices
    /// Returns (node_id, semitone_offset) pairs for per-voice pitch shifting
    /// Used to pre-query which (node, pitch) combinations need evaluation
    pub fn get_active_synthesis_node_ids_with_pitch(&self) -> Vec<(usize, f32)> {
        self.voices
            .iter()
            .filter_map(|v| {
                if v.synthesis_node_id.is_some() && v.state != VoiceState::Free {
                    v.synthesis_node_id
                        .map(|node_id| (node_id, v.synthesis_semitone_offset))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Update synthesis sample cache with pre-computed samples
    /// Takes a HashMap of node_id -> sample_value
    pub fn update_synthesis_cache_with_samples(
        &mut self,
        samples: &std::collections::HashMap<usize, f32>,
    ) {
        for voice in &mut self.voices {
            if let Some(node_id) = voice.synthesis_node_id {
                if voice.state != VoiceState::Free {
                    if let Some(&sample) = samples.get(&node_id) {
                        voice.synthesis_sample_cache = sample;
                        if std::env::var("DEBUG_SYNTHESIS_CACHE").is_ok() && sample.abs() > 0.001 {
                            eprintln!(
                                "[CACHE] Updated voice with node_id={} to cache={:.6}",
                                node_id, sample
                            );
                        }
                    }
                }
            }
        }
    }

    /// Process all synthesis voices for one sample
    /// Returns Vec of ((left, right), source_node) for each active synthesis voice
    pub fn process_synthesis_voices(&mut self) -> Vec<((f32, f32), usize)> {
        let outputs: Vec<((f32, f32), usize)> = self.voices.iter_mut()
            .filter(|v| v.synthesis_node_id.is_some() && v.state != VoiceState::Free)
            .map(|v| {
                let output = v.process_stereo();
                if std::env::var("DEBUG_SYNTHESIS_VOICE").is_ok() && output.0.abs() + output.1.abs() > 0.001 {
                    eprintln!("[SYNTHESIS] Voice node_id={:?} produced output: ({:.6}, {:.6}), envelope_active={}, cache={:.6}",
                        v.synthesis_node_id, output.0, output.1, v.envelope.is_active(), v.synthesis_sample_cache);
                }
                (output, v.source_node)
            })
            .collect();

        if std::env::var("DEBUG_SYNTHESIS_VOICE").is_ok() && !outputs.is_empty() {
            eprintln!("[SYNTHESIS] Processed {} synthesis voices", outputs.len());
        }

        outputs
    }

    /// Process one sample from all active voices (mono)
    pub fn process(&mut self) -> f32 {
        let (left, right) = self.process_stereo();
        // Mix down to mono with compensation for equal-power panning
        // At center pan, left=right=value*sqrt(0.5), so (left+right)=value*sqrt(2)
        // Divide by sqrt(2) to restore original amplitude
        (left + right) / std::f32::consts::SQRT_2
    }

    /// Process all voices and return per-node stereo mixes (HashMap: source_node -> (left, right))
    /// This is the stereo version - returns full stereo output without mixing down
    pub fn process_per_node_stereo(&mut self) -> std::collections::HashMap<usize, (f32, f32)> {
        use std::collections::HashMap;

        // PERFORMANCE: Use parallel processing for high voice counts
        let voice_outputs: Vec<((f32, f32), usize)> =
            if self.voices.len() >= self.parallel_threshold {
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

        node_sums
    }

    /// Process all voices and return per-node mixes (HashMap: source_node -> mono_output)
    /// This allows multiple outputs to have independent sample streams
    /// Each voice is processed ONCE, then outputs are grouped by source_node
    pub fn process_per_node(&mut self) -> std::collections::HashMap<usize, f32> {
        // Use stereo version and convert to mono
        self.process_per_node_stereo()
            .into_iter()
            .map(|(node, (left, right))| {
                let mono = (left + right) / std::f32::consts::SQRT_2;
                (node, mono)
            })
            .collect()
    }

    /// SIMD-accelerated batch processing: Process 8 voices simultaneously
    /// Using AVX2 intrinsics for 3× speedup on interpolation and panning
    #[cfg(target_arch = "x86_64")]
    fn process_voice_batch_simd(
        voices: &mut [Voice], // Exactly 8 voices
        output: &mut Vec<std::collections::HashMap<usize, f32>>,
        buffer_size: usize,
    ) {
        // Defensive check instead of assertion to prevent crashes during live reload
        if voices.len() != 8 {
            eprintln!(
                "⚠️  SIMD batch size mismatch: expected 8 voices, got {}. Skipping batch.",
                voices.len()
            );
            return;
        }

        // Process each sample in the buffer
        for sample_idx in 0..buffer_size {
            // Arrays to hold data from 8 voices for SIMD processing
            let mut positions = [0.0f32; 8];
            let mut samples_curr = [0.0f32; 8];
            let mut samples_next = [0.0f32; 8];
            let mut pans = [0.0f32; 8];
            let mut gains_envs = [0.0f32; 8];
            let mut source_nodes = [0usize; 8];
            let mut active_mask = [false; 8];

            // Extract data from each voice (scalar)
            for (i, voice) in voices.iter_mut().enumerate() {
                if voice.state == VoiceState::Free {
                    continue; // Skip free voices
                }

                // Process envelope (scalar - complex state machine)
                let env_value = if voice.speed < 0.0 {
                    1.0 // Full gain for reverse playback
                } else {
                    voice.envelope.process()
                };

                // Auto-release check
                if let Some(release_at) = voice.auto_release_at_sample {
                    if voice.age >= release_at {
                        voice.envelope.release();
                        voice.auto_release_at_sample = None;
                    }
                }

                // Check if envelope finished
                if !voice.envelope.is_active() {
                    voice.state = VoiceState::Free;
                    voice.sample_data = None;
                    continue;
                }

                // Extract sample data for SIMD processing
                if let Some(ref sample) = voice.sample_data {
                    let sample_len = sample.len() as f32;

                    // Handle looping
                    if voice.loop_enabled {
                        if voice.speed >= 0.0 && voice.position >= sample_len {
                            voice.position = voice.position % sample_len;
                        } else if voice.speed < 0.0 && voice.position < 0.0 {
                            voice.position = sample_len - 1.0 + (voice.position % sample_len);
                        }
                    }

                    // Check bounds
                    let is_in_bounds = voice.position >= 0.0 && voice.position < sample_len;

                    if is_in_bounds {
                        let pos_floor = voice.position.floor() as usize;

                        // Extract data for SIMD (only for forward playback with next sample available)
                        // Use left channel for SIMD batch processing (mono-ized path for performance)
                        if voice.speed >= 0.0 && pos_floor + 1 < sample.left.len() {
                            positions[i] = voice.position;
                            samples_curr[i] = sample.left[pos_floor];
                            samples_next[i] = sample.left[pos_floor + 1];
                            pans[i] = voice.pan;
                            gains_envs[i] = voice.gain * env_value;
                            source_nodes[i] = voice.source_node;
                            active_mask[i] = true;
                        } else {
                            // Fallback to scalar for reverse/edge cases
                            // Use mono interpolation for this code path
                            let sample_value = sample.get_mono_interpolated(voice.position);

                            // Apply panning and gain (scalar)
                            let output_value = sample_value * voice.gain * env_value;
                            let pan_radians = (voice.pan + 1.0) * std::f32::consts::FRAC_PI_4;
                            let left_gain = pan_radians.cos();
                            let right_gain = pan_radians.sin();
                            let left = output_value * left_gain;
                            let right = output_value * right_gain;
                            let mono = (left + right) / std::f32::consts::SQRT_2;

                            output[sample_idx]
                                .entry(voice.source_node)
                                .and_modify(|v| *v += mono)
                                .or_insert(mono);
                        }
                    }
                }
            }

            // SIMD processing for voices that can use it
            unsafe {
                // Interpolate all 8 samples simultaneously
                let interpolated =
                    interpolate_samples_simd_x8(&positions, &samples_curr, &samples_next);

                // Apply gains/envelopes (element-wise multiply)
                let mut gained = [0.0f32; 8];
                for i in 0..8 {
                    gained[i] = interpolated[i] * gains_envs[i];
                }

                // Pan all 8 voices simultaneously
                let (left_batch, right_batch) = apply_panning_simd_x8(&gained, &pans);

                // Accumulate to output (only for active voices)
                for i in 0..8 {
                    if active_mask[i] {
                        let mono = (left_batch[i] + right_batch[i]) / std::f32::consts::SQRT_2;
                        output[sample_idx]
                            .entry(source_nodes[i])
                            .and_modify(|v| *v += mono)
                            .or_insert(mono);
                    }
                }
            }

            // Advance voice positions (scalar - state update)
            for voice in voices.iter_mut() {
                if voice.state != VoiceState::Free {
                    voice.position += voice.speed;
                    voice.age += 1;
                }
            }
        }
    }

    /// Process buffer with PARALLEL SIMD batches using scoped threads
    ///
    /// This achieves the multiplicative speedup: SIMD (3×) × Threading (2-4×) = 6-12×
    ///
    /// Uses crossbeam::scope to safely pass mutable voice slices to parallel threads.
    /// Each thread processes a batch of 8 voices with SIMD, then results are merged.
    #[cfg(target_arch = "x86_64")]
    fn process_buffer_parallel_simd(
        &mut self,
        buffer_size: usize,
    ) -> Vec<std::collections::HashMap<usize, f32>> {
        use crossbeam::thread;
        use std::collections::HashMap;

        let num_full_batches = self.voices.len() / 8;
        let remainder_start = num_full_batches * 8;

        // Pre-allocate output for each batch (will be merged later)
        let mut batch_outputs: Vec<Vec<HashMap<usize, f32>>> = Vec::with_capacity(num_full_batches);

        // Split voices into mutable chunks of 8 for parallel processing
        let (batches, remainder) = self.voices.split_at_mut(remainder_start);

        // Process batches in parallel using scoped threads
        let scope_result = thread::scope(|s| {
            let handles: Vec<_> = batches
                .chunks_exact_mut(8)
                .map(|chunk| {
                    s.spawn(move |_| {
                        let mut local_output = vec![HashMap::new(); buffer_size];
                        Self::process_voice_batch_simd(chunk, &mut local_output, buffer_size);
                        local_output
                    })
                })
                .collect();

            // Collect results from all threads, handling panics gracefully
            for handle in handles {
                match handle.join() {
                    Ok(output) => batch_outputs.push(output),
                    Err(e) => {
                        eprintln!("⚠️  SIMD thread panicked: {:?}. Skipping batch to prevent audio dropout.", e);
                        // Push empty output to maintain buffer structure
                        batch_outputs.push(vec![HashMap::new(); buffer_size]);
                    }
                }
            }
        });

        // Handle scope panic gracefully
        if let Err(e) = scope_result {
            eprintln!(
                "⚠️  Thread scope panicked: {:?}. Returning silent output.",
                e
            );
            return vec![HashMap::new(); buffer_size];
        }

        // Process remainder voices (non-multiple of 8) with scalar
        let mut remainder_output = vec![HashMap::new(); buffer_size];
        for voice in remainder.iter_mut() {
            let source_node = voice.source_node;
            for i in 0..buffer_size {
                let (l, r) = voice.process_stereo();
                let mono = (l + r) / std::f32::consts::SQRT_2;
                remainder_output[i]
                    .entry(source_node)
                    .and_modify(|v| *v += mono)
                    .or_insert(mono);
            }
        }

        // Merge all outputs into final result
        let mut final_output = vec![HashMap::new(); buffer_size];

        // Merge batch outputs
        for batch_output in batch_outputs {
            for (i, sample_map) in batch_output.into_iter().enumerate() {
                for (node_id, value) in sample_map {
                    final_output[i]
                        .entry(node_id)
                        .and_modify(|v| *v += value)
                        .or_insert(value);
                }
            }
        }

        // Merge remainder output
        for (i, sample_map) in remainder_output.into_iter().enumerate() {
            for (node_id, value) in sample_map {
                final_output[i]
                    .entry(node_id)
                    .and_modify(|v| *v += value)
                    .or_insert(value);
            }
        }

        final_output
    }

    /// OPTIMIZED: Process an entire buffer from all voices grouped by source node
    /// This is MUCH faster than calling process_per_node() per sample because:
    /// - Rayon threads spawned ONCE instead of N times
    /// - HashMap created ONCE instead of N times
    /// - Better cache locality (process same voice consecutively)
    ///
    /// SIMD ACCELERATION: When AVX2 is available and ≥8 voices active, processes
    /// voices in batches of 8 using SIMD for 2-3× speedup
    ///
    /// PARALLEL SIMD: When ≥16 voices, processes multiple SIMD batches in parallel
    /// using scoped threads for additional 2-4× speedup (6-12× total with SIMD)
    /// Process all voices for the entire buffer, returning buffers grouped by source_node.
    /// Returns HashMap<source_node, Vec<f32>> - one buffer per source node.
    /// This is much more efficient than the old Vec<HashMap> (one HashMap per sample).
    pub fn process_buffer_per_node(
        &mut self,
        buffer_size: usize,
    ) -> std::collections::HashMap<usize, Vec<f32>> {
        use std::collections::HashMap;

        // Output: source_node -> buffer of samples (much more efficient than HashMap per sample!)
        let mut output: HashMap<usize, Vec<f32>> = HashMap::new();

        if self.voices.is_empty() {
            return output;
        }

        // Process each voice for the ENTIRE buffer
        // Skip synthesis voices - they're processed sample-by-sample in main loop
        if self.voices.len() >= self.parallel_threshold {
            // Parallel: process voices in parallel, each generating full buffer
            let voice_buffers: Vec<(Vec<f32>, usize)> = self
                .voices
                .par_iter_mut()
                .filter(|v| v.synthesis_node_id.is_none())
                .map(|voice| {
                    let mut buffer = Vec::with_capacity(buffer_size);
                    for _ in 0..buffer_size {
                        let (l, r) = voice.process_stereo();
                        let mono = (l + r) / std::f32::consts::SQRT_2;
                        buffer.push(mono);
                    }
                    (buffer, voice.source_node)
                })
                .collect();

            // Accumulate voice buffers by source_node
            for (voice_buffer, source_node) in voice_buffers {
                output
                    .entry(source_node)
                    .and_modify(|existing: &mut Vec<f32>| {
                        for (i, &val) in voice_buffer.iter().enumerate() {
                            existing[i] += val;
                        }
                    })
                    .or_insert(voice_buffer);
            }
        } else {
            // Sequential processing
            for voice in &mut self.voices {
                if voice.synthesis_node_id.is_some() {
                    continue;
                }

                let source_node = voice.source_node;

                // Get or create buffer for this source_node
                let buffer = output
                    .entry(source_node)
                    .or_insert_with(|| vec![0.0; buffer_size]);

                for i in 0..buffer_size {
                    let (l, r) = voice.process_stereo();
                    let mono = (l + r) / std::f32::consts::SQRT_2;
                    buffer[i] += mono;
                }
            }
        }

        output
    }

    /// OPTIMIZED: Process all voices for the entire buffer, returning Vec-based buffers.
    /// Returns VoiceBuffers with O(1) lookup by node_id and sample_idx.
    ///
    /// This is the high-performance version that eliminates HashMap overhead in the hot loop:
    /// - No HashMap allocation per sample
    /// - No hash computation or probing
    /// - Direct array indexing: buffers[node_id][sample_idx]
    ///
    /// Caller provides max_node_id to pre-size the buffers vector.
    pub fn process_buffer_vec(&mut self, buffer_size: usize, max_node_id: usize) -> VoiceBuffers {
        let mut output = VoiceBuffers::new(max_node_id, buffer_size);

        if self.voices.is_empty() {
            return output;
        }

        // Process each voice for the ENTIRE buffer
        // Skip synthesis voices - they're processed sample-by-sample in main loop
        if self.voices.len() >= self.parallel_threshold {
            // Parallel: process voices in parallel, each generating full buffer
            let voice_buffers: Vec<(Vec<f32>, usize)> = self
                .voices
                .par_iter_mut()
                .filter(|v| v.synthesis_node_id.is_none())
                .map(|voice| {
                    let mut buffer = Vec::with_capacity(buffer_size);
                    for _ in 0..buffer_size {
                        let (l, r) = voice.process_stereo();
                        let mono = (l + r) / std::f32::consts::SQRT_2;
                        buffer.push(mono);
                    }
                    (buffer, voice.source_node)
                })
                .collect();

            // Accumulate voice buffers by source_node into VoiceBuffers
            for (voice_buffer, source_node) in voice_buffers {
                output.add_to_node(source_node, &voice_buffer);
            }
        } else {
            // Sequential processing
            for voice in &mut self.voices {
                if voice.synthesis_node_id.is_some() {
                    continue;
                }

                let source_node = voice.source_node;

                // Render full buffer for this voice
                let mut voice_buffer = Vec::with_capacity(buffer_size);
                for _ in 0..buffer_size {
                    let (l, r) = voice.process_stereo();
                    let mono = (l + r) / std::f32::consts::SQRT_2;
                    voice_buffer.push(mono);
                }

                // Add to output buffers
                output.add_to_node(source_node, &voice_buffer);
            }
        }

        output
    }

    /// Process synthesis voices with pre-generated buffers
    ///
    /// This mirrors process_buffer_per_node() but for synthesis voices with
    /// pre-generated audio buffers. Applies envelope per-sample and returns
    /// mixed output in the same format as process_buffer_per_node().
    ///
    /// This enables SIMD auto-vectorization by processing entire buffers at once.
    ///
    /// The synthesis_buffers HashMap uses (node_id, semitone_key) as key to support
    /// per-voice pitch shifting for chords.
    pub fn process_synthesis_buffers(
        &mut self,
        synthesis_buffers: &std::collections::HashMap<(usize, i32), Vec<f32>>,
        buffer_size: usize,
    ) -> Vec<std::collections::HashMap<usize, f32>> {
        use std::collections::HashMap;

        // Pre-allocate output: one HashMap per sample in buffer
        let mut output: Vec<HashMap<usize, f32>> = vec![HashMap::new(); buffer_size];

        // Process each synthesis voice
        for voice in &mut self.voices {
            // Only process synthesis voices
            if let Some(synthesis_node_id) = voice.synthesis_node_id {
                // Calculate buffer key from voice's semitone offset
                // Round to avoid floating point precision issues
                let semitone_key = (voice.synthesis_semitone_offset * 100.0).round() as i32;
                let buffer_key = (synthesis_node_id, semitone_key);

                // Find the pre-generated buffer for this (node, pitch) combination
                if let Some(synth_buffer) = synthesis_buffers.get(&buffer_key) {
                    let source_node = voice.source_node;

                    // DEBUG: Log successful buffer lookup
                    if std::env::var("DEBUG_SYNTH_LOOKUP").is_ok() {
                        let sum: f32 = synth_buffer.iter().sum();
                        let sample_10 = synth_buffer.get(10).copied().unwrap_or(0.0);
                        eprintln!("[SYNTH_LOOKUP] Found buffer key=({}, {}), source={}, sum={:.2}, sample[10]={:.6}",
                            synthesis_node_id, semitone_key, source_node, sum, sample_10);
                    }

                    // Process each sample in the buffer
                    for i in 0..buffer_size.min(synth_buffer.len()) {
                        // Get raw synthesis sample
                        let sample_value = synth_buffer[i];

                        // Process envelope
                        let env_value = voice.envelope.process();

                        // Auto-release for legato
                        if let Some(release_at) = voice.auto_release_at_sample {
                            if voice.age >= release_at {
                                voice.envelope.release();
                                voice.auto_release_at_sample = None;
                            }
                        }

                        // Apply gain and envelope
                        let output_value = sample_value * voice.gain * env_value;

                        // Increment age
                        voice.age += 1;

                        // Equal-power panning
                        let pan_radians = (voice.pan + 1.0) * std::f32::consts::FRAC_PI_4;
                        let left_gain = pan_radians.cos();
                        let right_gain = pan_radians.sin();

                        let left = output_value * left_gain;
                        let right = output_value * right_gain;

                        // Convert stereo to mono for voice_buffers
                        let mono = (left + right) / std::f32::consts::SQRT_2;

                        // Accumulate into output
                        let old_val = output[i].get(&source_node).copied().unwrap_or(0.0);
                        output[i]
                            .entry(source_node)
                            .and_modify(|v| *v += mono)
                            .or_insert(mono);

                        // DEBUG: Log accumulation for first few samples
                        if std::env::var("DEBUG_SYNTH_OUTPUT").is_ok() && i < 3 {
                            eprintln!("[SYNTH_OUT] i={}, key={}, env={:.4}, mono={:.6}, old={:.6}, new={:.6}",
                                i, semitone_key, env_value, mono, old_val, old_val + mono);
                        }

                        // Check if envelope finished
                        if !voice.envelope.is_active() {
                            voice.state = VoiceState::Free;
                            voice.synthesis_node_id = None;
                            break; // Stop processing this voice
                        }
                    }
                } else if std::env::var("DEBUG_SYNTH_LOOKUP").is_ok() {
                    eprintln!(
                        "[SYNTH_LOOKUP] WARNING: No buffer for key=({}, {}), voice semitone={}",
                        synthesis_node_id, semitone_key, voice.synthesis_semitone_offset
                    );
                }
            }
        }

        output
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
            eprintln!(
                "[VOICE_MGR] process_stereo: total_voices={}, active_voices={}",
                self.voices.len(),
                active_count
            );
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

    /// Render block of samples with output organized by source node
    /// Returns one buffer per source node (for hybrid architecture)
    ///
    /// This is the new API for hybrid block processing where:
    /// - Pattern evaluation triggers voices (sample-accurate)
    /// - Voice rendering produces buffers (block-based, this method)
    /// - DSP processing reads from buffers (block-based, no recursion)
    ///
    /// Returns HashMap where:
    /// - Key: source_node_id (from set_default_source_node)
    /// - Value: Vec<f32> buffer of block_size samples
    pub fn render_block(
        &mut self,
        block_size: usize,
    ) -> std::collections::HashMap<usize, Vec<f32>> {
        use std::collections::HashMap;

        if self.voices.is_empty() {
            return HashMap::new();
        }

        // Initialize output buffers for each unique source node
        let mut output: HashMap<usize, Vec<f32>> = HashMap::new();

        // PARALLEL: Process voices in parallel when count is high
        if self.voices.len() >= self.parallel_threshold {
            // Each voice renders its full buffer independently
            let voice_buffers: Vec<(Vec<f32>, usize)> = self
                .voices
                .par_iter_mut()
                .map(|voice| {
                    let source_node = voice.source_node;
                    let trigger_offset = voice.buffer_trigger_offset.unwrap_or(0);
                    let mut buffer = Vec::with_capacity(block_size);

                    // Produce zeros before trigger offset
                    for _ in 0..trigger_offset {
                        buffer.push(0.0);
                    }

                    // Process audio from trigger offset onwards
                    for _ in trigger_offset..block_size {
                        let (l, r) = voice.process_stereo();
                        let mono = (l + r) / std::f32::consts::SQRT_2;
                        buffer.push(mono);
                    }

                    // Clear trigger offset (it's per-buffer)
                    // Note: This is safe because we're in par_iter_mut
                    // voice.buffer_trigger_offset = None;  // Can't do this in parallel

                    (buffer, source_node)
                })
                .collect();

            // Accumulate voice buffers by source node
            for (voice_buffer, source_node) in voice_buffers {
                output
                    .entry(source_node)
                    .and_modify(|node_buffer| {
                        // Accumulate into existing buffer
                        for (i, sample) in voice_buffer.iter().enumerate() {
                            node_buffer[i] += sample;
                        }
                    })
                    .or_insert(voice_buffer); // First voice for this node
            }
        } else {
            // SEQUENTIAL: For low voice counts, process sequentially
            for voice in &mut self.voices {
                let source_node = voice.source_node;
                let trigger_offset = voice.buffer_trigger_offset.unwrap_or(0);

                // Ensure buffer exists for this node
                let node_buffer = output
                    .entry(source_node)
                    .or_insert_with(|| vec![0.0; block_size]);

                // Render voice and accumulate into node buffer
                for i in 0..block_size {
                    if i < trigger_offset {
                        // Before trigger offset: produce zero (don't call process_stereo())
                        // node_buffer[i] += 0.0; // No-op
                    } else {
                        // After trigger offset: process normally
                        let (l, r) = voice.process_stereo();
                        let mono = (l + r) / std::f32::consts::SQRT_2;
                        node_buffer[i] += mono;
                    }
                }

                // Clear trigger offset after rendering this buffer
                voice.buffer_trigger_offset = None;
            }
        }

        // Clear trigger offsets for parallel-rendered voices
        for voice in &mut self.voices {
            voice.buffer_trigger_offset = None;
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

    /// Set buffer trigger offset for the last triggered voice (for hybrid architecture)
    /// Must be called immediately after a trigger_sample_* method
    /// The voice will produce zeros before this sample offset in the current buffer
    pub fn set_last_voice_trigger_offset(&mut self, offset: usize) {
        if let Some(idx) = self.last_triggered_voice_index {
            self.voices[idx].buffer_trigger_offset = Some(offset);
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

    /// Process the last triggered voice for one sample (stereo output)
    /// Used for immediate playback of newly triggered voices within a buffer
    pub fn process_last_voice_stereo(&mut self) -> (f32, f32) {
        // Find the last active voice (most recently triggered)
        if let Some(last_voice) = self
            .voices
            .iter_mut()
            .rev()
            .find(|v| v.state != VoiceState::Free)
        {
            last_voice.process_stereo()
        } else {
            (0.0, 0.0)
        }
    }

    /// Process a specific voice by index and return (stereo output, source_node)
    /// Returns None if voice index is invalid or voice is free
    pub fn process_voice_by_index(&mut self, index: usize) -> Option<((f32, f32), usize)> {
        if index < self.voices.len() {
            let voice = &mut self.voices[index];
            if voice.state != VoiceState::Free {
                let output = voice.process_stereo();
                let source_node = voice.source_node;
                return Some((output, source_node));
            }
        }
        None
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
        let avg_time_us: f32 =
            self.processing_times.iter().sum::<f32>() / self.processing_times.len() as f32;

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
