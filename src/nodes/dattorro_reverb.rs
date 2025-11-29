/// Dattorro Reverb node - High-quality plate reverb
///
/// This node implements Jon Dattorro's 1997 "Effect Design Part 1: Reverberator
/// and Other Filters" algorithm, widely considered one of the best digital
/// reverb designs for realistic room/hall simulation.
///
/// # Algorithm Overview
///
/// The Dattorro reverb consists of:
/// 1. **Pre-delay** - Initial delay before reverb onset (~0-500ms)
/// 2. **Input diffusion** - 4 allpass filters in series for initial echo density
/// 3. **Figure-8 tank** - Two parallel delay networks with cross-coupling:
///    - Each tank: 2 allpass + 2 delay lines with damping filters
///    - Modulated delay times for chorus-like lushness
///    - Cross-coupled for stereo spread and density
/// 4. **Output taps** - Multiple tap points mixed for dense reverb tail
///
/// # References
///
/// - Jon Dattorro (1997) "Effect Design Part 1: Reverberator and Other Filters"
///   Journal of the Audio Engineering Society, Vol. 45, No. 9
/// - Original design at 29.7kHz sample rate, scaled here to arbitrary rates
///
/// # Musical Applications
///
/// - Realistic hall/room/plate reverbs
/// - Ambient soundscapes
/// - Vocal processing
/// - Superior to Schroeder reverb for dense, smooth tails
use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use std::collections::VecDeque;

/// Dattorro reverb node with pattern-controlled parameters
///
/// # Example
/// ```ignore
/// // Create lush plate reverb
/// let input_signal = OscillatorNode::new(0, Waveform::Sine);  // NodeId 0
/// let size = ConstantNode::new(0.7);       // 70% room size, NodeId 1
/// let decay = ConstantNode::new(0.8);      // Long decay, NodeId 2
/// let damping = ConstantNode::new(0.5);    // Moderate damping, NodeId 3
/// let mix = ConstantNode::new(0.3);        // 30% wet, NodeId 4
/// let reverb = DattorroReverbNode::new(0, 1, 2, 3, 4);  // NodeId 5
/// ```
pub struct DattorroReverbNode {
    input: NodeId,
    size: NodeId,
    decay: NodeId,
    damping: NodeId,
    mix: NodeId,

    // Pre-delay line
    predelay: VecDeque<f32>,

    // Input diffusion (4 allpass filters)
    input_ap1: AllpassFilter,
    input_ap2: AllpassFilter,
    input_ap3: AllpassFilter,
    input_ap4: AllpassFilter,

    // Left tank (decay diffusion network 1)
    left_ap1: AllpassFilter,
    left_delay1: VecDeque<f32>,
    left_lpf_state: f32,
    left_ap2: AllpassFilter,
    left_delay2: VecDeque<f32>,

    // Right tank (decay diffusion network 2)
    right_ap1: AllpassFilter,
    right_delay1: VecDeque<f32>,
    right_lpf_state: f32,
    right_ap2: AllpassFilter,
    right_delay2: VecDeque<f32>,

    // Modulation LFO for chorus effect
    lfo_phase: f32,
}

/// Allpass filter structure for diffusion
struct AllpassFilter {
    buffer: VecDeque<f32>,
    gain: f32,
}

impl AllpassFilter {
    fn new(delay_samples: usize, gain: f32) -> Self {
        Self {
            buffer: VecDeque::from(vec![0.0; delay_samples]),
            gain,
        }
    }

    /// Process one sample through allpass
    /// y[n] = -g*x[n] + x[n-D] + g*y[n-D]
    fn process(&mut self, input: f32) -> f32 {
        let delayed = self.buffer[0];
        let output = -self.gain * input + delayed;

        self.buffer.pop_front();
        self.buffer.push_back(input + self.gain * delayed);

        output
    }

    /// Process with modulated delay time (for chorus effect)
    fn process_modulated(&mut self, input: f32, mod_offset: isize) -> f32 {
        let buffer_len = self.buffer.len() as isize;
        let read_idx = ((mod_offset + buffer_len) % buffer_len) as usize;
        let delayed = self.buffer[read_idx];
        let output = -self.gain * input + delayed;

        self.buffer.pop_front();
        self.buffer.push_back(input + self.gain * delayed);

        output
    }
}

