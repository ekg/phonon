/// Stereo splitter node - marker for stereo signal handling
///
/// This node represents the conceptual split of an interleaved stereo signal
/// into separate left and right channels. In the current architecture where
/// AudioNode returns a single output buffer, this node serves as:
///
/// 1. **Identity/Passthrough**: Passes stereo signal unchanged
/// 2. **Documentation Marker**: Signals that downstream nodes should interpret
///    the buffer as stereo (samples at even indices = left, odd = right)
/// 3. **Future Hook**: Foundation for true dual-output support
///
/// # Current Limitation
///
/// The AudioNode trait supports only one output buffer. To fully implement
/// stereo splitting, we need architectural changes:
///
/// **Option A**: Multi-output trait extension
/// ```ignore
/// trait MultiOutputAudioNode: AudioNode {
///     fn output_count(&self) -> usize;
///     fn process_multi_block(&mut self, inputs: &[&[f32]], outputs: &mut [&mut [f32]], ...);
/// }
/// ```
///
/// **Option B**: Stereo-aware buffer interpretation
/// ```ignore
/// // Current approach: Downstream nodes know to de-interleave
/// let stereo = splitter.process_block(...);  // Interleaved L,R,L,R,...
/// let left_sample = stereo[i * 2];
/// let right_sample = stereo[i * 2 + 1];
/// ```
///
/// For now, this node serves as an identity function with clear documentation
/// about its intended future behavior.
///
/// # Algorithm (Future Implementation)
///
/// When dual outputs are supported:
/// ```text
/// Input:  [L0, R0, L1, R1, L2, R2, ...]  (interleaved stereo)
/// Output1: [L0, L1, L2, ...]              (left channel)
/// Output2: [R0, R1, R2, ...]              (right channel)
/// ```
///
/// # Example Usage
///
/// ```ignore
/// // Create stereo signal (e.g., from reverb, chorus, stereo oscillator)
/// let stereo_signal = ReverbNode::new(...);  // Outputs interleaved stereo
///
/// // Split into L/R (currently just passes through)
/// let splitter = StereoSplitterNode::new(stereo_signal_id);
///
/// // Future: Access separate channels
/// let left_channel = splitter.left_output();
/// let right_channel = splitter.right_output();
/// ```

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Stereo splitter node - identity passthrough with stereo semantics
///
/// # Current Behavior
/// - Passes stereo input unchanged (identity function)
/// - Documents that signal should be interpreted as interleaved stereo
///
/// # Future Behavior (when multi-output support is added)
/// - Will de-interleave stereo signal into separate L/R buffers
/// - Downstream nodes will receive true mono L or R channels
///
/// # Example
/// ```ignore
/// // Mark that signal from reverb should be treated as stereo
/// let reverb = ReverbNode::new(...);          // NodeId 1 (stereo output)
/// let splitter = StereoSplitterNode::new(1);  // NodeId 2
/// ```
pub struct StereoSplitterNode {
    /// Interleaved stereo input signal (L,R,L,R,...)
    stereo_input: NodeId,
}

impl StereoSplitterNode {
    /// StereoSplitter - Identity passthrough for stereo signal semantics
    ///
    /// Marks that signal should be treated as interleaved stereo (L,R,L,R,...).
    /// Currently just passes signal through; true splitting awaits multi-output support.
    ///
    /// # Parameters
    /// - `stereo_input`: Interleaved stereo input signal
    ///
    /// # Example
    /// ```phonon
    /// ~reverb: sine 440 # reverb 0.5
    /// ~stereo: ~reverb # splitter
    /// out: ~stereo
    /// ```
    pub fn new(stereo_input: NodeId) -> Self {
        Self { stereo_input }
    }

    /// Get the stereo input node ID
    pub fn stereo_input(&self) -> NodeId {
        self.stereo_input
    }
}

