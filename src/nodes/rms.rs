/// RMS (Root Mean Square) node - calculates RMS level with windowing
///
/// This node computes the RMS (Root Mean Square) value of an input signal
/// over a sliding window. RMS is a measure of signal power/energy and is
/// commonly used for:
/// - Level metering
/// - Envelope following
/// - Compression/limiting (RMS-based dynamics)
/// - Audio analysis

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// RMS calculation node with pattern-controlled window size
///
/// # Example
/// ```ignore
/// // Calculate RMS of a signal with 100ms window
/// let input_signal = OscillatorNode::new(0, Waveform::Sine);  // NodeId 0
/// let window_time = ConstantNode::new(0.1);  // 0.1 seconds = 100ms, NodeId 1
/// let rms = RMSNode::new(0, 1, 44100.0);  // NodeId 2
/// ```
///
/// # Algorithm
/// Uses a circular buffer to maintain a sliding window of samples.
/// For each sample:
/// 1. Remove the oldest sample from the sum of squares
/// 2. Add the new sample to the sum of squares
/// 3. Calculate RMS = sqrt(sum_of_squares / window_length)
///
/// This approach is O(1) per sample (constant time) regardless of window size.
pub struct RMSNode {
    input: NodeId,                // Signal to analyze
    window_time_input: NodeId,    // RMS window time in seconds (can be modulated)
    buffer: Vec<f32>,             // Circular buffer for windowing
    write_pos: usize,             // Write position in circular buffer
    sum_of_squares: f32,          // Running sum of squares for efficiency
    sample_rate: f32,             // Sample rate for calculations
}

