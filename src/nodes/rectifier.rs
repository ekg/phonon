/// Rectifier node - half-wave and full-wave rectification
///
/// This node performs rectification on the input signal:
/// - Full-wave: Output[i] = |Input[i]| (same as absolute value)
/// - Half-wave: Output[i] = max(Input[i], 0.0) (negative values become zero)
///
/// Useful for envelope following, distortion effects, and signal conditioning.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Rectification mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RectifierMode {
    /// Full-wave rectification: output = |input|
    /// All negative values are flipped to positive
    FullWave,

    /// Half-wave rectification: output = max(input, 0.0)
    /// All negative values become zero
    HalfWave,
}

/// Rectifier node: performs full-wave or half-wave rectification
///
/// # Example
/// ```ignore
/// // Half-wave rectify a sine wave (classic envelope follower)
/// let freq = ConstantNode::new(440.0);      // NodeId 0
/// let sine = OscillatorNode::new(0, Waveform::Sine);  // NodeId 1
/// let rect = RectifierNode::new(1, RectifierMode::HalfWave);  // NodeId 2
/// // Output will be half-wave rectified sine (only positive peaks)
/// ```
pub struct RectifierNode {
    input: NodeId,
    mode: RectifierMode,
}

impl RectifierNode {
    /// Create a new rectifier node
    ///
    /// # Arguments
    /// * `input` - NodeId of input signal
    /// * `mode` - Rectification mode (FullWave or HalfWave)
    pub fn new(input: NodeId, mode: RectifierMode) -> Self {
        Self { input, mode }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }

    /// Get the rectification mode
    pub fn mode(&self) -> RectifierMode {
        self.mode
    }
}

