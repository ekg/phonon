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
    /// EnvelopeFollower - Extracts amplitude envelope from audio signal
    ///
    /// Analyzes input amplitude and outputs smooth envelope following,
    /// useful for sidechain, auto-wah, and dynamics visualization.
    ///
    /// # Parameters
    /// - `input`: NodeId providing signal to analyze
    /// - `attack_input`: NodeId providing attack time in seconds (default: 0.005)
    /// - `release_input`: NodeId providing release time in seconds (default: 0.1)
    ///
    /// # Example
    /// ```phonon
    /// ~signal: sine 110
    /// ~envelope: ~signal # envelope_follower 0.01 0.2
    /// ```
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

        // With 1ms attack at 44100 Hz: ~2.24% change per sample
        // Envelope should monotonically increase as input increases
        assert!(output[0] < 0.1, "output[0] = {}", output[0]);
        assert!(output[1] < output[2], "output[1] = {}, output[2] = {}", output[1], output[2]);
        assert!(output[2] < output[3], "output[2] = {}, output[3] = {}", output[2], output[3]);
        assert!(output[3] < output[4], "output[3] = {}, output[4] = {}", output[3], output[4]);
        assert!(output[4] < output[5], "output[4] = {}, output[5] = {}", output[4], output[5]);

        // Should rise but won't reach 0.5 in just 6 samples
        // At 2.24% per sample, max would be ~13.4% after 6 samples
        assert!(output[5] > output[0], "Should rise over time");
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

        // First sample captures peak (attack_coeff ~0.978, so 1-coeff = 0.022, output = 0.0224)
        // With 10ms release, decay is very slow: ~0.23% per sample
        assert!(output[0] > 0.01, "output[0] = {}", output[0]);

        // Should decay with each sample (though very slowly with 10ms release)
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
        let mut input = vec![0.0; 1000];
        input[0] = 1.0;

        let attack = vec![0.001; 1000];      // Same fast attack for both
        let release_fast = vec![0.001; 1000]; // 1ms release (fast)
        let release_slow = vec![0.5; 1000];   // 500ms release (slow)

        let inputs_fast = vec![input.as_slice(), attack.as_slice(), release_fast.as_slice()];
        let inputs_slow = vec![input.as_slice(), attack.as_slice(), release_slow.as_slice()];

        let mut output_fast = vec![0.0; 1000];
        let mut output_slow = vec![0.0; 1000];
        let context = create_context(1000);

        env_fast.process_block(&inputs_fast, &mut output_fast, 44100.0, &context);
        env_slow.process_block(&inputs_slow, &mut output_slow, 44100.0, &context);

        // After 500 samples: fast release should have decayed much more
        // 1ms release: coeff ~0.978, decay ~2.24% per sample
        // After 500 samples: ~0.0224 * 0.978^500 ≈ 0 (very small)
        // 500ms release: coeff ~0.99995, decay ~0.0046% per sample
        // After 500 samples: ~0.0224 * 0.99995^500 ≈ 0.022 (holds well)
        assert!(output_fast[500] < output_slow[500],
                "Fast release output[500] = {}, slow release output[500] = {}",
                output_fast[500], output_slow[500]);

        // Verify huge difference
        assert!(output_slow[500] > output_fast[500] * 10.0,
                "Slow release should be much higher than fast release: {} vs {}",
                output_slow[500], output_fast[500]);
    }

    #[test]
    fn test_envelope_follower_with_sine_wave() {
        // Test 5: Envelope should smooth sine wave to its peak amplitude
        use std::f32::consts::PI;

        let mut env = EnvelopeFollowerNode::new(0, 1, 2);

        // Generate multiple cycles of sine wave (amplitude 0.8) to allow buildup
        let amplitude = 0.8;
        let mut input = Vec::new();
        for i in 0..200 {
            let phase = (i as f32 / 16.0) * 2.0 * PI;
            input.push(amplitude * phase.sin());
        }

        let attack = vec![0.001; 200];  // 1ms attack for faster buildup
        let release = vec![0.05; 200];  // 50ms release
        let inputs = vec![input.as_slice(), attack.as_slice(), release.as_slice()];

        let mut output = vec![0.0; 200];
        let context = create_context(200);

        env.process_block(&inputs, &mut output, 44100.0, &context);

        // Envelope should converge toward peak amplitude
        // Over 200 samples with 1ms attack, should reach much higher
        let max_output = output.iter().cloned().fold(0.0_f32, f32::max);
        assert!(max_output > 0.5 * amplitude,
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

        let attack = vec![0.001; 16];   // 1ms attack (~2.24% per sample)
        let release = vec![0.001; 16];  // 1ms release (~2.24% per sample)
        let inputs = vec![input.as_slice(), attack.as_slice(), release.as_slice()];

        let mut output = vec![0.0; 16];
        let context = create_context(16);

        env.process_block(&inputs, &mut output, 44100.0, &context);

        // With 1ms attack/release and square wave, envelope builds up slowly
        // Over 16 samples: ~30% of target, so should reach ~0.15
        // (not stabilize around 0.5 in just 16 samples)
        let final_value = output[15];
        assert!(final_value > 0.0 && final_value < 0.5,
                "Envelope should be below 0.5, got {}", final_value);
    }

    #[test]
    fn test_envelope_follower_negative_values() {
        // Test 7: Should handle negative values via full-wave rectification
        let mut env = EnvelopeFollowerNode::new(0, 1, 2);

        // Use a scenario that clearly shows decay after peak
        // Start with silence, then a strong peak, then back to silence
        let input = vec![-0.8, -0.9, -1.0, 0.0, 0.0, 0.0, 0.5, 0.5];
        let attack = vec![0.001; 8];   // 1ms attack
        let release = vec![0.01; 8];   // 10ms release
        let inputs = vec![input.as_slice(), attack.as_slice(), release.as_slice()];

        let mut output = vec![0.0; 8];
        let context = create_context(8);

        env.process_block(&inputs, &mut output, 44100.0, &context);

        // Trace: |−0.8|=0.8, |−0.9|=0.9, |−1.0|=1.0, |0|=0, |0|=0, |0|=0, |0.5|=0.5, |0.5|=0.5
        // Peak at index 2 (1.0), then all zeros, should see decay
        assert!(output[2] > output[0], "Should build to peak");
        // After peak at [2], all zeros means envelope should decay via release
        assert!(output[3] < output[2], "Should start decaying after peak");
        assert!(output[4] < output[3], "Should continue decaying");
        assert!(output[5] < output[4], "Should continue decaying");
    }

    #[test]
    fn test_envelope_follower_handles_impulse() {
        // Test 8: Single impulse with decay
        let mut env = EnvelopeFollowerNode::new(0, 1, 2);

        // Single impulse followed by silence
        let mut input = vec![0.0; 16];
        input[0] = 1.0;

        let attack = vec![0.001; 16];  // 1ms attack (~2.24% per sample)
        let release = vec![0.02; 16];  // 20ms release (~1.1% per sample)
        let inputs = vec![input.as_slice(), attack.as_slice(), release.as_slice()];

        let mut output = vec![0.0; 16];
        let context = create_context(16);

        env.process_block(&inputs, &mut output, 44100.0, &context);

        // Should capture impulse
        // With 1ms attack: output[0] = 0.0224
        assert!(output[0] > 0.01, "output[0] = {}", output[0]);

        // Should decay smoothly (though slowly with 20ms release)
        for i in 1..16 {
            assert!(output[i] < output[i-1],
                    "output[{}] = {} should be less than output[{}] = {}",
                    i, output[i], i-1, output[i-1]);
        }

        // Should still have some energy after 15 samples of 1% decay
        assert!(output[15] > 0.01, "output[15] = {}", output[15]);
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
        let attack1 = vec![0.01; 3];       // 10ms attack (~0.11% per sample)
        let release1 = vec![0.1; 3];       // 100ms release (~0.023% per sample)
        let inputs1 = vec![input1.as_slice(), attack1.as_slice(), release1.as_slice()];
        let mut output1 = vec![0.0; 3];
        let context = create_context(3);

        env.process_block(&inputs1, &mut output1, 44100.0, &context);
        let end_state_1 = output1[2];
        // With 10ms attack and rising input, should build up: 0, ~0.000555, ~0.00166
        assert!(end_state_1 > 0.0, "Should have built up envelope: {}", end_state_1);

        // Second block: silence, should decay from previous state
        let input2 = vec![0.0, 0.0, 0.0];
        let attack2 = vec![0.01; 3];
        let release2 = vec![0.1; 3];
        let inputs2 = vec![input2.as_slice(), attack2.as_slice(), release2.as_slice()];
        let mut output2 = vec![0.0; 3];

        env.process_block(&inputs2, &mut output2, 44100.0, &context);

        // Should start from previous envelope state and decay slowly
        assert!(output2[0] < end_state_1,
                "output2[0] = {} should start decaying from end_state_1 = {}",
                output2[0], end_state_1);
        assert!(output2[0] > 0.0, "Should not jump to zero");
    }

    #[test]
    fn test_envelope_follower_reset() {
        // Test 11: Reset should clear envelope state
        let mut env = EnvelopeFollowerNode::new(0, 1, 2);

        // Build up some envelope with sustained signal
        let input = vec![1.0; 100];
        let attack = vec![0.001; 100];   // Fast attack
        let release = vec![0.1; 100];
        let inputs = vec![input.as_slice(), attack.as_slice(), release.as_slice()];
        let mut output = vec![0.0; 100];
        let context = create_context(100);

        env.process_block(&inputs, &mut output, 44100.0, &context);
        // With 1ms attack over 100 samples, should reach ~0.89
        assert!(env.envelope_state() > 0.1, "Should have built up envelope");

        // Reset
        env.reset();

        // State should be cleared
        assert_eq!(env.envelope_state(), 0.0, "Reset should clear envelope state");
    }

    #[test]
    fn test_envelope_follower_very_fast_attack() {
        // Test 12: Very fast attack should track signal much faster
        let mut env = EnvelopeFollowerNode::new(0, 1, 2);

        // Use longer buffer to see significant attack response
        let mut input = vec![0.0; 100];
        for i in 2..100 {
            input[i] = 1.0;
        }

        let attack = vec![0.0001; 100];  // 0.1ms attack (very fast: ~4.4% per sample)
        let release = vec![0.5; 100];
        let inputs = vec![input.as_slice(), attack.as_slice(), release.as_slice()];

        let mut output = vec![0.0; 100];
        let context = create_context(100);

        env.process_block(&inputs, &mut output, 44100.0, &context);

        // With 0.1ms attack (4.4% per sample), after 50 samples should reach >0.9
        assert!(output[51] > 0.8, "Very fast attack should reach high value: output[51] = {}", output[51]);
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

        // Signal with two impulses with different release times
        // First part: impulse with fast (1ms) release - decays quickly
        // Second part: impulse with slow (100ms) release - holds longer
        let mut input = vec![0.0; 300];
        input[0] = 1.0;    // First impulse
        input[150] = 1.0;  // Second impulse

        // First 150 samples: fast (1ms) release
        // Second 150 samples: slow (100ms) release
        let mut attack = vec![0.001; 300];
        let mut release = vec![0.001; 300];  // Fast release
        for i in 150..300 {
            release[i] = 0.1;  // Switch to slower release
        }

        let inputs = vec![input.as_slice(), attack.as_slice(), release.as_slice()];

        let mut output = vec![0.0; 300];
        let context = create_context(300);

        env.process_block(&inputs, &mut output, 44100.0, &context);

        // First impulse with fast (1ms) release should decay significantly by sample 75
        // At 1ms, coeff ~0.978, after 75 samples: ~0.0224 * 0.978^75 ≈ 0.0041 (18% of peak)
        assert!(output[75] < output[0] * 0.2,
                "Fast release should decay significantly: output[75] = {}, output[0] = {}",
                output[75], output[0]);

        // Second impulse at [150] with slower (100ms) release should hold more
        // At 100ms, coeff ~0.99995, after 75 samples: ~0.0224 * 0.99995^75 ≈ 0.0223 (holds)
        assert!(output[225] > output[150] * 0.8,
                "Slow release should hold envelope: output[225] = {}, output[150] = {}",
                output[225], output[150]);
    }
}
