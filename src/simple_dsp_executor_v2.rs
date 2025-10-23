#![allow(unused_assignments, unused_mut)]
#![allow(dead_code)]
//! DSP executor v2 - Supports pattern parameters
//!
//! This executor can generate audio from DSP chains with pattern parameters

use crate::dsp_parameter::DspParameter;
use crate::glicol_dsp_v2::{DspChain, DspEnvironment, DspNode};
use std::collections::HashMap;
use std::f32::consts::PI;

/// Simple oscillator state
struct OscState {
    phase: f32,
}

/// Simple filter state
struct FilterState {
    prev_in: f32,
    prev_out: f32,
}

/// Delay line for delay effects
struct DelayLine {
    buffer: Vec<f32>,
    write_idx: usize,
}

/// Node processor that evaluates parameters per sample
struct NodeProcessor {
    sample_rate: f32,
    osc_states: HashMap<usize, OscState>,
    filter_states: HashMap<usize, FilterState>,
    delay_lines: HashMap<usize, DelayLine>,
    noise_state: u32,
}

impl NodeProcessor {
    fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            osc_states: HashMap::new(),
            filter_states: HashMap::new(),
            delay_lines: HashMap::new(),
            noise_state: 12345,
        }
    }

    /// Evaluate a parameter at a given cycle position
    fn eval_param(
        &self,
        param: &DspParameter,
        cycle_pos: f64,
        references: &HashMap<String, f32>,
    ) -> f32 {
        param.evaluate(cycle_pos, references)
    }

    /// Process a single node
    fn process_node(
        &mut self,
        node_id: usize,
        node: &DspNode,
        input: f32,
        cycle_pos: f64,
        references: &HashMap<String, f32>,
    ) -> f32 {
        match node {
            DspNode::Sin { freq } => {
                let f = self.eval_param(freq, cycle_pos, references);
                let osc = self
                    .osc_states
                    .entry(node_id)
                    .or_insert(OscState { phase: 0.0 });
                let sample = (2.0 * PI * osc.phase).sin();
                osc.phase += f / self.sample_rate;
                if osc.phase >= 1.0 {
                    osc.phase -= 1.0;
                }
                sample
            }

            DspNode::Saw { freq } => {
                let f = self.eval_param(freq, cycle_pos, references);
                let osc = self
                    .osc_states
                    .entry(node_id)
                    .or_insert(OscState { phase: 0.0 });
                let sample = 2.0 * osc.phase - 1.0;
                osc.phase += f / self.sample_rate;
                if osc.phase >= 1.0 {
                    osc.phase -= 1.0;
                }
                sample
            }

            DspNode::Square { freq, duty } => {
                let f = self.eval_param(freq, cycle_pos, references);
                let d = self.eval_param(duty, cycle_pos, references);
                let osc = self
                    .osc_states
                    .entry(node_id)
                    .or_insert(OscState { phase: 0.0 });
                let sample = if osc.phase < d { 1.0 } else { -1.0 };
                osc.phase += f / self.sample_rate;
                if osc.phase >= 1.0 {
                    osc.phase -= 1.0;
                }
                sample
            }

            DspNode::Triangle { freq } => {
                let f = self.eval_param(freq, cycle_pos, references);
                let osc = self
                    .osc_states
                    .entry(node_id)
                    .or_insert(OscState { phase: 0.0 });
                let sample = if osc.phase < 0.5 {
                    4.0 * osc.phase - 1.0
                } else {
                    3.0 - 4.0 * osc.phase
                };
                osc.phase += f / self.sample_rate;
                if osc.phase >= 1.0 {
                    osc.phase -= 1.0;
                }
                sample
            }

            DspNode::Noise { seed } => {
                // Simple white noise using linear congruential generator
                self.noise_state = self
                    .noise_state
                    .wrapping_mul(1103515245)
                    .wrapping_add(12345);
                ((self.noise_state / 65536) % 32768) as f32 / 16384.0 - 1.0
            }

            DspNode::Lpf { cutoff, q } => {
                let fc = self
                    .eval_param(cutoff, cycle_pos, references)
                    .max(20.0)
                    .min(20000.0);
                let resonance = self.eval_param(q, cycle_pos, references).max(0.1).min(10.0);

                let filter = self.filter_states.entry(node_id).or_insert(FilterState {
                    prev_in: 0.0,
                    prev_out: 0.0,
                });

                // Simple one-pole lowpass filter
                let freq_normalized = fc / self.sample_rate;
                let rc = 1.0 / (2.0 * PI * freq_normalized);
                let dt = 1.0;
                let alpha = dt / (rc + dt);

                // Apply simple RC lowpass
                let output = filter.prev_out + alpha * (input - filter.prev_out);
                filter.prev_out = output;

                output
            }

            DspNode::Hpf { cutoff, q } => {
                let fc = self
                    .eval_param(cutoff, cycle_pos, references)
                    .max(20.0)
                    .min(20000.0);
                let _resonance = self.eval_param(q, cycle_pos, references).max(0.1).min(10.0);

                let filter = self.filter_states.entry(node_id).or_insert(FilterState {
                    prev_in: 0.0,
                    prev_out: 0.0,
                });

                // Simple one-pole highpass filter
                let freq_normalized = fc / self.sample_rate;
                let rc = 1.0 / (2.0 * PI * freq_normalized);
                let dt = 1.0;
                let alpha = rc / (rc + dt);

                // Apply simple RC highpass
                let output = alpha * (filter.prev_out + input - filter.prev_in);
                filter.prev_in = input;
                filter.prev_out = output;

                output
            }

            DspNode::Mul { factor } => {
                let v = self.eval_param(factor, cycle_pos, references);
                input * v
            }

            DspNode::Add { value } => {
                let v = self.eval_param(value, cycle_pos, references);
                input + v
            }

            DspNode::Delay {
                time,
                feedback,
                mix,
            } => {
                let delay_time = self.eval_param(time, cycle_pos, references);
                let fb = self.eval_param(feedback, cycle_pos, references);
                let mix_val = self.eval_param(mix, cycle_pos, references);

                let delay_samples = (delay_time * self.sample_rate) as usize;

                let delay_line = self.delay_lines.entry(node_id).or_insert_with(|| {
                    DelayLine {
                        buffer: vec![0.0; (self.sample_rate * 2.0) as usize], // Max 2 seconds
                        write_idx: 0,
                    }
                });

                let delay_samples = delay_samples.min(delay_line.buffer.len() - 1);

                // Read from delay line
                let read_idx = (delay_line.write_idx + delay_line.buffer.len() - delay_samples)
                    % delay_line.buffer.len();
                let delayed = delay_line.buffer[read_idx];

                // Write to delay line (input + feedback)
                delay_line.buffer[delay_line.write_idx] = input + delayed * fb;
                delay_line.write_idx = (delay_line.write_idx + 1) % delay_line.buffer.len();

                // Mix dry and wet
                input * (1.0 - mix_val) + delayed * mix_val
            }

            DspNode::Reverb {
                room_size,
                damping,
                mix,
            } => {
                let room = self.eval_param(room_size, cycle_pos, references);
                let damp = self.eval_param(damping, cycle_pos, references);
                let mix_val = self.eval_param(mix, cycle_pos, references);

                // Very simple reverb using delay
                let delay_time = 0.05 * room;
                let fb = 0.5 * (1.0 - damp);
                let delay_samples = (delay_time * self.sample_rate) as usize;

                let delay_line = self
                    .delay_lines
                    .entry(node_id)
                    .or_insert_with(|| DelayLine {
                        buffer: vec![0.0; (self.sample_rate * 2.0) as usize],
                        write_idx: 0,
                    });

                let delay_samples = delay_samples.min(delay_line.buffer.len() - 1);

                let read_idx = (delay_line.write_idx + delay_line.buffer.len() - delay_samples)
                    % delay_line.buffer.len();
                let delayed = delay_line.buffer[read_idx];

                delay_line.buffer[delay_line.write_idx] = input + delayed * fb;
                delay_line.write_idx = (delay_line.write_idx + 1) % delay_line.buffer.len();

                input * (1.0 - mix_val) + delayed * mix_val
            }

            // Reference to another chain
            DspNode::Ref { name } => {
                // Get value from references
                references.get(name).copied().unwrap_or(input)
            }

            // Signal arithmetic
            DspNode::SignalAdd { left, right } => {
                // For signal arithmetic, we'd need to process both chains
                // For now, just return input
                input
            }

            DspNode::SignalMul { left, right } => {
                // For signal arithmetic, we'd need to process both chains
                // For now, just return input
                input
            }

            DspNode::SignalSub { left, right } => input,

            DspNode::SignalDiv { left, right } => input,

            // Pattern sampler
            DspNode::S { pattern } => {
                // This would trigger samples based on pattern
                // For now, return silence
                0.0
            }

            _ => input,
        }
    }
}

