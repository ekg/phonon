/// Exponential node - computes e^x with overflow protection
///
/// This node computes the exponential function (e^x) of the input signal.
/// Output[i] = e^(Input[i]) for all samples.
///
/// The clamping protection prevents f32 overflow (e^88 ≈ 3e38, approaching f32::MAX).
use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Exponential node: out = e^input
///
/// # Example
/// ```ignore
/// // Exponential of a constant
/// let const_node = ConstantNode::new(1.0);   // NodeId 0
/// let exp = ExpNode::new(0);                  // NodeId 1
/// // Output will be e ≈ 2.718
/// ```
pub struct ExpNode {
    input: NodeId,
}

impl ExpNode {
    /// Exp - Computes e^x with overflow protection
    ///
    /// Applies exponential function to input signal, useful for
    /// waveshaping and exponential modulation.
    ///
    /// # Parameters
    /// - `input`: NodeId providing input signal
    ///
    /// # Example
    /// ```phonon
    /// ~lfo: sine 0.25
    /// ~exp: ~lfo # exp
    /// ```
    pub fn new(input: NodeId) -> Self {
        Self { input }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }
}

impl AudioNode for ExpNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(!inputs.is_empty(), "ExpNode requires 1 input, got 0");

        let buf = inputs[0];

        debug_assert_eq!(buf.len(), output.len(), "Input length mismatch");

        // Apply exp(clamp(x)) to each sample to prevent overflow
        // e^87 ≈ 6e37 (safe), e^88 ≈ 3e38 (near f32::MAX ≈ 3.4e38)
        for i in 0..output.len() {
            let x_clamped = buf[i].clamp(-87.0, 87.0);
            output[i] = x_clamped.exp();
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input]
    }

    fn name(&self) -> &str {
        "ExpNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_exp_of_zero() {
        let mut exp_node = ExpNode::new(0);

        let input = vec![0.0, 0.0, 0.0, 0.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![99.9; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        exp_node.process_block(&inputs, &mut output, 44100.0, &context);

        // e^0 = 1.0
        for sample in &output {
            assert_eq!(*sample, 1.0);
        }
    }

    #[test]
    fn test_exp_of_one() {
        let mut exp_node = ExpNode::new(0);

        let input = vec![1.0, 1.0, 1.0, 1.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        exp_node.process_block(&inputs, &mut output, 44100.0, &context);

        // e^1 ≈ 2.718281828
        for sample in &output {
            assert!((*sample - std::f32::consts::E).abs() < 0.00001);
        }
    }

    #[test]
    fn test_exp_of_negative() {
        let mut exp_node = ExpNode::new(0);

        let input = vec![-1.0, -2.0, -3.0, -10.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        exp_node.process_block(&inputs, &mut output, 44100.0, &context);

        // e^-1 ≈ 0.36788
        assert!((output[0] - 0.36788).abs() < 0.001);

        // e^-2 ≈ 0.13534
        assert!((output[1] - 0.13534).abs() < 0.001);

        // e^-3 ≈ 0.04979
        assert!((output[2] - 0.04979).abs() < 0.001);

        // e^-10 ≈ 0.0000454
        assert!((output[3] - 0.0000454).abs() < 0.00001);

        // All should be positive
        for sample in &output {
            assert!(*sample > 0.0);
        }
    }

    #[test]
    fn test_exp_large_positive() {
        let mut exp_node = ExpNode::new(0);

        // Test clamping prevents overflow
        let input = vec![87.0, 88.0, 100.0, 1000.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        exp_node.process_block(&inputs, &mut output, 44100.0, &context);

        // e^87 should compute normally (≈ 6e37)
        assert!(output[0] > 1e30);
        assert!(output[0].is_finite());

        // e^88 and higher get clamped to 87, so should all equal e^87
        assert_eq!(output[1], output[0]);
        assert_eq!(output[2], output[0]);
        assert_eq!(output[3], output[0]);

        // Ensure no infinity or NaN
        for sample in &output {
            assert!(
                sample.is_finite(),
                "exp should not produce infinity or NaN with clamping"
            );
        }
    }

    #[test]
    fn test_exp_large_negative() {
        let mut exp_node = ExpNode::new(0);

        // Test large negative values approach zero
        let input = vec![-10.0, -50.0, -87.0, -100.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![999.9; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        exp_node.process_block(&inputs, &mut output, 44100.0, &context);

        // e^-10 ≈ 0.000045
        assert!((output[0] - 0.0000454).abs() < 0.00001);

        // e^-50 is extremely small but positive
        assert!(output[1] > 0.0);
        assert!(output[1] < 1e-20);

        // e^-87 and e^-100 (clamped to -87) should be equal and very small
        assert_eq!(output[2], output[3]);
        assert!(output[2] > 0.0);
        assert!(output[2] < 1e-35);

        // All should be positive and finite
        for sample in &output {
            assert!(*sample > 0.0);
            assert!(sample.is_finite());
        }
    }

    #[test]
    fn test_exp_dependencies() {
        let exp_node = ExpNode::new(7);
        let deps = exp_node.input_nodes();

        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], 7);
    }

    #[test]
    fn test_exp_with_constant() {
        let mut const_node = ConstantNode::new(2.0);
        let mut exp_node = ExpNode::new(0);

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        // Process constant first
        let mut buf = vec![0.0; 512];
        const_node.process_block(&[], &mut buf, 44100.0, &context);

        // Now take exponential
        let inputs = vec![buf.as_slice()];
        let mut output = vec![0.0; 512];

        exp_node.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be e^2 ≈ 7.389
        for sample in &output {
            assert!((*sample - 7.389056).abs() < 0.001);
        }
    }

    #[test]
    fn test_exp_various_values() {
        let mut exp_node = ExpNode::new(0);

        let input = vec![0.0, 1.0, 2.0, -1.0, 0.5, -0.5];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 6];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 6, 2.0, 44100.0);

        exp_node.process_block(&inputs, &mut output, 44100.0, &context);

        // e^0 = 1.0
        assert!((output[0] - 1.0).abs() < 0.00001);

        // e^1 ≈ 2.718
        assert!((output[1] - std::f32::consts::E).abs() < 0.001);

        // e^2 ≈ 7.389
        assert!((output[2] - 7.389056).abs() < 0.001);

        // e^-1 ≈ 0.368
        assert!((output[3] - 0.36788).abs() < 0.001);

        // e^0.5 ≈ 1.649
        assert!((output[4] - 1.64872).abs() < 0.001);

        // e^-0.5 ≈ 0.606
        assert!((output[5] - 0.60653).abs() < 0.001);
    }

    #[test]
    fn test_exp_mixed_values() {
        let mut exp_node = ExpNode::new(0);

        // Mix of small, medium, large, positive, negative
        let input = vec![-10.0, -1.0, 0.0, 1.0, 5.0, 10.0, 50.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 7];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 7, 2.0, 44100.0);

        exp_node.process_block(&inputs, &mut output, 44100.0, &context);

        // All should be positive
        for sample in &output {
            assert!(*sample > 0.0, "exp should always produce positive values");
        }

        // Should be monotonically increasing (since input is sorted)
        for i in 1..output.len() {
            assert!(
                output[i] > output[i - 1],
                "exp should be monotonically increasing"
            );
        }

        // Ensure no NaN or infinity
        for sample in &output {
            assert!(sample.is_finite());
        }
    }

    #[test]
    fn test_exp_useful_for_envelopes() {
        let mut exp_node = ExpNode::new(0);

        // Exponential envelopes use exp(-t) for decay
        // Simulating time values from 0 to 5 seconds
        let time_values = vec![0.0, -1.0, -2.0, -3.0, -4.0, -5.0];
        let inputs = vec![time_values.as_slice()];

        let mut output = vec![0.0; 6];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 6, 2.0, 44100.0);

        exp_node.process_block(&inputs, &mut output, 44100.0, &context);

        // Should decay from 1.0 toward 0.0
        assert!((output[0] - 1.0).abs() < 0.001); // e^0 = 1.0
        assert!(output[1] < output[0]); // e^-1 < e^0
        assert!(output[2] < output[1]); // e^-2 < e^-1
        assert!(output[3] < output[2]); // Monotonic decay
        assert!(output[4] < output[3]);
        assert!(output[5] < output[4]);

        // Last value should be very small but not zero
        assert!(output[5] > 0.0);
        assert!(output[5] < 0.01); // e^-5 ≈ 0.0067
    }
}
