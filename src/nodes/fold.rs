/// Fold node - reflects input signal at min/max boundaries (wave folding distortion)
///
/// This node performs sample-by-sample wave folding.
/// Output[i] = fold(Input[i], Min[i], Max[i]) for all samples.
///
/// Wave folding reflects the signal when it exceeds the boundaries, creating
/// harmonic distortion. Unlike wrap (which uses modulo), fold reflects the
/// signal back into the range, producing different harmonic content.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Fold node: out = fold(input, min, max)
///
/// # Example
/// ```ignore
/// // Fold signal to [0.0, 1.0] range (wave folding distortion)
/// let input = OscillatorNode::new(0, Waveform::Sine);  // NodeId 1
/// let min = ConstantNode::new(0.0);                     // NodeId 2
/// let max = ConstantNode::new(1.0);                     // NodeId 3
/// let fold = FoldNode::new(1, 2, 3);                    // NodeId 4
/// // Output will reflect at the 0.0 and 1.0 boundaries
/// ```
pub struct FoldNode {
    input: NodeId,
    min_input: NodeId,
    max_input: NodeId,
}

impl FoldNode {
    /// Fold - Wave folding distortion that reflects signal at boundaries
    ///
    /// Reflects signal when exceeding min/max bounds, creating harmonic distortion
    /// with characteristic folding effects, different from hard clipping.
    ///
    /// # Parameters
    /// - `input`: NodeId providing signal to fold
    /// - `min_input`: NodeId providing minimum boundary (default: -1.0)
    /// - `max_input`: NodeId providing maximum boundary (default: 1.0)
    ///
    /// # Example
    /// ```phonon
    /// ~signal: sine 110 * 2
    /// ~folded: ~signal # fold -1 1
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

    /// Fold a value into [min, max] range by reflecting at boundaries
    ///
    /// # Arguments
    /// * `val` - Value to fold
    /// * `min` - Minimum of range
    /// * `max` - Maximum of range
    ///
    /// # Returns
    /// Folded value in [min, max]
    #[inline]
    fn fold_value(val: f32, min: f32, max: f32) -> f32 {
        let range = max - min;

        // Handle degenerate case: range is zero or nearly zero
        if range <= 0.0 {
            return min;
        }

        let mut value = val;

        // Fold signal when it exceeds boundaries
        // Use iteration limit to prevent infinite loops with edge cases
        for _ in 0..100 {
            if value >= min && value <= max {
                break;
            }

            if value > max {
                value = max - (value - max); // Reflect down from max
            } else if value < min {
                value = min + (min - value); // Reflect up from min
            }
        }

        // Final clamp in case we exceeded iteration limit
        value.clamp(min, max)
    }
}

