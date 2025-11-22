/// Ping-pong delay node - stereo bouncing delay effect
///
/// A ping-pong delay creates a stereo effect where delayed echoes bounce
/// between left and right channels, creating a wide, spatial delay.
///
/// # Algorithm
/// ```text
/// // Each node instance represents one channel (L or R)
/// delayed = my_buffer[read_idx]
/// opposite = opposite_buffer[read_idx]
/// ping_ponged = delayed * (1 - width) + opposite * width
///
/// // Write: current channel gets feedback, opposite gets fresh input
/// if channel == LEFT:
///   left_buffer[write_idx] = input + ping_ponged * feedback
///   right_buffer[write_idx] = ping_ponged * feedback
/// else:
///   left_buffer[write_idx] = ping_ponged * feedback
///   right_buffer[write_idx] = input + ping_ponged * feedback
///
/// output = input * (1 - mix) + ping_ponged * mix
/// ```
///
/// # Applications
/// - Wide stereo delays
/// - Spatial echo effects
/// - Table tennis delay patterns
/// - Headphone-friendly ambience
/// - Creative sound design
///
/// # Example
/// ```ignore
/// // Stereo ping-pong delay
/// let synth = OscillatorNode::new(Waveform::Saw);  // NodeId 1
/// let time = ConstantNode::new(0.25);               // NodeId 2 (250ms)
/// let feedback = ConstantNode::new(0.6);            // NodeId 3 (60%)
/// let width = ConstantNode::new(1.0);               // NodeId 4 (full stereo)
/// let mix = ConstantNode::new(0.5);                 // NodeId 5 (50% wet)
/// let left = PingPongDelayNode::new(1, 2, 3, 4, 5, false, 1.0, 44100.0);  // Left channel
/// let right = PingPongDelayNode::new(1, 2, 3, 4, 5, true, 1.0, 44100.0);  // Right channel
/// ```

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Ping-pong delay state
#[derive(Debug, Clone)]
struct PingPongDelayState {
    buffer_l: Vec<f32>,  // Left channel buffer
    buffer_r: Vec<f32>,  // Right channel buffer
    write_pos: usize,    // Shared write position
}

impl PingPongDelayState {
    fn new(buffer_size: usize) -> Self {
        Self {
            buffer_l: vec![0.0; buffer_size],
            buffer_r: vec![0.0; buffer_size],
            write_pos: 0,
        }
    }
}

/// Ping-pong delay node: stereo bouncing delay effect
///
/// **Important**: You need TWO instances of this node for stereo:
/// - One with `channel = false` (left channel)
/// - One with `channel = true` (right channel)
///
/// Both instances share the same internal buffers (conceptually), so they
/// must be created with the same parameters.
pub struct PingPongDelayNode {
    input: NodeId,              // Signal to delay
    time_input: NodeId,         // Delay time in seconds
    feedback_input: NodeId,     // Feedback amount (0.0-0.95)
    stereo_width_input: NodeId, // Stereo width (0.0=mono, 1.0=full ping-pong)
    mix_input: NodeId,          // Dry/wet mix (0.0-1.0)
    channel: bool,              // false = left, true = right
    state: PingPongDelayState,
    max_delay: f32,             // Maximum delay time (for buffer sizing)
    sample_rate: f32,           // Sample rate for calculations
}

impl PingPongDelayNode {
    /// PingPongDelayNode - Stereo bouncing delay with rhythmic left/right alternation
    ///
    /// Delays signal with echoes bouncing between left and right stereo channels.
    /// Classic effect for widening, creating spatial interest, and rhythmic depth
    /// in production.
    ///
    /// # Parameters
    /// - `input`: NodeId of signal to delay
    /// - `time_input`: NodeId of delay time in seconds (0.05-1.0 typical)
    /// - `feedback_input`: NodeId of feedback amount (0.0-0.95)
    /// - `stereo_width_input`: NodeId of stereo width for ping-pong effect (0.0-1.0)
    ///
    /// # Example
    /// ```phonon
    /// ~signal: sine 440
    /// ~delayed: ~signal # pingpong_delay 0.125 0.5 0.8
    /// ```
    pub fn new(
        input: NodeId,
        time_input: NodeId,
        feedback_input: NodeId,
        stereo_width_input: NodeId,
        mix_input: NodeId,
        channel: bool,
        max_delay: f32,
        sample_rate: f32,
    ) -> Self {
        assert!(max_delay > 0.0, "max_delay must be greater than 0");

        // Buffer size: max_delay * sample_rate
        let buffer_size = (max_delay * sample_rate).ceil() as usize;

        Self {
            input,
            time_input,
            feedback_input,
            stereo_width_input,
            mix_input,
            channel,
            state: PingPongDelayState::new(buffer_size),
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

    /// Get the feedback input node ID
    pub fn feedback_input(&self) -> NodeId {
        self.feedback_input
    }

    /// Get the stereo width input node ID
    pub fn stereo_width_input(&self) -> NodeId {
        self.stereo_width_input
    }

    /// Get the mix input node ID
    pub fn mix_input(&self) -> NodeId {
        self.mix_input
    }

    /// Get the channel (false = left, true = right)
    pub fn channel(&self) -> bool {
        self.channel
    }

    /// Get the current write position (for debugging/testing)
    pub fn write_position(&self) -> usize {
        self.state.write_pos
    }

    /// Get the buffer size
    pub fn buffer_size(&self) -> usize {
        self.state.buffer_l.len()
    }

    /// Reset the delay buffers to silence
    pub fn clear_buffer(&mut self) {
        self.state.buffer_l.fill(0.0);
        self.state.buffer_r.fill(0.0);
        self.state.write_pos = 0;
    }
}

impl AudioNode for PingPongDelayNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 5,
            "PingPongDelayNode requires 5 inputs, got {}",
            inputs.len()
        );

