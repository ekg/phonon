#![allow(unused_assignments, unused_mut)]
#![allow(dead_code)]
//! Simple DSP executor for testing
//!
//! A straightforward implementation that generates audio from DSP chains

use crate::glicol_dsp::{DspChain, DspEnvironment, DspNode};
use crate::sample_loader::SampleBank;
use crate::signal_executor::AudioBuffer;
use crate::synth_voice::VoiceAllocator;
use std::cell::RefCell;
use std::collections::HashMap;
use std::f32::consts::PI;
use std::sync::Arc;

/// Simple oscillator state
struct OscState {
    phase: RefCell<f32>,
    freq: f32,
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
    delay_samples: usize,
}

/// Sample playback state
struct SampleState {
    sample_data: Option<Arc<Vec<f32>>>,
    position: f32,
    loop_sample: bool,
}

/// DSP processor that can generate audio from a single node
struct NodeProcessor {
    sample_rate: f32,
    time: f32,
    osc_state: OscState,
    filter_state: FilterState,
    noise_state: u32,
    delay_line: DelayLine,
    sample_state: SampleState,
}

impl NodeProcessor {
    fn new(sample_rate: f32) -> Self {
        // Create a delay line with max 2 seconds of delay
        let max_delay_samples = (sample_rate * 2.0) as usize;
        Self {
            sample_rate,
            time: 0.0,
            osc_state: OscState {
                phase: RefCell::new(0.0),
                freq: 0.0,
            },
            filter_state: FilterState {
                prev_in: 0.0,
                prev_out: 0.0,
            },
            noise_state: 12345,
            delay_line: DelayLine {
                buffer: vec![0.0; max_delay_samples],
                write_idx: 0,
                delay_samples: 0,
            },
            sample_state: SampleState {
                sample_data: None,
                position: 0.0,
                loop_sample: false,
            },
        }
    }

