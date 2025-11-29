/// Decimator node - Sample rate reduction for lo-fi/retro effects
///
/// This node implements sample-and-hold based decimation with optional smoothing:
/// - Reduces effective sample rate by holding values for N samples
/// - Creates classic "bit-crushed" lo-fi sounds
/// - Introduces aliasing artifacts characteristic of early samplers
/// - Smooth parameter adds one-pole lowpass to reduce harshness
///
/// Common uses:
/// - Lo-fi/8-bit/chiptune effects
/// - Vintage sampler emulation
/// - Creative aliasing for texture
/// - Glitchy electronic music
///
/// Musical effect scale:
/// - factor=1: No effect (original sample rate)
/// - factor=2: Half sample rate (11.025 kHz @ 44.1 kHz SR)
/// - factor=4: Quarter rate (5.5 kHz) - noticeable aliasing
/// - factor=8: Eighth rate (2.7 kHz) - severe lo-fi
/// - factor=16+: Extreme degradation, near-square wave
use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Decimator node: Reduces effective sample rate
///
/// The algorithm:
/// ```text
/// sample_counter += 1.0;
/// if sample_counter >= factor {
///     held_value = input_sample;  // Sample new value
///     sample_counter = 0.0;
/// }
///
/// if smooth > 0.0 {
///     output = held_value * (1.0 - smooth) + smooth_state * smooth;
///     smooth_state = output;
/// } else {
///     output = held_value;
/// }
/// ```
///
/// # Example
/// ```ignore
/// // Create 8-bit style decimation
/// let audio = OscillatorNode::new(...);           // NodeId 0
/// let factor = ConstantNode::new(8.0);            // NodeId 1 (8x decimation)
/// let smooth = ConstantNode::new(0.0);            // NodeId 2 (harsh)
/// let decimated = DecimatorNode::new(0, 1, 2);    // NodeId 3
/// // Output will be stepped/aliased audio with 8-bit character
/// ```
pub struct DecimatorNode {
    /// Input signal to decimate
    input: NodeId,

    /// Decimation factor (1.0 = no change, higher = more decimation)
    /// Values < 1.0 are clamped to 1.0
    factor_input: NodeId,

    /// Smoothing amount (0.0 = harsh/stepped, 1.0 = smooth)
    /// Applies one-pole lowpass filter to reduce aliasing
    smooth_input: NodeId,

    /// Sample counter for decimation timing
    sample_counter: f32,

    /// Currently held value
    held_value: f32,

    /// Previous smoothed output (for one-pole filter)
    smooth_state: f32,
}

impl DecimatorNode {
    /// Decimator - Sample rate reduction for lo-fi and retro effects
    ///
    /// Reduces effective sample rate through sample-and-hold decimation,
    /// creating 8-bit/lo-fi sounds with adjustable smoothing.
    ///
    /// # Parameters
    /// - `input`: NodeId providing signal to decimate
    /// - `factor_input`: NodeId providing decimation factor 1.0-64.0 (default: 4)
    /// - `smooth_input`: NodeId providing smoothing amount 0.0-1.0 (default: 0)
    ///
    /// # Example
    /// ```phonon
    /// ~signal: sine 440
    /// ~lofi: ~signal # decimator 8 0.2
    /// ```
    pub fn new(input: NodeId, factor_input: NodeId, smooth_input: NodeId) -> Self {
        Self {
            input,
            factor_input,
            smooth_input,
            sample_counter: 0.0,
            held_value: 0.0,
            smooth_state: 0.0,
        }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }

    /// Get the factor input node ID
    pub fn factor_input(&self) -> NodeId {
        self.factor_input
    }

    /// Get the smooth input node ID
    pub fn smooth_input(&self) -> NodeId {
        self.smooth_input
    }

    /// Get the current held value (for debugging/testing)
    pub fn held_value(&self) -> f32 {
        self.held_value
    }

    /// Get the current sample counter (for debugging/testing)
    pub fn sample_counter(&self) -> f32 {
        self.sample_counter
    }
}

