/// Less-than comparison node - compares two input signals
///
/// This node demonstrates comparison logic and boolean signal generation.
/// Output[i] = 1.0 if Input_A[i] < Input_B[i], otherwise 0.0.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Less-than comparison node: out = (a < b) ? 1.0 : 0.0
///
/// # Example
/// ```ignore
/// // Compare 440 < 880 → true (1.0)
/// let const_a = ConstantNode::new(440.0);  // NodeId 0
/// let const_b = ConstantNode::new(880.0);  // NodeId 1
/// let lt = LessThanNode::new(0, 1);        // NodeId 2
/// // Output will be 1.0
/// ```
pub struct LessThanNode {
    input_a: NodeId,
    input_b: NodeId,
}

impl LessThanNode {
    /// Create a new less-than comparison node
    ///
    /// # Arguments
    /// * `input_a` - NodeId of first input
    /// * `input_b` - NodeId of second input
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

impl AudioNode for LessThanNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "LessThanNode requires 2 inputs, got {}",
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

        // Vectorized comparison
        for i in 0..output.len() {
            output[i] = if buf_a[i] < buf_b[i] { 1.0 } else { 0.0 };
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input_a, self.input_b]
    }

    fn name(&self) -> &str {
        "LessThanNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_less_than_true_case() {
        let mut lt = LessThanNode::new(0, 1);

        // Create input buffers where a < b
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

        lt.process_block(&inputs, &mut output, 44100.0, &context);

        // All comparisons should be true (1.0)
        assert_eq!(output[0], 1.0);  // 1 < 2
        assert_eq!(output[1], 1.0);  // 2 < 3
        assert_eq!(output[2], 1.0);  // 3 < 4
        assert_eq!(output[3], 1.0);  // 4 < 5
    }

    #[test]
    fn test_less_than_false_case() {
        let mut lt = LessThanNode::new(0, 1);

        // Create input buffers where a >= b
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

        lt.process_block(&inputs, &mut output, 44100.0, &context);

        // All comparisons should be false (0.0)
        assert_eq!(output[0], 0.0);  // 10 >= 5
        assert_eq!(output[1], 0.0);  // 20 >= 10
        assert_eq!(output[2], 0.0);  // 30 >= 15
        assert_eq!(output[3], 0.0);  // 40 >= 20
    }

    #[test]
    fn test_less_than_equal_case() {
        let mut lt = LessThanNode::new(0, 1);

        // Create input buffers where a == b
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

        lt.process_block(&inputs, &mut output, 44100.0, &context);

        // Equal values should return false (0.0)
        assert_eq!(output[0], 0.0);  // 5 == 5
        assert_eq!(output[1], 0.0);  // 10 == 10
        assert_eq!(output[2], 0.0);  // 15 == 15
        assert_eq!(output[3], 0.0);  // 20 == 20
    }

    #[test]
    fn test_less_than_mixed_values() {
        let mut lt = LessThanNode::new(0, 1);

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

        lt.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0);  // 1 < 5 → true
        assert_eq!(output[1], 0.0);  // 10 >= 5 → false
        assert_eq!(output[2], 0.0);  // 5 == 5 → false
        assert_eq!(output[3], 0.0);  // 20 >= 10 → false
    }

    #[test]
    fn test_less_than_with_constants() {
        let mut const_a = ConstantNode::new(100.0);
        let mut const_b = ConstantNode::new(200.0);
        let mut lt = LessThanNode::new(0, 1);

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

        // Now compare them (100 < 200 → true)
        let inputs = vec![buf_a.as_slice(), buf_b.as_slice()];
        let mut output = vec![0.0; 512];

        lt.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be 1.0 (100 < 200)
        for sample in &output {
            assert_eq!(*sample, 1.0);
        }
    }

    #[test]
    fn test_less_than_negative_values() {
        let mut lt = LessThanNode::new(0, 1);

        // Test with negative values
        let input_a = vec![-10.0, -5.0, 0.0, 5.0];
        let input_b = vec![-5.0, -10.0, 0.0, 10.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        lt.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0);  // -10 < -5 → true
        assert_eq!(output[1], 0.0);  // -5 >= -10 → false
        assert_eq!(output[2], 0.0);  // 0 == 0 → false
        assert_eq!(output[3], 1.0);  // 5 < 10 → true
    }

    #[test]
    fn test_less_than_dependencies() {
        let lt = LessThanNode::new(5, 10);
        let deps = lt.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 5);
        assert_eq!(deps[1], 10);
    }
}
