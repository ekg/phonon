/// Addition node - adds two input signals
///
/// This node demonstrates buffer combining and dependency handling.
/// Output[i] = Input_A[i] + Input_B[i] for all samples.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Addition node: out = a + b
///
/// # Example
/// ```ignore
/// // Add two constant values: 440 + 110 = 550
/// let const_a = ConstantNode::new(440.0);  // NodeId 0
/// let const_b = ConstantNode::new(110.0);  // NodeId 1
/// let add = AdditionNode::new(0, 1);       // NodeId 2
/// // Output will be 550.0
/// ```
pub struct AdditionNode {
    input_a: NodeId,
    input_b: NodeId,
}

impl AdditionNode {
    /// Create a new addition node
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

impl AudioNode for AdditionNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "AdditionNode requires 2 inputs, got {}",
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

        // Vectorized addition
        for i in 0..output.len() {
            output[i] = buf_a[i] + buf_b[i];
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input_a, self.input_b]
    }

    fn name(&self) -> &str {
        "AdditionNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_addition_node_simple() {
        let mut add = AdditionNode::new(0, 1);

        // Create input buffers
        let input_a = vec![1.0, 2.0, 3.0, 4.0];
        let input_b = vec![10.0, 20.0, 30.0, 40.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        add.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 11.0);  // 1 + 10
        assert_eq!(output[1], 22.0);  // 2 + 20
        assert_eq!(output[2], 33.0);  // 3 + 30
        assert_eq!(output[3], 44.0);  // 4 + 40
    }

    #[test]
    fn test_addition_node_with_constants() {
        let mut const_a = ConstantNode::new(100.0);
        let mut const_b = ConstantNode::new(50.0);
        let mut add = AdditionNode::new(0, 1);

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

        // Now add them
        let inputs = vec![buf_a.as_slice(), buf_b.as_slice()];
        let mut output = vec![0.0; 512];

        add.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be 150.0 (100 + 50)
        for sample in &output {
            assert_eq!(*sample, 150.0);
        }
    }

    #[test]
    fn test_addition_node_dependencies() {
        let add = AdditionNode::new(5, 10);
        let deps = add.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 5);
        assert_eq!(deps[1], 10);
    }

    #[test]
    fn test_addition_node_negative_values() {
        let mut add = AdditionNode::new(0, 1);

        let input_a = vec![1.0, -2.0, 3.0, -4.0];
        let input_b = vec![-1.0, 2.0, -3.0, 4.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        add.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0);   // 1 + (-1)
        assert_eq!(output[1], 0.0);   // -2 + 2
        assert_eq!(output[2], 0.0);   // 3 + (-3)
        assert_eq!(output[3], 0.0);   // -4 + 4
    }
}