impl RMSNode {
    /// RMSNode - Root mean square level meter with sliding window
    ///
    /// Computes RMS (Root Mean Square) value of input signal over a sliding window,
    /// used for level metering, envelope following, and RMS-based dynamics control.
    ///
    /// # Parameters
    /// - `input`: NodeId providing the signal to analyze
    /// - `window_time_input`: NodeId providing window time in seconds
    /// - `sample_rate`: Sample rate in Hz (usually 44100.0)
    ///
    /// # Example
    /// ```phonon
    /// ~signal: sine 440
    /// ~level: ~signal # rms 0.1
    /// ```
    pub fn new(input: NodeId, window_time_input: NodeId, sample_rate: f32) -> Self {
        // Start with a small default buffer (10ms @ 44.1kHz)
        let initial_buffer_size = (0.01 * sample_rate).ceil() as usize;

        Self {
            input,
            window_time_input,
            buffer: vec![0.0; initial_buffer_size],
            write_pos: 0,
            sum_of_squares: 0.0,
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

    /// Reset the RMS calculation state
    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_pos = 0;
        self.sum_of_squares = 0.0;
    }
}

impl AudioNode for RMSNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "RMSNode requires 2 inputs: signal and window_time"
        );

        let input_buffer = inputs[0];
        let window_time_buffer = inputs[1];

        debug_assert_eq!(
            input_buffer.len(),
            output.len(),
            "Input buffer length mismatch"
        );
        debug_assert_eq!(
            window_time_buffer.len(),
            output.len(),
            "Window time buffer length mismatch"
        );

        for i in 0..output.len() {
            let sample = input_buffer[i];

            // Get window time in seconds (minimum 0.001s = 1ms to avoid division by zero)
            let window_time = window_time_buffer[i].max(0.001);
            let window_samples = (window_time * self.sample_rate).ceil() as usize;
            let window_samples = window_samples.max(1); // At least 1 sample

            // Resize buffer if window size changed
            if self.buffer.len() != window_samples {
                // Reset state when buffer size changes
                self.buffer.resize(window_samples, 0.0);
                self.sum_of_squares = 0.0;
                self.write_pos = 0;
            }

            // Remove old sample from sum of squares
            let old_sample = self.buffer[self.write_pos];
            self.sum_of_squares -= old_sample * old_sample;

            // Add new sample to buffer and sum of squares
            self.buffer[self.write_pos] = sample;
            self.sum_of_squares += sample * sample;

            // Advance write position (circular)
            self.write_pos = (self.write_pos + 1) % self.buffer.len();

            // Calculate RMS: sqrt(mean of squares)
            // Clamp sum_of_squares to avoid numerical issues
            let mean_of_squares = (self.sum_of_squares / self.buffer.len() as f32).max(0.0);
            output[i] = mean_of_squares.sqrt();
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.window_time_input]
    }

    fn name(&self) -> &str {
        "RMSNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;
    use std::f32::consts::PI;

    fn create_test_context(block_size: usize) -> ProcessContext {
        ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            44100.0,
        )
    }

    #[test]
    fn test_rms_dc_signal() {
        // Test 1: RMS of a constant DC signal should equal the absolute value
        // RMS of DC = sqrt(mean(x^2)) = sqrt(x^2) = |x|

        let sample_rate = 44100.0;
        let block_size = 512;
        let dc_value = 0.5;

        let mut input_node = ConstantNode::new(dc_value);
        let mut window_time_node = ConstantNode::new(0.1); // 100ms window
        let mut rms = RMSNode::new(0, 1, sample_rate);

        let context = create_test_context(block_size);

        let mut input_buf = vec![0.0; block_size];
        let mut window_time_buf = vec![0.0; block_size];

        input_node.process_block(&[], &mut input_buf, sample_rate, &context);
        window_time_node.process_block(&[], &mut window_time_buf, sample_rate, &context);

        let inputs = vec![input_buf.as_slice(), window_time_buf.as_slice()];
        let mut output = vec![0.0; block_size];

        // Process several blocks to fill the RMS window
        for _ in 0..20 {
            rms.process_block(&inputs, &mut output, sample_rate, &context);
        }

        // After convergence, RMS should equal abs(dc_value)
        let last_sample = output[block_size - 1];
        assert!(
            (last_sample - dc_value.abs()).abs() < 0.01,
            "RMS of DC signal {} should be {}, got {}",
            dc_value,
            dc_value.abs(),
            last_sample
        );
    }

    #[test]
    fn test_rms_sine_wave() {
        // Test 2: RMS of a sine wave should be peak / sqrt(2) ≈ peak * 0.707
        // For sine wave: RMS = peak / √2

        let sample_rate = 44100.0;
        let block_size = 512;
        let frequency = 100.0; // 100 Hz
        let amplitude = 1.0;

        let mut window_time_node = ConstantNode::new(0.1); // 100ms window
        let mut rms = RMSNode::new(0, 1, sample_rate);

        let context = create_test_context(block_size);

        // Generate sine wave input
        let mut input_buf = vec![0.0; block_size];
        for i in 0..block_size {
            let t = i as f32 / sample_rate;
            input_buf[i] = amplitude * (2.0 * PI * frequency * t).sin();
        }

        let mut window_time_buf = vec![0.0; block_size];
        window_time_node.process_block(&[], &mut window_time_buf, sample_rate, &context);

        let inputs = vec![input_buf.as_slice(), window_time_buf.as_slice()];
        let mut output = vec![0.0; block_size];

        // Process many blocks to allow RMS to converge
        for block_idx in 0..50 {
            // Update sine wave for each block
            for i in 0..block_size {
                let t = (block_idx * block_size + i) as f32 / sample_rate;
                input_buf[i] = amplitude * (2.0 * PI * frequency * t).sin();
            }

            let inputs = vec![input_buf.as_slice(), window_time_buf.as_slice()];
            rms.process_block(&inputs, &mut output, sample_rate, &context);
        }

        // RMS of sine wave = peak / sqrt(2)
        let expected_rms = amplitude / 2.0_f32.sqrt();
        let last_sample = output[block_size - 1];

        assert!(
            (last_sample - expected_rms).abs() < 0.05,
            "RMS of sine wave (amplitude {}) should be {:.3}, got {:.3}",
            amplitude,
            expected_rms,
            last_sample
        );
    }

    #[test]
    fn test_rms_square_wave() {
        // Test 3: RMS of a square wave should equal the peak value
        // For square wave alternating between +A and -A: RMS = A

        let sample_rate = 44100.0;
        let block_size = 512;
        let amplitude = 0.8;

        let mut window_time_node = ConstantNode::new(0.1); // 100ms window
        let mut rms = RMSNode::new(0, 1, sample_rate);

        let context = create_test_context(block_size);

        // Generate square wave input (50 samples per half-cycle)
        let mut input_buf = vec![0.0; block_size];
        let half_period = 50;

        let mut window_time_buf = vec![0.0; block_size];
        window_time_node.process_block(&[], &mut window_time_buf, sample_rate, &context);

        // Process many blocks
        for block_idx in 0..50 {
            for i in 0..block_size {
                let sample_idx = block_idx * block_size + i;
                input_buf[i] = if (sample_idx / half_period) % 2 == 0 {
                    amplitude
                } else {
                    -amplitude
                };
            }

            let inputs = vec![input_buf.as_slice(), window_time_buf.as_slice()];
            let mut output = vec![0.0; block_size];
            rms.process_block(&inputs, &mut output, sample_rate, &context);

            // After convergence
            if block_idx > 20 {
                let last_sample = output[block_size - 1];
                assert!(
                    (last_sample - amplitude).abs() < 0.05,
                    "Block {}: RMS of square wave (amplitude {}) should be {}, got {}",
                    block_idx,
                    amplitude,
                    amplitude,
                    last_sample
                );
            }
        }
    }

    #[test]
    fn test_rms_window_size_affects_smoothing() {
        // Test 4: Smaller window = faster response, larger window = more smoothing

        let sample_rate = 44100.0;
        let block_size = 512;

        // Create two RMS nodes with different window sizes
        let mut rms_short = RMSNode::new(0, 1, sample_rate);
        let mut rms_long = RMSNode::new(0, 1, sample_rate);

        let mut short_window_node = ConstantNode::new(0.01); // 10ms
        let mut long_window_node = ConstantNode::new(0.2);   // 200ms

        let context = create_test_context(block_size);

        // Generate impulse (single spike)
        let mut input_buf = vec![0.0; block_size];
        input_buf[0] = 1.0; // Impulse at start

        let mut short_window_buf = vec![0.0; block_size];
        let mut long_window_buf = vec![0.0; block_size];

        short_window_node.process_block(&[], &mut short_window_buf, sample_rate, &context);
        long_window_node.process_block(&[], &mut long_window_buf, sample_rate, &context);

        let inputs_short = vec![input_buf.as_slice(), short_window_buf.as_slice()];
        let inputs_long = vec![input_buf.as_slice(), long_window_buf.as_slice()];

        let mut output_short = vec![0.0; block_size];
        let mut output_long = vec![0.0; block_size];

        rms_short.process_block(&inputs_short, &mut output_short, sample_rate, &context);
        rms_long.process_block(&inputs_long, &mut output_long, sample_rate, &context);

        // Short window should have higher peak (less smoothing)
        let peak_short = output_short.iter().cloned().fold(0.0_f32, f32::max);
        let peak_long = output_long.iter().cloned().fold(0.0_f32, f32::max);

        assert!(
            peak_short > peak_long,
            "Short window ({}) should have higher peak than long window ({})",
            peak_short,
            peak_long
        );

        // Verify the difference is significant
        assert!(
            peak_short > peak_long * 2.0,
            "Short window peak ({}) should be significantly higher than long window ({})",
            peak_short,
            peak_long
        );
    }

    #[test]
    fn test_rms_zero_signal() {
        // Test 5: RMS of zero signal should be zero

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut input_node = ConstantNode::new(0.0);
        let mut window_time_node = ConstantNode::new(0.1);
        let mut rms = RMSNode::new(0, 1, sample_rate);

        let context = create_test_context(block_size);

        let mut input_buf = vec![0.0; block_size];
        let mut window_time_buf = vec![0.0; block_size];

        input_node.process_block(&[], &mut input_buf, sample_rate, &context);
        window_time_node.process_block(&[], &mut window_time_buf, sample_rate, &context);

        let inputs = vec![input_buf.as_slice(), window_time_buf.as_slice()];
        let mut output = vec![0.0; block_size];

        rms.process_block(&inputs, &mut output, sample_rate, &context);

        // All samples should be zero
        for (i, &sample) in output.iter().enumerate() {
            assert_eq!(
                sample, 0.0,
                "Sample {} should be 0.0 for zero input",
                i
            );
        }
    }

    #[test]
    fn test_rms_dependencies() {
        // Test 6: Verify node has correct dependencies

        let rms = RMSNode::new(10, 20, 44100.0);
        let deps = rms.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 10); // input signal
        assert_eq!(deps[1], 20); // window_time_input
    }

    #[test]
    fn test_rms_with_constants() {
        // Test 7: Test with various constant inputs to verify algorithm

        let sample_rate = 44100.0;
        let block_size = 512;

        let test_values = vec![0.0, 0.5, 1.0, -0.5, -1.0];

        for &test_value in &test_values {
            let mut input_node = ConstantNode::new(test_value);
            let mut window_time_node = ConstantNode::new(0.05); // 50ms
            let mut rms = RMSNode::new(0, 1, sample_rate);

            let context = create_test_context(block_size);

            let mut input_buf = vec![0.0; block_size];
            let mut window_time_buf = vec![0.0; block_size];

            input_node.process_block(&[], &mut input_buf, sample_rate, &context);
            window_time_node.process_block(&[], &mut window_time_buf, sample_rate, &context);

            let inputs = vec![input_buf.as_slice(), window_time_buf.as_slice()];
            let mut output = vec![0.0; block_size];

            // Process enough blocks to converge
            for _ in 0..10 {
                rms.process_block(&inputs, &mut output, sample_rate, &context);
            }

            let last_sample = output[block_size - 1];
            let expected = test_value.abs();

            assert!(
                (last_sample - expected).abs() < 0.01,
                "RMS of constant {} should be {}, got {}",
                test_value,
                expected,
                last_sample
            );
        }
    }

    #[test]
    fn test_rms_reset() {
        // Test 8: Verify reset clears state

        let sample_rate = 44100.0;
        let mut rms = RMSNode::new(0, 1, sample_rate);

        // Fill buffer with non-zero values
        rms.buffer.fill(1.0);
        rms.write_pos = 42;
        rms.sum_of_squares = 123.456;

        // Reset
        rms.reset();

        // Verify state is cleared
        assert!(rms.buffer.iter().all(|&x| x == 0.0));
        assert_eq!(rms.write_position(), 0);
        assert_eq!(rms.sum_of_squares, 0.0);
    }

    #[test]
    fn test_rms_buffer_resizing() {
        // Test 9: Verify buffer resizes correctly when window time changes

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut rms = RMSNode::new(0, 1, sample_rate);

        let context = create_test_context(block_size);

        // Start with 50ms window
        let mut input_buf = vec![0.5; block_size];
        let mut window_time_buf = vec![0.05; block_size];

        let inputs = vec![input_buf.as_slice(), window_time_buf.as_slice()];
        let mut output = vec![0.0; block_size];

        rms.process_block(&inputs, &mut output, sample_rate, &context);

        let initial_buffer_size = rms.buffer_size();
        let expected_size = (0.05 * sample_rate).ceil() as usize;
        assert_eq!(initial_buffer_size, expected_size);

        // Change to 100ms window
        window_time_buf.fill(0.1);
        let inputs = vec![input_buf.as_slice(), window_time_buf.as_slice()];

        rms.process_block(&inputs, &mut output, sample_rate, &context);

        let new_buffer_size = rms.buffer_size();
        let expected_new_size = (0.1 * sample_rate).ceil() as usize;
        assert_eq!(new_buffer_size, expected_new_size);

        // Verify buffer size doubled
        assert_eq!(new_buffer_size, initial_buffer_size * 2);
    }
}
