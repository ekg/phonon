/// Multiplication node - multiplies two input signals
///
/// This node demonstrates buffer combining and dependency handling.
/// Output[i] = Input_A[i] * Input_B[i] for all samples.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Multiplication node: out = a * b
///
/// # Example
/// ```ignore
/// // Multiply two constant values: 3.0 * 2.0 = 6.0
/// let const_a = ConstantNode::new(3.0);   // NodeId 0
/// let const_b = ConstantNode::new(2.0);   // NodeId 1
/// let mul = MultiplicationNode::new(0, 1); // NodeId 2
/// // Output will be 6.0
/// ```
pub struct MultiplicationNode {
    input_a: NodeId,
    input_b: NodeId,
}

impl MultiplicationNode {
    /// Multiplication - Sample-by-sample product of two signals
    ///
    /// Multiplies two signals together sample-by-sample. Fundamental operation for
    /// amplitude modulation, gating, and mixing control signals with audio.
    ///
    /// # Parameters
    /// - `input_a`: First signal (multiplicand)
    /// - `input_b`: Second signal (multiplier)
    ///
    /// # Example
    /// ```phonon
    /// ~osc: sine 220
    /// ~env: impulse 2 # adsr 0.01 0.1 0.5 0.2
    /// ~gated: ~osc # * ~env
    /// out: ~gated * 0.5
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

impl AudioNode for MultiplicationNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "MultiplicationNode requires 2 inputs, got {}",
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

        // Vectorized multiplication
        for i in 0..output.len() {
            output[i] = buf_a[i] * buf_b[i];
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input_a, self.input_b]
    }

    fn name(&self) -> &str {
        "MultiplicationNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_multiplication_node_simple() {
        let mut mul = MultiplicationNode::new(0, 1);

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

        mul.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 10.0);   // 1 * 10
        assert_eq!(output[1], 40.0);   // 2 * 20
        assert_eq!(output[2], 90.0);   // 3 * 30
        assert_eq!(output[3], 160.0);  // 4 * 40
    }

    #[test]
    fn test_multiplication_node_with_constants() {
        let mut const_a = ConstantNode::new(3.0);
        let mut const_b = ConstantNode::new(2.0);
        let mut mul = MultiplicationNode::new(0, 1);

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

        // Now multiply them
        let inputs = vec![buf_a.as_slice(), buf_b.as_slice()];
        let mut output = vec![0.0; 512];

        mul.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be 6.0 (3 * 2)
        for sample in &output {
            assert_eq!(*sample, 6.0);
        }
    }

    #[test]
    fn test_multiplication_node_dependencies() {
        let mul = MultiplicationNode::new(5, 10);
        let deps = mul.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 5);
        assert_eq!(deps[1], 10);
    }

    #[test]
    fn test_multiplication_node_negative_values() {
        let mut mul = MultiplicationNode::new(0, 1);

        let input_a = vec![2.0, -3.0, 4.0, -5.0];
        let input_b = vec![3.0, 2.0, -2.0, -3.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        mul.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 6.0);    // 2 * 3
        assert_eq!(output[1], -6.0);   // -3 * 2
        assert_eq!(output[2], -8.0);   // 4 * -2
        assert_eq!(output[3], 15.0);   // -5 * -3
    }
}
