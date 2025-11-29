/// Clamp node - constrains input signal to [min, max] range
///
/// This node performs sample-by-sample clamping.
/// Output[i] = Input[i].clamp(Min[i], Max[i]) for all samples.
use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Clamp node: out = input.clamp(min, max)
///
/// # Example
/// ```ignore
/// // Clamp signal to [-0.5, 0.5] range
/// let input = OscillatorNode::new(0, Waveform::Sine);  // NodeId 1
/// let min = ConstantNode::new(-0.5);                    // NodeId 2
/// let max = ConstantNode::new(0.5);                     // NodeId 3
/// let clamp = ClampNode::new(1, 2, 3);                  // NodeId 4
/// // Output will be clamped sine wave
/// ```
pub struct ClampNode {
    input: NodeId,
    min_input: NodeId,
    max_input: NodeId,
}

impl ClampNode {
    /// Clamp - Constrains signal to [min, max] range
    ///
    /// Limits signal amplitude without clipping distortion, preserving signal shape.
    /// Useful for gate control, safety limiting, and range mapping.
    ///
    /// # Parameters
    /// - `input`: Signal to constrain
    /// - `min_input`: Minimum value (lower bound)
    /// - `max_input`: Maximum value (upper bound)
    ///
    /// # Example
    /// ```phonon
    /// ~signal: saw 220
    /// ~clamped: ~signal # clamp -0.5 0.5
    /// ```
    pub fn new(input: NodeId, min_input: NodeId, max_input: NodeId) -> Self {
        Self {
            input,
            min_input,
            max_input,
        }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }

    /// Get the min input node ID
    pub fn min_input(&self) -> NodeId {
        self.min_input
    }

    /// Get the max input node ID
    pub fn max_input(&self) -> NodeId {
        self.max_input
    }
}

impl AudioNode for ClampNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 3,
            "ClampNode requires 3 inputs, got {}",
            inputs.len()
        );

        let buf_input = inputs[0];
        let buf_min = inputs[1];
        let buf_max = inputs[2];

        debug_assert_eq!(
            buf_input.len(),
            output.len(),
            "Input buffer length mismatch"
        );
        debug_assert_eq!(buf_min.len(), output.len(), "Min buffer length mismatch");
        debug_assert_eq!(buf_max.len(), output.len(), "Max buffer length mismatch");

        // Sample-wise clamp operation
        for i in 0..output.len() {
            output[i] = buf_input[i].clamp(buf_min[i], buf_max[i]);
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.min_input, self.max_input]
    }

    fn name(&self) -> &str {
        "ClampNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_clamp_value_below_range_clamped_to_min() {
        // Test: Clamp -5.0 to [-1.0, 1.0] = -1.0
        let mut input_node = ConstantNode::new(-5.0);
        let mut min_node = ConstantNode::new(-1.0);
        let mut max_node = ConstantNode::new(1.0);
        let mut clamp = ClampNode::new(0, 1, 2);

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        // Process inputs
        let mut buf_input = vec![0.0; 512];
        let mut buf_min = vec![0.0; 512];
        let mut buf_max = vec![0.0; 512];

        input_node.process_block(&[], &mut buf_input, 44100.0, &context);
        min_node.process_block(&[], &mut buf_min, 44100.0, &context);
        max_node.process_block(&[], &mut buf_max, 44100.0, &context);

        // Process clamp
        let inputs = vec![buf_input.as_slice(), buf_min.as_slice(), buf_max.as_slice()];
        let mut output = vec![0.0; 512];

        clamp.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be -1.0 (clamped to min)
        for sample in &output {
            assert_eq!(*sample, -1.0);
        }
    }

    #[test]
    fn test_clamp_value_above_range_clamped_to_max() {
        // Test: Clamp 10.0 to [-1.0, 1.0] = 1.0
        let mut clamp = ClampNode::new(0, 1, 2);

        let input = vec![10.0; 512];
        let min = vec![-1.0; 512];
        let max = vec![1.0; 512];
        let inputs = vec![input.as_slice(), min.as_slice(), max.as_slice()];

        let mut output = vec![0.0; 512];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        clamp.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be 1.0 (clamped to max)
        for sample in &output {
            assert_eq!(*sample, 1.0);
        }
    }

    #[test]
    fn test_clamp_value_in_range_unchanged() {
        // Test: Clamp 0.5 to [-1.0, 1.0] = 0.5 (unchanged)
        let mut clamp = ClampNode::new(0, 1, 2);

        let input = vec![0.5; 512];
        let min = vec![-1.0; 512];
        let max = vec![1.0; 512];
        let inputs = vec![input.as_slice(), min.as_slice(), max.as_slice()];

        let mut output = vec![0.0; 512];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        clamp.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be 0.5 (unchanged)
        for sample in &output {
            assert_eq!(*sample, 0.5);
        }
    }

    #[test]
    fn test_clamp_min_equals_max_returns_that_value() {
        // Test: Clamp to [0.7, 0.7] always returns 0.7
        let mut clamp = ClampNode::new(0, 1, 2);

        // Try various input values
        let input = vec![-10.0, -1.0, 0.0, 0.7, 5.0, 100.0];
        let min = vec![0.7; 6];
        let max = vec![0.7; 6];
        let inputs = vec![input.as_slice(), min.as_slice(), max.as_slice()];

        let mut output = vec![0.0; 6];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 6, 2.0, 44100.0);

        clamp.process_block(&inputs, &mut output, 44100.0, &context);

        // All outputs should be 0.7 regardless of input
        for sample in &output {
            assert_eq!(*sample, 0.7);
        }
    }

    #[test]
    fn test_clamp_varying_values() {
        // Test: Clamp with varying values per sample
        let mut clamp = ClampNode::new(0, 1, 2);

        let input = vec![-5.0, -0.5, 0.0, 0.5, 2.0, 10.0];
        let min = vec![-1.0, -1.0, -1.0, -1.0, -1.0, -1.0];
        let max = vec![1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        let inputs = vec![input.as_slice(), min.as_slice(), max.as_slice()];

        let mut output = vec![0.0; 6];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 6, 2.0, 44100.0);

        clamp.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], -1.0); // -5.0 clamped to -1.0
        assert_eq!(output[1], -0.5); // -0.5 unchanged
        assert_eq!(output[2], 0.0); // 0.0 unchanged
        assert_eq!(output[3], 0.5); // 0.5 unchanged
        assert_eq!(output[4], 1.0); // 2.0 clamped to 1.0
        assert_eq!(output[5], 1.0); // 10.0 clamped to 1.0
    }

    #[test]
    fn test_clamp_dependencies() {
        let clamp = ClampNode::new(5, 10, 15);
        let deps = clamp.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 5);
        assert_eq!(deps[1], 10);
        assert_eq!(deps[2], 15);
    }
}