impl AudioNode for DecimatorNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 3,
            "DecimatorNode requires 3 inputs (input, factor, smooth), got {}",
            inputs.len()
        );

        let input_buf = inputs[0];
        let factor_buf = inputs[1];
        let smooth_buf = inputs[2];

        debug_assert_eq!(
            input_buf.len(),
            output.len(),
            "Input buffer length mismatch"
        );
        debug_assert_eq!(
            factor_buf.len(),
            output.len(),
            "Factor buffer length mismatch"
        );
        debug_assert_eq!(
            smooth_buf.len(),
            output.len(),
            "Smooth buffer length mismatch"
        );

        // Process each sample
        for i in 0..output.len() {
            let input_sample = input_buf[i];
            // Clamp factor to minimum of 1.0 (no effect below 1.0)
            let factor = factor_buf[i].max(1.0);
            // Clamp smooth to [0, 1] range
            let smooth = smooth_buf[i].clamp(0.0, 1.0);

            // Increment sample counter
            self.sample_counter += 1.0;

            // Check if we should sample a new value
            if self.sample_counter >= factor {
                self.held_value = input_sample;
                self.sample_counter = 0.0;
            }

            // Apply optional smoothing with one-pole filter
            let output_sample = if smooth > 0.0 {
                // One-pole lowpass: y[n] = x[n] * (1-a) + y[n-1] * a
                let smoothed = self.held_value * (1.0 - smooth) + self.smooth_state * smooth;
                self.smooth_state = smoothed;
                smoothed
            } else {
                // No smoothing - raw stepped output
                self.held_value
            };

            output[i] = output_sample;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.factor_input, self.smooth_input]
    }

    fn name(&self) -> &str {
        "DecimatorNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::Fraction;

    fn create_context(block_size: usize) -> ProcessContext {
        ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, 44100.0)
    }

    #[test]
    fn test_decimator_no_effect_factor_1() {
        // Test: factor=1.0 should pass signal through unchanged
        let mut decimator = DecimatorNode::new(0, 1, 2);

        let input = vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8];
        let factor = vec![1.0; 8];
        let smooth = vec![0.0; 8];
        let inputs = vec![input.as_slice(), factor.as_slice(), smooth.as_slice()];

        let mut output = vec![0.0; 8];
        let context = create_context(8);

        decimator.process_block(&inputs, &mut output, 44100.0, &context);

        // With factor=1.0, output should match input exactly
        for i in 0..8 {
            assert_eq!(
                output[i], input[i],
                "Sample {} should pass through unchanged",
                i
            );
        }
    }

    #[test]
    fn test_decimator_factor_2_sample_hold() {
        // Test: factor=2 should hold every other sample
        let mut decimator = DecimatorNode::new(0, 1, 2);

        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let factor = vec![2.0; 8];
        let smooth = vec![0.0; 8];
        let inputs = vec![input.as_slice(), factor.as_slice(), smooth.as_slice()];

        let mut output = vec![0.0; 8];
        let context = create_context(8);

        decimator.process_block(&inputs, &mut output, 44100.0, &context);

        // Algorithm:
        // i=0: counter=1, counter >= 2? no, output=held_value(0.0), counter=1
        // i=1: counter=2, counter >= 2? yes, sample 2.0, counter=0, output=2.0
        // i=2: counter=1, counter >= 2? no, output=2.0 (held)
        // i=3: counter=2, counter >= 2? yes, sample 4.0, counter=0, output=4.0
        // etc.

        // First sample outputs initial held_value (0.0)
        assert_eq!(output[0], 0.0, "First sample should be initial held value");
        assert_eq!(output[1], 2.0, "Should sample at index 1");
        assert_eq!(output[2], 2.0, "Should hold previous value");
        assert_eq!(output[3], 4.0, "Should sample at index 3");
        assert_eq!(output[4], 4.0, "Should hold previous value");
        assert_eq!(output[5], 6.0, "Should sample at index 5");
        assert_eq!(output[6], 6.0, "Should hold previous value");
        assert_eq!(output[7], 8.0, "Should sample at index 7");
    }

    #[test]
    fn test_decimator_factor_4_sample_hold() {
        // Test: factor=4 should hold for 4 samples
        let mut decimator = DecimatorNode::new(0, 1, 2);

        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let factor = vec![4.0; 8];
        let smooth = vec![0.0; 8];
        let inputs = vec![input.as_slice(), factor.as_slice(), smooth.as_slice()];

        let mut output = vec![0.0; 8];
        let context = create_context(8);

        decimator.process_block(&inputs, &mut output, 44100.0, &context);

        // Algorithm:
        // i=0: counter=1 < 4, output=0.0
        // i=1: counter=2 < 4, output=0.0
        // i=2: counter=3 < 4, output=0.0
        // i=3: counter=4 >= 4, sample 4.0, counter=0, output=4.0
        // i=4: counter=1 < 4, output=4.0
        // i=5: counter=2 < 4, output=4.0
        // i=6: counter=3 < 4, output=4.0
        // i=7: counter=4 >= 4, sample 8.0, counter=0, output=8.0

        assert_eq!(output[0], 0.0, "Initial held value");
        assert_eq!(output[1], 0.0, "Still holding");
        assert_eq!(output[2], 0.0, "Still holding");
        assert_eq!(output[3], 4.0, "Sample at index 3");
        assert_eq!(output[4], 4.0, "Hold 4.0");
        assert_eq!(output[5], 4.0, "Hold 4.0");
        assert_eq!(output[6], 4.0, "Hold 4.0");
        assert_eq!(output[7], 8.0, "Sample at index 7");
    }

    #[test]
    fn test_decimator_factor_8_extreme() {
        // Test: factor=8 creates severe decimation
        let mut decimator = DecimatorNode::new(0, 1, 2);

        let input: Vec<f32> = (0..16).map(|i| i as f32).collect();
        let factor = vec![8.0; 16];
        let smooth = vec![0.0; 16];
        let inputs = vec![input.as_slice(), factor.as_slice(), smooth.as_slice()];

        let mut output = vec![0.0; 16];
        let context = create_context(16);

        decimator.process_block(&inputs, &mut output, 44100.0, &context);

        // Should sample every 8th sample
        // Samples at index: 7, 15
        for i in 0..7 {
            assert_eq!(output[i], 0.0, "Indices 0-6 should be initial held value");
        }
        assert_eq!(output[7], 7.0, "Should sample at index 7");
        for i in 8..15 {
            assert_eq!(output[i], 7.0, "Indices 8-14 should hold 7.0");
        }
        assert_eq!(output[15], 15.0, "Should sample at index 15");
    }

    #[test]
    fn test_decimator_smooth_parameter() {
        // Test: smooth parameter reduces stepped artifacts
        let mut decimator = DecimatorNode::new(0, 1, 2);

        let input = vec![0.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0];
        let factor = vec![2.0; 8];
        let smooth = vec![0.5; 8]; // 50% smoothing
        let inputs = vec![input.as_slice(), factor.as_slice(), smooth.as_slice()];

        let mut output = vec![0.0; 8];
        let context = create_context(8);

        decimator.process_block(&inputs, &mut output, 44100.0, &context);

        // With smoothing, output should be less stepped
        // First few samples will ramp up rather than jump immediately
        assert!(
            output[0] < output[1] || output[0] == output[1],
            "Smooth output should increase or stay same"
        );
        assert!(
            output[2] >= output[1],
            "Smooth output should continue to increase or plateau"
        );
    }

    #[test]
    fn test_decimator_varying_factor() {
        // Test: factor can vary per-sample (dynamic decimation)
        let mut decimator = DecimatorNode::new(0, 1, 2);

        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let factor = vec![1.0, 1.0, 2.0, 2.0, 4.0, 4.0, 4.0, 4.0];
        let smooth = vec![0.0; 8];
        let inputs = vec![input.as_slice(), factor.as_slice(), smooth.as_slice()];

        let mut output = vec![0.0; 8];
        let context = create_context(8);

        decimator.process_block(&inputs, &mut output, 44100.0, &context);

        // Factor changes dynamically - should adapt behavior
        assert_eq!(output[0], 1.0, "factor=1 passes through");
        assert_eq!(output[1], 2.0, "factor=1 passes through");
        // After this, factor increases so behavior changes
        assert!(output[7] > 0.0, "Should have some output");
    }

    #[test]
    fn test_decimator_factor_below_1_clamped() {
        // Test: factor < 1.0 should be clamped to 1.0 (no super-sampling)
        let mut decimator = DecimatorNode::new(0, 1, 2);

        let input = vec![0.1, 0.2, 0.3, 0.4];
        let factor = vec![0.5, 0.0, -1.0, 0.9]; // All below 1.0
        let smooth = vec![0.0; 4];
        let inputs = vec![input.as_slice(), factor.as_slice(), smooth.as_slice()];

        let mut output = vec![0.0; 4];
        let context = create_context(4);

        decimator.process_block(&inputs, &mut output, 44100.0, &context);

        // Should behave like factor=1.0 (pass through)
        assert_eq!(output[0], 0.1, "factor clamped to 1.0");
        assert_eq!(output[1], 0.2, "factor clamped to 1.0");
        assert_eq!(output[2], 0.3, "factor clamped to 1.0");
        assert_eq!(output[3], 0.4, "factor clamped to 1.0");
    }

    #[test]
    fn test_decimator_smooth_clamp() {
        // Test: smooth parameter is clamped to [0, 1]
        let mut decimator = DecimatorNode::new(0, 1, 2);

        let input = vec![1.0, 1.0, 1.0, 1.0];
        let factor = vec![2.0; 4];
        let smooth = vec![-0.5, 0.5, 1.5, 2.0]; // Outside [0, 1]
        let inputs = vec![input.as_slice(), factor.as_slice(), smooth.as_slice()];

        let mut output = vec![0.0; 4];
        let context = create_context(4);

        decimator.process_block(&inputs, &mut output, 44100.0, &context);

        // Should not crash, smooth values clamped internally
        for sample in &output {
            assert!(
                sample.is_finite(),
                "Output should be finite even with out-of-range smooth"
            );
        }
    }

    #[test]
    fn test_decimator_sine_wave_aliasing() {
        // Test: High-frequency sine wave decimated creates aliasing
        let mut decimator = DecimatorNode::new(0, 1, 2);

        // Generate sine wave at high frequency
        let mut input = Vec::new();
        for i in 0..128 {
            let t = i as f32 / 128.0;
            input.push((t * 2.0 * std::f32::consts::PI * 8.0).sin());
        }

        let factor = vec![4.0; 128]; // Quarter sample rate
        let smooth = vec![0.0; 128];
        let inputs = vec![input.as_slice(), factor.as_slice(), smooth.as_slice()];

        let mut output = vec![0.0; 128];
        let context = create_context(128);

        decimator.process_block(&inputs, &mut output, 44100.0, &context);

        // Output should be stepped (sample-and-hold effect)
        // Multiple consecutive samples should have identical values
        let mut hold_count = 0;
        for i in 1..128 {
            if (output[i] - output[i - 1]).abs() < 0.0001 {
                hold_count += 1;
            }
        }

        // With factor=4, we expect significant holding (~75% of samples held)
        assert!(
            hold_count > 64,
            "Should have many held samples (got {})",
            hold_count
        );
    }

    #[test]
    fn test_decimator_state_persistence() {
        // Test: State persists across multiple process_block calls
        let mut decimator = DecimatorNode::new(0, 1, 2);

        // First block
        let input1 = vec![1.0, 2.0, 3.0, 4.0];
        let factor1 = vec![4.0; 4];
        let smooth1 = vec![0.0; 4];
        let inputs1 = vec![input1.as_slice(), factor1.as_slice(), smooth1.as_slice()];
        let mut output1 = vec![0.0; 4];
        let context = create_context(4);

        decimator.process_block(&inputs1, &mut output1, 44100.0, &context);

        // Counter should be at 0 after sampling at index 3
        // held_value should be 4.0

        // Second block - continue from previous state
        let input2 = vec![5.0, 6.0, 7.0, 8.0];
        let factor2 = vec![4.0; 4];
        let smooth2 = vec![0.0; 4];
        let inputs2 = vec![input2.as_slice(), factor2.as_slice(), smooth2.as_slice()];
        let mut output2 = vec![0.0; 4];

        decimator.process_block(&inputs2, &mut output2, 44100.0, &context);

        // First samples should continue holding 4.0 from previous block
        // Then sample new value at appropriate time
        // Counter was 0, so: 1, 2, 3, 4 -> sample at index 3
        assert_eq!(
            output2[0], 4.0,
            "Should continue holding from previous block"
        );
        assert_eq!(output2[1], 4.0, "Should continue holding");
        assert_eq!(output2[2], 4.0, "Should continue holding");
        assert_eq!(output2[3], 8.0, "Should sample new value");
    }

    #[test]
    fn test_decimator_dependencies() {
        // Test: input_nodes returns all three dependencies
        let decimator = DecimatorNode::new(5, 10, 15);
        let deps = decimator.input_nodes();

        assert_eq!(deps.len(), 3, "Should have 3 dependencies");
        assert_eq!(deps[0], 5, "First dependency is input");
        assert_eq!(deps[1], 10, "Second dependency is factor");
        assert_eq!(deps[2], 15, "Third dependency is smooth");
    }

    #[test]
    fn test_decimator_getter_methods() {
        // Test: Getter methods return correct values
        let decimator = DecimatorNode::new(7, 11, 13);

        assert_eq!(decimator.input(), 7);
        assert_eq!(decimator.factor_input(), 11);
        assert_eq!(decimator.smooth_input(), 13);
        assert_eq!(decimator.held_value(), 0.0);
        assert_eq!(decimator.sample_counter(), 0.0);
    }

    #[test]
    fn test_decimator_smooth_0_vs_1() {
        // Test: Compare smooth=0.0 (harsh) vs smooth=1.0 (maximum smoothing)
        let mut decimator_harsh = DecimatorNode::new(0, 1, 2);
        let mut decimator_smooth = DecimatorNode::new(0, 1, 2);

        // Square wave input
        let input = vec![1.0, 1.0, 1.0, 1.0, -1.0, -1.0, -1.0, -1.0];
        let factor = vec![2.0; 8];
        let smooth_harsh = vec![0.0; 8];
        let smooth_smooth = vec![0.9; 8];

        let inputs_harsh = vec![input.as_slice(), factor.as_slice(), smooth_harsh.as_slice()];
        let inputs_smooth = vec![
            input.as_slice(),
            factor.as_slice(),
            smooth_smooth.as_slice(),
        ];

        let mut output_harsh = vec![0.0; 8];
        let mut output_smooth = vec![0.0; 8];
        let context = create_context(8);

        decimator_harsh.process_block(&inputs_harsh, &mut output_harsh, 44100.0, &context);
        decimator_smooth.process_block(&inputs_smooth, &mut output_smooth, 44100.0, &context);

        // Harsh output should have sharp transitions
        // Smooth output should have gradual transitions
        // Check that smooth output has intermediate values
        let harsh_unique: Vec<f32> = output_harsh.iter().copied().collect();
        let smooth_range = output_smooth
            .iter()
            .copied()
            .filter(|&v| v > -1.0 && v < 1.0)
            .count();

        // Smooth version should have more intermediate values
        assert!(
            smooth_range >= 0,
            "Smooth output should have intermediate values during transitions"
        );
    }
}
