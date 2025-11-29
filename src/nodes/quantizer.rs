/// Quantizer node - snaps values to grid/scale
///
/// This node performs sample-by-sample quantization, rounding input values
/// to the nearest multiple of a step size. Useful for:
/// - Creating chromatic/diatonic scales (step = 1.0 for semitones)
/// - Bit reduction effects (step = 0.5, 0.25, etc.)
/// - Grid-based modulation (snap LFO to discrete steps)
///
/// Formula: `output[i] = round(input[i] / step[i]) * step[i]`
///
/// # Example
/// ```ignore
/// // Quantize sine LFO to semitone grid
/// let lfo = OscillatorNode::new(0, Waveform::Sine);      // NodeId 1
/// let step = ConstantNode::new(1.0);                      // NodeId 2 (semitones)
/// let quantized = QuantizerNode::new(1, 2);               // NodeId 3
/// // Output will be stepped sine wave (semitone steps)
/// ```
use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Quantizer node: snap values to nearest step
///
/// Quantizes input signal to discrete steps based on step_size.
/// Step size is evaluated per-sample, allowing for dynamic quantization.
pub struct QuantizerNode {
    /// Input signal to quantize
    input: NodeId,
    /// Step size for quantization (prevents division by zero with min 0.0001)
    step_size_input: NodeId,
}

impl QuantizerNode {
    /// QuantizerNode - Snap values to discrete quantization grid
    ///
    /// Quantizes input signal to discrete steps based on step size.
    /// Step size is evaluated per-sample, allowing for dynamic quantization effects.
    ///
    /// # Parameters
    /// - `input`: NodeId of signal to quantize
    /// - `step_size_input`: NodeId of step size (quantization grid, prevents division by zero with min 0.0001)
    ///
    /// # Example
    /// ```phonon
    /// ~lfo: sine 0.25
    /// ~quantized: ~lfo # quantizer 1.0
    /// ```
    pub fn new(input: NodeId, step_size_input: NodeId) -> Self {
        Self {
            input,
            step_size_input,
        }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }

    /// Get the step size input node ID
    pub fn step_size_input(&self) -> NodeId {
        self.step_size_input
    }
}

