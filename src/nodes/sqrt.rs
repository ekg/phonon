/// Square root node - computes square root with absolute value protection
///
/// This node computes the square root of the absolute value of the input signal.
/// Output[i] = sqrt(|Input[i]|) for all samples.
///
/// The abs() protection ensures no NaN values from negative inputs.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Square root node: out = sqrt(|input|)
///
/// # Example
/// ```ignore
/// // Square root of a constant
/// let const_node = ConstantNode::new(4.0);   // NodeId 0
/// let sqrt = SquareRootNode::new(0);         // NodeId 1
/// // Output will be 2.0
/// ```
pub struct SquareRootNode {
    input: NodeId,
}

impl SquareRootNode {
    /// SquareRootNode - Square root with absolute value protection
    ///
    /// Computes the square root of the input signal's absolute value,
    /// preventing NaN values from negative inputs.
    ///
    /// # Parameters
    /// - `input`: NodeId of input signal
    ///
    /// # Example
    /// ```phonon
    /// ~signal: sine 440 * sine 440
    /// ~rooted: ~signal # sqrt
    /// ```
    pub fn new(input: NodeId) -> Self {
        Self { input }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }
}

impl AudioNode for SquareRootNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            !inputs.is_empty(),
            "SquareRootNode requires 1 input, got 0"
        );

        let buf = inputs[0];

        debug_assert_eq!(
            buf.len(),
            output.len(),
            "Input length mismatch"
        );

        // Apply sqrt(abs(x)) to each sample to avoid NaN from negative inputs
        for i in 0..output.len() {
            output[i] = buf[i].abs().sqrt();
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input]
    }

    fn name(&self) -> &str {
        "SquareRootNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_sqrt_of_four_equals_two() {
        let mut sqrt_node = SquareRootNode::new(0);

        let input = vec![4.0, 4.0, 4.0, 4.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        sqrt_node.process_block(&inputs, &mut output, 44100.0, &context);

        for sample in &output {
            assert_eq!(*sample, 2.0);
        }
    }

    #[test]
    fn test_sqrt_of_nine_equals_three() {
        let mut sqrt_node = SquareRootNode::new(0);

        let input = vec![9.0, 9.0, 9.0, 9.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        sqrt_node.process_block(&inputs, &mut output, 44100.0, &context);

        for sample in &output {
            assert_eq!(*sample, 3.0);
        }
    }

    #[test]
    fn test_sqrt_of_negative_uses_abs() {
        let mut sqrt_node = SquareRootNode::new(0);

        // sqrt(-4) should use abs() and return 2.0, not NaN
        let input = vec![-4.0, -9.0, -16.0, -25.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        sqrt_node.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 2.0);  // sqrt(|-4|) = sqrt(4) = 2
        assert_eq!(output[1], 3.0);  // sqrt(|-9|) = sqrt(9) = 3
        assert_eq!(output[2], 4.0);  // sqrt(|-16|) = sqrt(16) = 4
        assert_eq!(output[3], 5.0);  // sqrt(|-25|) = sqrt(25) = 5

        // Ensure no NaN values
        for sample in &output {
            assert!(!sample.is_nan(), "sqrt should not produce NaN with abs() protection");
        }
    }

    #[test]
    fn test_sqrt_dependencies() {
        let sqrt_node = SquareRootNode::new(7);
        let deps = sqrt_node.input_nodes();

        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], 7);
    }

    #[test]
    fn test_sqrt_of_zero() {
        let mut sqrt_node = SquareRootNode::new(0);

        let input = vec![0.0, 0.0, 0.0, 0.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![99.9; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        sqrt_node.process_block(&inputs, &mut output, 44100.0, &context);

        for sample in &output {
            assert_eq!(*sample, 0.0);
        }
    }

    #[test]
    fn test_sqrt_of_one() {
        let mut sqrt_node = SquareRootNode::new(0);

        let input = vec![1.0, 1.0, 1.0, 1.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        sqrt_node.process_block(&inputs, &mut output, 44100.0, &context);

        for sample in &output {
            assert_eq!(*sample, 1.0);
        }
    }

    #[test]
    fn test_sqrt_various_values() {
        let mut sqrt_node = SquareRootNode::new(0);

        let input = vec![0.0, 1.0, 4.0, 9.0, 16.0, 25.0, 100.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 7];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            7,
            2.0,
            44100.0,
        );

        sqrt_node.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0);
        assert_eq!(output[1], 1.0);
        assert_eq!(output[2], 2.0);
        assert_eq!(output[3], 3.0);
        assert_eq!(output[4], 4.0);
        assert_eq!(output[5], 5.0);
        assert_eq!(output[6], 10.0);
    }

    #[test]
    fn test_sqrt_mixed_positive_negative() {
        let mut sqrt_node = SquareRootNode::new(0);

        // Mix of positive, negative, and zero - all should use abs()
        let input = vec![4.0, -4.0, 9.0, -9.0, 0.0, 16.0, -16.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 7];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            7,
            2.0,
            44100.0,
        );

        sqrt_node.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 2.0);
        assert_eq!(output[1], 2.0);  // sqrt(abs(-4)) = 2
        assert_eq!(output[2], 3.0);
        assert_eq!(output[3], 3.0);  // sqrt(abs(-9)) = 3
        assert_eq!(output[4], 0.0);
        assert_eq!(output[5], 4.0);
        assert_eq!(output[6], 4.0);  // sqrt(abs(-16)) = 4

        // Ensure no NaN values
        for sample in &output {
            assert!(!sample.is_nan());
        }
    }

    #[test]
    fn test_sqrt_with_constant_node() {
        let mut const_node = ConstantNode::new(25.0);
        let mut sqrt_node = SquareRootNode::new(0);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        // Process constant first
        let mut buf = vec![0.0; 512];
        const_node.process_block(&[], &mut buf, 44100.0, &context);

        // Now take square root
        let inputs = vec![buf.as_slice()];
        let mut output = vec![0.0; 512];

        sqrt_node.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be 5.0 (sqrt(25))
        for sample in &output {
            assert_eq!(*sample, 5.0);
        }
    }

    #[test]
    fn test_sqrt_fractional_values() {
        let mut sqrt_node = SquareRootNode::new(0);

        let input = vec![0.25, 0.5, 2.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 3];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            3,
            2.0,
            44100.0,
        );

        sqrt_node.process_block(&inputs, &mut output, 44100.0, &context);

        assert!((output[0] - 0.5).abs() < 0.0001);  // sqrt(0.25) ≈ 0.5
        assert!((output[1] - 0.707107).abs() < 0.001);  // sqrt(0.5) ≈ 0.707
        assert!((output[2] - 1.414214).abs() < 0.001);  // sqrt(2) ≈ 1.414
    }
}
