/// Limiter node - hard limiting dynamics processor
///
/// This node provides transparent peak limiting to prevent clipping.
/// Signals above threshold are hard limited to the ceiling value.
use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Limiter node: hard limiting dynamics processor
///
/// The limiting formula is:
/// ```text
/// threshold_linear = 10^(threshold_db / 20)
/// ceiling_linear = 10^(ceiling_db / 20)
///
/// if |input[i]| > threshold_linear:
///     output[i] = sign(input[i]) * ceiling_linear
/// else:
///     output[i] = input[i]
/// ```
///
/// This provides:
/// - Transparent limiting below threshold
/// - Hard ceiling enforcement above threshold
/// - Prevents clipping in final output
/// - Adjustable threshold and ceiling in dB
///
/// # Example
/// ```ignore
/// // Limit signal at -3 dB threshold, -0.1 dB ceiling
/// let input = OscillatorNode::new(0, Waveform::Sine);  // NodeId 1
/// let threshold = ConstantNode::new(-3.0);              // NodeId 2
/// let ceiling = ConstantNode::new(-0.1);                // NodeId 3
/// let limiter = LimiterNode::new(1, 2, 3);              // NodeId 4
/// // Output will be limited to prevent clipping
/// ```
pub struct LimiterNode {
    input: NodeId,
    threshold_input: NodeId, // Threshold in dB (e.g., -3.0)
    ceiling_input: NodeId,   // Output ceiling in dB (e.g., 0.0)
}

impl LimiterNode {
    /// Limiter - Dynamic amplitude ceiling with soft knee
    ///
    /// Compresses audio above threshold, hard-limiting at ceiling. Provides transparent
    /// limiting below threshold and enforces hard ceiling to prevent clipping.
    ///
    /// # Parameters
    /// - `input`: Audio signal to limit
    /// - `threshold_input`: Threshold in dB (default: -3.0)
    /// - `ceiling_input`: Output ceiling in dB (default: 0.0)
    ///
    /// # Example
    /// ```phonon
    /// ~signal: saw 110 + saw 165
    /// ~limited: ~signal # limiter -3 -0.1
    /// out: ~limited * 0.5
    /// ```
    pub fn new(input: NodeId, threshold_input: NodeId, ceiling_input: NodeId) -> Self {
        Self {
            input,
            threshold_input,
            ceiling_input,
        }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }

    /// Get the threshold input node ID
    pub fn threshold_input(&self) -> NodeId {
        self.threshold_input
    }

    /// Get the ceiling input node ID
    pub fn ceiling_input(&self) -> NodeId {
        self.ceiling_input
    }
}

