//! Shared Effect State for Parallel Rendering
//!
//! When rendering audio in parallel, each thread gets a cloned graph. Without shared state,
//! stateful effects like reverbs and delays lose their accumulated state between blocks,
//! causing audio discontinuities.
//!
//! This module provides a registry of shared state that persists across parallel clones.
//! Each stateful node gets an Arc<RwLock<State>> that all clones reference.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::nodes::lush_reverb::LushReverbState;
use crate::unified_graph::{
    ADSRState, ADState, ASRState, AllpassState, BitCrushState, BiquadState,
    BrownNoiseState, ChorusState, CompressorState, ConvolutionState, DattorroState,
    EnvState, ExpanderState, FilterState, FlangerState, FormantState, GranularState,
    ImpulseState, KarplusStrongState, LagState, MoogLadderState, ParametricEQState,
    PinkNoiseState, PitchShifterState, ReverbState, SVFState, SpectralFreezeState,
    TapeDelayState, VocoderState, WaveguideState, WavetableState, XLineState,
    AdditiveState,
};

/// Node identifier for shared state lookup
pub type NodeId = usize;

/// Shared state variants for all stateful nodes
/// Each variant wraps the node's state in Arc<RwLock<>> for thread-safe sharing
#[derive(Clone)]
pub enum SharedState {
    // === Critical: Delay-based effects (buffer accumulation) ===
    /// Lush reverb - complex 8-channel FDN
    LushReverb(Arc<RwLock<LushReverbState>>),
    /// Freeverb-style reverb
    Reverb(Arc<RwLock<ReverbState>>),
    /// Dattorro plate reverb
    Dattorro(Arc<RwLock<DattorroState>>),
    /// Simple delay line (buffer + write_idx)
    Delay(Arc<RwLock<DelayLineState>>),
    /// Tape delay with modulation
    TapeDelay(Arc<RwLock<TapeDelayState>>),
    /// Multi-tap delay
    MultiTapDelay(Arc<RwLock<DelayLineState>>),
    /// Ping-pong stereo delay
    PingPongDelay(Arc<RwLock<PingPongDelayState>>),
    /// Comb filter delay line
    Comb(Arc<RwLock<CombState>>),
    /// Convolution reverb
    Convolution(Arc<RwLock<ConvolutionState>>),

    // === High priority: Filters ===
    /// Generic filter state (LP, HP, BP, Notch, DJFilter)
    Filter(Arc<RwLock<FilterState>>),
    /// State variable filter
    SVF(Arc<RwLock<SVFState>>),
    /// Biquad filter
    Biquad(Arc<RwLock<BiquadState>>),
    /// Allpass filter
    Allpass(Arc<RwLock<AllpassState>>),
    /// Moog ladder filter
    MoogLadder(Arc<RwLock<MoogLadderState>>),
    /// Parametric EQ
    ParametricEQ(Arc<RwLock<ParametricEQState>>),
    /// Formant filter
    Formant(Arc<RwLock<FormantState>>),

    // === High priority: Oscillators ===
    /// Oscillator phase (wrapped f32)
    OscillatorPhase(Arc<RwLock<f32>>),
    /// FM oscillator phases (carrier, modulator)
    FMOscillatorPhase(Arc<RwLock<(f32, f32)>>),
    /// PM oscillator phase
    PMOscillatorPhase(Arc<RwLock<f32>>),
    /// VCO phase
    VCOPhase(Arc<RwLock<f32>>),
    /// Blip phase
    BlipPhase(Arc<RwLock<f32>>),
    /// Wavetable state
    Wavetable(Arc<RwLock<WavetableState>>),

    // === High priority: Envelopes ===
    /// Generic envelope state
    Envelope(Arc<RwLock<EnvState>>),
    /// ADSR envelope
    ADSR(Arc<RwLock<ADSRState>>),
    /// AD envelope
    AD(Arc<RwLock<ADState>>),
    /// ASR envelope
    ASR(Arc<RwLock<ASRState>>),
    /// Lag/slew limiter
    Lag(Arc<RwLock<LagState>>),
    /// XLine exponential envelope
    XLine(Arc<RwLock<XLineState>>),
    /// Impulse generator
    Impulse(Arc<RwLock<ImpulseState>>),

    // === Medium priority: Modulation effects ===
    /// Chorus
    Chorus(Arc<RwLock<ChorusState>>),
    /// Flanger
    Flanger(Arc<RwLock<FlangerState>>),
    /// Bitcrusher
    BitCrush(Arc<RwLock<BitCrushState>>),

    // === Medium priority: Dynamics ===
    /// Compressor
    Compressor(Arc<RwLock<CompressorState>>),
    /// Expander
    Expander(Arc<RwLock<ExpanderState>>),

    // === Medium priority: Synthesis ===
    /// Granular synthesis
    Granular(Arc<RwLock<GranularState>>),
    /// Karplus-Strong
    KarplusStrong(Arc<RwLock<KarplusStrongState>>),
    /// Waveguide
    Waveguide(Arc<RwLock<WaveguideState>>),
    /// Additive synthesis
    Additive(Arc<RwLock<AdditiveState>>),
    /// Vocoder
    Vocoder(Arc<RwLock<VocoderState>>),
    /// Pitch shifter
    PitchShift(Arc<RwLock<PitchShifterState>>),

