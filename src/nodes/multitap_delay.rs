/// Multi-tap delay node - multiple equally-spaced echoes for rhythmic patterns
///
/// A multi-tap delay creates complex rhythmic delay patterns by reading from
/// multiple positions in a delay buffer. Each tap is a multiple of the base
/// delay time, with amplitude decreasing for later taps to create natural decay.
///
/// # Algorithm
/// ```text
/// for tap in 1..=num_taps:
///   tap_delay = base_delay * tap
///   read_idx = (write_idx - tap_delay) mod buffer_len
///   tap_amp = 1.0 / tap  // Natural decay (1.0, 0.5, 0.33, 0.25...)
///   tap_sum += buffer[read_idx] * tap_amp
///
/// tap_sum /= num_taps  // Normalize
/// buffer[write_idx] = input + tap_sum * feedback  // Write with feedback
/// output = input * (1 - mix) + tap_sum * mix  // Blend wet/dry
/// ```
///
/// # Applications
/// - Rhythmic delay patterns (eighth notes, sixteenth notes)
/// - Complex echo effects
/// - Slap-back delays with multiple repeats
/// - Doubling and thickening effects
/// - Creative sound design
///
/// # Example
/// ```ignore
/// // Rhythmic 3-tap delay
/// let synth = OscillatorNode::new(Waveform::Saw);  // NodeId 1
/// let time = ConstantNode::new(0.125);              // NodeId 2 (125ms base)
/// let taps = ConstantNode::new(3.0);                // NodeId 3 (3 taps)
/// let feedback = ConstantNode::new(0.5);            // NodeId 4 (50% feedback)
/// let mix = ConstantNode::new(0.6);                 // NodeId 5 (60% wet)
/// let delay = MultiTapDelayNode::new(1, 2, 3, 4, 5, 1.0, 44100.0);  // NodeId 6
/// ```

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Multi-tap delay state
#[derive(Debug, Clone)]
struct MultiTapDelayState {
    buffer: Vec<f32>,   // Circular delay buffer
    write_pos: usize,   // Current write position
}

impl MultiTapDelayState {
    fn new(buffer_size: usize) -> Self {
        Self {
            buffer: vec![0.0; buffer_size],
            write_pos: 0,
        }
    }
}

/// Multi-tap delay node: creates rhythmic delay patterns with multiple taps
///
/// Each tap is positioned at a multiple of the base delay time:
/// - Tap 1: base_delay × 1 (e.g., 100ms)
/// - Tap 2: base_delay × 2 (e.g., 200ms)
/// - Tap 3: base_delay × 3 (e.g., 300ms)
/// - etc.
///
/// Tap amplitudes decrease naturally (1.0, 0.5, 0.33, 0.25...) to create
/// realistic decay patterns.
pub struct MultiTapDelayNode {
    input: NodeId,           // Signal to delay
    time_input: NodeId,      // Base delay time in seconds
    taps_input: NodeId,      // Number of taps (2-8)
    feedback_input: NodeId,  // Feedback amount (0.0-0.95)
    mix_input: NodeId,       // Dry/wet mix (0.0-1.0)
    state: MultiTapDelayState,
    max_delay: f32,          // Maximum delay time (for buffer sizing)
    sample_rate: f32,        // Sample rate for calculations
}

impl MultiTapDelayNode {
    /// Create a new multi-tap delay node
    ///
    /// # Arguments
    /// * `input` - NodeId of signal to delay
    /// * `time_input` - NodeId of base delay time in seconds (typical range: 0.01 to 0.5)
    ///   - 0.05 = 50ms (fast rhythmic patterns)
    ///   - 0.125 = 125ms (eighth note at 120 BPM)
    ///   - 0.25 = 250ms (quarter note at 120 BPM)
    /// * `taps_input` - NodeId of number of taps (2-8)
    ///   - 2 = simple doubling
    ///   - 3 = triplet feel
    ///   - 4 = quadruple echo
    ///   - 8 = dense rhythmic texture
    /// * `feedback_input` - NodeId of feedback amount (0.0 to 0.95)
    ///   - 0.0 = no feedback (single echoes only)
    ///   - 0.5 = moderate feedback (medium decay)
    ///   - 0.8 = high feedback (long decay)
    /// * `mix_input` - NodeId of wet/dry mix (0.0 to 1.0)
    ///   - 0.0 = completely dry (bypass)
    ///   - 0.5 = 50/50 blend
    ///   - 1.0 = completely wet (only delays)
    /// * `max_delay` - Maximum delay time in seconds (determines buffer size)
    /// * `sample_rate` - Sample rate in Hz (usually 44100.0)
    ///
    /// # Panics
    /// Panics if max_delay <= 0.0
    pub fn new(
        input: NodeId,
        time_input: NodeId,
        taps_input: NodeId,
        feedback_input: NodeId,
        mix_input: NodeId,
        max_delay: f32,
        sample_rate: f32,
    ) -> Self {
        assert!(max_delay > 0.0, "max_delay must be greater than 0");

        // Buffer size: max_delay * sample_rate * max_taps (8)
        // This ensures we have enough space for the longest tap
        let buffer_size = (max_delay * sample_rate * 8.0).ceil() as usize;

        Self {
            input,
            time_input,
            taps_input,
            feedback_input,
            mix_input,
            state: MultiTapDelayState::new(buffer_size),
            max_delay,
            sample_rate,
        }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }

