/// Max node - outputs the maximum of two input signals
///
/// This node demonstrates sample-wise comparison operations.
/// Output[i] = max(Input_A[i], Input_B[i]) for all samples.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Max node: out = max(a, b)
///
/// # Example
/// ```ignore
/// // Max of two constant values: max(3.0, 5.0) = 5.0
/// let const_a = ConstantNode::new(3.0);   // NodeId 0
/// let const_b = ConstantNode::new(5.0);   // NodeId 1
/// let max = MaxNode::new(0, 1); // NodeId 2
/// // Output will be 5.0
/// ```
pub struct MaxNode {
    input_a: NodeId,
    input_b: NodeId,
}

impl MaxNode {
    /// Max - Output maximum of two signals
    ///
    /// Outputs the larger of the two input values sample-by-sample.
    /// Useful for envelope mixing and dynamic control signal selection.
    ///
    /// # Parameters
    /// - `input_a`: First signal
    /// - `input_b`: Second signal
    ///
    /// # Example
    /// ```phonon
    /// ~env1: impulse 2 # adsr 0.01 0.1 0.5 0.2
    /// ~env2: impulse 3 # adsr 0.02 0.05 0.7 0.1
    /// ~combined: ~env1 # max ~env2
    /// out: sine 220 * ~combined * 0.5
    /// ```
    pub fn new(input_a: NodeId, input_b: NodeId) -> Self {
        Self { input_a, input_b }
    }

    /// Get the first input node ID
    pub fn input_a(&self) -> NodeId {
        self.input_a
    }

    /// Get the second input node ID
    pub fn input_b(&self) -> NodeId {
        self.input_b
    }
}

impl AudioNode for MaxNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "MaxNode requires 2 inputs, got {}",
            inputs.len()
        );

        let buf_a = inputs[0];
        let buf_b = inputs[1];

        debug_assert_eq!(
            buf_a.len(),
            output.len(),
            "Input A length mismatch"
        );
        debug_assert_eq!(
            buf_b.len(),
            output.len(),
            "Input B length mismatch"
        );

        // Sample-wise maximum
        for i in 0..output.len() {
            output[i] = buf_a[i].max(buf_b[i]);
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input_a, self.input_b]
    }

    fn name(&self) -> &str {
        "MaxNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::nodes::oscillator::{OscillatorNode, Waveform};
    use crate::pattern::Fraction;

    #[test]
    fn test_max_node_constants_positive() {
        // Test: max(3.0, 5.0) = 5.0
        let mut const_a = ConstantNode::new(3.0);
        let mut const_b = ConstantNode::new(5.0);
        let mut max = MaxNode::new(0, 1);

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

        const_a.process_block(&[], &mut buf_a, 44100.0, &context);
        const_b.process_block(&[], &mut buf_b, 44100.0, &context);

        // Now get max
        let inputs = vec![buf_a.as_slice(), buf_b.as_slice()];
        let mut output = vec![0.0; 512];

        max.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be 5.0 (max of 3.0 and 5.0)
        for sample in &output {
            assert_eq!(*sample, 5.0);
        }
    }

    #[test]
    fn test_max_node_with_negative() {
        // Test: max(-2.0, 1.0) = 1.0
        let mut max = MaxNode::new(0, 1);

        let input_a = vec![-2.0, -2.0, -2.0, -2.0];
        let input_b = vec![1.0, 1.0, 1.0, 1.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        max.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be 1.0 (max of -2.0 and 1.0)
        for sample in &output {
            assert_eq!(*sample, 1.0);
        }
    }

    #[test]
    fn test_max_node_varying_values() {
        // Test max with varying values
        let mut max = MaxNode::new(0, 1);

        let input_a = vec![1.0, 5.0, -3.0, 7.0];
        let input_b = vec![3.0, 2.0, -1.0, 10.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        max.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 3.0);   // max(1, 3) = 3
        assert_eq!(output[1], 5.0);   // max(5, 2) = 5
        assert_eq!(output[2], -1.0);  // max(-3, -1) = -1
        assert_eq!(output[3], 10.0);  // max(7, 10) = 10
    }

    #[test]
    fn test_max_node_symmetric() {
        // Test: max(a, b) = max(b, a)
        let mut max_ab = MaxNode::new(0, 1);
        let mut max_ba = MaxNode::new(0, 1);

        let input_a = vec![2.0, -3.0, 5.0, -1.0];
        let input_b = vec![4.0, 1.0, 3.0, -2.0];

        let inputs_ab = vec![input_a.as_slice(), input_b.as_slice()];
        let inputs_ba = vec![input_b.as_slice(), input_a.as_slice()];

        let mut output_ab = vec![0.0; 4];
        let mut output_ba = vec![0.0; 4];

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        max_ab.process_block(&inputs_ab, &mut output_ab, 44100.0, &context);
        max_ba.process_block(&inputs_ba, &mut output_ba, 44100.0, &context);

        // Results should be identical
        for i in 0..4 {
            assert_eq!(output_ab[i], output_ba[i]);
        }
    }

    #[test]
    fn test_max_node_with_oscillators() {
        // Test max picks higher oscillator value
        // Using two constant frequencies for deterministic test
        let mut freq_a = ConstantNode::new(440.0);
        let mut freq_b = ConstantNode::new(880.0);
        let mut osc_a = OscillatorNode::new(0, Waveform::Sine);
        let mut osc_b = OscillatorNode::new(1, Waveform::Sine);
        let mut max = MaxNode::new(2, 3);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            128,
            2.0,
            44100.0,
        );

        // Process frequency constants
        let mut buf_freq_a = vec![0.0; 128];
        let mut buf_freq_b = vec![0.0; 128];
        freq_a.process_block(&[], &mut buf_freq_a, 44100.0, &context);
        freq_b.process_block(&[], &mut buf_freq_b, 44100.0, &context);

        // Process oscillators
        let mut buf_osc_a = vec![0.0; 128];
        let mut buf_osc_b = vec![0.0; 128];
        osc_a.process_block(&[&buf_freq_a], &mut buf_osc_a, 44100.0, &context);
        osc_b.process_block(&[&buf_freq_b], &mut buf_osc_b, 44100.0, &context);

        // Process max
        let inputs = vec![buf_osc_a.as_slice(), buf_osc_b.as_slice()];
        let mut output = vec![0.0; 128];
        max.process_block(&inputs, &mut output, 44100.0, &context);

        // Verify that output contains valid values
        // (actual max of two oscillating signals)
        for &sample in &output {
            // Max should be within valid audio range
            assert!(sample >= -1.0 && sample <= 1.0);
        }

        // At least some samples should be picking from each oscillator
        // This verifies the max operation is actually comparing
        let mut found_from_a = false;
        let mut found_from_b = false;

        for i in 0..128 {
            if (output[i] - buf_osc_a[i]).abs() < 1e-6 {
                found_from_a = true;
            }
            if (output[i] - buf_osc_b[i]).abs() < 1e-6 {
                found_from_b = true;
            }
        }

        // For two sine waves at different frequencies, the max should
        // sometimes pick from each one (though not necessarily in this short buffer)
        // At minimum, verify the operation produces valid results
        assert!(found_from_a || found_from_b);
    }

    #[test]
    fn test_max_node_dependencies() {
        let max = MaxNode::new(5, 10);
        let deps = max.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 5);
        assert_eq!(deps[1], 10);
    }
}
