//! Unified Signal Graph - The heart of Phonon
//!
//! Everything is a signal. Patterns, audio, control data - all flow through
//! one unified graph where anything can modulate anything.

use crate::mini_notation_v3::parse_mini_notation;
use crate::pattern::{Fraction, Pattern, State, TimeSpan};
use crate::sample_loader::SampleBank;
use crate::voice_manager::VoiceManager;
use std::cell::RefCell;
use std::collections::HashMap;
use std::f32::consts::PI;

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

/// Types of nodes in the unified graph
#[derive(Debug, Clone)]
pub enum SignalNode {
    // === Sources ===
    /// Oscillator with modulatable frequency
    Oscillator {
        freq: Signal,
        waveform: Waveform,
        phase: f32,
    },

    /// Pattern as a signal source
    Pattern {
        pattern_str: String,
        pattern: Pattern<String>,
        last_value: f32,
        last_trigger_time: f32, // Cycle position of last trigger
    },

    /// Sample player triggered by pattern
    Sample {
        pattern_str: String,
        pattern: Pattern<String>,
        last_trigger_time: f32,
        playback_positions: HashMap<String, usize>,
    },

    /// Voice output - outputs mixed audio from all triggered samples
    /// This allows sample playback to be routed through effects
    VoiceOutput,

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

    /// Envelope generator
    Envelope {
        input: Signal,
        trigger: Signal,
        attack: f32,
        decay: f32,
        sustain: f32,
        release: f32,
        state: EnvState,
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
    RMS {
        input: Signal,
        window_size: f32,
        buffer: Vec<f32>,
        write_idx: usize,
    },

    /// Pitch detector
    Pitch { input: Signal, last_pitch: f32 },

    /// Transient detector
    Transient {
        input: Signal,
        threshold: f32,
        last_value: f32,
    },

    // === Math & Control ===
    /// Addition
    Add { a: Signal, b: Signal },

    /// Multiplication
    Multiply { a: Signal, b: Signal },

    /// Conditional gate
    When { input: Signal, condition: Signal },

    /// Signal router to multiple destinations
    Router {
        input: Signal,
        destinations: Vec<(NodeId, f32)>, // (target, amount)
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

/// Envelope state
#[derive(Debug, Clone)]
pub struct EnvState {
    phase: EnvPhase,
    level: f32,
    time_in_phase: f32,
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
        }
    }
}

/// The unified signal graph that processes everything
pub struct UnifiedSignalGraph {
    /// All nodes in the graph
    nodes: Vec<Option<SignalNode>>,

    /// Named buses for easy reference
    buses: HashMap<String, NodeId>,

    /// Output node ID
    output: Option<NodeId>,

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