impl AudioNode for RectifierNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            !inputs.is_empty(),
            "RectifierNode requires 1 input, got 0"
        );

        let buf = inputs[0];

        debug_assert_eq!(
            buf.len(),
            output.len(),
            "Input length mismatch"
        );

        // Apply rectification to each sample
        match self.mode {
            RectifierMode::FullWave => {
                for i in 0..output.len() {
                    output[i] = buf[i].abs();
                }
            }
            RectifierMode::HalfWave => {
                for i in 0..output.len() {
                    output[i] = buf[i].max(0.0);
                }
            }
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input]
    }

    fn name(&self) -> &str {
        match self.mode {
            RectifierMode::FullWave => "RectifierNode(FullWave)",
            RectifierMode::HalfWave => "RectifierNode(HalfWave)",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_rectifier_full_wave_positive() {
        let mut rect_node = RectifierNode::new(0, RectifierMode::FullWave);

        let input = vec![1.0, 2.0, 3.0, 4.5, 100.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 5];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            5,
            2.0,
            44100.0,
        );

        rect_node.process_block(&inputs, &mut output, 44100.0, &context);

        // Positive values should remain unchanged
        assert_eq!(output[0], 1.0);
        assert_eq!(output[1], 2.0);
        assert_eq!(output[2], 3.0);
        assert_eq!(output[3], 4.5);
        assert_eq!(output[4], 100.0);
    }

    #[test]
    fn test_rectifier_full_wave_negative() {
        let mut rect_node = RectifierNode::new(0, RectifierMode::FullWave);

        let input = vec![-1.0, -2.0, -3.0, -4.5, -100.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 5];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            5,
            2.0,
            44100.0,
        );

        rect_node.process_block(&inputs, &mut output, 44100.0, &context);

        // Negative values should be flipped to positive
        assert_eq!(output[0], 1.0);
        assert_eq!(output[1], 2.0);
        assert_eq!(output[2], 3.0);
        assert_eq!(output[3], 4.5);
        assert_eq!(output[4], 100.0);
    }

    #[test]
    fn test_rectifier_half_wave_positive() {
        let mut rect_node = RectifierNode::new(0, RectifierMode::HalfWave);

        let input = vec![1.0, 2.0, 3.0, 4.5, 100.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 5];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            5,
            2.0,
            44100.0,
        );

        rect_node.process_block(&inputs, &mut output, 44100.0, &context);

        // Positive values should remain unchanged
        assert_eq!(output[0], 1.0);
        assert_eq!(output[1], 2.0);
        assert_eq!(output[2], 3.0);
        assert_eq!(output[3], 4.5);
        assert_eq!(output[4], 100.0);
    }

    #[test]
    fn test_rectifier_half_wave_negative() {
        let mut rect_node = RectifierNode::new(0, RectifierMode::HalfWave);

        let input = vec![-1.0, -2.0, -3.0, -4.5, -100.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 5];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            5,
            2.0,
            44100.0,
        );

        rect_node.process_block(&inputs, &mut output, 44100.0, &context);

        // Negative values should become zero
        assert_eq!(output[0], 0.0);
        assert_eq!(output[1], 0.0);
        assert_eq!(output[2], 0.0);
        assert_eq!(output[3], 0.0);
        assert_eq!(output[4], 0.0);
    }

    #[test]
    fn test_rectifier_sine_wave_full() {
        use std::f32::consts::PI;

        let mut rect_node = RectifierNode::new(0, RectifierMode::FullWave);

        // Create one cycle of a sine wave (16 samples)
        let mut input = Vec::new();
        for i in 0..16 {
            let phase = (i as f32 / 16.0) * 2.0 * PI;
            input.push(phase.sin());
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

        rect_node.process_block(&inputs, &mut output, 44100.0, &context);

        // All output values should be positive (full-wave rectification)
        for sample in &output {
            assert!(*sample >= 0.0, "Sample {} should be non-negative", sample);
        }

        // First half of sine is positive (should be unchanged)
        for i in 0..8 {
            let phase = (i as f32 / 16.0) * 2.0 * PI;
            let expected = phase.sin();
            assert!(
                (output[i] - expected).abs() < 0.0001,
                "Sample {} should be close to original sine value {}",
                output[i],
                expected
            );
        }

        // Second half should be flipped positive
        for i in 8..16 {
            let phase = (i as f32 / 16.0) * 2.0 * PI;
            let expected = phase.sin().abs();
            assert!(
                (output[i] - expected).abs() < 0.0001,
                "Sample {} should be absolute value of sine",
                output[i]
            );
        }
    }

    #[test]
    fn test_rectifier_sine_wave_half() {
        use std::f32::consts::PI;

        let mut rect_node = RectifierNode::new(0, RectifierMode::HalfWave);

        // Create one cycle of a sine wave (16 samples)
        let mut input = Vec::new();
        for i in 0..16 {
            let phase = (i as f32 / 16.0) * 2.0 * PI;
            input.push(phase.sin());
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

        rect_node.process_block(&inputs, &mut output, 44100.0, &context);

        // All output values should be non-negative (half-wave rectification)
        for sample in &output {
            assert!(*sample >= 0.0, "Sample {} should be non-negative", sample);
        }

        // First half of sine is positive (should be unchanged)
        for i in 0..8 {
            let phase = (i as f32 / 16.0) * 2.0 * PI;
            let expected = phase.sin();
            assert!(
                (output[i] - expected).abs() < 0.0001,
                "Sample {} should be close to original sine value {}",
                output[i],
                expected
            );
        }

        // Second half should be zero (negative part clipped)
        for i in 8..16 {
            assert!(
                output[i].abs() < 0.0001,
                "Sample {} should be zero (negative clipped)",
                output[i]
            );
        }
    }

    #[test]
    fn test_rectifier_dependencies() {
        let rect_node = RectifierNode::new(7, RectifierMode::FullWave);
        let deps = rect_node.input_nodes();

        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], 7);
    }

    #[test]
    fn test_rectifier_mixed_values_full_wave() {
        let mut rect_node = RectifierNode::new(0, RectifierMode::FullWave);

        let input = vec![1.0, -2.0, 3.0, -4.0, 0.0, -5.5];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 6];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            6,
            2.0,
            44100.0,
        );

        rect_node.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0);
        assert_eq!(output[1], 2.0);
        assert_eq!(output[2], 3.0);
        assert_eq!(output[3], 4.0);
        assert_eq!(output[4], 0.0);
        assert_eq!(output[5], 5.5);
    }

    #[test]
    fn test_rectifier_mixed_values_half_wave() {
        let mut rect_node = RectifierNode::new(0, RectifierMode::HalfWave);

        let input = vec![1.0, -2.0, 3.0, -4.0, 0.0, -5.5];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 6];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            6,
            2.0,
            44100.0,
        );

        rect_node.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0);
        assert_eq!(output[1], 0.0);  // Negative becomes zero
        assert_eq!(output[2], 3.0);
        assert_eq!(output[3], 0.0);  // Negative becomes zero
        assert_eq!(output[4], 0.0);
        assert_eq!(output[5], 0.0);  // Negative becomes zero
    }

    #[test]
    fn test_rectifier_with_constant() {
        let mut const_node = ConstantNode::new(-5.0);
        let mut rect_full = RectifierNode::new(0, RectifierMode::FullWave);
        let mut rect_half = RectifierNode::new(0, RectifierMode::HalfWave);

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

        // Test full-wave rectification
        let inputs = vec![buf.as_slice()];
        let mut output_full = vec![0.0; 512];
        rect_full.process_block(&inputs, &mut output_full, 44100.0, &context);

        // Every sample should be 5.0 (|-5.0|)
        for sample in &output_full {
            assert_eq!(*sample, 5.0);
        }

        // Test half-wave rectification
        let mut output_half = vec![0.0; 512];
        rect_half.process_block(&inputs, &mut output_half, 44100.0, &context);

        // Every sample should be 0.0 (max(-5.0, 0.0))
        for sample in &output_half {
            assert_eq!(*sample, 0.0);
        }
    }

    #[test]
    fn test_rectifier_zero_stays_zero() {
        let mut rect_full = RectifierNode::new(0, RectifierMode::FullWave);
        let mut rect_half = RectifierNode::new(0, RectifierMode::HalfWave);

        let input = vec![0.0, 0.0, 0.0, -0.0, 0.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 5];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            5,
            2.0,
            44100.0,
        );

        // Test full-wave
        rect_full.process_block(&inputs, &mut output, 44100.0, &context);
        for sample in &output {
            assert_eq!(*sample, 0.0);
        }

        // Test half-wave
        rect_half.process_block(&inputs, &mut output, 44100.0, &context);
        for sample in &output {
            assert_eq!(*sample, 0.0);
        }
    }
}
