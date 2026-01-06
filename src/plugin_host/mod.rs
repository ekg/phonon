//! Plugin Host Module
//!
//! Provides hosting for external audio plugins (VST3, AU, CLAP, LV2).
//! Enables Phonon to load and control professional synthesizers and effects
//! with pattern-controlled parameters.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │  Phonon DSL                                                  │
//! │  ~synth $ vst "PluginName" # param value # note "pattern"   │
//! └─────────────────────────────────────────────────────────────┘
//!                               │
//!                               ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │  PluginRegistry: scan, cache, lookup                        │
//! └─────────────────────────────────────────────────────────────┘
//!                               │
//!                               ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │  PluginInstance: load, process, params, state               │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ```phonon
//! -- Load a synthesizer and control it with patterns
//! ~lfo $ sine 0.25
//! ~virus $ vst "Osirus" # cutoff (~lfo * 0.5 + 0.5) # note "c4 e4 g4"
//! out $ ~virus * 0.7
//! ```

pub mod types;
pub mod registry;
pub mod instance;
pub mod automation;
pub mod preset;
pub mod midi;

// Re-exports for convenience
pub use types::*;
pub use registry::PluginRegistry;
pub use instance::PluginInstanceHandle;
pub use automation::{ParameterMapper, ParameterAutomation};
pub use preset::PhononPreset;
pub use midi::MidiEventBuffer;
