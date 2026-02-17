/// Transient Shaper node - Independent control of attack (transient) and sustain portions
///
/// This node analyzes audio signals and allows independent control over the
/// transient (attack) and sustain portions of sounds. It's commonly used for:
///
/// - Drum enhancement (boost transients for more punch)
/// - Making drums softer/rounder (reduce transients)
/// - Adding body to thin sounds (boost sustain)
/// - Reducing room sound (reduce sustain)
/// - Creative sound design
///
/// # Algorithm
///
/// The algorithm uses two envelope followers with different time constants:
///
/// ```text
/// 1. Fast envelope (tracks transients): attack ~0.5ms, release ~5ms
/// 2. Slow envelope (tracks sustain): attack ~20ms, release ~50ms
///
/// 3. Transient component = fast_envelope - slow_envelope
///    (This isolates the attack portion of the signal)
///
/// 4. Apply transient gain: transient_out = signal * transient_component * transient_gain
/// 5. Apply sustain gain: sustain_out = signal * slow_envelope * sustain_gain
///
/// 6. Output = original * (1 - wet) + (transient_out + sustain_out) * wet
/// ```
///
/// # Parameters
///
/// - `input`: Audio signal to process
/// - `attack`: Transient gain in dB (-24 to +24)
///   - Positive values: boost transients (more punch/snap)
///   - Negative values: reduce transients (softer/rounder)
///   - 0: neutral
/// - `sustain`: Sustain gain in dB (-24 to +24)
///   - Positive values: boost sustain (more body/fullness)
///   - Negative values: reduce sustain (tighter/drier)
///   - 0: neutral
///
/// # Example
///
/// ```ignore
/// // Enhance drum transients and reduce room sound
/// let drums = SampleNode::new("drums.wav");       // NodeId 0
/// let attack_gain = ConstantNode::new(6.0);       // +6 dB transient boost
/// let sustain_gain = ConstantNode::new(-3.0);     // -3 dB sustain reduction
/// let shaper = TransientShaperNode::new(0, 1, 2); // NodeId 3
/// ```
///
/// # Typical Use Cases
///
/// | Sound                | Attack | Sustain | Effect                              |
/// |----------------------|--------|---------|-------------------------------------|
/// | Punchy drums         | +6     | 0       | More snap, same body                |
/// | Soft drums           | -6     | 0       | Rounder, less aggressive            |
/// | Full bass            | 0      | +6      | More sustain/body                   |
/// | Tight bass           | 0      | -6      | Less room, tighter                  |
/// | Aggressive snare     | +12    | -6      | Maximum punch, minimal ring         |
/// | Ambient drums        | -12    | +12     | Dreamy, washy, reversed-like        |

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// State for envelope followers used in transient detection
#[derive(Debug, Clone)]
pub struct TransientShaperState {
    /// Fast envelope (tracks transients) - responds quickly to attack
    fast_envelope: f32,
    /// Slow envelope (tracks sustain) - responds slowly, captures body
    slow_envelope: f32,
}

impl Default for TransientShaperState {
    fn default() -> Self {
        Self {
            fast_envelope: 0.0,
            slow_envelope: 0.0,
        }
    }
}

/// Transient shaper node: independent control of attack and sustain
///
/// Uses dual envelope followers to separate transient and sustain components,
/// then applies independent gain to each before recombining.
pub struct TransientShaperNode {
    /// Input signal to process
    input: NodeId,
    /// Attack/transient gain in dB (-24 to +24)
    attack_input: NodeId,
    /// Sustain gain in dB (-24 to +24)
    sustain_input: NodeId,
    /// Internal state
    state: TransientShaperState,
}

