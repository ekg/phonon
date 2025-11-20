/// Sign node - extracts sign of input signal
///
/// This node returns the sign of each input sample:
/// - Positive values → 1.0
/// - Negative values → -1.0
/// - Zero → 0.0
///
/// Output[i] = sign(Input[i]) for all samples.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Sign node: out = sign(input)
///
/// Returns:
/// - 1.0 for positive values
/// - -1.0 for negative values
/// - 0.0 for zero
///
/// # Example
/// ```ignore
/// // Extract sign of a sine wave
/// let freq = ConstantNode::new(440.0);      // NodeId 0
/// let sine = OscillatorNode::new(0, Waveform::Sine);  // NodeId 1
/// let sign = SignNode::new(1);              // NodeId 2
/// // Output will be a square wave (+1.0/-1.0)
/// ```
pub struct SignNode {
    input: NodeId,
}

impl SignNode {
    /// Create a new sign node
    ///
    /// # Arguments
    /// * `input` - NodeId of input signal
    pub fn new(input: NodeId) -> Self {
        Self { input }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }
}

impl AudioNode for SignNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            !inputs.is_empty(),
            "SignNode requires 1 input, got 0"
        );

        let buf = inputs[0];

        debug_assert_eq!(
            buf.len(),
            output.len(),
            "Input length mismatch"
        );

        // Apply sign function to each sample
        for i in 0..output.len() {
            output[i] = if buf[i] > 0.0 {
                1.0
            } else if buf[i] < 0.0 {
                -1.0
            } else {
                0.0
            };
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input]
    }

    fn name(&self) -> &str {
        "SignNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_sign_node_positive_becomes_one() {
        let mut sign_node = SignNode::new(0);

        let input = vec![1.0, 2.0, 3.0, 4.5, 100.0, 0.001];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 6];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            6,
            2.0,
            44100.0,
        );

        sign_node.process_block(&inputs, &mut output, 44100.0, &context);

        // All positive values should be 1.0
        for sample in &output {
            assert_eq!(*sample, 1.0);
        }
    }

    #[test]
    fn test_sign_node_negative_becomes_negative_one() {
        let mut sign_node = SignNode::new(0);

        let input = vec![-1.0, -2.0, -3.0, -4.5, -100.0, -0.001];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 6];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            6,
            2.0,
            44100.0,
        );

        sign_node.process_block(&inputs, &mut output, 44100.0, &context);

        // All negative values should be -1.0
        for sample in &output {
            assert_eq!(*sample, -1.0);
        }
    }

    #[test]
    fn test_sign_node_zero_stays_zero() {
        let mut sign_node = SignNode::new(0);

        let input = vec![0.0, 0.0, 0.0, -0.0, 0.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 5];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            5,
            2.0,
            44100.0,
        );

        sign_node.process_block(&inputs, &mut output, 44100.0, &context);

        // All zero values should remain 0.0
        for sample in &output {
            assert_eq!(*sample, 0.0);
        }
    }

    #[test]
    fn test_sign_node_mixed_values() {
        let mut sign_node = SignNode::new(0);

        let input = vec![1.0, -2.0, 3.0, -4.0, 0.0, 5.5, -0.1];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 7];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            7,
            2.0,
            44100.0,
        );

        sign_node.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0);   // 1.0 → 1.0
        assert_eq!(output[1], -1.0);  // -2.0 → -1.0
        assert_eq!(output[2], 1.0);   // 3.0 → 1.0
        assert_eq!(output[3], -1.0);  // -4.0 → -1.0
        assert_eq!(output[4], 0.0);   // 0.0 → 0.0
        assert_eq!(output[5], 1.0);   // 5.5 → 1.0
        assert_eq!(output[6], -1.0);  // -0.1 → -1.0
    }

    #[test]
    fn test_sign_node_sine_wave_to_square() {
        use std::f32::consts::PI;

        let mut sign_node = SignNode::new(0);

        // Create one cycle of a sine wave (16 samples)
        let mut input = Vec::new();
        for i in 0..16 {
            let phase = (i as f32 / 16.0) * 2.0 * PI;
            input.push(phase.sin());
        }
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 16];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            16,
            2.0,
            44100.0,
        );

        sign_node.process_block(&inputs, &mut output, 44100.0, &context);

        // First half of sine is positive → 1.0
        for i in 1..8 {
            assert_eq!(
                output[i], 1.0,
                "Sample {} should be 1.0 (positive part of sine)",
                i
            );
        }

        // Second half is negative → -1.0
        for i in 9..16 {
            assert_eq!(
                output[i], -1.0,
                "Sample {} should be -1.0 (negative part of sine)",
                i
            );
        }

        // Zero crossings might be 0.0 or ±1.0 depending on exact phase
        // We'll just check they're valid sign values
        assert!(
            output[0].abs() <= 1.0 && (output[0] == 0.0 || output[0] == 1.0 || output[0] == -1.0),
            "Sample 0 should be a valid sign value"
        );
        assert!(
            output[8].abs() <= 1.0 && (output[8] == 0.0 || output[8] == 1.0 || output[8] == -1.0),
            "Sample 8 should be a valid sign value"
        );
    }

    #[test]
    fn test_sign_node_dependencies() {
        let sign_node = SignNode::new(7);
        let deps = sign_node.input_nodes();

        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], 7);
    }

    #[test]
    fn test_sign_node_with_constant() {
        let mut const_node = ConstantNode::new(-5.0);
        let mut sign_node = SignNode::new(0);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        // Process constant first
        let mut buf = vec![0.0; 512];
        const_node.process_block(&[], &mut buf, 44100.0, &context);

        // Now take sign
        let inputs = vec![buf.as_slice()];
        let mut output = vec![0.0; 512];

        sign_node.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be -1.0 (sign of -5.0)
        for sample in &output {
            assert_eq!(*sample, -1.0);
        }
    }
}