impl AudioNode for StereoSplitterNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 1,
            "StereoSplitterNode requires 1 input (stereo signal), got {}",
            inputs.len()
        );

        let stereo_buf = inputs[0];

        debug_assert_eq!(
            stereo_buf.len(),
            output.len(),
            "Stereo buffer length mismatch"
        );

        // Identity passthrough - copy input to output unchanged
        // Future implementation will de-interleave into separate L/R buffers
        output.copy_from_slice(stereo_buf);
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.stereo_input]
    }

    fn name(&self) -> &str {
        "StereoSplitterNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_stereo_splitter_identity_passthrough() {
        // Test 1: Node passes signal unchanged (identity function)
        let mut splitter = StereoSplitterNode::new(0);

        let stereo_signal = vec![1.0, -0.5, 0.8, -0.3, 0.6, -0.2];
        let inputs = vec![stereo_signal.as_slice()];

        let mut output = vec![0.0; 6];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            6,
            2.0,
            44100.0,
        );

        splitter.process_block(&inputs, &mut output, 44100.0, &context);

        // Output should exactly match input
        for i in 0..6 {
            assert_eq!(
                output[i], stereo_signal[i],
                "Sample {} mismatch: got {}, expected {}",
                i, output[i], stereo_signal[i]
            );
        }
    }

    #[test]
    fn test_stereo_splitter_interleaved_interpretation() {
        // Test 2: Verify that signal can be interpreted as interleaved stereo
        let mut splitter = StereoSplitterNode::new(0);

        // Simulated interleaved stereo: L, R, L, R, L, R
        let stereo_signal = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let inputs = vec![stereo_signal.as_slice()];

        let mut output = vec![0.0; 6];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            6,
            2.0,
            44100.0,
        );

        splitter.process_block(&inputs, &mut output, 44100.0, &context);

        // Verify left channel (even indices)
        assert_eq!(output[0], 1.0);  // L0
        assert_eq!(output[2], 3.0);  // L1
        assert_eq!(output[4], 5.0);  // L2

        // Verify right channel (odd indices)
        assert_eq!(output[1], 2.0);  // R0
        assert_eq!(output[3], 4.0);  // R1
        assert_eq!(output[5], 6.0);  // R2
    }

    #[test]
    fn test_stereo_splitter_full_block() {
        // Test 3: Works with standard 512-sample blocks
        let mut splitter = StereoSplitterNode::new(0);

        let stereo_signal: Vec<f32> = (0..512)
            .map(|i| if i % 2 == 0 { 0.5 } else { -0.5 })
            .collect();

        let inputs = vec![stereo_signal.as_slice()];

        let mut output = vec![0.0; 512];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        splitter.process_block(&inputs, &mut output, 44100.0, &context);

        // Verify pattern maintained
        for i in 0..512 {
            let expected = if i % 2 == 0 { 0.5 } else { -0.5 };
            assert_eq!(
                output[i], expected,
                "Block sample {} mismatch: got {}, expected {}",
                i, output[i], expected
            );
        }
    }

    #[test]
    fn test_stereo_splitter_with_constant_input() {
        // Test 4: Works with constant node as input
        let mut const_node = ConstantNode::new(0.75);
        let mut splitter = StereoSplitterNode::new(0);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        // Process constant node first
        let mut const_buf = vec![0.0; 512];
        const_node.process_block(&[], &mut const_buf, 44100.0, &context);

        // Now process splitter
        let inputs = vec![const_buf.as_slice()];
        let mut output = vec![0.0; 512];

        splitter.process_block(&inputs, &mut output, 44100.0, &context);

        // All samples should be 0.75
        for sample in &output {
            assert_eq!(*sample, 0.75);
        }
    }

    #[test]
    fn test_stereo_splitter_preserves_stereo_width() {
        // Test 5: Verify that stereo signals maintain their left/right separation
        let mut splitter = StereoSplitterNode::new(0);

        // Clear stereo separation: left = 1.0, right = -1.0
        let stereo_signal: Vec<f32> = (0..100)
            .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
            .collect();

        let inputs = vec![stereo_signal.as_slice()];

        let mut output = vec![0.0; 100];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            100,
            2.0,
            44100.0,
        );

        splitter.process_block(&inputs, &mut output, 44100.0, &context);

        // Count left and right samples
        let left_samples: Vec<f32> = output.iter().step_by(2).copied().collect();
        let right_samples: Vec<f32> = output.iter().skip(1).step_by(2).copied().collect();

        // All left samples should be 1.0
        for sample in &left_samples {
            assert_eq!(*sample, 1.0);
        }

        // All right samples should be -1.0
        for sample in &right_samples {
            assert_eq!(*sample, -1.0);
        }
    }

    #[test]
    fn test_stereo_splitter_dependencies() {
        // Test 6: Verify node dependencies are correct
        let splitter = StereoSplitterNode::new(42);
        let deps = splitter.input_nodes();

        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], 42);
    }

    #[test]
    fn test_stereo_splitter_zero_signal() {
        // Test 7: Handles zero/silent signals correctly
        let mut splitter = StereoSplitterNode::new(0);

        let stereo_signal = vec![0.0; 512];
        let inputs = vec![stereo_signal.as_slice()];

        let mut output = vec![99.9; 512];  // Pre-fill with non-zero
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        splitter.process_block(&inputs, &mut output, 44100.0, &context);

        // All samples should be zero
        for sample in &output {
            assert_eq!(*sample, 0.0);
        }
    }

    #[test]
    fn test_stereo_splitter_alternating_polarity() {
        // Test 8: Handles alternating positive/negative correctly
        let mut splitter = StereoSplitterNode::new(0);

        let stereo_signal: Vec<f32> = (0..20)
            .map(|i| if i % 2 == 0 { (i as f32) / 10.0 } else { -(i as f32) / 10.0 })
            .collect();

        let inputs = vec![stereo_signal.as_slice()];

        let mut output = vec![0.0; 20];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            20,
            2.0,
            44100.0,
        );

        splitter.process_block(&inputs, &mut output, 44100.0, &context);

        // Verify alternating pattern preserved
        for i in 0..20 {
            let expected = if i % 2 == 0 { (i as f32) / 10.0 } else { -(i as f32) / 10.0 };
            assert_eq!(
                output[i], expected,
                "Alternating sample {} mismatch: got {}, expected {}",
                i, output[i], expected
            );
        }
    }

    #[test]
    fn test_stereo_splitter_documentation_example() {
        // Test 9: Example from documentation works correctly
        let mut splitter = StereoSplitterNode::new(1);

        // Simulate stereo signal from reverb (alternating values)
        let reverb_output: Vec<f32> = (0..512)
            .map(|i| ((i % 2) as f32) * 0.3 + 0.1)
            .collect();

        let inputs = vec![reverb_output.as_slice()];

        let mut output = vec![0.0; 512];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        splitter.process_block(&inputs, &mut output, 44100.0, &context);

        // Verify left channel is 0.1, right channel is 0.4
        for i in 0..512 {
            let expected = if i % 2 == 0 { 0.1 } else { 0.4 };
            assert_eq!(
                output[i], expected,
                "Example sample {} mismatch: got {}, expected {}",
                i, output[i], expected
            );
        }
    }

    #[test]
    fn test_stereo_splitter_name() {
        // Test 10: Node has correct name for debugging
        let splitter = StereoSplitterNode::new(0);
        assert_eq!(splitter.name(), "StereoSplitterNode");
    }
}