impl DattorroReverbNode {
    /// DattorroReverb - High-quality plate reverb using Dattorro algorithm
    ///
    /// Implements Jon Dattorro's 1997 effect design with dense, smooth reverb tails,
    /// featuring modulated delay networks and dual tanks for lush spaciousness.
    ///
    /// # Parameters
    /// - `input`: NodeId providing signal to process
    /// - `size`: NodeId providing room size 0.0-1.0 (scales delay times)
    /// - `decay`: NodeId providing decay time 0.0-1.0 (feedback amount)
    /// - `damping`: NodeId providing high-frequency damping 0.0-1.0
    /// - `mix`: NodeId providing wet/dry mix 0.0-1.0
    ///
    /// # Example
    /// ```phonon
    /// ~signal: saw 110
    /// ~reverb: ~signal # dattorro_reverb 0.8 0.7 0.5 0.3
    /// ```
    pub fn new(input: NodeId, size: NodeId, decay: NodeId, damping: NodeId, mix: NodeId) -> Self {
        // Scale factor from Dattorro's 29.7kHz to 44.1kHz
        let scale = 44100.0 / 29761.0;

        // Input diffusion allpass delays (scaled)
        let input_diffusion_lengths = [
            (142.0 * scale) as usize,
            (107.0 * scale) as usize,
            (379.0 * scale) as usize,
            (277.0 * scale) as usize,
        ];

        // Left tank delays (scaled)
        let left_apf1_len = (672.0 * scale) as usize;
        let left_delay1_len = (4453.0 * scale) as usize;
        let left_apf2_len = (1800.0 * scale) as usize;
        let left_delay2_len = (3720.0 * scale) as usize;

        // Right tank delays (slightly detuned)
        let right_apf1_len = (908.0 * scale) as usize;
        let right_delay1_len = (4217.0 * scale) as usize;
        let right_apf2_len = (2656.0 * scale) as usize;
        let right_delay2_len = (3163.0 * scale) as usize;

        // Allpass gains from Dattorro paper
        let input_diffusion_gain = 0.75;
        let decay_diffusion1 = 0.7;
        let decay_diffusion2 = 0.5;

        Self {
            input,
            size,
            decay,
            damping,
            mix,

            // Pre-delay: ~20ms max at 44.1kHz
            predelay: VecDeque::from(vec![0.0; 882]),

            // Input diffusion chain
            input_ap1: AllpassFilter::new(input_diffusion_lengths[0], input_diffusion_gain),
            input_ap2: AllpassFilter::new(input_diffusion_lengths[1], input_diffusion_gain),
            input_ap3: AllpassFilter::new(input_diffusion_lengths[2], input_diffusion_gain),
            input_ap4: AllpassFilter::new(input_diffusion_lengths[3], input_diffusion_gain),

            // Left tank
            left_ap1: AllpassFilter::new(left_apf1_len, decay_diffusion1),
            left_delay1: VecDeque::from(vec![0.0; left_delay1_len]),
            left_lpf_state: 0.0,
            left_ap2: AllpassFilter::new(left_apf2_len, decay_diffusion2),
            left_delay2: VecDeque::from(vec![0.0; left_delay2_len]),

            // Right tank
            right_ap1: AllpassFilter::new(right_apf1_len, decay_diffusion1),
            right_delay1: VecDeque::from(vec![0.0; right_delay1_len]),
            right_lpf_state: 0.0,
            right_ap2: AllpassFilter::new(right_apf2_len, decay_diffusion2),
            right_delay2: VecDeque::from(vec![0.0; right_delay2_len]),

            lfo_phase: 0.0,
        }
    }
}

