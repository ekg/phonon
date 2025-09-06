//! Simple DSP executor for testing
//! 
//! A straightforward implementation that generates audio from DSP chains

use crate::glicol_dsp::{DspChain, DspNode, DspEnvironment};
use crate::signal_executor::AudioBuffer;
use std::collections::HashMap;
use std::f32::consts::PI;

/// Simple oscillator state
struct OscState {
    phase: f32,
    freq: f32,
}

/// Simple filter state
struct FilterState {
    prev_in: f32,
    prev_out: f32,
}

/// DSP processor that can generate audio from a single node
struct NodeProcessor {
    sample_rate: f32,
    time: f32,
    osc_state: OscState,
    filter_state: FilterState,
    noise_state: u32,
}

impl NodeProcessor {
    fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            time: 0.0,
            osc_state: OscState { phase: 0.0, freq: 0.0 },
            filter_state: FilterState { prev_in: 0.0, prev_out: 0.0 },
            noise_state: 12345,
        }
    }
    
    fn process_node(&mut self, node: &DspNode, input: f32) -> f32 {
        match node {
            DspNode::Sin { freq } => {
                self.osc_state.freq = *freq as f32;
                let sample = (2.0 * PI * self.osc_state.phase).sin();
                self.osc_state.phase += *freq as f32 / self.sample_rate;
                if self.osc_state.phase >= 1.0 {
                    self.osc_state.phase -= 1.0;
                }
                sample
            }
            
            DspNode::Saw { freq } => {
                self.osc_state.freq = *freq as f32;
                let sample = 2.0 * self.osc_state.phase - 1.0;
                self.osc_state.phase += *freq as f32 / self.sample_rate;
                if self.osc_state.phase >= 1.0 {
                    self.osc_state.phase -= 1.0;
                }
                sample
            }
            
            DspNode::Square { freq } => {
                self.osc_state.freq = *freq as f32;
                let sample = if self.osc_state.phase < 0.5 { 1.0 } else { -1.0 };
                self.osc_state.phase += *freq as f32 / self.sample_rate;
                if self.osc_state.phase >= 1.0 {
                    self.osc_state.phase -= 1.0;
                }
                sample
            }
            
            DspNode::Triangle { freq } => {
                self.osc_state.freq = *freq as f32;
                let sample = if self.osc_state.phase < 0.5 {
                    4.0 * self.osc_state.phase - 1.0
                } else {
                    3.0 - 4.0 * self.osc_state.phase
                };
                self.osc_state.phase += *freq as f32 / self.sample_rate;
                if self.osc_state.phase >= 1.0 {
                    self.osc_state.phase -= 1.0;
                }
                sample
            }
            
            DspNode::Noise => {
                // Simple white noise using LCG
                self.noise_state = self.noise_state.wrapping_mul(1103515245).wrapping_add(12345);
                ((self.noise_state / 65536) % 32768) as f32 / 16384.0 - 1.0
            }
            
            DspNode::Pink => {
                // Simplified pink noise (just filtered white noise)
                let white = self.process_node(&DspNode::Noise, 0.0);
                self.filter_state.prev_out = 0.9 * self.filter_state.prev_out + 0.1 * white;
                self.filter_state.prev_out
            }
            
            DspNode::Brown => {
                // Brown noise (integrated white noise)
                let white = self.process_node(&DspNode::Noise, 0.0);
                self.filter_state.prev_out += 0.02 * white;
                self.filter_state.prev_out = self.filter_state.prev_out.max(-1.0).min(1.0);
                self.filter_state.prev_out
            }
            
            DspNode::Impulse { freq } => {
                let sample = if self.osc_state.phase < 0.01 { 1.0 } else { 0.0 };
                self.osc_state.phase += *freq as f32 / self.sample_rate;
                if self.osc_state.phase >= 1.0 {
                    self.osc_state.phase -= 1.0;
                }
                sample
            }
            
            // Math operations
            DspNode::Mul { value } => input * (*value as f32),
            DspNode::Add { value } => input + (*value as f32),
            DspNode::Sub { value } => input - (*value as f32),
            DspNode::Div { value } => {
                if *value != 0.0 {
                    input / (*value as f32)
                } else {
                    0.0
                }
            }
            
            // Filters (simplified)
            DspNode::Lpf { cutoff, q: _ } => {
                // Simple one-pole lowpass
                let rc = 1.0 / (2.0 * PI * (*cutoff as f32));
                let dt = 1.0 / self.sample_rate;
                let alpha = dt / (rc + dt);
                self.filter_state.prev_out = self.filter_state.prev_out + alpha * (input - self.filter_state.prev_out);
                self.filter_state.prev_out
            }
            
            DspNode::Hpf { cutoff, q: _ } => {
                // Simple one-pole highpass
                let rc = 1.0 / (2.0 * PI * (*cutoff as f32));
                let dt = 1.0 / self.sample_rate;
                let alpha = rc / (rc + dt);
                let output = alpha * (self.filter_state.prev_out + input - self.filter_state.prev_in);
                self.filter_state.prev_in = input;
                self.filter_state.prev_out = output;
                output
            }
            
            // Clipping
            DspNode::Clip { min, max } => {
                input.max(*min as f32).min(*max as f32)
            }
            
            // Envelope (simplified ADSR)
            DspNode::Env { stages } => {
                if stages.len() >= 4 {
                    let attack = stages[0].0 as f32;
                    let decay = stages[1].0 as f32;
                    let sustain = stages[1].1 as f32;
                    let release = stages[3].0 as f32;
                    
                    // Simple envelope based on time
                    let env = if self.time < attack {
                        self.time / attack
                    } else if self.time < attack + decay {
                        1.0 - (1.0 - sustain) * ((self.time - attack) / decay)
                    } else if self.time < attack + decay + 0.1 { // Hold time
                        sustain
                    } else {
                        sustain * (1.0 - (self.time - attack - decay - 0.1) / release).max(0.0)
                    };
                    
                    input * env
                } else {
                    input
                }
            }
            
            // For other nodes, pass through or return 0
            _ => input,
        }
    }
    
    fn tick(&mut self) {
        self.time += 1.0 / self.sample_rate;
    }
}

