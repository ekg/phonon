/// Greater-than-or-equal comparison node - compares two input signals with tolerance
///
/// This node implements sample-by-sample comparison: a >= b.
/// Returns 1.0 when a >= b (within epsilon), otherwise 0.0.
/// Useful for creating gates, triggers, and conditional logic with threshold detection.
///
/// Uses floating-point epsilon (default 1e-6) to handle numerical precision issues.
use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Greater-than-or-equal comparison node: out = (a >= b - tolerance) ? 1.0 : 0.0
///
/// # Example
/// ```ignore
/// // Compare two signals: trigger when A meets or exceeds B
/// let signal_a = OscillatorNode::new(...);  // NodeId 0
/// let threshold = ConstantNode::new(0.5);   // NodeId 1
/// let gate = GreaterThanOrEqualNode::new(0, 1, 1e-6);    // NodeId 2
/// // Output will be 1.0 when signal_a >= 0.5, else 0.0
/// ```
pub struct GreaterThanOrEqualNode {
    input_a: NodeId,
    input_b: NodeId,
    tolerance: f32,
}

impl GreaterThanOrEqualNode {
    /// GreaterThanOrEqual - Compares two signals with >= operator and tolerance
    ///
    /// Outputs 1.0 when first signal >= second signal within floating-point epsilon,
    /// useful for gates and conditional logic.
    ///
    /// # Parameters
    /// - `input_a`: NodeId providing first signal (left side)
    /// - `input_b`: NodeId providing second signal (right side)
    /// - `tolerance`: Epsilon for floating-point comparison (default: 1e-6)
    ///
    /// # Example
    /// ```phonon
    /// ~sig_a: sine 110
    /// ~trigger: ~sig_a # greater_than_or_equal 0.5 0.001
    /// ```
    pub fn new(input_a: NodeId, input_b: NodeId, tolerance: f32) -> Self {
        Self {
            input_a,
            input_b,
            tolerance,
        }
    }