impl TransientShaperNode {
    /// TransientShaper - Independent control of attack (transient) and sustain
    ///
    /// Separates audio into transient and sustain components for independent
    /// control. Perfect for drum shaping, adding punch, or creating ambient textures.
    ///
    /// # Parameters
    /// - `input`: Signal to process
    /// - `attack_input`: Transient gain in dB (-24 to +24, 0 = neutral)
    /// - `sustain_input`: Sustain gain in dB (-24 to +24, 0 = neutral)
    ///
    /// # Example
    /// ```phonon
    /// ~drums: s "bd sn hh cp"
    /// ~punchy: ~drums # transient_shaper 6 0     -- boost attack by 6dB
    /// ~soft: ~drums # transient_shaper -6 3      -- reduce attack, boost sustain
    /// ```
    pub fn new(input: NodeId, attack_input: NodeId, sustain_input: NodeId) -> Self {
        Self {
            input,
            attack_input,
            sustain_input,
            state: TransientShaperState::default(),
        }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }

    /// Get the attack gain input node ID
    pub fn attack_input(&self) -> NodeId {
        self.attack_input
    }

    /// Get the sustain gain input node ID
    pub fn sustain_input(&self) -> NodeId {
        self.sustain_input
    }

    /// Get the current fast envelope value (for testing/debugging)
    pub fn fast_envelope(&self) -> f32 {
        self.state.fast_envelope
    }

    /// Get the current slow envelope value (for testing/debugging)
    pub fn slow_envelope(&self) -> f32 {
        self.state.slow_envelope
    }

    /// Reset the transient shaper state
    pub fn reset(&mut self) {
        self.state = TransientShaperState::default();
    }
}

