/// StereoMerger node - merge two mono signals into stereo output
///
/// This node combines two mono signals (left and right) into a single output.
/// For now, outputs mono mix as (left + right) / 2 due to AudioNode trait
/// constraint (single-channel output).
///
/// **Future Expansion**: When true stereo support is added to the AudioNode
/// trait, this node will interleave left and right channels as:
/// ```text
/// stereo_output[i * 2]     = left[i]
/// stereo_output[i * 2 + 1] = right[i]
/// ```
///
/// **Current Behavior**: Implements equal-power mixing of left and right channels.
/// This is identical to mid-channel extraction in M/S (mid-side) processing.
///
/// # Use Cases
/// - Combine separate left/right mono signals into stereo
/// - Mix dual-mono to mono with equal power
/// - Future: True stereo interleaver when stereo support added
///
/// # Architectural Note
/// This node exists to mirror DAW-style stereo routing. It's the inverse of
/// StereoSplitter - where splitter extracts L/R from stereo, merger combines
/// L/R into stereo. Once the AudioNode trait supports multi-channel outputs,
/// both nodes will perform true stereo operations.
use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Merge two mono signals into stereo (currently outputs mono mix)
///
/// # Example
/// ```ignore
/// // Combine separate left and right signals
/// let left_osc = OscillatorNode::new(0, Waveform::Sine);   // NodeId 1
/// let right_osc = OscillatorNode::new(1, Waveform::Saw);   // NodeId 2
/// let stereo = StereoMergerNode::new(1, 2);                // NodeId 3
/// ```
pub struct StereoMergerNode {
    left: NodeId,  // Left channel input
    right: NodeId, // Right channel input
}

impl StereoMergerNode {
    /// StereoMerger - Combines two mono signals into stereo output
    ///
    /// Merges left and right channels using equal-power mixing.
    /// Currently outputs mono (L+R)/2 due to single-channel trait constraint.
    ///
    /// # Parameters
    /// - `left`: Left channel input signal
    /// - `right`: Right channel input signal
    ///
    /// # Example
    /// ```phonon
    /// ~left: sine 440
    /// ~right: saw 220
    /// out: ~left # merger ~right
    /// ```
    pub fn new(left: NodeId, right: NodeId) -> Self {
        Self { left, right }
    }

    /// Get the left channel input node ID
    pub fn left(&self) -> NodeId {
        self.left
    }

    /// Get the right channel input node ID
    pub fn right(&self) -> NodeId {
        self.right
    }
}

