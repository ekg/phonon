/// Greater Than node - compares two input signals
///
/// This node implements sample-by-sample comparison: a > b.
/// Returns 1.0 when a > b, otherwise 0.0.
/// Useful for creating gates, triggers, and conditional logic.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Greater Than node: out = (a > b) ? 1.0 : 0.0
///
/// # Example
/// ```ignore
/// // Compare two signals: trigger when A exceeds B
/// let signal_a = OscillatorNode::new(...);  // NodeId 0
/// let threshold = ConstantNode::new(0.5);   // NodeId 1
/// let gate = GreaterThanNode::new(0, 1);    // NodeId 2
/// // Output will be 1.0 when signal_a > 0.5, else 0.0
/// ```
pub struct GreaterThanNode {
    input_a: NodeId,
    input_b: NodeId,
}

impl GreaterThanNode {
    /// Greater Than - Compares two signals and outputs a gate signal
    ///
    /// Outputs 1.0 when input_a > input_b, otherwise 0.0. Useful for creating
    /// gates, triggers, and conditional logic in audio processing.
    ///
    /// # Parameters
    /// - `input_a`: First input signal (left side of comparison)
    /// - `input_b`: Second input signal (right side of comparison)
    ///
    /// # Example
    /// ```phonon
    /// ~signal: sine 0.5
    /// ~threshold: 0.5
    /// ~gate: ~signal > ~threshold
    /// out: sine 220 * ~gate
    /// ```
    pub fn new(input_a: NodeId, input_b: NodeId) -> Self {
        Self { input_a, input_b }
    }

    /// Get the first input node ID
    pub fn input_a(&self) -> NodeId {
        self.input_a
    }

    /// Get the second input node ID
    pub fn input_b(&self) -> NodeId {
        self.input_b
    }
}

impl AudioNode for GreaterThanNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "GreaterThanNode requires 2 inputs, got {}",
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

        // Vectorized comparison: 1.0 if a > b, else 0.0
        for i in 0..output.len() {
            output[i] = if buf_a[i] > buf_b[i] { 1.0 } else { 0.0 };
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input_a, self.input_b]
    }

    fn name(&self) -> &str {
        "GreaterThanNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_greater_than_true_case() {
        let mut gt = GreaterThanNode::new(0, 1);

        // a > b case: should output 1.0
        let input_a = vec![5.0, 10.0, 100.0, 2.0];
        let input_b = vec![1.0, 5.0, 50.0, 1.5];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        gt.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0);  // 5 > 1
        assert_eq!(output[1], 1.0);  // 10 > 5
        assert_eq!(output[2], 1.0);  // 100 > 50
        assert_eq!(output[3], 1.0);  // 2 > 1.5
    }

    #[test]
    fn test_greater_than_false_case() {
        let mut gt = GreaterThanNode::new(0, 1);

        // a <= b case: should output 0.0
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

        gt.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0);  // 1 < 5
        assert_eq!(output[1], 0.0);  // 5 < 10
        assert_eq!(output[2], 0.0);  // 50 < 100
        assert_eq!(output[3], 0.0);  // 1.5 < 2
    }

    #[test]
    fn test_greater_than_equal_case() {
        let mut gt = GreaterThanNode::new(0, 1);

        // a == b case: should output 0.0 (not greater, just equal)
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

        gt.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0);  // 5 == 5
        assert_eq!(output[1], 0.0);  // 10 == 10
        assert_eq!(output[2], 0.0);  // 0 == 0
        assert_eq!(output[3], 0.0);  // -3 == -3
    }

    #[test]
    fn test_greater_than_mixed_values() {
        let mut gt = GreaterThanNode::new(0, 1);

        // Mix of true and false cases
        let input_a = vec![10.0, 5.0, 8.0, 3.0];
        let input_b = vec![5.0, 10.0, 8.0, 7.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        gt.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0);  // 10 > 5 → true
        assert_eq!(output[1], 0.0);  // 5 < 10 → false
        assert_eq!(output[2], 0.0);  // 8 == 8 → false (not greater)
        assert_eq!(output[3], 0.0);  // 3 < 7 → false
    }

    #[test]
    fn test_greater_than_with_constants() {
        let mut const_a = ConstantNode::new(75.0);
        let mut const_b = ConstantNode::new(50.0);
        let mut gt = GreaterThanNode::new(0, 1);

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

        gt.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be 1.0 (75 > 50)
        for sample in &output {
            assert_eq!(*sample, 1.0);
        }
    }

    #[test]
    fn test_greater_than_negative_values() {
        let mut gt = GreaterThanNode::new(0, 1);

        // Test with negative values
        let input_a = vec![-1.0, -5.0, 3.0, -10.0];
        let input_b = vec![-5.0, -1.0, -3.0, -5.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        gt.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0);  // -1 > -5 → true
        assert_eq!(output[1], 0.0);  // -5 < -1 → false
        assert_eq!(output[2], 1.0);  // 3 > -3 → true
        assert_eq!(output[3], 0.0);  // -10 < -5 → false
    }

    #[test]
    fn test_greater_than_dependencies() {
        let gt = GreaterThanNode::new(5, 10);
        let deps = gt.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 5);
        assert_eq!(deps[1], 10);
    }

    #[test]
    fn test_greater_than_threshold_gate() {
        let mut gt = GreaterThanNode::new(0, 1);

        // Use case: threshold gate (signal above 0.5 outputs 1.0)
        let signal = vec![0.2, 0.6, 0.9, 0.3, 0.7, 0.1];
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

        gt.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0);  // 0.2 <= 0.5 → gate closed
        assert_eq!(output[1], 1.0);  // 0.6 > 0.5 → gate open
        assert_eq!(output[2], 1.0);  // 0.9 > 0.5 → gate open
        assert_eq!(output[3], 0.0);  // 0.3 <= 0.5 → gate closed
        assert_eq!(output[4], 1.0);  // 0.7 > 0.5 → gate open
        assert_eq!(output[5], 0.0);  // 0.1 <= 0.5 → gate closed
    }
}
