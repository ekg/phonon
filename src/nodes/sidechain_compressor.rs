/// Sidechain compressor node - compression triggered by external signal
///
/// This node provides sidechain compression where the gain reduction is triggered
/// by an external sidechain signal rather than the input itself. Classic use case
/// is "ducking" where a kick drum causes bass/pads to reduce in level.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Sidechain compressor state for envelope follower
#[derive(Debug, Clone)]
struct SidechainCompressorState {
    envelope: f32,  // Current gain reduction envelope in dB
}

impl Default for SidechainCompressorState {
    fn default() -> Self {
        Self { envelope: 0.0 }
    }
}

/// Sidechain compressor node: compression controlled by external signal
///
/// The sidechain compression algorithm:
/// ```text
/// 1. Analyze SIDECHAIN signal level (not main input)
/// 2. Calculate gain reduction based on sidechain level vs threshold
/// 3. Apply gain reduction to MAIN INPUT signal
/// ```
///
/// This creates the "ducking" effect:
/// - Kick drum (sidechain) triggers compression of bass (main input)
/// - Voice (sidechain) triggers compression of music (main input)
/// - Rhythmic modulation of any sound by another
///
/// # Example
/// ```ignore
/// // Kick ducks bass (EDM-style)
/// let kick = SamplePlayerNode::new(...);  // NodeId 1
/// let bass = OscillatorNode::new(...);     // NodeId 2
/// let threshold = ConstantNode::new(-10.0); // NodeId 3
/// let ratio = ConstantNode::new(4.0);       // NodeId 4
/// let attack = ConstantNode::new(0.01);     // NodeId 5
/// let release = ConstantNode::new(0.1);     // NodeId 6
///
/// let ducked = SidechainCompressorNode::new(
///     2,  // bass (main input)
///     1,  // kick (sidechain)
///     3, 4, 5, 6
/// );
/// ```
pub struct SidechainCompressorNode {
    main_input: NodeId,        // Signal to compress (e.g., bass)
    sidechain_input: NodeId,   // Signal controlling compression (e.g., kick)
    threshold_input: NodeId,   // Threshold in dB
    ratio_input: NodeId,       // Compression ratio
    attack_input: NodeId,      // Attack time in seconds
    release_input: NodeId,     // Release time in seconds
    state: SidechainCompressorState,
}

impl SidechainCompressorNode {
    /// SidechainCompressor - Compression triggered by external signal
    ///
    /// Analyze sidechain signal level, apply compression to main signal.
    /// Classic "ducking" effect for EDM, podcasts, dynamic mixing.
    ///
    /// # Parameters
    /// - `main_input`: Signal to compress (e.g., bass)
    /// - `sidechain_input`: Signal controlling compression (e.g., kick)
    /// - `threshold_input`: Threshold in dB (e.g., -10.0)
    /// - `ratio_input`: Compression ratio (1=none, 4=typical, 20=heavy)
    /// - `attack_input`: Attack time in seconds
    /// - `release_input`: Release time in seconds
    ///
    /// # Example
    /// ```phonon
    /// ~kick: s "bd*4"
    /// ~bass: saw 55
    /// ~ducked: ~bass # sidechain_comp ~kick -10 4.0 0.01 0.1
    /// ```
    pub fn new(
        main_input: NodeId,
        sidechain_input: NodeId,
        threshold_input: NodeId,
        ratio_input: NodeId,
        attack_input: NodeId,
        release_input: NodeId,
    ) -> Self {
        Self {
            main_input,
            sidechain_input,
            threshold_input,
            ratio_input,
            attack_input,
            release_input,
            state: SidechainCompressorState::default(),
        }
    }

    /// Get the main input node ID
    pub fn main_input(&self) -> NodeId {
        self.main_input
    }

    /// Get the sidechain input node ID
    pub fn sidechain_input(&self) -> NodeId {
        self.sidechain_input
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
        self.state = SidechainCompressorState::default();
    }
}