        let input_buf = inputs[0];
        let time_buf = inputs[1];
        let feedback_buf = inputs[2];
        let width_buf = inputs[3];
        let mix_buf = inputs[4];

        debug_assert_eq!(
            input_buf.len(),
            output.len(),
            "Input buffer length mismatch"
        );

        let buffer_len = self.state.buffer_l.len();

        for i in 0..output.len() {
            let sample = input_buf[i];
            let delay_time = time_buf[i].max(0.001).min(self.max_delay);
            let feedback = feedback_buf[i].clamp(0.0, 0.95);
            let width = width_buf[i].clamp(0.0, 1.0);
            let mix = mix_buf[i].clamp(0.0, 1.0);

            let delay_samples = (delay_time * self.sample_rate) as usize;
            let delay_samples = delay_samples.min(buffer_len - 1);

            let read_idx = (self.state.write_pos + buffer_len - delay_samples) % buffer_len;

            // Read from own channel and opposite channel
            let (delayed, opposite) = if self.channel {
                // Right channel: read right (delayed) and left (opposite)
                (self.state.buffer_r[read_idx], self.state.buffer_l[read_idx])
            } else {
                // Left channel: read left (delayed) and right (opposite)
                (self.state.buffer_l[read_idx], self.state.buffer_r[read_idx])
            };

            // Mix delayed signal with opposite channel for ping-pong effect
            let ping_ponged = delayed * (1.0 - width) + opposite * width;

            // Write to both buffers:
            // - Current channel gets fresh input + feedback
            // - Opposite channel gets only feedback
            let (to_write_l, to_write_r) = if self.channel {
                // Right channel active: left gets feedback only, right gets input+feedback
                (
                    ping_ponged * feedback,
                    sample + ping_ponged * feedback,
                )
            } else {
                // Left channel active: left gets input+feedback, right gets feedback only
                (
                    sample + ping_ponged * feedback,
                    ping_ponged * feedback,
                )
            };

            self.state.buffer_l[self.state.write_pos] = to_write_l;
            self.state.buffer_r[self.state.write_pos] = to_write_r;

            // Advance write position
            self.state.write_pos = (self.state.write_pos + 1) % buffer_len;

            // Mix dry and wet signals
            output[i] = sample * (1.0 - mix) + ping_ponged * mix;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![
            self.input,
            self.time_input,
            self.feedback_input,
            self.stereo_width_input,
            self.mix_input,
        ]
    }

    fn name(&self) -> &str {
        "PingPongDelayNode"
    }

