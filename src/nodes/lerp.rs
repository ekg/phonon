/// Lerp (linear interpolation) node - blend between two signals
///
/// This node performs linear interpolation between two input signals
/// based on a mix parameter that can vary per-sample.
///
/// Formula: `output[i] = a[i] * (1.0 - mix[i]) + b[i] * mix[i]`
///
/// # Mix Parameter Behavior
/// - `mix = 0.0` → 100% signal A
/// - `mix = 0.5` → 50/50 blend
/// - `mix = 1.0` → 100% signal B
/// - Mix values outside [0,1] will extrapolate (no clamping)
///
/// # Use Cases
/// - Crossfading between two audio sources
/// - Morphing between waveforms
/// - Creating smooth transitions
/// - Pattern-controlled blend amount
///
/// # Example
/// ```ignore
/// // Crossfade from sine to saw based on LFO
/// let sine = OscillatorNode::new(0, Waveform::Sine);     // NodeId 1
/// let saw = OscillatorNode::new(2, Waveform::Saw);       // NodeId 3
/// let lfo = OscillatorNode::new(4, Waveform::Sine);      // NodeId 5 (slow LFO)
///
/// // Lerp between sine and saw using LFO as mix control
/// let crossfade = LerpNode::new(1, 3, 5);
/// ```

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Lerp node: linear interpolation between two signals
///
/// Blends between input_a and input_b based on mix_input.
/// Mix is evaluated per-sample, allowing for dynamic crossfading.
pub struct LerpNode {
    /// First input signal (A)
    input_a: NodeId,
    /// Second input signal (B)
    input_b: NodeId,
    /// Mix amount (0.0 = A, 1.0 = B)
    mix_input: NodeId,
}

impl LerpNode {
    /// Lerp - Linear interpolation between two signals
    ///
    /// Blends between two signals based on pattern-controlled mix amount.
    /// Useful for crossfading, waveform morphing, and smooth transitions.
    ///
    /// # Parameters
    /// - `input_a`: Signal 1 (0% when mix = 0.0)
    /// - `input_b`: Signal 2 (100% when mix = 1.0)
    /// - `mix_input`: Blend amount (0.0-1.0, extrapolates outside range)
    ///
    /// # Example
    /// ```phonon
    /// ~lfo: sine 0.25
    /// ~mix: (~lfo + 1) * 0.5
    /// ~sine: sine 220
    /// ~saw: saw 220
    /// ~blend: ~sine # lerp ~saw ~mix
    /// out: ~blend * 0.5
    /// ```
    pub fn new(input_a: NodeId, input_b: NodeId, mix_input: NodeId) -> Self {
        Self {
            input_a,
            input_b,
            mix_input,
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

    /// Get the mix input node ID
    pub fn mix_input(&self) -> NodeId {
        self.mix_input
    }
}

impl AudioNode for LerpNode {
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
            "LerpNode expects 3 inputs (a, b, mix), got {}",
            inputs.len()
        );

        let input_a = inputs[0];
        let input_b = inputs[1];
        let mix = inputs[2];

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
            mix.len(),
            output.len(),
            "Mix buffer length mismatch"
        );

