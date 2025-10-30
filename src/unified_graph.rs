#![allow(unused_variables)]
//! Unified Signal Graph - The heart of Phonon
//!
//! Everything is a signal. Patterns, audio, control data - all flow through
//! one unified graph where anything can modulate anything.
//!
//! # Overview
//!
//! The `UnifiedSignalGraph` is Phonon's central audio processing engine. It provides:
//! - **Pattern-based sample playback** using Tidal Cycles mini-notation
//! - **Audio synthesis** with oscillators, filters, and envelopes
//! - **Cross-modulation** between patterns, audio, and control signals
//! - **Multi-output routing** for complex setups
//! - **DSP parameter modulation** where any signal can control any parameter
//!
//! # Core Concepts
//!
//! ## Signals
//!
//! In Phonon, everything is a [`Signal`]:
//! - `Signal::Value(f32)` - A constant value
//! - `Signal::Node(NodeId)` - Output from another node
//! - `Signal::Bus(String)` - Named signal bus
//! - `Signal::Pattern(String)` - Inline pattern string
//! - `Signal::Expression(...)` - Arithmetic combinations
//!
//! ## Nodes
//!
//! Nodes are the building blocks of your graph. Each [`SignalNode`] type has a specific purpose:
//! - **Sources**: `Oscillator`, `Pattern`, `Sample`, `Noise`
//! - **Processors**: `LowPass`, `HighPass`, `Envelope`, `Delay`
//! - **Analysis**: `RMS`, `Pitch`, `Transient`
//! - **Math**: `Add`, `Multiply`, `When`
//!
//! # Basic Example: Simple Sample Playback
//!
//! ```rust
//! use phonon::unified_graph::{UnifiedSignalGraph, SignalNode, Signal};
//! use phonon::mini_notation_v3::parse_mini_notation;
//! use std::collections::HashMap;
//!
//! let mut graph = UnifiedSignalGraph::new(44100.0);
//! graph.set_cps(2.0); // 2 cycles per second = 120 BPM
//!
//! // Create a kick drum pattern that triggers on beats 1 and 3
//! let pattern = parse_mini_notation("bd ~ bd ~");
//! let sample_node = graph.add_node(SignalNode::Sample {
//!     pattern_str: "bd ~ bd ~".to_string(),
//!     pattern,
//!     last_trigger_time: -1.0,
//!     last_cycle: -1,
//!     playback_positions: HashMap::new(),
//!     gain: Signal::Value(1.0),
//!     pan: Signal::Value(0.0),
//!     speed: Signal::Value(1.0),
//!     cut_group: Signal::Value(0.0),
//!     n: Signal::Value(0.0),
//!     note: Signal::Value(0.0),
//!     attack: Signal::Value(0.0),
//!     release: Signal::Value(0.0),
//! });
//!
//! graph.set_output(sample_node);
//!
//! // Render 1 second of audio
//! let buffer = graph.render(44100);
//! ```
//!
//! # Pattern-Based DSP Parameters
//!
//! One of Phonon's most powerful features is the ability to modulate DSP parameters
//! with patterns. This allows you to create complex, evolving sounds where
//! parameters change over time according to rhythmic patterns.
//!
//! ## Example: Panning Pattern
//!
//! ```rust
//! use phonon::unified_graph::{UnifiedSignalGraph, SignalNode, Signal};
//! use phonon::mini_notation_v3::parse_mini_notation;
//! use std::collections::HashMap;
//!
//! let mut graph = UnifiedSignalGraph::new(44100.0);
//! graph.set_cps(2.0);
//!
//! // Create a hi-hat pattern with alternating left/right panning
//! let pattern = parse_mini_notation("hh*8");
//! let pan_pattern = parse_mini_notation("-1 1"); // -1 = left, 1 = right
//!
//! // Create the pan pattern node
//! let pan_node = graph.add_node(SignalNode::Pattern {
//!     pattern_str: "-1 1".to_string(),
//!     pattern: pan_pattern,
//!     last_value: 0.0,
//!     last_trigger_time: -1.0,
//! });
//!
//! // Create sample node with pattern-based pan
//! let sample_node = graph.add_node(SignalNode::Sample {
//!     pattern_str: "hh*8".to_string(),
//!     pattern,
//!     last_trigger_time: -1.0,
//!     last_cycle: -1,
//!     playback_positions: HashMap::new(),
//!     gain: Signal::Value(1.0),
//!     pan: Signal::Node(pan_node), // Pan controlled by pattern!
//!     speed: Signal::Value(1.0),
//!     cut_group: Signal::Value(0.0),
//!     n: Signal::Value(0.0),
//!     note: Signal::Value(0.0),
//!     attack: Signal::Value(0.0),
//!     release: Signal::Value(0.0),
//! });
//!
//! graph.set_output(sample_node);
//! ```
//!
//! ## Example: Speed Modulation
//!
//! ```rust
//! use phonon::unified_graph::{UnifiedSignalGraph, SignalNode, Signal};
//! use phonon::mini_notation_v3::parse_mini_notation;
//! use std::collections::HashMap;
//!
//! let mut graph = UnifiedSignalGraph::new(44100.0);
//! graph.set_cps(1.0);
//!
//! // Create a sample pattern
//! let pattern = parse_mini_notation("bd*4");
//!
//! // Create a speed pattern that makes each hit play at different speeds
//! let speed_pattern = parse_mini_notation("1 2 0.5 1.5");
//! let speed_node = graph.add_node(SignalNode::Pattern {
//!     pattern_str: "1 2 0.5 1.5".to_string(),
//!     pattern: speed_pattern,
//!     last_value: 1.0,
//!     last_trigger_time: -1.0,
//! });
//!
//! let sample_node = graph.add_node(SignalNode::Sample {
//!     pattern_str: "bd*4".to_string(),
//!     pattern,
//!     last_trigger_time: -1.0,
//!     last_cycle: -1,
//!     playback_positions: HashMap::new(),
//!     gain: Signal::Value(1.0),
//!     pan: Signal::Value(0.0),
//!     speed: Signal::Node(speed_node), // Speed controlled by pattern!
//!     cut_group: Signal::Value(0.0),
//!     n: Signal::Value(0.0),
//!     note: Signal::Value(0.0),
//!     attack: Signal::Value(0.0),
//!     release: Signal::Value(0.0),
//! });
//!
//! graph.set_output(sample_node);
//! ```
//!
//! ## Example: Gain Envelope
//!
//! ```rust
//! use phonon::unified_graph::{UnifiedSignalGraph, SignalNode, Signal, SignalExpr};
//! use phonon::mini_notation_v3::parse_mini_notation;
//! use std::collections::HashMap;
//!
//! let mut graph = UnifiedSignalGraph::new(44100.0);
//! graph.set_cps(2.0);
//!
//! // Create LFO for gain modulation (0.5 Hz sine wave)
//! let lfo = graph.add_node(SignalNode::Oscillator {
//!     freq: Signal::Value(0.5),
//!     waveform: phonon::unified_graph::Waveform::Sine,
//!     phase: 0.0,
//! });
//!
//! // Scale LFO from -1..1 to 0.2..1.0 (quiet to loud)
//! let scaled_gain = Signal::Expression(Box::new(SignalExpr::Scale {
//!     input: Signal::Node(lfo),
//!     min: 0.2,
//!     max: 1.0,
//! }));
//!
//! let pattern = parse_mini_notation("hh*16");
//! let sample_node = graph.add_node(SignalNode::Sample {
//!     pattern_str: "hh*16".to_string(),
//!     pattern,
//!     last_trigger_time: -1.0,
//!     last_cycle: -1,
//!     playback_positions: HashMap::new(),
//!     gain: scaled_gain, // Gain controlled by LFO!
//!     pan: Signal::Value(0.0),
//!     speed: Signal::Value(1.0),
//!     cut_group: Signal::Value(0.0),
//!     n: Signal::Value(0.0),
//!     note: Signal::Value(0.0),
//!     attack: Signal::Value(0.0),
//!     release: Signal::Value(0.0),
//! });
//!
//! graph.set_output(sample_node);
//! ```
//!
//! # Cross-Modulation and Effects
//!
//! Phonon allows any signal to modulate any other signal, enabling complex
//! effects routing and modulation schemes.
//!
//! ## Example: Filter Controlled by Pattern
//!
//! ```rust
//! use phonon::unified_graph::{UnifiedSignalGraph, SignalNode, Signal, Waveform, FilterState};
//! use phonon::mini_notation_v3::parse_mini_notation;
//! use std::collections::HashMap;
//!
//! let mut graph = UnifiedSignalGraph::new(44100.0);
//! graph.set_cps(2.0);
//!
//! // Bass pattern
//! let pattern = parse_mini_notation("bd*4");
//! let sample_node = graph.add_node(SignalNode::Sample {
//!     pattern_str: "bd*4".to_string(),
//!     pattern,
//!     last_trigger_time: -1.0,
//!     last_cycle: -1,
//!     playback_positions: HashMap::new(),
//!     gain: Signal::Value(1.0),
//!     pan: Signal::Value(0.0),
//!     speed: Signal::Value(1.0),
//!     cut_group: Signal::Value(0.0),
//!     n: Signal::Value(0.0),
//!     note: Signal::Value(0.0),
//!     attack: Signal::Value(0.0),
//!     release: Signal::Value(0.0),
//! });
//!
//! // Cutoff frequency pattern (200 Hz to 2000 Hz)
//! let cutoff_pattern = parse_mini_notation("200 500 1000 2000");
//! let cutoff_node = graph.add_node(SignalNode::Pattern {
//!     pattern_str: "200 500 1000 2000".to_string(),
//!     pattern: cutoff_pattern,
//!     last_value: 500.0,
//!     last_trigger_time: -1.0,
//! });
//!
//! // Lowpass filter with pattern-controlled cutoff
//! let filtered = graph.add_node(SignalNode::LowPass {
//!     input: Signal::Node(sample_node),
//!     cutoff: Signal::Node(cutoff_node), // Cutoff controlled by pattern!
//!     q: Signal::Value(2.0),
//!     state: FilterState::default(),
//! });
//!
//! graph.set_output(filtered);
//! ```
//!
//! ## Example: Audio-Rate Modulation
//!
//! ```rust
//! use phonon::unified_graph::{UnifiedSignalGraph, SignalNode, Signal, Waveform, SignalExpr};
//! use phonon::mini_notation_v3::parse_mini_notation;
//! use std::collections::HashMap;
//!
//! let mut graph = UnifiedSignalGraph::new(44100.0);
//! graph.set_cps(2.0);
//!
//! // Modulator: 5 Hz sine wave
//! let modulator = graph.add_node(SignalNode::Oscillator {
//!     freq: Signal::Value(5.0),
//!     waveform: Waveform::Sine,
//!     phase: 0.0,
//! });
//!
//! // Carrier frequency: 220 Hz + modulation
//! let modulated_freq = Signal::Expression(Box::new(SignalExpr::Add(
//!     Signal::Value(220.0),
//!     Signal::Expression(Box::new(SignalExpr::Multiply(
//!         Signal::Node(modulator),
//!         Signal::Value(50.0), // Modulation depth
//!     ))),
//! )));
//!
//! // Carrier oscillator with FM
//! let carrier = graph.add_node(SignalNode::Oscillator {
//!     freq: modulated_freq,
//!     waveform: Waveform::Sine,
//!     phase: 0.0,
//! });
//!
//! graph.set_output(carrier);
//! ```
//!
//! # Multi-Output Routing
//!
//! Phonon supports multiple independent output channels for complex setups.
//!
//! ```rust
//! use phonon::unified_graph::{UnifiedSignalGraph, SignalNode, Signal};
//! use phonon::mini_notation_v3::parse_mini_notation;
//! use std::collections::HashMap;
//!
//! let mut graph = UnifiedSignalGraph::new(44100.0);
//! graph.set_cps(2.0);
//!
//! // Kick pattern on channel 1
//! let kick_pattern = parse_mini_notation("bd ~ bd ~");
//! let kick_node = graph.add_node(SignalNode::Sample {
//!     pattern_str: "bd ~ bd ~".to_string(),
//!     pattern: kick_pattern,
//!     last_trigger_time: -1.0,
//!     last_cycle: -1,
//!     playback_positions: HashMap::new(),
//!     gain: Signal::Value(1.0),
//!     pan: Signal::Value(0.0),
//!     speed: Signal::Value(1.0),
//!     cut_group: Signal::Value(0.0),
//!     n: Signal::Value(0.0),
//!     note: Signal::Value(0.0),
//!     attack: Signal::Value(0.0),
//!     release: Signal::Value(0.0),
//! });
//!
//! // Snare pattern on channel 2
//! let snare_pattern = parse_mini_notation("~ sn ~ sn");
//! let snare_node = graph.add_node(SignalNode::Sample {
//!     pattern_str: "~ sn ~ sn".to_string(),
//!     pattern: snare_pattern,
//!     last_trigger_time: -1.0,
//!     last_cycle: -1,
//!     playback_positions: HashMap::new(),
//!     gain: Signal::Value(1.0),
//!     pan: Signal::Value(0.0),
//!     speed: Signal::Value(1.0),
//!     cut_group: Signal::Value(0.0),
//!     n: Signal::Value(0.0),
//!     note: Signal::Value(0.0),
//!     attack: Signal::Value(0.0),
//!     release: Signal::Value(0.0),
//! });
//!
//! graph.set_output_channel(1, kick_node);  // Channel 1
//! graph.set_output_channel(2, snare_node); // Channel 2
//!
//! // Process multi-channel audio
//! let outputs = graph.process_sample_multi(); // Returns Vec<f32>
//! // outputs[0] = channel 1, outputs[1] = channel 2
//! ```
//!
//! # Mini-Notation Pattern Language
//!
//! Phonon uses Tidal Cycles mini-notation for pattern specification:
//!
//! - **Concatenation**: `"bd sn hh"` - play in sequence
//! - **Subdivision**: `"bd*4"` - repeat bd 4 times per cycle
//! - **Slow down**: `"bd/2"` - stretch bd over 2 cycles
//! - **Rests**: `"bd ~ sn ~"` - silence on ~ positions
//! - **Alternation**: `"<bd sn>"` - alternate between bd and sn each cycle
//! - **Layering**: `"[bd, sn]"` - play bd and sn simultaneously
//! - **Euclidean**: `"bd(3,8)"` - 3 hits distributed over 8 steps
//! - **Sample selection**: `"bd:0 bd:1 bd:2"` - choose specific samples from folder
//!
//! ## Pattern Examples
//!
//! ```rust
//! use phonon::mini_notation_v3::parse_mini_notation;
//!
//! // Basic beat: kick on 1 and 3, snare on 2 and 4
//! let pattern = parse_mini_notation("bd sn bd sn");
//!
//! // Fast hi-hats (16th notes)
//! let pattern = parse_mini_notation("hh*16");
//!
//! // Polyrhythm: 3 kicks against 4 snares
//! let pattern = parse_mini_notation("[bd*3, sn*4]");
//!
//! // Euclidean rhythm: 3 hits in 8 steps (tresillo pattern)
//! let pattern = parse_mini_notation("bd(3,8)");
//!
//! // Alternating samples each cycle
//! let pattern = parse_mini_notation("<bd:0 bd:1 bd:2>");
//! ```
//!
//! # Performance Tips
//!
//! 1. **Reuse patterns**: Parse patterns once and reuse the `Pattern` object
//! 2. **Cache nodes**: Store `NodeId` values to avoid repeated lookups
//! 3. **Minimize graph depth**: Flatten deeply nested signal chains when possible
//! 4. **Use constants**: `Signal::Value()` is faster than pattern evaluation
//!
//! # See Also
//!
//! - [`VoiceManager`] - Polyphonic voice allocation (64 voices)
//! - [`SampleBank`] - Sample loading from dirt-samples
//! - [`mini_notation_v3`] - Pattern parsing and querying

use crate::mini_notation_v3::parse_mini_notation;
use crate::pattern::{Fraction, Pattern, State, TimeSpan};
use crate::sample_loader::SampleBank;
use crate::synth_voice_manager::SynthVoiceManager;
use crate::voice_manager::VoiceManager;
use std::cell::RefCell;
use std::collections::HashMap;
use std::f32::consts::PI;
use std::sync::{Arc, Mutex};

/// Unique identifier for nodes in the graph
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct NodeId(pub usize);

/// A signal reference - can be a node, bus, constant, or expression
#[derive(Debug, Clone)]
pub enum Signal {
    /// Reference to another node
    Node(NodeId),
    /// Reference to a named bus
    Bus(String),
    /// Inline pattern string
    Pattern(String),
    /// Constant value
    Value(f32),
    /// Arithmetic expression
    Expression(Box<SignalExpr>),
}

/// Signal expressions for complex modulation
#[derive(Debug, Clone)]
pub enum SignalExpr {
    Add(Signal, Signal),
    Multiply(Signal, Signal),
    Subtract(Signal, Signal),
    Divide(Signal, Signal),
    Modulo(Signal, Signal),
    Scale { input: Signal, min: f32, max: f32 },
}

/// Runtime envelope type for Sample nodes (after compilation)
#[derive(Debug, Clone)]
pub enum RuntimeEnvelopeType {
    Percussion,  // Use attack + release
    ADSR {
        decay: Signal,
        sustain: Signal,
    },
    Segments {
        levels: Vec<f32>,
        times: Vec<f32>,
    },
    Curve {
        start: Signal,
        end: Signal,
        duration: Signal,
        curve: Signal,
    },
}

/// Types of nodes in the unified graph
#[derive(Debug, Clone)]
pub enum SignalNode {
    // === Sources ===
    /// Oscillator with modulatable frequency
    Oscillator {
        freq: Signal,
        waveform: Waveform,
        phase: f32,
        pending_freq: Option<f32>, // Frequency change waiting for zero-crossing
        last_sample: f32,          // For zero-crossing detection
    },

    /// FM (Frequency Modulation) oscillator
    /// output = sin(2π * carrier * t + mod_index * sin(2π * modulator * t))
    FMOscillator {
        carrier_freq: Signal,    // Carrier frequency in Hz
        modulator_freq: Signal,  // Modulator frequency in Hz
        mod_index: Signal,       // Modulation index (depth)
        carrier_phase: f32,      // Carrier phase (0.0 to 1.0)
        modulator_phase: f32,    // Modulator phase (0.0 to 1.0)
    },

    /// White noise generator
    /// Generates uniformly distributed random samples in range [-1, 1]
    WhiteNoise,

    /// Pink noise generator (1/f spectrum)
    /// Generates noise with equal energy per octave
    /// Uses Voss-McCartney algorithm with octave bins
    PinkNoise { state: PinkNoiseState },

    /// Brown noise generator (6dB/octave rolloff)
    /// Generates very "warm" noise using random walk
    /// Also called Brownian noise or red noise
    BrownNoise { state: BrownNoiseState },

    /// Impulse generator (single-sample spikes)
    /// Generates periodic impulses (1.0 for single sample, 0.0 otherwise)
    /// Useful for triggering envelopes, creating rhythmic gates
    Impulse {
        frequency: Signal, // Impulse frequency in Hz
        state: ImpulseState,
    },

    /// Lag (exponential slew limiter)
    /// Smooths abrupt changes with exponential approach to target
    /// Useful for portamento, click removal, parameter smoothing
    Lag {
        input: Signal,     // Input signal to smooth
        lag_time: Signal,  // Time constant in seconds
        state: LagState,
    },

    /// XLine (exponential envelope)
    /// Generates exponential ramp from start to end over duration
    /// More natural sounding than linear ramps for pitch/amplitude
    XLine {
        start: Signal,     // Starting value
        end: Signal,       // Ending value
        duration: Signal,  // Duration in seconds
        state: XLineState,
    },

    /// ASR (Attack-Sustain-Release) envelope
    /// Gate-based envelope: attacks when gate rises, sustains while high, releases when gate falls
    /// Perfect for organ-style sounds and continuous notes
    ASR {
        gate: Signal,      // Gate signal (0 = off, >0.5 = on)
        attack: Signal,    // Attack time in seconds
        release: Signal,   // Release time in seconds
        state: ASRState,
    },

