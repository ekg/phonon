/// Compressor node - dynamics compression with attack/release
///
/// This node provides smooth dynamic range compression. It reduces the level
/// of loud signals above the threshold according to the specified ratio.
/// The gain reduction envelope follows the input signal with attack and release times.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Compressor state for envelope follower
#[derive(Debug, Clone)]
struct CompressorState {
    envelope: f32,  // Current gain reduction envelope in dB
}

impl Default for CompressorState {
    fn default() -> Self {
        Self { envelope: 0.0 }
    }
}

/// Compressor node: smooth dynamics compression
///
/// The compression algorithm:
/// ```text
/// 1. Convert input to dB: input_db = 20 * log10(abs(sample))
/// 2. Calculate gain reduction when above threshold:
///    over = input_db - threshold
///    if over > 0:
///        gain_reduction = over * (1 - 1/ratio)
///    else:
///        gain_reduction = 0
/// 3. Smooth with attack/release envelope follower:
///    attack_coeff = exp(-1 / (attack_time * sample_rate))
///    release_coeff = exp(-1 / (release_time * sample_rate))
///    if target_gain < envelope:
///        envelope = attack_coeff * envelope + (1 - attack_coeff) * target_gain
///    else:
///        envelope = release_coeff * envelope + (1 - release_coeff) * target_gain
/// 4. Apply gain: output = input * 10^(envelope / 20)
/// ```
///
/// This provides:
/// - Smooth compression with no clicks
/// - Independent attack and release times
/// - Variable compression ratio
/// - Transparent when below threshold
///
/// # Example
/// ```ignore
/// // Compress signal with 4:1 ratio, -10 dB threshold
/// let input = OscillatorNode::new(0, Waveform::Sine);  // NodeId 1
/// let threshold = ConstantNode::new(-10.0);            // NodeId 2 (-10 dB)
/// let ratio = ConstantNode::new(4.0);                  // NodeId 3 (4:1)
/// let attack = ConstantNode::new(0.01);                // NodeId 4 (10ms)
/// let release = ConstantNode::new(0.1);                // NodeId 5 (100ms)
/// let comp = CompressorNode::new(1, 2, 3, 4, 5);       // NodeId 6
/// ```
pub struct CompressorNode {
    input: NodeId,
    threshold_input: NodeId,  // Threshold in dB (e.g., -10.0)
    ratio_input: NodeId,      // Compression ratio (1.0 to 20.0)
    attack_input: NodeId,     // Attack time in seconds (0.001 to 1.0)
    release_input: NodeId,    // Release time in seconds (0.01 to 3.0)
    state: CompressorState,   // Envelope follower state
}

impl CompressorNode {
    /// Create a new compressor node
    ///
    /// # Arguments
    /// * `input` - NodeId of signal to compress
    /// * `threshold_input` - NodeId of threshold in dB (e.g., -10.0)
    /// * `ratio_input` - NodeId of compression ratio (1.0 = no compression, 20.0 = heavy)
    /// * `attack_input` - NodeId of attack time in seconds (how fast compression kicks in)
    /// * `release_input` - NodeId of release time in seconds (how fast compression releases)
    pub fn new(
        input: NodeId,
        threshold_input: NodeId,
        ratio_input: NodeId,
        attack_input: NodeId,
        release_input: NodeId,
    ) -> Self {
        Self {
            input,
            threshold_input,
            ratio_input,
            attack_input,
            release_input,
            state: CompressorState::default(),
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

    /// Get the ratio input node ID
    pub fn ratio_input(&self) -> NodeId {
        self.ratio_input
    }

    /// Get the attack input node ID
    pub fn attack_input(&self) -> NodeId {
        self.attack_input
    }

    /// Get the release input node ID
    pub fn release_input(&self) -> NodeId {
        self.release_input
    }

    /// Reset compressor state
    pub fn reset(&mut self) {
        self.state = CompressorState::default();
    }
}

impl AudioNode for CompressorNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 5,
            "CompressorNode requires 5 inputs, got {}",
            inputs.len()
        );

        let input_buf = inputs[0];
        let threshold_buf = inputs[1];
        let ratio_buf = inputs[2];
        let attack_buf = inputs[3];
        let release_buf = inputs[4];

