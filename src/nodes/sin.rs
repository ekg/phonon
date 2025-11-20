/// Sine function node - applies sin(x) to input signal
///
/// This node applies the mathematical sine function to each sample.
/// Output[i] = sin(Input[i]) for all samples.
///
/// NOTE: This is NOT an oscillator. It's a waveshaper that applies
/// the sine function to transform the input signal. Useful for:
/// - Waveshaping/nonlinear distortion
/// - Frequency modulation (FM synthesis)
/// - Creating harmonic content from simple waveforms

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Sine function node: out = sin(input)
///
/// # Example
/// ```ignore
/// // Use as waveshaper on a ramp signal
/// let ramp = RampNode::new(0);      // NodeId 0 (generates -π to π)
/// let sine = SinNode::new(0);       // NodeId 1
/// // Output will be sine wave shaped from ramp
/// ```
pub struct SinNode {
    input: NodeId,
}

impl SinNode {
    /// Create a new sine function node
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

impl AudioNode for SinNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            !inputs.is_empty(),
            "SinNode requires 1 input, got 0"
        );

        let buf = inputs[0];

        debug_assert_eq!(
            buf.len(),
            output.len(),
            "Input length mismatch"
        );

        // Apply sine function to each sample
        for i in 0..output.len() {
            output[i] = buf[i].sin();
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input]
    }

    fn name(&self) -> &str {
        "SinNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;
    use std::f32::consts::PI;

    #[test]
    fn test_sin_of_zero() {
        let mut sin_node = SinNode::new(0);

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

        sin_node.process_block(&inputs, &mut output, 44100.0, &context);

        for sample in &output {
            assert!(
                sample.abs() < 0.0001,
                "sin(0) should be 0.0, got {}",
                sample
            );
        }
    }

    #[test]
    fn test_sin_of_pi_over_2() {
        let mut sin_node = SinNode::new(0);

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

        sin_node.process_block(&inputs, &mut output, 44100.0, &context);

        for sample in &output {
            assert!(
                (sample - 1.0).abs() < 0.0001,
                "sin(π/2) should be 1.0, got {}",
                sample
            );
        }
    }

    #[test]
    fn test_sin_of_pi() {
        let mut sin_node = SinNode::new(0);

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

        sin_node.process_block(&inputs, &mut output, 44100.0, &context);

        for sample in &output {
            assert!(
                sample.abs() < 0.0001,
                "sin(π) should be ~0.0, got {}",
                sample
            );
        }
    }

    #[test]
    fn test_sin_of_negative() {
        let mut sin_node = SinNode::new(0);

        // sin(-x) = -sin(x)
        let test_values = vec![PI / 6.0, PI / 4.0, PI / 3.0, PI / 2.0];
        let neg_input: Vec<f32> = test_values.iter().map(|x| -x).collect();
        let pos_input = test_values;

        // Process negative inputs
        let neg_inputs = vec![neg_input.as_slice()];
        let mut neg_output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        sin_node.process_block(&neg_inputs, &mut neg_output, 44100.0, &context);

        // Process positive inputs
        let pos_inputs = vec![pos_input.as_slice()];
        let mut pos_output = vec![0.0; 4];

        sin_node.process_block(&pos_inputs, &mut pos_output, 44100.0, &context);

        // Verify sin(-x) = -sin(x)
        for i in 0..4 {
            assert!(
                (neg_output[i] + pos_output[i]).abs() < 0.0001,
                "sin(-x) should equal -sin(x), got {} and {}",
                neg_output[i],
                pos_output[i]
            );
        }
    }

    #[test]
    fn test_sin_range() {
        let mut sin_node = SinNode::new(0);

        // Test various inputs from -2π to 2π
        let mut input = Vec::new();
        for i in 0..100 {
            let x = -2.0 * PI + (i as f32 / 99.0) * 4.0 * PI;
            input.push(x);
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

        sin_node.process_block(&inputs, &mut output, 44100.0, &context);

        // All output values should be in [-1, 1]
        for sample in &output {
            assert!(
                *sample >= -1.0 && *sample <= 1.0,
                "sin(x) should be in [-1, 1], got {}",
                sample
            );
        }
    }

    #[test]
    fn test_sin_dependencies() {
        let sin_node = SinNode::new(7);
        let deps = sin_node.input_nodes();

        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], 7);
    }

    #[test]
    fn test_sin_waveshaping() {
        use std::f32::consts::PI;

        let mut sin_node = SinNode::new(0);

        // Create a linear ramp from -π to π (16 samples)
        let mut input = Vec::new();
        for i in 0..16 {
            let x = -PI + (i as f32 / 15.0) * 2.0 * PI;
            input.push(x);
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

        sin_node.process_block(&inputs, &mut output, 44100.0, &context);

        // Verify that output = sin(input) for each sample
        for i in 0..16 {
            let x = -PI + (i as f32 / 15.0) * 2.0 * PI;
            let expected = x.sin();
            assert!(
                (output[i] - expected).abs() < 0.0001,
                "sin({}) should be {}, got {}",
                x,
                expected,
                output[i]
            );
        }

        // Also verify output is in valid range
        for sample in &output {
            assert!(
                *sample >= -1.0 && *sample <= 1.0,
                "Output should be in [-1, 1], got {}",
                sample
            );
        }
    }

    #[test]
    fn test_sin_with_constant() {
        let mut const_node = ConstantNode::new(PI / 6.0); // 30 degrees
        let mut sin_node = SinNode::new(0);

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

        // Now apply sine function
        let inputs = vec![buf.as_slice()];
        let mut output = vec![0.0; 512];

        sin_node.process_block(&inputs, &mut output, 44100.0, &context);

        // sin(π/6) = 0.5
        for sample in &output {
            assert!(
                (sample - 0.5).abs() < 0.0001,
                "sin(π/6) should be 0.5, got {}",
                sample
            );
        }
    }
}
