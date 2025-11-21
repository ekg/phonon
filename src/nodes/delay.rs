/// Delay node - simple delay line with circular buffer
///
/// This node implements a basic delay effect with pattern-controlled delay time.
/// Uses a circular buffer for efficient memory usage.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Delay node with pattern-controlled delay time
///
/// # Example
/// ```ignore
/// // 100ms delay on signal
/// let input_signal = OscillatorNode::new(0, Waveform::Sine);  // NodeId 0
/// let delay_time = ConstantNode::new(0.1);  // 0.1 seconds = 100ms, NodeId 1
/// let delay = DelayNode::new(0, 1, 1.0, 44100.0);  // NodeId 2, max_delay = 1.0s
/// ```
pub struct DelayNode {
    input: NodeId,           // Signal to delay
    delay_time_input: NodeId, // Delay time in seconds (can be modulated)
    buffer: Vec<f32>,        // Circular buffer
    write_pos: usize,        // Current write position
    max_delay: f32,          // Maximum delay time in seconds
    sample_rate: f32,        // Sample rate for calculations
}

impl DelayNode {
    /// Delay - Simple delay line with pattern-controlled timing
    ///
    /// Implements a basic delay effect using a circular buffer,
    /// with delay time controllable at sample rate.
    ///
    /// # Parameters
    /// - `input`: NodeId providing signal to delay
    /// - `delay_time_input`: NodeId providing delay time in seconds
    /// - `max_delay`: Maximum delay time in seconds (determines buffer size)
    /// - `sample_rate`: Sample rate in Hz (usually 44100.0)
    ///
    /// # Example
    /// ```phonon
    /// ~signal: sine 440
    /// ~delayed: ~signal # delay 0.2 1.0 44100
    /// ```
    pub fn new(input: NodeId, delay_time_input: NodeId, max_delay: f32, sample_rate: f32) -> Self {
        assert!(max_delay > 0.0, "max_delay must be greater than 0");

        // Allocate circular buffer: max_delay seconds * sample_rate samples/second
        let buffer_size = (max_delay * sample_rate).ceil() as usize;

        Self {
            input,
            delay_time_input,
            buffer: vec![0.0; buffer_size],
            write_pos: 0,
            max_delay,
            sample_rate,
        }
    }

    /// Get the current write position in the buffer
    pub fn write_position(&self) -> usize {
        self.write_pos
    }

    /// Get the buffer size
    pub fn buffer_size(&self) -> usize {
        self.buffer.len()
    }

    /// Reset the delay buffer to silence
    pub fn clear_buffer(&mut self) {
        self.buffer.fill(0.0);
        self.write_pos = 0;
    }
}

