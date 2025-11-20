/// XFade (crossfader) node - equal-power or linear crossfading
///
/// This node crossfades between two input signals based on a position parameter.
/// Supports both linear and equal-power crossfading curves.
///
/// # Crossfade Curves
///
/// ## Linear Crossfade
/// - Simple blend: `output = a * (1 - pos) + b * pos`
/// - Has a dip in perceived loudness at center (pos = 0.5)
/// - Computationally efficient
/// - Good for: automation, smooth transitions between similar signals
///
/// ## Equal-Power Crossfade
/// - Uses trigonometric functions to maintain constant energy
/// - Formula: `output = a * cos(pos * π/2) + b * sin(pos * π/2)`
/// - Maintains constant perceived loudness across the crossfade
/// - Good for: DJ-style mixing, transitioning between different sounds
/// - Used in professional mixers and DAWs
///
/// # Position Parameter Behavior
/// - `position = 0.0` → 100% signal A, 0% signal B
/// - `position = 0.5` → 50/50 blend (equal-power maintains constant energy here)
/// - `position = 1.0` → 0% signal A, 100% signal B
/// - Position is clamped to [0, 1] range for predictable behavior
///
/// # Use Cases
/// - DJ-style crossfading between tracks
/// - Smooth transitions in compositions
/// - Automated mix changes
/// - Pattern-controlled morphing between sounds
/// - Scene transitions in generative music
///
/// # Comparison with LerpNode
/// - [`LerpNode`](crate::nodes::lerp::LerpNode): Linear interpolation only, no clamping
/// - [`XFadeNode`]: Both linear and equal-power curves, clamped position
///
/// # Example
/// ```ignore
/// // DJ-style crossfade from track A to track B
/// let track_a = OscillatorNode::new(0, Waveform::Sine);     // NodeId 1
/// let track_b = OscillatorNode::new(2, Waveform::Saw);      // NodeId 3
/// let position = OscillatorNode::new(4, Waveform::Sine);    // NodeId 5 (slow LFO)
///
/// // Equal-power crossfade (maintains constant loudness)
/// let xfade = XFadeNode::new(1, 3, 5, XFadeCurve::EqualPower);
/// ```

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use std::f32::consts::PI;

/// Crossfade curve type
///
/// Determines how the two signals are blended.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum XFadeCurve {
    /// Linear crossfade: simple blend with energy dip at center
    ///
    /// Formula: `out = a * (1 - pos) + b * pos`
    Linear,

    /// Equal-power crossfade: maintains constant perceived loudness
    ///
    /// Formula: `out = a * cos(pos * π/2) + b * sin(pos * π/2)`
    ///
    /// This is the standard crossfade curve used in DJ mixers and professional audio.
    EqualPower,
}

/// XFade node: crossfader between two signals
///
/// Blends between input_a and input_b based on position, using either
/// linear or equal-power crossfade curves.
pub struct XFadeNode {
    /// First input signal (A)
    input_a: NodeId,
    /// Second input signal (B)
    input_b: NodeId,
    /// Position/crossfade amount (0.0 = A, 1.0 = B)
    position: NodeId,
    /// Crossfade curve type
    curve: XFadeCurve,
}

impl XFadeNode {
    /// Create a new crossfade node
    ///
    /// # Arguments
    /// * `input_a` - First input signal (NodeId)
    /// * `input_b` - Second input signal (NodeId)
    /// * `position` - Position/crossfade amount signal (NodeId), clamped to [0, 1]
    /// * `curve` - Crossfade curve type (Linear or EqualPower)
    ///
    /// # Example
    /// ```ignore
    /// // Equal-power crossfade (recommended for music)
    /// let xfade = XFadeNode::new(1, 2, 3, XFadeCurve::EqualPower);
    ///
    /// // Linear crossfade (simpler, faster)
    /// let xfade = XFadeNode::new(1, 2, 3, XFadeCurve::Linear);
    /// ```
    pub fn new(input_a: NodeId, input_b: NodeId, position: NodeId, curve: XFadeCurve) -> Self {
        Self {
            input_a,
            input_b,
            position,
            curve,
        }
    }

