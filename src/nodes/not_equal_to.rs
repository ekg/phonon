/// Not Equal To node - compares two input signals with tolerance
///
/// This node implements sample-by-sample comparison: a != b (with epsilon).
/// Returns 1.0 when |a - b| >= tolerance, otherwise 0.0.
/// Useful for creating gates, triggers, and conditional logic.
use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Not Equal To node: out = (|a - b| >= tolerance) ? 1.0 : 0.0
///
/// This is the inverse of EqualToNode. Uses floating-point epsilon comparison
/// to handle numerical precision issues.
///
/// # Example
/// ```ignore
/// // Compare two signals: trigger when A differs from B
/// let signal_a = OscillatorNode::new(...);      // NodeId 0
/// let reference = ConstantNode::new(0.5);       // NodeId 1
/// let gate = NotEqualToNode::new(0, 1, 1e-6);   // NodeId 2
/// // Output will be 1.0 when |signal_a - 0.5| >= 1e-6, else 0.0
/// ```
pub struct NotEqualToNode {
    input_a: NodeId,
    input_b: NodeId,
    tolerance: f32,
}

impl NotEqualToNode {
    /// NotEqualToNode - Floating-point inequality comparison with tolerance
    ///
    /// Compares two input signals and outputs 1.0 when they differ beyond tolerance,
    /// 0.0 when they're approximately equal. Used for conditional logic and pattern
    /// generation based on signal differences.
    ///
    /// # Parameters
    /// - `input_a`: NodeId of first value to compare
    /// - `input_b`: NodeId of second value to compare
    /// - `tolerance`: Epsilon for floating-point comparison (default: 1e-6)
    ///
    /// # Example
    /// ```phonon
    /// ~val1: 5.0
    /// ~val2: 5.1
    /// ~result: not_equal_to ~val1 ~val2 0.01
    /// ```
    pub fn new(input_a: NodeId, input_b: NodeId, tolerance: f32) -> Self {
        Self {
            input_a,
            input_b,
            tolerance,
        }
    }

    /// Create with default tolerance (1e-6)
    pub fn with_default_tolerance(input_a: NodeId, input_b: NodeId) -> Self {
        Self::new(input_a, input_b, 1e-6)
    }

    /// Get the first input node ID
    pub fn input_a(&self) -> NodeId {
        self.input_a
    }

    /// Get the second input node ID
    pub fn input_b(&self) -> NodeId {
        self.input_b
    }

    /// Get the tolerance value
    pub fn tolerance(&self) -> f32 {
        self.tolerance
    }
}