    /// Get the time input node ID
    pub fn time_input(&self) -> NodeId {
        self.time_input
    }

    /// Get the taps input node ID
    pub fn taps_input(&self) -> NodeId {
        self.taps_input
    }

    /// Get the feedback input node ID
    pub fn feedback_input(&self) -> NodeId {
        self.feedback_input
    }

    /// Get the mix input node ID
    pub fn mix_input(&self) -> NodeId {
        self.mix_input
    }

    /// Get the current write position (for debugging/testing)
    pub fn write_position(&self) -> usize {
        self.state.write_pos
    }

    /// Get the buffer size
    pub fn buffer_size(&self) -> usize {
        self.state.buffer.len()
    }

    /// Reset the delay buffer to silence
    pub fn clear_buffer(&mut self) {
        self.state.buffer.fill(0.0);
        self.state.write_pos = 0;
    }
}

impl AudioNode for MultiTapDelayNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 5,
            "MultiTapDelayNode requires 5 inputs, got {}",
            inputs.len()
        );

        let input_buf = inputs[0];
        let time_buf = inputs[1];
        let taps_buf = inputs[2];
        let feedback_buf = inputs[3];
        let mix_buf = inputs[4];

        debug_assert_eq!(
            input_buf.len(),
            output.len(),
            "Input buffer length mismatch"
        );

        let buffer_len = self.state.buffer.len();

        for i in 0..output.len() {
            let sample = input_buf[i];
            let base_time = time_buf[i].max(0.001).min(self.max_delay);
            let num_taps = taps_buf[i].round() as usize;
            let num_taps = num_taps.clamp(2, 8); // 2-8 taps
            let feedback = feedback_buf[i].clamp(0.0, 0.95);
            let mix = mix_buf[i].clamp(0.0, 1.0);

            let base_delay_samples = (base_time * self.sample_rate) as usize;

            // Sum multiple tap outputs
            let mut tap_sum = 0.0;

            for tap_num in 1..=num_taps {
                let tap_delay = base_delay_samples * tap_num;

                // Only read tap if it fits in the buffer
                if tap_delay < buffer_len {
                    let read_idx = (self.state.write_pos + buffer_len - tap_delay) % buffer_len;

                    // Natural amplitude decay: 1.0, 0.5, 0.33, 0.25...
                    let tap_amp = 1.0 / (tap_num as f32);

                    tap_sum += self.state.buffer[read_idx] * tap_amp;
                }
            }

            // Normalize by number of taps
            tap_sum /= num_taps as f32;

            // Write input plus feedback to buffer
            let to_write = sample + tap_sum * feedback;
            self.state.buffer[self.state.write_pos] = to_write;

            // Advance write position
            self.state.write_pos = (self.state.write_pos + 1) % buffer_len;

            // Mix dry and wet signals
            output[i] = sample * (1.0 - mix) + tap_sum * mix;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![
            self.input,
            self.time_input,
            self.taps_input,
            self.feedback_input,
            self.mix_input,
        ]
    }

    fn name(&self) -> &str {
        "MultiTapDelayNode"
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
    fn test_multitap_delay_bypass() {
        // Test that mix=0.0 passes signal through unchanged
        let size = 512;
        let sample_rate = 44100.0;

        let input = vec![0.5; size];
        let time = vec![0.1; size];    // 100ms
        let taps = vec![3.0; size];    // 3 taps
        let feedback = vec![0.5; size];
        let mix = vec![0.0; size];      // Bypass

        let inputs: Vec<&[f32]> = vec![&input, &time, &taps, &feedback, &mix];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut delay = MultiTapDelayNode::new(0, 1, 2, 3, 4, 1.0, sample_rate);
        delay.process_block(&inputs, &mut output, sample_rate, &context);

        // Output should equal input (bypass)
        for i in 0..size {
            assert!(
                (output[i] - input[i]).abs() < 0.0001,
                "With mix=0, output should equal input"
            );
        }
    }

    #[test]
    fn test_multitap_delay_creates_echoes() {
        // Test that delay creates multiple echoes
        let size = 1024;
        let sample_rate = 44100.0;

        // Create impulse at start
        let mut input = vec![0.0; size];
        input[0] = 1.0; // Impulse

        let delay_time = 0.01; // 10ms = 441 samples at 44.1kHz
        let time = vec![delay_time; size];
        let taps = vec![3.0; size];     // 3 taps
        let feedback = vec![0.0; size]; // No feedback (cleaner test)
        let mix = vec![1.0; size];       // Full wet

        let inputs: Vec<&[f32]> = vec![&input, &time, &taps, &feedback, &mix];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut delay = MultiTapDelayNode::new(0, 1, 2, 3, 4, 1.0, sample_rate);
        delay.process_block(&inputs, &mut output, sample_rate, &context);

        // Check for echoes at expected positions
        let delay_samples = (delay_time * sample_rate) as usize;

        // Tap 1 at delay_samples * 1
        let tap1_idx = delay_samples;
        if tap1_idx < size {
            assert!(
                output[tap1_idx] > 0.2,
                "First tap should appear at sample {}, got {}",
                tap1_idx,
                output[tap1_idx]
            );
        }

        // Tap 2 at delay_samples * 2
        let tap2_idx = delay_samples * 2;
        if tap2_idx < size {
            assert!(
                output[tap2_idx] > 0.1,
                "Second tap should appear at sample {}, got {}",
                tap2_idx,
                output[tap2_idx]
            );
        }

        // Tap 3 at delay_samples * 3
        let tap3_idx = delay_samples * 3;
        if tap3_idx < size {
            assert!(
                output[tap3_idx] > 0.05,
                "Third tap should appear at sample {}, got {}",
                tap3_idx,
                output[tap3_idx]
            );
        }
    }

    #[test]
    fn test_multitap_delay_amplitude_decay() {
        // Test that later taps have lower amplitude
        let size = 1024;
        let sample_rate = 44100.0;

        // Impulse
        let mut input = vec![0.0; size];
        input[0] = 1.0;

        let delay_time = 0.01; // 10ms
        let time = vec![delay_time; size];
        let taps = vec![4.0; size];      // 4 taps
        let feedback = vec![0.0; size];  // No feedback
        let mix = vec![1.0; size];

        let inputs: Vec<&[f32]> = vec![&input, &time, &taps, &feedback, &mix];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut delay = MultiTapDelayNode::new(0, 1, 2, 3, 4, 1.0, sample_rate);
        delay.process_block(&inputs, &mut output, sample_rate, &context);

        let delay_samples = (delay_time * sample_rate) as usize;

        // Find peak amplitudes for each tap
        let mut tap_amps = Vec::new();
        for tap_num in 1..=4 {
            let tap_idx = delay_samples * tap_num;
            if tap_idx < size {
                // Find peak in window around expected tap position
                let window_start = tap_idx.saturating_sub(5);
                let window_end = (tap_idx + 5).min(size);
                let peak = output[window_start..window_end]
                    .iter()
                    .copied()
                    .fold(0.0f32, f32::max);
                tap_amps.push(peak);
            }
        }

        // Verify decreasing amplitude
        for i in 1..tap_amps.len() {
            assert!(
                tap_amps[i] < tap_amps[i - 1],
                "Tap {} amplitude ({}) should be less than tap {} amplitude ({})",
                i + 1,
                tap_amps[i],
                i,
                tap_amps[i - 1]
            );
        }
    }

    #[test]
    fn test_multitap_delay_feedback() {
        // Test that feedback creates repeated echoes
        let size = 2048;
        let sample_rate = 44100.0;

        // Impulse
        let mut input = vec![0.0; size];
        input[0] = 1.0;

        let delay_time = 0.01; // 10ms
        let time = vec![delay_time; size];
        let taps = vec![2.0; size];      // 2 taps
        let feedback = vec![0.6; size];  // Significant feedback
        let mix = vec![1.0; size];

        let inputs: Vec<&[f32]> = vec![&input, &time, &taps, &feedback, &mix];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut delay = MultiTapDelayNode::new(0, 1, 2, 3, 4, 1.0, sample_rate);
        delay.process_block(&inputs, &mut output, sample_rate, &context);

        // Count number of peaks (echoes)
        let mut peak_count = 0;
        for i in 1..size {
            if output[i] > 0.05 && output[i] > output[i - 1] {
                peak_count += 1;
            }
        }

        // With feedback, we should have more than just the initial taps
        assert!(
            peak_count > 2,
            "Feedback should create multiple echoes, found {} peaks",
            peak_count
        );
    }

    #[test]
    fn test_multitap_delay_num_taps_clamping() {
        // Test that number of taps is clamped to 2-8
        let size = 512;
        let sample_rate = 44100.0;

        let input = vec![0.5; size];
        let time = vec![0.1; size];
        let feedback = vec![0.0; size];
        let mix = vec![0.5; size];

        // Test too few taps (should clamp to 2)
        let taps_low = vec![0.0; size];
        let inputs_low: Vec<&[f32]> = vec![&input, &time, &taps_low, &feedback, &mix];
        let mut output_low = vec![0.0; size];
        let context = create_context(size);

        let mut delay_low = MultiTapDelayNode::new(0, 1, 2, 3, 4, 1.0, sample_rate);
        delay_low.process_block(&inputs_low, &mut output_low, sample_rate, &context);

        // Should not panic, and should produce output
        let rms_low: f32 = output_low.iter().map(|x| x * x).sum::<f32>() / size as f32;
        assert!(rms_low > 0.0, "Should produce output even with invalid tap count");

        // Test too many taps (should clamp to 8)
        let taps_high = vec![100.0; size];
        let inputs_high: Vec<&[f32]> = vec![&input, &time, &taps_high, &feedback, &mix];
        let mut output_high = vec![0.0; size];

        let mut delay_high = MultiTapDelayNode::new(0, 1, 2, 3, 4, 1.0, sample_rate);
        delay_high.process_block(&inputs_high, &mut output_high, sample_rate, &context);

        // Should not panic
        let rms_high: f32 = output_high.iter().map(|x| x * x).sum::<f32>() / size as f32;
        assert!(rms_high > 0.0, "Should produce output with clamped tap count");
    }

    #[test]
    fn test_multitap_delay_varying_parameters() {
        // Test that parameters can vary over time
        let size = 512;
        let sample_rate = 44100.0;

        let input = vec![0.3; size];

        // Vary number of taps over time
        let mut taps = vec![0.0; size];
        for i in 0..size {
            taps[i] = 2.0 + (i as f32 / size as f32) * 6.0; // Ramp from 2 to 8
        }

        let time = vec![0.05; size];
        let feedback = vec![0.3; size];
        let mix = vec![0.5; size];

        let inputs: Vec<&[f32]> = vec![&input, &time, &taps, &feedback, &mix];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut delay = MultiTapDelayNode::new(0, 1, 2, 3, 4, 1.0, sample_rate);
        delay.process_block(&inputs, &mut output, sample_rate, &context);

        // Output should vary as tap count changes
        let early_rms: f32 = output[0..size / 4]
            .iter()
            .map(|x| x * x)
            .sum::<f32>()
            / (size / 4) as f32;
        let late_rms: f32 = output[3 * size / 4..]
            .iter()
            .map(|x| x * x)
            .sum::<f32>()
            / (size / 4) as f32;

        // More taps should generally create different spectral content
        assert!(
            early_rms > 0.0 && late_rms > 0.0,
            "Both sections should have energy"
        );
    }

    #[test]
    fn test_multitap_delay_mix_blending() {
        // Test wet/dry mix blending
        let size = 512;
        let sample_rate = 44100.0;

        let input = vec![0.5; size];
        let time = vec![0.05; size];
        let taps = vec![3.0; size];
        let feedback = vec![0.4; size];

        // Test different mix values
        for &mix_val in &[0.0, 0.25, 0.5, 0.75, 1.0] {
            let mix = vec![mix_val; size];
            let inputs: Vec<&[f32]> = vec![&input, &time, &taps, &feedback, &mix];
            let mut output = vec![0.0; size];
            let context = create_context(size);

            let mut delay = MultiTapDelayNode::new(0, 1, 2, 3, 4, 1.0, sample_rate);
            delay.process_block(&inputs, &mut output, sample_rate, &context);

            // At mix=0, should be mostly input
            // At mix=1, should be mostly delayed signal
            let avg = output.iter().sum::<f32>() / size as f32;

            if mix_val == 0.0 {
                // Should be close to input
                assert!(
                    (avg - 0.5).abs() < 0.1,
                    "Mix=0 should approximate input, got avg={}",
                    avg
                );
            }

            // All outputs should be valid
            for &val in &output {
                assert!(
                    val.is_finite(),
                    "Output should be finite at mix={}, got {}",
                    mix_val,
                    val
                );
            }
        }
    }

    #[test]
    fn test_multitap_delay_buffer_wraparound() {
        // Test that circular buffer wraps around correctly
        let size = 1024;
        let sample_rate = 44100.0;

        // Continuous input
        let input = vec![0.1; size];
        let time = vec![0.01; size]; // 10ms
        let taps = vec![2.0; size];
        let feedback = vec![0.3; size];
        let mix = vec![0.5; size];

        let inputs: Vec<&[f32]> = vec![&input, &time, &taps, &feedback, &mix];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut delay = MultiTapDelayNode::new(0, 1, 2, 3, 4, 0.5, sample_rate);

        // Process multiple blocks to test wraparound
        for _ in 0..5 {
            delay.process_block(&inputs, &mut output, sample_rate, &context);

            // Should not have NaN or inf
            for &val in &output {
                assert!(val.is_finite(), "Output should remain finite");
            }
        }

        // Write position should have wrapped
        assert!(
            delay.write_position() < delay.buffer_size(),
            "Write position should stay within buffer bounds"
        );
    }

    #[test]
    fn test_multitap_delay_node_interface() {
        // Test node getters
        let delay = MultiTapDelayNode::new(10, 11, 12, 13, 14, 1.0, 44100.0);

        assert_eq!(delay.input(), 10);
        assert_eq!(delay.time_input(), 11);
        assert_eq!(delay.taps_input(), 12);
        assert_eq!(delay.feedback_input(), 13);
        assert_eq!(delay.mix_input(), 14);

        let inputs = delay.input_nodes();
        assert_eq!(inputs.len(), 5);
        assert_eq!(inputs[0], 10);
        assert_eq!(inputs[1], 11);
        assert_eq!(inputs[2], 12);
        assert_eq!(inputs[3], 13);
        assert_eq!(inputs[4], 14);

        assert_eq!(delay.name(), "MultiTapDelayNode");
        assert!(delay.buffer_size() > 0);
    }

    #[test]
    fn test_multitap_delay_clear_buffer() {
        // Test that clearing buffer resets state
        let size = 512;
        let sample_rate = 44100.0;

        let input = vec![0.5; size];
        let time = vec![0.1; size];
        let taps = vec![3.0; size];
        let feedback = vec![0.5; size];
        let mix = vec![0.8; size];

        let inputs: Vec<&[f32]> = vec![&input, &time, &taps, &feedback, &mix];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut delay = MultiTapDelayNode::new(0, 1, 2, 3, 4, 1.0, sample_rate);

        // Process to build up state
        delay.process_block(&inputs, &mut output, sample_rate, &context);

        let pos_before = delay.write_position();
        assert!(pos_before > 0, "Write position should advance");

        // Clear buffer
        delay.clear_buffer();

        assert_eq!(delay.write_position(), 0, "Write position should be reset");

        // Process silence and verify buffer is clear
        let silence = vec![0.0; size];
        let inputs_silent: Vec<&[f32]> = vec![&silence, &time, &taps, &feedback, &mix];
        let mut output_silent = vec![0.0; size];

        delay.process_block(&inputs_silent, &mut output_silent, sample_rate, &context);

        // Output should be very close to 0 (only dry signal from mix)
        for &val in &output_silent {
            assert!(
                val.abs() < 0.001,
                "After clear, silent input should produce near-zero output"
            );
        }
    }
}