impl AudioNode for FoldNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 3,
            "FoldNode requires 3 inputs, got {}",
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
        debug_assert_eq!(
            buf_min.len(),
            output.len(),
            "Min buffer length mismatch"
        );
        debug_assert_eq!(
            buf_max.len(),
            output.len(),
            "Max buffer length mismatch"
        );

        // Sample-wise fold operation
        for i in 0..output.len() {
            output[i] = Self::fold_value(buf_input[i], buf_min[i], buf_max[i]);
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.min_input, self.max_input]
    }

    fn name(&self) -> &str {
        "FoldNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::Fraction;

    #[test]
    fn test_fold_within_range_unchanged() {
        // Test: Fold 0.5 into [0, 1] = 0.5 (already in range)
        let mut fold = FoldNode::new(0, 1, 2);

        let input = vec![0.5; 512];
        let min = vec![0.0; 512];
        let max = vec![1.0; 512];
        let inputs = vec![input.as_slice(), min.as_slice(), max.as_slice()];

        let mut output = vec![0.0; 512];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        fold.process_block(&inputs, &mut output, 44100.0, &context);

        // 0.5 stays 0.5 (already in range)
        for sample in &output {
            assert!((*sample - 0.5).abs() < 1e-6, "Expected ~0.5, got {}", sample);
        }
    }

    #[test]
    fn test_fold_above_max_reflects_down() {
        // Test: Fold 1.3 into [0, 1] = 0.7 (reflects down by 0.3)
        let mut fold = FoldNode::new(0, 1, 2);

        let input = vec![1.3; 512];
        let min = vec![0.0; 512];
        let max = vec![1.0; 512];
        let inputs = vec![input.as_slice(), min.as_slice(), max.as_slice()];

        let mut output = vec![0.0; 512];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        fold.process_block(&inputs, &mut output, 44100.0, &context);

        // 1.3 reflects to 0.7 (max - (1.3 - max) = 1.0 - 0.3 = 0.7)
        for sample in &output {
            assert!((*sample - 0.7).abs() < 1e-6, "Expected ~0.7, got {}", sample);
        }
    }

    #[test]
    fn test_fold_below_min_reflects_up() {
        // Test: Fold -0.3 into [0, 1] = 0.3 (reflects up by 0.3)
        let mut fold = FoldNode::new(0, 1, 2);

        let input = vec![-0.3; 512];
        let min = vec![0.0; 512];
        let max = vec![1.0; 512];
        let inputs = vec![input.as_slice(), min.as_slice(), max.as_slice()];

        let mut output = vec![0.0; 512];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        fold.process_block(&inputs, &mut output, 44100.0, &context);

        // -0.3 reflects to 0.3 (min + (min - (-0.3)) = 0.0 + 0.3 = 0.3)
        for sample in &output {
            assert!((*sample - 0.3).abs() < 1e-6, "Expected ~0.3, got {}", sample);
        }
    }

    #[test]
    fn test_fold_multiple_reflections() {
        // Test: Fold 2.5 into [0, 1] with multiple reflections
        let mut fold = FoldNode::new(0, 1, 2);

        let input = vec![2.5; 512];
        let min = vec![0.0; 512];
        let max = vec![1.0; 512];
        let inputs = vec![input.as_slice(), min.as_slice(), max.as_slice()];

        let mut output = vec![0.0; 512];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        fold.process_block(&inputs, &mut output, 44100.0, &context);

        // 2.5 reflects multiple times:
        // Step 1: 2.5 > 1.0, so: 1.0 - (2.5 - 1.0) = -0.5
        // Step 2: -0.5 < 0.0, so: 0.0 + (0.0 - (-0.5)) = 0.5
        for sample in &output {
            assert!((*sample - 0.5).abs() < 1e-6, "Expected ~0.5, got {}", sample);
        }
    }

    #[test]
    fn test_fold_creates_harmonics() {
        // Test: Verify fold creates different output than wrap (harmonics test)
        let mut fold = FoldNode::new(0, 1, 2);

        // Test with varying amplitudes to show distortion character
        let input = vec![-0.5, 0.0, 0.5, 1.0, 1.5, 2.0];
        let min = vec![0.0; 6];
        let max = vec![1.0; 6];
        let inputs = vec![input.as_slice(), min.as_slice(), max.as_slice()];

        let mut output = vec![0.0; 6];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            6,
            2.0,
            44100.0,
        );

        fold.process_block(&inputs, &mut output, 44100.0, &context);

        // -0.5: reflects to 0.5
        assert!((output[0] - 0.5).abs() < 1e-6, "Expected ~0.5, got {}", output[0]);
        // 0.0: stays 0.0
        assert!((output[1] - 0.0).abs() < 1e-6, "Expected ~0.0, got {}", output[1]);
        // 0.5: stays 0.5
        assert!((output[2] - 0.5).abs() < 1e-6, "Expected ~0.5, got {}", output[2]);
        // 1.0: stays 1.0
        assert!((output[3] - 1.0).abs() < 1e-6, "Expected ~1.0, got {}", output[3]);
        // 1.5: reflects to 0.5
        assert!((output[4] - 0.5).abs() < 1e-6, "Expected ~0.5, got {}", output[4]);
        // 2.0: multiple reflections to 0.0
        assert!((output[5] - 0.0).abs() < 1e-6, "Expected ~0.0, got {}", output[5]);
    }

    #[test]
    fn test_fold_dependencies() {
        let fold = FoldNode::new(5, 10, 15);
        let deps = fold.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 5);
        assert_eq!(deps[1], 10);
        assert_eq!(deps[2], 15);
    }

    #[test]
    fn test_fold_with_constants() {
        // Test: Fold with constant boundaries
        let mut fold = FoldNode::new(0, 1, 2);

        let input = vec![-1.0, -0.2, 0.3, 1.2, 2.5];
        let min = vec![-0.5; 5];
        let max = vec![0.5; 5]; // range = 1.0
        let inputs = vec![input.as_slice(), min.as_slice(), max.as_slice()];

        let mut output = vec![0.0; 5];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            5,
            2.0,
            44100.0,
        );

        fold.process_block(&inputs, &mut output, 44100.0, &context);

        // -1.0: reflects to -0.0 (0.5 - (0.5 - (-1.0)) = -1.0, then -0.5 + (-0.5 - (-1.0)) = 0.0)
        // Detailed: -1.0 < -0.5, so: -0.5 + (-0.5 - (-1.0)) = -0.5 + 0.5 = 0.0
        assert!((output[0] - 0.0).abs() < 1e-6, "Expected ~0.0, got {}", output[0]);
        // -0.2: stays -0.2
        assert!((output[1] - (-0.2)).abs() < 1e-6, "Expected ~-0.2, got {}", output[1]);
        // 0.3: stays 0.3
        assert!((output[2] - 0.3).abs() < 1e-6, "Expected ~0.3, got {}", output[2]);
        // 1.2: reflects to -0.2 (0.5 - (1.2 - 0.5) = -0.2)
        assert!((output[3] - (-0.2)).abs() < 1e-6, "Expected ~-0.2, got {}", output[3]);
        // 2.5: multiple reflections
        // Step 1: 0.5 - (2.5 - 0.5) = -1.5
        // Step 2: -0.5 + (-0.5 - (-1.5)) = 0.5
        assert!((output[4] - 0.5).abs() < 1e-6, "Expected ~0.5, got {}", output[4]);
    }

    #[test]
    fn test_fold_min_equals_max_returns_min() {
        // Test: Fold to [0.7, 0.7] (degenerate range) always returns 0.7
        let mut fold = FoldNode::new(0, 1, 2);

        // Try various input values
        let input = vec![-10.0, -1.0, 0.0, 0.7, 5.0, 100.0];
        let min = vec![0.7; 6];
        let max = vec![0.7; 6];
        let inputs = vec![input.as_slice(), min.as_slice(), max.as_slice()];

        let mut output = vec![0.0; 6];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            6,
            2.0,
            44100.0,
        );

        fold.process_block(&inputs, &mut output, 44100.0, &context);

        // All outputs should be 0.7 (degenerate range returns min)
        for sample in &output {
            assert_eq!(*sample, 0.7);
        }
    }
}
