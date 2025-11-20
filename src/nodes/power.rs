/// Power node - raises signal to a power
///
/// This node computes output[i] = input[i].powf(exponent[i])
/// with protection against NaN for negative bases with fractional exponents.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Power node: out = input ^ exponent
///
/// # NaN Protection
/// When input is negative and exponent is fractional, uses abs(input) to avoid NaN.
/// For example, (-4)^0.5 is computed as 4^0.5 = 2.0
///
/// # Example
/// ```ignore
/// // Square a signal (x^2)
/// let signal = OscillatorNode::new(0, Waveform::Sine);  // NodeId 1
/// let two = ConstantNode::new(2.0);                     // NodeId 2
/// let squared = PowerNode::new(1, 2);                   // NodeId 3
///
/// // Square root (x^0.5)
/// let half = ConstantNode::new(0.5);                    // NodeId 4
/// let sqrt = PowerNode::new(1, 4);                      // NodeId 5 (uses abs)
/// ```
pub struct PowerNode {
    input: NodeId,
    exponent_input: NodeId,
}

impl PowerNode {
    /// Create a new power node
    ///
    /// # Arguments
    /// * `input` - NodeId of base signal
    /// * `exponent_input` - NodeId of exponent signal
    pub fn new(input: NodeId, exponent_input: NodeId) -> Self {
        Self {
            input,
            exponent_input,
        }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }

    /// Get the exponent input node ID
    pub fn exponent_input(&self) -> NodeId {
        self.exponent_input
    }
}

