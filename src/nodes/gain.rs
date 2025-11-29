/// Gain node - multiplies input signal by gain amount
///
/// This is a simplified multiplication node optimized for the common use case
/// of applying a gain/volume control to a signal.
/// Output[i] = Input[i] * Gain[i] for all samples.
use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Gain node: out = input * gain
///
/// # Example
/// ```ignore
/// // Apply 0.5 gain (halve the volume) to an oscillator
/// let osc = OscillatorNode::new(0, Waveform::Sine);  // NodeId 0
/// let gain_amount = ConstantNode::new(0.5);          // NodeId 1
/// let gain = GainNode::new(0, 1);                    // NodeId 2
/// // Output will be oscillator at half amplitude
/// ```
pub struct GainNode {
    input: NodeId,
    gain_input: NodeId,
}

impl GainNode {
    /// Gain - Multiplies signal by gain amount for volume control
    ///
    /// Simple multiplication node optimized for amplitude control,
    /// with pattern-controllable gain modulation.
    ///
    /// # Parameters
    /// - `input`: NodeId providing signal to amplify
    /// - `gain_input`: NodeId providing gain amount (default: 1.0)
    ///
    /// # Example
    /// ```phonon
    /// ~signal: sine 440
    /// ~quiet: ~signal # gain 0.5
    /// ```
    pub fn new(input: NodeId, gain_input: NodeId) -> Self {
        Self { input, gain_input }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }

    /// Get the gain input node ID
    pub fn gain_input(&self) -> NodeId {
        self.gain_input
    }
}

impl AudioNode for GainNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "GainNode requires 2 inputs (signal + gain), got {}",
            inputs.len()
        );

        let signal = inputs[0];
        let gain = inputs[1];

        debug_assert_eq!(signal.len(), output.len(), "Signal input length mismatch");
        debug_assert_eq!(gain.len(), output.len(), "Gain input length mismatch");

        // Vectorized multiplication: output = signal * gain
        for i in 0..output.len() {
            output[i] = signal[i] * gain[i];
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.gain_input]
    }

    fn name(&self) -> &str {
        "GainNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_gain_node_unity() {
        // Unity gain (1.0) passes signal unchanged
        let mut gain = GainNode::new(0, 1);

        let input_signal = vec![0.5, -0.3, 0.8, -1.0];
        let gain_value = vec![1.0, 1.0, 1.0, 1.0];
        let inputs = vec![input_signal.as_slice(), gain_value.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        gain.process_block(&inputs, &mut output, 44100.0, &context);

        // Signal should pass through unchanged
        assert_eq!(output[0], 0.5);
        assert_eq!(output[1], -0.3);
        assert_eq!(output[2], 0.8);
        assert_eq!(output[3], -1.0);
    }

    #[test]
    fn test_gain_node_half_amplitude() {
        // Gain 0.5 halves amplitude
        let mut gain = GainNode::new(0, 1);

        let input_signal = vec![1.0, -1.0, 0.8, -0.6];
        let gain_value = vec![0.5, 0.5, 0.5, 0.5];
        let inputs = vec![input_signal.as_slice(), gain_value.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        gain.process_block(&inputs, &mut output, 44100.0, &context);

        // Amplitude should be halved
        assert_eq!(output[0], 0.5); // 1.0 * 0.5
        assert_eq!(output[1], -0.5); // -1.0 * 0.5
        assert_eq!(output[2], 0.4); // 0.8 * 0.5
        assert_eq!(output[3], -0.3); // -0.6 * 0.5
    }

    #[test]
    fn test_gain_node_double_amplitude() {
        // Gain 2.0 doubles amplitude
        let mut gain = GainNode::new(0, 1);

        let input_signal = vec![0.25, -0.5, 0.3, -0.4];
        let gain_value = vec![2.0, 2.0, 2.0, 2.0];
        let inputs = vec![input_signal.as_slice(), gain_value.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        gain.process_block(&inputs, &mut output, 44100.0, &context);

        // Amplitude should be doubled
        assert_eq!(output[0], 0.5); // 0.25 * 2.0
        assert_eq!(output[1], -1.0); // -0.5 * 2.0
        assert_eq!(output[2], 0.6); // 0.3 * 2.0
        assert_eq!(output[3], -0.8); // -0.4 * 2.0
    }

    #[test]
    fn test_gain_node_negative_inverts() {
        // Negative gain inverts signal
        let mut gain = GainNode::new(0, 1);

        let input_signal = vec![0.5, -0.3, 0.8, -0.2];
        let gain_value = vec![-1.0, -1.0, -1.0, -1.0];
        let inputs = vec![input_signal.as_slice(), gain_value.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        gain.process_block(&inputs, &mut output, 44100.0, &context);

        // Signal should be inverted
        assert_eq!(output[0], -0.5); // 0.5 * -1.0
        assert_eq!(output[1], 0.3); // -0.3 * -1.0
        assert_eq!(output[2], -0.8); // 0.8 * -1.0
        assert_eq!(output[3], 0.2); // -0.2 * -1.0
    }

    #[test]
    fn test_gain_node_with_constants() {
        // Integration test with ConstantNode
        let mut signal_node = ConstantNode::new(0.5);
        let mut gain_node = ConstantNode::new(3.0);
        let mut gain = GainNode::new(0, 1);

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        // Process constants first
        let mut signal_buf = vec![0.0; 512];
        let mut gain_buf = vec![0.0; 512];

        signal_node.process_block(&[], &mut signal_buf, 44100.0, &context);
        gain_node.process_block(&[], &mut gain_buf, 44100.0, &context);

        // Apply gain
        let inputs = vec![signal_buf.as_slice(), gain_buf.as_slice()];
        let mut output = vec![0.0; 512];

        gain.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be 1.5 (0.5 * 3.0)
        for sample in &output {
            assert_eq!(*sample, 1.5);
        }
    }

    #[test]
    fn test_gain_node_variable_gain() {
        // Test with varying gain values (like an LFO modulation)
        let mut gain = GainNode::new(0, 1);

        let input_signal = vec![1.0, 1.0, 1.0, 1.0];
        let gain_value = vec![0.0, 0.5, 1.0, 2.0]; // Varying gain
        let inputs = vec![input_signal.as_slice(), gain_value.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        gain.process_block(&inputs, &mut output, 44100.0, &context);

        // Output should follow gain curve
        assert_eq!(output[0], 0.0); // 1.0 * 0.0
        assert_eq!(output[1], 0.5); // 1.0 * 0.5
        assert_eq!(output[2], 1.0); // 1.0 * 1.0
        assert_eq!(output[3], 2.0); // 1.0 * 2.0
    }

    #[test]
    fn test_gain_node_dependencies() {
        let gain = GainNode::new(5, 10);
        let deps = gain.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 5);
        assert_eq!(deps[1], 10);
    }

    #[test]
    fn test_gain_node_zero_gain_silences() {
        // Zero gain should produce silence
        let mut gain = GainNode::new(0, 1);

        let input_signal = vec![0.8, -0.6, 1.0, -1.0];
        let gain_value = vec![0.0, 0.0, 0.0, 0.0];
        let inputs = vec![input_signal.as_slice(), gain_value.as_slice()];

        let mut output = vec![999.0; 4]; // Initialize with non-zero to verify it gets zeroed
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        gain.process_block(&inputs, &mut output, 44100.0, &context);

        // All output should be zero
        for sample in &output {
            assert_eq!(*sample, 0.0);
        }
    }
}
