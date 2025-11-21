/// Equal To node - compares two input signals with tolerance
///
/// This node implements sample-by-sample comparison: |a - b| < tolerance.
/// Returns 1.0 when a equals b (within epsilon), otherwise 0.0.
/// Useful for creating triggers, gates, and conditional logic based on equality.
///
/// Uses floating-point epsilon (default 1e-6) to handle numerical precision issues.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Equal To node: out = (|a - b| < tolerance) ? 1.0 : 0.0
///
/// # Example
/// ```ignore
/// // Compare two signals: trigger when A equals B
/// let signal_a = OscillatorNode::new(...);  // NodeId 0
/// let target = ConstantNode::new(0.5);      // NodeId 1
/// let gate = EqualToNode::new(0, 1, 1e-6);  // NodeId 2
/// // Output will be 1.0 when signal_a ≈ 0.5 (within 1e-6), else 0.0
/// ```
pub struct EqualToNode {
    input_a: NodeId,
    input_b: NodeId,
    tolerance: f32,
}

impl EqualToNode {
    /// EqualTo - Compares two signals for equality with tolerance
    ///
    /// Outputs 1.0 when inputs are equal within floating-point epsilon,
    /// useful for creating triggers and conditional logic.
    ///
    /// # Parameters
    /// - `input_a`: NodeId providing first signal
    /// - `input_b`: NodeId providing second signal
    /// - `tolerance`: Epsilon for floating-point comparison (default: 1e-6)
    ///
    /// # Example
    /// ```phonon
    /// ~sig_a: sine 110
    /// ~trigger: ~sig_a # equal_to 0.5 0.001
    /// ```
    pub fn new(input_a: NodeId, input_b: NodeId, tolerance: f32) -> Self {
        Self { input_a, input_b, tolerance }
    }

    /// Create a new equal to node with default tolerance (1e-6)
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

    /// Get the comparison tolerance
    pub fn tolerance(&self) -> f32 {
        self.tolerance
    }
}