    /// Pulse wave oscillator (variable pulse width)
    /// Output: +1 when phase < width, -1 otherwise
    /// width=0.5 creates square wave (only odd harmonics)
    /// Other widths create different harmonic content
    Pulse {
        freq: Signal,      // Frequency in Hz
        width: Signal,     // Pulse width / duty cycle (0.0 to 1.0)
        phase: f32,        // Phase (0.0 to 1.0)
    },

    /// Brick-wall limiter (prevents signal from exceeding threshold)
    /// Clamps signal to [-threshold, +threshold]
    Limiter {
        input: Signal,     // Input signal
        threshold: Signal, // Maximum allowed amplitude
    },

    /// Pan2 Left channel (equal-power panning law)
    /// Takes mono input and pan position (-1=left, 0=center, 1=right)
    /// Outputs left channel component
    Pan2Left {
        input: Signal,    // Mono input signal
        position: Signal, // Pan position (-1 to 1)
    },

    /// Pan2 Right channel (equal-power panning law)
    /// Takes mono input and pan position (-1=left, 0=center, 1=right)
    /// Outputs right channel component
    Pan2Right {
        input: Signal,    // Mono input signal
        position: Signal, // Pan position (-1 to 1)
    },

    /// Pattern as a signal source
    Pattern {
        pattern_str: String,
        pattern: Pattern<String>,
        last_value: f32,
        last_trigger_time: f32, // Cycle position of last trigger
    },

    /// Cycle trigger: generates a short pulse at the start of each cycle
    /// Useful for triggering envelopes rhythmically
    CycleTrigger {
        last_cycle: i32,  // Track which cycle triggered last
        pulse_width: f32, // Duration of the pulse in seconds
    },

    /// Sample player triggered by pattern
    Sample {
        pattern_str: String,
        pattern: Pattern<String>,
        last_trigger_time: f32,
        last_cycle: i32, // Track which cycle we processed last
        playback_positions: HashMap<String, usize>,
        gain: Signal,
        pan: Signal,
        speed: Signal,
        cut_group: Signal, // Cut group for voice stealing (0 = no cut group)
        n: Signal,         // Sample number selection (0 = first sample in bank)
        note: Signal,      // Note/pitch shift in semitones (0 = original, 12 = octave up)
        attack: Signal,    // Attack time in seconds (0.0 = no attack envelope)
        release: Signal,   // Release time in seconds (0.0 = no release envelope)
        envelope_type: Option<RuntimeEnvelopeType>, // Envelope type (None = percussion)
    },

    /// Pattern-triggered synthesizer with ADSR envelopes
    /// Each note in the pattern triggers a new synth voice
    SynthPattern {
        pattern_str: String,
        pattern: Pattern<String>,
        last_trigger_time: f32,
        waveform: Waveform,
        attack: f32,
        decay: f32,
        sustain: f32,
        release: f32,
        gain: Signal,
        pan: Signal,
    },

    /// Pattern-triggered envelope gate
    /// Gates an input signal with rhythm from a pattern
    EnvelopePattern {
        input: Signal,
        pattern_str: String,
        pattern: Pattern<String>,
        last_trigger_time: f32,
        last_cycle: i32,
        attack: f32,
        decay: f32,
        sustain: f32,
        release: f32,
        state: EnvState,
    },

    /// Structured signal: boolean pattern imposes rhythmic structure on signal
    /// Each "true" event in the pattern triggers an envelope on the input signal
    /// This is what makes `struct "t(3,8)" (sine 440)` work
    StructuredSignal {
        input: Signal,
        bool_pattern_str: String,
        bool_pattern: Pattern<bool>,
        last_trigger_time: f32,
        last_cycle: i32,
        attack: f32,
        decay: f32,
        sustain: f32,
        release: f32,
        state: EnvState,
    },

    /// Voice output - outputs mixed audio from all triggered samples
    /// This allows sample playback to be routed through effects
    VoiceOutput,

    /// Scale quantization - maps scale degrees to frequencies
    /// Pattern contains scale degrees (0, 1, 2, 3...), quantized to musical scale
    ScaleQuantize {
        pattern_str: String,
        pattern: Pattern<String>,
        scale_name: String,
        root_note: u8, // MIDI note number
        last_value: f32,
    },

    /// Constant value
    Constant { value: f32 },

    /// Noise generator
    Noise { seed: u32 },

    // === Processors ===
    /// Lowpass filter
    LowPass {
        input: Signal,
        cutoff: Signal,
        q: Signal,
        state: FilterState,
    },

    /// Highpass filter
    HighPass {
        input: Signal,
        cutoff: Signal,
        q: Signal,
        state: FilterState,
    },

    /// Bandpass filter
    BandPass {
        input: Signal,
        center: Signal,
        q: Signal,
        state: FilterState,
    },

    /// Notch filter (band-reject)
    /// Removes frequencies at center frequency while passing all others
    /// Useful for removing unwanted resonances, hum, or feedback
    Notch {
        input: Signal,
        center: Signal,
        q: Signal,
        state: FilterState,
    },

    /// Comb filter (feedback delay line)
    /// Creates resonant peaks by feeding delayed signal back
    /// Useful for physical modeling, bells, metallic sounds, and adding character
    Comb {
        input: Signal,
        frequency: Signal, // Resonant frequency in Hz (converted to delay time)
        feedback: Signal,  // Feedback amount (0.0-0.99, higher = more resonance)
        buffer: Vec<f32>,  // Circular buffer for delay line
        write_pos: usize,  // Current write position in buffer
    },

    /// Moog Ladder Filter (4-pole 24dB/octave lowpass with resonance)
    /// Classic analog filter with warm sound and self-oscillation
    MoogLadder {
        input: Signal,
        cutoff: Signal,   // Cutoff frequency in Hz
        resonance: Signal, // Resonance (0.0-1.0, self-oscillates near 1.0)
        state: MoogLadderState,
    },

    /// Parametric EQ (3-band peaking equalizer)
    /// Each band can boost or cut frequencies independently
    ParametricEQ {
        input: Signal,
        // Low band
        low_freq: Signal,   // Center frequency in Hz
        low_gain: Signal,   // Gain in dB (-20 to +20)
        low_q: Signal,      // Bandwidth (0.1 to 10.0)
        // Mid band
        mid_freq: Signal,
        mid_gain: Signal,
        mid_q: Signal,
        // High band
        high_freq: Signal,
        high_gain: Signal,
        high_q: Signal,
        state: ParametricEQState,
    },

    /// Envelope generator (triggered)
    Envelope {
        input: Signal,
        trigger: Signal,
        attack: f32,
        decay: f32,
        sustain: f32,
        release: f32,
        state: EnvState,
    },

    /// ADSR envelope generator (continuous, one per cycle)
    /// Generates envelope over one cycle: Attack -> Decay -> Sustain -> Release
    ADSR {
        attack: Signal,  // Attack time in seconds
        decay: Signal,   // Decay time in seconds
        sustain: Signal, // Sustain level (0.0-1.0)
        release: Signal, // Release time in seconds
        state: ADSRState,
    },

    /// AD envelope generator (continuous, one per cycle)
    /// Generates envelope over one cycle: Attack -> Decay (no sustain/release)
    AD {
        attack: Signal, // Attack time in seconds
        decay: Signal,  // Decay time in seconds
        state: ADState,
    },

    /// Line envelope generator (continuous, one per cycle)
    /// Linear ramp from start to end value over one cycle
    Line {
        start: Signal, // Start value
        end: Signal,   // End value
    },

    /// Curve envelope generator (continuous)
    /// Curved ramp from start to end over duration
    /// Curve parameter controls shape: 0=linear, +ve=exponential, -ve=logarithmic
    Curve {
        start: Signal,     // Start value
        end: Signal,       // End value
        duration: Signal,  // Duration in seconds
        curve: Signal,     // Curve shape (-10 to +10, 0=linear)
        elapsed_time: f32, // Time since start
    },

    /// Segments envelope (arbitrary breakpoint)
    /// Multi-segment envelope with linear interpolation
    /// Takes two pattern strings: levels and times
    Segments {
        levels: Vec<f32>,        // Target levels for each breakpoint
        times: Vec<f32>,         // Duration for each segment
        current_segment: usize,  // Which segment we're in
        segment_elapsed: f32,    // Time elapsed in current segment
        current_value: f32,      // Current interpolated value
    },

    /// Delay line
    Delay {
        input: Signal,
        time: Signal,
        feedback: Signal,
        mix: Signal,
        buffer: Vec<f32>,
        write_idx: usize,
    },

    // === Analysis ===
    /// RMS analyzer
    /// Window size in seconds (supports pattern modulation!)
    RMS {
        input: Signal,
        window_size: Signal,
        buffer: Vec<f32>,
        write_idx: usize,
    },

    /// Schmidt trigger (gate with hysteresis)
    /// Converts analog signal to digital gate with noise immunity
    /// high_threshold: level where gate turns ON
    /// low_threshold: level where gate turns OFF
    Schmidt {
        input: Signal,
        high_threshold: Signal,
        low_threshold: Signal,
        state: bool, // Current gate state (true = high, false = low)
    },

    /// Latch (Sample & Hold)
    /// Samples input when gate transitions from low to high and holds until next trigger
    Latch {
        input: Signal,
        gate: Signal,
        held_value: f32,     // The currently held sample
        last_gate: f32,      // Previous gate value (for edge detection)
    },

    /// Timer
    /// Measures elapsed time since last trigger reset
    /// Resets to 0 on rising edge, counts up in seconds
    Timer {
        trigger: Signal,
        elapsed_time: f32,   // Current elapsed time in seconds
        last_trigger: f32,   // Previous trigger value (for edge detection)
    },

    /// Pitch detector
    Pitch { input: Signal, last_pitch: f32 },

    /// Transient detector
    Transient {
        input: Signal,
        threshold: f32,
        last_value: f32,
    },

    /// Peak Follower
    /// Tracks the peak amplitude of an input signal
    /// Fast attack, slow decay
    PeakFollower {
        input: Signal,
        attack_time: Signal,   // Attack time in seconds
        release_time: Signal,  // Release/decay time in seconds
        current_peak: f32,     // Current peak level
    },

    /// Amp Follower
    /// RMS-based envelope follower with attack/release smoothing
    /// Smoother than peak follower for amplitude tracking
    AmpFollower {
        input: Signal,
        attack_time: Signal,    // Attack time in seconds
        release_time: Signal,   // Release time in seconds
        window_size: Signal,    // RMS window size in seconds
        buffer: Vec<f32>,       // Circular buffer for RMS
        write_idx: usize,       // Write position in buffer
        current_envelope: f32,  // Smoothed RMS value
    },

    // === Math & Control ===
    /// Addition
    Add { a: Signal, b: Signal },

    /// Multiplication
    Multiply { a: Signal, b: Signal },

    /// Crossfader between two signals
    /// position = 0.0 → 100% signal_a
    /// position = 0.5 → 50% signal_a + 50% signal_b
    /// position = 1.0 → 100% signal_b
    XFade {
        signal_a: Signal,
        signal_b: Signal,
        position: Signal, // 0.0 to 1.0
    },

    /// Mix (sum) multiple signals
    /// Sums all input signals together
    Mix { signals: Vec<Signal> },

    /// Allpass filter (phase manipulation, reverb building block)
    /// Passes all frequencies but changes phase relationships
    Allpass {
        input: Signal,
        coefficient: Signal, // Feedback coefficient (-1.0 to 1.0)
        state: AllpassState,
    },

    /// Conditional gate
    When { input: Signal, condition: Signal },

    /// Signal router to multiple destinations
    Router {
        input: Signal,
        destinations: Vec<(NodeId, f32)>, // (target, amount)
    },

    // === Effects ===
    /// Reverb (Freeverb-style)
    Reverb {
        input: Signal,
        room_size: Signal, // 0.0-1.0
        damping: Signal,   // 0.0-1.0
        mix: Signal,       // 0.0-1.0 (dry/wet)
        state: ReverbState,
    },

    /// Distortion / Waveshaper
    Distortion {
        input: Signal,
        drive: Signal, // 1.0-100.0
        mix: Signal,   // 0.0-1.0
    },

    /// Bitcrusher
    BitCrush {
        input: Signal,
        bits: Signal,        // 1.0-16.0
        sample_rate: Signal, // Sample rate reduction factor
        state: BitCrushState,
    },

    /// Chorus effect
    Chorus {
        input: Signal,
        rate: Signal,  // LFO rate in Hz
        depth: Signal, // Delay modulation depth
        mix: Signal,   // 0.0-1.0
        state: ChorusState,
    },

    /// Flanger effect (sweeping comb filter via delay modulation)
    Flanger {
        input: Signal,
        depth: Signal,    // Modulation depth (0.0-1.0)
        rate: Signal,     // LFO rate in Hz
        feedback: Signal, // Feedback amount (0.0-0.95)
        state: FlangerState,
    },

    /// Compressor (dynamic range compression)
    Compressor {
        input: Signal,
        threshold: Signal,   // Threshold in dB (-60.0 to 0.0)
        ratio: Signal,       // Compression ratio (1.0 to 20.0)
        attack: Signal,      // Attack time in seconds (0.001 to 1.0)
        release: Signal,     // Release time in seconds (0.01 to 3.0)
        makeup_gain: Signal, // Makeup gain in dB (0.0 to 30.0)
        state: CompressorState,
    },

    /// Tremolo (amplitude modulation)
    /// Classic effect that modulates amplitude with an LFO
    Tremolo {
        input: Signal,  // Input signal
        rate: Signal,   // LFO rate in Hz (0.1 to 20.0)
        depth: Signal,  // Modulation depth (0.0 to 1.0)
        phase: f32,     // LFO phase accumulator
    },

    /// fundsp Unit Wrapper (wraps fundsp AudioUnit for pattern modulation)
    /// Allows using fundsp's 60+ battle-tested UGens with Phonon's pattern system
    /// Pattern signals can modulate fundsp parameters at audio rate
    FundspUnit {
        unit_type: FundspUnitType,      // Which fundsp unit this is
        input: Signal,                   // Audio input from Phonon
        params: Vec<Signal>,             // Pattern-modulatable parameters!
        state: Arc<Mutex<FundspState>>,  // Thread-safe shared mutable fundsp unit state
    },

    /// Output node
    Output { input: Signal },
}

/// Oscillator waveforms
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Waveform {
    Sine,
    Saw,
    Square,
    Triangle,
}

/// fundsp Unit Types
/// Identifies which fundsp AudioUnit is wrapped
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FundspUnitType {
    /// Organ-like oscillator (additive synthesis with multiple harmonics)
    OrganHz,
    /// Moog ladder filter (4-pole 24dB/octave lowpass)
    MoogHz,
    /// Stereo reverb (1 mono input, 2 stereo outputs - currently outputs left only)
    ReverbStereo,
    /// Chorus effect (5-voice mono chorus with LFO modulation)
    Chorus,
    /// Bandlimited sawtooth oscillator
    SawHz,
    /// Bandlimited square wave oscillator
    SquareHz,
    /// Bandlimited triangle wave oscillator
    TriangleHz,
    /// White noise generator
    Noise,
    /// Phaser effect (frequency-domain comb filtering)
    Phaser,
    /// Nonlinear lowpass filter (Jatin Chowdhury's design)
    DLowpassHz,
}

/// fundsp State Wrapper
/// Uses a tick function pointer to avoid complex generic types
/// This allows us to store fundsp units without exposing their concrete types
pub struct FundspState {
    /// Function that processes one sample
    tick_fn: Box<dyn FnMut(f32) -> f32 + Send>,
    /// Type of the unit (for debugging and parameter updates)
    unit_type: FundspUnitType,
    /// Current parameters (for recreation if needed)
    params: Vec<f32>,
    sample_rate: f64,
}

impl FundspState {
    /// Create a new organ_hz unit
    pub fn new_organ_hz(frequency: f32, sample_rate: f64) -> Self {
        use fundsp::prelude::AudioUnit;

        let mut unit = fundsp::prelude::organ_hz(frequency);
        unit.reset();
        unit.set_sample_rate(sample_rate);

        // Create a closure that owns the unit and calls tick
        let tick_fn = Box::new(move |_input: f32| -> f32 {
            let output_frame = unit.tick(&Default::default());
            output_frame[0]
        });

        Self {
            tick_fn,
            unit_type: FundspUnitType::OrganHz,
            params: vec![frequency],
            sample_rate,
        }
    }

    /// Create a new moog_hz unit (Moog ladder filter)
    pub fn new_moog_hz(cutoff: f32, resonance: f32, sample_rate: f64) -> Self {
        use fundsp::prelude::AudioUnit;

        let mut unit = fundsp::prelude::moog_hz(cutoff, resonance);
        unit.reset();
        unit.set_sample_rate(sample_rate);

        // Create a closure that owns the unit and calls tick
        let tick_fn = Box::new(move |input: f32| -> f32 {
            // moog_hz takes 1 input, returns 1 output
            // Convert f32 array to fundsp Frame type
            let output_frame = unit.tick(&[input].into());
            output_frame[0]
        });

        Self {
            tick_fn,
            unit_type: FundspUnitType::MoogHz,
            params: vec![cutoff, resonance],
            sample_rate,
        }
    }

    /// Create a new reverb_stereo unit (Stereo reverb - stereo in, stereo out)
    pub fn new_reverb_stereo(wet: f32, time: f32, sample_rate: f64) -> Self {
        // reverb_stereo takes (wet, time, diffusion) and expects stereo input
        // Convert parameters to f64 for fundsp
        let diffusion = 0.5;  // Fixed diffusion parameter
        let mut unit = fundsp::prelude::reverb_stereo(wet as f64, time as f64, diffusion);
        unit.reset();
        unit.set_sample_rate(sample_rate);

        // Create a closure that owns the unit and calls tick
        let tick_fn = Box::new(move |input: f32| -> f32 {
            // reverb_stereo: 2 inputs (stereo) -> 2 outputs (stereo)
            // Convert mono to stereo input, return left channel
            let output_frame = unit.tick(&[input, input].into());
            output_frame[0]  // Left channel only
        });

        Self {
            tick_fn,
            unit_type: FundspUnitType::ReverbStereo,
            params: vec![wet, time],
            sample_rate,
        }
    }

    /// Create a new chorus unit (5-voice mono chorus)
    pub fn new_chorus(
        seed: u64,
        separation: f32,
        variation: f32,
        mod_frequency: f32,
        sample_rate: f64,
    ) -> Self {
        use fundsp::prelude::AudioUnit;

        let mut unit = fundsp::prelude::chorus(seed, separation, variation, mod_frequency);
        unit.reset();
        unit.set_sample_rate(sample_rate);

        // Create a closure that owns the unit and calls tick
        let tick_fn = Box::new(move |input: f32| -> f32 {
            // chorus: 1 input -> 1 output
            let output_frame = unit.tick(&[input].into());
            output_frame[0]
        });

        Self {
            tick_fn,
            unit_type: FundspUnitType::Chorus,
            params: vec![seed as f32, separation, variation, mod_frequency],
            sample_rate,
        }
    }

    /// Create a new saw_hz unit (bandlimited sawtooth oscillator)
    pub fn new_saw_hz(frequency: f32, sample_rate: f64) -> Self {
        use fundsp::prelude::AudioUnit;

        let mut unit = fundsp::prelude::saw_hz(frequency);
        unit.reset();
        unit.set_sample_rate(sample_rate);

        // Create a closure that owns the unit and calls tick
        let tick_fn = Box::new(move |_input: f32| -> f32 {
            // saw_hz: 0 inputs -> 1 output (generator)
            let output_frame = unit.tick(&Default::default());
            output_frame[0]
        });

        Self {
            tick_fn,
            unit_type: FundspUnitType::SawHz,
            params: vec![frequency],
            sample_rate,
        }
    }

