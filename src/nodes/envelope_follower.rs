/// Envelope Follower node - Extract amplitude envelope from audio signal
///
/// This node analyzes an input signal and produces an output that follows its amplitude
/// envelope, similar to how a VU meter or dynamics processor tracks signal level.
///
/// Common uses:
/// - Sidechain compression (use kick drum envelope to duck bass)
/// - Auto-wah effects (envelope controls filter cutoff)
/// - Envelope-following synthesis (amplitude-to-control signal conversion)
/// - Dynamics visualization and metering
/// - Adaptive effects processing
/// - Vocoder-style envelope following
///
/// # Algorithm
///
/// Classic envelope follower with separate attack and release time constants:
///
/// ```text
/// for each sample:
///     rectified = abs(input[i])  // Full-wave rectification
///
///     attack_time = max(0.00001, attack_input[i])  // seconds
///     release_time = max(0.00001, release_input[i])  // seconds
///
///     attack_coeff = exp(-1.0 / (attack_time * sample_rate))
///     release_coeff = exp(-1.0 / (release_time * sample_rate))
///
///     if rectified > envelope:
///         // Rising signal - use attack time
///         envelope = attack_coeff * envelope + (1.0 - attack_coeff) * rectified
///     else:
///         // Falling signal - use release time
///         envelope = release_coeff * envelope + (1.0 - release_coeff) * rectified
///
///     output[i] = envelope
/// ```
///
/// This creates an exponential smoothing filter that responds quickly to rising signals
/// (fast attack) and slowly to falling signals (slow release), mimicking how analog
/// envelope followers and audio compressors work.
///
/// # Example
///
/// ```ignore
/// // Extract envelope from a drum signal with fast attack, slow release
/// let drum_signal = OscillatorNode::new(...);     // NodeId 0
/// let attack = ConstantNode::new(0.005);          // 5ms attack, NodeId 1
/// let release = ConstantNode::new(0.1);           // 100ms release, NodeId 2
/// let envelope = EnvelopeFollowerNode::new(0, 1, 2);  // NodeId 3
/// // Output will be a smooth envelope following the drum amplitude
/// ```
///
/// # Typical Parameter Ranges
///
/// - **Attack time**: 0.001-0.01s (1-10ms)
///   - Fast attack (1-5ms): Tracks transients, preserves punch
///   - Slow attack (5-10ms): Smooths envelope, reduces jitter
///
/// - **Release time**: 0.01-3.0s (10ms-3s)
///   - Fast release (10-50ms): Tight tracking, good for percussion
///   - Medium release (100-300ms): Natural compression feel
///   - Slow release (500ms-3s): Smooth pumping, ambient effects

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Envelope follower node: tracks signal amplitude with attack/release smoothing
///
/// Uses exponential smoothing with separate attack and release time constants
/// to create a control signal that follows the amplitude envelope of an audio signal.
pub struct EnvelopeFollowerNode {
    /// Input signal to extract envelope from
    input: NodeId,

    /// Attack time in seconds (rising signal response time)
    attack_input: NodeId,

    /// Release time in seconds (falling signal response time)
    release_input: NodeId,

    /// Current envelope state (0.0 to peak amplitude)
    envelope_state: f32,
}

impl EnvelopeFollowerNode {
    /// Create a new envelope follower node
    ///
    /// # Arguments
    /// * `input` - NodeId of signal to extract envelope from
    /// * `attack_input` - NodeId of attack time signal (in seconds)
    /// * `release_input` - NodeId of release time signal (in seconds)
    ///
    /// # Initial State
    /// - `envelope_state` starts at 0.0
    pub fn new(input: NodeId, attack_input: NodeId, release_input: NodeId) -> Self {
        Self {
            input,
            attack_input,
            release_input,
            envelope_state: 0.0,
        }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }

    /// Get the attack time input node ID
    pub fn attack_input(&self) -> NodeId {
        self.attack_input
    }

    /// Get the release time input node ID
    pub fn release_input(&self) -> NodeId {
        self.release_input
    }

    /// Get the current envelope state (for debugging/testing)
    pub fn envelope_state(&self) -> f32 {
        self.envelope_state
    }

    /// Reset the envelope state to zero
    pub fn reset(&mut self) {
        self.envelope_state = 0.0;
    }
}

