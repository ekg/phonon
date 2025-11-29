/// Division node - divides signal A by signal B
///
/// This node demonstrates safe division with protection against division by zero.
/// Output[i] = Input_A[i] / Input_B[i] for all samples, with safeguards.
///
/// Division by zero protection: If |B[i]| < 1e-10, output is 0.0 to prevent NaN/infinity.
use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Division node: out = a / b
///
/// # Example
/// ```ignore
/// // Divide two constant values: 10.0 / 2.0 = 5.0
/// let const_a = ConstantNode::new(10.0);  // NodeId 0
/// let const_b = ConstantNode::new(2.0);   // NodeId 1
/// let div = DivisionNode::new(0, 1);      // NodeId 2
/// // Output will be 5.0
/// ```
///
/// # Division by Zero Protection
///
/// To prevent NaN and infinity values that can corrupt audio processing:
/// - If |input_b[i]| < 1e-10, output[i] = 0.0
/// - This threshold is chosen to be well below typical audio signal levels
pub struct DivisionNode {
    input_a: NodeId,
    input_b: NodeId,
}

impl DivisionNode {
    /// Division - Divides signal A by signal B with zero protection
    ///
    /// Performs element-wise division with safeguards against division by zero,
    /// outputting 0.0 when denominator is near zero.
    ///
    /// # Parameters
    /// - `input_a`: NodeId providing numerator (dividend)
    /// - `input_b`: NodeId providing denominator (divisor)
    ///
    /// # Example
    /// ```phonon
    /// ~signal_a: sine 110
    /// ~signal_b: sine 55 * 0.5 + 0.5
    /// ~result: ~signal_a # division ~signal_b
    /// ```
    pub fn new(input_a: NodeId, input_b: NodeId) -> Self {
        Self { input_a, input_b }
    }

    /// Get the numerator input node ID
    pub fn input_a(&self) -> NodeId {
        self.input_a
    }

    /// Get the denominator input node ID
    pub fn input_b(&self) -> NodeId {
        self.input_b
    }
}

impl AudioNode for DivisionNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "DivisionNode requires 2 inputs, got {}",
            inputs.len()
        );

        let buf_a = inputs[0];
        let buf_b = inputs[1];

        debug_assert_eq!(buf_a.len(), output.len(), "Input A length mismatch");
        debug_assert_eq!(buf_b.len(), output.len(), "Input B length mismatch");

        // Safe division with zero protection
        const EPSILON: f32 = 1e-10;

        for i in 0..output.len() {
            if buf_b[i].abs() < EPSILON {
                output[i] = 0.0; // Division by zero protection
            } else {
                output[i] = buf_a[i] / buf_b[i];
            }
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input_a, self.input_b]
    }

    fn name(&self) -> &str {
        "DivisionNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_division_node_simple() {
        let mut div = DivisionNode::new(0, 1);

        // Create input buffers: simple division
        let input_a = vec![10.0, 20.0, 30.0, 40.0];
        let input_b = vec![2.0, 4.0, 5.0, 8.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        div.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 5.0); // 10 / 2
        assert_eq!(output[1], 5.0); // 20 / 4
        assert_eq!(output[2], 6.0); // 30 / 5
        assert_eq!(output[3], 5.0); // 40 / 8
    }

    #[test]
    fn test_division_node_with_constants() {
        let mut const_a = ConstantNode::new(10.0);
        let mut const_b = ConstantNode::new(2.0);
        let mut div = DivisionNode::new(0, 1);

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        // Process constants first
        let mut buf_a = vec![0.0; 512];
        let mut buf_b = vec![0.0; 512];

        const_a.process_block(&[], &mut buf_a, 44100.0, &context);
        const_b.process_block(&[], &mut buf_b, 44100.0, &context);

        // Now divide them
        let inputs = vec![buf_a.as_slice(), buf_b.as_slice()];
        let mut output = vec![0.0; 512];

        div.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be 5.0 (10 / 2)
        for sample in &output {
            assert_eq!(*sample, 5.0);
        }
    }

    #[test]
    fn test_division_node_by_small_number() {
        let mut div = DivisionNode::new(0, 1);

        // Divide by very small but non-zero numbers
        let input_a = vec![1.0, 2.0, 3.0, 4.0];
        let input_b = vec![0.1, 0.01, 0.001, 0.0001];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        div.process_block(&inputs, &mut output, 44100.0, &context);

        // Use approximate equality for floating point division
        assert!(
            (output[0] - 10.0).abs() < 0.01,
            "Expected ~10.0, got {}",
            output[0]
        );
        assert!(
            (output[1] - 200.0).abs() < 0.01,
            "Expected ~200.0, got {}",
            output[1]
        );
        assert!(
            (output[2] - 3000.0).abs() < 0.01,
            "Expected ~3000.0, got {}",
            output[2]
        );
        assert!(
            (output[3] - 40000.0).abs() < 0.01,
            "Expected ~40000.0, got {}",
            output[3]
        );

        // Verify none are NaN or infinity
        for sample in &output {
            assert!(
                sample.is_finite(),
                "Output should be finite, got {}",
                sample
            );
        }
    }

    #[test]
    fn test_division_node_zero_protection() {
        let mut div = DivisionNode::new(0, 1);

        // Test division by zero and near-zero values
        let input_a = vec![10.0, 20.0, 30.0, 40.0];
        let input_b = vec![0.0, 1e-11, -1e-11, 0.0]; // All below epsilon threshold
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![999.0; 4]; // Initialize with non-zero to verify overwrite
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        div.process_block(&inputs, &mut output, 44100.0, &context);

        // All should be 0.0 due to division by zero protection
        assert_eq!(output[0], 0.0, "Division by zero should yield 0.0");
        assert_eq!(output[1], 0.0, "Division by near-zero should yield 0.0");
        assert_eq!(
            output[2], 0.0,
            "Division by negative near-zero should yield 0.0"
        );
        assert_eq!(output[3], 0.0, "Division by zero should yield 0.0");

        // Verify no NaN or infinity
        for sample in &output {
            assert!(
                sample.is_finite(),
                "Output should be finite, got {}",
                sample
            );
            assert!(!sample.is_nan(), "Output should not be NaN, got {}", sample);
        }
    }

    #[test]
    fn test_division_node_negative_values() {
        let mut div = DivisionNode::new(0, 1);

        // Test with negative numerators and denominators
        let input_a = vec![10.0, -20.0, 30.0, -40.0];
        let input_b = vec![2.0, 4.0, -5.0, -8.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        div.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 5.0); // 10 / 2 = 5
        assert_eq!(output[1], -5.0); // -20 / 4 = -5
        assert_eq!(output[2], -6.0); // 30 / -5 = -6
        assert_eq!(output[3], 5.0); // -40 / -8 = 5
    }

    #[test]
    fn test_division_node_dependencies() {
        let div = DivisionNode::new(5, 10);
        let deps = div.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 5);
        assert_eq!(deps[1], 10);
    }

    #[test]
    fn test_division_node_fractional_results() {
        let mut div = DivisionNode::new(0, 1);

        // Test fractional division results
        let input_a = vec![1.0, 3.0, 7.0, 9.0];
        let input_b = vec![2.0, 2.0, 2.0, 2.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        div.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.5); // 1 / 2
        assert_eq!(output[1], 1.5); // 3 / 2
        assert_eq!(output[2], 3.5); // 7 / 2
        assert_eq!(output[3], 4.5); // 9 / 2
    }
}
