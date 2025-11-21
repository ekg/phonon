/// Less-than-or-equal comparison node - compares two input signals with tolerance
///
/// This node implements sample-by-sample comparison: a <= b.
/// Returns 1.0 when a <= b (within epsilon), otherwise 0.0.
/// Useful for creating gates, triggers, and conditional logic with threshold detection.
///
/// Uses floating-point epsilon (default 1e-6) to handle numerical precision issues.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Less-than-or-equal comparison node: out = (a <= b + tolerance) ? 1.0 : 0.0
///
/// # Example
/// ```ignore
/// // Compare signal <= threshold: trigger when signal is at or below threshold
/// let signal = OscillatorNode::new(...);  // NodeId 0
/// let threshold = ConstantNode::new(0.5); // NodeId 1
/// let gate = LessThanOrEqualNode::new(0, 1); // NodeId 2
/// // Output will be 1.0 when signal <= 0.5, else 0.0
/// ```
pub struct LessThanOrEqualNode {
    input_a: NodeId,
    input_b: NodeId,
    tolerance: f32,
}

impl LessThanOrEqualNode {
    /// Less-Than-or-Equal - Comparison gate with tolerance
    ///
    /// Outputs 1.0 when input_a <= input_b (within tolerance), else 0.0.
    /// Useful for threshold detection and conditional signal routing.
    ///
    /// # Parameters
    /// - `input_a`: First signal (left side of comparison)
    /// - `input_b`: Second signal (right side of comparison)
    /// - `tolerance`: Floating-point epsilon (default: 1e-6)
    ///
    /// # Example
    /// ```phonon
    /// ~signal: sine 220
    /// ~threshold: 0.5
    /// ~gate: ~signal # lte ~threshold 1e-6
    /// out: sine 440 * ~gate
    /// ```
    pub fn new(input_a: NodeId, input_b: NodeId, tolerance: f32) -> Self {
        Self { input_a, input_b, tolerance }
    }

    /// Create a new less-than-or-equal node with default tolerance (1e-6)
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

impl AudioNode for LessThanOrEqualNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "LessThanOrEqualNode requires 2 inputs, got {}",
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