    /// Sample counter for debugging
    sample_count: usize,
}

impl UnifiedSignalGraph {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            nodes: Vec::new(),
            buses: HashMap::new(),
            output: None,
            sample_rate,
            cycle_position: 0.0,
            cps: 0.5, // Default 0.5 cycles per second
            next_node_id: 0,
            value_cache: HashMap::new(),
            sample_bank: RefCell::new(SampleBank::new()),
            voice_manager: RefCell::new(VoiceManager::new()),
            sample_count: 0,
        }
    }

    pub fn set_cps(&mut self, cps: f32) {
        self.cps = cps;
    }

    pub fn get_cps(&self) -> f32 {
        self.cps
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

    /// Set the output node
    pub fn set_output(&mut self, node_id: NodeId) {
        self.output = Some(node_id);
    }

    /// Check if output is set
    pub fn has_output(&self) -> bool {
        self.output.is_some()
    }

    /// Evaluate a signal to get its current value
    fn eval_signal(&mut self, signal: &Signal) -> f32 {
        match signal {
            Signal::Node(id) => self.eval_node(id),
            Signal::Bus(name) => {
                if let Some(id) = self.buses.get(name).cloned() {
                    self.eval_node(&id)
                } else {
                    0.0
                }
            }
            Signal::Pattern(pattern_str) => {
                // Parse and evaluate pattern at current cycle position
                let pattern = parse_mini_notation(pattern_str);
                let state = State {
                    span: TimeSpan::new(
                        Fraction::from_float(self.cycle_position),
                        Fraction::from_float(self.cycle_position + 0.001),
                    ),
                    controls: HashMap::new(),
                };

                let events = pattern.query(&state);
                if let Some(event) = events.first() {
                    // Convert pattern value to float (simplified)
                    match event.value.as_str() {
                        "bd" | "kick" => 1.0,
                        "sn" | "snare" => 0.8,
                        "hh" | "hat" => 0.6,
                        "~" | "" => 0.0,
                        s => {
                            // Try to parse as number
                            s.parse::<f32>().unwrap_or(1.0)
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
            } => {
                let f = self.eval_signal(&freq);
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

                // Update phase for next sample
                if let Some(Some(node)) = self.nodes.get_mut(node_id.0) {
                    if let SignalNode::Oscillator { phase: p, .. } = node {
                        *p += f / self.sample_rate;
                        if *p >= 1.0 {
                            *p -= 1.0;
                        }
                    }
                }

                sample
            }

            SignalNode::Constant { value } => value,

            SignalNode::Add { a, b } => self.eval_signal(&a) + self.eval_signal(&b),

            SignalNode::Multiply { a, b } => self.eval_signal(&a) * self.eval_signal(&b),

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

            SignalNode::Output { input } => self.eval_signal(&input),

            SignalNode::Pattern {
                pattern_str,
                pattern,
                last_value,
                last_trigger_time,
            } => {
                // Query pattern for events at current cycle position
                // Use absolute cycle position for alternation to work correctly
                let sample_width = 1.0 / self.sample_rate as f64 / self.cps as f64;
                let state = State {
                    span: TimeSpan::new(
                        Fraction::from_float(self.cycle_position),
                        Fraction::from_float(self.cycle_position + sample_width),
                    ),
                    controls: HashMap::new(),
                };

                let events = pattern.query(&state);
                let cycle_frac = self.cycle_position.fract();
                let mut trigger_value = last_value; // Hold last value between triggers

                if let Some(event) = events.first() {
                    // Skip rests
                    if event.value.trim() != "~" && !event.value.is_empty() {
                        // Get event start time (absolute cycle position)
                        let event_start_abs = if let Some(whole) = &event.whole {
                            whole.begin.to_float()
                        } else {
                            event.part.begin.to_float()
                        };

                        // Get fractional part for comparison with cycle_frac
                        let event_start_frac = event_start_abs.fract();

                        // Check if we're at the start of the event
                        let at_event_start = (cycle_frac - event_start_frac).abs() < sample_width * 2.0;

                        if at_event_start {
                            // Check if we haven't already triggered this event (compare absolute times)
                            let already_triggered = (last_trigger_time as f64 - event_start_abs).abs() < sample_width;

                            if !already_triggered {
                                // Generate trigger pulse - convert pattern value to amplitude
                                trigger_value = match event.value.as_str() {
                                    "bd" | "kick" => 1.0,
                                    "sn" | "snare" => 0.8,
                                    "hh" | "hat" => 0.6,
                                    s => s.parse::<f32>().unwrap_or(1.0),
                                };

                                // Update last trigger time (store absolute position)
                                if let Some(Some(SignalNode::Pattern { last_trigger_time: ltt, last_value: lv, .. })) =
                                    self.nodes.get_mut(node_id.0)
                                {
                                    *ltt = event_start_abs as f32;
                                    *lv = trigger_value;
                                }
                            }
                        }
                    }
                }

                trigger_value
            }

            SignalNode::Sample {
                pattern_str,
                pattern,
                last_trigger_time,
                playback_positions: _,
            } => {
                // Query pattern for events at current cycle position
                // Use absolute cycle position for alternation to work correctly
                let sample_width = 1.0 / self.sample_rate as f64 / self.cps as f64;
                let state = State {
                    span: TimeSpan::new(
                        Fraction::from_float(self.cycle_position),
                        Fraction::from_float(self.cycle_position + sample_width),
                    ),
                    controls: HashMap::new(),
                };
                let events = pattern.query(&state);

                // Get the last EVENT start time we triggered
                // This is the actual event start time, not cycle position
                let last_event_start = if let Some(Some(SignalNode::Sample { last_trigger_time: lt, .. })) = self.nodes.get(node_id.0) {
                    *lt as f64
                } else {
                    -1.0
                };

                // Track the latest event start time we trigger in this sample
                let mut latest_triggered_start = last_event_start;

                // Trigger voices for ALL new events
                // An event should be triggered if its START is after the last event we triggered
                for event in events.iter() {
                    let sample_name = event.value.trim();

                    // Skip rests
                    if sample_name == "~" || sample_name.is_empty() {
                        continue;
                    }

                    // Get the event start time (absolute cycle position)
                    let event_start_abs = if let Some(whole) = &event.whole {
                        whole.begin.to_float()
                    } else {
                        event.part.begin.to_float()
                    };

                    // Only trigger events that start AFTER the last event we triggered
                    // Use a very small tolerance for floating point comparison
                    let tolerance = sample_width * 0.001;
                    let event_is_new = event_start_abs > last_event_start + tolerance;

                    if event_is_new {
                        // Get sample from bank and trigger a new voice
                        if let Some(sample_data) = self.sample_bank.borrow_mut().get_sample(sample_name) {
                            self.voice_manager.borrow_mut().trigger_sample(sample_data, 1.0);

                            // Track this as the latest event we've triggered
                            if event_start_abs > latest_triggered_start {
                                latest_triggered_start = event_start_abs;
                            }
                        }
                    }
                }

                // Update last_trigger_time to the latest event start time we triggered
                // This ensures we don't re-trigger the same events
                if latest_triggered_start > last_event_start {
                    if let Some(Some(SignalNode::Sample { last_trigger_time: lt, .. })) = self.nodes.get_mut(node_id.0) {
                        *lt = latest_triggered_start as f32;
                    }
                }

                // Sample nodes trigger voices AND output the voice audio
                // This allows them to work standalone or be routed through effects
                self.voice_manager.borrow_mut().process()
            }

            SignalNode::VoiceOutput => {
                // Output the mixed audio from all active voices
                // This is the same as what Sample nodes output,
                // provided as an explicit node for clarity
                self.voice_manager.borrow_mut().process()
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
                            env_state.level *= (1.0 - env_state.time_in_phase / release);
                            if env_state.level <= 0.0 {
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

                // Calculate RMS
                let sum: f32 = buffer.iter().sum();
                (sum / buffer.len() as f32).sqrt()
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

        // Process output node if it exists
        // Note: Voices are now part of the signal graph via VoiceOutput nodes
        // They will be processed during graph evaluation
        let output = if let Some(output_id) = self.output {
            self.eval_node(&output_id)
        } else {
            0.0
        };

        // Advance cycle position
        self.cycle_position += self.cps as f64 / self.sample_rate as f64;

        // Increment sample counter
        self.sample_count += 1;

        output
    }

    /// Render a buffer of audio
    pub fn render(&mut self, num_samples: usize) -> Vec<f32> {
        let mut buffer = Vec::with_capacity(num_samples);
        for _ in 0..num_samples {
            buffer.push(self.process_sample());
        }
        buffer
    }
}