impl AudioNode for TransientShaperNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 3,
            "TransientShaperNode requires 3 inputs, got {}",
            inputs.len()
        );

        let input_buf = inputs[0];
        let attack_buf = inputs[1];
        let sustain_buf = inputs[2];

        debug_assert_eq!(
            input_buf.len(),
            output.len(),
            "Input buffer length mismatch"
        );

        // Time constants for envelope followers
        // Fast envelope: very quick attack (~0.3ms), quick release (~5ms)
        // This captures transients accurately
        let fast_attack_time = 0.0003; // 0.3ms
        let fast_release_time = 0.005; // 5ms

        // Slow envelope: slower attack (~15ms), slower release (~40ms)
        // This represents the "body" or sustain of the sound
        let slow_attack_time = 0.015; // 15ms
        let slow_release_time = 0.040; // 40ms

        // Calculate coefficients (these are fixed, could be cached)
        let fast_attack_coeff = (-1.0 / (fast_attack_time * sample_rate)).exp();
        let fast_release_coeff = (-1.0 / (fast_release_time * sample_rate)).exp();
        let slow_attack_coeff = (-1.0 / (slow_attack_time * sample_rate)).exp();
        let slow_release_coeff = (-1.0 / (slow_release_time * sample_rate)).exp();

        for i in 0..output.len() {
            let sample = input_buf[i];
            let attack_db = attack_buf[i].clamp(-24.0, 24.0);
            let sustain_db = sustain_buf[i].clamp(-24.0, 24.0);

            // Full-wave rectification for envelope detection
            let rectified = sample.abs();

            // Update fast envelope (transient tracker)
            if rectified > self.state.fast_envelope {
                // Attack phase
                self.state.fast_envelope = fast_attack_coeff * self.state.fast_envelope
                    + (1.0 - fast_attack_coeff) * rectified;
            } else {
                // Release phase
                self.state.fast_envelope = fast_release_coeff * self.state.fast_envelope
                    + (1.0 - fast_release_coeff) * rectified;
            }

            // Update slow envelope (sustain tracker)
            if rectified > self.state.slow_envelope {
                // Attack phase
                self.state.slow_envelope = slow_attack_coeff * self.state.slow_envelope
                    + (1.0 - slow_attack_coeff) * rectified;
            } else {
                // Release phase
                self.state.slow_envelope = slow_release_coeff * self.state.slow_envelope
                    + (1.0 - slow_release_coeff) * rectified;
            }

            // Compute transient component (difference between fast and slow envelopes)
            // The fast envelope "leads" the slow envelope during attacks
            let transient_envelope = (self.state.fast_envelope - self.state.slow_envelope).max(0.0);

            // Sustain is captured by the slow envelope
            let sustain_envelope = self.state.slow_envelope;

            // Convert dB gains to linear
            let attack_gain = 10.0_f32.powf(attack_db / 20.0);
            let sustain_gain = 10.0_f32.powf(sustain_db / 20.0);

            // The output signal is shaped by applying different gains to different parts
            // We use the envelopes as mixing coefficients, normalized to avoid excessive gain

            // Avoid division by zero
            let total_envelope = (transient_envelope + sustain_envelope).max(0.0001);

            // Compute the ratio of transient vs sustain in the current envelope
            let transient_ratio = transient_envelope / total_envelope;
            let sustain_ratio = sustain_envelope / total_envelope;

            // Apply gains proportionally based on transient/sustain content
            // At neutral (0dB), both gains are 1.0, so output equals input
            let combined_gain = transient_ratio * attack_gain + sustain_ratio * sustain_gain;

            // Apply combined gain to original signal
            output[i] = sample * combined_gain;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.attack_input, self.sustain_input]
    }

    fn name(&self) -> &str {
        "TransientShaperNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::Fraction;
    use std::f32::consts::PI;

    fn create_context(block_size: usize) -> ProcessContext {
        ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, 44100.0)
    }

    #[test]
    fn test_neutral_settings_pass_through() {
        // With attack=0 and sustain=0 (neutral), output should closely match input
        let mut shaper = TransientShaperNode::new(0, 1, 2);

        // Generate a simple test signal (sine wave)
        let mut input = vec![0.0; 1024];
        for i in 0..1024 {
            let t = i as f32 / 44100.0;
            input[i] = 0.5 * (2.0 * PI * 440.0 * t).sin();
        }

        let attack_db = vec![0.0; 1024]; // Neutral
        let sustain_db = vec![0.0; 1024]; // Neutral
        let inputs = vec![input.as_slice(), attack_db.as_slice(), sustain_db.as_slice()];

        let mut output = vec![0.0; 1024];
        let context = create_context(1024);

        shaper.process_block(&inputs, &mut output, 44100.0, &context);

        // Calculate RMS of input and output
        let input_rms: f32 = (input.iter().map(|x| x * x).sum::<f32>() / 1024.0).sqrt();
        let output_rms: f32 = (output.iter().map(|x| x * x).sum::<f32>() / 1024.0).sqrt();

        // Output should be similar to input (within ~10% due to envelope detection)
        let ratio = output_rms / input_rms;
        assert!(
            ratio > 0.8 && ratio < 1.2,
            "Neutral settings should pass through similar level: ratio = {}",
            ratio
        );
    }

    #[test]
    fn test_boost_attack_increases_transients() {
        // Boosting attack should increase level during transients
        let mut shaper_boosted = TransientShaperNode::new(0, 1, 2);
        let mut shaper_neutral = TransientShaperNode::new(0, 1, 2);

        // Create an impulse signal (simulating a drum hit)
        let mut input = vec![0.0; 2048];
        // Sharp attack followed by decay
        for i in 0..200 {
            let t = i as f32;
            input[i] = 0.8 * (-t / 20.0).exp(); // Fast exponential decay
        }

        let boost_attack = vec![12.0; 2048]; // +12 dB boost
        let neutral = vec![0.0; 2048];

        let inputs_boosted = vec![input.as_slice(), boost_attack.as_slice(), neutral.as_slice()];
        let inputs_neutral = vec![input.as_slice(), neutral.as_slice(), neutral.as_slice()];

        let mut output_boosted = vec![0.0; 2048];
        let mut output_neutral = vec![0.0; 2048];
        let context = create_context(2048);

        shaper_boosted.process_block(&inputs_boosted, &mut output_boosted, 44100.0, &context);
        shaper_neutral.process_block(&inputs_neutral, &mut output_neutral, 44100.0, &context);

        // Measure peak level in early portion (transient region)
        let peak_boosted = output_boosted[0..50]
            .iter()
            .map(|x| x.abs())
            .fold(0.0_f32, f32::max);
        let peak_neutral = output_neutral[0..50]
            .iter()
            .map(|x| x.abs())
            .fold(0.0_f32, f32::max);

        // Boosted should have higher peak in transient region
        assert!(
            peak_boosted > peak_neutral,
            "Boosted attack should have higher transient peak: boosted={}, neutral={}",
            peak_boosted,
            peak_neutral
        );
    }

    #[test]
    fn test_reduce_attack_decreases_transients() {
        // Reducing attack should decrease level during transients
        let mut shaper_reduced = TransientShaperNode::new(0, 1, 2);
        let mut shaper_neutral = TransientShaperNode::new(0, 1, 2);

        // Create an impulse signal
        let mut input = vec![0.0; 2048];
        for i in 0..200 {
            let t = i as f32;
            input[i] = 0.8 * (-t / 20.0).exp();
        }

        let reduce_attack = vec![-12.0; 2048]; // -12 dB reduction
        let neutral = vec![0.0; 2048];

        let inputs_reduced = vec![input.as_slice(), reduce_attack.as_slice(), neutral.as_slice()];
        let inputs_neutral = vec![input.as_slice(), neutral.as_slice(), neutral.as_slice()];

        let mut output_reduced = vec![0.0; 2048];
        let mut output_neutral = vec![0.0; 2048];
        let context = create_context(2048);

        shaper_reduced.process_block(&inputs_reduced, &mut output_reduced, 44100.0, &context);
        shaper_neutral.process_block(&inputs_neutral, &mut output_neutral, 44100.0, &context);

        // Measure peak level in early portion
        let peak_reduced = output_reduced[0..50]
            .iter()
            .map(|x| x.abs())
            .fold(0.0_f32, f32::max);
        let peak_neutral = output_neutral[0..50]
            .iter()
            .map(|x| x.abs())
            .fold(0.0_f32, f32::max);

        // Reduced should have lower peak in transient region
        assert!(
            peak_reduced < peak_neutral,
            "Reduced attack should have lower transient peak: reduced={}, neutral={}",
            peak_reduced,
            peak_neutral
        );
    }

    #[test]
    fn test_boost_sustain_increases_body() {
        // Boosting sustain should increase level in sustained portion
        let mut shaper_boosted = TransientShaperNode::new(0, 1, 2);
        let mut shaper_neutral = TransientShaperNode::new(0, 1, 2);

        // Create a sustained signal (long envelope)
        let mut input = vec![0.0; 4096];
        // Quick attack, long sustain
        for i in 0..4000 {
            let t = i as f32;
            // Long decay to represent sustain
            input[i] = 0.5 * (-t / 2000.0).exp();
        }

        let neutral = vec![0.0; 4096];
        let boost_sustain = vec![12.0; 4096]; // +12 dB boost

        let inputs_boosted = vec![input.as_slice(), neutral.as_slice(), boost_sustain.as_slice()];
        let inputs_neutral = vec![input.as_slice(), neutral.as_slice(), neutral.as_slice()];

        let mut output_boosted = vec![0.0; 4096];
        let mut output_neutral = vec![0.0; 4096];
        let context = create_context(4096);

        shaper_boosted.process_block(&inputs_boosted, &mut output_boosted, 44100.0, &context);
        shaper_neutral.process_block(&inputs_neutral, &mut output_neutral, 44100.0, &context);

        // Measure RMS in sustain portion (after initial attack settles)
        let rms_boosted: f32 = (output_boosted[1000..3000]
            .iter()
            .map(|x| x * x)
            .sum::<f32>()
            / 2000.0)
            .sqrt();
        let rms_neutral: f32 = (output_neutral[1000..3000]
            .iter()
            .map(|x| x * x)
            .sum::<f32>()
            / 2000.0)
            .sqrt();

        // Boosted should have higher RMS in sustain region
        assert!(
            rms_boosted > rms_neutral,
            "Boosted sustain should have higher body: boosted={}, neutral={}",
            rms_boosted,
            rms_neutral
        );
    }

    #[test]
    fn test_reduce_sustain_decreases_body() {
        // Reducing sustain should decrease level in sustained portion
        let mut shaper_reduced = TransientShaperNode::new(0, 1, 2);
        let mut shaper_neutral = TransientShaperNode::new(0, 1, 2);

        // Create a sustained signal
        let mut input = vec![0.0; 4096];
        for i in 0..4000 {
            let t = i as f32;
            input[i] = 0.5 * (-t / 2000.0).exp();
        }

        let neutral = vec![0.0; 4096];
        let reduce_sustain = vec![-12.0; 4096]; // -12 dB reduction

        let inputs_reduced = vec![input.as_slice(), neutral.as_slice(), reduce_sustain.as_slice()];
        let inputs_neutral = vec![input.as_slice(), neutral.as_slice(), neutral.as_slice()];

        let mut output_reduced = vec![0.0; 4096];
        let mut output_neutral = vec![0.0; 4096];
        let context = create_context(4096);

        shaper_reduced.process_block(&inputs_reduced, &mut output_reduced, 44100.0, &context);
        shaper_neutral.process_block(&inputs_neutral, &mut output_neutral, 44100.0, &context);

        // Measure RMS in sustain portion
        let rms_reduced: f32 = (output_reduced[1000..3000]
            .iter()
            .map(|x| x * x)
            .sum::<f32>()
            / 2000.0)
            .sqrt();
        let rms_neutral: f32 = (output_neutral[1000..3000]
            .iter()
            .map(|x| x * x)
            .sum::<f32>()
            / 2000.0)
            .sqrt();

        // Reduced should have lower RMS in sustain region
        assert!(
            rms_reduced < rms_neutral,
            "Reduced sustain should have lower body: reduced={}, neutral={}",
            rms_reduced,
            rms_neutral
        );
    }

    #[test]
    fn test_envelope_tracking() {
        // Test that envelope followers track signal correctly
        let mut shaper = TransientShaperNode::new(0, 1, 2);

        // Create an impulse
        let mut input = vec![0.0; 512];
        input[0] = 1.0; // Single impulse

        let neutral = vec![0.0; 512];
        let inputs = vec![input.as_slice(), neutral.as_slice(), neutral.as_slice()];

        let mut output = vec![0.0; 512];
        let context = create_context(512);

        shaper.process_block(&inputs, &mut output, 44100.0, &context);

        // Fast envelope should have tracked and decayed
        assert!(
            shaper.fast_envelope() > 0.0,
            "Fast envelope should track signal"
        );
        assert!(
            shaper.fast_envelope() < 1.0,
            "Fast envelope should have decayed from peak"
        );

        // Slow envelope should also have tracked (but more slowly)
        assert!(
            shaper.slow_envelope() > 0.0,
            "Slow envelope should track signal"
        );
    }

    #[test]
    fn test_preserves_sign() {
        // Transient shaper should preserve signal polarity
        let mut shaper = TransientShaperNode::new(0, 1, 2);

        let input = vec![0.5, -0.5, 0.3, -0.3, 0.8, -0.8];
        let neutral = vec![0.0; 6];
        let inputs = vec![input.as_slice(), neutral.as_slice(), neutral.as_slice()];

        let mut output = vec![0.0; 6];
        let context = create_context(6);

        shaper.process_block(&inputs, &mut output, 44100.0, &context);

        // Check signs are preserved
        assert!(output[0] > 0.0, "Positive input should stay positive");
        assert!(output[1] < 0.0, "Negative input should stay negative");
        assert!(output[2] > 0.0, "Positive input should stay positive");
        assert!(output[3] < 0.0, "Negative input should stay negative");
        assert!(output[4] > 0.0, "Positive input should stay positive");
        assert!(output[5] < 0.0, "Negative input should stay negative");
    }

    #[test]
    fn test_extreme_boost_is_bounded() {
        // Even with extreme boost, output should remain bounded
        let mut shaper = TransientShaperNode::new(0, 1, 2);

        let input = vec![0.5; 512];
        let max_boost = vec![24.0; 512]; // Maximum +24 dB
        let inputs = vec![input.as_slice(), max_boost.as_slice(), max_boost.as_slice()];

        let mut output = vec![0.0; 512];
        let context = create_context(512);

        shaper.process_block(&inputs, &mut output, 44100.0, &context);

        // All outputs should be finite
        for sample in &output {
            assert!(sample.is_finite(), "Output should be finite");
        }
    }

    #[test]
    fn test_extreme_reduction_doesnt_silence() {
        // Even with extreme reduction, signal shouldn't completely disappear
        let mut shaper = TransientShaperNode::new(0, 1, 2);

        let input = vec![0.5; 512];
        let max_reduce = vec![-24.0; 512]; // Maximum -24 dB
        let inputs = vec![
            input.as_slice(),
            max_reduce.as_slice(),
            max_reduce.as_slice(),
        ];

        let mut output = vec![0.0; 512];
        let context = create_context(512);

        shaper.process_block(&inputs, &mut output, 44100.0, &context);

        // Output should still have some level (reduced but not zero)
        let rms: f32 = (output.iter().map(|x| x * x).sum::<f32>() / 512.0).sqrt();
        assert!(rms > 0.0, "Output should not be completely silent");
    }

    #[test]
    fn test_dependencies() {
        let shaper = TransientShaperNode::new(5, 10, 15);
        let deps = shaper.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 5); // input
        assert_eq!(deps[1], 10); // attack
        assert_eq!(deps[2], 15); // sustain
    }

    #[test]
    fn test_reset() {
        let mut shaper = TransientShaperNode::new(0, 1, 2);

        // Process some signal to build up state
        let input = vec![0.8; 512];
        let neutral = vec![0.0; 512];
        let inputs = vec![input.as_slice(), neutral.as_slice(), neutral.as_slice()];
        let mut output = vec![0.0; 512];
        let context = create_context(512);

        shaper.process_block(&inputs, &mut output, 44100.0, &context);

        assert!(shaper.fast_envelope() > 0.0, "State should have built up");
        assert!(shaper.slow_envelope() > 0.0, "State should have built up");

        shaper.reset();

        assert_eq!(
            shaper.fast_envelope(),
            0.0,
            "Fast envelope should be reset"
        );
        assert_eq!(
            shaper.slow_envelope(),
            0.0,
            "Slow envelope should be reset"
        );
    }

    #[test]
    fn test_pattern_modulated_attack() {
        // Test that attack parameter can vary per sample
        let mut shaper = TransientShaperNode::new(0, 1, 2);

        // Impulse signal
        let mut input = vec![0.0; 1024];
        for i in 0..100 {
            let t = i as f32;
            input[i] = 0.8 * (-t / 20.0).exp();
        }
        for i in 512..612 {
            let t = (i - 512) as f32;
            input[i] = 0.8 * (-t / 20.0).exp();
        }

        // First half: boost attack, second half: reduce attack
        let mut attack = vec![12.0; 1024];
        for i in 512..1024 {
            attack[i] = -12.0;
        }
        let neutral = vec![0.0; 1024];

        let inputs = vec![input.as_slice(), attack.as_slice(), neutral.as_slice()];
        let mut output = vec![0.0; 1024];
        let context = create_context(1024);

        shaper.process_block(&inputs, &mut output, 44100.0, &context);

        // First impulse (boosted) should be louder than second (reduced)
        let peak_first = output[0..100]
            .iter()
            .map(|x| x.abs())
            .fold(0.0_f32, f32::max);
        let peak_second = output[512..612]
            .iter()
            .map(|x| x.abs())
            .fold(0.0_f32, f32::max);

        assert!(
            peak_first > peak_second,
            "Boosted attack should be louder: first={}, second={}",
            peak_first,
            peak_second
        );
    }

    #[test]
    fn test_node_name() {
        let shaper = TransientShaperNode::new(0, 1, 2);
        assert_eq!(shaper.name(), "TransientShaperNode");
    }
}
