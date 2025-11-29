/// Gate node - threshold gate that passes signal above threshold
///
/// This is a dynamics processing node that acts as a gate. It passes the signal
/// when the absolute value is above the threshold, otherwise outputs 0.0.
/// Output[i] = if |Input[i]| > Threshold[i] { Input[i] } else { 0.0 }
use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Gate node: out = if |input| > threshold { input } else { 0.0 }
///
/// # Example
/// ```ignore
/// // Gate a noisy signal, only passing peaks above 0.5
/// let noisy_signal = NoiseNode::new();              // NodeId 0
/// let threshold_value = ConstantNode::new(0.5);     // NodeId 1
/// let gate = GateNode::new(0, 1);                   // NodeId 2
/// // Only samples with |amplitude| > 0.5 pass through
/// ```
pub struct GateNode {
    input: NodeId,
    threshold_input: NodeId,
}

impl GateNode {
    /// Gate - Threshold gate that passes signal above threshold
    ///
    /// Dynamics processor that passes signal when exceeding threshold, mutes below,
    /// useful for noise gates and trigger detection.
    ///
    /// # Parameters
    /// - `input`: NodeId providing signal to gate
    /// - `threshold_input`: NodeId providing threshold in linear amplitude (default: 0.1)
    ///
    /// # Example
    /// ```phonon
    /// ~noisy: sine 440 + white_noise 0.1
    /// ~clean: ~noisy # gate 0.5
    /// ```
    pub fn new(input: NodeId, threshold_input: NodeId) -> Self {
        Self {
            input,
            threshold_input,
        }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }

    /// Get the threshold input node ID
    pub fn threshold_input(&self) -> NodeId {
        self.threshold_input
    }
}

