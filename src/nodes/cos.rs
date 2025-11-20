/// Cosine function node - applies cosine function to input signal
///
/// This node applies the mathematical cosine function to each sample.
/// Output[i] = cos(Input[i]) for all samples.
///
/// Useful for:
/// - Waveshaping and distortion
/// - Modulation synthesis
/// - Phase manipulation
/// - Creating circular/cyclical modulation patterns

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Cosine function node: out = cos(input)
///
/// # Example
/// ```ignore
/// // Create a cosine-shaped LFO
/// let lfo_freq = ConstantNode::new(2.0);        // NodeId 0
/// let lfo_osc = OscillatorNode::new(0, Waveform::Sine);  // NodeId 1
/// let cos_shape = CosNode::new(1);              // NodeId 2
/// // Output will be cos(lfo), creating a different modulation shape
/// ```
pub struct CosNode {
    input: NodeId,
}

impl CosNode {
    /// Create a new cosine function node
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

impl AudioNode for CosNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            !inputs.is_empty(),
            "CosNode requires 1 input, got 0"
        );

        let buf = inputs[0];

        debug_assert_eq!(
            buf.len(),
            output.len(),
            "Input length mismatch"
        );

        // Apply cosine function to each sample
        for i in 0..output.len() {
            output[i] = buf[i].cos();
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input]
    }

    fn name(&self) -> &str {
        "CosNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;
    use std::f32::consts::PI;

    #[test]
    fn test_cos_of_zero() {
        let mut cos_node = CosNode::new(0);

        let input = vec![0.0, 0.0, 0.0, 0.0, 0.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 5];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            5,
            2.0,
            44100.0,
        );

        cos_node.process_block(&inputs, &mut output, 44100.0, &context);

        // cos(0) = 1.0
        for sample in &output {
            assert_eq!(*sample, 1.0);
        }
    }

    #[test]
    fn test_cos_of_pi_over_2() {
        let mut cos_node = CosNode::new(0);

        let input = vec![PI / 2.0, PI / 2.0, PI / 2.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 3];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            3,
            2.0,
            44100.0,
        );

        cos_node.process_block(&inputs, &mut output, 44100.0, &context);

        // cos(π/2) ≈ 0.0
        for sample in &output {
            assert!(sample.abs() < 0.0001, "cos(π/2) should be close to 0, got {}", sample);
        }
    }

    #[test]
    fn test_cos_of_pi() {
        let mut cos_node = CosNode::new(0);

        let input = vec![PI, PI, PI, PI];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        cos_node.process_block(&inputs, &mut output, 44100.0, &context);

        // cos(π) = -1.0
        for sample in &output {
            assert!((*sample - (-1.0)).abs() < 0.0001, "cos(π) should be -1.0, got {}", sample);
        }
    }

    #[test]
    fn test_cos_of_negative() {
        let mut cos_node = CosNode::new(0);

        // Test that cos(-x) = cos(x) (even function)
        let positive_input = vec![0.5, 1.0, PI / 4.0, PI / 3.0];
        let negative_input = vec![-0.5, -1.0, -PI / 4.0, -PI / 3.0];

        let positive_inputs = vec![positive_input.as_slice()];
        let negative_inputs = vec![negative_input.as_slice()];

        let mut positive_output = vec![0.0; 4];
        let mut negative_output = vec![0.0; 4];

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        cos_node.process_block(&positive_inputs, &mut positive_output, 44100.0, &context);
        cos_node.process_block(&negative_inputs, &mut negative_output, 44100.0, &context);

        // cos(-x) should equal cos(x)
        for i in 0..4 {
            assert!(
                (positive_output[i] - negative_output[i]).abs() < 0.0001,
                "cos(-x) should equal cos(x), got {} vs {}",
                positive_output[i],
                negative_output[i]
            );
        }
    }

    #[test]
    fn test_cos_range() {
        let mut cos_node = CosNode::new(0);

        // Test various inputs across multiple periods
        let mut input = Vec::new();
        for i in 0..100 {
            input.push((i as f32 / 10.0) * PI); // 0 to 10π
        }
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 100];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            100,
            2.0,
            44100.0,
        );

        cos_node.process_block(&inputs, &mut output, 44100.0, &context);

        // All cosine outputs must be in range [-1, 1]
        for sample in &output {
            assert!(
                *sample >= -1.0 && *sample <= 1.0,
                "Cosine output {} should be in range [-1, 1]",
                sample
            );
        }
    }

    #[test]
    fn test_cos_dependencies() {
        let cos_node = CosNode::new(7);
        let deps = cos_node.input_nodes();

        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], 7);
    }

    #[test]
    fn test_cos_with_constant() {
        let mut const_node = ConstantNode::new(PI);
        let mut cos_node = CosNode::new(0);

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

        // Now take cosine
        let inputs = vec![buf.as_slice()];
        let mut output = vec![0.0; 512];

        cos_node.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be -1.0 (cos(π))
        for sample in &output {
            assert!(
                (*sample - (-1.0)).abs() < 0.0001,
                "cos(π) should be -1.0, got {}",
                sample
            );
        }
    }

    #[test]
    fn test_cos_waveshaping() {
        let mut cos_node = CosNode::new(0);

        // Create a ramp from -2π to 2π
        let mut input = Vec::new();
        for i in 0..32 {
            let phase = ((i as f32 / 32.0) - 0.5) * 4.0 * PI; // -2π to 2π
            input.push(phase);
        }
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 32];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            32,
            2.0,
            44100.0,
        );

        cos_node.process_block(&inputs, &mut output, 44100.0, &context);

        // Verify some key points
        // At i=0: phase ≈ -2π, cos(-2π) = 1
        assert!((output[0] - 1.0).abs() < 0.1);

        // At i=8: phase ≈ -π, cos(-π) = -1
        assert!((output[8] - (-1.0)).abs() < 0.1);

        // At i=16: phase ≈ 0, cos(0) = 1
        assert!((output[16] - 1.0).abs() < 0.1);

        // At i=24: phase ≈ π, cos(π) = -1
        assert!((output[24] - (-1.0)).abs() < 0.1);

        // At i=31: phase ≈ 2π, cos(2π) = 1
        assert!((output[31] - 1.0).abs() < 0.1);
    }
}
