/// Mix node - weighted sum of N inputs
///
/// This node combines multiple input signals with static weights.
/// Output[i] = sum(inputs[j][i] * weights[j]) for all samples.
///
/// # Use Cases
/// - Mixing multiple audio sources
/// - Creating submixes before effects
/// - Balancing different instruments
///
/// # Example
/// ```ignore
/// // Mix three oscillators with different levels
/// let osc1 = OscillatorNode::new(0, Waveform::Sine);      // NodeId 1 (depends on node 0 for freq)
/// let osc2 = OscillatorNode::new(2, Waveform::Saw);       // NodeId 3 (depends on node 2 for freq)
/// let osc3 = OscillatorNode::new(4, Waveform::Square);    // NodeId 5 (depends on node 4 for freq)
///
/// // Mix with weights: 50% sine, 30% saw, 20% square
/// let mix = MixNode::new(vec![1, 3, 5], vec![0.5, 0.3, 0.2]);
/// ```
use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Mix node: weighted sum of N inputs
///
/// Each input is multiplied by its corresponding weight and summed.
/// Weights are static (not pattern-controlled) for efficiency.
pub struct MixNode {
    /// Input node IDs
    inputs: Vec<NodeId>,
    /// Static weights for each input (length must match inputs)
    weights: Vec<f32>,
}

impl MixNode {
    /// Mix - Weighted sum of N signals
    ///
    /// Combines multiple input signals using static weights.
    /// Each input is multiplied by its weight and summed together.
    ///
    /// # Parameters
    /// - `inputs`: Vector of signal NodeIds to mix
    /// - `weights`: Vector of weights (must match inputs length)
    ///
    /// # Example
    /// ```phonon
    /// ~sig1: sine 220
    /// ~sig2: sine 330
    /// ~sig3: sine 440
    /// ~mixed: mix [~sig1, ~sig2, ~sig3] [0.5, 0.3, 0.2]
    /// out: ~mixed
    /// ```
    pub fn new(inputs: Vec<NodeId>, weights: Vec<f32>) -> Self {
        assert_eq!(
            inputs.len(),
            weights.len(),
            "MixNode: inputs and weights must have same length"
        );
        Self { inputs, weights }
    }

    /// Get the input node IDs
    pub fn inputs(&self) -> &[NodeId] {
        &self.inputs
    }

    /// Get the weights
    pub fn weights(&self) -> &[f32] {
        &self.weights
    }
}