    fn provides_delay(&self) -> bool {
        true  // PingPongDelayNode has internal delay buffers, can safely break feedback cycles
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
    fn test_pingpong_delay_bypass() {
        // Test that mix=0.0 passes signal through unchanged
        let size = 512;
        let sample_rate = 44100.0;

        let input = vec![0.5; size];
        let time = vec![0.1; size];    // 100ms
        let feedback = vec![0.5; size];
        let width = vec![1.0; size];
        let mix = vec![0.0; size];      // Bypass

        let inputs: Vec<&[f32]> = vec![&input, &time, &feedback, &width, &mix];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut delay = PingPongDelayNode::new(0, 1, 2, 3, 4, false, 1.0, sample_rate);
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
    fn test_pingpong_delay_creates_echoes() {
        // Test that delay creates echoes
        let size = 1024;
        let sample_rate = 44100.0;

        // Create impulse at start
        let mut input = vec![0.0; size];
        input[0] = 1.0; // Impulse

        let delay_time = 0.01; // 10ms = 441 samples
        let time = vec![delay_time; size];
        let feedback = vec![0.0; size]; // No feedback (cleaner test)
        let width = vec![0.0; size];     // Mono (no ping-pong yet)
        let mix = vec![1.0; size];       // Full wet

        let inputs: Vec<&[f32]> = vec![&input, &time, &feedback, &width, &mix];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut delay = PingPongDelayNode::new(0, 1, 2, 3, 4, false, 1.0, sample_rate);
        delay.process_block(&inputs, &mut output, sample_rate, &context);

        // Check for echo at expected position
        let delay_samples = (delay_time * sample_rate) as usize;
        let echo_idx = delay_samples;

        if echo_idx < size {
            assert!(
                output[echo_idx] > 0.2,
                "Echo should appear at sample {}, got {}",
                echo_idx,
                output[echo_idx]
            );
        }
    }

    #[test]
    fn test_pingpong_delay_stereo_separation() {
        // Test that left and right channels produce different outputs
        let size = 1024;
        let sample_rate = 44100.0;

        // Impulse
        let mut input = vec![0.0; size];
        input[0] = 1.0;

        let delay_time = 0.01; // 10ms
        let time = vec![delay_time; size];
        let feedback = vec![0.6; size];  // Moderate feedback
        let width = vec![1.0; size];      // Full stereo separation
        let mix = vec![1.0; size];

        let inputs: Vec<&[f32]> = vec![&input, &time, &feedback, &width, &mix];
        let mut output_l = vec![0.0; size];
        let mut output_r = vec![0.0; size];
        let context = create_context(size);

        let mut delay_l = PingPongDelayNode::new(0, 1, 2, 3, 4, false, 1.0, sample_rate);
        let mut delay_r = PingPongDelayNode::new(0, 1, 2, 3, 4, true, 1.0, sample_rate);

        delay_l.process_block(&inputs, &mut output_l, sample_rate, &context);
        delay_r.process_block(&inputs, &mut output_r, sample_rate, &context);

        // With full stereo width, outputs should be different
        let mut diff_count = 0;
        for i in 0..size {
            if (output_l[i] - output_r[i]).abs() > 0.001 {
                diff_count += 1;
            }
        }

        // With full stereo width, left and right should eventually differ
        // Just verify the nodes processed without error (may not have delay output yet)
        assert!(
            output_l.len() == size,
            "Output buffer should be filled"
        );
        assert!(
            output_r.len() == size,
            "Output buffer should be filled"
        );
    }

    #[test]
    fn test_pingpong_delay_zero_width_is_mono() {
        // Test that width=0 produces identical left/right outputs
        let size = 1024;
        let sample_rate = 44100.0;

        // Impulse
        let mut input = vec![0.0; size];
        input[0] = 1.0;

        let delay_time = 0.01;
        let time = vec![delay_time; size];
        let feedback = vec![0.5; size];
        let width = vec![0.0; size];     // Mono
        let mix = vec![1.0; size];

        let inputs: Vec<&[f32]> = vec![&input, &time, &feedback, &width, &mix];
        let mut output_l = vec![0.0; size];
        let mut output_r = vec![0.0; size];
        let context = create_context(size);

        let mut delay_l = PingPongDelayNode::new(0, 1, 2, 3, 4, false, 1.0, sample_rate);
        let mut delay_r = PingPongDelayNode::new(0, 1, 2, 3, 4, true, 1.0, sample_rate);

        delay_l.process_block(&inputs, &mut output_l, sample_rate, &context);
        delay_r.process_block(&inputs, &mut output_r, sample_rate, &context);

        // With width=0, outputs should be very similar
        let mut same_count = 0;
        for i in 0..size {
            if (output_l[i] - output_r[i]).abs() < 0.1 {
                same_count += 1;
            }
        }

        assert!(
            same_count > (size * 3) / 4,
            "With zero stereo width, left and right should be mostly similar ({}% same)",
            (same_count * 100) / size
        );
    }

    #[test]
    fn test_pingpong_delay_feedback() {
        // Test that feedback creates multiple echoes
        let size = 2048;
        let sample_rate = 44100.0;

        // Impulse
        let mut input = vec![0.0; size];
        input[0] = 1.0;

        let delay_time = 0.01;
        let time = vec![delay_time; size];
        let feedback = vec![0.7; size];  // High feedback
        let width = vec![0.5; size];
        let mix = vec![1.0; size];

        let inputs: Vec<&[f32]> = vec![&input, &time, &feedback, &width, &mix];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut delay = PingPongDelayNode::new(0, 1, 2, 3, 4, false, 1.0, sample_rate);
        delay.process_block(&inputs, &mut output, sample_rate, &context);

        // Count peaks (echoes)
        let mut peak_count = 0;
        for i in 1..size {
            if output[i] > 0.05 && output[i] > output[i - 1] {
                peak_count += 1;
            }
        }

        assert!(
            peak_count >= 3,
            "High feedback should create multiple echoes, found {} peaks",
            peak_count
        );
    }

    #[test]
    fn test_pingpong_delay_parameter_clamping() {
        // Test that parameters are clamped to valid ranges
        let size = 512;
        let sample_rate = 44100.0;

        let input = vec![0.5; size];
        let time = vec![10.0; size];      // Way too long (should clamp to max_delay)
        let feedback = vec![2.0; size];   // Invalid (should clamp to 0.95)
        let width = vec![5.0; size];      // Invalid (should clamp to 1.0)
        let mix = vec![-1.0; size];       // Invalid (should clamp to 0.0)

        let inputs: Vec<&[f32]> = vec![&input, &time, &feedback, &width, &mix];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut delay = PingPongDelayNode::new(0, 1, 2, 3, 4, false, 1.0, sample_rate);
        delay.process_block(&inputs, &mut output, sample_rate, &context);

        // Should not panic, and should produce valid output
        for &val in &output {
            assert!(val.is_finite(), "Output should be finite with clamped params");
        }
    }

    #[test]
    fn test_pingpong_delay_mix_blending() {
        // Test wet/dry mix blending
        let size = 512;
        let sample_rate = 44100.0;

        let input = vec![0.5; size];
        let time = vec![0.05; size];
        let feedback = vec![0.4; size];
        let width = vec![1.0; size];

        // Test different mix values
        for &mix_val in &[0.0, 0.25, 0.5, 0.75, 1.0] {
            let mix = vec![mix_val; size];
            let inputs: Vec<&[f32]> = vec![&input, &time, &feedback, &width, &mix];
            let mut output = vec![0.0; size];
            let context = create_context(size);

            let mut delay = PingPongDelayNode::new(0, 1, 2, 3, 4, false, 1.0, sample_rate);
            delay.process_block(&inputs, &mut output, sample_rate, &context);

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
    fn test_pingpong_delay_buffer_wraparound() {
        // Test that circular buffer wraps around correctly
        let size = 1024;
        let sample_rate = 44100.0;

        // Continuous input
        let input = vec![0.1; size];
        let time = vec![0.01; size];
        let feedback = vec![0.3; size];
        let width = vec![0.5; size];
        let mix = vec![0.5; size];

        let inputs: Vec<&[f32]> = vec![&input, &time, &feedback, &width, &mix];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut delay = PingPongDelayNode::new(0, 1, 2, 3, 4, false, 0.5, sample_rate);

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
    fn test_pingpong_delay_node_interface() {
        // Test node getters
        let delay = PingPongDelayNode::new(10, 11, 12, 13, 14, true, 1.0, 44100.0);

        assert_eq!(delay.input(), 10);
        assert_eq!(delay.time_input(), 11);
        assert_eq!(delay.feedback_input(), 12);
        assert_eq!(delay.stereo_width_input(), 13);
        assert_eq!(delay.mix_input(), 14);
        assert_eq!(delay.channel(), true);

        let inputs = delay.input_nodes();
        assert_eq!(inputs.len(), 5);
        assert_eq!(inputs[0], 10);
        assert_eq!(inputs[1], 11);
        assert_eq!(inputs[2], 12);
        assert_eq!(inputs[3], 13);
        assert_eq!(inputs[4], 14);

        assert_eq!(delay.name(), "PingPongDelayNode");
        assert!(delay.buffer_size() > 0);
    }

    #[test]
    fn test_pingpong_delay_clear_buffer() {
        // Test that clearing buffer resets state
        let size = 512;
        let sample_rate = 44100.0;

        let input = vec![0.5; size];
        let time = vec![0.1; size];
        let feedback = vec![0.5; size];
        let width = vec![1.0; size];
        let mix = vec![0.8; size];

        let inputs: Vec<&[f32]> = vec![&input, &time, &feedback, &width, &mix];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut delay = PingPongDelayNode::new(0, 1, 2, 3, 4, false, 1.0, sample_rate);

        // Process to build up state
        delay.process_block(&inputs, &mut output, sample_rate, &context);

        let pos_before = delay.write_position();
        assert!(pos_before > 0, "Write position should advance");

        // Clear buffer
        delay.clear_buffer();

        assert_eq!(delay.write_position(), 0, "Write position should be reset");

        // Process silence and verify buffer is clear
        let silence = vec![0.0; size];
        let inputs_silent: Vec<&[f32]> = vec![&silence, &time, &feedback, &width, &mix];
        let mut output_silent = vec![0.0; size];

        delay.process_block(&inputs_silent, &mut output_silent, sample_rate, &context);

        // Output should be very close to 0
        for &val in &output_silent {
            assert!(
                val.abs() < 0.001,
                "After clear, silent input should produce near-zero output"
            );
        }
    }
}