        debug_assert_eq!(
            input_buf.len(),
            output.len(),
            "Input buffer length mismatch"
        );
        debug_assert_eq!(
            threshold_buf.len(),
            output.len(),
            "Threshold buffer length mismatch"
        );
        debug_assert_eq!(
            ratio_buf.len(),
            output.len(),
            "Ratio buffer length mismatch"
        );
        debug_assert_eq!(
            attack_buf.len(),
            output.len(),
            "Attack buffer length mismatch"
        );
        debug_assert_eq!(
            release_buf.len(),
            output.len(),
            "Release buffer length mismatch"
        );

        // Apply compression with envelope follower
        for i in 0..output.len() {
            let sample = input_buf[i];
            let threshold_db = threshold_buf[i];
            let ratio = ratio_buf[i].clamp(1.0, 20.0);
            let attack_time = attack_buf[i].max(0.0001); // Min 0.1ms
            let release_time = release_buf[i].max(0.001); // Min 1ms

            // Calculate attack/release coefficients (exponential smoothing)
            let attack_coeff = (-1.0 / (attack_time * sample_rate)).exp();
            let release_coeff = (-1.0 / (release_time * sample_rate)).exp();

            // Convert input to dB (with safety for zero/negative)
            let abs_sample = sample.abs().max(1e-10); // Prevent log(0)
            let input_db = 20.0 * abs_sample.log10();

            // Calculate target gain reduction
            let over_db = input_db - threshold_db;
            let target_gain_reduction = if over_db > 0.0 {
                // Apply compression ratio
                over_db * (1.0 - 1.0 / ratio)
            } else {
                0.0
            };

            // Smooth gain reduction with attack/release envelope
            if target_gain_reduction > self.state.envelope {
                // Attack: fast response when gain reduction increases (signal gets louder)
                self.state.envelope = attack_coeff * self.state.envelope
                    + (1.0 - attack_coeff) * target_gain_reduction;
            } else {
                // Release: slow response when gain reduction decreases (signal gets quieter)
                self.state.envelope = release_coeff * self.state.envelope
                    + (1.0 - release_coeff) * target_gain_reduction;
            }

            // Apply negative gain reduction (reduce level)
            let gain_linear = 10.0_f32.powf(-self.state.envelope / 20.0);
            output[i] = sample * gain_linear;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![
            self.input,
            self.threshold_input,
            self.ratio_input,
            self.attack_input,
            self.release_input,
        ]
    }

    fn name(&self) -> &str {
        "CompressorNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    fn create_context(size: usize) -> ProcessContext {
        ProcessContext::new(Fraction::from_float(0.0), 0, size, 2.0, 44100.0)
    }

    #[test]
    fn test_compressor_below_threshold_unchanged() {
        // Signals below threshold should pass unchanged
        let mut comp = CompressorNode::new(0, 1, 2, 3, 4);

        // Input: 0.1 (-20 dB), Threshold: -10 dB, Ratio: 4:1
        let input = vec![0.1; 512];
        let threshold = vec![-10.0; 512];
        let ratio = vec![4.0; 512];
        let attack = vec![0.01; 512];
        let release = vec![0.1; 512];
        let inputs = vec![
            input.as_slice(),
            threshold.as_slice(),
            ratio.as_slice(),
            attack.as_slice(),
            release.as_slice(),
        ];

        let mut output = vec![0.0; 512];
        let context = create_context(512);

        comp.process_block(&inputs, &mut output, 44100.0, &context);

        // Signal below threshold should be mostly unchanged (within 1%)
        for sample in &output {
            assert!((*sample - 0.1).abs() < 0.001);
        }
    }

    #[test]
    fn test_compressor_reduces_peaks_above_threshold() {
        // Signals above threshold should be reduced
        let mut comp = CompressorNode::new(0, 1, 2, 3, 4);

        // Input: 1.0 (0 dB), Threshold: -20 dB, Ratio: 4:1
        // Expected gain reduction: 20 dB over * (1 - 1/4) = 15 dB reduction
        let input = vec![1.0; 512];
        let threshold = vec![-20.0; 512];
        let ratio = vec![4.0; 512];
        let attack = vec![0.001; 512]; // Very fast attack
        let release = vec![0.1; 512];
        let inputs = vec![
            input.as_slice(),
            threshold.as_slice(),
            ratio.as_slice(),
            attack.as_slice(),
            release.as_slice(),
        ];

        let mut output = vec![0.0; 512];
        let context = create_context(512);

        comp.process_block(&inputs, &mut output, 44100.0, &context);

        // Output should be less than input (compressed)
        let avg_output: f32 = output.iter().skip(100).take(400).sum::<f32>() / 400.0;
        assert!(avg_output < 1.0);
        assert!(avg_output > 0.1); // But not completely crushed
    }

