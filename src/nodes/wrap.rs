/// Wrap node - wraps input signal into [min, max] range using modulo
///
/// This node performs sample-by-sample wrapping.
/// Output[i] = wrap(Input[i], Min[i], Max[i]) for all samples.
///
/// The wrapping operation uses modulo arithmetic to fold values outside
/// the range back into the range, creating a periodic wrapping effect.
use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Wrap node: out = wrap(input, min, max)
///
/// # Example
/// ```ignore
/// // Wrap signal to [0.0, 1.0] range
/// let input = OscillatorNode::new(0, Waveform::Sine);  // NodeId 1
/// let min = ConstantNode::new(0.0);                     // NodeId 2
/// let max = ConstantNode::new(1.0);                     // NodeId 3
/// let wrap = WrapNode::new(1, 2, 3);                    // NodeId 4
/// // Output will wrap around the 0.0-1.0 range
/// ```
pub struct WrapNode {
    input: NodeId,
    min_input: NodeId,
    max_input: NodeId,
}

impl WrapNode {
    /// Wrap - Wraps signal into [min, max] range using modulo
    ///
    /// Folds values outside range back into range periodically
    /// using modulo arithmetic. Useful for wavetable indexing.
    ///
    /// # Parameters
    /// - `input`: Signal to wrap
    /// - `min_input`: Minimum of range
    /// - `max_input`: Maximum of range
    ///
    /// # Example
    /// ```phonon
    /// ~phase: lfo 1.0 -2 2
    /// out: wrap ~phase 0 1
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

    /// Wrap a value into [min, max] range using modulo arithmetic
    ///
    /// # Arguments
    /// * `val` - Value to wrap
    /// * `min` - Minimum of range
    /// * `max` - Maximum of range
    ///
    /// # Returns
    /// Wrapped value in [min, max]
    #[inline]
    fn wrap_value(val: f32, min: f32, max: f32) -> f32 {
        let range = max - min;

        // Handle degenerate case: range is zero or nearly zero
        if range.abs() < 1e-10 {
            return min;
        }

        // Normalize to [0, range), then shift back to [min, max)
        let normalized = (val - min) % range;

        // Handle negative modulo results (Rust's % can return negative)
        if normalized < 0.0 {
            normalized + range + min
        } else {
            normalized + min
        }
    }
}