    /// Get the first input node ID
    pub fn input_a(&self) -> NodeId {
        self.input_a
    }

    /// Get the second input node ID
    pub fn input_b(&self) -> NodeId {
        self.input_b
    }

    /// Get the position input node ID
    pub fn position(&self) -> NodeId {
        self.position
    }

    /// Get the crossfade curve type
    pub fn curve(&self) -> XFadeCurve {
        self.curve
    }
}

impl AudioNode for XFadeNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert_eq!(
            inputs.len(),
            3,
            "XFadeNode expects 3 inputs (a, b, position), got {}",
            inputs.len()
        );

        let input_a = inputs[0];
        let input_b = inputs[1];
        let position = inputs[2];

        debug_assert_eq!(
            input_a.len(),
            output.len(),
            "Input A buffer length mismatch"
        );
        debug_assert_eq!(
            input_b.len(),
            output.len(),
            "Input B buffer length mismatch"
        );
        debug_assert_eq!(
            position.len(),
            output.len(),
            "Position buffer length mismatch"
        );

        match self.curve {
            XFadeCurve::Linear => {
                // Linear crossfade: output = a * (1 - pos) + b * pos
                for i in 0..output.len() {
                    let pos = position[i].clamp(0.0, 1.0);
                    output[i] = input_a[i] * (1.0 - pos) + input_b[i] * pos;
                }
            }
            XFadeCurve::EqualPower => {
                // Equal-power crossfade: output = a * cos(pos * π/2) + b * sin(pos * π/2)
                // This maintains constant energy/loudness across the crossfade
                for i in 0..output.len() {
                    let pos = position[i].clamp(0.0, 1.0);
                    let angle = pos * PI * 0.5; // Map [0, 1] to [0, π/2]
                    let gain_a = angle.cos();   // Starts at 1.0, ends at 0.0
                    let gain_b = angle.sin();   // Starts at 0.0, ends at 1.0
                    output[i] = input_a[i] * gain_a + input_b[i] * gain_b;
                }
            }
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input_a, self.input_b, self.position]
    }

    fn name(&self) -> &str {
        "XFadeNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_xfade_position_zero_returns_a() {
        // Position = 0.0 should return 100% signal A (both curves)
        let mut xfade_linear = XFadeNode::new(0, 1, 2, XFadeCurve::Linear);
        let mut xfade_power = XFadeNode::new(0, 1, 2, XFadeCurve::EqualPower);

        let input_a = vec![10.0, 20.0, 30.0, 40.0];
        let input_b = vec![100.0, 200.0, 300.0, 400.0];
        let position = vec![0.0, 0.0, 0.0, 0.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice(), position.as_slice()];

        let mut output_linear = vec![0.0; 4];
        let mut output_power = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        xfade_linear.process_block(&inputs, &mut output_linear, 44100.0, &context);
        xfade_power.process_block(&inputs, &mut output_power, 44100.0, &context);

        // Both should return input_a unchanged
        for i in 0..4 {
            assert_eq!(output_linear[i], input_a[i]);
            assert!((output_power[i] - input_a[i]).abs() < 0.001);
        }
    }

    #[test]
    fn test_xfade_position_one_returns_b() {
        // Position = 1.0 should return 100% signal B (both curves)
        let mut xfade_linear = XFadeNode::new(0, 1, 2, XFadeCurve::Linear);
        let mut xfade_power = XFadeNode::new(0, 1, 2, XFadeCurve::EqualPower);

        let input_a = vec![10.0, 20.0, 30.0, 40.0];
        let input_b = vec![100.0, 200.0, 300.0, 400.0];
        let position = vec![1.0, 1.0, 1.0, 1.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice(), position.as_slice()];

        let mut output_linear = vec![0.0; 4];
        let mut output_power = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        xfade_linear.process_block(&inputs, &mut output_linear, 44100.0, &context);
        xfade_power.process_block(&inputs, &mut output_power, 44100.0, &context);

        // Both should return input_b unchanged
        for i in 0..4 {
            assert_eq!(output_linear[i], input_b[i]);
            assert!((output_power[i] - input_b[i]).abs() < 0.001);
        }
    }

    #[test]
    fn test_xfade_linear_center_blend() {
        // Position = 0.5 should give 50/50 blend for linear
        let mut xfade = XFadeNode::new(0, 1, 2, XFadeCurve::Linear);

        let input_a = vec![10.0, 20.0, 30.0, 40.0];
        let input_b = vec![100.0, 200.0, 300.0, 400.0];
        let position = vec![0.5, 0.5, 0.5, 0.5];
        let inputs = vec![input_a.as_slice(), input_b.as_slice(), position.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        xfade.process_block(&inputs, &mut output, 44100.0, &context);

        // Expected: (10 + 100) / 2 = 55, etc.
        assert_eq!(output[0], 55.0);
        assert_eq!(output[1], 110.0);
        assert_eq!(output[2], 165.0);
        assert_eq!(output[3], 220.0);
    }

    #[test]
    fn test_xfade_equal_power_center_blend() {
        // Position = 0.5 should give equal-power blend
        // At pos=0.5, angle = π/4, so cos(π/4) = sin(π/4) = √2/2 ≈ 0.707
        let mut xfade = XFadeNode::new(0, 1, 2, XFadeCurve::EqualPower);

        let input_a = vec![10.0, 20.0, 30.0, 40.0];
        let input_b = vec![10.0, 20.0, 30.0, 40.0]; // Same as A for simplicity
        let position = vec![0.5, 0.5, 0.5, 0.5];
        let inputs = vec![input_a.as_slice(), input_b.as_slice(), position.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        xfade.process_block(&inputs, &mut output, 44100.0, &context);

        // When both inputs are the same, equal-power should maintain amplitude
        // cos(π/4) + sin(π/4) = 2 * √2/2 = √2 ≈ 1.414
        // So output ≈ input * 1.414
        let sqrt2 = 2.0_f32.sqrt();
        for i in 0..4 {
            let expected = input_a[i] * sqrt2;
            assert!((output[i] - expected).abs() < 0.01);
        }
    }

    #[test]
    fn test_xfade_linear_curve() {
        // Test linear crossfade at various positions
        let mut xfade = XFadeNode::new(0, 1, 2, XFadeCurve::Linear);

        let input_a = vec![0.0, 0.0, 0.0, 0.0];
        let input_b = vec![100.0, 100.0, 100.0, 100.0];
        let position = vec![0.0, 0.25, 0.5, 1.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice(), position.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        xfade.process_block(&inputs, &mut output, 44100.0, &context);

        // Linear interpolation from 0 to 100
        assert_eq!(output[0], 0.0);    // pos=0.0: 100% A
        assert_eq!(output[1], 25.0);   // pos=0.25: 75% A, 25% B
        assert_eq!(output[2], 50.0);   // pos=0.5: 50% A, 50% B
        assert_eq!(output[3], 100.0);  // pos=1.0: 100% B
    }

    #[test]
    fn test_xfade_equal_power_curve() {
        // Test equal-power crossfade characteristics
        let mut xfade = XFadeNode::new(0, 1, 2, XFadeCurve::EqualPower);

        let input_a = vec![1.0, 1.0, 1.0];
        let input_b = vec![0.0, 0.0, 0.0];
        let position = vec![0.0, 0.5, 1.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice(), position.as_slice()];

        let mut output = vec![0.0; 3];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            3,
            2.0,
            44100.0,
        );

        xfade.process_block(&inputs, &mut output, 44100.0, &context);

        // pos=0.0: cos(0) = 1.0, sin(0) = 0.0 → 1.0*1.0 + 0.0*0.0 = 1.0
        assert!((output[0] - 1.0).abs() < 0.001);

        // pos=0.5: cos(π/4) ≈ 0.707, sin(π/4) ≈ 0.707 → 1.0*0.707 + 0.0*0.707 ≈ 0.707
        let sqrt2_over_2 = (2.0_f32.sqrt()) / 2.0;
        assert!((output[1] - sqrt2_over_2).abs() < 0.001);

        // pos=1.0: cos(π/2) = 0.0, sin(π/2) = 1.0 → 1.0*0.0 + 0.0*1.0 = 0.0
        assert!(output[2].abs() < 0.001);
    }

    #[test]
    fn test_xfade_position_clamping() {
        // Test that position is clamped to [0, 1]
        let mut xfade = XFadeNode::new(0, 1, 2, XFadeCurve::Linear);

        let input_a = vec![0.0, 0.0, 0.0, 0.0];
        let input_b = vec![100.0, 100.0, 100.0, 100.0];

        // Test positions outside [0, 1]
        let position = vec![-0.5, -10.0, 1.5, 10.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice(), position.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        xfade.process_block(&inputs, &mut output, 44100.0, &context);

        // Negative positions should clamp to 0.0 (100% A)
        assert_eq!(output[0], 0.0);
        assert_eq!(output[1], 0.0);

        // Positions > 1.0 should clamp to 1.0 (100% B)
        assert_eq!(output[2], 100.0);
        assert_eq!(output[3], 100.0);
    }

    #[test]
    fn test_xfade_varying_position_per_sample() {
        // Test that position can vary per-sample
        let mut xfade = XFadeNode::new(0, 1, 2, XFadeCurve::Linear);

        let input_a = vec![10.0; 5];
        let input_b = vec![20.0; 5];
        let position = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice(), position.as_slice()];

        let mut output = vec![0.0; 5];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            5,
            2.0,
            44100.0,
        );

        xfade.process_block(&inputs, &mut output, 44100.0, &context);

        // Smooth transition from A to B
        assert_eq!(output[0], 10.0);   // 100% A
        assert_eq!(output[1], 12.5);   // 75% A, 25% B
        assert_eq!(output[2], 15.0);   // 50% A, 50% B
        assert_eq!(output[3], 17.5);   // 25% A, 75% B
        assert_eq!(output[4], 20.0);   // 100% B
    }

    #[test]
    fn test_xfade_dependencies() {
        // Test that input_nodes returns all three dependencies
        let xfade = XFadeNode::new(5, 10, 15, XFadeCurve::EqualPower);
        let deps = xfade.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 5);   // input_a
        assert_eq!(deps[1], 10);  // input_b
        assert_eq!(deps[2], 15);  // position
    }

    #[test]
    fn test_xfade_curve_difference() {
        // Demonstrate that equal-power has higher energy at center than linear
        let mut xfade_linear = XFadeNode::new(0, 1, 2, XFadeCurve::Linear);
        let mut xfade_power = XFadeNode::new(0, 1, 2, XFadeCurve::EqualPower);

        // Same signal on both inputs
        let input_a = vec![1.0; 4];
        let input_b = vec![1.0; 4];
        let position = vec![0.5; 4]; // Test at center
        let inputs = vec![input_a.as_slice(), input_b.as_slice(), position.as_slice()];

        let mut output_linear = vec![0.0; 4];
        let mut output_power = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        xfade_linear.process_block(&inputs, &mut output_linear, 44100.0, &context);
        xfade_power.process_block(&inputs, &mut output_power, 44100.0, &context);

        // Linear at center with same input: (1*0.5 + 1*0.5) = 1.0
        assert!((output_linear[0] - 1.0).abs() < 0.001);

        // Equal-power at center: cos(π/4) + sin(π/4) = √2 ≈ 1.414
        // This is why equal-power maintains constant perceived loudness!
        let sqrt2 = 2.0_f32.sqrt();
        assert!((output_power[0] - sqrt2).abs() < 0.01);

        // Equal-power should have higher amplitude at center
        assert!(output_power[0] > output_linear[0]);
    }

    #[test]
    fn test_xfade_with_constants() {
        // Integration test with ConstantNode
        let mut const_a = ConstantNode::new(50.0);
        let mut const_b = ConstantNode::new(150.0);
        let mut const_pos = ConstantNode::new(0.3);
        let mut xfade = XFadeNode::new(0, 1, 2, XFadeCurve::Linear);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        // Process constants first
        let mut buf_a = vec![0.0; 512];
        let mut buf_b = vec![0.0; 512];
        let mut buf_pos = vec![0.0; 512];

        const_a.process_block(&[], &mut buf_a, 44100.0, &context);
        const_b.process_block(&[], &mut buf_b, 44100.0, &context);
        const_pos.process_block(&[], &mut buf_pos, 44100.0, &context);

        // Now crossfade them
        let inputs = vec![buf_a.as_slice(), buf_b.as_slice(), buf_pos.as_slice()];
        let mut output = vec![0.0; 512];

        xfade.process_block(&inputs, &mut output, 44100.0, &context);

        // Expected: 50 * (1 - 0.3) + 150 * 0.3 = 50 * 0.7 + 150 * 0.3 = 35 + 45 = 80
        for sample in &output {
            assert_eq!(*sample, 80.0);
        }
    }

    #[test]
    fn test_xfade_dj_scenario() {
        // Realistic DJ crossfade scenario
        let mut xfade = XFadeNode::new(0, 1, 2, XFadeCurve::EqualPower);

        // Two different "tracks"
        let track_a = vec![100.0; 8];
        let track_b = vec![200.0; 8];

        // Crossfade from A to B over 8 samples
        let position = vec![0.0, 0.14, 0.29, 0.43, 0.57, 0.71, 0.86, 1.0];
        let inputs = vec![track_a.as_slice(), track_b.as_slice(), position.as_slice()];

        let mut output = vec![0.0; 8];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            8,
            2.0,
            44100.0,
        );

        xfade.process_block(&inputs, &mut output, 44100.0, &context);

        // Verify smooth crossfade (values should be monotonically increasing)
        for i in 1..8 {
            assert!(output[i] >= output[i - 1]);
        }

        // First sample should be close to track A
        assert!((output[0] - 100.0).abs() < 5.0);

        // Last sample should be close to track B
        assert!((output[7] - 200.0).abs() < 5.0);
    }

    #[test]
    fn test_xfade_negative_values() {
        // Test crossfade with negative input values
        let mut xfade = XFadeNode::new(0, 1, 2, XFadeCurve::Linear);

        let input_a = vec![-50.0, -40.0, -30.0, -20.0];
        let input_b = vec![50.0, 40.0, 30.0, 20.0];
        let position = vec![0.0, 0.25, 0.5, 1.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice(), position.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        xfade.process_block(&inputs, &mut output, 44100.0, &context);

        // pos=0.0: -50
        assert_eq!(output[0], -50.0);

        // pos=0.25: -40 * 0.75 + 40 * 0.25 = -30 + 10 = -20
        assert_eq!(output[1], -20.0);

        // pos=0.5: -30 * 0.5 + 30 * 0.5 = -15 + 15 = 0
        assert_eq!(output[2], 0.0);

        // pos=1.0: 20
        assert_eq!(output[3], 20.0);
    }

    #[test]
    fn test_xfade_getter_methods() {
        // Test getter methods
        let xfade = XFadeNode::new(5, 10, 15, XFadeCurve::EqualPower);

        assert_eq!(xfade.input_a(), 5);
        assert_eq!(xfade.input_b(), 10);
        assert_eq!(xfade.position(), 15);
        assert_eq!(xfade.curve(), XFadeCurve::EqualPower);
    }
}
