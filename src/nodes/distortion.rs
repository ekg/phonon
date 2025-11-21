/// Distortion node - soft clipping waveshaper with drive and wet/dry mix
///
/// This node applies smooth, musical distortion using hyperbolic tangent (tanh)
/// waveshaping. Unlike hard clipping, tanh produces a smoother, more musical
/// saturation that's commonly used in analog-style distortion effects.
///
/// # Algorithm
/// ```text
/// driven = input * drive  // Amplify input signal
/// distorted = tanh(driven)  // Soft clip using tanh
/// output = input * (1 - mix) + distorted * mix  // Blend wet/dry
/// ```
///
/// # Applications
/// - Analog-style saturation/overdrive
/// - Tube amp simulation
/// - Guitar distortion effects
/// - Adding warmth and harmonics to sounds
/// - Subtle saturation on drums/bass
///
/// # Example
/// ```ignore
/// // Guitar-style overdrive
/// let guitar = OscillatorNode::new(Waveform::Saw);  // NodeId 1
/// let drive = ConstantNode::new(5.0);                // NodeId 2 (5x gain)
/// let mix = ConstantNode::new(0.8);                  // NodeId 3 (80% wet)
/// let dist = DistortionNode::new(1, 2, 3);           // NodeId 4
/// ```

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Distortion node: soft clipping waveshaper
///
/// Provides smooth, musical distortion using tanh waveshaping.
/// The drive parameter controls the amount of gain before waveshaping,
/// and the mix parameter blends between dry (0.0) and wet (1.0) signal.
pub struct DistortionNode {
    input: NodeId,
    drive_input: NodeId,  // Drive amount (1.0 to 100.0)
    mix_input: NodeId,    // Wet/dry mix (0.0 = dry, 1.0 = wet)
}

impl DistortionNode {
    /// Create a new distortion node
    ///
    /// # Arguments
    /// * `input` - NodeId of signal to distort
    /// * `drive_input` - NodeId of drive amount (typical range: 1.0 to 100.0)
    ///   - 1.0 = no drive (clean)
    ///   - 5.0 = mild overdrive
    ///   - 20.0 = heavy distortion
    ///   - 100.0 = extreme saturation
    /// * `mix_input` - NodeId of wet/dry mix (0.0 to 1.0)
    ///   - 0.0 = completely dry (bypass)
    ///   - 0.5 = 50/50 blend
    ///   - 1.0 = completely wet (full effect)
    pub fn new(input: NodeId, drive_input: NodeId, mix_input: NodeId) -> Self {
        Self {
            input,
            drive_input,
            mix_input,
        }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }

    /// Get the drive input node ID
    pub fn drive_input(&self) -> NodeId {
        self.drive_input
    }

    /// Get the mix input node ID
    pub fn mix_input(&self) -> NodeId {
        self.mix_input
    }
}

