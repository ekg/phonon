/// Subtraction node - subtracts signal B from signal A
///
/// This node demonstrates buffer combining and dependency handling.
/// Output[i] = Input_A[i] - Input_B[i] for all samples.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Subtraction node: out = a - b
///
/// # Example
/// ```ignore
/// // Subtract two constant values: 100 - 50 = 50
/// let const_a = ConstantNode::new(100.0);  // NodeId 0
/// let const_b = ConstantNode::new(50.0);   // NodeId 1
/// let sub = SubtractionNode::new(0, 1);    // NodeId 2
/// // Output will be 50.0
/// ```
pub struct SubtractionNode {
    input_a: NodeId,
    input_b: NodeId,
}

impl SubtractionNode {
    /// Subtraction - Subtracts signal B from signal A (A - B)
    ///
    /// Performs sample-by-sample subtraction: output[i] = a[i] - b[i].
    ///
    /// # Parameters
    /// - `input_a`: Minuend (signal to subtract from)
    /// - `input_b`: Subtrahend (signal to subtract)
    ///
    /// # Example
    /// ```phonon
    /// ~sig_a: sine 440
    /// ~sig_b: sine 330
    /// out: ~sig_a # sub ~sig_b
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

impl AudioNode for SubtractionNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "SubtractionNode requires 2 inputs, got {}",
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

        // Vectorized subtraction
        for i in 0..output.len() {
            output[i] = buf_a[i] - buf_b[i];
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input_a, self.input_b]
    }

    fn name(&self) -> &str {
        "SubtractionNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_subtraction_node_simple() {
        let mut sub = SubtractionNode::new(0, 1);

        // Create input buffers: 5.0 - 3.0 = 2.0
        let input_a = vec![5.0, 5.0, 5.0, 5.0];
        let input_b = vec![3.0, 3.0, 3.0, 3.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        sub.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 2.0);  // 5 - 3
        assert_eq!(output[1], 2.0);  // 5 - 3
        assert_eq!(output[2], 2.0);  // 5 - 3
        assert_eq!(output[3], 2.0);  // 5 - 3
    }

    #[test]
    fn test_subtraction_node_subtract_from_zero() {
        let mut sub = SubtractionNode::new(0, 1);

        // Create input buffers: 0.0 - 5.0 = -5.0
        let input_a = vec![0.0, 0.0, 0.0, 0.0];
        let input_b = vec![5.0, 5.0, 5.0, 5.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        sub.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], -5.0);  // 0 - 5
        assert_eq!(output[1], -5.0);  // 0 - 5
        assert_eq!(output[2], -5.0);  // 0 - 5
        assert_eq!(output[3], -5.0);  // 0 - 5
    }

    #[test]
    fn test_subtraction_node_subtract_negative() {
        let mut sub = SubtractionNode::new(0, 1);

        // Create input buffers: 5.0 - (-3.0) = 8.0
        let input_a = vec![5.0, 5.0, 5.0, 5.0];
        let input_b = vec![-3.0, -3.0, -3.0, -3.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        sub.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 8.0);  // 5 - (-3)
        assert_eq!(output[1], 8.0);  // 5 - (-3)
        assert_eq!(output[2], 8.0);  // 5 - (-3)
        assert_eq!(output[3], 8.0);  // 5 - (-3)
    }

    #[test]
    fn test_subtraction_node_dependencies() {
        let sub = SubtractionNode::new(5, 10);
        let deps = sub.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 5);
        assert_eq!(deps[1], 10);
    }

    #[test]
    fn test_subtraction_node_with_constants() {
        let mut const_a = ConstantNode::new(100.0);
        let mut const_b = ConstantNode::new(25.0);
        let mut sub = SubtractionNode::new(0, 1);

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

        // Now subtract them
        let inputs = vec![buf_a.as_slice(), buf_b.as_slice()];
        let mut output = vec![0.0; 512];

        sub.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be 75.0 (100 - 25)
        for sample in &output {
            assert_eq!(*sample, 75.0);
        }
    }

    #[test]
    fn test_subtraction_node_varying_values() {
        let mut sub = SubtractionNode::new(0, 1);

        let input_a = vec![10.0, 20.0, 30.0, 40.0];
        let input_b = vec![3.0, 7.0, 15.0, 25.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        sub.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 7.0);   // 10 - 3
        assert_eq!(output[1], 13.0);  // 20 - 7
        assert_eq!(output[2], 15.0);  // 30 - 15
        assert_eq!(output[3], 15.0);  // 40 - 25
    }
}