impl AudioNode for EnvelopeFollowerNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 3,
            "EnvelopeFollowerNode requires 3 inputs, got {}",
            inputs.len()
        );

        let input_buf = inputs[0];
        let attack_buf = inputs[1];
        let release_buf = inputs[2];

        debug_assert_eq!(
            input_buf.len(),
            output.len(),
            "Input buffer length mismatch"
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

        // Process each sample
        for i in 0..output.len() {
            // Full-wave rectification (absolute value)
            let rectified = input_buf[i].abs();

            // Get time constants (clamped to prevent numerical issues)
            let attack_time = attack_buf[i].max(0.00001); // Minimum 10 microseconds
            let release_time = release_buf[i].max(0.00001);

            // Calculate exponential smoothing coefficients
            // coeff = exp(-1 / (time * sample_rate))
            // This gives us the classic analog envelope follower response
            let attack_coeff = (-1.0 / (attack_time * sample_rate)).exp();
            let release_coeff = (-1.0 / (release_time * sample_rate)).exp();

            // Apply appropriate filter based on whether signal is rising or falling
            if rectified > self.envelope_state {
                // Rising signal: use attack time
                self.envelope_state = attack_coeff * self.envelope_state
                    + (1.0 - attack_coeff) * rectified;
            } else {
                // Falling signal: use release time
                self.envelope_state = release_coeff * self.envelope_state
                    + (1.0 - release_coeff) * rectified;
            }

            output[i] = self.envelope_state;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.attack_input, self.release_input]
    }

    fn name(&self) -> &str {
        "EnvelopeFollowerNode"
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
    fn test_envelope_follower_tracks_rising_signal() {
        // Test 1: Envelope should track increasing amplitude
        let mut env = EnvelopeFollowerNode::new(0, 1, 2);

        let input = vec![0.0, 0.2, 0.4, 0.6, 0.8, 1.0];
        let attack = vec![0.001; 6]; // Very fast attack (1ms)
        let release = vec![0.1; 6];   // 100ms release
        let inputs = vec![input.as_slice(), attack.as_slice(), release.as_slice()];

        let mut output = vec![0.0; 6];
        let context = create_context(6);

        env.process_block(&inputs, &mut output, 44100.0, &context);

        // With very fast attack, envelope should closely track rising signal
        assert!(output[0] < 0.1, "output[0] = {}", output[0]);
        assert!(output[1] < output[2], "output[1] = {}, output[2] = {}", output[1], output[2]);
        assert!(output[2] < output[3], "output[2] = {}, output[3] = {}", output[2], output[3]);
        assert!(output[3] < output[4], "output[3] = {}, output[4] = {}", output[3], output[4]);
        assert!(output[4] < output[5], "output[4] = {}, output[5] = {}", output[4], output[5]);

        // Should reach close to peak with fast attack
        assert!(output[5] > 0.5, "output[5] = {}", output[5]);
    }

    #[test]
    fn test_envelope_follower_tracks_falling_signal() {
        // Test 2: Envelope should decay when signal drops
        let mut env = EnvelopeFollowerNode::new(0, 1, 2);

        // Peak followed by silence
        let input = vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let attack = vec![0.001; 6];   // Fast attack
        let release = vec![0.01; 6];   // Fast release (10ms)
        let inputs = vec![input.as_slice(), attack.as_slice(), release.as_slice()];

        let mut output = vec![0.0; 6];
        let context = create_context(6);

        env.process_block(&inputs, &mut output, 44100.0, &context);

        // First sample captures peak
        assert!(output[0] > 0.5, "output[0] = {}", output[0]);

        // Should decay with each sample
        assert!(output[1] < output[0], "output[1] = {}, output[0] = {}", output[1], output[0]);
        assert!(output[2] < output[1], "output[2] = {}, output[1] = {}", output[2], output[1]);
        assert!(output[3] < output[2], "output[3] = {}, output[2] = {}", output[3], output[2]);
        assert!(output[4] < output[3], "output[4] = {}, output[3] = {}", output[4], output[3]);
        assert!(output[5] < output[4], "output[5] = {}, output[4] = {}", output[5], output[4]);
    }

    #[test]
    fn test_envelope_follower_fast_attack_vs_slow_attack() {
        // Test 3: Fast attack should reach peak faster than slow attack
        let mut env_fast = EnvelopeFollowerNode::new(0, 1, 2);
        let mut env_slow = EnvelopeFollowerNode::new(0, 1, 2);

        // Impulse (single peak)
        let input = vec![1.0, 1.0, 1.0, 1.0];
        let attack_fast = vec![0.001; 4];  // 1ms attack
        let attack_slow = vec![0.05; 4];   // 50ms attack
        let release = vec![0.5; 4];        // Same release for both

        let inputs_fast = vec![input.as_slice(), attack_fast.as_slice(), release.as_slice()];
        let inputs_slow = vec![input.as_slice(), attack_slow.as_slice(), release.as_slice()];

        let mut output_fast = vec![0.0; 4];
        let mut output_slow = vec![0.0; 4];
        let context = create_context(4);

        env_fast.process_block(&inputs_fast, &mut output_fast, 44100.0, &context);
        env_slow.process_block(&inputs_slow, &mut output_slow, 44100.0, &context);

        // Fast attack should reach higher value sooner
        assert!(output_fast[1] > output_slow[1],
                "Fast attack output[1] = {}, slow attack output[1] = {}",
                output_fast[1], output_slow[1]);
        assert!(output_fast[2] > output_slow[2],
                "Fast attack output[2] = {}, slow attack output[2] = {}",
                output_fast[2], output_slow[2]);
        assert!(output_fast[3] > output_slow[3],
                "Fast attack output[3] = {}, slow attack output[3] = {}",
                output_fast[3], output_slow[3]);
    }

    #[test]
    fn test_envelope_follower_fast_release_vs_slow_release() {
        // Test 4: Fast release should decay faster than slow release
        let mut env_fast = EnvelopeFollowerNode::new(0, 1, 2);
        let mut env_slow = EnvelopeFollowerNode::new(0, 1, 2);

        // Peak at start, then silence
        let input = vec![1.0, 0.0, 0.0, 0.0, 0.0];
        let attack = vec![0.001; 5];        // Same fast attack for both
        let release_fast = vec![0.01; 5];   // 10ms release
        let release_slow = vec![0.5; 5];    // 500ms release

        let inputs_fast = vec![input.as_slice(), attack.as_slice(), release_fast.as_slice()];
        let inputs_slow = vec![input.as_slice(), attack.as_slice(), release_slow.as_slice()];

        let mut output_fast = vec![0.0; 5];
        let mut output_slow = vec![0.0; 5];
        let context = create_context(5);

        env_fast.process_block(&inputs_fast, &mut output_fast, 44100.0, &context);
        env_slow.process_block(&inputs_slow, &mut output_slow, 44100.0, &context);

        // After peak capture, fast release should decay more
        assert!(output_fast[4] < output_slow[4],
                "Fast release output[4] = {}, slow release output[4] = {}",
                output_fast[4], output_slow[4]);

        // Verify significant difference
        assert!(output_slow[4] > output_fast[4] * 2.0,
                "Slow release should be significantly higher than fast release");
    }

    #[test]
    fn test_envelope_follower_with_sine_wave() {
        // Test 5: Envelope should smooth sine wave to its peak amplitude
        use std::f32::consts::PI;

        let mut env = EnvelopeFollowerNode::new(0, 1, 2);

        // Generate 2 cycles of sine wave (amplitude 0.8)
        let amplitude = 0.8;
        let mut input = Vec::new();
        for i in 0..32 {
            let phase = (i as f32 / 16.0) * 2.0 * PI;
            input.push(amplitude * phase.sin());
        }

        let attack = vec![0.005; 32];  // 5ms attack
        let release = vec![0.05; 32];  // 50ms release
        let inputs = vec![input.as_slice(), attack.as_slice(), release.as_slice()];

        let mut output = vec![0.0; 32];
        let context = create_context(32);

        env.process_block(&inputs, &mut output, 44100.0, &context);

        // Envelope should converge toward peak amplitude
        let max_output = output.iter().cloned().fold(0.0_f32, f32::max);
        assert!(max_output > 0.6 * amplitude,
                "Envelope max ({}) should approach sine amplitude ({})",
                max_output, amplitude);
    }

    #[test]
    fn test_envelope_follower_with_square_wave() {
        // Test 6: Envelope should track square wave amplitude
        let mut env = EnvelopeFollowerNode::new(0, 1, 2);

        // Square wave: +0.5, -0.5, +0.5, -0.5...
        let input: Vec<f32> = (0..16)
            .map(|i| if i % 2 == 0 { 0.5 } else { -0.5 })
            .collect();

        let attack = vec![0.001; 16];   // Fast attack
        let release = vec![0.001; 16];  // Fast release
        let inputs = vec![input.as_slice(), attack.as_slice(), release.as_slice()];

        let mut output = vec![0.0; 16];
        let context = create_context(16);

        env.process_block(&inputs, &mut output, 44100.0, &context);

        // With fast attack/release and consistent amplitude, should stabilize near 0.5
        // (the absolute value of both +0.5 and -0.5)
        let final_value = output[15];
        assert!(final_value > 0.3 && final_value < 0.7,
                "Envelope should stabilize around 0.5, got {}", final_value);
    }

    #[test]
    fn test_envelope_follower_negative_values() {
        // Test 7: Should handle negative values via full-wave rectification
        let mut env = EnvelopeFollowerNode::new(0, 1, 2);

        let input = vec![-0.5, -1.0, -0.3, 0.0, 0.5, 1.0];
        let attack = vec![0.001; 6];
        let release = vec![0.1; 6];
        let inputs = vec![input.as_slice(), attack.as_slice(), release.as_slice()];

        let mut output = vec![0.0; 6];
        let context = create_context(6);

        env.process_block(&inputs, &mut output, 44100.0, &context);

        // Should track absolute values
        // Peak at index 1 (|-1.0| = 1.0)
        assert!(output[1] > output[0], "Should rise to peak at |-1.0|");
        assert!(output[2] < output[1], "Should decay after peak");

        // Another peak at index 5 (|1.0| = 1.0)
        assert!(output[5] > output[4], "Should rise to peak at |1.0|");
    }

    #[test]
    fn test_envelope_follower_handles_impulse() {
        // Test 8: Single impulse with decay
        let mut env = EnvelopeFollowerNode::new(0, 1, 2);

        // Single impulse followed by silence
        let mut input = vec![0.0; 16];
        input[0] = 1.0;

        let attack = vec![0.001; 16];  // Fast attack
        let release = vec![0.02; 16];  // 20ms release
        let inputs = vec![input.as_slice(), attack.as_slice(), release.as_slice()];

        let mut output = vec![0.0; 16];
        let context = create_context(16);

        env.process_block(&inputs, &mut output, 44100.0, &context);

        // Should capture impulse
        assert!(output[0] > 0.1, "output[0] = {}", output[0]);

        // Should decay smoothly
        for i in 1..16 {
            assert!(output[i] < output[i-1],
                    "output[{}] = {} should be less than output[{}] = {}",
                    i, output[i], i-1, output[i-1]);
        }

        // Should approach zero
        assert!(output[15] < 0.5, "output[15] = {}", output[15]);
    }

    #[test]
    fn test_envelope_follower_dependencies() {
        // Test 9: Verify node has correct dependencies
        let env = EnvelopeFollowerNode::new(3, 7, 11);
        let deps = env.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 3);  // input
        assert_eq!(deps[1], 7);  // attack
        assert_eq!(deps[2], 11); // release
    }

    #[test]
    fn test_envelope_follower_state_persistence() {
        // Test 10: State should persist across multiple process_block calls
        let mut env = EnvelopeFollowerNode::new(0, 1, 2);

        // First block: establish envelope
        let input1 = vec![0.5, 0.8, 1.0];
        let attack1 = vec![0.01; 3];
        let release1 = vec![0.1; 3];
        let inputs1 = vec![input1.as_slice(), attack1.as_slice(), release1.as_slice()];
        let mut output1 = vec![0.0; 3];
        let context = create_context(3);

        env.process_block(&inputs1, &mut output1, 44100.0, &context);
        let end_state_1 = output1[2];
        assert!(end_state_1 > 0.5, "Should have built up envelope: {}", end_state_1);

        // Second block: silence, should decay from previous state
        let input2 = vec![0.0, 0.0, 0.0];
        let attack2 = vec![0.01; 3];
        let release2 = vec![0.1; 3];
        let inputs2 = vec![input2.as_slice(), attack2.as_slice(), release2.as_slice()];
        let mut output2 = vec![0.0; 3];

        env.process_block(&inputs2, &mut output2, 44100.0, &context);

        // Should start from previous envelope state
        assert!(output2[0] < end_state_1,
                "output2[0] = {} should start decaying from end_state_1 = {}",
                output2[0], end_state_1);
        assert!(output2[0] > 0.0, "Should not jump to zero");
    }

    #[test]
    fn test_envelope_follower_reset() {
        // Test 11: Reset should clear envelope state
        let mut env = EnvelopeFollowerNode::new(0, 1, 2);

        // Build up some envelope
        let input = vec![1.0; 10];
        let attack = vec![0.01; 10];
        let release = vec![0.1; 10];
        let inputs = vec![input.as_slice(), attack.as_slice(), release.as_slice()];
        let mut output = vec![0.0; 10];
        let context = create_context(10);

        env.process_block(&inputs, &mut output, 44100.0, &context);
        assert!(env.envelope_state() > 0.5, "Should have built up envelope");

        // Reset
        env.reset();

        // State should be cleared
        assert_eq!(env.envelope_state(), 0.0, "Reset should clear envelope state");
    }

    #[test]
    fn test_envelope_follower_very_fast_attack() {
        // Test 12: Very fast attack should track signal almost instantly
        let mut env = EnvelopeFollowerNode::new(0, 1, 2);

        let input = vec![0.0, 0.0, 1.0, 1.0, 1.0];
        let attack = vec![0.0001; 5];  // 0.1ms attack (very fast)
        let release = vec![0.5; 5];
        let inputs = vec![input.as_slice(), attack.as_slice(), release.as_slice()];

        let mut output = vec![0.0; 5];
        let context = create_context(5);

        env.process_block(&inputs, &mut output, 44100.0, &context);

        // With very fast attack, should reach close to peak within 2 samples
        assert!(output[3] > 0.8, "Very fast attack should reach peak quickly: output[3] = {}", output[3]);
    }

    #[test]
    fn test_envelope_follower_very_slow_release() {
        // Test 13: Very slow release should hold envelope
        let mut env = EnvelopeFollowerNode::new(0, 1, 2);

        // Peak followed by silence
        let input = vec![1.0, 0.0, 0.0, 0.0, 0.0];
        let attack = vec![0.001; 5];
        let release = vec![10.0; 5];  // 10 second release (very slow)
        let inputs = vec![input.as_slice(), attack.as_slice(), release.as_slice()];

        let mut output = vec![0.0; 5];
        let context = create_context(5);

        env.process_block(&inputs, &mut output, 44100.0, &context);

        // With very slow release, should barely decay over 4 samples
        assert!(output[4] > 0.95 * output[0],
                "Very slow release should hold envelope: output[4] = {}, output[0] = {}",
                output[4], output[0]);
    }

    #[test]
    fn test_envelope_follower_pattern_modulated_times() {
        // Test 14: Attack/release times can vary per sample (pattern control)
        let mut env = EnvelopeFollowerNode::new(0, 1, 2);

        // Signal with varying envelope parameters
        let input = vec![1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

        // First impulse: fast attack/release
        // Second impulse: slow attack/release
        let attack = vec![0.001, 0.001, 0.001, 0.1, 0.1, 0.1];
        let release = vec![0.01, 0.01, 0.01, 0.5, 0.5, 0.5];
        let inputs = vec![input.as_slice(), attack.as_slice(), release.as_slice()];

        let mut output = vec![0.0; 6];
        let context = create_context(6);

        env.process_block(&inputs, &mut output, 44100.0, &context);

        // First impulse should decay quickly (fast release)
        assert!(output[2] < 0.5 * output[0],
                "Fast release should decay significantly: output[2] = {}, output[0] = {}",
                output[2], output[0]);

        // Second impulse should decay slowly (slow release)
        assert!(output[5] > 0.8 * output[3],
                "Slow release should hold envelope: output[5] = {}, output[3] = {}",
                output[5], output[3]);
    }
}