impl AudioNode for EqualToNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "EqualToNode requires 2 inputs, got {}",
            inputs.len()
        );

        let buf_a = inputs[0];
        let buf_b = inputs[1];

        debug_assert_eq!(
            buf_a.len(),
            output.len(),
            "Input A length mismatch"
        );
        debug_assert_eq!(
            buf_b.len(),
            output.len(),
            "Input B length mismatch"
        );

        // Vectorized comparison: 1.0 if |a - b| < tolerance, else 0.0
        for i in 0..output.len() {
            output[i] = if (buf_a[i] - buf_b[i]).abs() < self.tolerance { 1.0 } else { 0.0 };
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input_a, self.input_b]
    }

    fn name(&self) -> &str {
        "EqualToNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_equal_to_true_case() {
        let mut eq = EqualToNode::new(0, 1, 1e-6);

        // a == b case: should output 1.0
        let input_a = vec![5.0, 10.0, 0.0, -3.0];
        let input_b = vec![5.0, 10.0, 0.0, -3.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        eq.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0);  // 5 == 5
        assert_eq!(output[1], 1.0);  // 10 == 10
        assert_eq!(output[2], 1.0);  // 0 == 0
        assert_eq!(output[3], 1.0);  // -3 == -3
    }

    #[test]
    fn test_equal_to_false_case() {
        let mut eq = EqualToNode::new(0, 1, 1e-6);

        // a != b case: should output 0.0
        let input_a = vec![1.0, 5.0, 50.0, 1.5];
        let input_b = vec![5.0, 10.0, 100.0, 2.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        eq.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0);  // 1 != 5
        assert_eq!(output[1], 0.0);  // 5 != 10
        assert_eq!(output[2], 0.0);  // 50 != 100
        assert_eq!(output[3], 0.0);  // 1.5 != 2
    }

    #[test]
    fn test_equal_to_within_tolerance() {
        let mut eq = EqualToNode::new(0, 1, 1e-6);

        // Values within tolerance: should output 1.0
        let input_a = vec![5.0, 10.0, 0.0, -3.0];
        let input_b = vec![5.0 + 5e-7, 10.0 - 5e-7, 0.0 + 5e-7, -3.0 - 5e-7];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        eq.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0);  // |5.0 - 5.0000005| < 1e-6 → true
        assert_eq!(output[1], 1.0);  // |10.0 - 9.9999995| < 1e-6 → true
        assert_eq!(output[2], 1.0);  // |0.0 - 0.0000005| < 1e-6 → true
        assert_eq!(output[3], 1.0);  // |-3.0 - -3.0000005| < 1e-6 → true
    }

    #[test]
    fn test_equal_to_outside_tolerance() {
        let mut eq = EqualToNode::new(0, 1, 1e-6);

        // Values outside tolerance: should output 0.0
        let input_a = vec![5.0, 10.0, 0.0, -3.0];
        let input_b = vec![5.0 + 1e-5, 10.0 - 1e-5, 0.0 + 1e-5, -3.0 - 1e-5];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        eq.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0);  // |5.0 - 5.00001| >= 1e-6 → false
        assert_eq!(output[1], 0.0);  // |10.0 - 9.99999| >= 1e-6 → false
        assert_eq!(output[2], 0.0);  // |0.0 - 0.00001| >= 1e-6 → false
        assert_eq!(output[3], 0.0);  // |-3.0 - -3.00001| >= 1e-6 → false
    }

    #[test]
    fn test_equal_to_with_constants() {
        let mut const_a = ConstantNode::new(75.0);
        let mut const_b = ConstantNode::new(75.0);
        let mut eq = EqualToNode::new(0, 1, 1e-6);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        // Process constants first
        let mut buf_a = vec![0.0; 512];
        let mut buf_b = vec![0.0; 512];

        const_a.process_block(&[], &mut buf_a, 44100.0, &context);
        const_b.process_block(&[], &mut buf_b, 44100.0, &context);

        // Now compare them
        let inputs = vec![buf_a.as_slice(), buf_b.as_slice()];
        let mut output = vec![0.0; 512];

        eq.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be 1.0 (75 == 75)
        for sample in &output {
            assert_eq!(*sample, 1.0);
        }
    }

    #[test]
    fn test_equal_to_negative_values() {
        let mut eq = EqualToNode::new(0, 1, 1e-6);

        // Test with negative values
        let input_a = vec![-1.0, -5.0, 3.0, -10.0];
        let input_b = vec![-1.0, -5.0, 3.0, -10.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        eq.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0);  // -1 == -1 → true
        assert_eq!(output[1], 1.0);  // -5 == -5 → true
        assert_eq!(output[2], 1.0);  // 3 == 3 → true
        assert_eq!(output[3], 1.0);  // -10 == -10 → true
    }

    #[test]
    fn test_equal_to_dependencies() {
        let eq = EqualToNode::new(5, 10, 1e-6);
        let deps = eq.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 5);
        assert_eq!(deps[1], 10);
    }

    #[test]
    fn test_equal_to_mixed_values() {
        let mut eq = EqualToNode::new(0, 1, 1e-6);

        // Mix of true and false cases
        let input_a = vec![10.0, 5.0, 8.0, 3.0];
        let input_b = vec![10.0, 10.0, 8.0, 7.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        eq.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0);  // 10 == 10 → true
        assert_eq!(output[1], 0.0);  // 5 != 10 → false
        assert_eq!(output[2], 1.0);  // 8 == 8 → true
        assert_eq!(output[3], 0.0);  // 3 != 7 → false
    }

    #[test]
    fn test_equal_to_custom_tolerance() {
        let mut eq = EqualToNode::new(0, 1, 0.1);  // Larger tolerance

        // Values within 0.1 tolerance: should output 1.0
        let input_a = vec![5.0, 10.0, 0.0, -3.0];
        let input_b = vec![5.05, 9.95, 0.08, -2.92];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        eq.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0);  // |5.0 - 5.05| = 0.05 < 0.1 → true
        assert_eq!(output[1], 1.0);  // |10.0 - 9.95| = 0.05 < 0.1 → true
        assert_eq!(output[2], 1.0);  // |0.0 - 0.08| = 0.08 < 0.1 → true
        assert_eq!(output[3], 1.0);  // |-3.0 - -2.92| = 0.08 < 0.1 → true
    }

    #[test]
    fn test_equal_to_zero_detection() {
        let mut eq = EqualToNode::new(0, 1, 1e-6);

        // Use case: detect zero crossings or exact values
        let signal = vec![0.0, 1e-7, -1e-7, 1e-5];
        let zero = vec![0.0, 0.0, 0.0, 0.0];
        let inputs = vec![signal.as_slice(), zero.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        eq.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0);  // 0.0 == 0.0 → true
        assert_eq!(output[1], 1.0);  // |1e-7 - 0| < 1e-6 → true
        assert_eq!(output[2], 1.0);  // |-1e-7 - 0| < 1e-6 → true
        assert_eq!(output[3], 0.0);  // |1e-5 - 0| >= 1e-6 → false
    }
}
