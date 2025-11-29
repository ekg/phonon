/// Flanger node - swept comb filter effect
///
/// This node implements a classic flanger effect using a modulated delay line.
/// The LFO sweeps the delay time, creating the characteristic "swooshing" comb filter sound.
use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use std::f32::consts::PI;

/// Flanger node with pattern-controlled rate, depth, and feedback
///
/// # Example
/// ```ignore
/// // Classic flanger on signal
/// let input_signal = OscillatorNode::new(0, Waveform::Saw);  // NodeId 0
/// let rate = ConstantNode::new(0.5);  // 0.5 Hz LFO, NodeId 1
/// let depth = ConstantNode::new(0.005);  // 5ms depth, NodeId 2
/// let feedback = ConstantNode::new(0.7);  // 70% feedback, NodeId 3
/// let flanger = FlangerNode::new(0, 1, 2, 3, 44100.0);  // NodeId 4
/// ```
pub struct FlangerNode {
    input: NodeId,          // Signal to flange
    rate_input: NodeId,     // LFO rate in Hz (can be modulated)
    depth_input: NodeId,    // Delay time modulation depth in seconds (can be modulated)
    feedback_input: NodeId, // Feedback amount (can be modulated)
    buffer: Vec<f32>,       // Delay buffer
    write_pos: usize,       // Current write position
    phase: f32,             // LFO phase (0.0 to 1.0)
    sample_rate: f32,       // Sample rate for calculations
}

impl FlangerNode {
    /// Flanger - Swept comb filter with modulated delay and feedback
    ///
    /// Creates characteristic "swooshing" effect using LFO-modulated delay line,
    /// perfect for classic chorus/flanger sounds and creative effects.
    ///
    /// # Parameters
    /// - `input`: NodeId providing signal to flange
    /// - `rate_input`: NodeId providing LFO rate in Hz (default: 0.5)
    /// - `depth_input`: NodeId providing delay depth in seconds (default: 0.005)
    /// - `feedback_input`: NodeId providing feedback 0.0-0.95 (default: 0.7)
    /// - `sample_rate`: Sample rate in Hz (usually 44100.0)
    ///
    /// # Example
    /// ```phonon
    /// ~signal: saw 110
    /// ~flanged: ~signal # flanger 0.5 0.005 0.7 44100
    /// ```
    pub fn new(
        input: NodeId,
        rate_input: NodeId,
        depth_input: NodeId,
        feedback_input: NodeId,
        sample_rate: f32,
    ) -> Self {
        // Flanger typically uses short delays: 1-20ms
        let max_delay = 0.02; // 20ms maximum
        let buffer_size = (max_delay * sample_rate).ceil() as usize;

        Self {
            input,
            rate_input,
            depth_input,
            feedback_input,
            buffer: vec![0.0; buffer_size],
            write_pos: 0,
            phase: 0.0,
            sample_rate,
        }
    }

    /// Get the current write position in the buffer
    pub fn write_position(&self) -> usize {
        self.write_pos
    }

    /// Get the buffer size
    pub fn buffer_size(&self) -> usize {
        self.buffer.len()
    }

    /// Get the current LFO phase (0.0 to 1.0)
    pub fn lfo_phase(&self) -> f32 {
        self.phase
    }

    /// Reset the flanger buffer to silence and reset phase
    pub fn clear_buffer(&mut self) {
        self.buffer.fill(0.0);
        self.write_pos = 0;
        self.phase = 0.0;
    }
}

