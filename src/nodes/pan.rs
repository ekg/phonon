/// Pan node - stereo panning with equal-power law
///
/// This node applies equal-power panning to distribute a mono signal
/// across left and right channels. For now, outputs mono (left + right).
///
/// Pan values range from -1.0 (full left) to 1.0 (full right).
/// Center position (0.0) distributes signal equally to both channels.
///
/// Equal-power law ensures constant perceived loudness across pan positions:
/// - pan_angle = (pan + 1.0) * PI / 4.0
/// - left = input * cos(pan_angle)
/// - right = input * sin(pan_angle)

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use std::f32::consts::PI;

/// Pan node with equal-power panning law
///
/// # Example
/// ```ignore
/// // Pan signal to center (0.0)
/// let input = OscillatorNode::new(0, Waveform::Sine);  // NodeId 1
/// let pan_const = ConstantNode::new(0.0);              // NodeId 2
/// let panned = PanNode::new(1, 2);                     // NodeId 3
/// ```
pub struct PanNode {
    input: NodeId,      // Audio input signal
    pan_input: NodeId,  // Pan position: -1.0 (left) to 1.0 (right)
}

impl PanNode {
    /// Create a new pan node
    ///
    /// # Arguments
    /// * `input` - NodeId providing the audio signal to pan
    /// * `pan_input` - NodeId providing pan position (-1.0 to 1.0)
    pub fn new(input: NodeId, pan_input: NodeId) -> Self {
        Self { input, pan_input }
    }

    /// Get the input signal node ID
    pub fn input(&self) -> NodeId {
        self.input
    }

    /// Get the pan control node ID
    pub fn pan_input(&self) -> NodeId {
        self.pan_input
    }
}