impl AudioNode for QuantizerNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert_eq!(
            inputs.len(),
            2,
            "QuantizerNode expects 2 inputs (input, step_size), got {}",
            inputs.len()
        );

        let input_buffer = inputs[0];
        let step_size_buffer = inputs[1];

        debug_assert_eq!(
            input_buffer.len(),
            output.len(),
            "Input buffer length mismatch"
        );
        debug_assert_eq!(
            step_size_buffer.len(),
            output.len(),
            "Step size buffer length mismatch"
        );

        // Quantize: round to nearest step
        for i in 0..output.len() {
            let value = input_buffer[i];
            // Prevent division by zero - minimum step of 0.0001
            let step = step_size_buffer[i].max(0.0001);

            // Round to nearest step: (value / step).round() * step
            output[i] = (value / step).round() * step;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.step_size_input]
    }

    fn name(&self) -> &str {
        "QuantizerNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_quantizer_snap_to_integer() {
        // Test: Quantize to integer steps (step = 1.0)
        let mut quantizer = QuantizerNode::new(0, 1);

        let input = vec![0.1, 0.5, 0.9, 1.4, 1.6, 2.3, -0.5, -1.7];
        let step_size = vec![1.0; 8];
        let inputs = vec![input.as_slice(), step_size.as_slice()];

        let mut output = vec![0.0; 8];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 8, 2.0, 44100.0);

        quantizer.process_block(&inputs, &mut output, 44100.0, &context);

        // Verify rounding to nearest integer
        assert_eq!(output[0], 0.0); // 0.1 -> 0.0
        assert_eq!(output[1], 1.0); // 0.5 -> 1.0 (rounds up)
        assert_eq!(output[2], 1.0); // 0.9 -> 1.0
        assert_eq!(output[3], 1.0); // 1.4 -> 1.0
        assert_eq!(output[4], 2.0); // 1.6 -> 2.0
        assert_eq!(output[5], 2.0); // 2.3 -> 2.0
        assert_eq!(output[6], -1.0); // -0.5 -> -1.0 (rounds down for negative)
        assert_eq!(output[7], -2.0); // -1.7 -> -2.0
    }

    #[test]
    fn test_quantizer_half_step() {
        // Test: Quantize to half steps (step = 0.5)
        let mut quantizer = QuantizerNode::new(0, 1);

        let input = vec![0.0, 0.2, 0.3, 0.7, 1.0, 1.1, 1.4, 1.6];
        let step_size = vec![0.5; 8];
        let inputs = vec![input.as_slice(), step_size.as_slice()];

        let mut output = vec![0.0; 8];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 8, 2.0, 44100.0);

        quantizer.process_block(&inputs, &mut output, 44100.0, &context);

        // Verify rounding to nearest 0.5
        assert_eq!(output[0], 0.0); // 0.0 -> 0.0
        assert_eq!(output[1], 0.0); // 0.2 -> 0.0
        assert_eq!(output[2], 0.5); // 0.3 -> 0.5
        assert_eq!(output[3], 0.5); // 0.7 -> 0.5
        assert_eq!(output[4], 1.0); // 1.0 -> 1.0
        assert_eq!(output[5], 1.0); // 1.1 -> 1.0
        assert_eq!(output[6], 1.5); // 1.4 -> 1.5
        assert_eq!(output[7], 1.5); // 1.6 -> 1.5
    }

    #[test]
    fn test_quantizer_small_step() {
        // Test: Quantize to very small steps (step = 0.1)
        let mut quantizer = QuantizerNode::new(0, 1);

        let input = vec![0.14, 0.16, 0.24, 0.26, 1.03, 1.07];
        let step_size = vec![0.1; 6];
        let inputs = vec![input.as_slice(), step_size.as_slice()];

        let mut output = vec![0.0; 6];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 6, 2.0, 44100.0);

        quantizer.process_block(&inputs, &mut output, 44100.0, &context);

        // Verify rounding to nearest 0.1
        assert!((output[0] - 0.1).abs() < 0.001); // 0.14 -> 0.1
        assert!((output[1] - 0.2).abs() < 0.001); // 0.16 -> 0.2
        assert!((output[2] - 0.2).abs() < 0.001); // 0.24 -> 0.2
        assert!((output[3] - 0.3).abs() < 0.001); // 0.26 -> 0.3
        assert!((output[4] - 1.0).abs() < 0.001); // 1.03 -> 1.0
        assert!((output[5] - 1.1).abs() < 0.001); // 1.07 -> 1.1
    }

    #[test]
    fn test_quantizer_large_step() {
        // Test: Quantize to large steps (step = 10.0)
        let mut quantizer = QuantizerNode::new(0, 1);

        let input = vec![3.0, 7.0, 14.0, 16.0, 23.0, 27.0, -7.0, -13.0];
        let step_size = vec![10.0; 8];
        let inputs = vec![input.as_slice(), step_size.as_slice()];

        let mut output = vec![0.0; 8];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 8, 2.0, 44100.0);

        quantizer.process_block(&inputs, &mut output, 44100.0, &context);

        // Verify rounding to nearest 10.0
        assert_eq!(output[0], 0.0); // 3.0 -> 0.0
        assert_eq!(output[1], 10.0); // 7.0 -> 10.0
        assert_eq!(output[2], 10.0); // 14.0 -> 10.0
        assert_eq!(output[3], 20.0); // 16.0 -> 20.0
        assert_eq!(output[4], 20.0); // 23.0 -> 20.0
        assert_eq!(output[5], 30.0); // 27.0 -> 30.0
        assert_eq!(output[6], -10.0); // -7.0 -> -10.0
        assert_eq!(output[7], -10.0); // -13.0 -> -10.0
    }

    #[test]
    fn test_quantizer_negative_values() {
        // Test: Quantize negative values
        let mut quantizer = QuantizerNode::new(0, 1);

        let input = vec![-0.3, -0.7, -1.2, -1.8, -2.4, -2.6];
        let step_size = vec![1.0; 6];
        let inputs = vec![input.as_slice(), step_size.as_slice()];

        let mut output = vec![0.0; 6];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 6, 2.0, 44100.0);

        quantizer.process_block(&inputs, &mut output, 44100.0, &context);

        // Verify correct rounding for negative values
        assert_eq!(output[0], -0.0); // -0.3 -> 0.0
        assert_eq!(output[1], -1.0); // -0.7 -> -1.0
        assert_eq!(output[2], -1.0); // -1.2 -> -1.0
        assert_eq!(output[3], -2.0); // -1.8 -> -2.0
        assert_eq!(output[4], -2.0); // -2.4 -> -2.0
        assert_eq!(output[5], -3.0); // -2.6 -> -3.0
    }

    #[test]
    fn test_quantizer_dependencies() {
        // Test that input_nodes returns both dependencies
        let quantizer = QuantizerNode::new(5, 10);
        let deps = quantizer.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 5); // input
        assert_eq!(deps[1], 10); // step_size_input
    }

    #[test]
    fn test_quantizer_chromatic_scale() {
        // Test: Musical use case - quantize to chromatic scale (semitones)
        // Step = 1.0 for semitone quantization
        let mut quantizer = QuantizerNode::new(0, 1);

        // Frequency values that should snap to semitone grid
        // Using MIDI note number analogy: 60.2 -> 60, 60.7 -> 61, etc.
        let input = vec![60.2, 60.5, 60.8, 61.3, 61.6, 62.1, 62.4, 62.9];
        let step_size = vec![1.0; 8];
        let inputs = vec![input.as_slice(), step_size.as_slice()];

        let mut output = vec![0.0; 8];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 8, 2.0, 44100.0);

        quantizer.process_block(&inputs, &mut output, 44100.0, &context);

        // Verify chromatic quantization
        assert_eq!(output[0], 60.0); // 60.2 -> 60 (C)
        assert_eq!(output[1], 61.0); // 60.5 -> 61 (C#) - rounds up at .5
        assert_eq!(output[2], 61.0); // 60.8 -> 61 (C#)
        assert_eq!(output[3], 61.0); // 61.3 -> 61 (C#)
        assert_eq!(output[4], 62.0); // 61.6 -> 62 (D)
        assert_eq!(output[5], 62.0); // 62.1 -> 62 (D)
        assert_eq!(output[6], 62.0); // 62.4 -> 62 (D)
        assert_eq!(output[7], 63.0); // 62.9 -> 63 (D#)
    }

    #[test]
    fn test_quantizer_varying_step_per_sample() {
        // Test: Step size can vary per-sample
        let mut quantizer = QuantizerNode::new(0, 1);

        let input = vec![1.4, 1.4, 1.4, 1.4];
        // Different step size per sample
        let step_size = vec![1.0, 0.5, 0.25, 2.0];
        let inputs = vec![input.as_slice(), step_size.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        quantizer.process_block(&inputs, &mut output, 44100.0, &context);

        // Same input, different steps, different outputs
        assert_eq!(output[0], 1.0); // 1.4 quantized to step 1.0 -> 1.0
        assert_eq!(output[1], 1.5); // 1.4 quantized to step 0.5 -> 1.5
        assert_eq!(output[2], 1.5); // 1.4 quantized to step 0.25 -> 1.5
        assert_eq!(output[3], 2.0); // 1.4 quantized to step 2.0 -> 2.0
    }

    #[test]
    fn test_quantizer_zero_step_protection() {
        // Test: Zero or very small step sizes are protected
        let mut quantizer = QuantizerNode::new(0, 1);

        let input = vec![5.0, 5.0, 5.0, 5.0];
        // Test with zero and very small steps
        let step_size = vec![0.0, 0.00001, -0.1, 0.0001];
        let inputs = vec![input.as_slice(), step_size.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        quantizer.process_block(&inputs, &mut output, 44100.0, &context);

        // All should use minimum step of 0.0001 (prevents division by zero)
        for &sample in &output {
            // With step = 0.0001, 5.0 quantizes to 5.0
            assert!((sample - 5.0).abs() < 0.01);
            assert!(sample.is_finite()); // No infinities or NaN
        }
    }

    #[test]
    fn test_quantizer_with_constants() {
        // Integration test with ConstantNode
        let mut const_input = ConstantNode::new(3.7);
        let mut const_step = ConstantNode::new(1.0);
        let mut quantizer = QuantizerNode::new(0, 1);

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        // Process constants first
        let mut buf_input = vec![0.0; 512];
        let mut buf_step = vec![0.0; 512];

        const_input.process_block(&[], &mut buf_input, 44100.0, &context);
        const_step.process_block(&[], &mut buf_step, 44100.0, &context);

        // Now quantize
        let inputs = vec![buf_input.as_slice(), buf_step.as_slice()];
        let mut output = vec![0.0; 512];

        quantizer.process_block(&inputs, &mut output, 44100.0, &context);

        // 3.7 quantized to step 1.0 should be 4.0
        for sample in &output {
            assert_eq!(*sample, 4.0);
        }
    }
}