impl AudioNode for FlangerNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 4,
            "FlangerNode requires 4 inputs: signal, rate, depth, feedback"
        );

        let input_buffer = inputs[0];
        let rate_buffer = inputs[1];
        let depth_buffer = inputs[2];
        let feedback_buffer = inputs[3];

        debug_assert_eq!(
            input_buffer.len(),
            output.len(),
            "Input buffer length mismatch"
        );
        debug_assert_eq!(
            rate_buffer.len(),
            output.len(),
            "Rate buffer length mismatch"
        );
        debug_assert_eq!(
            depth_buffer.len(),
            output.len(),
            "Depth buffer length mismatch"
        );
        debug_assert_eq!(
            feedback_buffer.len(),
            output.len(),
            "Feedback buffer length mismatch"
        );

        let buffer_len = self.buffer.len();

        for i in 0..output.len() {
            let sample = input_buffer[i];
            let rate = rate_buffer[i];
            let depth = depth_buffer[i].clamp(0.0, 0.02); // Max 20ms delay
            let feedback = feedback_buffer[i].clamp(0.0, 0.95); // Max 95% feedback

            // Generate LFO (0.0 to 1.0)
            let lfo = (self.phase * 2.0 * PI).sin() * 0.5 + 0.5;

            // Calculate delay time: 1ms base + modulation
            let delay_time = 0.001 + (depth * lfo);
            let delay_samples = delay_time * self.sample_rate;

            // Read from delay buffer with linear interpolation
            let read_pos = (self.write_pos as f32 + buffer_len as f32 - delay_samples)
                .rem_euclid(buffer_len as f32);
            let index = read_pos as usize;
            let frac = read_pos - index as f32;
            let delayed = self.buffer[index]
                + frac * (self.buffer[(index + 1) % buffer_len] - self.buffer[index]);

            // Mix dry + wet
            output[i] = sample + delayed;

            // Write to buffer with feedback
            self.buffer[self.write_pos] = sample + delayed * feedback;
            self.write_pos = (self.write_pos + 1) % buffer_len;

            // Advance phase
            self.phase += rate / self.sample_rate;
            while self.phase >= 1.0 {
                self.phase -= 1.0;
            }
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![
            self.input,
            self.rate_input,
            self.depth_input,
            self.feedback_input,
        ]
    }

    fn name(&self) -> &str {
        "FlangerNode"
    }

    fn provides_delay(&self) -> bool {
        true // FlangerNode has modulated delay buffer, can safely break feedback cycles
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_flanger_zero_depth_minimal_effect() {
        // Test 1: With zero depth, flanger should have minimal effect (just 1ms fixed delay)

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut input_node = ConstantNode::new(1.0);
        let mut rate_node = ConstantNode::new(0.5); // 0.5 Hz
        let mut depth_node = ConstantNode::new(0.0); // Zero depth
        let mut feedback_node = ConstantNode::new(0.0); // Zero feedback
        let mut flanger = FlangerNode::new(0, 1, 2, 3, sample_rate);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Generate input buffers
        let mut input_buf = vec![1.0; block_size];
        let mut rate_buf = vec![0.5; block_size];
        let mut depth_buf = vec![0.0; block_size];
        let mut feedback_buf = vec![0.0; block_size];

        input_node.process_block(&[], &mut input_buf, sample_rate, &context);
        rate_node.process_block(&[], &mut rate_buf, sample_rate, &context);
        depth_node.process_block(&[], &mut depth_buf, sample_rate, &context);
        feedback_node.process_block(&[], &mut feedback_buf, sample_rate, &context);

        let inputs = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
            feedback_buf.as_slice(),
        ];

        // Process several blocks to fill the delay buffer
        for _ in 0..10 {
            let mut output = vec![0.0; block_size];
            flanger.process_block(&inputs, &mut output, sample_rate, &context);
        }

        // Final block should have minimal change (just 1ms fixed delay)
        let mut output = vec![0.0; block_size];
        flanger.process_block(&inputs, &mut output, sample_rate, &context);

        // With zero depth and feedback, output should be input + 1ms delayed signal
        // After buffer fills, should approach 2.0 (dry + wet both at 1.0)
        let avg = output.iter().sum::<f32>() / output.len() as f32;
        assert!(
            avg > 1.8 && avg < 2.2,
            "Average should be ~2.0, got {}",
            avg
        );
    }

    #[test]
    fn test_flanger_creates_comb_filtering() {
        // Test 2: Flanger should create time-varying comb filtering

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut input_node = ConstantNode::new(1.0);
        let mut rate_node = ConstantNode::new(1.0); // 1 Hz LFO
        let mut depth_node = ConstantNode::new(0.005); // 5ms depth
        let mut feedback_node = ConstantNode::new(0.5); // 50% feedback
        let mut flanger = FlangerNode::new(0, 1, 2, 3, sample_rate);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Generate input buffers
        let mut input_buf = vec![1.0; block_size];
        let mut rate_buf = vec![1.0; block_size];
        let mut depth_buf = vec![0.005; block_size];
        let mut feedback_buf = vec![0.5; block_size];

        input_node.process_block(&[], &mut input_buf, sample_rate, &context);
        rate_node.process_block(&[], &mut rate_buf, sample_rate, &context);
        depth_node.process_block(&[], &mut depth_buf, sample_rate, &context);
        feedback_node.process_block(&[], &mut feedback_buf, sample_rate, &context);

        let inputs = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
            feedback_buf.as_slice(),
        ];

        // Process several blocks
        let mut outputs = Vec::new();
        for _ in 0..8 {
            let mut output = vec![0.0; block_size];
            flanger.process_block(&inputs, &mut output, sample_rate, &context);
            outputs.extend_from_slice(&output);
        }

        // With modulation, output should vary (not constant)
        let min = outputs.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = outputs.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let range = max - min;

        assert!(
            range > 0.5,
            "Output should vary with modulation, range: {}",
            range
        );
    }

    #[test]
    fn test_flanger_rate_modulation() {
        // Test 3: Different LFO rates should produce different modulation speeds

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut input_node = ConstantNode::new(1.0);
        let mut depth_node = ConstantNode::new(0.005);
        let mut feedback_node = ConstantNode::new(0.5);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Test with slow LFO (0.5 Hz)
        let mut rate_node_slow = ConstantNode::new(0.5);
        let mut flanger_slow = FlangerNode::new(0, 1, 2, 3, sample_rate);

        let mut input_buf = vec![1.0; block_size];
        let mut rate_buf = vec![0.5; block_size];
        let mut depth_buf = vec![0.005; block_size];
        let mut feedback_buf = vec![0.5; block_size];

        input_node.process_block(&[], &mut input_buf, sample_rate, &context);
        rate_node_slow.process_block(&[], &mut rate_buf, sample_rate, &context);
        depth_node.process_block(&[], &mut depth_buf, sample_rate, &context);
        feedback_node.process_block(&[], &mut feedback_buf, sample_rate, &context);

        let inputs_slow = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
            feedback_buf.as_slice(),
        ];

        // Process blocks and record phase advancement
        let mut slow_phases = Vec::new();
        for _ in 0..4 {
            let mut output = vec![0.0; block_size];
            flanger_slow.process_block(&inputs_slow, &mut output, sample_rate, &context);
            slow_phases.push(flanger_slow.lfo_phase());
        }

        // Test with fast LFO (2.0 Hz)
        let mut rate_node_fast = ConstantNode::new(2.0);
        let mut flanger_fast = FlangerNode::new(0, 1, 2, 3, sample_rate);

        let mut rate_buf_fast = vec![2.0; block_size];
        rate_node_fast.process_block(&[], &mut rate_buf_fast, sample_rate, &context);

        let inputs_fast = vec![
            input_buf.as_slice(),
            rate_buf_fast.as_slice(),
            depth_buf.as_slice(),
            feedback_buf.as_slice(),
        ];

        let mut fast_phases = Vec::new();
        for _ in 0..4 {
            let mut output = vec![0.0; block_size];
            flanger_fast.process_block(&inputs_fast, &mut output, sample_rate, &context);
            fast_phases.push(flanger_fast.lfo_phase());
        }

        // Fast LFO should advance phase faster
        let slow_delta = slow_phases.last().unwrap() - slow_phases.first().unwrap();
        let fast_delta = fast_phases.last().unwrap() - fast_phases.first().unwrap();

        assert!(
            fast_delta > slow_delta * 3.0,
            "Fast LFO should advance phase faster: fast_delta={}, slow_delta={}",
            fast_delta,
            slow_delta
        );
    }

    #[test]
    fn test_flanger_feedback_increases_resonance() {
        // Test 4: Higher feedback should increase resonance/amplitude

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut input_node = ConstantNode::new(1.0);
        let mut rate_node = ConstantNode::new(0.5);
        let mut depth_node = ConstantNode::new(0.005);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Test with low feedback
        let mut feedback_node_low = ConstantNode::new(0.1);
        let mut flanger_low = FlangerNode::new(0, 1, 2, 3, sample_rate);

        let mut input_buf = vec![1.0; block_size];
        let mut rate_buf = vec![0.5; block_size];
        let mut depth_buf = vec![0.005; block_size];
        let mut feedback_buf_low = vec![0.1; block_size];

        input_node.process_block(&[], &mut input_buf, sample_rate, &context);
        rate_node.process_block(&[], &mut rate_buf, sample_rate, &context);
        depth_node.process_block(&[], &mut depth_buf, sample_rate, &context);
        feedback_node_low.process_block(&[], &mut feedback_buf_low, sample_rate, &context);

        let inputs_low = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
            feedback_buf_low.as_slice(),
        ];

        // Process several blocks
        let mut low_outputs = Vec::new();
        for _ in 0..10 {
            let mut output = vec![0.0; block_size];
            flanger_low.process_block(&inputs_low, &mut output, sample_rate, &context);
            low_outputs.extend_from_slice(&output);
        }

        let low_max = low_outputs
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max);

        // Test with high feedback
        let mut feedback_node_high = ConstantNode::new(0.8);
        let mut flanger_high = FlangerNode::new(0, 1, 2, 3, sample_rate);

        let mut feedback_buf_high = vec![0.8; block_size];
        feedback_node_high.process_block(&[], &mut feedback_buf_high, sample_rate, &context);

        let inputs_high = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
            feedback_buf_high.as_slice(),
        ];

        let mut high_outputs = Vec::new();
        for _ in 0..10 {
            let mut output = vec![0.0; block_size];
            flanger_high.process_block(&inputs_high, &mut output, sample_rate, &context);
            high_outputs.extend_from_slice(&output);
        }

        let high_max = high_outputs
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max);

        // Higher feedback should produce higher peaks (resonance)
        assert!(
            high_max > low_max * 1.2,
            "High feedback should increase resonance: high_max={}, low_max={}",
            high_max,
            low_max
        );
    }

    #[test]
    fn test_flanger_phase_advances() {
        // Test 5: LFO phase should advance continuously

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut flanger = FlangerNode::new(0, 1, 2, 3, sample_rate);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        let input_buf = vec![1.0; block_size];
        let rate_buf = vec![1.0; block_size]; // 1 Hz
        let depth_buf = vec![0.005; block_size];
        let feedback_buf = vec![0.5; block_size];

        let inputs = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
            feedback_buf.as_slice(),
        ];

        let initial_phase = flanger.lfo_phase();

        // Process one block
        let mut output = vec![0.0; block_size];
        flanger.process_block(&inputs, &mut output, sample_rate, &context);

        let final_phase = flanger.lfo_phase();

        // Phase should have advanced
        // Expected: block_size / sample_rate * rate = 512 / 44100 * 1.0 ≈ 0.0116
        let expected_delta = block_size as f32 / sample_rate * 1.0;
        let actual_delta = final_phase - initial_phase;

        assert!(
            (actual_delta - expected_delta).abs() < 0.001,
            "Phase should advance by ~{}, got {}",
            expected_delta,
            actual_delta
        );
    }

    #[test]
    fn test_flanger_dependencies() {
        // Test 6: Verify flanger reports correct dependencies

        let flanger = FlangerNode::new(10, 20, 30, 40, 44100.0);
        let deps = flanger.input_nodes();

        assert_eq!(deps.len(), 4);
        assert_eq!(deps[0], 10); // input
        assert_eq!(deps[1], 20); // rate_input
        assert_eq!(deps[2], 30); // depth_input
        assert_eq!(deps[3], 40); // feedback_input
    }

    #[test]
    fn test_flanger_with_constants() {
        // Test 7: Flanger should work with constant parameters

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut input_node = ConstantNode::new(0.5);
        let mut rate_node = ConstantNode::new(0.3); // 0.3 Hz
        let mut depth_node = ConstantNode::new(0.003); // 3ms
        let mut feedback_node = ConstantNode::new(0.6); // 60%
        let mut flanger = FlangerNode::new(0, 1, 2, 3, sample_rate);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        let mut input_buf = vec![0.0; block_size];
        let mut rate_buf = vec![0.0; block_size];
        let mut depth_buf = vec![0.0; block_size];
        let mut feedback_buf = vec![0.0; block_size];

        input_node.process_block(&[], &mut input_buf, sample_rate, &context);
        rate_node.process_block(&[], &mut rate_buf, sample_rate, &context);
        depth_node.process_block(&[], &mut depth_buf, sample_rate, &context);
        feedback_node.process_block(&[], &mut feedback_buf, sample_rate, &context);

        let inputs = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
            feedback_buf.as_slice(),
        ];

        // Process multiple blocks
        let mut all_outputs = Vec::new();
        for _ in 0..20 {
            let mut output = vec![0.0; block_size];
            flanger.process_block(&inputs, &mut output, sample_rate, &context);
            all_outputs.extend_from_slice(&output);

            // All outputs should be finite
            for (i, &sample) in output.iter().enumerate() {
                assert!(sample.is_finite(), "Sample {} is not finite: {}", i, sample);
            }
        }

        // Output should have variation (LFO modulation)
        let min = all_outputs.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = all_outputs
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max);
        let range = max - min;

        assert!(range > 0.1, "Output should vary, range: {}", range);
    }
}