impl AudioNode for PowerNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "PowerNode requires 2 inputs (input, exponent), got {}",
            inputs.len()
        );

        let input_buf = inputs[0];
        let exponent_buf = inputs[1];

        debug_assert_eq!(
            input_buf.len(),
            output.len(),
            "Input length mismatch"
        );
        debug_assert_eq!(
            exponent_buf.len(),
            output.len(),
            "Exponent length mismatch"
        );

        // Compute power with NaN protection
        for i in 0..output.len() {
            let base = input_buf[i];
            let exp = exponent_buf[i];

            // Protect against NaN: if base is negative and exponent is fractional,
            // use abs(base) to avoid complex numbers
            if base < 0.0 && exp.fract() != 0.0 {
                output[i] = base.abs().powf(exp);
            } else {
                output[i] = base.powf(exp);
            }
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.exponent_input]
    }

    fn name(&self) -> &str {
        "PowerNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_power_node_square() {
        let mut power_node = PowerNode::new(0, 1);

        // Input values to square
        let input = vec![0.0, 1.0, 2.0, 3.0, -4.0];
        // Exponent is 2.0 (square)
        let exponent = vec![2.0; 5];
        let inputs = vec![input.as_slice(), exponent.as_slice()];

        let mut output = vec![0.0; 5];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            5,
            2.0,
            44100.0,
        );

        power_node.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0);   // 0^2 = 0
        assert_eq!(output[1], 1.0);   // 1^2 = 1
        assert_eq!(output[2], 4.0);   // 2^2 = 4
        assert_eq!(output[3], 9.0);   // 3^2 = 9
        assert_eq!(output[4], 16.0);  // (-4)^2 = 16
    }

    #[test]
    fn test_power_node_square_root() {
        let mut power_node = PowerNode::new(0, 1);

        // Input values (note: negative values will use abs)
        let input = vec![0.0, 1.0, 4.0, 9.0, -16.0];
        // Exponent is 0.5 (square root)
        let exponent = vec![0.5; 5];
        let inputs = vec![input.as_slice(), exponent.as_slice()];

        let mut output = vec![0.0; 5];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            5,
            2.0,
            44100.0,
        );

        power_node.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0);  // 0^0.5 = 0
        assert_eq!(output[1], 1.0);  // 1^0.5 = 1
        assert_eq!(output[2], 2.0);  // 4^0.5 = 2
        assert_eq!(output[3], 3.0);  // 9^0.5 = 3
        assert_eq!(output[4], 4.0);  // |-16|^0.5 = 16^0.5 = 4 (uses abs to avoid NaN)
    }

    #[test]
    fn test_power_node_cube() {
        let mut power_node = PowerNode::new(0, 1);

        // Input values to cube
        let input = vec![0.0, 1.0, 2.0, 3.0, -2.0];
        // Exponent is 3.0 (cube)
        let exponent = vec![3.0; 5];
        let inputs = vec![input.as_slice(), exponent.as_slice()];

        let mut output = vec![0.0; 5];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            5,
            2.0,
            44100.0,
        );

        power_node.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0);    // 0^3 = 0
        assert_eq!(output[1], 1.0);    // 1^3 = 1
        assert_eq!(output[2], 8.0);    // 2^3 = 8
        assert_eq!(output[3], 27.0);   // 3^3 = 27
        assert_eq!(output[4], -8.0);   // (-2)^3 = -8
    }

    #[test]
    fn test_power_node_power_of_zero() {
        let mut power_node = PowerNode::new(0, 1);

        // Various exponents with base 0
        let input = vec![0.0; 5];
        let exponent = vec![0.0, 1.0, 2.0, 0.5, 3.0];
        let inputs = vec![input.as_slice(), exponent.as_slice()];

        let mut output = vec![0.0; 5];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            5,
            2.0,
            44100.0,
        );

        power_node.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0);  // 0^0 = 1 (by Rust's powf convention)
        assert_eq!(output[1], 0.0);  // 0^1 = 0
        assert_eq!(output[2], 0.0);  // 0^2 = 0
        assert_eq!(output[3], 0.0);  // 0^0.5 = 0
        assert_eq!(output[4], 0.0);  // 0^3 = 0
    }

    #[test]
    fn test_power_node_dependencies() {
        let power_node = PowerNode::new(3, 7);
        let deps = power_node.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 3);  // input
        assert_eq!(deps[1], 7);  // exponent
    }

    #[test]
    fn test_power_node_varying_exponents() {
        let mut power_node = PowerNode::new(0, 1);

        // Same base, different exponents
        let input = vec![2.0; 5];
        let exponent = vec![0.0, 1.0, 2.0, 3.0, 0.5];
        let inputs = vec![input.as_slice(), exponent.as_slice()];

        let mut output = vec![0.0; 5];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            5,
            2.0,
            44100.0,
        );

        power_node.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0);  // 2^0 = 1
        assert_eq!(output[1], 2.0);  // 2^1 = 2
        assert_eq!(output[2], 4.0);  // 2^2 = 4
        assert_eq!(output[3], 8.0);  // 2^3 = 8
        assert!((output[4] - 1.414213562).abs() < 0.0001);  // 2^0.5 ≈ 1.414
    }

    #[test]
    fn test_power_node_negative_base_fractional_exponent() {
        let mut power_node = PowerNode::new(0, 1);

        // Negative bases with fractional exponents (should use abs)
        let input = vec![-4.0, -9.0, -16.0];
        let exponent = vec![0.5, 0.5, 0.5];
        let inputs = vec![input.as_slice(), exponent.as_slice()];

        let mut output = vec![0.0; 3];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            3,
            2.0,
            44100.0,
        );

        power_node.process_block(&inputs, &mut output, 44100.0, &context);

        // Should compute as abs(base)^exp to avoid NaN
        assert_eq!(output[0], 2.0);  // |-4|^0.5 = 2
        assert_eq!(output[1], 3.0);  // |-9|^0.5 = 3
        assert_eq!(output[2], 4.0);  // |-16|^0.5 = 4

        // Verify no NaN values
        for sample in &output {
            assert!(!sample.is_nan(), "Output should not contain NaN");
        }
    }

    #[test]
    fn test_power_node_with_constants() {
        let mut const_base = ConstantNode::new(3.0);
        let mut const_exp = ConstantNode::new(2.0);
        let mut power_node = PowerNode::new(0, 1);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        // Process constants first
        let mut base_buf = vec![0.0; 512];
        let mut exp_buf = vec![0.0; 512];
        const_base.process_block(&[], &mut base_buf, 44100.0, &context);
        const_exp.process_block(&[], &mut exp_buf, 44100.0, &context);

        // Now compute power
        let inputs = vec![base_buf.as_slice(), exp_buf.as_slice()];
        let mut output = vec![0.0; 512];

        power_node.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be 9.0 (3^2)
        for sample in &output {
            assert_eq!(*sample, 9.0);
        }
    }
}