    #[test]
    fn test_compressor_ratio_affects_amount() {
        // Higher ratio = more compression
        let mut comp_light = CompressorNode::new(0, 1, 2, 3, 4);
        let mut comp_heavy = CompressorNode::new(0, 1, 2, 3, 4);

        let input = vec![1.0; 512];
        let threshold = vec![-20.0; 512];
        let attack = vec![0.001; 512];
        let release = vec![0.1; 512];

        // Light compression (2:1)
        let ratio_light = vec![2.0; 512];
        let inputs_light = vec![
            input.as_slice(),
            threshold.as_slice(),
            ratio_light.as_slice(),
            attack.as_slice(),
            release.as_slice(),
        ];
        let mut output_light = vec![0.0; 512];
        let context = create_context(512);
        comp_light.process_block(&inputs_light, &mut output_light, 44100.0, &context);

        // Heavy compression (10:1)
        let ratio_heavy = vec![10.0; 512];
        let inputs_heavy = vec![
            input.as_slice(),
            threshold.as_slice(),
            ratio_heavy.as_slice(),
            attack.as_slice(),
            release.as_slice(),
        ];
        let mut output_heavy = vec![0.0; 512];
        comp_heavy.process_block(&inputs_heavy, &mut output_heavy, 44100.0, &context);

        // Heavy compression should have lower average level
        let avg_light: f32 = output_light.iter().skip(100).take(400).sum::<f32>() / 400.0;
        let avg_heavy: f32 = output_heavy.iter().skip(100).take(400).sum::<f32>() / 400.0;

        assert!(avg_heavy < avg_light);
    }

    #[test]
    fn test_compressor_attack_controls_onset_speed() {
        // Slow attack should have gradual compression onset
        let mut comp_slow = CompressorNode::new(0, 1, 2, 3, 4);

        let input = vec![1.0; 512];
        let threshold = vec![-20.0; 512];
        let ratio = vec![4.0; 512];
        let attack = vec![0.1; 512]; // Slow attack (100ms)
        let release = vec![0.1; 512];
        let inputs = vec![
            input.as_slice(),
            threshold.as_slice(),
            ratio.as_slice(),
            attack.as_slice(),
            release.as_slice(),
        ];

        let mut output = vec![0.0; 512];
        let context = create_context(512);

        comp_slow.process_block(&inputs, &mut output, 44100.0, &context);

        // First sample should be less compressed than later samples
        assert!(output[0] > output[100]);
        assert!(output[100] > output[400]);
    }

    #[test]
    fn test_compressor_release_controls_decay_speed() {
        // Test release phase by going from loud to quiet
        let mut comp = CompressorNode::new(0, 1, 2, 3, 4);

        // First half loud, second half quiet
        let mut input = vec![1.0; 512];
        for i in 256..512 {
            input[i] = 0.01; // Very quiet (-40 dB)
        }

        let threshold = vec![-20.0; 512];
        let ratio = vec![4.0; 512];
        let attack = vec![0.001; 512]; // Fast attack
        let release = vec![0.1; 512];   // Slow release
        let inputs = vec![
            input.as_slice(),
            threshold.as_slice(),
            ratio.as_slice(),
            attack.as_slice(),
            release.as_slice(),
        ];

        let mut output = vec![0.0; 512];
        let context = create_context(512);

        comp.process_block(&inputs, &mut output, 44100.0, &context);

        // After transition to quiet, should gradually release
        // Sample at transition should be more compressed than later samples
        assert!(output[260] < output[300]);
        assert!(output[300] < output[400]);
    }

    #[test]
    fn test_compressor_no_compression_at_ratio_1() {
        // Ratio of 1.0 should produce no compression
        let mut comp = CompressorNode::new(0, 1, 2, 3, 4);

        let input = vec![1.0; 512];
        let threshold = vec![-20.0; 512];
        let ratio = vec![1.0; 512]; // No compression
        let attack = vec![0.01; 512];
        let release = vec![0.1; 512];
        let inputs = vec![
            input.as_slice(),
            threshold.as_slice(),
            ratio.as_slice(),
            attack.as_slice(),
            release.as_slice(),
        ];

        let mut output = vec![0.0; 512];
        let context = create_context(512);

        comp.process_block(&inputs, &mut output, 44100.0, &context);

        // Output should match input (no compression)
        for sample in &output {
            assert!((*sample - 1.0).abs() < 0.01);
        }
    }