    /// Create a new greater-than-or-equal node with default tolerance (1e-6)
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

impl AudioNode for GreaterThanOrEqualNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "GreaterThanOrEqualNode requires 2 inputs, got {}",
            inputs.len()
        );

        let buf_a = inputs[0];
        let buf_b = inputs[1];

        debug_assert_eq!(buf_a.len(), output.len(), "Input A length mismatch");
        debug_assert_eq!(buf_b.len(), output.len(), "Input B length mismatch");

        // Vectorized comparison: 1.0 if a >= b - tolerance, else 0.0
        // This handles both a > b and a == b cases with floating-point precision
        for i in 0..output.len() {
            output[i] = if buf_a[i] >= buf_b[i] - self.tolerance {
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
        "GreaterThanOrEqualNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_gte_greater_case() {
        let mut gte = GreaterThanOrEqualNode::new(0, 1, 1e-6);

        // a > b case: should output 1.0
        let input_a = vec![5.0, 10.0, 100.0, 2.0];
        let input_b = vec![1.0, 5.0, 50.0, 1.5];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        gte.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0); // 5 > 1
        assert_eq!(output[1], 1.0); // 10 > 5
        assert_eq!(output[2], 1.0); // 100 > 50
        assert_eq!(output[3], 1.0); // 2 > 1.5
    }

    #[test]
    fn test_gte_equal_case() {
        let mut gte = GreaterThanOrEqualNode::new(0, 1, 1e-6);

        // a == b case: should output 1.0 (greater than or EQUAL)
        let input_a = vec![5.0, 10.0, 0.0, -3.0];
        let input_b = vec![5.0, 10.0, 0.0, -3.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        gte.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0); // 5 == 5 → true (equal)
        assert_eq!(output[1], 1.0); // 10 == 10 → true (equal)
        assert_eq!(output[2], 1.0); // 0 == 0 → true (equal)
        assert_eq!(output[3], 1.0); // -3 == -3 → true (equal)
    }

    #[test]
    fn test_gte_less_case() {
        let mut gte = GreaterThanOrEqualNode::new(0, 1, 1e-6);

        // a < b case: should output 0.0
        let input_a = vec![1.0, 5.0, 50.0, 1.5];
        let input_b = vec![5.0, 10.0, 100.0, 2.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        gte.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0); // 1 < 5
        assert_eq!(output[1], 0.0); // 5 < 10
        assert_eq!(output[2], 0.0); // 50 < 100
        assert_eq!(output[3], 0.0); // 1.5 < 2
    }

    #[test]
    fn test_gte_mixed_values() {
        let mut gte = GreaterThanOrEqualNode::new(0, 1, 1e-6);

        // Mix of greater, equal, and less cases
        let input_a = vec![10.0, 5.0, 8.0, 3.0];
        let input_b = vec![5.0, 10.0, 8.0, 7.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        gte.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0); // 10 >= 5 → true (greater)
        assert_eq!(output[1], 0.0); // 5 < 10 → false
        assert_eq!(output[2], 1.0); // 8 >= 8 → true (equal)
        assert_eq!(output[3], 0.0); // 3 < 7 → false
    }

    #[test]
    fn test_gte_with_constants() {
        let mut const_a = ConstantNode::new(75.0);
        let mut const_b = ConstantNode::new(50.0);
        let mut gte = GreaterThanOrEqualNode::new(0, 1, 1e-6);

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        // Process constants first
        let mut buf_a = vec![0.0; 512];
        let mut buf_b = vec![0.0; 512];

        const_a.process_block(&[], &mut buf_a, 44100.0, &context);
        const_b.process_block(&[], &mut buf_b, 44100.0, &context);

        // Now compare them
        let inputs = vec![buf_a.as_slice(), buf_b.as_slice()];
        let mut output = vec![0.0; 512];

        gte.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be 1.0 (75 >= 50)
        for sample in &output {
            assert_eq!(*sample, 1.0);
        }
    }

    #[test]
    fn test_gte_negative_values() {
        let mut gte = GreaterThanOrEqualNode::new(0, 1, 1e-6);

        // Test with negative values
        let input_a = vec![-1.0, -5.0, 3.0, -10.0];
        let input_b = vec![-5.0, -1.0, -3.0, -5.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        gte.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0); // -1 >= -5 → true
        assert_eq!(output[1], 0.0); // -5 < -1 → false
        assert_eq!(output[2], 1.0); // 3 >= -3 → true
        assert_eq!(output[3], 0.0); // -10 < -5 → false
    }

    #[test]
    fn test_gte_dependencies() {
        let gte = GreaterThanOrEqualNode::new(5, 10, 1e-6);
        let deps = gte.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 5);
        assert_eq!(deps[1], 10);
    }

    #[test]
    fn test_gte_threshold_gate() {
        let mut gte = GreaterThanOrEqualNode::new(0, 1, 1e-6);

        // Use case: threshold gate (signal at or above 0.5 outputs 1.0)
        let signal = vec![0.2, 0.5, 0.9, 0.3, 0.7, 0.1];
        let threshold = vec![0.5, 0.5, 0.5, 0.5, 0.5, 0.5];
        let inputs = vec![signal.as_slice(), threshold.as_slice()];

        let mut output = vec![0.0; 6];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 6, 2.0, 44100.0);

        gte.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0); // 0.2 < 0.5 → gate closed
        assert_eq!(output[1], 1.0); // 0.5 >= 0.5 → gate open (equal)
        assert_eq!(output[2], 1.0); // 0.9 >= 0.5 → gate open
        assert_eq!(output[3], 0.0); // 0.3 < 0.5 → gate closed
        assert_eq!(output[4], 1.0); // 0.7 >= 0.5 → gate open
        assert_eq!(output[5], 0.0); // 0.1 < 0.5 → gate closed
    }

    #[test]
    fn test_gte_with_tolerance() {
        let mut gte = GreaterThanOrEqualNode::new(0, 1, 1e-6);

        // Values within tolerance of equality: should output 1.0
        let input_a = vec![5.0, 10.0 + 5e-7, 0.0, -3.0];
        let input_b = vec![5.0 - 5e-7, 10.0, 0.0 - 5e-7, -3.0 + 5e-7];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        gte.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0); // 5.0 >= 4.9999995 → true (a > b)
        assert_eq!(output[1], 1.0); // 10.0000005 >= 10.0 → true (a > b)
        assert_eq!(output[2], 1.0); // 0.0 >= -0.0000005 → true (within tolerance)
        assert_eq!(output[3], 1.0); // -3.0 >= -2.9999995 → true (within tolerance)
    }

    #[test]
    fn test_gte_zero_boundary() {
        let mut gte = GreaterThanOrEqualNode::new(0, 1, 1e-6);

        // Test boundary cases around zero
        let input_a = vec![0.1, 0.0, -0.1, 1e-7, -1e-7];
        let input_b = vec![0.0, 0.0, 0.0, 0.0, 0.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 5];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 5, 2.0, 44100.0);

        gte.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0); // 0.1 >= 0.0 → true
        assert_eq!(output[1], 1.0); // 0.0 >= 0.0 → true
        assert_eq!(output[2], 0.0); // -0.1 < 0.0 → false
        assert_eq!(output[3], 1.0); // 1e-7 >= 0.0 (within tolerance) → true
        assert_eq!(output[4], 1.0); // -1e-7 >= 0.0 (within tolerance) → true
    }

    #[test]
    fn test_gte_entire_buffer() {
        let mut gte = GreaterThanOrEqualNode::new(0, 1, 1e-6);

        // Test entire buffer with pattern
        let mut input_a = vec![0.0; 512];
        let mut input_b = vec![0.0; 512];

        // Create a pattern where first half is < and second half is >=
        for i in 0..256 {
            input_a[i] = i as f32 / 512.0; // 0.0 to ~0.5
            input_b[i] = 0.5;
        }
        for i in 256..512 {
            input_a[i] = i as f32 / 512.0; // ~0.5 to 1.0
            input_b[i] = 0.5;
        }

        let inputs = vec![input_a.as_slice(), input_b.as_slice()];
        let mut output = vec![0.0; 512];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        gte.process_block(&inputs, &mut output, 44100.0, &context);

        // First half should mostly be 0.0 (values < 0.5)
        let first_half_sum: f32 = output[0..256].iter().sum();
        assert!(first_half_sum < 10.0); // Most should be 0.0

        // Second half should mostly be 1.0 (values >= 0.5)
        let second_half_sum: f32 = output[256..512].iter().sum();
        assert!(second_half_sum > 250.0); // Most should be 1.0
    }
}
