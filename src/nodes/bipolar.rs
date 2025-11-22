/// Bipolar clamper - clamps signals to -1 to 1 range
///
/// Ensures signals stay within the standard bipolar range.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

pub struct BipolarNode {
    input: NodeId,
}

impl BipolarNode {
    pub fn new(input: NodeId) -> Self {
        Self { input }
    }
}

impl AudioNode for BipolarNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 1,
            "BipolarNode requires 1 input"
        );

        let input_buffer = inputs[0];
        debug_assert_eq!(
            input_buffer.len(),
            output.len(),
            "Input buffer length mismatch"
        );

        for i in 0..output.len() {
            output[i] = input_buffer[i].clamp(-1.0, 1.0);
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input]
    }

    fn name(&self) -> &str {
        "BipolarNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::Fraction;

    #[test]
    fn test_bipolar_clamping() {
        let mut node = BipolarNode::new(0);

        let input = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
        let inputs = vec![input.as_slice()];
        let mut output = vec![0.0; 5];

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            5,
            2.0,
            44100.0,
        );

        node.process_block(&inputs, &mut output, 44100.0, &context);

        // Verify clamping to [-1, 1]
        assert_eq!(output[0], -1.0);  // -2 clamped to -1
        assert_eq!(output[1], -1.0);
        assert_eq!(output[2], 0.0);
        assert_eq!(output[3], 1.0);
        assert_eq!(output[4], 1.0);   // 2 clamped to 1
    }
}