    #[test]
    fn test_compressor_preserves_sign() {
        // Compressor should preserve positive and negative signs
        let mut comp = CompressorNode::new(0, 1, 2, 3, 4);

        let input = vec![1.0, -1.0, 0.8, -0.8];
        let threshold = vec![-20.0; 4];
        let ratio = vec![4.0; 4];
        let attack = vec![0.001; 4];
        let release = vec![0.1; 4];
        let inputs = vec![
            input.as_slice(),
            threshold.as_slice(),
            ratio.as_slice(),
            attack.as_slice(),
            release.as_slice(),
        ];

        let mut output = vec![0.0; 4];
        let context = create_context(4);

        comp.process_block(&inputs, &mut output, 44100.0, &context);

        // Check signs are preserved
        assert!(output[0] > 0.0); // Positive input → positive output
        assert!(output[1] < 0.0); // Negative input → negative output
        assert!(output[2] > 0.0); // Positive input → positive output
        assert!(output[3] < 0.0); // Negative input → negative output
    }

    #[test]
    fn test_compressor_threshold_boundary() {
        // Test signal exactly at threshold
        let mut comp = CompressorNode::new(0, 1, 2, 3, 4);

        // Input at exactly -10 dB = 0.316
        let input = vec![0.316; 512];
        let threshold = vec![-10.0; 512];
        let ratio = vec![4.0; 512];
        let attack = vec![0.01; 512];
        let release = vec![0.1; 512];
        let inputs = vec![
            input.as_slice(),
            threshold.as_slice(),
            ratio.as_slice(),
            attack.as_slice(),
            release.as_slice(),
        ];

        let mut output = vec![0.0; 512];
        let context = create_context(512);

        comp.process_block(&inputs, &mut output, 44100.0, &context);

        // At threshold, very minimal compression
        let avg_output: f32 = output.iter().skip(100).take(400).sum::<f32>() / 400.0;
        assert!((avg_output - 0.316).abs() < 0.05);
    }

    #[test]
    fn test_compressor_pattern_modulation_threshold() {
        // Test varying threshold parameter
        let mut comp = CompressorNode::new(0, 1, 2, 3, 4);

        let input = vec![1.0; 512];
        let mut threshold = vec![-30.0; 512]; // Low threshold
        for i in 256..512 {
            threshold[i] = -5.0; // High threshold
        }
        let ratio = vec![4.0; 512];
        let attack = vec![0.001; 512];
        let release = vec![0.001; 512]; // Fast release
        let inputs = vec![
            input.as_slice(),
            threshold.as_slice(),
            ratio.as_slice(),
            attack.as_slice(),
            release.as_slice(),
        ];

        let mut output = vec![0.0; 512];
        let context = create_context(512);

        comp.process_block(&inputs, &mut output, 44100.0, &context);

        // Low threshold = more compression = lower level
        let avg_low: f32 = output.iter().skip(50).take(100).sum::<f32>() / 100.0;
        // High threshold = less compression = higher level
        let avg_high: f32 = output.iter().skip(300).take(100).sum::<f32>() / 100.0;

        assert!(avg_high > avg_low);
    }

    #[test]
    fn test_compressor_pattern_modulation_ratio() {
        // Test varying ratio parameter
        let mut comp = CompressorNode::new(0, 1, 2, 3, 4);

        let input = vec![1.0; 512];
        let threshold = vec![-20.0; 512];
        let mut ratio = vec![2.0; 512]; // Light compression
        for i in 256..512 {
            ratio[i] = 10.0; // Heavy compression
        }
        let attack = vec![0.001; 512];
        let release = vec![0.001; 512];
        let inputs = vec![
            input.as_slice(),
            threshold.as_slice(),
            ratio.as_slice(),
            attack.as_slice(),
            release.as_slice(),
        ];

        let mut output = vec![0.0; 512];
        let context = create_context(512);

        comp.process_block(&inputs, &mut output, 44100.0, &context);

        // Light compression = higher level
        let avg_light: f32 = output.iter().skip(50).take(100).sum::<f32>() / 100.0;
        // Heavy compression = lower level
        let avg_heavy: f32 = output.iter().skip(300).take(100).sum::<f32>() / 100.0;

        assert!(avg_light > avg_heavy);
    }

