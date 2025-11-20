/// Clip node - soft clipping/distortion using tanh
///
/// This node provides smooth, musical distortion by clipping signals
/// above a threshold using a hyperbolic tangent transfer function.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Clip node: soft clipping with configurable threshold
///
/// The clipping formula is:
/// ```text
/// output[i] = (input[i] / threshold).tanh() * threshold
/// ```
///
/// This provides:
/// - Smooth saturation (not hard clipping)
/// - Symmetrical clipping for positive/negative signals
/// - Adjustable threshold (0.0 to 1.0)
///
/// # Example
/// ```ignore
/// // Clip signal at 0.5 threshold
/// let input = ConstantNode::new(1.0);      // NodeId 0
/// let threshold = ConstantNode::new(0.5);  // NodeId 1
/// let clip = ClipNode::new(0, 1);          // NodeId 2
/// // Output will be ~0.38 (tanh(1.0/0.5) * 0.5)
/// ```
pub struct ClipNode {
    input: NodeId,
    threshold_input: NodeId,
}

impl ClipNode {
    /// Create a new clip node
    ///
    /// # Arguments
    /// * `input` - NodeId of signal to clip
    /// * `threshold_input` - NodeId of threshold (typically 0.0 to 1.0)
    pub fn new(input: NodeId, threshold_input: NodeId) -> Self {
        Self {
            input,
            threshold_input,
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
}

impl AudioNode for ClipNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "ClipNode requires 2 inputs, got {}",
            inputs.len()
        );

        let input_buf = inputs[0];
        let threshold_buf = inputs[1];

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

        // Apply soft clipping: (input / threshold).tanh() * threshold
        for i in 0..output.len() {
            let threshold = threshold_buf[i];
            let input = input_buf[i];

            // Avoid division by zero for very small thresholds
            if threshold.abs() < 1e-6 {
                output[i] = 0.0;
            } else {
                output[i] = (input / threshold).tanh() * threshold;
            }
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.threshold_input]
    }

    fn name(&self) -> &str {
        "ClipNode"
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
    fn test_clip_node_small_signals_pass_unchanged() {
        // Small signals (well below threshold) should pass through unchanged
        let mut clip = ClipNode::new(0, 1);

        let input = vec![0.1, 0.2, -0.1, -0.2];
        let threshold = vec![1.0, 1.0, 1.0, 1.0];
        let inputs = vec![input.as_slice(), threshold.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        clip.process_block(&inputs, &mut output, 44100.0, &context);

        // Small values should be nearly unchanged (tanh(x) ≈ x for small x)
        assert!((output[0] - 0.1).abs() < 0.01);
        assert!((output[1] - 0.2).abs() < 0.01);
        assert!((output[2] - (-0.1)).abs() < 0.01);
        assert!((output[3] - (-0.2)).abs() < 0.01);
    }

    #[test]
    fn test_clip_node_large_signals_clipped() {
        // Large signals should be clipped to near threshold
        let mut clip = ClipNode::new(0, 1);

        let input = vec![5.0, -5.0, 10.0, -10.0];
        let threshold = vec![1.0, 1.0, 1.0, 1.0];
        let inputs = vec![input.as_slice(), threshold.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        clip.process_block(&inputs, &mut output, 44100.0, &context);

        // tanh(5) ≈ 0.9999, so 5.0 → ~1.0
        // tanh(10) ≈ 1.0, so 10.0 → ~1.0
        assert!(output[0] > 0.99 && output[0] <= 1.0);
        assert!(output[1] < -0.99 && output[1] >= -1.0);
        assert!(output[2] > 0.99 && output[2] <= 1.0);
        assert!(output[3] < -0.99 && output[3] >= -1.0);
    }

    #[test]
    fn test_clip_node_threshold_0_5_clips_earlier() {
        // Lower threshold should clip earlier
        let mut clip = ClipNode::new(0, 1);

        // Same input signal
        let input = vec![2.0, 2.0];
        let threshold = vec![0.5, 1.0]; // Different thresholds
        let inputs = vec![input.as_slice(), threshold.as_slice()];

        let mut output = vec![0.0; 2];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 2, 2.0, 44100.0);

        clip.process_block(&inputs, &mut output, 44100.0, &context);

        // With threshold 0.5: (2.0/0.5).tanh() * 0.5 = tanh(4) * 0.5 ≈ 0.999 * 0.5 ≈ 0.5
        // With threshold 1.0: (2.0/1.0).tanh() * 1.0 = tanh(2) * 1.0 ≈ 0.964
        assert!(output[0] < 0.51 && output[0] > 0.49); // ~0.5
        assert!(output[1] > 0.96 && output[1] < 0.97); // ~0.964
        assert!(output[0] < output[1]); // Lower threshold = more clipping
    }

    #[test]
    fn test_clip_node_negative_signals_clipped_symmetrically() {
        // Positive and negative signals should clip symmetrically
        let mut clip = ClipNode::new(0, 1);

        let input = vec![3.0, -3.0, 5.0, -5.0];
        let threshold = vec![0.8, 0.8, 0.8, 0.8];
        let inputs = vec![input.as_slice(), threshold.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        clip.process_block(&inputs, &mut output, 44100.0, &context);

        // Positive and negative of same magnitude should clip to same absolute value
        assert!((output[0] + output[1]).abs() < 0.001); // 3.0 and -3.0 should be symmetric
        assert!((output[2] + output[3]).abs() < 0.001); // 5.0 and -5.0 should be symmetric

        // All should be clipped to near threshold
        assert!(output[0] > 0.75 && output[0] <= 0.8);
        assert!(output[1] < -0.75 && output[1] >= -0.8);
    }

    #[test]
    fn test_clip_node_dependencies() {
        let clip = ClipNode::new(5, 10);
        let deps = clip.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 5);
        assert_eq!(deps[1], 10);
    }

    #[test]
    fn test_clip_node_zero_threshold() {
        // Zero threshold should produce zero output (avoid division by zero)
        let mut clip = ClipNode::new(0, 1);

        let input = vec![1.0, 2.0, -1.0, -2.0];
        let threshold = vec![0.0, 0.0, 0.0, 0.0];
        let inputs = vec![input.as_slice(), threshold.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        clip.process_block(&inputs, &mut output, 44100.0, &context);

        for sample in &output {
            assert_eq!(*sample, 0.0);
        }
    }

    #[test]
    fn test_clip_node_full_block() {
        // Test with full 512-sample block
        let mut const_input = ConstantNode::new(3.0);
        let mut const_threshold = ConstantNode::new(1.0);
        let mut clip = ClipNode::new(0, 1);

        let context = create_context();

        // Process constants first
        let mut buf_input = vec![0.0; 512];
        let mut buf_threshold = vec![0.0; 512];

        const_input.process_block(&[], &mut buf_input, 44100.0, &context);
        const_threshold.process_block(&[], &mut buf_threshold, 44100.0, &context);

        // Now clip them
        let inputs = vec![buf_input.as_slice(), buf_threshold.as_slice()];
        let mut output = vec![0.0; 512];

        clip.process_block(&inputs, &mut output, 44100.0, &context);

        // tanh(3.0) ≈ 0.995, so all samples should be ~0.995
        for sample in &output {
            assert!(*sample > 0.99 && *sample < 1.0);
        }
    }
}
