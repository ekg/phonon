/// Invert node - phase inversion (multiply by -1)
///
/// This node inverts the phase of an audio signal by multiplying all samples by -1.
/// Useful for phase cancellation effects and signal processing.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Invert node: out = -input
///
/// # Example
/// ```ignore
/// // Invert a constant value: -5.0 becomes 5.0
/// let const_node = ConstantNode::new(5.0);   // NodeId 0
/// let invert = InvertNode::new(0);            // NodeId 1
/// // Output will be -5.0
/// ```
pub struct InvertNode {
    input: NodeId,
}

impl InvertNode {
    /// Invert - Phase inversion (multiply by -1)
    ///
    /// Inverts the phase of an audio signal by multiplying all samples by -1.
    /// Useful for phase cancellation effects and signal processing.
    ///
    /// # Parameters
    /// - `input`: Audio signal to invert
    ///
    /// # Example
    /// ```phonon
    /// ~signal: sine 220
    /// ~inverted: ~signal # invert
    /// out: (~signal + ~inverted) * 0.5
    /// ```
    pub fn new(input: NodeId) -> Self {
        Self { input }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }
}

impl AudioNode for InvertNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 1,
            "InvertNode requires 1 input, got {}",
            inputs.len()
        );

        let input_buf = inputs[0];

        debug_assert_eq!(
            input_buf.len(),
            output.len(),
            "Input length mismatch"
        );

        // Phase inversion: multiply by -1
        for i in 0..output.len() {
            output[i] = -input_buf[i];
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input]
    }

    fn name(&self) -> &str {
        "InvertNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_invert_positive_becomes_negative() {
        let mut invert = InvertNode::new(0);

        // Positive input values
        let input = vec![1.0, 2.0, 3.5, 100.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        invert.process_block(&inputs, &mut output, 44100.0, &context);

        // All positive values should become negative
        assert_eq!(output[0], -1.0);
        assert_eq!(output[1], -2.0);
        assert_eq!(output[2], -3.5);
        assert_eq!(output[3], -100.0);
    }

    #[test]
    fn test_invert_negative_becomes_positive() {
        let mut invert = InvertNode::new(0);

        // Negative input values
        let input = vec![-1.0, -2.0, -3.5, -100.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        invert.process_block(&inputs, &mut output, 44100.0, &context);

        // All negative values should become positive
        assert_eq!(output[0], 1.0);
        assert_eq!(output[1], 2.0);
        assert_eq!(output[2], 3.5);
        assert_eq!(output[3], 100.0);
    }

    #[test]
    fn test_invert_zero_stays_zero() {
        let mut invert = InvertNode::new(0);

        // Zero input values
        let input = vec![0.0, 0.0, 0.0, 0.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![99.9; 4]; // Initialize with non-zero to ensure zeros are written
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        invert.process_block(&inputs, &mut output, 44100.0, &context);

        // All zeros should stay zero (-0.0 == 0.0 in floating point)
        assert_eq!(output[0], 0.0);
        assert_eq!(output[1], 0.0);
        assert_eq!(output[2], 0.0);
        assert_eq!(output[3], 0.0);
    }

    #[test]
    fn test_invert_double_inversion() {
        let mut invert1 = InvertNode::new(0);
        let mut invert2 = InvertNode::new(1);

        // Original signal
        let input = vec![1.0, -2.0, 3.5, -100.0];
        let inputs1 = vec![input.as_slice()];

        let mut buf1 = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        // First inversion
        invert1.process_block(&inputs1, &mut buf1, 44100.0, &context);

        // Second inversion
        let inputs2 = vec![buf1.as_slice()];
        let mut buf2 = vec![0.0; 4];
        invert2.process_block(&inputs2, &mut buf2, 44100.0, &context);

        // Double inversion should restore original values
        assert_eq!(buf2[0], 1.0);
        assert_eq!(buf2[1], -2.0);
        assert_eq!(buf2[2], 3.5);
        assert_eq!(buf2[3], -100.0);
    }

    #[test]
    fn test_invert_with_constant_node() {
        let mut constant = ConstantNode::new(5.0);
        let mut invert = InvertNode::new(0);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        // Process constant first
        let mut const_buf = vec![0.0; 512];
        constant.process_block(&[], &mut const_buf, 44100.0, &context);

        // Invert it
        let inputs = vec![const_buf.as_slice()];
        let mut output = vec![0.0; 512];
        invert.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be -5.0
        for sample in &output {
            assert_eq!(*sample, -5.0);
        }
    }

    #[test]
    fn test_invert_dependencies() {
        let invert = InvertNode::new(42);
        let deps = invert.input_nodes();

        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], 42);
    }

    #[test]
    fn test_invert_mixed_values() {
        let mut invert = InvertNode::new(0);

        // Mixed positive, negative, and zero
        let input = vec![5.0, -3.0, 0.0, -7.5, 2.5, 0.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 6];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            6,
            2.0,
            44100.0,
        );

        invert.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], -5.0);
        assert_eq!(output[1], 3.0);
        assert_eq!(output[2], 0.0);
        assert_eq!(output[3], 7.5);
        assert_eq!(output[4], -2.5);
        assert_eq!(output[5], 0.0);
    }
}
