/// Modulo node - computes remainder of signal A divided by signal B
///
/// This node implements the modulo (remainder) operation with protection against
/// division by zero. Uses rem_euclid for consistent positive results.
/// Output[i] = Input_A[i] % Input_B[i] for all samples, with safeguards.
///
/// Division by zero protection: If |B[i]| < 1e-10, uses 1e-10 as the divisor.
///
/// # Applications
/// - Phase wrapping for oscillators (wrap phase 0.0 to 2π)
/// - Cyclic value generation (repeating envelopes, patterns)
/// - Quantization and grid snapping
/// - Wave folding and wrapping effects
use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Modulo node: out = a % b
///
/// # Example
/// ```ignore
/// // Wrap phase from 0.0 to 2π
/// let phase = LineNode::new(0.0, 1.0);     // NodeId 0 (ramp 0 to 1)
/// let twopi = ConstantNode::new(6.28318);  // NodeId 1
/// let wrapped = ModNode::new(0, 1);        // NodeId 2
/// // Output will be phase wrapped to [0, 2π)
/// ```
///
/// # Division by Zero Protection
///
/// To prevent NaN and infinity values that can corrupt audio processing:
/// - If |input_b[i]| < 1e-10, uses 1e-10 as the divisor
/// - This threshold is chosen to be well below typical audio signal levels
///
/// # Euclidean Modulo
///
/// Uses `rem_euclid` instead of `%` to ensure positive results:
/// - `rem_euclid` always returns values in [0, divisor)
/// - Standard `%` can return negative values when dividend is negative
/// - Example: -10 % 3 = -1 (standard), but -10.rem_euclid(3) = 2 (euclidean)
pub struct ModNode {
    input_a: NodeId,
    input_b: NodeId,
}

impl ModNode {
    /// Modulo - Wrapping remainder operation
    ///
    /// Computes fmod(a, b) - the floating-point remainder of a/b.
    /// Useful for phase wrapping, periodic modulation, and signal shaping.
    ///
    /// # Parameters
    /// - `input_a`: Dividend (value to be wrapped)
    /// - `input_b`: Divisor (wrapping period)
    ///
    /// # Example
    /// ```phonon
    /// ~phase: line 0 10 2 # trigger
    /// ~wrapped: ~phase # modulo 2
    /// out: sine (~wrapped * 3.14159) * 0.5
    /// ```
    pub fn new(input_a: NodeId, input_b: NodeId) -> Self {
        Self { input_a, input_b }
    }

    /// Get the dividend input node ID
    pub fn input_a(&self) -> NodeId {
        self.input_a
    }

    /// Get the divisor input node ID
    pub fn input_b(&self) -> NodeId {
        self.input_b
    }
}

