/// Natural logarithm node - computes natural logarithm with safety protections
///
/// This node computes the natural logarithm (ln) of the absolute value of the input signal,
/// with a minimum threshold to prevent log(0) = -infinity.
/// Output[i] = ln(max(|Input[i]|, 1e-10)) for all samples.
///
/// The abs() and max() protections ensure no NaN or -infinity values.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Natural logarithm node: out = ln(max(|input|, 1e-10))
///
/// # Example
/// ```ignore
/// // Natural log of a constant
/// let const_node = ConstantNode::new(2.71828);  // NodeId 0 (e)
/// let log = LogNode::new(0);                     // NodeId 1
/// // Output will be approximately 1.0 (ln(e) = 1)
/// ```
pub struct LogNode {
    input: NodeId,
}

impl LogNode {
    /// Create a new natural logarithm node
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

impl AudioNode for LogNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            !inputs.is_empty(),
            "LogNode requires 1 input, got 0"
        );

        let buf = inputs[0];

        debug_assert_eq!(
            buf.len(),
            output.len(),
            "Input length mismatch"
        );

        // Apply ln(max(abs(x), 1e-10)) to each sample to avoid NaN from negative inputs
        // and -infinity from zero inputs
        for i in 0..output.len() {
            let x_safe = buf[i].abs().max(1e-10);
            output[i] = x_safe.ln();
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input]
    }

    fn name(&self) -> &str {
        "LogNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_log_of_one() {
        let mut log_node = LogNode::new(0);

        // ln(1) = 0
        let input = vec![1.0, 1.0, 1.0, 1.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![99.9; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        log_node.process_block(&inputs, &mut output, 44100.0, &context);

        for sample in &output {
            assert_eq!(*sample, 0.0);
        }
    }

    #[test]
    fn test_log_of_e() {
        let mut log_node = LogNode::new(0);

        // ln(e) = 1.0
        let e = std::f32::consts::E;
        let input = vec![e, e, e, e];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        log_node.process_block(&inputs, &mut output, 44100.0, &context);

        for sample in &output {
            assert!((*sample - 1.0).abs() < 0.0001, "ln(e) should be 1.0, got {}", sample);
        }
    }

    #[test]
    fn test_log_of_large_value() {
        let mut log_node = LogNode::new(0);

        // ln(1000) ≈ 6.907755
        let input = vec![1000.0, 1000.0, 1000.0, 1000.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        log_node.process_block(&inputs, &mut output, 44100.0, &context);

        let expected = 1000.0_f32.ln();
        for sample in &output {
            assert!((*sample - expected).abs() < 0.0001, "ln(1000) should be {}, got {}", expected, sample);
        }
    }

    #[test]
    fn test_log_of_small_value() {
        let mut log_node = LogNode::new(0);

        // ln(0.001) ≈ -6.907755
        let input = vec![0.001, 0.001, 0.001, 0.001];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        log_node.process_block(&inputs, &mut output, 44100.0, &context);

        let expected = 0.001_f32.ln();
        for sample in &output {
            assert!((*sample - expected).abs() < 0.0001, "ln(0.001) should be {}, got {}", expected, sample);
        }
    }

    #[test]
    fn test_log_of_negative_uses_abs() {
        let mut log_node = LogNode::new(0);

        // ln(-e) should use abs() and return 1.0, not NaN
        let e = std::f32::consts::E;
        let input = vec![-e, -10.0, -100.0, -1000.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        log_node.process_block(&inputs, &mut output, 44100.0, &context);

        assert!((output[0] - 1.0).abs() < 0.0001);  // ln(|−e|) = ln(e) = 1
        assert!((output[1] - 10.0_f32.ln()).abs() < 0.0001);  // ln(|−10|) = ln(10)
        assert!((output[2] - 100.0_f32.ln()).abs() < 0.0001);  // ln(|−100|) = ln(100)
        assert!((output[3] - 1000.0_f32.ln()).abs() < 0.0001);  // ln(|−1000|) = ln(1000)

        // Ensure no NaN values
        for sample in &output {
            assert!(!sample.is_nan(), "log should not produce NaN with abs() protection");
        }
    }

    #[test]
    fn test_log_dependencies() {
        let log_node = LogNode::new(7);
        let deps = log_node.input_nodes();

        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], 7);
    }

    #[test]
    fn test_log_with_constant() {
        let mut const_node = ConstantNode::new(std::f32::consts::E * std::f32::consts::E);
        let mut log_node = LogNode::new(0);

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

        // Now take log
        let inputs = vec![buf.as_slice()];
        let mut output = vec![0.0; 512];

        log_node.process_block(&inputs, &mut output, 44100.0, &context);

        // ln(e^2) = 2.0
        for sample in &output {
            assert!((*sample - 2.0).abs() < 0.0001, "ln(e^2) should be 2.0, got {}", sample);
        }
    }

    #[test]
    fn test_log_various_values() {
        let mut log_node = LogNode::new(0);

        let e = std::f32::consts::E;
        let input = vec![1.0, e, e * e, 10.0, 100.0, 1000.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 6];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            6,
            2.0,
            44100.0,
        );

        log_node.process_block(&inputs, &mut output, 44100.0, &context);

        assert!((output[0] - 0.0).abs() < 0.0001);  // ln(1) = 0
        assert!((output[1] - 1.0).abs() < 0.0001);  // ln(e) = 1
        assert!((output[2] - 2.0).abs() < 0.0001);  // ln(e^2) = 2
        assert!((output[3] - 10.0_f32.ln()).abs() < 0.0001);
        assert!((output[4] - 100.0_f32.ln()).abs() < 0.0001);
        assert!((output[5] - 1000.0_f32.ln()).abs() < 0.0001);
    }

    #[test]
    fn test_log_of_zero_protected() {
        let mut log_node = LogNode::new(0);

        // ln(0) should be protected to prevent -infinity
        let input = vec![0.0, 0.0, 0.0, 0.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        log_node.process_block(&inputs, &mut output, 44100.0, &context);

        // Should return ln(1e-10), not -infinity
        let expected = 1e-10_f32.ln();
        for sample in &output {
            assert!(sample.is_finite(), "log(0) should be finite (protected by epsilon)");
            assert!((sample - expected).abs() < 0.1, "log(0) should be ln(1e-10) ≈ {}, got {}", expected, sample);
        }
    }

    #[test]
    fn test_log_mixed_positive_negative_zero() {
        let mut log_node = LogNode::new(0);

        let e = std::f32::consts::E;
        // Mix of positive, negative, and zero - all should use abs() + epsilon protection
        let input = vec![e, -e, 0.0, 10.0, -10.0, 1.0, -1.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 7];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            7,
            2.0,
            44100.0,
        );

        log_node.process_block(&inputs, &mut output, 44100.0, &context);

        assert!((output[0] - 1.0).abs() < 0.0001);  // ln(e) = 1
        assert!((output[1] - 1.0).abs() < 0.0001);  // ln(|−e|) = ln(e) = 1
        assert!(output[2].is_finite());  // ln(0) protected
        assert!((output[3] - 10.0_f32.ln()).abs() < 0.0001);  // ln(10)
        assert!((output[4] - 10.0_f32.ln()).abs() < 0.0001);  // ln(|−10|) = ln(10)
        assert!((output[5] - 0.0).abs() < 0.0001);  // ln(1) = 0
        assert!((output[6] - 0.0).abs() < 0.0001);  // ln(|−1|) = ln(1) = 0

        // Ensure no NaN or infinity values
        for sample in &output {
            assert!(!sample.is_nan(), "log should not produce NaN");
            assert!(sample.is_finite(), "log should not produce infinity");
        }
    }

    #[test]
    fn test_log_fractional_values() {
        let mut log_node = LogNode::new(0);

        let input = vec![0.1, 0.5, 2.0, 10.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        log_node.process_block(&inputs, &mut output, 44100.0, &context);

        assert!((output[0] - 0.1_f32.ln()).abs() < 0.0001);  // ln(0.1) ≈ -2.303
        assert!((output[1] - 0.5_f32.ln()).abs() < 0.0001);  // ln(0.5) ≈ -0.693
        assert!((output[2] - 2.0_f32.ln()).abs() < 0.0001);  // ln(2) ≈ 0.693
        assert!((output[3] - 10.0_f32.ln()).abs() < 0.0001);  // ln(10) ≈ 2.303
    }
}
