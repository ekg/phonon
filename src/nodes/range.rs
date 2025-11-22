/// Range mapper - maps input range to output range
///
/// Linear mapping from [in_min, in_max] to [out_min, out_max]

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

pub struct RangeNode {
    input: NodeId,
    in_min: f32,
    in_max: f32,
    out_min: f32,
    out_max: f32,
}

impl RangeNode {
    pub fn new(input: NodeId, in_min: f32, in_max: f32, out_min: f32, out_max: f32) -> Self {
        Self {
            input,
            in_min,
            in_max,
            out_min,
            out_max,
        }
    }
}

impl AudioNode for RangeNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 1,
            "RangeNode requires 1 input"
        );

        let input_buffer = inputs[0];
        debug_assert_eq!(
            input_buffer.len(),
            output.len(),
            "Input buffer length mismatch"
        );

        let in_range = self.in_max - self.in_min;
        let out_range = self.out_max - self.out_min;

        for i in 0..output.len() {
            // Normalize to 0-1
            let normalized = (input_buffer[i] - self.in_min) / in_range;
            // Map to output range
            let mapped = normalized * out_range + self.out_min;
            // Clamp to output range
            output[i] = mapped.clamp(self.out_min, self.out_max);
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input]
    }

    fn name(&self) -> &str {
        "RangeNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::Fraction;

    #[test]
    fn test_range_mapping() {
        let mut node = RangeNode::new(0, 0.0, 1.0, 10.0, 20.0);

        let input = vec![0.0, 0.25, 0.5, 0.75, 1.0];
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

        // Verify mapping [0, 1] -> [10, 20]
        assert_eq!(output[0], 10.0);   // 0 -> 10
        assert_eq!(output[1], 12.5);   // 0.25 -> 12.5
        assert_eq!(output[2], 15.0);   // 0.5 -> 15
        assert_eq!(output[3], 17.5);   // 0.75 -> 17.5
        assert_eq!(output[4], 20.0);   // 1 -> 20
    }
}
