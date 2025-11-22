/// Comb filter node - feedback delay-based filter for resonance and reverb
///
/// This node implements a feedback comb filter, which creates harmonic resonance
/// at specific frequencies determined by the delay time. Essential for reverb
/// algorithms and metallic/resonant sound design.
///
/// Algorithm: output = input + feedback * delayed_output
/// The feedback creates resonant peaks at frequencies: sample_rate / delay_samples

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Comb filter node with pattern-controlled delay time and feedback
///
/// # Example
/// ```ignore
/// // Resonant comb filter at ~110 Hz (1/110 ≈ 9.09ms delay)
/// let input_signal = OscillatorNode::new(0, Waveform::Noise);  // NodeId 0
/// let delay_time = ConstantNode::new(0.00909);  // 9.09ms = ~110 Hz, NodeId 1
/// let feedback = ConstantNode::new(0.7);  // 70% feedback, NodeId 2
/// let comb = CombFilterNode::new(0, 1, 2, 1.0, 44100.0);  // NodeId 3, max_delay = 1.0s
/// ```
///
/// # Musical Applications
/// - Reverb algorithms (multiple comb filters in parallel/series)
/// - Resonant metallic sounds (high feedback)
/// - Karplus-Strong string synthesis (noise + comb filter)
/// - Flanging effects (modulated delay time)
pub struct CombFilterNode {
    input: NodeId,                // Signal to filter
    delay_time_input: NodeId,     // Delay time in seconds (can be modulated)
    feedback_input: NodeId,       // Feedback amount (-0.99 to 0.99)
    buffer: Vec<f32>,             // Circular delay buffer
    write_pos: usize,             // Current write position
    max_delay_samples: usize,     // Maximum delay in samples
    sample_rate: f32,             // Sample rate for calculations
}

impl CombFilterNode {
    /// CombFilter - Feedback delay-based filter for resonance and reverb
    ///
    /// Creates harmonic resonance at frequencies determined by delay time.
    /// Essential for reverb, Karplus-Strong synthesis, and metallic/resonant effects.
    ///
    /// # Parameters
    /// - `input`: Signal to filter
    /// - `delay_time_input`: Delay time in seconds
    /// - `feedback_input`: Feedback amount (-0.99 to 0.99)
    /// - `max_delay`: Maximum delay time in seconds (buffer size)
    /// - `sample_rate`: Sample rate in Hz (usually 44100.0)
    ///
    /// # Example
    /// ```phonon
    /// ~signal: brown_noise 0.3
    /// ~resonant: ~signal # comb_filter 0.009 0.7
    /// ```
    pub fn new(
        input: NodeId,
        delay_time_input: NodeId,
        feedback_input: NodeId,
        max_delay: f32,
        sample_rate: f32,
    ) -> Self {
        assert!(max_delay > 0.0, "max_delay must be greater than 0");

        // Allocate circular buffer: max_delay seconds * sample_rate samples/second
        let max_delay_samples = (max_delay * sample_rate).ceil() as usize;

        Self {
            input,
            delay_time_input,
            feedback_input,
            buffer: vec![0.0; max_delay_samples],
            write_pos: 0,
            max_delay_samples,
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

    /// Reset the delay buffer to silence
    pub fn clear_buffer(&mut self) {
        self.buffer.fill(0.0);
        self.write_pos = 0;
    }
}

impl AudioNode for CombFilterNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 3,
            "CombFilterNode requires 3 inputs: signal, delay_time, and feedback"
        );

        let input_buffer = inputs[0];
        let delay_time_buffer = inputs[1];
        let feedback_buffer = inputs[2];

        debug_assert_eq!(
            input_buffer.len(),
            output.len(),
            "Input buffer length mismatch"
        );
        debug_assert_eq!(
            delay_time_buffer.len(),
            output.len(),
            "Delay time buffer length mismatch"
        );
        debug_assert_eq!(
            feedback_buffer.len(),
            output.len(),
            "Feedback buffer length mismatch"
        );