        // Linear interpolation: output = a * (1 - mix) + b * mix
        for i in 0..output.len() {
            let m = mix[i];
            output[i] = input_a[i] * (1.0 - m) + input_b[i] * m;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input_a, self.input_b, self.mix_input]
    }

    fn name(&self) -> &str {
        "LerpNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_lerp_mix_zero_returns_a() {
        // Mix = 0.0 should return 100% signal A
        let mut lerp = LerpNode::new(0, 1, 2);

        let input_a = vec![10.0, 20.0, 30.0, 40.0];
        let input_b = vec![100.0, 200.0, 300.0, 400.0];
        let mix = vec![0.0, 0.0, 0.0, 0.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice(), mix.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        lerp.process_block(&inputs, &mut output, 44100.0, &context);

        // Should return input_a unchanged
        assert_eq!(output[0], 10.0);
        assert_eq!(output[1], 20.0);
        assert_eq!(output[2], 30.0);
        assert_eq!(output[3], 40.0);
    }

    #[test]
    fn test_lerp_mix_one_returns_b() {
        // Mix = 1.0 should return 100% signal B
        let mut lerp = LerpNode::new(0, 1, 2);

        let input_a = vec![10.0, 20.0, 30.0, 40.0];
        let input_b = vec![100.0, 200.0, 300.0, 400.0];
        let mix = vec![1.0, 1.0, 1.0, 1.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice(), mix.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        lerp.process_block(&inputs, &mut output, 44100.0, &context);

        // Should return input_b unchanged
        assert_eq!(output[0], 100.0);
        assert_eq!(output[1], 200.0);
        assert_eq!(output[2], 300.0);
        assert_eq!(output[3], 400.0);
    }

    #[test]
    fn test_lerp_mix_half_returns_average() {
        // Mix = 0.5 should return 50/50 blend
        let mut lerp = LerpNode::new(0, 1, 2);

        let input_a = vec![10.0, 20.0, 30.0, 40.0];
        let input_b = vec![100.0, 200.0, 300.0, 400.0];
        let mix = vec![0.5, 0.5, 0.5, 0.5];
        let inputs = vec![input_a.as_slice(), input_b.as_slice(), mix.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        lerp.process_block(&inputs, &mut output, 44100.0, &context);

        // Expected: (10 + 100) / 2 = 55, etc.
        assert_eq!(output[0], 55.0);
        assert_eq!(output[1], 110.0);
        assert_eq!(output[2], 165.0);
        assert_eq!(output[3], 220.0);
    }

    #[test]
    fn test_lerp_mix_extrapolation() {
        // Mix values outside [0,1] should extrapolate (no clamping)
        let mut lerp = LerpNode::new(0, 1, 2);

        let input_a = vec![0.0, 0.0, 0.0, 0.0];
        let input_b = vec![100.0, 100.0, 100.0, 100.0];

        // Test mix values: -0.5, 1.5, 2.0, -1.0
        let mix = vec![-0.5, 1.5, 2.0, -1.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice(), mix.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        lerp.process_block(&inputs, &mut output, 44100.0, &context);

        // Mix = -0.5: 0 * (1 - (-0.5)) + 100 * (-0.5) = 0 * 1.5 + 100 * (-0.5) = -50
        assert_eq!(output[0], -50.0);

        // Mix = 1.5: 0 * (1 - 1.5) + 100 * 1.5 = 0 * (-0.5) + 100 * 1.5 = 150
        assert_eq!(output[1], 150.0);

        // Mix = 2.0: 0 * (1 - 2.0) + 100 * 2.0 = 0 * (-1.0) + 100 * 2.0 = 200
        assert_eq!(output[2], 200.0);

        // Mix = -1.0: 0 * (1 - (-1.0)) + 100 * (-1.0) = 0 * 2.0 + 100 * (-1.0) = -100
        assert_eq!(output[3], -100.0);
    }

    #[test]
    fn test_lerp_varying_mix_per_sample() {
        // Test that mix can vary per-sample
        let mut lerp = LerpNode::new(0, 1, 2);

        let input_a = vec![10.0, 10.0, 10.0, 10.0];
        let input_b = vec![20.0, 20.0, 20.0, 20.0];

        // Different mix value per sample
        let mix = vec![0.0, 0.25, 0.5, 1.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice(), mix.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        lerp.process_block(&inputs, &mut output, 44100.0, &context);

        // Mix = 0.0: 10 * 1.0 + 20 * 0.0 = 10
        assert_eq!(output[0], 10.0);

        // Mix = 0.25: 10 * 0.75 + 20 * 0.25 = 7.5 + 5.0 = 12.5
        assert_eq!(output[1], 12.5);

        // Mix = 0.5: 10 * 0.5 + 20 * 0.5 = 5.0 + 10.0 = 15.0
        assert_eq!(output[2], 15.0);

        // Mix = 1.0: 10 * 0.0 + 20 * 1.0 = 20.0
        assert_eq!(output[3], 20.0);
    }

    #[test]
    fn test_lerp_dependencies() {
        // Test that input_nodes returns all three dependencies
        let lerp = LerpNode::new(5, 10, 15);
        let deps = lerp.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 5);   // input_a
        assert_eq!(deps[1], 10);  // input_b
        assert_eq!(deps[2], 15);  // mix_input
    }

    #[test]
    fn test_lerp_with_constants() {
        // Integration test with ConstantNode
        let mut const_a = ConstantNode::new(50.0);
        let mut const_b = ConstantNode::new(150.0);
        let mut const_mix = ConstantNode::new(0.3);
        let mut lerp = LerpNode::new(0, 1, 2);

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
        let mut buf_mix = vec![0.0; 512];

        const_a.process_block(&[], &mut buf_a, 44100.0, &context);
        const_b.process_block(&[], &mut buf_b, 44100.0, &context);
        const_mix.process_block(&[], &mut buf_mix, 44100.0, &context);

        // Now lerp them
        let inputs = vec![buf_a.as_slice(), buf_b.as_slice(), buf_mix.as_slice()];
        let mut output = vec![0.0; 512];

        lerp.process_block(&inputs, &mut output, 44100.0, &context);

        // Expected: 50 * (1 - 0.3) + 150 * 0.3 = 50 * 0.7 + 150 * 0.3 = 35 + 45 = 80
        for sample in &output {
            assert_eq!(*sample, 80.0);
        }
    }

    #[test]
    fn test_lerp_crossfade_scenario() {
        // Realistic crossfade scenario: fade from one signal to another
        let mut lerp = LerpNode::new(0, 1, 2);

        // Two different constant signals
        let input_a = vec![100.0; 8];
        let input_b = vec![200.0; 8];

        // Crossfade from A to B over 8 samples
        let mix = vec![0.0, 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 0.875];
        let inputs = vec![input_a.as_slice(), input_b.as_slice(), mix.as_slice()];

        let mut output = vec![0.0; 8];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            8,
            2.0,
            44100.0,
        );

        lerp.process_block(&inputs, &mut output, 44100.0, &context);

        // Verify smooth crossfade
        assert_eq!(output[0], 100.0);  // 100% A
        assert!((output[1] - 112.5).abs() < 0.001);  // 87.5% A, 12.5% B
        assert_eq!(output[2], 125.0);  // 75% A, 25% B
        assert!((output[3] - 137.5).abs() < 0.001);  // 62.5% A, 37.5% B
        assert_eq!(output[4], 150.0);  // 50% A, 50% B
        assert!((output[5] - 162.5).abs() < 0.001);  // 37.5% A, 62.5% B
        assert_eq!(output[6], 175.0);  // 25% A, 75% B
        assert!((output[7] - 187.5).abs() < 0.001);  // 12.5% A, 87.5% B
    }

    #[test]
    fn test_lerp_negative_values() {
        // Test lerp with negative input values
        let mut lerp = LerpNode::new(0, 1, 2);

        let input_a = vec![-50.0, -40.0, -30.0, -20.0];
        let input_b = vec![50.0, 40.0, 30.0, 20.0];
        let mix = vec![0.0, 0.25, 0.5, 1.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice(), mix.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        lerp.process_block(&inputs, &mut output, 44100.0, &context);

        // Mix = 0.0: -50
        assert_eq!(output[0], -50.0);

        // Mix = 0.25: -40 * 0.75 + 40 * 0.25 = -30 + 10 = -20
        assert_eq!(output[1], -20.0);

        // Mix = 0.5: -30 * 0.5 + 30 * 0.5 = -15 + 15 = 0
        assert_eq!(output[2], 0.0);

        // Mix = 1.0: 20
        assert_eq!(output[3], 20.0);
    }
}