impl AudioNode for NotEqualToNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "NotEqualToNode requires 2 inputs, got {}",
            inputs.len()
        );

        let buf_a = inputs[0];
        let buf_b = inputs[1];

        debug_assert_eq!(buf_a.len(), output.len(), "Input A length mismatch");
        debug_assert_eq!(buf_b.len(), output.len(), "Input B length mismatch");

        // Vectorized comparison: 1.0 if |a - b| >= tolerance, else 0.0
        for i in 0..output.len() {
            output[i] = if (buf_a[i] - buf_b[i]).abs() >= self.tolerance {
                1.0
            } else {
                0.0
            };
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input_a, self.input_b]
    }

    fn name(&self) -> &str {
        "NotEqualToNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_not_equal_to_true_case() {
        let mut neq = NotEqualToNode::new(0, 1, 1e-6);

        // a != b case: should output 1.0
        let input_a = vec![5.0, 10.0, 100.0, 2.0];
        let input_b = vec![1.0, 5.0, 50.0, 1.5];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        neq.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0); // 5 != 1
        assert_eq!(output[1], 1.0); // 10 != 5
        assert_eq!(output[2], 1.0); // 100 != 50
        assert_eq!(output[3], 1.0); // 2 != 1.5
    }

    #[test]
    fn test_not_equal_to_false_case() {
        let mut neq = NotEqualToNode::new(0, 1, 1e-6);

        // a == b case: should output 0.0
        let input_a = vec![5.0, 10.0, 0.0, -3.0];
        let input_b = vec![5.0, 10.0, 0.0, -3.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        neq.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0); // 5 == 5
        assert_eq!(output[1], 0.0); // 10 == 10
        assert_eq!(output[2], 0.0); // 0 == 0
        assert_eq!(output[3], 0.0); // -3 == -3
    }

    #[test]
    fn test_not_equal_to_within_tolerance() {
        let mut neq = NotEqualToNode::new(0, 1, 0.01);

        // Values very close (within tolerance): should output 0.0
        let input_a = vec![5.0, 10.0, 100.0, 1.0];
        let input_b = vec![5.005, 10.008, 100.003, 1.0001];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        neq.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0); // |5 - 5.005| = 0.005 < 0.01
        assert_eq!(output[1], 0.0); // |10 - 10.008| = 0.008 < 0.01
        assert_eq!(output[2], 0.0); // |100 - 100.003| = 0.003 < 0.01
        assert_eq!(output[3], 0.0); // |1 - 1.0001| = 0.0001 < 0.01
    }

    #[test]
    fn test_not_equal_to_outside_tolerance() {
        let mut neq = NotEqualToNode::new(0, 1, 0.01);

        // Values different (outside tolerance): should output 1.0
        let input_a = vec![5.0, 10.0, 100.0, 1.0];
        let input_b = vec![5.02, 9.98, 100.05, 0.98];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        neq.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0); // |5 - 5.02| = 0.02 >= 0.01
        assert_eq!(output[1], 1.0); // |10 - 9.98| = 0.02 >= 0.01
        assert_eq!(output[2], 1.0); // |100 - 100.05| = 0.05 >= 0.01
        assert_eq!(output[3], 1.0); // |1 - 0.98| = 0.02 >= 0.01
    }

    #[test]
    fn test_not_equal_to_with_constants() {
        let mut const_a = ConstantNode::new(75.0);
        let mut const_b = ConstantNode::new(75.0);
        let mut neq = NotEqualToNode::new(0, 1, 1e-6);

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        // Process constants first
        let mut buf_a = vec![0.0; 512];
        let mut buf_b = vec![0.0; 512];

        const_a.process_block(&[], &mut buf_a, 44100.0, &context);
        const_b.process_block(&[], &mut buf_b, 44100.0, &context);

        // Now compare them
        let inputs = vec![buf_a.as_slice(), buf_b.as_slice()];
        let mut output = vec![0.0; 512];

        neq.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be 0.0 (75.0 == 75.0)
        for sample in &output {
            assert_eq!(*sample, 0.0);
        }
    }

    #[test]
    fn test_not_equal_to_negative_values() {
        let mut neq = NotEqualToNode::new(0, 1, 1e-6);

        // Test with negative values
        let input_a = vec![-1.0, -5.0, 3.0, -10.0];
        let input_b = vec![-1.0, -5.1, 3.0, -10.05];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        neq.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0); // -1 == -1 → false (equal)
        assert_eq!(output[1], 1.0); // -5 != -5.1 → true
        assert_eq!(output[2], 0.0); // 3 == 3 → false (equal)
        assert_eq!(output[3], 1.0); // -10 != -10.05 → true
    }

    #[test]
    fn test_not_equal_to_dependencies() {
        let neq = NotEqualToNode::new(5, 10, 1e-6);
        let deps = neq.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 5);
        assert_eq!(deps[1], 10);
    }

    #[test]
    fn test_not_equal_to_change_detector() {
        let mut neq = NotEqualToNode::new(0, 1, 0.01);

        // Use case: detect when signal changes from reference value
        let signal = vec![0.5, 0.5, 0.7, 0.7, 0.5, 0.5];
        let reference = vec![0.5, 0.5, 0.5, 0.5, 0.5, 0.5];
        let inputs = vec![signal.as_slice(), reference.as_slice()];

        let mut output = vec![0.0; 6];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 6, 2.0, 44100.0);

        neq.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0); // 0.5 == 0.5 → no change
        assert_eq!(output[1], 0.0); // 0.5 == 0.5 → no change
        assert_eq!(output[2], 1.0); // 0.7 != 0.5 → changed!
        assert_eq!(output[3], 1.0); // 0.7 != 0.5 → changed!
        assert_eq!(output[4], 0.0); // 0.5 == 0.5 → back to reference
        assert_eq!(output[5], 0.0); // 0.5 == 0.5 → still at reference
    }

    #[test]
    fn test_not_equal_to_mixed_values() {
        let mut neq = NotEqualToNode::new(0, 1, 1e-6);

        // Mix of equal and not-equal cases
        let input_a = vec![10.0, 5.0, 8.0, 3.0];
        let input_b = vec![10.0, 10.0, 8.0, 7.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        neq.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0); // 10 == 10 → false
        assert_eq!(output[1], 1.0); // 5 != 10 → true
        assert_eq!(output[2], 0.0); // 8 == 8 → false
        assert_eq!(output[3], 1.0); // 3 != 7 → true
    }

    #[test]
    fn test_not_equal_to_default_tolerance() {
        let mut neq = NotEqualToNode::with_default_tolerance(0, 1);

        // Verify default tolerance is 1e-6
        assert_eq!(neq.tolerance(), 1e-6);

        // Test that very small differences are considered equal
        let input_a = vec![1.0, 2.0, 3.0];
        let input_b = vec![1.0000001, 2.0000001, 3.0000001];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 3];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 3, 2.0, 44100.0);

        neq.process_block(&inputs, &mut output, 44100.0, &context);

        // All differences < 1e-6, should be considered equal (output 0.0)
        assert_eq!(output[0], 0.0);
        assert_eq!(output[1], 0.0);
        assert_eq!(output[2], 0.0);
    }
}
