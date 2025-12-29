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
//!     phase: RefCell::new(0.0),
//! });
//!
//! // Scale LFO from -1..1 to 0.2..1.0 (quiet to loud)
//! // min and max are now Signal types for pattern modulation
//! let scaled_gain = Signal::Expression(Box::new(SignalExpr::Scale {
//!     input: Signal::Node(lfo),
//!     min: Signal::Value(0.2),
//!     max: Signal::Value(1.0),
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
//!     phase: RefCell::new(0.0),
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
//!     phase: RefCell::new(0.0),
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

use crate::midi_input::{ArpPattern, Arpeggiator, Scale, scale_lock};
use crate::mini_notation_v3::parse_mini_notation;
use crate::pattern::{Fraction, Pattern, State, TimeSpan};
use crate::sample_loader::SampleBank;
use crate::synth_voice_manager::SynthVoiceManager;
use crate::voice_manager::{VoiceBuffers, VoiceManager};
use rayon::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::f32::consts::PI;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

/// Unique identifier for nodes in the graph
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct NodeId(pub usize);

/// Dependency graph for block-based parallel processing
///
/// This structure represents the DAG (Directed Acyclic Graph) of node dependencies,
/// allowing us to:
/// 1. Determine evaluation order (topological sort)
/// 2. Identify independent nodes that can run in parallel
/// 3. Detect feedback loops that need buffering
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// Dependencies for each node: node_id -> Vec<dependency_node_ids>
    pub dependencies: HashMap<NodeId, Vec<NodeId>>,
    /// Reverse dependencies: node_id -> Vec<nodes_that_depend_on_it>
    pub dependents: HashMap<NodeId, Vec<NodeId>>,
}

/// Parallel execution stages
/// Each stage contains nodes that can be evaluated in parallel
/// Stages must be executed sequentially (stage N before stage N+1)
#[derive(Debug, Clone)]
pub struct ExecutionStages {
    /// Stages of node IDs, each stage can run in parallel
    pub stages: Vec<Vec<NodeId>>,
    /// Nodes involved in feedback loops (need special handling)
    pub feedback_nodes: Vec<NodeId>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
            dependents: HashMap::new(),
        }
    }

    /// Add a dependency: from_node depends on to_node
    pub fn add_dependency(&mut self, from_node: NodeId, to_node: NodeId) {
        self.dependencies
            .entry(from_node)
            .or_insert_with(Vec::new)
            .push(to_node);
        self.dependents
            .entry(to_node)
            .or_insert_with(Vec::new)
            .push(from_node);
    }

    /// Get all dependencies of a node (nodes it depends on)
    pub fn get_dependencies(&self, node_id: NodeId) -> &[NodeId] {
        self.dependencies
            .get(&node_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Perform topological sort using Kahn's algorithm
    /// Returns stages of nodes that can run in parallel
    pub fn topological_sort(&self) -> Result<ExecutionStages, String> {
        use std::collections::VecDeque;

        let mut in_degree: HashMap<NodeId, usize> = HashMap::new();
        let all_nodes: std::collections::HashSet<NodeId> = self
            .dependencies
            .keys()
            .chain(self.dependents.keys())
            .copied()
            .collect();

        // Calculate in-degrees
        for &node in &all_nodes {
            in_degree.insert(node, self.get_dependencies(node).len());
        }

        let mut stages = Vec::new();
        let mut queue: VecDeque<NodeId> = in_degree
            .iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(&node, _)| node)
            .collect();

        while !queue.is_empty() {
            // All nodes in queue can run in parallel (same stage)
            let current_stage: Vec<NodeId> = queue.drain(..).collect();

            // Update in-degrees for dependent nodes
            for &node in &current_stage {
                if let Some(deps) = self.dependents.get(&node) {
                    for &dependent in deps {
                        if let Some(degree) = in_degree.get_mut(&dependent) {
                            *degree -= 1;
                            if *degree == 0 {
                                queue.push_back(dependent);
                            }
                        }
                    }
                }
            }

            stages.push(current_stage);
        }

        // Check for cycles (feedback loops)
        let total_processed: usize = stages.iter().map(|s| s.len()).sum();
        if total_processed < all_nodes.len() {
            let unprocessed: Vec<NodeId> = all_nodes
                .iter()
                .filter(|n| !stages.iter().any(|stage| stage.contains(n)))
                .copied()
                .collect();

            // Cycles detected - these are feedback loops
            Ok(ExecutionStages {
                stages,
                feedback_nodes: unprocessed,
            })
        } else {
            Ok(ExecutionStages {
                stages,
                feedback_nodes: Vec::new(),
            })
        }
    }
}

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
    Min(Signal, Signal),
    Scale { input: Signal, min: Signal, max: Signal }, // Pattern-modulatable scaling
}

/// Runtime envelope type for Sample nodes (after compilation)
#[derive(Debug, Clone)]
pub enum RuntimeEnvelopeType {
    Percussion, // Use attack + release
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
    /// STATEFUL: Uses RefCell for interior mutability (thread-safe phase tracking)
    Oscillator {
        freq: Signal,
        waveform: Waveform,
        semitone_offset: f32, // Semitone offset for note triggering (+0.5, -2.3, etc.)
        phase: std::cell::RefCell<f32>, // Interior mutability for parallel synthesis
        pending_freq: std::cell::RefCell<Option<f32>>, // Frequency change waiting for zero-crossing
        last_sample: std::cell::RefCell<f32>, // For zero-crossing detection
    },

    /// FM (Frequency Modulation) oscillator
    /// output = sin(2π * carrier * t + mod_index * sin(2π * modulator * t))
    FMOscillator {
        carrier_freq: Signal,                     // Carrier frequency in Hz
        modulator_freq: Signal,                   // Modulator frequency in Hz
        mod_index: Signal,                        // Modulation index (depth)
        carrier_phase: std::cell::RefCell<f32>,   // Carrier phase (0.0 to 1.0)
        modulator_phase: std::cell::RefCell<f32>, // Modulator phase (0.0 to 1.0)
    },

    /// Phase Modulation (PM) oscillator
    /// Takes external modulation signal directly (not internal oscillator)
    /// PM: output = sin(2π * carrier_phase + mod_index * modulation_signal)
    /// Unlike FM, modulator can be any signal (noise, envelope, audio, etc.)
    PMOscillator {
        carrier_freq: Signal,                   // Carrier frequency in Hz
        modulation: Signal,                     // External modulation signal
        mod_index: Signal,                      // Modulation index (depth)
        carrier_phase: std::cell::RefCell<f32>, // Carrier phase (0.0 to 1.0)
    },

    /// Blip oscillator (Band-Limited Impulse Train)
    /// Generates periodic band-limited impulses using PolyBLEP algorithm
    /// Creates a train of narrow pulses that are band-limited to prevent aliasing
    /// Rich harmonic content up to Nyquist frequency
    /// Useful for percussive sounds and as building block for other waveforms
    Blip {
        frequency: Signal,              // Impulse train frequency in Hz
        phase: std::cell::RefCell<f32>, // Current phase (0.0 to 1.0)
    },

    /// VCO (Voltage-Controlled Oscillator)
    /// Analog-style oscillator with multiple waveforms and PWM
    /// Models classic synthesizer oscillators (Moog, ARP, Sequential)
    /// Waveforms: 0=saw, 1=square, 2=triangle, 3=sine
    /// Band-limited using PolyBLEP algorithm
    VCO {
        frequency: Signal,              // Oscillator frequency in Hz
        waveform: Signal,               // Waveform selection (0-3)
        pulse_width: Signal,            // Pulse width for square wave (0.0-1.0, default 0.5)
        phase: std::cell::RefCell<f32>, // Current phase (0.0 to 1.0)
    },

    /// White noise generator
    /// Generates uniformly distributed random samples in range [-1, 1]
    WhiteNoise,

    /// UnitDelay (z^-1) for feedback loops
    /// Returns the previous sample's value of a bus, enabling self-referential feedback
    /// This is the fundamental building block for IIR filters and feedback systems
    /// Example: ~x $ ~input * 0.5 + ~x * 0.3
    /// The ~x reference becomes a UnitDelay that reads the previous sample
    UnitDelay {
        bus_name: String, // Name of the bus to read previous value from
    },

    /// Pink noise generator (1/f spectrum)
    /// Generates noise with equal energy per octave
    /// Uses Voss-McCartney algorithm with octave bins
    PinkNoise { state: PinkNoiseState },

    /// Brown noise generator (6dB/octave rolloff)
    /// Generates very "warm" noise using random walk
    /// Also called Brownian noise or red noise
    BrownNoise { state: BrownNoiseState },

    /// MIDI Input - Real-time MIDI note triggering
    /// Receives MIDI events from external keyboard/controller
    /// Outputs frequency corresponding to currently pressed notes
    /// Supports polyphony tracking and per-channel filtering
    ///
    /// Usage: ~midi (all channels) or ~midi1 through ~midi16 (specific channel)
    /// Example: ~keys: ~midi1 # saw 440 # adsr 0.01 0.1 0.7 0.2
    MidiInput {
        /// MIDI channel filter: None = all channels, Some(0-15) = specific channel
        channel: Option<u8>,
        /// Active notes currently being played (note → velocity 0-1)
        active_notes: std::cell::RefCell<std::collections::HashMap<u8, f32>>,
        /// Shared event queue from MIDI input handler
        event_queue: crate::midi_input::MidiEventQueue,
        /// Current output frequency (Hz)
        last_freq: std::cell::RefCell<f32>,
        /// Current gate value (0.0 = off, 1.0 = on) - for envelope triggering
        gate: std::cell::RefCell<f32>,
    },

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
        input: Signal,    // Input signal to smooth
        lag_time: Signal, // Time constant in seconds
        state: LagState,
    },

    /// XLine (exponential envelope)
    /// Generates exponential ramp from start to end over duration
    /// More natural sounding than linear ramps for pitch/amplitude
    XLine {
        start: Signal,    // Starting value
        end: Signal,      // Ending value
        duration: Signal, // Duration in seconds
        state: XLineState,
    },

    /// ASR (Attack-Sustain-Release) envelope
    /// Gate-based envelope: attacks when gate rises, sustains while high, releases when gate falls
    /// Perfect for organ-style sounds and continuous notes
    ASR {
        gate: Signal,    // Gate signal (0 = off, >0.5 = on)
        attack: Signal,  // Attack time in seconds
        release: Signal, // Release time in seconds
        state: ASRState,
    },

    /// Pulse wave oscillator (variable pulse width)
    /// Output: +1 when phase < width, -1 otherwise
    /// width=0.5 creates square wave (only odd harmonics)
    /// Other widths create different harmonic content
    Pulse {
        freq: Signal,  // Frequency in Hz
        width: Signal, // Pulse width / duty cycle (0.0 to 1.0)
        phase: f32,    // Phase (0.0 to 1.0)
    },

    /// Wavetable oscillator
    /// Reads through stored waveform at variable speeds for pitch control
    /// Classic technique: PPG Wave, Waldorf, Serum
    Wavetable {
        freq: Signal,          // Frequency in Hz
        state: WavetableState, // Wavetable data and phase
    },

    /// Granular synthesis
    /// Breaks audio into small grains (5-100ms) and overlaps them
    /// Classic technique: Reaktor, Ableton Granulator, Max/MSP
    Granular {
        source: Signal,        // Input audio source
        grain_size_ms: Signal, // Grain duration in milliseconds
        density: Signal,       // Grain spawn rate (0.0 to 1.0)
        pitch: Signal,         // Playback speed/pitch multiplier
        state: GranularState,  // Grain buffer and active grains
    },

    /// Karplus-Strong string synthesis
    /// Physical modeling of plucked strings
    /// Uses delay line + averaging for realistic string decay
    KarplusStrong {
        freq: Signal,              // Fundamental frequency in Hz
        damping: Signal,           // Damping factor (0.0 = fast decay, 1.0 = slow)
        trigger: Signal,           // Trigger signal (rising edge re-plucks the string)
        state: KarplusStrongState, // Delay line state
        last_freq: f32,            // Previous frequency (for detecting changes)
        last_trigger: f32,         // Previous trigger value (for edge detection)
    },

    /// Digital Waveguide Physical Modeling
    /// Simulates wave propagation in strings/tubes using bidirectional delay lines
    /// More sophisticated than Karplus-Strong with separate forward/backward waves
    Waveguide {
        freq: Signal,            // Fundamental frequency in Hz
        damping: Signal,         // Energy loss at boundaries (0.0 = no loss, 1.0 = max loss)
        pickup_position: Signal, // Where to read from string (0.0 to 1.0)
        state: WaveguideState,   // Bidirectional delay line state
        last_freq: f32,          // Previous frequency (for detecting changes)
    },

    /// Formant Synthesis
    /// Filters source signal through three resonant bandpass filters to create vowel sounds
    /// Each vowel is characterized by specific formant frequencies (F1, F2, F3)
    Formant {
        source: Signal,      // Input signal to filter
        f1: Signal,          // First formant frequency (Hz)
        f2: Signal,          // Second formant frequency (Hz)
        f3: Signal,          // Third formant frequency (Hz)
        bw1: Signal,         // First formant bandwidth (Hz)
        bw2: Signal,         // Second formant bandwidth (Hz)
        bw3: Signal,         // Third formant bandwidth (Hz)
        state: FormantState, // Bandpass filter state
    },

    /// Vowel Filter (TidalCycles-style formant filter)
    /// Simplified formant filter using vowel selector: 0=a, 1=e, 2=i, 3=o, 4=u
    /// Pattern-controllable vowel selection for live coding convenience
    Vowel {
        source: Signal,      // Input signal to filter
        vowel: Signal,       // Vowel selector (0-4 maps to a,e,i,o,u)
        state: FormantState, // Bandpass filter state
    },

    /// Additive Synthesis
    /// Creates complex timbres by summing multiple sine wave partials (harmonics)
    /// Each partial is a multiple of the fundamental frequency with independent amplitude
    /// Example: additive 440 "1.0 0.5 0.25" → 440Hz + 880Hz(×0.5) + 1320Hz(×0.25)
    Additive {
        freq: Signal,         // Fundamental frequency (Hz) - pattern-modulatable
        amplitudes: Vec<f32>, // Fixed amplitude for each partial [1, 2, 3, ...]
        state: AdditiveState, // Phase tracking state
    },

    /// Vocoder
    /// Analyzes modulator amplitude envelope in frequency bands and applies to carrier
    /// Classic use: Robot voice effect (voice modulating synth)
    /// Example: vocoder ~voice ~synth 16 → 16-band vocoder
    Vocoder {
        modulator: Signal, // Modulator signal (usually voice/rhythmic)
        carrier: Signal,   // Carrier signal (usually synth/rich harmonics)
        num_bands: usize,  // Number of frequency bands (2-32, default 8)
        state: VocoderState,
    },

    PitchShift {
        input: Signal,     // Input signal to pitch shift
        semitones: Signal, // Pitch shift amount in semitones (can be pattern-modulated)
        state: PitchShifterState,
    },

    /// Brick-wall limiter (prevents signal from exceeding threshold)
    /// Clamps signal to [-threshold, +threshold]
    Limiter {
        input: Signal,     // Input signal
        threshold: Signal, // Maximum allowed amplitude
    },

    /// State Variable Filter (Chamberlin topology)
    /// Multi-mode filter producing LP, HP, BP, and Notch outputs
    /// Mode: 0=lowpass, 1=highpass, 2=bandpass, 3=notch
    SVF {
        input: Signal,     // Input signal
        frequency: Signal, // Cutoff/center frequency in Hz
        resonance: Signal, // Resonance/Q (0.0 to ~10.0)
        mode: usize,       // Filter mode (0=LP, 1=HP, 2=BP, 3=Notch)
        state: SVFState,   // Filter state (integrators)
    },

    /// Biquad Filter (high-quality second-order IIR)
    /// Based on RBJ Audio EQ Cookbook formulas
    /// Mode: 0=lowpass, 1=highpass, 2=bandpass, 3=notch
    Biquad {
        input: Signal,      // Input signal
        frequency: Signal,  // Cutoff/center frequency in Hz
        q: Signal,          // Quality factor (0.1 to ~20.0)
        mode: usize,        // Filter mode (0=LP, 1=HP, 2=BP, 3=Notch)
        state: BiquadState, // Filter state (coefficients and history)
    },

    /// Resonz - Resonant Bandpass Filter
    /// Highly resonant bandpass with sharp peak at center frequency
    /// Used for formant synthesis, resonant effects, and plucked string simulation
    Resonz {
        input: Signal,      // Input signal
        frequency: Signal,  // Center frequency in Hz
        q: Signal,          // Q factor (resonance, 1.0 to ~100.0)
        state: BiquadState, // Filter state (reuses biquad implementation)
    },

    /// RLPF - Resonant Lowpass Filter
    /// Classic analog synth lowpass with resonant peak at cutoff
    /// Used for filter sweeps, bass sounds, and acid basslines
    RLPF {
        input: Signal,      // Input signal
        cutoff: Signal,     // Cutoff frequency in Hz
        resonance: Signal,  // Resonance/Q (0.5 to ~20.0)
        state: BiquadState, // Filter state (reuses biquad implementation)
    },

    /// RHPF - Resonant Highpass Filter
    /// Highpass filter with resonant peak at cutoff
    /// Used for removing low end, creating air, and rhythmic filtering
    RHPF {
        input: Signal,      // Input signal
        cutoff: Signal,     // Cutoff frequency in Hz
        resonance: Signal,  // Resonance/Q (0.5 to ~20.0)
        state: BiquadState, // Filter state (reuses biquad implementation)
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

    /// Signal as a pattern source (audio→pattern modulation)
    /// Samples a signal once per cycle and exposes it as a pattern value
    /// Thread-safe with Arc<Mutex> for pattern closures
    SignalAsPattern {
        signal: Signal,
        last_sampled_value: std::sync::Arc<std::sync::Mutex<f32>>,
        last_sample_cycle: std::sync::Arc<std::sync::Mutex<f32>>,
    },

    /// Cycle trigger: generates a short pulse at the start of each cycle
    /// Useful for triggering envelopes rhythmically
    CycleTrigger {
        last_cycle: i32,       // Track which cycle triggered last (internal state)
        pulse_width: Signal,   // Pattern-modulatable pulse width in seconds
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
        unit_mode: Signal, // Unit mode: 0="r" (rate), 1="c" (cycle-sync)
        loop_enabled: Signal, // Loop mode: 0=play once, 1=loop continuously
        begin: Signal,     // Sample start point (0.0 = start, 0.5 = middle, 1.0 = end)
        end: Signal,       // Sample end point (0.0 = start, 1.0 = end)
    },

    /// Pattern-triggered synthesizer with ADSR envelopes
    /// Each note in the pattern triggers a new synth voice
    SynthPattern {
        pattern_str: String,
        pattern: Pattern<String>,
        last_trigger_time: f32,
        waveform: Waveform,
        attack: Signal,          // ADSR attack time in seconds (pattern-modulatable)
        decay: Signal,           // ADSR decay time in seconds (pattern-modulatable)
        sustain: Signal,         // ADSR sustain level 0.0-1.0 (pattern-modulatable)
        release: Signal,         // ADSR release time in seconds (pattern-modulatable)
        filter_cutoff: Signal,   // Filter cutoff frequency in Hz (pattern-modulatable, 20000 = no filter)
        filter_resonance: Signal, // Filter resonance/Q 0.0-1.0 (pattern-modulatable)
        filter_env_amount: Signal, // Filter envelope modulation amount in Hz (pattern-modulatable)
        gain: Signal,
        pan: Signal,
    },

    /// MIDI-triggered polyphonic synthesizer
    /// Each MIDI note-on triggers a new synth voice with ADSR envelope
    /// Note-off releases the voice's envelope
    MidiSynth {
        waveform: Waveform,
        attack: Signal,          // ADSR attack time (pattern-modulatable)
        decay: Signal,           // ADSR decay time (pattern-modulatable)
        sustain: Signal,         // ADSR sustain level (pattern-modulatable)
        release: Signal,         // ADSR release time (pattern-modulatable)
        filter_cutoff: Signal,   // Filter cutoff (pattern-modulatable)
        filter_resonance: Signal, // Filter resonance (pattern-modulatable)
        channel: Option<u8>,     // None = all channels
        event_queue: crate::midi_input::MidiEventQueue,
        /// Maps MIDI note number to voice index for proper release
        note_to_voice: RefCell<HashMap<u8, usize>>,
        gain: Signal,
    },

    /// MIDI Voice Frequency - reads from per-voice frequency context
    /// Used within MidiPolySynth signal templates to get the current voice's pitch
    /// Returns 0.0 if called outside of voice context
    MidiVoiceFreq,

    /// MIDI Voice Gate - reads from per-voice gate context
    /// Returns 1.0 when note is held, 0.0 when released
    /// Used to drive envelope generators within MidiPolySynth signal templates
    MidiVoiceGate,

    /// Composable polyphonic MIDI synthesizer
    /// Each voice has its own oscillator phase and envelope state
    /// Voices grow as needed (no stealing) and release naturally
    ///
    /// Example: `saw ~midi` creates per-voice saw oscillators with ASR envelope
    MidiPolySynth {
        /// Waveform type for per-voice oscillators
        waveform: Waveform,
        /// Attack time in seconds (pattern-modulatable)
        attack: Signal,
        /// Release time in seconds (pattern-modulatable)
        release: Signal,
        /// Per-voice state including note, frequency, phase, envelope
        voices: RefCell<Vec<MidiPolyVoice>>,
        /// Shared event queue from MIDI input handler
        event_queue: crate::midi_input::MidiEventQueue,
        /// Maps MIDI note number to voice index for proper release
        note_to_voice: RefCell<HashMap<u8, usize>>,
        /// MIDI channel filter: None = all channels, Some(0-15) = specific channel
        channel: Option<u8>,
        /// Scale root note (MIDI note number, e.g., 60 for C4)
        scale_root: Option<u8>,
        /// Scale type for quantizing incoming notes
        scale_type: Option<Scale>,
        /// Optional arpeggiator state (None = disabled)
        arpeggiator: RefCell<Option<Arpeggiator>>,
    },

    /// Pattern-triggered envelope gate
    /// Gates an input signal with rhythm from a pattern
    EnvelopePattern {
        input: Signal,
        pattern_str: String,
        pattern: Pattern<String>,
        last_trigger_time: f32,
        last_cycle: i32,
        attack: Signal,          // ADSR attack time (pattern-modulatable)
        decay: Signal,           // ADSR decay time (pattern-modulatable)
        sustain: Signal,         // ADSR sustain level (pattern-modulatable)
        release: Signal,         // ADSR release time (pattern-modulatable)
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
        attack: Signal,          // ADSR attack time (pattern-modulatable)
        decay: Signal,           // ADSR decay time (pattern-modulatable)
        sustain: Signal,         // ADSR sustain level (pattern-modulatable)
        release: Signal,         // ADSR release time (pattern-modulatable)
        state: EnvState,
    },

    /// Triggered AR envelope - outputs envelope value (0-1) triggered by pattern
    /// Unlike StructuredSignal, this outputs the envelope directly (not multiplied by input)
    /// Usage: tar "t(3,8)" 0.1 0.5 -> envelope that attacks on each pattern event
    TriggeredAR {
        pattern_str: String,
        pattern: Pattern<bool>,
        attack: Signal,
        release: Signal,
        last_trigger_time: f32,
        last_cycle: i32,
        state: EnvState,
    },

    /// Triggered ADSR envelope - outputs envelope value (0-1) triggered by pattern
    /// Usage: tadsr "t(3,8)" 0.1 0.1 0.8 0.5
    TriggeredADSR {
        pattern_str: String,
        pattern: Pattern<bool>,
        attack: Signal,
        decay: Signal,
        sustain: Signal,
        release: Signal,
        last_trigger_time: f32,
        last_cycle: i32,
        state: EnvState,
    },

    /// Pattern to gate signal - outputs 1.0 during pattern events, 0.0 otherwise
    /// Usage: gate "t(3,8)" -> continuous gate signal
    PatternGate {
        pattern_str: String,
        pattern: Pattern<bool>,
    },

    /// Pattern to trigger pulse - outputs 1.0 for one sample at event onset, 0.0 otherwise
    /// Usage: trig "t(3,8)" -> trigger pulses
    PatternTrigger {
        pattern_str: String,
        pattern: Pattern<bool>,
        last_trigger_time: f32,
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

    /// Pattern evaluator - evaluates a numeric pattern at current cycle position
    /// Used for functions like run, scan that generate numeric patterns
    PatternEvaluator { pattern: Pattern<f64> },

    // === Conditional Effects ===
    /// Apply effect every N cycles, bypass otherwise
    /// Enables syntax like: s "bd" $ every 4 (# lpf 300)
    EveryEffect {
        input: Signal,
        effect: Signal,
        n: i32,
    },

    /// Apply effect with probability per cycle
    /// Enables syntax like: s "bd" $ sometimes (# lpf 300)
    SometimesEffect {
        input: Signal,
        effect: Signal,
        prob: Signal, // Pattern-modulatable probability
    },

    /// Apply effect when (cycle - offset) % modulo == 0
    /// Enables syntax like: s "bd" $ whenmod 3 1 (# lpf 300)
    WhenmodEffect {
        input: Signal,
        effect: Signal,
        modulo: i32,
        offset: i32,
    },

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

    /// DJ Filter (sweep from lowpass to highpass)
    /// TidalCycles-style DJ filter: value 0-0.5 = lowpass, 0.5-1 = highpass
    /// Single parameter control makes it perfect for live performance
    DJFilter {
        input: Signal,
        value: Signal, // 0-1: 0=full lowpass, 0.5=neutral, 1=full highpass
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
        cutoff: Signal,    // Cutoff frequency in Hz
        resonance: Signal, // Resonance (0.0-1.0, self-oscillates near 1.0)
        state: MoogLadderState,
    },

    /// Parametric EQ (3-band peaking equalizer)
    /// Each band can boost or cut frequencies independently
    ParametricEQ {
        input: Signal,
        // Low band
        low_freq: Signal, // Center frequency in Hz
        low_gain: Signal, // Gain in dB (-20 to +20)
        low_q: Signal,    // Bandwidth (0.1 to 10.0)
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
    /// Parameters are pattern-modulatable signals
    Envelope {
        input: Signal,
        trigger: Signal,
        attack: Signal,
        decay: Signal,
        sustain: Signal,
        release: Signal,
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
        levels: Vec<f32>,       // Target levels for each breakpoint
        times: Vec<f32>,        // Duration for each segment
        current_segment: usize, // Which segment we're in
        segment_elapsed: f32,   // Time elapsed in current segment
        current_value: f32,     // Current interpolated value
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

    /// Tape Delay (analog tape simulation with wow, flutter, saturation)
    /// Emulates vintage tape delay machines with realistic tape artifacts
    TapeDelay {
        input: Signal,
        time: Signal,          // Delay time in seconds
        feedback: Signal,      // Feedback amount (0.0 to 0.95)
        wow_rate: Signal,      // Wow modulation rate (Hz, 0.1-2.0)
        wow_depth: Signal,     // Wow modulation depth (0.0-1.0)
        flutter_rate: Signal,  // Flutter modulation rate (Hz, 5.0-10.0)
        flutter_depth: Signal, // Flutter modulation depth (0.0-1.0)
        saturation: Signal,    // Tape saturation (0.0-1.0)
        mix: Signal,           // Dry/wet mix (0.0-1.0)
        state: TapeDelayState,
    },

    /// Multi-Tap Delay (multiple equally-spaced echoes)
    /// Creates rhythmic delay patterns with multiple taps
    MultiTapDelay {
        input: Signal,
        time: Signal,     // Base delay time in seconds
        taps: usize,      // Number of taps (2-8)
        feedback: Signal, // Feedback amount
        mix: Signal,      // Dry/wet mix
        buffer: Vec<f32>,
        write_idx: usize,
    },

    /// Ping-Pong Delay (stereo bouncing delay)
    /// NOTE: Returns only one channel - use two nodes for stereo
    PingPongDelay {
        input: Signal,
        time: Signal,         // Delay time per side
        feedback: Signal,     // Feedback amount
        stereo_width: Signal, // Stereo spread (0.0-1.0)
        channel: bool,        // false = left, true = right
        mix: Signal,          // Dry/wet mix
        buffer_l: Vec<f32>,   // Left channel buffer
        buffer_r: Vec<f32>,   // Right channel buffer
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
        held_value: f32, // The currently held sample
        last_gate: f32,  // Previous gate value (for edge detection)
    },

    /// Timer
    /// Measures elapsed time since last trigger reset
    /// Resets to 0 on rising edge, counts up in seconds
    Timer {
        trigger: Signal,
        elapsed_time: f32, // Current elapsed time in seconds
        last_trigger: f32, // Previous trigger value (for edge detection)
    },

    /// Pitch detector
    Pitch { input: Signal, last_pitch: f32 },

    /// Transient detector - threshold is now pattern-modulatable
    Transient {
        input: Signal,
        threshold: Signal, // Pattern-modulatable threshold
        last_value: f32,   // Internal state - stays f32
    },

    /// Peak Follower
    /// Tracks the peak amplitude of an input signal
    /// Fast attack, slow decay
    PeakFollower {
        input: Signal,
        attack_time: Signal,  // Attack time in seconds
        release_time: Signal, // Release/decay time in seconds
        current_peak: f32,    // Current peak level
    },

    /// Amp Follower
    /// RMS-based envelope follower with attack/release smoothing
    /// Smoother than peak follower for amplitude tracking
    AmpFollower {
        input: Signal,
        attack_time: Signal,   // Attack time in seconds
        release_time: Signal,  // Release time in seconds
        window_size: Signal,   // RMS window size in seconds
        buffer: Vec<f32>,      // Circular buffer for RMS
        write_idx: usize,      // Write position in buffer
        current_envelope: f32, // Smoothed RMS value
    },

    /// Zero Crossing Detector
    /// Detects when signal crosses zero and outputs crossing frequency
    /// Useful for pitch tracking and triggering on transients
    ZeroCrossing {
        input: Signal,
        last_sample: f32,    // Previous sample for detecting zero crossings
        crossing_count: u32, // Number of crossings in current window
        sample_count: u32,   // Samples since last frequency update
        window_samples: u32, // Window size in samples for frequency calculation
        last_frequency: f32, // Last calculated frequency
    },

    // === Math & Control ===
    /// Addition
    Add { a: Signal, b: Signal },

    /// Multiplication
    Multiply { a: Signal, b: Signal },

    /// Minimum of two signals (sample-by-sample)
    Min { a: Signal, b: Signal },

    /// Wrap signal into [min, max] range using modulo
    /// Wraps values outside the range back into the range periodically
    Wrap {
        input: Signal,
        min: Signal,
        max: Signal,
    },

    /// Sample-and-hold - captures input when trigger crosses from negative to positive
    /// Classic analog-style S&H: monitors trigger for zero crossings, samples input, holds value
    /// Useful for stepped modulation, random voltage generation, rhythmic parameter automation
    SampleAndHold {
        input: Signal,
        trigger: Signal,
        held_value: std::cell::RefCell<f32>, // Currently held value
        last_trigger: std::cell::RefCell<f32>, // Previous trigger value for crossing detection
    },

    /// Decimator - sample rate reduction for lo-fi/retro effects
    /// Reduces effective sample rate by holding values for N samples
    /// Creates classic bit-crushed/aliased sounds with optional smoothing
    Decimator {
        input: Signal,
        factor: Signal, // Decimation factor (1.0 = no effect, higher = more decimation)
        smooth: Signal, // Smoothing amount (0.0 = harsh, 1.0 = smooth)
        sample_counter: std::cell::RefCell<f32>, // Counter for decimation timing
        held_value: std::cell::RefCell<f32>, // Currently held value
        smooth_state: std::cell::RefCell<f32>, // Previous smoothed output for one-pole filter
    },

    /// Crossfader between two signals
    /// position = 0.0 → 100% signal_a
    /// position = 0.5 → 50% signal_a + 50% signal_b
    /// position = 1.0 → 100% signal_b
    XFade {
        signal_a: Signal,
        signal_b: Signal,
        position: Signal, // 0.0 to 1.0
    },

    /// Mix multiple signals with normalization
    /// Sums all input signals and divides by N to prevent volume multiplication
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

    /// Conditional signal routing (if-then-else)
    /// Routes signal based on condition: condition > 0.5 ? then_signal : else_signal
    /// Enables dynamic signal routing and conditional effects
    /// Example: if ~envelope > 0.5 then ~wet else ~dry
    Conditional {
        condition: Signal,   // Condition signal (> 0.5 = true)
        then_signal: Signal, // Signal when condition is true
        else_signal: Signal, // Signal when condition is false
    },

    /// Select/Multiplex between multiple signals
    /// Routes one of N signals based on index (pattern-modulatable!)
    /// Index is rounded and wrapped to valid range [0, N-1]
    /// Example: select "0 1 2 3" [~bus0, ~bus1, ~bus2, ~bus3]
    Select {
        index: Signal,       // Which input to select (0, 1, 2, ...)
        inputs: Vec<Signal>, // Available signals to select from
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

    /// Dattorro Plate Reverb (professional plate/hall reverb)
    /// Based on Jon Dattorro's figure-8 reverberator design
    /// Used in Lexicon, Valhalla, and other pro reverbs
    /// Produces rich, dense, smooth reverb tails
    DattorroReverb {
        input: Signal,
        pre_delay: Signal, // Pre-delay time in ms (0-500)
        decay: Signal,     // Decay time multiplier (0.1-10.0)
        diffusion: Signal, // Input diffusion (0.0-1.0)
        damping: Signal,   // High-frequency damping (0.0-1.0)
        mod_depth: Signal, // Modulation depth (0.0-1.0) for lushness
        mix: Signal,       // Dry/wet mix (0.0-1.0)
        state: DattorroState,
    },

    /// Lush Reverb - Rich algorithmic reverb with complex modulation
    /// Combines diffuser + FDN + pink/brown noise modulation
    /// Inspired by Lexicon, Valhalla, and modern reverb design
    LushReverb {
        input: Signal,
        predelay: Signal,   // Pre-delay time in seconds (0-0.5)
        decay: Signal,      // Decay amount (0.0-0.9999)
        size: Signal,       // Room size multiplier (0.5-2.0)
        diffusion: Signal,  // Input diffusion (0.0-1.0)
        damping: Signal,    // High-frequency damping (0.0-1.0)
        spin: Signal,       // Fast modulation depth (0.0-1.0)
        wander: Signal,     // Slow modulation depth (0.0-1.0)
        freeze: Signal,     // Freeze mode (>0.5 = frozen)
        mix: Signal,        // Dry/wet mix (0.0-1.0)
        state: crate::nodes::lush_reverb::LushReverbState,
    },

    /// Convolution Reverb
    Convolution {
        input: Signal,
        state: ConvolutionState,
    },

    /// Spectral Freeze - FFT-based spectrum freezing
    SpectralFreeze {
        input: Signal,
        trigger: Signal, // Freeze on rising edge (0.0 to 1.0)
        state: SpectralFreezeState,
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

    /// Sidechain Compressor - compression controlled by external signal
    /// Analyzes sidechain signal but applies gain reduction to main input
    SidechainCompressor {
        main_input: Signal,      // Signal to compress
        sidechain_input: Signal, // Signal controlling compression
        threshold: Signal,       // Threshold in dB (-60.0 to 0.0)
        ratio: Signal,           // Compression ratio (1.0 to 20.0)
        attack: Signal,          // Attack time in seconds (0.001 to 1.0)
        release: Signal,         // Release time in seconds (0.01 to 3.0)
        state: CompressorState,
    },

    /// Expander (upward expansion - boosts signals above threshold)
    /// Opposite of compressor: increases dynamic range by boosting loud signals
    Expander {
        input: Signal,
        threshold: Signal, // Threshold in dB (-60.0 to 0.0)
        ratio: Signal,     // Expansion ratio (1.0 to 10.0)
        attack: Signal,    // Attack time in seconds (0.001 to 1.0)
        release: Signal,   // Release time in seconds (0.01 to 3.0)
        state: ExpanderState,
    },

    /// Adaptive Compressor - compression that adapts to signal analysis
    /// Uses sidechain RMS/peak analysis to modulate threshold and ratio
    /// Enables complex feedback networks where compression responds to signal characteristics
    AdaptiveCompressor {
        main_input: Signal,      // Signal to compress
        sidechain_input: Signal, // Signal to analyze for adaptation
        threshold: Signal,       // Base threshold in dB (-60.0 to 0.0)
        ratio: Signal,           // Base compression ratio (1.0 to 20.0)
        attack: Signal,          // Attack time in seconds (0.001 to 1.0)
        release: Signal,         // Release time in seconds (0.01 to 3.0)
        adaptive_factor: Signal, // How much analysis affects compression (0.0-1.0)
        state: AdaptiveCompressorState,
    },

    /// Tremolo (amplitude modulation)
    /// Classic effect that modulates amplitude with an LFO
    Tremolo {
        input: Signal, // Input signal
        rate: Signal,  // LFO rate in Hz (0.1 to 20.0)
        depth: Signal, // Modulation depth (0.0 to 1.0)
        phase: f32,    // LFO phase accumulator
    },

    /// Vibrato (pitch modulation)
    /// Classic effect that modulates pitch with an LFO using time-varying delay
    Vibrato {
        input: Signal,          // Input signal
        rate: Signal,           // LFO rate in Hz (0.1 to 20.0)
        depth: Signal,          // Modulation depth in semitones (0.0 to 2.0)
        phase: f32,             // LFO phase accumulator
        delay_buffer: Vec<f32>, // Circular delay buffer (50ms)
        buffer_pos: usize,      // Current write position in buffer
    },

    /// Phaser (spectral sweeping via allpass filter cascade)
    /// Classic effect that creates moving notches in the frequency spectrum
    Phaser {
        input: Signal,        // Input signal
        rate: Signal,         // LFO rate in Hz (0.05 to 5.0)
        depth: Signal,        // Modulation depth (0.0 to 1.0)
        feedback: Signal,     // Feedback amount (0.0 to 0.95)
        stages: usize,        // Number of allpass filter stages (2 to 12)
        phase: f32,           // LFO phase accumulator
        allpass_z1: Vec<f32>, // Previous input per stage
        allpass_y1: Vec<f32>, // Previous output per stage
        feedback_sample: f32, // Feedback buffer
    },

    /// Ring Modulation
    /// Classic effect that multiplies input by a carrier frequency
    /// Creates metallic, inharmonic tones
    RingMod {
        input: Signal, // Input signal
        freq: Signal,  // Carrier frequency in Hz (20.0 to 5000.0)
        phase: f32,    // Carrier oscillator phase
    },

    /// FM Cross-Modulation
    /// Phase modulation using any audio signal as modulator
    /// Formula: carrier * cos(2π * mod_depth * modulator)
    /// Use cases: drums modulating bass, LFO modulating pad, etc.
    FMCrossMod {
        carrier: Signal,   // Carrier signal to be modulated
        modulator: Signal, // Modulator signal (any audio)
        mod_depth: Signal, // Modulation depth/intensity
    },

    /// fundsp Unit Wrapper (wraps fundsp AudioUnit for pattern modulation)
    /// Allows using fundsp's 60+ battle-tested UGens with Phonon's pattern system
    /// Pattern signals can modulate fundsp parameters at audio rate
    FundspUnit {
        unit_type: FundspUnitType,      // Which fundsp unit this is
        inputs: Vec<Signal>,            // All inputs: [audio_input?, param1?, param2?, ...]
        state: Arc<Mutex<FundspState>>, // Thread-safe shared mutable fundsp unit state
    },

    /// Tap/Probe - Records signal to buffer for debugging
    /// Passes signal through unchanged while recording to file
    /// Useful for debugging signal flow and analyzing what's happening at different points
    Tap {
        input: Signal,               // Input signal to record
        state: Arc<Mutex<TapState>>, // Shared mutable state for recording
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
    /// Pink noise generator (1/f spectrum)
    Pink,
    /// Pulse wave oscillator with variable pulse width (PWM)
    Pulse,
    /// Phaser effect (frequency-domain comb filtering)
    Phaser,
    /// Nonlinear lowpass filter (Jatin Chowdhury's design)
    DLowpassHz,
    /// Soft sawtooth oscillator (fewer harmonics than regular saw)
    SoftSawHz,
}

/// fundsp State Wrapper
/// Uses a tick function pointer to avoid complex generic types
/// This allows us to store fundsp units without exposing their concrete types
pub struct FundspState {
    /// Function that processes one sample (now supports multiple inputs)
    tick_fn: Box<dyn FnMut(&[f32]) -> f32 + Send>,
    /// Type of the unit (for debugging and parameter updates)
    unit_type: FundspUnitType,
    /// Number of inputs this unit expects (0 = generator, 1+ = processor/multi-input)
    num_inputs: usize,
    /// Current parameters (for recreation if needed)
    params: Vec<f32>,
    sample_rate: f64,
}

impl FundspState {
    /// Create a new organ_hz unit
    pub fn new_organ_hz(frequency: f32, sample_rate: f64) -> Self {
        let mut unit = fundsp::prelude::organ_hz(frequency);
        unit.reset();
        unit.set_sample_rate(sample_rate);

        // Create a closure that owns the unit and calls tick
        let tick_fn = Box::new(move |_inputs: &[f32]| -> f32 {
            // Generator: ignores inputs
            let output_frame = unit.tick(&Default::default());
            output_frame[0]
        });

        Self {
            tick_fn,
            unit_type: FundspUnitType::OrganHz,
            num_inputs: 0, // Generator (no inputs)
            params: vec![frequency],
            sample_rate,
        }
    }

    /// Create a new moog_hz unit (Moog ladder filter)
    pub fn new_moog_hz(cutoff: f32, resonance: f32, sample_rate: f64) -> Self {
        let mut unit = fundsp::prelude::moog_hz(cutoff, resonance);
        unit.reset();
        unit.set_sample_rate(sample_rate);

        // Create a closure that owns the unit and calls tick
        let tick_fn = Box::new(move |inputs: &[f32]| -> f32 {
            // Processor: takes 1 audio input
            let audio_input = inputs.get(0).copied().unwrap_or(0.0);
            // moog_hz takes 1 input, returns 1 output
            let output_frame = unit.tick(&[audio_input].into());
            output_frame[0]
        });

        Self {
            tick_fn,
            unit_type: FundspUnitType::MoogHz,
            num_inputs: 1, // Processor (1 audio input)
            params: vec![cutoff, resonance],
            sample_rate,
        }
    }

    /// Create a new reverb_stereo unit (Stereo reverb - stereo in, stereo out)
    pub fn new_reverb_stereo(wet: f32, time: f32, sample_rate: f64) -> Self {
        // reverb_stereo takes (wet, time, diffusion) and expects stereo input
        // Convert parameters to f64 for fundsp
        let diffusion = 0.5; // Fixed diffusion parameter
        let mut unit = fundsp::prelude::reverb_stereo(wet as f64, time as f64, diffusion);
        unit.reset();
        unit.set_sample_rate(sample_rate);

        // Create a closure that owns the unit and calls tick
        let tick_fn = Box::new(move |inputs: &[f32]| -> f32 {
            // Processor: takes 1 audio input
            let audio_input = inputs.get(0).copied().unwrap_or(0.0);
            // reverb_stereo: 2 inputs (stereo) -> 2 outputs (stereo)
            // Convert mono to stereo input, return left channel
            let output_frame = unit.tick(&[audio_input, audio_input].into());
            output_frame[0] // Left channel only
        });

        Self {
            tick_fn,
            unit_type: FundspUnitType::ReverbStereo,
            num_inputs: 1, // Processor (1 audio input)
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
        let mut unit = fundsp::prelude::chorus(seed, separation, variation, mod_frequency);
        unit.reset();
        unit.set_sample_rate(sample_rate);

        // Create a closure that owns the unit and calls tick
        let tick_fn = Box::new(move |inputs: &[f32]| -> f32 {
            // Processor: takes 1 audio input
            let audio_input = inputs.get(0).copied().unwrap_or(0.0);
            // chorus: 1 input -> 1 output
            let output_frame = unit.tick(&[audio_input].into());
            output_frame[0]
        });

        Self {
            tick_fn,
            unit_type: FundspUnitType::Chorus,
            num_inputs: 1, // Processor (1 audio input)
            params: vec![seed as f32, separation, variation, mod_frequency],
            sample_rate,
        }
    }

    /// Create a new saw_hz unit (bandlimited sawtooth oscillator)
    pub fn new_saw_hz(frequency: f32, sample_rate: f64) -> Self {
        let mut unit = fundsp::prelude::saw_hz(frequency);
        unit.reset();
        unit.set_sample_rate(sample_rate);

        // Create a closure that owns the unit and calls tick
        let tick_fn = Box::new(move |_inputs: &[f32]| -> f32 {
            // saw_hz: 0 inputs -> 1 output (generator)
            let output_frame = unit.tick(&Default::default());
            output_frame[0]
        });

        Self {
            tick_fn,
            unit_type: FundspUnitType::SawHz,
            params: vec![frequency],
            num_inputs: 0, // Generator (no inputs)
            sample_rate,
        }
    }

    /// Create a new soft_saw_hz unit (softer sawtooth with fewer harmonics)
    pub fn new_soft_saw_hz(frequency: f32, sample_rate: f64) -> Self {
        let mut unit = fundsp::prelude::soft_saw_hz(frequency);
        unit.reset();
        unit.set_sample_rate(sample_rate);

        // Create a closure that owns the unit and calls tick
        let tick_fn = Box::new(move |_inputs: &[f32]| -> f32 {
            // soft_saw_hz: 0 inputs -> 1 output (generator)
            let output_frame = unit.tick(&Default::default());
            output_frame[0]
        });

        Self {
            tick_fn,
            unit_type: FundspUnitType::SoftSawHz,
            params: vec![frequency],
            num_inputs: 0, // Generator (no inputs)
            sample_rate,
        }
    }

    /// Create a new square_hz unit (bandlimited square wave oscillator)
    pub fn new_square_hz(frequency: f32, sample_rate: f64) -> Self {
        let mut unit = fundsp::prelude::square_hz(frequency);
        unit.reset();
        unit.set_sample_rate(sample_rate);

        // Create a closure that owns the unit and calls tick
        let tick_fn = Box::new(move |_inputs: &[f32]| -> f32 {
            // Generator: ignores inputs
            let output_frame = unit.tick(&Default::default());
            output_frame[0]
        });

        Self {
            tick_fn,
            unit_type: FundspUnitType::SquareHz,
            num_inputs: 0, // Generator (no inputs)
            params: vec![frequency],
            sample_rate,
        }
    }

    pub fn new_triangle_hz(frequency: f32, sample_rate: f64) -> Self {
        let mut unit = fundsp::prelude::triangle_hz(frequency);
        unit.reset();
        unit.set_sample_rate(sample_rate);

        // Create a closure that owns the unit and calls tick
        let tick_fn = Box::new(move |_inputs: &[f32]| -> f32 {
            // Generator: ignores inputs
            let output_frame = unit.tick(&Default::default());
            output_frame[0]
        });

        Self {
            tick_fn,
            num_inputs: 0, // Generator (no inputs)
            unit_type: FundspUnitType::TriangleHz,
            params: vec![frequency],
            sample_rate,
        }
    }

    pub fn new_noise(sample_rate: f64) -> Self {
        let mut unit = fundsp::prelude::noise();
        unit.reset();
        unit.set_sample_rate(sample_rate);

        // Create a closure that owns the unit and calls tick
        let tick_fn = Box::new(move |_inputs: &[f32]| -> f32 {
            // Generator: ignores inputs
            let output_frame = unit.tick(&Default::default());
            output_frame[0]
        });

        Self {
            num_inputs: 0, // Generator (no inputs)
            tick_fn,
            unit_type: FundspUnitType::Noise,
            params: vec![], // No parameters!
            sample_rate,
        }
    }

    /// Create a new pink noise unit (1/f spectrum)
    pub fn new_pink(sample_rate: f64) -> Self {
        // pink::<f32>() requires type annotation
        let mut unit = fundsp::prelude::pink::<f32>();
        unit.reset();
        unit.set_sample_rate(sample_rate);

        // Create a closure that owns the unit and calls tick
        let tick_fn = Box::new(move |_inputs: &[f32]| -> f32 {
            // Generator: ignores inputs
            let output_frame = unit.tick(&Default::default());
            output_frame[0]
        });

        Self {
            tick_fn,
            unit_type: FundspUnitType::Pink,
            num_inputs: 0,  // Generator (no inputs)
            params: vec![], // No parameters!
            sample_rate,
        }
    }

    /// Create pulse wave oscillator with audio-rate frequency and pulse width
    ///
    /// Inputs: [frequency (Hz), pulse_width (0.0 to 1.0)]
    ///
    /// Unlike saw_hz/square_hz which have static frequency parameters,
    /// pulse() takes both frequency and pulse width as audio-rate inputs,
    /// enabling audio-rate pulse width modulation (PWM).
    pub fn new_pulse(sample_rate: f64) -> Self {
        // pulse() takes 2 audio-rate inputs
        let mut unit = fundsp::prelude::pulse();
        unit.reset();
        unit.set_sample_rate(sample_rate);

        // Create a closure that owns the unit and calls tick with inputs
        let tick_fn = Box::new(move |inputs: &[f32]| -> f32 {
            // Multi-input UGen: expects [frequency, pulse_width]
            let freq = inputs.get(0).copied().unwrap_or(440.0);
            let width = inputs.get(1).copied().unwrap_or(0.5);

            let output_frame = unit.tick(&[freq, width].into());
            output_frame[0]
        });

        Self {
            tick_fn,
            unit_type: FundspUnitType::Pulse,
            num_inputs: 2,  // Multi-input (frequency + pulse_width)
            params: vec![], // No static parameters (all audio-rate)
            sample_rate,
        }
    }

    /// Process one sample through the fundsp unit
    /// Now takes a slice of inputs to support multi-input UGens
    pub fn tick(&mut self, inputs: &[f32]) -> f32 {
        (self.tick_fn)(inputs)
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
            FundspUnitType::Pink => Self::new_pink(self.sample_rate),
            FundspUnitType::Pulse => Self::new_pulse(self.sample_rate),
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
    // Cached coefficients for SVF (Chamberlin) - avoid sin() every sample
    pub cached_fc: f32,   // Last cutoff frequency used
    pub cached_q: f32,    // Last Q value used
    pub cached_f: f32,    // Cached frequency coefficient
    pub cached_damp: f32, // Cached damping coefficient
}

impl Default for FilterState {
    fn default() -> Self {
        Self {
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
            cached_fc: -1.0, // Invalid value to force initial computation
            cached_q: -1.0,
            cached_f: 0.0,
            cached_damp: 1.0,
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

/// SVF (State Variable Filter) state
/// Chamberlin topology for multi-mode filtering
#[derive(Debug, Clone)]
pub struct SVFState {
    pub low: f32,  // Lowpass integrator state
    pub band: f32, // Bandpass integrator state
}

impl Default for SVFState {
    fn default() -> Self {
        Self {
            low: 0.0,
            band: 0.0,
        }
    }
}

/// Biquad Filter state
/// High-quality second-order IIR filter (uses `biquad` crate)
/// Stores filter coefficients and internal state
#[derive(Debug, Clone)]
pub struct BiquadState {
    pub x1: f32, // Previous input sample 1
    pub x2: f32, // Previous input sample 2
    pub y1: f32, // Previous output sample 1
    pub y2: f32, // Previous output sample 2
    pub b0: f32, // Feedforward coefficient 0
    pub b1: f32, // Feedforward coefficient 1
    pub b2: f32, // Feedforward coefficient 2
    pub a1: f32, // Feedback coefficient 1
    pub a2: f32, // Feedback coefficient 2
}

impl Default for BiquadState {
    fn default() -> Self {
        Self {
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        }
    }
}

/// Envelope state
#[derive(Debug, Clone)]
pub struct EnvState {
    phase: RefCell<EnvPhase>,
    level: RefCell<f32>,
    time_in_phase: RefCell<f32>,
    release_start_level: RefCell<f32>, // Level when release phase began
}

#[derive(Debug, Clone)]
pub struct ADSRState {
    phase: RefCell<ADSRPhase>,
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
            phase: RefCell::new(ADSRPhase::Attack),
            level: 0.0,
            cycle_pos: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ADState {
    phase: RefCell<ADPhase>,
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
            phase: RefCell::new(ADPhase::Attack),
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
            phase: RefCell::new(EnvPhase::Idle),
            level: RefCell::new(0.0),
            time_in_phase: RefCell::new(0.0),
            release_start_level: RefCell::new(0.0),
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

/// Dattorro Reverb State
/// Based on Jon Dattorro's 1997 AES paper "Effect Design, Part 1: Reverberator and Other Filters"
/// Figure-8 feedback delay network with modulated allpass filters
#[derive(Debug, Clone)]
pub struct DattorroState {
    // Pre-delay
    predelay_buffer: Vec<f32>,
    predelay_idx: usize,

    // Input diffusion (4 allpass filters)
    input_diffusion_buffers: [Vec<f32>; 4],
    input_diffusion_indices: [usize; 4],

    // Left tank (decay diffusion network 1)
    left_apf1_buffer: Vec<f32>,
    left_apf1_idx: usize,
    left_delay1_buffer: Vec<f32>,
    left_delay1_idx: usize,
    left_apf2_buffer: Vec<f32>,
    left_apf2_idx: usize,
    left_delay2_buffer: Vec<f32>,
    left_delay2_idx: usize,
    left_lpf_state: f32, // One-pole lowpass for damping

    // Right tank (decay diffusion network 2)
    right_apf1_buffer: Vec<f32>,
    right_apf1_idx: usize,
    right_delay1_buffer: Vec<f32>,
    right_delay1_idx: usize,
    right_apf2_buffer: Vec<f32>,
    right_apf2_idx: usize,
    right_delay2_buffer: Vec<f32>,
    right_delay2_idx: usize,
    right_lpf_state: f32,

    // Modulation LFOs
    lfo_phase: f32,

    sample_rate: f32,
}

impl DattorroState {
    pub fn new(sample_rate: f32) -> Self {
        let sr = sample_rate;

        // Dattorro delay line lengths (in samples at given sample rate)
        // Scaled from original 29.7kHz design
        let scale = sr / 29761.0;

        let predelay_samples = (sr * 0.5) as usize; // 500ms max pre-delay

        // Input diffusion allpass delays
        let input_diffusion_lengths = [
            (142.0 * scale) as usize,
            (107.0 * scale) as usize,
            (379.0 * scale) as usize,
            (277.0 * scale) as usize,
        ];

        // Left tank delays
        let left_apf1_len = (672.0 * scale) as usize;
        let left_delay1_len = (4453.0 * scale) as usize;
        let left_apf2_len = (1800.0 * scale) as usize;
        let left_delay2_len = (3720.0 * scale) as usize;

        // Right tank delays (slightly detuned for stereo spread)
        let right_apf1_len = (908.0 * scale) as usize;
        let right_delay1_len = (4217.0 * scale) as usize;
        let right_apf2_len = (2656.0 * scale) as usize;
        let right_delay2_len = (3163.0 * scale) as usize;

        Self {
            predelay_buffer: vec![0.0; predelay_samples],
            predelay_idx: 0,

            input_diffusion_buffers: [
                vec![0.0; input_diffusion_lengths[0]],
                vec![0.0; input_diffusion_lengths[1]],
                vec![0.0; input_diffusion_lengths[2]],
                vec![0.0; input_diffusion_lengths[3]],
            ],
            input_diffusion_indices: [0; 4],

            left_apf1_buffer: vec![0.0; left_apf1_len],
            left_apf1_idx: 0,
            left_delay1_buffer: vec![0.0; left_delay1_len],
            left_delay1_idx: 0,
            left_apf2_buffer: vec![0.0; left_apf2_len],
            left_apf2_idx: 0,
            left_delay2_buffer: vec![0.0; left_delay2_len],
            left_delay2_idx: 0,
            left_lpf_state: 0.0,

            right_apf1_buffer: vec![0.0; right_apf1_len],
            right_apf1_idx: 0,
            right_delay1_buffer: vec![0.0; right_delay1_len],
            right_delay1_idx: 0,
            right_apf2_buffer: vec![0.0; right_apf2_len],
            right_apf2_idx: 0,
            right_delay2_buffer: vec![0.0; right_delay2_len],
            right_delay2_idx: 0,
            right_lpf_state: 0.0,

            lfo_phase: 0.0,
            sample_rate: sr,
        }
    }
}

impl Default for DattorroState {
    fn default() -> Self {
        Self::new(44100.0)
    }
}

/// Tape Delay State
#[derive(Debug, Clone)]
pub struct TapeDelayState {
    buffer: Vec<f32>,
    write_idx: usize,
    // LFO phases for wow and flutter
    wow_phase: f32,
    flutter_phase: f32,
    // One-pole lowpass for tape head filtering
    lpf_state: f32,
    sample_rate: f32,
}

impl TapeDelayState {
    pub fn new(sample_rate: f32) -> Self {
        let buffer_size = sample_rate as usize; // 1 second max
        Self {
            buffer: vec![0.0; buffer_size],
            write_idx: 0,
            wow_phase: 0.0,
            flutter_phase: 0.0,
            lpf_state: 0.0,
            sample_rate,
        }
    }
}

impl Default for TapeDelayState {
    fn default() -> Self {
        Self::new(44100.0)
    }
}

/// Bitcrusher state
#[derive(Debug, Clone)]
pub struct BitCrushState {
    phase: RefCell<f32>,
    last_sample: RefCell<f32>,
}

impl Default for BitCrushState {
    fn default() -> Self {
        Self {
            phase: RefCell::new(0.0),
            last_sample: RefCell::new(0.0),
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
    bins: [f32; 16], // 16 octave bins for quality pink noise
    counter: u32,    // Sample counter for bin update decisions
}

impl PinkNoiseState {
    pub fn new() -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut bins = [0.0f32; 16];
        for bin in &mut bins {
            *bin = rng.gen_range(-1.0..1.0);
        }
        Self { bins, counter: 0 }
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
        Self { accumulator: 0.0 }
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

/// Wavetable oscillator state
/// Reads through a stored waveform at variable speeds for different pitches
#[derive(Debug, Clone)]
pub struct WavetableState {
    table: Vec<f32>, // Wavetable data (one cycle)
    phase: f32,      // Current phase position [0, 1)
}

impl WavetableState {
    pub fn new() -> Self {
        // Default: 2048-sample sine wave for high quality
        let table_size = 2048;
        let mut table = Vec::with_capacity(table_size);

        for i in 0..table_size {
            let phase = i as f32 / table_size as f32;
            table.push((phase * 2.0 * std::f32::consts::PI).sin());
        }

        Self { table, phase: 0.0 }
    }

    /// Create wavetable with custom waveform
    pub fn with_table(table: Vec<f32>) -> Self {
        Self { table, phase: 0.0 }
    }

    /// Get interpolated sample at given phase [0, 1)
    pub fn get_sample(&self, phase: f32) -> f32 {
        // Handle empty table
        if self.table.is_empty() {
            return 0.0;
        }

        let table_size = self.table.len() as f32;
        let index = (phase * table_size) % table_size;
        let i0 = index.floor() as usize % self.table.len();
        let i1 = (i0 + 1) % self.table.len();
        let frac = index.fract();

        // Linear interpolation
        self.table[i0] * (1.0 - frac) + self.table[i1] * frac
    }
}

impl Default for WavetableState {
    fn default() -> Self {
        Self::new()
    }
}

/// Individual grain for granular synthesis
#[derive(Debug, Clone)]
pub struct Grain {
    position: f32,          // Read position in source buffer (samples)
    playback_rate: f32,     // Speed/pitch multiplier (1.0 = normal)
    age_samples: usize,     // How many samples this grain has played
    grain_length: usize,    // Total length of this grain in samples
    window_table: Vec<f32>, // Pre-computed Hann window values
}

impl Grain {
    pub fn new(position: f32, playback_rate: f32, grain_length: usize) -> Self {
        // Pre-compute Hann window for this grain
        // Hann window: 0.5 * (1 - cos(2π * t))
        let window_table: Vec<f32> = (0..grain_length)
            .map(|i| {
                let t = i as f32 / grain_length as f32;
                0.5 * (1.0 - (2.0 * std::f32::consts::PI * t).cos())
            })
            .collect();

        Self {
            position,
            playback_rate,
            age_samples: 0,
            grain_length,
            window_table,
        }
    }

    /// Get windowed sample from this grain (Hann window)
    pub fn get_sample(&self, source_buffer: &[f32]) -> f32 {
        if self.age_samples >= self.grain_length {
            return 0.0; // Grain finished
        }

        // Use pre-computed window value (PERFORMANCE FIX)
        let window = self.window_table[self.age_samples];

        // Get sample from source buffer with linear interpolation
        let buffer_len = source_buffer.len();
        if buffer_len == 0 {
            return 0.0;
        }

        let index = (self.position as usize) % buffer_len;
        let sample = source_buffer[index];

        sample * window
    }

    /// Advance grain by one sample
    pub fn advance(&mut self) {
        self.position += self.playback_rate;
        self.age_samples += 1;
    }

    /// Check if grain is finished
    pub fn is_finished(&self) -> bool {
        self.age_samples >= self.grain_length
    }
}

/// Granular synthesis state
/// Breaks audio into small grains and overlaps them with varying parameters
#[derive(Debug, Clone)]
pub struct GranularState {
    source_buffer: Vec<f32>,   // Circular buffer storing source audio
    buffer_write_pos: usize,   // Current write position in buffer
    active_grains: Vec<Grain>, // Currently playing grains
    grain_spawn_phase: f32,    // Phase for spawning new grains [0, 1)
}

impl GranularState {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            source_buffer: vec![0.0; buffer_size],
            buffer_write_pos: 0,
            active_grains: Vec::new(),
            grain_spawn_phase: 0.0,
        }
    }

    /// Write a sample to the source buffer
    pub fn write_sample(&mut self, sample: f32) {
        self.source_buffer[self.buffer_write_pos] = sample;
        self.buffer_write_pos = (self.buffer_write_pos + 1) % self.source_buffer.len();
    }

    /// Spawn a new grain at current position
    pub fn spawn_grain(&mut self, grain_size_samples: usize, playback_rate: f32) {
        // PERFORMANCE: Limit max active grains to prevent exponential slowdown
        // With very high density (0.9+), thousands of grains can accumulate
        const MAX_ACTIVE_GRAINS: usize = 128;

        if self.active_grains.len() >= MAX_ACTIVE_GRAINS {
            return; // Skip grain spawn if at limit
        }

        // Random position in buffer for variety
        let position = (self.buffer_write_pos as f32 * 0.8) % self.source_buffer.len() as f32;
        let grain = Grain::new(position, playback_rate, grain_size_samples);
        self.active_grains.push(grain);
    }

    /// Get mixed output from all active grains
    pub fn get_sample(&mut self) -> f32 {
        let mut output = 0.0;
        let count = self.active_grains.len() as f32;

        // Mix all active grains
        for grain in &self.active_grains {
            output += grain.get_sample(&self.source_buffer);
        }

        // Normalize by grain count to prevent clipping
        // Linear normalization: with N overlapping grains, divide by N
        if count > 0.0 {
            output / count
        } else {
            0.0
        }
    }

    /// Advance all grains and remove finished ones
    pub fn advance(&mut self) {
        for grain in &mut self.active_grains {
            grain.advance();
        }

        // Remove finished grains
        self.active_grains.retain(|g| !g.is_finished());
    }
}

impl Default for GranularState {
    fn default() -> Self {
        // Default: 2 second buffer at 44.1kHz
        Self::new(88200)
    }
}

/// Karplus-Strong string synthesis state
/// Physical modeling of plucked strings using delay line + lowpass filter
#[derive(Debug, Clone)]
pub struct KarplusStrongState {
    delay_line: Vec<f32>, // Circular buffer for string simulation
    write_pos: usize,     // Current write position
    initialized: bool,    // Whether delay line has been filled with noise
}

impl KarplusStrongState {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            delay_line: vec![0.0; buffer_size.max(2)], // Minimum 2 samples
            write_pos: 0,
            initialized: false,
        }
    }

    /// Initialize delay line with noise (simulates initial pluck)
    pub fn initialize_with_noise(&mut self) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        for sample in &mut self.delay_line {
            *sample = rng.gen_range(-1.0..1.0);
        }
        self.initialized = true;
        self.write_pos = 0;
    }

    /// Get sample from delay line with averaging (Karplus-Strong algorithm)
    pub fn get_sample(&mut self, damping: f32) -> f32 {
        if !self.initialized {
            self.initialize_with_noise();
        }

        let len = self.delay_line.len();
        let current_pos = self.write_pos;
        let prev_pos = if current_pos == 0 {
            len - 1
        } else {
            current_pos - 1
        };

        // Karplus-Strong algorithm: average current + previous sample
        // Damping: 0.0 = long sustain (low energy loss), 1.0 = short sustain (high energy loss)
        let current = self.delay_line[current_pos];
        let previous = self.delay_line[prev_pos];
        let averaged = (current + previous) * 0.5;

        // Energy retention factor: higher damping = less energy retained (faster decay)
        // Keep values close to 1.0 for longer sustain
        let energy_retention = 0.9995 - (damping * 0.001); // Range: 0.9995 (no damp) to 0.9985 (max damp)
        let output = averaged * energy_retention;

        // Write back to delay line
        self.delay_line[self.write_pos] = output;

        // Advance write position
        self.write_pos = (self.write_pos + 1) % len;

        output
    }

    /// Resize delay line (for frequency changes)
    pub fn resize(&mut self, new_size: usize) {
        let new_size = new_size.max(2); // Minimum 2 samples
        if new_size != self.delay_line.len() {
            self.delay_line.resize(new_size, 0.0);
            self.write_pos = 0;
            self.initialized = false; // Will re-initialize on next sample
        }
    }
}

impl Default for KarplusStrongState {
    fn default() -> Self {
        // Default: 100 samples (440Hz at 44.1kHz)
        Self::new(100)
    }
}

/// Digital Waveguide Physical Modeling state
/// Uses bidirectional delay lines to simulate wave propagation in physical media
/// More sophisticated than Karplus-Strong, can model various acoustic instruments
#[derive(Debug, Clone)]
pub struct WaveguideState {
    forward_delay: Vec<f32>,  // Forward-propagating wave
    backward_delay: Vec<f32>, // Backward-propagating wave
    forward_pos: usize,       // Write position for forward delay
    backward_pos: usize,      // Write position for backward delay
    initialized: bool,        // Whether delay lines have been filled with noise
}

impl WaveguideState {
    pub fn new(delay_length: usize) -> Self {
        let delay_length = delay_length.max(2); // Minimum 2 samples
        Self {
            forward_delay: vec![0.0; delay_length],
            backward_delay: vec![0.0; delay_length],
            forward_pos: 0,
            backward_pos: 0,
            initialized: false,
        }
    }

    /// Initialize delay lines with noise (simulates initial pluck/bow)
    pub fn initialize_with_noise(&mut self) {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        for sample in &mut self.forward_delay {
            *sample = rng.gen_range(-1.0..1.0);
        }
        for sample in &mut self.backward_delay {
            *sample = rng.gen_range(-1.0..1.0);
        }

        self.initialized = true;
    }

    /// Get sample from waveguide at given pickup position and damping
    /// pickup_position: 0.0 to 1.0 (where along string to read)
    /// damping: 0.0 to 1.0 (energy loss at boundaries)
    pub fn get_sample(&mut self, pickup_position: f32, damping: f32) -> f32 {
        if !self.initialized {
            self.initialize_with_noise();
        }

        let len = self.forward_delay.len();

        // Clamp pickup position to valid range
        let pickup_pos = pickup_position.clamp(0.0, 1.0);

        // Calculate read positions for pickup
        let pickup_idx = (pickup_pos * (len - 1) as f32) as usize;

        // Read from both delay lines at pickup position
        let forward_sample = self.forward_delay[pickup_idx];
        let backward_sample = self.backward_delay[pickup_idx];

        // Output is sum of both waves at pickup point
        let output = (forward_sample + backward_sample) * 0.5;

        // Calculate reflection coefficient (energy retention at boundaries)
        // Higher damping = more energy loss
        let reflection_coeff = 0.999 - (damping * 0.002);

        // Read from ends of delay lines for reflection
        let forward_end = self.forward_delay[self.forward_pos];
        let backward_end = self.backward_delay[self.backward_pos];

        // Reflect with damping: forward wave becomes backward wave (and vice versa)
        // Simple lowpass: average with previous sample for damping
        let forward_prev_pos = if self.forward_pos == 0 {
            len - 1
        } else {
            self.forward_pos - 1
        };
        let backward_prev_pos = if self.backward_pos == 0 {
            len - 1
        } else {
            self.backward_pos - 1
        };

        let forward_prev = self.forward_delay[forward_prev_pos];
        let backward_prev = self.backward_delay[backward_prev_pos];

        // Average for lowpass filtering effect
        let forward_averaged = (forward_end + forward_prev) * 0.5;
        let backward_averaged = (backward_end + backward_prev) * 0.5;

        // Write reflected waves with damping
        self.forward_delay[self.forward_pos] = -backward_averaged * reflection_coeff;
        self.backward_delay[self.backward_pos] = -forward_averaged * reflection_coeff;

        // Advance write positions
        self.forward_pos = (self.forward_pos + 1) % len;
        self.backward_pos = (self.backward_pos + 1) % len;

        output
    }

    /// Resize delay lines (for frequency changes)
    pub fn resize(&mut self, new_size: usize) {
        let new_size = new_size.max(2); // Minimum 2 samples
        if new_size != self.forward_delay.len() {
            self.forward_delay.resize(new_size, 0.0);
            self.backward_delay.resize(new_size, 0.0);
            self.forward_pos = 0;
            self.backward_pos = 0;
            self.initialized = false; // Will re-initialize on next sample
        }
    }
}

impl Default for WaveguideState {
    fn default() -> Self {
        // Default: 100 samples (440Hz at 44.1kHz)
        Self::new(100)
    }
}

/// Formant Synthesis state
/// Filters source signal through three resonant bandpass filters (formants)
/// to create vowel sounds. Each vowel is characterized by specific formant
/// frequencies that resonate in the vocal tract.
///
/// Uses Chamberlin state variable filters (same as BandPass node) for each formant.
///
/// Common vowel formants (male voice, Hz):
/// - /a/ (father): F1=730, F2=1090, F3=2440
/// - /e/ (bet):    F1=530, F2=1840, F3=2480
/// - /i/ (beet):   F1=270, F2=2290, F3=3010
/// - /o/ (boat):   F1=570, F2=840,  F3=2410
/// - /u/ (boot):   F1=300, F2=870,  F3=2240
#[derive(Debug, Clone)]
pub struct FormantState {
    /// State variable filter states for each formant
    state1: FilterState,
    state2: FilterState,
    state3: FilterState,

    sample_rate: f32,

    // PERFORMANCE OPTIMIZATION: Cache last formant frequencies and computed coefficients
    // Only recompute expensive sin() when frequencies change
    last_f1: f32,
    last_f2: f32,
    last_f3: f32,
    last_bw1: f32,
    last_bw2: f32,
    last_bw3: f32,
    cached_f_1: f32,
    cached_f_2: f32,
    cached_f_3: f32,
    cached_damp1: f32,
    cached_damp2: f32,
    cached_damp3: f32,
}

impl FormantState {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            state1: FilterState::default(),
            state2: FilterState::default(),
            state3: FilterState::default(),
            sample_rate,
            last_f1: -1.0,
            last_f2: -1.0,
            last_f3: -1.0,
            last_bw1: -1.0,
            last_bw2: -1.0,
            last_bw3: -1.0,
            cached_f_1: 0.0,
            cached_f_2: 0.0,
            cached_f_3: 0.0,
            cached_damp1: 0.0,
            cached_damp2: 0.0,
            cached_damp3: 0.0,
        }
    }

    /// Process input through three formant bandpass filters
    /// Uses Chamberlin state variable filter (same as BandPass node)
    pub fn process(
        &mut self,
        input: f32,
        f1: f32,
        f2: f32,
        f3: f32,
        bw1: f32,
        bw2: f32,
        bw3: f32,
    ) -> f32 {
        use std::f32::consts::PI;

        // PERFORMANCE FIX: Only recompute coefficients when formants change
        // Formant 1
        let (f_1, damp1) = if f1 != self.last_f1 || bw1 != self.last_bw1 {
            let q1 = f1 / bw1.max(1.0);
            let f_1 = 2.0 * (PI * f1 / self.sample_rate).sin();
            let damp1 = 1.0 / q1.max(0.5);
            self.last_f1 = f1;
            self.last_bw1 = bw1;
            self.cached_f_1 = f_1;
            self.cached_damp1 = damp1;
            (f_1, damp1)
        } else {
            (self.cached_f_1, self.cached_damp1)
        };

        let mut low1 = self.state1.y1;
        let mut band1 = self.state1.x1;
        let mut high1 = self.state1.y2;

        high1 = input - low1 - damp1 * band1;
        band1 += f_1 * high1;
        low1 += f_1 * band1;

        self.state1.y1 = low1;
        self.state1.x1 = band1;
        self.state1.y2 = high1;

        // Formant 2
        let (f_2, damp2) = if f2 != self.last_f2 || bw2 != self.last_bw2 {
            let q2 = f2 / bw2.max(1.0);
            let f_2 = 2.0 * (PI * f2 / self.sample_rate).sin();
            let damp2 = 1.0 / q2.max(0.5);
            self.last_f2 = f2;
            self.last_bw2 = bw2;
            self.cached_f_2 = f_2;
            self.cached_damp2 = damp2;
            (f_2, damp2)
        } else {
            (self.cached_f_2, self.cached_damp2)
        };

        let mut low2 = self.state2.y1;
        let mut band2 = self.state2.x1;
        let mut high2 = self.state2.y2;

        high2 = input - low2 - damp2 * band2;
        band2 += f_2 * high2;
        low2 += f_2 * band2;

        self.state2.y1 = low2;
        self.state2.x1 = band2;
        self.state2.y2 = high2;

        // Formant 3
        let (f_3, damp3) = if f3 != self.last_f3 || bw3 != self.last_bw3 {
            let q3 = f3 / bw3.max(1.0);
            let f_3 = 2.0 * (PI * f3 / self.sample_rate).sin();
            let damp3 = 1.0 / q3.max(0.5);
            self.last_f3 = f3;
            self.last_bw3 = bw3;
            self.cached_f_3 = f_3;
            self.cached_damp3 = damp3;
            (f_3, damp3)
        } else {
            (self.cached_f_3, self.cached_damp3)
        };

        let mut low3 = self.state3.y1;
        let mut band3 = self.state3.x1;
        let mut high3 = self.state3.y2;

        high3 = input - low3 - damp3 * band3;
        band3 += f_3 * high3;
        low3 += f_3 * band3;

        self.state3.y1 = low3;
        self.state3.x1 = band3;
        self.state3.y2 = high3;

        // Sum the three bandpass outputs (formants)
        // Weight them to balance energy across frequency ranges
        (band1 + band2 + band3) * 0.5
    }
}

impl Default for FormantState {
    fn default() -> Self {
        Self::new(44100.0)
    }
}

/// Additive Synthesis state
/// Creates complex timbres by summing multiple sine waves (partials/harmonics)
/// Each partial has independent amplitude control based on the amplitude pattern
///
/// Classic additive synthesis: fundamental + harmonics weighted by amplitudes
/// Example: additive 440 "1.0 0.5 0.25" creates 440Hz + 880Hz + 1320Hz
#[derive(Debug, Clone)]
pub struct AdditiveState {
    phase: f32,       // Phase accumulator [0, 1)
    sample_rate: f32, // Sample rate for phase increment calculation
}

impl AdditiveState {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            phase: 0.0,
            sample_rate,
        }
    }

    /// Generate one sample of additive synthesis
    /// freq: fundamental frequency (Hz)
    /// amplitudes: amplitude for each partial (partial 1, 2, 3, ...)
    pub fn process(&mut self, freq: f32, amplitudes: &[f32]) -> f32 {
        use std::f32::consts::PI;

        // Calculate phase increment for fundamental
        let phase_inc = freq / self.sample_rate;

        // Sum all partials
        let mut output = 0.0;
        let amp_sum: f32 = amplitudes.iter().sum();
        let norm_factor = if amp_sum > 0.0 { amp_sum } else { 1.0 };

        for (i, &amp) in amplitudes.iter().enumerate() {
            let partial_num = (i + 1) as f32; // Partial 1, 2, 3, ...
            let partial_phase = (self.phase * partial_num).fract(); // Wrap [0, 1)
            output += amp * (2.0 * PI * partial_phase).sin();
        }

        // Advance phase for next sample
        self.phase = (self.phase + phase_inc).fract();

        // Normalize by sum of amplitudes to prevent clipping
        // This keeps relative loudness proportional to total amplitude
        output / norm_factor
    }
}

impl Default for AdditiveState {
    fn default() -> Self {
        Self::new(44100.0)
    }
}

/// Vocoder state
/// Analyzes modulator amplitude in multiple frequency bands and applies
/// those envelopes to carrier bands to create robot voice effects
///
/// Uses bandpass filters to split signals into bands + envelope followers
#[derive(Debug, Clone)]
pub struct VocoderState {
    num_bands: usize,
    /// Bandpass filter states for modulator bands
    modulator_filters: Vec<FilterState>,
    /// Bandpass filter states for carrier bands
    carrier_filters: Vec<FilterState>,
    /// Envelope follower state for each band
    envelopes: Vec<f32>,
    sample_rate: f32,
    /// Pre-calculated filter coefficients (computed once at initialization)
    filter_f: Vec<f32>, // f coefficient for each band
    filter_damp: Vec<f32>, // damp coefficient for each band
}

impl VocoderState {
    pub fn new(num_bands: usize, sample_rate: f32) -> Self {
        use std::f32::consts::PI;

        let num_bands = num_bands.max(2).min(32); // Limit 2-32 bands

        // Pre-calculate filter coefficients for all bands
        let min_freq: f32 = 100.0;
        let max_freq: f32 = 10000.0;
        let freq_ratio = (max_freq / min_freq).powf(1.0 / (num_bands as f32));

        let mut filter_f = Vec::with_capacity(num_bands);
        let mut filter_damp = Vec::with_capacity(num_bands);

        for band in 0..num_bands {
            let center_freq = min_freq * freq_ratio.powi(band as i32);
            let bandwidth = center_freq * 0.5;
            let q = center_freq / bandwidth;
            let f = 2.0 * (PI * center_freq / sample_rate).sin();
            let damp = 1.0 / q.max(0.5);

            filter_f.push(f);
            filter_damp.push(damp);
        }

        Self {
            num_bands,
            modulator_filters: vec![FilterState::default(); num_bands],
            carrier_filters: vec![FilterState::default(); num_bands],
            envelopes: vec![0.0; num_bands],
            sample_rate,
            filter_f,
            filter_damp,
        }
    }

    /// Process one sample through the vocoder
    pub fn process(&mut self, modulator_sample: f32, carrier_sample: f32) -> f32 {
        let mut output = 0.0;

        for band in 0..self.num_bands {
            // Use pre-calculated filter coefficients
            let f = self.filter_f[band];
            let damp = self.filter_damp[band];

            // Filter modulator through bandpass
            let mod_state = &mut self.modulator_filters[band];
            let mut low_mod = mod_state.y1;
            let mut band_mod = mod_state.x1;
            let mut high_mod = mod_state.y2;

            high_mod = modulator_sample - low_mod - damp * band_mod;
            band_mod += f * high_mod;
            low_mod += f * band_mod;

            mod_state.y1 = low_mod;
            mod_state.x1 = band_mod;
            mod_state.y2 = high_mod;

            // Filter carrier through bandpass
            let carr_state = &mut self.carrier_filters[band];
            let mut low_carr = carr_state.y1;
            let mut band_carr = carr_state.x1;
            let mut high_carr = carr_state.y2;

            high_carr = carrier_sample - low_carr - damp * band_carr;
            band_carr += f * high_carr;
            low_carr += f * band_carr;

            carr_state.y1 = low_carr;
            carr_state.x1 = band_carr;
            carr_state.y2 = high_carr;

            // Envelope follower on modulator band (smoothed rectifier)
            let modulator_amplitude = band_mod.abs();
            let attack = 0.01; // Fast attack (10ms)
            let release = 0.1; // Slower release (100ms)

            if modulator_amplitude > self.envelopes[band] {
                // Attack
                self.envelopes[band] += (modulator_amplitude - self.envelopes[band]) * attack;
            } else {
                // Release
                self.envelopes[band] += (modulator_amplitude - self.envelopes[band]) * release;
            }

            // Apply modulator envelope to carrier band
            output += band_carr * self.envelopes[band];
        }

        // Normalize by number of bands
        output / (self.num_bands as f32).sqrt()
    }
}

impl Default for VocoderState {
    fn default() -> Self {
        Self::new(8, 44100.0) // Default: 8 bands
    }
}

/// Pitch Shifter state
/// Shifts pitch of audio without changing duration using granular synthesis
/// Converts semitones to playback rate and uses overlapping grains
#[derive(Debug, Clone)]
pub struct PitchShifterState {
    delay_buffer: Vec<f32>, // Circular buffer for input audio
    write_pos: usize,       // Write position in buffer
    grain1_pos: f32,        // Read position for grain 1
    grain2_pos: f32,        // Read position for grain 2
    grain1_phase: f32,      // Phase for grain 1 window [0, 1]
    grain2_phase: f32,      // Phase for grain 2 window [0, 1]
    grain_size: usize,      // Size of each grain in samples
    sample_rate: f32,
}

impl PitchShifterState {
    pub fn new(grain_size_ms: f32, sample_rate: f32) -> Self {
        let grain_size = ((grain_size_ms / 1000.0) * sample_rate) as usize;
        let grain_size = grain_size.max(128); // Minimum grain size
        let buffer_size = grain_size * 4; // 4x grain size for buffer

        Self {
            delay_buffer: vec![0.0; buffer_size],
            write_pos: 0,
            grain1_pos: 0.0,
            grain2_pos: (grain_size / 2) as f32, // Offset by half grain
            grain1_phase: 0.0,
            grain2_phase: 0.5, // 50% phase offset
            grain_size,
            sample_rate,
        }
    }

    /// Process one sample with pitch shifting
    /// semitones: pitch shift in semitones (positive = higher, negative = lower)
    pub fn process(&mut self, input: f32, semitones: f32) -> f32 {
        // Write input to delay buffer
        self.delay_buffer[self.write_pos] = input;
        self.write_pos = (self.write_pos + 1) % self.delay_buffer.len();

        // Convert semitones to playback rate: rate = 2^(semitones/12)
        let playback_rate = (semitones / 12.0).exp2();

        // Hann window function
        let window = |phase: f32| -> f32 {
            let phase = phase.clamp(0.0, 1.0);
            0.5 * (1.0 - (2.0 * std::f32::consts::PI * phase).cos())
        };

        // Read from grain 1
        let grain1_idx = self.grain1_pos as usize % self.delay_buffer.len();
        let grain1_sample = self.delay_buffer[grain1_idx];
        let grain1_window = window(self.grain1_phase);
        let grain1_out = grain1_sample * grain1_window;

        // Read from grain 2
        let grain2_idx = self.grain2_pos as usize % self.delay_buffer.len();
        let grain2_sample = self.delay_buffer[grain2_idx];
        let grain2_window = window(self.grain2_phase);
        let grain2_out = grain2_sample * grain2_window;

        // Mix grains
        let output = grain1_out + grain2_out;

        // Advance grain positions at playback rate
        self.grain1_pos += playback_rate;
        self.grain2_pos += playback_rate;

        // Advance phases (always at normal rate to maintain duration)
        let phase_inc = 1.0 / self.grain_size as f32;
        self.grain1_phase += phase_inc;
        self.grain2_phase += phase_inc;

        // Reset grain 1 when complete
        if self.grain1_phase >= 1.0 {
            self.grain1_phase = 0.0;
            // Start reading from current write position
            self.grain1_pos = self.write_pos as f32;
        }

        // Reset grain 2 when complete
        if self.grain2_phase >= 1.0 {
            self.grain2_phase = 0.0;
            self.grain2_pos = self.write_pos as f32;
        }

        output * 0.5 // Normalize for 2 grains
    }
}

impl Default for PitchShifterState {
    fn default() -> Self {
        Self::new(50.0, 44100.0) // Default: 50ms grains at 44.1kHz
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

/// Tap State - Records signal to buffer for debugging
#[derive(Debug, Clone)]
pub struct TapState {
    pub buffer: Vec<f32>,   // Recording buffer
    pub filename: String,   // Output filename
    pub max_samples: usize, // Maximum samples to record
    pub sample_rate: f32,   // Sample rate for WAV output
    pub enabled: bool,      // Whether recording is active
}

impl TapState {
    pub fn new(filename: String, duration_secs: f32, sample_rate: f32) -> Self {
        let max_samples = (duration_secs * sample_rate) as usize;
        Self {
            buffer: Vec::with_capacity(max_samples),
            filename,
            max_samples,
            sample_rate,
            enabled: true,
        }
    }

    /// Record a sample (if still recording)
    pub fn record(&mut self, sample: f32) {
        if self.enabled && self.buffer.len() < self.max_samples {
            self.buffer.push(sample);

            // Disable when full
            if self.buffer.len() >= self.max_samples {
                self.enabled = false;
            }
        }
    }

    /// Write buffer to WAV file
    pub fn write_to_file(&self) -> Result<(), String> {
        use hound::{WavSpec, WavWriter};

        let spec = WavSpec {
            channels: 1,
            sample_rate: self.sample_rate as u32,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        let mut writer = WavWriter::create(&self.filename, spec)
            .map_err(|e| format!("Failed to create WAV file: {}", e))?;

        for &sample in &self.buffer {
            writer
                .write_sample(sample)
                .map_err(|e| format!("Failed to write sample: {}", e))?;
        }

        writer
            .finalize()
            .map_err(|e| format!("Failed to finalize WAV file: {}", e))?;

        Ok(())
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
        Self { elapsed_samples: 0 }
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
    Idle,    // Envelope at 0, waiting for gate
    Attack,  // Rising from 0 to 1
    Sustain, // Holding at 1 while gate is high
    Release, // Falling from current level to 0
}

/// ASR (Attack-Sustain-Release) envelope state
/// Gate-based envelope: attacks when gate goes high, sustains while high, releases when gate goes low
#[derive(Debug, Clone)]
pub struct ASRState {
    phase: RefCell<ASRPhase>,
    current_level: f32, // Current envelope output [0, 1]
    previous_gate: f32, // Previous gate value for edge detection
}

impl ASRState {
    pub fn new() -> Self {
        Self {
            phase: RefCell::new(ASRPhase::Idle),
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

/// MIDI polyphonic voice state
/// Represents a single voice in the MidiPolySynth node
#[derive(Debug, Clone)]
pub struct MidiPolyVoice {
    /// MIDI note number (0-127)
    pub note: u8,
    /// Calculated frequency in Hz
    pub frequency: f32,
    /// Gate value: 1.0 when note is held, 0.0 after release
    pub gate: f32,
    /// Whether this voice is actively producing sound
    /// (remains true during release phase until envelope completes)
    pub active: bool,
    /// Sample counter since voice started (for voice stealing priority)
    pub age: u64,
    /// Per-voice oscillator phase (0.0 to 1.0)
    pub phase: f32,
    /// Per-voice envelope level (ASR envelope)
    pub envelope_level: f32,
    /// Release time remaining (samples) - voice becomes inactive when 0
    pub release_samples_remaining: u32,
}

impl MidiPolyVoice {
    pub fn new() -> Self {
        Self {
            note: 0,
            frequency: 0.0,
            gate: 0.0,
            active: false,
            age: 0,
            phase: 0.0,
            envelope_level: 0.0,
            release_samples_remaining: 0,
        }
    }

    /// Trigger this voice with a new note
    pub fn trigger(&mut self, note: u8, frequency: f32) {
        self.note = note;
        self.frequency = frequency;
        self.gate = 1.0;
        self.active = true;
        self.age = 0;
        self.phase = 0.0; // Reset phase on retrigger
        self.envelope_level = 0.0;
        self.release_samples_remaining = 0;
    }

    /// Release this voice (note-off)
    pub fn release(&mut self) {
        self.gate = 0.0;
        // Voice stays active during release phase
        // MidiPolySynth will set release_samples_remaining based on signal template
    }
}

impl Default for MidiPolyVoice {
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

/// Expander state (upward expander - opposite of compressor)
#[derive(Debug, Clone)]
pub struct ExpanderState {
    envelope: f32, // Current envelope follower value
}

impl ExpanderState {
    pub fn new() -> Self {
        Self { envelope: 0.0 }
    }
}

impl Default for ExpanderState {
    fn default() -> Self {
        Self::new()
    }
}

/// Adaptive Compressor state
/// Tracks both envelope follower and RMS analysis for adaptive behavior
#[derive(Debug, Clone)]
pub struct AdaptiveCompressorState {
    envelope: f32,        // Current envelope follower value
    rms_buffer: Vec<f32>, // Circular buffer for RMS calculation
    rms_write_idx: usize, // Write position in RMS buffer
    current_rms: f32,     // Current RMS level for adaptive modulation
}

impl AdaptiveCompressorState {
    pub fn new() -> Self {
        Self {
            envelope: 0.0,
            rms_buffer: vec![0.0; 4410], // 100ms at 44.1kHz
            rms_write_idx: 0,
            current_rms: 0.0,
        }
    }
}

impl Default for AdaptiveCompressorState {
    fn default() -> Self {
        Self::new()
    }
}

/// Convolution reverb state
#[derive(Debug, Clone)]
pub struct ConvolutionState {
    // Input buffer for convolution (stores recent samples)
    input_buffer: Vec<f32>,
    buffer_index: usize,

    // Impulse response (IR) - hardcoded for now
    impulse_response: Vec<f32>,
}

impl ConvolutionState {
    pub fn new(sample_rate: f32) -> Self {
        // Create a simple built-in impulse response
        // This creates a small room-like reverb with early reflections
        let ir_length = (sample_rate * 0.5) as usize; // 500ms IR
        let mut impulse_response = vec![0.0; ir_length];

        // Initial impulse
        impulse_response[0] = 1.0;

        // Early reflections at various delays with decay
        let reflections = [
            (0.021, 0.6),  // 21ms, -4.4dB
            (0.043, 0.4),  // 43ms, -8dB
            (0.067, 0.3),  // 67ms, -10.5dB
            (0.089, 0.2),  // 89ms, -14dB
            (0.121, 0.15), // 121ms, -16.5dB
            (0.156, 0.1),  // 156ms, -20dB
        ];

        for (delay_sec, gain) in reflections.iter() {
            let delay_samples = (delay_sec * sample_rate) as usize;
            if delay_samples < ir_length {
                impulse_response[delay_samples] = *gain;
            }
        }

        // Add exponential decay tail
        for i in 1..ir_length {
            let t = i as f32 / sample_rate;
            let decay = (-3.0 * t).exp(); // RT60 ≈ 0.3 seconds
            impulse_response[i] += decay * 0.05; // Add diffuse tail
        }

        // Input buffer needs to be at least IR length
        let input_buffer = vec![0.0; ir_length];

        Self {
            input_buffer,
            buffer_index: 0,
            impulse_response,
        }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        // Store input in circular buffer
        self.input_buffer[self.buffer_index] = input;

        // Perform convolution
        let mut output = 0.0;
        let ir_len = self.impulse_response.len();
        let buf_len = self.input_buffer.len();

        for i in 0..ir_len {
            // Read backwards through input buffer (circular)
            let buffer_pos = (self.buffer_index + buf_len - i) % buf_len;
            output += self.input_buffer[buffer_pos] * self.impulse_response[i];
        }

        // Advance buffer index
        self.buffer_index = (self.buffer_index + 1) % buf_len;

        output
    }
}

impl Default for ConvolutionState {
    fn default() -> Self {
        Self::new(44100.0)
    }
}

/// Spectral Freeze state - FFT-based spectrum freezing
pub struct SpectralFreezeState {
    // FFT parameters
    fft_size: usize,
    hop_size: usize,

    // FFT/IFFT processors
    r2c: std::sync::Arc<dyn realfft::RealToComplex<f32>>,
    c2r: std::sync::Arc<dyn realfft::ComplexToReal<f32>>,

    // Buffers
    input_buffer: Vec<f32>,
    output_buffer: Vec<f32>,
    buffer_index: usize,

    // Window function (Hann window)
    window: Vec<f32>,

    // Frozen spectrum (complex values)
    frozen_spectrum: Option<Vec<num_complex::Complex<f32>>>,

    // Overlap-add output accumulator
    overlap_add: Vec<f32>,
    read_index: usize,

    // Last trigger state (for edge detection)
    last_trigger: f32,
}

impl SpectralFreezeState {
    pub fn new() -> Self {
        let fft_size = 2048;
        let hop_size = 512; // 75% overlap

        // Create FFT planner
        let mut real_planner = realfft::RealFftPlanner::<f32>::new();
        let r2c = real_planner.plan_fft_forward(fft_size);
        let c2r = real_planner.plan_fft_inverse(fft_size);

        // Create Hann window
        let window: Vec<f32> = (0..fft_size)
            .map(|i| {
                let t = i as f32 / (fft_size - 1) as f32;
                0.5 * (1.0 - (2.0 * std::f32::consts::PI * t).cos())
            })
            .collect();

        Self {
            fft_size,
            hop_size,
            r2c,
            c2r,
            input_buffer: vec![0.0; fft_size],
            output_buffer: vec![0.0; fft_size],
            buffer_index: 0,
            window,
            frozen_spectrum: None,
            overlap_add: vec![0.0; fft_size],
            read_index: 0,
            last_trigger: 0.0,
        }
    }

    pub fn process(&mut self, input: f32, trigger: f32) -> f32 {
        // Store input sample
        self.input_buffer[self.buffer_index] = input;
        self.buffer_index += 1;

        // Detect trigger (rising edge)
        let triggered = trigger > 0.5 && self.last_trigger <= 0.5;
        self.last_trigger = trigger;

        // Process FFT frame when buffer is full
        if self.buffer_index >= self.hop_size {
            // Apply window
            let mut windowed: Vec<f32> = self
                .input_buffer
                .iter()
                .zip(self.window.iter())
                .map(|(x, w)| x * w)
                .collect();

            // Perform FFT
            let mut spectrum = self.r2c.make_output_vec();
            self.r2c.process(&mut windowed, &mut spectrum).unwrap_or(());

            // Freeze spectrum on trigger
            if triggered {
                self.frozen_spectrum = Some(spectrum.clone());
            }

            // Use frozen spectrum if available, otherwise pass through
            let output_spectrum = if let Some(ref frozen) = self.frozen_spectrum {
                frozen.clone()
            } else {
                spectrum
            };

            // Perform IFFT
            let mut output = self.c2r.make_output_vec();
            self.c2r
                .process(&mut output_spectrum.clone(), &mut output)
                .unwrap_or(());

            // Normalize IFFT output
            let scale = 1.0 / self.fft_size as f32;
            for x in output.iter_mut() {
                *x *= scale;
            }

            // Apply window again and accumulate (overlap-add)
            for (i, (out_sample, window_sample)) in
                output.iter().zip(self.window.iter()).enumerate()
            {
                self.overlap_add[i] += out_sample * window_sample;
            }

            // Shift buffers
            self.input_buffer.copy_within(self.hop_size.., 0);
            for i in (self.fft_size - self.hop_size)..self.fft_size {
                self.input_buffer[i] = 0.0;
            }
            self.buffer_index = self.fft_size - self.hop_size;

            // Copy overlap-add to output and shift
            for i in 0..self.hop_size {
                self.output_buffer[i] = self.overlap_add[i];
            }
            self.overlap_add.copy_within(self.hop_size.., 0);
            for i in (self.fft_size - self.hop_size)..self.fft_size {
                self.overlap_add[i] = 0.0;
            }
            self.read_index = 0;
        }

        // Return output sample
        let output = if self.read_index < self.hop_size {
            self.output_buffer[self.read_index]
        } else {
            0.0
        };
        self.read_index += 1;

        output
    }
}

impl Default for SpectralFreezeState {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for SpectralFreezeState {
    fn clone(&self) -> Self {
        // Recreate FFT planners (they can't be cloned directly)
        let mut real_planner = realfft::RealFftPlanner::<f32>::new();
        let r2c = real_planner.plan_fft_forward(self.fft_size);
        let c2r = real_planner.plan_fft_inverse(self.fft_size);

        Self {
            fft_size: self.fft_size,
            hop_size: self.hop_size,
            r2c,
            c2r,
            input_buffer: self.input_buffer.clone(),
            output_buffer: self.output_buffer.clone(),
            buffer_index: self.buffer_index,
            window: self.window.clone(),
            frozen_spectrum: self.frozen_spectrum.clone(),
            overlap_add: self.overlap_add.clone(),
            read_index: self.read_index,
            last_trigger: self.last_trigger,
        }
    }
}

impl std::fmt::Debug for SpectralFreezeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpectralFreezeState")
            .field("fft_size", &self.fft_size)
            .field("hop_size", &self.hop_size)
            .field("buffer_index", &self.buffer_index)
            .field("read_index", &self.read_index)
            .field("frozen", &self.frozen_spectrum.is_some())
            .finish()
    }
}

/// Output mixing mode - how multiple output channels are combined
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputMixMode {
    /// Automatic gain compensation - divide by number of channels
    /// Simple and predictable, prevents clipping
    Gain,

    /// RMS-based mixing - divide by sqrt(num_channels)
    /// Preserves perceived loudness, best for music (default)
    Sqrt,

    /// Soft saturation using tanh
    /// Prevents clipping with warm analog-style distortion
    Tanh,

    /// Hard limiting at ±1.0
    /// Prevents clipping with brick-wall limiting
    Hard,

    /// No compensation - sum outputs directly
    /// Can cause clipping, use with caution
    None,
}

impl Default for OutputMixMode {
    fn default() -> Self {
        OutputMixMode::None // Direct sum - like a hardware mixer
    }
}

impl OutputMixMode {
    /// Parse from string (for DSL)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "gain" => Some(OutputMixMode::Gain),
            "sqrt" => Some(OutputMixMode::Sqrt),
            "tanh" => Some(OutputMixMode::Tanh),
            "hard" => Some(OutputMixMode::Hard),
            "none" => Some(OutputMixMode::None),
            _ => None,
        }
    }
}

/// Request for parallel bus synthesis
/// Collects all parameters needed to synthesize a bus buffer independently
#[derive(Clone)]
struct BusSynthesisRequest {
    bus_node_id: NodeId,
    duration_samples: usize,
    event_index: usize, // To match back to original event after parallel synthesis
}

/// Synthesize a bus buffer in an isolated context (for parallel synthesis)
/// Takes cloned nodes (independent RefCell state) and synthesizes buffer
/// This is a simplified evaluator that only handles node types used in bus synthesis
fn synthesize_bus_buffer_parallel(
    mut nodes: Vec<Option<Rc<SignalNode>>>,
    bus_node_id: NodeId,
    duration_samples: usize,
    sample_rate: f32,
) -> Vec<f32> {
    // CRITICAL: Reset all oscillator phases to 0 before synthesis
    // Without this, cloned oscillators start at arbitrary phases, causing:
    // - DC offset (buffer doesn't contain full periods)
    // - Clicks (buffer doesn't start at zero crossing)
    // - Rough sound (phase discontinuities on every trigger)
    for node_opt in nodes.iter_mut() {
        if let Some(node_rc) = node_opt {
            let node = Rc::make_mut(node_rc);
            if let SignalNode::Oscillator { phase, .. } = node {
                *phase.borrow_mut() = 0.0;
            }
        }
    }

    let mut buffer = Vec::with_capacity(duration_samples);

    // Synthesize each sample by evaluating the bus node
    // Stateful oscillators (with RefCell) maintain phase across samples
    for _ in 0..duration_samples {
        let sample_value = eval_node_isolated(&mut nodes, &bus_node_id, sample_rate);
        buffer.push(sample_value);
    }

    // DEBUG: Check if buffer contains audio
    if std::env::var("DEBUG_BUS_SYNTHESIS").is_ok() {
        let rms: f32 = buffer.iter().map(|&s| s * s).sum::<f32>() / buffer.len() as f32;
        let rms = rms.sqrt();
        eprintln!(
            "  Synthesized buffer: {} samples, RMS={:.6}, first_10={:?}",
            buffer.len(),
            rms,
            &buffer[..buffer.len().min(10)]
        );
    }

    buffer
}

/// Simplified node evaluator for isolated bus synthesis
/// No caching needed - stateful nodes use RefCell for state management
fn eval_node_isolated(
    nodes: &mut Vec<Option<Rc<SignalNode>>>,
    node_id: &NodeId,
    sample_rate: f32,
) -> f32 {
    let node = if let Some(Some(node_rc)) = nodes.get(node_id.0) {
        Rc::clone(node_rc)
    } else {
        return 0.0;
    };

    // Evaluate based on node type
    match &*node {
        SignalNode::Constant { value } => *value,

        SignalNode::Oscillator {
            freq,
            waveform,
            semitone_offset,
            phase,
            pending_freq,
            last_sample,
        } => {
            let base_freq = eval_signal_isolated(nodes, &freq, sample_rate);

            // Decode pitch value:
            // >= 1000: Absolute MIDI note (subtract 1000, convert to Hz)
            // < 1000: Relative semitone offset from base frequency
            let freq_val = if *semitone_offset >= 1000.0 {
                // Absolute MIDI: convert to Hz using A4=440Hz as reference
                let midi = *semitone_offset - 1000.0;
                440.0 * 2.0_f32.powf((midi - 69.0) / 12.0)
            } else if *semitone_offset != 0.0 {
                // Relative: apply semitone offset to base frequency
                base_freq * 2.0_f32.powf(*semitone_offset / 12.0)
            } else {
                base_freq
            };

            // Generate sample based on waveform
            let phase_val = *phase.borrow();
            let sample = match waveform {
                Waveform::Sine => (2.0 * PI * phase_val).sin(),
                Waveform::Saw => 2.0 * phase_val - 1.0,
                Waveform::Square => {
                    if phase_val < 0.5 {
                        1.0
                    } else {
                        -1.0
                    }
                }
                Waveform::Triangle => {
                    if phase_val < 0.5 {
                        4.0 * phase_val - 1.0
                    } else {
                        -4.0 * phase_val + 3.0
                    }
                }
            };

            // Update phase for next sample
            {
                let mut p = phase.borrow_mut();
                *p += freq_val / sample_rate;
                if *p >= 1.0 {
                    *p -= 1.0;
                }
            }

            sample
        }

        SignalNode::Biquad {
            input,
            frequency,
            q,
            mode,
            state,
        } => {
            // Biquad Filter (RBJ Audio EQ Cookbook)
            let input_val = eval_signal_isolated(nodes, &input, sample_rate);
            let freq = eval_signal_isolated(nodes, &frequency, sample_rate)
                .clamp(10.0, sample_rate * 0.45);
            let q_val = eval_signal_isolated(nodes, &q, sample_rate).clamp(0.1, 20.0);

            // Calculate normalized frequency
            let omega = 2.0 * std::f32::consts::PI * freq / sample_rate;
            let sin_omega = omega.sin();
            let cos_omega = omega.cos();
            let alpha = sin_omega / (2.0 * q_val);

            // Calculate coefficients based on mode (RBJ formulas)
            let (b0, b1, b2, a0, a1, a2) = match mode {
                0 => {
                    // Lowpass
                    let b1_temp = 1.0 - cos_omega;
                    let b0_temp = b1_temp / 2.0;
                    let b2_temp = b0_temp;
                    let a0_temp = 1.0 + alpha;
                    let a1_temp = -2.0 * cos_omega;
                    let a2_temp = 1.0 - alpha;
                    (b0_temp, b1_temp, b2_temp, a0_temp, a1_temp, a2_temp)
                }
                1 => {
                    // Highpass
                    let b1_temp = -(1.0 + cos_omega);
                    let b0_temp = -b1_temp / 2.0;
                    let b2_temp = b0_temp;
                    let a0_temp = 1.0 + alpha;
                    let a1_temp = -2.0 * cos_omega;
                    let a2_temp = 1.0 - alpha;
                    (b0_temp, b1_temp, b2_temp, a0_temp, a1_temp, a2_temp)
                }
                2 => {
                    // Bandpass
                    let b0_temp = alpha;
                    let b1_temp = 0.0;
                    let b2_temp = -alpha;
                    let a0_temp = 1.0 + alpha;
                    let a1_temp = -2.0 * cos_omega;
                    let a2_temp = 1.0 - alpha;
                    (b0_temp, b1_temp, b2_temp, a0_temp, a1_temp, a2_temp)
                }
                _ => (1.0, 0.0, 0.0, 1.0, 0.0, 0.0), // Passthrough
            };

            // Normalize coefficients
            let b0_norm = b0 / a0;
            let b1_norm = b1 / a0;
            let b2_norm = b2 / a0;
            let a1_norm = a1 / a0;
            let a2_norm = a2 / a0;

            // Get current state values
            let x1 = state.x1;
            let x2 = state.x2;
            let y1 = state.y1;
            let y2 = state.y2;

            // Apply biquad difference equation
            let output =
                b0_norm * input_val + b1_norm * x1 + b2_norm * x2 - a1_norm * y1 - a2_norm * y2;

            // Clamp and check for stability
            let output_clamped = output.clamp(-10.0, 10.0);
            let final_output = if output_clamped.is_finite() {
                output_clamped
            } else {
                0.0
            };

            // Update state in nodes vec
            if let Some(Some(node_rc)) = nodes.get_mut(node_id.0) {
                let node_mut = Rc::make_mut(node_rc);
                if let SignalNode::Biquad { state: s, .. } = node_mut {
                    s.x2 = x1;
                    s.x1 = input_val;
                    s.y2 = y1;
                    s.y1 = final_output;
                }
            }

            final_output
        }

        // Add more node types as needed for bus synthesis
        // For now, return 0.0 for unsupported types
        _ => {
            // Most node types not needed for basic bus synthesis
            // Main graph's eval_node handles the complex cases
            0.0
        }
    }
}

/// Evaluate signal in isolated context
fn eval_signal_isolated(
    nodes: &mut Vec<Option<Rc<SignalNode>>>,
    signal: &Signal,
    sample_rate: f32,
) -> f32 {
    match signal {
        Signal::Value(v) => *v,
        Signal::Node(id) => eval_node_isolated(nodes, id, sample_rate),
        Signal::Expression(expr) => match &**expr {
            SignalExpr::Add(left, right) => {
                eval_signal_isolated(nodes, left, sample_rate)
                    + eval_signal_isolated(nodes, right, sample_rate)
            }
            SignalExpr::Subtract(left, right) => {
                eval_signal_isolated(nodes, left, sample_rate)
                    - eval_signal_isolated(nodes, right, sample_rate)
            }
            SignalExpr::Multiply(left, right) => {
                eval_signal_isolated(nodes, left, sample_rate)
                    * eval_signal_isolated(nodes, right, sample_rate)
            }
            SignalExpr::Divide(left, right) => {
                let r = eval_signal_isolated(nodes, right, sample_rate);
                if r != 0.0 {
                    eval_signal_isolated(nodes, left, sample_rate) / r
                } else {
                    0.0
                }
            }
            SignalExpr::Modulo(left, right) => {
                let r = eval_signal_isolated(nodes, right, sample_rate);
                if r != 0.0 {
                    eval_signal_isolated(nodes, left, sample_rate) % r
                } else {
                    0.0
                }
            }
            SignalExpr::Min(left, right) => eval_signal_isolated(nodes, left, sample_rate)
                .min(eval_signal_isolated(nodes, right, sample_rate)),
            SignalExpr::Scale { input, min, max } => {
                let val = eval_signal_isolated(nodes, input, sample_rate);
                let min_val = eval_signal_isolated(nodes, min, sample_rate);
                let max_val = eval_signal_isolated(nodes, max, sample_rate);
                min_val + val * (max_val - min_val)
            }
        },
        _ => 0.0, // Simplified - buses, patterns not needed for basic synthesis
    }
}

/// Cycle-level cache for parallel bus synthesis (Phase 2 optimization)
/// Caches bus buffers for an entire cycle to avoid redundant preprocessing
#[derive(Clone)]
struct CycleBusCache {
    /// Which cycle this cache is for
    cycle_floor: i64,
    /// Pre-synthesized bus buffers: (bus_name, duration_samples) -> buffer
    buffers: HashMap<(String, usize), Arc<Vec<f32>>>,
}

impl Default for CycleBusCache {
    fn default() -> Self {
        Self {
            cycle_floor: -1, // Invalid cycle - forces first synthesis
            buffers: HashMap::new(),
        }
    }
}

// === FX State Persistence for Live Code Reloading ===
// This system preserves delay/reverb/chorus buffers across hot-swaps
// so that FX tails continue smoothly during live coding

/// Key for identifying FX nodes across graph reloads
/// Format: (bus_name, fx_type, index_in_chain)
/// - bus_name: "out" for main output, or the bus name like "drums"
/// - fx_type: "delay", "reverb", "chorus", etc.
/// - index_in_chain: 0 for first occurrence, 1 for second, etc.
pub type FxStateKey = (String, String, usize);

/// Extracted FX state that can be transferred between graphs
/// Contains the internal buffers/state that we want to preserve
#[derive(Debug, Clone)]
pub enum ExtractedFxState {
    // Time-domain effects (most important - audible discontinuity if reset)
    Delay {
        buffer: Vec<f32>,
        write_idx: usize,
    },
    TapeDelay(TapeDelayState),
    MultiTapDelay {
        buffer: Vec<f32>,
        write_idx: usize,
    },
    PingPongDelay {
        buffer_l: Vec<f32>,
        buffer_r: Vec<f32>,
        write_idx: usize,
    },

    // Reverbs (preserves reverb tails)
    Reverb(ReverbState),
    DattorroReverb(DattorroState),
    Convolution(ConvolutionState),

    // Modulation effects (preserves LFO phase and buffers)
    Chorus(ChorusState),
    Flanger(FlangerState),

    // Dynamics (preserves envelope state)
    Compressor(CompressorState),
    Expander(ExpanderState),

    // Filters (preserves filter state - prevents clicks)
    Filter(FilterState),
    MoogLadder(MoogLadderState),
}

/// Map of FX state keyed by (bus_name, fx_type, index)
pub type FxStateMap = HashMap<FxStateKey, ExtractedFxState>;

/// The unified signal graph that processes everything
pub struct UnifiedSignalGraph {
    /// All nodes in the graph (Rc for cheap cloning - eliminates deep clone overhead)
    pub nodes: Vec<Option<std::rc::Rc<SignalNode>>>,

    /// Named buses for easy reference
    buses: HashMap<String, NodeId>,

    /// Output node ID (for backwards compatibility - single output)
    output: Option<NodeId>,

    /// Multi-output: channel number -> node ID
    outputs: HashMap<usize, NodeId>,

    /// Hushed (silenced) output channels
    hushed_channels: std::collections::HashSet<usize>,

    /// Output mixing mode (how to combine multiple outputs)
    output_mix_mode: OutputMixMode,

    /// Sample rate
    sample_rate: f32,

    /// Session start time (wall-clock) - for drift-free timing in LIVE mode
    /// In offline rendering, timing is sample-count based instead
    pub session_start_time: std::time::Instant,

    /// Cycle offset for resetCycles command
    /// Formula: cycle_position = (now - session_start_time).as_secs_f64() * cps + cycle_offset
    pub cycle_offset: f64,

    /// Use wall-clock timing (true for live mode, false for offline rendering)
    pub use_wall_clock: bool,

    /// Cycles per second (tempo)
    pub cps: f32,

    /// Buffer size for audio processing (samples per buffer)
    /// Default is 512, can be set via "buffer: 1024" in code
    pub buffer_size: usize,

    /// Cached cycle position for current sample
    /// Updated once at start of process_sample(), then stays constant during processing
    /// This ensures all evaluations within a single sample see the same time
    pub cached_cycle_position: f64,

    /// Node ID counter
    next_node_id: usize,

    /// Computed values cache for current sample
    value_cache: HashMap<NodeId, f32>,

    /// Per-sample cache for stateful nodes (ASR, ADSR, oscillators, filters)
    /// Unlike value_cache which is only cleared at buffer start, this is cleared
    /// at the start of EACH sample to ensure stateful nodes are evaluated exactly
    /// once per sample (not twice due to update_bus_previous_values calling eval_node)
    stateful_value_cache: HashMap<NodeId, f32>,

    /// Pattern event cache for current buffer (Option B optimization)
    /// Maps Pattern node ID -> (cycle_position, Vec of events in buffer span)
    /// Pre-computed once per buffer to avoid 512 pattern.query() calls
    pattern_event_cache: HashMap<NodeId, Vec<crate::pattern::Hap<String>>>,

    /// Node buffers for block-based processing (DAW-style)
    /// Each node renders to its own 512-sample buffer
    /// This enables parallel stage execution and eliminates 512x graph traversal
    node_buffers: HashMap<NodeId, Vec<f32>>,

    /// Previous block's node buffers (for feedback loops in DAG processing)
    /// When a node depends on itself or has circular dependencies, it reads from
    /// the previous block's output (1-block delay). After each block, node_buffers
    /// is swapped into prev_node_buffers.
    prev_node_buffers: HashMap<usize, Vec<f32>>,

    /// Zero buffer for missing dependencies in DAG processing
    /// Pre-allocated buffer of zeros to avoid per-node allocation
    dag_zero_buffer: Vec<f32>,

    /// Current block's DAG buffer cache for signal evaluation
    /// When processing nodes in topological order, this holds the computed buffers
    /// for all already-processed nodes. Used to break circular bus dependencies.
    dag_buffer_cache: HashMap<usize, Vec<f32>>,

    /// Flag indicating we're currently in DAG processing mode.
    /// When true, bus references to nodes not yet in dag_buffer_cache return 0.0
    /// instead of recursively calling eval_node (which would cause stack overflow).
    in_dag_processing: bool,

    /// Sample bank for loading and playing samples (RefCell for interior mutability)
    sample_bank: RefCell<SampleBank>,

    /// Voice manager for polyphonic sample playback
    voice_manager: RefCell<VoiceManager>,

    /// Cached voice manager output for current sample (processed once per sample)
    /// Maps source node ID -> mixed voice output for that node
    /// This allows multiple outputs to have independent sample streams
    /// DEPRECATED: Use voice_buffers + current_sample_idx for O(1) lookup
    voice_output_cache: HashMap<usize, f32>,

    /// Stereo version of voice output cache: Maps source node ID -> (left, right)
    voice_output_cache_stereo: HashMap<usize, (f32, f32)>,

    /// OPTIMIZED: Vec-based voice buffers for O(1) lookup in hot loop
    /// Replaces per-sample HashMap rebuilding with direct array indexing
    voice_buffers: VoiceBuffers,

    /// Current sample index within the buffer (for voice_buffers lookup)
    current_sample_idx: usize,

    /// Current DAG node ID being processed (for UnitDelay feedback within bus expressions)
    current_dag_node_id: Option<usize>,

    /// Nodes currently being evaluated in the call stack (for cycle detection).
    /// If we encounter a node that's already in this set, we have a cycle and should
    /// return 0.0 or a cached value to break the recursion.
    eval_call_stack: std::collections::HashSet<usize>,

    /// Maximum node ID in the graph (for pre-sizing VoiceBuffers)
    max_node_id: usize,

    /// Synth voice manager for polyphonic synthesis
    synth_voice_manager: RefCell<SynthVoiceManager>,

    /// Cycle-level cache for parallel bus synthesis (Phase 2 optimization)
    /// Reduces preprocessing frequency from per-buffer to per-cycle
    cycle_bus_cache: CycleBusCache,

    /// Sample counter for debugging
    sample_count: usize,

    /// Buffer cache: prevents re-evaluation within a single buffer render
    /// Maps NodeId -> Option<Vec<f32>> (None = being evaluated, Some = cached result)
    /// CRITICAL: This must be cleared at the start of each buffer render
    buffer_cache: std::cell::RefCell<HashMap<NodeId, Option<Vec<f32>>>>,

    /// Buffer cache enabled flag: Only enabled during process_buffer() to avoid
    /// caching stateful nodes (oscillators) across multiple independent evaluations
    buffer_cache_enabled: std::cell::Cell<bool>,

    /// Flag indicating whether node timing state has been initialized
    /// Set to false when graph is created, true after first buffer processes
    /// This ensures timing state is initialized based on actual buffer start time
    nodes_initialized: bool,

    /// Per-pitch phase tracking for synthesis voices
    /// Key: (oscillator_node_id, semitone_key) where semitone_key = (semitone_offset * 100).round() as i32
    /// Value: phase value in [0, 1)
    /// This allows each pitch variant (chord note) to maintain its own phase continuity
    synthesis_phase_cache: std::cell::RefCell<HashMap<(usize, i32), f32>>,

    /// z^-1 storage for feedback loops
    /// Stores the previous sample's output value for each bus
    /// Used by UnitDelay nodes to implement feedback without cycles
    /// Updated at the end of each sample after all buses are evaluated
    bus_previous_values: HashMap<String, f32>,

    /// Per-voice frequency context for polyphonic MIDI synthesis
    /// When evaluating a signal template within MidiPolySynth, this is set to
    /// the current voice's frequency, allowing `~midi` references to resolve
    /// to the appropriate pitch for each voice
    pub current_voice_frequency: std::cell::Cell<Option<f32>>,

    /// Per-voice gate context for polyphonic MIDI synthesis
    /// 1.0 when note is held, 0.0 when released
    /// Used by envelope nodes within signal templates
    pub current_voice_gate: std::cell::Cell<Option<f32>>,

    /// Shared state registry for parallel rendering
    /// When enabled, stateful nodes (reverbs, delays, filters, etc.) share their state
    /// across parallel graph clones via Arc<RwLock<State>>
    /// None = normal mode (each clone has independent state)
    /// Some = parallel mode (all clones share state)
    pub shared_state: Option<crate::shared_effect_state::SharedStateRegistry>,

    /// When true, sequential effects (reverb, delay) pass through unchanged (output = input)
    /// Used for pipelined rendering where dry signal is computed in parallel
    /// and sequential effects are applied in a separate sequential pass
    pub bypass_sequential_effects: bool,
}

// SAFETY: UnifiedSignalGraph contains RefCell which is !Send and !Sync, but we ensure
// that each graph instance is only accessed by a single thread at a time.
// In live mode:
// - Audio thread has its own Arc instance (via ArcSwap::load())
// - File watcher creates NEW graphs and stores them atomically
// - They never access the same graph instance concurrently
// Therefore, it's safe to send UnifiedSignalGraph between threads and share references.
unsafe impl Send for UnifiedSignalGraph {}
unsafe impl Sync for UnifiedSignalGraph {}

impl Clone for UnifiedSignalGraph {
    fn clone(&self) -> Self {
        Self {
            // CRITICAL: Deep clone nodes, not just Rc wrappers
            // Each thread needs independent SignalNode instances with their own RefCells
            nodes: self
                .nodes
                .iter()
                .map(|opt| opt.as_ref().map(|rc| std::rc::Rc::new((**rc).clone())))
                .collect(),
            buses: self.buses.clone(),
            output: self.output,
            outputs: self.outputs.clone(),
            hushed_channels: self.hushed_channels.clone(),
            output_mix_mode: self.output_mix_mode,
            sample_rate: self.sample_rate,
            session_start_time: std::time::Instant::now(), // New instance gets fresh start time
            cycle_offset: self.cycle_offset,
            use_wall_clock: self.use_wall_clock,
            cps: self.cps,
            cached_cycle_position: self.cached_cycle_position,
            next_node_id: self.next_node_id,
            value_cache: HashMap::new(), // Fresh cache for cloned instance
            stateful_value_cache: HashMap::new(), // Fresh per-sample cache for cloned instance
            pattern_event_cache: HashMap::new(), // Fresh cache for cloned instance
            node_buffers: HashMap::new(), // Fresh buffers for cloned instance
            prev_node_buffers: HashMap::new(), // Fresh DAG feedback buffers
            dag_zero_buffer: vec![0.0; self.buffer_size], // Sized to match buffer_size
            dag_buffer_cache: HashMap::new(), // Fresh DAG buffer cache
            in_dag_processing: false,
            sample_bank: RefCell::new(self.sample_bank.borrow().clone()), // Clone loaded samples (cheap Arc increment)
            voice_manager: RefCell::new(VoiceManager::new()),
            voice_output_cache: HashMap::new(), // Fresh cache
            voice_output_cache_stereo: HashMap::new(), // Fresh stereo cache
            voice_buffers: VoiceBuffers::default(), // Fresh Vec-based buffers
            current_sample_idx: 0,
            current_dag_node_id: None,
            eval_call_stack: std::collections::HashSet::new(),
            max_node_id: self.max_node_id,
            synth_voice_manager: RefCell::new(SynthVoiceManager::new(self.sample_rate)),
            cycle_bus_cache: self.cycle_bus_cache.clone(),
            sample_count: self.sample_count,
            buffer_cache: RefCell::new(HashMap::new()), // Fresh cache for cloned instance
            buffer_cache_enabled: std::cell::Cell::new(false),
            nodes_initialized: false, // Cloned graph needs initialization on first buffer
            synthesis_phase_cache: RefCell::new(HashMap::new()), // Fresh phase cache
            bus_previous_values: self.bus_previous_values.clone(), // Preserve feedback state
            buffer_size: self.buffer_size,
            current_voice_frequency: std::cell::Cell::new(None),
            current_voice_gate: std::cell::Cell::new(None),
            // Shared state is preserved on clone (Arc gives cheap reference)
            shared_state: self.shared_state.clone(),
            bypass_sequential_effects: self.bypass_sequential_effects,
        }
    }
}

/// Convert MIDI note number to frequency in Hz
/// MIDI note 69 (A4) = 440 Hz
/// Each semitone is a factor of 2^(1/12)
#[inline]
pub fn midi_note_to_freq(note: u8) -> f32 {
    440.0 * 2.0f32.powf((note as f32 - 69.0) / 12.0)
}

impl UnifiedSignalGraph {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            nodes: Vec::new(),
            buses: HashMap::new(),
            output: None,
            outputs: HashMap::new(),
            hushed_channels: std::collections::HashSet::new(),
            output_mix_mode: OutputMixMode::default(),
            sample_rate,
            session_start_time: std::time::Instant::now(),
            cycle_offset: 0.0,
            use_wall_clock: false, // Default to sample-based for offline rendering
            cps: 0.5,              // Default 0.5 cycles per second
            buffer_size: 512,      // Default buffer size
            cached_cycle_position: 0.0,
            next_node_id: 0,
            value_cache: HashMap::new(),
            stateful_value_cache: HashMap::new(),
            pattern_event_cache: HashMap::new(),
            node_buffers: HashMap::new(),
            prev_node_buffers: HashMap::new(),
            dag_zero_buffer: vec![0.0; 512], // Default buffer size
            dag_buffer_cache: HashMap::new(),
            in_dag_processing: false,
            sample_bank: RefCell::new(SampleBank::new()),
            voice_manager: RefCell::new(VoiceManager::new()),
            voice_output_cache: HashMap::new(),
            voice_output_cache_stereo: HashMap::new(),
            voice_buffers: VoiceBuffers::default(),
            current_sample_idx: 0,
            current_dag_node_id: None,
            eval_call_stack: std::collections::HashSet::new(),
            max_node_id: 0,
            synth_voice_manager: RefCell::new(SynthVoiceManager::new(sample_rate)),
            cycle_bus_cache: CycleBusCache::default(),
            sample_count: 0,
            buffer_cache: RefCell::new(HashMap::new()),
            buffer_cache_enabled: std::cell::Cell::new(false),
            nodes_initialized: false,
            synthesis_phase_cache: RefCell::new(HashMap::new()),
            bus_previous_values: HashMap::new(),
            current_voice_frequency: std::cell::Cell::new(None),
            current_voice_gate: std::cell::Cell::new(None),
            shared_state: None, // Disabled by default
            bypass_sequential_effects: false, // Normal mode by default
        }
    }

    pub fn set_cps(&mut self, cps: f32) {
        self.cps = cps;
    }

    pub fn get_cps(&self) -> f32 {
        self.cps
    }

    pub fn set_buffer_size(&mut self, size: usize) {
        self.buffer_size = size.clamp(64, 16384);
    }

    pub fn get_buffer_size(&self) -> usize {
        self.buffer_size
    }

    /// Set bypass mode for sequential effects (reverb, delay)
    /// When true, these effects pass through unchanged (output = input)
    /// Used for pipelined rendering
    pub fn set_bypass_sequential_effects(&mut self, bypass: bool) {
        self.bypass_sequential_effects = bypass;
    }

    /// Enable shared state for parallel rendering
    ///
    /// This extracts state from all stateful nodes and stores it in a SharedStateRegistry.
    /// After calling this, clone_for_parallel() can be used to create graph clones that
    /// share state with the original (and each other).
    ///
    /// IMPORTANT: Call this BEFORE creating any clones for parallel rendering.
    pub fn enable_shared_state(&mut self) {
        use crate::shared_effect_state::{
            SharedState, SharedStateRegistry, DelayLineState, PingPongDelayState,
            CombState, RMSState, AmpFollowerState, PhaserState, VibratoState,
        };
        use std::sync::{Arc, RwLock};

        let registry = SharedStateRegistry::new();

        // Walk through all nodes and extract state
        for (node_id, opt_node) in self.nodes.iter().enumerate() {
            if let Some(node_rc) = opt_node {
                match &**node_rc {
                    // === Critical: Reverbs and Delays ===
                    // NOTE: These effects have SEQUENTIAL dependencies - block N depends on the state
                    // after block N-1. We still register them, but parallel rendering will process
                    // blocks out of order, causing state corruption for long tails.
                    // The eval_node implementation should detect this and handle appropriately.
                    SignalNode::LushReverb { state, .. } => {
                        registry.register(node_id, SharedState::LushReverb(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }
                    SignalNode::Reverb { state, .. } => {
                        registry.register(node_id, SharedState::Reverb(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }
                    SignalNode::DattorroReverb { state, .. } => {
                        registry.register(node_id, SharedState::Dattorro(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }
                    SignalNode::Delay { buffer, write_idx, .. } => {
                        registry.register(node_id, SharedState::Delay(
                            Arc::new(RwLock::new(DelayLineState {
                                buffer: buffer.clone(),
                                write_idx: *write_idx,
                            }))
                        ));
                    }
                    SignalNode::TapeDelay { state, .. } => {
                        registry.register(node_id, SharedState::TapeDelay(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }
                    SignalNode::MultiTapDelay { buffer, write_idx, .. } => {
                        registry.register(node_id, SharedState::MultiTapDelay(
                            Arc::new(RwLock::new(DelayLineState {
                                buffer: buffer.clone(),
                                write_idx: *write_idx,
                            }))
                        ));
                    }
                    SignalNode::PingPongDelay { buffer_l, buffer_r, write_idx, .. } => {
                        registry.register(node_id, SharedState::PingPongDelay(
                            Arc::new(RwLock::new(PingPongDelayState {
                                buffer_l: buffer_l.clone(),
                                buffer_r: buffer_r.clone(),
                                write_idx: *write_idx,
                            }))
                        ));
                    }
                    SignalNode::Comb { buffer, write_pos, .. } => {
                        registry.register(node_id, SharedState::Comb(
                            Arc::new(RwLock::new(CombState {
                                buffer: buffer.clone(),
                                write_pos: *write_pos,
                            }))
                        ));
                    }
                    SignalNode::Convolution { state, .. } => {
                        registry.register(node_id, SharedState::Convolution(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }

                    // === High priority: Filters ===
                    SignalNode::LowPass { state, .. } |
                    SignalNode::HighPass { state, .. } |
                    SignalNode::BandPass { state, .. } |
                    SignalNode::DJFilter { state, .. } |
                    SignalNode::Notch { state, .. } => {
                        registry.register(node_id, SharedState::Filter(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }
                    SignalNode::SVF { state, .. } => {
                        registry.register(node_id, SharedState::SVF(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }
                    SignalNode::Biquad { state, .. } |
                    SignalNode::Resonz { state, .. } |
                    SignalNode::RLPF { state, .. } |
                    SignalNode::RHPF { state, .. } => {
                        registry.register(node_id, SharedState::Biquad(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }
                    SignalNode::Allpass { state, .. } => {
                        registry.register(node_id, SharedState::Allpass(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }
                    SignalNode::MoogLadder { state, .. } => {
                        registry.register(node_id, SharedState::MoogLadder(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }
                    SignalNode::ParametricEQ { state, .. } => {
                        registry.register(node_id, SharedState::ParametricEQ(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }
                    SignalNode::Formant { state, .. } |
                    SignalNode::Vowel { state, .. } => {
                        registry.register(node_id, SharedState::Formant(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }

                    // === High priority: Oscillators (phase state) ===
                    SignalNode::Oscillator { phase, .. } => {
                        registry.register(node_id, SharedState::OscillatorPhase(
                            Arc::new(RwLock::new(*phase.borrow()))
                        ));
                    }
                    SignalNode::FMOscillator { carrier_phase, modulator_phase, .. } => {
                        registry.register(node_id, SharedState::FMOscillatorPhase(
                            Arc::new(RwLock::new((*carrier_phase.borrow(), *modulator_phase.borrow())))
                        ));
                    }
                    SignalNode::PMOscillator { carrier_phase, .. } => {
                        registry.register(node_id, SharedState::PMOscillatorPhase(
                            Arc::new(RwLock::new(*carrier_phase.borrow()))
                        ));
                    }
                    SignalNode::VCO { phase, .. } => {
                        registry.register(node_id, SharedState::VCOPhase(
                            Arc::new(RwLock::new(*phase.borrow()))
                        ));
                    }
                    SignalNode::Blip { phase, .. } => {
                        registry.register(node_id, SharedState::BlipPhase(
                            Arc::new(RwLock::new(*phase.borrow()))
                        ));
                    }
                    SignalNode::Wavetable { state, .. } => {
                        registry.register(node_id, SharedState::Wavetable(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }

                    // === High priority: Envelopes ===
                    SignalNode::Envelope { state, .. } |
                    SignalNode::EnvelopePattern { state, .. } |
                    SignalNode::StructuredSignal { state, .. } |
                    SignalNode::TriggeredAR { state, .. } |
                    SignalNode::TriggeredADSR { state, .. } => {
                        registry.register(node_id, SharedState::Envelope(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }
                    SignalNode::ADSR { state, .. } => {
                        registry.register(node_id, SharedState::ADSR(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }
                    SignalNode::AD { state, .. } => {
                        registry.register(node_id, SharedState::AD(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }
                    SignalNode::ASR { state, .. } => {
                        registry.register(node_id, SharedState::ASR(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }
                    SignalNode::Lag { state, .. } => {
                        registry.register(node_id, SharedState::Lag(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }
                    SignalNode::XLine { state, .. } => {
                        registry.register(node_id, SharedState::XLine(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }
                    SignalNode::Impulse { state, .. } => {
                        registry.register(node_id, SharedState::Impulse(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }

                    // === Medium priority: Modulation effects ===
                    SignalNode::Chorus { state, .. } => {
                        registry.register(node_id, SharedState::Chorus(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }
                    SignalNode::Flanger { state, .. } => {
                        registry.register(node_id, SharedState::Flanger(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }
                    SignalNode::BitCrush { state, .. } => {
                        registry.register(node_id, SharedState::BitCrush(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }
                    SignalNode::Phaser { phase, allpass_z1, allpass_y1, feedback_sample, .. } => {
                        registry.register(node_id, SharedState::Phaser(
                            Arc::new(RwLock::new(PhaserState {
                                phase: *phase,
                                allpass_z1: allpass_z1.clone(),
                                allpass_y1: allpass_y1.clone(),
                                feedback_sample: *feedback_sample,
                            }))
                        ));
                    }
                    SignalNode::Vibrato { phase, delay_buffer, buffer_pos, .. } => {
                        registry.register(node_id, SharedState::Vibrato(
                            Arc::new(RwLock::new(VibratoState {
                                phase: *phase,
                                delay_buffer: delay_buffer.clone(),
                                buffer_pos: *buffer_pos,
                            }))
                        ));
                    }
                    SignalNode::Tremolo { phase, .. } => {
                        registry.register(node_id, SharedState::TremoloPhase(
                            Arc::new(RwLock::new(*phase))
                        ));
                    }
                    SignalNode::RingMod { phase, .. } => {
                        registry.register(node_id, SharedState::RingModPhase(
                            Arc::new(RwLock::new(*phase))
                        ));
                    }

                    // === Medium priority: Dynamics ===
                    SignalNode::Compressor { state, .. } |
                    SignalNode::SidechainCompressor { state, .. } => {
                        registry.register(node_id, SharedState::Compressor(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }
                    SignalNode::Expander { state, .. } => {
                        registry.register(node_id, SharedState::Expander(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }

                    // === Medium priority: Synthesis ===
                    SignalNode::Granular { state, .. } => {
                        registry.register(node_id, SharedState::Granular(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }
                    SignalNode::KarplusStrong { state, .. } => {
                        registry.register(node_id, SharedState::KarplusStrong(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }
                    SignalNode::Waveguide { state, .. } => {
                        registry.register(node_id, SharedState::Waveguide(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }
                    SignalNode::Additive { state, .. } => {
                        registry.register(node_id, SharedState::Additive(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }
                    SignalNode::Vocoder { state, .. } => {
                        registry.register(node_id, SharedState::Vocoder(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }
                    SignalNode::PitchShift { state, .. } => {
                        registry.register(node_id, SharedState::PitchShift(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }

                    // === Lower priority: Noise generators ===
                    SignalNode::PinkNoise { state } => {
                        registry.register(node_id, SharedState::PinkNoise(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }
                    SignalNode::BrownNoise { state } => {
                        registry.register(node_id, SharedState::BrownNoise(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }

                    // === Lower priority: Analysis ===
                    SignalNode::RMS { buffer, write_idx, .. } => {
                        registry.register(node_id, SharedState::RMS(
                            Arc::new(RwLock::new(RMSState {
                                buffer: buffer.clone(),
                                write_idx: *write_idx,
                            }))
                        ));
                    }
                    SignalNode::SpectralFreeze { state, .. } => {
                        registry.register(node_id, SharedState::SpectralFreeze(
                            Arc::new(RwLock::new(state.clone()))
                        ));
                    }
                    SignalNode::AmpFollower { buffer, write_idx, current_envelope, .. } => {
                        registry.register(node_id, SharedState::AmpFollower(
                            Arc::new(RwLock::new(AmpFollowerState {
                                buffer: buffer.clone(),
                                write_idx: *write_idx,
                                current_envelope: *current_envelope,
                            }))
                        ));
                    }
                    SignalNode::PeakFollower { current_peak, .. } => {
                        registry.register(node_id, SharedState::PeakFollower(
                            Arc::new(RwLock::new(*current_peak))
                        ));
                    }

                    // Non-stateful nodes - skip
                    _ => {}
                }
            }
        }

        self.shared_state = Some(registry);
    }

    /// Create a clone for parallel rendering that shares state with this graph
    ///
    /// Unlike regular clone() which deep-copies all state, this creates a clone
    /// that references the same SharedStateRegistry. All clones will read/write
    /// to the same shared state via Arc<RwLock<>>.
    ///
    /// IMPORTANT: enable_shared_state() must be called first.
    pub fn clone_for_parallel(&self) -> Self {
        // Regular clone already preserves shared_state (it's Option<SharedStateRegistry>
        // which clones the Arc inside, not the data)
        self.clone()
    }

    /// Check if this graph contains effects that require sequential processing
    ///
    /// Returns true if the graph contains reverbs, delays, or other effects with
    /// long-tail dependencies that would be corrupted by parallel block processing.
    /// When this returns true, parallel rendering should fall back to sequential mode.
    pub fn has_sequential_dependencies(&self) -> bool {
        for opt_node in &self.nodes {
            if let Some(node_rc) = opt_node {
                match &**node_rc {
                    // Long-tail effects with sequential dependencies
                    SignalNode::LushReverb { .. } |
                    SignalNode::Reverb { .. } |
                    SignalNode::DattorroReverb { .. } |
                    SignalNode::Delay { .. } |
                    SignalNode::TapeDelay { .. } |
                    SignalNode::MultiTapDelay { .. } |
                    SignalNode::PingPongDelay { .. } |
                    SignalNode::Comb { .. } |
                    SignalNode::Convolution { .. } |
                    SignalNode::SpectralFreeze { .. } |
                    SignalNode::Granular { .. } |
                    SignalNode::KarplusStrong { .. } |
                    SignalNode::Waveguide { .. } |
                    SignalNode::Vocoder { .. } |
                    SignalNode::PitchShift { .. } => {
                        return true;
                    }
                    _ => {}
                }
            }
        }
        false
    }

    /// Apply sequential effects (reverb, delay) to a pre-rendered dry sample
    /// Used in pipelined rendering where dry signal is computed in parallel
    /// and sequential effects are applied in a separate pass
    pub fn apply_sequential_effects(&mut self, dry_sample: f32) -> f32 {
        // First, find LushReverb node IDs and clone their parameter signals
        let mut reverb_nodes: Vec<(usize, Signal, Signal, Signal, Signal, Signal, Signal, Signal, Signal, Signal)> = Vec::new();

        for (node_id, opt_node) in self.nodes.iter().enumerate() {
            if let Some(node_rc) = opt_node {
                if let SignalNode::LushReverb {
                    predelay, decay, size, diffusion, damping,
                    spin, wander, freeze, mix, ..
                } = &**node_rc {
                    reverb_nodes.push((
                        node_id,
                        predelay.clone(), decay.clone(), size.clone(),
                        diffusion.clone(), damping.clone(), spin.clone(),
                        wander.clone(), freeze.clone(), mix.clone()
                    ));
                }
            }
        }

        let mut result = dry_sample;

        // Now process each reverb node
        for (node_id, predelay, decay, size, diffusion, damping, spin, wander, freeze, mix) in reverb_nodes {
            // Evaluate reverb parameters
            let predelay_val = self.eval_signal(&predelay);
            let decay_val = self.eval_signal(&decay);
            let size_val = self.eval_signal(&size);
            let diffusion_val = self.eval_signal(&diffusion);
            let damping_val = self.eval_signal(&damping);
            let spin_val = self.eval_signal(&spin);
            let wander_val = self.eval_signal(&wander);
            let freeze_val = self.eval_signal(&freeze);
            let mix_val = self.eval_signal(&mix);

            // Get mutable access to state and process
            if let Some(Some(node_rc)) = self.nodes.get_mut(node_id) {
                let node = std::rc::Rc::make_mut(node_rc);
                if let SignalNode::LushReverb { state: s, .. } = node {
                    result = s.process(
                        result, // Chain through previous reverbs if multiple
                        predelay_val,
                        decay_val,
                        size_val,
                        diffusion_val,
                        damping_val,
                        spin_val,
                        wander_val,
                        freeze_val,
                        mix_val,
                    );
                }
            }
        }

        result
    }

    /// Preload all samples referenced in pattern nodes
    /// This should be called before swapping a graph into the audio thread
    /// to avoid disk I/O during audio processing
    pub fn preload_samples(&self) {
        use std::collections::HashSet;
        let mut sample_names: HashSet<String> = HashSet::new();

        // Walk through all nodes and collect sample names from Pattern<String> patterns
        for opt_node in &self.nodes {
            if let Some(node_rc) = opt_node {
                match &**node_rc {
                    SignalNode::Sample { pattern, .. } => {
                        // Query the pattern for several cycles to capture all samples
                        // (handles euclidean patterns, alternation, etc.)
                        for cycle in 0..16 {
                            let state = crate::pattern::State {
                                span: crate::pattern::TimeSpan::new(
                                    crate::pattern::Fraction::from_float(cycle as f64),
                                    crate::pattern::Fraction::from_float((cycle + 1) as f64),
                                ),
                                controls: std::collections::HashMap::new(),
                            };
                            for event in pattern.query(&state) {
                                // Skip rest markers
                                let name = event.value.trim();
                                if !name.is_empty() && name != "~" {
                                    sample_names.insert(name.to_string());
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Preload all discovered samples
        if !sample_names.is_empty() {
            let mut bank = self.sample_bank.borrow_mut();
            let start = std::time::Instant::now();
            for name in &sample_names {
                let _ = bank.get_sample(name);
            }
            eprintln!(
                "🎵 Preloaded {} samples in {:.1}ms",
                sample_names.len(),
                start.elapsed().as_secs_f64() * 1000.0
            );
        }
    }

    /// Get reference to buffer cache enabled flag (for testing)
    pub fn buffer_cache_enabled(&self) -> &std::cell::Cell<bool> {
        &self.buffer_cache_enabled
    }

    /// Seek to a specific sample position (for parallel offline rendering)
    /// This advances the graph's internal time state without processing audio
    pub fn seek_to_sample(&mut self, sample_index: usize) {
        self.sample_count = sample_index;
        // Update cycle position based on sample count (offline timing)
        if !self.use_wall_clock {
            let time_in_seconds = sample_index as f64 / self.sample_rate as f64;
            self.cached_cycle_position = time_in_seconds * self.cps as f64 + self.cycle_offset;
        }
    }

    /// Take the VoiceManager out of this graph (for transfer to new graph)
    /// Replaces with a fresh VoiceManager
    pub fn take_voice_manager(&mut self) -> crate::voice_manager::VoiceManager {
        use std::mem;
        let fresh_vm = crate::voice_manager::VoiceManager::new();
        mem::replace(self.voice_manager.get_mut(), fresh_vm)
    }

    /// Transfer a VoiceManager into this graph (from old graph)
    /// Release all voices with quick fade to prevent accumulation during rapid graph swaps
    pub fn transfer_voice_manager(&mut self, mut voice_manager: crate::voice_manager::VoiceManager) {
        // Release synthesis voices - they reference old graph's node IDs which no longer exist
        voice_manager.release_synthesis_voices();
        // Release sample voices - they would accumulate during rapid graph swaps
        voice_manager.release_sample_voices();
        *self.voice_manager.get_mut() = voice_manager;
    }

    /// Get the number of nodes in the graph (for diagnostics)
    pub fn node_count(&self) -> usize {
        self.nodes.iter().filter(|n| n.is_some()).count()
    }

    /// Get the voice pool size (for diagnostics)
    pub fn voice_pool_size(&self) -> usize {
        self.voice_manager.borrow().pool_size()
    }

    /// Transfer session timing from old graph to maintain global clock continuity
    /// This ensures the beat never drops during graph reload
    ///
    /// IMPORTANT: We DO NOT transfer cps - the new graph's tempo from code should be respected!
    /// Only the clock (session_start_time, cycle_offset) is transferred for continuity.
    pub fn transfer_session_timing(&mut self, old_graph: &UnifiedSignalGraph) {
        // CRITICAL: Enable wall-clock mode (must be done BEFORE transferring session_start_time)
        self.use_wall_clock = true;

        // CRITICAL: Transfer the wall-clock reference point
        self.session_start_time = old_graph.session_start_time;

        // Adjust cycle_offset to account for any CPS difference
        // If old graph had different CPS, we need to compute what cycle position the
        // old graph was at, then set our offset so we continue from that position
        let old_elapsed = old_graph.session_start_time.elapsed().as_secs_f64();
        let old_cycle_pos = old_elapsed * old_graph.cps as f64 + old_graph.cycle_offset;

        // Set our offset so that: old_elapsed * new_cps + new_offset = old_cycle_pos
        // => new_offset = old_cycle_pos - old_elapsed * new_cps
        self.cycle_offset = old_cycle_pos - old_elapsed * self.cps as f64;

        // DEBUG: Log timing transfer details
        eprintln!("🔍 TIMING TRANSFER:");
        eprintln!(
            "   Old: elapsed={:.4}s, cps={:.2}, offset={:.4}, position={:.4}",
            old_elapsed, old_graph.cps, old_graph.cycle_offset, old_cycle_pos
        );
        eprintln!(
            "   New: cps={:.2}, offset={:.4}, will continue from position={:.4}",
            self.cps, self.cycle_offset, old_cycle_pos
        );
        eprintln!(
            "   Wall-clock mode: old={}, new={}",
            old_graph.use_wall_clock, self.use_wall_clock
        );

        // NOTE: We keep self.cps as-is (from compile_program's tempo: statement)
        // This allows tempo changes to take effect immediately!

        // CRITICAL: Transfer cycle bus cache to prevent spurious resynthesis on reload
        // Without this, new graph has cache_floor=-1, causing unnecessary cache invalidation
        self.cycle_bus_cache = old_graph.cycle_bus_cache.clone();

        // Also transfer the cached cycle position to ensure consistency
        self.cached_cycle_position = old_cycle_pos;

        // CRITICAL: Update ALL Sample/Pattern node states to prevent re-triggering
        // When we reload at (e.g.) cycle 5.3, nodes must know we've already processed up to 5.3
        // Without this, last_trigger_time defaults to -1.0 and events from 0.0-5.3 re-trigger!
        let current_cycle = old_cycle_pos.floor() as i32;

        for node_opt in self.nodes.iter_mut() {
            if let Some(node_rc) = node_opt {
                // Use Rc::make_mut to get mutable access (will clone if needed)
                let node = Rc::make_mut(node_rc);
                match node {
                    SignalNode::Sample {
                        last_cycle,
                        last_trigger_time,
                        ..
                    } => {
                        *last_cycle = current_cycle;
                        *last_trigger_time = old_cycle_pos as f32;
                    }
                    SignalNode::CycleTrigger { last_cycle, .. } => {
                        *last_cycle = current_cycle;
                    }
                    SignalNode::Pattern {
                        last_trigger_time, ..
                    } => {
                        *last_trigger_time = old_cycle_pos as f32;
                    }
                    _ => {}
                }
            }
        }

        eprintln!(
            "🔧 Updated {} nodes with cycle position {:.4}",
            self.nodes.iter().filter(|n| n.is_some()).count(),
            old_cycle_pos
        );
    }

    /// Extract all FX state from this graph for preservation across hot-swaps
    /// Returns a map keyed by (bus_name, fx_type, index) for matching during injection
    pub fn extract_fx_states(&self) -> FxStateMap {
        let mut state_map = FxStateMap::new();

        // Track FX counts per (bus, fx_type) for indexing
        let mut fx_counters: HashMap<(String, String), usize> = HashMap::new();

        // First, build a reverse map: node_id -> bus_name
        let mut node_to_bus: HashMap<usize, String> = HashMap::new();
        for (bus_name, &node_id) in &self.buses {
            // Walk the chain from this bus node, marking all nodes as belonging to this bus
            self.mark_nodes_for_bus(&mut node_to_bus, node_id.0, bus_name.clone());
        }
        // Also mark output chain nodes
        if let Some(output_id) = self.output {
            self.mark_nodes_for_bus(&mut node_to_bus, output_id.0, "out".to_string());
        }
        for (&_ch, &node_id) in &self.outputs {
            self.mark_nodes_for_bus(&mut node_to_bus, node_id.0, "out".to_string());
        }

        // Now extract state from all FX nodes
        for (idx, node_opt) in self.nodes.iter().enumerate() {
            if let Some(node_rc) = node_opt {
                let bus_name = node_to_bus
                    .get(&idx)
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());

                match &**node_rc {
                    SignalNode::Delay {
                        buffer, write_idx, ..
                    } => {
                        let key = self.make_fx_key(&mut fx_counters, &bus_name, "delay");
                        state_map.insert(
                            key,
                            ExtractedFxState::Delay {
                                buffer: buffer.clone(),
                                write_idx: *write_idx,
                            },
                        );
                    }
                    SignalNode::TapeDelay { state, .. } => {
                        let key = self.make_fx_key(&mut fx_counters, &bus_name, "tapedelay");
                        state_map.insert(key, ExtractedFxState::TapeDelay(state.clone()));
                    }
                    SignalNode::MultiTapDelay {
                        buffer, write_idx, ..
                    } => {
                        let key = self.make_fx_key(&mut fx_counters, &bus_name, "multitapdelay");
                        state_map.insert(
                            key,
                            ExtractedFxState::MultiTapDelay {
                                buffer: buffer.clone(),
                                write_idx: *write_idx,
                            },
                        );
                    }
                    SignalNode::PingPongDelay {
                        buffer_l,
                        buffer_r,
                        write_idx,
                        ..
                    } => {
                        let key = self.make_fx_key(&mut fx_counters, &bus_name, "pingpongdelay");
                        state_map.insert(
                            key,
                            ExtractedFxState::PingPongDelay {
                                buffer_l: buffer_l.clone(),
                                buffer_r: buffer_r.clone(),
                                write_idx: *write_idx,
                            },
                        );
                    }
                    SignalNode::Reverb { state, .. } => {
                        let key = self.make_fx_key(&mut fx_counters, &bus_name, "reverb");
                        state_map.insert(key, ExtractedFxState::Reverb(state.clone()));
                    }
                    SignalNode::DattorroReverb { state, .. } => {
                        let key = self.make_fx_key(&mut fx_counters, &bus_name, "dattorroreverb");
                        state_map.insert(key, ExtractedFxState::DattorroReverb(state.clone()));
                    }
                    SignalNode::Convolution { state, .. } => {
                        let key = self.make_fx_key(&mut fx_counters, &bus_name, "convolution");
                        state_map.insert(key, ExtractedFxState::Convolution(state.clone()));
                    }
                    SignalNode::Chorus { state, .. } => {
                        let key = self.make_fx_key(&mut fx_counters, &bus_name, "chorus");
                        state_map.insert(key, ExtractedFxState::Chorus(state.clone()));
                    }
                    SignalNode::Flanger { state, .. } => {
                        let key = self.make_fx_key(&mut fx_counters, &bus_name, "flanger");
                        state_map.insert(key, ExtractedFxState::Flanger(state.clone()));
                    }
                    SignalNode::Compressor { state, .. } => {
                        let key = self.make_fx_key(&mut fx_counters, &bus_name, "compressor");
                        state_map.insert(key, ExtractedFxState::Compressor(state.clone()));
                    }
                    SignalNode::SidechainCompressor { state, .. } => {
                        let key =
                            self.make_fx_key(&mut fx_counters, &bus_name, "sidechaincompressor");
                        state_map.insert(key, ExtractedFxState::Compressor(state.clone()));
                    }
                    SignalNode::Expander { state, .. } => {
                        let key = self.make_fx_key(&mut fx_counters, &bus_name, "expander");
                        state_map.insert(key, ExtractedFxState::Expander(state.clone()));
                    }
                    SignalNode::LowPass { state, .. }
                    | SignalNode::HighPass { state, .. }
                    | SignalNode::BandPass { state, .. } => {
                        let key = self.make_fx_key(&mut fx_counters, &bus_name, "filter");
                        state_map.insert(key, ExtractedFxState::Filter(state.clone()));
                    }
                    SignalNode::MoogLadder { state, .. } => {
                        let key = self.make_fx_key(&mut fx_counters, &bus_name, "moogladder");
                        state_map.insert(key, ExtractedFxState::MoogLadder(state.clone()));
                    }
                    _ => {}
                }
            }
        }

        if std::env::var("DEBUG_FX_STATE").is_ok() {
            eprintln!("[FX_STATE] Extracted {} FX states", state_map.len());
            for (key, _state) in &state_map {
                eprintln!("  - {:?}", key);
            }
        }

        state_map
    }

    /// Helper: Create FX key and increment counter
    fn make_fx_key(
        &self,
        counters: &mut HashMap<(String, String), usize>,
        bus: &str,
        fx_type: &str,
    ) -> FxStateKey {
        let counter_key = (bus.to_string(), fx_type.to_string());
        let idx = *counters.get(&counter_key).unwrap_or(&0);
        counters.insert(counter_key, idx + 1);
        (bus.to_string(), fx_type.to_string(), idx)
    }

    /// Helper: Mark nodes as belonging to a bus (iterative to avoid stack overflow on deep chains)
    fn mark_nodes_for_bus(
        &self,
        node_to_bus: &mut HashMap<usize, String>,
        start_node_id: usize,
        bus_name: String,
    ) {
        let mut stack = vec![start_node_id];

        while let Some(node_id) = stack.pop() {
            // Don't overwrite if already marked (first assignment wins - closest to output)
            // Also prevents infinite loops on feedback graphs
            if node_to_bus.contains_key(&node_id) {
                continue;
            }
            node_to_bus.insert(node_id, bus_name.clone());

            // Add input nodes to stack
            if let Some(Some(node_rc)) = self.nodes.get(node_id) {
                let inputs = self.get_node_input_ids(&**node_rc);
                for input_id in inputs {
                    stack.push(input_id);
                }
            }
        }
    }

    // =======================================================================
    // Buffer-Passing Graph Infrastructure (supports cycles via 1-block delay)
    // =======================================================================

    /// Recursively collect all node IDs referenced by a Signal
    fn collect_signal_node_ids(&self, signal: &Signal, ids: &mut Vec<usize>) {
        match signal {
            Signal::Node(node_id) => {
                ids.push(node_id.0);
            }
            Signal::Bus(bus_name) => {
                // Resolve bus to its node ID
                if let Some(&node_id) = self.buses.get(bus_name) {
                    ids.push(node_id.0);
                }
            }
            Signal::Expression(expr) => {
                // Recursively collect from expression operands
                match &**expr {
                    SignalExpr::Add(a, b)
                    | SignalExpr::Multiply(a, b)
                    | SignalExpr::Subtract(a, b)
                    | SignalExpr::Divide(a, b)
                    | SignalExpr::Modulo(a, b)
                    | SignalExpr::Min(a, b) => {
                        self.collect_signal_node_ids(a, ids);
                        self.collect_signal_node_ids(b, ids);
                    }
                    SignalExpr::Scale { input, min, max } => {
                        self.collect_signal_node_ids(input, ids);
                        self.collect_signal_node_ids(min, ids);
                        self.collect_signal_node_ids(max, ids);
                    }
                }
            }
            Signal::Pattern(_) | Signal::Value(_) => {
                // No node dependencies
            }
        }
    }

    /// Build dependency map: node_id -> Vec of input node IDs
    /// Used for buffer-passing graph processing (cycles allowed via 1-block delay)
    pub fn build_dag_dependencies(&self) -> HashMap<usize, Vec<usize>> {
        let mut deps: HashMap<usize, Vec<usize>> = HashMap::new();

        for (node_id, node_opt) in self.nodes.iter().enumerate() {
            if let Some(node_rc) = node_opt {
                let input_ids = self.get_all_node_inputs(&**node_rc);
                deps.insert(node_id, input_ids);
            }
        }

        deps
    }

    /// Topological sort with cycle detection (Kahn's algorithm)
    /// Returns nodes in execution order. Cycles are handled by processing
    /// cyclic nodes after acyclic ones (they'll read from prev_node_buffers).
    pub fn topological_order(&self, deps: &HashMap<usize, Vec<usize>>) -> Vec<usize> {
        let mut in_degree: HashMap<usize, usize> = HashMap::new();
        let mut result = Vec::new();

        // Initialize in-degrees (all nodes start with 0)
        for &node_id in deps.keys() {
            in_degree.entry(node_id).or_insert(0);
            for &dep_id in deps.get(&node_id).unwrap_or(&vec![]) {
                // Ensure all dependencies are in the map
                in_degree.entry(dep_id).or_insert(0);
            }
        }

        // Calculate in-degrees
        for (&node_id, dep_list) in deps.iter() {
            for &dep_id in dep_list {
                if deps.contains_key(&dep_id) {
                    *in_degree.entry(node_id).or_insert(0) += 1;
                }
            }
        }

        // Start with nodes that have no dependencies
        let mut queue: std::collections::VecDeque<usize> = in_degree
            .iter()
            .filter(|&(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        // Sort queue for deterministic order
        let mut queue_vec: Vec<usize> = queue.drain(..).collect();
        queue_vec.sort();
        queue = queue_vec.into_iter().collect();

        while let Some(node_id) = queue.pop_front() {
            result.push(node_id);

            // Find nodes that depend on this one and decrement their in-degree
            for (&dependent_id, dep_list) in deps.iter() {
                if dep_list.contains(&node_id) {
                    if let Some(deg) = in_degree.get_mut(&dependent_id) {
                        *deg = deg.saturating_sub(1);
                        if *deg == 0 {
                            queue.push_back(dependent_id);
                        }
                    }
                }
            }
        }

        // Handle cycles: add remaining nodes in ID order
        let mut remaining: Vec<usize> = deps
            .keys()
            .filter(|id| !result.contains(id))
            .copied()
            .collect();
        remaining.sort();
        result.extend(remaining);

        result
    }

    /// Group nodes into parallel batches
    /// Nodes in the same batch have no dependencies on each other
    pub fn parallel_batches(
        &self,
        topo_order: &[usize],
        deps: &HashMap<usize, Vec<usize>>,
    ) -> Vec<Vec<usize>> {
        let mut node_batch: HashMap<usize, usize> = HashMap::new();
        let mut batches: Vec<Vec<usize>> = Vec::new();

        for &node_id in topo_order {
            // Find the maximum batch level of all dependencies
            let max_dep_batch = deps
                .get(&node_id)
                .map(|dep_list| {
                    dep_list
                        .iter()
                        .filter_map(|&dep_id| node_batch.get(&dep_id))
                        .max()
                        .copied()
                        .unwrap_or(0)
                })
                .unwrap_or(0);

            // This node goes in the batch AFTER its dependencies
            let this_batch = if deps.get(&node_id).map(|d| d.is_empty()).unwrap_or(true) {
                0 // No dependencies -> batch 0
            } else {
                max_dep_batch + 1
            };

            // Ensure we have enough batches
            while batches.len() <= this_batch {
                batches.push(Vec::new());
            }

            batches[this_batch].push(node_id);
            node_batch.insert(node_id, this_batch);
        }

        batches
    }

    /// Get ALL input node IDs from a SignalNode (comprehensive version)
    fn get_all_node_inputs(&self, node: &SignalNode) -> Vec<usize> {
        let mut inputs = Vec::new();

        // Helper macro to collect from a Signal field
        macro_rules! collect {
            ($signal:expr) => {
                self.collect_signal_node_ids($signal, &mut inputs);
            };
        }

        match node {
            // === Sources (no audio inputs, but may have parameter inputs) ===
            SignalNode::Oscillator { freq, .. } => {
                collect!(freq);
            }
            SignalNode::FMOscillator {
                carrier_freq,
                modulator_freq,
                mod_index,
                ..
            } => {
                collect!(carrier_freq);
                collect!(modulator_freq);
                collect!(mod_index);
            }
            SignalNode::PMOscillator {
                carrier_freq,
                modulation,
                mod_index,
                ..
            } => {
                collect!(carrier_freq);
                collect!(modulation);
                collect!(mod_index);
            }
            SignalNode::Constant { .. }
            | SignalNode::WhiteNoise { .. }
            | SignalNode::PinkNoise { .. }
            | SignalNode::BrownNoise { .. }
            | SignalNode::Pattern { .. }
            | SignalNode::Sample { .. }
            | SignalNode::PatternTrigger { .. } => {
                // No signal inputs for sources
            }

            // === Single input effects ===
            SignalNode::LowPass { input, cutoff, q, .. }
            | SignalNode::HighPass { input, cutoff, q, .. } => {
                collect!(input);
                collect!(cutoff);
                collect!(q);
            }
            SignalNode::BandPass { input, center, q, .. } => {
                collect!(input);
                collect!(center);
                collect!(q);
            }
            SignalNode::Delay {
                input,
                time,
                feedback,
                mix,
                ..
            } => {
                collect!(input);
                collect!(time);
                collect!(feedback);
                collect!(mix);
            }
            SignalNode::TapeDelay {
                input,
                time,
                feedback,
                wow_rate,
                wow_depth,
                flutter_rate,
                flutter_depth,
                saturation,
                mix,
                ..
            } => {
                collect!(input);
                collect!(time);
                collect!(feedback);
                collect!(wow_rate);
                collect!(wow_depth);
                collect!(flutter_rate);
                collect!(flutter_depth);
                collect!(saturation);
                collect!(mix);
            }
            SignalNode::MultiTapDelay {
                input,
                time,
                feedback,
                mix,
                ..
            } => {
                collect!(input);
                collect!(time);
                collect!(feedback);
                collect!(mix);
            }
            SignalNode::PingPongDelay {
                input,
                time,
                feedback,
                stereo_width,
                mix,
                ..
            } => {
                collect!(input);
                collect!(time);
                collect!(feedback);
                collect!(stereo_width);
                collect!(mix);
            }
            SignalNode::Reverb {
                input,
                room_size,
                damping,
                mix,
                ..
            } => {
                collect!(input);
                collect!(room_size);
                collect!(damping);
                collect!(mix);
            }
            SignalNode::DattorroReverb {
                input,
                pre_delay,
                decay,
                diffusion,
                damping,
                mod_depth,
                mix,
                ..
            } => {
                collect!(input);
                collect!(pre_delay);
                collect!(decay);
                collect!(diffusion);
                collect!(damping);
                collect!(mod_depth);
                collect!(mix);
            }
            SignalNode::LushReverb {
                input,
                predelay,
                decay,
                size,
                diffusion,
                damping,
                spin,
                wander,
                freeze,
                mix,
                ..
            } => {
                collect!(input);
                collect!(predelay);
                collect!(decay);
                collect!(size);
                collect!(diffusion);
                collect!(damping);
                collect!(spin);
                collect!(wander);
                collect!(freeze);
                collect!(mix);
            }
            SignalNode::Convolution { input, .. } => {
                collect!(input);
            }
            SignalNode::Distortion { input, drive, mix } => {
                collect!(input);
                collect!(drive);
                collect!(mix);
            }
            SignalNode::BitCrush {
                input,
                bits,
                sample_rate,
                ..
            } => {
                collect!(input);
                collect!(bits);
                collect!(sample_rate);
            }
            SignalNode::Chorus {
                input,
                rate,
                depth,
                mix,
                ..
            } => {
                collect!(input);
                collect!(rate);
                collect!(depth);
                collect!(mix);
            }
            SignalNode::Flanger {
                input,
                rate,
                depth,
                feedback,
                ..
            } => {
                collect!(input);
                collect!(rate);
                collect!(depth);
                collect!(feedback);
            }
            SignalNode::Compressor {
                input,
                threshold,
                ratio,
                attack,
                release,
                makeup_gain,
                ..
            } => {
                collect!(input);
                collect!(threshold);
                collect!(ratio);
                collect!(attack);
                collect!(release);
                collect!(makeup_gain);
            }
            SignalNode::Expander {
                input,
                threshold,
                ratio,
                attack,
                release,
                ..
            } => {
                collect!(input);
                collect!(threshold);
                collect!(ratio);
                collect!(attack);
                collect!(release);
            }
            SignalNode::MoogLadder {
                input,
                cutoff,
                resonance,
                ..
            } => {
                collect!(input);
                collect!(cutoff);
                collect!(resonance);
            }
            SignalNode::Limiter { input, threshold } => {
                collect!(input);
                collect!(threshold);
            }

            // === Binary operations ===
            SignalNode::Add { a, b }
            | SignalNode::Multiply { a, b }
            | SignalNode::Min { a, b } => {
                collect!(a);
                collect!(b);
            }

            // === Multi-input ===
            SignalNode::Mix { signals } => {
                for sig in signals {
                    collect!(sig);
                }
            }
            SignalNode::SidechainCompressor {
                main_input,
                sidechain_input,
                threshold,
                ratio,
                attack,
                release,
                ..
            } => {
                collect!(main_input);
                collect!(sidechain_input);
                collect!(threshold);
                collect!(ratio);
                collect!(attack);
                collect!(release);
            }

            // === Pass-through / utility ===
            SignalNode::Output { input } => {
                collect!(input);
            }

            // === Allpass filter ===
            SignalNode::Allpass {
                input, coefficient, ..
            } => {
                collect!(input);
                collect!(coefficient);
            }

            // === AmpFollower (envelope follower) ===
            SignalNode::AmpFollower {
                input,
                attack_time,
                release_time,
                window_size,
                ..
            } => {
                collect!(input);
                collect!(attack_time);
                collect!(release_time);
                collect!(window_size);
            }

            // === Catch-all for nodes not yet covered ===
            _ => {
                // Many more node types exist - add as needed
                // For now, use the legacy get_node_input_ids as fallback
                let legacy = self.get_node_input_ids(node);
                inputs.extend(legacy);
            }
        }

        // Deduplicate
        inputs.sort();
        inputs.dedup();
        inputs
    }

    /// Helper: Get input node IDs from a SignalNode (legacy - limited coverage)
    fn get_node_input_ids(&self, node: &SignalNode) -> Vec<usize> {
        let mut inputs = Vec::new();
        match node {
            SignalNode::Delay { input, .. }
            | SignalNode::TapeDelay { input, .. }
            | SignalNode::MultiTapDelay { input, .. }
            | SignalNode::Reverb { input, .. }
            | SignalNode::DattorroReverb { input, .. }
            | SignalNode::Convolution { input, .. }
            | SignalNode::Chorus { input, .. }
            | SignalNode::Flanger { input, .. }
            | SignalNode::Compressor { input, .. }
            | SignalNode::Expander { input, .. }
            | SignalNode::LowPass { input, .. }
            | SignalNode::HighPass { input, .. }
            | SignalNode::BandPass { input, .. }
            | SignalNode::MoogLadder { input, .. }
            | SignalNode::Distortion { input, .. }
            | SignalNode::BitCrush { input, .. } => {
                if let Signal::Node(id) = input {
                    inputs.push(id.0);
                }
            }
            SignalNode::PingPongDelay { input, .. } => {
                if let Signal::Node(id) = input {
                    inputs.push(id.0);
                }
            }
            SignalNode::SidechainCompressor {
                main_input,
                sidechain_input,
                ..
            } => {
                if let Signal::Node(id) = main_input {
                    inputs.push(id.0);
                }
                if let Signal::Node(id) = sidechain_input {
                    inputs.push(id.0);
                }
            }
            _ => {}
        }
        inputs
    }

    // ========================================================================
    // Buffer-Passing Graph Processing (Modular Synthesis Architecture)
    // ========================================================================

    /// Process audio buffer using dependency-ordered buffer passing
    ///
    /// This is the new modular synthesis architecture where:
    /// - Each node processes its entire buffer at once
    /// - Nodes read input buffers from their dependencies
    /// - Dependencies are processed in topological order (when acyclic)
    /// - CYCLES ARE ALLOWED: Feedback loops work via 1-block delay (prev_node_buffers)
    /// - Independent nodes can be parallelized (future work)
    ///
    /// # Arguments
    /// * `buffer` - Output buffer to fill (stereo interleaved)
    /// * `buffer_start_cycle` - Cycle position at buffer start
    /// * `sample_increment` - Cycles per sample
    pub fn process_buffer_dag(
        &mut self,
        buffer: &mut [f32],
        buffer_start_cycle: f64,
        sample_increment: f64,
    ) {
        static CALL_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let call = CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if std::env::var("DEBUG_DAG").is_ok() && call < 3 {
            eprintln!("process_buffer_dag called #{}", call);
        }

        let buffer_size = buffer.len() / 2; // Stereo: left/right pairs

        // Ensure zero buffer is correctly sized
        if self.dag_zero_buffer.len() != buffer_size {
            self.dag_zero_buffer = vec![0.0; buffer_size];
        }

        // CRITICAL: Initialize sample node timing (same as legacy path)
        for node_opt in self.nodes.iter_mut() {
            if let Some(node_rc) = node_opt {
                let node = std::rc::Rc::make_mut(node_rc);
                if let SignalNode::Sample {
                    last_trigger_time,
                    last_cycle,
                    ..
                } = node
                {
                    if *last_trigger_time < 0.0 {
                        *last_trigger_time = buffer_start_cycle as f32 - 0.001;
                        *last_cycle = buffer_start_cycle.floor() as i32;
                    }
                }
            }
        }

        // CRITICAL: Process voice buffers (same as legacy path)
        // This processes sample playback voices for the entire buffer
        self.voice_buffers = self
            .voice_manager
            .borrow_mut()
            .process_buffer_vec(buffer.len(), self.max_node_id);

        // CRITICAL: Precompute pattern events (same as legacy path)
        // This caches pattern events to avoid 512 query() calls per Pattern node
        self.precompute_pattern_events(buffer.len());

        // SYNTHESIS VOICES: Generate buffers for active synthesis voices
        // This enables bus-triggered synthesis (e.g., s "~synth*4")
        {
            let node_pitch_pairs: Vec<(usize, f32)> = self
                .voice_manager
                .borrow()
                .get_active_synthesis_node_ids_with_pitch();
            if !node_pitch_pairs.is_empty() {
                let mut synthesis_buffers: std::collections::HashMap<(usize, i32), Vec<f32>> =
                    std::collections::HashMap::new();

                for &(node_id, semitone_offset) in &node_pitch_pairs {
                    let semitone_key = (semitone_offset * 100.0).round() as i32;
                    let buffer_key = (node_id, semitone_key);

                    if synthesis_buffers.contains_key(&buffer_key) {
                        continue;
                    }

                    // Find oscillator nodes and save their state
                    let oscillator_ids = self.find_oscillator_nodes_in_chain(node_id);
                    let mut original_state: Vec<(usize, f32, f32)> = Vec::new();
                    for &osc_id in &oscillator_ids {
                        if let Some(Some(node_rc)) = self.nodes.get(osc_id) {
                            if let SignalNode::Oscillator {
                                semitone_offset: node_offset,
                                phase,
                                ..
                            } = &**node_rc
                            {
                                original_state.push((osc_id, *node_offset, *phase.borrow()));
                            }
                        }
                    }

                    // Apply pitch offset and restore per-pitch phase
                    for &osc_id in &oscillator_ids {
                        if let Some(Some(node_rc)) = self.nodes.get_mut(osc_id) {
                            let node = std::rc::Rc::make_mut(node_rc);
                            if let SignalNode::Oscillator {
                                semitone_offset: node_offset,
                                phase,
                                ..
                            } = node
                            {
                                let phase_key = (osc_id, semitone_key);
                                let cached_phase = self
                                    .synthesis_phase_cache
                                    .borrow()
                                    .get(&phase_key)
                                    .copied()
                                    .unwrap_or(0.0);
                                *node_offset = semitone_offset;
                                *phase.borrow_mut() = cached_phase;
                            }
                        }
                    }

                    // Clear buffer cache and generate synthesis buffer
                    self.buffer_cache.borrow_mut().clear();
                    let mut synth_buffer = vec![0.0; buffer_size];
                    self.eval_node_buffer(&NodeId(node_id), &mut synth_buffer);
                    synthesis_buffers.insert(buffer_key, synth_buffer);

                    // Save phase to cache for next buffer
                    for &osc_id in &oscillator_ids {
                        if let Some(Some(node_rc)) = self.nodes.get(osc_id) {
                            if let SignalNode::Oscillator { phase, .. } = &**node_rc {
                                let phase_key = (osc_id, semitone_key);
                                self.synthesis_phase_cache
                                    .borrow_mut()
                                    .insert(phase_key, *phase.borrow());
                            }
                        }
                    }

                    // Restore original oscillator state
                    for (osc_id, original_offset, original_phase) in original_state {
                        if let Some(Some(node_rc)) = self.nodes.get_mut(osc_id) {
                            let node = std::rc::Rc::make_mut(node_rc);
                            if let SignalNode::Oscillator {
                                semitone_offset: node_offset,
                                phase,
                                ..
                            } = node
                            {
                                *node_offset = original_offset;
                                *phase.borrow_mut() = original_phase;
                            }
                        }
                    }
                }

                // Process synthesis voices with envelopes and mix into voice_buffers
                let synthesis_voice_buffers = self
                    .voice_manager
                    .borrow_mut()
                    .process_synthesis_buffers(&synthesis_buffers, buffer_size);

                // Merge synthesis buffers into VoiceBuffers
                for (i, synth_outputs) in synthesis_voice_buffers.iter().enumerate() {
                    for (&source_node, &value) in synth_outputs {
                        if source_node < self.voice_buffers.buffers.len() {
                            // Ensure buffer is allocated for this node
                            if self.voice_buffers.buffers[source_node].is_empty() {
                                self.voice_buffers.buffers[source_node] = vec![0.0; buffer_size];
                            }
                            self.voice_buffers.buffers[source_node][i] += value;
                            if source_node >= self.voice_buffers.max_active_node {
                                self.voice_buffers.max_active_node = source_node + 1;
                            }
                        }
                    }
                }
            }
        }

        // Phase 1: Build dependency graph
        if std::env::var("DEBUG_DAG").is_ok() { eprintln!("  Building dependencies..."); }
        let deps = self.build_dag_dependencies();
        if std::env::var("DEBUG_DAG").is_ok() { eprintln!("  Building topo order..."); }
        let full_topo_order = self.topological_order(&deps);

        // Filter topo order based on whether we have buses:
        // - With buses: only process bus nodes and output. Intermediate expression nodes
        //   are evaluated inline when their containing bus is processed. This is critical
        //   for self-referential buses (z^-1 feedback) to work correctly.
        // - Without buses: process all nodes. This is the simple case with no feedback.
        let bus_node_ids: std::collections::HashSet<usize> =
            self.buses.values().map(|id| id.0).collect();
        let output_node_id = self.output.map(|id| id.0);

        // Collect numbered output node IDs (out1, out2, etc.)
        let numbered_output_ids: std::collections::HashSet<usize> =
            self.outputs.values().map(|id| id.0).collect();

        let topo_order: Vec<usize> = if bus_node_ids.is_empty() {
            // No buses - process all nodes
            full_topo_order
        } else {
            // Has buses - only process buses, main output, and numbered outputs
            full_topo_order
                .into_iter()
                .filter(|&node_id| {
                    bus_node_ids.contains(&node_id)
                        || Some(node_id) == output_node_id
                        || numbered_output_ids.contains(&node_id)
                })
                .collect()
        };

        if std::env::var("DEBUG_DAG").is_ok() { eprintln!("  Building batches..."); }
        let batches = self.parallel_batches(&topo_order, &deps);
        if std::env::var("DEBUG_DAG").is_ok() { eprintln!("  Starting batch processing..."); }

        if std::env::var("DEBUG_UNIT_DELAY").is_ok() && self.sample_count == 0 {
            eprintln!("=== DAG Processing ===");
            eprintln!("deps: {:?}", deps);
            eprintln!("topo_order: {:?}", topo_order);
            eprintln!("batches: {:?}", batches);
            eprintln!("buses: {:?}", self.buses);
            eprintln!("output: {:?}", self.output);
            // Print node types (abbreviated)
            for (i, node_opt) in self.nodes.iter().enumerate() {
                if let Some(node_rc) = node_opt {
                    let node_type = match &**node_rc {
                        SignalNode::Constant { value } => format!("Constant({})", value),
                        SignalNode::UnitDelay { bus_name } => format!("UnitDelay({})", bus_name),
                        SignalNode::Add { .. } => "Add".to_string(),
                        SignalNode::Multiply { .. } => "Multiply".to_string(),
                        SignalNode::Pattern { pattern_str, .. } => format!("Pattern({})", pattern_str),
                        SignalNode::Oscillator { waveform, .. } => format!("Oscillator({:?})", waveform),
                        _ => "Other".to_string(),
                    };
                    eprintln!("Node {}: {}", i, node_type);
                }
            }
        }

        // Temporary storage for this block's node outputs
        // Key: node_id, Value: mono buffer
        let mut current_buffers: HashMap<usize, Vec<f32>> = HashMap::new();

        // Clear the DAG buffer cache for this block and enter DAG processing mode
        self.dag_buffer_cache.clear();
        self.in_dag_processing = true;

        // Pre-allocate buffers for ALL buses BEFORE processing any nodes.
        // This is critical for self-referential buses (z^-1 feedback) to work.
        // UnitDelay nodes need the bus's buffer to exist so they can read previous samples.
        for &bus_node_id in self.buses.values() {
            self.dag_buffer_cache
                .insert(bus_node_id.0, vec![0.0; buffer_size]);
        }
        // Also pre-allocate for output node if different from buses
        if let Some(output_id) = self.output {
            if !self.dag_buffer_cache.contains_key(&output_id.0) {
                self.dag_buffer_cache
                    .insert(output_id.0, vec![0.0; buffer_size]);
            }
        }

        // Phase 2: Process batches (sequential between batches)
        for (batch_idx, batch) in batches.iter().enumerate() {
            if std::env::var("DEBUG_DAG").is_ok() { eprintln!("  Processing batch {}...", batch_idx); }
            // Process each node in the batch
            // (Phase 5 will parallelize this with rayon)
            for &node_id in batch {
                // Skip UnitDelay nodes - they are evaluated inline when their
                // containing expression needs them, not as standalone DAG nodes.
                // This is critical for z^-1 feedback to work correctly.
                let is_unit_delay = if let Some(Some(node_rc)) = self.nodes.get(node_id) {
                    matches!(&**node_rc, SignalNode::UnitDelay { .. })
                } else {
                    false
                };
                if is_unit_delay {
                    if std::env::var("DEBUG_DAG").is_ok() {
                        eprintln!("    Skipping UnitDelay node {}", node_id);
                    }
                    continue;
                }

                if std::env::var("DEBUG_DAG").is_ok() {
                    let node_type = if let Some(Some(node_rc)) = self.nodes.get(node_id) {
                        match &**node_rc {
                            SignalNode::LowPass { .. } => "LowPass",
                            SignalNode::Add { .. } => "Add",
                            SignalNode::Multiply { .. } => "Multiply",
                            SignalNode::Oscillator { .. } => "Oscillator",
                            SignalNode::UnitDelay { .. } => "UnitDelay",
                            SignalNode::Constant { .. } => "Constant",
                            _ => "Other",
                        }
                    } else {
                        "None"
                    };
                    eprintln!("    Processing node {} ({})...", node_id, node_type);
                }
                // Gather input buffers from predecessors
                let input_ids = deps.get(&node_id).cloned().unwrap_or_default();

                // Allocate output buffer for this node
                let mut node_output = vec![0.0; buffer_size];

                // Process this node
                self.eval_node_buffer_dag(
                    node_id,
                    &input_ids,
                    &current_buffers,
                    &mut node_output,
                    buffer_start_cycle,
                    sample_increment,
                );

                // Store output for downstream nodes
                current_buffers.insert(node_id, node_output.clone());

                // CRITICAL: Also update dag_buffer_cache so eval_signal_at_time can find cached values.
                // This is necessary for all nodes, not just buses, because later nodes may reference
                // earlier nodes through their input signals.
                self.dag_buffer_cache.insert(node_id, node_output);
            }
        }

        // Phase 3: Copy output to buffer (stereo interleave)
        // Check hushed_channels before outputting

        // Collect numbered outputs first to avoid borrow checker issues
        let output_channels: Vec<(usize, NodeId)> = self.outputs.iter()
            .map(|(&ch, &node)| (ch, node))
            .collect();

        // DEBUG: Log numbered outputs if enabled
        if std::env::var("DEBUG_OUTPUT").is_ok() && !output_channels.is_empty() {
            eprintln!("[DAG] Numbered outputs: {:?}", output_channels);
            eprintln!("[DAG] current_buffers keys: {:?}", current_buffers.keys().collect::<Vec<_>>());
        }

        // Count active channels for potential gain compensation
        let mut _num_active_channels = 0;

        // Handle main output (channel 0)
        let output_id = self.output.map(|id| id.0);
        if let Some(out_id) = output_id {
            // Check if channel 0 (main output) is hushed
            if !self.hushed_channels.contains(&0) {
                _num_active_channels += 1;
                if let Some(mono_buf) = current_buffers.get(&out_id) {
                    // Convert mono to stereo interleaved
                    for i in 0..buffer_size {
                        let sample = mono_buf[i];
                        buffer[i * 2] = sample;     // Left
                        buffer[i * 2 + 1] = sample; // Right
                    }
                }
            }
            // If hushed, buffer stays at 0.0 (initialized earlier)
        }

        // Handle numbered outputs (out1, out2, etc.)
        for (ch, node_id) in output_channels {
            // Skip hushed channels
            if self.hushed_channels.contains(&ch) {
                continue;
            }
            _num_active_channels += 1;

            if let Some(mono_buf) = current_buffers.get(&node_id.0) {
                // Mix into buffer (stereo interleaved)
                for i in 0..buffer_size {
                    buffer[i * 2] += mono_buf[i];     // Left
                    buffer[i * 2 + 1] += mono_buf[i]; // Right
                }
            }
        }

        // Phase 4: Swap buffers for feedback support
        // Move current buffers to prev_node_buffers for next block
        self.prev_node_buffers.clear();
        for (node_id, buf) in current_buffers.drain() {
            self.prev_node_buffers.insert(node_id, buf);
        }

        // Exit DAG processing mode
        self.in_dag_processing = false;

        // Update sample count for timing
        self.sample_count += buffer_size;
    }

    /// Get input buffer for a node (current block or previous block for feedback)
    fn get_dag_input_buffer<'a>(
        &'a self,
        input_id: usize,
        current_buffers: &'a HashMap<usize, Vec<f32>>,
    ) -> &'a [f32] {
        // Try current block first
        if let Some(buf) = current_buffers.get(&input_id) {
            return buf.as_slice();
        }
        // Fall back to previous block (feedback)
        if let Some(buf) = self.prev_node_buffers.get(&input_id) {
            return buf.as_slice();
        }
        // Return zeros if no buffer available
        &self.dag_zero_buffer
    }

    /// Process a single node for the entire buffer (DAG mode)
    ///
    /// This is a placeholder that will be filled in Phase 4.
    /// For now, it delegates to per-sample evaluation for correctness.
    fn eval_node_buffer_dag(
        &mut self,
        node_id: usize,
        input_ids: &[usize],
        current_buffers: &HashMap<usize, Vec<f32>>,
        output: &mut [f32],
        buffer_start_cycle: f64,
        sample_increment: f64,
    ) {
        let buffer_size = output.len();

        // Get the node
        let node_opt = self.nodes.get(node_id).and_then(|n| n.clone());
        if node_opt.is_none() {
            return;
        }
        let node_rc = node_opt.unwrap();
        let node = &*node_rc;

        // Skip UnitDelay nodes - they should only be evaluated fresh when their
        // containing bus expression is processed, not as standalone DAG nodes.
        // This is critical for z^-1 feedback to work correctly.
        if matches!(node, SignalNode::UnitDelay { .. }) {
            // Don't create a buffer for UnitDelay - it will be evaluated fresh
            // when the containing expression needs it
            return;
        }

        // Pre-allocate a buffer in dag_buffer_cache for self-referential feedback (z^-1)
        // This allows UnitDelay to look at previous samples within the same buffer
        self.dag_buffer_cache.insert(node_id, vec![0.0; buffer_size]);

        // Track which node we're processing so UnitDelay can find the right buffer
        self.current_dag_node_id = Some(node_id);

        // Check if this is a bus node
        let is_bus = self.buses.values().any(|&bid| bid.0 == node_id);
        if std::env::var("DEBUG_UNIT_DELAY").is_ok() && self.sample_count < 20 {
            eprintln!("eval_node_buffer_dag: node_id={}, is_bus={}", node_id, is_bus);
        }

        // For now, use simple per-sample evaluation
        // Phase 4 will optimize this with block-based processing for efficiency
        for i in 0..buffer_size {
            let cycle_pos = buffer_start_cycle + (i as f64) * sample_increment;
            self.cached_cycle_position = cycle_pos;
            self.current_sample_idx = i;

            // CRITICAL: Clear stateful cache and call stack per sample (like legacy path)
            // Without this, oscillators return cached values instead of advancing
            self.stateful_value_cache.clear();
            self.eval_call_stack.clear();

            // Evaluate the node at this sample
            // This uses the existing eval_node infrastructure
            let sample = self.eval_node(&NodeId(node_id));
            output[i] = sample;

            // Update dag_buffer_cache immediately so UnitDelay can access previous samples
            // This is critical for self-referential feedback loops like ~accum $ 0.1 + ~accum * 0.9
            if let Some(cache_buf) = self.dag_buffer_cache.get_mut(&node_id) {
                cache_buf[i] = sample;
            }

            if std::env::var("DEBUG_UNIT_DELAY").is_ok() && is_bus && i < 5 {
                eprintln!("  bus sample[{}] = {}", i, sample);
            }
        }
    }

    /// Transfer FX state from old graph to this graph
    /// Matches by (bus_name, fx_type, index) and replaces nodes with state-injected versions
    pub fn transfer_fx_states(&mut self, old_graph: &UnifiedSignalGraph) {
        let state_map = old_graph.extract_fx_states();
        if state_map.is_empty() {
            return;
        }

        // Build node_to_bus map for this graph
        let mut node_to_bus: HashMap<usize, String> = HashMap::new();
        for (bus_name, &node_id) in &self.buses {
            self.mark_nodes_for_bus(&mut node_to_bus, node_id.0, bus_name.clone());
        }
        if let Some(output_id) = self.output {
            self.mark_nodes_for_bus(&mut node_to_bus, output_id.0, "out".to_string());
        }
        for (&_ch, &node_id) in &self.outputs {
            self.mark_nodes_for_bus(&mut node_to_bus, node_id.0, "out".to_string());
        }

        // Track FX counts for matching
        let mut fx_counters: HashMap<(String, String), usize> = HashMap::new();
        let mut transferred = 0;

        // Iterate through nodes and inject matching state
        for idx in 0..self.nodes.len() {
            let node_opt = self.nodes[idx].clone();
            if let Some(node_rc) = node_opt {
                let bus_name = node_to_bus
                    .get(&idx)
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());

                let new_node: Option<SignalNode> = match &*node_rc {
                    SignalNode::Delay {
                        input,
                        time,
                        feedback,
                        mix,
                        ..
                    } => {
                        let key = self.make_fx_key(&mut fx_counters, &bus_name, "delay");
                        if let Some(ExtractedFxState::Delay { buffer, write_idx }) =
                            state_map.get(&key)
                        {
                            transferred += 1;
                            Some(SignalNode::Delay {
                                input: input.clone(),
                                time: time.clone(),
                                feedback: feedback.clone(),
                                mix: mix.clone(),
                                buffer: buffer.clone(),
                                write_idx: *write_idx,
                            })
                        } else {
                            None
                        }
                    }
                    SignalNode::Reverb {
                        input,
                        room_size,
                        damping,
                        mix,
                        ..
                    } => {
                        let key = self.make_fx_key(&mut fx_counters, &bus_name, "reverb");
                        if let Some(ExtractedFxState::Reverb(state)) = state_map.get(&key) {
                            transferred += 1;
                            Some(SignalNode::Reverb {
                                input: input.clone(),
                                room_size: room_size.clone(),
                                damping: damping.clone(),
                                mix: mix.clone(),
                                state: state.clone(),
                            })
                        } else {
                            None
                        }
                    }
                    SignalNode::Chorus {
                        input,
                        rate,
                        depth,
                        mix,
                        ..
                    } => {
                        let key = self.make_fx_key(&mut fx_counters, &bus_name, "chorus");
                        if let Some(ExtractedFxState::Chorus(state)) = state_map.get(&key) {
                            transferred += 1;
                            Some(SignalNode::Chorus {
                                input: input.clone(),
                                rate: rate.clone(),
                                depth: depth.clone(),
                                mix: mix.clone(),
                                state: state.clone(),
                            })
                        } else {
                            None
                        }
                    }
                    SignalNode::Flanger {
                        input,
                        depth,
                        rate,
                        feedback,
                        ..
                    } => {
                        let key = self.make_fx_key(&mut fx_counters, &bus_name, "flanger");
                        if let Some(ExtractedFxState::Flanger(state)) = state_map.get(&key) {
                            transferred += 1;
                            Some(SignalNode::Flanger {
                                input: input.clone(),
                                depth: depth.clone(),
                                rate: rate.clone(),
                                feedback: feedback.clone(),
                                state: state.clone(),
                            })
                        } else {
                            None
                        }
                    }
                    SignalNode::Compressor {
                        input,
                        threshold,
                        ratio,
                        attack,
                        release,
                        makeup_gain,
                        ..
                    } => {
                        let key = self.make_fx_key(&mut fx_counters, &bus_name, "compressor");
                        if let Some(ExtractedFxState::Compressor(state)) = state_map.get(&key) {
                            transferred += 1;
                            Some(SignalNode::Compressor {
                                input: input.clone(),
                                threshold: threshold.clone(),
                                ratio: ratio.clone(),
                                attack: attack.clone(),
                                release: release.clone(),
                                makeup_gain: makeup_gain.clone(),
                                state: state.clone(),
                            })
                        } else {
                            None
                        }
                    }
                    SignalNode::LowPass {
                        input, cutoff, q, ..
                    } => {
                        let key = self.make_fx_key(&mut fx_counters, &bus_name, "filter");
                        if let Some(ExtractedFxState::Filter(state)) = state_map.get(&key) {
                            transferred += 1;
                            Some(SignalNode::LowPass {
                                input: input.clone(),
                                cutoff: cutoff.clone(),
                                q: q.clone(),
                                state: state.clone(),
                            })
                        } else {
                            None
                        }
                    }
                    SignalNode::HighPass {
                        input, cutoff, q, ..
                    } => {
                        let key = self.make_fx_key(&mut fx_counters, &bus_name, "filter");
                        if let Some(ExtractedFxState::Filter(state)) = state_map.get(&key) {
                            transferred += 1;
                            Some(SignalNode::HighPass {
                                input: input.clone(),
                                cutoff: cutoff.clone(),
                                q: q.clone(),
                                state: state.clone(),
                            })
                        } else {
                            None
                        }
                    }
                    SignalNode::BandPass {
                        input, center, q, ..
                    } => {
                        let key = self.make_fx_key(&mut fx_counters, &bus_name, "filter");
                        if let Some(ExtractedFxState::Filter(state)) = state_map.get(&key) {
                            transferred += 1;
                            Some(SignalNode::BandPass {
                                input: input.clone(),
                                center: center.clone(),
                                q: q.clone(),
                                state: state.clone(),
                            })
                        } else {
                            None
                        }
                    }
                    _ => {
                        // For other FX types, just count them
                        match &*node_rc {
                            SignalNode::TapeDelay { .. } => {
                                self.make_fx_key(&mut fx_counters, &bus_name, "tapedelay");
                            }
                            SignalNode::MultiTapDelay { .. } => {
                                self.make_fx_key(&mut fx_counters, &bus_name, "multitapdelay");
                            }
                            SignalNode::PingPongDelay { .. } => {
                                self.make_fx_key(&mut fx_counters, &bus_name, "pingpongdelay");
                            }
                            SignalNode::DattorroReverb { .. } => {
                                self.make_fx_key(&mut fx_counters, &bus_name, "dattorroreverb");
                            }
                            SignalNode::Convolution { .. } => {
                                self.make_fx_key(&mut fx_counters, &bus_name, "convolution");
                            }
                            SignalNode::SidechainCompressor { .. } => {
                                self.make_fx_key(
                                    &mut fx_counters,
                                    &bus_name,
                                    "sidechaincompressor",
                                );
                            }
                            SignalNode::Expander { .. } => {
                                self.make_fx_key(&mut fx_counters, &bus_name, "expander");
                            }
                            SignalNode::MoogLadder { .. } => {
                                self.make_fx_key(&mut fx_counters, &bus_name, "moogladder");
                            }
                            _ => {}
                        }
                        None
                    }
                };

                if let Some(new) = new_node {
                    self.nodes[idx] = Some(std::rc::Rc::new(new));
                }
            }
        }

        if std::env::var("DEBUG_FX_STATE").is_ok() || transferred > 0 {
            eprintln!("[FX_STATE] Transferred {} FX states", transferred);
        }
    }

    /// Reset cycles to 0 (like Tidal's resetCycles)
    pub fn reset_cycles(&mut self) {
        if self.use_wall_clock {
            // LIVE MODE: Reset wall-clock offset
            self.cycle_offset = 0.0;
            self.session_start_time = std::time::Instant::now();
            self.cached_cycle_position = 0.0;
        } else {
            // OFFLINE MODE: Directly set position
            self.cached_cycle_position = 0.0;
        }
    }

    /// Jump to a specific cycle position
    pub fn set_cycle(&mut self, cycle: f64) {
        if self.use_wall_clock {
            // LIVE MODE: Adjust offset to reach target cycle
            let elapsed = self.session_start_time.elapsed().as_secs_f64();
            self.cycle_offset = cycle - (elapsed * self.cps as f64);
            self.cached_cycle_position = cycle;
        } else {
            // OFFLINE MODE: Directly set position
            self.cached_cycle_position = cycle;
        }
    }

    /// Nudge timing by a small amount
    /// Positive values shift later (delay), negative values shift earlier (advance)
    /// Example: nudge(0.01) delays by 0.01 cycles, nudge(-0.01) advances by 0.01 cycles
    pub fn nudge(&mut self, amount: f64) {
        if self.use_wall_clock {
            // LIVE MODE: Adjust offset
            self.cycle_offset += amount;
            self.cached_cycle_position += amount;
        } else {
            // OFFLINE MODE: Directly adjust position
            self.cached_cycle_position += amount;
        }
    }

    /// Get current cycle position from wall-clock time
    /// IMPORTANT: During sample processing, returns cached value (constant per sample)
    /// Only advances once per sample in process_sample()
    pub fn get_cycle_position(&self) -> f64 {
        self.cached_cycle_position
    }

    /// Get cycle position for a sample at a given offset from current position
    /// Used in buffer-based evaluation to calculate correct cycle position for each sample
    fn get_cycle_position_for_sample_offset(&self, sample_offset: usize) -> f64 {
        // Calculate how many cycles advance per sample
        let cycles_per_sample = self.cps as f64 / self.sample_rate as f64;

        // Add offset to current cached position
        self.cached_cycle_position + (sample_offset as f64 * cycles_per_sample)
    }

    /// Enable wall-clock based timing (for live mode)
    /// In live mode, timing is based on real wall-clock time
    /// This prevents drift and ensures beat never drops during code reloads
    pub fn enable_wall_clock_timing(&mut self) {
        // Preserve current cycle position when switching to wall-clock mode
        let current_position = self.cached_cycle_position;
        self.use_wall_clock = true;
        self.session_start_time = std::time::Instant::now();
        // Set offset so we start at the current position
        self.cycle_offset = current_position;
    }

    /// Update cached cycle position from clock or sample count
    /// Called once at the start of each sample
    fn update_cycle_position_from_clock(&mut self) {
        if self.use_wall_clock {
            // LIVE MODE: Wall-clock based - never drifts, survives underruns
            let elapsed = self.session_start_time.elapsed().as_secs_f64();
            self.cached_cycle_position = elapsed * self.cps as f64 + self.cycle_offset;
        } else {
            // OFFLINE RENDERING: Sample-count based - deterministic
            self.cached_cycle_position += self.cps as f64 / self.sample_rate as f64;
        }
    }

    /// Set cycle position by adjusting offset
    /// Used during graph reload to maintain timing continuity
    pub fn set_cycle_position(&mut self, position: f64) {
        // Calculate what offset would give us this position at current wall-clock time
        let elapsed = self.session_start_time.elapsed().as_secs_f64();
        self.cycle_offset = position - (elapsed * self.cps as f64);
        // Also update cache
        self.cached_cycle_position = position;

        // CRITICAL: Update ALL timing state in pattern nodes to prevent re-triggering
        // When we reload at cycle 5.3, nodes must know:
        // 1. We're already IN cycle 5 (not entering it for the first time)
        // 2. Don't re-trigger events that already happened earlier in this cycle
        // 3. last_trigger_time = current position means "act like we just processed up to here"
        let current_cycle = position.floor() as i32;

        for node_opt in self.nodes.iter_mut() {
            if let Some(node_rc) = node_opt {
                // Use Rc::make_mut to get mutable access (will clone if needed)
                let node = Rc::make_mut(node_rc);
                match node {
                    SignalNode::Sample {
                        last_cycle,
                        last_trigger_time,
                        ..
                    } => {
                        *last_cycle = current_cycle;
                        *last_trigger_time = position as f32;
                    }
                    SignalNode::CycleTrigger { last_cycle, .. } => {
                        *last_cycle = current_cycle;
                    }
                    SignalNode::Pattern {
                        last_trigger_time, ..
                    } => {
                        *last_trigger_time = position as f32;
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn set_output_mix_mode(&mut self, mode: OutputMixMode) {
        self.output_mix_mode = mode;
    }

    pub fn get_output_mix_mode(&self) -> OutputMixMode {
        self.output_mix_mode
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Write all tap buffers to their respective files
    /// Call this after rendering is complete to save debug recordings
    pub fn write_tap_files(&self) -> Vec<String> {
        let mut written_files = Vec::new();

        for node_option in &self.nodes {
            if let Some(node) = node_option {
                if let SignalNode::Tap { state, .. } = &**node {
                    if let Ok(tap_state) = state.lock() {
                        match tap_state.write_to_file() {
                            Ok(()) => {
                                written_files.push(tap_state.filename.clone());
                            }
                            Err(e) => {
                                eprintln!(
                                    "⚠️  Failed to write tap file {}: {}",
                                    tap_state.filename, e
                                );
                            }
                        }
                    }
                }
            }
        }

        written_files
    }

    /// Get a reference to a node by its ID
    pub fn get_node(&self, node_id: NodeId) -> Option<&SignalNode> {
        self.nodes
            .get(node_id.0)
            .and_then(|opt| opt.as_ref().map(|rc| &**rc))
    }

    /// Add a node to the graph and return its ID
    pub fn add_node(&mut self, node: SignalNode) -> NodeId {
        let id = NodeId(self.next_node_id);
        self.next_node_id += 1;

        // Track max node ID for VoiceBuffers pre-sizing
        if id.0 > self.max_node_id {
            self.max_node_id = id.0;
        }

        // Ensure vector is large enough
        while self.nodes.len() <= id.0 {
            self.nodes.push(None);
        }

        self.nodes[id.0] = Some(std::rc::Rc::new(node));
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

    /// Add an oscillator node (helper for testing)
    pub fn add_oscillator(&mut self, freq: Signal, waveform: Waveform) -> NodeId {
        use std::cell::RefCell;
        let node = SignalNode::Oscillator {
            freq,
            waveform,
            semitone_offset: 0.0,
            phase: RefCell::new(0.0),
            pending_freq: RefCell::new(None),
            last_sample: RefCell::new(0.0),
        };
        self.add_node(node)
    }

    /// Add an Add node (helper for testing)
    pub fn add_add_node(&mut self, a: Signal, b: Signal) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        let node = SignalNode::Add { a, b };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Add a Multiply node (helper for testing)
    pub fn add_multiply_node(&mut self, a: Signal, b: Signal) -> NodeId {
        let node = SignalNode::Multiply { a, b };
        self.add_node(node)
    }

    /// Add a Min node (helper for testing)
    pub fn add_min_node(&mut self, a: Signal, b: Signal) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        let node = SignalNode::Min { a, b };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Add a LowPass filter node (helper for testing)
    pub fn add_lowpass_node(&mut self, input: Signal, cutoff: Signal, q: Signal) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        let node = SignalNode::LowPass {
            input,
            cutoff,
            q,
            state: FilterState::default(),
        };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Add a HighPass filter node (helper for testing)
    pub fn add_highpass_node(&mut self, input: Signal, cutoff: Signal, q: Signal) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        let node = SignalNode::HighPass {
            input,
            cutoff,
            q,
            state: FilterState::default(),
        };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Add a BandPass filter node (helper for testing)
    pub fn add_bandpass_node(&mut self, input: Signal, center: Signal, q: Signal) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        let node = SignalNode::BandPass {
            input,
            center,
            q,
            state: FilterState::default(),
        };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Add a Notch filter node (helper for testing)
    pub fn add_notch_node(&mut self, input: Signal, center: Signal, q: Signal) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        let node = SignalNode::Notch {
            input,
            center,
            q,
            state: FilterState::default(),
        };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Add an SVF filter node (helper for testing)
    /// SVF (State Variable Filter) produces LP, HP, BP, and Notch outputs
    /// mode: 0=LP, 1=HP, 2=BP, 3=Notch
    pub fn add_svf_node(
        &mut self,
        input: Signal,
        frequency: Signal,
        resonance: Signal,
        mode: usize,
    ) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        let node = SignalNode::SVF {
            input,
            frequency,
            resonance,
            mode,
            state: SVFState::default(),
        };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Add a DJFilter node (helper for testing)
    /// DJ-style filter that sweeps from lowpass (0.0) through neutral (0.5) to highpass (1.0)
    pub fn add_djfilter_node(&mut self, input: Signal, value: Signal) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        let node = SignalNode::DJFilter {
            input,
            value,
            state: FilterState::default(),
        };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Add a Moog Ladder filter node (helper for testing)
    pub fn add_moogladder_node(
        &mut self,
        input: Signal,
        cutoff: Signal,
        resonance: Signal,
    ) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        let node = SignalNode::MoogLadder {
            input,
            cutoff,
            resonance,
            state: MoogLadderState::new(),
        };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Add a distortion node (waveshaper with drive and wet/dry mix)
    pub fn add_distortion_node(&mut self, input: Signal, drive: Signal, mix: Signal) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        let node = SignalNode::Distortion { input, drive, mix };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Add a chorus node (modulated delay for thickening/doubling effect)
    pub fn add_chorus_node(
        &mut self,
        input: Signal,
        rate: Signal,
        depth: Signal,
        mix: Signal,
    ) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        let node = SignalNode::Chorus {
            input,
            rate,
            depth,
            mix,
            state: ChorusState::new(self.sample_rate),
        };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Add a vibrato node (pitch modulation via LFO-controlled delay)
    pub fn add_vibrato_node(&mut self, input: Signal, rate: Signal, depth: Signal) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        let node = SignalNode::Vibrato {
            input,
            rate,
            depth,
            phase: 0.0,
            delay_buffer: Vec::new(),
            buffer_pos: 0,
        };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Add a comb filter node (feedback delay line for resonant effects)
    pub fn add_comb_node(&mut self, input: Signal, frequency: Signal, feedback: Signal) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        // Create delay buffer (max 2 seconds at sample rate)
        let max_delay_samples = (self.sample_rate * 2.0) as usize;
        let node = SignalNode::Comb {
            input,
            frequency,
            feedback,
            buffer: vec![0.0; max_delay_samples],
            write_pos: 0,
        };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Add a reverb node (Freeverb algorithm with room_size, damping, and wet/dry mix)
    pub fn add_reverb_node(
        &mut self,
        input: Signal,
        room_size: Signal,
        damping: Signal,
        mix: Signal,
    ) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        let node = SignalNode::Reverb {
            input,
            room_size,
            damping,
            mix,
            state: ReverbState::new(self.sample_rate),
        };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Add a Dattorro reverb node (professional studio-quality reverb based on Dattorro's 1997 paper)
    pub fn add_dattorroreverb_node(
        &mut self,
        input: Signal,
        pre_delay: Signal,
        decay: Signal,
        damping: Signal,
        diffusion: Signal,
        mix: Signal,
    ) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        let node = SignalNode::DattorroReverb {
            input,
            pre_delay,
            decay,
            diffusion,
            damping,
            mod_depth: Signal::Value(0.5), // Default moderate modulation
            mix,
            state: DattorroState::new(self.sample_rate),
        };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Add a parametric EQ node (3-band peaking equalizer for mixing/mastering)
    /// Add a Convolution node (helper for testing)
    pub fn add_convolution_node(&mut self, input: Signal) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        let node = SignalNode::Convolution {
            input,
            state: ConvolutionState::new(self.sample_rate),
        };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    pub fn add_parametriceq_node(
        &mut self,
        input: Signal,
        low_freq: Signal,
        low_gain: Signal,
        low_q: Signal,
        mid_freq: Signal,
        mid_gain: Signal,
        mid_q: Signal,
        high_freq: Signal,
        high_gain: Signal,
        high_q: Signal,
    ) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        let node = SignalNode::ParametricEQ {
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
            state: ParametricEQState::new(),
        };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Add a tape delay node (vintage tape echo with wow, flutter, and saturation)
    pub fn add_tapedelay_node(
        &mut self,
        input: Signal,
        time: Signal,
        feedback: Signal,
        wow_rate: Signal,
        wow_depth: Signal,
        flutter_rate: Signal,
        flutter_depth: Signal,
        saturation: Signal,
        mix: Signal,
    ) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        let node = SignalNode::TapeDelay {
            input,
            time,
            feedback,
            wow_rate,
            wow_depth,
            flutter_rate,
            flutter_depth,
            saturation,
            mix,
            state: TapeDelayState::new(self.sample_rate),
        };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Add a compressor node (dynamic range compression with attack/release envelope)
    pub fn add_compressor_node(
        &mut self,
        input: Signal,
        threshold: Signal,
        ratio: Signal,
        attack: Signal,
        release: Signal,
        makeup_gain: Signal,
    ) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        let node = SignalNode::Compressor {
            input,
            threshold,
            ratio,
            attack,
            release,
            makeup_gain,
            state: CompressorState::new(),
        };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Add an expander node (upward expansion - boosts signals above threshold)
    pub fn add_expander_node(
        &mut self,
        input: Signal,
        threshold: Signal,
        ratio: Signal,
        attack: Signal,
        release: Signal,
    ) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        let node = SignalNode::Expander {
            input,
            threshold,
            ratio,
            attack,
            release,
            state: ExpanderState::new(),
        };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Add a bitcrush node (bit reduction + sample rate reduction for lo-fi effect)
    pub fn add_bitcrush_node(
        &mut self,
        input: Signal,
        bits: Signal,
        sample_rate: Signal,
    ) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        let node = SignalNode::BitCrush {
            input,
            bits,
            sample_rate,
            state: BitCrushState::default(),
        };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Add a tremolo node (amplitude modulation effect)
    pub fn add_tremolo_node(&mut self, input: Signal, rate: Signal, depth: Signal) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        let node = SignalNode::Tremolo {
            input,
            rate,
            depth,
            phase: 0.0, // Start at phase 0
        };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Add a RingMod node (helper for testing)
    pub fn add_ringmod_node(&mut self, input: Signal, freq: Signal) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        let node = SignalNode::RingMod {
            input,
            freq,
            phase: 0.0,
        };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Add a ping-pong delay node (helper for testing)
    pub fn add_pingpongdelay_node(
        &mut self,
        input: Signal,
        time: Signal,
        feedback: Signal,
        stereo_width: Signal,
        mix: Signal,
    ) -> NodeId {
        let buffer_size = (self.sample_rate * 2.0) as usize; // 2 second max delay
        let node_id = NodeId(self.nodes.len());
        let node = SignalNode::PingPongDelay {
            input,
            time,
            feedback,
            stereo_width,
            channel: false, // Start with left channel
            mix,
            buffer_l: vec![0.0; buffer_size],
            buffer_r: vec![0.0; buffer_size],
            write_idx: 0,
        };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Add a white noise generator node (helper for testing)
    pub fn add_whitenoise_node(&mut self) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        let node = SignalNode::WhiteNoise;
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Add a SpectralFreeze node (helper for testing)
    pub fn add_spectralfreeze_node(&mut self, input: Signal, trigger: Signal) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        let node = SignalNode::SpectralFreeze {
            input,
            trigger,
            state: SpectralFreezeState::new(),
        };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Add a Curve node (helper for testing)
    /// Creates a curved ramp from start to end over duration
    /// Curve parameter controls shape: < 0 = concave, 0 = linear, > 0 = convex
    pub fn add_curve_node(
        &mut self,
        start: Signal,
        end: Signal,
        duration: Signal,
        curve: Signal,
    ) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        let node = SignalNode::Curve {
            start,
            end,
            duration,
            curve,
            elapsed_time: 0.0,
        };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Add a Formant filter node
    /// Creates vocal tract resonances for vowel synthesis
    pub fn add_formant_node(
        &mut self,
        source: Signal,
        f1: Signal,
        f2: Signal,
        f3: Signal,
        bw1: Signal,
        bw2: Signal,
        bw3: Signal,
    ) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        let node = SignalNode::Formant {
            source,
            f1,
            f2,
            f3,
            bw1,
            bw2,
            bw3,
            state: FormantState::new(self.sample_rate),
        };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Add a Resonz (resonant bandpass) filter node
    pub fn add_resonz_node(&mut self, input: Signal, frequency: Signal, q: Signal) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        let node = SignalNode::Resonz {
            input,
            frequency,
            q,
            state: BiquadState::default(),
        };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Add a Waveguide (physical modeling) filter node
    pub fn add_waveguide_node(
        &mut self,
        freq: Signal,
        damping: Signal,
        pickup_position: Signal,
    ) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        let max_delay = (self.sample_rate / 20.0) as usize; // 20Hz = lowest freq
        let node = SignalNode::Waveguide {
            freq,
            damping,
            pickup_position,
            state: WaveguideState::new(max_delay),
            last_freq: 0.0,
        };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Add an XFade (crossfade) node (helper for testing)
    pub fn add_xfade_node(
        &mut self,
        signal_a: Signal,
        signal_b: Signal,
        position: Signal,
    ) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        let node = SignalNode::XFade {
            signal_a,
            signal_b,
            position,
        };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Add an XLine (exponential envelope) node (helper for testing)
    pub fn add_xline_node(&mut self, start: Signal, end: Signal, duration: Signal) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        let node = SignalNode::XLine {
            start,
            end,
            duration,
            state: XLineState::default(),
        };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Add a VCO (Voltage-Controlled Oscillator) node (helper for testing)
    pub fn add_vco_node(
        &mut self,
        frequency: Signal,
        waveform: Signal,
        pulse_width: Signal,
    ) -> NodeId {
        use std::cell::RefCell;
        let node_id = NodeId(self.nodes.len());
        let node = SignalNode::VCO {
            frequency,
            waveform,
            pulse_width,
            phase: RefCell::new(0.0),
        };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Set the output node
    pub fn set_output(&mut self, node_id: NodeId) {
        self.output = Some(node_id);
    }

    /// Check if output is set
    pub fn has_output(&self) -> bool {
        self.output.is_some() || !self.outputs.is_empty()
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

    /// Get breakdown of voice types (for diagnostics)
    /// Returns (sample_voices, synthesis_voices, free_voices)
    pub fn voice_type_breakdown(&self) -> (usize, usize, usize) {
        self.voice_manager.borrow().voice_type_breakdown()
    }

    // ========================================================================
    // DEPENDENCY ANALYSIS FOR BLOCK-BASED PARALLEL PROCESSING
    // ========================================================================

    /// Collect all Sample node IDs reachable from output nodes.
    /// These are the only Sample nodes that should trigger voices in Phase 1.
    fn collect_output_sample_nodes(&self) -> std::collections::HashSet<usize> {
        let mut sample_nodes = std::collections::HashSet::new();
        let mut visited = std::collections::HashSet::new();

        // Collect all output node IDs
        let mut output_ids: Vec<NodeId> = Vec::new();
        if let Some(output) = self.output {
            output_ids.push(output);
        }
        for &node_id in self.outputs.values() {
            output_ids.push(node_id);
        }

        // Traverse from each output to find reachable Sample nodes
        for output_id in output_ids {
            self.collect_sample_nodes_recursive(output_id, &mut visited, &mut sample_nodes);
        }

        sample_nodes
    }

    /// Recursively traverse graph to find Sample nodes
    fn collect_sample_nodes_recursive(
        &self,
        node_id: NodeId,
        visited: &mut std::collections::HashSet<NodeId>,
        sample_nodes: &mut std::collections::HashSet<usize>,
    ) {
        if visited.contains(&node_id) {
            return;
        }
        visited.insert(node_id);

        if let Some(Some(node_rc)) = self.nodes.get(node_id.0) {
            // If this is a Sample or SynthPattern node, add it to the set
            // Both need pattern evaluation and voice triggering in Phase 1
            if matches!(&**node_rc, SignalNode::Sample { .. } | SignalNode::SynthPattern { .. }) {
                sample_nodes.insert(node_id.0);
            }

            // Continue traversing to find dependencies
            // This handles nodes with Signal inputs
            self.traverse_node_for_samples(&**node_rc, visited, sample_nodes);
        }
    }

    /// Traverse a node's children to find Sample nodes
    fn traverse_node_for_samples(
        &self,
        node: &SignalNode,
        visited: &mut std::collections::HashSet<NodeId>,
        sample_nodes: &mut std::collections::HashSet<usize>,
    ) {
        // Extract all Signal fields from the node and traverse them
        match node {
            SignalNode::Oscillator { freq, .. } => {
                self.traverse_signal_for_samples(freq, visited, sample_nodes);
            }
            SignalNode::Add { a, b }
            | SignalNode::Multiply { a, b }
            | SignalNode::Min { a, b } => {
                self.traverse_signal_for_samples(a, visited, sample_nodes);
                self.traverse_signal_for_samples(b, visited, sample_nodes);
            }
            SignalNode::Mix { signals } => {
                for signal in signals {
                    self.traverse_signal_for_samples(signal, visited, sample_nodes);
                }
            }
            SignalNode::Output { input }
            | SignalNode::Delay { input, .. }
            | SignalNode::Allpass { input, .. }
            | SignalNode::Chorus { input, .. }
            | SignalNode::BitCrush { input, .. }
            | SignalNode::SampleAndHold { input, .. }
            | SignalNode::LowPass { input, .. }
            | SignalNode::HighPass { input, .. }
            | SignalNode::BandPass { input, .. }
            | SignalNode::Reverb { input, .. }
            | SignalNode::DattorroReverb { input, .. }
            | SignalNode::Convolution { input, .. }
            | SignalNode::MoogLadder { input, .. }
            | SignalNode::Limiter { input, .. }
            | SignalNode::SVF { input, .. }
            | SignalNode::Biquad { input, .. }
            | SignalNode::Resonz { input, .. }
            | SignalNode::RLPF { input, .. }
            | SignalNode::RHPF { input, .. }
            | SignalNode::DJFilter { input, .. }
            | SignalNode::Notch { input, .. }
            | SignalNode::Comb { input, .. }
            | SignalNode::Distortion { input, .. }
            | SignalNode::Pan2Left { input, .. }
            | SignalNode::Pan2Right { input, .. }
            | SignalNode::PitchShift { input, .. } => {
                self.traverse_signal_for_samples(input, visited, sample_nodes);
            }
            SignalNode::Sample { .. } | SignalNode::SynthPattern { .. } => {
                // Sample and SynthPattern nodes are leaf nodes for this traversal
                // (they don't have Signal children we need to traverse)
            }
            SignalNode::Constant { .. }
            | SignalNode::WhiteNoise
            | SignalNode::PinkNoise { .. }
            | SignalNode::BrownNoise { .. }
            | SignalNode::Noise { .. }
            | SignalNode::UnitDelay { .. }
            | SignalNode::VoiceOutput
            | SignalNode::MidiVoiceFreq
            | SignalNode::MidiVoiceGate => {
                // Leaf nodes - no children
            }
            _ => {
                // For any other nodes, we rely on the catchall
                // The main Sample/output traversal paths are covered above
            }
        }
    }

    /// Traverse a Signal to find Sample nodes
    fn traverse_signal_for_samples(
        &self,
        signal: &Signal,
        visited: &mut std::collections::HashSet<NodeId>,
        sample_nodes: &mut std::collections::HashSet<usize>,
    ) {
        match signal {
            Signal::Node(node_id) => {
                self.collect_sample_nodes_recursive(*node_id, visited, sample_nodes);
            }
            Signal::Bus(bus_name) => {
                if let Some(&bus_id) = self.buses.get(bus_name) {
                    self.collect_sample_nodes_recursive(bus_id, visited, sample_nodes);
                }
            }
            Signal::Expression(expr) => {
                self.traverse_expr_for_samples(expr, visited, sample_nodes);
            }
            Signal::Value(_) | Signal::Pattern(_) => {
                // No children
            }
        }
    }

    /// Traverse a SignalExpr to find Sample nodes
    fn traverse_expr_for_samples(
        &self,
        expr: &SignalExpr,
        visited: &mut std::collections::HashSet<NodeId>,
        sample_nodes: &mut std::collections::HashSet<usize>,
    ) {
        match expr {
            SignalExpr::Add(a, b)
            | SignalExpr::Multiply(a, b)
            | SignalExpr::Subtract(a, b)
            | SignalExpr::Divide(a, b)
            | SignalExpr::Modulo(a, b)
            | SignalExpr::Min(a, b) => {
                self.traverse_signal_for_samples(a, visited, sample_nodes);
                self.traverse_signal_for_samples(b, visited, sample_nodes);
            }
            SignalExpr::Scale { input, min, max } => {
                self.traverse_signal_for_samples(input, visited, sample_nodes);
                self.traverse_signal_for_samples(min, visited, sample_nodes);
                self.traverse_signal_for_samples(max, visited, sample_nodes);
            }
        }
    }

    /// Find all nodes that a given node depends on (recursive)
    fn find_node_dependencies(
        &self,
        node_id: NodeId,
        visited: &mut std::collections::HashSet<NodeId>,
    ) {
        if visited.contains(&node_id) {
            return; // Already visited (handles potential cycles)
        }
        visited.insert(node_id);

        if let Some(Some(node_rc)) = self.nodes.get(node_id.0) {
            let node = &**node_rc;

            // Add dependencies based on node type (partial implementation - will expand)
            match node {
                SignalNode::Oscillator { freq, .. } => {
                    self.find_signal_dependencies(freq, visited);
                }
                SignalNode::Add { a, b } => {
                    self.find_signal_dependencies(a, visited);
                    self.find_signal_dependencies(b, visited);
                }
                SignalNode::Multiply { a, b } => {
                    self.find_signal_dependencies(a, visited);
                    self.find_signal_dependencies(b, visited);
                }
                SignalNode::Min { a, b } => {
                    self.find_signal_dependencies(a, visited);
                    self.find_signal_dependencies(b, visited);
                }
                SignalNode::Wrap { input, min, max } => {
                    self.find_signal_dependencies(input, visited);
                    self.find_signal_dependencies(min, visited);
                    self.find_signal_dependencies(max, visited);
                }
                SignalNode::Output { input } => {
                    self.find_signal_dependencies(input, visited);
                }
                _ => {
                    // TODO: Handle all node types
                }
            }
        }
    }

    /// Find dependencies within a Signal
    fn find_signal_dependencies(
        &self,
        signal: &Signal,
        visited: &mut std::collections::HashSet<NodeId>,
    ) {
        match signal {
            Signal::Node(node_id) => {
                self.find_node_dependencies(*node_id, visited);
            }
            Signal::Bus(bus_name) => {
                if let Some(&bus_id) = self.buses.get(bus_name) {
                    self.find_node_dependencies(bus_id, visited);
                }
            }
            Signal::Expression(expr) => {
                self.find_expr_dependencies(expr, visited);
            }
            Signal::Value(_) | Signal::Pattern(_) => {
                // No dependencies
            }
        }
    }

    /// Find dependencies within a SignalExpr
    fn find_expr_dependencies(
        &self,
        expr: &SignalExpr,
        visited: &mut std::collections::HashSet<NodeId>,
    ) {
        match expr {
            SignalExpr::Add(a, b)
            | SignalExpr::Multiply(a, b)
            | SignalExpr::Subtract(a, b)
            | SignalExpr::Divide(a, b)
            | SignalExpr::Modulo(a, b)
            | SignalExpr::Min(a, b) => {
                self.find_signal_dependencies(a, visited);
                self.find_signal_dependencies(b, visited);
            }
            SignalExpr::Scale { input, min, max } => {
                self.find_signal_dependencies(input, visited);
                self.find_signal_dependencies(min, visited);
                self.find_signal_dependencies(max, visited);
            }
        }
    }

    /// Build a dependency graph for all outputs and buses
    pub fn build_dependency_graph(&self) -> DependencyGraph {
        let mut dep_graph = DependencyGraph::new();

        // Collect all output nodes
        let mut all_outputs: Vec<NodeId> = Vec::new();
        if let Some(output) = self.output {
            all_outputs.push(output);
        }
        all_outputs.extend(self.outputs.values().copied());

        // For each output, find its dependencies
        for &output_id in &all_outputs {
            let mut deps = std::collections::HashSet::new();
            self.find_node_dependencies(output_id, &mut deps);

            // Add edges to dependency graph
            for &dep_id in &deps {
                if dep_id != output_id {
                    dep_graph.add_dependency(output_id, dep_id);
                }
            }

            // IMPORTANT: Even if a node has no dependencies, we need it in the graph
            // Ensure it's in the dependencies map (with empty vec if needed)
            dep_graph
                .dependencies
                .entry(output_id)
                .or_insert_with(Vec::new);
        }

        // Also include bus dependencies
        for &bus_id in self.buses.values() {
            let mut deps = std::collections::HashSet::new();
            self.find_node_dependencies(bus_id, &mut deps);

            for &dep_id in &deps {
                if dep_id != bus_id {
                    dep_graph.add_dependency(bus_id, dep_id);
                }
            }

            // Ensure bus nodes are in the graph even if they have no dependencies
            dep_graph
                .dependencies
                .entry(bus_id)
                .or_insert_with(Vec::new);
        }

        dep_graph
    }

    /// Analyze the graph and compute execution stages for parallel processing
    pub fn compute_execution_stages(&self) -> Result<ExecutionStages, String> {
        let dep_graph = self.build_dependency_graph();
        dep_graph.topological_sort()
    }

    // ========================================================================
    // END DEPENDENCY ANALYSIS
    // ========================================================================

    // ========================================================================
    // BLOCK-BASED RENDERING (DAW-style parallel processing)
    // ========================================================================

    /// Evaluate a Signal by reading from pre-rendered buffers instead of recursively evaluating
    /// This is used in block-based rendering where dependencies are already in buffers
    fn eval_signal_from_buffers(&self, signal: &Signal, sample_idx: usize) -> f32 {
        match signal {
            Signal::Value(v) => *v,
            Signal::Node(node_id) => {
                // Read from pre-rendered buffer
                self.node_buffers
                    .get(node_id)
                    .and_then(|buf| buf.get(sample_idx))
                    .copied()
                    .unwrap_or(0.0)
            }
            Signal::Bus(bus_name) => {
                // Read from bus buffer
                self.buses
                    .get(bus_name)
                    .and_then(|bus_id| self.node_buffers.get(bus_id))
                    .and_then(|buf| buf.get(sample_idx))
                    .copied()
                    .unwrap_or(0.0)
            }
            Signal::Pattern(_pattern_str) => {
                // Pattern signals should be evaluated through their node
                // For now, return 0.0 as they should be handled by Pattern nodes
                0.0
            }
            Signal::Expression(expr) => self.eval_signal_expr_from_buffers(expr, sample_idx),
        }
    }

    /// Evaluate a SignalExpr by reading from pre-rendered buffers
    fn eval_signal_expr_from_buffers(&self, expr: &SignalExpr, sample_idx: usize) -> f32 {
        match expr {
            SignalExpr::Add(a, b) => {
                self.eval_signal_from_buffers(a, sample_idx)
                    + self.eval_signal_from_buffers(b, sample_idx)
            }
            SignalExpr::Subtract(a, b) => {
                self.eval_signal_from_buffers(a, sample_idx)
                    - self.eval_signal_from_buffers(b, sample_idx)
            }
            SignalExpr::Multiply(a, b) => {
                self.eval_signal_from_buffers(a, sample_idx)
                    * self.eval_signal_from_buffers(b, sample_idx)
            }
            SignalExpr::Divide(a, b) => {
                let divisor = self.eval_signal_from_buffers(b, sample_idx);
                if divisor.abs() < 1e-10 {
                    0.0
                } else {
                    self.eval_signal_from_buffers(a, sample_idx) / divisor
                }
            }
            SignalExpr::Modulo(a, b) => {
                let divisor = self.eval_signal_from_buffers(b, sample_idx);
                if divisor.abs() < 1e-10 {
                    0.0
                } else {
                    self.eval_signal_from_buffers(a, sample_idx) % divisor
                }
            }
            SignalExpr::Min(a, b) => self
                .eval_signal_from_buffers(a, sample_idx)
                .min(self.eval_signal_from_buffers(b, sample_idx)),
            SignalExpr::Scale { input, min, max } => {
                let val = self.eval_signal_from_buffers(input, sample_idx);
                let min_val = self.eval_signal_from_buffers(min, sample_idx);
                let max_val = self.eval_signal_from_buffers(max, sample_idx);
                // Scale from -1..1 to min..max
                let normalized = (val + 1.0) / 2.0; // -1..1 -> 0..1
                min_val + normalized * (max_val - min_val)
            }
        }
    }

    /// Evaluate a node at specific sample index by reading from dependency buffers
    /// This avoids recursive graph traversal by reading pre-rendered buffers
    fn eval_node_from_buffers(&self, node_id: &NodeId, sample_idx: usize) -> Option<f32> {
        let node = self.nodes.get(node_id.0)?.as_ref()?;

        match &**node {
            SignalNode::Add { a, b } => {
                let a_val = self.eval_signal_from_buffers(a, sample_idx);
                let b_val = self.eval_signal_from_buffers(b, sample_idx);
                Some(a_val + b_val)
            }
            SignalNode::Multiply { a, b } => {
                let a_val = self.eval_signal_from_buffers(a, sample_idx);
                let b_val = self.eval_signal_from_buffers(b, sample_idx);
                Some(a_val * b_val)
            }
            SignalNode::Min { a, b } => {
                let a_val = self.eval_signal_from_buffers(a, sample_idx);
                let b_val = self.eval_signal_from_buffers(b, sample_idx);
                Some(a_val.min(b_val))
            }
            SignalNode::Wrap { input, min, max } => {
                let input_val = self.eval_signal_from_buffers(input, sample_idx);
                let min_val = self.eval_signal_from_buffers(min, sample_idx);
                let max_val = self.eval_signal_from_buffers(max, sample_idx);

                let range = max_val - min_val;
                if range.abs() < 1e-10 {
                    return Some(min_val);
                }

                let normalized = (input_val - min_val) % range;
                let result = if normalized < 0.0 {
                    normalized + range + min_val
                } else {
                    normalized + min_val
                };
                Some(result)
            }
            SignalNode::Conditional {
                condition,
                then_signal,
                else_signal,
            } => {
                let cond_val = self.eval_signal_from_buffers(condition, sample_idx);
                if cond_val > 0.5 {
                    Some(self.eval_signal_from_buffers(then_signal, sample_idx))
                } else {
                    Some(self.eval_signal_from_buffers(else_signal, sample_idx))
                }
            }
            SignalNode::Select { index, inputs } => {
                if inputs.is_empty() {
                    return Some(0.0);
                }

                let index_val = self.eval_signal_from_buffers(index, sample_idx);
                let num_inputs = inputs.len();
                let selected_idx =
                    ((index_val.round() as i32).rem_euclid(num_inputs as i32)) as usize;

                Some(self.eval_signal_from_buffers(&inputs[selected_idx], sample_idx))
            }
            // For other node types, fall back to eval_node for now
            // TODO: Add buffer-based evaluation for all node types
            _ => None,
        }
    }

    /// Render a single node to its buffer (all samples in block)
    /// Reads from dependency buffers, writes to own buffer
    /// Pre-compute all cycle positions for a buffer
    /// This eliminates redundant calculations during rendering
    fn precompute_cycle_positions(&self, buffer_size: usize) -> Vec<f64> {
        let mut positions = Vec::with_capacity(buffer_size);

        if self.use_wall_clock {
            // LIVE MODE: Wall-clock based
            let base_elapsed = self.session_start_time.elapsed().as_secs_f64();
            let delta_per_sample = 1.0 / self.sample_rate as f64;

            for i in 0..buffer_size {
                let elapsed = base_elapsed + (i as f64 * delta_per_sample);
                positions.push(elapsed * self.cps as f64 + self.cycle_offset);
            }
        } else {
            // OFFLINE RENDERING: Sample-count based
            let mut position = self.cached_cycle_position;
            let delta = self.cps as f64 / self.sample_rate as f64;

            for _ in 0..buffer_size {
                positions.push(position);
                position += delta;
            }
        }

        positions
    }

    // ========================================================================
    // SAMPLE-BY-SAMPLE RENDERING
    // ========================================================================

    /// Process one sample and return all output channels
    /// Returns a vector where outputs[0] = channel 1, outputs[1] = channel 2, etc.
    pub fn process_sample_multi(&mut self) -> Vec<f32> {
        // CRITICAL: Update cycle position from wall-clock ONCE per sample
        self.update_cycle_position_from_clock();

        // OPTIMIZATION: Don't clear cache every sample!
        // Pattern values only change at event boundaries, not per-sample.
        // Clearing every sample forces re-evaluation of the entire graph 44,100 times/second.
        // This was causing 4x slowdown in file rendering vs buffer processing.
        // TODO: Only clear cache when cycle position crosses event boundary
        // self.value_cache.clear();

        // Process voice manager ONCE per sample and cache per-node outputs
        // This separates outputs so each output only hears its own samples
        // Sample nodes will look up their node ID in this cache
        self.voice_output_cache = self.voice_manager.borrow_mut().process_per_node();

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

        // NO cycle_position increment needed!
        // Clock is wall-clock based - it advances automatically via get_cycle_position()

        // Update z^-1 storage for feedback loops
        self.update_bus_previous_values();

        // Increment sample counter (for debugging only)
        self.sample_count += 1;

        outputs_vec
    }

    /// Evaluate a signal to get its current value
    #[inline(always)]
    fn eval_signal(&mut self, signal: &Signal) -> f32 {
        let cycle_position = self.get_cycle_position();
        self.eval_signal_at_time(signal, cycle_position)
    }

    /// Evaluate a signal for the note parameter (converts notes to semitone offsets)
    /// Reference pitch is C4 (MIDI 60) = 0 semitones
    fn eval_note_signal_at_time(&mut self, signal: &Signal, cycle_pos: f64) -> f32 {
        match signal {
            Signal::Node(id) => {
                if let Some(Some(node)) = self.nodes.get(id.0) {
                    if let SignalNode::Pattern { pattern, .. } = &**node {
                        // PERFORMANCE FIX: Use pre-computed pattern_event_cache if available
                        let events: Vec<crate::pattern::Hap<String>> =
                            if let Some(cached_events) = self.pattern_event_cache.get(id) {
                                cached_events
                                    .iter()
                                    .filter(|event| {
                                        let begin = event.part.begin.to_float();
                                        let end = event.part.end.to_float();
                                        cycle_pos >= begin && cycle_pos < end
                                    })
                                    .cloned()
                                    .collect()
                            } else {
                                let sample_width = 1.0 / self.sample_rate as f64 / self.cps as f64;
                                let state = State {
                                    span: TimeSpan::new(
                                        Fraction::from_float(cycle_pos),
                                        Fraction::from_float(cycle_pos + sample_width),
                                    ),
                                    controls: HashMap::new(),
                                };
                                pattern.query(&state)
                            };

                        if let Some(event) = events.first() {
                            let s = event.value.as_str();
                            if s == "~" || s.is_empty() {
                                0.0
                            } else {
                                use crate::pattern_tonal::note_to_midi;

                                // Try parsing as number first (semitone offset)
                                if let Ok(numeric_value) = s.parse::<f32>() {
                                    numeric_value
                                }
                                // Try parsing as note name (convert to semitone offset from C4)
                                else if let Some(midi) = note_to_midi(s) {
                                    (midi as i32 - 60) as f32 // C4 (MIDI 60) = 0 semitones
                                }
                                // Check for solfège
                                else {
                                    match s.to_lowercase().as_str() {
                                        "do" => 0.0,
                                        "re" => 2.0,
                                        "mi" => 4.0,
                                        "fa" => 5.0,
                                        "sol" | "so" => 7.0,
                                        "la" => 9.0,
                                        "ti" | "si" => 11.0,
                                        _ => 0.0, // Unknown, treat as 0
                                    }
                                }
                            }
                        } else {
                            0.0
                        }
                    } else {
                        self.eval_node(id)
                    }
                } else {
                    self.eval_node(id)
                }
            }
            Signal::Value(v) => *v,
            Signal::Bus(name) => {
                if let Some(id) = self.buses.get(name).cloned() {
                    // In DAG mode, check caches first to avoid infinite recursion
                    if let Some(buffer) = self.dag_buffer_cache.get(&id.0) {
                        if let Some(&value) = buffer.get(self.current_sample_idx) {
                            return value;
                        }
                    }
                    if let Some(buffer) = self.prev_node_buffers.get(&id.0) {
                        if let Some(&value) = buffer.get(self.current_sample_idx) {
                            return value;
                        }
                    }
                    if self.in_dag_processing {
                        return 0.0;
                    }
                    self.eval_node(&id)
                } else {
                    0.0
                }
            }
            Signal::Pattern(pattern_str) => {
                // Parse and evaluate inline pattern at specified cycle position
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
                    let s = event.value.as_str();
                    if s == "~" || s.is_empty() {
                        0.0
                    } else {
                        use crate::pattern_tonal::note_to_midi;

                        // Try parsing as number first (semitone offset)
                        if let Ok(numeric_value) = s.parse::<f32>() {
                            numeric_value
                        }
                        // Try parsing as note name (convert to semitone offset from C4)
                        else if let Some(midi) = note_to_midi(s) {
                            (midi as i32 - 60) as f32
                        }
                        // Check for solfège
                        else {
                            match s.to_lowercase().as_str() {
                                "do" => 0.0,
                                "re" => 2.0,
                                "mi" => 4.0,
                                "fa" => 5.0,
                                "sol" | "so" => 7.0,
                                "la" => 9.0,
                                "ti" | "si" => 11.0,
                                _ => 0.0,
                            }
                        }
                    }
                } else {
                    0.0
                }
            }
            Signal::Expression(expr) => self.eval_note_expression(expr),
        }
    }

    /// Evaluate a signal as a chord, returning pitch values
    ///
    /// Return value encoding:
    /// - Values >= 1000: Absolute MIDI note (subtract 1000 to get MIDI number)
    ///   e.g., 1060.0 = MIDI 60 = C4 = 261.63 Hz
    /// - Values < 1000: Relative semitone offset from base frequency
    ///   e.g., 12.0 = +12 semitones = 1 octave up
    ///
    /// For chords like "c4'maj" returns vec![1060.0, 1064.0, 1067.0] (C4, E4, G4 as MIDI)
    fn eval_note_signal_as_chord(&mut self, signal: &Signal, cycle_pos: f64) -> Vec<f32> {
        match signal {
            Signal::Node(id) => {
                if let Some(Some(node)) = self.nodes.get(id.0) {
                    if let SignalNode::Pattern { pattern, .. } = &**node {
                        let sample_width = 1.0 / self.sample_rate as f64 / self.cps as f64;
                        let state = State {
                            span: TimeSpan::new(
                                Fraction::from_float(cycle_pos),
                                Fraction::from_float(cycle_pos + sample_width),
                            ),
                            controls: HashMap::new(),
                        };

                        let events = pattern.query(&state);

                        if events.is_empty() {
                            vec![0.0]
                        } else {
                            use crate::pattern_tonal::{note_to_midi, CHORD_INTERVALS};

                            // Process ALL events (for chord notation like [a3, g3] with comma for simultaneous)
                            let mut all_notes = Vec::new();

                            for event in &events {
                                let s = event.value.as_str();

                                if s == "~" || s.is_empty() {
                                    // Skip rests, don't add 0.0 (that would affect other notes)
                                    continue;
                                }

                                // Check if this is chord notation (contains apostrophe)
                                if s.contains('\'') {
                                    // Parse chord: "c4'maj" -> root note + chord intervals
                                    // Return ABSOLUTE MIDI values (+ 1000 offset to distinguish from relative)
                                    if let Some(midi_root) = note_to_midi(s) {
                                        // Extract chord type from notation (everything after ')
                                        if let Some(apostrophe_pos) = s.find('\'') {
                                            let chord_type = &s[apostrophe_pos + 1..];

                                            // Look up chord intervals
                                            if let Some(intervals) = CHORD_INTERVALS.get(chord_type)
                                            {
                                                // Add root + all intervals as ABSOLUTE MIDI (+ 1000)
                                                for &interval in intervals.iter() {
                                                    all_notes.push(
                                                        1000.0 + midi_root as f32 + interval as f32,
                                                    );
                                                }
                                            } else {
                                                // Unknown chord type, just play root
                                                all_notes.push(1000.0 + midi_root as f32);
                                            }
                                        } else {
                                            all_notes.push(1000.0 + midi_root as f32);
                                        }
                                    }
                                } else {
                                    // Single note or numeric offset
                                    let note_value = if let Ok(numeric_value) = s.parse::<f32>() {
                                        // Numeric: direct semitone offset (RELATIVE)
                                        numeric_value
                                    } else if let Some(midi) = note_to_midi(s) {
                                        // Named note: ABSOLUTE MIDI (+ 1000)
                                        1000.0 + midi as f32
                                    } else {
                                        // Solfège: treat as relative semitones in current octave
                                        match s.to_lowercase().as_str() {
                                            "do" => 0.0,
                                            "re" => 2.0,
                                            "mi" => 4.0,
                                            "fa" => 5.0,
                                            "sol" | "so" => 7.0,
                                            "la" => 9.0,
                                            "ti" | "si" => 11.0,
                                            _ => 0.0,
                                        }
                                    };
                                    all_notes.push(note_value);
                                }
                            }

                            // If no valid notes were found (all rests), return empty vector
                            // This signals "don't play anything" rather than "play at default pitch"
                            all_notes
                        }
                    } else {
                        vec![self.eval_node(id)]
                    }
                } else {
                    vec![self.eval_node(id)]
                }
            }
            Signal::Value(v) => vec![*v],
            Signal::Bus(name) => {
                if let Some(id) = self.buses.get(name).cloned() {
                    // In DAG mode, check caches first to avoid infinite recursion
                    if let Some(buffer) = self.dag_buffer_cache.get(&id.0) {
                        if let Some(&value) = buffer.get(self.current_sample_idx) {
                            return vec![value];
                        }
                    }
                    if let Some(buffer) = self.prev_node_buffers.get(&id.0) {
                        if let Some(&value) = buffer.get(self.current_sample_idx) {
                            return vec![value];
                        }
                    }
                    if self.in_dag_processing {
                        return vec![0.0];
                    }
                    vec![self.eval_node(&id)]
                } else {
                    vec![0.0]
                }
            }
            Signal::Pattern(pattern_str) => {
                // For inline patterns, evaluate as single note
                let note_val = self.eval_note_signal_at_time(signal, cycle_pos);
                vec![note_val]
            }
            Signal::Expression(expr) => vec![self.eval_note_expression(expr)],
        }
    }

    /// Evaluate expression for note parameter (delegates to standard evaluation)
    fn eval_note_expression(&mut self, expr: &SignalExpr) -> f32 {
        self.eval_expression(expr)
    }

    /// Evaluate a signal at a specific cycle position
    /// This allows per-event DSP parameter evaluation
    fn eval_signal_at_time(&mut self, signal: &Signal, cycle_pos: f64) -> f32 {
        if std::env::var("DEBUG_DAG").is_ok() && self.sample_count < 2 && self.current_sample_idx == 0 {
            if let Signal::Node(id) = signal {
                eprintln!("        eval_signal_at_time: Node({}), call_stack_len={}", id.0, self.eval_call_stack.len());
            }
        }
        match signal {
            Signal::Node(id) => {
                // CYCLE DETECTION: Check if we're already evaluating this node.
                // If so, we have a circular reference and must break the cycle.
                if self.eval_call_stack.contains(&id.0) {
                    // Already evaluating this node - we have a cycle!
                    // Try to return a cached/previous value if available.
                    if let Some(buffer) = self.dag_buffer_cache.get(&id.0) {
                        if self.current_sample_idx > 0 {
                            // Use previous sample from current buffer (z^-1)
                            if let Some(&value) = buffer.get(self.current_sample_idx - 1) {
                                return value;
                            }
                        }
                    }
                    if let Some(buffer) = self.prev_node_buffers.get(&id.0) {
                        // Use value from previous block
                        if let Some(&value) = buffer.last() {
                            return value;
                        }
                    }
                    // No cached value available - return 0.0 to break cycle
                    return 0.0;
                }

                // In DAG mode, check if we have a pre-computed buffer first.
                // This prevents infinite recursion for circular dependencies.
                // Bus references get resolved to Signal::Node at compile time,
                // so cycles go through Signal::Node, not Signal::Bus.
                if self.in_dag_processing {
                    let is_current_bus = self.current_dag_node_id == Some(id.0);

                    if let Some(buffer) = self.dag_buffer_cache.get(&id.0) {
                        if is_current_bus {
                            // Self-reference: this IS the bus being processed.
                            // Return the PREVIOUS sample (z^-1 delay) to break the cycle.
                            if self.current_sample_idx > 0 {
                                if let Some(&value) = buffer.get(self.current_sample_idx - 1) {
                                    return value;
                                }
                            }
                            // At sample 0, check prev_node_buffers for last block's final sample
                            if let Some(prev_buffer) = self.prev_node_buffers.get(&id.0) {
                                if let Some(&value) = prev_buffer.last() {
                                    return value;
                                }
                            }
                            // First sample ever - return 0.0 to start accumulation
                            return 0.0;
                        } else {
                            // Different node: use cached value at current sample index
                            if let Some(&value) = buffer.get(self.current_sample_idx) {
                                return value;
                            }
                        }
                    }

                    // Check prev_node_buffers for nodes not in current cache
                    if let Some(buffer) = self.prev_node_buffers.get(&id.0) {
                        if let Some(&value) = buffer.get(self.current_sample_idx) {
                            return value;
                        }
                    }

                    // At this point, we've checked caches. For complex nodes that could
                    // recurse (filters, effects), we need to evaluate them. The eval_call_stack
                    // in eval_node will catch actual cycles.
                }

                // NOTE: Don't add to call stack here - eval_node handles call stack management.
                // The cycle check above uses the call stack that eval_node maintains.

                // CRITICAL FIX: For Pattern nodes, query at the specified cycle_pos
                // instead of self.get_cycle_position() to ensure each event gets the correct
                // parameter value from pattern-valued DSP parameters like gain "1.0 0.5"
                if let Some(Some(node)) = self.nodes.get(id.0) {
                    if let SignalNode::Pattern {
                        pattern,
                        pattern_str,
                        ..
                    } = &**node
                    {
                        // PERFORMANCE FIX: Use pre-computed pattern_event_cache if available
                        // This eliminates 44,100 pattern.query() calls per second per pattern parameter!
                        let (events, cache_hit): (Vec<crate::pattern::Hap<String>>, bool) =
                            if let Some(cached_events) = self.pattern_event_cache.get(id) {
                                // Filter cached events to find one active at cycle_pos
                                (cached_events
                                    .iter()
                                    .filter(|event| {
                                        let begin = event.part.begin.to_float();
                                        let end = event.part.end.to_float();
                                        cycle_pos >= begin && cycle_pos < end
                                    })
                                    .cloned()
                                    .collect(), true)
                            } else {
                                // Fallback: query pattern directly (shouldn't happen if cache is populated)
                                let sample_width = 1.0 / self.sample_rate as f64 / self.cps as f64;
                                let state = State {
                                    span: TimeSpan::new(
                                        Fraction::from_float(cycle_pos),
                                        Fraction::from_float(cycle_pos + sample_width),
                                    ),
                                    controls: HashMap::new(),
                                };
                                (pattern.query(&state), false)
                            };

                        // DEBUG: Log pattern signal evaluation
                        if std::env::var("DEBUG_PATTERN").is_ok()
                            && self.sample_count < 44200
                            && self.sample_count % 2200 == 0
                        {
                            eprintln!(
                                "Signal Pattern '{}' at cycle {:.6}, sample {}: {} events",
                                pattern_str,
                                cycle_pos,
                                self.sample_count,
                                events.len()
                            );
                            if let Some(event) = events.first() {
                                eprintln!(
                                    "  First event: '{}' at [{:.6}, {:.6})",
                                    event.value,
                                    event.part.begin.to_float(),
                                    event.part.end.to_float()
                                );
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
                } else {
                    self.eval_node(id)
                }
            }
            Signal::Bus(name) => {
                if let Some(id) = self.buses.get(name).cloned() {
                    // In DAG mode, check if we have a pre-computed buffer first.
                    // This prevents infinite recursion for circular bus dependencies
                    // (e.g., ~a -> ~b -> ~c -> ~a). For cycles, the bus value comes
                    // from the previous block's output (1-block delay).
                    if let Some(buffer) = self.dag_buffer_cache.get(&id.0) {
                        if let Some(&value) = buffer.get(self.current_sample_idx) {
                            return value;
                        }
                    }
                    // Check previous block's buffer for feedback loops
                    if let Some(buffer) = self.prev_node_buffers.get(&id.0) {
                        if let Some(&value) = buffer.get(self.current_sample_idx) {
                            return value;
                        }
                    }
                    // If we're in DAG processing mode and the bus hasn't been computed yet,
                    // this indicates a circular dependency. Return 0.0 to break the cycle.
                    // The bus will get its proper value after one block of delay.
                    if self.in_dag_processing {
                        // We're in DAG mode and this bus hasn't been computed yet.
                        // This is a cycle - return 0.0 to prevent infinite recursion.
                        return 0.0;
                    }
                    // Fallback to recursive evaluation (legacy path when not in DAG mode)
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
            SignalExpr::Min(a, b) => self.eval_signal(a).min(self.eval_signal(b)),
            SignalExpr::Scale { input, min, max } => {
                let v = self.eval_signal(input);
                let min_val = self.eval_signal(min);
                let max_val = self.eval_signal(max);
                v * (max_val - min_val) + min_val
            }
        }
    }

    /// Presynthesize buses in parallel (Phase 1 optimization)
    /// DISABLED: Now using continuous synthesis voices instead of pre-rendered buffers
    /// Returns empty HashMap to maintain API compatibility
    fn presynthesize_buses_parallel(
        &self,
        _events: &[crate::pattern::Hap<String>],
        _last_event_start: f64,
    ) -> HashMap<(String, usize), Arc<Vec<f32>>> {
        // OLD APPROACH (disabled): Pre-render buses to fixed-length buffers
        // NEW APPROACH: Trigger continuous synthesis voices that evaluate sample-by-sample
        return HashMap::new();
    }

    /// Evaluate a node to get its current output value
    #[inline(always)]
    fn eval_node(&mut self, node_id: &NodeId) -> f32 {
        // Use call_stack size as recursion depth indicator
        let depth = self.eval_call_stack.len();
        if depth > 100 {
            if std::env::var("DEBUG_OVERFLOW").is_ok() {
                eprintln!("DEEP eval_node: node_id={}, call_stack_len={}, stack={:?}",
                    node_id.0, depth, self.eval_call_stack);
            }
            // Safety limit to prevent actual stack overflow
            if depth > 500 {
                return 0.0;
            }
        }

        // CYCLE DETECTION: Check if we're already evaluating this node.
        // If so, we have a circular reference and must break the cycle.
        if self.eval_call_stack.contains(&node_id.0) {
            // Already evaluating this node - we have a cycle!
            // Try to return a cached/previous value if available.
            if let Some(buffer) = self.dag_buffer_cache.get(&node_id.0) {
                if self.current_sample_idx > 0 {
                    if let Some(&value) = buffer.get(self.current_sample_idx - 1) {
                        return value;
                    }
                }
            }
            if let Some(buffer) = self.prev_node_buffers.get(&node_id.0) {
                if let Some(&value) = buffer.last() {
                    return value;
                }
            }
            // No cached value - return 0.0 to break cycle
            return 0.0;
        }

        // Track cache stats if profiling
        static CACHE_HITS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        static CACHE_MISSES: std::sync::atomic::AtomicUsize =
            std::sync::atomic::AtomicUsize::new(0);

        // Check cache first (for non-stateful nodes, cleared per buffer)
        if let Some(&cached) = self.value_cache.get(node_id) {
            if std::env::var("PROFILE_CACHE").is_ok() {
                CACHE_HITS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            return cached;
        }

        // Check stateful cache (for stateful nodes, cleared per sample)
        // This prevents double evaluation of stateful nodes like ASR within a single sample
        if let Some(&cached) = self.stateful_value_cache.get(node_id) {
            return cached;
        }

        // Add this node to call stack before evaluating
        self.eval_call_stack.insert(node_id.0);

        if std::env::var("PROFILE_CACHE").is_ok() {
            CACHE_MISSES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }

        // PERFORMANCE: Use Rc::clone (cheap reference count increment)
        // This eliminates the catastrophic deep clone overhead (~1000ns → <10ns)
        // The Rc is cloned (cheap), then dereferenced to access the SignalNode
        let node_rc = if let Some(Some(node_rc)) = self.nodes.get(node_id.0) {
            std::rc::Rc::clone(node_rc)
        } else {
            return 0.0;
        };

        // Dereference Rc to access the actual SignalNode for pattern matching
        // This is a borrow, not a clone - no performance cost
        let node = &*node_rc;

        // ULTRA-CONSERVATIVE: Only cache Constant nodes
        // Everything else is treated as stateful to be safe
        // This still gives us significant speedup since the value_cache
        // is only cleared once per buffer instead of per sample
        let is_cacheable = matches!(node, SignalNode::Constant { .. });
        let is_stateful = !is_cacheable;

        let value = match node {
            SignalNode::Oscillator {
                freq,
                waveform,
                semitone_offset,
                phase,
                pending_freq,
                last_sample,
            } => {
                if std::env::var("DEBUG_DAG").is_ok() && self.sample_count < 5 && self.current_sample_idx == 0 {
                    eprintln!("      Oscillator evaluating freq: {:?}", freq);
                }
                let requested_freq = self.eval_signal(&freq);
                let mut current_freq = requested_freq;

                // Zero-crossing detection for anti-click frequency changes
                // If there's a pending frequency change, use it until zero-crossing
                if let Some(pending) = *pending_freq.borrow() {
                    current_freq = pending; // Use pending freq until zero-crossing
                }

                // Decode pitch value:
                // >= 1000: Absolute MIDI note (subtract 1000, convert to Hz)
                // < 1000: Relative semitone offset from base frequency
                current_freq = if *semitone_offset >= 1000.0 {
                    // Absolute MIDI: convert to Hz using A4=440Hz as reference
                    let midi = *semitone_offset - 1000.0;
                    440.0 * 2.0_f32.powf((midi - 69.0) / 12.0)
                } else if *semitone_offset != 0.0 {
                    // Relative: apply semitone offset to base frequency
                    current_freq * 2.0_f32.powf(*semitone_offset / 12.0)
                } else {
                    current_freq
                };

                // Generate sample based on waveform
                // Extract phase value to drop borrow immediately
                let phase_val = {
                    let p = phase.borrow();
                    *p
                };
                let sample = match waveform {
                    Waveform::Sine => (2.0 * PI * phase_val).sin(),
                    Waveform::Saw => 2.0 * phase_val - 1.0,
                    Waveform::Square => {
                        if phase_val < 0.5 {
                            1.0
                        } else {
                            -1.0
                        }
                    }
                    Waveform::Triangle => {
                        if phase_val < 0.5 {
                            4.0 * phase_val - 1.0
                        } else {
                            3.0 - 4.0 * phase_val
                        }
                    }
                };

                // Update phase and detect zero-crossings
                if let Some(Some(node)) = self.nodes.get(node_id.0) {
                    if let SignalNode::Oscillator {
                        phase,
                        pending_freq,
                        last_sample,
                        ..
                    } = &**node
                    {
                        // Check if frequency changed
                        if (requested_freq - current_freq).abs() > 0.1 {
                            // Frequency change requested - set it as pending
                            *pending_freq.borrow_mut() = Some(current_freq);
                        }

                        // Check for zero-crossing (sign change from negative to positive)
                        // Must check and clear pending_freq without overlapping borrows
                        let has_pending = pending_freq.borrow().is_some();
                        if has_pending {
                            if *last_sample.borrow() < 0.0 && sample >= 0.0 {
                                // Zero-crossing detected! Apply the frequency change
                                *pending_freq.borrow_mut() = None; // Clear pending
                            }
                        }

                        // Update phase for next sample
                        // CRITICAL FIX: Always use current_freq (which includes pitch shift)
                        // for phase increment. pending_freq is only for anti-click smoothing
                        // during frequency changes, not for pitch shifting.
                        {
                            let mut p = phase.borrow_mut();
                            *p += current_freq / self.sample_rate;
                            if *p >= 1.0 {
                                *p -= 1.0;
                            }
                        }

                        // Store sample for next zero-crossing detection
                        *last_sample.borrow_mut() = sample;
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
                let carrier_p = *carrier_phase.borrow();
                let modulator_p = *modulator_phase.borrow();
                let modulator_value = (2.0 * PI * modulator_p).sin();
                let modulation = index * modulator_value;
                let sample = (2.0 * PI * carrier_p + modulation).sin();

                // Update phases for next sample
                if let Some(Some(node)) = self.nodes.get(node_id.0) {
                    if let SignalNode::FMOscillator {
                        carrier_phase,
                        modulator_phase,
                        ..
                    } = &**node
                    {
                        {
                            let mut cp = carrier_phase.borrow_mut();
                            *cp += carrier_f / self.sample_rate;
                            if *cp >= 1.0 {
                                *cp -= 1.0;
                            }
                        }

                        {
                            let mut mp = modulator_phase.borrow_mut();
                            *mp += modulator_f / self.sample_rate;
                            if *mp >= 1.0 {
                                *mp -= 1.0;
                            }
                        }
                    }
                }

                sample
            }

            SignalNode::PMOscillator {
                carrier_freq,
                modulation,
                mod_index,
                carrier_phase,
            } => {
                // Evaluate modulatable parameters
                let carrier_f = self.eval_signal(&carrier_freq).max(0.0);
                let mod_signal = self.eval_signal(&modulation);
                let index = self.eval_signal(&mod_index);

                // PM synthesis: carrier phase modulated directly by external signal
                // output = sin(2π * carrier_phase + mod_index * modulation_signal)
                let carrier_p = *carrier_phase.borrow();
                let modulation_value = index * mod_signal;
                let sample = (2.0 * PI * carrier_p + modulation_value).sin();

                // Update carrier phase for next sample
                if let Some(Some(node)) = self.nodes.get(node_id.0) {
                    if let SignalNode::PMOscillator { carrier_phase, .. } = &**node {
                        let mut cp = carrier_phase.borrow_mut();
                        *cp += carrier_f / self.sample_rate;
                        if *cp >= 1.0 {
                            *cp -= 1.0;
                        }
                    }
                }

                sample
            }

            SignalNode::Blip { frequency, phase } => {
                // Band-limited impulse train using explicit harmonic summation
                // This is more stable and clearly band-limited than closed-form sinc
                // Formula: blip(phase) = sum(cos(2πkφ) for k=1 to N) + 0.5
                // where N = number of harmonics limited by Nyquist frequency

                let freq = self.eval_signal(&frequency).max(0.1); // Avoid division by zero
                let phase_val = *phase.borrow();

                // Calculate number of harmonics before aliasing
                // Limit to Nyquist frequency to prevent aliasing
                let nyquist = self.sample_rate * 0.5;
                let num_harmonics = (nyquist / freq).floor() as usize;

                // Limit total harmonics for performance (max 1000)
                let num_harmonics = num_harmonics.min(1000);

                // Sum harmonics explicitly
                // Each harmonic is a cosine wave at frequency k*fundamental
                let mut sample = 0.0;
                let two_pi_phase = 2.0 * PI * phase_val;

                for k in 1..=num_harmonics {
                    sample += (k as f32 * two_pi_phase).cos();
                }

                // Normalize to prevent clipping
                // Peak value at phase=0 is num_harmonics (all cosines sum to 1)
                // Divide by num_harmonics to get peak of 1.0
                let output = sample / num_harmonics.max(1) as f32;

                // Update phase for next sample
                let phase_inc = freq / self.sample_rate;
                if let Some(Some(node)) = self.nodes.get(node_id.0) {
                    if let SignalNode::Blip { phase, .. } = &**node {
                        let mut p = phase.borrow_mut();
                        *p += phase_inc;
                        if *p >= 1.0 {
                            *p -= 1.0;
                        }
                    }
                }

                output
            }

            SignalNode::VCO {
                frequency,
                waveform,
                pulse_width,
                phase,
            } => {
                // Analog-style VCO with multiple waveforms and PolyBLEP anti-aliasing
                let freq = self.eval_signal(&frequency).max(0.0);
                let waveform_select = self.eval_signal(&waveform);
                let pw = self.eval_signal(&pulse_width).clamp(0.01, 0.99);

                let phase_val = *phase.borrow();
                let phase_inc = freq / self.sample_rate;

                // PolyBLEP function for band-limiting discontinuities
                fn poly_blep(t: f32, dt: f32) -> f32 {
                    if t < dt {
                        let t = t / dt;
                        2.0 * t - t * t - 1.0
                    } else if t > 1.0 - dt {
                        let t = (t - 1.0) / dt;
                        t * t + 2.0 * t + 1.0
                    } else {
                        0.0
                    }
                }

                // Generate waveform based on selection
                let sample = if waveform_select < 0.5 {
                    // 0: Saw wave (ramp down from 1 to -1)
                    let mut s = 2.0 * phase_val - 1.0;
                    s -= poly_blep(phase_val, phase_inc);
                    s
                } else if waveform_select < 1.5 {
                    // 1: Square wave with PWM
                    let mut s = if phase_val < pw { 1.0 } else { -1.0 };
                    s += poly_blep(phase_val, phase_inc);
                    s -= poly_blep((phase_val + (1.0 - pw)).rem_euclid(1.0), phase_inc);
                    s
                } else if waveform_select < 2.5 {
                    // 2: Triangle wave (integrate square wave)
                    // Triangle is band-limited by nature, no PolyBLEP needed
                    let triangle_val = if phase_val < 0.5 {
                        4.0 * phase_val - 1.0
                    } else {
                        3.0 - 4.0 * phase_val
                    };
                    triangle_val
                } else {
                    // 3: Sine wave (naturally band-limited)
                    (2.0 * PI * phase_val).sin()
                };

                // Update phase for next sample
                if let Some(Some(node)) = self.nodes.get(node_id.0) {
                    if let SignalNode::VCO { phase, .. } = &**node {
                        let mut p = phase.borrow_mut();
                        *p += phase_inc;
                        if *p >= 1.0 {
                            *p -= 1.0;
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

            // z^-1 unit delay for feedback loops
            // Returns the previous sample's value of the named bus
            SignalNode::UnitDelay { bus_name } => {
                // In DAG mode, look at the current buffer at (current_sample_idx - 1)
                // For sample 0, use the last sample from the previous block
                if self.in_dag_processing {
                    if let Some(&bus_node_id) = self.buses.get(bus_name) {
                        // Determine which buffer to use for feedback:
                        // - If we're currently processing the bus node, use THAT buffer
                        // - Otherwise, use the bus node's buffer if it exists
                        if std::env::var("DEBUG_UNIT_DELAY").is_ok() && self.sample_count < 20 && self.current_sample_idx < 5 {
                            eprintln!(
                                "UnitDelay[{}] sample={}: current_dag_node_id={:?}, bus_node_id={}, node_id={}",
                                bus_name, self.current_sample_idx, self.current_dag_node_id, bus_node_id.0, node_id.0
                            );
                        }
                        let feedback_node_id = if self.current_dag_node_id == Some(bus_node_id.0) {
                            // We're processing the bus that contains this UnitDelay
                            // Use the current node's buffer for proper z^-1 feedback
                            bus_node_id.0
                        } else if self.dag_buffer_cache.contains_key(&bus_node_id.0) {
                            // Bus has already been processed
                            bus_node_id.0
                        } else {
                            // Bus hasn't been processed yet, use current node's buffer
                            self.current_dag_node_id.unwrap_or(node_id.0)
                        };

                        if self.current_sample_idx > 0 {
                            // Look at previous sample in current buffer
                            if let Some(buffer) = self.dag_buffer_cache.get(&feedback_node_id) {
                                let val = buffer
                                    .get(self.current_sample_idx - 1)
                                    .copied()
                                    .unwrap_or(0.0);
                                if std::env::var("DEBUG_UNIT_DELAY").is_ok() && self.sample_count < 20 && self.current_sample_idx < 5 {
                                    eprintln!("  -> returning cache[{}][{}] = {}", feedback_node_id, self.current_sample_idx - 1, val);
                                }
                                return val;
                            }
                        } else {
                            // First sample: use last sample from previous block
                            if let Some(prev_buffer) = self.prev_node_buffers.get(&feedback_node_id)
                            {
                                let val = prev_buffer.last().copied().unwrap_or(0.0);
                                if std::env::var("DEBUG_UNIT_DELAY").is_ok() && self.sample_count < 20 {
                                    eprintln!("  -> returning prev_block = {}", val);
                                }
                                return val;
                            }
                        }
                    }
                    return 0.0;
                }

                // Legacy path: look up the previous sample's value for this bus
                // Returns 0.0 on first sample (no history yet)
                self.bus_previous_values
                    .get(bus_name)
                    .copied()
                    .unwrap_or(0.0)
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
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
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
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::BrownNoise { state: s } = node {
                        s.accumulator = new_accumulator;
                    }
                }

                // Normalize output to approximately -1 to 1
                new_accumulator * 0.7
            }

            SignalNode::MidiInput {
                channel,
                active_notes,
                event_queue,
                last_freq,
                gate,
            } => {
                use crate::midi_input::MidiMessageType;

                // Process all pending MIDI events from the queue
                if let Ok(mut queue) = event_queue.lock() {
                    while let Some(event) = queue.pop_front() {
                        // Filter by channel if specified
                        if let Some(ch) = channel {
                            if event.channel != *ch {
                                continue; // Skip events from other channels
                            }
                        }

                        // Update active notes based on event type
                        match event.message_type {
                            MidiMessageType::NoteOn { note, velocity } if velocity > 0 => {
                                // Note on: add to active notes with normalized velocity
                                active_notes
                                    .borrow_mut()
                                    .insert(note, velocity as f32 / 127.0);
                                *gate.borrow_mut() = 1.0; // Gate on
                            }
                            MidiMessageType::NoteOff { note, .. }
                            | MidiMessageType::NoteOn { note, velocity: 0 } => {
                                // Note off: remove from active notes
                                active_notes.borrow_mut().remove(&note);
                                // Gate off only if no notes are active
                                if active_notes.borrow().is_empty() {
                                    *gate.borrow_mut() = 0.0;
                                }
                            }
                            _ => {} // Ignore other MIDI messages for now
                        }
                    }
                }

                // Get the highest active note (monophonic for now)
                let freq = if let Some(&note) = active_notes.borrow().keys().max() {
                    let f = midi_note_to_freq(note);
                    *last_freq.borrow_mut() = f; // Store for when no notes active
                    f
                } else {
                    // No notes active, return last frequency (for release phase)
                    *last_freq.borrow()
                };

                freq
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
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
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
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
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
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
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

                // DEBUG: Log ASR parameters on first sample
                if std::env::var("DEBUG_ASR").is_ok() && self.sample_count < 10 {
                    eprintln!(
                        "ASR DEBUG sample {}: gate={:.4}, attack_time={:.4}, release_time={:.4}, sample_rate={}",
                        self.sample_count, gate_val, attack_time, release_time, self.sample_rate
                    );
                }

                let current_phase = state.phase.borrow().clone();
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
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::ASR { state: s, .. } = node {
                        *s.phase.borrow_mut() = next_phase;
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
                let sample = if *phase < w { 1.0 } else { -1.0 };

                // Update phase for next sample
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Pulse { phase: p, .. } = node {
                        *p += f / self.sample_rate;
                        if *p >= 1.0 {
                            *p -= 1.0;
                        }
                    }
                }

                sample
            }

            SignalNode::Wavetable { freq, state } => {
                // Evaluate frequency (pattern-modulatable)
                let f = self.eval_signal(&freq).max(0.0);

                // Get interpolated sample at current phase
                let sample = state.get_sample(state.phase);

                // Update phase for next sample
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Wavetable { state: s, .. } = node {
                        s.phase += f / self.sample_rate;
                        if s.phase >= 1.0 {
                            s.phase -= 1.0;
                        }
                    }
                }

                sample
            }

            SignalNode::Granular {
                source,
                grain_size_ms,
                density,
                pitch,
                state,
            } => {
                // Evaluate pattern-modulatable parameters
                let source_sample = self.eval_signal(&source);

                // BYPASS MODE: For pipelined rendering, pass through unchanged
                if self.bypass_sequential_effects {
                    return source_sample;
                }

                let grain_ms = self.eval_signal(&grain_size_ms).max(5.0).min(500.0);
                let density_val = self.eval_signal(&density).clamp(0.0, 1.0);
                let pitch_val = self.eval_signal(&pitch).max(0.1).min(4.0);

                // Convert grain size from milliseconds to samples
                let grain_size_samples = (grain_ms * self.sample_rate / 1000.0) as usize;

                // Write source sample to buffer
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Granular { state: s, .. } = node {
                        s.write_sample(source_sample);

                        // Spawn new grain based on density
                        // density controls spawn rate: 0.0 = never, 1.0 = every sample
                        s.grain_spawn_phase += density_val;
                        if s.grain_spawn_phase >= 1.0 {
                            s.grain_spawn_phase -= 1.0;
                            s.spawn_grain(grain_size_samples, pitch_val);
                        }

                        // Get mixed output from all active grains
                        let output = s.get_sample();

                        // Advance all grains
                        s.advance();

                        return output;
                    }
                }

                0.0
            }

            SignalNode::KarplusStrong {
                freq,
                damping,
                trigger,
                state,
                last_freq,
                last_trigger,
            } => {
                // BYPASS MODE: For pipelined rendering, generators produce silence
                if self.bypass_sequential_effects {
                    return 0.0;
                }

                // Evaluate pattern-modulatable parameters
                let f = self.eval_signal(&freq).max(20.0).min(10000.0);
                let damp = self.eval_signal(&damping).clamp(0.0, 1.0);
                let trig = self.eval_signal(&trigger);

                // Calculate required delay line size for this frequency
                let required_size = (self.sample_rate / f) as usize;

                // Check if frequency changed significantly (need to resize delay line)
                let freq_changed = (f - *last_freq).abs() > 1.0;

                // Detect rising edge trigger (0 -> 1)
                let trigger_edge = trig > 0.5 && *last_trigger <= 0.5;

                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::KarplusStrong {
                        state: s,
                        last_freq: lf,
                        last_trigger: lt,
                        ..
                    } = node
                    {
                        // Resize delay line if frequency changed
                        if freq_changed {
                            s.resize(required_size);
                            *lf = f;
                        }

                        // Re-initialize with noise on trigger
                        if trigger_edge {
                            s.initialize_with_noise();
                        }

                        // Update last_trigger
                        *lt = trig;

                        // Get sample from Karplus-Strong algorithm
                        return s.get_sample(damp);
                    }
                }

                0.0
            }

            SignalNode::Waveguide {
                freq,
                damping,
                pickup_position,
                state,
                last_freq,
            } => {
                // BYPASS MODE: For pipelined rendering, generators produce silence
                if self.bypass_sequential_effects {
                    return 0.0;
                }

                // Evaluate pattern-modulatable parameters
                let f = self.eval_signal(&freq).max(20.0).min(10000.0);
                let damp = self.eval_signal(&damping).clamp(0.0, 1.0);
                let pickup = self.eval_signal(&pickup_position).clamp(0.0, 1.0);

                // Calculate required delay line size for this frequency
                let required_size = (self.sample_rate / f) as usize;

                // Check if frequency changed significantly (need to resize delay lines)
                let freq_changed = (f - *last_freq).abs() > 1.0;

                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Waveguide {
                        state: s,
                        last_freq: lf,
                        ..
                    } = node
                    {
                        // Resize delay lines if frequency changed
                        if freq_changed {
                            s.resize(required_size);
                            *lf = f;
                        }

                        // Get sample from waveguide algorithm
                        return s.get_sample(pickup, damp);
                    }
                }

                0.0
            }

            SignalNode::Formant {
                source,
                f1,
                f2,
                f3,
                bw1,
                bw2,
                bw3,
                state,
            } => {
                // Evaluate input source signal
                let input = self.eval_signal(&source);

                // Evaluate formant parameters (all pattern-modulatable)
                let f1_val = self.eval_signal(&f1).max(50.0).min(5000.0);
                let f2_val = self.eval_signal(&f2).max(50.0).min(5000.0);
                let f3_val = self.eval_signal(&f3).max(50.0).min(10000.0);
                let bw1_val = self.eval_signal(&bw1).max(10.0).min(1000.0);
                let bw2_val = self.eval_signal(&bw2).max(10.0).min(1000.0);
                let bw3_val = self.eval_signal(&bw3).max(10.0).min(1000.0);

                // Get mutable state and process
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Formant { state: s, .. } = node {
                        return s.process(input, f1_val, f2_val, f3_val, bw1_val, bw2_val, bw3_val);
                    }
                }

                0.0
            }

            SignalNode::Vowel {
                source,
                vowel,
                state,
            } => {
                // Evaluate input source signal
                let input = self.eval_signal(&source);

                // Evaluate vowel selector (0-4 maps to a,e,i,o,u)
                let vowel_val = self.eval_signal(&vowel).round().clamp(0.0, 4.0) as i32;

                // Map vowel to formant frequencies (male voice)
                // a, e, i, o, u
                let (f1, f2, f3) = match vowel_val {
                    0 => (730.0, 1090.0, 2440.0), // 'a' (father)
                    1 => (530.0, 1840.0, 2480.0), // 'e' (bet)
                    2 => (270.0, 2290.0, 3010.0), // 'i' (beet)
                    3 => (570.0, 840.0, 2410.0),  // 'o' (boat)
                    4 => (300.0, 870.0, 2240.0),  // 'u' (boot)
                    _ => (730.0, 1090.0, 2440.0), // Default to 'a'
                };

                // Standard bandwidths for formants
                let bw1 = 60.0;
                let bw2 = 80.0;
                let bw3 = 100.0;

                // Get mutable state and process
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Vowel { state: s, .. } = node {
                        return s.process(input, f1, f2, f3, bw1, bw2, bw3);
                    }
                }

                0.0
            }

            SignalNode::Additive {
                freq,
                amplitudes,
                state,
            } => {
                // Evaluate fundamental frequency (pattern-modulatable)
                let f = self.eval_signal(&freq).max(20.0).min(10000.0);

                // Get mutable state and process with fixed amplitudes
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Additive {
                        state: s,
                        amplitudes: amps,
                        ..
                    } = node
                    {
                        return s.process(f, amps);
                    }
                }

                0.0
            }

            SignalNode::Vocoder {
                modulator,
                carrier,
                num_bands,
                state,
            } => {
                // Evaluate modulator and carrier signals
                let mod_sample = self.eval_signal(&modulator);
                let carr_sample = self.eval_signal(&carrier);

                // BYPASS MODE: For pipelined rendering, pass through carrier unchanged
                if self.bypass_sequential_effects {
                    return carr_sample;
                }

                // Get mutable state and process
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Vocoder { state: s, .. } = node {
                        return s.process(mod_sample, carr_sample);
                    }
                }

                0.0
            }

            SignalNode::PitchShift {
                input,
                semitones,
                state,
            } => {
                // Evaluate input and semitones
                let input_sample = self.eval_signal(&input);

                // BYPASS MODE: For pipelined rendering, pass through unchanged
                if self.bypass_sequential_effects {
                    return input_sample;
                }

                let semitones_val = self.eval_signal(&semitones);

                // Get mutable state and process
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::PitchShift { state: s, .. } = node {
                        return s.process(input_sample, semitones_val);
                    }
                }

                0.0
            }

            SignalNode::Limiter {
                input, threshold, ..
            } => {
                // Evaluate input signal and threshold
                let input_val = self.eval_signal(&input);
                let thresh = self.eval_signal(&threshold).max(0.0);

                // Brick-wall limiting: clamp to [-threshold, +threshold]
                input_val.clamp(-thresh, thresh)
            }

            SignalNode::SVF {
                input,
                frequency,
                resonance,
                mode,
                state,
            } => {
                // Chamberlin State Variable Filter
                // Produces LP, HP, BP, and Notch outputs simultaneously

                let input_val = self.eval_signal(&input);
                let freq = self
                    .eval_signal(&frequency)
                    .clamp(10.0, self.sample_rate * 0.45);
                let res = self.eval_signal(&resonance).max(0.1); // Prevent division by zero

                // Calculate filter coefficients
                // f = 2 * sin(π * cutoff / sampleRate)
                // Prevent instability at high frequencies
                let f = (std::f32::consts::PI * freq / self.sample_rate)
                    .sin()
                    .min(0.95);
                let q = 1.0 / res.max(0.1); // Convert resonance to damping

                // Get current state
                let mut low = state.low;
                let mut band = state.band;

                // Update filter
                low = low + f * band;
                let high = input_val - low - q * band;
                band = f * high + band;
                let notch = high + low;

                // Clamp state to prevent runaway values and NaN
                low = low.clamp(-10.0, 10.0);
                band = band.clamp(-10.0, 10.0);

                // Check for NaN and reset if needed
                if !low.is_finite() || !band.is_finite() {
                    low = 0.0;
                    band = 0.0;
                }

                // Update state
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::SVF { state: s, .. } = node {
                        s.low = low;
                        s.band = band;
                    }
                }

                // Select output based on mode
                match mode {
                    0 => low,   // Lowpass
                    1 => high,  // Highpass
                    2 => band,  // Bandpass
                    3 => notch, // Notch
                    _ => low,   // Default to lowpass
                }
            }

            SignalNode::Biquad {
                input,
                frequency,
                q,
                mode,
                state,
            } => {
                // Biquad Filter (RBJ Audio EQ Cookbook)
                // High-quality second-order IIR filter with multiple modes

                let input_val = self.eval_signal(&input);
                let freq = self
                    .eval_signal(&frequency)
                    .clamp(10.0, self.sample_rate * 0.45);
                let q_val = self.eval_signal(&q).clamp(0.1, 20.0); // Prevent instability

                // Calculate normalized frequency
                let omega = 2.0 * std::f32::consts::PI * freq / self.sample_rate;
                let sin_omega = omega.sin();
                let cos_omega = omega.cos();
                let alpha = sin_omega / (2.0 * q_val);

                // Calculate coefficients based on mode (RBJ formulas)
                let (b0, b1, b2, a0, a1, a2) = match mode {
                    0 => {
                        // Lowpass
                        let b1_temp = 1.0 - cos_omega;
                        let b0_temp = b1_temp / 2.0;
                        let b2_temp = b0_temp;
                        let a0_temp = 1.0 + alpha;
                        let a1_temp = -2.0 * cos_omega;
                        let a2_temp = 1.0 - alpha;
                        (b0_temp, b1_temp, b2_temp, a0_temp, a1_temp, a2_temp)
                    }
                    1 => {
                        // Highpass
                        let b0_temp = (1.0 + cos_omega) / 2.0;
                        let b1_temp = -(1.0 + cos_omega);
                        let b2_temp = b0_temp;
                        let a0_temp = 1.0 + alpha;
                        let a1_temp = -2.0 * cos_omega;
                        let a2_temp = 1.0 - alpha;
                        (b0_temp, b1_temp, b2_temp, a0_temp, a1_temp, a2_temp)
                    }
                    2 => {
                        // Bandpass (constant skirt gain)
                        let b0_temp = alpha;
                        let b1_temp = 0.0;
                        let b2_temp = -alpha;
                        let a0_temp = 1.0 + alpha;
                        let a1_temp = -2.0 * cos_omega;
                        let a2_temp = 1.0 - alpha;
                        (b0_temp, b1_temp, b2_temp, a0_temp, a1_temp, a2_temp)
                    }
                    3 => {
                        // Notch
                        let b0_temp = 1.0;
                        let b1_temp = -2.0 * cos_omega;
                        let b2_temp = 1.0;
                        let a0_temp = 1.0 + alpha;
                        let a1_temp = -2.0 * cos_omega;
                        let a2_temp = 1.0 - alpha;
                        (b0_temp, b1_temp, b2_temp, a0_temp, a1_temp, a2_temp)
                    }
                    _ => {
                        // Default to lowpass
                        let b1_temp = 1.0 - cos_omega;
                        let b0_temp = b1_temp / 2.0;
                        let b2_temp = b0_temp;
                        let a0_temp = 1.0 + alpha;
                        let a1_temp = -2.0 * cos_omega;
                        let a2_temp = 1.0 - alpha;
                        (b0_temp, b1_temp, b2_temp, a0_temp, a1_temp, a2_temp)
                    }
                };

                // Normalize coefficients by a0
                let b0_norm = b0 / a0;
                let b1_norm = b1 / a0;
                let b2_norm = b2 / a0;
                let a1_norm = a1 / a0;
                let a2_norm = a2 / a0;

                // Get current state
                let x1 = state.x1;
                let x2 = state.x2;
                let y1 = state.y1;
                let y2 = state.y2;

                // Apply biquad difference equation (Direct Form II)
                // y[n] = b0*x[n] + b1*x[n-1] + b2*x[n-2] - a1*y[n-1] - a2*y[n-2]
                let output =
                    b0_norm * input_val + b1_norm * x1 + b2_norm * x2 - a1_norm * y1 - a2_norm * y2;

                // Clamp output to prevent runaway values
                let output_clamped = output.clamp(-10.0, 10.0);

                // Check for NaN and reset if needed
                let final_output = if output_clamped.is_finite() {
                    output_clamped
                } else {
                    0.0
                };

                // Update state
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Biquad { state: s, .. } = node {
                        s.x2 = x1;
                        s.x1 = input_val;
                        s.y2 = y1;
                        s.y1 = final_output;
                        s.b0 = b0_norm;
                        s.b1 = b1_norm;
                        s.b2 = b2_norm;
                        s.a1 = a1_norm;
                        s.a2 = a2_norm;
                    }
                }

                final_output
            }

            SignalNode::Resonz {
                input,
                frequency,
                q,
                state,
            } => {
                // Resonz - Resonant Bandpass Filter
                // Implemented as biquad bandpass with emphasis on resonance
                // Similar to Biquad BP but optimized for high Q values

                let input_val = self.eval_signal(&input);
                let freq = self
                    .eval_signal(&frequency)
                    .clamp(10.0, self.sample_rate * 0.45);
                let q_val = self.eval_signal(&q).clamp(0.5, 100.0); // Allow higher Q for more resonance

                // Calculate normalized frequency
                let omega = 2.0 * std::f32::consts::PI * freq / self.sample_rate;
                let sin_omega = omega.sin();
                let cos_omega = omega.cos();
                let alpha = sin_omega / (2.0 * q_val);

                // Bandpass filter coefficients (constant 0 dB peak gain)
                let b0 = alpha;
                let b1 = 0.0;
                let b2 = -alpha;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_omega;
                let a2 = 1.0 - alpha;

                // Normalize coefficients by a0
                let b0_norm = b0 / a0;
                let b1_norm = b1 / a0;
                let b2_norm = b2 / a0;
                let a1_norm = a1 / a0;
                let a2_norm = a2 / a0;

                // Get current state
                let x1 = state.x1;
                let x2 = state.x2;
                let y1 = state.y1;
                let y2 = state.y2;

                // Apply biquad difference equation
                let output =
                    b0_norm * input_val + b1_norm * x1 + b2_norm * x2 - a1_norm * y1 - a2_norm * y2;

                // Clamp output to prevent runaway values
                let output_clamped = output.clamp(-10.0, 10.0);

                // Check for NaN and reset if needed
                let final_output = if output_clamped.is_finite() {
                    output_clamped
                } else {
                    0.0
                };

                // Update state
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Resonz { state: s, .. } = node {
                        s.x2 = x1;
                        s.x1 = input_val;
                        s.y2 = y1;
                        s.y1 = final_output;
                        s.b0 = b0_norm;
                        s.b1 = b1_norm;
                        s.b2 = b2_norm;
                        s.a1 = a1_norm;
                        s.a2 = a2_norm;
                    }
                }

                final_output
            }

            SignalNode::RLPF {
                input,
                cutoff,
                resonance,
                state,
            } => {
                // RLPF - Resonant Lowpass Filter
                // Classic analog synthesizer lowpass with resonance
                // Implemented as biquad lowpass with Q parameter

                let input_val = self.eval_signal(&input);
                let freq = self
                    .eval_signal(&cutoff)
                    .clamp(10.0, self.sample_rate * 0.45);
                let q_val = self.eval_signal(&resonance).clamp(0.1, 20.0);

                // Calculate normalized frequency
                let omega = 2.0 * std::f32::consts::PI * freq / self.sample_rate;
                let sin_omega = omega.sin();
                let cos_omega = omega.cos();
                let alpha = sin_omega / (2.0 * q_val);

                // Lowpass filter coefficients (RBJ)
                let b1_temp = 1.0 - cos_omega;
                let b0 = b1_temp / 2.0;
                let b1 = b1_temp;
                let b2 = b0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_omega;
                let a2 = 1.0 - alpha;

                // Normalize coefficients by a0
                let b0_norm = b0 / a0;
                let b1_norm = b1 / a0;
                let b2_norm = b2 / a0;
                let a1_norm = a1 / a0;
                let a2_norm = a2 / a0;

                // Get current state
                let x1 = state.x1;
                let x2 = state.x2;
                let y1 = state.y1;
                let y2 = state.y2;

                // Apply biquad difference equation
                let output =
                    b0_norm * input_val + b1_norm * x1 + b2_norm * x2 - a1_norm * y1 - a2_norm * y2;

                // Clamp output to prevent runaway values
                let output_clamped = output.clamp(-10.0, 10.0);

                // Check for NaN and reset if needed
                let final_output = if output_clamped.is_finite() {
                    output_clamped
                } else {
                    0.0
                };

                // Update state
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::RLPF { state: s, .. } = node {
                        s.x2 = x1;
                        s.x1 = input_val;
                        s.y2 = y1;
                        s.y1 = final_output;
                        s.b0 = b0_norm;
                        s.b1 = b1_norm;
                        s.b2 = b2_norm;
                        s.a1 = a1_norm;
                        s.a2 = a2_norm;
                    }
                }

                final_output
            }

            SignalNode::RHPF {
                input,
                cutoff,
                resonance,
                state,
            } => {
                // RHPF - Resonant Highpass Filter
                // Highpass filter with resonance peak at cutoff
                // Implemented as biquad highpass with Q parameter

                let input_val = self.eval_signal(&input);
                let freq = self
                    .eval_signal(&cutoff)
                    .clamp(10.0, self.sample_rate * 0.45);
                let q_val = self.eval_signal(&resonance).clamp(0.1, 20.0);

                // Calculate normalized frequency
                let omega = 2.0 * std::f32::consts::PI * freq / self.sample_rate;
                let sin_omega = omega.sin();
                let cos_omega = omega.cos();
                let alpha = sin_omega / (2.0 * q_val);

                // Highpass filter coefficients (RBJ)
                let b0 = (1.0 + cos_omega) / 2.0;
                let b1 = -(1.0 + cos_omega);
                let b2 = b0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_omega;
                let a2 = 1.0 - alpha;

                // Normalize coefficients by a0
                let b0_norm = b0 / a0;
                let b1_norm = b1 / a0;
                let b2_norm = b2 / a0;
                let a1_norm = a1 / a0;
                let a2_norm = a2 / a0;

                // Get current state
                let x1 = state.x1;
                let x2 = state.x2;
                let y1 = state.y1;
                let y2 = state.y2;

                // Apply biquad difference equation
                let output =
                    b0_norm * input_val + b1_norm * x1 + b2_norm * x2 - a1_norm * y1 - a2_norm * y2;

                // Clamp output to prevent runaway values
                let output_clamped = output.clamp(-10.0, 10.0);

                // Check for NaN and reset if needed
                let final_output = if output_clamped.is_finite() {
                    output_clamped
                } else {
                    0.0
                };

                // Update state
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::RHPF { state: s, .. } = node {
                        s.x2 = x1;
                        s.x1 = input_val;
                        s.y2 = y1;
                        s.y1 = final_output;
                        s.b0 = b0_norm;
                        s.b1 = b1_norm;
                        s.b2 = b2_norm;
                        s.a1 = a1_norm;
                        s.a2 = a2_norm;
                    }
                }

                final_output
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

            SignalNode::Constant { value } => *value,

            SignalNode::PatternEvaluator { pattern } => {
                // Evaluate the pattern at the current cycle position
                use crate::pattern::{Fraction, State, TimeSpan};
                use std::collections::HashMap;

                let cycle_pos = self.get_cycle_position();
                let state = State {
                    span: TimeSpan::new(
                        Fraction::from_float(cycle_pos),
                        Fraction::from_float(cycle_pos + 0.001), // Small query window
                    ),
                    controls: HashMap::new(),
                };

                // Query the pattern and return the first event's value (or 0.0)
                pattern
                    .query(&state)
                    .first()
                    .map(|hap| hap.value as f32)
                    .unwrap_or(0.0)
            }

            SignalNode::EveryEffect { input, effect, n } => {
                // Apply effect every N cycles, bypass otherwise
                let current_cycle = self.get_cycle_position().floor() as i32;
                if current_cycle % n == 0 {
                    self.eval_signal_at_time(&effect, self.get_cycle_position())
                } else {
                    self.eval_signal_at_time(&input, self.get_cycle_position())
                }
            }

            SignalNode::SometimesEffect {
                input,
                effect,
                prob,
            } => {
                // Apply effect with probability, based on cycle seed
                use rand::{rngs::StdRng, Rng, SeedableRng};
                let current_cycle = self.get_cycle_position().floor() as u64;
                let mut rng = StdRng::seed_from_u64(current_cycle);
                let prob_val = self.eval_signal(&prob) as f64; // Pattern-modulatable probability

                if rng.gen::<f64>() < prob_val {
                    self.eval_signal_at_time(&effect, self.get_cycle_position())
                } else {
                    self.eval_signal_at_time(&input, self.get_cycle_position())
                }
            }

            SignalNode::WhenmodEffect {
                input,
                effect,
                modulo,
                offset,
            } => {
                // Apply effect when (cycle - offset) % modulo == 0
                let current_cycle = self.get_cycle_position().floor() as i32;
                if (current_cycle - offset) % modulo == 0 {
                    self.eval_signal_at_time(&effect, self.get_cycle_position())
                } else {
                    self.eval_signal_at_time(&input, self.get_cycle_position())
                }
            }

            SignalNode::Add { a, b } => self.eval_signal(&a) + self.eval_signal(&b),

            SignalNode::Multiply { a, b } => self.eval_signal(&a) * self.eval_signal(&b),

            SignalNode::Min { a, b } => self.eval_signal(&a).min(self.eval_signal(&b)),

            SignalNode::Wrap { input, min, max } => {
                let input_val = self.eval_signal(&input);
                let min_val = self.eval_signal(&min);
                let max_val = self.eval_signal(&max);

                let range = max_val - min_val;
                if range.abs() < 1e-10 {
                    min_val
                } else {
                    let normalized = (input_val - min_val) % range;
                    if normalized < 0.0 {
                        normalized + range + min_val
                    } else {
                        normalized + min_val
                    }
                }
            }

            SignalNode::SampleAndHold {
                input,
                trigger,
                held_value,
                last_trigger,
            } => {
                let input_val = self.eval_signal(&input);
                let trigger_val = self.eval_signal(&trigger);

                // Check for zero crossing (negative or zero to positive)
                let last = *last_trigger.borrow();
                if last <= 0.0 && trigger_val > 0.0 {
                    // Zero crossing detected - sample the input
                    *held_value.borrow_mut() = input_val;
                }

                // Update last_trigger for next sample
                *last_trigger.borrow_mut() = trigger_val;

                // Return held value
                *held_value.borrow()
            }

            SignalNode::Decimator {
                input,
                factor,
                smooth,
                sample_counter,
                held_value,
                smooth_state,
            } => {
                let input_val = self.eval_signal(&input);
                let factor_val = self.eval_signal(&factor).max(1.0); // Clamp to minimum 1.0
                let smooth_val = self.eval_signal(&smooth).clamp(0.0, 0.99); // Clamp to [0, 0.99] - must be <1.0 to allow new values through

                // Increment sample counter
                let mut counter = sample_counter.borrow_mut();
                *counter += 1.0;

                // Check if we should sample a new value
                if *counter >= factor_val {
                    *held_value.borrow_mut() = input_val;
                    *counter = 0.0;
                }

                // Apply optional smoothing with one-pole filter
                let output = if smooth_val > 0.0 {
                    let held = *held_value.borrow();
                    let last_smooth = *smooth_state.borrow();
                    // One-pole lowpass: y[n] = x[n] * (1-a) + y[n-1] * a
                    let smoothed = held * (1.0 - smooth_val) + last_smooth * smooth_val;
                    *smooth_state.borrow_mut() = smoothed;
                    smoothed
                } else {
                    *held_value.borrow()
                };

                output
            }

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
                // Mix all input signals with normalization
                // Sum and divide by N to prevent volume multiplication
                let sum: f32 = signals.iter().map(|s| self.eval_signal(s)).sum();
                let n = signals.len() as f32;
                if n > 0.0 {
                    sum / n
                } else {
                    0.0
                }
            }

            SignalNode::Allpass {
                input, coefficient, ..
            } => {
                let x = self.eval_signal(&input);
                let g = self.eval_signal(&coefficient).clamp(-1.0, 1.0);

                // Get previous state
                let (x1, y1) = if let Some(Some(node_rc)) = self.nodes.get(node_id.0) {
                    if let SignalNode::Allpass { state, .. } = &**node_rc {
                        (state.x1, state.y1)
                    } else {
                        (0.0, 0.0)
                    }
                } else {
                    (0.0, 0.0)
                };

                // First-order allpass filter
                // y[n] = g * (x[n] - y[n-1]) + x[n-1]
                let y = g * (x - y1) + x1;

                // Update state
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Allpass { state, .. } = node {
                        state.x1 = x;
                        state.y1 = y;
                    }
                }

                y
            }

            SignalNode::LowPass {
                input, cutoff, q, ..
            } => {
                let input_val = self.eval_signal(&input);
                let fc = self.eval_signal(&cutoff).max(20.0).min(20000.0);
                let q_val = self.eval_signal(&q).max(0.5).min(20.0);

                // Get state and cached coefficients
                let (mut low, mut band, mut high, mut f, mut damp) =
                    if let Some(Some(node_rc)) = self.nodes.get(node_id.0) {
                        if let SignalNode::LowPass { state, .. } = &**node_rc {
                            (
                                state.y1,
                                state.x1,
                                state.y2,
                                state.cached_f,
                                state.cached_damp,
                            )
                        } else {
                            (0.0, 0.0, 0.0, 0.0, 1.0)
                        }
                    } else {
                        (0.0, 0.0, 0.0, 0.0, 1.0)
                    };

                // Only recompute coefficients if parameters changed (OPTIMIZATION!)
                let params_changed = if let Some(Some(node_rc)) = self.nodes.get(node_id.0) {
                    if let SignalNode::LowPass { state, .. } = &**node_rc {
                        (fc - state.cached_fc).abs() > 0.1 || (q_val - state.cached_q).abs() > 0.001
                    } else {
                        true
                    }
                } else {
                    true
                };

                if params_changed {
                    // State variable filter (Chamberlin)
                    // Recompute coefficients only when needed
                    f = 2.0 * (PI * fc / self.sample_rate).sin();
                    damp = 1.0 / q_val;
                }

                // Process filter
                high = input_val - low - damp * band;
                band += f * high;
                low += f * band;

                // Update state and cache coefficients
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::LowPass { state, .. } = node {
                        state.y1 = low;
                        state.x1 = band;
                        state.y2 = high;
                        if params_changed {
                            state.cached_fc = fc;
                            state.cached_q = q_val;
                            state.cached_f = f;
                            state.cached_damp = damp;
                        }
                    }
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

                // BYPASS MODE: For pipelined rendering, pass through unchanged
                if self.bypass_sequential_effects {
                    return input_val;
                }

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
                    if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                        let node = Rc::make_mut(node_rc);
                        if let SignalNode::Reverb { state: s, .. } = node {
                            s.comb_buffers[i][read_idx] = to_write;
                            s.comb_indices[i] = (read_idx + 1) % buf_len;
                            s.comb_filter_stores[i] = filtered;
                        }
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

                    if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                        let node = Rc::make_mut(node_rc);
                        if let SignalNode::Reverb { state: s, .. } = node {
                            s.allpass_buffers[i][read_idx] = to_write;
                            s.allpass_indices[i] = (read_idx + 1) % buf_len;
                        }
                    }
                }

                // Mix dry and wet
                input_val * (1.0 - mix_val) + allpass_out * mix_val
            }

            SignalNode::DattorroReverb {
                input,
                pre_delay,
                decay,
                diffusion,
                damping,
                mod_depth,
                mix,
                state,
            } => {
                // Full Dattorro reverb implementation
                // Based on Jon Dattorro's 1997 AES paper
                let input_val = self.eval_signal(&input);

                // BYPASS MODE: For pipelined rendering, pass through unchanged
                if self.bypass_sequential_effects {
                    return input_val;
                }

                let pre_delay_ms = self.eval_signal(&pre_delay).clamp(0.0, 500.0);
                let decay_val = self.eval_signal(&decay).clamp(0.1, 10.0);
                let diffusion_val = self.eval_signal(&diffusion).clamp(0.0, 1.0);
                let damping_val = self.eval_signal(&damping).clamp(0.0, 1.0);
                let mod_depth_val = self.eval_signal(&mod_depth).clamp(0.0, 1.0);
                let mix_val = self.eval_signal(&mix).clamp(0.0, 1.0);

                // Helper function for allpass filter
                // y[n] = -x[n] + x[n-D] + g * (x[n] - y[n-D])
                let allpass =
                    |buffer: &mut Vec<f32>, idx: &mut usize, input: f32, gain: f32| -> f32 {
                        let buffer_len = buffer.len();
                        let delayed = buffer[*idx];
                        let output = -input + delayed + gain * (input - delayed);
                        buffer[*idx] = input + gain * delayed;
                        *idx = (*idx + 1) % buffer_len;
                        output
                    };

                // Helper function for simple delay
                let delay = |buffer: &mut Vec<f32>, idx: &mut usize, input: f32| -> f32 {
                    let buffer_len = buffer.len();
                    let output = buffer[*idx];
                    buffer[*idx] = input;
                    *idx = (*idx + 1) % buffer_len;
                    output
                };

                // Get mutable state
                let (left_out, right_out) = if let Some(Some(node_rc)) =
                    self.nodes.get_mut(node_id.0)
                {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::DattorroReverb { state: s, .. } = node {
                        // 1. PRE-DELAY
                        let pre_delay_samples = ((pre_delay_ms / 1000.0) * s.sample_rate) as usize;
                        let pre_delay_samples = pre_delay_samples.min(s.predelay_buffer.len() - 1);

                        let predelay_out = if pre_delay_samples > 0 {
                            let read_idx = (s.predelay_idx + s.predelay_buffer.len()
                                - pre_delay_samples)
                                % s.predelay_buffer.len();
                            let output = s.predelay_buffer[read_idx];
                            s.predelay_buffer[s.predelay_idx] = input_val;
                            s.predelay_idx = (s.predelay_idx + 1) % s.predelay_buffer.len();
                            output
                        } else {
                            input_val
                        };

                        // 2. INPUT DIFFUSION (4 series allpass filters)
                        let input_diffusion_gain = 0.75 * diffusion_val;
                        let mut diffused = predelay_out;

                        for i in 0..4 {
                            diffused = allpass(
                                &mut s.input_diffusion_buffers[i],
                                &mut s.input_diffusion_indices[i],
                                diffused,
                                input_diffusion_gain,
                            );
                        }

                        // Split into left and right for the figure-8 network
                        let input_to_tanks = diffused;

                        // 3. FIGURE-8 DECAY NETWORK
                        // Coefficients from Dattorro paper
                        let decay_diffusion1 = 0.7 * diffusion_val;
                        let decay_diffusion2 = 0.5 * diffusion_val;
                        let decay_gain = 0.4 + (decay_val - 0.1) / 9.9 * 0.55; // Map 0.1-10.0 to 0.4-0.95

                        // Damping (one-pole lowpass coefficient)
                        let damp_coef = 1.0 - damping_val * 0.7; // Higher damping = darker sound

                        // Modulation (simple LFO for chorus effect)
                        let lfo_rate = 0.8; // Hz
                        let lfo = (s.lfo_phase * std::f32::consts::TAU).sin() * mod_depth_val * 8.0; // ±8 samples modulation
                        s.lfo_phase = (s.lfo_phase + lfo_rate / s.sample_rate) % 1.0;

                        // LEFT TANK
                        // Read previous right tank output for cross-coupling
                        let right_to_left = s.right_delay2_buffer[s.right_delay2_idx];

                        // Input to left tank (with cross-coupling from right)
                        let left_input = input_to_tanks + right_to_left * decay_gain;

                        // Left APF1 (modulated)
                        let left_apf1_out = {
                            // Apply modulation by varying read position slightly
                            let mod_offset = lfo as isize;
                            let read_idx = ((s.left_apf1_idx as isize
                                + s.left_apf1_buffer.len() as isize
                                + mod_offset)
                                % s.left_apf1_buffer.len() as isize)
                                as usize;
                            let delayed = s.left_apf1_buffer[read_idx];
                            let output =
                                -left_input + delayed + decay_diffusion1 * (left_input - delayed);
                            s.left_apf1_buffer[s.left_apf1_idx] =
                                left_input + decay_diffusion1 * delayed;
                            s.left_apf1_idx = (s.left_apf1_idx + 1) % s.left_apf1_buffer.len();
                            output
                        };

                        // Left Delay1
                        let left_delay1_out = delay(
                            &mut s.left_delay1_buffer,
                            &mut s.left_delay1_idx,
                            left_apf1_out,
                        );

                        // Left APF2 (modulated differently)
                        let left_apf2_out = {
                            let mod_offset = -lfo as isize;
                            let read_idx = ((s.left_apf2_idx as isize
                                + s.left_apf2_buffer.len() as isize
                                + mod_offset)
                                % s.left_apf2_buffer.len() as isize)
                                as usize;
                            let delayed = s.left_apf2_buffer[read_idx];
                            let output = -left_delay1_out
                                + delayed
                                + decay_diffusion2 * (left_delay1_out - delayed);
                            s.left_apf2_buffer[s.left_apf2_idx] =
                                left_delay1_out + decay_diffusion2 * delayed;
                            s.left_apf2_idx = (s.left_apf2_idx + 1) % s.left_apf2_buffer.len();
                            output
                        };

                        // Damping LPF and Delay2
                        let left_damped =
                            s.left_lpf_state * damp_coef + left_apf2_out * (1.0 - damp_coef);
                        s.left_lpf_state = left_damped;

                        let left_delay2_out = delay(
                            &mut s.left_delay2_buffer,
                            &mut s.left_delay2_idx,
                            left_damped * decay_gain,
                        );

                        // RIGHT TANK
                        // Read previous left tank output for cross-coupling
                        let left_to_right = left_delay2_out;

                        // Input to right tank (with cross-coupling from left)
                        let right_input = input_to_tanks + left_to_right;

                        // Right APF1 (modulated)
                        let right_apf1_out = {
                            let mod_offset = -lfo as isize;
                            let read_idx = ((s.right_apf1_idx as isize
                                + s.right_apf1_buffer.len() as isize
                                + mod_offset)
                                % s.right_apf1_buffer.len() as isize)
                                as usize;
                            let delayed = s.right_apf1_buffer[read_idx];
                            let output =
                                -right_input + delayed + decay_diffusion1 * (right_input - delayed);
                            s.right_apf1_buffer[s.right_apf1_idx] =
                                right_input + decay_diffusion1 * delayed;
                            s.right_apf1_idx = (s.right_apf1_idx + 1) % s.right_apf1_buffer.len();
                            output
                        };

                        // Right Delay1
                        let right_delay1_out = delay(
                            &mut s.right_delay1_buffer,
                            &mut s.right_delay1_idx,
                            right_apf1_out,
                        );

                        // Right APF2 (modulated differently)
                        let right_apf2_out = {
                            let mod_offset = lfo as isize;
                            let read_idx = ((s.right_apf2_idx as isize
                                + s.right_apf2_buffer.len() as isize
                                + mod_offset)
                                % s.right_apf2_buffer.len() as isize)
                                as usize;
                            let delayed = s.right_apf2_buffer[read_idx];
                            let output = -right_delay1_out
                                + delayed
                                + decay_diffusion2 * (right_delay1_out - delayed);
                            s.right_apf2_buffer[s.right_apf2_idx] =
                                right_delay1_out + decay_diffusion2 * delayed;
                            s.right_apf2_idx = (s.right_apf2_idx + 1) % s.right_apf2_buffer.len();
                            output
                        };

                        // Damping LPF and Delay2
                        let right_damped =
                            s.right_lpf_state * damp_coef + right_apf2_out * (1.0 - damp_coef);
                        s.right_lpf_state = right_damped;

                        let right_delay2_out = delay(
                            &mut s.right_delay2_buffer,
                            &mut s.right_delay2_idx,
                            right_damped * decay_gain,
                        );

                        // 4. OUTPUT TAPS (sum multiple points for density)
                        // Using multiple tap points as suggested by Dattorro
                        let left_output =
                            (left_delay1_out + left_apf2_out + left_delay2_out) * 0.33;
                        let right_output =
                            (right_delay1_out + right_apf2_out + right_delay2_out) * 0.33;

                        (left_output, right_output)
                    } else {
                        (0.0, 0.0)
                    }
                } else {
                    (0.0, 0.0)
                };

                // Mix stereo output (average L+R for mono)
                let wet = (left_out + right_out) * 0.5;
                input_val * (1.0 - mix_val) + wet * mix_val
            }

            SignalNode::LushReverb {
                input,
                predelay,
                decay,
                size,
                diffusion,
                damping,
                spin,
                wander,
                freeze,
                mix,
                state: _,
            } => {
                // Evaluate input first
                let input_val = self.eval_signal(&input);

                // BYPASS MODE: For pipelined rendering, pass through unchanged
                if self.bypass_sequential_effects {
                    return input_val;
                }

                // Evaluate remaining parameters
                let predelay_val = self.eval_signal(&predelay);
                let decay_val = self.eval_signal(&decay);
                let size_val = self.eval_signal(&size);
                let diffusion_val = self.eval_signal(&diffusion);
                let damping_val = self.eval_signal(&damping);
                let spin_val = self.eval_signal(&spin);
                let wander_val = self.eval_signal(&wander);
                let freeze_val = self.eval_signal(&freeze);
                let mix_val = self.eval_signal(&mix);

                // SHARED STATE: Check if shared state is enabled for parallel rendering
                if let Some(ref registry) = self.shared_state {
                    if let Some(crate::shared_effect_state::SharedState::LushReverb(state_lock)) = registry.get(node_id.0) {
                        // Use shared state via RwLock
                        if let Ok(mut state) = state_lock.write() {
                            return state.process(
                                input_val,
                                predelay_val,
                                decay_val,
                                size_val,
                                diffusion_val,
                                damping_val,
                                spin_val,
                                wander_val,
                                freeze_val,
                                mix_val,
                            );
                        }
                    }
                }

                // Fallback: use inline state (non-parallel mode)
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::LushReverb { state: s, .. } = node {
                        s.process(
                            input_val,
                            predelay_val,
                            decay_val,
                            size_val,
                            diffusion_val,
                            damping_val,
                            spin_val,
                            wander_val,
                            freeze_val,
                            mix_val,
                        )
                    } else {
                        input_val
                    }
                } else {
                    input_val
                }
            }

            SignalNode::Convolution { input, state } => {
                let input_val = self.eval_signal(&input);

                // BYPASS MODE: For pipelined rendering, pass through unchanged
                if self.bypass_sequential_effects {
                    return input_val;
                }

                // Process through convolution
                let output = if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Convolution { state: s, .. } = node {
                        s.process(input_val)
                    } else {
                        input_val
                    }
                } else {
                    input_val // Fallback: pass through
                };

                output
            }

            SignalNode::SpectralFreeze {
                input,
                trigger,
                state,
            } => {
                let input_val = self.eval_signal(&input);

                // BYPASS MODE: For pipelined rendering, pass through unchanged
                if self.bypass_sequential_effects {
                    return input_val;
                }

                let trigger_val = self.eval_signal(&trigger);

                // Process through spectral freeze
                let output = if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::SpectralFreeze { state: s, .. } = node {
                        s.process(input_val, trigger_val)
                    } else {
                        input_val
                    }
                } else {
                    input_val // Fallback: pass through
                };

                output
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

                let phase = *state.phase.borrow() + rate_reduction;
                let mut output = *state.last_sample.borrow();

                if phase >= 1.0 {
                    // Reduce bit depth
                    let levels = (2.0_f32).powf(bit_depth);
                    let quantized = (input_val * levels).round() / levels;
                    output = quantized;

                    if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                        let node = Rc::make_mut(node_rc);
                        if let SignalNode::BitCrush { state: s, .. } = node {
                            *s.phase.borrow_mut() = phase - phase.floor();
                            *s.last_sample.borrow_mut() = quantized;
                        }
                    }
                } else if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::BitCrush { state: s, .. } = node {
                        *s.phase.borrow_mut() = phase;
                    }
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
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Chorus { state: s, .. } = node {
                        s.delay_buffer[s.write_idx] = input_val;
                        s.write_idx = (s.write_idx + 1) % buf_len;
                        s.lfo_phase = (lfo_phase + lfo_rate / self.sample_rate) % 1.0;
                    }
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
                    if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                        let node = Rc::make_mut(node_rc);
                        if let SignalNode::Flanger { state: s, .. } = node {
                            s.lfo_phase = (state.lfo_phase + lfo_rate / self.sample_rate) % 1.0;
                        }
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
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Flanger { state: s, .. } = node {
                        s.delay_buffer[s.write_idx] = input_val;
                        s.write_idx = (s.write_idx + 1) % buf_len;
                        s.lfo_phase = (lfo_phase + lfo_rate / self.sample_rate) % 1.0;
                        s.feedback_sample = wet;
                    }
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
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Compressor { state: s, .. } = node {
                        s.envelope = envelope;
                    }
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

            SignalNode::SidechainCompressor {
                main_input,
                sidechain_input,
                threshold,
                ratio,
                attack,
                release,
                state,
            } => {
                // Evaluate both inputs
                let main_val = self.eval_signal(&main_input);
                let sidechain_val = self.eval_signal(&sidechain_input);

                let threshold_db = self.eval_signal(&threshold).clamp(-60.0, 0.0);
                let ratio_val = self.eval_signal(&ratio).clamp(1.0, 20.0);
                let attack_time = self.eval_signal(&attack).clamp(0.001, 1.0);
                let release_time = self.eval_signal(&release).clamp(0.01, 3.0);

                // Convert threshold from dB to linear
                let threshold_lin = 10.0_f32.powf(threshold_db / 20.0);

                // Envelope follower tracks SIDECHAIN signal (not main)
                let sidechain_level = sidechain_val.abs();
                let mut envelope = state.envelope;

                // Envelope follower: attack when sidechain > envelope, release when sidechain < envelope
                let coeff = if sidechain_level > envelope {
                    // Attack: faster response to increasing levels
                    (-(1.0 / (attack_time * self.sample_rate))).exp()
                } else {
                    // Release: slower response to decreasing levels
                    (-(1.0 / (release_time * self.sample_rate))).exp()
                };

                envelope = coeff * envelope + (1.0 - coeff) * sidechain_level;

                // Update envelope state
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::SidechainCompressor { state: s, .. } = node {
                        s.envelope = envelope;
                    }
                }

                // Calculate gain reduction based on SIDECHAIN level
                let gain_reduction = if envelope > threshold_lin {
                    // Above threshold: apply compression
                    let envelope_db = 20.0 * envelope.log10();
                    let over_db = envelope_db - threshold_db;
                    let reduction_db = over_db * (1.0 - 1.0 / ratio_val);
                    10.0_f32.powf(-reduction_db / 20.0) // Convert to linear gain reduction
                } else {
                    1.0 // No reduction below threshold
                };

                // Apply compression to MAIN input
                main_val * gain_reduction
            }

            SignalNode::Expander {
                input,
                threshold,
                ratio,
                attack,
                release,
                state,
            } => {
                let input_val = self.eval_signal(&input);
                let threshold_db = self.eval_signal(&threshold).clamp(-60.0, 0.0);
                let ratio_val = self.eval_signal(&ratio).clamp(1.0, 10.0);
                let attack_time = self.eval_signal(&attack).clamp(0.001, 1.0);
                let release_time = self.eval_signal(&release).clamp(0.01, 3.0);

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
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Expander { state: s, .. } = node {
                        s.envelope = envelope;
                    }
                }

                // Calculate gain boost (inverse of compressor)
                let gain_boost = if envelope > threshold_lin {
                    // Above threshold: apply expansion (BOOST instead of reduction)
                    // For expander: boost = (envelope - threshold) * (ratio - 1)
                    let envelope_db = 20.0 * envelope.log10();
                    let over_db = envelope_db - threshold_db;
                    let boost_db = over_db * (ratio_val - 1.0); // Note: (ratio - 1), not (1 - 1/ratio)
                    10.0_f32.powf(boost_db / 20.0) // Convert to linear gain boost
                } else {
                    1.0 // No boost below threshold
                };

                // Apply expansion
                input_val * gain_boost
            }

            SignalNode::AdaptiveCompressor {
                main_input,
                sidechain_input,
                threshold,
                ratio,
                attack,
                release,
                adaptive_factor,
                state,
            } => {
                // Evaluate both inputs
                let main_val = self.eval_signal(&main_input);
                let sidechain_val = self.eval_signal(&sidechain_input);

                let threshold_db = self.eval_signal(&threshold).clamp(-60.0, 0.0);
                let ratio_val = self.eval_signal(&ratio).clamp(1.0, 20.0);
                let attack_time = self.eval_signal(&attack).clamp(0.001, 1.0);
                let release_time = self.eval_signal(&release).clamp(0.01, 3.0);
                let adapt_factor = self.eval_signal(&adaptive_factor).clamp(0.0, 1.0);

                // Convert threshold from dB to linear
                let threshold_lin = 10.0_f32.powf(threshold_db / 20.0);

                // Envelope follower tracks SIDECHAIN signal (like sidechain compressor)
                let sidechain_level = sidechain_val.abs();
                let mut envelope = state.envelope;
                let mut rms = state.current_rms;
                let mut rms_idx = state.rms_write_idx;

                // Envelope follower: attack when sidechain > envelope, release when sidechain < envelope
                let coeff = if sidechain_level > envelope {
                    // Attack: faster response to increasing levels
                    (-(1.0 / (attack_time * self.sample_rate))).exp()
                } else {
                    // Release: slower response to decreasing levels
                    (-(1.0 / (release_time * self.sample_rate))).exp()
                };

                envelope = coeff * envelope + (1.0 - coeff) * sidechain_level;

                // Calculate RMS of sidechain for adaptive behavior
                // Update RMS buffer
                let mut rms_buffer_copy = state.rms_buffer.clone();
                rms_buffer_copy[rms_idx] = sidechain_val * sidechain_val;
                rms_idx = (rms_idx + 1) % rms_buffer_copy.len();

                // Calculate RMS (average of squared values, then square root)
                let sum: f32 = rms_buffer_copy.iter().sum();
                rms = (sum / rms_buffer_copy.len() as f32).sqrt();

                // Update state
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::AdaptiveCompressor { state: s, .. } = node {
                        s.envelope = envelope;
                        s.current_rms = rms;
                        s.rms_buffer = rms_buffer_copy;
                        s.rms_write_idx = rms_idx;
                    }
                }

                // Adaptive threshold: modulate based on RMS level
                // When RMS is high, increase threshold (less compression)
                // When RMS is low, decrease threshold (more compression)
                let adaptive_threshold_db = threshold_db + (rms * 20.0 * adapt_factor);
                let adaptive_threshold_lin = 10.0_f32.powf(adaptive_threshold_db / 20.0);

                // Adaptive ratio: modulate based on RMS level
                // When RMS is high, reduce ratio (gentler compression)
                // When RMS is low, increase ratio (harder compression)
                let adaptive_ratio = ratio_val * (1.0 - (rms * adapt_factor * 0.5));
                let adaptive_ratio = adaptive_ratio.clamp(1.0, 20.0);

                // Calculate gain reduction based on SIDECHAIN level with adaptive parameters
                let gain_reduction = if envelope > adaptive_threshold_lin {
                    // Above threshold: apply compression
                    let envelope_db = 20.0 * envelope.log10();
                    let over_db = envelope_db - adaptive_threshold_db;
                    let reduction_db = over_db * (1.0 - 1.0 / adaptive_ratio);
                    10.0_f32.powf(-reduction_db / 20.0) // Convert to linear gain reduction
                } else {
                    1.0 // No reduction below threshold
                };

                // Apply compression to MAIN input
                main_val * gain_reduction
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
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Tremolo { phase: p, .. } = node {
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
                }

                output_val
            }

            SignalNode::Vibrato {
                input,
                rate,
                depth,
                phase,
                delay_buffer,
                buffer_pos,
            } => {
                let input_val = self.eval_signal(&input);
                let rate_hz = self.eval_signal(&rate).clamp(0.1, 20.0);
                let depth_semitones = self.eval_signal(&depth).clamp(0.0, 2.0);

                // Fast bypass for zero depth
                if depth_semitones < 0.001 {
                    return input_val;
                }

                let mut output_val = input_val;

                // Access and update vibrato state
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Vibrato {
                        phase,
                        delay_buffer: buf,
                        buffer_pos: pos,
                        ..
                    } = node
                    {
                        // Initialize buffer if empty (first call)
                        let buffer_size = (self.sample_rate * 0.05) as usize; // 50ms buffer
                        if buf.is_empty() {
                            buf.resize(buffer_size, 0.0);
                        }

                        // Write input to delay buffer
                        buf[*pos] = input_val;

                        // Advance phase
                        *phase += rate_hz * 2.0 * std::f32::consts::PI / self.sample_rate;

                        // Wrap phase to [0, 2π]
                        if *phase >= 2.0 * std::f32::consts::PI {
                            *phase -= 2.0 * std::f32::consts::PI;
                        }

                        // Calculate LFO (sine wave, -1 to +1)
                        let lfo = phase.sin();

                        // Convert depth from semitones to delay time
                        // depth in semitones -> frequency ratio -> time ratio
                        // 1 semitone = 2^(1/12) ≈ 1.059 frequency ratio
                        let max_delay_ms = 10.0; // Maximum 10ms delay
                        let delay_ms = max_delay_ms * (depth_semitones / 2.0) * (1.0 + lfo);
                        let delay_samples = (delay_ms * self.sample_rate / 1000.0).max(0.0);

                        // Calculate read position (fractional)
                        let read_pos_float = *pos as f32 - delay_samples;
                        let read_pos_wrapped = if read_pos_float < 0.0 {
                            read_pos_float + buf.len() as f32
                        } else {
                            read_pos_float
                        };

                        // Linear interpolation for fractional delay
                        let read_pos_int = read_pos_wrapped as usize % buf.len();
                        let read_pos_next = (read_pos_int + 1) % buf.len();
                        let frac = read_pos_wrapped.fract();

                        output_val = buf[read_pos_int] * (1.0 - frac) + buf[read_pos_next] * frac;

                        // Advance buffer position
                        *pos = (*pos + 1) % buf.len();
                    }
                }

                output_val
            }

            SignalNode::Phaser {
                input,
                rate,
                depth,
                feedback,
                stages,
                phase,
                allpass_z1,
                allpass_y1,
                feedback_sample,
            } => {
                let input_val = self.eval_signal(&input);
                let rate_hz = self.eval_signal(&rate).clamp(0.05, 5.0);
                let depth_val = self.eval_signal(&depth).clamp(0.0, 1.0);
                let feedback_val = self.eval_signal(&feedback).clamp(0.0, 0.95);

                // Fast bypass for zero depth
                if depth_val < 0.001 {
                    return input_val;
                }

                let mut output_val = input_val;

                // Access and update phaser state
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Phaser {
                        phase,
                        allpass_z1: z1,
                        allpass_y1: y1,
                        feedback_sample: fb_sample,
                        stages: num_stages,
                        ..
                    } = node
                    {
                        // Initialize allpass filter states if needed
                        if z1.is_empty() {
                            z1.resize(*num_stages, 0.0);
                            y1.resize(*num_stages, 0.0);
                        }

                        // Advance LFO phase
                        *phase += rate_hz * 2.0 * std::f32::consts::PI / self.sample_rate;
                        if *phase >= 2.0 * std::f32::consts::PI {
                            *phase -= 2.0 * std::f32::consts::PI;
                        }

                        // Calculate LFO (sine wave, 0 to 1)
                        let lfo = (phase.sin() + 1.0) * 0.5;

                        // Map LFO to cutoff frequency (200 Hz to 2000 Hz sweep)
                        let min_freq = 200.0;
                        let max_freq = 2000.0;
                        let cutoff = min_freq + (max_freq - min_freq) * lfo * depth_val;

                        // Calculate allpass coefficient
                        // a = (tan(π*fc/fs) - 1) / (tan(π*fc/fs) + 1)
                        let tan_val = (std::f32::consts::PI * cutoff / self.sample_rate).tan();
                        let a = (tan_val - 1.0) / (tan_val + 1.0);

                        // Apply feedback
                        let mut signal = input_val + *fb_sample * feedback_val;

                        // Apply allpass filter cascade
                        for stage in 0..*num_stages {
                            // First-order allpass: y[n] = a*x[n] + x[n-1] - a*y[n-1]
                            let output = a * signal + z1[stage] - a * y1[stage];

                            // Update state
                            z1[stage] = signal;
                            y1[stage] = output;

                            signal = output;
                        }

                        // Store for feedback
                        *fb_sample = signal;

                        // Mix filtered signal with dry signal (creates notches)
                        output_val = (input_val + signal) * 0.5;
                    }
                }

                output_val
            }

            SignalNode::RingMod { input, freq, phase } => {
                let input_val = self.eval_signal(&input);
                let carrier_freq = self.eval_signal(&freq).clamp(20.0, 5000.0);

                let mut output_val = input_val;

                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::RingMod { phase: p, .. } = node {
                        *p += carrier_freq * 2.0 * std::f32::consts::PI / self.sample_rate;
                        if *p >= 2.0 * std::f32::consts::PI {
                            *p -= 2.0 * std::f32::consts::PI;
                        }
                        let carrier = p.sin();
                        output_val = input_val * carrier;
                    }
                }

                output_val
            }

            SignalNode::FMCrossMod {
                carrier,
                modulator,
                mod_depth,
            } => {
                let carrier_val = self.eval_signal(&carrier);
                let modulator_val = self.eval_signal(&modulator);
                let depth_val = self.eval_signal(&mod_depth);

                // FM cross-modulation: carrier * cos(2π * depth * modulator)
                use std::f32::consts::PI;
                let phase_offset = 2.0 * PI * depth_val * modulator_val;
                carrier_val * phase_offset.cos()
            }

            SignalNode::FundspUnit {
                unit_type,
                inputs,
                state,
            } => {
                // 1. Evaluate ALL input signals (audio + parameters)
                let input_values: Vec<f32> = inputs
                    .iter()
                    .map(|signal| self.eval_signal(signal))
                    .collect();

                // 2. For units with static constructors, check if parameters changed
                //    and recreate unit if needed (to update internal state)
                let state_guard = state.lock().unwrap();
                let needs_recreation = match unit_type {
                    // Units with static constructors need recreation when params change
                    FundspUnitType::OrganHz => {
                        input_values.len() >= 1
                            && (state_guard.params[0] - input_values[0]).abs() > 0.1
                    }
                    FundspUnitType::MoogHz => {
                        input_values.len() >= 3
                            && ((state_guard.params[0] - input_values[1]).abs() > 1.0
                                || (state_guard.params[1] - input_values[2]).abs() > 0.01)
                    }
                    FundspUnitType::ReverbStereo => {
                        input_values.len() >= 3
                            && ((state_guard.params[0] - input_values[1]).abs() > 0.01
                                || (state_guard.params[1] - input_values[2]).abs() > 0.01)
                    }
                    FundspUnitType::Chorus => {
                        input_values.len() >= 5
                            && ((state_guard.params[0] - input_values[1]).abs() > 0.5
                                || (state_guard.params[1] - input_values[2]).abs() > 0.01
                                || (state_guard.params[2] - input_values[3]).abs() > 0.01
                                || (state_guard.params[3] - input_values[4]).abs() > 0.01)
                    }
                    FundspUnitType::SawHz => {
                        input_values.len() >= 1
                            && (state_guard.params[0] - input_values[0]).abs() > 0.1
                    }
                    FundspUnitType::SquareHz => {
                        input_values.len() >= 1
                            && (state_guard.params[0] - input_values[0]).abs() > 0.1
                    }
                    FundspUnitType::TriangleHz => {
                        input_values.len() >= 1
                            && (state_guard.params[0] - input_values[0]).abs() > 0.1
                    }
                    // Parameterless units or audio-rate-only units never need recreation
                    FundspUnitType::Noise | FundspUnitType::Pink | FundspUnitType::Pulse => false,
                    _ => false,
                };

                // CRITICAL: Drop the lock guard before attempting to lock again for tick
                // Otherwise we get a deadlock for units that don't need recreation
                drop(state_guard);

                if needs_recreation {
                    let mut state_mut = state.lock().unwrap();

                    // Recreate unit with new parameters
                    *state_mut = match unit_type {
                        FundspUnitType::OrganHz => {
                            FundspState::new_organ_hz(input_values[0], self.sample_rate as f64)
                        }
                        FundspUnitType::MoogHz => FundspState::new_moog_hz(
                            input_values[1],
                            input_values[2],
                            self.sample_rate as f64,
                        ),
                        FundspUnitType::ReverbStereo => FundspState::new_reverb_stereo(
                            input_values[1],
                            input_values[2],
                            self.sample_rate as f64,
                        ),
                        FundspUnitType::Chorus => FundspState::new_chorus(
                            input_values[1] as u64,
                            input_values[2],
                            input_values[3],
                            input_values[4],
                            self.sample_rate as f64,
                        ),
                        FundspUnitType::SawHz => {
                            FundspState::new_saw_hz(input_values[0], self.sample_rate as f64)
                        }
                        FundspUnitType::SquareHz => {
                            FundspState::new_square_hz(input_values[0], self.sample_rate as f64)
                        }
                        FundspUnitType::TriangleHz => {
                            FundspState::new_triangle_hz(input_values[0], self.sample_rate as f64)
                        }
                        _ => return 0.0, // Should never happen
                    };
                }

                // 3. Call fundsp tick() with all inputs
                let output = state.lock().unwrap().tick(&input_values);

                output
            }

            SignalNode::Tap { input, state } => {
                // Evaluate input signal
                let sample = self.eval_signal(&input);

                // Record sample (thread-safe)
                if let Ok(mut tap_state) = state.lock() {
                    tap_state.record(sample);
                }

                // Pass through unchanged
                sample
            }

            SignalNode::Output { input } => self.eval_signal(&input),

            SignalNode::Pattern {
                pattern_str,
                pattern,
                last_value,
                last_trigger_time: _,
            } => {
                // OPTION B OPTIMIZATION: Use pre-computed events if available
                let current_cycle = self.get_cycle_position();
                let events = if let Some(cached_events) = self.pattern_event_cache.get(node_id) {
                    // Find events active at current cycle position from cached events
                    cached_events
                        .iter()
                        .filter(|event| {
                            let begin = event.part.begin.to_float();
                            let end = event.part.end.to_float();
                            current_cycle >= begin && current_cycle < end
                        })
                        .cloned()
                        .collect::<Vec<_>>()
                } else {
                    // Fallback: Query pattern directly (shouldn't happen in normal operation)
                    let sample_width = 1.0 / self.sample_rate as f64 / self.cps as f64;
                    let state = State {
                        span: TimeSpan::new(
                            Fraction::from_float(current_cycle),
                            Fraction::from_float(current_cycle + sample_width),
                        ),
                        controls: HashMap::new(),
                    };
                    pattern.query(&state)
                };

                let mut current_value = *last_value; // Default to last value

                // DEBUG: Log all pattern queries
                if std::env::var("DEBUG_PATTERN").is_ok()
                    && self.sample_count < 200
                    && self.sample_count % 20 == 0
                {
                    eprintln!(
                        "Pattern '{}' at cycle {:.6}, sample {}: {} events",
                        pattern_str,
                        self.get_cycle_position(),
                        self.sample_count,
                        events.len()
                    );
                    if let Some(event) = events.first() {
                        eprintln!(
                            "  First event: '{}' at [{:.6}, {:.6})",
                            event.value,
                            event.part.begin.to_float(),
                            event.part.end.to_float()
                        );
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
                        if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                            let node = Rc::make_mut(node_rc);
                            if let SignalNode::Pattern { last_value: lv, .. } = node {
                                *lv = 0.0;
                            }
                        }

                        // DEBUG: Log rests
                        if std::env::var("DEBUG_PATTERN").is_ok() && *last_value != 0.0 {
                            eprintln!(
                                "Pattern '{}' at cycle {:.4}: REST (was {})",
                                pattern_str,
                                self.get_cycle_position(),
                                last_value
                            );
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
                            current_value = *last_value;
                        }

                        // DEBUG: Log pattern value changes
                        if std::env::var("DEBUG_PATTERN").is_ok() && current_value != *last_value {
                            eprintln!(
                                "Pattern '{}' at cycle {:.4}: value changed {} -> {} (event: '{}')",
                                pattern_str,
                                self.get_cycle_position(),
                                last_value,
                                current_value,
                                s
                            );
                        }

                        // Update last_value for next time
                        if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                            let node = Rc::make_mut(node_rc);
                            if let SignalNode::Pattern { last_value: lv, .. } = node {
                                *lv = current_value;
                            }
                        }
                    }
                }

                current_value
            }

            SignalNode::SignalAsPattern {
                signal,
                last_sampled_value,
                last_sample_cycle,
            } => {
                // Sample the signal once per cycle and cache the value
                let current_cycle = self.get_cycle_position().floor();
                let last_cycle = *last_sample_cycle.lock().unwrap() as f64;

                // If we've moved to a new cycle, sample the signal
                if (current_cycle - last_cycle).abs() > 0.01 {
                    let sampled = self.eval_signal(signal);
                    *last_sampled_value.lock().unwrap() = sampled;
                    *last_sample_cycle.lock().unwrap() = current_cycle as f32;
                }

                // Return the cached value
                *last_sampled_value.lock().unwrap()
            }

            SignalNode::CycleTrigger {
                last_cycle,
                pulse_width,
            } => {
                let cycle_position = self.get_cycle_position();
                let current_cycle = cycle_position.floor() as i32;
                let cycle_fraction = cycle_position - cycle_position.floor();
                let pulse_width_val = self.eval_signal(&pulse_width); // Pattern-modulatable
                let pulse_duration = pulse_width_val / self.cps as f32; // Convert pulse width to cycles

                // Output 1.0 if we're within the pulse duration at the start of a new cycle
                // Output 0.0 otherwise
                let output = if cycle_fraction < pulse_duration as f64 {
                    1.0
                } else {
                    0.0
                };

                // Update last_cycle for state tracking (not currently used but good to have)
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::CycleTrigger { last_cycle: lc, .. } = node {
                        *lc = current_cycle;
                    }
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
                unit_mode,
                loop_enabled,
                begin,
                end,
            } => {
                // DEBUG: Log Sample node evaluation (disabled - too verbose)
                // if std::env::var("DEBUG_SAMPLE_EVENTS").is_ok() && self.sample_count < 100 {
                //     eprintln!(
                //         "Evaluating Sample node '{}' at sample {}, cycle_pos={:.6}",
                //         pattern_str, self.sample_count, self.get_cycle_position()
                //     );
                // }

                // Set the default source node for all voice triggers in this Sample node
                // This separates outputs so each output only hears its own samples
                self.voice_manager
                    .borrow_mut()
                    .set_default_source_node(node_id.0);
                if std::env::var("DEBUG_SOURCE_NODE").is_ok() {
                    eprintln!(
                        "[SOURCE_NODE] Sample node {} set as default source",
                        node_id.0
                    );
                }

                // OPTION B OPTIMIZATION: Use pre-computed events if available
                let events = if let Some(cached_events) = self.pattern_event_cache.get(node_id) {
                    // Use cached events (computed once per buffer for entire buffer span)
                    // Filter to current cycle
                    let current_cycle_start = self.get_cycle_position().floor();

                    cached_events
                        .iter()
                        .filter(|event| {
                            let begin = event.part.begin.to_float();
                            begin >= current_cycle_start && begin < current_cycle_start + 1.0
                        })
                        .cloned()
                        .collect::<Vec<_>>()
                } else {
                    // Fallback: Query pattern directly (shouldn't happen in normal operation)
                    // Use full-cycle window to ensure transforms like degrade see all events
                    // The event deduplication logic below prevents re-triggering
                    let current_cycle_start = self.get_cycle_position().floor();
                    let state = State {
                        span: TimeSpan::new(
                            Fraction::from_float(current_cycle_start),
                            Fraction::from_float(current_cycle_start + 1.0),
                        ),
                        controls: HashMap::new(),
                    };
                    pattern.query(&state)
                };

                // Check if we've crossed into a new cycle
                let current_cycle = self.get_cycle_position().floor() as i32;
                let cycle_changed = current_cycle != *last_cycle;

                // Get the last EVENT start time we triggered
                // DON'T reset on cycle boundaries - events can span across cycles
                let mut last_event_start = if let Some(Some(node)) = self.nodes.get(node_id.0) {
                    if let SignalNode::Sample {
                        last_trigger_time: lt,
                        ..
                    } = &**node
                    {
                        if std::env::var("DEBUG_SAMPLE_EVENTS").is_ok() {
                            eprintln!(
                                "[DEDUP] Node {} reading last_trigger_time={:.6} from Sample node",
                                node_id.0, *lt
                            );
                        }
                        *lt as f64
                    } else {
                        -1.0
                    }
                } else {
                    -1.0
                };

                // NOTE: We used to reset last_event_start on cycle boundaries,
                // but this caused duplicate triggers for events that span cycles
                // (e.g., "bd ~bass bd ~bass" $ slow 3 would trigger ~bass twice)
                // The absolute event start time is sufficient for deduplication

                // Track the latest event start time we trigger in this sample
                let mut latest_triggered_start = last_event_start;

                // DEBUG: Log event processing (disabled - too verbose)
                // if std::env::var("DEBUG_SAMPLE_EVENTS").is_ok() && !events.is_empty() {
                //     eprintln!(
                //         "Sample node at cycle {:.3}: {} events",
                //         self.get_cycle_position(),
                //         events.len()
                //     );
                // }

                // PHASE 2: Cycle-level caching for parallel bus synthesis
                // Check if we need to resynthesize for this cycle
                let current_cycle_floor = self.cached_cycle_position.floor() as i64;
                if self.cycle_bus_cache.cycle_floor != current_cycle_floor {
                    // Cache miss - new cycle, need to presynthesize
                    let new_buffers = self.presynthesize_buses_parallel(&events, last_event_start);
                    self.cycle_bus_cache.cycle_floor = current_cycle_floor;
                    self.cycle_bus_cache.buffers = new_buffers;
                }
                // Clone the cache for use in this buffer (Arc makes this cheap)
                let bus_buffer_cache = self.cycle_bus_cache.buffers.clone();

                // Trigger voices for ALL new events
                // An event should be triggered if its START is after the last event we triggered
                if std::env::var("DEBUG_SAMPLE_EVENTS").is_ok() && !events.is_empty() {
                    eprintln!(
                        "[SAMPLE_EVENTS] Node {} processing {} events",
                        node_id.0,
                        events.len()
                    );
                }
                for event in events.iter() {
                    let sample_name = event.value.trim();

                    // Skip rests
                    if sample_name == "~" || sample_name.is_empty() {
                        continue;
                    }

                    if std::env::var("DEBUG_SAMPLE_EVENTS").is_ok() {
                        eprintln!(
                            "[SAMPLE_EVENTS] Processing event: sample_name='{}', is_bus_trigger={}",
                            sample_name,
                            sample_name.starts_with('~')
                        );
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
                    // 2. Start BEFORE the current cycle position (we've passed the event time)
                    // Use tiny epsilon for floating-point comparison (1 microsecond in cycle time)
                    let epsilon = 1e-6;
                    let event_is_new = event_start_abs > last_event_start + epsilon
                        && event_start_abs < self.get_cycle_position() + epsilon;

                    // DEBUG: Log event evaluation (disabled - too verbose)
                    if std::env::var("DEBUG_SAMPLE_EVENTS").is_ok() {
                        eprintln!(
                            "  Event '{}' at {:.6}: event_is_new={} (last={:.6}, current={:.6})",
                            sample_name,
                            event_start_abs,
                            event_is_new,
                            last_event_start,
                            self.get_cycle_position()
                        );
                    }

                    if event_is_new {
                        // DEBUG: Log triggered events
                        if std::env::var("DEBUG_SAMPLE_EVENTS").is_ok() {
                            let audio_time = self.sample_count as f64 / self.sample_rate as f64;
                            eprintln!(
                                "  Triggering: '{}' at cycle {:.6} (cycle_pos={:.6}, sample={}, audio_time={:.6}s)",
                                sample_name, event_start_abs, self.get_cycle_position(), self.sample_count, audio_time
                            );
                        }

                        // Evaluate DSP parameters at THIS EVENT'S start time
                        // This ensures each event gets its own parameter values from the pattern
                        let mut gain_val = self
                            .eval_signal_at_time(&gain, event_start_abs)
                            .max(0.0)
                            .min(10.0);

                        // Check event context for stut_gain multiplier (set by stut transform)
                        if let Some(stut_gain_str) = event.context.get("stut_gain") {
                            if let Ok(stut_mult) = stut_gain_str.parse::<f32>() {
                                gain_val *= stut_mult;
                            }
                        }

                        // Check event context for pan override (set by transforms like jux)
                        let pan_val = if let Some(pan_str) = event.context.get("pan") {
                            pan_str.parse::<f32>().unwrap_or(0.0).clamp(-1.0, 1.0)
                        } else {
                            self.eval_signal_at_time(&pan, event_start_abs)
                                .clamp(-1.0, 1.0)
                        };

                        // Check event context for speed override (set by transforms like loopAt, hurry)
                        let speed_val = if let Some(hurry_str) = event.context.get("hurry_speed") {
                            // hurry transform sets hurry_speed (combines fast + speed)
                            hurry_str.parse::<f32>().unwrap_or(1.0).clamp(-10.0, 10.0)
                        } else if let Some(speed_str) = event.context.get("speed") {
                            // speed parameter from loopAt or explicit speed control
                            speed_str.parse::<f32>().unwrap_or(1.0).clamp(-10.0, 10.0)
                        } else {
                            self.eval_signal_at_time(&speed, event_start_abs)
                                .clamp(-10.0, 10.0)
                        };
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
                        // Supports: numbers (5), letter notes (c4, e4, g4), solfège (do, re, mi)
                        // Also supports chord notation: "c4'maj" -> vec![0, 4, 7] (C, E, G)
                        let chord_notes = self.eval_note_signal_as_chord(&note, event_start_abs);

                        // CRITICAL: If note pattern returned empty (rest), skip this event entirely
                        // This handles `# note "~ c4"` where ~ should produce silence
                        if chord_notes.is_empty() {
                            continue;
                        }

                        // Scale gain by 1/sqrt(n) to prevent clipping when multiple voices sum
                        // Using sqrt gives perceptually correct loudness (RMS scaling)
                        let chord_size = chord_notes.len();
                        let chord_gain_scale = if chord_size > 1 {
                            1.0 / (chord_size as f32).sqrt()
                        } else {
                            1.0
                        };
                        gain_val *= chord_gain_scale;

                        // DEBUG: Log chord notes
                        if std::env::var("DEBUG_SAMPLE_EVENTS").is_ok() {
                            eprintln!(
                                "    Chord notes for '{}': {:?} (gain scaled by {:.3})",
                                sample_name, chord_notes, chord_gain_scale
                            );
                        }

                        // Evaluate envelope parameters
                        let attack_val = self
                            .eval_signal_at_time(&attack, event_start_abs)
                            .max(0.0)
                            .min(10.0); // Attack time in seconds
                        let mut release_val = self
                            .eval_signal_at_time(&release, event_start_abs)
                            .max(0.0)
                            .min(10.0); // Release time in seconds

                        // Check if event has legato duration in context (from legato transform)
                        // Store for later use in auto-release calculation
                        let legato_duration_opt = event
                            .context
                            .get("legato_duration")
                            .and_then(|s| s.parse::<f32>().ok());

                        // Legacy: Update release_val for old code paths (will be superseded by ADSR+auto-release)
                        if let Some(duration_cycles) = legato_duration_opt {
                            // Convert duration from cycles to seconds using tempo
                            // cps is cycles/second, so seconds = cycles / cps
                            let duration_seconds = duration_cycles / self.cps;
                            release_val = duration_seconds.max(0.001).min(10.0);
                        }

                        // CRITICAL FIX: When attack=0 and release=0 (default), don't apply
                        // a short envelope that cuts off samples. Instead use sensible defaults
                        // that let samples play through naturally.
                        let (final_attack, final_release) =
                            if attack_val == 0.0 && release_val == 0.0 {
                                // No envelope requested: use anti-click attack and long release
                                // to let the sample play through completely (TidalCycles behavior)
                                // Long release (10s) ensures even long samples/loops play through
                                (0.001, 10.0) // 1ms attack (anti-click), 10s release (enough for long samples)
                            } else {
                                // Explicit envelope requested: use the values as-is
                                (attack_val, release_val)
                            };

                        // Evaluate unit mode and loop parameters
                        let unit_mode_val = self.eval_signal_at_time(&unit_mode, event_start_abs);
                        let loop_enabled_val =
                            self.eval_signal_at_time(&loop_enabled, event_start_abs);

                        // Convert to appropriate types
                        let unit_mode_enum = if unit_mode_val > 0.5 {
                            crate::voice_manager::UnitMode::Cycle
                        } else {
                            crate::voice_manager::UnitMode::Rate
                        };
                        let loop_enabled_bool = loop_enabled_val > 0.5;

                        // Evaluate begin and end parameters for sample slicing
                        // begin and end are 0.0-1.0 values representing fraction of sample
                        // Check event context first (set by transforms like striate/slice)
                        let begin_val = if let Some(begin_str) = event.context.get("begin") {
                            begin_str.parse::<f32>().unwrap_or(0.0).clamp(0.0, 1.0)
                        } else {
                            self.eval_signal_at_time(&begin, event_start_abs)
                                .clamp(0.0, 1.0)
                        };
                        let end_val = if let Some(end_str) = event.context.get("end") {
                            end_str.parse::<f32>().unwrap_or(1.0).clamp(0.0, 1.0)
                        } else {
                            self.eval_signal_at_time(&end, event_start_abs)
                                .clamp(0.0, 1.0)
                        };

                        // DEBUG: Print cut group info
                        if std::env::var("DEBUG_CUT_GROUPS").is_ok() {
                            eprintln!("Triggering {} at cycle {:.3}, cut_group_val={:.1}, cut_group_opt={:?}",
                                final_sample_name, event_start_abs, cut_group_val, cut_group_opt);
                        }

                        // Loop over all chord notes (for single notes, this is just one iteration)
                        for &note_semitones in &chord_notes {
                            // Calculate pitch shift for this specific chord note
                            // note_semitones can be:
                            // - >= 1000: ABSOLUTE MIDI (offset by 1000), e.g., 1048 = C3 (MIDI 48)
                            // - < 1000: RELATIVE semitones, e.g., 12 = one octave up
                            // For samples, we convert absolute MIDI to relative semitones from C4 (MIDI 60)
                            let relative_semitones = if note_semitones >= 1000.0 {
                                // Absolute MIDI: convert to semitones relative to C4 (MIDI 60)
                                // C3 (MIDI 48) -> 48 - 60 = -12 semitones (one octave down)
                                // C5 (MIDI 72) -> 72 - 60 = +12 semitones (one octave up)
                                note_semitones - 1000.0 - 60.0
                            } else {
                                // Already relative semitones
                                note_semitones
                            };

                            let pitch_shift_multiplier = if relative_semitones != 0.0 {
                                2.0_f32.powf(relative_semitones / 12.0)
                            } else {
                                1.0
                            };
                            let final_speed = speed_val * pitch_shift_multiplier;

                            // Handle bus triggering vs regular sample loading
                            if is_bus_trigger {
                                if std::env::var("DEBUG_BUS_LOOKUP").is_ok() {
                                    eprintln!(
                                        "[BUS] Looking up bus '{}', is_bus_trigger={}",
                                        actual_name, is_bus_trigger
                                    );
                                }
                                // Look up the bus
                                if let Some(bus_node_id) = self.buses.get(actual_name).copied() {
                                    if std::env::var("DEBUG_BUS_LOOKUP").is_ok() {
                                        eprintln!(
                                            "[BUS] Found bus '{}' -> node_id={}",
                                            actual_name, bus_node_id.0
                                        );
                                    }
                                    // CONTINUOUS SYNTHESIS ARCHITECTURE:
                                    // Instead of pre-rendering bus to a fixed-length buffer,
                                    // trigger a synthesis voice that evaluates the bus node continuously.
                                    // This ensures synthesis state persists across buffer boundaries.

                                    // Use envelope parameters - respect user settings or use defaults
                                    // If user set AR parameter explicitly, use those values
                                    // Otherwise use sensible defaults for synthesis (smooth onset + moderate release)
                                    let (bus_attack, bus_release) =
                                        if attack_val > 0.0 || release_val > 0.0 {
                                            // User explicitly set envelope: use their values
                                            (final_attack, final_release)
                                        } else {
                                            // No explicit envelope: use defaults (10ms attack, 0.3s release)
                                            // 0.3s allows natural decay without turning melodies into drones
                                            (0.01, 0.3) // 10ms attack (anti-click), 300ms release (melodic)
                                        };

                                    // DEBUG: Log synthesis voice triggering
                                    if std::env::var("DEBUG_VOICE_TRIGGER").is_ok() {
                                        eprintln!("    Triggering continuous synthesis voice: bus_node_id={}, gain={}, pan={}, source_node={}",
                                            bus_node_id.0, gain_val, pan_val, node_id.0);
                                    }

                                    // Trigger continuous synthesis voice
                                    // TODO: Support other envelope types (ADSR, Segments, Curve) for synthesis voices
                                    // For now, use simple percussion envelope
                                    self.voice_manager.borrow_mut().trigger_synthesis_voice(
                                        bus_node_id.0, // Pass raw NodeId (usize)
                                        gain_val,
                                        pan_val,
                                        cut_group_opt,
                                        bus_attack,
                                        bus_release,
                                        note_semitones, // Pitch offset for note parameter
                                    );

                                    // Note: unit mode and loop don't apply to synthesis voices
                                    // Synthesis continues until envelope finishes
                                } else {
                                    eprintln!(
                                        "Warning: Bus '{}' not found for trigger",
                                        actual_name
                                    );
                                }
                            } else {
                                // Regular sample loading
                                let sample_data_opt =
                                    self.sample_bank.borrow_mut().get_sample(&final_sample_name);
                                // DEBUG: Log sample loading
                                if std::env::var("DEBUG_SAMPLE_EVENTS").is_ok()
                                    && self.sample_count < 20
                                {
                                    eprintln!(
                                        "  Sample '{}' loaded: {}",
                                        final_sample_name,
                                        sample_data_opt.is_some()
                                    );
                                }
                                if let Some(sample_data) = sample_data_opt {
                                    // Apply begin/end slicing if specified
                                    let sliced_sample_data = if begin_val > 0.0 || end_val < 1.0 {
                                        let sample_len = sample_data.len();
                                        let begin_sample = (begin_val * sample_len as f32) as usize;
                                        let end_sample = (end_val * sample_len as f32) as usize;

                                        // Ensure valid range
                                        let begin_sample =
                                            begin_sample.min(sample_len.saturating_sub(1));
                                        let end_sample =
                                            end_sample.clamp(begin_sample + 1, sample_len);

                                        // Create sliced copy of the sample (preserves stereo)
                                        let sliced_sample =
                                            sample_data.slice(begin_sample, end_sample);
                                        std::sync::Arc::new(sliced_sample)
                                    } else {
                                        // No slicing needed, use original sample
                                        sample_data
                                    };

                                    // ENVELOPE STRATEGY:
                                    // - If user set AR explicitly: use their values (full control)
                                    // - Otherwise: use final_release which is 10s (let samples play through completely)
                                    // NO SMART RELEASE: Don't override defaults to prevent voice accumulation.
                                    // Samples should play naturally unless user specifies otherwise.
                                    let smart_release = final_release;

                                    // Trigger voice using appropriate envelope type
                                    // LEGATO OVERRIDE: When legato is present, use ADSR with sharp settings
                                    if let Some(legato_cycles) = legato_duration_opt {
                                        // Use ADSR with brick-wall envelope for legato
                                        // Attack: 1ms (instant), Decay: 1ms, Sustain: 100%, Release: 3ms (instant)
                                        let sharp_attack = 0.001;
                                        let sharp_decay = 0.001;
                                        let sharp_sustain = 1.0;
                                        let sharp_release = 0.003;

                                        self.voice_manager.borrow_mut().trigger_sample_with_adsr(
                                            sliced_sample_data.clone(),
                                            gain_val,
                                            pan_val,
                                            final_speed,
                                            cut_group_opt,
                                            sharp_attack,
                                            sharp_decay,
                                            sharp_sustain,
                                            sharp_release,
                                        );

                                        // Calculate auto-release time
                                        // Convert legato duration from cycles to seconds
                                        let duration_seconds = legato_cycles / self.cps;
                                        // Subtract attack and release times to get sustain duration
                                        let sustain_seconds =
                                            (duration_seconds - sharp_attack - sharp_release)
                                                .max(0.0);
                                        // Convert to samples
                                        let auto_release_samples =
                                            (sustain_seconds * self.sample_rate as f32) as usize;

                                        // Set auto-release on the last triggered voice
                                        self.voice_manager
                                            .borrow_mut()
                                            .set_last_voice_auto_release(auto_release_samples);
                                    } else {
                                        // No legato: use smart envelope with sample-duration-based release
                                        match envelope_type {
                                            Some(RuntimeEnvelopeType::Percussion) | None => {
                                                self.voice_manager
                                                    .borrow_mut()
                                                    .trigger_sample_with_envelope(
                                                        sliced_sample_data.clone(),
                                                        gain_val,
                                                        pan_val,
                                                        final_speed,
                                                        cut_group_opt,
                                                        final_attack,
                                                        smart_release,
                                                    );
                                            }
                                            Some(RuntimeEnvelopeType::ADSR {
                                                ref decay,
                                                ref sustain,
                                            }) => {
                                                let decay_val = self
                                                    .eval_signal_at_time(decay, event_start_abs)
                                                    .max(0.001);
                                                let sustain_val = self
                                                    .eval_signal_at_time(sustain, event_start_abs)
                                                    .clamp(0.0, 1.0);
                                                self.voice_manager
                                                    .borrow_mut()
                                                    .trigger_sample_with_adsr(
                                                        sliced_sample_data.clone(),
                                                        gain_val,
                                                        pan_val,
                                                        final_speed,
                                                        cut_group_opt,
                                                        final_attack,
                                                        decay_val,
                                                        sustain_val,
                                                        smart_release,
                                                    );
                                            }
                                            Some(RuntimeEnvelopeType::Segments {
                                                ref levels,
                                                ref times,
                                            }) => {
                                                self.voice_manager
                                                    .borrow_mut()
                                                    .trigger_sample_with_segments(
                                                        sliced_sample_data.clone(),
                                                        gain_val,
                                                        pan_val,
                                                        final_speed,
                                                        cut_group_opt,
                                                        levels.clone(),
                                                        times.clone(),
                                                    );
                                            }
                                            Some(RuntimeEnvelopeType::Curve {
                                                ref start,
                                                ref end,
                                                ref duration,
                                                ref curve,
                                            }) => {
                                                let start_val = self
                                                    .eval_signal_at_time(start, event_start_abs);
                                                let end_val =
                                                    self.eval_signal_at_time(end, event_start_abs);
                                                let duration_val = self
                                                    .eval_signal_at_time(duration, event_start_abs)
                                                    .max(0.001);
                                                let curve_val = self
                                                    .eval_signal_at_time(curve, event_start_abs);
                                                self.voice_manager
                                                    .borrow_mut()
                                                    .trigger_sample_with_curve(
                                                        sliced_sample_data.clone(),
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
                                    }

                                    // Configure unit mode and loop for this voice
                                    self.voice_manager
                                        .borrow_mut()
                                        .set_last_voice_unit_mode(unit_mode_enum);
                                    self.voice_manager
                                        .borrow_mut()
                                        .set_last_voice_loop_enabled(loop_enabled_bool);
                                }
                            }
                        } // End chord loop

                        // Track trigger time once per event (not per chord note)
                        if event_start_abs > latest_triggered_start {
                            latest_triggered_start = event_start_abs;
                        }
                    }
                }

                // Update last_trigger_time and last_cycle
                // This ensures we don't re-trigger the same events
                // IMPORTANT: Only update when we actually triggered a new event
                // The old condition `|| cycle_changed` caused duplicate triggers
                if latest_triggered_start > last_event_start {
                    // DEBUG: Log update
                    if std::env::var("DEBUG_SAMPLE_EVENTS").is_ok() && self.sample_count < 20 {
                        eprintln!(
                            "  Updating last_trigger_time: {:.6} -> {:.6}",
                            last_event_start, latest_triggered_start
                        );
                    }
                    if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                        let node = Rc::make_mut(node_rc);
                        if let SignalNode::Sample {
                            last_trigger_time: lt,
                            last_cycle: lc,
                            ..
                        } = node
                        {
                            *lt = latest_triggered_start as f32;
                            *lc = current_cycle;
                        }
                    }
                }

                // Sample nodes trigger voices AND return their cached voice output
                // The voice manager was processed ONCE at the start of process_sample()
                // Each Sample node returns only its own voice mix (by node ID)
                // This allows multiple outputs to have independent sample streams
                //
                // OPTIMIZED: Use Vec-based voice_buffers for O(1) lookup
                // voice_output_cache is only used for newly triggered voices within this buffer
                let buffer_output = self.voice_buffers.get(node_id.0, self.current_sample_idx);
                let newly_triggered = self.voice_output_cache.get(&node_id.0).copied().unwrap_or(0.0);
                let output = buffer_output + newly_triggered;

                // Debug for samples 520-530 (second buffer, after synthesis should be mixed)
                if std::env::var("DEBUG_VOICE_CACHE").is_ok()
                    && self.sample_count >= 520
                    && self.sample_count < 530
                {
                    eprintln!(
                        "[VOICE_CACHE] sample_count={}, Sample node {} reading: buf={:.6} + new={:.6} = {:.6}",
                        self.sample_count, node_id.0, buffer_output, newly_triggered, output
                    );
                }
                output
            }

            SignalNode::SynthPattern {
                pattern,
                last_trigger_time,
                waveform,
                attack,
                decay,
                sustain,
                release,
                filter_cutoff,
                filter_resonance,
                filter_env_amount,
                gain,
                pan,
                ..
            } => {
                use crate::pattern_tonal::{midi_to_freq, note_to_midi};
                use crate::synth_voice_manager::{ADSRParams, FilterParams, SynthWaveform};

                // Evaluate DSP parameters (all pattern-modulatable at sample rate)
                let gain_val = self.eval_signal(&gain).max(0.0).min(10.0);
                let pan_val = self.eval_signal(&pan).clamp(-1.0, 1.0);

                // Evaluate envelope parameters (sampled at trigger time for each note)
                let attack_val = self.eval_signal(&attack).max(0.0001);
                let decay_val = self.eval_signal(&decay).max(0.0);
                let sustain_val = self.eval_signal(&sustain).clamp(0.0, 1.0);
                let release_val = self.eval_signal(&release).max(0.0001);

                // Evaluate filter parameters (sampled at trigger time for each note)
                let filter_cutoff_val = self.eval_signal(&filter_cutoff).max(20.0).min(20000.0);
                let filter_resonance_val = self.eval_signal(&filter_resonance).clamp(0.0, 1.0);
                let filter_env_amount_val = self.eval_signal(&filter_env_amount);

                // Query pattern for note events
                let sample_width = 1.0 / self.sample_rate as f64 / self.cps as f64;
                let state = State {
                    span: TimeSpan::new(
                        Fraction::from_float(self.get_cycle_position()),
                        Fraction::from_float(self.get_cycle_position() + sample_width),
                    ),
                    controls: HashMap::new(),
                };
                let events = pattern.query(&state);

                // Get last event start time
                let last_event_start = if let Some(Some(node)) = self.nodes.get(node_id.0) {
                    if let SignalNode::SynthPattern {
                        last_trigger_time: lt,
                        ..
                    } = &**node
                    {
                        *lt as f64
                    } else {
                        -1.0
                    }
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
                        // Expand chord notation to multiple MIDI notes
                        use crate::pattern_tonal::note_to_midi_chord;
                        let midi_notes = note_to_midi_chord(note_name);

                        // Convert Waveform to SynthWaveform (once for all chord notes)
                        let synth_waveform = match waveform {
                            Waveform::Sine => SynthWaveform::Sine,
                            Waveform::Saw => SynthWaveform::Saw,
                            Waveform::Square => SynthWaveform::Square,
                            Waveform::Triangle => SynthWaveform::Triangle,
                        };

                        // ADSR parameters (evaluated at trigger time - pattern modulatable)
                        let adsr = ADSRParams {
                            attack: attack_val,
                            decay: decay_val,
                            sustain: sustain_val,
                            release: release_val,
                        };

                        // Filter parameters (evaluated at trigger time - pattern modulatable)
                        // Enable filter if cutoff is below Nyquist-ish OR if there's envelope modulation
                        let filter = FilterParams {
                            cutoff: filter_cutoff_val,
                            resonance: filter_resonance_val,
                            env_amount: filter_env_amount_val,
                            enabled: filter_cutoff_val < 19000.0 || filter_env_amount_val != 0.0,
                        };

                        // TRIGGER VOICES FOR EACH NOTE IN CHORD
                        // For chords like "c4'maj", this triggers C, E, G simultaneously
                        // Just like stacking samples!

                        // Scale gain by 1/sqrt(n) to prevent clipping when multiple voices sum
                        // Using sqrt gives perceptually correct loudness (RMS scaling)
                        let chord_size = midi_notes.len();
                        let chord_gain_scale = if chord_size > 1 {
                            1.0 / (chord_size as f32).sqrt()
                        } else {
                            1.0
                        };
                        let scaled_gain = gain_val * chord_gain_scale;

                        for midi_note in midi_notes {
                            let frequency = midi_to_freq(midi_note) as f32;

                            self.synth_voice_manager.borrow_mut().trigger_note(
                                frequency,
                                synth_waveform,
                                adsr,
                                filter,
                                scaled_gain,
                                pan_val,
                            );
                        }

                        // Track latest event
                        if event_start_abs > latest_triggered_start {
                            latest_triggered_start = event_start_abs;
                        }
                    }
                }

                // Update last_trigger_time
                if latest_triggered_start > last_event_start {
                    if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                        let node = Rc::make_mut(node_rc);
                        if let SignalNode::SynthPattern {
                            last_trigger_time: lt,
                            ..
                        } = node
                        {
                            *lt = latest_triggered_start as f32;
                        }
                    }
                }

                // Output mixed audio from all synth voices
                self.synth_voice_manager.borrow_mut().process()
            }

            SignalNode::MidiSynth {
                waveform,
                attack,
                decay,
                sustain,
                release,
                filter_cutoff,
                filter_resonance,
                channel,
                event_queue,
                note_to_voice,
                gain,
            } => {
                use crate::midi_input::MidiMessageType;
                use crate::synth_voice_manager::{ADSRParams, FilterParams, SynthWaveform};

                // Evaluate all parameters (pattern-modulatable at sample rate)
                let gain_val = self.eval_signal(gain).max(0.0).min(10.0);
                let attack_val = self.eval_signal(&attack).max(0.0001);
                let decay_val = self.eval_signal(&decay).max(0.0);
                let sustain_val = self.eval_signal(&sustain).clamp(0.0, 1.0);
                let release_val = self.eval_signal(&release).max(0.0001);
                let filter_cutoff_val = self.eval_signal(&filter_cutoff).max(20.0).min(20000.0);
                let filter_resonance_val = self.eval_signal(&filter_resonance).clamp(0.0, 1.0);

                // Process MIDI events from queue
                if let Ok(mut queue) = event_queue.lock() {
                    while let Some(event) = queue.pop_front() {
                        // Filter by channel if specified
                        if let Some(ch) = channel {
                            if event.channel != *ch {
                                continue;
                            }
                        }

                        match event.message_type {
                            MidiMessageType::NoteOn { note, velocity } if velocity > 0 => {
                                // Note on: trigger a new voice
                                let freq = midi_note_to_freq(note);
                                let vel_gain = velocity as f32 / 127.0;

                                let synth_waveform = match waveform {
                                    Waveform::Sine => SynthWaveform::Sine,
                                    Waveform::Saw => SynthWaveform::Saw,
                                    Waveform::Square => SynthWaveform::Square,
                                    Waveform::Triangle => SynthWaveform::Triangle,
                                };

                                // ADSR parameters (evaluated at trigger time - pattern modulatable)
                                let adsr_params = ADSRParams {
                                    attack: attack_val,
                                    decay: decay_val,
                                    sustain: sustain_val,
                                    release: release_val,
                                };

                                // Filter parameters (evaluated at trigger time - pattern modulatable)
                                let filter_params = FilterParams {
                                    cutoff: filter_cutoff_val,
                                    resonance: filter_resonance_val,
                                    env_amount: 0.0,
                                    enabled: filter_cutoff_val < 19000.0,
                                };

                                // Trigger voice and track note->voice mapping
                                let mut manager = self.synth_voice_manager.borrow_mut();
                                manager.trigger_note(
                                    freq,
                                    synth_waveform,
                                    adsr_params,
                                    filter_params,
                                    gain_val * vel_gain,
                                    0.0, // pan
                                );
                                // Note: We don't track voice index since SynthVoiceManager
                                // handles voice allocation internally. For proper release,
                                // we'd need to extend SynthVoiceManager to return voice index.
                            }
                            MidiMessageType::NoteOff { note, .. }
                            | MidiMessageType::NoteOn { note, velocity: 0 } => {
                                // Note off: release voices playing this note
                                // For now, we release all voices since we can't track
                                // which voice is playing which note without extending SynthVoiceManager
                                // TODO: Add note-based release to SynthVoiceManager
                                let freq = midi_note_to_freq(note);
                                let mut manager = self.synth_voice_manager.borrow_mut();
                                manager.release_note(freq);
                            }
                            _ => {} // Ignore other MIDI messages
                        }
                    }
                }

                // Output mixed audio from all synth voices
                self.synth_voice_manager.borrow_mut().process()
            }

            SignalNode::MidiVoiceFreq => {
                // Return the current voice's frequency from context
                // This is set by MidiPolySynth when evaluating each voice
                self.current_voice_frequency.get().unwrap_or(0.0)
            }

            SignalNode::MidiVoiceGate => {
                // Return the current voice's gate value from context
                // 1.0 when note is held, 0.0 after release
                self.current_voice_gate.get().unwrap_or(0.0)
            }

            SignalNode::MidiPolySynth {
                waveform,
                attack,
                release,
                voices,
                event_queue,
                note_to_voice,
                channel,
                scale_root,
                scale_type,
                arpeggiator,
            } => {
                use crate::midi_input::MidiMessageType;

                // Evaluate attack and release (pattern-modulatable at sample rate)
                let attack_val = self.eval_signal(&attack).max(0.0001);
                let release_val = self.eval_signal(&release).max(0.0001);

                let sample_rate = self.sample_rate;
                let attack_rate = if attack_val > 0.0 { 1.0 / (attack_val * sample_rate) } else { 1.0 };
                let release_rate = if release_val > 0.0 { 1.0 / (release_val * sample_rate) } else { 1.0 };
                let release_samples = (release_val * sample_rate) as u32;

                // Helper to trigger a voice
                let trigger_voice = |note: u8, voices: &RefCell<Vec<MidiPolyVoice>>, note_map: &RefCell<HashMap<u8, usize>>, scale_root: &Option<u8>, scale_type: &Option<Scale>| {
                    // Apply scale locking if configured
                    let effective_note = if let (Some(root), Some(scale)) = (scale_root, scale_type) {
                        scale_lock(note, *root, *scale)
                    } else {
                        note
                    };

                    let freq = midi_note_to_freq(effective_note);
                    let mut voices_mut = voices.borrow_mut();
                    let mut note_map_mut = note_map.borrow_mut();

                    // Find an inactive voice or create new one (no stealing!)
                    let voice_idx = if let Some(free_idx) = voices_mut.iter().position(|v| !v.active) {
                        free_idx
                    } else {
                        voices_mut.push(MidiPolyVoice::new());
                        voices_mut.len() - 1
                    };

                    voices_mut[voice_idx].trigger(effective_note, freq);
                    note_map_mut.insert(note, voice_idx);
                };

                // Helper to release a voice
                let release_voice = |note: u8, voices: &RefCell<Vec<MidiPolyVoice>>, note_map: &RefCell<HashMap<u8, usize>>, release_samples: u32| {
                    let note_map_ref = note_map.borrow();
                    if let Some(&voice_idx) = note_map_ref.get(&note) {
                        drop(note_map_ref);
                        let mut voices_mut = voices.borrow_mut();
                        if voice_idx < voices_mut.len() {
                            voices_mut[voice_idx].release();
                            voices_mut[voice_idx].release_samples_remaining = release_samples;
                        }
                    }
                };

                // Check if arpeggiator is enabled
                let arp_enabled = arpeggiator.borrow().is_some();

                // Process MIDI events from queue
                if let Ok(mut queue) = event_queue.lock() {
                    while let Some(event) = queue.pop_front() {
                        // Filter by channel if specified
                        if let Some(ch) = channel {
                            if event.channel != *ch {
                                continue;
                            }
                        }

                        match event.message_type {
                            MidiMessageType::NoteOn { note, velocity } if velocity > 0 => {
                                if arp_enabled {
                                    // Route to arpeggiator
                                    if let Some(ref mut arp) = *arpeggiator.borrow_mut() {
                                        arp.note_on(note);
                                    }
                                } else {
                                    // Direct trigger
                                    trigger_voice(note, voices, note_to_voice, scale_root, scale_type);
                                }
                            }
                            MidiMessageType::NoteOff { note, .. }
                            | MidiMessageType::NoteOn { note, velocity: 0 } => {
                                if arp_enabled {
                                    // Route to arpeggiator
                                    if let Some(ref mut arp) = *arpeggiator.borrow_mut() {
                                        arp.note_off(note);
                                    }
                                } else {
                                    // Direct release
                                    release_voice(note, voices, note_to_voice, release_samples);
                                }
                            }
                            _ => {} // Ignore other MIDI messages
                        }
                    }
                }

                // Process arpeggiator output (if enabled)
                if let Some(ref mut arp) = *arpeggiator.borrow_mut() {
                    let (note_on, note_off) = arp.process_sample();

                    // Handle arp note off first
                    if let Some(off_note) = note_off {
                        let note_map_ref = note_to_voice.borrow();
                        if let Some(&voice_idx) = note_map_ref.get(&off_note) {
                            drop(note_map_ref);
                            let mut voices_mut = voices.borrow_mut();
                            if voice_idx < voices_mut.len() {
                                voices_mut[voice_idx].release();
                                voices_mut[voice_idx].release_samples_remaining = release_samples;
                            }
                        }
                    }

                    // Handle arp note on
                    if let Some(on_note) = note_on {
                        // Apply scale locking if configured
                        let effective_note = if let (Some(root), Some(scale)) = (scale_root, scale_type) {
                            scale_lock(on_note, *root, *scale)
                        } else {
                            on_note
                        };

                        let freq = midi_note_to_freq(effective_note);
                        let mut voices_mut = voices.borrow_mut();
                        let mut note_map_mut = note_to_voice.borrow_mut();

                        // Find an inactive voice or create new one
                        let voice_idx = if let Some(free_idx) = voices_mut.iter().position(|v| !v.active) {
                            free_idx
                        } else {
                            voices_mut.push(MidiPolyVoice::new());
                            voices_mut.len() - 1
                        };

                        voices_mut[voice_idx].trigger(effective_note, freq);
                        note_map_mut.insert(on_note, voice_idx);
                    }
                }

                // Sum output from all active voices with per-voice oscillator + envelope
                let mut output = 0.0f32;
                let mut active_count = 0usize;

                {
                    let mut voices_mut = voices.borrow_mut();

                    for voice in voices_mut.iter_mut() {
                        if !voice.active {
                            continue;
                        }

                        // Update envelope (simple ASR)
                        if voice.gate > 0.0 {
                            // Attack phase
                            voice.envelope_level = (voice.envelope_level + attack_rate).min(1.0);
                        } else {
                            // Release phase
                            voice.envelope_level = (voice.envelope_level - release_rate).max(0.0);
                            if voice.release_samples_remaining > 0 {
                                voice.release_samples_remaining -= 1;
                            }
                            if voice.envelope_level <= 0.0 {
                                voice.active = false;
                                continue;
                            }
                        }

                        // Compute oscillator with per-voice phase
                        let phase_inc = voice.frequency / sample_rate;
                        voice.phase += phase_inc;
                        if voice.phase >= 1.0 {
                            voice.phase -= 1.0;
                        }

                        let osc_out = match waveform {
                            Waveform::Sine => (voice.phase * std::f32::consts::TAU).sin(),
                            Waveform::Saw => 2.0 * voice.phase - 1.0,
                            Waveform::Square => if voice.phase < 0.5 { 1.0 } else { -1.0 },
                            Waveform::Triangle => {
                                if voice.phase < 0.5 {
                                    4.0 * voice.phase - 1.0
                                } else {
                                    3.0 - 4.0 * voice.phase
                                }
                            }
                        };

                        // Apply envelope and sum
                        output += osc_out * voice.envelope_level;
                        active_count += 1;
                    }
                } // voices_mut borrow dropped here

                // Clean up note map for deactivated voices
                {
                    let voices_ref = voices.borrow();
                    note_to_voice.borrow_mut().retain(|_, &mut idx| {
                        idx < voices_ref.len() && voices_ref[idx].active
                    });
                }

                // Scale output for polyphony (soft knee)
                if active_count > 1 {
                    output /= (active_count as f32).sqrt();
                }

                output
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
                        Fraction::from_float(self.get_cycle_position()),
                        Fraction::from_float(self.get_cycle_position() + sample_width),
                    ),
                    controls: HashMap::new(),
                };

                let events = pattern.query(&state);
                let mut current_value = *last_value; // Default to last value

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
                                let midi_note = *root_note as i32 + octave * 12 + interval;

                                // Convert to frequency
                                current_value = midi_to_freq(midi_note.clamp(0, 127) as u8) as f32;

                                // Update last_value for next time
                                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                                    let node = Rc::make_mut(node_rc);
                                    if let SignalNode::ScaleQuantize { last_value: lv, .. } = node {
                                        *lv = current_value;
                                    }
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
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Noise { seed: s } = node {
                        *s = next;
                    }
                }

                (next as f32 / (1 << 30) as f32) - 1.0
            }

            SignalNode::HighPass {
                input, cutoff, q, ..
            } => {
                let input_val = self.eval_signal(&input);
                let fc = self.eval_signal(&cutoff).max(20.0).min(20000.0);
                let q_val = self.eval_signal(&q).max(0.5).min(20.0);

                // Get state and cached coefficients
                let (mut low, mut band, mut high, mut f, mut damp) =
                    if let Some(Some(node)) = self.nodes.get(node_id.0) {
                        if let SignalNode::HighPass { state, .. } = &**node {
                            (
                                state.y1,
                                state.x1,
                                state.y2,
                                state.cached_f,
                                state.cached_damp,
                            )
                        } else {
                            (0.0, 0.0, 0.0, 0.0, 1.0)
                        }
                    } else {
                        (0.0, 0.0, 0.0, 0.0, 1.0)
                    };

                // Only recompute coefficients if parameters changed (OPTIMIZATION!)
                let params_changed = if let Some(Some(node)) = self.nodes.get(node_id.0) {
                    if let SignalNode::HighPass { state, .. } = &**node {
                        (fc - state.cached_fc).abs() > 0.1 || (q_val - state.cached_q).abs() > 0.001
                    } else {
                        true
                    }
                } else {
                    true
                };

                if params_changed {
                    // State variable filter (Chamberlin) - recompute only when needed
                    f = 2.0 * (PI * fc / self.sample_rate).sin();
                    damp = 1.0 / q_val;
                }

                // Process filter
                high = input_val - low - damp * band;
                band += f * high;
                low += f * band;

                // Update state and cache coefficients
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::HighPass { state, .. } = node {
                        state.y1 = low;
                        state.x1 = band;
                        state.y2 = high;
                        if params_changed {
                            state.cached_fc = fc;
                            state.cached_q = q_val;
                            state.cached_f = f;
                            state.cached_damp = damp;
                        }
                    }
                }

                high // Output high-pass signal
            }

            SignalNode::BandPass {
                input, center, q, ..
            } => {
                let input_val = self.eval_signal(&input);
                let fc = self.eval_signal(&center).max(20.0).min(20000.0);
                let q_val = self.eval_signal(&q).max(0.5).min(20.0);

                // Get state and cached coefficients
                let (mut low, mut band, mut high, mut f, mut damp) =
                    if let Some(Some(node)) = self.nodes.get(node_id.0) {
                        if let SignalNode::BandPass { state, .. } = &**node {
                            (
                                state.y1,
                                state.x1,
                                state.y2,
                                state.cached_f,
                                state.cached_damp,
                            )
                        } else {
                            (0.0, 0.0, 0.0, 0.0, 1.0)
                        }
                    } else {
                        (0.0, 0.0, 0.0, 0.0, 1.0)
                    };

                // OPTIMIZATION: Only recompute coefficients if parameters changed
                let params_changed = if let Some(Some(node)) = self.nodes.get(node_id.0) {
                    if let SignalNode::BandPass { state, .. } = &**node {
                        (fc - state.cached_fc).abs() > 0.1 || (q_val - state.cached_q).abs() > 0.001
                    } else {
                        true
                    }
                } else {
                    true
                };

                if params_changed {
                    // State variable filter (Chamberlin) - recompute coefficients
                    f = 2.0 * (PI * fc / self.sample_rate).sin();
                    damp = 1.0 / q_val;
                }

                // Process filter
                high = input_val - low - damp * band;
                band += f * high;
                low += f * band;

                // Update state and cache coefficients
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::BandPass { state, .. } = node {
                        state.y1 = low;
                        state.x1 = band;
                        state.y2 = high;
                        if params_changed {
                            state.cached_fc = fc;
                            state.cached_q = q_val;
                            state.cached_f = f;
                            state.cached_damp = damp;
                        }
                    }
                }

                band // Output band-pass signal
            }

            SignalNode::DJFilter { input, value, .. } => {
                let input_val = self.eval_signal(&input);
                let djf_value = self.eval_signal(&value).clamp(0.0, 1.0);

                // Map djf value to filter cutoff frequency
                // Keep cutoff well below Nyquist (22050 Hz) to avoid instability
                // 0.0 -> very low cutoff (80 Hz) for aggressive lowpass
                // 0.5 -> mid cutoff (800 Hz) - neutral point
                // 1.0 -> high cutoff (8000 Hz) for aggressive highpass
                let cutoff = if djf_value < 0.5 {
                    // Lowpass mode: map 0-0.5 to 80-800 Hz
                    80.0 + (djf_value * 2.0) * 720.0
                } else {
                    // Highpass mode: map 0.5-1.0 to 800-8000 Hz
                    800.0 + ((djf_value - 0.5) * 2.0) * 7200.0
                };

                // Clamp cutoff to safe range to prevent filter instability
                let cutoff = cutoff.max(20.0).min(self.sample_rate * 0.4);
                // Use Q=1.0 for stability at high frequencies (Q=0.707 causes instability)
                let q_val = 1.0;

                // State variable filter (Chamberlin)
                let f = (2.0 * (PI * cutoff / self.sample_rate).sin()).min(1.9);
                let damp = 1.0 / q_val;

                // Get state
                let (mut low, mut band, mut high) =
                    if let Some(Some(node)) = self.nodes.get(node_id.0) {
                        if let SignalNode::DJFilter { state, .. } = &**node {
                            (state.y1, state.x1, state.y2)
                        } else {
                            (0.0, 0.0, 0.0)
                        }
                    } else {
                        (0.0, 0.0, 0.0)
                    };

                // Process
                high = input_val - low - damp * band;
                band += f * high;
                low += f * band;

                // Flush denormals to zero to prevent numerical instability
                const DENORMAL_THRESHOLD: f32 = 1e-30;
                if high.abs() < DENORMAL_THRESHOLD {
                    high = 0.0;
                }
                if band.abs() < DENORMAL_THRESHOLD {
                    band = 0.0;
                }
                if low.abs() < DENORMAL_THRESHOLD {
                    low = 0.0;
                }

                // Update state with sanitized values
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::DJFilter { state, .. } = node {
                        state.y1 = if low.is_finite() { low } else { 0.0 };
                        state.x1 = if band.is_finite() { band } else { 0.0 };
                        state.y2 = if high.is_finite() { high } else { 0.0 };
                    }
                }

                // Output selection: lowpass for < 0.5, highpass for > 0.5
                let output = if djf_value < 0.5 {
                    low // Lowpass output
                } else {
                    high // Highpass output
                };

                // Ensure output is finite
                if output.is_finite() {
                    output
                } else {
                    0.0
                }
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
                    if let Some(Some(node)) = self.nodes.get(node_id.0) {
                        if let SignalNode::Notch { state, .. } = &**node {
                            (state.y1, state.x1, state.y2)
                        } else {
                            (0.0, 0.0, 0.0)
                        }
                    } else {
                        (0.0, 0.0, 0.0)
                    };

                // Process
                high = input_val - low - damp * band;
                band += f * high;
                low += f * band;

                // Update state
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Notch { state, .. } = node {
                        state.y1 = low;
                        state.x1 = band;
                        state.y2 = high;
                    }
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

                // BYPASS MODE: For pipelined rendering, pass through unchanged
                if self.bypass_sequential_effects {
                    return input_val;
                }

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
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Comb {
                        buffer: buf,
                        write_pos: idx,
                        ..
                    } = node
                    {
                        buf[*idx] = output;
                        *idx = (*idx + 1) % buf.len();
                    }
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
                let (s1, s2, s3, s4) = if let Some(Some(node)) = self.nodes.get(node_id.0) {
                    if let SignalNode::MoogLadder { state, .. } = &**node {
                        (state.stage1, state.stage2, state.stage3, state.stage4)
                    } else {
                        (0.0, 0.0, 0.0, 0.0)
                    }
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
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::MoogLadder { state, .. } = node {
                        state.stage1 = stage1_new;
                        state.stage2 = stage2_new;
                        state.stage3 = stage3_new;
                        state.stage4 = stage4_new;
                    }
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
                    if let Some(Some(node)) = self.nodes.get(node_id.0) {
                        if let SignalNode::ParametricEQ { state, .. } = &**node {
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
                        }
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
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::ParametricEQ { state, .. } = node {
                        state.low_band = new_low_state;
                        state.mid_band = new_mid_state;
                        state.high_band = new_high_state;
                    }
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

                // Evaluate pattern-modulatable parameters
                let attack_val = self.eval_signal(&attack);
                let decay_val = self.eval_signal(&decay);
                let sustain_val = self.eval_signal(&sustain);
                let release_val = self.eval_signal(&release);

                // Work with state in place (no clone needed)
                let mut output_level = *state.level.borrow();

                // Check for trigger
                {
                    let phase = state.phase.borrow();
                    if trig > 0.5 && matches!(*phase, EnvPhase::Idle | EnvPhase::Release) {
                        drop(phase); // Release borrow before mutable borrow
                        *state.phase.borrow_mut() = EnvPhase::Attack;
                        *state.time_in_phase.borrow_mut() = 0.0;
                    } else if trig <= 0.5
                        && matches!(
                            *phase,
                            EnvPhase::Attack | EnvPhase::Decay | EnvPhase::Sustain
                        )
                    {
                        drop(phase); // Release borrow before mutable borrow
                                     // Store current level before entering release phase
                        *state.release_start_level.borrow_mut() = *state.level.borrow();
                        *state.phase.borrow_mut() = EnvPhase::Release;
                        *state.time_in_phase.borrow_mut() = 0.0;
                    }
                }

                // Process envelope
                let dt = 1.0 / self.sample_rate;
                *state.time_in_phase.borrow_mut() += dt;

                // Get current phase (clone to avoid holding borrow during match)
                let current_phase = state.phase.borrow().clone();
                match current_phase {
                    EnvPhase::Attack => {
                        if attack_val > 0.0 {
                            let new_level = *state.time_in_phase.borrow() / attack_val;
                            *state.level.borrow_mut() = new_level;
                            if new_level >= 1.0 {
                                *state.level.borrow_mut() = 1.0;
                                *state.phase.borrow_mut() = EnvPhase::Decay;
                                *state.time_in_phase.borrow_mut() = 0.0;
                            }
                        } else {
                            *state.level.borrow_mut() = 1.0;
                            *state.phase.borrow_mut() = EnvPhase::Decay;
                            *state.time_in_phase.borrow_mut() = 0.0;
                        }
                    }
                    EnvPhase::Decay => {
                        if decay_val > 0.0 {
                            let new_level = 1.0
                                - (1.0 - sustain_val) * (*state.time_in_phase.borrow() / decay_val);
                            *state.level.borrow_mut() = new_level;
                            if new_level <= sustain_val {
                                *state.level.borrow_mut() = sustain_val;
                                *state.phase.borrow_mut() = EnvPhase::Sustain;
                                *state.time_in_phase.borrow_mut() = 0.0;
                            }
                        } else {
                            *state.level.borrow_mut() = sustain_val;
                            *state.phase.borrow_mut() = EnvPhase::Sustain;
                            *state.time_in_phase.borrow_mut() = 0.0;
                        }
                    }
                    EnvPhase::Sustain => {
                        *state.level.borrow_mut() = sustain_val;
                    }
                    EnvPhase::Release => {
                        if release_val > 0.0 {
                            // Linear decay from release_start_level to 0 over release time
                            let progress = (*state.time_in_phase.borrow() / release_val).min(1.0);
                            *state.level.borrow_mut() =
                                *state.release_start_level.borrow() * (1.0 - progress);

                            if progress >= 1.0 {
                                *state.level.borrow_mut() = 0.0;
                                *state.phase.borrow_mut() = EnvPhase::Idle;
                            }
                        } else {
                            *state.level.borrow_mut() = 0.0;
                            *state.phase.borrow_mut() = EnvPhase::Idle;
                        }
                    }
                    EnvPhase::Idle => {
                        *state.level.borrow_mut() = 0.0;
                    }
                }

                output_level = *state.level.borrow();

                input_val * output_level
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
                let cycle_pos = (self.get_cycle_position() % 1.0) as f32;
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
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::ADSR { state: s, .. } = node {
                        *s = adsr_state.clone();
                    }
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
                let cycle_pos = (self.get_cycle_position() % 1.0) as f32;
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
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::AD { state: s, .. } = node {
                        *s = ad_state.clone();
                    }
                }

                ad_state.level
            }

            SignalNode::Line { start, end } => {
                // Evaluate start and end values (supports pattern modulation!)
                let start_val = self.eval_signal(&start);
                let end_val = self.eval_signal(&end);

                // Calculate position within current cycle (0.0 to 1.0)
                let cycle_pos = (self.get_cycle_position() % 1.0) as f32;

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
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Curve {
                        elapsed_time: elapsed,
                        ..
                    } = node
                    {
                        // Increment elapsed time
                        *elapsed += 1.0 / self.sample_rate;

                        // Calculate normalized time (0 to 1)
                        let t = (*elapsed / duration_val).min(1.0);

                        // Apply curve formula
                        // Based on SuperCollider's Env.curve
                        // Negative curve = convex (fast start, slow end)
                        // Positive curve = concave (slow start, fast end)
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
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Segments {
                        levels: seg_levels,
                        times: seg_times,
                        current_segment: seg_idx,
                        segment_elapsed: seg_elapsed,
                        current_value: seg_value,
                    } = node
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

                // Evaluate envelope parameters (pattern-modulatable at sample rate)
                let attack_val = self.eval_signal(&attack).max(0.0);
                let decay_val = self.eval_signal(&decay).max(0.0);
                let sustain_val = self.eval_signal(&sustain).clamp(0.0, 1.0);
                let release_val = self.eval_signal(&release).max(0.0);

                // Query pattern for trigger events
                let sample_width = 1.0 / self.sample_rate as f64 / self.cps as f64;
                let current_cycle = self.get_cycle_position().floor() as i32;

                let query_state = State {
                    span: TimeSpan::new(
                        Fraction::from_float(self.get_cycle_position()),
                        Fraction::from_float(self.get_cycle_position() + sample_width),
                    ),
                    controls: HashMap::new(),
                };
                let events = pattern.query(&query_state);

                // Get last event start time and cycle
                let (last_event_start, prev_cycle) =
                    if let Some(Some(node)) = self.nodes.get(node_id.0) {
                        if let SignalNode::EnvelopePattern {
                            last_trigger_time: lt,
                            last_cycle: lc,
                            ..
                        } = &**node
                        {
                            (*lt as f64, *lc)
                        } else {
                            (-1.0, -1)
                        }
                    } else {
                        (-1.0, -1)
                    };

                // Work with state in place (no clone needed)
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
                    let event_is_new =
                        event_start_abs > last_event_start + tolerance || cycle_changed;

                    if event_is_new && event_start_abs > latest_triggered_start {
                        latest_triggered_start = event_start_abs;
                    }
                }

                // Process envelope based on trigger
                {
                    let phase = state.phase.borrow();
                    if trigger_active && matches!(*phase, EnvPhase::Idle | EnvPhase::Release) {
                        drop(phase);
                        // Start attack phase
                        *state.phase.borrow_mut() = EnvPhase::Attack;
                        *state.time_in_phase.borrow_mut() = 0.0;
                    } else if !trigger_active
                        && matches!(
                            *phase,
                            EnvPhase::Attack | EnvPhase::Decay | EnvPhase::Sustain
                        )
                    {
                        drop(phase);
                        // Enter release phase
                        *state.release_start_level.borrow_mut() = *state.level.borrow();
                        *state.phase.borrow_mut() = EnvPhase::Release;
                        *state.time_in_phase.borrow_mut() = 0.0;
                    }
                }

                // Advance envelope state
                let dt = 1.0 / self.sample_rate;
                *state.time_in_phase.borrow_mut() += dt;

                // Get current phase (clone to avoid holding borrow during match)
                let current_phase = state.phase.borrow().clone();
                match current_phase {
                    EnvPhase::Attack => {
                        if attack_val > 0.0 {
                            let new_level = *state.time_in_phase.borrow() / attack_val;
                            *state.level.borrow_mut() = new_level;
                            if new_level >= 1.0 {
                                *state.level.borrow_mut() = 1.0;
                                *state.phase.borrow_mut() = EnvPhase::Decay;
                                *state.time_in_phase.borrow_mut() = 0.0;
                            }
                        } else {
                            *state.level.borrow_mut() = 1.0;
                            *state.phase.borrow_mut() = EnvPhase::Decay;
                            *state.time_in_phase.borrow_mut() = 0.0;
                        }
                    }
                    EnvPhase::Decay => {
                        if decay_val > 0.0 {
                            let new_level =
                                1.0 - (1.0 - sustain_val) * (*state.time_in_phase.borrow() / decay_val);
                            *state.level.borrow_mut() = new_level;
                            if new_level <= sustain_val {
                                *state.level.borrow_mut() = sustain_val;
                                *state.phase.borrow_mut() = EnvPhase::Sustain;
                                *state.time_in_phase.borrow_mut() = 0.0;
                            }
                        } else {
                            *state.level.borrow_mut() = sustain_val;
                            *state.phase.borrow_mut() = EnvPhase::Sustain;
                            *state.time_in_phase.borrow_mut() = 0.0;
                        }
                    }
                    EnvPhase::Sustain => {
                        *state.level.borrow_mut() = sustain_val;
                    }
                    EnvPhase::Release => {
                        if release_val > 0.0 {
                            let progress = (*state.time_in_phase.borrow() / release_val).min(1.0);
                            *state.level.borrow_mut() =
                                *state.release_start_level.borrow() * (1.0 - progress);

                            if progress >= 1.0 {
                                *state.level.borrow_mut() = 0.0;
                                *state.phase.borrow_mut() = EnvPhase::Idle;
                            }
                        } else {
                            *state.level.borrow_mut() = 0.0;
                            *state.phase.borrow_mut() = EnvPhase::Idle;
                        }
                    }
                    EnvPhase::Idle => {
                        *state.level.borrow_mut() = 0.0;
                    }
                }

                let output_level = *state.level.borrow();

                // Update state in node
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::EnvelopePattern {
                        last_trigger_time: lt,
                        last_cycle: lc,
                        ..
                    } = node
                    {
                        *lt = latest_triggered_start as f32;
                        *lc = current_cycle;
                    }
                }

                // Output: input signal gated by envelope
                input_val * output_level
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

                // Evaluate envelope parameters (pattern-modulatable at sample rate)
                let attack_val = self.eval_signal(&attack).max(0.0);
                let decay_val = self.eval_signal(&decay).max(0.0);
                let sustain_val = self.eval_signal(&sustain).clamp(0.0, 1.0);
                let release_val = self.eval_signal(&release).max(0.0);

                // Query boolean pattern for trigger events
                let sample_width = 1.0 / self.sample_rate as f64 / self.cps as f64;
                let current_cycle = self.get_cycle_position().floor() as i32;

                let query_state = State {
                    span: TimeSpan::new(
                        Fraction::from_float(self.get_cycle_position()),
                        Fraction::from_float(self.get_cycle_position() + sample_width),
                    ),
                    controls: HashMap::new(),
                };
                let events = bool_pattern.query(&query_state);

                // Get last event start time and cycle
                let (last_event_start, prev_cycle) =
                    if let Some(Some(node)) = self.nodes.get(node_id.0) {
                        if let SignalNode::StructuredSignal {
                            last_trigger_time: lt,
                            last_cycle: lc,
                            ..
                        } = &**node
                        {
                            (*lt as f64, *lc)
                        } else {
                            (-1.0, -1)
                        }
                    } else {
                        (-1.0, -1)
                    };

                // Work with state in place (no clone needed)
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
                    let event_is_new =
                        event_start_abs > last_event_start + tolerance || cycle_changed;

                    if event_is_new && event_start_abs > latest_triggered_start {
                        latest_triggered_start = event_start_abs;
                    }
                }

                // Process envelope based on trigger
                {
                    let phase = state.phase.borrow();
                    if trigger_active && matches!(*phase, EnvPhase::Idle | EnvPhase::Release) {
                        drop(phase);
                        // Start attack phase
                        *state.phase.borrow_mut() = EnvPhase::Attack;
                        *state.time_in_phase.borrow_mut() = 0.0;
                    } else if !trigger_active
                        && matches!(
                            *phase,
                            EnvPhase::Attack | EnvPhase::Decay | EnvPhase::Sustain
                        )
                    {
                        drop(phase);
                        // Enter release phase
                        *state.release_start_level.borrow_mut() = *state.level.borrow();
                        *state.phase.borrow_mut() = EnvPhase::Release;
                        *state.time_in_phase.borrow_mut() = 0.0;
                    }
                }

                // Advance envelope state
                let dt = 1.0 / self.sample_rate;
                *state.time_in_phase.borrow_mut() += dt;

                // Get current phase (clone to avoid holding borrow during match)
                let current_phase = state.phase.borrow().clone();
                match current_phase {
                    EnvPhase::Attack => {
                        if attack_val > 0.0 {
                            let new_level = *state.time_in_phase.borrow() / attack_val;
                            *state.level.borrow_mut() = new_level;
                            if new_level >= 1.0 {
                                *state.level.borrow_mut() = 1.0;
                                *state.phase.borrow_mut() = EnvPhase::Decay;
                                *state.time_in_phase.borrow_mut() = 0.0;
                            }
                        } else {
                            *state.level.borrow_mut() = 1.0;
                            *state.phase.borrow_mut() = EnvPhase::Decay;
                            *state.time_in_phase.borrow_mut() = 0.0;
                        }
                    }
                    EnvPhase::Decay => {
                        if decay_val > 0.0 {
                            let new_level =
                                1.0 - (1.0 - sustain_val) * (*state.time_in_phase.borrow() / decay_val);
                            *state.level.borrow_mut() = new_level;
                            if new_level <= sustain_val {
                                *state.level.borrow_mut() = sustain_val;
                                *state.phase.borrow_mut() = EnvPhase::Sustain;
                                *state.time_in_phase.borrow_mut() = 0.0;
                            }
                        } else {
                            *state.level.borrow_mut() = sustain_val;
                            *state.phase.borrow_mut() = EnvPhase::Sustain;
                            *state.time_in_phase.borrow_mut() = 0.0;
                        }
                    }
                    EnvPhase::Sustain => {
                        *state.level.borrow_mut() = sustain_val;
                    }
                    EnvPhase::Release => {
                        if release_val > 0.0 {
                            let progress = (*state.time_in_phase.borrow() / release_val).min(1.0);
                            *state.level.borrow_mut() =
                                *state.release_start_level.borrow() * (1.0 - progress);

                            if progress >= 1.0 {
                                *state.level.borrow_mut() = 0.0;
                                *state.phase.borrow_mut() = EnvPhase::Idle;
                            }
                        } else {
                            *state.level.borrow_mut() = 0.0;
                            *state.phase.borrow_mut() = EnvPhase::Idle;
                        }
                    }
                    EnvPhase::Idle => {
                        *state.level.borrow_mut() = 0.0;
                    }
                }

                let output_level = *state.level.borrow();

                // Update state in node
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::StructuredSignal {
                        last_trigger_time: lt,
                        last_cycle: lc,
                        ..
                    } = node
                    {
                        *lt = latest_triggered_start as f32;
                        *lc = current_cycle;
                    }
                }

                // Output: input signal gated by envelope
                input_val * output_level
            }

            SignalNode::TriggeredAR {
                pattern,
                attack,
                release,
                last_trigger_time,
                last_cycle,
                state,
                ..
            } => {
                let attack_val = self.eval_signal(&attack).max(0.001);
                let release_val = self.eval_signal(&release).max(0.001);

                // Query boolean pattern for trigger events
                let sample_width = 1.0 / self.sample_rate as f64 / self.cps as f64;
                let current_cycle = self.get_cycle_position().floor() as i32;

                let query_state = State {
                    span: TimeSpan::new(
                        Fraction::from_float(self.get_cycle_position()),
                        Fraction::from_float(self.get_cycle_position() + sample_width),
                    ),
                    controls: HashMap::new(),
                };
                let events = pattern.query(&query_state);

                // Get last event start time
                let (last_event_start, prev_cycle) =
                    if let Some(Some(node)) = self.nodes.get(node_id.0) {
                        if let SignalNode::TriggeredAR {
                            last_trigger_time: lt,
                            last_cycle: lc,
                            ..
                        } = &**node
                        {
                            (*lt as f64, *lc)
                        } else {
                            (-1.0, -1)
                        }
                    } else {
                        (-1.0, -1)
                    };

                let mut latest_triggered_start = last_event_start;
                let mut new_trigger = false;
                let cycle_changed = current_cycle != prev_cycle;

                // Check for new trigger events
                for event in events.iter() {
                    if !event.value {
                        continue;
                    }

                    let event_start_abs = if let Some(whole) = &event.whole {
                        whole.begin.to_float()
                    } else {
                        event.part.begin.to_float()
                    };

                    let tolerance = sample_width * 0.001;
                    let event_is_new =
                        event_start_abs > last_event_start + tolerance || cycle_changed;

                    if event_is_new && event_start_abs > latest_triggered_start {
                        latest_triggered_start = event_start_abs;
                        new_trigger = true;
                    }
                }

                // On new trigger, restart envelope from attack phase
                if new_trigger {
                    *state.phase.borrow_mut() = EnvPhase::Attack;
                    *state.time_in_phase.borrow_mut() = 0.0;
                }

                // Advance envelope
                let dt = 1.0 / self.sample_rate;
                *state.time_in_phase.borrow_mut() += dt;

                let current_phase = state.phase.borrow().clone();
                match current_phase {
                    EnvPhase::Attack => {
                        let new_level = *state.time_in_phase.borrow() / attack_val;
                        *state.level.borrow_mut() = new_level.min(1.0);
                        if new_level >= 1.0 {
                            *state.phase.borrow_mut() = EnvPhase::Release;
                            *state.time_in_phase.borrow_mut() = 0.0;
                            *state.release_start_level.borrow_mut() = 1.0;
                        }
                    }
                    EnvPhase::Release => {
                        let progress = (*state.time_in_phase.borrow() / release_val).min(1.0);
                        *state.level.borrow_mut() =
                            *state.release_start_level.borrow() * (1.0 - progress);
                        if progress >= 1.0 {
                            *state.level.borrow_mut() = 0.0;
                            *state.phase.borrow_mut() = EnvPhase::Idle;
                        }
                    }
                    _ => {
                        *state.level.borrow_mut() = 0.0;
                    }
                }

                let output = *state.level.borrow();

                // Update state
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::TriggeredAR {
                        last_trigger_time: lt,
                        last_cycle: lc,
                        ..
                    } = node
                    {
                        *lt = latest_triggered_start as f32;
                        *lc = current_cycle;
                    }
                }

                output
            }

            SignalNode::TriggeredADSR {
                pattern,
                attack,
                decay,
                sustain,
                release,
                last_trigger_time,
                last_cycle,
                state,
                ..
            } => {
                let attack_val = self.eval_signal(&attack).max(0.001);
                let decay_val = self.eval_signal(&decay).max(0.001);
                let sustain_val = self.eval_signal(&sustain).clamp(0.0, 1.0);
                let release_val = self.eval_signal(&release).max(0.001);

                // Query boolean pattern
                let sample_width = 1.0 / self.sample_rate as f64 / self.cps as f64;
                let current_cycle = self.get_cycle_position().floor() as i32;

                let query_state = State {
                    span: TimeSpan::new(
                        Fraction::from_float(self.get_cycle_position()),
                        Fraction::from_float(self.get_cycle_position() + sample_width),
                    ),
                    controls: HashMap::new(),
                };
                let events = pattern.query(&query_state);

                let (last_event_start, prev_cycle) =
                    if let Some(Some(node)) = self.nodes.get(node_id.0) {
                        if let SignalNode::TriggeredADSR {
                            last_trigger_time: lt,
                            last_cycle: lc,
                            ..
                        } = &**node
                        {
                            (*lt as f64, *lc)
                        } else {
                            (-1.0, -1)
                        }
                    } else {
                        (-1.0, -1)
                    };

                let mut latest_triggered_start = last_event_start;
                let mut trigger_active = false;
                let mut new_trigger = false;
                let cycle_changed = current_cycle != prev_cycle;

                for event in events.iter() {
                    if !event.value {
                        continue;
                    }

                    trigger_active = true;

                    let event_start_abs = if let Some(whole) = &event.whole {
                        whole.begin.to_float()
                    } else {
                        event.part.begin.to_float()
                    };

                    let tolerance = sample_width * 0.001;
                    let event_is_new =
                        event_start_abs > last_event_start + tolerance || cycle_changed;

                    if event_is_new && event_start_abs > latest_triggered_start {
                        latest_triggered_start = event_start_abs;
                        new_trigger = true;
                    }
                }

                // Start attack on new trigger
                if new_trigger {
                    *state.phase.borrow_mut() = EnvPhase::Attack;
                    *state.time_in_phase.borrow_mut() = 0.0;
                }

                // Handle gate-off (release phase)
                {
                    let phase = state.phase.borrow();
                    if !trigger_active
                        && matches!(
                            *phase,
                            EnvPhase::Attack | EnvPhase::Decay | EnvPhase::Sustain
                        )
                    {
                        drop(phase);
                        *state.release_start_level.borrow_mut() = *state.level.borrow();
                        *state.phase.borrow_mut() = EnvPhase::Release;
                        *state.time_in_phase.borrow_mut() = 0.0;
                    }
                }

                // Advance envelope
                let dt = 1.0 / self.sample_rate;
                *state.time_in_phase.borrow_mut() += dt;

                let current_phase = state.phase.borrow().clone();
                match current_phase {
                    EnvPhase::Attack => {
                        let new_level = *state.time_in_phase.borrow() / attack_val;
                        *state.level.borrow_mut() = new_level.min(1.0);
                        if new_level >= 1.0 {
                            *state.phase.borrow_mut() = EnvPhase::Decay;
                            *state.time_in_phase.borrow_mut() = 0.0;
                        }
                    }
                    EnvPhase::Decay => {
                        let new_level =
                            1.0 - (1.0 - sustain_val) * (*state.time_in_phase.borrow() / decay_val);
                        *state.level.borrow_mut() = new_level.max(sustain_val);
                        if new_level <= sustain_val {
                            *state.phase.borrow_mut() = EnvPhase::Sustain;
                        }
                    }
                    EnvPhase::Sustain => {
                        *state.level.borrow_mut() = sustain_val;
                    }
                    EnvPhase::Release => {
                        let progress = (*state.time_in_phase.borrow() / release_val).min(1.0);
                        *state.level.borrow_mut() =
                            *state.release_start_level.borrow() * (1.0 - progress);
                        if progress >= 1.0 {
                            *state.level.borrow_mut() = 0.0;
                            *state.phase.borrow_mut() = EnvPhase::Idle;
                        }
                    }
                    EnvPhase::Idle => {
                        *state.level.borrow_mut() = 0.0;
                    }
                }

                let output = *state.level.borrow();

                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::TriggeredADSR {
                        last_trigger_time: lt,
                        last_cycle: lc,
                        ..
                    } = node
                    {
                        *lt = latest_triggered_start as f32;
                        *lc = current_cycle;
                    }
                }

                output
            }

            SignalNode::PatternGate { pattern, .. } => {
                // Query pattern and output 1.0 if inside a true event, 0.0 otherwise
                let sample_width = 1.0 / self.sample_rate as f64 / self.cps as f64;

                let query_state = State {
                    span: TimeSpan::new(
                        Fraction::from_float(self.get_cycle_position()),
                        Fraction::from_float(self.get_cycle_position() + sample_width),
                    ),
                    controls: HashMap::new(),
                };
                let events = pattern.query(&query_state);

                // Output 1.0 if any true event is active
                let gate_active = events.iter().any(|e| e.value);
                if gate_active {
                    1.0
                } else {
                    0.0
                }
            }

            SignalNode::PatternTrigger {
                pattern,
                last_trigger_time,
                ..
            } => {
                // Output 1.0 for one sample at the start of each true event
                let sample_width = 1.0 / self.sample_rate as f64 / self.cps as f64;

                let query_state = State {
                    span: TimeSpan::new(
                        Fraction::from_float(self.get_cycle_position()),
                        Fraction::from_float(self.get_cycle_position() + sample_width),
                    ),
                    controls: HashMap::new(),
                };
                let events = pattern.query(&query_state);

                let last_event_start = if let Some(Some(node)) = self.nodes.get(node_id.0) {
                    if let SignalNode::PatternTrigger {
                        last_trigger_time: lt,
                        ..
                    } = &**node
                    {
                        *lt as f64
                    } else {
                        -1.0
                    }
                } else {
                    -1.0
                };

                let mut output = 0.0_f32;
                let mut latest_triggered = last_event_start;

                for event in events.iter() {
                    if !event.value {
                        continue;
                    }

                    let event_start_abs = if let Some(whole) = &event.whole {
                        whole.begin.to_float()
                    } else {
                        event.part.begin.to_float()
                    };

                    let tolerance = sample_width * 0.001;
                    if event_start_abs > last_event_start + tolerance {
                        output = 1.0;
                        if event_start_abs > latest_triggered {
                            latest_triggered = event_start_abs;
                        }
                    }
                }

                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::PatternTrigger {
                        last_trigger_time: lt,
                        ..
                    } = node
                    {
                        *lt = latest_triggered as f32;
                    }
                }

                output
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

                // BYPASS MODE: For pipelined rendering, pass through unchanged
                if self.bypass_sequential_effects {
                    return input_val;
                }

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
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Delay {
                        buffer: buf,
                        write_idx: idx,
                        ..
                    } = node
                    {
                        buf[*idx] = to_write;
                        *idx = (*idx + 1) % buf.len();
                    }
                }

                // Mix dry and wet
                input_val * (1.0 - mix_val) + delayed * mix_val
            }

            SignalNode::TapeDelay {
                input,
                time,
                feedback,
                wow_rate,
                wow_depth,
                flutter_rate,
                flutter_depth,
                saturation,
                mix,
                state,
            } => {
                let input_val = self.eval_signal(&input);

                // BYPASS MODE: For pipelined rendering, pass through unchanged
                if self.bypass_sequential_effects {
                    return input_val;
                }

                let delay_time = self.eval_signal(&time).max(0.001).min(1.0);
                let fb = self.eval_signal(&feedback).clamp(0.0, 0.95);
                let wow_r = self.eval_signal(&wow_rate).clamp(0.1, 2.0);
                let wow_d = self.eval_signal(&wow_depth).clamp(0.0, 1.0);
                let flutter_r = self.eval_signal(&flutter_rate).clamp(5.0, 10.0);
                let flutter_d = self.eval_signal(&flutter_depth).clamp(0.0, 1.0);
                let sat = self.eval_signal(&saturation).clamp(0.0, 1.0);
                let mix_val = self.eval_signal(&mix).clamp(0.0, 1.0);

                let buffer_len = state.buffer.len();
                let sample_rate = state.sample_rate;

                // Update wow and flutter LFOs
                let wow_phase_inc = wow_r / sample_rate;
                let flutter_phase_inc = flutter_r / sample_rate;

                // Modulate delay time with wow (slow) and flutter (fast)
                let wow = (state.wow_phase * std::f32::consts::TAU).sin() * wow_d * 0.001;
                let flutter =
                    (state.flutter_phase * std::f32::consts::TAU).sin() * flutter_d * 0.0001;

                let modulated_time = delay_time + wow + flutter;
                let delay_samples = (modulated_time * sample_rate)
                    .max(1.0)
                    .min(buffer_len as f32 - 1.0);

                // Fractional delay using linear interpolation
                let read_pos_f = (state.write_idx as f32) - delay_samples;
                let read_pos = if read_pos_f < 0.0 {
                    read_pos_f + buffer_len as f32
                } else {
                    read_pos_f
                };

                let read_idx = read_pos as usize % buffer_len;
                let next_idx = (read_idx + 1) % buffer_len;
                let frac = read_pos.fract();

                let delayed = state.buffer[read_idx] * (1.0 - frac) + state.buffer[next_idx] * frac;

                // Tape saturation (soft clipping)
                let saturated = if sat > 0.01 {
                    let drive = 1.0 + sat * 3.0;
                    (delayed * drive).tanh() / drive
                } else {
                    delayed
                };

                // Tape head filtering (one-pole lowpass)
                let cutoff_coef = 0.7 + sat * 0.2;
                let filtered = state.lpf_state * cutoff_coef + saturated * (1.0 - cutoff_coef);

                // Write to buffer
                let to_write = input_val + filtered * fb;

                // Update state
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::TapeDelay { state: s, .. } = node {
                        s.buffer[s.write_idx] = to_write;
                        s.write_idx = (s.write_idx + 1) % buffer_len;
                        s.wow_phase = (s.wow_phase + wow_phase_inc) % 1.0;
                        s.flutter_phase = (s.flutter_phase + flutter_phase_inc) % 1.0;
                        s.lpf_state = filtered;
                    }
                }

                // Mix dry and wet
                input_val * (1.0 - mix_val) + filtered * mix_val
            }

            SignalNode::MultiTapDelay {
                input,
                time,
                taps,
                feedback,
                mix,
                buffer,
                write_idx,
            } => {
                let input_val = self.eval_signal(&input);

                // BYPASS MODE: For pipelined rendering, pass through unchanged
                if self.bypass_sequential_effects {
                    return input_val;
                }

                let base_time = self.eval_signal(&time).max(0.001).min(1.0);
                let fb = self.eval_signal(&feedback).clamp(0.0, 0.95);
                let mix_val = self.eval_signal(&mix).clamp(0.0, 1.0);

                let buffer_len = buffer.len();
                let sample_rate = self.sample_rate();
                let base_delay_samples = (base_time * sample_rate) as usize;

                // Sum multiple tap outputs
                let mut tap_sum = 0.0;
                let tap_count = (*taps).min(8).max(2);

                for i in 1..=tap_count {
                    let tap_delay = base_delay_samples * i;
                    if tap_delay < buffer_len {
                        let read_idx = (*write_idx + buffer_len - tap_delay) % buffer_len;
                        let tap_amp = 1.0 / (i as f32);
                        tap_sum += buffer[read_idx] * tap_amp;
                    }
                }

                tap_sum /= tap_count as f32;

                // Write with feedback
                let to_write = input_val + tap_sum * fb;

                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::MultiTapDelay {
                        buffer: buf,
                        write_idx: idx,
                        ..
                    } = node
                    {
                        buf[*idx] = to_write;
                        *idx = (*idx + 1) % buffer_len;
                    }
                }

                // Mix
                input_val * (1.0 - mix_val) + tap_sum * mix_val
            }

            SignalNode::PingPongDelay {
                input,
                time,
                feedback,
                stereo_width,
                channel,
                mix,
                buffer_l,
                buffer_r,
                write_idx,
            } => {
                let input_val = self.eval_signal(&input);

                // BYPASS MODE: For pipelined rendering, pass through unchanged
                if self.bypass_sequential_effects {
                    return input_val;
                }

                let delay_time = self.eval_signal(&time).max(0.001).min(1.0);
                let fb = self.eval_signal(&feedback).clamp(0.0, 0.95);
                let width = self.eval_signal(&stereo_width).clamp(0.0, 1.0);
                let mix_val = self.eval_signal(&mix).clamp(0.0, 1.0);

                let buffer_len = buffer_l.len();
                let sample_rate = self.sample_rate();
                let delay_samples = (delay_time * sample_rate) as usize;

                let read_idx =
                    (*write_idx + buffer_len - delay_samples.min(buffer_len - 1)) % buffer_len;

                // Read from opposite channel for ping-pong effect
                let (delayed, opposite) = if *channel {
                    (buffer_r[read_idx], buffer_l[read_idx])
                } else {
                    (buffer_l[read_idx], buffer_r[read_idx])
                };

                // Mix with opposite channel
                let ping_ponged = delayed * (1.0 - width) + opposite * width;

                // Write to both buffers
                let to_write_l = if *channel {
                    ping_ponged * fb
                } else {
                    input_val + ping_ponged * fb
                };
                let to_write_r = if *channel {
                    input_val + ping_ponged * fb
                } else {
                    ping_ponged * fb
                };

                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::PingPongDelay {
                        buffer_l: buf_l,
                        buffer_r: buf_r,
                        write_idx: idx,
                        ..
                    } = node
                    {
                        buf_l[*idx] = to_write_l;
                        buf_r[*idx] = to_write_r;
                        *idx = (*idx + 1) % buffer_len;
                    }
                }

                // Mix
                input_val * (1.0 - mix_val) + ping_ponged * mix_val
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
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::RMS {
                        buffer: buf,
                        write_idx: idx,
                        ..
                    } = node
                    {
                        buf[*idx] = input_val * input_val;
                        *idx = (*idx + 1) % buf.len();
                    }
                }

                // Calculate RMS over the specified window
                // Sum only the most recent window_samples
                let mut sum: f32 = 0.0;
                for i in 0..window_samples {
                    let idx = (*write_idx + buffer.len() - i) % buffer.len();
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
                let mut output_state = *state;

                // Update state based on hysteresis logic
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Schmidt {
                        state: current_state,
                        ..
                    } = node
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
                let mut output_val = *held_value;

                // Update state if gate has rising edge (0→1)
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Latch {
                        held_value: stored_val,
                        last_gate: stored_gate,
                        ..
                    } = node
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
                let mut output_val = *elapsed_time;

                // Update state if trigger has rising edge (0→1)
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Timer {
                        elapsed_time: stored_time,
                        last_trigger: stored_trigger,
                        ..
                    } = node
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
                }

                output_val
            }

            SignalNode::Pitch { input, last_pitch } => {
                // Simplified pitch detection - would need more sophisticated algorithm
                let _input_val = self.eval_signal(&input);

                // For now, just return last pitch
                // Real implementation would do autocorrelation or FFT
                *last_pitch
            }

            SignalNode::Transient {
                input,
                threshold,
                last_value,
            } => {
                let input_val = self.eval_signal(&input).abs();
                let threshold_val = self.eval_signal(&threshold); // Pattern-modulatable threshold
                let last = *last_value;

                // Detect sharp changes (for saw wave discontinuities)
                let diff = (input_val - last).abs();

                // Generate transient pulse on significant changes
                let transient = if diff > threshold_val {
                    1.0
                } else if last > 1.5 && input_val < 0.5 {
                    // Detect saw wave reset (big drop)
                    1.0
                } else {
                    0.0
                };

                // Update last value with actual input (not transient)
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Transient { last_value: lv, .. } = node {
                        *lv = input_val;
                    }
                }

                transient
            }

            SignalNode::ZeroCrossing {
                input,
                last_sample,
                crossing_count,
                sample_count,
                window_samples,
                last_frequency,
            } => {
                let input_val = self.eval_signal(&input);
                let last = *last_sample;

                let mut output_freq = *last_frequency;
                let mut crossings = *crossing_count;
                let mut samples = *sample_count;

                // Detect zero crossing (sign change)
                if (last < 0.0 && input_val >= 0.0) || (last >= 0.0 && input_val < 0.0) {
                    crossings += 1;
                }

                samples += 1;

                // Update frequency estimate every window
                if samples >= *window_samples {
                    // Frequency = (crossings / 2) / (samples / sample_rate)
                    // Divide by 2 because each cycle has 2 crossings (positive and negative)
                    let time_seconds = samples as f32 / self.sample_rate;
                    output_freq = if crossings > 0 {
                        (crossings as f32 / 2.0) / time_seconds
                    } else {
                        0.0
                    };

                    // Reset counters for next window
                    crossings = 0;
                    samples = 0;
                }

                // Update state
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::ZeroCrossing {
                        last_sample: ls,
                        crossing_count: cc,
                        sample_count: sc,
                        last_frequency: lf,
                        ..
                    } = node
                    {
                        *ls = input_val;
                        *cc = crossings;
                        *sc = samples;
                        *lf = output_freq;
                    }
                }

                output_freq
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

                let mut output_val = *current_peak;

                // Update peak follower state
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::PeakFollower {
                        current_peak: stored_peak,
                        ..
                    } = node
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

                let mut output_val = *current_envelope;

                // Update amp follower state
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::AmpFollower {
                        buffer: buf,
                        write_idx: idx,
                        current_envelope: env,
                        ..
                    } = node
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
                }

                output_val
            }

            // NOTE: SignalNode::Wrap is already handled above (line 7879)
            SignalNode::Router {
                input,
                destinations: _,
            } => {
                // Router just passes through input, destinations are handled separately
                self.eval_signal(&input)
            }

            SignalNode::Conditional {
                condition,
                then_signal,
                else_signal,
            } => {
                // Evaluate condition
                let cond_value = self.eval_signal(&condition);

                // Route based on condition (> 0.5 = true)
                if cond_value > 0.5 {
                    self.eval_signal(&then_signal)
                } else {
                    self.eval_signal(&else_signal)
                }
            }

            SignalNode::Select { index, inputs } => {
                // Evaluate index signal
                let index_value = self.eval_signal(&index);

                // Handle empty inputs
                if inputs.is_empty() {
                    return 0.0;
                }

                // Round index and wrap to valid range [0, N-1]
                let num_inputs = inputs.len();
                let selected_idx =
                    ((index_value.round() as i32).rem_euclid(num_inputs as i32)) as usize;

                // Evaluate and return selected signal
                self.eval_signal(&inputs[selected_idx])
            }
        };

        // Cache the value appropriately:
        // - Non-stateful nodes go in value_cache (cleared per buffer - very fast)
        // - Stateful nodes go in stateful_value_cache (cleared per sample - prevents double eval)
        if is_stateful {
            // Stateful nodes: cache in per-sample cache to prevent double evaluation
            // within the same sample (e.g., from output eval + update_bus_previous_values)
            self.stateful_value_cache.insert(*node_id, value);
        } else {
            // Non-stateful nodes: cache in per-buffer cache (faster, less clearing)
            self.value_cache.insert(*node_id, value);
        }

        // Remove from call stack before returning
        self.eval_call_stack.remove(&node_id.0);

        value
    }

    /// Process one sample and advance time
    #[inline]
    pub fn process_sample(&mut self) -> f32 {
        // CRITICAL: Update cycle position from wall-clock ONCE per sample
        self.update_cycle_position_from_clock();

        // OPTIMIZATION: Don't clear value_cache every sample!
        // Pattern values only change at event boundaries, not per-sample.
        // Clearing every sample forces re-evaluation of the entire graph 44,100 times/second.
        // This was causing 4x slowdown in file rendering vs buffer processing.
        // TODO: Only clear cache when cycle position crosses event boundary
        // self.value_cache.clear();

        // Clear stateful_value_cache every sample to prevent double evaluation of stateful nodes
        self.stateful_value_cache.clear();

        // Process voice manager ONCE per sample and cache per-node outputs
        // This separates outputs so each output only hears its own samples
        // Sample nodes will look up their node ID in this cache
        self.voice_output_cache = self.voice_manager.borrow_mut().process_per_node();

        // Count active channels for gain compensation
        let mut num_active_channels = 0;

        // Start with single output (for backwards compatibility)
        // Check if channel 0 is hushed
        let mut mixed_output = if let Some(output_id) = self.output {
            if self.hushed_channels.contains(&0) {
                0.0 // Silenced
            } else {
                num_active_channels += 1;
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

            // Count active channel and mix the output
            num_active_channels += 1;
            mixed_output += self.eval_node(&node_id);
        }

        // Apply output mixing strategy
        mixed_output = match self.output_mix_mode {
            OutputMixMode::Gain => {
                // Automatic gain compensation - divide by number of channels
                if num_active_channels > 1 {
                    mixed_output / num_active_channels as f32
                } else {
                    mixed_output
                }
            }
            OutputMixMode::Sqrt => {
                // RMS-based mixing - divide by sqrt(num_channels)
                // Preserves perceived loudness
                if num_active_channels > 1 {
                    mixed_output / (num_active_channels as f32).sqrt()
                } else {
                    mixed_output
                }
            }
            OutputMixMode::Tanh => {
                // Soft saturation - prevents clipping with analog warmth
                mixed_output.tanh()
            }
            OutputMixMode::Hard => {
                // Hard limiting - brick-wall at ±1.0
                mixed_output.clamp(-1.0, 1.0)
            }
            OutputMixMode::None => {
                // No compensation - sum directly (can clip)
                mixed_output
            }
        };

        // Advance cycle position
        // REMOVED: Wall-clock based timing - no increment needed!

        // Update z^-1 storage for feedback loops
        // This stores the current sample's bus values for next sample's UnitDelay nodes
        self.update_bus_previous_values();

        // Increment sample counter
        self.sample_count += 1;

        mixed_output
    }

    /// Update bus_previous_values with current sample's bus outputs
    /// Called at the end of each sample after all buses have been evaluated
    /// This enables z^-1 (unit delay) for feedback loops
    fn update_bus_previous_values(&mut self) {
        // Collect bus names first to avoid borrow issues
        let bus_names: Vec<String> = self.buses.keys().cloned().collect();

        for bus_name in bus_names {
            if let Some(&node_id) = self.buses.get(&bus_name) {
                // Evaluate the bus (will hit cache if already evaluated this sample)
                let value = self.eval_node(&node_id);
                self.bus_previous_values.insert(bus_name, value);
            }
        }
    }

    /// Process one sample and return stereo output (left, right)
    /// This uses the VoiceManager's stereo processing for proper panning
    /// NOTE: Currently only sample playback is stereo. DSP chain (oscillators, filters)
    /// processes mono. Stereo separation comes from sample panning (jux, pan).
    #[inline]
    pub fn process_sample_stereo(&mut self) -> (f32, f32) {
        // CRITICAL: Update cycle position from wall-clock ONCE per sample
        self.update_cycle_position_from_clock();

        // Clear stateful_value_cache every sample to prevent double evaluation of stateful nodes
        self.stateful_value_cache.clear();

        // Process voice manager in STEREO mode - this gives us panned sample output
        // We use process_per_node_stereo to get stereo per source node
        self.voice_output_cache_stereo = self.voice_manager.borrow_mut().process_per_node_stereo();

        // Also populate mono cache (from stereo by mixing down)
        // DSP nodes that process sample output need mono values
        self.voice_output_cache = self
            .voice_output_cache_stereo
            .iter()
            .map(|(&node, &(l, r))| (node, (l + r) / std::f32::consts::SQRT_2))
            .collect();

        // CRITICAL: Call eval_node() on the output to trigger pattern evaluation!
        // This is what schedules new voices when patterns fire events.
        // Without this call, patterns never trigger and we get silence.
        if let Some(output_id) = self.output {
            // Evaluate the graph - this triggers pattern evaluation
            // We discard the mono result since we want stereo voice output
            let _mono_result = self.eval_node(&output_id);
        }

        // Also evaluate numbered outputs (out1, out2, etc.)
        let channels: Vec<(usize, NodeId)> =
            self.outputs.iter().map(|(&ch, &node)| (ch, node)).collect();
        for (_ch, node_id) in channels {
            let _result = self.eval_node(&node_id);
        }

        // Sum all stereo voice outputs
        let (mut total_left, mut total_right) = (0.0f32, 0.0f32);
        for &(l, r) in self.voice_output_cache_stereo.values() {
            total_left += l;
            total_right += r;
        }

        // Update z^-1 storage for feedback loops
        self.update_bus_previous_values();

        self.sample_count += 1;

        // Return stereo sample output
        // Note: In the future, we could add stereo DSP chain support here
        (total_left, total_right)
    }

    /// Process a buffer of stereo samples
    /// Returns interleaved stereo: [L0, R0, L1, R1, ...]
    pub fn process_buffer_stereo(&mut self, left: &mut [f32], right: &mut [f32]) {
        debug_assert_eq!(
            left.len(),
            right.len(),
            "Stereo buffers must be same length"
        );

        // CRITICAL: Clear buffer cache at start of each buffer render
        self.buffer_cache.borrow_mut().clear();
        self.buffer_cache_enabled.set(true);

        for i in 0..left.len() {
            let (l, r) = self.process_sample_stereo();
            left[i] = l;
            right[i] = r;
        }

        self.buffer_cache_enabled.set(false);
    }

    /// Pre-compute pattern events for the entire buffer (Option B optimization)
    /// This eliminates 512 pattern.query() calls per buffer by querying once
    fn precompute_pattern_events(&mut self, buffer_len: usize) {
        use crate::pattern::{Fraction, State, TimeSpan};
        use std::collections::HashMap;

        self.pattern_event_cache.clear();

        // Calculate buffer time span
        let start_cycle = self.get_cycle_position();
        let buffer_duration_cycles =
            (buffer_len as f64 / self.sample_rate as f64) * self.cps as f64;
        let end_cycle = start_cycle + buffer_duration_cycles;

        // Query each Pattern node AND Sample node once for the entire buffer span
        for (node_idx, node_opt) in self.nodes.iter().enumerate() {
            if let Some(node_rc) = node_opt {
                let pattern_opt = match &**node_rc {
                    SignalNode::Pattern { pattern, .. } => Some(pattern),
                    SignalNode::Sample { pattern, .. } => Some(pattern),
                    _ => None,
                };

                if let Some(pattern) = pattern_opt {
                    let state = State {
                        span: TimeSpan::new(
                            Fraction::from_float(start_cycle),
                            Fraction::from_float(end_cycle),
                        ),
                        controls: HashMap::new(),
                    };

                    // Query pattern once for entire buffer
                    let events = pattern.query(&state);

                    // Cache events for this node
                    self.pattern_event_cache.insert(NodeId(node_idx), events);
                }
            }
        }
    }

    /// EVENT-DRIVEN pattern triggering - O(events) instead of O(buffer_size * nodes)
    ///
    /// This is the key performance optimization. Instead of iterating over every sample
    /// and checking if any events should trigger, we iterate over the cached events directly
    /// and trigger voices only at the exact sample offsets where events occur.
    ///
    /// For a typical 512-sample buffer with ~4 events, this reduces iterations from
    /// 512 * N_sample_nodes to just 4, a ~100x improvement.
    fn process_pattern_events_event_driven(
        &mut self,
        buffer_size: usize,
        buffer_start_cycle: f64,
        sample_increment: f64,
    ) {
        let buffer_end_cycle = buffer_start_cycle + (buffer_size as f64 * sample_increment);

        // Collect (node_id, event_start_cycle, sample_offset) tuples for all events in this buffer
        // We need to collect first because we'll be mutating self when triggering
        let mut trigger_list: Vec<(usize, f64, usize)> = Vec::new();

        for (node_id, events) in &self.pattern_event_cache {
            // Only process Sample nodes (not Pattern nodes - those are for value patterns)
            let is_sample_node = if let Some(Some(node_rc)) = self.nodes.get(node_id.0) {
                matches!(&**node_rc, SignalNode::Sample { .. })
            } else {
                false
            };

            if !is_sample_node {
                continue;
            }

            // Get last trigger time for deduplication
            let last_trigger_time = if let Some(Some(node_rc)) = self.nodes.get(node_id.0) {
                if let SignalNode::Sample { last_trigger_time, .. } = &**node_rc {
                    *last_trigger_time as f64
                } else {
                    -1.0
                }
            } else {
                -1.0
            };

            for event in events {
                let sample_name = event.value.trim();

                // Skip rests
                if sample_name == "~" || sample_name.is_empty() {
                    continue;
                }

                // Get event start time
                let event_start = if let Some(whole) = &event.whole {
                    whole.begin.to_float()
                } else {
                    event.part.begin.to_float()
                };

                // Skip already triggered (dedup)
                let epsilon = 1e-6;
                if event_start <= last_trigger_time + epsilon {
                    continue;
                }

                // Skip events outside this buffer
                if event_start < buffer_start_cycle - epsilon || event_start >= buffer_end_cycle {
                    continue;
                }

                // Calculate sample offset within buffer
                let cycles_into_buffer = event_start - buffer_start_cycle;
                let sample_offset = ((cycles_into_buffer / sample_increment).round() as usize)
                    .min(buffer_size.saturating_sub(1));

                trigger_list.push((node_id.0, event_start, sample_offset));
            }
        }

        // Sort by sample offset to process in chronological order
        trigger_list.sort_by_key(|&(_, _, offset)| offset);

        // Now trigger voices for each event
        for (node_id, event_start, sample_offset) in trigger_list {
            // Set cycle position to just after the event start (so it passes the "event_is_new" check)
            self.cached_cycle_position = event_start + 1e-6;

            // Set the default source node for voice triggering
            self.voice_manager.borrow_mut().set_default_source_node(node_id);

            // Evaluate the Sample node - this will trigger the voice
            // The node's logic will see the event as "new" and trigger it
            let _ = self.eval_node(&NodeId(node_id));

            // Set the trigger offset for sample-accurate playback
            self.voice_manager.borrow_mut().set_last_voice_trigger_offset(sample_offset);
        }
    }

    /// Process a buffer of audio samples with timing provided by GlobalClock
    ///
    /// CRITICAL: This is the main entry point for live audio rendering.
    /// Timing comes FROM the GlobalClock (single source of truth), not from this graph.
    ///
    /// Arguments:
    /// - buffer: Output buffer to fill
    /// - buffer_start_cycle: Cycle position at start of buffer (from GlobalClock)
    /// - sample_increment: Cycle increment per sample (cps / sample_rate)
    /// - cps: Cycles per second (tempo) from GlobalClock
    #[inline]
    pub fn process_buffer_at(
        &mut self,
        buffer: &mut [f32],
        buffer_start_cycle: f64,
        sample_increment: f64,
        cps: f32,
    ) {
        // Update CPS from GlobalClock (single source of truth for tempo)
        self.cps = cps;

        // Process using the externally-provided timing
        self.process_buffer_internal(buffer, buffer_start_cycle, sample_increment);
    }

    /// Process a buffer of audio samples (for offline rendering)
    /// Calculates timing internally - use process_buffer_at() for live rendering!
    #[inline]
    pub fn process_buffer(&mut self, buffer: &mut [f32]) {
        // Calculate timing internally (for offline rendering)
        let buffer_start_cycle = if self.use_wall_clock {
            let elapsed = self.session_start_time.elapsed().as_secs_f64();
            elapsed * self.cps as f64 + self.cycle_offset
        } else {
            self.cached_cycle_position
        };
        let sample_increment = self.cps as f64 / self.sample_rate as f64;

        self.process_buffer_internal(buffer, buffer_start_cycle, sample_increment);
    }

    /// Internal buffer processing - timing is passed in
    #[inline]
    fn process_buffer_internal(
        &mut self,
        buffer: &mut [f32],
        buffer_start_cycle: f64,
        sample_increment: f64,
    ) {
        // CRITICAL: Initialize Sample node timing on first buffer to prevent double-triggering
        // When a graph is first used, Sample nodes have last_trigger_time = -1.0 (uninitialized)
        // This would cause ALL events at the current cycle position to trigger, even those
        // that already triggered in the previous graph. Fix: set last_trigger_time to just
        // before buffer_start_cycle so only NEW events in this buffer will trigger.
        for node_opt in self.nodes.iter_mut() {
            if let Some(node_rc) = node_opt {
                let node = std::rc::Rc::make_mut(node_rc);
                if let SignalNode::Sample {
                    last_trigger_time,
                    last_cycle,
                    ..
                } = node
                {
                    // Only initialize if uninitialized (-1.0 is the default)
                    if *last_trigger_time < 0.0 {
                        *last_trigger_time = buffer_start_cycle as f32 - 0.001;
                        *last_cycle = buffer_start_cycle.floor() as i32;
                    }
                }
            }
        }

        // CRITICAL: Clear buffer cache at start of each buffer render
        // This prevents stale cached values from previous buffer
        self.buffer_cache.borrow_mut().clear();

        // Enable buffer caching for this render pass
        self.buffer_cache_enabled.set(true);

        // TEMPORARILY DISABLED: Hybrid architecture has timing bugs
        // Produces audio immediately instead of waiting for sample triggers
        // Legacy RMS: 0.047, Hybrid RMS: 0.786 - not correctly timing
        // Set ENABLE_HYBRID_ARCH=1 to test the new path
        if std::env::var("ENABLE_HYBRID_ARCH").is_ok() {
            return self.process_buffer_hybrid(buffer, buffer_start_cycle, sample_increment);
        }

        // Buffer-passing graph processing: Modular synthesis architecture
        // DEFAULT: Always use DAG processing for proper cycle/feedback handling
        // Supports cycles (feedback loops) via 1-block delay, plus future parallelization
        return self.process_buffer_dag(buffer, buffer_start_cycle, sample_increment);

        // DEBUG: Write to file to confirm this is being called
        static CALL_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let count = CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if count == 0 {
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .open("/tmp/phonon_process_buffer_called.log")
            {
                use std::io::Write;
                let _ = writeln!(file, "process_buffer() IS being called!");
            }
        }

        // Optional profiling (enable with PROFILE_BUFFER=1)
        let enable_profiling = std::env::var("PROFILE_BUFFER").is_ok();
        let mut voice_time_us = 0u128;
        let mut eval_time_us = 0u128;
        let mut mix_time_us = 0u128;

        // HUGE OPTIMIZATION: Process all voices for entire buffer ONCE
        // Instead of calling process_per_node() 512 times, we call process_buffer_per_node() ONCE
        // This eliminates 511 redundant Rayon thread spawns and HashMap allocations
        let voice_start = if enable_profiling {
            Some(std::time::Instant::now())
        } else {
            None
        };

        // DEBUG: Check voice count before processing
        if std::env::var("DEBUG_VOICE_COUNT").is_ok() {
            let voice_count = self.voice_manager.borrow().active_voice_count();
            if voice_count > 0 {
                eprintln!("  Processing {} active voices in buffer", voice_count);
            }
        }

        // OPTIMIZED: Use Vec-based VoiceBuffers for O(1) lookup in hot loop
        // This replaces HashMap<usize, Vec<f32>> with direct array indexing
        self.voice_buffers = self
            .voice_manager
            .borrow_mut()
            .process_buffer_vec(buffer.len(), self.max_node_id);
        if let Some(start) = voice_start {
            voice_time_us = start.elapsed().as_micros();
        }

        // OPTION B OPTIMIZATION: Pre-compute pattern events once per buffer
        // This eliminates 512 pattern.query() calls per Pattern node
        self.precompute_pattern_events(buffer.len());

        // BUFFER-BASED SYNTHESIS: Generate synthesis buffers ONCE per buffer (not per sample!)
        // This matches the voice buffer architecture and enables SIMD auto-vectorization
        {
            let node_pitch_pairs: Vec<(usize, f32)> = self
                .voice_manager
                .borrow()
                .get_active_synthesis_node_ids_with_pitch();
            if !node_pitch_pairs.is_empty() {
                // Generate buffers for each unique (node_id, semitone_offset) combination
                // Use a HashMap with (node_id, rounded semitone) as key to deduplicate
                let mut synthesis_buffers: std::collections::HashMap<(usize, i32), Vec<f32>> =
                    std::collections::HashMap::new();

                for &(node_id, semitone_offset) in &node_pitch_pairs {
                    // Round semitone offset to avoid floating point precision issues
                    let semitone_key = (semitone_offset * 100.0).round() as i32;
                    let buffer_key = (node_id, semitone_key);

                    // Skip if we already generated this combination
                    if synthesis_buffers.contains_key(&buffer_key) {
                        continue;
                    }

                    // Find ALL oscillator nodes in the signal chain (bus may have multiple oscillators
                    // or be wrapped in effects like gain, filter, etc.)
                    let oscillator_ids = self.find_oscillator_nodes_in_chain(node_id);

                    // DEBUG: Log oscillator discovery
                    if std::env::var("DEBUG_SYNTH_BUFFERS").is_ok() {
                        eprintln!(
                            "[SYNTH_BUF] node_id={}, found oscillators: {:?}",
                            node_id, oscillator_ids
                        );
                    }

                    // Store original offsets AND phases for all oscillators
                    let mut original_state: Vec<(usize, f32, f32)> = Vec::new(); // (osc_id, offset, phase)
                    for &osc_id in &oscillator_ids {
                        if let Some(Some(node_rc)) = self.nodes.get(osc_id) {
                            if let SignalNode::Oscillator {
                                semitone_offset: node_offset,
                                phase,
                                ..
                            } = &**node_rc
                            {
                                original_state.push((osc_id, *node_offset, *phase.borrow()));
                            }
                        }
                    }

                    // Apply pitch offset to ALL oscillators in the chain AND set per-pitch phase
                    // Each pitch variant maintains its own phase for continuity across buffers
                    for &osc_id in &oscillator_ids {
                        if let Some(Some(node_rc)) = self.nodes.get_mut(osc_id) {
                            let node = std::rc::Rc::make_mut(node_rc);
                            if let SignalNode::Oscillator {
                                semitone_offset: node_offset,
                                phase,
                                ..
                            } = node
                            {
                                // Get cached phase for this (osc, pitch) combination, or start from 0
                                let phase_key = (osc_id, semitone_key);
                                let cached_phase = self
                                    .synthesis_phase_cache
                                    .borrow()
                                    .get(&phase_key)
                                    .copied()
                                    .unwrap_or(0.0);

                                // DEBUG: Log the offset and phase being applied
                                if std::env::var("DEBUG_SYNTH_BUFFERS").is_ok() {
                                    eprintln!("[SYNTH_BUF] Setting osc_id={} semitone_offset from {} to {}, phase from {:.4} to {:.4}",
                                        osc_id, *node_offset, semitone_offset, *phase.borrow(), cached_phase);
                                }
                                *node_offset = semitone_offset;
                                *phase.borrow_mut() = cached_phase;
                            }
                        }
                    }

                    // CRITICAL: Clear buffer cache before generating buffer with new semitone_offset
                    // Otherwise the cache returns stale results from previous pitch variants
                    self.buffer_cache.borrow_mut().clear();

                    // Generate buffer with the temporary semitone_offset(s)
                    let mut synth_buffer = vec![0.0; buffer.len()];
                    self.eval_node_buffer(&NodeId(node_id), &mut synth_buffer);

                    // DEBUG: Log buffer generation
                    if std::env::var("DEBUG_SYNTH_BUFFERS").is_ok() {
                        let first_samples: Vec<f32> =
                            synth_buffer.iter().take(10).cloned().collect();
                        eprintln!(
                            "[SYNTH_BUF] Generated buffer key=({}, {}), first_10={:?}",
                            node_id, semitone_key, first_samples
                        );
                    }

                    synthesis_buffers.insert(buffer_key, synth_buffer);

                    // Save phase to per-pitch cache BEFORE restoring original state
                    // This maintains phase continuity for this pitch variant across buffers
                    for &osc_id in &oscillator_ids {
                        if let Some(Some(node_rc)) = self.nodes.get(osc_id) {
                            if let SignalNode::Oscillator { phase, .. } = &**node_rc {
                                let phase_key = (osc_id, semitone_key);
                                let new_phase = *phase.borrow();
                                self.synthesis_phase_cache
                                    .borrow_mut()
                                    .insert(phase_key, new_phase);
                            }
                        }
                    }

                    // Restore original state (offset AND phase) for all oscillators
                    // This ensures the next pitch variant starts with clean state
                    for (osc_id, original_offset, original_phase) in original_state {
                        if let Some(Some(node_rc)) = self.nodes.get_mut(osc_id) {
                            let node = std::rc::Rc::make_mut(node_rc);
                            if let SignalNode::Oscillator {
                                semitone_offset: node_offset,
                                phase,
                                ..
                            } = node
                            {
                                *node_offset = original_offset;
                                *phase.borrow_mut() = original_phase;
                            }
                        }
                    }
                }

                // Process synthesis voices with envelopes and mix into voice_buffers
                let synthesis_voice_buffers = self
                    .voice_manager
                    .borrow_mut()
                    .process_synthesis_buffers(&synthesis_buffers, buffer.len());

                // Mix synthesis outputs into voice_buffers (now VoiceBuffers)
                if std::env::var("DEBUG_SYNTH_MIX").is_ok() {
                    eprintln!(
                        "[SYNTH_MIX] synthesis_voice_buffers.len()={}, voice_buffers.max_active_node={}",
                        synthesis_voice_buffers.len(),
                        self.voice_buffers.max_active_node
                    );
                    // Check samples 10 and 100 instead of 0 (sin(0)=0)
                    for idx in [10, 100] {
                        if let Some(sample) = synthesis_voice_buffers.get(idx) {
                            for (&k, &v) in sample.iter() {
                                eprintln!("[SYNTH_MIX] synth[{}][{}]={:.6}", idx, k, v);
                            }
                        }
                    }
                }
                // Merge synthesis buffers into VoiceBuffers
                for (i, synth_outputs) in synthesis_voice_buffers.iter().enumerate() {
                    for (&source_node, &value) in synth_outputs {
                        // Ensure buffer exists for this source_node
                        while self.voice_buffers.buffers.len() <= source_node {
                            self.voice_buffers.buffers.push(Vec::new());
                        }
                        let buf = &mut self.voice_buffers.buffers[source_node];
                        if buf.is_empty() {
                            *buf = vec![0.0; buffer.len()];
                        }
                        if i < buf.len() {
                            buf[i] += value;
                        }
                        if source_node > self.voice_buffers.max_active_node {
                            self.voice_buffers.max_active_node = source_node;
                        }
                    }
                }
                // DEBUG: Check voice_buffers AFTER mixing
                if std::env::var("DEBUG_SYNTH_MIX").is_ok() {
                    for node_id in 0..=self.voice_buffers.max_active_node {
                        let buf = &self.voice_buffers.buffers[node_id];
                        if buf.len() > 10 {
                            eprintln!(
                                "[SYNTH_MIX] voice_buffers[{}] sample 10: {:.6}",
                                node_id, buf[10]
                            );
                        }
                    }
                }
            }
        }

        // PERFORMANCE: Collect outputs ONCE per buffer instead of 512 times per buffer
        let output_channels: Vec<(usize, crate::unified_graph::NodeId)> =
            self.outputs.iter().map(|(&ch, &node)| (ch, node)).collect();

        // NOTE: buffer_start_cycle is now passed in as a parameter from GlobalClock
        // This is THE SINGLE SOURCE OF TRUTH for timing

        // Initialize node timing state on first buffer after graph swap
        if !self.nodes_initialized {
            // CRITICAL FIX: For cycle 0, use -1.0 to allow first event to trigger
            // Events at time 0.0 need last_trigger_time < 0.0 to pass the check
            // "event_start_abs > last_event_start + epsilon"
            let init_trigger_time = if buffer_start_cycle < 0.001 {
                -1.0_f32
            } else {
                buffer_start_cycle as f32
            };
            let current_cycle_i32 = buffer_start_cycle.floor() as i32;

            for node_opt in self.nodes.iter_mut() {
                if let Some(node_rc) = node_opt {
                    let node = std::rc::Rc::make_mut(node_rc);
                    match node {
                        SignalNode::Sample {
                            last_cycle,
                            last_trigger_time,
                            ..
                        } => {
                            *last_cycle = current_cycle_i32;
                            *last_trigger_time = init_trigger_time;
                        }
                        SignalNode::Pattern {
                            last_trigger_time, ..
                        } => {
                            *last_trigger_time = init_trigger_time;
                        }
                        SignalNode::SynthPattern {
                            last_trigger_time, ..
                        } => {
                            *last_trigger_time = init_trigger_time;
                        }
                        SignalNode::EnvelopePattern {
                            last_cycle,
                            last_trigger_time,
                            ..
                        } => {
                            *last_cycle = current_cycle_i32;
                            *last_trigger_time = init_trigger_time;
                        }
                        _ => {}
                    }
                }
            }

            self.nodes_initialized = true;

            if std::env::var("DEBUG_TIMING").is_ok() {
                eprintln!("🔧 Nodes initialized to cycle {:.4}", buffer_start_cycle);
            }
        }

        // Set initial cycle position for this buffer
        self.cached_cycle_position = buffer_start_cycle;

        // Track voices triggered during this buffer so we can process them live
        let initial_voice_count = self.voice_manager.borrow().active_voice_count();
        let mut newly_triggered_voices: Vec<usize> = Vec::new(); // Indices of newly triggered voices

        // NOTE: sample_increment is now passed in as a parameter from GlobalClock
        // This ensures timing is independent of buffer processing time

        for i in 0..buffer.len() {
            // CRITICAL: Increment cycle position deterministically using passed-in increment
            // This ensures timing comes from GlobalClock (single source of truth)
            if i > 0 {
                self.cached_cycle_position += sample_increment;
            }

            // CRITICAL OPTIMIZATION: Only clear value_cache at buffer start!
            // Most signal graph nodes compute static values that don't change every sample.
            // Pattern values only change at event boundaries (every N samples), not per-sample.
            // voice_output_cache is updated every sample below, so voice changes are tracked.
            if i == 0 {
                self.value_cache.clear();
            }
            // DON'T clear value_cache per-sample - that's the bottleneck!

            // Clear stateful_value_cache EVERY sample to ensure stateful nodes (ASR, ADSR, etc.)
            // are evaluated once per sample but not twice (from output eval + update_bus_previous_values)
            self.stateful_value_cache.clear();

            // NOTE: Synthesis buffer generation moved BEFORE the sample loop (buffer-based!)
            // No more per-sample synthesis evaluation - all done in buffers now

            // OPTIMIZED: Just set current sample index for O(1) lookup via voice_buffers.get()
            // This replaces per-sample HashMap rebuilding (~800μs/buffer savings)
            self.current_sample_idx = i;

            // Clear voice_output_cache - now only used for newly triggered voices
            self.voice_output_cache.clear();

            // CRITICAL FIX: Check if new voices were triggered in previous samples
            // Process them live for this sample
            let current_voice_count = self.voice_manager.borrow().active_voice_count();
            if current_voice_count > initial_voice_count + newly_triggered_voices.len() {
                // New voice(s) were just triggered!
                // Add them to our tracking list and process them for this sample
                while newly_triggered_voices.len() < (current_voice_count - initial_voice_count) {
                    newly_triggered_voices.push(initial_voice_count + newly_triggered_voices.len());
                }
            }

            // Process newly triggered voices for this sample
            if !newly_triggered_voices.is_empty() {
                for &voice_idx in &newly_triggered_voices {
                    if let Some(((left, right), source_node)) = self
                        .voice_manager
                        .borrow_mut()
                        .process_voice_by_index(voice_idx)
                    {
                        let mono = (left + right) / std::f32::consts::SQRT_2;

                        // Add to voice_output_cache
                        self.voice_output_cache
                            .entry(source_node)
                            .and_modify(|v| *v += mono)
                            .or_insert(mono);
                    }
                }
            }

            // Count active channels for gain compensation
            let mut num_active_channels = 0;

            // Start with single output (for backwards compatibility)
            let eval_start = if enable_profiling {
                Some(std::time::Instant::now())
            } else {
                None
            };
            let mut mixed_output = if let Some(output_id) = self.output {
                if self.hushed_channels.contains(&0) {
                    0.0 // Silenced
                } else {
                    num_active_channels += 1;
                    self.eval_node(&output_id)
                }
            } else {
                0.0
            };

            // Mix in all numbered output channels
            // Use pre-collected output_channels to avoid borrow checker issues
            for (ch, node_id) in &output_channels {
                if self.hushed_channels.contains(&ch) {
                    continue;
                }
                num_active_channels += 1;
                mixed_output += self.eval_node(&node_id);
            }
            if let Some(start) = eval_start {
                eval_time_us += start.elapsed().as_micros();
            }

            // Apply output mixing strategy
            let mix_start = if enable_profiling {
                Some(std::time::Instant::now())
            } else {
                None
            };
            mixed_output = match self.output_mix_mode {
                OutputMixMode::Gain => {
                    if num_active_channels > 1 {
                        mixed_output / num_active_channels as f32
                    } else {
                        mixed_output
                    }
                }
                OutputMixMode::Sqrt => {
                    if num_active_channels > 1 {
                        mixed_output / (num_active_channels as f32).sqrt()
                    } else {
                        mixed_output
                    }
                }
                OutputMixMode::Tanh => mixed_output.tanh(),
                OutputMixMode::Hard => mixed_output.clamp(-1.0, 1.0),
                OutputMixMode::None => mixed_output,
            };
            if let Some(start) = mix_start {
                mix_time_us += start.elapsed().as_micros();
            }

            // Increment sample counter
            self.sample_count += 1;

            // Update bus previous values for z^-1 feedback
            self.update_bus_previous_values();

            buffer[i] = mixed_output;
        }

        // Print profiling breakdown if enabled
        if enable_profiling {
            let total_us = voice_time_us + eval_time_us + mix_time_us;

            // Also write to file for live mode debugging
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/phonon_buffer_profile.log")
            {
                use std::io::Write;
                let _ = writeln!(file, "=== BUFFER PROFILING ({}samples) ===", buffer.len());
                let _ = writeln!(
                    file,
                    "Voice processing: {:.2}ms ({:.1}%)",
                    voice_time_us as f64 / 1000.0,
                    (voice_time_us as f64 / total_us as f64) * 100.0
                );
                let _ = writeln!(
                    file,
                    "Graph evaluation: {:.2}ms ({:.1}%)",
                    eval_time_us as f64 / 1000.0,
                    (eval_time_us as f64 / total_us as f64) * 100.0
                );
                let _ = writeln!(
                    file,
                    "Output mixing:    {:.2}ms ({:.1}%)",
                    mix_time_us as f64 / 1000.0,
                    (mix_time_us as f64 / total_us as f64) * 100.0
                );
            }
            let total_ms = total_us as f64 / 1000.0;
            let voice_ms = voice_time_us as f64 / 1000.0;
            let eval_ms = eval_time_us as f64 / 1000.0;
            let mix_ms = mix_time_us as f64 / 1000.0;

            eprintln!("=== BUFFER PROFILING ({}samples) ===", buffer.len());
            eprintln!(
                "Voice processing: {:.2}ms ({:.1}%)",
                voice_ms,
                100.0 * voice_ms / total_ms
            );
            eprintln!(
                "Graph evaluation: {:.2}ms ({:.1}%)",
                eval_ms,
                100.0 * eval_ms / total_ms
            );
            eprintln!(
                "Output mixing:    {:.2}ms ({:.1}%)",
                mix_ms,
                100.0 * mix_ms / total_ms
            );
            eprintln!("TOTAL:            {:.2}ms", total_ms);
            eprintln!();
        }
    }

    /// Hybrid architecture process_buffer (3-phase approach)
    /// PHASE 1: Pattern evaluation + voice triggering (sample-accurate)
    /// PHASE 2: Voice rendering (block-based)
    /// PHASE 3: DSP evaluation from buffers
    pub fn process_buffer_hybrid(
        &mut self,
        buffer: &mut [f32],
        buffer_start_cycle: f64,
        sample_increment: f64,
    ) {
        let buffer_size = buffer.len();
        let enable_profiling = std::env::var("PROFILE_BUFFER").is_ok();

        // Set cycle position to start of buffer
        self.cached_cycle_position = buffer_start_cycle;

        // Note: Sample node timing initialization is done in process_buffer_internal()
        // before this method is called, so we don't need to do it here.

        // CRITICAL: Clear buffer cache at start of each buffer render
        // This prevents stale cached values from previous buffer
        self.buffer_cache.borrow_mut().clear();

        // Enable buffer caching for this render pass
        self.buffer_cache_enabled.set(true);

        // Pre-compute pattern events once
        self.precompute_pattern_events(buffer_size);

        // PHASE 1: Pattern evaluation and voice triggering (sample-accurate)
        // Only trigger Sample nodes that are reachable from output nodes.
        // Intermediate Sample nodes (e.g., inputs to # note, # n modifiers) must NOT
        // be triggered directly - they're wrapped by the output Sample nodes.
        let phase1_start = if enable_profiling {
            Some(std::time::Instant::now())
        } else {
            None
        };

        // Collect only output-reachable Sample nodes (avoids triggering intermediate nodes)
        let output_sample_nodes = self.collect_output_sample_nodes();

        // Reset cycle position to buffer start for Phase 1
        self.cached_cycle_position = buffer_start_cycle;

        for i in 0..buffer_size {
            // Increment cycle position (skip first sample since we already set it)
            if i > 0 {
                self.cached_cycle_position += sample_increment;
            }

            // CRITICAL: Clear stateful_value_cache for each sample to allow re-evaluation
            // Without this, Sample nodes return cached values from sample 0 and never trigger
            // subsequent events. Legacy path clears this at line 14401.
            self.stateful_value_cache.clear();

            // Only evaluate output-reachable Sample nodes
            for &node_id in &output_sample_nodes {
                // Temporarily store current buffer position
                let current_buffer_pos = i;

                // Track voice count before eval to detect new triggers
                let voice_count_before = self.voice_manager.borrow().active_voice_count();

                // Evaluate Sample node (triggers voices, but we'll discard audio output)
                let _ = self.eval_node(&NodeId(node_id));

                // Only set trigger offset if a voice was actually triggered
                let voice_count_after = self.voice_manager.borrow().active_voice_count();
                if voice_count_after > voice_count_before {
                    // CRITICAL: Match old path timing - voice triggered at sample i produces
                    // first audio at sample i+1 (old path detects new voices AFTER eval_node)
                    self.voice_manager
                        .borrow_mut()
                        .set_last_voice_trigger_offset(current_buffer_pos + 1);
                }
            }

            self.sample_count += 1;
        }

        let phase1_time_us = phase1_start.map(|t| t.elapsed().as_micros()).unwrap_or(0);

        // PHASE 2: Voice rendering (block-based)
        let phase2_start = if enable_profiling {
            Some(std::time::Instant::now())
        } else {
            None
        };

        // PHASE 2a: Render sample voices (these work with render_block)
        let mut voice_buffers_map = self.voice_manager.borrow_mut().render_block(buffer_size);

        // PHASE 2b: Render synthesis voices (these need pre-generated buffers)
        // This mirrors the legacy path's synthesis handling
        let node_pitch_pairs = self
            .voice_manager
            .borrow()
            .get_active_synthesis_node_ids_with_pitch();

        if !node_pitch_pairs.is_empty() {
            // Generate buffers for each unique (node_id, semitone_offset) combination
            let mut synthesis_buffers: std::collections::HashMap<(usize, i32), Vec<f32>> =
                std::collections::HashMap::new();

            for &(node_id, semitone_offset) in &node_pitch_pairs {
                let semitone_key = (semitone_offset * 100.0).round() as i32;
                let buffer_key = (node_id, semitone_key);

                // Skip if we already generated this combination
                if synthesis_buffers.contains_key(&buffer_key) {
                    continue;
                }

                // Find ALL oscillator nodes in the signal chain
                let oscillator_ids = self.find_oscillator_nodes_in_chain(node_id);

                // Store original offsets AND phases for all oscillators
                let mut original_state: Vec<(usize, f32, f32)> = Vec::new();
                for &osc_id in &oscillator_ids {
                    if let Some(Some(node_rc)) = self.nodes.get(osc_id) {
                        if let SignalNode::Oscillator {
                            semitone_offset: node_offset,
                            phase,
                            ..
                        } = &**node_rc
                        {
                            original_state.push((osc_id, *node_offset, *phase.borrow()));
                        }
                    }
                }

                // Apply pitch offset to ALL oscillators in the chain
                for &osc_id in &oscillator_ids {
                    if let Some(Some(node_rc)) = self.nodes.get_mut(osc_id) {
                        let node = std::rc::Rc::make_mut(node_rc);
                        if let SignalNode::Oscillator {
                            semitone_offset: node_offset,
                            phase,
                            ..
                        } = node
                        {
                            // Get cached phase for this (osc, pitch) combination
                            let phase_key = (osc_id, semitone_key);
                            let cached_phase = self
                                .synthesis_phase_cache
                                .borrow()
                                .get(&phase_key)
                                .copied()
                                .unwrap_or(0.0);
                            *node_offset = semitone_offset;
                            *phase.borrow_mut() = cached_phase;
                        }
                    }
                }

                // CRITICAL: Clear buffer cache before generating buffer with new semitone_offset
                self.buffer_cache.borrow_mut().clear();

                // Generate buffer with the temporary semitone_offset(s)
                let mut synth_buffer = vec![0.0; buffer_size];
                self.eval_node_buffer(&NodeId(node_id), &mut synth_buffer);

                synthesis_buffers.insert(buffer_key, synth_buffer);

                // Save phase to per-pitch cache BEFORE restoring original state
                for &osc_id in &oscillator_ids {
                    if let Some(Some(node_rc)) = self.nodes.get(osc_id) {
                        if let SignalNode::Oscillator { phase, .. } = &**node_rc {
                            let phase_key = (osc_id, semitone_key);
                            let new_phase = *phase.borrow();
                            self.synthesis_phase_cache
                                .borrow_mut()
                                .insert(phase_key, new_phase);
                        }
                    }
                }

                // Restore original state for all oscillators
                for (osc_id, original_offset, original_phase) in original_state {
                    if let Some(Some(node_rc)) = self.nodes.get_mut(osc_id) {
                        let node = std::rc::Rc::make_mut(node_rc);
                        if let SignalNode::Oscillator {
                            semitone_offset: node_offset,
                            phase,
                            ..
                        } = node
                        {
                            *node_offset = original_offset;
                            *phase.borrow_mut() = original_phase;
                        }
                    }
                }
            }

            // Process synthesis voices with envelopes and mix
            let synthesis_voice_buffers = self
                .voice_manager
                .borrow_mut()
                .process_synthesis_buffers(&synthesis_buffers, buffer_size);

            // Mix synthesis outputs into voice_buffers_map
            for (i, sample_map) in synthesis_voice_buffers.into_iter().enumerate() {
                for (source_node, sample) in sample_map {
                    voice_buffers_map
                        .entry(source_node)
                        .and_modify(|buf| {
                            if i < buf.len() {
                                buf[i] += sample;
                            }
                        })
                        .or_insert_with(|| {
                            let mut new_buf = vec![0.0; buffer_size];
                            if i < new_buf.len() {
                                new_buf[i] = sample;
                            }
                            new_buf
                        });
                }
            }
        }

        let phase2_time_us = phase2_start.map(|t| t.elapsed().as_micros()).unwrap_or(0);

        // PHASE 3: DSP evaluation from voice buffers
        let phase3_start = if enable_profiling {
            Some(std::time::Instant::now())
        } else {
            None
        };

        // Convert HashMap to VoiceBuffers struct for O(1) lookup by Sample nodes
        // This is critical - Sample nodes read from self.voice_buffers, not voice_output_cache!
        let max_node_id = voice_buffers_map.keys().max().copied().unwrap_or(0).max(self.max_node_id);
        let mut vb = crate::voice_manager::VoiceBuffers::new(max_node_id, buffer_size);

        for (node_id, node_buffer) in voice_buffers_map {
            if node_id < vb.buffers.len() {
                vb.buffers[node_id] = node_buffer;
                if node_id > vb.max_active_node {
                    vb.max_active_node = node_id;
                }
            }
        }

        self.voice_buffers = vb;

        // DEBUG: Check voice_buffers content
        if std::env::var("DEBUG_VOICE_BUFFERS").is_ok() {
            let non_empty: Vec<_> = self.voice_buffers.buffers.iter().enumerate()
                .filter(|(_, b)| !b.is_empty())
                .map(|(i, b)| (i, b.len(), b.iter().take(5).cloned().collect::<Vec<_>>()))
                .collect();
            if !non_empty.is_empty() {
                eprintln!("[VOICE_BUFFERS] Non-empty buffers: {:?}", non_empty);
            }
        }

        // Pre-collect outputs to avoid borrow checker issues
        let output_channels: Vec<(usize, NodeId)> =
            self.outputs.iter().map(|(&ch, &node)| (ch, node)).collect();

        // Reset cycle position to buffer start for Phase 3 DSP evaluation
        self.cached_cycle_position = buffer_start_cycle;

        // BUFFER-BASED DSP EVALUATION
        // Instead of 512 per-sample eval_node() calls, we use eval_node_buffer() once per output
        // This enables SIMD auto-vectorization and reduces function call overhead

        // Clear caches once at start
        self.value_cache.clear();
        self.buffer_cache.borrow_mut().clear();

        // Count active channels for gain compensation (matches legacy path)
        let mut num_active_channels = 0;

        // CRITICAL: Zero the buffer first! Without this, buffer accumulates across calls
        // when self.output is None and numbered outputs just ADD to it.
        buffer.fill(0.0);

        // Start with the main output (for backwards compatibility)
        if let Some(output_id) = self.output {
            if !self.hushed_channels.contains(&0) {
                num_active_channels += 1;
                self.eval_node_buffer(&output_id, buffer);
            }
        }

        // Mix in numbered outputs using buffer-based evaluation
        if !output_channels.is_empty() {
            let mut temp_buffer = vec![0.0; buffer_size];
            for (ch, node_id) in &output_channels {
                if !self.hushed_channels.contains(ch) {
                    num_active_channels += 1;
                    // Clear temp buffer and buffer cache for each output
                    temp_buffer.fill(0.0);
                    self.buffer_cache.borrow_mut().clear();

                    self.eval_node_buffer(node_id, &mut temp_buffer);

                    // DEBUG
                    if std::env::var("DEBUG_OUTPUT_BUFFER").is_ok() {
                        let rms: f32 = (temp_buffer.iter().map(|x| x*x).sum::<f32>() / temp_buffer.len() as f32).sqrt();
                        let node_type = if let Some(Some(node_rc)) = self.nodes.get(node_id.0) {
                            format!("{:?}", std::mem::discriminant(&**node_rc))
                        } else {
                            "Unknown".to_string()
                        };
                        eprintln!("[OUTPUT_BUFFER] ch={}, node={}, type={}, RMS={:.4}", ch, node_id.0, node_type, rms);
                    }

                    // Mix into main buffer
                    for i in 0..buffer_size {
                        buffer[i] += temp_buffer[i];
                    }
                }
            }
        }

        // Apply output mixing strategy (matches legacy path)
        match self.output_mix_mode {
            OutputMixMode::Gain => {
                if num_active_channels > 1 {
                    let gain = 1.0 / num_active_channels as f32;
                    for sample in buffer.iter_mut() {
                        *sample *= gain;
                    }
                }
            }
            OutputMixMode::Sqrt => {
                if num_active_channels > 1 {
                    let gain = 1.0 / (num_active_channels as f32).sqrt();
                    for sample in buffer.iter_mut() {
                        *sample *= gain;
                    }
                }
            }
            OutputMixMode::Tanh => {
                for sample in buffer.iter_mut() {
                    *sample = sample.tanh();
                }
            }
            OutputMixMode::Hard => {
                for sample in buffer.iter_mut() {
                    *sample = sample.clamp(-1.0, 1.0);
                }
            }
            OutputMixMode::None => {
                // Direct sum - no gain compensation
            }
        }

        let phase3_time_us = phase3_start.map(|t| t.elapsed().as_micros()).unwrap_or(0);

        if enable_profiling {
            let total_us = phase1_time_us + phase2_time_us + phase3_time_us;
            let total_ms = total_us as f64 / 1000.0;
            eprintln!("=== HYBRID BUFFER PROFILING ({}samples) ===", buffer_size);
            eprintln!(
                "Phase 1 (Pattern eval): {:.2}ms ({:.1}%)",
                phase1_time_us as f64 / 1000.0,
                100.0 * phase1_time_us as f64 / total_us as f64
            );
            eprintln!(
                "Phase 2 (Voice render): {:.2}ms ({:.1}%)",
                phase2_time_us as f64 / 1000.0,
                100.0 * phase2_time_us as f64 / total_us as f64
            );
            eprintln!(
                "Phase 3 (DSP eval):     {:.2}ms ({:.1}%)",
                phase3_time_us as f64 / 1000.0,
                100.0 * phase3_time_us as f64 / total_us as f64
            );
            eprintln!("TOTAL:                  {:.2}ms", total_ms);
            eprintln!();
        }

        // Match old path behavior: leave cycle position at last sample's position
        // Old path increments BEFORE processing, so ends at (N-1) * increment
        // Next buffer will start at same position (i=0 has no increment)
        self.cached_cycle_position = buffer_start_cycle + ((buffer_size - 1) as f64 * sample_increment);
    }

    /// Render a buffer of audio (mono - mixes all channels)
    pub fn render(&mut self, num_samples: usize) -> Vec<f32> {
        // process_buffer_dag outputs stereo interleaved, so we need 2x the size
        let mut stereo_buffer = vec![0.0; num_samples * 2];
        self.process_buffer(&mut stereo_buffer);

        // Extract mono from stereo (left channel, every other sample)
        let mut mono_buffer = Vec::with_capacity(num_samples);
        for i in 0..num_samples {
            mono_buffer.push(stereo_buffer[i * 2]);
        }
        mono_buffer
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

    // ============================================================================
    // BUFFER-BASED EVALUATION (NEW ARCHITECTURE)
    // ============================================================================
    // These methods evaluate entire buffers at once instead of sample-by-sample.
    // This reduces function call overhead from 512 calls to 1 call per buffer,
    // enables SIMD vectorization, and improves cache locality.
    //
    // Expected speedup: 3-5x for Phase 3 DSP evaluation
    // ============================================================================

    /// Evaluate a node for an entire buffer
    ///
    /// This is the core buffer evaluation method that processes an entire buffer
    /// in one call instead of 512 sample-by-sample eval_node() calls.
    ///
    /// # Arguments
    /// * `node_id` - The node to evaluate
    /// * `output` - Pre-allocated output buffer to fill
    ///
    /// # Performance
    /// - Reduces function call overhead: 512 calls → 1 call
    /// - Enables SIMD vectorization by compiler
    /// - Improves cache locality (sequential buffer access)
    /// - Foundation for parallelization
    ///
    /// # Migration Status
    /// During gradual migration, not all nodes support buffer evaluation yet.
    /// Unsupported nodes fall back to sample-by-sample evaluation.

    /// Find all oscillator nodes in a signal chain by traversing from the output node
    /// This is needed because a bus like `sine 440 # gain 0.3` creates a chain where
    /// the bus points to the Multiply (gain) node, not the Oscillator.
    fn find_oscillator_nodes_in_chain(&self, start_node_id: usize) -> Vec<usize> {
        let mut oscillators = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut stack = vec![start_node_id];

        while let Some(node_id) = stack.pop() {
            if visited.contains(&node_id) {
                continue;
            }
            visited.insert(node_id);

            if let Some(Some(node_rc)) = self.nodes.get(node_id) {
                match &**node_rc {
                    SignalNode::Oscillator { .. } => {
                        oscillators.push(node_id);
                    }
                    // Traverse through common wrapper nodes - binary ops
                    SignalNode::Multiply { a, b }
                    | SignalNode::Add { a, b }
                    | SignalNode::Min { a, b } => {
                        if let Signal::Node(id) = a {
                            stack.push(id.0);
                        }
                        if let Signal::Node(id) = b {
                            stack.push(id.0);
                        }
                    }
                    // Filters
                    SignalNode::LowPass { input, .. }
                    | SignalNode::HighPass { input, .. }
                    | SignalNode::BandPass { input, .. }
                    | SignalNode::Notch { input, .. }
                    | SignalNode::MoogLadder { input, .. }
                    | SignalNode::SVF { input, .. }
                    | SignalNode::DJFilter { input, .. }
                    | SignalNode::Resonz { input, .. } => {
                        if let Signal::Node(id) = input {
                            stack.push(id.0);
                        }
                    }
                    // Effects
                    SignalNode::Reverb { input, .. }
                    | SignalNode::DattorroReverb { input, .. }
                    | SignalNode::Distortion { input, .. }
                    | SignalNode::Compressor { input, .. }
                    | SignalNode::BitCrush { input, .. }
                    | SignalNode::Chorus { input, .. }
                    | SignalNode::Vibrato { input, .. }
                    | SignalNode::Tremolo { input, .. }
                    | SignalNode::RingMod { input, .. }
                    | SignalNode::Expander { input, .. }
                    | SignalNode::Comb { input, .. }
                    | SignalNode::TapeDelay { input, .. }
                    | SignalNode::PingPongDelay { input, .. }
                    | SignalNode::ParametricEQ { input, .. } => {
                        if let Signal::Node(id) = input {
                            stack.push(id.0);
                        }
                    }
                    // Formant has "source" not "input"
                    SignalNode::Formant { source, .. } => {
                        if let Signal::Node(id) = source {
                            stack.push(id.0);
                        }
                    }
                    // Wrap and other utility nodes
                    SignalNode::Wrap { input, .. }
                    | SignalNode::Output { input }
                    | SignalNode::Lag { input, .. } => {
                        if let Signal::Node(id) = input {
                            stack.push(id.0);
                        }
                    }
                    // Crossfade
                    SignalNode::XFade {
                        signal_a, signal_b, ..
                    } => {
                        if let Signal::Node(id) = signal_a {
                            stack.push(id.0);
                        }
                        if let Signal::Node(id) = signal_b {
                            stack.push(id.0);
                        }
                    }
                    _ => {
                        // Other nodes - don't traverse (constants, patterns, noise, etc.)
                    }
                }
            }
        }

        oscillators
    }

    pub fn eval_node_buffer(&mut self, node_id: &NodeId, output: &mut [f32]) {
        let buffer_size = output.len();

        // If caching is not enabled (e.g., in tests), clear cache to avoid stale data
        // BUT only if we're not in the middle of an evaluation (no None entries)
        if !self.buffer_cache_enabled.get() {
            let can_clear = {
                let cache = self.buffer_cache.borrow();
                // Safe to clear if empty or all entries are Some (no active evaluations)
                cache.is_empty() || cache.values().all(|v| v.is_some())
            };
            if can_clear {
                self.buffer_cache.borrow_mut().clear();
            }
        }

        // Check cache first - prevents re-evaluation and stack overflow
        // This includes cycle detection (None = currently being evaluated)
        {
            let cache = self.buffer_cache.borrow();
            if let Some(cached_opt) = cache.get(node_id) {
                if let Some(cached) = cached_opt {
                    // Cached result available - only use if buffer size matches
                    if cached.len() == buffer_size {
                        output.copy_from_slice(cached);
                        return;
                    }
                    // Buffer size mismatch - fall through to re-evaluate
                } else {
                    // None = circular dependency detected (this node is currently being evaluated)
                    // Fill with silence to break the cycle
                    output.fill(0.0);
                    return;
                }
            }
        }

        // Mark node as being evaluated (insert None to detect circular dependencies)
        {
            let mut cache = self.buffer_cache.borrow_mut();
            cache.insert(*node_id, None);
        }

        // Get node (cheap Rc::clone)
        let node_rc = if let Some(Some(node_rc)) = self.nodes.get(node_id.0) {
            std::rc::Rc::clone(node_rc)
        } else {
            // Node doesn't exist, fill with silence
            output.fill(0.0);
            return;
        };

        let node = &*node_rc;

        // Dispatch to node-specific buffer evaluation
        // TODO: Migrate nodes one-by-one from sample-by-sample to buffer-based
        match node {
            SignalNode::Constant { value } => {
                // Simple case: fill buffer with constant value
                output.fill(*value);
            }

            SignalNode::Oscillator {
                freq,
                waveform,
                semitone_offset,
                phase,
                pending_freq,
                last_sample,
            } => {
                // Evaluate frequency signal once (if constant) or per-sample (if dynamic)
                let freq_signal = freq.clone();

                // Check if frequency is constant
                let is_constant_freq = matches!(freq_signal, Signal::Value(_));
                let constant_freq = if is_constant_freq {
                    if let Signal::Value(f) = freq_signal {
                        f
                    } else {
                        440.0
                    }
                } else {
                    0.0 // Will be evaluated per-sample
                };

                // Get current state
                let mut current_phase = *phase.borrow();
                let mut current_pending = *pending_freq.borrow();
                let mut current_last_sample = *last_sample.borrow();

                // Generate buffer
                for i in 0..buffer_size {
                    // Evaluate frequency for this sample
                    let requested_freq = if is_constant_freq {
                        constant_freq
                    } else {
                        self.eval_signal(&freq_signal)
                    };

                    let mut current_freq = requested_freq;

                    // Zero-crossing detection for anti-click frequency changes
                    if let Some(pending) = current_pending {
                        current_freq = pending;
                    }

                    // Decode pitch value:
                    // >= 1000: Absolute MIDI note (subtract 1000, convert to Hz)
                    // < 1000: Relative semitone offset from base frequency
                    let final_freq = if *semitone_offset >= 1000.0 {
                        // Absolute MIDI: convert to Hz using A4=440Hz as reference
                        let midi = *semitone_offset - 1000.0;
                        440.0 * 2.0_f32.powf((midi - 69.0) / 12.0)
                    } else if *semitone_offset != 0.0 {
                        // Relative: apply semitone offset to base frequency
                        current_freq * 2.0_f32.powf(*semitone_offset / 12.0)
                    } else {
                        current_freq
                    };

                    // DEBUG: Log frequency calculation (only for first sample)
                    if i == 0 && std::env::var("DEBUG_OSC_FREQ").is_ok() {
                        eprintln!(
                            "[OSC_FREQ] semitone_offset={}, requested_freq={:.2}, final_freq={:.2}",
                            *semitone_offset, requested_freq, final_freq
                        );
                    }
                    if i == 0 && std::env::var("DEBUG_OSC_FREQ_OLD").is_ok() {
                        // Log when final_freq is close to 440 Hz (the base oscillator freq)
                        if (final_freq - 440.0).abs() < 5.0 {
                            eprintln!(
                                "[OSC_FREQ] WARNING: 440Hz! semitone_offset={}, requested_freq={}",
                                *semitone_offset, requested_freq
                            );
                        }
                    }

                    current_freq = final_freq;

                    // Generate sample based on waveform
                    let sample = match waveform {
                        Waveform::Sine => (2.0 * std::f32::consts::PI * current_phase).sin(),
                        Waveform::Saw => 2.0 * current_phase - 1.0,
                        Waveform::Square => {
                            if current_phase < 0.5 {
                                1.0
                            } else {
                                -1.0
                            }
                        }
                        Waveform::Triangle => {
                            if current_phase < 0.5 {
                                4.0 * current_phase - 1.0
                            } else {
                                3.0 - 4.0 * current_phase
                            }
                        }
                    };

                    output[i] = sample;

                    // Check if frequency changed
                    if (requested_freq - current_freq).abs() > 0.1 {
                        current_pending = Some(current_freq);
                    }

                    // Check for zero-crossing (sign change from negative to positive)
                    if let Some(_pending) = current_pending {
                        if current_last_sample < 0.0 && sample >= 0.0 {
                            // Zero-crossing detected! Apply the frequency change
                            current_pending = None;
                        }
                    }

                    // Update phase for next sample
                    // CRITICAL FIX: Use final_freq (pitch-shifted) for phase increment,
                    // not requested_freq (original oscillator freq)
                    // pending_freq only affects anti-click smoothing, not pitch shifting
                    current_phase += final_freq / self.sample_rate;
                    if current_phase >= 1.0 {
                        current_phase -= 1.0;
                    }

                    // Store sample for next zero-crossing detection
                    current_last_sample = sample;
                }

                // Update state after processing entire buffer
                *phase.borrow_mut() = current_phase;
                *pending_freq.borrow_mut() = current_pending;
                *last_sample.borrow_mut() = current_last_sample;
            }

            SignalNode::Add { a, b } => {
                // Allocate buffers for both inputs
                let mut a_buffer = vec![0.0; buffer_size];
                let mut b_buffer = vec![0.0; buffer_size];

                // Evaluate both signals
                self.eval_signal_buffer(a, &mut a_buffer);
                self.eval_signal_buffer(b, &mut b_buffer);

                // Add element-wise
                for i in 0..buffer_size {
                    output[i] = a_buffer[i] + b_buffer[i];
                }
            }

            SignalNode::Multiply { a, b } => {
                // Allocate buffers for both inputs
                let mut a_buffer = vec![0.0; buffer_size];
                let mut b_buffer = vec![0.0; buffer_size];

                // Evaluate both signals
                self.eval_signal_buffer(a, &mut a_buffer);
                self.eval_signal_buffer(b, &mut b_buffer);

                // Multiply element-wise
                for i in 0..buffer_size {
                    output[i] = a_buffer[i] * b_buffer[i];
                }
            }

            SignalNode::Min { a, b } => {
                // Allocate buffers for both inputs
                let mut a_buffer = vec![0.0; buffer_size];
                let mut b_buffer = vec![0.0; buffer_size];

                // Evaluate both signals
                self.eval_signal_buffer(a, &mut a_buffer);
                self.eval_signal_buffer(b, &mut b_buffer);

                // Min element-wise
                for i in 0..buffer_size {
                    output[i] = a_buffer[i].min(b_buffer[i]);
                }
            }

            SignalNode::Wrap { input, min, max } => {
                // Allocate buffers for all three inputs
                let mut input_buffer = vec![0.0; buffer_size];
                let mut min_buffer = vec![0.0; buffer_size];
                let mut max_buffer = vec![0.0; buffer_size];

                // Evaluate all signals
                self.eval_signal_buffer(input, &mut input_buffer);
                self.eval_signal_buffer(min, &mut min_buffer);
                self.eval_signal_buffer(max, &mut max_buffer);

                // Wrap element-wise
                for i in 0..buffer_size {
                    let input_val = input_buffer[i];
                    let min_val = min_buffer[i];
                    let max_val = max_buffer[i];

                    let range = max_val - min_val;
                    if range.abs() < 1e-10 {
                        output[i] = min_val;
                    } else {
                        let normalized = (input_val - min_val) % range;
                        output[i] = if normalized < 0.0 {
                            normalized + range + min_val
                        } else {
                            normalized + min_val
                        };
                    }
                }
            }

            SignalNode::LowPass {
                input,
                cutoff,
                q,
                state,
            } => {
                // Allocate buffers for input and parameters
                let mut input_buffer = vec![0.0; buffer_size];
                let mut cutoff_buffer = vec![0.0; buffer_size];
                let mut q_buffer = vec![0.0; buffer_size];

                // Evaluate input and parameter signals to buffers
                self.eval_signal_buffer(input, &mut input_buffer);
                self.eval_signal_buffer(cutoff, &mut cutoff_buffer);
                self.eval_signal_buffer(q, &mut q_buffer);

                // Get current filter state
                let mut low = state.y1;
                let mut band = state.x1;
                let mut high = state.y2;

                // Process entire buffer
                for i in 0..buffer_size {
                    // Clamp parameters to valid ranges
                    let fc = cutoff_buffer[i].max(20.0).min(20000.0);
                    let q_val = q_buffer[i].max(0.5).min(20.0);

                    // Compute SVF coefficients (Chamberlin)
                    // f = 2 * sin(π * fc / fs)
                    // Clamp f to prevent instability (must be < 2.0)
                    let f = (2.0 * (std::f32::consts::PI * fc / self.sample_rate).sin()).min(1.99);
                    let damp = 1.0 / q_val;

                    // SVF tick (State Variable Filter)
                    high = input_buffer[i] - low - damp * band;
                    band += f * high;
                    low += f * band;

                    // Output is lowpass (low)
                    output[i] = low;
                }

                // Update filter state after processing entire buffer
                // We need to get mutable access to the node's state
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::LowPass { state, .. } = node {
                        state.y1 = low;
                        state.x1 = band;
                        state.y2 = high;
                        // Note: We're not caching coefficients in buffer mode
                        // since they might change per-sample
                    }
                }
            }

            SignalNode::HighPass {
                input,
                cutoff,
                q,
                state,
            } => {
                // Allocate buffers for input and parameters
                let mut input_buffer = vec![0.0; buffer_size];
                let mut cutoff_buffer = vec![0.0; buffer_size];
                let mut q_buffer = vec![0.0; buffer_size];

                // Evaluate input and parameter signals to buffers
                self.eval_signal_buffer(input, &mut input_buffer);
                self.eval_signal_buffer(cutoff, &mut cutoff_buffer);
                self.eval_signal_buffer(q, &mut q_buffer);

                // Get current filter state
                let mut low = state.y1;
                let mut band = state.x1;
                let mut high = state.y2;

                // Process entire buffer
                for i in 0..buffer_size {
                    // Clamp parameters to valid ranges
                    let fc = cutoff_buffer[i].max(20.0).min(20000.0);
                    let q_val = q_buffer[i].max(0.5).min(20.0);

                    // Compute SVF coefficients (Chamberlin)
                    // f = 2 * sin(π * fc / fs)
                    // Clamp f to prevent instability (must be < 2.0)
                    let f = (2.0 * (std::f32::consts::PI * fc / self.sample_rate).sin()).min(1.99);
                    let damp = 1.0 / q_val;

                    // SVF tick (State Variable Filter)
                    high = input_buffer[i] - low - damp * band;
                    band += f * high;
                    low += f * band;

                    // Output is highpass (high)
                    output[i] = high;
                }

                // Update filter state after processing entire buffer
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::HighPass { state, .. } = node {
                        state.y1 = low;
                        state.x1 = band;
                        state.y2 = high;
                    }
                }
            }

            SignalNode::BandPass {
                input,
                center,
                q,
                state,
            } => {
                // Allocate buffers for input and parameters
                let mut input_buffer = vec![0.0; buffer_size];
                let mut center_buffer = vec![0.0; buffer_size];
                let mut q_buffer = vec![0.0; buffer_size];

                // Evaluate input and parameter signals to buffers
                self.eval_signal_buffer(input, &mut input_buffer);
                self.eval_signal_buffer(center, &mut center_buffer);
                self.eval_signal_buffer(q, &mut q_buffer);

                // Get current filter state
                let mut low = state.y1;
                let mut band = state.x1;
                let mut high = state.y2;

                // Process entire buffer
                for i in 0..buffer_size {
                    // Clamp parameters to valid ranges
                    let fc = center_buffer[i].max(20.0).min(20000.0);
                    let q_val = q_buffer[i].max(0.5).min(20.0);

                    // Compute SVF coefficients (Chamberlin)
                    // f = 2 * sin(π * fc / fs)
                    // Clamp f to prevent instability (must be < 2.0)
                    let f = (2.0 * (std::f32::consts::PI * fc / self.sample_rate).sin()).min(1.99);
                    let damp = 1.0 / q_val;

                    // SVF tick (State Variable Filter)
                    high = input_buffer[i] - low - damp * band;
                    band += f * high;
                    low += f * band;

                    // Output is bandpass (band)
                    output[i] = band;
                }

                // Update filter state after processing entire buffer
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::BandPass { state, .. } = node {
                        state.y1 = low;
                        state.x1 = band;
                        state.y2 = high;
                    }
                }
            }

            SignalNode::Notch {
                input,
                center,
                q,
                state,
            } => {
                // Allocate buffers for input and parameters
                let mut input_buffer = vec![0.0; buffer_size];
                let mut center_buffer = vec![0.0; buffer_size];
                let mut q_buffer = vec![0.0; buffer_size];

                // Evaluate input and parameter signals to buffers
                self.eval_signal_buffer(input, &mut input_buffer);
                self.eval_signal_buffer(center, &mut center_buffer);
                self.eval_signal_buffer(q, &mut q_buffer);

                // Get current filter state
                let mut low = state.y1;
                let mut band = state.x1;
                let mut high = state.y2;

                // Process entire buffer
                for i in 0..buffer_size {
                    // Clamp parameters to valid ranges
                    let fc = center_buffer[i].max(20.0).min(20000.0);
                    let q_val = q_buffer[i].max(0.5).min(20.0);

                    // Compute SVF coefficients (Chamberlin)
                    // f = 2 * sin(π * fc / fs)
                    // Clamp f to prevent instability (must be < 2.0)
                    let f = (2.0 * (std::f32::consts::PI * fc / self.sample_rate).sin()).min(1.99);
                    let damp = 1.0 / q_val;

                    // SVF tick (State Variable Filter)
                    high = input_buffer[i] - low - damp * band;
                    band += f * high;
                    low += f * band;

                    // Output is notch (low + high = everything except band)
                    output[i] = low + high;
                }

                // Update filter state after processing entire buffer
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Notch { state, .. } = node {
                        state.y1 = low;
                        state.x1 = band;
                        state.y2 = high;
                    }
                }
            }

            SignalNode::Distortion { input, drive, mix } => {
                // Allocate buffers for input and parameters
                let mut input_buffer = vec![0.0; buffer_size];
                let mut drive_buffer = vec![0.0; buffer_size];
                let mut mix_buffer = vec![0.0; buffer_size];

                // Evaluate input and parameter signals to buffers
                self.eval_signal_buffer(input, &mut input_buffer);
                self.eval_signal_buffer(drive, &mut drive_buffer);
                self.eval_signal_buffer(mix, &mut mix_buffer);

                // Process entire buffer (stateless waveshaping)
                for i in 0..buffer_size {
                    // Clamp parameters to valid ranges
                    let drive_val = drive_buffer[i].clamp(1.0, 100.0);
                    let mix_val = mix_buffer[i].clamp(0.0, 1.0);

                    // Soft clipping waveshaper (tanh)
                    let driven = input_buffer[i] * drive_val;
                    let distorted = driven.tanh();

                    // Mix wet/dry
                    output[i] = input_buffer[i] * (1.0 - mix_val) + distorted * mix_val;
                }
            }

            SignalNode::Chorus {
                input,
                rate,
                depth,
                mix,
                state,
            } => {
                // Allocate buffers for input and parameters
                let mut input_buffer = vec![0.0; buffer_size];
                let mut rate_buffer = vec![0.0; buffer_size];
                let mut depth_buffer = vec![0.0; buffer_size];
                let mut mix_buffer = vec![0.0; buffer_size];

                // Evaluate input and parameter signals to buffers
                self.eval_signal_buffer(input, &mut input_buffer);
                self.eval_signal_buffer(rate, &mut rate_buffer);
                self.eval_signal_buffer(depth, &mut depth_buffer);
                self.eval_signal_buffer(mix, &mut mix_buffer);

                // Get current chorus state
                let buf_len = state.delay_buffer.len();
                let current_write_idx = state.write_idx;
                let current_lfo_phase = state.lfo_phase;

                // Create a copy of the delay buffer to work with
                let mut delay_buffer = state.delay_buffer.clone();
                let mut write_idx = current_write_idx;
                let mut lfo_phase = current_lfo_phase;

                // Process entire buffer
                for i in 0..buffer_size {
                    // Clamp parameters to valid ranges
                    let lfo_rate = rate_buffer[i].clamp(0.1, 10.0);
                    let mod_depth = depth_buffer[i].clamp(0.0, 1.0);
                    let mix_val = mix_buffer[i].clamp(0.0, 1.0);

                    // LFO for delay modulation (sine wave)
                    let lfo = (lfo_phase * 2.0 * std::f32::consts::PI).sin();

                    // Modulated delay time (5-25ms for chorus)
                    let base_delay = 0.015; // 15ms
                    let delay_time = base_delay + lfo * mod_depth * 0.010; // ±10ms
                    let delay_samples = (delay_time * self.sample_rate) as f32;

                    // Read from delay buffer with linear interpolation
                    let read_pos =
                        (write_idx as f32 + buf_len as f32 - delay_samples) % buf_len as f32;
                    let read_idx = read_pos.floor() as usize;
                    let frac = read_pos - read_pos.floor();

                    let sample1 = delay_buffer[read_idx % buf_len];
                    let sample2 = delay_buffer[(read_idx + 1) % buf_len];
                    let delayed = sample1 + (sample2 - sample1) * frac;

                    // Mix dry and wet
                    output[i] = input_buffer[i] * (1.0 - mix_val) + delayed * mix_val;

                    // Write input sample to delay buffer
                    delay_buffer[write_idx] = input_buffer[i];

                    // Update phase and write index for next sample
                    lfo_phase = (lfo_phase + lfo_rate / self.sample_rate) % 1.0;
                    write_idx = (write_idx + 1) % buf_len;
                }

                // Update chorus state after processing entire buffer
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Chorus { state: s, .. } = node {
                        s.delay_buffer = delay_buffer;
                        s.write_idx = write_idx;
                        s.lfo_phase = lfo_phase;
                    }
                }
            }

            SignalNode::Reverb {
                input,
                room_size,
                damping,
                mix,
                state,
            } => {
                // Allocate buffers for input and parameters
                let mut input_buffer = vec![0.0; buffer_size];
                let mut room_buffer = vec![0.0; buffer_size];
                let mut damping_buffer = vec![0.0; buffer_size];
                let mut mix_buffer = vec![0.0; buffer_size];

                // Evaluate input and parameter signals to buffers
                self.eval_signal_buffer(input, &mut input_buffer);
                self.eval_signal_buffer(room_size, &mut room_buffer);
                self.eval_signal_buffer(damping, &mut damping_buffer);
                self.eval_signal_buffer(mix, &mut mix_buffer);

                // Process entire buffer through Freeverb algorithm
                // Update state directly sample-by-sample to match the original eval_node behavior
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Reverb { state: s, .. } = node {
                        for i in 0..buffer_size {
                            let input_val = input_buffer[i];
                            let room = room_buffer[i].clamp(0.0, 1.0);
                            let damp = damping_buffer[i].clamp(0.0, 1.0);
                            let mix_val = mix_buffer[i].clamp(0.0, 1.0);

                            // Process comb filters (8 parallel)
                            let mut comb_out = 0.0;
                            for j in 0..8 {
                                let buf_len = s.comb_buffers[j].len();
                                let read_idx = s.comb_indices[j];
                                let delayed = s.comb_buffers[j][read_idx];

                                // Lowpass filter for damping
                                let filtered =
                                    s.comb_filter_stores[j] * damp + delayed * (1.0 - damp);

                                // Feedback with room-size dependent gain
                                let feedback = 0.84 * room;
                                let to_write = input_val + filtered * feedback;

                                comb_out += delayed;

                                // Update buffer and state
                                s.comb_buffers[j][read_idx] = to_write;
                                s.comb_indices[j] = (read_idx + 1) % buf_len;
                                s.comb_filter_stores[j] = filtered;
                            }

                            let mut allpass_out = comb_out / 8.0;

                            // Process allpass filters (4 in series)
                            for j in 0..4 {
                                let buf_len = s.allpass_buffers[j].len();
                                let read_idx = s.allpass_indices[j];
                                let delayed = s.allpass_buffers[j][read_idx];

                                let to_write = allpass_out + delayed * 0.5;
                                allpass_out = delayed - allpass_out * 0.5;

                                // Update buffer and state
                                s.allpass_buffers[j][read_idx] = to_write;
                                s.allpass_indices[j] = (read_idx + 1) % buf_len;
                            }

                            // Mix dry and wet
                            output[i] = input_val * (1.0 - mix_val) + allpass_out * mix_val;
                        }
                    }
                } else {
                    // Fallback: fill with zeros if node not found
                    output.fill(0.0);
                }
            }

            SignalNode::Delay {
                input,
                time,
                feedback,
                mix,
                buffer: _,
                write_idx: _,
            } => {
                // Clone signals for evaluation (needed due to borrowing rules)
                let input = input.clone();
                let time = time.clone();
                let feedback = feedback.clone();
                let mix = mix.clone();

                // Allocate buffers for input and parameters
                let mut input_buffer = vec![0.0; buffer_size];
                let mut time_buffer = vec![0.0; buffer_size];
                let mut feedback_buffer = vec![0.0; buffer_size];
                let mut mix_buffer = vec![0.0; buffer_size];

                // Evaluate signals
                self.eval_signal_buffer(&input, &mut input_buffer);
                self.eval_signal_buffer(&time, &mut time_buffer);
                self.eval_signal_buffer(&feedback, &mut feedback_buffer);
                self.eval_signal_buffer(&mix, &mut mix_buffer);

                // Get mutable access to the delay buffer and process in-place
                // This avoids the expensive clone() of the 44100-sample buffer
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Delay {
                        buffer: delay_buffer,
                        write_idx,
                        ..
                    } = node
                    {
                        let buffer_len = delay_buffer.len();
                        let mut current_write_idx = *write_idx;

                        // Process buffer: for each sample, read from delay line, write new sample
                        for i in 0..buffer_size {
                            // Clamp parameters to reasonable ranges
                            let delay_time = time_buffer[i].max(0.0).min(2.0);
                            let fb = feedback_buffer[i].max(0.0).min(0.99);
                            let mix_val = mix_buffer[i].max(0.0).min(1.0);

                            // Convert delay time to samples
                            let delay_samples = (delay_time * self.sample_rate) as usize;
                            let delay_samples = delay_samples.min(buffer_len - 1).max(1);

                            // Read from delay line
                            let read_idx =
                                (current_write_idx + buffer_len - delay_samples) % buffer_len;
                            let delayed = delay_buffer[read_idx];

                            // Write to delay line (input + feedback)
                            // Apply soft clipping to prevent feedback explosion
                            let to_write = (input_buffer[i] + delayed * fb).tanh();
                            delay_buffer[current_write_idx] = to_write;

                            // Mix dry and wet
                            output[i] = input_buffer[i] * (1.0 - mix_val) + delayed * mix_val;

                            // Advance write index
                            current_write_idx = (current_write_idx + 1) % buffer_len;
                        }

                        // Update write index
                        *write_idx = current_write_idx;
                    }
                }
            }

            SignalNode::Comb {
                input,
                frequency,
                feedback,
                buffer: _,
                write_pos: _,
            } => {
                // Clone signals for evaluation (needed due to borrowing rules)
                let input = input.clone();
                let frequency = frequency.clone();
                let feedback = feedback.clone();

                // Allocate buffers for input and parameters
                let mut input_buffer = vec![0.0; buffer_size];
                let mut frequency_buffer = vec![0.0; buffer_size];
                let mut feedback_buffer = vec![0.0; buffer_size];

                // Evaluate signals
                self.eval_signal_buffer(&input, &mut input_buffer);
                self.eval_signal_buffer(&frequency, &mut frequency_buffer);
                self.eval_signal_buffer(&feedback, &mut feedback_buffer);

                // Get mutable access to the comb buffer and process in-place
                // This avoids the expensive clone() of the buffer
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Comb {
                        buffer: comb_buffer,
                        write_pos,
                        ..
                    } = node
                    {
                        let buffer_len = comb_buffer.len();
                        let mut current_write_pos = *write_pos;

                        // Process buffer: for each sample, read from delay line and apply comb filter
                        for i in 0..buffer_size {
                            // Clamp parameters to reasonable ranges
                            let freq = frequency_buffer[i].max(20.0).min(20000.0);
                            let fb = feedback_buffer[i].clamp(0.0, 0.99);

                            // Convert frequency to delay time in samples
                            let delay_samples = (self.sample_rate / freq).round() as usize;
                            let delay_samples = delay_samples.clamp(1, buffer_len - 1);

                            // Calculate read position (write_pos - delay_samples, wrapped)
                            let read_pos =
                                (current_write_pos + buffer_len - delay_samples) % buffer_len;
                            let delayed = comb_buffer[read_pos];

                            // Comb filter: output = input + feedback * delayed_output
                            let out_sample = input_buffer[i] + fb * delayed;
                            output[i] = out_sample;

                            // Write output to buffer (feedback loop)
                            comb_buffer[current_write_pos] = out_sample;

                            // Advance write position
                            current_write_pos = (current_write_pos + 1) % buffer_len;
                        }

                        // Update write position
                        *write_pos = current_write_pos;
                    }
                }
            }

            SignalNode::Noise { seed } => {
                // Seeded white noise using Linear Congruential Generator (LCG)
                // This produces deterministic noise sequences for a given seed
                // Useful for reproducible sound design and testing
                let mut rng = *seed;

                for i in 0..buffer_size {
                    // LCG algorithm: X_{n+1} = (a * X_n + c) mod m
                    // Using standard glibc parameters
                    rng = rng.wrapping_mul(1103515245).wrapping_add(12345);

                    // Convert to [-1, 1] range
                    // Extract bits 16-30 (15 bits) for better randomness
                    let value = ((rng >> 16) & 0x7FFF) as f32 / 32768.0 * 2.0 - 1.0;
                    output[i] = value;
                }

                // Update seed for next buffer (stateful noise)
                // This ensures continuous noise across buffer boundaries
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = std::rc::Rc::make_mut(node_rc);
                    if let SignalNode::Noise { seed: s } = node {
                        *s = rng;
                    }
                }
            }

            SignalNode::PinkNoise { state } => {
                // Voss-McCartney algorithm for pink noise (1/f spectrum)
                // Maintains 16 octave bins updated at different rates
                use rand::Rng;
                let mut rng = rand::thread_rng();

                // Get current state
                let mut bins = state.bins;
                let mut counter = state.counter;

                // Generate buffer
                for i in 0..buffer_size {
                    // Update bins whose bit changed from 0 to 1
                    // Each bin updates at 1/2^i rate (bin 0 every sample, bin 1 every 2, etc.)
                    for j in 0..16 {
                        let mask = 1u32 << j;
                        if (counter & mask) == 0 {
                            // This bin should update (its bit is 0, was 1)
                            bins[j] = rng.gen_range(-1.0..1.0);
                        }
                    }

                    // Sum all bins and normalize
                    let sum: f32 = bins.iter().sum();
                    output[i] = sum / 16.0; // Normalize by number of bins

                    // Increment counter for next sample
                    counter = counter.wrapping_add(1);
                }

                // Update state after processing entire buffer
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::PinkNoise { state: s } = node {
                        s.bins = bins;
                        s.counter = counter;
                    }
                }
            }

            SignalNode::BitCrush {
                input,
                bits,
                sample_rate,
                state,
            } => {
                // Allocate buffers for input and parameters
                let mut input_buffer = vec![0.0; buffer_size];
                let mut bits_buffer = vec![0.0; buffer_size];
                let mut rate_buffer = vec![0.0; buffer_size];

                // Evaluate input and parameter signals to buffers
                self.eval_signal_buffer(input, &mut input_buffer);
                self.eval_signal_buffer(bits, &mut bits_buffer);
                self.eval_signal_buffer(sample_rate, &mut rate_buffer);

                // Get current state (phase is fractional sample counter, last_sample is held value)
                let mut phase = *state.phase.borrow();
                let mut held_sample = *state.last_sample.borrow();

                // Process entire buffer
                for i in 0..buffer_size {
                    // Clamp parameters to valid ranges
                    let bit_depth = bits_buffer[i].clamp(1.0, 16.0);
                    let rate_reduction = rate_buffer[i].clamp(1.0, 64.0);

                    // Increment phase (fractional sample counter)
                    phase += 1.0 / rate_reduction;

                    // Sample-and-hold: update held sample when phase crosses 1.0
                    if phase >= 1.0 {
                        // Reduce bit depth (quantization)
                        let levels = (2.0_f32).powf(bit_depth);
                        let quantized = (input_buffer[i] * levels).round() / levels;
                        held_sample = quantized;

                        // Wrap phase
                        phase = phase - phase.floor();
                    }

                    // Output the held sample
                    output[i] = held_sample;
                }

                // Update state after processing entire buffer
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::BitCrush { state: s, .. } = node {
                        *s.phase.borrow_mut() = phase;
                        *s.last_sample.borrow_mut() = held_sample;
                    }
                }
            }

            SignalNode::RingMod { input, freq, phase } => {
                // Allocate buffers for input and carrier frequency
                let mut input_buffer = vec![0.0; buffer_size];
                let mut freq_buffer = vec![0.0; buffer_size];

                // Evaluate input and frequency signals to buffers
                self.eval_signal_buffer(input, &mut input_buffer);
                self.eval_signal_buffer(freq, &mut freq_buffer);

                // Get current carrier phase
                let mut current_phase = *phase;

                // Process entire buffer
                for i in 0..buffer_size {
                    // Clamp carrier frequency to valid range
                    let carrier_freq = freq_buffer[i].clamp(20.0, 5000.0);

                    // Generate carrier sine wave sample
                    let carrier = current_phase.sin();

                    // Ring modulation is simple multiplication
                    output[i] = input_buffer[i] * carrier;

                    // Update carrier phase for next sample
                    current_phase += carrier_freq * 2.0 * std::f32::consts::PI / self.sample_rate;

                    // Wrap phase to [0, 2π)
                    if current_phase >= 2.0 * std::f32::consts::PI {
                        current_phase -= 2.0 * std::f32::consts::PI;
                    }
                }

                // Update carrier phase state after processing entire buffer
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::RingMod { phase: p, .. } = node {
                        *p = current_phase;
                    }
                }
            }

            SignalNode::FMCrossMod {
                carrier,
                modulator,
                mod_depth,
            } => {
                // Allocate buffers for carrier, modulator, and mod_depth
                let mut carrier_buffer = vec![0.0; buffer_size];
                let mut modulator_buffer = vec![0.0; buffer_size];
                let mut depth_buffer = vec![0.0; buffer_size];

                // Evaluate all signals to buffers
                self.eval_signal_buffer(carrier, &mut carrier_buffer);
                self.eval_signal_buffer(modulator, &mut modulator_buffer);
                self.eval_signal_buffer(mod_depth, &mut depth_buffer);

                // Process entire buffer: carrier * cos(2π * depth * modulator)
                use std::f32::consts::PI;
                for i in 0..buffer_size {
                    let phase_offset = 2.0 * PI * depth_buffer[i] * modulator_buffer[i];
                    output[i] = carrier_buffer[i] * phase_offset.cos();
                }
            }

            SignalNode::Tremolo {
                input,
                rate,
                depth,
                phase,
            } => {
                // Allocate buffers for input and parameters
                let mut input_buffer = vec![0.0; buffer_size];
                let mut rate_buffer = vec![0.0; buffer_size];
                let mut depth_buffer = vec![0.0; buffer_size];

                // Evaluate input and parameter signals to buffers
                self.eval_signal_buffer(input, &mut input_buffer);
                self.eval_signal_buffer(rate, &mut rate_buffer);
                self.eval_signal_buffer(depth, &mut depth_buffer);

                // Get current LFO phase
                let mut lfo_phase = *phase;

                // Process entire buffer
                for i in 0..buffer_size {
                    // Clamp parameters to valid ranges
                    let lfo_rate = rate_buffer[i].clamp(0.1, 20.0);
                    let mod_depth = depth_buffer[i].clamp(0.0, 1.0);

                    // Generate LFO (sine wave)
                    let lfo = lfo_phase.sin();

                    // Convert LFO to modulation amount
                    // depth=0: mod=1 (no effect, signal passes through)
                    // depth=1: mod oscillates between 0 and 1 (full amplitude modulation)
                    // Formula: 1 - depth/2 + depth/2 * lfo
                    // When lfo = -1: mod = 1 - depth/2 - depth/2 = 1 - depth
                    // When lfo = +1: mod = 1 - depth/2 + depth/2 = 1
                    let modulation = 1.0 - mod_depth * 0.5 + mod_depth * 0.5 * lfo;

                    // Apply amplitude modulation
                    output[i] = input_buffer[i] * modulation;

                    // Advance LFO phase
                    lfo_phase += lfo_rate * 2.0 * std::f32::consts::PI / self.sample_rate;

                    // Wrap phase to [0, 2π]
                    if lfo_phase >= 2.0 * std::f32::consts::PI {
                        lfo_phase -= 2.0 * std::f32::consts::PI;
                    }
                }

                // Update phase state after processing entire buffer
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Tremolo { phase: p, .. } = node {
                        *p = lfo_phase;
                    }
                }
            }

            SignalNode::MoogLadder {
                input,
                cutoff,
                resonance,
                state,
            } => {
                // Allocate buffers for input and parameters
                let mut input_buffer = vec![0.0; buffer_size];
                let mut cutoff_buffer = vec![0.0; buffer_size];
                let mut resonance_buffer = vec![0.0; buffer_size];

                // Evaluate input and parameter signals to buffers
                self.eval_signal_buffer(input, &mut input_buffer);
                self.eval_signal_buffer(cutoff, &mut cutoff_buffer);
                self.eval_signal_buffer(resonance, &mut resonance_buffer);

                // Get current ladder state (4 stages)
                let mut stage1 = state.stage1;
                let mut stage2 = state.stage2;
                let mut stage3 = state.stage3;
                let mut stage4 = state.stage4;

                // Process entire buffer
                for i in 0..buffer_size {
                    // Clamp parameters to valid ranges
                    let fc = cutoff_buffer[i].clamp(20.0, 20000.0);
                    let res = resonance_buffer[i].clamp(0.0, 1.0);

                    // Calculate cutoff coefficient (g) from frequency
                    // g = tan(π * fc / sr) / (1 + tan(π * fc / sr))
                    let g = (std::f32::consts::PI * fc / self.sample_rate).tan();
                    let g_normalized = g / (1.0 + g);

                    // Resonance scaling (0-4 is typical, higher = more resonance)
                    let resonance_amt = res * 4.0;

                    // Feedback from output to input (creates resonance)
                    let input_with_fb = input_buffer[i] - resonance_amt * stage4;

                    // Four cascaded 1-pole filters (linear stages)
                    stage1 += g_normalized * (input_with_fb - stage1);
                    stage2 += g_normalized * (stage1 - stage2);
                    stage3 += g_normalized * (stage2 - stage3);
                    stage4 += g_normalized * (stage3 - stage4);

                    // Output from final stage
                    output[i] = stage4;
                }

                // Update filter state after processing entire buffer
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::MoogLadder { state: s, .. } = node {
                        s.stage1 = stage1;
                        s.stage2 = stage2;
                        s.stage3 = stage3;
                        s.stage4 = stage4;
                    }
                }
            }

            SignalNode::DJFilter {
                input,
                value,
                state,
            } => {
                // Allocate buffers for input and parameter
                let mut input_buffer = vec![0.0; buffer_size];
                let mut value_buffer = vec![0.0; buffer_size];

                // Evaluate input and parameter signals to buffers
                self.eval_signal_buffer(input, &mut input_buffer);
                self.eval_signal_buffer(value, &mut value_buffer);

                // Get current filter state (y1=low, x1=band, y2=high in SVF)
                let mut low = state.y1;
                let mut band = state.x1;
                let mut high = state.y2;

                // Process entire buffer
                for i in 0..buffer_size {
                    // Clamp DJ filter value to 0-1 range
                    let djf_value = value_buffer[i].clamp(0.0, 1.0);

                    // Map djf value to filter cutoff frequency
                    // 0.0 -> very low cutoff (80 Hz) for aggressive lowpass
                    // 0.5 -> mid cutoff (800 Hz) - neutral point
                    // 1.0 -> high cutoff (8000 Hz) for aggressive highpass
                    let cutoff = if djf_value < 0.5 {
                        // Lowpass mode: map 0-0.5 to 80-800 Hz
                        80.0 + (djf_value * 2.0) * 720.0
                    } else {
                        // Highpass mode: map 0.5-1.0 to 800-8000 Hz
                        800.0 + ((djf_value - 0.5) * 2.0) * 7200.0
                    };

                    // Clamp cutoff to safe range to prevent filter instability
                    let cutoff = cutoff.max(20.0).min(self.sample_rate * 0.4);
                    // Use Q=1.0 for stability at high frequencies (Q=0.707 causes instability)
                    let q_val = 1.0;

                    // Compute SVF coefficients (Chamberlin)
                    // f = 2 * sin(π * fc / fs)
                    // Clamp f to prevent instability (must be < 2.0)
                    let f =
                        (2.0 * (std::f32::consts::PI * cutoff / self.sample_rate).sin()).min(1.9);
                    let damp = 1.0 / q_val;

                    // SVF tick (State Variable Filter)
                    high = input_buffer[i] - low - damp * band;
                    band += f * high;
                    low += f * band;

                    // Flush denormals to zero to prevent numerical instability
                    const DENORMAL_THRESHOLD: f32 = 1e-30;
                    if high.abs() < DENORMAL_THRESHOLD {
                        high = 0.0;
                    }
                    if band.abs() < DENORMAL_THRESHOLD {
                        band = 0.0;
                    }
                    if low.abs() < DENORMAL_THRESHOLD {
                        low = 0.0;
                    }

                    // Output selection: lowpass for < 0.5, highpass for > 0.5
                    let sample_output = if djf_value < 0.5 {
                        low // Lowpass output
                    } else {
                        high // Highpass output
                    };

                    // Ensure output is finite
                    output[i] = if sample_output.is_finite() {
                        sample_output
                    } else {
                        0.0
                    };
                }

                // Update filter state after processing entire buffer
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::DJFilter { state: s, .. } = node {
                        s.y1 = if low.is_finite() { low } else { 0.0 };
                        s.x1 = if band.is_finite() { band } else { 0.0 };
                        s.y2 = if high.is_finite() { high } else { 0.0 };
                    }
                }
            }

            SignalNode::Vibrato {
                input,
                rate,
                depth,
                phase,
                delay_buffer,
                buffer_pos,
            } => {
                // Allocate buffers for input and parameters
                let mut input_buffer = vec![0.0; buffer_size];
                let mut rate_buffer = vec![0.0; buffer_size];
                let mut depth_buffer = vec![0.0; buffer_size];

                // Evaluate input and parameter signals to buffers
                self.eval_signal_buffer(input, &mut input_buffer);
                self.eval_signal_buffer(rate, &mut rate_buffer);
                self.eval_signal_buffer(depth, &mut depth_buffer);

                // Get current vibrato state
                // Initialize buffer if empty (first call)
                let buffer_size_samples = (self.sample_rate * 0.05) as usize; // 50ms buffer
                let buf_len = if delay_buffer.is_empty() {
                    buffer_size_samples
                } else {
                    delay_buffer.len()
                };

                // Create working copies of state
                let mut delay_buf = if delay_buffer.is_empty() {
                    vec![0.0; buffer_size_samples]
                } else {
                    delay_buffer.clone()
                };
                let mut write_idx = *buffer_pos;
                let mut lfo_phase = *phase;

                // Process entire buffer
                for i in 0..buffer_size {
                    // Clamp parameters to valid ranges
                    let lfo_rate = rate_buffer[i].clamp(0.1, 20.0);
                    let depth_semitones = depth_buffer[i].clamp(0.0, 2.0);

                    // Fast bypass for zero depth
                    if depth_semitones < 0.001 {
                        output[i] = input_buffer[i];
                        // Still write to buffer and update indices for state continuity
                        delay_buf[write_idx] = input_buffer[i];
                        write_idx = (write_idx + 1) % buf_len;
                        continue;
                    }

                    // Write input to delay buffer
                    delay_buf[write_idx] = input_buffer[i];

                    // Calculate LFO (sine wave, -1 to +1)
                    let lfo = (lfo_phase * 2.0 * std::f32::consts::PI).sin();

                    // Convert depth from semitones to delay time
                    // Vibrato uses pitch modulation via time-varying delay
                    // depth in semitones -> frequency ratio -> time ratio
                    let max_delay_ms = 10.0; // Maximum 10ms delay for vibrato
                    let delay_ms = max_delay_ms * (depth_semitones / 2.0) * (1.0 + lfo);
                    let delay_samples = (delay_ms * self.sample_rate / 1000.0).max(0.0);

                    // Calculate read position (fractional) with wrapping
                    let read_pos_float = write_idx as f32 - delay_samples;
                    let read_pos_wrapped = if read_pos_float < 0.0 {
                        read_pos_float + buf_len as f32
                    } else {
                        read_pos_float
                    };

                    // Linear interpolation for fractional delay
                    let read_pos_int = (read_pos_wrapped as usize) % buf_len;
                    let read_pos_next = (read_pos_int + 1) % buf_len;
                    let frac = read_pos_wrapped.fract();

                    // Read delayed sample with interpolation
                    output[i] =
                        delay_buf[read_pos_int] * (1.0 - frac) + delay_buf[read_pos_next] * frac;

                    // Update phase and write index for next sample
                    lfo_phase += lfo_rate * 2.0 * std::f32::consts::PI / self.sample_rate;

                    // Wrap phase to [0, 2π]
                    if lfo_phase >= 2.0 * std::f32::consts::PI {
                        lfo_phase -= 2.0 * std::f32::consts::PI;
                    }

                    // Advance buffer position
                    write_idx = (write_idx + 1) % buf_len;
                }

                // Update vibrato state after processing entire buffer
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Vibrato {
                        phase: p,
                        delay_buffer: buf,
                        buffer_pos: pos,
                        ..
                    } = node
                    {
                        *p = lfo_phase;
                        *buf = delay_buf;
                        *pos = write_idx;
                    }
                }
            }

            SignalNode::Phaser {
                input,
                rate,
                depth,
                feedback,
                stages,
                phase,
                allpass_z1,
                allpass_y1,
                feedback_sample,
            } => {
                // Allocate buffers for input and parameters
                let mut input_buffer = vec![0.0; buffer_size];
                let mut rate_buffer = vec![0.0; buffer_size];
                let mut depth_buffer = vec![0.0; buffer_size];
                let mut feedback_buffer = vec![0.0; buffer_size];

                // Evaluate input and parameter signals to buffers
                self.eval_signal_buffer(input, &mut input_buffer);
                self.eval_signal_buffer(rate, &mut rate_buffer);
                self.eval_signal_buffer(depth, &mut depth_buffer);
                self.eval_signal_buffer(feedback, &mut feedback_buffer);

                // Get current phaser state
                let num_stages = *stages;
                let mut lfo_phase = *phase;
                let mut z1 = allpass_z1.clone();
                let mut y1 = allpass_y1.clone();
                let mut fb_sample = *feedback_sample;

                // Initialize allpass filter states if needed
                if z1.is_empty() {
                    z1.resize(num_stages, 0.0);
                    y1.resize(num_stages, 0.0);
                }

                // Process entire buffer
                for i in 0..buffer_size {
                    // Clamp parameters to valid ranges
                    let rate_hz = rate_buffer[i].clamp(0.05, 5.0);
                    let depth_val = depth_buffer[i].clamp(0.0, 1.0);
                    let feedback_val = feedback_buffer[i].clamp(0.0, 0.95);

                    // Fast bypass for zero depth
                    if depth_val < 0.001 {
                        output[i] = input_buffer[i];
                        continue;
                    }

                    // Advance LFO phase
                    lfo_phase += rate_hz * 2.0 * std::f32::consts::PI / self.sample_rate;
                    if lfo_phase >= 2.0 * std::f32::consts::PI {
                        lfo_phase -= 2.0 * std::f32::consts::PI;
                    }

                    // Calculate LFO (sine wave, 0 to 1)
                    let lfo = (lfo_phase.sin() + 1.0) * 0.5;

                    // Map LFO to cutoff frequency (200 Hz to 2000 Hz sweep)
                    let min_freq = 200.0;
                    let max_freq = 2000.0;
                    let cutoff = min_freq + (max_freq - min_freq) * lfo * depth_val;

                    // Calculate allpass coefficient
                    // a = (tan(π*fc/fs) - 1) / (tan(π*fc/fs) + 1)
                    let tan_val = (std::f32::consts::PI * cutoff / self.sample_rate).tan();
                    let a = (tan_val - 1.0) / (tan_val + 1.0);

                    // Apply feedback
                    let mut signal = input_buffer[i] + fb_sample * feedback_val;

                    // Apply allpass filter cascade
                    for stage in 0..num_stages {
                        // First-order allpass: y[n] = a*x[n] + x[n-1] - a*y[n-1]
                        let out = a * signal + z1[stage] - a * y1[stage];

                        // Update state
                        z1[stage] = signal;
                        y1[stage] = out;

                        signal = out;
                    }

                    // Store for feedback
                    fb_sample = signal;

                    // Mix filtered signal with dry signal (creates notches)
                    output[i] = (input_buffer[i] + signal) * 0.5;
                }

                // Update phaser state after processing entire buffer
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Phaser {
                        phase: p,
                        allpass_z1: z1_state,
                        allpass_y1: y1_state,
                        feedback_sample: fb_state,
                        ..
                    } = node
                    {
                        *p = lfo_phase;
                        *z1_state = z1;
                        *y1_state = y1;
                        *fb_state = fb_sample;
                    }
                }
            }

            // TODO: Add more nodes as they are migrated
            SignalNode::TapeDelay {
                input,
                time,
                feedback,
                wow_rate,
                wow_depth,
                flutter_rate,
                flutter_depth,
                saturation,
                mix,
                state,
            } => {
                // Allocate buffers for input and parameters
                let mut input_buffer = vec![0.0; buffer_size];
                let mut time_buffer = vec![0.0; buffer_size];
                let mut feedback_buffer = vec![0.0; buffer_size];
                let mut wow_rate_buffer = vec![0.0; buffer_size];
                let mut wow_depth_buffer = vec![0.0; buffer_size];
                let mut flutter_rate_buffer = vec![0.0; buffer_size];
                let mut flutter_depth_buffer = vec![0.0; buffer_size];
                let mut saturation_buffer = vec![0.0; buffer_size];
                let mut mix_buffer = vec![0.0; buffer_size];

                // Evaluate input and parameter signals to buffers
                self.eval_signal_buffer(input, &mut input_buffer);
                self.eval_signal_buffer(time, &mut time_buffer);
                self.eval_signal_buffer(feedback, &mut feedback_buffer);
                self.eval_signal_buffer(wow_rate, &mut wow_rate_buffer);
                self.eval_signal_buffer(wow_depth, &mut wow_depth_buffer);
                self.eval_signal_buffer(flutter_rate, &mut flutter_rate_buffer);
                self.eval_signal_buffer(flutter_depth, &mut flutter_depth_buffer);
                self.eval_signal_buffer(saturation, &mut saturation_buffer);
                self.eval_signal_buffer(mix, &mut mix_buffer);

                // Get current state
                let buffer_len = state.buffer.len();
                let mut delay_buffer = state.buffer.clone();
                let mut write_idx = state.write_idx;
                let mut wow_phase = state.wow_phase;
                let mut flutter_phase = state.flutter_phase;
                let mut lpf_state = state.lpf_state;

                // Process entire buffer
                for i in 0..buffer_size {
                    // Clamp parameters to valid ranges
                    let delay_time = time_buffer[i].max(0.001).min(1.0);
                    let fb = feedback_buffer[i].clamp(0.0, 0.95);
                    let wow_r = wow_rate_buffer[i].clamp(0.1, 2.0);
                    let wow_d = wow_depth_buffer[i].clamp(0.0, 1.0);
                    let flutter_r = flutter_rate_buffer[i].clamp(5.0, 10.0);
                    let flutter_d = flutter_depth_buffer[i].clamp(0.0, 1.0);
                    let sat = saturation_buffer[i].clamp(0.0, 1.0);
                    let mix_val = mix_buffer[i].clamp(0.0, 1.0);

                    // Update wow and flutter LFOs
                    let wow_phase_inc = wow_r / self.sample_rate;
                    let flutter_phase_inc = flutter_r / self.sample_rate;

                    // Modulate delay time with wow (slow) and flutter (fast)
                    let wow = (wow_phase * std::f32::consts::TAU).sin() * wow_d * 0.001;
                    let flutter =
                        (flutter_phase * std::f32::consts::TAU).sin() * flutter_d * 0.0001;

                    let modulated_time = delay_time + wow + flutter;
                    let delay_samples = (modulated_time * self.sample_rate)
                        .max(1.0)
                        .min(buffer_len as f32 - 1.0);

                    // Fractional delay using linear interpolation
                    let read_pos_f = (write_idx as f32) - delay_samples;
                    let read_pos = if read_pos_f < 0.0 {
                        read_pos_f + buffer_len as f32
                    } else {
                        read_pos_f
                    };

                    let read_idx = read_pos as usize % buffer_len;
                    let next_idx = (read_idx + 1) % buffer_len;
                    let frac = read_pos.fract();

                    let delayed =
                        delay_buffer[read_idx] * (1.0 - frac) + delay_buffer[next_idx] * frac;

                    // Tape saturation (soft clipping)
                    let saturated = if sat > 0.01 {
                        let drive = 1.0 + sat * 3.0;
                        (delayed * drive).tanh() / drive
                    } else {
                        delayed
                    };

                    // Tape head filtering (one-pole lowpass)
                    let cutoff_coef = 0.7 + sat * 0.2;
                    let filtered = lpf_state * cutoff_coef + saturated * (1.0 - cutoff_coef);

                    // Write to buffer
                    delay_buffer[write_idx] = input_buffer[i] + filtered * fb;

                    // Mix dry and wet
                    output[i] = input_buffer[i] * (1.0 - mix_val) + filtered * mix_val;

                    // Update phases and write index
                    wow_phase = (wow_phase + wow_phase_inc) % 1.0;
                    flutter_phase = (flutter_phase + flutter_phase_inc) % 1.0;
                    lpf_state = filtered;
                    write_idx = (write_idx + 1) % buffer_len;
                }

                // Update tape delay state after processing entire buffer
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::TapeDelay { state: s, .. } = node {
                        s.buffer = delay_buffer;
                        s.write_idx = write_idx;
                        s.wow_phase = wow_phase;
                        s.flutter_phase = flutter_phase;
                        s.lpf_state = lpf_state;
                    }
                }
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

                // Allocate buffers for input and all parameters
                let mut input_buffer = vec![0.0; buffer_size];
                let mut low_freq_buffer = vec![0.0; buffer_size];
                let mut low_gain_buffer = vec![0.0; buffer_size];
                let mut low_q_buffer = vec![0.0; buffer_size];
                let mut mid_freq_buffer = vec![0.0; buffer_size];
                let mut mid_gain_buffer = vec![0.0; buffer_size];
                let mut mid_q_buffer = vec![0.0; buffer_size];
                let mut high_freq_buffer = vec![0.0; buffer_size];
                let mut high_gain_buffer = vec![0.0; buffer_size];
                let mut high_q_buffer = vec![0.0; buffer_size];

                // Evaluate input and parameter signals to buffers
                self.eval_signal_buffer(input, &mut input_buffer);
                self.eval_signal_buffer(low_freq, &mut low_freq_buffer);
                self.eval_signal_buffer(low_gain, &mut low_gain_buffer);
                self.eval_signal_buffer(low_q, &mut low_q_buffer);
                self.eval_signal_buffer(mid_freq, &mut mid_freq_buffer);
                self.eval_signal_buffer(mid_gain, &mut mid_gain_buffer);
                self.eval_signal_buffer(mid_q, &mut mid_q_buffer);
                self.eval_signal_buffer(high_freq, &mut high_freq_buffer);
                self.eval_signal_buffer(high_gain, &mut high_gain_buffer);
                self.eval_signal_buffer(high_q, &mut high_q_buffer);

                // Get current filter states
                let mut low_x1 = state.low_band.x1;
                let mut low_x2 = state.low_band.x2;
                let mut mid_x1 = state.mid_band.x1;
                let mut mid_x2 = state.mid_band.x2;
                let mut high_x1 = state.high_band.x1;
                let mut high_x2 = state.high_band.x2;

                let sample_rate = self.sample_rate;

                // Process entire buffer
                for i in 0..buffer_size {
                    let mut signal = input_buffer[i];

                    // Low band (process first)
                    {
                        let fc = low_freq_buffer[i].clamp(20.0, 20000.0);
                        let gain_db = low_gain_buffer[i].clamp(-20.0, 20.0);
                        let q = low_q_buffer[i].clamp(0.1, 10.0);

                        // Only apply filter if gain is significant
                        if gain_db.abs() >= 0.1 {
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
                            let output_val = b0_norm * signal + low_x1;
                            let new_x1 = b1_norm * signal - a1_norm * output_val + low_x2;
                            let new_x2 = b2_norm * signal - a2_norm * output_val;

                            signal = output_val;
                            low_x1 = new_x1;
                            low_x2 = new_x2;
                        }
                    }

                    // Mid band (process second)
                    {
                        let fc = mid_freq_buffer[i].clamp(20.0, 20000.0);
                        let gain_db = mid_gain_buffer[i].clamp(-20.0, 20.0);
                        let q = mid_q_buffer[i].clamp(0.1, 10.0);

                        // Only apply filter if gain is significant
                        if gain_db.abs() >= 0.1 {
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
                            let output_val = b0_norm * signal + mid_x1;
                            let new_x1 = b1_norm * signal - a1_norm * output_val + mid_x2;
                            let new_x2 = b2_norm * signal - a2_norm * output_val;

                            signal = output_val;
                            mid_x1 = new_x1;
                            mid_x2 = new_x2;
                        }
                    }

                    // High band (process third)
                    {
                        let fc = high_freq_buffer[i].clamp(20.0, 20000.0);
                        let gain_db = high_gain_buffer[i].clamp(-20.0, 20.0);
                        let q = high_q_buffer[i].clamp(0.1, 10.0);

                        // Only apply filter if gain is significant
                        if gain_db.abs() >= 0.1 {
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
                            let output_val = b0_norm * signal + high_x1;
                            let new_x1 = b1_norm * signal - a1_norm * output_val + high_x2;
                            let new_x2 = b2_norm * signal - a2_norm * output_val;

                            signal = output_val;
                            high_x1 = new_x1;
                            high_x2 = new_x2;
                        }
                    }

                    output[i] = signal;
                }

                // Update filter states after processing entire buffer
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::ParametricEQ { state: s, .. } = node {
                        s.low_band.x1 = low_x1;
                        s.low_band.x2 = low_x2;
                        s.mid_band.x1 = mid_x1;
                        s.mid_band.x2 = mid_x2;
                        s.high_band.x1 = high_x1;
                        s.high_band.x2 = high_x2;
                    }
                }
            }

            SignalNode::Convolution { input, state } => {
                // Allocate buffer for input
                let mut input_buffer = vec![0.0; buffer_size];

                // Evaluate input signal to buffer
                self.eval_signal_buffer(input, &mut input_buffer);

                // Get impulse response length
                let ir_len = state.impulse_response.len();
                let buf_len = state.input_buffer.len();

                // Get current buffer index
                let mut current_buffer_index = state.buffer_index;

                // Process entire buffer
                for i in 0..buffer_size {
                    // Perform convolution for this sample
                    let mut sum = 0.0;
                    for j in 0..ir_len {
                        // Read backwards through input buffer (circular)
                        // We need to account for samples we've already stored in this buffer
                        let sample = if j <= i {
                            // Sample is in the current input_buffer
                            input_buffer[i - j]
                        } else {
                            // Sample is in the state's input_buffer (from previous buffers)
                            let lookback = j - i - 1;
                            let pos = (current_buffer_index + buf_len - lookback) % buf_len;
                            state.input_buffer[pos]
                        };

                        sum += sample * state.impulse_response[j];
                    }

                    output[i] = sum;
                }

                // Update state after processing entire buffer
                // Copy the input samples into the state's circular buffer
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Convolution { state: s, .. } = node {
                        // Copy all samples from input_buffer into the circular buffer
                        for i in 0..buffer_size {
                            s.input_buffer[current_buffer_index] = input_buffer[i];
                            current_buffer_index = (current_buffer_index + 1) % buf_len;
                        }
                        s.buffer_index = current_buffer_index;
                    }
                }
            }

            SignalNode::DattorroReverb {
                input,
                pre_delay,
                decay,
                diffusion,
                damping,
                mod_depth,
                mix,
                state,
            } => {
                // Allocate buffers for input and parameters
                let mut input_buffer = vec![0.0; buffer_size];
                let mut pre_delay_buffer = vec![0.0; buffer_size];
                let mut decay_buffer = vec![0.0; buffer_size];
                let mut diffusion_buffer = vec![0.0; buffer_size];
                let mut damping_buffer = vec![0.0; buffer_size];
                let mut mod_depth_buffer = vec![0.0; buffer_size];
                let mut mix_buffer = vec![0.0; buffer_size];

                // Evaluate input and parameter signals to buffers
                self.eval_signal_buffer(input, &mut input_buffer);
                self.eval_signal_buffer(pre_delay, &mut pre_delay_buffer);
                self.eval_signal_buffer(decay, &mut decay_buffer);
                self.eval_signal_buffer(diffusion, &mut diffusion_buffer);
                self.eval_signal_buffer(damping, &mut damping_buffer);
                self.eval_signal_buffer(mod_depth, &mut mod_depth_buffer);
                self.eval_signal_buffer(mix, &mut mix_buffer);

                // Clone all state buffers and variables locally for processing
                let mut predelay_buffer = state.predelay_buffer.clone();
                let mut predelay_idx = state.predelay_idx;

                let mut input_diffusion_buffers = state.input_diffusion_buffers.clone();
                let mut input_diffusion_indices = state.input_diffusion_indices;

                let mut left_apf1_buffer = state.left_apf1_buffer.clone();
                let mut left_apf1_idx = state.left_apf1_idx;
                let mut left_delay1_buffer = state.left_delay1_buffer.clone();
                let mut left_delay1_idx = state.left_delay1_idx;
                let mut left_apf2_buffer = state.left_apf2_buffer.clone();
                let mut left_apf2_idx = state.left_apf2_idx;
                let mut left_delay2_buffer = state.left_delay2_buffer.clone();
                let mut left_delay2_idx = state.left_delay2_idx;
                let mut left_lpf_state = state.left_lpf_state;

                let mut right_apf1_buffer = state.right_apf1_buffer.clone();
                let mut right_apf1_idx = state.right_apf1_idx;
                let mut right_delay1_buffer = state.right_delay1_buffer.clone();
                let mut right_delay1_idx = state.right_delay1_idx;
                let mut right_apf2_buffer = state.right_apf2_buffer.clone();
                let mut right_apf2_idx = state.right_apf2_idx;
                let mut right_delay2_buffer = state.right_delay2_buffer.clone();
                let mut right_delay2_idx = state.right_delay2_idx;
                let mut right_lpf_state = state.right_lpf_state;

                let mut lfo_phase = state.lfo_phase;
                let sample_rate = state.sample_rate;

                // Helper function for allpass filter
                let allpass =
                    |buffer: &mut Vec<f32>, idx: &mut usize, input: f32, gain: f32| -> f32 {
                        let buffer_len = buffer.len();
                        let delayed = buffer[*idx];
                        let output = -input + delayed + gain * (input - delayed);
                        buffer[*idx] = input + gain * delayed;
                        *idx = (*idx + 1) % buffer_len;
                        output
                    };

                // Helper function for simple delay
                let delay = |buffer: &mut Vec<f32>, idx: &mut usize, input: f32| -> f32 {
                    let buffer_len = buffer.len();
                    let output = buffer[*idx];
                    buffer[*idx] = input;
                    *idx = (*idx + 1) % buffer_len;
                    output
                };

                // Process entire buffer sample-by-sample (complex algorithm requires this)
                for i in 0..buffer_size {
                    let input_val = input_buffer[i];
                    let pre_delay_ms = pre_delay_buffer[i].clamp(0.0, 500.0);
                    let decay_val = decay_buffer[i].clamp(0.1, 10.0);
                    let diffusion_val = diffusion_buffer[i].clamp(0.0, 1.0);
                    let damping_val = damping_buffer[i].clamp(0.0, 1.0);
                    let mod_depth_val = mod_depth_buffer[i].clamp(0.0, 1.0);
                    let mix_val = mix_buffer[i].clamp(0.0, 1.0);

                    // 1. PRE-DELAY
                    let pre_delay_samples = ((pre_delay_ms / 1000.0) * sample_rate) as usize;
                    let pre_delay_samples = pre_delay_samples.min(predelay_buffer.len() - 1);

                    let predelay_out = if pre_delay_samples > 0 {
                        let read_idx = (predelay_idx + predelay_buffer.len() - pre_delay_samples)
                            % predelay_buffer.len();
                        let output = predelay_buffer[read_idx];
                        predelay_buffer[predelay_idx] = input_val;
                        predelay_idx = (predelay_idx + 1) % predelay_buffer.len();
                        output
                    } else {
                        input_val
                    };

                    // 2. INPUT DIFFUSION (4 series allpass filters)
                    let input_diffusion_gain = 0.75 * diffusion_val;
                    let mut diffused = predelay_out;

                    for j in 0..4 {
                        diffused = allpass(
                            &mut input_diffusion_buffers[j],
                            &mut input_diffusion_indices[j],
                            diffused,
                            input_diffusion_gain,
                        );
                    }

                    // Split into left and right for the figure-8 network
                    let input_to_tanks = diffused;

                    // 3. FIGURE-8 DECAY NETWORK
                    // Coefficients from Dattorro paper
                    let decay_diffusion1 = 0.7 * diffusion_val;
                    let decay_diffusion2 = 0.5 * diffusion_val;
                    let decay_gain = 0.4 + (decay_val - 0.1) / 9.9 * 0.55; // Map 0.1-10.0 to 0.4-0.95

                    // Damping (one-pole lowpass coefficient)
                    let damp_coef = 1.0 - damping_val * 0.7; // Higher damping = darker sound

                    // Modulation (simple LFO for chorus effect)
                    let lfo_rate = 0.8; // Hz
                    let lfo = (lfo_phase * std::f32::consts::TAU).sin() * mod_depth_val * 8.0; // ±8 samples modulation
                    lfo_phase = (lfo_phase + lfo_rate / sample_rate) % 1.0;

                    // LEFT TANK
                    // Read previous right tank output for cross-coupling
                    let right_to_left = right_delay2_buffer[right_delay2_idx];

                    // Input to left tank (with cross-coupling from right)
                    let left_input = input_to_tanks + right_to_left * decay_gain;

                    // Left APF1 (modulated)
                    let left_apf1_out = {
                        // Apply modulation by varying read position slightly
                        let mod_offset = lfo as isize;
                        let read_idx = ((left_apf1_idx as isize
                            + left_apf1_buffer.len() as isize
                            + mod_offset)
                            % left_apf1_buffer.len() as isize)
                            as usize;
                        let delayed = left_apf1_buffer[read_idx];
                        let output_apf =
                            -left_input + delayed + decay_diffusion1 * (left_input - delayed);
                        left_apf1_buffer[left_apf1_idx] = left_input + decay_diffusion1 * delayed;
                        left_apf1_idx = (left_apf1_idx + 1) % left_apf1_buffer.len();
                        output_apf
                    };

                    // Left Delay1
                    let left_delay1_out =
                        delay(&mut left_delay1_buffer, &mut left_delay1_idx, left_apf1_out);

                    // Left APF2 (modulated differently)
                    let left_apf2_out = {
                        let mod_offset = -lfo as isize;
                        let read_idx = ((left_apf2_idx as isize
                            + left_apf2_buffer.len() as isize
                            + mod_offset)
                            % left_apf2_buffer.len() as isize)
                            as usize;
                        let delayed = left_apf2_buffer[read_idx];
                        let output_apf = -left_delay1_out
                            + delayed
                            + decay_diffusion2 * (left_delay1_out - delayed);
                        left_apf2_buffer[left_apf2_idx] =
                            left_delay1_out + decay_diffusion2 * delayed;
                        left_apf2_idx = (left_apf2_idx + 1) % left_apf2_buffer.len();
                        output_apf
                    };

                    // Damping LPF and Delay2
                    let left_damped =
                        left_lpf_state * damp_coef + left_apf2_out * (1.0 - damp_coef);
                    left_lpf_state = left_damped;

                    let left_delay2_out = delay(
                        &mut left_delay2_buffer,
                        &mut left_delay2_idx,
                        left_damped * decay_gain,
                    );

                    // RIGHT TANK
                    // Read previous left tank output for cross-coupling
                    let left_to_right = left_delay2_out;

                    // Input to right tank (with cross-coupling from left)
                    let right_input = input_to_tanks + left_to_right;

                    // Right APF1 (modulated)
                    let right_apf1_out = {
                        let mod_offset = -lfo as isize;
                        let read_idx = ((right_apf1_idx as isize
                            + right_apf1_buffer.len() as isize
                            + mod_offset)
                            % right_apf1_buffer.len() as isize)
                            as usize;
                        let delayed = right_apf1_buffer[read_idx];
                        let output_apf =
                            -right_input + delayed + decay_diffusion1 * (right_input - delayed);
                        right_apf1_buffer[right_apf1_idx] =
                            right_input + decay_diffusion1 * delayed;
                        right_apf1_idx = (right_apf1_idx + 1) % right_apf1_buffer.len();
                        output_apf
                    };

                    // Right Delay1
                    let right_delay1_out = delay(
                        &mut right_delay1_buffer,
                        &mut right_delay1_idx,
                        right_apf1_out,
                    );

                    // Right APF2 (modulated differently)
                    let right_apf2_out = {
                        let mod_offset = lfo as isize;
                        let read_idx = ((right_apf2_idx as isize
                            + right_apf2_buffer.len() as isize
                            + mod_offset)
                            % right_apf2_buffer.len() as isize)
                            as usize;
                        let delayed = right_apf2_buffer[read_idx];
                        let output_apf = -right_delay1_out
                            + delayed
                            + decay_diffusion2 * (right_delay1_out - delayed);
                        right_apf2_buffer[right_apf2_idx] =
                            right_delay1_out + decay_diffusion2 * delayed;
                        right_apf2_idx = (right_apf2_idx + 1) % right_apf2_buffer.len();
                        output_apf
                    };

                    // Damping LPF and Delay2
                    let right_damped =
                        right_lpf_state * damp_coef + right_apf2_out * (1.0 - damp_coef);
                    right_lpf_state = right_damped;

                    let right_delay2_out = delay(
                        &mut right_delay2_buffer,
                        &mut right_delay2_idx,
                        right_damped * decay_gain,
                    );

                    // 4. OUTPUT TAPS (sum multiple points for density)
                    // Using multiple tap points as suggested by Dattorro
                    let left_output = (left_delay1_out + left_apf2_out + left_delay2_out) * 0.33;
                    let right_output =
                        (right_delay1_out + right_apf2_out + right_delay2_out) * 0.33;

                    // Mix stereo output (average L+R for mono)
                    let wet = (left_output + right_output) * 0.5;
                    output[i] = input_val * (1.0 - mix_val) + wet * mix_val;
                }

                // Update all state after processing entire buffer
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::DattorroReverb { state: s, .. } = node {
                        s.predelay_buffer = predelay_buffer;
                        s.predelay_idx = predelay_idx;

                        s.input_diffusion_buffers = input_diffusion_buffers;
                        s.input_diffusion_indices = input_diffusion_indices;

                        s.left_apf1_buffer = left_apf1_buffer;
                        s.left_apf1_idx = left_apf1_idx;
                        s.left_delay1_buffer = left_delay1_buffer;
                        s.left_delay1_idx = left_delay1_idx;
                        s.left_apf2_buffer = left_apf2_buffer;
                        s.left_apf2_idx = left_apf2_idx;
                        s.left_delay2_buffer = left_delay2_buffer;
                        s.left_delay2_idx = left_delay2_idx;
                        s.left_lpf_state = left_lpf_state;

                        s.right_apf1_buffer = right_apf1_buffer;
                        s.right_apf1_idx = right_apf1_idx;
                        s.right_delay1_buffer = right_delay1_buffer;
                        s.right_delay1_idx = right_delay1_idx;
                        s.right_apf2_buffer = right_apf2_buffer;
                        s.right_apf2_idx = right_apf2_idx;
                        s.right_delay2_buffer = right_delay2_buffer;
                        s.right_delay2_idx = right_delay2_idx;
                        s.right_lpf_state = right_lpf_state;

                        s.lfo_phase = lfo_phase;
                    }
                }
            }

            SignalNode::SpectralFreeze {
                input,
                trigger,
                state,
            } => {
                // Allocate buffers for input and trigger signal
                let mut input_buffer = vec![0.0; buffer_size];
                let mut trigger_buffer = vec![0.0; buffer_size];

                // Evaluate input and trigger signals to buffers
                self.eval_signal_buffer(input, &mut input_buffer);
                self.eval_signal_buffer(trigger, &mut trigger_buffer);

                // Process entire buffer through spectral freeze
                // We need to call the state's process method for each sample
                // The state itself handles FFT processing and spectrum freezing
                for i in 0..buffer_size {
                    let input_val = input_buffer[i];
                    let trigger_val = trigger_buffer[i];

                    // Process through spectral freeze
                    if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                        let node = Rc::make_mut(node_rc);
                        if let SignalNode::SpectralFreeze { state: s, .. } = node {
                            output[i] = s.process(input_val, trigger_val);
                        } else {
                            output[i] = input_val; // Fallback
                        }
                    } else {
                        output[i] = input_val; // Fallback
                    }
                }
                // Note: State is updated internally by process() method
            }

            SignalNode::PingPongDelay {
                input,
                time,
                feedback,
                stereo_width,
                channel,
                mix,
                buffer_l,
                buffer_r,
                write_idx,
            } => {
                let mut input_buffer = vec![0.0; buffer_size];
                let mut time_buffer = vec![0.0; buffer_size];
                let mut feedback_buffer = vec![0.0; buffer_size];
                let mut stereo_width_buffer = vec![0.0; buffer_size];
                let mut mix_buffer = vec![0.0; buffer_size];

                self.eval_signal_buffer(input, &mut input_buffer);
                self.eval_signal_buffer(time, &mut time_buffer);
                self.eval_signal_buffer(feedback, &mut feedback_buffer);
                self.eval_signal_buffer(stereo_width, &mut stereo_width_buffer);
                self.eval_signal_buffer(mix, &mut mix_buffer);

                let buf_len = buffer_l.len();
                let mut left_buf = buffer_l.clone();
                let mut right_buf = buffer_r.clone();
                let mut current_write_idx = *write_idx;
                let current_channel = *channel;

                for i in 0..buffer_size {
                    let delay_time = time_buffer[i].max(0.001).min(1.0);
                    let fb = feedback_buffer[i].clamp(0.0, 0.95);
                    let width = stereo_width_buffer[i].clamp(0.0, 1.0);
                    let mix_val = mix_buffer[i].clamp(0.0, 1.0);

                    let delay_samples = (delay_time * self.sample_rate) as usize;
                    let delay_samples = delay_samples.min(buf_len - 1);

                    let read_idx = (current_write_idx + buf_len - delay_samples) % buf_len;

                    let (delayed, opposite) = if current_channel {
                        (right_buf[read_idx], left_buf[read_idx])
                    } else {
                        (left_buf[read_idx], right_buf[read_idx])
                    };

                    let ping_ponged = delayed * (1.0 - width) + opposite * width;

                    output[i] = input_buffer[i] * (1.0 - mix_val) + ping_ponged * mix_val;

                    let to_write_l = if current_channel {
                        ping_ponged * fb
                    } else {
                        input_buffer[i] + ping_ponged * fb
                    };
                    let to_write_r = if current_channel {
                        input_buffer[i] + ping_ponged * fb
                    } else {
                        ping_ponged * fb
                    };

                    left_buf[current_write_idx] = to_write_l;
                    right_buf[current_write_idx] = to_write_r;

                    current_write_idx = (current_write_idx + 1) % buf_len;
                }

                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::PingPongDelay {
                        buffer_l: buf_l,
                        buffer_r: buf_r,
                        write_idx: idx,
                        ..
                    } = node
                    {
                        *buf_l = left_buf;
                        *buf_r = right_buf;
                        *idx = current_write_idx;
                    }
                }
            }

            SignalNode::SVF {
                input,
                frequency,
                resonance,
                mode,
                state,
            } => {
                // Chamberlin State Variable Filter - Buffer evaluation
                // Produces LP, HP, BP, and Notch outputs based on mode parameter

                // Allocate buffers for input and parameters
                let mut input_buffer = vec![0.0; buffer_size];
                let mut frequency_buffer = vec![0.0; buffer_size];
                let mut resonance_buffer = vec![0.0; buffer_size];

                // Evaluate input and parameter signals to buffers
                self.eval_signal_buffer(input, &mut input_buffer);
                self.eval_signal_buffer(frequency, &mut frequency_buffer);
                self.eval_signal_buffer(resonance, &mut resonance_buffer);

                // Get current filter state
                let mut low = state.low;
                let mut band = state.band;

                // Process entire buffer
                for i in 0..buffer_size {
                    // Clamp parameters to valid ranges
                    let freq = frequency_buffer[i].clamp(10.0, self.sample_rate * 0.45);
                    let res = resonance_buffer[i].max(0.1); // Prevent division by zero

                    // Calculate filter coefficients
                    // f = 2 * sin(π * cutoff / sampleRate)
                    // Prevent instability at high frequencies
                    let f = (std::f32::consts::PI * freq / self.sample_rate)
                        .sin()
                        .min(0.95);
                    let q = 1.0 / res.max(0.1); // Convert resonance to damping

                    // Update filter (Chamberlin topology)
                    low = low + f * band;
                    let high = input_buffer[i] - low - q * band;
                    band = f * high + band;
                    let notch = high + low;

                    // Clamp state to prevent runaway values and NaN
                    low = low.clamp(-10.0, 10.0);
                    band = band.clamp(-10.0, 10.0);

                    // Check for NaN and reset if needed
                    if !low.is_finite() || !band.is_finite() {
                        low = 0.0;
                        band = 0.0;
                    }

                    // Select output based on mode
                    output[i] = match mode {
                        0 => low,   // Lowpass
                        1 => high,  // Highpass
                        2 => band,  // Bandpass
                        3 => notch, // Notch
                        _ => low,   // Default to lowpass
                    };
                }

                // Update filter state after processing entire buffer
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::SVF { state: s, .. } = node {
                        s.low = low;
                        s.band = band;
                    }
                }
            }

            SignalNode::Wavetable { freq, state } => {
                // Evaluate frequency signal once (if constant) or per-sample (if dynamic)
                let freq_signal = freq.clone();

                // Check if frequency is constant
                let is_constant_freq = matches!(freq_signal, Signal::Value(_));
                let constant_freq = if is_constant_freq {
                    if let Signal::Value(f) = freq_signal {
                        f
                    } else {
                        440.0
                    }
                } else {
                    0.0 // Will be evaluated per-sample
                };

                // Get current phase
                let mut current_phase = state.phase;

                // Generate buffer
                for i in 0..buffer_size {
                    // Evaluate frequency for this sample
                    let f = if is_constant_freq {
                        constant_freq
                    } else {
                        self.eval_signal(&freq_signal)
                    }
                    .max(0.0);

                    // Get interpolated sample at current phase
                    let sample = state.get_sample(current_phase);
                    output[i] = sample;

                    // Update phase for next sample
                    current_phase += f / self.sample_rate;
                    if current_phase >= 1.0 {
                        current_phase -= 1.0;
                    }
                }

                // Update phase after processing entire buffer
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Wavetable { state: s, .. } = node {
                        s.phase = current_phase;
                    }
                }
            }

            SignalNode::Curve {
                start,
                end,
                duration,
                curve,
                elapsed_time,
            } => {
                // Allocate buffers for parameters
                let mut start_buffer = vec![0.0; buffer_size];
                let mut end_buffer = vec![0.0; buffer_size];
                let mut duration_buffer = vec![0.0; buffer_size];
                let mut curve_buffer = vec![0.0; buffer_size];

                // Evaluate parameter signals to buffers
                self.eval_signal_buffer(start, &mut start_buffer);
                self.eval_signal_buffer(end, &mut end_buffer);
                self.eval_signal_buffer(duration, &mut duration_buffer);
                self.eval_signal_buffer(curve, &mut curve_buffer);

                // Get current elapsed time
                let mut current_elapsed = *elapsed_time;

                // Process entire buffer
                for i in 0..buffer_size {
                    // Get parameter values for this sample
                    let start_val = start_buffer[i];
                    let end_val = end_buffer[i];
                    let duration_val = duration_buffer[i].max(0.001); // Min 1ms
                    let curve_val = curve_buffer[i];

                    // Calculate normalized time (0 to 1)
                    let t = (current_elapsed / duration_val).min(1.0);

                    // Apply curve formula
                    // Based on SuperCollider's Env.curve
                    // Negative curve = convex (fast start, slow end)
                    // Positive curve = concave (slow start, fast end)
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
                    output[i] = start_val + (end_val - start_val) * curved_t;

                    // Advance time
                    current_elapsed += 1.0 / self.sample_rate;
                }

                // Update elapsed time state after processing entire buffer
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Curve {
                        elapsed_time: et, ..
                    } = node
                    {
                        *et = current_elapsed;
                    }
                }
            }

            SignalNode::Granular {
                grain_size_ms,
                density,
                pitch,
                state,
                ..
            } => {
                // Evaluate pattern-modulatable parameters
                // Check if parameters are constant for optimization
                let grain_ms_signal = grain_size_ms.clone();
                let density_signal = density.clone();
                let pitch_signal = pitch.clone();

                let is_constant_params = matches!(grain_ms_signal, Signal::Value(_))
                    && matches!(density_signal, Signal::Value(_))
                    && matches!(pitch_signal, Signal::Value(_));

                let (constant_grain_ms, constant_density, constant_pitch) = if is_constant_params {
                    let gms = if let Signal::Value(v) = grain_ms_signal {
                        v
                    } else {
                        50.0
                    };
                    let dens = if let Signal::Value(v) = density_signal {
                        v
                    } else {
                        0.5
                    };
                    let ptch = if let Signal::Value(v) = pitch_signal {
                        v
                    } else {
                        1.0
                    };
                    (gms, dens, ptch)
                } else {
                    (0.0, 0.0, 0.0) // Will be evaluated per-sample
                };

                // Process buffer
                for i in 0..buffer_size {
                    // Evaluate parameters for this sample
                    let grain_ms = if is_constant_params {
                        constant_grain_ms
                    } else {
                        self.eval_signal(&grain_ms_signal)
                    }
                    .max(5.0)
                    .min(500.0);

                    let density_val = if is_constant_params {
                        constant_density
                    } else {
                        self.eval_signal(&density_signal)
                    }
                    .clamp(0.0, 1.0);

                    let pitch_val = if is_constant_params {
                        constant_pitch
                    } else {
                        self.eval_signal(&pitch_signal)
                    }
                    .max(0.1)
                    .min(4.0);

                    // Convert grain size from milliseconds to samples
                    let grain_size_samples = (grain_ms * self.sample_rate / 1000.0) as usize;

                    // Update granular state with mutable access
                    if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                        let node = Rc::make_mut(node_rc);
                        if let SignalNode::Granular { state: s, .. } = node {
                            // Spawn new grain based on density
                            // density controls spawn rate: 0.0 = never, 1.0 = every sample
                            s.grain_spawn_phase += density_val;
                            if s.grain_spawn_phase >= 1.0 {
                                s.grain_spawn_phase -= 1.0;
                                s.spawn_grain(grain_size_samples, pitch_val);
                            }

                            // Get mixed output from all active grains
                            output[i] = s.get_sample();

                            // Advance all grains
                            s.advance();
                        }
                    }
                }
            }

            SignalNode::XFade {
                signal_a,
                signal_b,
                position,
            } => {
                // Allocate temporary buffers for child signals
                let mut buffer_a = vec![0.0; buffer_size];
                let mut buffer_b = vec![0.0; buffer_size];
                let mut buffer_pos = vec![0.0; buffer_size];

                // Evaluate child signals
                self.eval_signal_buffer(signal_a, &mut buffer_a);
                self.eval_signal_buffer(signal_b, &mut buffer_b);
                self.eval_signal_buffer(position, &mut buffer_pos);

                // Crossfade: (1-pos)*a + pos*b
                for i in 0..buffer_size {
                    let pos = buffer_pos[i].clamp(0.0, 1.0);
                    output[i] = (1.0 - pos) * buffer_a[i] + pos * buffer_b[i];
                }
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
                // Allocate temporary buffers for input and parameters
                let mut input_buffer = vec![0.0; buffer_size];
                let mut threshold_buffer = vec![0.0; buffer_size];
                let mut ratio_buffer = vec![0.0; buffer_size];
                let mut attack_buffer = vec![0.0; buffer_size];
                let mut release_buffer = vec![0.0; buffer_size];
                let mut makeup_buffer = vec![0.0; buffer_size];

                // Evaluate all parameter signals
                self.eval_signal_buffer(input, &mut input_buffer);
                self.eval_signal_buffer(threshold, &mut threshold_buffer);
                self.eval_signal_buffer(ratio, &mut ratio_buffer);
                self.eval_signal_buffer(attack, &mut attack_buffer);
                self.eval_signal_buffer(release, &mut release_buffer);
                self.eval_signal_buffer(makeup_gain, &mut makeup_buffer);

                // Process buffer with stateful compression
                // We need mutable access to update the envelope state
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::Compressor { state: s, .. } = node {
                        let mut envelope = s.envelope;

                        for i in 0..buffer_size {
                            // Clamp parameters to reasonable ranges
                            let threshold_db = threshold_buffer[i].clamp(-60.0, 0.0);
                            let ratio_val = ratio_buffer[i].clamp(1.0, 20.0);
                            let attack_time = attack_buffer[i].clamp(0.001, 1.0);
                            let release_time = release_buffer[i].clamp(0.01, 3.0);
                            let makeup_db = makeup_buffer[i].clamp(0.0, 30.0);

                            // Convert threshold from dB to linear
                            let threshold_lin = 10.0_f32.powf(threshold_db / 20.0);

                            // Envelope follower (peak detector with attack/release)
                            let input_level = input_buffer[i].abs();

                            // Envelope follower: attack when input > envelope, release when input < envelope
                            let coeff = if input_level > envelope {
                                // Attack: faster response to increasing levels
                                (-(1.0 / (attack_time * self.sample_rate))).exp()
                            } else {
                                // Release: slower response to decreasing levels
                                (-(1.0 / (release_time * self.sample_rate))).exp()
                            };

                            envelope = coeff * envelope + (1.0 - coeff) * input_level;

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
                            output[i] = input_buffer[i] * gain_reduction * makeup_gain_lin;
                        }

                        // Update state with final envelope value
                        s.envelope = envelope;
                    }
                } else {
                    // Fallback: fill with zeros if node not found
                    output.fill(0.0);
                }
            }

            SignalNode::Sample { .. } => {
                // In hybrid mode, Sample nodes read from voice_buffers which contain
                // pre-rendered audio from Phase 2. Just copy the buffer directly.
                // CRITICAL: Do NOT fall back to eval_node - that would re-trigger voices!
                let node_idx = node_id.0;
                if node_idx < self.voice_buffers.buffers.len() {
                    let voice_buf = &self.voice_buffers.buffers[node_idx];
                    if voice_buf.len() >= buffer_size {
                        output.copy_from_slice(&voice_buf[..buffer_size]);
                        // DEBUG
                        if std::env::var("DEBUG_SAMPLE_BUFFER").is_ok() {
                            let rms: f32 = (output.iter().map(|x| x*x).sum::<f32>() / output.len() as f32).sqrt();
                            if rms > 0.001 {
                                eprintln!("[SAMPLE_BUFFER] node={}, RMS={:.4}", node_idx, rms);
                            }
                        }
                    } else if !voice_buf.is_empty() {
                        output[..voice_buf.len()].copy_from_slice(voice_buf);
                    }
                    // Else: voice_buffers is empty for this node, output stays at 0.0
                }
                // No fallback - if voice_buffers not available, output stays silent
            }

            SignalNode::ADSR {
                attack,
                decay,
                sustain,
                release,
                state,
            } => {
                // ADSR envelope generator - buffer-based evaluation
                // Evaluates envelope over entire buffer based on cycle position

                // Evaluate modulatable parameters (constant for this buffer)
                let attack_time = self.eval_signal(&attack).max(0.001); // Min 1ms
                let decay_time = self.eval_signal(&decay).max(0.001);
                let sustain_level = self.eval_signal(&sustain).clamp(0.0, 1.0);
                let release_time = self.eval_signal(&release).max(0.001);

                let mut adsr_state = state.clone();
                let cycle_duration = 1.0 / self.cps;

                // Calculate envelope for each sample in buffer
                for i in 0..buffer_size {
                    // Calculate cycle position for this sample
                    let sample_cycle_pos = self.get_cycle_position_for_sample_offset(i);
                    let cycle_pos = (sample_cycle_pos % 1.0) as f32;
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

                    output[i] = level.clamp(0.0, 1.0);
                    adsr_state.level = output[i];
                    adsr_state.cycle_pos = cycle_pos;
                }

                // Update state in graph with final values
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::ADSR { state: s, .. } = node {
                        *s = adsr_state;
                    }
                }
            }

            SignalNode::AD {
                attack,
                decay,
                state,
            } => {
                // AD envelope generator - buffer-based evaluation

                // Evaluate modulatable parameters (constant for this buffer)
                let attack_time = self.eval_signal(&attack).max(0.001); // Min 1ms
                let decay_time = self.eval_signal(&decay).max(0.001);

                let mut ad_state = state.clone();
                let cycle_duration = 1.0 / self.cps;

                // Calculate envelope for each sample in buffer
                for i in 0..buffer_size {
                    // Calculate cycle position for this sample
                    let sample_cycle_pos = self.get_cycle_position_for_sample_offset(i);
                    let cycle_pos = (sample_cycle_pos % 1.0) as f32;
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

                    output[i] = level.clamp(0.0, 1.0);
                    ad_state.level = output[i];
                    ad_state.cycle_pos = cycle_pos;
                }

                // Update state in graph with final values
                if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
                    let node = Rc::make_mut(node_rc);
                    if let SignalNode::AD { state: s, .. } = node {
                        *s = ad_state;
                    }
                }
            }

            // SynthPattern nodes: Read from voice_buffers (voice triggering was done in Phase 1)
            // CRITICAL: Do NOT call eval_node - that would re-trigger voices!
            SignalNode::SynthPattern { .. } => {
                // Just read from pre-rendered voice buffers
                for i in 0..buffer_size {
                    output[i] = self.voice_buffers.get(node_id.0, i);
                }
            }

            // Fallback: Use old sample-by-sample evaluation for not-yet-migrated nodes
            // CRITICAL: Update current_sample_idx so Sample nodes read correct voice_buffers index
            _ => {
                // Save base cycle position and compute sample increment
                let base_cycle_pos = self.cached_cycle_position;
                let sample_increment = self.cps as f64 / self.sample_rate as f64;

                for i in 0..buffer_size {
                    self.current_sample_idx = i;
                    // Set correct cycle position for this sample (needed for Pattern nodes)
                    self.cached_cycle_position = base_cycle_pos + (i as f64 * sample_increment);
                    // CRITICAL: Clear stateful_value_cache per sample to ensure stateful nodes
                    // (Envelope, ASR, ADSR, etc.) are evaluated properly each sample
                    self.stateful_value_cache.clear();
                    output[i] = self.eval_node(node_id);
                }

                // CRITICAL: Restore cycle position to BASE, not end of buffer!
                // Other nodes evaluated at the same buffer level need the same base position.
                // The main process_buffer_hybrid will advance cycle_position after all outputs.
                self.cached_cycle_position = base_cycle_pos;
            }
        }

        // Cache result to prevent re-evaluation in this buffer render
        // Only cache the result if caching is enabled (during process_buffer)
        // In tests/standalone calls, we only use None markers for cycle detection
        //
        // CRITICAL: NEVER cache Sample nodes - they are STATEFUL (pattern advances with time)
        // Caching would freeze patterns and cause timing issues
        let should_cache = self.buffer_cache_enabled.get() && {
            // Check if this node is a Sample or SynthPattern node (both are STATEFUL)
            // Caching would freeze patterns and cause timing issues
            if let Some(Some(node_rc)) = self.nodes.get(node_id.0) {
                !matches!(&**node_rc, SignalNode::Sample { .. } | SignalNode::SynthPattern { .. })
            } else {
                true // If node doesn't exist, safe to cache (will be silence anyway)
            }
        };

        if should_cache {
            let mut cache = self.buffer_cache.borrow_mut();
            cache.insert(*node_id, Some(output.to_vec()));
        } else {
            // Not caching mode or Sample node - just remove the "being evaluated" marker
            let mut cache = self.buffer_cache.borrow_mut();
            cache.remove(node_id);
        }
    }

    /// Evaluate a signal for an entire buffer
    ///
    /// Fills output buffer with signal values. Handles all Signal variants:
    /// - Value: Constant (fill buffer)
    /// - Node: Node reference (evaluate node buffer)
    /// - Bus: Bus reference (evaluate bus node buffer)
    /// - Pattern: Pattern string (query pattern for each sample)
    /// - Expression: Arithmetic expression (evaluate recursively)
    ///
    /// # Arguments
    /// * `signal` - The signal to evaluate
    /// * `output` - Pre-allocated output buffer to fill
    pub fn eval_signal_buffer(&mut self, signal: &Signal, output: &mut [f32]) {
        match signal {
            Signal::Value(v) => {
                // Constant: fill entire buffer with same value
                output.fill(*v);
            }

            Signal::Node(id) => {
                // Node reference: evaluate node for buffer
                self.eval_node_buffer(id, output);
            }

            Signal::Bus(name) => {
                // Bus reference: evaluate bus node for buffer
                if let Some(&id) = self.buses.get(name) {
                    // In DAG mode, check caches first to avoid infinite recursion
                    if let Some(buffer) = self.dag_buffer_cache.get(&id.0) {
                        let copy_len = output.len().min(buffer.len());
                        output[..copy_len].copy_from_slice(&buffer[..copy_len]);
                        return;
                    }
                    if let Some(buffer) = self.prev_node_buffers.get(&id.0) {
                        let copy_len = output.len().min(buffer.len());
                        output[..copy_len].copy_from_slice(&buffer[..copy_len]);
                        return;
                    }
                    if self.in_dag_processing {
                        output.fill(0.0);
                        return;
                    }
                    self.eval_node_buffer(&id, output);
                } else {
                    // Bus doesn't exist, fill with silence
                    output.fill(0.0);
                }
            }

            Signal::Pattern(pattern_str) => {
                // Pattern: query pattern for each sample in buffer
                // TODO: This could be optimized further by batch querying
                for i in 0..output.len() {
                    output[i] = self.eval_signal(signal); // Use old method for now
                }
            }

            Signal::Expression(expr) => {
                // Arithmetic expression: evaluate recursively
                self.eval_expression_buffer(expr, output);
            }
        }
    }

    /// Evaluate an arithmetic expression for an entire buffer
    ///
    /// Handles: Add, Multiply, Subtract, Divide, Modulo, Scale
    pub fn eval_expression_buffer(&mut self, expr: &SignalExpr, output: &mut [f32]) {
        let buffer_size = output.len();

        match expr {
            SignalExpr::Add(a, b) => {
                // Allocate temporary buffers for operands
                let mut a_buffer = vec![0.0; buffer_size];
                let mut b_buffer = vec![0.0; buffer_size];

                // Evaluate operands
                self.eval_signal_buffer(a, &mut a_buffer);
                self.eval_signal_buffer(b, &mut b_buffer);

                // Element-wise addition
                for i in 0..buffer_size {
                    output[i] = a_buffer[i] + b_buffer[i];
                }
            }

            SignalExpr::Multiply(a, b) => {
                let mut a_buffer = vec![0.0; buffer_size];
                let mut b_buffer = vec![0.0; buffer_size];

                self.eval_signal_buffer(a, &mut a_buffer);
                self.eval_signal_buffer(b, &mut b_buffer);

                for i in 0..buffer_size {
                    output[i] = a_buffer[i] * b_buffer[i];
                }
            }

            SignalExpr::Subtract(a, b) => {
                let mut a_buffer = vec![0.0; buffer_size];
                let mut b_buffer = vec![0.0; buffer_size];

                self.eval_signal_buffer(a, &mut a_buffer);
                self.eval_signal_buffer(b, &mut b_buffer);

                for i in 0..buffer_size {
                    output[i] = a_buffer[i] - b_buffer[i];
                }
            }

            SignalExpr::Divide(a, b) => {
                let mut a_buffer = vec![0.0; buffer_size];
                let mut b_buffer = vec![0.0; buffer_size];

                self.eval_signal_buffer(a, &mut a_buffer);
                self.eval_signal_buffer(b, &mut b_buffer);

                for i in 0..buffer_size {
                    let b_val = b_buffer[i];
                    output[i] = if b_val != 0.0 {
                        a_buffer[i] / b_val
                    } else {
                        0.0
                    };
                }
            }

            SignalExpr::Modulo(a, b) => {
                let mut a_buffer = vec![0.0; buffer_size];
                let mut b_buffer = vec![0.0; buffer_size];

                self.eval_signal_buffer(a, &mut a_buffer);
                self.eval_signal_buffer(b, &mut b_buffer);

                for i in 0..buffer_size {
                    let b_val = b_buffer[i];
                    output[i] = if b_val != 0.0 {
                        a_buffer[i] % b_val
                    } else {
                        0.0
                    };
                }
            }

            SignalExpr::Min(a, b) => {
                let mut a_buffer = vec![0.0; buffer_size];
                let mut b_buffer = vec![0.0; buffer_size];

                self.eval_signal_buffer(a, &mut a_buffer);
                self.eval_signal_buffer(b, &mut b_buffer);

                for i in 0..buffer_size {
                    output[i] = a_buffer[i].min(b_buffer[i]);
                }
            }

            SignalExpr::Scale { input, min, max } => {
                let mut input_buffer = vec![0.0; buffer_size];
                let mut min_buffer = vec![0.0; buffer_size];
                let mut max_buffer = vec![0.0; buffer_size];
                self.eval_signal_buffer(input, &mut input_buffer);
                self.eval_signal_buffer(min, &mut min_buffer);
                self.eval_signal_buffer(max, &mut max_buffer);

                // Scale from [0,1] to [min,max] (pattern-modulatable)
                for i in 0..buffer_size {
                    let range = max_buffer[i] - min_buffer[i];
                    output[i] = min_buffer[i] + input_buffer[i] * range;
                }
            }
        }
    }

    /// Add a Wavetable oscillator node (helper for testing)
    pub fn add_wavetable_node(&mut self, freq: Signal, table: Vec<f32>) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        let node = SignalNode::Wavetable {
            freq,
            state: WavetableState::with_table(table),
        };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Add a Granular synthesis node (helper for testing)
    pub fn add_granular_node(
        &mut self,
        source_buffer: Vec<f32>,
        grain_size_ms: Signal,
        density: Signal,
        pitch: Signal,
    ) -> NodeId {
        let node_id = NodeId(self.nodes.len());

        // Create granular state with pre-loaded source buffer
        let buffer_size = source_buffer.len().max(44100); // At least 1 second
        let mut state = GranularState::new(buffer_size);

        // Copy source buffer into granular state
        for &sample in &source_buffer {
            state.write_sample(sample);
        }

        // Create a constant signal from the source buffer
        // In the actual implementation, we'll use the pre-loaded buffer
        let source_signal = Signal::Value(0.0); // Dummy - state already has buffer

        let node = SignalNode::Granular {
            source: source_signal,
            grain_size_ms,
            density,
            pitch,
            state,
        };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }

    /// Add a pitch shift node (granular synthesis-based pitch shifting)
    /// semitones: pitch shift in semitones (0 = no shift, +12 = octave up, -12 = octave down)
    pub fn add_pitchshift_node(&mut self, input: Signal, semitones: Signal) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        let node = SignalNode::PitchShift {
            input,
            semitones,
            state: PitchShifterState::new(50.0, self.sample_rate), // 50ms grain size
        };
        self.nodes.push(Some(Rc::new(node)));
        node_id
    }
}
