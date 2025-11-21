/// Expander node - upward dynamics expansion with attack/release
///
/// This node provides upward dynamic range expansion. It increases the level
/// of loud signals above the threshold according to the specified ratio.
/// The gain boost envelope follows the input signal with attack and release times.
///
/// Unlike a compressor which reduces loud signals, an expander BOOSTS loud signals,
/// increasing dynamic range.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Expander state for envelope follower
#[derive(Debug, Clone)]
struct ExpanderState {
    envelope: f32,  // Current envelope level (linear amplitude)
}

impl Default for ExpanderState {
    fn default() -> Self {
        Self { envelope: 0.0 }
    }
}

/// Expander node: upward dynamics expansion
///
/// The expansion algorithm:
/// ```text
/// 1. Envelope follower tracks input amplitude with attack/release
/// 2. Convert envelope to dB: envelope_db = 20 * log10(envelope)
/// 3. Calculate gain boost when above threshold:
///    over = envelope_db - threshold
///    if over > 0:
///        boost_db = over * (ratio - 1)
///    else:
///        boost_db = 0
/// 4. Apply boost: output = input * 10^(boost_db / 20)
/// ```
///
/// This provides:
/// - Smooth expansion with no clicks
/// - Independent attack and release times
/// - Variable expansion ratio
/// - Transparent when below threshold
///
/// # Example
/// ```ignore
/// // Expand signal with 2:1 ratio, -10 dB threshold
/// let input = OscillatorNode::new(0, Waveform::Sine);  // NodeId 1
/// let threshold = ConstantNode::new(-10.0);            // NodeId 2 (-10 dB)
/// let ratio = ConstantNode::new(2.0);                  // NodeId 3 (2:1)
/// let attack = ConstantNode::new(0.001);               // NodeId 4 (1ms)
/// let release = ConstantNode::new(0.1);                // NodeId 5 (100ms)
/// let exp = ExpanderNode::new(1, 2, 3, 4, 5);          // NodeId 6
/// ```
pub struct ExpanderNode {
    input: NodeId,
    threshold_input: NodeId,  // Threshold in dB (e.g., -10.0)
    ratio_input: NodeId,      // Expansion ratio (1.0 to 10.0)
    attack_input: NodeId,     // Attack time in seconds (0.001 to 1.0)
    release_input: NodeId,    // Release time in seconds (0.01 to 3.0)
    state: ExpanderState,     // Envelope follower state
}

impl ExpanderNode {
    /// Create a new expander node
    ///
    /// # Arguments
    /// * `input` - NodeId of signal to expand
    /// * `threshold_input` - NodeId of threshold in dB (e.g., -10.0)
    /// * `ratio_input` - NodeId of expansion ratio (1.0 = no expansion, 10.0 = aggressive)
    /// * `attack_input` - NodeId of attack time in seconds (how fast expansion kicks in)
    /// * `release_input` - NodeId of release time in seconds (how fast expansion releases)
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
            state: ExpanderState::default(),
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

    /// Reset expander state
    pub fn reset(&mut self) {
        self.state = ExpanderState::default();
    }
}