/// DSP executor v2 with pattern parameter support
pub struct SimpleDspExecutorV2 {
    sample_rate: f32,
    processor: NodeProcessor,
    cps: f32, // Cycles per second
}

impl SimpleDspExecutorV2 {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            processor: NodeProcessor::new(sample_rate),
            cps: 0.5, // Default 0.5 cycles per second
        }
    }

    pub fn set_cps(&mut self, cps: f32) {
        self.cps = cps;
    }

    /// Render a DSP chain to audio
    pub fn render_chain(&mut self, chain: &DspChain, duration_secs: f32) -> Vec<f32> {
        let num_samples = (self.sample_rate * duration_secs) as usize;
        let mut output = vec![0.0; num_samples];
        let references = HashMap::new();

        for i in 0..num_samples {
            // Calculate current cycle position
            let time = i as f32 / self.sample_rate;
            let cycle_pos = (time * self.cps) as f64;

            // Process the chain
            let mut signal = 0.0;
            for (node_idx, node) in chain.nodes.iter().enumerate() {
                signal =
                    self.processor
                        .process_node(node_idx, node, signal, cycle_pos, &references);
            }

            output[i] = signal;
        }

        output
    }

    /// Render a complete DSP environment to audio
    pub fn render(&mut self, env: &DspEnvironment, duration_secs: f32) -> Result<Vec<f32>, String> {
        let num_samples = (self.sample_rate * duration_secs) as usize;
        let mut output = vec![0.0; num_samples];

        // Process sample by sample to handle references properly
        for i in 0..num_samples {
            // Calculate current cycle position
            let time = i as f32 / self.sample_rate;
            let cycle_pos = (time * self.cps) as f64;

            // First evaluate all named chains for this sample
            let mut references = HashMap::new();
            for (name, chain) in &env.chains {
                let mut signal = 0.0;
                for (node_idx, node) in chain.nodes.iter().enumerate() {
                    signal = self.processor.process_node(
                        node_idx + name.len(), // Unique node ID
                        node,
                        signal,
                        cycle_pos,
                        &references,
                    );
                }
                references.insert(name.clone(), signal);
            }

            // Then render the output chain with references available
            if let Some(output_chain) = &env.output {
                let mut signal = 0.0;
                for (node_idx, node) in output_chain.nodes.iter().enumerate() {
                    signal = self.processor.process_node(
                        node_idx + 1000, // Unique node ID for output chain
                        node,
                        signal,
                        cycle_pos,
                        &references,
                    );
                }
                output[i] = signal;
            }
        }

        Ok(output)
    }

    /// Render and return stereo audio (duplicating mono for now)
    pub fn render_stereo(
        &mut self,
        env: &DspEnvironment,
        duration_secs: f32,
    ) -> Result<Vec<f32>, String> {
        let mono = self.render(env, duration_secs)?;
        let mut stereo = Vec::with_capacity(mono.len() * 2);

        for sample in mono {
            stereo.push(sample); // Left
            stereo.push(sample); // Right
        }

        Ok(stereo)
    }
}
