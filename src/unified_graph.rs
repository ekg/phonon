//! Unified Signal Graph - The heart of Phonon
//!
//! Everything is a signal. Patterns, audio, control data - all flow through
//! one unified graph where anything can modulate anything.

use crate::pattern::{Pattern, State, TimeSpan, Fraction};
use crate::mini_notation_v3::parse_mini_notation;
use crate::dsp_parameter::DspParameter;
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
    },

    /// Constant value
    Constant {
        value: f32,
    },

    /// Noise generator
    Noise {
        seed: u32,
    },

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
    Pitch {
        input: Signal,
        last_pitch: f32,
    },

    /// Transient detector
    Transient {
        input: Signal,
        threshold: f32,
        last_value: f32,
    },

    // === Math & Control ===

    /// Addition
    Add {
        a: Signal,
        b: Signal,
    },

    /// Multiplication
    Multiply {
        a: Signal,
        b: Signal,
    },

    /// Conditional gate
    When {
        input: Signal,
        condition: Signal,
    },

    /// Signal router to multiple destinations
    Router {
        input: Signal,
        destinations: Vec<(NodeId, f32)>, // (target, amount)
    },

    /// Output node
    Output {
        input: Signal,
    },
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
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
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
        }
    }

    pub fn set_cps(&mut self, cps: f32) {
        self.cps = cps;
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
            SignalNode::Oscillator { freq, waveform, phase } => {
                let f = self.eval_signal(&freq);
                // Generate sample based on waveform
                let sample = match waveform {
                    Waveform::Sine => (2.0 * PI * phase).sin(),
                    Waveform::Saw => 2.0 * phase - 1.0,
                    Waveform::Square => if phase < 0.5 { 1.0 } else { -1.0 },
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

            SignalNode::Add { a, b } => {
                self.eval_signal(&a) + self.eval_signal(&b)
            }

            SignalNode::Multiply { a, b } => {
                self.eval_signal(&a) * self.eval_signal(&b)
            }

            SignalNode::LowPass { input, cutoff, q, .. } => {
                let input_val = self.eval_signal(&input);
                let fc = self.eval_signal(&cutoff).max(20.0).min(20000.0);
                let q_val = self.eval_signal(&q).max(0.5).min(20.0);

                // State variable filter (Chamberlin)
                // Better frequency response and resonance
                let f = 2.0 * (PI * fc / self.sample_rate).sin();
                let damp = 1.0 / q_val;

                // Get state
                let (mut low, mut band, mut high) = if let Some(Some(SignalNode::LowPass { state, .. })) = self.nodes.get(node_id.0) {
                    (state.y1, state.x1, state.y2)
                } else {
                    (0.0, 0.0, 0.0)
                };

                // Process
                high = input_val - low - damp * band;
                band = band + f * high;
                low = low + f * band;

                // Update state
                if let Some(Some(SignalNode::LowPass { state, .. })) = self.nodes.get_mut(node_id.0) {
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

            SignalNode::Output { input } => {
                self.eval_signal(&input)
            }

            SignalNode::Pattern { pattern_str, pattern, last_value } => {
                // Evaluate pattern at current cycle position
                // Use a point query instead of a span
                let cycle_frac = self.cycle_position.fract();
                let state = State {
                    span: TimeSpan::new(
                        Fraction::from_float(cycle_frac),
                        Fraction::from_float(cycle_frac + 0.0001),
                    ),
                    controls: HashMap::new(),
                };

                let events = pattern.query(&state);
                let value = if let Some(event) = events.first() {
                    // Convert pattern value to float
                    let parsed_value = match event.value.as_str() {
                        "bd" | "kick" => 1.0,
                        "sn" | "snare" => 0.8,
                        "hh" | "hat" => 0.6,
                        "~" | "" => 0.0,
                        s => s.parse::<f32>().unwrap_or(1.0)
                    };


                    parsed_value
                } else {
                    last_value
                };

                // Update last value
                if let Some(Some(SignalNode::Pattern { last_value: lv, .. })) = self.nodes.get_mut(node_id.0) {
                    *lv = value;
                }

                value
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

            SignalNode::HighPass { input, cutoff, q, .. } => {
                let input_val = self.eval_signal(&input);
                let fc = self.eval_signal(&cutoff).max(20.0).min(20000.0);
                let q_val = self.eval_signal(&q).max(0.5).min(20.0);

                // State variable filter (Chamberlin) - high pass output
                let f = 2.0 * (PI * fc / self.sample_rate).sin();
                let damp = 1.0 / q_val;

                // Get state
                let (mut low, mut band, mut high) = if let Some(Some(SignalNode::HighPass { state, .. })) = self.nodes.get(node_id.0) {
                    (state.y1, state.x1, state.y2)
                } else {
                    (0.0, 0.0, 0.0)
                };

                // Process
                high = input_val - low - damp * band;
                band = band + f * high;
                low = low + f * band;

                // Update state
                if let Some(Some(SignalNode::HighPass { state, .. })) = self.nodes.get_mut(node_id.0) {
                    state.y1 = low;
                    state.x1 = band;
                    state.y2 = high;
                }

                high // Output high-pass signal
            }

            SignalNode::Envelope { input, trigger, attack, decay, sustain, release, state } => {
                let input_val = self.eval_signal(&input);
                let trig = self.eval_signal(&trigger);

                // Clone state to work with it
                let mut env_state = state.clone();

                // Check for trigger
                if trig > 0.5 && matches!(env_state.phase, EnvPhase::Idle | EnvPhase::Release) {
                    env_state.phase = EnvPhase::Attack;
                    env_state.time_in_phase = 0.0;
                } else if trig <= 0.5 && matches!(env_state.phase, EnvPhase::Attack | EnvPhase::Decay | EnvPhase::Sustain) {
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
                            env_state.level = 1.0 - (1.0 - sustain) * (env_state.time_in_phase / decay);
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
                            env_state.level = env_state.level * (1.0 - env_state.time_in_phase / release);
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
                if let Some(Some(SignalNode::Envelope { state: s, .. })) = self.nodes.get_mut(node_id.0) {
                    *s = env_state.clone();
                }

                input_val * env_state.level
            }

            SignalNode::Delay { input, time, feedback, mix, buffer, write_idx } => {
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
                let to_write = input_val + delayed * fb;

                // Update buffer and write index
                if let Some(Some(SignalNode::Delay { buffer: buf, write_idx: idx, .. })) = self.nodes.get_mut(node_id.0) {
                    buf[*idx] = to_write;
                    *idx = (*idx + 1) % buf.len();
                }

                // Mix dry and wet
                input_val * (1.0 - mix_val) + delayed * mix_val
            }

            SignalNode::RMS { input, window_size, buffer, write_idx } => {
                let input_val = self.eval_signal(&input);

                // Update buffer
                if let Some(Some(SignalNode::RMS { buffer: buf, write_idx: idx, .. })) = self.nodes.get_mut(node_id.0) {
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

            SignalNode::Transient { input, threshold, last_value } => {
                let input_val = self.eval_signal(&input).abs();
                let last = last_value;

                // Simple transient detection based on energy increase
                let transient = if input_val > last * (1.0 + threshold) {
                    1.0
                } else {
                    0.0
                };

                // Update last value
                if let Some(Some(SignalNode::Transient { last_value: lv, .. })) = self.nodes.get_mut(node_id.0) {
                    *lv = input_val;
                }

                transient
            }

            SignalNode::Router { input, destinations: _ } => {
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
        let output = if let Some(output_id) = self.output.clone() {
            self.eval_node(&output_id)
        } else {
            0.0
        };

        // Advance cycle position
        self.cycle_position += self.cps as f64 / self.sample_rate as f64;

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