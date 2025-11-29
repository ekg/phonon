/// Tangent function node - applies tan(x) to input signal
///
/// This node performs tangent transformation with clamping to avoid asymptotes.
/// Useful for soft saturation and waveshaping.
use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Tangent node: out = tan(input)
///
/// Clamps input to ±1.5 (≈±π/2) to avoid asymptotes where tan() approaches infinity.
/// Creates smooth waveshaping/saturation characteristics.
///
/// # Example
/// ```ignore
/// // Soft saturation of sine wave
/// let freq = ConstantNode::new(440.0);      // NodeId 0
/// let sine = OscillatorNode::new(0, Waveform::Sine);  // NodeId 1
/// let tan = TanNode::new(1);                // NodeId 2
/// // Output will be soft-clipped sine wave
/// ```
pub struct TanNode {
    input: NodeId,
}

impl TanNode {
    /// Tan - Tangent waveshaper for soft saturation
    ///
    /// Applies tan(x) with clamping to avoid asymptotes, creating smooth
    /// non-linear saturation/waveshaping effects.
    ///
    /// # Parameters
    /// - `input`: Signal to transform (clamped to ±1.5 to avoid asymptotes)
    ///
    /// # Example
    /// ```phonon
    /// ~osc: sine 440
    /// out: ~osc # tan
    /// ```
    pub fn new(input: NodeId) -> Self {
        Self { input }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }
}

impl AudioNode for TanNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(!inputs.is_empty(), "TanNode requires 1 input, got 0");

        let buf = inputs[0];

        debug_assert_eq!(buf.len(), output.len(), "Input length mismatch");

        // Apply tangent with clamping to avoid asymptotes
        for i in 0..output.len() {
            // Clamp to ±1.5 to avoid asymptotes at ±π/2 (≈±1.5708)
            let x_clamped = buf[i].clamp(-1.5, 1.5);
            output[i] = x_clamped.tan();
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input]
    }

    fn name(&self) -> &str {
        "TanNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;
    use std::f32::consts::PI;

    #[test]
    fn test_tan_of_zero() {
        let mut tan_node = TanNode::new(0);

        let input = vec![0.0, 0.0, 0.0, -0.0, 0.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 5];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 5, 2.0, 44100.0);

        tan_node.process_block(&inputs, &mut output, 44100.0, &context);

        for sample in &output {
            assert!(sample.abs() < 0.0001, "tan(0) should be 0, got {}", sample);
        }
    }

    #[test]
    fn test_tan_of_pi_over_4() {
        let mut tan_node = TanNode::new(0);

        // tan(π/4) ≈ 1.0
        let input = vec![PI / 4.0, PI / 4.0, PI / 4.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 3];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 3, 2.0, 44100.0);

        tan_node.process_block(&inputs, &mut output, 44100.0, &context);

        for sample in &output {
            assert!(
                (sample - 1.0).abs() < 0.01,
                "tan(π/4) should be ~1.0, got {}",
                sample
            );
        }
    }

    #[test]
    fn test_tan_of_negative() {
        let mut tan_node = TanNode::new(0);

        // tan(-x) = -tan(x)
        let input = vec![-0.5, -1.0, -0.25];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 3];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 3, 2.0, 44100.0);

        tan_node.process_block(&inputs, &mut output, 44100.0, &context);

        // Verify negative property
        assert!((output[0] - (-0.5_f32).tan()).abs() < 0.0001);
        assert!((output[1] - (-1.0_f32).tan()).abs() < 0.0001);
        assert!((output[2] - (-0.25_f32).tan()).abs() < 0.0001);

        // All should be negative
        for sample in &output {
            assert!(
                *sample < 0.0,
                "tan(negative) should be negative, got {}",
                sample
            );
        }
    }

    #[test]
    fn test_tan_near_asymptote() {
        let mut tan_node = TanNode::new(0);

        // Values near ±π/2 where tan() would go to infinity
        // Without clamping, these would produce very large values
        let input = vec![
            1.57,  // Close to π/2
            -1.57, // Close to -π/2
            2.0,   // Beyond π/2 (will be clamped to 1.5)
            -2.0,  // Beyond -π/2 (will be clamped to -1.5)
            100.0, // Very large (will be clamped to 1.5)
        ];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 5];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 5, 2.0, 44100.0);

        tan_node.process_block(&inputs, &mut output, 44100.0, &context);

        // All outputs should be finite (no infinity)
        for sample in &output {
            assert!(
                sample.is_finite(),
                "Output should be finite, got {}",
                sample
            );
            assert!(
                sample.abs() < 100.0,
                "Output should be bounded due to clamping, got {}",
                sample
            );
        }

        // Clamped values should produce tan(1.5) ≈ 14.1
        let expected_max = 1.5_f32.tan();
        assert!(
            (output[3] - (-expected_max)).abs() < 0.1,
            "Clamped -2.0 should give tan(-1.5)"
        );
        assert!(
            (output[4] - expected_max).abs() < 0.1,
            "Clamped 100.0 should give tan(1.5)"
        );
    }

    #[test]
    fn test_tan_soft_clipping() {
        let mut tan_node = TanNode::new(0);

        // Test that tan() provides soft clipping (non-linear saturation)
        // Create a signal that goes from -1.0 to 1.0
        let mut input = Vec::new();
        for i in 0..16 {
            let x = -1.0 + (i as f32 / 15.0) * 2.0; // -1.0 to 1.0
            input.push(x);
        }
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 16];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 16, 2.0, 44100.0);

        tan_node.process_block(&inputs, &mut output, 44100.0, &context);

        // Verify soft clipping characteristics:
        // 1. Monotonically increasing (tan is monotonic in valid range)
        for i in 1..16 {
            assert!(
                output[i] > output[i - 1],
                "tan should be monotonically increasing, but {} <= {}",
                output[i],
                output[i - 1]
            );
        }

        // 2. Center value should be near zero (tan(0) = 0)
        assert!(output[7].abs() < 0.1, "Center should be near zero");

        // 3. Edges should show saturation (approaching but not reaching asymptote)
        assert!(output[0] < -0.7, "Negative edge should be saturated");
        assert!(output[15] > 0.7, "Positive edge should be saturated");
    }

    #[test]
    fn test_tan_dependencies() {
        let tan_node = TanNode::new(7);
        let deps = tan_node.input_nodes();

        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], 7);
    }

    #[test]
    fn test_tan_with_constant() {
        let mut const_node = ConstantNode::new(0.5);
        let mut tan_node = TanNode::new(0);

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        // Process constant first
        let mut buf = vec![0.0; 512];
        const_node.process_block(&[], &mut buf, 44100.0, &context);

        // Now apply tangent
        let inputs = vec![buf.as_slice()];
        let mut output = vec![0.0; 512];

        tan_node.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be tan(0.5) ≈ 0.546
        let expected = 0.5_f32.tan();
        for sample in &output {
            assert!(
                (sample - expected).abs() < 0.0001,
                "Expected {}, got {}",
                expected,
                sample
            );
        }
    }
}
