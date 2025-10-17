//! # Phonon - Live Coding Audio System
//!
//! Phonon is a powerful live coding audio synthesis and pattern sequencing system,
//! inspired by Tidal Cycles and SuperCollider. It combines the expressiveness of
//! pattern languages with the flexibility of modular synthesis.
//!
//! ## Core Features
//!
//! - **Pattern-Based Sequencing**: Tidal Cycles mini-notation for rhythmic patterns
//! - **Unified Signal Graph**: Everything is a signal that can modulate everything
//! - **Polyphonic Sample Playback**: 64-voice engine with per-voice DSP control
//! - **Modular Synthesis**: Oscillators, filters, envelopes, and effects
//! - **Synth Library**: 7 SuperDirt-inspired synthesizers (kick, snare, hat, saw, pwm, chip, fm)
//! - **Audio Effects**: Reverb, distortion, bitcrusher, and chorus effects
//! - **Cross-Modulation**: Patterns can control audio parameters, audio can control patterns
//! - **Multi-Output Routing**: Route different patterns to different output channels
//! - **Live Coding Support**: Real-time code evaluation and hot-reloading
//!
//! ## Quick Start
//!
//! ### Basic Sample Playback
//!
//! ```rust
//! use phonon::unified_graph::{UnifiedSignalGraph, SignalNode, Signal};
//! use phonon::mini_notation_v3::parse_mini_notation;
//! use std::collections::HashMap;
//!
//! let mut graph = UnifiedSignalGraph::new(44100.0);
//! graph.set_cps(2.0); // 2 cycles per second = 120 BPM
//!
//! // Create a kick/snare pattern
//! let pattern = parse_mini_notation("bd ~ bd ~ ~ sn ~ ~");
//! let sample_node = graph.add_node(SignalNode::Sample {
//!     pattern_str: "bd ~ bd ~ ~ sn ~ ~".to_string(),
//!     pattern,
//!     last_trigger_time: -1.0,
//!     playback_positions: HashMap::new(),
//!     gain: Signal::Value(1.0),
//!     pan: Signal::Value(0.0),
//!     speed: Signal::Value(1.0),
//!     cut_group: Signal::Value(0.0),
//! });
//!
//! graph.set_output(sample_node);
//!
//! // Render audio
//! let buffer = graph.render(44100); // 1 second
//! ```
//!
//! ### Pattern-Controlled Parameters
//!
//! ```rust
//! use phonon::unified_graph::{UnifiedSignalGraph, SignalNode, Signal};
//! use phonon::mini_notation_v3::parse_mini_notation;
//! use std::collections::HashMap;
//!
//! let mut graph = UnifiedSignalGraph::new(44100.0);
//! graph.set_cps(2.0);
//!
//! // Create a panning pattern
//! let pan_pattern = parse_mini_notation("-1 0 1 0"); // left, center, right, center
//! let pan_node = graph.add_node(SignalNode::Pattern {
//!     pattern_str: "-1 0 1 0".to_string(),
//!     pattern: pan_pattern,
//!     last_value: 0.0,
//!     last_trigger_time: -1.0,
//! });
//!
//! // Hi-hats with pattern-controlled panning
//! let pattern = parse_mini_notation("hh*8");
//! let sample_node = graph.add_node(SignalNode::Sample {
//!     pattern_str: "hh*8".to_string(),
//!     pattern,
//!     last_trigger_time: -1.0,
//!     playback_positions: HashMap::new(),
//!     gain: Signal::Value(0.8),
//!     pan: Signal::Node(pan_node), // Pan controlled by pattern!
//!     speed: Signal::Value(1.0),
//!     cut_group: Signal::Value(0.0),
//! });
//!
//! graph.set_output(sample_node);
//! ```
//!
//! ### Synthesis with Filters
//!
//! ```rust
//! use phonon::unified_graph::{UnifiedSignalGraph, SignalNode, Signal, Waveform, FilterState};
//!
//! let mut graph = UnifiedSignalGraph::new(44100.0);
//!
//! // Create a saw wave oscillator
//! let osc = graph.add_node(SignalNode::Oscillator {
//!     freq: Signal::Value(110.0), // A2 note
//!     waveform: Waveform::Saw,
//!     phase: 0.0,
//! });
//!
//! // Add a lowpass filter
//! let filtered = graph.add_node(SignalNode::LowPass {
//!     input: Signal::Node(osc),
//!     cutoff: Signal::Value(1000.0),
//!     q: Signal::Value(2.0),
//!     state: FilterState::default(),
//! });
//!
//! graph.set_output(filtered);
//! ```
//!
//! ## Mini-Notation Pattern Language
//!
//! Phonon uses Tidal Cycles mini-notation for expressive pattern specification:
//!
//! - **`bd sn hh`** - Sequence: play kick, snare, hi-hat
//! - **`bd*4`** - Repeat: play kick 4 times per cycle
//! - **`bd/2`** - Slow: stretch kick over 2 cycles
//! - **`bd ~ ~ ~`** - Rests: kick followed by three silences
//! - **`<bd sn cp>`** - Alternation: alternate each cycle
//! - **`[bd, sn]`** - Layering: play kick and snare simultaneously
//! - **`bd(3,8)`** - Euclidean: 3 kicks distributed over 8 steps
//! - **`bd:0 bd:1`** - Sample selection: choose specific samples
//!
//! ## Architecture
//!
//! ### Main Modules
//!
//! - [`unified_graph`] - Central signal processing graph (start here!)
//! - [`superdirt_synths`] - Synthesizer library and audio effects
//! - [`mini_notation_v3`] - Pattern language parser and evaluator
//! - [`voice_manager`] - Polyphonic voice allocation and sample playback
//! - [`sample_loader`] - Sample loading from dirt-samples
//! - [`pattern`] - Core pattern data structures
//!
//! ### Signal Flow
//!
//! 1. **Patterns** are parsed from mini-notation strings
//! 2. **Nodes** are added to the UnifiedSignalGraph
//! 3. **Signals** connect nodes and enable modulation
//! 4. **Process loop** evaluates the graph at sample rate
//! 5. **Voices** play back triggered samples with DSP parameters
//! 6. **Output** is mixed and sent to audio device
//!
//! ### Using the Synth Library
//!
//! ```rust
//! use phonon::superdirt_synths::SynthLibrary;
//! use phonon::unified_graph::{UnifiedSignalGraph, Signal};
//!
//! let mut graph = UnifiedSignalGraph::new(44100.0);
//! let library = SynthLibrary::new();
//!
//! // Create a supersaw synth at 110 Hz
//! let saw = library.build_supersaw(&mut graph, Signal::Value(110.0), Some(0.5), Some(5));
//!
//! // Add effects: distortion -> chorus -> reverb
//! let distorted = library.add_distortion(&mut graph, saw, 3.0, 0.3);
//! let chorused = library.add_chorus(&mut graph, distorted, 1.0, 0.5, 0.3);
//! let reverbed = library.add_reverb(&mut graph, chorused, 0.7, 0.5, 0.4);
//!
//! graph.set_output(reverbed);
//! let buffer = graph.render(44100); // 1 second
//! ```
//!
//! ## DSP Parameters
//!
//! All sample playback parameters can be controlled by:
//! - **Constants**: `Signal::Value(1.0)`
//! - **Patterns**: `Signal::Pattern("0.5 1.0 0.8")`
//! - **Audio signals**: `Signal::Node(lfo_node)`
//! - **Buses**: `Signal::Bus("control_bus")`
//! - **Expressions**: `Signal::Expression(Box::new(...))`
//!
//! ### Available Parameters
//!
//! - **`gain`**: Amplitude (0.0 to 10.0)
//! - **`pan`**: Stereo position (-1.0 = left, 1.0 = right)
//! - **`speed`**: Playback rate (0.5 = octave down, 2.0 = octave up)
//!
//! ## Examples
//!
//! ### Euclidean Rhythm
//!
//! ```rust
//! use phonon::mini_notation_v3::parse_mini_notation;
//!
//! // 3 hits distributed over 8 steps (tresillo pattern)
//! let pattern = parse_mini_notation("bd(3,8)");
//! // Results in: X..X..X. (where X = hit, . = rest)
//! ```
//!
//! ### Polyrhythm
//!
//! ```rust
//! use phonon::mini_notation_v3::parse_mini_notation;
//!
//! // 3 kicks against 4 snares
//! let pattern = parse_mini_notation("[bd*3, sn*4]");
//! ```
//!
//! ### Multi-Output Routing
//!
//! ```rust
//! use phonon::unified_graph::{UnifiedSignalGraph, SignalNode, Signal};
//! use phonon::mini_notation_v3::parse_mini_notation;
//! use std::collections::HashMap;
//!
//! let mut graph = UnifiedSignalGraph::new(44100.0);
//! graph.set_cps(2.0);
//!
//! // Kick on channel 1
//! let kick_pattern = parse_mini_notation("bd ~ bd ~");
//! let kick_node = graph.add_node(SignalNode::Sample {
//!     pattern_str: "bd ~ bd ~".to_string(),
//!     pattern: kick_pattern,
//!     last_trigger_time: -1.0,
//!     playback_positions: HashMap::new(),
//!     gain: Signal::Value(1.0),
//!     pan: Signal::Value(0.0),
//!     speed: Signal::Value(1.0),
//!     cut_group: Signal::Value(0.0),
//! });
//!
//! // Snare on channel 2
//! let snare_pattern = parse_mini_notation("~ sn ~ sn");
//! let snare_node = graph.add_node(SignalNode::Sample {
//!     pattern_str: "~ sn ~ sn".to_string(),
//!     pattern: snare_pattern,
//!     last_trigger_time: -1.0,
//!     playback_positions: HashMap::new(),
//!     gain: Signal::Value(1.0),
//!     pan: Signal::Value(0.0),
//!     speed: Signal::Value(1.0),
//!     cut_group: Signal::Value(0.0),
//! });
//!
//! graph.set_output_channel(1, kick_node);
//! graph.set_output_channel(2, snare_node);
//!
//! // Process multi-channel output
//! let outputs = graph.process_sample_multi(); // Vec<f32>
//! // outputs[0] = channel 1, outputs[1] = channel 2
//! ```
//!
//! ## Performance
//!
//! - **64 simultaneous voices** with automatic voice stealing
//! - **Sample-accurate timing** for pattern events
//! - **Equal-power panning** for proper stereo imaging
//! - **Linear interpolation** for smooth pitch shifting
//! - **Soft clipping** to prevent distortion from voice mixing
//!
//! ## Sample Format
//!
//! Phonon uses the dirt-samples library format:
//! - Samples organized in folders by name: `bd/`, `sn/`, `hh/`, etc.
//! - WAV files inside each folder numbered: `bd/BT0A0A7.wav`, `bd/BT0AADA.wav`
//! - Access specific samples with colon notation: `"bd:0"`, `"bd:1"`, `"bd:2"`
//!
//! ## See Also
//!
//! - [Tidal Cycles](https://tidalcycles.org/) - Pattern language inspiration
//! - [SuperCollider](https://supercollider.github.io/) - Audio engine architecture
//! - [Strudel](https://strudel.cc/) - Browser-based live coding
//!
//! ## License
//!
//! Phonon is open source. Check the repository for licensing details.