impl AudioNode for MixNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert_eq!(
            inputs.len(),
            self.weights.len(),
            "MixNode received {} inputs but has {} weights",
            inputs.len(),
            self.weights.len()
        );

        // Initialize output to zero
        for sample in output.iter_mut() {
            *sample = 0.0;
        }

        // Accumulate weighted inputs
        for (input_buf, weight) in inputs.iter().zip(&self.weights) {
            debug_assert_eq!(
                input_buf.len(),
                output.len(),
                "Input buffer length mismatch"
            );

            for (i, sample) in input_buf.iter().enumerate() {
                output[i] += sample * weight;
            }
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        self.inputs.clone()
    }

    fn name(&self) -> &str {
        "MixNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_mix_node_two_inputs_equal_weights() {
        // Test mixing two signals with equal 0.5 weights
        let mut mix = MixNode::new(vec![0, 1], vec![0.5, 0.5]);

        let input_a = vec![1.0, 2.0, 3.0, 4.0];
        let input_b = vec![10.0, 20.0, 30.0, 40.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        mix.process_block(&inputs, &mut output, 44100.0, &context);

        // Expected: 1.0*0.5 + 10.0*0.5 = 5.5, etc.
        assert_eq!(output[0], 5.5); // 0.5 + 5.0
        assert_eq!(output[1], 11.0); // 1.0 + 10.0
        assert_eq!(output[2], 16.5); // 1.5 + 15.0
        assert_eq!(output[3], 22.0); // 2.0 + 20.0
    }

    #[test]
    fn test_mix_node_three_inputs_different_weights() {
        // Test mixing three signals with weights 0.5, 0.3, 0.2
        let mut mix = MixNode::new(vec![0, 1, 2], vec![0.5, 0.3, 0.2]);

        let input_a = vec![10.0, 20.0, 30.0, 40.0];
        let input_b = vec![100.0, 200.0, 300.0, 400.0];
        let input_c = vec![1000.0, 2000.0, 3000.0, 4000.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice(), input_c.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        mix.process_block(&inputs, &mut output, 44100.0, &context);

        // Expected: 10*0.5 + 100*0.3 + 1000*0.2 = 5 + 30 + 200 = 235
        assert_eq!(output[0], 235.0);
        assert_eq!(output[1], 470.0); // 20*0.5 + 200*0.3 + 2000*0.2
        assert_eq!(output[2], 705.0); // 30*0.5 + 300*0.3 + 3000*0.2
        assert_eq!(output[3], 940.0); // 40*0.5 + 400*0.3 + 4000*0.2
    }

    #[test]
    fn test_mix_node_single_input_passthrough() {
        // Test single input (should act as weighted passthrough)
        let mut mix = MixNode::new(vec![0], vec![1.0]);

        let input_a = vec![1.5, 2.5, 3.5, 4.5];
        let inputs = vec![input_a.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        mix.process_block(&inputs, &mut output, 44100.0, &context);

        // Should pass through unchanged with weight 1.0
        assert_eq!(output[0], 1.5);
        assert_eq!(output[1], 2.5);
        assert_eq!(output[2], 3.5);
        assert_eq!(output[3], 4.5);
    }

    #[test]
    fn test_mix_node_zero_weight_excludes_input() {
        // Test that zero weight effectively excludes an input
        let mut mix = MixNode::new(vec![0, 1, 2], vec![1.0, 0.0, 0.5]);

        let input_a = vec![10.0, 20.0, 30.0, 40.0];
        let input_b = vec![999.0, 999.0, 999.0, 999.0]; // Should be ignored
        let input_c = vec![100.0, 200.0, 300.0, 400.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice(), input_c.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        mix.process_block(&inputs, &mut output, 44100.0, &context);

        // Expected: 10*1.0 + 999*0.0 + 100*0.5 = 10 + 0 + 50 = 60
        assert_eq!(output[0], 60.0);
        assert_eq!(output[1], 120.0); // 20*1.0 + 999*0.0 + 200*0.5
        assert_eq!(output[2], 180.0); // 30*1.0 + 999*0.0 + 300*0.5
        assert_eq!(output[3], 240.0); // 40*1.0 + 999*0.0 + 400*0.5
    }

    #[test]
    fn test_mix_node_dependencies() {
        let mix = MixNode::new(vec![5, 10, 15], vec![0.3, 0.4, 0.3]);
        let deps = mix.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 5);
        assert_eq!(deps[1], 10);
        assert_eq!(deps[2], 15);
    }

    #[test]
    fn test_mix_node_negative_weights() {
        // Test mixing with negative weights (phase inversion)
        let mut mix = MixNode::new(vec![0, 1], vec![1.0, -0.5]);

        let input_a = vec![10.0, 20.0, 30.0, 40.0];
        let input_b = vec![4.0, 8.0, 12.0, 16.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        mix.process_block(&inputs, &mut output, 44100.0, &context);

        // Expected: 10*1.0 + 4*(-0.5) = 10 - 2 = 8
        assert_eq!(output[0], 8.0);
        assert_eq!(output[1], 16.0); // 20*1.0 + 8*(-0.5) = 20 - 4
        assert_eq!(output[2], 24.0); // 30*1.0 + 12*(-0.5) = 30 - 6
        assert_eq!(output[3], 32.0); // 40*1.0 + 16*(-0.5) = 40 - 8
    }

    #[test]
    fn test_mix_node_with_constants() {
        // Integration test with ConstantNode
        let mut const_a = ConstantNode::new(100.0);
        let mut const_b = ConstantNode::new(50.0);
        let mut const_c = ConstantNode::new(25.0);
        let mut mix = MixNode::new(vec![0, 1, 2], vec![0.5, 0.3, 0.2]);

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        // Process constants first
        let mut buf_a = vec![0.0; 512];
        let mut buf_b = vec![0.0; 512];
        let mut buf_c = vec![0.0; 512];

        const_a.process_block(&[], &mut buf_a, 44100.0, &context);
        const_b.process_block(&[], &mut buf_b, 44100.0, &context);
        const_c.process_block(&[], &mut buf_c, 44100.0, &context);

        // Now mix them
        let inputs = vec![buf_a.as_slice(), buf_b.as_slice(), buf_c.as_slice()];
        let mut output = vec![0.0; 512];

        mix.process_block(&inputs, &mut output, 44100.0, &context);

        // Expected: 100*0.5 + 50*0.3 + 25*0.2 = 50 + 15 + 5 = 70
        for sample in &output {
            assert_eq!(*sample, 70.0);
        }
    }

    #[test]
    #[should_panic(expected = "inputs and weights must have same length")]
    fn test_mix_node_length_mismatch() {
        // Should panic when inputs and weights have different lengths
        MixNode::new(vec![0, 1, 2], vec![0.5, 0.5]);
    }
}
