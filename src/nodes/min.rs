/// Minimum node - outputs the minimum of two input signals
///
/// This node performs sample-by-sample minimum comparison.
/// Output[i] = min(Input_A[i], Input_B[i]) for all samples.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Minimum node: out = min(a, b)
///
/// # Example
/// ```ignore
/// // Min of two constant values: min(3.0, 5.0) = 3.0
/// let const_a = ConstantNode::new(3.0);   // NodeId 0
/// let const_b = ConstantNode::new(5.0);   // NodeId 1
/// let min = MinNode::new(0, 1);           // NodeId 2
/// // Output will be 3.0
/// ```
pub struct MinNode {
    input_a: NodeId,
    input_b: NodeId,
}

impl MinNode {
    /// Create a new minimum node
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

impl AudioNode for MinNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "MinNode requires 2 inputs, got {}",
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

        // Vectorized minimum operation
        for i in 0..output.len() {
            output[i] = buf_a[i].min(buf_b[i]);
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input_a, self.input_b]
    }

    fn name(&self) -> &str {
        "MinNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_min_node_constants_3_and_5() {
        // Test: Min of constants (3.0, 5.0) = 3.0
        let mut const_a = ConstantNode::new(3.0);
        let mut const_b = ConstantNode::new(5.0);
        let mut min = MinNode::new(0, 1);

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

        // Now take minimum
        let inputs = vec![buf_a.as_slice(), buf_b.as_slice()];
        let mut output = vec![0.0; 512];

        min.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be 3.0 (min of 3.0 and 5.0)
        for sample in &output {
            assert_eq!(*sample, 3.0);
        }
    }

    #[test]
    fn test_min_node_with_negative() {
        // Test: Min with negative (-2.0, 1.0) = -2.0
        let mut const_a = ConstantNode::new(-2.0);
        let mut const_b = ConstantNode::new(1.0);
        let mut min = MinNode::new(0, 1);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        let mut buf_a = vec![0.0; 512];
        let mut buf_b = vec![0.0; 512];

        const_a.process_block(&[], &mut buf_a, 44100.0, &context);
        const_b.process_block(&[], &mut buf_b, 44100.0, &context);

        let inputs = vec![buf_a.as_slice(), buf_b.as_slice()];
        let mut output = vec![0.0; 512];

        min.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be -2.0 (min of -2.0 and 1.0)
        for sample in &output {
            assert_eq!(*sample, -2.0);
        }
    }

    #[test]
    fn test_min_node_buffer_comparison() {
        // Test: Min of oscillators (picks lower sample-by-sample)
        let mut min = MinNode::new(0, 1);

        // Create input buffers with varying values
        let input_a = vec![1.0, 5.0, 3.0, 8.0];
        let input_b = vec![2.0, 3.0, 7.0, 6.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        min.process_block(&inputs, &mut output, 44100.0, &context);

        // Verify each sample is the minimum
        assert_eq!(output[0], 1.0);  // min(1.0, 2.0)
        assert_eq!(output[1], 3.0);  // min(5.0, 3.0)
        assert_eq!(output[2], 3.0);  // min(3.0, 7.0)
        assert_eq!(output[3], 6.0);  // min(8.0, 6.0)
    }

    #[test]
    fn test_min_node_symmetric() {
        // Test: Symmetric (min(a,b) == min(b,a))
        let mut min_ab = MinNode::new(0, 1);
        let mut min_ba = MinNode::new(1, 0);

        let input_a = vec![3.0, -1.0, 5.0, 0.0];
        let input_b = vec![1.0, 2.0, -3.0, 0.0];

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        // Test min(a, b)
        let inputs_ab = vec![input_a.as_slice(), input_b.as_slice()];
        let mut output_ab = vec![0.0; 4];
        min_ab.process_block(&inputs_ab, &mut output_ab, 44100.0, &context);

        // Test min(b, a)
        let inputs_ba = vec![input_b.as_slice(), input_a.as_slice()];
        let mut output_ba = vec![0.0; 4];
        min_ba.process_block(&inputs_ba, &mut output_ba, 44100.0, &context);

        // Results should be identical
        for i in 0..4 {
            assert_eq!(output_ab[i], output_ba[i]);
        }
    }

    #[test]
    fn test_min_node_dependencies() {
        let min = MinNode::new(5, 10);
        let deps = min.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 5);
        assert_eq!(deps[1], 10);
    }
}
