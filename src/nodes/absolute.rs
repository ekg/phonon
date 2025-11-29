/// Absolute value node - takes absolute value of input signal
///
/// This node performs full-wave rectification on the input signal.
/// Output[i] = |Input[i]| for all samples.
use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Absolute value node: out = |input|
///
/// # Example
/// ```ignore
/// // Rectify a sine wave
/// let freq = ConstantNode::new(440.0);      // NodeId 0
/// let sine = OscillatorNode::new(0, Waveform::Sine);  // NodeId 1
/// let abs = AbsoluteNode::new(1);           // NodeId 2
/// // Output will be full-wave rectified sine
/// ```
pub struct AbsoluteNode {
    input: NodeId,
}

impl AbsoluteNode {
    /// Absolute - Full-wave rectification of input signal
    ///
    /// Takes the absolute value of each sample, converting all negative values to positive.
    ///
    /// # Parameters
    /// - `input`: Input signal to rectify
    ///
    /// # Example
    /// ```phonon
    /// ~signal: sine 440
    /// ~rectified: ~signal # absolute
    /// ```
    pub fn new(input: NodeId) -> Self {
        Self { input }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }
}

impl AudioNode for AbsoluteNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(!inputs.is_empty(), "AbsoluteNode requires 1 input, got 0");

        let buf = inputs[0];

        debug_assert_eq!(buf.len(), output.len(), "Input length mismatch");

        // Apply absolute value to each sample
        for i in 0..output.len() {
            output[i] = buf[i].abs();
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input]
    }

    fn name(&self) -> &str {
        "AbsoluteNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_absolute_node_positive_stays_positive() {
        let mut abs_node = AbsoluteNode::new(0);

        let input = vec![1.0, 2.0, 3.0, 4.5, 100.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 5];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 5, 2.0, 44100.0);

        abs_node.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0);
        assert_eq!(output[1], 2.0);
        assert_eq!(output[2], 3.0);
        assert_eq!(output[3], 4.5);
        assert_eq!(output[4], 100.0);
    }

    #[test]
    fn test_absolute_node_negative_becomes_positive() {
        let mut abs_node = AbsoluteNode::new(0);

        let input = vec![-1.0, -2.0, -3.0, -4.5, -100.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 5];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 5, 2.0, 44100.0);

        abs_node.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0);
        assert_eq!(output[1], 2.0);
        assert_eq!(output[2], 3.0);
        assert_eq!(output[3], 4.5);
        assert_eq!(output[4], 100.0);
    }

    #[test]
    fn test_absolute_node_zero_stays_zero() {
        let mut abs_node = AbsoluteNode::new(0);

        let input = vec![0.0, 0.0, 0.0, -0.0, 0.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 5];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 5, 2.0, 44100.0);

        abs_node.process_block(&inputs, &mut output, 44100.0, &context);

        for sample in &output {
            assert_eq!(*sample, 0.0);
        }
    }

    #[test]
    fn test_absolute_node_mixed_values() {
        let mut abs_node = AbsoluteNode::new(0);

        let input = vec![1.0, -2.0, 3.0, -4.0, 0.0, -5.5];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 6];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 6, 2.0, 44100.0);

        abs_node.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0);
        assert_eq!(output[1], 2.0);
        assert_eq!(output[2], 3.0);
        assert_eq!(output[3], 4.0);
        assert_eq!(output[4], 0.0);
        assert_eq!(output[5], 5.5);
    }

    #[test]
    fn test_absolute_node_sine_wave_rectification() {
        use std::f32::consts::PI;

        let mut abs_node = AbsoluteNode::new(0);

        // Create one cycle of a sine wave (16 samples)
        let mut input = Vec::new();
        for i in 0..16 {
            let phase = (i as f32 / 16.0) * 2.0 * PI;
            input.push(phase.sin());
        }
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 16];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 16, 2.0, 44100.0);

        abs_node.process_block(&inputs, &mut output, 44100.0, &context);

        // All output values should be positive
        for sample in &output {
            assert!(*sample >= 0.0, "Sample {} should be non-negative", sample);
        }

        // First half of sine is positive (should be unchanged)
        for i in 0..8 {
            let phase = (i as f32 / 16.0) * 2.0 * PI;
            let expected = phase.sin();
            assert!(
                (output[i] - expected).abs() < 0.0001,
                "Sample {} should be close to original sine value {}",
                output[i],
                expected
            );
        }

        // Second half should be flipped positive
        for i in 8..16 {
            let phase = (i as f32 / 16.0) * 2.0 * PI;
            let expected = phase.sin().abs();
            assert!(
                (output[i] - expected).abs() < 0.0001,
                "Sample {} should be absolute value of sine",
                output[i]
            );
        }
    }

    #[test]
    fn test_absolute_node_dependencies() {
        let abs_node = AbsoluteNode::new(7);
        let deps = abs_node.input_nodes();

        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], 7);
    }

    #[test]
    fn test_absolute_node_with_constant() {
        let mut const_node = ConstantNode::new(-5.0);
        let mut abs_node = AbsoluteNode::new(0);

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        // Process constant first
        let mut buf = vec![0.0; 512];
        const_node.process_block(&[], &mut buf, 44100.0, &context);

        // Now take absolute value
        let inputs = vec![buf.as_slice()];
        let mut output = vec![0.0; 512];

        abs_node.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be 5.0 (|-5.0|)
        for sample in &output {
            assert_eq!(*sample, 5.0);
        }
    }
}