/// Simple DSP executor
pub struct SimpleDspExecutor {
    sample_rate: f32,
    buses: HashMap<String, Vec<f32>>,
}

impl SimpleDspExecutor {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            buses: HashMap::new(),
        }
    }
    
    /// Render a DSP environment to audio
    pub fn render(&mut self, env: &DspEnvironment, duration_secs: f32) -> Result<AudioBuffer, String> {
        // First render all reference chains
        for (name, chain) in &env.ref_chains {
            let samples = self.render_chain(chain, duration_secs)?;
            self.buses.insert(name.clone(), samples);
        }
        
        // Then render the output chain
        if let Some(output_chain) = &env.output_chain {
            let samples = self.render_chain(output_chain, duration_secs)?;
            let mut buffer = AudioBuffer::mono(samples.len(), self.sample_rate);
            buffer.data = samples;
            Ok(buffer)
        } else {
            Err("No output chain defined".to_string())
        }
    }
    
    /// Render a single DSP chain
    fn render_chain(&mut self, chain: &DspChain, duration_secs: f32) -> Result<Vec<f32>, String> {
        let num_samples = (self.sample_rate * duration_secs) as usize;
        let mut output = vec![0.0; num_samples];
        
        // Handle Mix nodes specially
        if chain.nodes.len() == 1 {
            if let DspNode::Mix { sources } = &chain.nodes[0] {
                // Render each source and sum them
                for source in sources {
                    let source_samples = self.render_chain(source, duration_secs)?;
                    for (i, sample) in source_samples.iter().enumerate() {
                        if i < output.len() {
                            output[i] += sample;
                        }
                    }
                }
                return Ok(output);
            }
        }
        
        // Create processors for each node
        let mut processors: Vec<NodeProcessor> = chain.nodes.iter()
            .map(|_| NodeProcessor::new(self.sample_rate))
            .collect();
        
        // Process each sample
        for i in 0..num_samples {
            let mut signal = 0.0;
            
            // Process through the chain
            for (j, node) in chain.nodes.iter().enumerate() {
                // Handle references
                if let DspNode::Ref { name } = node {
                    if let Some(bus_data) = self.buses.get(name) {
                        if i < bus_data.len() {
                            signal = bus_data[i];
                        }
                    }
                } else {
                    signal = processors[j].process_node(node, signal);
                }
                processors[j].tick();
            }
            
            output[i] = signal;
        }
        
        Ok(output)
    }
}

/// Render DSP code to audio
pub fn render_dsp_to_audio_simple(code: &str, sample_rate: f32, duration_secs: f32) -> Result<AudioBuffer, String> {
    use crate::glicol_parser::parse_glicol;
    
    let env = parse_glicol(code)?;
    let mut executor = SimpleDspExecutor::new(sample_rate);
    executor.render(&env, duration_secs)
}