    fn process_node(&mut self, node: &DspNode, input: f32) -> f32 {
        match node {
            DspNode::Sin { freq } => {
                self.osc_state.freq = *freq;
                let sample = (2.0 * PI * *self.osc_state.phase.borrow()).sin();
                {
                    let mut p = self.osc_state.phase.borrow_mut();
                    *p += *freq / self.sample_rate;
                    if *p >= 1.0 {
                        *p -= 1.0;
                    }
                }
                sample
            }

            DspNode::Saw { freq } => {
                self.osc_state.freq = *freq;
                let sample = 2.0 * *self.osc_state.phase.borrow() - 1.0;
                {
                    let mut p = self.osc_state.phase.borrow_mut();
                    *p += *freq / self.sample_rate;
                    if *p >= 1.0 {
                        *p -= 1.0;
                    }
                }
                sample
            }

            DspNode::Square { freq, duty: _ } => {
                self.osc_state.freq = *freq;
                let sample = if *self.osc_state.phase.borrow() < 0.5 {
                    1.0
                } else {
                    -1.0
                };
                {
                    let mut p = self.osc_state.phase.borrow_mut();
                    *p += *freq / self.sample_rate;
                    if *p >= 1.0 {
                        *p -= 1.0;
                    }
                }
                sample
            }

            DspNode::Triangle { freq } => {
                self.osc_state.freq = *freq;
                let phase_val = *self.osc_state.phase.borrow();
                let sample = if phase_val < 0.5 {
                    4.0 * phase_val - 1.0
                } else {
                    3.0 - 4.0 * phase_val
                };
                {
                    let mut p = self.osc_state.phase.borrow_mut();
                    *p += *freq / self.sample_rate;
                    if *p >= 1.0 {
                        *p -= 1.0;
                    }
                }
                sample
            }

            DspNode::Noise { seed: _ } => {
                // Simple white noise using LCG
                self.noise_state = self
                    .noise_state
                    .wrapping_mul(1103515245)
                    .wrapping_add(12345);
                ((self.noise_state / 65536) % 32768) as f32 / 16384.0 - 1.0
            }

            DspNode::Pink { seed: _ } => {
                // Simplified pink noise (just filtered white noise)
                let white = self.process_node(&DspNode::Noise { seed: 42 }, 0.0);
                self.filter_state.prev_out = 0.9 * self.filter_state.prev_out + 0.1 * white;
                self.filter_state.prev_out
            }

            DspNode::Brown { seed: _ } => {
                // Brown noise (integrated white noise)
                let white = self.process_node(&DspNode::Noise { seed: 42 }, 0.0);
                self.filter_state.prev_out += 0.02 * white;
                self.filter_state.prev_out = self.filter_state.prev_out.max(-1.0).min(1.0);
                self.filter_state.prev_out
            }

            DspNode::Impulse { freq } => {
                let sample = if *self.osc_state.phase.borrow() < 0.01 {
                    1.0
                } else {
                    0.0
                };
                {
                    let mut p = self.osc_state.phase.borrow_mut();
                    *p += *freq / self.sample_rate;
                    if *p >= 1.0 {
                        *p -= 1.0;
                    }
                }
                sample
            }

            // Math operations
            DspNode::Mul { factor } => input * (*factor),
            DspNode::Add { value } => input + (*value),
            DspNode::Sub { value } => input - (*value),
            DspNode::Div { divisor } => {
                if *divisor != 0.0 {
                    input / (*divisor)
                } else {
                    0.0
                }
            }

            // Filters (simplified)
            DspNode::Lpf { cutoff, q: _ } => {
                // Simple one-pole lowpass
                let rc = 1.0 / (2.0 * PI * (*cutoff));
                let dt = 1.0 / self.sample_rate;
                let alpha = dt / (rc + dt);
                self.filter_state.prev_out =
                    self.filter_state.prev_out + alpha * (input - self.filter_state.prev_out);
                self.filter_state.prev_out
            }

            DspNode::Hpf { cutoff, q: _ } => {
                // Simple one-pole highpass
                let rc = 1.0 / (2.0 * PI * (*cutoff));
                let dt = 1.0 / self.sample_rate;
                let alpha = rc / (rc + dt);
                let output =
                    alpha * (self.filter_state.prev_out + input - self.filter_state.prev_in);
                self.filter_state.prev_in = input;
                self.filter_state.prev_out = output;
                output
            }

            // Clipping
            DspNode::Clip { min, max } => input.max(*min).min(*max),

            // Envelope (simplified ADSR)
            DspNode::Env { stages } => {
                if stages.len() >= 4 {
                    let attack = stages[0].0;
                    let decay = stages[1].0;
                    let sustain = stages[1].1;
                    let release = stages[3].0;

                    // Simple envelope based on time
                    let env = if self.time < attack {
                        self.time / attack
                    } else if self.time < attack + decay {
                        1.0 - (1.0 - sustain) * ((self.time - attack) / decay)
                    } else if self.time < attack + decay + 0.1 {
                        // Hold time
                        sustain
                    } else {
                        sustain * (1.0 - (self.time - attack - decay - 0.1) / release).max(0.0)
                    };

                    input * env
                } else {
                    input
                }
            }

            // Delay effect
            DspNode::Delay {
                time,
                feedback,
                mix: _,
            } => {
                let delay_time = *time;
                let fb = *feedback;

                // Calculate delay in samples
                self.delay_line.delay_samples = (delay_time * self.sample_rate) as usize;
                self.delay_line.delay_samples = self
                    .delay_line
                    .delay_samples
                    .min(self.delay_line.buffer.len() - 1);

                // Read from delay line
                let read_idx = (self.delay_line.write_idx + self.delay_line.buffer.len()
                    - self.delay_line.delay_samples)
                    % self.delay_line.buffer.len();
                let delayed = self.delay_line.buffer[read_idx];

                // Write to delay line (input + feedback)
                self.delay_line.buffer[self.delay_line.write_idx] = input + delayed * fb;
                self.delay_line.write_idx =
                    (self.delay_line.write_idx + 1) % self.delay_line.buffer.len();

                // Output is input + delayed signal
                input + delayed
            }

            // Reverb (simplified as multiple delays)
            DspNode::Reverb {
                room_size,
                damping,
                mix,
            } => {
                // Very simple reverb using a single delay
                let delay_time = 0.05 * (*room_size);
                let fb = 0.5 * (1.0 - *damping);

                // Calculate delay in samples
                self.delay_line.delay_samples = (delay_time * self.sample_rate) as usize;
                self.delay_line.delay_samples = self
                    .delay_line
                    .delay_samples
                    .min(self.delay_line.buffer.len() - 1);

                // Read from delay line
                let read_idx = (self.delay_line.write_idx + self.delay_line.buffer.len()
                    - self.delay_line.delay_samples)
                    % self.delay_line.buffer.len();
                let delayed = self.delay_line.buffer[read_idx];

                // Simple lowpass filter for damping
                let filtered = delayed * (1.0 - *damping) + self.filter_state.prev_out * (*damping);
                self.filter_state.prev_out = filtered;

                // Write to delay line
                self.delay_line.buffer[self.delay_line.write_idx] = input + filtered * fb;
                self.delay_line.write_idx =
                    (self.delay_line.write_idx + 1) % self.delay_line.buffer.len();

                // Mix dry and wet
                input * 0.7 + delayed * 0.3
            }

            // Sample playback - this is handled at the chain level, not per-node
            DspNode::Sp { sample: _ } => {
                // This should be handled at chain level - return input for now
                input
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
    sample_bank: SampleBank,
    voice_allocator: VoiceAllocator,
    cps: f32, // cycles per second (tempo)
}

impl SimpleDspExecutor {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            buses: HashMap::new(),
            sample_bank: SampleBank::new(),
            voice_allocator: VoiceAllocator::new(16, sample_rate), // 16 voice polyphony
            cps: 0.5, // Default to 0.5 cycles per second (120 BPM)
        }
    }

    /// Set the cycles per second (tempo)
    pub fn set_cps(&mut self, cps: f32) {
        self.cps = cps;
    }

    /// Render a DSP environment to audio
    pub fn render(
        &mut self,
        env: &DspEnvironment,
        duration_secs: f32,
    ) -> Result<AudioBuffer, String> {
        // First register all reference chains as potential synth voices
        for (name, chain) in &env.ref_chains {
            // Register this chain for voice allocation
            self.voice_allocator
                .register_channel(name.clone(), chain.clone());

            // Also render it as a bus for direct references
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

    /// Render a sample pattern using mini-notation
    fn render_sample_pattern(
        &mut self,
        pattern: &str,
        duration_secs: f32,
    ) -> Result<Vec<f32>, String> {
        // Use v3 parser for better nested pattern support
        use crate::mini_notation_v3::parse_mini_notation;
        use crate::pattern::{Fraction, State, TimeSpan};
        use std::collections::HashMap;

        let num_samples = (self.sample_rate * duration_secs) as usize;
        let mut output = vec![0.0; num_samples];

        // Parse the mini-notation pattern
        let parsed_pattern = parse_mini_notation(pattern);

        // Determine how many cycles fit in the duration
        let cycle_duration = 1.0 / self.cps; // seconds per cycle
        let num_cycles = (duration_secs / cycle_duration).ceil() as i64;

        // Query events for all cycles
        let mut all_events = Vec::new();
        for cycle in 0..num_cycles {
            let state = State {
                span: TimeSpan::new(Fraction::new(cycle, 1), Fraction::new(cycle + 1, 1)),
                controls: HashMap::new(),
            };

            let cycle_events = parsed_pattern.query(&state);

            // The events already have the correct timing from the query
            // No need to adjust for cycle offset - the pattern.query() already did that
            for event in cycle_events {
                all_events.push(event);
            }
        }

        // First, collect all synth trigger events
        let mut synth_triggers = Vec::new();

        // Render each event
        for event in all_events {
            let sample_name = &event.value;

            // Calculate timing (event times are in cycles, convert to seconds)
            let event_begin_cycles = event.part.begin.to_float();
            let event_duration_cycles = event.part.duration().to_float();
            let start_time = event_begin_cycles * cycle_duration as f64;
            let duration = event_duration_cycles * cycle_duration as f64;
            let start_sample = (start_time * self.sample_rate as f64) as usize;
            let end_sample = ((start_time + duration) * self.sample_rate as f64) as usize;

            // Check if this is a channel reference (starts with ~)
            if sample_name.starts_with('~') {
                // Store this trigger for later processing
                synth_triggers.push((sample_name.clone(), start_sample, end_sample));
            } else {
                // Regular sample playback (mono output for this simple executor)
                if let Some(sample_data) = self.sample_bank.get_sample(sample_name) {
                    for i in start_sample..end_sample.min(num_samples) {
                        let sample_offset = i - start_sample;
                        if sample_offset < sample_data.len() {
                            // Use mono interpolation (average of L/R for stereo, left for mono)
                            output[i] += sample_data.get_mono_interpolated(sample_offset as f32);
                        }
                    }
                }
            }
        }

        // Process synth triggers - render each one individually at the correct position
        for (sample_name, start_sample, end_sample) in synth_triggers {
            let channel_name = &sample_name[1..]; // Remove the ~

            // Extract frequency if specified
            let (base_name, freq) = if let Some(paren_idx) = channel_name.find('(') {
                let base = &channel_name[..paren_idx];
                let freq_str = channel_name[paren_idx + 1..].trim_end_matches(')');
                let freq = freq_str.parse::<f32>().ok();
                (base, freq)
            } else {
                (channel_name, None)
            };

            // Create a temporary voice for this trigger
            // This is a simplified approach - just generate a sine wave with envelope
            if let Some(chain) = self.voice_allocator.channel_chains.get(base_name).cloned() {
                // Calculate envelope duration based on event duration
                let event_duration_samples = end_sample - start_sample;
                let attack_samples = (0.001 * self.sample_rate) as usize; // 1ms attack
                                                                          // Make decay last for the rest of the event duration
                let decay_samples = event_duration_samples.saturating_sub(attack_samples);

                for i in start_sample..end_sample.min(num_samples) {
                    let sample_offset = i - start_sample;

                    // Generate envelope that spans the full event duration
                    let env = if sample_offset < attack_samples {
                        sample_offset as f32 / attack_samples as f32
                    } else if sample_offset < event_duration_samples {
                        let decay_progress =
                            (sample_offset - attack_samples) as f32 / decay_samples.max(1) as f32;
                        1.0 * (-5.0 * decay_progress).exp()
                    } else {
                        0.0
                    };

                    // Generate simple sine wave with correct phase
                    // Use absolute sample position for phase continuity
                    let absolute_time = i as f32 / self.sample_rate;
                    let freq_to_use = freq.unwrap_or(440.0);
                    let sample = (absolute_time * freq_to_use * 2.0 * std::f32::consts::PI).sin();

                    output[i] += sample * env * 0.3;
                }
            }
        }

        Ok(output)
    }

    /// Render a single DSP chain  
    fn render_chain(&mut self, chain: &DspChain, duration_secs: f32) -> Result<Vec<f32>, String> {
        let num_samples = (self.sample_rate * duration_secs) as usize;
        let mut output = vec![0.0; num_samples];

        // Handle special nodes
        if chain.nodes.len() == 1 {
            // Handle sample playback
            if let DspNode::Sp { sample } = &chain.nodes[0] {
                // Load and play the sample (mono output)
                if let Some(sample_data) = self.sample_bank.get_sample(sample) {
                    let sample_len = sample_data.len();
                    for i in 0..num_samples.min(sample_len) {
                        // Use mono interpolation for this simple executor
                        output[i] = sample_data.get_mono_interpolated(i as f32);
                    }
                }
                return Ok(output);
            }

            // Handle Tidal-style sample patterns
            if let DspNode::S { pattern } = &chain.nodes[0] {
                return self.render_sample_pattern(pattern, duration_secs);
            }

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
            } else if let DspNode::Multiply { sources } = &chain.nodes[0] {
                // Render each source and multiply them
                if !sources.is_empty() {
                    // Start with first source
                    let first_samples = self.render_chain(&sources[0], duration_secs)?;
                    for (i, sample) in first_samples.iter().enumerate() {
                        if i < output.len() {
                            output[i] = *sample;
                        }
                    }
                    // Multiply by remaining sources
                    for source in sources.iter().skip(1) {
                        let source_samples = self.render_chain(source, duration_secs)?;
                        for (i, sample) in source_samples.iter().enumerate() {
                            if i < output.len() {
                                output[i] *= sample;
                            }
                        }
                    }
                }
                return Ok(output);
            }
        }

        // Create processors for each node
        let mut processors: Vec<NodeProcessor> = chain
            .nodes
            .iter()
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
pub fn render_dsp_to_audio_simple(
    code: &str,
    sample_rate: f32,
    duration_secs: f32,
) -> Result<AudioBuffer, String> {
    use crate::glicol_parser::parse_glicol;

    let env = parse_glicol(code)?;
    let mut executor = SimpleDspExecutor::new(sample_rate);
    executor.render(&env, duration_secs)
}