impl AudioNode for GateNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "GateNode requires 2 inputs (signal + threshold), got {}",
            inputs.len()
        );

        let signal = inputs[0];
        let threshold = inputs[1];

        debug_assert_eq!(signal.len(), output.len(), "Signal input length mismatch");
        debug_assert_eq!(
            threshold.len(),
            output.len(),
            "Threshold input length mismatch"
        );

        // Gate processing: pass signal if |signal| > threshold, else 0.0
        for i in 0..output.len() {
            output[i] = if signal[i].abs() > threshold[i] {
                signal[i]
            } else {
                0.0
            };
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.threshold_input]
    }

    fn name(&self) -> &str {
        "GateNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_gate_above_threshold_passes() {
        // Signal above threshold passes through unchanged
        let mut gate = GateNode::new(0, 1);

        let input_signal = vec![0.6, -0.8, 0.9, -1.0];
        let threshold_value = vec![0.5, 0.5, 0.5, 0.5];
        let inputs = vec![input_signal.as_slice(), threshold_value.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        gate.process_block(&inputs, &mut output, 44100.0, &context);

        // All values have |value| > 0.5, so should pass through
        assert_eq!(output[0], 0.6);
        assert_eq!(output[1], -0.8);
        assert_eq!(output[2], 0.9);
        assert_eq!(output[3], -1.0);
    }

    #[test]
    fn test_gate_below_threshold_blocked() {
        // Signal below threshold outputs 0.0
        let mut gate = GateNode::new(0, 1);

        let input_signal = vec![0.3, -0.2, 0.4, -0.1];
        let threshold_value = vec![0.5, 0.5, 0.5, 0.5];
        let inputs = vec![input_signal.as_slice(), threshold_value.as_slice()];

        let mut output = vec![999.0; 4]; // Initialize with non-zero to verify zeroing
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        gate.process_block(&inputs, &mut output, 44100.0, &context);

        // All values have |value| <= 0.5, so should be zeroed
        assert_eq!(output[0], 0.0);
        assert_eq!(output[1], 0.0);
        assert_eq!(output[2], 0.0);
        assert_eq!(output[3], 0.0);
    }

    #[test]
    fn test_gate_at_threshold_blocked() {
        // Signal exactly at threshold is blocked (> not >=)
        let mut gate = GateNode::new(0, 1);

        let input_signal = vec![0.5, -0.5, 0.5, -0.5];
        let threshold_value = vec![0.5, 0.5, 0.5, 0.5];
        let inputs = vec![input_signal.as_slice(), threshold_value.as_slice()];

        let mut output = vec![999.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        gate.process_block(&inputs, &mut output, 44100.0, &context);

        // |0.5| == 0.5, not > 0.5, so should be blocked
        for sample in &output {
            assert_eq!(*sample, 0.0);
        }
    }

    #[test]
    fn test_gate_negative_values() {
        // Gate uses absolute value, so negative signals work correctly
        let mut gate = GateNode::new(0, 1);

        let input_signal = vec![-0.8, -0.3, -0.6, -0.1];
        let threshold_value = vec![0.5, 0.5, 0.5, 0.5];
        let inputs = vec![input_signal.as_slice(), threshold_value.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        gate.process_block(&inputs, &mut output, 44100.0, &context);

        // |-0.8| = 0.8 > 0.5: pass (preserve negative)
        assert_eq!(output[0], -0.8);

        // |-0.3| = 0.3 < 0.5: block
        assert_eq!(output[1], 0.0);

        // |-0.6| = 0.6 > 0.5: pass (preserve negative)
        assert_eq!(output[2], -0.6);

        // |-0.1| = 0.1 < 0.5: block
        assert_eq!(output[3], 0.0);
    }

    #[test]
    fn test_gate_varying_threshold() {
        // Test with varying threshold values
        let mut gate = GateNode::new(0, 1);

        let input_signal = vec![0.5, 0.5, 0.5, 0.5];
        let threshold_value = vec![0.3, 0.5, 0.6, 0.1]; // Varying threshold
        let inputs = vec![input_signal.as_slice(), threshold_value.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        gate.process_block(&inputs, &mut output, 44100.0, &context);

        // 0.5 > 0.3: pass
        assert_eq!(output[0], 0.5);

        // 0.5 == 0.5: block
        assert_eq!(output[1], 0.0);

        // 0.5 < 0.6: block
        assert_eq!(output[2], 0.0);

        // 0.5 > 0.1: pass
        assert_eq!(output[3], 0.5);
    }

    #[test]
    fn test_gate_dependencies() {
        // Verify dependencies are reported correctly
        let gate = GateNode::new(5, 10);
        let deps = gate.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 5);
        assert_eq!(deps[1], 10);
    }

    #[test]
    fn test_gate_with_constants() {
        // Integration test with ConstantNode
        let mut signal_node = ConstantNode::new(0.8);
        let mut threshold_node = ConstantNode::new(0.5);
        let mut gate = GateNode::new(0, 1);

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        // Process constants first
        let mut signal_buf = vec![0.0; 512];
        let mut threshold_buf = vec![0.0; 512];

        signal_node.process_block(&[], &mut signal_buf, 44100.0, &context);
        threshold_node.process_block(&[], &mut threshold_buf, 44100.0, &context);

        // Apply gate
        let inputs = vec![signal_buf.as_slice(), threshold_buf.as_slice()];
        let mut output = vec![0.0; 512];

        gate.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be 0.8 (0.8 > 0.5, so passes)
        for sample in &output {
            assert_eq!(*sample, 0.8);
        }
    }

    #[test]
    fn test_gate_mixed_signal() {
        // Test realistic scenario: mixed signal with peaks and quiet parts
        let mut gate = GateNode::new(0, 1);

        let input_signal = vec![0.9, 0.2, -0.8, 0.1, 0.6, -0.3, 0.7, -0.05];
        let threshold_value = vec![0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5];
        let inputs = vec![input_signal.as_slice(), threshold_value.as_slice()];

        let mut output = vec![0.0; 8];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 8, 2.0, 44100.0);

        gate.process_block(&inputs, &mut output, 44100.0, &context);

        // Check each sample
        assert_eq!(output[0], 0.9); // 0.9 > 0.5: pass
        assert_eq!(output[1], 0.0); // 0.2 < 0.5: block
        assert_eq!(output[2], -0.8); // |-0.8| = 0.8 > 0.5: pass
        assert_eq!(output[3], 0.0); // 0.1 < 0.5: block
        assert_eq!(output[4], 0.6); // 0.6 > 0.5: pass
        assert_eq!(output[5], 0.0); // |-0.3| = 0.3 < 0.5: block
        assert_eq!(output[6], 0.7); // 0.7 > 0.5: pass
        assert_eq!(output[7], 0.0); // |-0.05| = 0.05 < 0.5: block
    }

    #[test]
    fn test_gate_zero_threshold_passes_all_nonzero() {
        // Zero threshold should pass all non-zero samples
        let mut gate = GateNode::new(0, 1);

        let input_signal = vec![0.1, -0.01, 0.001, -0.0001];
        let threshold_value = vec![0.0, 0.0, 0.0, 0.0];
        let inputs = vec![input_signal.as_slice(), threshold_value.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        gate.process_block(&inputs, &mut output, 44100.0, &context);

        // All non-zero values should pass
        assert_eq!(output[0], 0.1);
        assert_eq!(output[1], -0.01);
        assert_eq!(output[2], 0.001);
        assert_eq!(output[3], -0.0001);
    }

    #[test]
    fn test_gate_high_threshold_blocks_all() {
        // Very high threshold should block everything
        let mut gate = GateNode::new(0, 1);

        let input_signal = vec![0.9, -0.8, 0.7, -0.6];
        let threshold_value = vec![2.0, 2.0, 2.0, 2.0];
        let inputs = vec![input_signal.as_slice(), threshold_value.as_slice()];

        let mut output = vec![999.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        gate.process_block(&inputs, &mut output, 44100.0, &context);

        // All samples below threshold, should all be zero
        for sample in &output {
            assert_eq!(*sample, 0.0);
        }
    }
}