impl AudioNode for DelayNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "DelayNode requires 2 inputs: signal and delay_time"
        );

        let input_buffer = inputs[0];
        let delay_time_buffer = inputs[1];

        debug_assert_eq!(
            input_buffer.len(),
            output.len(),
            "Input buffer length mismatch"
        );
        debug_assert_eq!(
            delay_time_buffer.len(),
            output.len(),
            "Delay time buffer length mismatch"
        );

        let buffer_len = self.buffer.len();

        for i in 0..output.len() {
            // Get current delay time in seconds
            let delay_time_sec = delay_time_buffer[i].max(0.0).min(self.max_delay);

            // Convert to samples
            let delay_samples = (delay_time_sec * self.sample_rate).round() as usize;

            // Calculate read position: (write_pos - delay_samples) % buffer_len
            // Using wrapping subtraction to handle underflow
            let read_pos = (self.write_pos + buffer_len - delay_samples) % buffer_len;

            // Read delayed sample
            output[i] = self.buffer[read_pos];

            // Write current input to buffer
            self.buffer[self.write_pos] = input_buffer[i];

            // Advance write position (circular)
            self.write_pos = (self.write_pos + 1) % buffer_len;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.delay_time_input]
    }

    fn name(&self) -> &str {
        "DelayNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_delay_zero_seconds_behavior() {
        // Test 1: Delay of 0.0 seconds
        // With 0 delay, read_pos = write_pos, so we read the value that was written
        // at this position in a previous cycle. For the first pass through the buffer,
        // we read zeros. After the buffer wraps, we get 1-sample delay.

        let sample_rate = 44100.0;
        let max_delay = 0.01; // Small buffer: 441 samples (10ms max delay)
        let block_size = 128;

        let mut input_node = ConstantNode::new(1.0);
        let mut delay_time_node = ConstantNode::new(0.0); // 0 second delay
        let mut delay = DelayNode::new(0, 1, max_delay, sample_rate);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        let buffer_size = delay.buffer_size(); // Should be 441

        // Generate input buffers
        let mut input_buf = vec![1.0; block_size];
        let mut delay_time_buf = vec![0.0; block_size];

        input_node.process_block(&[], &mut input_buf, sample_rate, &context);
        delay_time_node.process_block(&[], &mut delay_time_buf, sample_rate, &context);

        let inputs = vec![input_buf.as_slice(), delay_time_buf.as_slice()];

        // Process enough blocks to wrap the buffer
        let blocks_to_wrap = (buffer_size / block_size) + 2;

        for block_idx in 0..blocks_to_wrap {
            let mut output = vec![0.0; block_size];
            delay.process_block(&inputs, &mut output, sample_rate, &context);

            if block_idx == 0 {
                // First block: reading from empty buffer
                assert_eq!(output[0], 0.0, "First sample should be 0 (empty buffer)");
            } else if block_idx >= blocks_to_wrap - 1 {
                // After wrap: should have signal
                // With 0 delay, we read from same position we write to,
                // so we get the value from the previous wrap (1-sample delay effectively)
                for (i, &sample) in output.iter().enumerate() {
                    assert_eq!(
                        sample, 1.0,
                        "Block {}, sample {} should be 1.0 after buffer wrap",
                        block_idx, i
                    );
                }
            }
        }
    }

    #[test]
    fn test_delay_100ms_shift() {
        // Test 2: Delay of 0.1 seconds (100ms) should shift signal by 4410 samples @ 44.1kHz

        let sample_rate = 44100.0;
        let delay_time = 0.1; // 100ms
        let delay_samples = (delay_time * sample_rate) as usize; // 4410 samples
        let block_size = 512;

        let mut delay_time_node = ConstantNode::new(delay_time);
        let mut delay = DelayNode::new(0, 1, 1.0, sample_rate);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        // Create input signal: impulse at sample 0
        let mut input_buf = vec![0.0; block_size];
        input_buf[0] = 1.0; // Impulse

        let mut delay_time_buf = vec![0.0; block_size];
        delay_time_node.process_block(&[], &mut delay_time_buf, sample_rate, &context);

        // Process enough blocks to see the impulse come out
        let total_blocks = (delay_samples / block_size) + 2;
        let mut found_impulse_at = None;

        for block_idx in 0..total_blocks {
            let inputs = vec![input_buf.as_slice(), delay_time_buf.as_slice()];
            let mut output = vec![0.0; block_size];
            delay.process_block(&inputs, &mut output, sample_rate, &context);

            // After first block, input is silent
            input_buf.fill(0.0);

            // Look for the impulse
            for (i, &sample) in output.iter().enumerate() {
                if sample > 0.5 {
                    found_impulse_at = Some(block_idx * block_size + i);
                    break;
                }
            }

            if found_impulse_at.is_some() {
                break;
            }
        }

        assert!(
            found_impulse_at.is_some(),
            "Impulse never appeared in output"
        );

        let actual_delay = found_impulse_at.unwrap();

        // Should be delayed by approximately delay_samples (allowing ±1 sample for rounding)
        assert!(
            (actual_delay as i32 - delay_samples as i32).abs() <= 1,
            "Expected delay of {} samples, got {}",
            delay_samples,
            actual_delay
        );
    }

    #[test]
    fn test_delay_buffer_wrapping() {
        // Test 3: Verify circular buffer wraps correctly with long delays

        let sample_rate = 44100.0;
        let max_delay = 0.5; // 500ms max
        let delay_time = 0.25; // 250ms actual delay
        let block_size = 512;

        let mut delay_time_node = ConstantNode::new(delay_time);
        let mut delay = DelayNode::new(0, 1, max_delay, sample_rate);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        // Verify buffer was allocated correctly
        let expected_buffer_size = (max_delay * sample_rate).ceil() as usize;
        assert_eq!(
            delay.buffer_size(),
            expected_buffer_size,
            "Buffer size mismatch"
        );

        // Process many blocks to ensure wrap-around happens
        let blocks_to_process = expected_buffer_size / block_size + 10;

        let mut input_buf = vec![1.0; block_size];
        let mut delay_time_buf = vec![0.0; block_size];
        delay_time_node.process_block(&[], &mut delay_time_buf, sample_rate, &context);

        for _ in 0..blocks_to_process {
            let inputs = vec![input_buf.as_slice(), delay_time_buf.as_slice()];
            let mut output = vec![0.0; block_size];
            delay.process_block(&inputs, &mut output, sample_rate, &context);

            // Write position should always be within buffer bounds
            assert!(
                delay.write_position() < delay.buffer_size(),
                "Write position {} exceeds buffer size {}",
                delay.write_position(),
                delay.buffer_size()
            );
        }

        // After many blocks, output should stabilize to input (delayed)
        let inputs = vec![input_buf.as_slice(), delay_time_buf.as_slice()];
        let mut output = vec![0.0; block_size];
        delay.process_block(&inputs, &mut output, sample_rate, &context);

        // All samples should be approximately 1.0 (the steady input value)
        for (i, &sample) in output.iter().enumerate() {
            assert!(
                (sample - 1.0).abs() < 0.01,
                "Sample {} should be ~1.0, got {}",
                i,
                sample
            );
        }
    }

    #[test]
    fn test_delay_modulated_delay_time() {
        // Test 4: Verify delay works with time-varying delay time

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut delay = DelayNode::new(0, 1, 1.0, sample_rate);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        // Input: constant signal
        let input_buf = vec![1.0; block_size];

        // Delay time: varying from 0.0 to 0.1 seconds across the block
        let mut delay_time_buf = vec![0.0; block_size];
        for i in 0..block_size {
            delay_time_buf[i] = (i as f32 / block_size as f32) * 0.1;
        }

        // Fill the buffer with signal first
        let const_delay_buf = vec![0.05; block_size]; // 50ms constant delay
        for _ in 0..20 {
            let inputs = vec![input_buf.as_slice(), const_delay_buf.as_slice()];
            let mut output = vec![0.0; block_size];
            delay.process_block(&inputs, &mut output, sample_rate, &context);
        }

        // Now use varying delay time
        let inputs = vec![input_buf.as_slice(), delay_time_buf.as_slice()];
        let mut output = vec![0.0; block_size];
        delay.process_block(&inputs, &mut output, sample_rate, &context);

        // Output should contain valid samples (no NaN or extreme values)
        for (i, &sample) in output.iter().enumerate() {
            assert!(
                sample.is_finite(),
                "Sample {} is not finite: {}",
                i,
                sample
            );
            assert!(
                sample.abs() <= 2.0,
                "Sample {} has extreme value: {}",
                i,
                sample
            );
        }
    }

    #[test]
    fn test_delay_clear_buffer() {
        let sample_rate = 44100.0;
        let mut delay = DelayNode::new(0, 1, 1.0, sample_rate);

        // Fill buffer with non-zero values
        delay.buffer.fill(1.0);
        delay.write_pos = 42;

        // Clear buffer
        delay.clear_buffer();

        // Buffer should be all zeros
        assert!(delay.buffer.iter().all(|&x| x == 0.0));
        assert_eq!(delay.write_position(), 0);
    }

    #[test]
    fn test_delay_dependencies() {
        let delay = DelayNode::new(10, 20, 1.0, 44100.0);
        let deps = delay.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 10); // input
        assert_eq!(deps[1], 20); // delay_time_input
    }

    #[test]
    #[should_panic(expected = "max_delay must be greater than 0")]
    fn test_delay_invalid_max_delay() {
        let _ = DelayNode::new(0, 1, 0.0, 44100.0);
    }
}
