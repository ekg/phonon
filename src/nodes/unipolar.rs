/// Unipolar converter - maps signals to 0-1 range
///
/// Converts bipolar signals (-1 to 1) to unipolar (0 to 1).
/// Useful for using oscillators as control signals.
use crate::audio_node::{AudioNode, NodeId, ProcessContext};

pub struct UnipolarNode {
    input: NodeId,
}

impl UnipolarNode {
    pub fn new(input: NodeId) -> Self {
        Self { input }
    }
}

impl AudioNode for UnipolarNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(inputs.len() >= 1, "UnipolarNode requires 1 input");

        let input_buffer = inputs[0];
        debug_assert_eq!(
            input_buffer.len(),
            output.len(),
            "Input buffer length mismatch"
        );

        for i in 0..output.len() {
            // Map [-1, 1] to [0, 1]
            output[i] = ((input_buffer[i] + 1.0) / 2.0).clamp(0.0, 1.0);
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input]
    }

    fn name(&self) -> &str {
        "UnipolarNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::Fraction;

    #[test]
    fn test_unipolar_conversion() {
        let mut node = UnipolarNode::new(0);

        let input = vec![-1.0, -0.5, 0.0, 0.5, 1.0];
        let inputs = vec![input.as_slice()];
        let mut output = vec![0.0; 5];

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 5, 2.0, 44100.0);

        node.process_block(&inputs, &mut output, 44100.0, &context);

        // Verify mapping: [-1, 1] -> [0, 1]
        assert_eq!(output[0], 0.0); // -1 -> 0
        assert_eq!(output[1], 0.25); // -0.5 -> 0.25
        assert_eq!(output[2], 0.5); // 0 -> 0.5
        assert_eq!(output[3], 0.75); // 0.5 -> 0.75
        assert_eq!(output[4], 1.0); // 1 -> 1
    }
}