        // Vectorized comparison: 1.0 if a <= b + tolerance, else 0.0
        // This handles both a < b and a == b cases with floating-point precision
        for i in 0..output.len() {
            output[i] = if buf_a[i] <= buf_b[i] + self.tolerance { 1.0 } else { 0.0 };
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input_a, self.input_b]
    }

    fn name(&self) -> &str {
        "LessThanOrEqualNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_lte_less_than_case() {
        let mut lte = LessThanOrEqualNode::new(0, 1, 1e-6);

        // a < b case: should output 1.0
        let input_a = vec![1.0, 2.0, 3.0, 4.0];
        let input_b = vec![2.0, 3.0, 4.0, 5.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        lte.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0);  // 1 < 2 → true
        assert_eq!(output[1], 1.0);  // 2 < 3 → true
        assert_eq!(output[2], 1.0);  // 3 < 4 → true
        assert_eq!(output[3], 1.0);  // 4 < 5 → true
    }

    #[test]
    fn test_lte_equal_case() {
        let mut lte = LessThanOrEqualNode::new(0, 1, 1e-6);

        // a == b case: should output 1.0 (equality counts as "less than or equal")
        let input_a = vec![5.0, 10.0, 15.0, 20.0];
        let input_b = vec![5.0, 10.0, 15.0, 20.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        lte.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0);  // 5 == 5 → true
        assert_eq!(output[1], 1.0);  // 10 == 10 → true
        assert_eq!(output[2], 1.0);  // 15 == 15 → true
        assert_eq!(output[3], 1.0);  // 20 == 20 → true
    }

    #[test]
    fn test_lte_greater_than_case() {
        let mut lte = LessThanOrEqualNode::new(0, 1, 1e-6);

        // a > b case: should output 0.0
        let input_a = vec![10.0, 20.0, 30.0, 40.0];
        let input_b = vec![5.0, 10.0, 15.0, 20.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        lte.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0);  // 10 > 5 → false
        assert_eq!(output[1], 0.0);  // 20 > 10 → false
        assert_eq!(output[2], 0.0);  // 30 > 15 → false
        assert_eq!(output[3], 0.0);  // 40 > 20 → false
    }

    #[test]
    fn test_lte_mixed_values() {
        let mut lte = LessThanOrEqualNode::new(0, 1, 1e-6);

        // Mix of true and false cases
        let input_a = vec![1.0, 10.0, 5.0, 20.0];
        let input_b = vec![5.0, 5.0, 5.0, 10.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        lte.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0);  // 1 <= 5 → true
        assert_eq!(output[1], 0.0);  // 10 > 5 → false
        assert_eq!(output[2], 1.0);  // 5 <= 5 → true (equality)
        assert_eq!(output[3], 0.0);  // 20 > 10 → false
    }

    #[test]
    fn test_lte_with_tolerance() {
        let mut lte = LessThanOrEqualNode::new(0, 1, 1e-6);

        // Values within tolerance of equality: should output 1.0
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

        lte.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0);  // 5.0 <= 5.0000005 → true (a <= b)
        assert_eq!(output[1], 1.0);  // 10.0 <= 9.9999995 → true (within tolerance)
        assert_eq!(output[2], 1.0);  // 0.0 <= 0.0000005 → true (a < b)
        assert_eq!(output[3], 1.0);  // -3.0 <= -3.0000005 → true (within tolerance)
    }

    #[test]
    fn test_lte_with_constants() {
        let mut const_a = ConstantNode::new(100.0);
        let mut const_b = ConstantNode::new(200.0);
        let mut lte = LessThanOrEqualNode::new(0, 1, 1e-6);

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

        // Now compare them (100 <= 200 → true)
        let inputs = vec![buf_a.as_slice(), buf_b.as_slice()];
        let mut output = vec![0.0; 512];

        lte.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be 1.0 (100 <= 200)
        for sample in &output {
            assert_eq!(*sample, 1.0);
        }
    }

    #[test]
    fn test_lte_negative_values() {
        let mut lte = LessThanOrEqualNode::new(0, 1, 1e-6);

        // Test with negative values
        let input_a = vec![-10.0, -5.0, 0.0, 5.0];
        let input_b = vec![-5.0, -5.0, 0.0, 10.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        lte.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0);  // -10 <= -5 → true
        assert_eq!(output[1], 1.0);  // -5 <= -5 → true (equality)
        assert_eq!(output[2], 1.0);  // 0 <= 0 → true (equality)
        assert_eq!(output[3], 1.0);  // 5 <= 10 → true
    }

    #[test]
    fn test_lte_dependencies() {
        let lte = LessThanOrEqualNode::new(5, 10, 1e-6);
        let deps = lte.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 5);
        assert_eq!(deps[1], 10);
    }

    #[test]
    fn test_lte_threshold_gate() {
        let mut lte = LessThanOrEqualNode::new(0, 1, 1e-6);

        // Use case: threshold gate (signal at or below 0.5 outputs 1.0)
        let signal = vec![0.2, 0.6, 0.9, 0.3, 0.5, 0.1];
        let threshold = vec![0.5, 0.5, 0.5, 0.5, 0.5, 0.5];
        let inputs = vec![signal.as_slice(), threshold.as_slice()];

        let mut output = vec![0.0; 6];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            6,
            2.0,
            44100.0,
        );

        lte.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0);  // 0.2 <= 0.5 → gate open
        assert_eq!(output[1], 0.0);  // 0.6 > 0.5 → gate closed
        assert_eq!(output[2], 0.0);  // 0.9 > 0.5 → gate closed
        assert_eq!(output[3], 1.0);  // 0.3 <= 0.5 → gate open
        assert_eq!(output[4], 1.0);  // 0.5 <= 0.5 → gate open (boundary case)
        assert_eq!(output[5], 1.0);  // 0.1 <= 0.5 → gate open
    }

    #[test]
    fn test_lte_zero_boundary() {
        let mut lte = LessThanOrEqualNode::new(0, 1, 1e-6);

        // Test boundary cases around zero
        let input_a = vec![-0.1, 0.0, 0.1, -1e-7, 1e-7];
        let input_b = vec![0.0, 0.0, 0.0, 0.0, 0.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 5];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            5,
            2.0,
            44100.0,
        );

        lte.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0);  // -0.1 <= 0.0 → true
        assert_eq!(output[1], 1.0);  // 0.0 <= 0.0 → true
        assert_eq!(output[2], 0.0);  // 0.1 > 0.0 → false
        assert_eq!(output[3], 1.0);  // -1e-7 <= 0.0 → true
        assert_eq!(output[4], 1.0);  // 1e-7 <= 0.0 (within tolerance) → true
    }

    #[test]
    fn test_lte_entire_buffer() {
        let mut lte = LessThanOrEqualNode::new(0, 1, 1e-6);

        // Test entire buffer with pattern
        let mut input_a = vec![0.0; 512];
        let mut input_b = vec![0.0; 512];

        // Create a pattern where first half is <= and second half is >
        for i in 0..256 {
            input_a[i] = i as f32 / 512.0;        // 0.0 to ~0.5
            input_b[i] = 0.5;
        }
        for i in 256..512 {
            input_a[i] = i as f32 / 512.0;        // ~0.5 to 1.0
            input_b[i] = 0.5;
        }

        let inputs = vec![input_a.as_slice(), input_b.as_slice()];
        let mut output = vec![0.0; 512];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        lte.process_block(&inputs, &mut output, 44100.0, &context);

        // First half should mostly be 1.0 (values <= 0.5)
        let first_half_sum: f32 = output[0..256].iter().sum();
        assert!(first_half_sum > 250.0);  // Most should be 1.0

        // Second half should mostly be 0.0 (values > 0.5)
        let second_half_sum: f32 = output[256..512].iter().sum();
        assert!(second_half_sum < 10.0);  // Most should be 0.0
    }
}