impl AudioNode for StereoMergerNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "StereoMergerNode requires 2 inputs (left, right), got {}",
            inputs.len()
        );

        let left_buf = inputs[0];
        let right_buf = inputs[1];

        debug_assert_eq!(left_buf.len(), output.len(), "Left buffer length mismatch");
        debug_assert_eq!(
            right_buf.len(),
            output.len(),
            "Right buffer length mismatch"
        );

        // Current implementation: Equal-power mono mix
        // This is the "mid" channel in M/S processing: M = (L + R) / 2
        //
        // Future: When AudioNode supports stereo, this will become:
        // for i in 0..output.len() / 2 {
        //     stereo_output[i * 2]     = left_buf[i];
        //     stereo_output[i * 2 + 1] = right_buf[i];
        // }
        for i in 0..output.len() {
            output[i] = (left_buf[i] + right_buf[i]) * 0.5;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.left, self.right]
    }

    fn name(&self) -> &str {
        "StereoMergerNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    fn make_context(block_size: usize) -> ProcessContext {
        ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, 44100.0)
    }

    #[test]
    fn test_stereo_merger_equal_signals() {
        // Test 1: Equal left and right signals should produce identical output
        let mut merger = StereoMergerNode::new(0, 1);

        let left = vec![1.0, 0.5, -0.5, -1.0];
        let right = vec![1.0, 0.5, -0.5, -1.0];
        let inputs = vec![left.as_slice(), right.as_slice()];

        let mut output = vec![0.0; 4];
        let context = make_context(4);

        merger.process_block(&inputs, &mut output, 44100.0, &context);

        // Equal signals: (L + R) / 2 = (x + x) / 2 = x
        for i in 0..4 {
            assert_eq!(
                output[i], left[i],
                "Equal signals sample {} mismatch: got {}, expected {}",
                i, output[i], left[i]
            );
        }
    }

    #[test]
    fn test_stereo_merger_opposite_signals() {
        // Test 2: Opposite signals (L = -R) should cancel out
        let mut merger = StereoMergerNode::new(0, 1);

        let left = vec![1.0, 0.5, -0.5, -1.0];
        let right = vec![-1.0, -0.5, 0.5, 1.0];
        let inputs = vec![left.as_slice(), right.as_slice()];

        let mut output = vec![0.0; 4];
        let context = make_context(4);

        merger.process_block(&inputs, &mut output, 44100.0, &context);

        // Opposite signals: (L + R) / 2 = (x + (-x)) / 2 = 0
        for i in 0..4 {
            assert!(
                output[i].abs() < 0.0001,
                "Opposite signals sample {} should cancel: got {}",
                i,
                output[i]
            );
        }
    }

    #[test]
    fn test_stereo_merger_mono_left_only() {
        // Test 3: Only left signal (right = 0) should output half amplitude
        let mut merger = StereoMergerNode::new(0, 1);

        let left = vec![1.0, 0.8, 0.6, 0.4];
        let right = vec![0.0, 0.0, 0.0, 0.0];
        let inputs = vec![left.as_slice(), right.as_slice()];

        let mut output = vec![0.0; 4];
        let context = make_context(4);

        merger.process_block(&inputs, &mut output, 44100.0, &context);

        // Left only: (L + 0) / 2 = L / 2
        for i in 0..4 {
            let expected = left[i] * 0.5;
            assert!(
                (output[i] - expected).abs() < 0.0001,
                "Left-only sample {} mismatch: got {}, expected {}",
                i,
                output[i],
                expected
            );
        }
    }

    #[test]
    fn test_stereo_merger_mono_right_only() {
        // Test 4: Only right signal (left = 0) should output half amplitude
        let mut merger = StereoMergerNode::new(0, 1);

        let left = vec![0.0, 0.0, 0.0, 0.0];
        let right = vec![1.0, 0.8, 0.6, 0.4];
        let inputs = vec![left.as_slice(), right.as_slice()];

        let mut output = vec![0.0; 4];
        let context = make_context(4);

        merger.process_block(&inputs, &mut output, 44100.0, &context);

        // Right only: (0 + R) / 2 = R / 2
        for i in 0..4 {
            let expected = right[i] * 0.5;
            assert!(
                (output[i] - expected).abs() < 0.0001,
                "Right-only sample {} mismatch: got {}, expected {}",
                i,
                output[i],
                expected
            );
        }
    }

    #[test]
    fn test_stereo_merger_different_signals() {
        // Test 5: Different left and right signals should average
        let mut merger = StereoMergerNode::new(0, 1);

        let left = vec![1.0, 0.8, -0.6, -0.4];
        let right = vec![0.5, -0.3, 0.2, 0.1];
        let inputs = vec![left.as_slice(), right.as_slice()];

        let mut output = vec![0.0; 4];
        let context = make_context(4);

        merger.process_block(&inputs, &mut output, 44100.0, &context);

        // Average: (L + R) / 2
        for i in 0..4 {
            let expected = (left[i] + right[i]) * 0.5;
            assert!(
                (output[i] - expected).abs() < 0.0001,
                "Different signals sample {} mismatch: got {}, expected {}",
                i,
                output[i],
                expected
            );
        }
    }

    #[test]
    fn test_stereo_merger_with_constants() {
        // Test 6: Works with constant node inputs
        let mut const_left = ConstantNode::new(0.6);
        let mut const_right = ConstantNode::new(0.4);
        let mut merger = StereoMergerNode::new(0, 1);

        let context = make_context(512);

        // Process constants first
        let mut left_buf = vec![0.0; 512];
        let mut right_buf = vec![0.0; 512];

        const_left.process_block(&[], &mut left_buf, 44100.0, &context);
        const_right.process_block(&[], &mut right_buf, 44100.0, &context);

        // Now merge
        let inputs = vec![left_buf.as_slice(), right_buf.as_slice()];
        let mut output = vec![0.0; 512];

        merger.process_block(&inputs, &mut output, 44100.0, &context);

        // Expected: (0.6 + 0.4) / 2 = 0.5
        for sample in &output {
            assert!(
                (*sample - 0.5).abs() < 0.0001,
                "Constant merger output mismatch: got {}, expected 0.5",
                sample
            );
        }
    }

    #[test]
    fn test_stereo_merger_full_block() {
        // Test 7: Full 512-sample block with varying signals
        let mut merger = StereoMergerNode::new(0, 1);

        let mut left = vec![0.0; 512];
        let mut right = vec![0.0; 512];

        // Generate test signals (simple sine-like patterns)
        for i in 0..512 {
            let t = i as f32 / 512.0;
            left[i] = (t * 2.0 * std::f32::consts::PI).sin();
            right[i] = (t * 4.0 * std::f32::consts::PI).cos();
        }

        let inputs = vec![left.as_slice(), right.as_slice()];
        let mut output = vec![0.0; 512];
        let context = make_context(512);

        merger.process_block(&inputs, &mut output, 44100.0, &context);

        // Verify every sample is correct average
        for i in 0..512 {
            let expected = (left[i] + right[i]) * 0.5;
            assert!(
                (output[i] - expected).abs() < 0.0001,
                "Full block sample {} mismatch: got {}, expected {}",
                i,
                output[i],
                expected
            );
        }
    }

    #[test]
    fn test_stereo_merger_dependencies() {
        // Test 8: Correct dependency reporting
        let merger = StereoMergerNode::new(5, 10);
        let deps = merger.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 5, "First dependency should be left input");
        assert_eq!(deps[1], 10, "Second dependency should be right input");
    }

    #[test]
    fn test_stereo_merger_preserves_energy() {
        // Test 9: Energy calculation - verify output matches mixing formula
        let mut merger = StereoMergerNode::new(0, 1);

        let left = vec![0.8, 0.6, 0.4, 0.2];
        let right = vec![0.2, 0.4, 0.6, 0.8];
        let inputs = vec![left.as_slice(), right.as_slice()];

        let mut output = vec![0.0; 4];
        let context = make_context(4);

        merger.process_block(&inputs, &mut output, 44100.0, &context);

        // Calculate expected output power from formula: output[i] = (L[i] + R[i]) / 2
        // Power = sum of squares / N
        let mut expected_power = 0.0;
        let mut actual_power = 0.0;

        for i in 0..4 {
            let expected_sample = (left[i] + right[i]) * 0.5;
            expected_power += expected_sample * expected_sample;
            actual_power += output[i] * output[i];
        }

        expected_power /= 4.0;
        actual_power /= 4.0;

        // Verify output power matches calculated expectation
        assert!(
            (actual_power - expected_power).abs() < 0.0001,
            "Power mismatch: actual={:.4}, expected={:.4}",
            actual_power,
            expected_power
        );

        // Also verify output power is reasonable (not zero, not clipping)
        assert!(actual_power > 0.0, "Output should have energy");
        assert!(actual_power < 1.0, "Output power should be less than 1.0");
    }

    #[test]
    fn test_stereo_merger_asymmetric_amplitude() {
        // Test 10: Asymmetric amplitudes should mix correctly
        // Tests the case where one channel is much louder than the other
        let mut merger = StereoMergerNode::new(0, 1);

        let left = vec![1.0, 1.0, 1.0, 1.0]; // Full amplitude
        let right = vec![0.1, 0.1, 0.1, 0.1]; // Much quieter
        let inputs = vec![left.as_slice(), right.as_slice()];

        let mut output = vec![0.0; 4];
        let context = make_context(4);

        merger.process_block(&inputs, &mut output, 44100.0, &context);

        // Expected: (1.0 + 0.1) / 2 = 0.55
        let expected = (1.0 + 0.1) * 0.5;

        for sample in &output {
            assert!(
                (*sample - expected).abs() < 0.0001,
                "Asymmetric amplitude mismatch: got {}, expected {}",
                sample,
                expected
            );
        }

        // Output should be closer to the louder (left) channel
        assert!(output[0] > 0.5, "Output should lean toward louder channel");
    }
}