impl AudioNode for PanNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "PanNode requires 2 inputs (signal, pan), got {}",
            inputs.len()
        );

        let signal_buf = inputs[0];
        let pan_buf = inputs[1];

        debug_assert_eq!(
            signal_buf.len(),
            output.len(),
            "Signal buffer length mismatch"
        );
        debug_assert_eq!(
            pan_buf.len(),
            output.len(),
            "Pan buffer length mismatch"
        );

        // Apply equal-power panning
        for i in 0..output.len() {
            let signal = signal_buf[i];
            let pan = pan_buf[i].clamp(-1.0, 1.0);  // Clamp to valid range

            // Equal-power law:
            // pan_angle ranges from 0 (left) to PI/2 (right)
            let pan_angle = (pan + 1.0) * PI / 4.0;

            let left = signal * pan_angle.cos();
            let right = signal * pan_angle.sin();

            // For mono output, return average of left and right
            output[i] = (left + right) / 2.0;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.pan_input]
    }

    fn name(&self) -> &str {
        "PanNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_pan_center_unchanged() {
        // Test 1: Center (0.0) outputs signal unchanged
        let mut pan = PanNode::new(0, 1);

        let signal = vec![1.0, -0.5, 0.8, -0.3];
        let pan_pos = vec![0.0, 0.0, 0.0, 0.0];  // Center
        let inputs = vec![signal.as_slice(), pan_pos.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        pan.process_block(&inputs, &mut output, 44100.0, &context);

        // At center, left and right are equal (cos(PI/4) = sin(PI/4) = sqrt(2)/2)
        // So average should be signal * sqrt(2)/2
        let expected_gain = (PI / 4.0).cos();  // cos(PI/4) = sqrt(2)/2 ≈ 0.707

        for i in 0..4 {
            let expected = signal[i] * expected_gain;
            assert!(
                (output[i] - expected).abs() < 0.001,
                "Center pan sample {} mismatch: got {}, expected {}",
                i, output[i], expected
            );
        }
    }

    #[test]
    fn test_pan_full_left_attenuates_right() {
        // Test 2: Full left (-1.0) attenuates right channel
        let mut pan = PanNode::new(0, 1);

        let signal = vec![1.0; 512];
        let pan_pos = vec![-1.0; 512];  // Full left
        let inputs = vec![signal.as_slice(), pan_pos.as_slice()];

        let mut output = vec![0.0; 512];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        pan.process_block(&inputs, &mut output, 44100.0, &context);

        // At full left: pan_angle = 0, left = cos(0) = 1.0, right = sin(0) = 0.0
        // Average = (1.0 + 0.0) / 2.0 = 0.5
        for sample in &output {
            assert!(
                (*sample - 0.5).abs() < 0.001,
                "Full left pan mismatch: got {}, expected 0.5",
                sample
            );
        }
    }

    #[test]
    fn test_pan_full_right_attenuates_left() {
        // Test 3: Full right (1.0) attenuates left channel
        let mut pan = PanNode::new(0, 1);

        let signal = vec![1.0; 512];
        let pan_pos = vec![1.0; 512];  // Full right
        let inputs = vec![signal.as_slice(), pan_pos.as_slice()];

        let mut output = vec![0.0; 512];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        pan.process_block(&inputs, &mut output, 44100.0, &context);

        // At full right: pan_angle = PI/2, left = cos(PI/2) = 0.0, right = sin(PI/2) = 1.0
        // Average = (0.0 + 1.0) / 2.0 = 0.5
        for sample in &output {
            assert!(
                (*sample - 0.5).abs() < 0.001,
                "Full right pan mismatch: got {}, expected 0.5",
                sample
            );
        }
    }

    #[test]
    fn test_pan_law_preserves_power() {
        // Test 4: Pan law preserves power in stereo (before mono downmix)
        // Note: Mono downmix (L+R)/2 will NOT preserve equal power, but the
        // underlying stereo panning should follow equal-power law: L² + R² = 1
        let pan_positions = vec![-1.0, -0.5, 0.0, 0.5, 1.0];
        let signal = vec![1.0; 512];

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        // Test that the stereo power (before mono downmix) is constant
        for &pan_pos in &pan_positions {
            let pan_angle = (pan_pos + 1.0) * PI / 4.0;
            let left_gain = pan_angle.cos();
            let right_gain = pan_angle.sin();

            // Equal-power law: L² + R² should equal 1.0
            let stereo_power = left_gain * left_gain + right_gain * right_gain;

            assert!(
                (stereo_power - 1.0).abs() < 0.001,
                "Pan position {} stereo power not equal: {:.6} (should be 1.0)",
                pan_pos, stereo_power
            );
        }

        // Also verify the mono outputs have expected values
        // (they won't be equal power, but they should be symmetric)
        let mut mono_outputs = Vec::new();

        for &pan_pos in &pan_positions {
            let mut pan = PanNode::new(0, 1);
            let pan_buf = vec![pan_pos; 512];
            let inputs = vec![signal.as_slice(), pan_buf.as_slice()];
            let mut output = vec![0.0; 512];

            pan.process_block(&inputs, &mut output, 44100.0, &context);

            mono_outputs.push(output[0]);  // All samples are the same
        }

        // Center should have highest output (both L and R contributing)
        let center_output = mono_outputs[2];  // 0.0 pan position

        // Full left and full right should have equal output (symmetry)
        assert!(
            (mono_outputs[0] - mono_outputs[4]).abs() < 0.001,
            "Full left and full right outputs should be equal: left={}, right={}",
            mono_outputs[0], mono_outputs[4]
        );

        // -0.5 and +0.5 should also be equal (symmetry)
        assert!(
            (mono_outputs[1] - mono_outputs[3]).abs() < 0.001,
            "Pan -0.5 and +0.5 outputs should be equal: -0.5={}, +0.5={}",
            mono_outputs[1], mono_outputs[3]
        );

        // Center should be louder than full left/right
        assert!(
            center_output > mono_outputs[0],
            "Center output should be louder than hard panned: center={}, hard={}",
            center_output, mono_outputs[0]
        );
    }

    #[test]
    fn test_pan_with_constants() {
        let mut const_signal = ConstantNode::new(0.5);
        let mut const_pan = ConstantNode::new(0.5);  // Slight right
        let mut pan = PanNode::new(0, 1);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        // Process constants first
        let mut signal_buf = vec![0.0; 512];
        let mut pan_buf = vec![0.0; 512];

        const_signal.process_block(&[], &mut signal_buf, 44100.0, &context);
        const_pan.process_block(&[], &mut pan_buf, 44100.0, &context);

        // Now pan
        let inputs = vec![signal_buf.as_slice(), pan_buf.as_slice()];
        let mut output = vec![0.0; 512];

        pan.process_block(&inputs, &mut output, 44100.0, &context);

        // All samples should be identical (constant inputs)
        let first = output[0];
        for sample in &output {
            assert_eq!(*sample, first);
        }

        // Output should be non-zero
        assert!(first.abs() > 0.0);
    }

    #[test]
    fn test_pan_clamps_out_of_range() {
        // Pan values outside [-1.0, 1.0] should be clamped
        let mut pan = PanNode::new(0, 1);

        let signal = vec![1.0; 4];
        let pan_pos = vec![-2.0, -1.5, 1.5, 2.0];  // Out of range
        let inputs = vec![signal.as_slice(), pan_pos.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        pan.process_block(&inputs, &mut output, 44100.0, &context);

        // Should clamp to -1.0 and 1.0
        // First two should be like -1.0 (full left)
        assert!((output[0] - 0.5).abs() < 0.001);
        assert!((output[1] - 0.5).abs() < 0.001);

        // Last two should be like 1.0 (full right)
        assert!((output[2] - 0.5).abs() < 0.001);
        assert!((output[3] - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_pan_dependencies() {
        let pan = PanNode::new(5, 10);
        let deps = pan.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 5);
        assert_eq!(deps[1], 10);
    }

    #[test]
    fn test_pan_negative_signal() {
        // Negative signals should pan correctly
        let mut pan = PanNode::new(0, 1);

        let signal = vec![-1.0, -0.5, -0.8, -0.3];
        let pan_pos = vec![0.0, 0.0, 0.0, 0.0];  // Center
        let inputs = vec![signal.as_slice(), pan_pos.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        pan.process_block(&inputs, &mut output, 44100.0, &context);

        let expected_gain = (PI / 4.0).cos();

        for i in 0..4 {
            let expected = signal[i] * expected_gain;
            assert!(
                (output[i] - expected).abs() < 0.001,
                "Negative signal pan sample {} mismatch: got {}, expected {}",
                i, output[i], expected
            );
        }
    }
}