impl AudioNode for SidechainCompressorNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 6,
            "SidechainCompressorNode requires 6 inputs, got {}",
            inputs.len()
        );

        let main_buf = inputs[0];      // Signal to compress
        let sidechain_buf = inputs[1]; // Signal controlling compression
        let threshold_buf = inputs[2];
        let ratio_buf = inputs[3];
        let attack_buf = inputs[4];
        let release_buf = inputs[5];

        debug_assert_eq!(
            main_buf.len(),
            output.len(),
            "Main buffer length mismatch"
        );
        debug_assert_eq!(
            sidechain_buf.len(),
            output.len(),
            "Sidechain buffer length mismatch"
        );

        // Apply sidechain compression
        for i in 0..output.len() {
            let main_sample = main_buf[i];
            let sidechain_sample = sidechain_buf[i];
            let threshold_db = threshold_buf[i];
            let ratio = ratio_buf[i].clamp(1.0, 20.0);
            let attack_time = attack_buf[i].max(0.0001); // Min 0.1ms
            let release_time = release_buf[i].max(0.001); // Min 1ms

            // Calculate attack/release coefficients
            let attack_coeff = (-1.0 / (attack_time * sample_rate)).exp();
            let release_coeff = (-1.0 / (release_time * sample_rate)).exp();

            // Convert SIDECHAIN signal to dB (not main input!)
            let abs_sidechain = sidechain_sample.abs().max(1e-10);
            let sidechain_db = 20.0 * abs_sidechain.log10();

            // Calculate target gain reduction based on SIDECHAIN level
            let over_db = sidechain_db - threshold_db;
            let target_gain_reduction = if over_db > 0.0 {
                over_db * (1.0 - 1.0 / ratio)
            } else {
                0.0
            };

            // Smooth gain reduction with attack/release envelope
            if target_gain_reduction > self.state.envelope {
                // Attack: fast response when sidechain gets louder
                self.state.envelope = attack_coeff * self.state.envelope
                    + (1.0 - attack_coeff) * target_gain_reduction;
            } else {
                // Release: slow response when sidechain gets quieter
                self.state.envelope = release_coeff * self.state.envelope
                    + (1.0 - release_coeff) * target_gain_reduction;
            }

            // Apply negative gain reduction to MAIN signal
            let gain_linear = 10.0_f32.powf(-self.state.envelope / 20.0);
            output[i] = main_sample * gain_linear;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![
            self.main_input,
            self.sidechain_input,
            self.threshold_input,
            self.ratio_input,
            self.attack_input,
            self.release_input,
        ]
    }

    fn name(&self) -> &str {
        "SidechainCompressorNode"
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
    fn test_sidechain_quiet_sidechain_no_compression() {
        // When sidechain is quiet, main signal should pass unchanged
        let mut comp = SidechainCompressorNode::new(0, 1, 2, 3, 4, 5);

        let main_input = vec![0.5; 512];        // Loud main signal
        let sidechain = vec![0.01; 512];       // Quiet sidechain (below threshold)
        let threshold = vec![-10.0; 512];
        let ratio = vec![4.0; 512];
        let attack = vec![0.01; 512];
        let release = vec![0.1; 512];

        let inputs = vec![
            main_input.as_slice(),
            sidechain.as_slice(),
            threshold.as_slice(),
            ratio.as_slice(),
            attack.as_slice(),
            release.as_slice(),
        ];

        let mut output = vec![0.0; 512];
        let context = create_context(512);

        comp.process_block(&inputs, &mut output, 44100.0, &context);

        // Main signal should be mostly unchanged (< 1% change)
        for sample in &output {
            assert!((*sample - 0.5).abs() < 0.01);
        }
    }

    #[test]
    fn test_sidechain_loud_sidechain_compresses_main() {
        // When sidechain is loud, main signal should be compressed
        let mut comp = SidechainCompressorNode::new(0, 1, 2, 3, 4, 5);

        let main_input = vec![0.5; 512];        // Main signal stays constant
        let sidechain = vec![1.0; 512];         // Loud sidechain (above threshold)
        let threshold = vec![-20.0; 512];
        let ratio = vec![4.0; 512];
        let attack = vec![0.001; 512];          // Fast attack
        let release = vec![0.1; 512];

        let inputs = vec![
            main_input.as_slice(),
            sidechain.as_slice(),
            threshold.as_slice(),
            ratio.as_slice(),
            attack.as_slice(),
            release.as_slice(),
        ];

        let mut output = vec![0.0; 512];
        let context = create_context(512);

        comp.process_block(&inputs, &mut output, 44100.0, &context);

        // Main signal should be compressed (reduced)
        // With 4:1 ratio, -20dB threshold, 1.0 sidechain (0 dB):
        // Gain reduction = 20dB * (1 - 1/4) = 15dB
        // Expected: 0.5 * 10^(-15/20) ≈ 0.089
        let avg_output: f32 = output.iter().skip(100).take(400).sum::<f32>() / 400.0;
        assert!(avg_output < 0.5, "Output {} should be less than input 0.5", avg_output);
        assert!(avg_output > 0.05, "Output {} should not be completely crushed", avg_output);
        assert!(avg_output < 0.15, "Output {} should be compressed (< 0.15)", avg_output);
    }

    #[test]
    fn test_sidechain_ducking_effect() {
        // Simulate kick ducking bass: kick pulses cause bass to duck
        let mut comp = SidechainCompressorNode::new(0, 1, 2, 3, 4, 5);

        // Main input: steady bass
        let main_input = vec![0.5; 512];

        // Sidechain: kick pulse (loud then quiet)
        let mut sidechain = vec![0.01; 512];
        for i in 0..128 {
            sidechain[i] = 1.0; // Kick hit
        }
        // Rest is quiet (bass recovers)

        let threshold = vec![-20.0; 512];
        let ratio = vec![8.0; 512];             // Heavy compression for obvious ducking
        let attack = vec![0.001; 512];          // Fast attack (kick is percussive)
        let release = vec![0.02; 512];          // Fast release (20ms) for quick recovery

        let inputs = vec![
            main_input.as_slice(),
            sidechain.as_slice(),
            threshold.as_slice(),
            ratio.as_slice(),
            attack.as_slice(),
            release.as_slice(),
        ];

        let mut output = vec![0.0; 512];
        let context = create_context(512);

        comp.process_block(&inputs, &mut output, 44100.0, &context);

        // During kick: bass should be ducked (compressed)
        // With ratio 8:1, threshold -20 dB, sidechain 1.0 (0 dB):
        // Gain reduction = 20 dB * (1 - 1/8) = 17.5 dB
        // Expected: 0.5 * 10^(-17.5/20) ≈ 0.067
        let during_kick_avg: f32 = output.iter().skip(64).take(32).sum::<f32>() / 32.0;

        // After kick (sidechain quiet): bass should recover (less compressed)
        // With fast attack fully engaged, should be back near 0.5 after release
        let after_kick_avg: f32 = output.iter().skip(450).take(50).sum::<f32>() / 50.0;

        // Verify ducking behavior
        assert!(during_kick_avg < 0.15, "Bass during kick {} should be ducked < 0.15", during_kick_avg);
        assert!(after_kick_avg > during_kick_avg,
            "Bass after kick {} should be higher than during kick {}",
            after_kick_avg, during_kick_avg);
        // With heavy compression (8:1) and relatively fast release (20ms),
        // expect partial but not full recovery in 512 samples (~11.6ms after transition)
        assert!(after_kick_avg > 0.12, "Bass after kick {} should partially recover > 0.12", after_kick_avg);
    }

    #[test]
    fn test_sidechain_ratio_affects_amount() {
        // Higher ratio = more compression
        let mut comp_light = SidechainCompressorNode::new(0, 1, 2, 3, 4, 5);
        let mut comp_heavy = SidechainCompressorNode::new(0, 1, 2, 3, 4, 5);

        let main_input = vec![0.5; 512];
        let sidechain = vec![1.0; 512];
        let threshold = vec![-20.0; 512];
        let attack = vec![0.001; 512];
        let release = vec![0.1; 512];

        // Light compression (2:1)
        let ratio_light = vec![2.0; 512];
        let inputs_light = vec![
            main_input.as_slice(),
            sidechain.as_slice(),
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
            main_input.as_slice(),
            sidechain.as_slice(),
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

        assert!(avg_heavy < avg_light,
            "Heavy compression {} should be less than light {}",
            avg_heavy, avg_light);
    }

    #[test]
    fn test_sidechain_attack_controls_onset() {
        // Slow attack should have gradual compression onset
        let mut comp = SidechainCompressorNode::new(0, 1, 2, 3, 4, 5);

        let main_input = vec![0.5; 512];
        let sidechain = vec![1.0; 512];
        let threshold = vec![-20.0; 512];
        let ratio = vec![4.0; 512];
        let attack = vec![0.1; 512];          // Slow attack
        let release = vec![0.1; 512];

        let inputs = vec![
            main_input.as_slice(),
            sidechain.as_slice(),
            threshold.as_slice(),
            ratio.as_slice(),
            attack.as_slice(),
            release.as_slice(),
        ];

        let mut output = vec![0.0; 512];
        let context = create_context(512);

        comp.process_block(&inputs, &mut output, 44100.0, &context);

        // First sample should be less compressed than later samples
        assert!(output[0] > output[100],
            "Sample 0 {} should be greater than sample 100 {}",
            output[0], output[100]);
        assert!(output[100] > output[400],
            "Sample 100 {} should be greater than sample 400 {}",
            output[100], output[400]);
    }

    #[test]
    fn test_sidechain_release_controls_recovery() {
        // Test release phase: sidechain goes from loud to quiet
        let mut comp = SidechainCompressorNode::new(0, 1, 2, 3, 4, 5);

        let main_input = vec![0.5; 512];

        // Sidechain: first half loud, second half quiet
        let mut sidechain = vec![1.0; 512];
        for i in 256..512 {
            sidechain[i] = 0.01;
        }

        let threshold = vec![-20.0; 512];
        let ratio = vec![4.0; 512];
        let attack = vec![0.001; 512];         // Fast attack
        let release = vec![0.1; 512];          // Slow release

        let inputs = vec![
            main_input.as_slice(),
            sidechain.as_slice(),
            threshold.as_slice(),
            ratio.as_slice(),
            attack.as_slice(),
            release.as_slice(),
        ];

        let mut output = vec![0.0; 512];
        let context = create_context(512);

        comp.process_block(&inputs, &mut output, 44100.0, &context);

        // After sidechain goes quiet, should gradually release
        assert!(output[260] < output[300],
            "Sample 260 {} should be less than sample 300 {}",
            output[260], output[300]);
        assert!(output[300] < output[400],
            "Sample 300 {} should be less than sample 400 {}",
            output[300], output[400]);
    }

    #[test]
    fn test_sidechain_threshold_boundary() {
        // Test sidechain at exactly threshold
        let mut comp = SidechainCompressorNode::new(0, 1, 2, 3, 4, 5);

        let main_input = vec![0.5; 512];
        let sidechain = vec![0.316; 512];     // Exactly -10 dB
        let threshold = vec![-10.0; 512];
        let ratio = vec![4.0; 512];
        let attack = vec![0.01; 512];
        let release = vec![0.1; 512];

        let inputs = vec![
            main_input.as_slice(),
            sidechain.as_slice(),
            threshold.as_slice(),
            ratio.as_slice(),
            attack.as_slice(),
            release.as_slice(),
        ];

        let mut output = vec![0.0; 512];
        let context = create_context(512);

        comp.process_block(&inputs, &mut output, 44100.0, &context);

        // At threshold, minimal compression
        let avg_output: f32 = output.iter().skip(100).take(400).sum::<f32>() / 400.0;
        assert!((avg_output - 0.5).abs() < 0.1,
            "At threshold, output {} should be close to input 0.5",
            avg_output);
    }

    #[test]
    fn test_sidechain_preserves_sign() {
        // Compressor should preserve positive and negative signs
        let mut comp = SidechainCompressorNode::new(0, 1, 2, 3, 4, 5);

        let main_input = vec![0.5, -0.5, 0.3, -0.3];
        let sidechain = vec![1.0; 4];
        let threshold = vec![-20.0; 4];
        let ratio = vec![4.0; 4];
        let attack = vec![0.001; 4];
        let release = vec![0.1; 4];

        let inputs = vec![
            main_input.as_slice(),
            sidechain.as_slice(),
            threshold.as_slice(),
            ratio.as_slice(),
            attack.as_slice(),
            release.as_slice(),
        ];

        let mut output = vec![0.0; 4];
        let context = create_context(4);

        comp.process_block(&inputs, &mut output, 44100.0, &context);

        // Check signs are preserved
        assert!(output[0] > 0.0);
        assert!(output[1] < 0.0);
        assert!(output[2] > 0.0);
        assert!(output[3] < 0.0);
    }

    #[test]
    fn test_sidechain_dependencies() {
        let comp = SidechainCompressorNode::new(5, 10, 15, 20, 25, 30);
        let deps = comp.input_nodes();

        assert_eq!(deps.len(), 6);
        assert_eq!(deps[0], 5);  // main_input
        assert_eq!(deps[1], 10); // sidechain_input
        assert_eq!(deps[2], 15); // threshold
        assert_eq!(deps[3], 20); // ratio
        assert_eq!(deps[4], 25); // attack
        assert_eq!(deps[5], 30); // release
    }

    #[test]
    fn test_sidechain_zero_sidechain_safe() {
        // Verify sidechain compressor handles zero sidechain safely
        let mut comp = SidechainCompressorNode::new(0, 1, 2, 3, 4, 5);

        let main_input = vec![0.5; 4];
        let sidechain = vec![0.0, 0.00001, -0.00001, 0.0];
        let threshold = vec![-20.0; 4];
        let ratio = vec![4.0; 4];
        let attack = vec![0.01; 4];
        let release = vec![0.1; 4];

        let inputs = vec![
            main_input.as_slice(),
            sidechain.as_slice(),
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
    fn test_sidechain_no_compression_at_ratio_1() {
        // Ratio of 1.0 should produce no compression
        let mut comp = SidechainCompressorNode::new(0, 1, 2, 3, 4, 5);

        let main_input = vec![0.5; 512];
        let sidechain = vec![1.0; 512];
        let threshold = vec![-20.0; 512];
        let ratio = vec![1.0; 512];          // No compression
        let attack = vec![0.01; 512];
        let release = vec![0.1; 512];

        let inputs = vec![
            main_input.as_slice(),
            sidechain.as_slice(),
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
            assert!((*sample - 0.5).abs() < 0.01);
        }
    }

    #[test]
    fn test_sidechain_pattern_modulation() {
        // Test varying sidechain level creates varying compression
        let mut comp = SidechainCompressorNode::new(0, 1, 2, 3, 4, 5);

        let main_input = vec![0.5; 512];

        // Sidechain varies: quiet -> loud -> quiet
        let mut sidechain = vec![0.01; 512];
        for i in 128..384 {
            sidechain[i] = 1.0;
        }

        let threshold = vec![-20.0; 512];
        let ratio = vec![8.0; 512];
        let attack = vec![0.001; 512];
        let release = vec![0.001; 512];      // Fast release to show change

        let inputs = vec![
            main_input.as_slice(),
            sidechain.as_slice(),
            threshold.as_slice(),
            ratio.as_slice(),
            attack.as_slice(),
            release.as_slice(),
        ];

        let mut output = vec![0.0; 512];
        let context = create_context(512);

        comp.process_block(&inputs, &mut output, 44100.0, &context);

        // Before loud section: less compression
        let before: f32 = output.iter().skip(50).take(50).sum::<f32>() / 50.0;

        // During loud section: more compression
        let during: f32 = output.iter().skip(200).take(50).sum::<f32>() / 50.0;

        // After loud section: less compression again
        let after: f32 = output.iter().skip(450).take(50).sum::<f32>() / 50.0;

        assert!(before > during, "Before {} should be greater than during {}", before, during);
        assert!(after > during, "After {} should be greater than during {}", after, during);
    }
}