    // === Lower priority: Noise generators ===
    /// Pink noise
    PinkNoise(Arc<RwLock<PinkNoiseState>>),
    /// Brown noise
    BrownNoise(Arc<RwLock<BrownNoiseState>>),

    // === Lower priority: Analysis ===
    /// RMS buffer
    RMS(Arc<RwLock<RMSState>>),
    /// Spectral freeze
    SpectralFreeze(Arc<RwLock<SpectralFreezeState>>),
    /// Amp follower
    AmpFollower(Arc<RwLock<AmpFollowerState>>),
    /// Peak follower
    PeakFollower(Arc<RwLock<f32>>),

    // === Modulation with inline state ===
    /// Phaser (multiple allpass stages)
    Phaser(Arc<RwLock<PhaserState>>),
    /// Vibrato (delay buffer)
    Vibrato(Arc<RwLock<VibratoState>>),
    /// Tremolo phase
    TremoloPhase(Arc<RwLock<f32>>),
    /// Ring mod phase
    RingModPhase(Arc<RwLock<f32>>),
}

/// Simple delay line state (buffer + write index)
#[derive(Clone, Debug)]
pub struct DelayLineState {
    pub buffer: Vec<f32>,
    pub write_idx: usize,
}

/// Ping-pong delay state (left + right buffers)
#[derive(Clone, Debug)]
pub struct PingPongDelayState {
    pub buffer_l: Vec<f32>,
    pub buffer_r: Vec<f32>,
    pub write_idx: usize,
}

/// Comb filter state
#[derive(Clone, Debug)]
pub struct CombState {
    pub buffer: Vec<f32>,
    pub write_pos: usize,
}

/// RMS analysis state
#[derive(Clone, Debug)]
pub struct RMSState {
    pub buffer: Vec<f32>,
    pub write_idx: usize,
}

/// Amp follower state
#[derive(Clone, Debug)]
pub struct AmpFollowerState {
    pub buffer: Vec<f32>,
    pub write_idx: usize,
    pub current_envelope: f32,
}

/// Phaser state (allpass stages)
#[derive(Clone, Debug)]
pub struct PhaserState {
    pub phase: f32,
    pub allpass_z1: Vec<f32>,
    pub allpass_y1: Vec<f32>,
    pub feedback_sample: f32,
}

/// Vibrato state
#[derive(Clone, Debug)]
pub struct VibratoState {
    pub phase: f32,
    pub delay_buffer: Vec<f32>,
    pub buffer_pos: usize,
}

/// Registry of shared state for all stateful nodes
#[derive(Clone, Default)]
pub struct SharedStateRegistry {
    /// Map from node ID to shared state
    states: Arc<RwLock<HashMap<NodeId, SharedState>>>,
}

impl SharedStateRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a shared state for a node
    pub fn register(&self, node_id: NodeId, state: SharedState) {
        if let Ok(mut states) = self.states.write() {
            states.insert(node_id, state);
        }
    }

    /// Get the shared state for a node
    pub fn get(&self, node_id: NodeId) -> Option<SharedState> {
        if let Ok(states) = self.states.read() {
            states.get(&node_id).cloned()
        } else {
            None
        }
    }

    /// Check if a node has registered shared state
    pub fn contains(&self, node_id: NodeId) -> bool {
        if let Ok(states) = self.states.read() {
            states.contains_key(&node_id)
        } else {
            false
        }
    }

    /// Get the number of registered states
    pub fn len(&self) -> usize {
        if let Ok(states) = self.states.read() {
            states.len()
        } else {
            0
        }
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl std::fmt::Debug for SharedStateRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Ok(states) = self.states.read() {
            f.debug_struct("SharedStateRegistry")
                .field("num_states", &states.len())
                .finish()
        } else {
            f.debug_struct("SharedStateRegistry")
                .field("status", &"locked")
                .finish()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_basic_operations() {
        let registry = SharedStateRegistry::new();
        assert!(registry.is_empty());

        // Register a filter state
        let filter_state = FilterState::default();
        registry.register(0, SharedState::Filter(Arc::new(RwLock::new(filter_state))));

        assert_eq!(registry.len(), 1);
        assert!(registry.contains(0));
        assert!(!registry.contains(1));

        // Get the state back
        let state = registry.get(0);
        assert!(state.is_some());
    }

    #[test]
    fn test_registry_clone_shares_state() {
        let registry = SharedStateRegistry::new();

        // Register a state with a known value
        registry.register(0, SharedState::OscillatorPhase(Arc::new(RwLock::new(0.5))));

        // Clone the registry
        let cloned = registry.clone();

        // Modify through one registry
        if let Some(SharedState::OscillatorPhase(phase)) = registry.get(0) {
            if let Ok(mut p) = phase.write() {
                *p = 0.75;
            }
        }

        // Check that the other registry sees the change
        if let Some(SharedState::OscillatorPhase(phase)) = cloned.get(0) {
            if let Ok(p) = phase.read() {
                assert_eq!(*p, 0.75);
            }
        }
    }
}