pub mod audio;
pub mod audio_analysis;
pub mod dsl_osc_handler;
pub mod dsp_parameter;
pub mod engine;
pub mod enhanced_parser;
pub mod envelope;
pub mod glicol_dsp;
pub mod glicol_dsp_v2;
pub mod glicol_parser;
pub mod glicol_parser_v2;
pub mod glicol_pattern_bridge;
pub mod live;
pub mod live_engine;
pub mod midi_output;
pub mod mini_notation;
pub mod mini_notation_v3;
pub mod modal_editor;
pub mod modulation_router;
pub mod nom_parser;
pub mod osc_control;
pub mod pattern;
pub mod pattern_bridge;
pub mod pattern_debug;
pub mod pattern_lang_parser;
pub mod pattern_midi;
pub mod pattern_ops;
pub mod pattern_ops_extended;
pub mod pattern_query;
pub mod pattern_sequencer_voice;
pub mod pattern_signal;
pub mod pattern_structure;
pub mod pattern_test;
pub mod pattern_tonal;
pub mod phonon_lang;
pub mod render;
pub mod sample_loader;
pub mod signal_executor;
pub mod signal_graph;
pub mod signal_parser;
pub mod simple_dsp_executor;
pub mod simple_dsp_executor_v2;
pub mod superdirt_synths;
pub mod synth;
pub mod synth_defs;
pub mod synth_voice;
pub mod synth_voice_manager;
mod test_methods;
pub mod unified_graph;
pub mod unified_graph_parser;
pub mod voice_manager;

#[cfg(test)]
pub mod test_utils;