impl AudioNode for ModNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "ModNode requires 2 inputs, got {}",
            inputs.len()
        );

        let buf_a = inputs[0];
        let buf_b = inputs[1];

        debug_assert_eq!(buf_a.len(), output.len(), "Input A length mismatch");
        debug_assert_eq!(buf_b.len(), output.len(), "Input B length mismatch");

        // Safe modulo with zero protection
        const EPSILON: f32 = 1e-10;

        for i in 0..output.len() {
            let a = buf_a[i];
            let b = buf_b[i];

            // Protect against division by zero
            let b_safe = if b.abs() < EPSILON { EPSILON } else { b };

            // Use rem_euclid for positive-only modulo
            output[i] = a.rem_euclid(b_safe);
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input_a, self.input_b]
    }

    fn name(&self) -> &str {
        "ModNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_mod_positive_values() {
        let mut mod_node = ModNode::new(0, 1);

        // Test basic modulo: 10 % 3 = 1
        let input_a = vec![10.0, 11.0, 12.0, 13.0];
        let input_b = vec![3.0, 3.0, 3.0, 3.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        mod_node.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0); // 10 % 3 = 1
        assert_eq!(output[1], 2.0); // 11 % 3 = 2
        assert_eq!(output[2], 0.0); // 12 % 3 = 0
        assert_eq!(output[3], 1.0); // 13 % 3 = 1
    }

    #[test]
    fn test_mod_negative_dividend() {
        let mut mod_node = ModNode::new(0, 1);

        // Test with negative dividends - rem_euclid gives positive results
        let input_a = vec![-10.0, -7.0, -4.0, -1.0];
        let input_b = vec![3.0, 3.0, 3.0, 3.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        mod_node.process_block(&inputs, &mut output, 44100.0, &context);

        // rem_euclid ensures positive results
        assert_eq!(output[0], 2.0); // -10 % 3 = 2 (not -1)
        assert_eq!(output[1], 2.0); // -7 % 3 = 2 (not -1)
        assert_eq!(output[2], 2.0); // -4 % 3 = 2 (not -1)
        assert_eq!(output[3], 2.0); // -1 % 3 = 2 (not -1)
    }

    #[test]
    fn test_mod_fractional() {
        let mut mod_node = ModNode::new(0, 1);

        // Test fractional modulo
        let input_a = vec![5.5, 7.3, 9.8, 2.1];
        let input_b = vec![2.0, 2.0, 2.0, 2.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        mod_node.process_block(&inputs, &mut output, 44100.0, &context);

        // Use approximate equality for floating point
        assert!(
            (output[0] - 1.5).abs() < 0.0001,
            "Expected ~1.5, got {}",
            output[0]
        );
        assert!(
            (output[1] - 1.3).abs() < 0.0001,
            "Expected ~1.3, got {}",
            output[1]
        );
        assert!(
            (output[2] - 1.8).abs() < 0.0001,
            "Expected ~1.8, got {}",
            output[2]
        );
        assert!(
            (output[3] - 0.1).abs() < 0.0001,
            "Expected ~0.1, got {}",
            output[3]
        );
    }

    #[test]
    fn test_mod_zero_divisor_protection() {
        let mut mod_node = ModNode::new(0, 1);

        // Test division by zero and near-zero values
        let input_a = vec![10.0, 20.0, 30.0, 40.0];
        let input_b = vec![0.0, 1e-11, -1e-11, 0.0]; // All below epsilon threshold
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![999.0; 4]; // Initialize with non-zero to verify overwrite
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        mod_node.process_block(&inputs, &mut output, 44100.0, &context);

        // All should use epsilon (1e-10) as divisor, results will be very small
        for sample in &output {
            assert!(
                sample.is_finite(),
                "Output should be finite, got {}",
                sample
            );
            assert!(!sample.is_nan(), "Output should not be NaN, got {}", sample);
            assert!(
                *sample >= 0.0,
                "rem_euclid should give positive results, got {}",
                sample
            );
        }
    }

    #[test]
    fn test_mod_phase_wrapping() {
        let mut mod_node = ModNode::new(0, 1);

        // Simulate phase wrapping 0 to 2π (approximately 6.28318)
        let twopi = 6.28318;
        let input_a = vec![0.0, 3.14159, 6.28318, 9.42477, 12.56636]; // 0, π, 2π, 3π, 4π
        let input_b = vec![twopi, twopi, twopi, twopi, twopi];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 5];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 5, 2.0, 44100.0);

        mod_node.process_block(&inputs, &mut output, 44100.0, &context);

        // All phases should wrap to [0, 2π)
        assert!((output[0] - 0.0).abs() < 0.001); // 0 % 2π = 0
        assert!((output[1] - 3.14159).abs() < 0.001); // π % 2π = π
        assert!((output[2] - 0.0).abs() < 0.001); // 2π % 2π = 0
        assert!((output[3] - 3.14159).abs() < 0.001); // 3π % 2π = π
        assert!((output[4] - 0.0).abs() < 0.001); // 4π % 2π = 0

        // Verify all wrapped values are in valid range [0, 2π)
        for sample in &output {
            assert!(
                *sample >= 0.0,
                "Wrapped phase should be >= 0, got {}",
                sample
            );
            assert!(
                *sample < twopi,
                "Wrapped phase should be < 2π, got {}",
                sample
            );
        }
    }

    #[test]
    fn test_mod_dependencies() {
        let mod_node = ModNode::new(5, 10);
        let deps = mod_node.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 5);
        assert_eq!(deps[1], 10);
    }

    #[test]
    fn test_mod_with_constants() {
        let mut const_a = ConstantNode::new(10.0);
        let mut const_b = ConstantNode::new(3.0);
        let mut mod_node = ModNode::new(0, 1);

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        // Process constants first
        let mut buf_a = vec![0.0; 512];
        let mut buf_b = vec![0.0; 512];

        const_a.process_block(&[], &mut buf_a, 44100.0, &context);
        const_b.process_block(&[], &mut buf_b, 44100.0, &context);

        // Now compute modulo
        let inputs = vec![buf_a.as_slice(), buf_b.as_slice()];
        let mut output = vec![0.0; 512];

        mod_node.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be 1.0 (10 % 3)
        for sample in &output {
            assert_eq!(*sample, 1.0);
        }
    }

    #[test]
    fn test_mod_varying_divisor() {
        let mut mod_node = ModNode::new(0, 1);

        // Same dividend, varying divisor
        let input_a = vec![10.0, 10.0, 10.0, 10.0];
        let input_b = vec![3.0, 4.0, 5.0, 6.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        mod_node.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0); // 10 % 3 = 1
        assert_eq!(output[1], 2.0); // 10 % 4 = 2
        assert_eq!(output[2], 0.0); // 10 % 5 = 0
        assert_eq!(output[3], 4.0); // 10 % 6 = 4
    }

    #[test]
    fn test_mod_cyclic_pattern() {
        let mut mod_node = ModNode::new(0, 1);

        // Generate cyclic pattern: 0, 1, 2, 0, 1, 2, ...
        let input_a = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        let input_b = vec![3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 8];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 8, 2.0, 44100.0);

        mod_node.process_block(&inputs, &mut output, 44100.0, &context);

        // Should produce repeating pattern: 0, 1, 2, 0, 1, 2, 0, 1
        assert_eq!(output[0], 0.0);
        assert_eq!(output[1], 1.0);
        assert_eq!(output[2], 2.0);
        assert_eq!(output[3], 0.0);
        assert_eq!(output[4], 1.0);
        assert_eq!(output[5], 2.0);
        assert_eq!(output[6], 0.0);
        assert_eq!(output[7], 1.0);
    }
}