impl AudioNode for LimiterNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 3,
            "LimiterNode requires 3 inputs, got {}",
            inputs.len()
        );

        let input_buf = inputs[0];
        let threshold_buf = inputs[1];
        let ceiling_buf = inputs[2];

        debug_assert_eq!(
            input_buf.len(),
            output.len(),
            "Input buffer length mismatch"
        );
        debug_assert_eq!(
            threshold_buf.len(),
            output.len(),
            "Threshold buffer length mismatch"
        );
        debug_assert_eq!(
            ceiling_buf.len(),
            output.len(),
            "Ceiling buffer length mismatch"
        );

        // Apply hard limiting
        for i in 0..output.len() {
            let sample = input_buf[i];
            let threshold_db = threshold_buf[i];
            let ceiling_db = ceiling_buf[i];

            // Convert dB to linear
            let threshold_linear = 10.0_f32.powf(threshold_db / 20.0);
            let ceiling_linear = 10.0_f32.powf(ceiling_db / 20.0);

            // Apply limiting
            let abs_sample = sample.abs();
            if abs_sample > threshold_linear {
                // Hard limit to ceiling
                output[i] = sample.signum() * ceiling_linear;
            } else {
                output[i] = sample;
            }
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.threshold_input, self.ceiling_input]
    }

    fn name(&self) -> &str {
        "LimiterNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    fn create_context() -> ProcessContext {
        ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0)
    }

    #[test]
    fn test_limiter_below_threshold_unchanged() {
        // Signals below threshold should pass unchanged
        let mut limiter = LimiterNode::new(0, 1, 2);

        // Input: 0.5 (= -6 dB), Threshold: -3 dB, Ceiling: 0 dB
        // 0.5 < 10^(-3/20) ≈ 0.708, so should pass unchanged
        let input = vec![0.5; 512];
        let threshold = vec![-3.0; 512];
        let ceiling = vec![0.0; 512];
        let inputs = vec![input.as_slice(), threshold.as_slice(), ceiling.as_slice()];

        let mut output = vec![0.0; 512];
        let context = create_context();

        limiter.process_block(&inputs, &mut output, 44100.0, &context);

        for sample in &output {
            assert_eq!(*sample, 0.5);
        }
    }

    #[test]
    fn test_limiter_above_threshold_limited() {
        // Signals above threshold should be limited to ceiling
        let mut limiter = LimiterNode::new(0, 1, 2);

        // Input: 1.0 (0 dB), Threshold: -3 dB (≈0.708), Ceiling: -0.1 dB (≈0.989)
        // 1.0 > 0.708, so should limit to 0.989
        let input = vec![1.0; 512];
        let threshold = vec![-3.0; 512];
        let ceiling = vec![-0.1; 512];
        let inputs = vec![input.as_slice(), threshold.as_slice(), ceiling.as_slice()];

        let mut output = vec![0.0; 512];
        let context = create_context();

        limiter.process_block(&inputs, &mut output, 44100.0, &context);

        let expected_ceiling = 10.0_f32.powf(-0.1 / 20.0);
        for sample in &output {
            assert!((*sample - expected_ceiling).abs() < 0.001);
        }
    }

    #[test]
    fn test_limiter_ceiling_enforcement() {
        // Output should never exceed ceiling
        let mut limiter = LimiterNode::new(0, 1, 2);

        // Very hot input (10.0), threshold -6 dB, ceiling -1 dB
        let input = vec![10.0; 512];
        let threshold = vec![-6.0; 512];
        let ceiling = vec![-1.0; 512];
        let inputs = vec![input.as_slice(), threshold.as_slice(), ceiling.as_slice()];

        let mut output = vec![0.0; 512];
        let context = create_context();

        limiter.process_block(&inputs, &mut output, 44100.0, &context);

        let expected_ceiling = 10.0_f32.powf(-1.0 / 20.0);
        for sample in &output {
            assert!(sample.abs() <= expected_ceiling + 0.001);
            assert!((*sample - expected_ceiling).abs() < 0.001);
        }
    }

    #[test]
    fn test_limiter_preserves_sign() {
        // Positive and negative signals should preserve sign
        let mut limiter = LimiterNode::new(0, 1, 2);

        let input = vec![2.0, -2.0, 1.5, -1.5];
        let threshold = vec![-3.0; 4]; // ≈0.708
        let ceiling = vec![0.0; 4]; // 1.0
        let inputs = vec![input.as_slice(), threshold.as_slice(), ceiling.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        limiter.process_block(&inputs, &mut output, 44100.0, &context);

        // All inputs exceed threshold, so limited to ±1.0
        assert!((output[0] - 1.0).abs() < 0.001); // Positive → positive
        assert!((output[1] - (-1.0)).abs() < 0.001); // Negative → negative
        assert!((output[2] - 1.0).abs() < 0.001); // Positive → positive
        assert!((output[3] - (-1.0)).abs() < 0.001); // Negative → negative
    }

    #[test]
    fn test_limiter_zero_db_ceiling() {
        // 0 dB ceiling = 1.0 linear (prevents clipping)
        let mut limiter = LimiterNode::new(0, 1, 2);

        let input = vec![1.5, 2.0, 5.0, 10.0];
        let threshold = vec![-6.0; 4];
        let ceiling = vec![0.0; 4]; // 0 dB = 1.0 linear
        let inputs = vec![input.as_slice(), threshold.as_slice(), ceiling.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        limiter.process_block(&inputs, &mut output, 44100.0, &context);

        // All limited to 1.0
        for sample in &output {
            assert!((*sample - 1.0).abs() < 0.001);
        }
    }

    #[test]
    fn test_limiter_dependencies() {
        let limiter = LimiterNode::new(5, 10, 15);
        let deps = limiter.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 5);
        assert_eq!(deps[1], 10);
        assert_eq!(deps[2], 15);
    }

    #[test]
    fn test_limiter_prevents_clipping() {
        // Verify that limiting to -0.1 dB prevents clipping at 0 dB
        let mut limiter = LimiterNode::new(0, 1, 2);

        // Hot signal that would clip
        let input = vec![1.2, 1.5, 2.0, 3.0];
        let threshold = vec![-3.0; 4];
        let ceiling = vec![-0.1; 4]; // Safety headroom
        let inputs = vec![input.as_slice(), threshold.as_slice(), ceiling.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        limiter.process_block(&inputs, &mut output, 44100.0, &context);

        let max_allowed = 10.0_f32.powf(-0.1 / 20.0); // ≈0.989

        // All outputs should be at or below ceiling
        for sample in &output {
            assert!(sample.abs() <= max_allowed + 0.001);
            // All should be limited to ceiling
            assert!((*sample - max_allowed).abs() < 0.001);
        }

        // Verify all outputs are below 1.0 (no clipping)
        for sample in &output {
            assert!(sample.abs() < 1.0);
        }
    }

    #[test]
    fn test_limiter_mixed_signals() {
        // Test mix of signals above and below threshold
        let mut limiter = LimiterNode::new(0, 1, 2);

        let input = vec![0.3, 0.9, -0.4, -1.2];
        let threshold = vec![-3.0; 4]; // ≈0.708
        let ceiling = vec![0.0; 4]; // 1.0
        let inputs = vec![input.as_slice(), threshold.as_slice(), ceiling.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        limiter.process_block(&inputs, &mut output, 44100.0, &context);

        let threshold_linear = 10.0_f32.powf(-3.0 / 20.0);

        // 0.3 < threshold → unchanged
        assert!((output[0] - 0.3).abs() < 0.001);

        // 0.9 > threshold → limited to 1.0
        assert!((output[1] - 1.0).abs() < 0.001);

        // -0.4 < threshold → unchanged
        assert!((output[2] - (-0.4)).abs() < 0.001);

        // -1.2 > threshold → limited to -1.0
        assert!((output[3] - (-1.0)).abs() < 0.001);
    }
}