impl AudioNode for WrapNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 3,
            "WrapNode requires 3 inputs, got {}",
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

        // Sample-wise wrap operation
        for i in 0..output.len() {
            output[i] = Self::wrap_value(buf_input[i], buf_min[i], buf_max[i]);
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.min_input, self.max_input]
    }

    fn name(&self) -> &str {
        "WrapNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::Fraction;

    #[test]
    fn test_wrap_value_at_upper_bound_wraps_to_min() {
        // Test: Wrap 5.0 into [0, 1] = 0.0 (exactly at upper bound after 5 wraps)
        let mut wrap = WrapNode::new(0, 1, 2);

        let input = vec![5.0; 512];
        let min = vec![0.0; 512];
        let max = vec![1.0; 512];
        let inputs = vec![input.as_slice(), min.as_slice(), max.as_slice()];

        let mut output = vec![0.0; 512];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        wrap.process_block(&inputs, &mut output, 44100.0, &context);

        // 5.0 wraps to 0.0 (5 complete cycles)
        for sample in &output {
            assert!(
                (*sample - 0.0).abs() < 1e-6,
                "Expected ~0.0, got {}",
                sample
            );
        }
    }

    #[test]
    fn test_wrap_negative_value_wraps_into_range() {
        // Test: Wrap -0.5 into [0, 1] = 0.5
        let mut wrap = WrapNode::new(0, 1, 2);

        let input = vec![-0.5; 512];
        let min = vec![0.0; 512];
        let max = vec![1.0; 512];
        let inputs = vec![input.as_slice(), min.as_slice(), max.as_slice()];

        let mut output = vec![0.0; 512];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        wrap.process_block(&inputs, &mut output, 44100.0, &context);

        // -0.5 wraps to 0.5 (wraps from below)
        for sample in &output {
            assert!(
                (*sample - 0.5).abs() < 1e-6,
                "Expected ~0.5, got {}",
                sample
            );
        }
    }

    #[test]
    fn test_wrap_positive_value_wraps_around() {
        // Test: Wrap 2.5 into [0, 1] = 0.5 (wraps around twice)
        let mut wrap = WrapNode::new(0, 1, 2);

        let input = vec![2.5; 512];
        let min = vec![0.0; 512];
        let max = vec![1.0; 512];
        let inputs = vec![input.as_slice(), min.as_slice(), max.as_slice()];

        let mut output = vec![0.0; 512];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        wrap.process_block(&inputs, &mut output, 44100.0, &context);

        // 2.5 wraps to 0.5 (2 full cycles + 0.5)
        for sample in &output {
            assert!(
                (*sample - 0.5).abs() < 1e-6,
                "Expected ~0.5, got {}",
                sample
            );
        }
    }

    #[test]
    fn test_wrap_value_in_range_unchanged() {
        // Test: Wrap 0.3 into [0, 1] = 0.3 (already in range)
        let mut wrap = WrapNode::new(0, 1, 2);

        let input = vec![0.3; 512];
        let min = vec![0.0; 512];
        let max = vec![1.0; 512];
        let inputs = vec![input.as_slice(), min.as_slice(), max.as_slice()];

        let mut output = vec![0.0; 512];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        wrap.process_block(&inputs, &mut output, 44100.0, &context);

        // 0.3 stays 0.3 (already in range)
        for sample in &output {
            assert!(
                (*sample - 0.3).abs() < 1e-6,
                "Expected ~0.3, got {}",
                sample
            );
        }
    }

    #[test]
    fn test_wrap_min_equals_max_returns_min() {
        // Test: Wrap to [0.7, 0.7] (degenerate range) always returns 0.7
        let mut wrap = WrapNode::new(0, 1, 2);

        // Try various input values
        let input = vec![-10.0, -1.0, 0.0, 0.7, 5.0, 100.0];
        let min = vec![0.7; 6];
        let max = vec![0.7; 6];
        let inputs = vec![input.as_slice(), min.as_slice(), max.as_slice()];

        let mut output = vec![0.0; 6];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 6, 2.0, 44100.0);

        wrap.process_block(&inputs, &mut output, 44100.0, &context);

        // All outputs should be 0.7 (degenerate range returns min)
        for sample in &output {
            assert_eq!(*sample, 0.7);
        }
    }

    #[test]
    fn test_wrap_varying_values() {
        // Test: Wrap with varying values per sample
        let mut wrap = WrapNode::new(0, 1, 2);

        let input = vec![-1.5, -0.5, 0.0, 0.5, 1.5, 3.7];
        let min = vec![0.0; 6];
        let max = vec![1.0; 6];
        let inputs = vec![input.as_slice(), min.as_slice(), max.as_slice()];

        let mut output = vec![0.0; 6];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 6, 2.0, 44100.0);

        wrap.process_block(&inputs, &mut output, 44100.0, &context);

        // -1.5 wraps to 0.5 (wraps from below)
        assert!(
            (output[0] - 0.5).abs() < 1e-6,
            "Expected ~0.5, got {}",
            output[0]
        );
        // -0.5 wraps to 0.5
        assert!(
            (output[1] - 0.5).abs() < 1e-6,
            "Expected ~0.5, got {}",
            output[1]
        );
        // 0.0 stays 0.0
        assert!(
            (output[2] - 0.0).abs() < 1e-6,
            "Expected ~0.0, got {}",
            output[2]
        );
        // 0.5 stays 0.5
        assert!(
            (output[3] - 0.5).abs() < 1e-6,
            "Expected ~0.5, got {}",
            output[3]
        );
        // 1.5 wraps to 0.5
        assert!(
            (output[4] - 0.5).abs() < 1e-6,
            "Expected ~0.5, got {}",
            output[4]
        );
        // 3.7 wraps to 0.7 (3 complete cycles + 0.7)
        assert!(
            (output[5] - 0.7).abs() < 1e-6,
            "Expected ~0.7, got {}",
            output[5]
        );
    }

    #[test]
    fn test_wrap_non_zero_min() {
        // Test: Wrap into [2.0, 5.0] range
        let mut wrap = WrapNode::new(0, 1, 2);

        let input = vec![1.0, 2.5, 6.5, 8.0, -1.0];
        let min = vec![2.0; 5];
        let max = vec![5.0; 5]; // range = 3.0
        let inputs = vec![input.as_slice(), min.as_slice(), max.as_slice()];

        let mut output = vec![0.0; 5];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 5, 2.0, 44100.0);

        wrap.process_block(&inputs, &mut output, 44100.0, &context);

        // 1.0: (1.0 - 2.0) % 3.0 = -1.0 % 3.0 = -1.0, then +3.0 +2.0 = 4.0
        assert!(
            (output[0] - 4.0).abs() < 1e-6,
            "Expected ~4.0, got {}",
            output[0]
        );
        // 2.5: in range, stays 2.5
        assert!(
            (output[1] - 2.5).abs() < 1e-6,
            "Expected ~2.5, got {}",
            output[1]
        );
        // 6.5: (6.5 - 2.0) % 3.0 = 4.5 % 3.0 = 1.5, then +2.0 = 3.5
        assert!(
            (output[2] - 3.5).abs() < 1e-6,
            "Expected ~3.5, got {}",
            output[2]
        );
        // 8.0: (8.0 - 2.0) % 3.0 = 6.0 % 3.0 = 0.0, then +2.0 = 2.0
        assert!(
            (output[3] - 2.0).abs() < 1e-6,
            "Expected ~2.0, got {}",
            output[3]
        );
        // -1.0: (-1.0 - 2.0) % 3.0 = -3.0 % 3.0 = 0.0, then +2.0 = 2.0
        assert!(
            (output[4] - 2.0).abs() < 1e-6,
            "Expected ~2.0, got {}",
            output[4]
        );
    }

    #[test]
    fn test_wrap_dependencies() {
        let wrap = WrapNode::new(5, 10, 15);
        let deps = wrap.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 5);
        assert_eq!(deps[1], 10);
        assert_eq!(deps[2], 15);
    }
}