    /// Create a new square_hz unit (bandlimited square wave oscillator)
    pub fn new_square_hz(frequency: f32, sample_rate: f64) -> Self {
        use fundsp::prelude::AudioUnit;

        let mut unit = fundsp::prelude::square_hz(frequency);
        unit.reset();
        unit.set_sample_rate(sample_rate);

        // Create a closure that owns the unit and calls tick
        let tick_fn = Box::new(move |_input: f32| -> f32 {
            // square_hz: 0 inputs -> 1 output (generator)
            let output_frame = unit.tick(&Default::default());
            output_frame[0]
        });

        Self {
            tick_fn,
            unit_type: FundspUnitType::SquareHz,
            params: vec![frequency],
            sample_rate,
        }
    }

    pub fn new_triangle_hz(frequency: f32, sample_rate: f64) -> Self {
        use fundsp::prelude::AudioUnit;

        let mut unit = fundsp::prelude::triangle_hz(frequency);
        unit.reset();
        unit.set_sample_rate(sample_rate);

        // Create a closure that owns the unit and calls tick
        let tick_fn = Box::new(move |_input: f32| -> f32 {
            // triangle_hz: 0 inputs -> 1 output (generator)
            let output_frame = unit.tick(&Default::default());
            output_frame[0]
        });

        Self {
            tick_fn,
            unit_type: FundspUnitType::TriangleHz,
            params: vec![frequency],
            sample_rate,
        }
    }

    pub fn new_noise(sample_rate: f64) -> Self {
        use fundsp::prelude::AudioUnit;

        let mut unit = fundsp::prelude::noise();
        unit.reset();
        unit.set_sample_rate(sample_rate);

        // Create a closure that owns the unit and calls tick
        let tick_fn = Box::new(move |_input: f32| -> f32 {
            // noise: 0 inputs -> 1 output (generator)
            let output_frame = unit.tick(&Default::default());
            output_frame[0]
        });

        Self {
            tick_fn,
            unit_type: FundspUnitType::Noise,
            params: vec![],  // No parameters!
            sample_rate,
        }
    }

    /// Process one sample through the fundsp unit
    pub fn tick(&mut self, input: f32) -> f32 {
        (self.tick_fn)(input)
    }

    /// Update frequency parameter (for organ_hz)
    pub fn update_frequency(&mut self, new_freq: f32, sample_rate: f64) {
        if (self.params[0] - new_freq).abs() > 0.1 {
            // Recreate the unit with new parameters
            *self = Self::new_organ_hz(new_freq, sample_rate);
        }
    }

    /// Update frequency parameter (for saw_hz)
    pub fn update_saw_frequency(&mut self, new_freq: f32, sample_rate: f64) {
        if (self.params[0] - new_freq).abs() > 0.1 {
            // Recreate the unit with new parameters
            *self = Self::new_saw_hz(new_freq, sample_rate);
        }
    }

    /// Update frequency parameter (for square_hz)
    pub fn update_square_frequency(&mut self, new_freq: f32, sample_rate: f64) {
        if (self.params[0] - new_freq).abs() > 0.1 {
            // Recreate the unit with new parameters
            *self = Self::new_square_hz(new_freq, sample_rate);
        }
    }

    pub fn update_triangle_frequency(&mut self, new_freq: f32, sample_rate: f64) {
        if (self.params[0] - new_freq).abs() > 0.1 {
            // Recreate the unit with new parameters
            *self = Self::new_triangle_hz(new_freq, sample_rate);
        }
    }

    /// Update moog filter parameters (for moog_hz)
    pub fn update_moog_params(&mut self, new_cutoff: f32, new_resonance: f32, sample_rate: f64) {
        let cutoff_changed = (self.params[0] - new_cutoff).abs() > 1.0;
        let resonance_changed = (self.params[1] - new_resonance).abs() > 0.01;

        if cutoff_changed || resonance_changed {
            // Recreate the unit with new parameters
            *self = Self::new_moog_hz(new_cutoff, new_resonance, sample_rate);
        }
    }

    /// Update reverb parameters (for reverb_stereo)
    pub fn update_reverb_params(&mut self, new_wet: f32, new_time: f32, sample_rate: f64) {
        let wet_changed = (self.params[0] - new_wet).abs() > 0.01;
        let time_changed = (self.params[1] - new_time).abs() > 0.05;

        if wet_changed || time_changed {
            // Recreate the unit with new parameters
            *self = Self::new_reverb_stereo(new_wet, new_time, sample_rate);
        }
    }

    /// Update chorus parameters (for chorus)
    pub fn update_chorus_params(
        &mut self,
        new_seed: u64,
        new_separation: f32,
        new_variation: f32,
        new_mod_frequency: f32,
        sample_rate: f64,
    ) {
        let seed_changed = (self.params[0] as u64) != new_seed;
        let separation_changed = (self.params[1] - new_separation).abs() > 0.001;
        let variation_changed = (self.params[2] - new_variation).abs() > 0.0001;
        let mod_freq_changed = (self.params[3] - new_mod_frequency).abs() > 0.01;

        if seed_changed || separation_changed || variation_changed || mod_freq_changed {
            // Recreate the unit with new parameters
            *self = Self::new_chorus(
                new_seed,
                new_separation,
                new_variation,
                new_mod_frequency,
                sample_rate,
            );
        }
    }
}

impl Clone for FundspState {
    fn clone(&self) -> Self {
        // Recreate the unit based on its type and parameters
        match self.unit_type {
            FundspUnitType::OrganHz => Self::new_organ_hz(self.params[0], self.sample_rate),
            FundspUnitType::MoogHz => {
                Self::new_moog_hz(self.params[0], self.params[1], self.sample_rate)
            }
            FundspUnitType::ReverbStereo => {
                Self::new_reverb_stereo(self.params[0], self.params[1], self.sample_rate)
            }
            FundspUnitType::Chorus => Self::new_chorus(
                self.params[0] as u64,
                self.params[1],
                self.params[2],
                self.params[3],
                self.sample_rate,
            ),
            FundspUnitType::SawHz => Self::new_saw_hz(self.params[0], self.sample_rate),
            FundspUnitType::SquareHz => Self::new_square_hz(self.params[0], self.sample_rate),
            FundspUnitType::TriangleHz => Self::new_triangle_hz(self.params[0], self.sample_rate),
            FundspUnitType::Noise => Self::new_noise(self.sample_rate),
            _ => panic!("Clone not implemented for this fundsp unit type"),
        }
    }
}

// Manual Debug implementation since closures don't implement Debug
impl std::fmt::Debug for FundspState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FundspState")
            .field("unit_type", &self.unit_type)
            .field("params", &self.params)
            .field("sample_rate", &self.sample_rate)
            .finish()
    }
}

/// Filter state for biquad filters
#[derive(Debug, Clone)]
pub struct FilterState {
    pub x1: f32,
    pub x2: f32,
    pub y1: f32,
    pub y2: f32,
}