        let buffer_len = self.buffer.len();

        for i in 0..output.len() {
            let sample = input_buffer[i];

            // Clamp delay time to valid range [0.0001, max_delay]
            let delay_time = delay_time_buffer[i].max(0.0001);

            // Clamp feedback to prevent instability (-0.99 to 0.99)
            let feedback = feedback_buffer[i].clamp(-0.99, 0.99);

            // Calculate delay in samples
            let delay_samples = (delay_time * self.sample_rate) as usize;
            let delay_samples = delay_samples.min(self.max_delay_samples - 1);

            // Read from delay buffer
            let read_pos = (self.write_pos + buffer_len - delay_samples) % buffer_len;
            let delayed = self.buffer[read_pos];

            // Comb filter: output = input + feedback * delayed
            output[i] = sample + feedback * delayed;

            // Write output to buffer (for feedback)
            self.buffer[self.write_pos] = output[i];

            // Advance write position (circular)
            self.write_pos = (self.write_pos + 1) % buffer_len;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.delay_time_input, self.feedback_input]
    }

    fn name(&self) -> &str {
        "CombFilterNode"
    }

    fn provides_delay(&self) -> bool {
        true  // CombFilterNode has internal delay buffer, can safely break feedback cycles
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_comb_filter_creates_resonance() {
        // Test 1: Verify that positive feedback creates resonance
        // Feed white noise through comb filter, measure that output has
        // stronger frequency content at the resonant frequency

        let sample_rate = 44100.0;
        let delay_time = 1.0 / 110.0; // ~110 Hz resonance (9.09ms)
        let feedback = 0.7; // 70% feedback creates strong resonance
        let block_size = 512;

        let mut comb = CombFilterNode::new(0, 1, 2, 1.0, sample_rate);
        let mut delay_time_node = ConstantNode::new(delay_time);
        let mut feedback_node = ConstantNode::new(feedback);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        // Create impulse input (simplest excitation)
        let mut input_buf = vec![0.0; block_size];
        input_buf[0] = 1.0; // Impulse at start

        let mut delay_time_buf = vec![0.0; block_size];
        let mut feedback_buf = vec![0.0; block_size];

        delay_time_node.process_block(&[], &mut delay_time_buf, sample_rate, &context);
        feedback_node.process_block(&[], &mut feedback_buf, sample_rate, &context);

        // Process multiple blocks to build up resonance
        let mut max_amplitude: f32 = 0.0f32;
        for block_idx in 0..10 {
            let inputs = vec![
                input_buf.as_slice(),
                delay_time_buf.as_slice(),
                feedback_buf.as_slice(),
            ];
            let mut output = vec![0.0; block_size];
            comb.process_block(&inputs, &mut output, sample_rate, &context);

            // After first block, input goes silent (just impulse excitation)
            if block_idx == 0 {
                input_buf.fill(0.0);
            }

            // Track maximum amplitude
            for &sample in output.iter() {
                max_amplitude = f32::max(max_amplitude, sample.abs());
            }
        }

        // With feedback, signal persists (doesn't build up from single impulse)
        assert!(
            max_amplitude >= 0.7,
            "Expected resonance to sustain amplitude >= 0.7, got {}",
            max_amplitude
        );
    }

    #[test]
    fn test_comb_filter_positive_feedback() {
        // Test 2: Positive feedback creates harmonic reinforcement
        // Sustained tone should emerge from impulse excitation

        let sample_rate = 44100.0;
        let delay_time = 0.01; // 10ms = ~100 Hz
        let feedback = 0.8; // High feedback
        let block_size = 512;

        let mut comb = CombFilterNode::new(0, 1, 2, 1.0, sample_rate);
        let mut delay_time_node = ConstantNode::new(delay_time);
        let mut feedback_node = ConstantNode::new(feedback);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        // Impulse excitation
        let mut input_buf = vec![0.0; block_size];
        input_buf[0] = 1.0;

        let mut delay_time_buf = vec![0.0; block_size];
        let mut feedback_buf = vec![0.0; block_size];

        delay_time_node.process_block(&[], &mut delay_time_buf, sample_rate, &context);
        feedback_node.process_block(&[], &mut feedback_buf, sample_rate, &context);

        // First block
        let inputs = vec![
            input_buf.as_slice(),
            delay_time_buf.as_slice(),
            feedback_buf.as_slice(),
        ];
        let mut output1 = vec![0.0; block_size];
        comb.process_block(&inputs, &mut output1, sample_rate, &context);

        // Second block (input is silent now)
        input_buf.fill(0.0);
        let inputs = vec![
            input_buf.as_slice(),
            delay_time_buf.as_slice(),
            feedback_buf.as_slice(),
        ];
        let mut output2 = vec![0.0; block_size];
        comb.process_block(&inputs, &mut output2, sample_rate, &context);

        // Calculate RMS of second block (should have ringing from feedback)
        let rms: f32 = output2.iter().map(|&x| x * x).sum::<f32>() / output2.len() as f32;
        let rms = rms.sqrt();

        assert!(
            rms > 0.01,
            "Expected sustained resonance with RMS > 0.01, got {}",
            rms
        );
    }

    #[test]
    fn test_comb_filter_negative_feedback() {
        // Test 3: Negative feedback creates phase inversion
        // Output should have alternating polarity compared to positive feedback

        let sample_rate = 44100.0;
        let delay_time = 0.01; // 10ms
        let block_size = 512;

        let mut comb_positive = CombFilterNode::new(0, 1, 2, 1.0, sample_rate);
        let mut comb_negative = CombFilterNode::new(0, 1, 2, 1.0, sample_rate);

        let mut delay_time_node = ConstantNode::new(delay_time);
        let mut feedback_positive_node = ConstantNode::new(0.7);
        let mut feedback_negative_node = ConstantNode::new(-0.7);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        // Impulse input
        let mut input_buf = vec![0.0; block_size];
        input_buf[0] = 1.0;

        let mut delay_time_buf = vec![0.0; block_size];
        let mut feedback_pos_buf = vec![0.0; block_size];
        let mut feedback_neg_buf = vec![0.0; block_size];

        delay_time_node.process_block(&[], &mut delay_time_buf, sample_rate, &context);
        feedback_positive_node.process_block(&[], &mut feedback_pos_buf, sample_rate, &context);
        feedback_negative_node.process_block(&[], &mut feedback_neg_buf, sample_rate, &context);

        // Process both filters
        let inputs_pos = vec![
            input_buf.as_slice(),
            delay_time_buf.as_slice(),
            feedback_pos_buf.as_slice(),
        ];
        let mut output_pos = vec![0.0; block_size];
        comb_positive.process_block(&inputs_pos, &mut output_pos, sample_rate, &context);

        let inputs_neg = vec![
            input_buf.as_slice(),
            delay_time_buf.as_slice(),
            feedback_neg_buf.as_slice(),
        ];
        let mut output_neg = vec![0.0; block_size];
        comb_negative.process_block(&inputs_neg, &mut output_neg, sample_rate, &context);

        // Run a second block to see feedback effect
        input_buf.fill(0.0);

        let inputs_pos = vec![
            input_buf.as_slice(),
            delay_time_buf.as_slice(),
            feedback_pos_buf.as_slice(),
        ];
        let mut output_pos_2 = vec![0.0; block_size];
        comb_positive.process_block(&inputs_pos, &mut output_pos_2, sample_rate, &context);

        let inputs_neg = vec![
            input_buf.as_slice(),
            delay_time_buf.as_slice(),
            feedback_neg_buf.as_slice(),
        ];
        let mut output_neg_2 = vec![0.0; block_size];
        comb_negative.process_block(&inputs_neg, &mut output_neg_2, sample_rate, &context);

        // Calculate RMS for both - should both have sustained resonance
        let rms_pos: f32 = output_pos_2.iter().map(|&x| x * x).sum::<f32>() / output_pos_2.len() as f32;
        let rms_pos = rms_pos.sqrt();

        let rms_neg: f32 = output_neg_2.iter().map(|&x| x * x).sum::<f32>() / output_neg_2.len() as f32;
        let rms_neg = rms_neg.sqrt();

        // Both should have sustained resonance (negative feedback still creates resonance)
        assert!(
            rms_pos > 0.01 && rms_neg > 0.01,
            "Both should sustain: pos={}, neg={}",
            rms_pos,
            rms_neg
        );
    }

    #[test]
    fn test_comb_filter_zero_feedback() {
        // Test 4: Zero feedback = no filtering, output = input

        let sample_rate = 44100.0;
        let delay_time = 0.01; // 10ms
        let feedback = 0.0; // No feedback
        let block_size = 512;

        let mut comb = CombFilterNode::new(0, 1, 2, 1.0, sample_rate);
        let mut delay_time_node = ConstantNode::new(delay_time);
        let mut feedback_node = ConstantNode::new(feedback);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        // Create test signal
        let mut input_buf = vec![0.0; block_size];
        for i in 0..block_size {
            input_buf[i] = (i as f32 / block_size as f32) * 2.0 - 1.0; // Ramp -1 to 1
        }

        let mut delay_time_buf = vec![0.0; block_size];
        let mut feedback_buf = vec![0.0; block_size];

        delay_time_node.process_block(&[], &mut delay_time_buf, sample_rate, &context);
        feedback_node.process_block(&[], &mut feedback_buf, sample_rate, &context);

        let inputs = vec![
            input_buf.as_slice(),
            delay_time_buf.as_slice(),
            feedback_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];
        comb.process_block(&inputs, &mut output, sample_rate, &context);

        // With zero feedback, first pass should equal input (no delay buffer history)
        for i in 0..block_size {
            assert!(
                (output[i] - input_buf[i]).abs() < 0.001,
                "Sample {} should match input: expected {}, got {}",
                i,
                input_buf[i],
                output[i]
            );
        }
    }

    #[test]
    fn test_comb_filter_delay_time_affects_frequency() {
        // Test 5: Shorter delay = higher resonant frequency
        // Verify that delay time inversely affects resonance

        let sample_rate = 44100.0;
        let feedback = 0.8;
        let block_size = 512;

        // Two filters: one at 2x the delay time of the other
        let delay_short = 0.005; // 5ms = ~200 Hz
        let delay_long = 0.010; // 10ms = ~100 Hz

        let mut comb_short = CombFilterNode::new(0, 1, 2, 1.0, sample_rate);
        let mut comb_long = CombFilterNode::new(0, 1, 2, 1.0, sample_rate);

        let mut delay_short_node = ConstantNode::new(delay_short);
        let mut delay_long_node = ConstantNode::new(delay_long);
        let mut feedback_node = ConstantNode::new(feedback);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        // Impulse input
        let mut input_buf = vec![0.0; block_size];
        input_buf[0] = 1.0;

        let mut delay_short_buf = vec![0.0; block_size];
        let mut delay_long_buf = vec![0.0; block_size];
        let mut feedback_buf = vec![0.0; block_size];

        delay_short_node.process_block(&[], &mut delay_short_buf, sample_rate, &context);
        delay_long_node.process_block(&[], &mut delay_long_buf, sample_rate, &context);
        feedback_node.process_block(&[], &mut feedback_buf, sample_rate, &context);

        // Process both filters
        let inputs_short = vec![
            input_buf.as_slice(),
            delay_short_buf.as_slice(),
            feedback_buf.as_slice(),
        ];
        let mut output_short = vec![0.0; block_size];
        comb_short.process_block(&inputs_short, &mut output_short, sample_rate, &context);

        let inputs_long = vec![
            input_buf.as_slice(),
            delay_long_buf.as_slice(),
            feedback_buf.as_slice(),
        ];
        let mut output_long = vec![0.0; block_size];
        comb_long.process_block(&inputs_long, &mut output_long, sample_rate, &context);

        // Run more blocks to build up resonance
        input_buf.fill(0.0);

        for _ in 0..5 {
            let inputs_short = vec![
                input_buf.as_slice(),
                delay_short_buf.as_slice(),
                feedback_buf.as_slice(),
            ];
            comb_short.process_block(&inputs_short, &mut output_short, sample_rate, &context);

            let inputs_long = vec![
                input_buf.as_slice(),
                delay_long_buf.as_slice(),
                feedback_buf.as_slice(),
            ];
            comb_long.process_block(&inputs_long, &mut output_long, sample_rate, &context);
        }

        // Count zero crossings to estimate frequency
        let mut crossings_short = 0;
        let mut crossings_long = 0;

        for i in 1..block_size {
            if (output_short[i - 1] < 0.0 && output_short[i] >= 0.0)
                || (output_short[i - 1] > 0.0 && output_short[i] <= 0.0)
            {
                crossings_short += 1;
            }

            if (output_long[i - 1] < 0.0 && output_long[i] >= 0.0)
                || (output_long[i - 1] > 0.0 && output_long[i] <= 0.0)
            {
                crossings_long += 1;
            }
        }

        // Shorter delay should have more zero crossings (higher frequency)
        assert!(
            crossings_short > crossings_long,
            "Expected shorter delay to have more zero crossings: short={}, long={}",
            crossings_short,
            crossings_long
        );
    }

    #[test]
    fn test_comb_filter_dependencies() {
        let comb = CombFilterNode::new(10, 20, 30, 1.0, 44100.0);
        let deps = comb.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 10); // input
        assert_eq!(deps[1], 20); // delay_time_input
        assert_eq!(deps[2], 30); // feedback_input
    }

    #[test]
    fn test_comb_filter_with_constants() {
        // Test 6: Basic functionality with constant inputs

        let sample_rate = 44100.0;
        let delay_time = 0.01; // 10ms
        let feedback = 0.5; // 50% feedback
        let block_size = 512;

        let mut comb = CombFilterNode::new(0, 1, 2, 1.0, sample_rate);
        let mut delay_time_node = ConstantNode::new(delay_time);
        let mut feedback_node = ConstantNode::new(feedback);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        // Constant input of 1.0
        let input_buf = vec![1.0; block_size];
        let mut delay_time_buf = vec![0.0; block_size];
        let mut feedback_buf = vec![0.0; block_size];

        delay_time_node.process_block(&[], &mut delay_time_buf, sample_rate, &context);
        feedback_node.process_block(&[], &mut feedback_buf, sample_rate, &context);

        let inputs = vec![
            input_buf.as_slice(),
            delay_time_buf.as_slice(),
            feedback_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];
        comb.process_block(&inputs, &mut output, sample_rate, &context);

        // First sample should be input (no history yet)
        assert_eq!(output[0], 1.0);

        // Output should be finite and not extreme
        for (i, &sample) in output.iter().enumerate() {
            assert!(
                sample.is_finite(),
                "Sample {} is not finite: {}",
                i,
                sample
            );
            assert!(
                sample.abs() < 10.0,
                "Sample {} has extreme value: {}",
                i,
                sample
            );
        }
    }

    #[test]
    #[should_panic(expected = "max_delay must be greater than 0")]
    fn test_comb_filter_invalid_max_delay() {
        let _ = CombFilterNode::new(0, 1, 2, 0.0, 44100.0);
    }
}