impl AudioNode for ExpanderNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 5,
            "ExpanderNode requires 5 inputs, got {}",
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

        // Apply expansion with envelope follower
        for i in 0..output.len() {
            let sample = input_buf[i];
            let threshold_db = threshold_buf[i].clamp(-60.0, 0.0);
            let ratio = ratio_buf[i].clamp(1.0, 10.0);
            let attack_time = attack_buf[i].max(0.0001); // Min 0.1ms
            let release_time = release_buf[i].max(0.001); // Min 1ms

            // Calculate attack/release coefficients (exponential smoothing)
            let attack_coeff = (-1.0 / (attack_time * sample_rate)).exp();
            let release_coeff = (-1.0 / (release_time * sample_rate)).exp();

            // Envelope follower: track input amplitude
            let input_level = sample.abs();

            // Update envelope with attack/release
            let coeff = if input_level > self.state.envelope {
                attack_coeff  // Fast response to increasing levels
            } else {
                release_coeff  // Slow response to decreasing levels
            };

            self.state.envelope = coeff * self.state.envelope + (1.0 - coeff) * input_level;

            // Calculate gain boost based on envelope level
            let envelope = self.state.envelope.max(1e-10); // Prevent log(0)
            let envelope_db = 20.0 * envelope.log10();

            let over_db = envelope_db - threshold_db;
            let boost_db = if over_db > 0.0 {
                // Apply expansion ratio: boost = over * (ratio - 1)
                // ratio=1.0 means no expansion, ratio=2.0 doubles the over amount
                over_db * (ratio - 1.0)
            } else {
                0.0  // No boost below threshold
            };

            // Apply positive gain boost
            let gain_linear = 10.0_f32.powf(boost_db / 20.0);
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
        "ExpanderNode"
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
    fn test_expander_below_threshold() {
        // Test that signals below threshold pass through unchanged (gain = 1.0)
        let size = 512;
        let sample_rate = 44100.0;

        // Create a quiet input signal (-30 dB)
        let mut input = vec![0.0316_f32; size]; // ~-30 dB
        let mut threshold = vec![-10.0; size];   // Threshold at -10 dB
        let mut ratio = vec![2.0; size];
        let mut attack = vec![0.001; size];
        let mut release = vec![0.1; size];

        let inputs: Vec<&[f32]> = vec![&input, &threshold, &ratio, &attack, &release];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut expander = ExpanderNode::new(0, 1, 2, 3, 4);

        // Process several blocks to let envelope settle
        for _ in 0..10 {
            expander.process_block(&inputs, &mut output, sample_rate, &context);
        }

        // Output should be approximately equal to input (no expansion below threshold)
        let max_diff = input.iter().zip(output.iter())
            .map(|(i, o)| (i - o).abs())
            .fold(0.0_f32, f32::max);

        assert!(max_diff < 0.01, "Below threshold should pass through, max_diff: {}", max_diff);
    }

    #[test]
    fn test_expander_above_threshold() {
        // Test that loud signals above threshold get boosted
        let size = 512;
        let sample_rate = 44100.0;

        // Create a loud input signal (-5 dB)
        let input_level = 0.562; // ~-5 dB
        let input = vec![input_level; size];
        let threshold = vec![-10.0; size];   // Threshold at -10 dB
        let ratio = vec![2.0; size];         // 2:1 expansion
        let attack = vec![0.001; size];
        let release = vec![0.1; size];

        let inputs: Vec<&[f32]> = vec![&input, &threshold, &ratio, &attack, &release];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut expander = ExpanderNode::new(0, 1, 2, 3, 4);

        // Process several blocks to let envelope settle
        for _ in 0..20 {
            expander.process_block(&inputs, &mut output, sample_rate, &context);
        }

        // Output should be louder than input (expansion boosts loud signals)
        let avg_output: f32 = output.iter().sum::<f32>() / output.len() as f32;
        let avg_input: f32 = input.iter().sum::<f32>() / input.len() as f32;

        assert!(avg_output > avg_input,
            "Above threshold should be boosted: output {} > input {}",
            avg_output, avg_input);

        // With 2:1 ratio and 5dB over threshold, expect boost
        // over = -5 - (-10) = 5 dB
        // boost = 5 * (2 - 1) = 5 dB
        // So output should be ~5dB louder
        let output_db = 20.0 * avg_output.log10();
        let input_db = 20.0 * avg_input.log10();
        let boost_applied = output_db - input_db;

        assert!(boost_applied > 3.0 && boost_applied < 7.0,
            "Expected ~5dB boost, got {:.1}dB", boost_applied);
    }

    #[test]
    fn test_expander_ratio() {
        // Test that higher ratios produce more expansion
        let size = 512;
        let sample_rate = 44100.0;

        let input = vec![0.562; size];  // ~-5 dB
        let threshold = vec![-10.0; size];
        let attack = vec![0.001; size];
        let release = vec![0.1; size];
        let context = create_context(size);

        // Test ratio 1.0 (no expansion)
        let ratio1 = vec![1.0; size];
        let inputs1: Vec<&[f32]> = vec![&input, &threshold, &ratio1, &attack, &release];
        let mut output1 = vec![0.0; size];
        let mut exp1 = ExpanderNode::new(0, 1, 2, 3, 4);
        for _ in 0..20 {
            exp1.process_block(&inputs1, &mut output1, sample_rate, &context);
        }

        // Test ratio 3.0 (strong expansion)
        let ratio3 = vec![3.0; size];
        let inputs3: Vec<&[f32]> = vec![&input, &threshold, &ratio3, &attack, &release];
        let mut output3 = vec![0.0; size];
        let mut exp3 = ExpanderNode::new(0, 1, 2, 3, 4);
        for _ in 0..20 {
            exp3.process_block(&inputs3, &mut output3, sample_rate, &context);
        }

        let avg1: f32 = output1.iter().sum::<f32>() / output1.len() as f32;
        let avg3: f32 = output3.iter().sum::<f32>() / output3.len() as f32;

        assert!(avg3 > avg1 * 1.5,
            "Higher ratio should produce more boost: ratio3 {} > ratio1 {}",
            avg3, avg1);
    }

    #[test]
    fn test_expander_attack_release() {
        // Test that attack/release affect envelope following
        let size = 512;
        let sample_rate = 44100.0;

        // Create signal that goes from quiet to loud
        let mut input = vec![0.0; size];
        for i in 0..size/2 {
            input[i] = 0.01; // Quiet
        }
        for i in size/2..size {
            input[i] = 0.5; // Loud
        }

        let threshold = vec![-10.0; size];
        let ratio = vec![2.0; size];

        // Fast attack
        let fast_attack = vec![0.001; size];
        let slow_release = vec![0.5; size];
        let inputs_fast: Vec<&[f32]> = vec![&input, &threshold, &ratio, &fast_attack, &slow_release];
        let mut output_fast = vec![0.0; size];
        let mut exp_fast = ExpanderNode::new(0, 1, 2, 3, 4);
        let context = create_context(size);

        exp_fast.process_block(&inputs_fast, &mut output_fast, sample_rate, &context);

        // Check that envelope responds to level change
        let first_half_avg: f32 = output_fast[..size/4].iter().sum::<f32>() / (size/4) as f32;
        let second_half_avg: f32 = output_fast[3*size/4..].iter().sum::<f32>() / (size/4) as f32;

        assert!(second_half_avg > first_half_avg,
            "Loud section should have higher output than quiet section");
    }

    #[test]
    fn test_expander_node_interface() {
        // Test node getters
        let exp = ExpanderNode::new(1, 2, 3, 4, 5);

        assert_eq!(exp.input(), 1);
        assert_eq!(exp.threshold_input(), 2);
        assert_eq!(exp.ratio_input(), 3);
        assert_eq!(exp.attack_input(), 4);
        assert_eq!(exp.release_input(), 5);

        let inputs = exp.input_nodes();
        assert_eq!(inputs.len(), 5);
        assert_eq!(inputs[0], 1);

        assert_eq!(exp.name(), "ExpanderNode");
    }

    #[test]
    fn test_expander_reset() {
        // Test that reset clears envelope state
        let size = 512;
        let sample_rate = 44100.0;

        let input = vec![0.5; size];
        let threshold = vec![-10.0; size];
        let ratio = vec![2.0; size];
        let attack = vec![0.001; size];
        let release = vec![0.1; size];
        let inputs: Vec<&[f32]> = vec![&input, &threshold, &ratio, &attack, &release];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut exp = ExpanderNode::new(0, 1, 2, 3, 4);

        // Build up envelope
        for _ in 0..10 {
            exp.process_block(&inputs, &mut output, sample_rate, &context);
        }

        let envelope_before = exp.state.envelope;
        assert!(envelope_before > 0.0, "Envelope should build up");

        // Reset
        exp.reset();
        assert_eq!(exp.state.envelope, 0.0, "Envelope should be cleared after reset");
    }
}