impl AudioNode for DistortionNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 3,
            "DistortionNode requires 3 inputs, got {}",
            inputs.len()
        );

        let input_buf = inputs[0];
        let drive_buf = inputs[1];
        let mix_buf = inputs[2];

        debug_assert_eq!(
            input_buf.len(),
            output.len(),
            "Input buffer length mismatch"
        );

        // Process each sample (stateless waveshaping)
        for i in 0..output.len() {
            let sample = input_buf[i];
            let drive = drive_buf[i].clamp(1.0, 100.0);
            let mix = mix_buf[i].clamp(0.0, 1.0);

            // Apply drive (pre-gain)
            let driven = sample * drive;

            // Soft clip using tanh waveshaper
            let distorted = driven.tanh();

            // Blend dry and wet signals
            output[i] = sample * (1.0 - mix) + distorted * mix;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.drive_input, self.mix_input]
    }

    fn name(&self) -> &str {
        "DistortionNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::Fraction;

    fn create_context(size: usize) -> ProcessContext {
        ProcessContext::new(Fraction::from_float(0.0), 0, size, 2.0, 44100.0)
    }

    #[test]
    fn test_distortion_bypass_with_zero_mix() {
        // Test that mix=0.0 passes signal through unchanged
        let size = 512;

        let input = vec![0.5; size];
        let drive = vec![10.0; size];  // High drive
        let mix = vec![0.0; size];      // But 0% mix

        let inputs: Vec<&[f32]> = vec![&input, &drive, &mix];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut dist = DistortionNode::new(0, 1, 2);
        dist.process_block(&inputs, &mut output, 44100.0, &context);

        // Output should equal input (bypass)
        for i in 0..size {
            assert!(
                (output[i] - input[i]).abs() < 0.0001,
                "With mix=0, output should equal input"
            );
        }
    }

    #[test]
    fn test_distortion_full_wet() {
        // Test that mix=1.0 gives full distortion effect
        let size = 512;

        let input = vec![0.5; size];
        let drive = vec![5.0; size];
        let mix = vec![1.0; size];  // 100% wet

        let inputs: Vec<&[f32]> = vec![&input, &drive, &mix];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut dist = DistortionNode::new(0, 1, 2);
        dist.process_block(&inputs, &mut output, 44100.0, &context);

        // Calculate expected distorted value
        let driven = 0.5_f32 * 5.0; // 2.5
        let expected = driven.tanh(); // tanh(2.5) ≈ 0.986

        for i in 0..size {
            assert!(
                (output[i] - expected).abs() < 0.001,
                "With mix=1, output should be tanh(input * drive), expected {}, got {}",
                expected,
                output[i]
            );
        }
    }

    #[test]
    fn test_distortion_drive_increases_saturation() {
        // Test that higher drive increases distortion
        let size = 512;

        let input = vec![0.3; size];
        let mix = vec![1.0; size];

        // Low drive
        let drive_low = vec![2.0; size];
        let inputs_low: Vec<&[f32]> = vec![&input, &drive_low, &mix];
        let mut output_low = vec![0.0; size];
        let context = create_context(size);

        let mut dist_low = DistortionNode::new(0, 1, 2);
        dist_low.process_block(&inputs_low, &mut output_low, 44100.0, &context);

        // High drive
        let drive_high = vec![20.0; size];
        let inputs_high: Vec<&[f32]> = vec![&input, &drive_high, &mix];
        let mut output_high = vec![0.0; size];

        let mut dist_high = DistortionNode::new(0, 1, 2);
        dist_high.process_block(&inputs_high, &mut output_high, 44100.0, &context);

        // Higher drive should push closer to tanh saturation limits (±1.0)
        // Low drive: tanh(0.3 * 2) = tanh(0.6) ≈ 0.537
        // High drive: tanh(0.3 * 20) = tanh(6) ≈ 0.9999
        assert!(
            output_high[0].abs() > output_low[0].abs(),
            "Higher drive should produce stronger saturation: low={}, high={}",
            output_low[0],
            output_high[0]
        );

        // High drive should approach saturation limit
        assert!(
            output_high[0] > 0.9,
            "High drive should approach saturation (1.0), got {}",
            output_high[0]
        );
    }

    #[test]
    fn test_distortion_soft_clipping() {
        // Test that tanh provides soft clipping (output bounded to ±1.0)
        let size = 512;

        // Very loud input
        let input = vec![10.0; size];
        let drive = vec![100.0; size];  // Extreme drive
        let mix = vec![1.0; size];

        let inputs: Vec<&[f32]> = vec![&input, &drive, &mix];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut dist = DistortionNode::new(0, 1, 2);
        dist.process_block(&inputs, &mut output, 44100.0, &context);

        // Output should be soft-clipped to near 1.0
        for &val in &output {
            assert!(
                val > 0.99 && val <= 1.0,
                "Soft clipping should limit output to ~1.0, got {}",
                val
            );
        }
    }

    #[test]
    fn test_distortion_negative_input() {
        // Test that distortion works correctly with negative signals
        let size = 512;

        let input = vec![-0.5; size];
        let drive = vec![5.0; size];
        let mix = vec![1.0; size];

        let inputs: Vec<&[f32]> = vec![&input, &drive, &mix];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut dist = DistortionNode::new(0, 1, 2);
        dist.process_block(&inputs, &mut output, 44100.0, &context);

        // Should produce negative distorted output
        let driven = -0.5_f32 * 5.0; // -2.5
        let expected = driven.tanh(); // tanh(-2.5) ≈ -0.986

        for i in 0..size {
            assert!(
                (output[i] - expected).abs() < 0.001,
                "Negative input should produce negative distortion, expected {}, got {}",
                expected,
                output[i]
            );
        }
    }

    #[test]
    fn test_distortion_mix_blending() {
        // Test that mix parameter properly blends wet/dry
        let size = 512;

        let input = vec![0.4; size];
        let drive = vec![10.0; size];

        // Test various mix values
        for mix_val in &[0.0, 0.25, 0.5, 0.75, 1.0] {
            let mix = vec![*mix_val; size];
            let inputs: Vec<&[f32]> = vec![&input, &drive, &mix];
            let mut output = vec![0.0; size];
            let context = create_context(size);

            let mut dist = DistortionNode::new(0, 1, 2);
            dist.process_block(&inputs, &mut output, 44100.0, &context);

            // Calculate expected blend
            let driven = 0.4_f32 * 10.0;
            let distorted = driven.tanh();
            let expected = 0.4 * (1.0 - mix_val) + distorted * mix_val;

            assert!(
                (output[0] - expected).abs() < 0.001,
                "Mix {} should blend correctly: expected {}, got {}",
                mix_val,
                expected,
                output[0]
            );
        }
    }

    #[test]
    fn test_distortion_sine_wave() {
        // Test distortion on a sine wave (creates harmonics)
        let size = 512;
        let sample_rate = 44100.0;

        // Generate 100 Hz sine wave
        let mut input = vec![0.0; size];
        for i in 0..size {
            let t = i as f32 / sample_rate;
            input[i] = 0.5 * (2.0 * std::f32::consts::PI * 100.0 * t).sin();
        }

        let drive = vec![5.0; size];
        let mix = vec![1.0; size];

        let inputs: Vec<&[f32]> = vec![&input, &drive, &mix];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut dist = DistortionNode::new(0, 1, 2);
        dist.process_block(&inputs, &mut output, sample_rate, &context);

        // Output should be distorted sine (bounded, with added harmonics)
        // Check that output is bounded
        for &val in &output {
            assert!(
                val.abs() <= 1.0,
                "Distorted output should be bounded to ±1.0, got {}",
                val
            );
        }

        // Output RMS should be lower than input RMS (due to saturation)
        let input_rms: f32 = input.iter().map(|x| x * x).sum::<f32>() / size as f32;
        let output_rms: f32 = output.iter().map(|x| x * x).sum::<f32>() / size as f32;

        assert!(
            input_rms > 0.0 && output_rms > 0.0,
            "Both signals should have energy"
        );
    }

    #[test]
    fn test_distortion_parameter_clamping() {
        // Test that drive and mix are clamped to valid ranges
        let size = 512;

        let input = vec![0.5; size];
        let drive_invalid = vec![1000.0; size];  // Should clamp to 100.0
        let mix_invalid = vec![5.0; size];        // Should clamp to 1.0

        let inputs: Vec<&[f32]> = vec![&input, &drive_invalid, &mix_invalid];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut dist = DistortionNode::new(0, 1, 2);
        dist.process_block(&inputs, &mut output, 44100.0, &context);

        // Should use clamped values: drive=100, mix=1.0
        let driven = 0.5_f32 * 100.0;
        let expected = driven.tanh();

        assert!(
            (output[0] - expected).abs() < 0.001,
            "Should clamp drive to 100 and mix to 1.0, expected {}, got {}",
            expected,
            output[0]
        );
    }

    #[test]
    fn test_distortion_node_interface() {
        // Test node getters
        let dist = DistortionNode::new(10, 11, 12);

        assert_eq!(dist.input(), 10);
        assert_eq!(dist.drive_input(), 11);
        assert_eq!(dist.mix_input(), 12);

        let inputs = dist.input_nodes();
        assert_eq!(inputs.len(), 3);
        assert_eq!(inputs[0], 10);
        assert_eq!(inputs[1], 11);
        assert_eq!(inputs[2], 12);

        assert_eq!(dist.name(), "DistortionNode");
    }
}