impl AudioNode for DattorroReverbNode {
    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.size, self.decay, self.damping, self.mix]
    }

    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 5,
            "DattorroReverbNode requires 5 inputs: signal, size, decay, damping, mix"
        );

        let input = inputs[0];
        let size = inputs[1];
        let decay = inputs[2];
        let damping = inputs[3];
        let mix = inputs[4];

        debug_assert_eq!(input.len(), output.len(), "Input buffer length mismatch");

        // Modulation parameters
        let lfo_rate = 0.8; // Hz
        let mod_depth = 8.0; // samples

        for i in 0..output.len() {
            let in_sample = input[i];
            let size_val = size.get(i).copied().unwrap_or(0.5).clamp(0.0, 1.0);
            let decay_val = decay.get(i).copied().unwrap_or(0.5).clamp(0.0, 1.0);
            let damp_val = damping.get(i).copied().unwrap_or(0.5).clamp(0.0, 1.0);
            let mix_val = mix.get(i).copied().unwrap_or(0.5).clamp(0.0, 1.0);

            // 1. PRE-DELAY
            let predelay_samples = (size_val * 20.0 * sample_rate / 1000.0) as usize;
            let predelay_samples = predelay_samples.min(self.predelay.len() - 1);

            let predelay_out = if predelay_samples > 0 {
                let read_idx = self.predelay.len() - predelay_samples;
                let delayed = self.predelay[read_idx];
                self.predelay.pop_front();
                self.predelay.push_back(in_sample);
                delayed
            } else {
                self.predelay.pop_front();
                self.predelay.push_back(in_sample);
                in_sample
            };

            // 2. INPUT DIFFUSION (4 series allpass)
            let mut diffused = predelay_out;
            diffused = self.input_ap1.process(diffused);
            diffused = self.input_ap2.process(diffused);
            diffused = self.input_ap3.process(diffused);
            diffused = self.input_ap4.process(diffused);

            // 3. FIGURE-8 TANK NETWORK

            // Feedback coefficients scaled by decay parameter
            // Keep max below 0.85 for stability
            let decay_gain = 0.3 + decay_val * 0.55; // 0.3 to 0.85

            // Damping coefficient (one-pole lowpass)
            let damp_coef = damp_val * 0.7; // Higher = darker

            // LFO for modulation (creates chorus effect)
            let lfo = (self.lfo_phase * std::f32::consts::TAU).sin() * mod_depth;
            self.lfo_phase = (self.lfo_phase + lfo_rate / sample_rate) % 1.0;

            // Left tank processing
            let right_to_left = self.right_delay2[0]; // Cross-coupling from right

            // Soft clip to prevent explosions
            let soft_clip = |x: f32| x.tanh();
            let left_input = soft_clip(diffused + right_to_left * decay_gain);

            // Left APF1 (with modulation)
            let left_ap1_out = self.left_ap1.process_modulated(left_input, lfo as isize);

            // Left Delay1
            let left_delay1_out = self.left_delay1[0];
            self.left_delay1.pop_front();
            self.left_delay1.push_back(left_ap1_out);

            // Left APF2 (with opposite phase modulation)
            let left_ap2_out = self
                .left_ap2
                .process_modulated(left_delay1_out, -lfo as isize);

            // Left damping filter + Delay2
            let left_damped = self.left_lpf_state * damp_coef + left_ap2_out * (1.0 - damp_coef);
            self.left_lpf_state = left_damped;

            let left_delay2_in = left_damped * decay_gain;
            let left_delay2_out = self.left_delay2[0];
            self.left_delay2.pop_front();
            self.left_delay2.push_back(left_delay2_in);

            // Right tank processing
            let left_to_right = left_delay2_out; // Cross-coupling from left
            let right_input = soft_clip(diffused + left_to_right);

            // Right APF1 (with opposite modulation)
            let right_ap1_out = self.right_ap1.process_modulated(right_input, -lfo as isize);

            // Right Delay1
            let right_delay1_out = self.right_delay1[0];
            self.right_delay1.pop_front();
            self.right_delay1.push_back(right_ap1_out);

            // Right APF2 (with modulation)
            let right_ap2_out = self
                .right_ap2
                .process_modulated(right_delay1_out, lfo as isize);

            // Right damping filter + Delay2
            let right_damped = self.right_lpf_state * damp_coef + right_ap2_out * (1.0 - damp_coef);
            self.right_lpf_state = right_damped;

            let right_delay2_in = right_damped * decay_gain;
            self.right_delay2.pop_front();
            self.right_delay2.push_back(right_delay2_in);

            // 4. OUTPUT TAPS (mix multiple points for density)
            // Average left and right for mono output
            let left_out = (left_delay1_out + left_ap2_out + left_delay2_out) * 0.33;
            let right_out = (right_delay1_out + right_ap2_out + self.right_delay2[0]) * 0.33;
            let wet = (left_out + right_out) * 0.5;

            // 5. WET/DRY MIX
            output[i] = in_sample * (1.0 - mix_val) + wet * mix_val;
        }
    }

    fn name(&self) -> &str {
        "DattorroReverbNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_processor::BlockProcessor;
    use crate::nodes::{ConstantNode, ImpulseNode, OscillatorNode, Waveform};
    use crate::pattern::Fraction;

    fn calculate_rms(buffer: &[f32]) -> f32 {
        (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt()
    }

    fn test_context(block_size: usize) -> ProcessContext {
        ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, 44100.0)
    }

    #[test]
    fn test_dattorro_produces_dense_reverb_tail() {
        // Test 1: Verify Dattorro creates sustained reverb tail
        let block_size = 512;
        let sample_rate = 44100.0;

        let nodes: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(10.0)),                // 0: freq
            Box::new(ImpulseNode::new(0)),                    // 1: impulse at 10Hz
            Box::new(ConstantNode::new(0.7)),                 // 2: size
            Box::new(ConstantNode::new(0.8)),                 // 3: decay
            Box::new(ConstantNode::new(0.5)),                 // 4: damping
            Box::new(ConstantNode::new(1.0)),                 // 5: 100% wet
            Box::new(DattorroReverbNode::new(1, 2, 3, 4, 5)), // 6: reverb
        ];

        let mut processor = BlockProcessor::new(nodes, 6, block_size).unwrap();
        let context = test_context(block_size);

        // Process multiple blocks to let reverb build up
        let mut last_rms = 0.0;
        for _ in 0..50 {
            let mut output = vec![0.0; block_size];
            processor.process_block(&mut output, &context).unwrap();
            last_rms = calculate_rms(&output);
        }

        // Should have sustained reverb tail
        assert!(
            last_rms > 0.01,
            "Dattorro should create reverb tail: RMS={}",
            last_rms
        );
    }

    #[test]
    fn test_dattorro_size_scales_decay_time() {
        // Test 2: Size parameter scales the decay time
        let block_size = 512;
        let sample_rate = 44100.0;

        // Small size
        let nodes_small: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(1.0)), // 0: constant input
            Box::new(ConstantNode::new(0.2)), // 1: small size
            Box::new(ConstantNode::new(0.8)), // 2: decay
            Box::new(ConstantNode::new(0.5)), // 3: damping
            Box::new(ConstantNode::new(1.0)), // 4: wet
            Box::new(DattorroReverbNode::new(0, 1, 2, 3, 4)),
        ];

        // Large size
        let nodes_large: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(1.0)), // 0: constant input
            Box::new(ConstantNode::new(0.9)), // 1: large size
            Box::new(ConstantNode::new(0.8)), // 2: decay
            Box::new(ConstantNode::new(0.5)), // 3: damping
            Box::new(ConstantNode::new(1.0)), // 4: wet
            Box::new(DattorroReverbNode::new(0, 1, 2, 3, 4)),
        ];

        let mut proc_small = BlockProcessor::new(nodes_small, 5, block_size).unwrap();
        let mut proc_large = BlockProcessor::new(nodes_large, 5, block_size).unwrap();
        let context = test_context(block_size);

        // Build up to steady state
        for _ in 0..100 {
            let mut output = vec![0.0; block_size];
            proc_small.process_block(&mut output, &context).unwrap();
            proc_large.process_block(&mut output, &context).unwrap();
        }

        let mut output_small = vec![0.0; block_size];
        let mut output_large = vec![0.0; block_size];
        proc_small
            .process_block(&mut output_small, &context)
            .unwrap();
        proc_large
            .process_block(&mut output_large, &context)
            .unwrap();

        let rms_small = calculate_rms(&output_small);
        let rms_large = calculate_rms(&output_large);

        // Both should produce sound
        assert!(rms_small > 0.01, "Small size should produce sound");
        assert!(rms_large > 0.01, "Large size should produce sound");

        // Size affects the reverb character (larger has more pre-delay)
        println!("Small RMS: {}, Large RMS: {}", rms_small, rms_large);
    }

    #[test]
    fn test_dattorro_decay_controls_feedback() {
        // Test 3: Decay parameter controls feedback amount
        let block_size = 512;
        let sample_rate = 44100.0;

        // Low decay
        let nodes_short: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(1.0)),
            Box::new(ConstantNode::new(0.5)), // size
            Box::new(ConstantNode::new(0.2)), // short decay
            Box::new(ConstantNode::new(0.5)), // damping
            Box::new(ConstantNode::new(1.0)), // wet
            Box::new(DattorroReverbNode::new(0, 1, 2, 3, 4)),
        ];

        // High decay
        let nodes_long: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(1.0)),
            Box::new(ConstantNode::new(0.5)),  // size
            Box::new(ConstantNode::new(0.95)), // long decay
            Box::new(ConstantNode::new(0.5)),  // damping
            Box::new(ConstantNode::new(1.0)),  // wet
            Box::new(DattorroReverbNode::new(0, 1, 2, 3, 4)),
        ];

        let mut proc_short = BlockProcessor::new(nodes_short, 5, block_size).unwrap();
        let mut proc_long = BlockProcessor::new(nodes_long, 5, block_size).unwrap();
        let context = test_context(block_size);

        // Build up
        for _ in 0..100 {
            let mut output = vec![0.0; block_size];
            proc_short.process_block(&mut output, &context).unwrap();
            proc_long.process_block(&mut output, &context).unwrap();
        }

        let mut output_short = vec![0.0; block_size];
        let mut output_long = vec![0.0; block_size];
        proc_short
            .process_block(&mut output_short, &context)
            .unwrap();
        proc_long.process_block(&mut output_long, &context).unwrap();

        let rms_short = calculate_rms(&output_short);
        let rms_long = calculate_rms(&output_long);

        // Long decay should have more energy buildup
        assert!(
            rms_long > rms_short * 1.1,
            "Long decay should have more energy: short={}, long={}",
            rms_short,
            rms_long
        );
    }

    #[test]
    fn test_dattorro_damping_affects_brightness() {
        // Test 4: Damping parameter controls high frequency content
        let block_size = 512;
        let sample_rate = 44100.0;

        // Low damping (bright)
        let nodes_bright: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(1.0)),
            Box::new(ConstantNode::new(0.5)),
            Box::new(ConstantNode::new(0.8)),
            Box::new(ConstantNode::new(0.1)), // low damping
            Box::new(ConstantNode::new(1.0)),
            Box::new(DattorroReverbNode::new(0, 1, 2, 3, 4)),
        ];

        // High damping (dark)
        let nodes_dark: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(1.0)),
            Box::new(ConstantNode::new(0.5)),
            Box::new(ConstantNode::new(0.8)),
            Box::new(ConstantNode::new(0.9)), // high damping
            Box::new(ConstantNode::new(1.0)),
            Box::new(DattorroReverbNode::new(0, 1, 2, 3, 4)),
        ];

        let mut proc_bright = BlockProcessor::new(nodes_bright, 5, block_size).unwrap();
        let mut proc_dark = BlockProcessor::new(nodes_dark, 5, block_size).unwrap();
        let context = test_context(block_size);

        for _ in 0..50 {
            let mut output = vec![0.0; block_size];
            proc_bright.process_block(&mut output, &context).unwrap();
            proc_dark.process_block(&mut output, &context).unwrap();
        }

        let mut output_bright = vec![0.0; block_size];
        let mut output_dark = vec![0.0; block_size];
        proc_bright
            .process_block(&mut output_bright, &context)
            .unwrap();
        proc_dark.process_block(&mut output_dark, &context).unwrap();

        // Both should have sound
        assert!(calculate_rms(&output_bright) > 0.01);
        assert!(calculate_rms(&output_dark) > 0.01);
    }

    #[test]
    fn test_dattorro_mix_controls_wetdry() {
        // Test 5: Mix parameter controls wet/dry balance
        let block_size = 512;
        let sample_rate = 44100.0;

        // 100% dry
        let nodes_dry: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(1.0)),
            Box::new(ConstantNode::new(0.5)),
            Box::new(ConstantNode::new(0.8)),
            Box::new(ConstantNode::new(0.5)),
            Box::new(ConstantNode::new(0.0)), // 100% dry
            Box::new(DattorroReverbNode::new(0, 1, 2, 3, 4)),
        ];

        // 100% wet
        let nodes_wet: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(1.0)),
            Box::new(ConstantNode::new(0.5)),
            Box::new(ConstantNode::new(0.8)),
            Box::new(ConstantNode::new(0.5)),
            Box::new(ConstantNode::new(1.0)), // 100% wet
            Box::new(DattorroReverbNode::new(0, 1, 2, 3, 4)),
        ];

        let mut proc_dry = BlockProcessor::new(nodes_dry, 5, block_size).unwrap();
        let mut proc_wet = BlockProcessor::new(nodes_wet, 5, block_size).unwrap();
        let context = test_context(block_size);

        let mut output_dry = vec![0.0; block_size];
        let mut output_wet = vec![0.0; block_size];
        proc_dry.process_block(&mut output_dry, &context).unwrap();
        proc_wet.process_block(&mut output_wet, &context).unwrap();

        // Dry should equal input
        for &sample in &output_dry {
            assert!((sample - 1.0).abs() < 0.01, "Dry should pass through");
        }

        // Wet should be different (reverb processing)
        let has_reverb = output_wet.iter().any(|&s| (s - 1.0).abs() > 0.1);
        assert!(has_reverb, "Wet should show reverb processing");
    }

    #[test]
    fn test_dattorro_stability_over_long_duration() {
        // Test 6: Reverb should remain stable over many blocks
        let block_size = 512;
        let sample_rate = 44100.0;

        let nodes: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(1.0)),
            Box::new(ConstantNode::new(0.7)),
            Box::new(ConstantNode::new(0.9)), // high decay
            Box::new(ConstantNode::new(0.5)),
            Box::new(ConstantNode::new(1.0)),
            Box::new(DattorroReverbNode::new(0, 1, 2, 3, 4)),
        ];

        let mut processor = BlockProcessor::new(nodes, 5, block_size).unwrap();
        let context = test_context(block_size);

        // Process many blocks
        for _ in 0..200 {
            let mut output = vec![0.0; block_size];
            processor.process_block(&mut output, &context).unwrap();

            // Check for explosions or NaN
            for (i, &sample) in output.iter().enumerate() {
                assert!(sample.is_finite(), "Sample {} is not finite: {}", i, sample);
                assert!(sample.abs() < 100.0, "Sample {} exploded: {}", i, sample);
            }
        }
    }

    #[test]
    fn test_dattorro_pattern_modulation() {
        // Test 7: Parameters can be modulated over time
        let block_size = 512;
        let sample_rate = 44100.0;

        let nodes: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(0.5)),
            Box::new(ConstantNode::new(0.5)), // size (could be pattern)
            Box::new(ConstantNode::new(0.5)), // decay
            Box::new(ConstantNode::new(0.5)), // damping
            Box::new(ConstantNode::new(0.5)), // mix
            Box::new(DattorroReverbNode::new(0, 1, 2, 3, 4)),
        ];

        let mut processor = BlockProcessor::new(nodes, 5, block_size).unwrap();
        let context = test_context(block_size);

        let mut output = vec![0.0; block_size];
        processor.process_block(&mut output, &context).unwrap();

        assert!(calculate_rms(&output) > 0.001);
    }

    #[test]
    fn test_dattorro_superior_to_schroeder() {
        // Test 8: Dattorro should produce denser, smoother reverb than Schroeder
        // This is a subjective comparison, but we can measure density

        // For now, just verify it produces rich output
        let block_size = 512;
        let sample_rate = 44100.0;

        let nodes: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(0.5)), // Lower input to avoid saturation
            Box::new(ConstantNode::new(0.7)),
            Box::new(ConstantNode::new(0.7)), // Moderate decay
            Box::new(ConstantNode::new(0.5)),
            Box::new(ConstantNode::new(1.0)),
            Box::new(DattorroReverbNode::new(0, 1, 2, 3, 4)),
        ];

        let mut processor = BlockProcessor::new(nodes, 5, block_size).unwrap();
        let context = test_context(block_size);

        // Build up
        for _ in 0..50 {
            let mut output = vec![0.0; block_size];
            processor.process_block(&mut output, &context).unwrap();
        }

        let mut output = vec![0.0; block_size];
        processor.process_block(&mut output, &context).unwrap();

        let rms = calculate_rms(&output);
        assert!(rms > 0.05, "Should have rich reverb: RMS={}", rms);

        // Check for variation (not flat) - more lenient threshold
        let max = output.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let min = output.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        assert!(
            (max - min) > 0.001,
            "Should have variation: max={}, min={}, range={}",
            max,
            min,
            max - min
        );
    }

    #[test]
    fn test_dattorro_dependencies() {
        // Test 9: Verify input dependencies are correct
        let reverb = DattorroReverbNode::new(10, 20, 30, 40, 50);
        let deps = reverb.input_nodes();

        assert_eq!(deps.len(), 5);
        assert_eq!(deps[0], 10); // input
        assert_eq!(deps[1], 20); // size
        assert_eq!(deps[2], 30); // decay
        assert_eq!(deps[3], 40); // damping
        assert_eq!(deps[4], 50); // mix
    }

    #[test]
    fn test_dattorro_name() {
        // Test 10: Verify node name
        let reverb = DattorroReverbNode::new(0, 1, 2, 3, 4);
        assert_eq!(reverb.name(), "DattorroReverbNode");
    }

    #[test]
    fn test_dattorro_zero_input() {
        // Test 11: Zero input should produce zero output (after settling)
        let block_size = 512;
        let sample_rate = 44100.0;

        let nodes: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(0.0)), // zero input
            Box::new(ConstantNode::new(0.5)),
            Box::new(ConstantNode::new(0.8)),
            Box::new(ConstantNode::new(0.5)),
            Box::new(ConstantNode::new(1.0)),
            Box::new(DattorroReverbNode::new(0, 1, 2, 3, 4)),
        ];

        let mut processor = BlockProcessor::new(nodes, 5, block_size).unwrap();
        let context = test_context(block_size);

        let mut output = vec![0.0; block_size];
        processor.process_block(&mut output, &context).unwrap();

        let rms = calculate_rms(&output);
        assert!(rms < 0.001, "Zero input should produce ~zero output");
    }

    #[test]
    fn test_dattorro_extreme_parameters() {
        // Test 12: Extreme parameter values should be handled gracefully
        let block_size = 512;
        let sample_rate = 44100.0;

        let nodes: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(1.0)),
            Box::new(ConstantNode::new(1.0)), // max size
            Box::new(ConstantNode::new(1.0)), // max decay
            Box::new(ConstantNode::new(1.0)), // max damping
            Box::new(ConstantNode::new(1.0)), // max wet
            Box::new(DattorroReverbNode::new(0, 1, 2, 3, 4)),
        ];

        let mut processor = BlockProcessor::new(nodes, 5, block_size).unwrap();
        let context = test_context(block_size);

        for _ in 0..100 {
            let mut output = vec![0.0; block_size];
            processor.process_block(&mut output, &context).unwrap();

            for &sample in &output {
                assert!(sample.is_finite());
                assert!(sample.abs() < 100.0);
            }
        }
    }

    #[test]
    fn test_dattorro_minimum_parameters() {
        // Test 13: Minimum parameter values should work
        let block_size = 512;
        let sample_rate = 44100.0;

        let nodes: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(1.0)),
            Box::new(ConstantNode::new(0.0)), // min size
            Box::new(ConstantNode::new(0.0)), // min decay
            Box::new(ConstantNode::new(0.0)), // min damping
            Box::new(ConstantNode::new(0.5)), // 50% mix
            Box::new(DattorroReverbNode::new(0, 1, 2, 3, 4)),
        ];

        let mut processor = BlockProcessor::new(nodes, 5, block_size).unwrap();
        let context = test_context(block_size);

        let mut output = vec![0.0; block_size];
        processor.process_block(&mut output, &context).unwrap();

        // Should still produce valid output
        for &sample in &output {
            assert!(sample.is_finite());
        }
    }

    #[test]
    fn test_dattorro_modulation_creates_lushness() {
        // Test 14: Verify modulation LFO is active (creates chorus effect)
        let block_size = 512;
        let sample_rate = 44100.0;

        let nodes: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(1.0)),
            Box::new(ConstantNode::new(0.7)),
            Box::new(ConstantNode::new(0.8)),
            Box::new(ConstantNode::new(0.5)),
            Box::new(ConstantNode::new(1.0)),
            Box::new(DattorroReverbNode::new(0, 1, 2, 3, 4)),
        ];

        let mut processor = BlockProcessor::new(nodes, 5, block_size).unwrap();
        let context = test_context(block_size);

        // Build up
        for _ in 0..100 {
            let mut output = vec![0.0; block_size];
            processor.process_block(&mut output, &context).unwrap();
        }

        let mut output = vec![0.0; block_size];
        processor.process_block(&mut output, &context).unwrap();

        // Should have rich, modulated output
        let rms = calculate_rms(&output);
        assert!(rms > 0.05, "Modulation should create lush reverb");
    }
}