    #[test]
    fn test_compressor_dependencies() {
        let comp = CompressorNode::new(5, 10, 15, 20, 25);
        let deps = comp.input_nodes();

        assert_eq!(deps.len(), 5);
        assert_eq!(deps[0], 5);  // input
        assert_eq!(deps[1], 10); // threshold
        assert_eq!(deps[2], 15); // ratio
        assert_eq!(deps[3], 20); // attack
        assert_eq!(deps[4], 25); // release
    }

    #[test]
    fn test_compressor_zero_input_safe() {
        // Verify compressor handles zero/very quiet input safely
        let mut comp = CompressorNode::new(0, 1, 2, 3, 4);

        let input = vec![0.0, 0.00001, -0.00001, 0.0];
        let threshold = vec![-20.0; 4];
        let ratio = vec![4.0; 4];
        let attack = vec![0.01; 4];
        let release = vec![0.1; 4];
        let inputs = vec![
            input.as_slice(),
            threshold.as_slice(),
            ratio.as_slice(),
            attack.as_slice(),
            release.as_slice(),
        ];

        let mut output = vec![0.0; 4];
        let context = create_context(4);

        comp.process_block(&inputs, &mut output, 44100.0, &context);

        // Should not produce NaN or infinity
        for sample in &output {
            assert!(sample.is_finite());
        }
    }

    #[test]
    fn test_compressor_heavy_limiting() {
        // Test very high ratio (approaching limiting)
        let mut comp = CompressorNode::new(0, 1, 2, 3, 4);

        let input = vec![1.0; 512];
        let threshold = vec![-20.0; 512];
        let ratio = vec![20.0; 512]; // Very heavy compression
        let attack = vec![0.001; 512];
        let release = vec![0.1; 512];
        let inputs = vec![
            input.as_slice(),
            threshold.as_slice(),
            ratio.as_slice(),
            attack.as_slice(),
            release.as_slice(),
        ];

        let mut output = vec![0.0; 512];
        let context = create_context(512);

        comp.process_block(&inputs, &mut output, 44100.0, &context);

        // Very heavy compression should significantly reduce level
        // With 20:1 ratio, 1.0 (0 dB) input, -20 dB threshold:
        // Gain reduction = 20 dB * (1 - 1/20) = 19 dB
        // Output ≈ 10^(-19/20) ≈ 0.112
        // But attack smoothing means it builds up, so later samples are more compressed
        let late_avg: f32 = output.iter().skip(400).take(100).sum::<f32>() / 100.0;
        assert!(late_avg < 0.3, "Late average was {}, expected < 0.3", late_avg);
        assert!(late_avg > 0.0, "Late average was {}, expected > 0.0", late_avg);

        // Early samples should be less compressed (attack not yet fully engaged)
        let early_avg: f32 = output.iter().skip(0).take(10).sum::<f32>() / 10.0;
        assert!(early_avg > late_avg, "Early avg {} should be > late avg {}", early_avg, late_avg);
    }

    #[test]
    fn test_compressor_mixed_levels() {
        // Test with varying input levels
        let mut comp = CompressorNode::new(0, 1, 2, 3, 4);

        let input = vec![0.1, 0.3, 0.6, 1.0, 0.8, 0.4, 0.2, 0.05];
        let threshold = vec![-20.0; 8];
        let ratio = vec![4.0; 8];
        let attack = vec![0.001; 8];
        let release = vec![0.01; 8];
        let inputs = vec![
            input.as_slice(),
            threshold.as_slice(),
            ratio.as_slice(),
            attack.as_slice(),
            release.as_slice(),
        ];

        let mut output = vec![0.0; 8];
        let context = create_context(8);

        comp.process_block(&inputs, &mut output, 44100.0, &context);

        // All outputs should be valid and finite
        for sample in &output {
            assert!(sample.is_finite());
            assert!(sample.abs() <= 1.5); // Reasonable range
        }

        // Louder inputs should still be louder after compression (preserves dynamics)
        assert!(output[3] > output[0]); // 1.0 > 0.1
    }
}