impl Default for FilterState {
    fn default() -> Self {
        Self {
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }
}

/// Allpass filter state
#[derive(Debug, Clone)]
pub struct AllpassState {
    pub x1: f32, // Previous input sample
    pub y1: f32, // Previous output sample
}

impl Default for AllpassState {
    fn default() -> Self {
        Self { x1: 0.0, y1: 0.0 }
    }
}

/// Envelope state
#[derive(Debug, Clone)]
pub struct EnvState {
    phase: EnvPhase,
    level: f32,
    time_in_phase: f32,
    release_start_level: f32, // Level when release phase began
}

#[derive(Debug, Clone)]
pub struct ADSRState {
    phase: ADSRPhase,
    level: f32,
    cycle_pos: f32, // Current position in cycle (0.0 to 1.0)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ADSRPhase {
    Attack,
    Decay,
    Sustain,
    Release,
}

impl Default for ADSRState {
    fn default() -> Self {
        ADSRState {
            phase: ADSRPhase::Attack,
            level: 0.0,
            cycle_pos: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ADState {
    phase: ADPhase,
    level: f32,
    cycle_pos: f32, // Current position in cycle (0.0 to 1.0)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ADPhase {
    Attack,
    Decay,
}

impl Default for ADState {
    fn default() -> Self {
        ADState {
            phase: ADPhase::Attack,
            level: 0.0,
            cycle_pos: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EnvPhase {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

impl Default for EnvState {
    fn default() -> Self {
        Self {
            phase: EnvPhase::Idle,
            level: 0.0,
            time_in_phase: 0.0,
            release_start_level: 0.0,
        }
    }
}

/// Reverb state (Freeverb algorithm)
#[derive(Debug, Clone)]
pub struct ReverbState {
    // Comb filter buffers (8 parallel combs)
    comb_buffers: Vec<Vec<f32>>,
    comb_indices: Vec<usize>,
    comb_filter_stores: Vec<f32>,

    // Allpass filter buffers (4 series allpasses)
    allpass_buffers: Vec<Vec<f32>>,
    allpass_indices: Vec<usize>,
}

impl ReverbState {
    pub fn new(sample_rate: f32) -> Self {
        // Freeverb comb filter delay times (in samples at 44.1kHz)
        let comb_tunings = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
        let allpass_tunings = [556, 441, 341, 225];

        let scale = sample_rate / 44100.0;

        let comb_buffers: Vec<Vec<f32>> = comb_tunings
            .iter()
            .map(|&size| vec![0.0; (size as f32 * scale) as usize])
            .collect();

        let allpass_buffers: Vec<Vec<f32>> = allpass_tunings
            .iter()
            .map(|&size| vec![0.0; (size as f32 * scale) as usize])
            .collect();

        Self {
            comb_buffers,
            comb_indices: vec![0; 8],
            comb_filter_stores: vec![0.0; 8],
            allpass_buffers,
            allpass_indices: vec![0; 4],
        }
    }
}

impl Default for ReverbState {
    fn default() -> Self {
        Self::new(44100.0)
    }
}

/// Bitcrusher state
#[derive(Debug, Clone)]
pub struct BitCrushState {
    phase: f32,
    last_sample: f32,
}

impl Default for BitCrushState {
    fn default() -> Self {
        Self {
            phase: 0.0,
            last_sample: 0.0,
        }
    }
}

/// Chorus state
#[derive(Debug, Clone)]
pub struct ChorusState {
    delay_buffer: Vec<f32>,
    write_idx: usize,
    lfo_phase: f32,
}

impl ChorusState {
    pub fn new(sample_rate: f32) -> Self {
        // 50ms max delay
        let buffer_size = (sample_rate * 0.05) as usize;
        Self {
            delay_buffer: vec![0.0; buffer_size],
            write_idx: 0,
            lfo_phase: 0.0,
        }
    }
}

impl Default for ChorusState {
    fn default() -> Self {
        Self::new(44100.0)
    }
}

/// Flanger state
#[derive(Debug, Clone)]
pub struct FlangerState {
    delay_buffer: Vec<f32>,
    write_idx: usize,
    lfo_phase: f32,
    feedback_sample: f32, // Previous output for feedback loop
}

impl FlangerState {
    pub fn new(sample_rate: f32) -> Self {
        // 10ms max delay for flanging (shorter than chorus)
        let buffer_size = (sample_rate * 0.01) as usize;
        Self {
            delay_buffer: vec![0.0; buffer_size],
            write_idx: 0,
            lfo_phase: 0.0,
            feedback_sample: 0.0,
        }
    }
}

impl Default for FlangerState {
    fn default() -> Self {
        Self::new(44100.0)
    }
}

/// Moog Ladder Filter state
#[derive(Debug, Clone)]
pub struct MoogLadderState {
    stage1: f32, // First filter stage
    stage2: f32, // Second filter stage
    stage3: f32, // Third filter stage
    stage4: f32, // Fourth filter stage (output)
}

impl MoogLadderState {
    pub fn new() -> Self {
        Self {
            stage1: 0.0,
            stage2: 0.0,
            stage3: 0.0,
            stage4: 0.0,
        }
    }
}

impl Default for MoogLadderState {
    fn default() -> Self {
        Self::new()
    }
}

/// Parametric EQ state (3 peaking filters)
#[derive(Debug, Clone)]
pub struct ParametricEQState {
    low_band: FilterState,
    mid_band: FilterState,
    high_band: FilterState,
}

impl ParametricEQState {
    pub fn new() -> Self {
        Self {
            low_band: FilterState::default(),
            mid_band: FilterState::default(),
            high_band: FilterState::default(),
        }
    }
}

impl Default for ParametricEQState {
    fn default() -> Self {
        Self::new()
    }
}

/// Pink noise state (Voss-McCartney algorithm)
/// Uses multiple octave bins updated at different rates
#[derive(Debug, Clone)]
pub struct PinkNoiseState {
    bins: [f32; 16],  // 16 octave bins for quality pink noise
    counter: u32,     // Sample counter for bin update decisions
}

impl PinkNoiseState {
    pub fn new() -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut bins = [0.0f32; 16];
        for bin in &mut bins {
            *bin = rng.gen_range(-1.0..1.0);
        }
        Self {
            bins,
            counter: 0,
        }
    }
}

impl Default for PinkNoiseState {
    fn default() -> Self {
        Self::new()
    }
}

/// Brown noise state (random walk / Brownian motion)
/// Uses leaky integrator to prevent DC drift
#[derive(Debug, Clone)]
pub struct BrownNoiseState {
    accumulator: f32, // Current accumulated value
}

impl BrownNoiseState {
    pub fn new() -> Self {
        Self {
            accumulator: 0.0,
        }
    }
}

impl Default for BrownNoiseState {
    fn default() -> Self {
        Self::new()
    }
}

/// Impulse generator state
/// Generates single-sample impulses at specified frequency
#[derive(Debug, Clone)]
pub struct ImpulseState {
    phase: f32, // Current phase position [0, 1)
}

impl ImpulseState {
    pub fn new() -> Self {
        // Start at 1.0 so first evaluation triggers immediately
        Self { phase: 1.0 }
    }
}

impl Default for ImpulseState {
    fn default() -> Self {
        Self::new()
    }
}

/// Lag (exponential slew limiter) state
/// Smooths abrupt changes with exponential approach
#[derive(Debug, Clone)]
pub struct LagState {
    previous_output: f32, // Previous smoothed output value
}

impl LagState {
    pub fn new() -> Self {
        Self {
            previous_output: 0.0,
        }
    }
}

impl Default for LagState {
    fn default() -> Self {
        Self::new()
    }
}

/// XLine (exponential envelope) state
/// Generates exponential ramp from start to end over duration
#[derive(Debug, Clone)]
pub struct XLineState {
    elapsed_samples: usize, // Number of samples generated so far
}

impl XLineState {
    pub fn new() -> Self {
        Self {
            elapsed_samples: 0,
        }
    }
}

impl Default for XLineState {
    fn default() -> Self {
        Self::new()
    }
}

/// ASR envelope phase
#[derive(Debug, Clone, PartialEq)]
pub enum ASRPhase {
    Idle,     // Envelope at 0, waiting for gate
    Attack,   // Rising from 0 to 1
    Sustain,  // Holding at 1 while gate is high
    Release,  // Falling from current level to 0
}

/// ASR (Attack-Sustain-Release) envelope state
/// Gate-based envelope: attacks when gate goes high, sustains while high, releases when gate goes low
#[derive(Debug, Clone)]
pub struct ASRState {
    phase: ASRPhase,
    current_level: f32,     // Current envelope output [0, 1]
    previous_gate: f32,     // Previous gate value for edge detection
}

impl ASRState {
    pub fn new() -> Self {
        Self {
            phase: ASRPhase::Idle,
            current_level: 0.0,
            previous_gate: 0.0,
        }
    }
}

impl Default for ASRState {
    fn default() -> Self {
        Self::new()
    }
}

/// Compressor state
#[derive(Debug, Clone)]
pub struct CompressorState {
    envelope: f32, // Current envelope follower value
}

impl CompressorState {
    pub fn new() -> Self {
        Self { envelope: 0.0 }
    }
}

impl Default for CompressorState {
    fn default() -> Self {
        Self::new()
    }
}

/// The unified signal graph that processes everything
pub struct UnifiedSignalGraph {
    /// All nodes in the graph
    nodes: Vec<Option<SignalNode>>,

    /// Named buses for easy reference
    buses: HashMap<String, NodeId>,

    /// Output node ID (for backwards compatibility - single output)
    output: Option<NodeId>,

    /// Multi-output: channel number -> node ID
    outputs: HashMap<usize, NodeId>,

    /// Hushed (silenced) output channels
    hushed_channels: std::collections::HashSet<usize>,

    /// Sample rate
    sample_rate: f32,

    /// Current cycle position for patterns
    cycle_position: f64,

    /// Cycles per second (tempo)
    cps: f32,

    /// Node ID counter
    next_node_id: usize,

    /// Computed values cache for current sample
    value_cache: HashMap<NodeId, f32>,

    /// Sample bank for loading and playing samples (RefCell for interior mutability)
    sample_bank: RefCell<SampleBank>,

    /// Voice manager for polyphonic sample playback
    voice_manager: RefCell<VoiceManager>,

    /// Synth voice manager for polyphonic synthesis
    synth_voice_manager: RefCell<SynthVoiceManager>,

    /// Sample counter for debugging
    sample_count: usize,
}

impl UnifiedSignalGraph {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            nodes: Vec::new(),
            buses: HashMap::new(),
            output: None,
            outputs: HashMap::new(),
            hushed_channels: std::collections::HashSet::new(),
            sample_rate,
            cycle_position: 0.0,
            cps: 0.5, // Default 0.5 cycles per second
            next_node_id: 0,
            value_cache: HashMap::new(),
            sample_bank: RefCell::new(SampleBank::new()),
            voice_manager: RefCell::new(VoiceManager::new()),
            synth_voice_manager: RefCell::new(SynthVoiceManager::new(sample_rate)),
            sample_count: 0,
        }
    }

    pub fn set_cps(&mut self, cps: f32) {
        self.cps = cps;
    }

    pub fn get_cps(&self) -> f32 {
        self.cps
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Get a reference to a node by its ID
    pub fn get_node(&self, node_id: NodeId) -> Option<&SignalNode> {
        self.nodes.get(node_id.0).and_then(|opt| opt.as_ref())
    }

    /// Add a node to the graph and return its ID
    pub fn add_node(&mut self, node: SignalNode) -> NodeId {
        let id = NodeId(self.next_node_id);
        self.next_node_id += 1;

        // Ensure vector is large enough
        while self.nodes.len() <= id.0 {
            self.nodes.push(None);
        }

        self.nodes[id.0] = Some(node);
        id
    }

    /// Register a named bus
    pub fn add_bus(&mut self, name: String, node_id: NodeId) {
        self.buses.insert(name, node_id);
    }

    /// Get a bus by name
    pub fn get_bus(&self, name: &str) -> Option<NodeId> {
        self.buses.get(name).copied()
    }

    /// Get all bus names
    pub fn get_all_bus_names(&self) -> Vec<String> {
        self.buses.keys().cloned().collect()
    }

    /// Set the output node
    pub fn set_output(&mut self, node_id: NodeId) {
        self.output = Some(node_id);
    }

    /// Check if output is set
    pub fn has_output(&self) -> bool {
        self.output.is_some()
    }

    /// Set a specific output channel (1-indexed for user convenience)
    pub fn set_output_channel(&mut self, channel: usize, node_id: NodeId) {
        self.outputs.insert(channel, node_id);
    }

    /// Silence all output channels
    pub fn hush_all(&mut self) {
        for &channel in self.outputs.keys() {
            self.hushed_channels.insert(channel);
        }
        // Also hush single output if it exists
        if self.output.is_some() {
            self.hushed_channels.insert(0);
        }
    }

    /// Silence a specific output channel
    pub fn hush_channel(&mut self, channel: usize) {
        self.hushed_channels.insert(channel);
    }

    /// Panic: kill all voices and silence all outputs
    pub fn panic(&mut self) {
        // Kill all active voices (samples and synths)
        self.voice_manager.borrow_mut().kill_all();
        self.synth_voice_manager.borrow_mut().kill_all();

        // Hush all outputs
        self.hush_all();
    }

    /// Get the number of currently active voices
    pub fn active_voice_count(&self) -> usize {
        self.voice_manager.borrow().active_voice_count()
    }

    /// Process one sample and return all output channels
    /// Returns a vector where outputs[0] = channel 1, outputs[1] = channel 2, etc.
    pub fn process_sample_multi(&mut self) -> Vec<f32> {
        // Clear cache for new sample
        self.value_cache.clear();

        // Collect outputs to avoid borrow checker issues
        let outputs_to_process: Vec<(usize, NodeId)> =
            self.outputs.iter().map(|(&ch, &node)| (ch, node)).collect();

        let single_output = self.output;

        // Determine max channel number
        let max_channel = outputs_to_process
            .iter()
            .map(|(ch, _)| *ch)
            .max()
            .unwrap_or(0);

        // Number of channels = max channel number (since channels are 1-indexed)
        let num_channels = max_channel;

        let mut outputs_vec = vec![0.0; num_channels];

        // Evaluate each output channel
        // Channel numbers are 1-indexed, but vec indices are 0-indexed
        // So channel N goes to outputs_vec[N-1]
        for (channel, node_id) in outputs_to_process {
            if channel > 0 && channel <= num_channels {
                let value = if self.hushed_channels.contains(&channel) {
                    0.0 // Silenced channel
                } else {
                    self.eval_node(&node_id)
                };
                outputs_vec[channel - 1] = value;
            }
        }

        // Handle backwards compatibility - single output goes to first position if no multi-outputs
        if outputs_vec.is_empty() {
            if let Some(output_id) = single_output {
                let value = if self.hushed_channels.contains(&0) {
                    0.0
                } else {
                    self.eval_node(&output_id)
                };
                outputs_vec.push(value);
            }
        }

        // Advance cycle position
        self.cycle_position += self.cps as f64 / self.sample_rate as f64;

        // Increment sample counter
        self.sample_count += 1;

        outputs_vec
    }

    /// Evaluate a signal to get its current value
    fn eval_signal(&mut self, signal: &Signal) -> f32 {
        self.eval_signal_at_time(signal, self.cycle_position)
    }

    /// Evaluate a signal at a specific cycle position
    /// This allows per-event DSP parameter evaluation
    fn eval_signal_at_time(&mut self, signal: &Signal, cycle_pos: f64) -> f32 {
        match signal {
            Signal::Node(id) => {
                // CRITICAL FIX: For Pattern nodes, query at the specified cycle_pos
                // instead of self.cycle_position to ensure each event gets the correct
                // parameter value from pattern-valued DSP parameters like gain "1.0 0.5"
                if let Some(Some(SignalNode::Pattern { pattern, pattern_str, .. })) = self.nodes.get(id.0) {
                    let sample_width = 1.0 / self.sample_rate as f64 / self.cps as f64;
                    let state = State {
                        span: TimeSpan::new(
                            Fraction::from_float(cycle_pos),
                            Fraction::from_float(cycle_pos + sample_width),
                        ),
                        controls: HashMap::new(),
                    };

                    let events = pattern.query(&state);

                    // DEBUG: Log pattern signal evaluation
                    if std::env::var("DEBUG_PATTERN").is_ok() && self.sample_count < 44200 && self.sample_count % 2200 == 0 {
                        eprintln!("Signal Pattern '{}' at cycle {:.6}, sample {}: {} events",
                                 pattern_str, cycle_pos, self.sample_count, events.len());
                        if let Some(event) = events.first() {
                            eprintln!("  First event: '{}' at [{:.6}, {:.6})",
                                     event.value, event.part.begin.to_float(), event.part.end.to_float());
                        }
                    }

                    if let Some(event) = events.first() {
                        let s = event.value.as_str();
                        if s == "~" || s.is_empty() {
                            0.0
                        } else {
                            use crate::pattern_tonal::{midi_to_freq, note_to_midi};
                            if let Ok(numeric_value) = s.parse::<f32>() {
                                numeric_value
                            } else if let Some(midi) = note_to_midi(s) {
                                midi_to_freq(midi) as f32
                            } else {
                                1.0
                            }
                        }
                    } else {
                        0.0
                    }
                } else {
                    // For non-Pattern nodes (oscillators, filters, etc.),
                    // use eval_node which evaluates at current cycle position
                    self.eval_node(id)
                }
            }
            Signal::Bus(name) => {
                if let Some(id) = self.buses.get(name).cloned() {
                    self.eval_node(&id)
                } else {
                    0.0
                }
            }
            Signal::Pattern(pattern_str) => {
                // Parse and evaluate pattern at specified cycle position
                let pattern = parse_mini_notation(pattern_str);
                let sample_width = 1.0 / self.sample_rate as f64 / self.cps as f64;
                let state = State {
                    span: TimeSpan::new(
                        Fraction::from_float(cycle_pos),
                        Fraction::from_float(cycle_pos + sample_width),
                    ),
                    controls: HashMap::new(),
                };

                let events = pattern.query(&state);
                if let Some(event) = events.first() {
                    // Convert pattern value to float
                    // Signal::Pattern is for NUMERIC patterns (frequencies, control values)
                    let s = event.value.as_str();
                    if s == "~" || s.is_empty() {
                        0.0
                    } else {
                        // Try numeric parsing first, then fall back to note names
                        // This ensures "110", "220", "440" etc are treated as numbers, not MIDI notes
                        use crate::pattern_tonal::{midi_to_freq, note_to_midi};
                        if let Ok(numeric_value) = s.parse::<f32>() {
                            numeric_value
                        } else if let Some(midi) = note_to_midi(s) {
                            // Fall back to note name parsing (e.g., "c4", "a4", "cs4")
                            midi_to_freq(midi) as f32
                        } else {
                            // If neither works, default to 1.0
                            1.0
                        }
                    }
                } else {
                    0.0
                }
            }
            Signal::Value(v) => *v,
            Signal::Expression(expr) => self.eval_expression(expr),
        }
    }

    /// Evaluate a signal expression
    fn eval_expression(&mut self, expr: &SignalExpr) -> f32 {
        match expr {
            SignalExpr::Add(a, b) => self.eval_signal(a) + self.eval_signal(b),
            SignalExpr::Multiply(a, b) => self.eval_signal(a) * self.eval_signal(b),
            SignalExpr::Subtract(a, b) => self.eval_signal(a) - self.eval_signal(b),
            SignalExpr::Divide(a, b) => {
                let b_val = self.eval_signal(b);
                if b_val != 0.0 {
                    self.eval_signal(a) / b_val
                } else {
                    0.0
                }
            }
            SignalExpr::Modulo(a, b) => {
                let b_val = self.eval_signal(b);
                if b_val != 0.0 {
                    self.eval_signal(a) % b_val
                } else {
                    0.0
                }
            }
            SignalExpr::Scale { input, min, max } => {
                let v = self.eval_signal(input);
                v * (max - min) + min
            }
        }
    }

    /// Evaluate a node to get its current output value
    fn eval_node(&mut self, node_id: &NodeId) -> f32 {
        // Check cache first
        if let Some(&cached) = self.value_cache.get(node_id) {
            return cached;
        }

        // Get node (have to clone due to borrow checker)
        let node = if let Some(Some(node)) = self.nodes.get(node_id.0) {
            node.clone()
        } else {
            return 0.0;
        };

        let value = match node {
            SignalNode::Oscillator {
                freq,
                waveform,
                phase,
                pending_freq,
                last_sample,
            } => {
                let requested_freq = self.eval_signal(&freq);
                let mut current_freq = requested_freq;

                // Zero-crossing detection for anti-click frequency changes
                // If there's a pending frequency change, use it until zero-crossing
                if let Some(pending) = pending_freq {
                    current_freq = pending; // Use pending freq until zero-crossing
                }

                // Generate sample based on waveform
                let sample = match waveform {
                    Waveform::Sine => (2.0 * PI * phase).sin(),
                    Waveform::Saw => 2.0 * phase - 1.0,
                    Waveform::Square => {
                        if phase < 0.5 {
                            1.0
                        } else {
                            -1.0
                        }
                    }
                    Waveform::Triangle => {
                        if phase < 0.5 {
                            4.0 * phase - 1.0
                        } else {
                            3.0 - 4.0 * phase
                        }
                    }
                };

                // Update phase and detect zero-crossings
                if let Some(Some(node)) = self.nodes.get_mut(node_id.0) {
                    if let SignalNode::Oscillator { phase: p, pending_freq: pf, last_sample: ls, .. } = node {
                        // Check if frequency changed
                        if (requested_freq - current_freq).abs() > 0.1 {
                            // Frequency change requested - set it as pending
                            *pf = Some(current_freq);
                        }

                        // Check for zero-crossing (sign change from negative to positive)
                        if let Some(_pending) = pf {
                            if *ls < 0.0 && sample >= 0.0 {
                                // Zero-crossing detected! Apply the frequency change
                                *pf = None; // Clear pending
                            }
                        }

                        // Update phase for next sample
                        let freq_to_use = if pf.is_some() { current_freq } else { requested_freq };
                        *p += freq_to_use / self.sample_rate;
                        if *p >= 1.0 {
                            *p -= 1.0;
                        }

                        // Store sample for next zero-crossing detection
                        *ls = sample;
                    }
                }

                sample
            }

            SignalNode::FMOscillator {
                carrier_freq,
                modulator_freq,
                mod_index,
                carrier_phase,
                modulator_phase,
            } => {
                // Evaluate modulatable parameters
                let carrier_f = self.eval_signal(&carrier_freq).max(0.0);
                let modulator_f = self.eval_signal(&modulator_freq).max(0.0);
                let index = self.eval_signal(&mod_index).max(0.0);

                // FM synthesis: carrier modulated by modulator
                // output = sin(2π * carrier_phase + mod_index * sin(2π * modulator_phase))
                let modulator_value = (2.0 * PI * modulator_phase).sin();
                let modulation = index * modulator_value;
                let sample = (2.0 * PI * carrier_phase + modulation).sin();

                // Update phases for next sample
                if let Some(Some(node)) = self.nodes.get_mut(node_id.0) {
                    if let SignalNode::FMOscillator {
                        carrier_phase: cp,
                        modulator_phase: mp,
                        ..
                    } = node
                    {
                        *cp += carrier_f / self.sample_rate;
                        if *cp >= 1.0 {
                            *cp -= 1.0;
                        }

                        *mp += modulator_f / self.sample_rate;
                        if *mp >= 1.0 {
                            *mp -= 1.0;
                        }
                    }
                }

                sample
            }

            SignalNode::WhiteNoise => {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                // Generate uniformly distributed random sample in [-1, 1]
                rng.gen_range(-1.0..1.0)
            }

            SignalNode::PinkNoise { state } => {
                use rand::Rng;
                let mut rng = rand::thread_rng();

                // Voss-McCartney algorithm: update bins based on counter bit patterns
                // Each bin updates at 1/2^i rate (bin 0 every sample, bin 1 every 2, etc.)
                let counter = state.counter;
                let mut bins = state.bins;

                // Update bins whose bit changed from 0 to 1
                for i in 0..16 {
                    let mask = 1u32 << i;
                    if (counter & mask) == 0 {
                        // This bin should update (its bit is 0, was 1)
                        bins[i] = rng.gen_range(-1.0..1.0);
                    }
                }

                // Update state for next sample
                if let Some(Some(node)) = self.nodes.get_mut(node_id.0) {
                    if let SignalNode::PinkNoise { state: s } = node {
                        s.bins = bins;
                        s.counter = counter.wrapping_add(1);
                    }
                }

                // Sum all bins and normalize
                let sum: f32 = bins.iter().sum();
                sum / 16.0 // Normalize by number of bins
            }

            SignalNode::BrownNoise { state } => {
                use rand::Rng;
                let mut rng = rand::thread_rng();

                // Random walk / Brownian motion algorithm
                // Add small random step to accumulator
                let current = state.accumulator;
                let step = rng.gen_range(-1.0..1.0) * 0.1; // Small random step
                let mut new_accumulator = current + step;

                // Leaky integrator to prevent DC drift (decay toward zero)
                new_accumulator *= 0.998; // 0.2% decay per sample

                // Soft clip to prevent explosion
                new_accumulator = new_accumulator.clamp(-1.5, 1.5);

                // Update state for next sample
                if let Some(Some(node)) = self.nodes.get_mut(node_id.0) {
                    if let SignalNode::BrownNoise { state: s } = node {
                        s.accumulator = new_accumulator;
                    }
                }

                // Normalize output to approximately -1 to 1
                new_accumulator * 0.7
            }

            SignalNode::Impulse { frequency, state } => {
                let freq = self.eval_signal(&frequency).max(0.0);
                let current_phase = state.phase;

                // Calculate phase increment based on frequency
                let phase_increment = freq / self.sample_rate;

                // Increment phase
                let new_phase = current_phase + phase_increment;

                // Determine output (impulse occurs when phase wraps around 1.0)
                let output = if new_phase >= 1.0 {
                    1.0 // Impulse! Phase just wrapped around
                } else {
                    0.0 // Silence
                };

                // Wrap phase to [0, 1)
                let wrapped_phase = if new_phase >= 1.0 {
                    new_phase.fract()
                } else {
                    new_phase
                };

                // Update state for next sample
                if let Some(Some(node)) = self.nodes.get_mut(node_id.0) {
                    if let SignalNode::Impulse { state: s, .. } = node {
                        s.phase = wrapped_phase;
                    }
                }

                output
            }

            SignalNode::Lag {
                input,
                lag_time,
                state,
            } => {
                let input_val = self.eval_signal(&input);
                let time = self.eval_signal(&lag_time).max(0.0);
                let prev = state.previous_output;

                // Calculate smoothing coefficient using exponential formula
                // coefficient = 1 - e^(-1 / (lag_time * sample_rate))
                // For lag_time = 0, coefficient ≈ 1 (bypass)
                // For larger lag_time, coefficient gets smaller (slower response)
                let coefficient = if time < 0.00001 {
                    // Avoid division by zero, bypass for very small lag times
                    1.0
                } else {
                    let samples_per_time_constant = time * self.sample_rate;
                    1.0 - (-1.0 / samples_per_time_constant).exp()
                };

                // Exponential smoothing: approach target exponentially
                let output = prev + (input_val - prev) * coefficient;

                // Update state for next sample
                if let Some(Some(node)) = self.nodes.get_mut(node_id.0) {
                    if let SignalNode::Lag { state: s, .. } = node {
                        s.previous_output = output;
                    }
                }

                output
            }

            SignalNode::XLine {
                start,
                end,
                duration,
                state,
            } => {
                let start_val = self.eval_signal(&start);
                let end_val = self.eval_signal(&end);
                let dur = self.eval_signal(&duration).max(0.0);
                let elapsed = state.elapsed_samples;

                // Calculate progress (0.0 to 1.0)
                let total_samples = (dur * self.sample_rate).max(1.0);
                let progress = (elapsed as f32 / total_samples).min(1.0);

                // Generate exponential curve
                // Formula: value = start * (end/start)^progress
                // This creates exponential interpolation between start and end
                let output = if progress >= 1.0 {
                    // After duration, hold at end value
                    end_val
                } else if dur < 0.00001 {
                    // Very short duration, jump to end immediately
                    end_val
                } else if start_val.abs() < 0.00001 {
                    // Start is zero, use linear interpolation
                    start_val + (end_val - start_val) * progress
                } else if (start_val > 0.0) != (end_val > 0.0) {
                    // Different signs, use linear interpolation
                    start_val + (end_val - start_val) * progress
                } else {
                    // Both same sign and non-zero, use exponential curve
                    let ratio = end_val / start_val;
                    start_val * ratio.powf(progress)
                };

                // Update state for next sample
                if let Some(Some(node)) = self.nodes.get_mut(node_id.0) {
                    if let SignalNode::XLine { state: s, .. } = node {
                        s.elapsed_samples = elapsed + 1;
                    }
                }

                output
            }

            SignalNode::ASR {
                gate,
                attack,
                release,
                state,
            } => {
                let gate_val = self.eval_signal(&gate);
                let attack_time = self.eval_signal(&attack).max(0.0001);
                let release_time = self.eval_signal(&release).max(0.0001);

                let current_phase = state.phase.clone();
                let current_level = state.current_level;
                let prev_gate = state.previous_gate;

                // Detect gate transitions
                let gate_rising = gate_val > 0.5 && prev_gate <= 0.5;
                let gate_high = gate_val > 0.5;

                // Determine next phase
                let next_phase = match current_phase {
                    ASRPhase::Idle => {
                        if gate_rising {
                            ASRPhase::Attack
                        } else {
                            ASRPhase::Idle
                        }
                    }
                    ASRPhase::Attack => {
                        if !gate_high {
                            ASRPhase::Release
                        } else if current_level >= 0.99 {
                            ASRPhase::Sustain
                        } else {
                            ASRPhase::Attack
                        }
                    }
                    ASRPhase::Sustain => {
                        if !gate_high {
                            ASRPhase::Release
                        } else {
                            ASRPhase::Sustain
                        }
                    }
                    ASRPhase::Release => {
                        if gate_rising {
                            ASRPhase::Attack
                        } else if current_level <= 0.01 {
                            ASRPhase::Idle
                        } else {
                            ASRPhase::Release
                        }
                    }
                };

                // Calculate envelope output based on phase
                let output = match next_phase {
                    ASRPhase::Idle => 0.0,
                    ASRPhase::Attack => {
                        // Linear ramp up to 1.0
                        let increment = 1.0 / (attack_time * self.sample_rate);
                        (current_level + increment).min(1.0)
                    }
                    ASRPhase::Sustain => 1.0,
                    ASRPhase::Release => {
                        // Linear ramp down to 0.0
                        let decrement = 1.0 / (release_time * self.sample_rate);
                        (current_level - decrement).max(0.0)
                    }
                };

                // Update state for next sample
                if let Some(Some(node)) = self.nodes.get_mut(node_id.0) {
                    if let SignalNode::ASR { state: s, .. } = node {
                        s.phase = next_phase;
                        s.current_level = output;
                        s.previous_gate = gate_val;
                    }
                }

                output
            }

            SignalNode::Pulse { freq, width, phase } => {
                // Evaluate modulatable parameters
                let f = self.eval_signal(&freq).max(0.0);
                let w = self.eval_signal(&width).clamp(0.0, 1.0);

                // Pulse wave: output +1 when phase < width, -1 otherwise
                let sample = if phase < w { 1.0 } else { -1.0 };

                // Update phase for next sample
                if let Some(Some(node)) = self.nodes.get_mut(node_id.0) {
                    if let SignalNode::Pulse { phase: p, .. } = node {
                        *p += f / self.sample_rate;
                        if *p >= 1.0 {
                            *p -= 1.0;
                        }
                    }
                }

                sample
            }

            SignalNode::Limiter { input, threshold } => {
                // Evaluate input signal and threshold
                let input_val = self.eval_signal(&input);
                let thresh = self.eval_signal(&threshold).max(0.0);

                // Brick-wall limiting: clamp to [-threshold, +threshold]
                input_val.clamp(-thresh, thresh)
            }

            SignalNode::Pan2Left { input, position } => {
                // Evaluate input signal and pan position
                let input_val = self.eval_signal(&input);
                let pan = self.eval_signal(&position).clamp(-1.0, 1.0);

                // Equal-power panning law
                // Map pan from [-1, 1] to angle [0, π/2]
                let angle = (pan + 1.0) * std::f32::consts::PI / 4.0;
                let left_gain = angle.cos();

                input_val * left_gain
            }

            SignalNode::Pan2Right { input, position } => {
                // Evaluate input signal and pan position
                let input_val = self.eval_signal(&input);
                let pan = self.eval_signal(&position).clamp(-1.0, 1.0);

                // Equal-power panning law
                // Map pan from [-1, 1] to angle [0, π/2]
                let angle = (pan + 1.0) * std::f32::consts::PI / 4.0;
                let right_gain = angle.sin();

                input_val * right_gain
            }

            SignalNode::Constant { value } => value,

            SignalNode::Add { a, b } => self.eval_signal(&a) + self.eval_signal(&b),

            SignalNode::Multiply { a, b } => self.eval_signal(&a) * self.eval_signal(&b),

            SignalNode::XFade {
                signal_a,
                signal_b,
                position,
            } => {
                let a_val = self.eval_signal(&signal_a);
                let b_val = self.eval_signal(&signal_b);
                let pos = self.eval_signal(&position).clamp(0.0, 1.0);

                // Linear crossfade: (1-pos)*a + pos*b
                (1.0 - pos) * a_val + pos * b_val
            }

            SignalNode::Mix { signals } => {
                // Sum all input signals
                signals.iter().map(|s| self.eval_signal(s)).sum()
            }

            SignalNode::Allpass {
                input,
                coefficient,
                ..
            } => {
                let x = self.eval_signal(&input);
                let g = self.eval_signal(&coefficient).clamp(-1.0, 1.0);

                // Get previous state
                let (x1, y1) = if let Some(Some(SignalNode::Allpass { state, .. })) =
                    self.nodes.get(node_id.0)
                {
                    (state.x1, state.y1)
                } else {
                    (0.0, 0.0)
                };

                // First-order allpass filter
                // y[n] = g * (x[n] - y[n-1]) + x[n-1]
                let y = g * (x - y1) + x1;

                // Update state
                if let Some(Some(SignalNode::Allpass { state, .. })) =
                    self.nodes.get_mut(node_id.0)
                {
                    state.x1 = x;
                    state.y1 = y;
                }

                y
            }

            SignalNode::LowPass {
                input, cutoff, q, ..
            } => {
                let input_val = self.eval_signal(&input);
                let fc = self.eval_signal(&cutoff).max(20.0).min(20000.0);
                let q_val = self.eval_signal(&q).max(0.5).min(20.0);

                // State variable filter (Chamberlin)
                // Better frequency response and resonance
                let f = 2.0 * (PI * fc / self.sample_rate).sin();
                let damp = 1.0 / q_val;

                // Get state
                let (mut low, mut band, mut high) = if let Some(Some(SignalNode::LowPass {
                    state,
                    ..
                })) = self.nodes.get(node_id.0)
                {
                    (state.y1, state.x1, state.y2)
                } else {
                    (0.0, 0.0, 0.0)
                };

                // Process
                high = input_val - low - damp * band;
                band += f * high;
                low += f * band;

                // Update state
                if let Some(Some(SignalNode::LowPass { state, .. })) = self.nodes.get_mut(node_id.0)
                {
                    state.y1 = low;
                    state.x1 = band;
                    state.y2 = high;
                }

                low
            }

            SignalNode::When { input, condition } => {
                let cond = self.eval_signal(&condition);
                if cond > 0.0 {
                    self.eval_signal(&input)
                } else {
                    0.0
                }
            }

            SignalNode::Reverb {
                input,
                room_size,
                damping,
                mix,
                state,
            } => {
                let input_val = self.eval_signal(&input);
                let room = self.eval_signal(&room_size).clamp(0.0, 1.0);
                let damp = self.eval_signal(&damping).clamp(0.0, 1.0);
                let mix_val = self.eval_signal(&mix).clamp(0.0, 1.0);

                // Process comb filters (parallel)
                let mut comb_out = 0.0;
                for i in 0..8 {
                    let buf_len = state.comb_buffers[i].len();
                    let read_idx = state.comb_indices[i];
                    let delayed = state.comb_buffers[i][read_idx];

                    // Lowpass filter for damping
                    let filtered = state.comb_filter_stores[i] * damp + delayed * (1.0 - damp);

                    // Feedback
                    let feedback = 0.84 * room;
                    let to_write = input_val + filtered * feedback;

                    comb_out += delayed;

                    // Update state
                    if let Some(Some(SignalNode::Reverb { state: s, .. })) =
                        self.nodes.get_mut(node_id.0)
                    {
                        s.comb_buffers[i][read_idx] = to_write;
                        s.comb_indices[i] = (read_idx + 1) % buf_len;
                        s.comb_filter_stores[i] = filtered;
                    }
                }

                let mut allpass_out = comb_out / 8.0;

                // Process allpass filters (series)
                for i in 0..4 {
                    let buf_len = state.allpass_buffers[i].len();
                    let read_idx = state.allpass_indices[i];
                    let delayed = state.allpass_buffers[i][read_idx];

                    let to_write = allpass_out + delayed * 0.5;
                    allpass_out = delayed - allpass_out * 0.5;

                    if let Some(Some(SignalNode::Reverb { state: s, .. })) =
                        self.nodes.get_mut(node_id.0)
                    {
                        s.allpass_buffers[i][read_idx] = to_write;
                        s.allpass_indices[i] = (read_idx + 1) % buf_len;
                    }
                }

                // Mix dry and wet
                input_val * (1.0 - mix_val) + allpass_out * mix_val
            }

            SignalNode::Distortion { input, drive, mix } => {
                let input_val = self.eval_signal(&input);
                let drive_val = self.eval_signal(&drive).clamp(1.0, 100.0);
                let mix_val = self.eval_signal(&mix).clamp(0.0, 1.0);

                // Soft clipping waveshaper
                let driven = input_val * drive_val;
                let distorted = driven.tanh();

                input_val * (1.0 - mix_val) + distorted * mix_val
            }

            SignalNode::BitCrush {
                input,
                bits,
                sample_rate,
                state,
            } => {
                let input_val = self.eval_signal(&input);
                let bit_depth = self.eval_signal(&bits).clamp(1.0, 16.0);
                let rate_reduction = self.eval_signal(&sample_rate).clamp(1.0, 64.0);

                let phase = state.phase + rate_reduction;
                let mut output = state.last_sample;

                if phase >= 1.0 {
                    // Reduce bit depth
                    let levels = (2.0_f32).powf(bit_depth);
                    let quantized = (input_val * levels).round() / levels;
                    output = quantized;

                    if let Some(Some(SignalNode::BitCrush { state: s, .. })) =
                        self.nodes.get_mut(node_id.0)
                    {
                        s.phase = phase - phase.floor();
                        s.last_sample = quantized;
                    }
                } else if let Some(Some(SignalNode::BitCrush { state: s, .. })) =
                    self.nodes.get_mut(node_id.0)
                {
                    s.phase = phase;
                }

                output
            }

            SignalNode::Chorus {
                input,
                rate,
                depth,
                mix,
                state,
            } => {
                let input_val = self.eval_signal(&input);
                let lfo_rate = self.eval_signal(&rate).clamp(0.1, 10.0);
                let mod_depth = self.eval_signal(&depth).clamp(0.0, 1.0);
                let mix_val = self.eval_signal(&mix).clamp(0.0, 1.0);

                // LFO for delay modulation
                let lfo_phase = state.lfo_phase;
                let lfo = (lfo_phase * 2.0 * std::f32::consts::PI).sin();

                // Modulated delay time (5-25ms)
                let base_delay = 0.015; // 15ms
                let delay_time = base_delay + lfo * mod_depth * 0.010; // ±10ms
                let delay_samples = (delay_time * self.sample_rate) as f32;

                // Read from delay buffer with linear interpolation
                let buf_len = state.delay_buffer.len();
                let read_pos =
                    (state.write_idx as f32 + buf_len as f32 - delay_samples) % buf_len as f32;
                let read_idx = read_pos.floor() as usize;
                let frac = read_pos - read_pos.floor();

                let sample1 = state.delay_buffer[read_idx % buf_len];
                let sample2 = state.delay_buffer[(read_idx + 1) % buf_len];
                let delayed = sample1 + (sample2 - sample1) * frac;

                // Update state
                if let Some(Some(SignalNode::Chorus { state: s, .. })) =
                    self.nodes.get_mut(node_id.0)
                {
                    s.delay_buffer[s.write_idx] = input_val;
                    s.write_idx = (s.write_idx + 1) % buf_len;
                    s.lfo_phase = (lfo_phase + lfo_rate / self.sample_rate) % 1.0;
                }

                // Mix dry and wet
                input_val * (1.0 - mix_val) + delayed * mix_val
            }

            SignalNode::Flanger {
                input,
                depth,
                rate,
                feedback,
                state,
            } => {
                let input_val = self.eval_signal(&input);
                let lfo_rate = self.eval_signal(&rate).clamp(0.1, 10.0);
                let mod_depth = self.eval_signal(&depth).clamp(0.0, 1.0);
                let feedback_amt = self.eval_signal(&feedback).clamp(0.0, 0.95);

                // Bypass effect if depth is very small
                if mod_depth < 0.01 {
                    // Still update LFO phase for continuity
                    if let Some(Some(SignalNode::Flanger { state: s, .. })) =
                        self.nodes.get_mut(node_id.0)
                    {
                        s.lfo_phase = (state.lfo_phase + lfo_rate / self.sample_rate) % 1.0;
                    }
                    return input_val;
                }

                // LFO for delay modulation (sine wave)
                let lfo_phase = state.lfo_phase;
                let lfo = (lfo_phase * 2.0 * std::f32::consts::PI).sin();

                // Modulated delay time (1-5ms for flanging)
                let base_delay = 0.003; // 3ms
                let delay_time = base_delay + lfo * mod_depth * 0.002; // ±2ms
                let delay_samples = (delay_time * self.sample_rate) as f32;

                // Read from delay buffer with linear interpolation
                let buf_len = state.delay_buffer.len();
                let read_pos =
                    (state.write_idx as f32 + buf_len as f32 - delay_samples) % buf_len as f32;
                let read_idx = read_pos.floor() as usize;
                let frac = read_pos - read_pos.floor();

                let sample1 = state.delay_buffer[read_idx % buf_len];
                let sample2 = state.delay_buffer[(read_idx + 1) % buf_len];
                let delayed = sample1 + (sample2 - sample1) * frac;

                // Apply feedback (with feedback limiting to prevent explosion)
                let wet = delayed + state.feedback_sample * feedback_amt;

                // Update state
                if let Some(Some(SignalNode::Flanger { state: s, .. })) =
                    self.nodes.get_mut(node_id.0)
                {
                    s.delay_buffer[s.write_idx] = input_val;
                    s.write_idx = (s.write_idx + 1) % buf_len;
                    s.lfo_phase = (lfo_phase + lfo_rate / self.sample_rate) % 1.0;
                    s.feedback_sample = wet;
                }

                // Classic flanger: equal mix of dry and wet, scaled by depth
                let mix = 0.5 * mod_depth; // depth controls wet amount
                input_val * (1.0 - mix) + wet * mix
            }

            SignalNode::Compressor {
                input,
                threshold,
                ratio,
                attack,
                release,
                makeup_gain,
                state,
            } => {
                let input_val = self.eval_signal(&input);
                let threshold_db = self.eval_signal(&threshold).clamp(-60.0, 0.0);
                let ratio_val = self.eval_signal(&ratio).clamp(1.0, 20.0);
                let attack_time = self.eval_signal(&attack).clamp(0.001, 1.0);
                let release_time = self.eval_signal(&release).clamp(0.01, 3.0);
                let makeup_db = self.eval_signal(&makeup_gain).clamp(0.0, 30.0);

                // Convert threshold from dB to linear
                let threshold_lin = 10.0_f32.powf(threshold_db / 20.0);

                // Envelope follower (peak detector with attack/release)
                let input_level = input_val.abs();
                let mut envelope = state.envelope;

                // Envelope follower: attack when input > envelope, release when input < envelope
                let coeff = if input_level > envelope {
                    // Attack: faster response to increasing levels
                    (-(1.0 / (attack_time * self.sample_rate))).exp()
                } else {
                    // Release: slower response to decreasing levels
                    (-(1.0 / (release_time * self.sample_rate))).exp()
                };

                envelope = coeff * envelope + (1.0 - coeff) * input_level;

                // Update envelope state
                if let Some(Some(SignalNode::Compressor { state: s, .. })) =
                    self.nodes.get_mut(node_id.0)
                {
                    s.envelope = envelope;
                }

                // Calculate gain reduction
                let gain_reduction = if envelope > threshold_lin {
                    // Above threshold: apply compression
                    // Gain reduction (dB) = (threshold - envelope) * (1 - 1/ratio)
                    let envelope_db = 20.0 * envelope.log10();
                    let over_db = envelope_db - threshold_db;
                    let reduction_db = over_db * (1.0 - 1.0 / ratio_val);
                    10.0_f32.powf(-reduction_db / 20.0) // Convert to linear gain reduction
                } else {
                    1.0 // No reduction below threshold
                };

                // Apply makeup gain
                let makeup_gain_lin = 10.0_f32.powf(makeup_db / 20.0);

                // Apply compression and makeup gain
                input_val * gain_reduction * makeup_gain_lin
            }

            SignalNode::Tremolo {
                input,
                rate,
                depth,
                phase,
            } => {
                let input_val = self.eval_signal(&input);
                let rate_hz = self.eval_signal(&rate).clamp(0.1, 20.0);
                let depth_val = self.eval_signal(&depth).clamp(0.0, 1.0);

                // Fast bypass for zero depth
                if depth_val < 0.001 {
                    return input_val;
                }

                let mut output_val = input_val;

                // Update phase and calculate LFO
                if let Some(Some(SignalNode::Tremolo { phase: p, .. })) =
                    self.nodes.get_mut(node_id.0)
                {
                    // Advance phase
                    *p += rate_hz * 2.0 * std::f32::consts::PI / self.sample_rate;

                    // Wrap phase to [0, 2π]
                    if *p >= 2.0 * std::f32::consts::PI {
                        *p -= 2.0 * std::f32::consts::PI;
                    }

                    // Calculate LFO (sine wave, -1 to +1)
                    let lfo = p.sin();

                    // Convert LFO to modulation amount
                    // depth=0: mod=1 (no effect)
                    // depth=1: mod oscillates 0 to 1
                    let modulation = 1.0 - depth_val * 0.5 + depth_val * 0.5 * lfo;

                    // Apply amplitude modulation
                    output_val = input_val * modulation;
                }

                output_val
            }

            SignalNode::FundspUnit {
                unit_type,
                input,
                params,
                state,
            } => {
                // 1. Evaluate Phonon input signal
                let input_sample = self.eval_signal(&input);

                // 2. Evaluate Phonon parameter signals (PATTERN MODULATION!)
                let mut param_values = Vec::new();
                for param_signal in &params {
                    param_values.push(self.eval_signal(param_signal));
                }

                // 3. Update fundsp unit parameters based on unit type
                let mut state_guard = state.lock().unwrap();
                match unit_type {
                    FundspUnitType::OrganHz => {
                        // Parameter 0: frequency
                        if !params.is_empty() {
                            let freq = param_values[0];
                            state_guard.update_frequency(freq, self.sample_rate as f64);
                        }
                    }
                    FundspUnitType::MoogHz => {
                        // Parameters: 0=cutoff, 1=resonance
                        if param_values.len() >= 2 {
                            let cutoff = param_values[0];
                            let resonance = param_values[1];
                            state_guard.update_moog_params(cutoff, resonance, self.sample_rate as f64);
                        }
                    }
                    FundspUnitType::ReverbStereo => {
                        // Parameters: 0=wet, 1=time
                        if param_values.len() >= 2 {
                            let wet = param_values[0];
                            let time = param_values[1];
                            state_guard.update_reverb_params(wet, time, self.sample_rate as f64);
                        }
                    }
                    FundspUnitType::Chorus => {
                        // Parameters: 0=seed, 1=separation, 2=variation, 3=mod_frequency
                        if param_values.len() >= 4 {
                            let seed = param_values[0] as u64;
                            let separation = param_values[1];
                            let variation = param_values[2];
                            let mod_frequency = param_values[3];
                            state_guard.update_chorus_params(
                                seed,
                                separation,
                                variation,
                                mod_frequency,
                                self.sample_rate as f64,
                            );
                        }
                    }
                    FundspUnitType::SawHz => {
                        // Parameters: 0=frequency
                        if param_values.len() >= 1 {
                            let frequency = param_values[0];
                            state_guard.update_saw_frequency(frequency, self.sample_rate as f64);
                        }
                    }
                    FundspUnitType::SquareHz => {
                        // Parameters: 0=frequency
                        if param_values.len() >= 1 {
                            let frequency = param_values[0];
                            state_guard.update_square_frequency(frequency, self.sample_rate as f64);
                        }
                    }
                    FundspUnitType::TriangleHz => {
                        // Parameters: 0=frequency
                        if param_values.len() >= 1 {
                            let frequency = param_values[0];
                            state_guard.update_triangle_frequency(frequency, self.sample_rate as f64);
                        }
                    }
                    FundspUnitType::Noise => {
                        // No parameters to update!
                    }
                    _ => {
                        // TODO: Implement other unit types
                    }
                }

                // 4. Call fundsp tick() to generate output
                let output = state_guard.tick(input_sample);

                output
            }

            SignalNode::Output { input } => self.eval_signal(&input),

            SignalNode::Pattern {
                pattern_str,
                pattern,
                last_value,
                last_trigger_time: _,
            } => {
                // Query pattern for events at current cycle position
                let sample_width = 1.0 / self.sample_rate as f64 / self.cps as f64;
                let state = State {
                    span: TimeSpan::new(
                        Fraction::from_float(self.cycle_position),
                        Fraction::from_float(self.cycle_position + sample_width),
                    ),
                    controls: HashMap::new(),
                };

                let events = pattern.query(&state);
                let mut current_value = last_value; // Default to last value

                // DEBUG: Log all pattern queries
                if std::env::var("DEBUG_PATTERN").is_ok() && self.sample_count < 200 && self.sample_count % 20 == 0 {
                    eprintln!("Pattern '{}' at cycle {:.6}, sample {}: {} events",
                             pattern_str, self.cycle_position, self.sample_count, events.len());
                    if let Some(event) = events.first() {
                        eprintln!("  First event: '{}' at [{:.6}, {:.6})",
                                 event.value, event.part.begin.to_float(), event.part.end.to_float());
                    }
                }

                // If there's an event at this cycle position, use its value
                if let Some(event) = events.first() {
                    let s = event.value.as_str();

                    // Check for explicit rest
                    if s.trim() == "~" {
                        // Explicit rest: output 0.0 (silence)
                        current_value = 0.0;

                        // Update last_value to 0 so we know we're in rest state
                        if let Some(Some(SignalNode::Pattern { last_value: lv, .. })) =
                            self.nodes.get_mut(node_id.0)
                        {
                            *lv = 0.0;
                        }

                        // DEBUG: Log rests
                        if std::env::var("DEBUG_PATTERN").is_ok() && last_value != 0.0 {
                            eprintln!("Pattern '{}' at cycle {:.4}: REST (was {})",
                                     pattern_str, self.cycle_position, last_value);
                        }
                    } else if !s.is_empty() {
                        // Parse the event value - Pattern nodes are for NUMERIC values
                        // (frequencies, control values, etc.), not sample names

                        // Try numeric parsing first, then fall back to note names
                        // This ensures "1", "0", "440" etc are treated as numbers, not MIDI notes
                        use crate::pattern_tonal::{midi_to_freq, note_to_midi};
                        if let Ok(numeric_value) = s.parse::<f32>() {
                            current_value = numeric_value;
                        } else if let Some(midi) = note_to_midi(s) {
                            // Fall back to note name parsing (e.g., "c4", "a4", "cs4")
                            current_value = midi_to_freq(midi) as f32;
                        } else {
                            // If neither works, keep last value
                            current_value = last_value;
                        }

                        // DEBUG: Log pattern value changes
                        if std::env::var("DEBUG_PATTERN").is_ok() && current_value != last_value {
                            eprintln!("Pattern '{}' at cycle {:.4}: value changed {} -> {} (event: '{}')",
                                     pattern_str, self.cycle_position, last_value, current_value, s);
                        }

                        // Update last_value for next time
                        if let Some(Some(SignalNode::Pattern { last_value: lv, .. })) =
                            self.nodes.get_mut(node_id.0)
                        {
                            *lv = current_value;
                        }
                    }
                }

                current_value
            }

            SignalNode::CycleTrigger { last_cycle, pulse_width } => {
                let current_cycle = self.cycle_position.floor() as i32;
                let cycle_fraction = self.cycle_position - self.cycle_position.floor();
                let pulse_duration = pulse_width / self.cps as f32; // Convert pulse width to cycles

                // Output 1.0 if we're within the pulse duration at the start of a new cycle
                // Output 0.0 otherwise
                let output = if cycle_fraction < pulse_duration as f64 {
                    1.0
                } else {
                    0.0
                };

                // Update last_cycle for state tracking (not currently used but good to have)
                if let Some(Some(SignalNode::CycleTrigger { last_cycle: lc, .. })) =
                    self.nodes.get_mut(node_id.0)
                {
                    *lc = current_cycle;
                }

                output
            }

            SignalNode::Sample {
                pattern_str,
                pattern,
                last_trigger_time,
                last_cycle,
                playback_positions: _,
                gain,
                pan,
                speed,
                cut_group,
                n,
                note,
                attack,
                release,
                envelope_type,
            } => {
                // DEBUG: Log Sample node evaluation
                if std::env::var("DEBUG_SAMPLE_EVENTS").is_ok() && self.sample_count < 100 {
                    eprintln!("Evaluating Sample node '{}' at sample {}, cycle_pos={:.6}",
                             pattern_str, self.sample_count, self.cycle_position);
                }

                // Query pattern for events in the current cycle
                // Use full-cycle window to ensure transforms like degrade see all events
                // The event deduplication logic below prevents re-triggering
                let current_cycle_start = self.cycle_position.floor();
                let state = State {
                    span: TimeSpan::new(
                        Fraction::from_float(current_cycle_start),
                        Fraction::from_float(current_cycle_start + 1.0),
                    ),
                    controls: HashMap::new(),
                };
                let events = pattern.query(&state);

                // Check if we've crossed into a new cycle
                let current_cycle = self.cycle_position.floor() as i32;
                let cycle_changed = current_cycle != last_cycle;

                // Get the last EVENT start time we triggered
                // DON'T reset on cycle boundaries - events can span across cycles
                let mut last_event_start = if let Some(Some(SignalNode::Sample {
                    last_trigger_time: lt,
                    ..
                })) = self.nodes.get(node_id.0)
                {
                    *lt as f64
                } else {
                    -1.0
                };

                // NOTE: We used to reset last_event_start on cycle boundaries,
                // but this caused duplicate triggers for events that span cycles
                // (e.g., "bd ~bass bd ~bass" $ slow 3 would trigger ~bass twice)
                // The absolute event start time is sufficient for deduplication

                // Track the latest event start time we trigger in this sample
                let mut latest_triggered_start = last_event_start;

                // DEBUG: Log event processing
                if std::env::var("DEBUG_SAMPLE_EVENTS").is_ok() && !events.is_empty() {
                    eprintln!("Sample node at cycle {:.3}: {} events", self.cycle_position, events.len());
                }

                // Trigger voices for ALL new events
                // An event should be triggered if its START is after the last event we triggered
                for event in events.iter() {
                    let sample_name = event.value.trim();

                    // Skip rests
                    if sample_name == "~" || sample_name.is_empty() {
                        continue;
                    }

                    // Check for bus trigger prefix (~busname)
                    let is_bus_trigger = sample_name.starts_with('~');
                    let actual_name = if is_bus_trigger {
                        &sample_name[1..] // Strip ~ prefix
                    } else {
                        sample_name
                    };

                    // Get the event start time (absolute cycle position)
                    let event_start_abs = if let Some(whole) = &event.whole {
                        whole.begin.to_float()
                    } else {
                        event.part.begin.to_float()
                    };

                    // Only trigger events that:
                    // 1. Start AFTER the last event we triggered (prevent re-triggering)
                    // 2. Start at or before the current cycle position (event has "arrived")
                    // Use tiny epsilon for floating-point comparison (1 microsecond in cycle time)
                    let epsilon = 1e-6;
                    let event_is_new = event_start_abs > last_event_start + epsilon
                        && event_start_abs <= self.cycle_position + epsilon;

                    // DEBUG: Log event evaluation
                    if std::env::var("DEBUG_SAMPLE_EVENTS").is_ok() && self.sample_count < 20 {
                        eprintln!("  Event '{}' at {:.6}: event_is_new={} (last={:.6}, current={:.6})",
                                 sample_name, event_start_abs, event_is_new, last_event_start, self.cycle_position);
                    }

                    if event_is_new {
                        // DEBUG: Log triggered events
                        if std::env::var("DEBUG_SAMPLE_EVENTS").is_ok() {
                            eprintln!("  Triggering: '{}' at {:.3} (cycle_pos={:.3})",
                                     sample_name, event_start_abs, self.cycle_position);
                        }

                        // Evaluate DSP parameters at THIS EVENT'S start time
                        // This ensures each event gets its own parameter values from the pattern
                        let gain_val = self
                            .eval_signal_at_time(&gain, event_start_abs)
                            .max(0.0)
                            .min(10.0);
                        let pan_val = self
                            .eval_signal_at_time(&pan, event_start_abs)
                            .clamp(-1.0, 1.0);
                        let speed_val = self
                            .eval_signal_at_time(&speed, event_start_abs)
                            .max(0.01)
                            .min(10.0);
                        let cut_group_val = self.eval_signal_at_time(&cut_group, event_start_abs);
                        let cut_group_opt = if cut_group_val > 0.0 {
                            Some(cut_group_val as u32)
                        } else {
                            None
                        };

                        // Evaluate n modifier for sample number selection
                        let n_val = self.eval_signal_at_time(&n, event_start_abs);
                        let n_index = n_val.round().max(0.0) as usize;

                        // Modify sample name with n index if n > 0
                        // e.g., "bd" with n=2 becomes "bd:2"
                        let final_sample_name = if n_index > 0 {
                            format!("{}:{}", actual_name, n_index)
                        } else {
                            actual_name.to_string()
                        };

                        // Evaluate note modifier for pitch shifting
                        // Note is in semitones: 0 = original, 12 = octave up, -12 = octave down
                        let note_val = self.eval_signal_at_time(&note, event_start_abs);

                        // Calculate pitch shift: speed = original_speed * 2^(semitones/12)
                        let pitch_shift_multiplier = if note_val != 0.0 {
                            2.0_f32.powf(note_val / 12.0)
                        } else {
                            1.0
                        };

                        // Apply pitch shift to speed
                        let final_speed = speed_val * pitch_shift_multiplier;

                        // Evaluate envelope parameters
                        let attack_val = self
                            .eval_signal_at_time(&attack, event_start_abs)
                            .max(0.0)
                            .min(10.0); // Attack time in seconds
                        let release_val = self
                            .eval_signal_at_time(&release, event_start_abs)
                            .max(0.0)
                            .min(10.0); // Release time in seconds

                        // CRITICAL FIX: When attack=0 and release=0 (default), don't apply
                        // a short envelope that cuts off samples. Instead use sensible defaults
                        // that let samples play through naturally.
                        let (final_attack, final_release) =
                            if attack_val == 0.0 && release_val == 0.0 {
                                // No envelope requested: use anti-click attack and long release
                                // to let the sample play through completely
                                (0.005, 10.0) // 5ms attack (anti-click), 10s release (longer than any sample)
                            } else {
                                // Explicit envelope requested: use the values as-is
                                (attack_val, release_val)
                            };

                        // DEBUG: Print cut group info
                        if std::env::var("DEBUG_CUT_GROUPS").is_ok() {
                            eprintln!("Triggering {} at cycle {:.3}, cut_group_val={:.1}, cut_group_opt={:?}",
                                final_sample_name, event_start_abs, cut_group_val, cut_group_opt);
                        }

                        // Handle bus triggering vs regular sample loading
                        if is_bus_trigger {
                            // Look up the bus
                            if let Some(bus_node_id) = self.buses.get(actual_name).copied() {
                                // Calculate event duration from pattern
                                let event_duration = if let Some(whole) = &event.whole {
                                    whole.end.to_float() - whole.begin.to_float()
                                } else {
                                    event.part.end.to_float() - event.part.begin.to_float()
                                };

                                // Convert duration to samples (duration is in cycles)
                                let duration_samples =
                                    (event_duration * self.sample_rate as f64 * self.cps as f64)
                                        as usize;
                                let duration_samples =
                                    duration_samples.max(1).min(self.sample_rate as usize * 2); // Cap at 2 seconds

                                // Create synthetic sample buffer by evaluating bus signal
                                // IMPORTANT: Clear cache between each sample to get fresh oscillator values
                                let mut synthetic_buffer = Vec::with_capacity(duration_samples);
                                for _ in 0..duration_samples {
                                    self.value_cache.clear();
                                    let sample_value = self.eval_node(&bus_node_id);
                                    synthetic_buffer.push(sample_value);
                                }

                                // Trigger voice with synthetic buffer using appropriate envelope type
                                match envelope_type {
                                    Some(RuntimeEnvelopeType::Percussion) | None => {
                                        self.voice_manager.borrow_mut().trigger_sample_with_envelope(
                                            std::sync::Arc::new(synthetic_buffer),
                                            gain_val,
                                            pan_val,
                                            final_speed,
                                            cut_group_opt,
                                            final_attack,
                                            final_release,
                                        );
                                    }
                                    Some(RuntimeEnvelopeType::ADSR { ref decay, ref sustain }) => {
                                        let decay_val = self.eval_signal_at_time(decay, event_start_abs).max(0.001);
                                        let sustain_val = self.eval_signal_at_time(sustain, event_start_abs).clamp(0.0, 1.0);
                                        self.voice_manager.borrow_mut().trigger_sample_with_adsr(
                                            std::sync::Arc::new(synthetic_buffer),
                                            gain_val,
                                            pan_val,
                                            final_speed,
                                            cut_group_opt,
                                            final_attack,
                                            decay_val,
                                            sustain_val,
                                            final_release,
                                        );
                                    }
                                    Some(RuntimeEnvelopeType::Segments { ref levels, ref times }) => {
                                        self.voice_manager.borrow_mut().trigger_sample_with_segments(
                                            std::sync::Arc::new(synthetic_buffer),
                                            gain_val,
                                            pan_val,
                                            final_speed,
                                            cut_group_opt,
                                            levels.clone(),
                                            times.clone(),
                                        );
                                    }
                                    Some(RuntimeEnvelopeType::Curve { ref start, ref end, ref duration, ref curve }) => {
                                        let start_val = self.eval_signal_at_time(start, event_start_abs);
                                        let end_val = self.eval_signal_at_time(end, event_start_abs);
                                        let duration_val = self.eval_signal_at_time(duration, event_start_abs).max(0.001);
                                        let curve_val = self.eval_signal_at_time(curve, event_start_abs);
                                        self.voice_manager.borrow_mut().trigger_sample_with_curve(
                                            std::sync::Arc::new(synthetic_buffer),
                                            gain_val,
                                            pan_val,
                                            final_speed,
                                            cut_group_opt,
                                            start_val,
                                            end_val,
                                            duration_val,
                                            curve_val,
                                        );
                                    }
                                }

                                // Track trigger time
                                if event_start_abs > latest_triggered_start {
                                    latest_triggered_start = event_start_abs;
                                }
                            } else {
                                eprintln!("Warning: Bus '{}' not found for trigger", actual_name);
                            }
                        } else {
                            // Regular sample loading
                            let sample_data_opt = self.sample_bank.borrow_mut().get_sample(&final_sample_name);
                            // DEBUG: Log sample loading
                            if std::env::var("DEBUG_SAMPLE_EVENTS").is_ok() && self.sample_count < 20 {
                                eprintln!("  Sample '{}' loaded: {}", final_sample_name, sample_data_opt.is_some());
                            }
                            if let Some(sample_data) = sample_data_opt {
                                // Trigger voice using appropriate envelope type
                                match envelope_type {
                                    Some(RuntimeEnvelopeType::Percussion) | None => {
                                        self.voice_manager.borrow_mut().trigger_sample_with_envelope(
                                            sample_data,
                                            gain_val,
                                            pan_val,
                                            final_speed,
                                            cut_group_opt,
                                            final_attack,
                                            final_release,
                                        );
                                    }
                                    Some(RuntimeEnvelopeType::ADSR { ref decay, ref sustain }) => {
                                        let decay_val = self.eval_signal_at_time(decay, event_start_abs).max(0.001);
                                        let sustain_val = self.eval_signal_at_time(sustain, event_start_abs).clamp(0.0, 1.0);
                                        self.voice_manager.borrow_mut().trigger_sample_with_adsr(
                                            sample_data,
                                            gain_val,
                                            pan_val,
                                            final_speed,
                                            cut_group_opt,
                                            final_attack,
                                            decay_val,
                                            sustain_val,
                                            final_release,
                                        );
                                    }
                                    Some(RuntimeEnvelopeType::Segments { ref levels, ref times }) => {
                                        self.voice_manager.borrow_mut().trigger_sample_with_segments(
                                            sample_data,
                                            gain_val,
                                            pan_val,
                                            final_speed,
                                            cut_group_opt,
                                            levels.clone(),
                                            times.clone(),
                                        );
                                    }
                                    Some(RuntimeEnvelopeType::Curve { ref start, ref end, ref duration, ref curve }) => {
                                        let start_val = self.eval_signal_at_time(start, event_start_abs);
                                        let end_val = self.eval_signal_at_time(end, event_start_abs);
                                        let duration_val = self.eval_signal_at_time(duration, event_start_abs).max(0.001);
                                        let curve_val = self.eval_signal_at_time(curve, event_start_abs);
                                        self.voice_manager.borrow_mut().trigger_sample_with_curve(
                                            sample_data,
                                            gain_val,
                                            pan_val,
                                            final_speed,
                                            cut_group_opt,
                                            start_val,
                                            end_val,
                                            duration_val,
                                            curve_val,
                                        );
                                    }
                                }

                                // Track this as the latest event we've triggered
                                if event_start_abs > latest_triggered_start {
                                    latest_triggered_start = event_start_abs;
                                }
                            }
                        }
                    }
                }

                // Update last_trigger_time and last_cycle
                // This ensures we don't re-trigger the same events
                if latest_triggered_start > last_event_start || cycle_changed {
                    // DEBUG: Log update
                    if std::env::var("DEBUG_SAMPLE_EVENTS").is_ok() && self.sample_count < 20 {
                        eprintln!("  Updating last_trigger_time: {:.6} -> {:.6} (cycle_changed={})",
                                 last_event_start, latest_triggered_start, cycle_changed);
                    }
                    if let Some(Some(SignalNode::Sample {
                        last_trigger_time: lt,
                        last_cycle: lc,
                        ..
                    })) = self.nodes.get_mut(node_id.0)
                    {
                        *lt = latest_triggered_start as f32;
                        *lc = current_cycle;
                    }
                }

                // Sample nodes trigger voices AND output the voice audio
                // This allows them to work standalone or be routed through effects
                self.voice_manager.borrow_mut().process()
            }

            SignalNode::SynthPattern {
                pattern,
                last_trigger_time,
                waveform,
                attack,
                decay,
                sustain,
                release,
                gain,
                pan,
                ..
            } => {
                use crate::pattern_tonal::{midi_to_freq, note_to_midi};
                use crate::synth_voice_manager::{ADSRParams, SynthWaveform};

                // Evaluate DSP parameters
                let gain_val = self.eval_signal(&gain).max(0.0).min(10.0);
                let pan_val = self.eval_signal(&pan).clamp(-1.0, 1.0);

                // Query pattern for note events
                let sample_width = 1.0 / self.sample_rate as f64 / self.cps as f64;
                let state = State {
                    span: TimeSpan::new(
                        Fraction::from_float(self.cycle_position),
                        Fraction::from_float(self.cycle_position + sample_width),
                    ),
                    controls: HashMap::new(),
                };
                let events = pattern.query(&state);

                // Get last event start time
                let last_event_start = if let Some(Some(SignalNode::SynthPattern {
                    last_trigger_time: lt,
                    ..
                })) = self.nodes.get(node_id.0)
                {
                    *lt as f64
                } else {
                    -1.0
                };

                let mut latest_triggered_start = last_event_start;

                // Trigger synth voices for new note events
                for event in events.iter() {
                    let note_name = event.value.trim();

                    // Skip rests
                    if note_name == "~" || note_name.is_empty() {
                        continue;
                    }

                    // Get event start time
                    let event_start_abs = if let Some(whole) = &event.whole {
                        whole.begin.to_float()
                    } else {
                        event.part.begin.to_float()
                    };

                    // Only trigger NEW events
                    let tolerance = sample_width * 0.001;
                    let event_is_new = event_start_abs > last_event_start + tolerance;

                    if event_is_new {
                        // Parse note name to frequency
                        let frequency = if let Ok(numeric) = note_name.parse::<f32>() {
                            numeric
                        } else if let Some(midi) = note_to_midi(note_name) {
                            midi_to_freq(midi) as f32
                        } else {
                            440.0 // Default to A4
                        };

                        // Convert Waveform to SynthWaveform
                        let synth_waveform = match waveform {
                            Waveform::Sine => SynthWaveform::Sine,
                            Waveform::Saw => SynthWaveform::Saw,
                            Waveform::Square => SynthWaveform::Square,
                            Waveform::Triangle => SynthWaveform::Triangle,
                        };

                        // ADSR parameters
                        let adsr = ADSRParams {
                            attack,
                            decay,
                            sustain,
                            release,
                        };

                        // TRIGGER SYNTH VOICE (NOTE ON!)
                        self.synth_voice_manager.borrow_mut().trigger_note(
                            frequency,
                            synth_waveform,
                            adsr,
                            gain_val,
                            pan_val,
                        );

                        // Track latest event
                        if event_start_abs > latest_triggered_start {
                            latest_triggered_start = event_start_abs;
                        }
                    }
                }

                // Update last_trigger_time
                if latest_triggered_start > last_event_start {
                    if let Some(Some(SignalNode::SynthPattern {
                        last_trigger_time: lt,
                        ..
                    })) = self.nodes.get_mut(node_id.0)
                    {
                        *lt = latest_triggered_start as f32;
                    }
                }

                // Output mixed audio from all synth voices
                self.synth_voice_manager.borrow_mut().process()
            }

            SignalNode::VoiceOutput => {
                // Output the mixed audio from all active voices
                // This is the same as what Sample nodes output,
                // provided as an explicit node for clarity
                self.voice_manager.borrow_mut().process()
            }

            SignalNode::ScaleQuantize {
                pattern,
                scale_name,
                root_note,
                last_value,
                ..
            } => {
                use crate::pattern_tonal::{midi_to_freq, SCALES};

                // Query pattern for events at current cycle position
                let sample_width = 1.0 / self.sample_rate as f64 / self.cps as f64;
                let state = State {
                    span: TimeSpan::new(
                        Fraction::from_float(self.cycle_position),
                        Fraction::from_float(self.cycle_position + sample_width),
                    ),
                    controls: HashMap::new(),
                };

                let events = pattern.query(&state);
                let mut current_value = last_value; // Default to last value

                // If there's an event at this cycle position, quantize it to the scale
                if let Some(event) = events.first() {
                    if event.value.trim() != "~" && !event.value.is_empty() {
                        // Parse scale degree (e.g., "0", "1", "2", "3"...)
                        if let Ok(scale_degree) = event.value.parse::<i32>() {
                            // Get scale intervals
                            if let Some(scale_intervals) = SCALES.get(scale_name.as_str()) {
                                // Calculate octave and degree within scale
                                let octave = scale_degree / scale_intervals.len() as i32;
                                let degree = scale_degree.rem_euclid(scale_intervals.len() as i32);

                                // Get MIDI note
                                let interval = scale_intervals[degree as usize];
                                let midi_note = root_note as i32 + octave * 12 + interval;

                                // Convert to frequency
                                current_value = midi_to_freq(midi_note.clamp(0, 127) as u8) as f32;

                                // Update last_value for next time
                                if let Some(Some(SignalNode::ScaleQuantize {
                                    last_value: lv,
                                    ..
                                })) = self.nodes.get_mut(node_id.0)
                                {
                                    *lv = current_value;
                                }
                            }
                        }
                    }
                }

                current_value
            }

            SignalNode::Noise { seed } => {
                // Simple white noise using linear congruential generator
                let seed_val = seed;
                let next = (seed_val.wrapping_mul(1103515245).wrapping_add(12345)) % (1 << 31);

                // Update seed for next sample
                if let Some(Some(SignalNode::Noise { seed: s })) = self.nodes.get_mut(node_id.0) {
                    *s = next;
                }

                (next as f32 / (1 << 30) as f32) - 1.0
            }

            SignalNode::HighPass {
                input, cutoff, q, ..
            } => {
                let input_val = self.eval_signal(&input);
                let fc = self.eval_signal(&cutoff).max(20.0).min(20000.0);
                let q_val = self.eval_signal(&q).max(0.5).min(20.0);

                // State variable filter (Chamberlin) - high pass output
                let f = 2.0 * (PI * fc / self.sample_rate).sin();
                let damp = 1.0 / q_val;

                // Get state
                let (mut low, mut band, mut high) =
                    if let Some(Some(SignalNode::HighPass { state, .. })) =
                        self.nodes.get(node_id.0)
                    {
                        (state.y1, state.x1, state.y2)
                    } else {
                        (0.0, 0.0, 0.0)
                    };

                // Process
                high = input_val - low - damp * band;
                band += f * high;
                low += f * band;

                // Update state
                if let Some(Some(SignalNode::HighPass { state, .. })) =
                    self.nodes.get_mut(node_id.0)
                {
                    state.y1 = low;
                    state.x1 = band;
                    state.y2 = high;
                }

                high // Output high-pass signal
            }

            SignalNode::BandPass {
                input, center, q, ..
            } => {
                let input_val = self.eval_signal(&input);
                let fc = self.eval_signal(&center).max(20.0).min(20000.0);
                let q_val = self.eval_signal(&q).max(0.5).min(20.0);

                // State variable filter (Chamberlin) - band pass output
                let f = 2.0 * (PI * fc / self.sample_rate).sin();
                let damp = 1.0 / q_val;

                // Get state
                let (mut low, mut band, mut high) =
                    if let Some(Some(SignalNode::BandPass { state, .. })) =
                        self.nodes.get(node_id.0)
                    {
                        (state.y1, state.x1, state.y2)
                    } else {
                        (0.0, 0.0, 0.0)
                    };

                // Process
                high = input_val - low - damp * band;
                band += f * high;
                low += f * band;

                // Update state
                if let Some(Some(SignalNode::BandPass { state, .. })) =
                    self.nodes.get_mut(node_id.0)
                {
                    state.y1 = low;
                    state.x1 = band;
                    state.y2 = high;
                }

                band // Output band-pass signal
            }

            SignalNode::Notch {
                input, center, q, ..
            } => {
                let input_val = self.eval_signal(&input);
                let fc = self.eval_signal(&center).max(20.0).min(20000.0);
                let q_val = self.eval_signal(&q).max(0.5).min(20.0);

                // State variable filter (Chamberlin) - notch is low + high
                let f = 2.0 * (PI * fc / self.sample_rate).sin();
                let damp = 1.0 / q_val;

                // Get state
                let (mut low, mut band, mut high) =
                    if let Some(Some(SignalNode::Notch { state, .. })) =
                        self.nodes.get(node_id.0)
                    {
                        (state.y1, state.x1, state.y2)
                    } else {
                        (0.0, 0.0, 0.0)
                    };

                // Process
                high = input_val - low - damp * band;
                band += f * high;
                low += f * band;

                // Update state
                if let Some(Some(SignalNode::Notch { state, .. })) =
                    self.nodes.get_mut(node_id.0)
                {
                    state.y1 = low;
                    state.x1 = band;
                    state.y2 = high;
                }

                low + high // Output notch (low + high = everything except band)
            }

            SignalNode::Comb {
                input,
                frequency,
                feedback,
                buffer,
                write_pos,
            } => {
                let input_val = self.eval_signal(&input);
                let freq = self.eval_signal(&frequency).max(20.0).min(20000.0);
                let fb = self.eval_signal(&feedback).clamp(0.0, 0.99);

                // Convert frequency to delay time in samples
                let delay_samples = (self.sample_rate / freq).round() as usize;
                let delay_samples = delay_samples.clamp(1, buffer.len() - 1);

                // Calculate read position (write_pos - delay_samples, wrapped)
                let read_pos = (write_pos + buffer.len() - delay_samples) % buffer.len();
                let delayed = buffer[read_pos];

                // Comb filter: output = input + feedback * delayed_output
                let output = input_val + fb * delayed;

                // Update buffer and write position
                if let Some(Some(SignalNode::Comb {
                    buffer: buf,
                    write_pos: idx,
                    ..
                })) = self.nodes.get_mut(node_id.0)
                {
                    buf[*idx] = output;
                    *idx = (*idx + 1) % buf.len();
                }

                output
            }

            SignalNode::MoogLadder {
                input,
                cutoff,
                resonance,
                state,
            } => {
                let input_val = self.eval_signal(&input);
                let fc = self.eval_signal(&cutoff).clamp(20.0, 20000.0);
                let res = self.eval_signal(&resonance).clamp(0.0, 1.0);

                // Calculate cutoff coefficient (g) from frequency
                // g = tan(π * fc / sr) / (1 + tan(π * fc / sr))
                let g = (PI * fc / self.sample_rate).tan();
                let g_normalized = g / (1.0 + g);

                // Resonance scaling (0-4 is typical, higher = more resonance)
                let resonance_amt = res * 4.0;

                // Get current state
                let (s1, s2, s3, s4) = if let Some(Some(SignalNode::MoogLadder { state, .. })) =
                    self.nodes.get(node_id.0)
                {
                    (state.stage1, state.stage2, state.stage3, state.stage4)
                } else {
                    (0.0, 0.0, 0.0, 0.0)
                };

                // Feedback from output to input (raw, no saturation for better level)
                let input_with_fb = input_val - resonance_amt * s4;

                // Four cascaded 1-pole filters (linear stages for better response)
                let stage1_new = s1 + g_normalized * (input_with_fb - s1);
                let stage2_new = s2 + g_normalized * (stage1_new - s2);
                let stage3_new = s3 + g_normalized * (stage2_new - s3);
                let stage4_new = s4 + g_normalized * (stage3_new - s4);

                // Update state
                if let Some(Some(SignalNode::MoogLadder { state, .. })) =
                    self.nodes.get_mut(node_id.0)
                {
                    state.stage1 = stage1_new;
                    state.stage2 = stage2_new;
                    state.stage3 = stage3_new;
                    state.stage4 = stage4_new;
                }

                // Output from final stage
                stage4_new
            }

            SignalNode::ParametricEQ {
                input,
                low_freq,
                low_gain,
                low_q,
                mid_freq,
                mid_gain,
                mid_q,
                high_freq,
                high_gain,
                high_q,
                state,
            } => {
                use std::f32::consts::PI;

                let input_val = self.eval_signal(&input);
                let sample_rate = self.sample_rate; // Extract to avoid borrow issues

                // Helper function to apply peaking filter
                let apply_peaking_filter = |input: f32,
                                            fc: f32,
                                            gain_db: f32,
                                            q: f32,
                                            filter_state: &FilterState|
                 -> (f32, FilterState) {
                    // Clamp parameters
                    let fc = fc.clamp(20.0, 20000.0);
                    let gain_db = gain_db.clamp(-20.0, 20.0);
                    let q = q.clamp(0.1, 10.0);

                    // If gain is near zero, bypass filter
                    if gain_db.abs() < 0.1 {
                        return (input, filter_state.clone());
                    }

                    // Calculate biquad coefficients for peaking filter
                    let a = 10.0_f32.powf(gain_db / 40.0); // Amplitude
                    let omega = 2.0 * PI * fc / sample_rate;
                    let alpha = omega.sin() / (2.0 * q);
                    let cos_omega = omega.cos();

                    let b0 = 1.0 + alpha * a;
                    let b1 = -2.0 * cos_omega;
                    let b2 = 1.0 - alpha * a;
                    let a0 = 1.0 + alpha / a;
                    let a1 = -2.0 * cos_omega;
                    let a2 = 1.0 - alpha / a;

                    // Normalize coefficients
                    let b0_norm = b0 / a0;
                    let b1_norm = b1 / a0;
                    let b2_norm = b2 / a0;
                    let a1_norm = a1 / a0;
                    let a2_norm = a2 / a0;

                    // Apply biquad filter (Direct Form II)
                    let output = b0_norm * input + filter_state.x1;

                    let mut new_state = filter_state.clone();
                    new_state.x1 = b1_norm * input - a1_norm * output + filter_state.x2;
                    new_state.x2 = b2_norm * input - a2_norm * output;

                    (output, new_state)
                };

                // Get state
                let (low_state, mid_state, high_state) =
                    if let Some(Some(SignalNode::ParametricEQ { state, .. })) =
                        self.nodes.get(node_id.0)
                    {
                        (
                            state.low_band.clone(),
                            state.mid_band.clone(),
                            state.high_band.clone(),
                        )
                    } else {
                        (
                            FilterState::default(),
                            FilterState::default(),
                            FilterState::default(),
                        )
                    };

                // Evaluate parameters
                let low_f = self.eval_signal(&low_freq);
                let low_g = self.eval_signal(&low_gain);
                let low_q_val = self.eval_signal(&low_q);

                let mid_f = self.eval_signal(&mid_freq);
                let mid_g = self.eval_signal(&mid_gain);
                let mid_q_val = self.eval_signal(&mid_q);

                let high_f = self.eval_signal(&high_freq);
                let high_g = self.eval_signal(&high_gain);
                let high_q_val = self.eval_signal(&high_q);

                // Apply all three bands in series
                let (after_low, new_low_state) =
                    apply_peaking_filter(input_val, low_f, low_g, low_q_val, &low_state);
                let (after_mid, new_mid_state) =
                    apply_peaking_filter(after_low, mid_f, mid_g, mid_q_val, &mid_state);
                let (output, new_high_state) =
                    apply_peaking_filter(after_mid, high_f, high_g, high_q_val, &high_state);

                // Update state
                if let Some(Some(SignalNode::ParametricEQ { state, .. })) =
                    self.nodes.get_mut(node_id.0)
                {
                    state.low_band = new_low_state;
                    state.mid_band = new_mid_state;
                    state.high_band = new_high_state;
                }

                output
            }

            SignalNode::Envelope {
                input,
                trigger,
                attack,
                decay,
                sustain,
                release,
                state,
            } => {
                let input_val = self.eval_signal(&input);
                let trig = self.eval_signal(&trigger);

                // Clone state to work with it
                let mut env_state = state.clone();

                // Check for trigger
                if trig > 0.5 && matches!(env_state.phase, EnvPhase::Idle | EnvPhase::Release) {
                    env_state.phase = EnvPhase::Attack;
                    env_state.time_in_phase = 0.0;
                } else if trig <= 0.5
                    && matches!(
                        env_state.phase,
                        EnvPhase::Attack | EnvPhase::Decay | EnvPhase::Sustain
                    )
                {
                    // Store current level before entering release phase
                    env_state.release_start_level = env_state.level;
                    env_state.phase = EnvPhase::Release;
                    env_state.time_in_phase = 0.0;
                }

                // Process envelope
                let dt = 1.0 / self.sample_rate;
                env_state.time_in_phase += dt;

                match env_state.phase {
                    EnvPhase::Attack => {
                        if attack > 0.0 {
                            env_state.level = env_state.time_in_phase / attack;
                            if env_state.level >= 1.0 {
                                env_state.level = 1.0;
                                env_state.phase = EnvPhase::Decay;
                                env_state.time_in_phase = 0.0;
                            }
                        } else {
                            env_state.level = 1.0;
                            env_state.phase = EnvPhase::Decay;
                            env_state.time_in_phase = 0.0;
                        }
                    }
                    EnvPhase::Decay => {
                        if decay > 0.0 {
                            env_state.level =
                                1.0 - (1.0 - sustain) * (env_state.time_in_phase / decay);
                            if env_state.level <= sustain {
                                env_state.level = sustain;
                                env_state.phase = EnvPhase::Sustain;
                                env_state.time_in_phase = 0.0;
                            }
                        } else {
                            env_state.level = sustain;
                            env_state.phase = EnvPhase::Sustain;
                            env_state.time_in_phase = 0.0;
                        }
                    }
                    EnvPhase::Sustain => {
                        env_state.level = sustain;
                    }
                    EnvPhase::Release => {
                        if release > 0.0 {
                            // Linear decay from release_start_level to 0 over release time
                            let progress = (env_state.time_in_phase / release).min(1.0);
                            env_state.level = env_state.release_start_level * (1.0 - progress);

                            if progress >= 1.0 {
                                env_state.level = 0.0;
                                env_state.phase = EnvPhase::Idle;
                            }
                        } else {
                            env_state.level = 0.0;
                            env_state.phase = EnvPhase::Idle;
                        }
                    }
                    EnvPhase::Idle => {
                        env_state.level = 0.0;
                    }
                }

                // Update state in node
                if let Some(Some(SignalNode::Envelope { state: s, .. })) =
                    self.nodes.get_mut(node_id.0)
                {
                    *s = env_state.clone();
                }

                input_val * env_state.level
            }

            SignalNode::ADSR {
                attack,
                decay,
                sustain,
                release,
                state,
            } => {
                // Evaluate modulatable parameters
                let attack_time = self.eval_signal(&attack).max(0.001); // Min 1ms
                let decay_time = self.eval_signal(&decay).max(0.001);
                let sustain_level = self.eval_signal(&sustain).clamp(0.0, 1.0);
                let release_time = self.eval_signal(&release).max(0.001);

                let mut adsr_state = state.clone();

                // Calculate position within current cycle (0.0 to 1.0)
                let cycle_duration = 1.0 / self.cps;
                let cycle_pos = (self.cycle_position % 1.0) as f32;
                let time_in_cycle = cycle_pos * cycle_duration;

                // Calculate phase boundaries (in seconds)
                let attack_end = attack_time;
                let decay_end = attack_end + decay_time;
                let release_start = cycle_duration - release_time;

                // Determine phase and calculate envelope value
                let level = if time_in_cycle < attack_end {
                    // Attack phase: rise from 0 to 1
                    if attack_time > 0.0 {
                        time_in_cycle / attack_time
                    } else {
                        1.0
                    }
                } else if time_in_cycle < decay_end {
                    // Decay phase: fall from 1 to sustain level
                    let decay_progress = (time_in_cycle - attack_end) / decay_time;
                    1.0 - (1.0 - sustain_level) * decay_progress
                } else if time_in_cycle < release_start {
                    // Sustain phase: hold at sustain level
                    sustain_level
                } else {
                    // Release phase: fall from sustain level to 0
                    let release_progress = (time_in_cycle - release_start) / release_time;
                    sustain_level * (1.0 - release_progress)
                };

                adsr_state.level = level.clamp(0.0, 1.0);
                adsr_state.cycle_pos = cycle_pos;

                // Update state in graph
                if let Some(Some(SignalNode::ADSR { state: s, .. })) =
                    self.nodes.get_mut(node_id.0)
                {
                    *s = adsr_state.clone();
                }

                adsr_state.level
            }

            SignalNode::AD {
                attack,
                decay,
                state,
            } => {
                // Evaluate modulatable parameters
                let attack_time = self.eval_signal(&attack).max(0.001); // Min 1ms
                let decay_time = self.eval_signal(&decay).max(0.001);

                let mut ad_state = state.clone();

                // Calculate position within current cycle (0.0 to 1.0)
                let cycle_duration = 1.0 / self.cps;
                let cycle_pos = (self.cycle_position % 1.0) as f32;
                let time_in_cycle = cycle_pos * cycle_duration;

                // Calculate phase boundaries (in seconds)
                let attack_end = attack_time;
                let decay_end = attack_end + decay_time;

                // Determine phase and calculate envelope value
                let level = if time_in_cycle < attack_end {
                    // Attack phase: rise from 0 to 1
                    if attack_time > 0.0 {
                        time_in_cycle / attack_time
                    } else {
                        1.0
                    }
                } else if time_in_cycle < decay_end {
                    // Decay phase: fall from 1 to 0
                    let decay_progress = (time_in_cycle - attack_end) / decay_time;
                    1.0 - decay_progress
                } else {
                    // After decay: silent
                    0.0
                };

                ad_state.level = level.clamp(0.0, 1.0);
                ad_state.cycle_pos = cycle_pos;

                // Update state in graph
                if let Some(Some(SignalNode::AD { state: s, .. })) =
                    self.nodes.get_mut(node_id.0)
                {
                    *s = ad_state.clone();
                }

                ad_state.level
            }

            SignalNode::Line { start, end } => {
                // Evaluate start and end values (supports pattern modulation!)
                let start_val = self.eval_signal(&start);
                let end_val = self.eval_signal(&end);

                // Calculate position within current cycle (0.0 to 1.0)
                let cycle_pos = (self.cycle_position % 1.0) as f32;

                // Linear interpolation from start to end
                let value = start_val + (end_val - start_val) * cycle_pos;

                value
            }

            SignalNode::Curve {
                start,
                end,
                duration,
                curve,
                elapsed_time,
            } => {
                let start_val = self.eval_signal(&start);
                let end_val = self.eval_signal(&end);
                let duration_val = self.eval_signal(&duration).max(0.001); // Min 1ms
                let curve_val = self.eval_signal(&curve);

                let mut output_val = start_val;

                // Update elapsed time
                if let Some(Some(SignalNode::Curve {
                    elapsed_time: elapsed,
                    ..
                })) = self.nodes.get_mut(node_id.0)
                {
                    // Increment elapsed time
                    *elapsed += 1.0 / self.sample_rate;

                    // Calculate normalized time (0 to 1)
                    let t = (*elapsed / duration_val).min(1.0);

                    // Apply curve formula
                    let curved_t = if curve_val.abs() < 0.001 {
                        // Linear (curve ≈ 0)
                        t
                    } else {
                        // Exponential curve
                        // Formula: (exp(curve * t) - 1) / (exp(curve) - 1)
                        let exp_curve = curve_val.exp();
                        let exp_curve_t = (curve_val * t).exp();
                        (exp_curve_t - 1.0) / (exp_curve - 1.0)
                    };

                    // Interpolate between start and end
                    output_val = start_val + (end_val - start_val) * curved_t;
                }

                output_val
            }

            SignalNode::Segments {
                levels,
                times,
                current_segment,
                segment_elapsed,
                current_value,
            } => {
                let mut output_val = current_value.clone();

                // Update state in the graph
                if let Some(Some(SignalNode::Segments {
                    levels: seg_levels,
                    times: seg_times,
                    current_segment: seg_idx,
                    segment_elapsed: seg_elapsed,
                    current_value: seg_value,
                })) = self.nodes.get_mut(node_id.0)
                {
                    // Advance time
                    *seg_elapsed += 1.0 / self.sample_rate;

                    // Check if we're beyond the last segment
                    if *seg_idx >= seg_times.len() {
                        // Hold final level
                        output_val = if !seg_levels.is_empty() {
                            seg_levels[seg_levels.len() - 1]
                        } else {
                            0.0
                        };
                        *seg_value = output_val;
                    } else {
                        // Get current segment info
                        let segment_duration = seg_times[*seg_idx];
                        let start_level = if *seg_idx == 0 {
                            seg_levels[0]
                        } else {
                            seg_levels[*seg_idx]
                        };
                        let end_level = seg_levels[*seg_idx + 1];

                        // Calculate interpolation factor
                        let t = (*seg_elapsed / segment_duration).min(1.0);

                        // Linear interpolation
                        output_val = start_level + (end_level - start_level) * t;
                        *seg_value = output_val;

                        // Check if segment is complete
                        if *seg_elapsed >= segment_duration {
                            // Move to next segment
                            *seg_idx += 1;
                            *seg_elapsed = 0.0;
                        }
                    }
                }

                output_val
            }

            SignalNode::EnvelopePattern {
                input,
                pattern,
                last_trigger_time,
                last_cycle,
                attack,
                decay,
                sustain,
                release,
                state,
                ..
            } => {
                let input_val = self.eval_signal(&input);

                // Query pattern for trigger events
                let sample_width = 1.0 / self.sample_rate as f64 / self.cps as f64;
                let current_cycle = self.cycle_position.floor() as i32;

                let query_state = State {
                    span: TimeSpan::new(
                        Fraction::from_float(self.cycle_position),
                        Fraction::from_float(self.cycle_position + sample_width),
                    ),
                    controls: HashMap::new(),
                };
                let events = pattern.query(&query_state);

                // Get last event start time and cycle
                let (last_event_start, prev_cycle) =
                    if let Some(Some(SignalNode::EnvelopePattern {
                        last_trigger_time: lt,
                        last_cycle: lc,
                        ..
                    })) = self.nodes.get(node_id.0)
                    {
                        (*lt as f64, *lc)
                    } else {
                        (-1.0, -1)
                    };

                let mut env_state = state.clone();
                let mut latest_triggered_start = last_event_start;
                let mut trigger_active = false;

                // Check if cycle changed
                let cycle_changed = current_cycle != prev_cycle;

                // Check for new trigger events
                for event in events.iter() {
                    let note_name = event.value.trim();

                    // Skip rests
                    if note_name == "~" || note_name.is_empty() {
                        continue;
                    }

                    // Get event start time
                    let event_start_abs = if let Some(whole) = &event.whole {
                        whole.begin.to_float()
                    } else {
                        event.part.begin.to_float()
                    };

                    // We're inside an event (it spans the current position)
                    // This means we should keep the envelope active
                    trigger_active = true;

                    // Only update last_trigger_time for NEW events
                    let tolerance = sample_width * 0.001;
                    let event_is_new = event_start_abs > last_event_start + tolerance || cycle_changed;

                    if event_is_new && event_start_abs > latest_triggered_start {
                        latest_triggered_start = event_start_abs;
                    }
                }

                // Process envelope based on trigger
                if trigger_active
                    && matches!(env_state.phase, EnvPhase::Idle | EnvPhase::Release)
                {
                    // Start attack phase
                    env_state.phase = EnvPhase::Attack;
                    env_state.time_in_phase = 0.0;
                } else if !trigger_active
                    && matches!(
                        env_state.phase,
                        EnvPhase::Attack | EnvPhase::Decay | EnvPhase::Sustain
                    )
                {
                    // Enter release phase
                    env_state.release_start_level = env_state.level;
                    env_state.phase = EnvPhase::Release;
                    env_state.time_in_phase = 0.0;
                }

                // Advance envelope state
                let dt = 1.0 / self.sample_rate;
                env_state.time_in_phase += dt;

                match env_state.phase {
                    EnvPhase::Attack => {
                        if attack > 0.0 {
                            env_state.level = env_state.time_in_phase / attack;
                            if env_state.level >= 1.0 {
                                env_state.level = 1.0;
                                env_state.phase = EnvPhase::Decay;
                                env_state.time_in_phase = 0.0;
                            }
                        } else {
                            env_state.level = 1.0;
                            env_state.phase = EnvPhase::Decay;
                            env_state.time_in_phase = 0.0;
                        }
                    }
                    EnvPhase::Decay => {
                        if decay > 0.0 {
                            env_state.level =
                                1.0 - (1.0 - sustain) * (env_state.time_in_phase / decay);
                            if env_state.level <= sustain {
                                env_state.level = sustain;
                                env_state.phase = EnvPhase::Sustain;
                                env_state.time_in_phase = 0.0;
                            }
                        } else {
                            env_state.level = sustain;
                            env_state.phase = EnvPhase::Sustain;
                            env_state.time_in_phase = 0.0;
                        }
                    }
                    EnvPhase::Sustain => {
                        env_state.level = sustain;
                    }
                    EnvPhase::Release => {
                        if release > 0.0 {
                            let progress = (env_state.time_in_phase / release).min(1.0);
                            env_state.level = env_state.release_start_level * (1.0 - progress);

                            if progress >= 1.0 {
                                env_state.level = 0.0;
                                env_state.phase = EnvPhase::Idle;
                            }
                        } else {
                            env_state.level = 0.0;
                            env_state.phase = EnvPhase::Idle;
                        }
                    }
                    EnvPhase::Idle => {
                        env_state.level = 0.0;
                    }
                }

                // Update state in node
                if let Some(Some(SignalNode::EnvelopePattern {
                    state: s,
                    last_trigger_time: lt,
                    last_cycle: lc,
                    ..
                })) = self.nodes.get_mut(node_id.0)
                {
                    *s = env_state.clone();
                    *lt = latest_triggered_start as f32;
                    *lc = current_cycle;
                }

                // Output: input signal gated by envelope
                input_val * env_state.level
            }

            SignalNode::StructuredSignal {
                input,
                bool_pattern,
                last_trigger_time,
                last_cycle,
                attack,
                decay,
                sustain,
                release,
                state,
                ..
            } => {
                let input_val = self.eval_signal(&input);

                // Query boolean pattern for trigger events
                let sample_width = 1.0 / self.sample_rate as f64 / self.cps as f64;
                let current_cycle = self.cycle_position.floor() as i32;

                let query_state = State {
                    span: TimeSpan::new(
                        Fraction::from_float(self.cycle_position),
                        Fraction::from_float(self.cycle_position + sample_width),
                    ),
                    controls: HashMap::new(),
                };
                let events = bool_pattern.query(&query_state);

                // Get last event start time and cycle
                let (last_event_start, prev_cycle) =
                    if let Some(Some(SignalNode::StructuredSignal {
                        last_trigger_time: lt,
                        last_cycle: lc,
                        ..
                    })) = self.nodes.get(node_id.0)
                    {
                        (*lt as f64, *lc)
                    } else {
                        (-1.0, -1)
                    };

                let mut env_state = state.clone();
                let mut latest_triggered_start = last_event_start;
                let mut trigger_active = false;

                // Check if cycle changed
                let cycle_changed = current_cycle != prev_cycle;

                // Check for new trigger events (true values in boolean pattern)
                for event in events.iter() {
                    // Only trigger on "true" values
                    if !event.value {
                        continue;
                    }

                    // Get event start time
                    let event_start_abs = if let Some(whole) = &event.whole {
                        whole.begin.to_float()
                    } else {
                        event.part.begin.to_float()
                    };

                    // We're inside a "true" event - keep envelope active
                    trigger_active = true;

                    // Only update last_trigger_time for NEW events
                    let tolerance = sample_width * 0.001;
                    let event_is_new = event_start_abs > last_event_start + tolerance || cycle_changed;

                    if event_is_new && event_start_abs > latest_triggered_start {
                        latest_triggered_start = event_start_abs;
                    }
                }

                // Process envelope based on trigger
                if trigger_active
                    && matches!(env_state.phase, EnvPhase::Idle | EnvPhase::Release)
                {
                    // Start attack phase
                    env_state.phase = EnvPhase::Attack;
                    env_state.time_in_phase = 0.0;
                } else if !trigger_active
                    && matches!(
                        env_state.phase,
                        EnvPhase::Attack | EnvPhase::Decay | EnvPhase::Sustain
                    )
                {
                    // Enter release phase
                    env_state.release_start_level = env_state.level;
                    env_state.phase = EnvPhase::Release;
                    env_state.time_in_phase = 0.0;
                }

                // Advance envelope state
                let dt = 1.0 / self.sample_rate;
                env_state.time_in_phase += dt;

                match env_state.phase {
                    EnvPhase::Attack => {
                        if attack > 0.0 {
                            env_state.level = env_state.time_in_phase / attack;
                            if env_state.level >= 1.0 {
                                env_state.level = 1.0;
                                env_state.phase = EnvPhase::Decay;
                                env_state.time_in_phase = 0.0;
                            }
                        } else {
                            env_state.level = 1.0;
                            env_state.phase = EnvPhase::Decay;
                            env_state.time_in_phase = 0.0;
                        }
                    }
                    EnvPhase::Decay => {
                        if decay > 0.0 {
                            env_state.level =
                                1.0 - (1.0 - sustain) * (env_state.time_in_phase / decay);
                            if env_state.level <= sustain {
                                env_state.level = sustain;
                                env_state.phase = EnvPhase::Sustain;
                                env_state.time_in_phase = 0.0;
                            }
                        } else {
                            env_state.level = sustain;
                            env_state.phase = EnvPhase::Sustain;
                            env_state.time_in_phase = 0.0;
                        }
                    }
                    EnvPhase::Sustain => {
                        env_state.level = sustain;
                    }
                    EnvPhase::Release => {
                        if release > 0.0 {
                            let progress = (env_state.time_in_phase / release).min(1.0);
                            env_state.level = env_state.release_start_level * (1.0 - progress);

                            if progress >= 1.0 {
                                env_state.level = 0.0;
                                env_state.phase = EnvPhase::Idle;
                            }
                        } else {
                            env_state.level = 0.0;
                            env_state.phase = EnvPhase::Idle;
                        }
                    }
                    EnvPhase::Idle => {
                        env_state.level = 0.0;
                    }
                }

                // Update state in node
                if let Some(Some(SignalNode::StructuredSignal {
                    state: s,
                    last_trigger_time: lt,
                    last_cycle: lc,
                    ..
                })) = self.nodes.get_mut(node_id.0)
                {
                    *s = env_state.clone();
                    *lt = latest_triggered_start as f32;
                    *lc = current_cycle;
                }

                // Output: input signal gated by envelope
                input_val * env_state.level
            }

            SignalNode::Delay {
                input,
                time,
                feedback,
                mix,
                buffer,
                write_idx,
            } => {
                let input_val = self.eval_signal(&input);
                let delay_time = self.eval_signal(&time).max(0.0).min(2.0);
                let fb = self.eval_signal(&feedback).max(0.0).min(0.99);
                let mix_val = self.eval_signal(&mix).max(0.0).min(1.0);

                let delay_samples = (delay_time * self.sample_rate) as usize;
                let delay_samples = delay_samples.min(buffer.len() - 1).max(1);

                // Read from delay line
                let read_idx = (write_idx + buffer.len() - delay_samples) % buffer.len();
                let delayed = buffer[read_idx];

                // Write to delay line (input + feedback)
                // Apply soft clipping to prevent feedback explosion
                let to_write = (input_val + delayed * fb).tanh();

                // Update buffer and write index
                if let Some(Some(SignalNode::Delay {
                    buffer: buf,
                    write_idx: idx,
                    ..
                })) = self.nodes.get_mut(node_id.0)
                {
                    buf[*idx] = to_write;
                    *idx = (*idx + 1) % buf.len();
                }

                // Mix dry and wet
                input_val * (1.0 - mix_val) + delayed * mix_val
            }

            SignalNode::RMS {
                input,
                window_size,
                buffer,
                write_idx,
            } => {
                let input_val = self.eval_signal(&input);
                let window_seconds = self.eval_signal(&window_size).max(0.001).min(1.0);

                // Convert window size (seconds) to samples
                let window_samples = (window_seconds * self.sample_rate) as usize;
                let window_samples = window_samples.clamp(1, buffer.len());

                // Update buffer
                if let Some(Some(SignalNode::RMS {
                    buffer: buf,
                    write_idx: idx,
                    ..
                })) = self.nodes.get_mut(node_id.0)
                {
                    buf[*idx] = input_val * input_val;
                    *idx = (*idx + 1) % buf.len();
                }

                // Calculate RMS over the specified window
                // Sum only the most recent window_samples
                let mut sum: f32 = 0.0;
                for i in 0..window_samples {
                    let idx = (write_idx + buffer.len() - i) % buffer.len();
                    sum += buffer[idx];
                }

                (sum / window_samples as f32).sqrt()
            }

            SignalNode::Schmidt {
                input,
                high_threshold,
                low_threshold,
                state,
            } => {
                let input_val = self.eval_signal(&input);
                let high = self.eval_signal(&high_threshold);
                let low = self.eval_signal(&low_threshold);

                // Current state (captured from the pattern match)
                let mut output_state = state;

                // Update state based on hysteresis logic
                if let Some(Some(SignalNode::Schmidt {
                    state: current_state,
                    ..
                })) = self.nodes.get_mut(node_id.0)
                {
                    // If currently low and input exceeds high threshold, turn on
                    if !*current_state && input_val > high {
                        *current_state = true;
                        output_state = true;
                    }
                    // If currently high and input falls below low threshold, turn off
                    else if *current_state && input_val < low {
                        *current_state = false;
                        output_state = false;
                    } else {
                        output_state = *current_state;
                    }
                }

                // Output 1.0 if high, 0.0 if low
                if output_state {
                    1.0
                } else {
                    0.0
                }
            }

            SignalNode::Latch {
                input,
                gate,
                held_value,
                last_gate,
            } => {
                let input_val = self.eval_signal(&input);
                let gate_val = self.eval_signal(&gate);

                // Current held value and last gate (captured from pattern match)
                let mut output_val = held_value;

                // Update state if gate has rising edge (0→1)
                if let Some(Some(SignalNode::Latch {
                    held_value: stored_val,
                    last_gate: stored_gate,
                    ..
                })) = self.nodes.get_mut(node_id.0)
                {
                    // Detect rising edge: last_gate < 0.5 and gate_val >= 0.5
                    if *stored_gate < 0.5 && gate_val >= 0.5 {
                        // Sample the input
                        *stored_val = input_val;
                        output_val = input_val;
                    } else {
                        output_val = *stored_val;
                    }

                    // Update last_gate for next sample
                    *stored_gate = gate_val;
                }

                output_val
            }

            SignalNode::Timer {
                trigger,
                elapsed_time,
                last_trigger,
            } => {
                let trigger_val = self.eval_signal(&trigger);

                // Current elapsed time (captured from pattern match)
                let mut output_val = elapsed_time;

                // Update state if trigger has rising edge (0→1)
                if let Some(Some(SignalNode::Timer {
                    elapsed_time: stored_time,
                    last_trigger: stored_trigger,
                    ..
                })) = self.nodes.get_mut(node_id.0)
                {
                    // Detect rising edge: last_trigger < 0.5 and trigger_val >= 0.5
                    if *stored_trigger < 0.5 && trigger_val >= 0.5 {
                        // Reset timer to 0
                        *stored_time = 0.0;
                        output_val = 0.0;
                    } else {
                        // Increment elapsed time by one sample
                        *stored_time += 1.0 / self.sample_rate;
                        output_val = *stored_time;
                    }

                    // Update last_trigger for next sample
                    *stored_trigger = trigger_val;
                }

                output_val
            }

            SignalNode::Pitch { input, last_pitch } => {
                // Simplified pitch detection - would need more sophisticated algorithm
                let _input_val = self.eval_signal(&input);

                // For now, just return last pitch
                // Real implementation would do autocorrelation or FFT
                last_pitch
            }

            SignalNode::Transient {
                input,
                threshold,
                last_value,
            } => {
                let input_val = self.eval_signal(&input).abs();
                let last = last_value;

                // Detect sharp changes (for saw wave discontinuities)
                let diff = (input_val - last).abs();

                // Generate transient pulse on significant changes
                let transient = if diff > threshold {
                    1.0
                } else if last > 1.5 && input_val < 0.5 {
                    // Detect saw wave reset (big drop)
                    1.0
                } else {
                    0.0
                };

                // Update last value with actual input (not transient)
                if let Some(Some(SignalNode::Transient { last_value: lv, .. })) =
                    self.nodes.get_mut(node_id.0)
                {
                    *lv = input_val;
                }

                transient
            }

            SignalNode::PeakFollower {
                input,
                attack_time,
                release_time,
                current_peak,
            } => {
                let input_val = self.eval_signal(&input).abs();
                let attack_sec = self.eval_signal(&attack_time).max(0.00001); // Min 10μs
                let release_sec = self.eval_signal(&release_time).max(0.00001);

                let mut output_val = current_peak;

                // Update peak follower state
                if let Some(Some(SignalNode::PeakFollower {
                    current_peak: stored_peak,
                    ..
                })) = self.nodes.get_mut(node_id.0)
                {
                    // Calculate attack and release coefficients
                    // Coefficient determines how fast we approach target
                    // coeff = 1 - exp(-1 / (time * sample_rate))
                    let attack_coeff = 1.0 - (-1.0 / (attack_sec * self.sample_rate)).exp();
                    let release_coeff = 1.0 - (-1.0 / (release_sec * self.sample_rate)).exp();

                    if input_val > *stored_peak {
                        // Attack: quickly follow increases
                        *stored_peak += (input_val - *stored_peak) * attack_coeff;
                    } else {
                        // Release: slowly decay
                        *stored_peak += (input_val - *stored_peak) * release_coeff;
                    }

                    output_val = *stored_peak;
                }

                output_val
            }

            SignalNode::AmpFollower {
                input,
                attack_time,
                release_time,
                window_size,
                buffer,
                write_idx,
                current_envelope,
            } => {
                let input_val = self.eval_signal(&input);
                let attack_sec = self.eval_signal(&attack_time).max(0.00001);
                let release_sec = self.eval_signal(&release_time).max(0.00001);
                let window_sec = self.eval_signal(&window_size).max(0.0001);

                let mut output_val = current_envelope;

                // Update amp follower state
                if let Some(Some(SignalNode::AmpFollower {
                    buffer: buf,
                    write_idx: idx,
                    current_envelope: env,
                    ..
                })) = self.nodes.get_mut(node_id.0)
                {
                    // Update buffer size if window changed
                    let target_size = (window_sec * self.sample_rate) as usize;
                    let target_size = target_size.max(1).min(88200); // Max 2 seconds

                    if buf.len() != target_size {
                        buf.resize(target_size, 0.0);
                        *idx = 0;
                    }

                    // Write new sample to circular buffer
                    buf[*idx] = input_val * input_val; // Store squared value for RMS
                    *idx = (*idx + 1) % buf.len();

                    // Calculate RMS
                    let sum: f32 = buf.iter().sum();
                    let rms = (sum / buf.len() as f32).sqrt();

                    // Apply attack/release smoothing to RMS
                    let attack_coeff = 1.0 - (-1.0 / (attack_sec * self.sample_rate)).exp();
                    let release_coeff = 1.0 - (-1.0 / (release_sec * self.sample_rate)).exp();

                    if rms > *env {
                        // Attack: quickly follow increases
                        *env += (rms - *env) * attack_coeff;
                    } else {
                        // Release: slowly decay
                        *env += (rms - *env) * release_coeff;
                    }

                    output_val = *env;
                }

                output_val
            }

            SignalNode::Router {
                input,
                destinations: _,
            } => {
                // Router just passes through input, destinations are handled separately
                self.eval_signal(&input)
            }
        };

        // Cache the value
        self.value_cache.insert(*node_id, value);

        value
    }

    /// Process one sample and advance time
    pub fn process_sample(&mut self) -> f32 {
        // Clear cache for new sample
        self.value_cache.clear();

        // Start with single output (for backwards compatibility)
        // Check if channel 0 is hushed
        let mut mixed_output = if let Some(output_id) = self.output {
            if self.hushed_channels.contains(&0) {
                0.0 // Silenced
            } else {
                self.eval_node(&output_id)
            }
        } else {
            0.0
        };

        // Mix in all numbered output channels (out1, out2, etc.)
        // Collect channel numbers first to avoid borrow checker issues
        let channels: Vec<(usize, crate::unified_graph::NodeId)> =
            self.outputs.iter().map(|(&ch, &node)| (ch, node)).collect();

        for (ch, node_id) in channels {
            // Skip hushed channels
            if self.hushed_channels.contains(&ch) {
                continue;
            }

            // Mix the channel output
            mixed_output += self.eval_node(&node_id);
        }

        // Advance cycle position
        self.cycle_position += self.cps as f64 / self.sample_rate as f64;

        // Increment sample counter
        self.sample_count += 1;

        mixed_output
    }

    /// Render a buffer of audio (mono - mixes all channels)
    pub fn render(&mut self, num_samples: usize) -> Vec<f32> {
        let mut buffer = Vec::with_capacity(num_samples);
        for _ in 0..num_samples {
            buffer.push(self.process_sample());
        }
        buffer
    }

    /// Render stereo audio (left = out1, right = out2)
    /// Returns (left_channel, right_channel)
    pub fn render_stereo(&mut self, num_samples: usize) -> (Vec<f32>, Vec<f32>) {
        let mut left = Vec::with_capacity(num_samples);
        let mut right = Vec::with_capacity(num_samples);

        for _ in 0..num_samples {
            // Get multi-channel output
            let channels = self.process_sample_multi();

            // Extract left (channel 0/out1) and right (channel 1/out2)
            let left_sample = channels.get(0).copied().unwrap_or(0.0);
            let right_sample = channels.get(1).copied().unwrap_or(0.0);

            left.push(left_sample);
            right.push(right_sample);
        }

        (left, right)
    }
}
