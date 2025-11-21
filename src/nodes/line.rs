/// Line Generator - Linear ramp generator
///
/// Generates a linear ramp from start to end value over a specified duration.
/// Triggered by a gate/trigger signal (rising edge detection).
///
/// Key features:
/// - Linear interpolation from start to end
/// - Trigger input restarts the ramp
/// - Holds at end value after completion
/// - All parameters pattern-controllable

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Line Generator Node
///
/// # Inputs
/// 1. Start value - Beginning of the ramp
/// 2. End value - Target of the ramp
/// 3. Duration in seconds - Time to complete ramp
/// 4. Trigger input (> 0.5 = trigger, rising edge detection)
///
/// # Example
/// ```ignore
/// // Create line that ramps from 0 to 1 over 2 seconds
/// let start = ConstantNode::new(0.0);      // NodeId 0
/// let end = ConstantNode::new(1.0);        // NodeId 1
/// let duration = ConstantNode::new(2.0);   // NodeId 2
/// let trigger = ConstantNode::new(1.0);    // NodeId 3 (trigger on)
/// let line = LineNode::new(0, 1, 2, 3);    // NodeId 4
/// ```
pub struct LineNode {
    start_input: NodeId,      // Starting value
    end_input: NodeId,        // Ending value
    duration_input: NodeId,   // Duration in seconds
    trigger_input: NodeId,    // Trigger to restart ramp
    current_value: f32,       // Current output value
    elapsed_time: f32,        // Time elapsed in current ramp (seconds)
    is_active: bool,          // Is ramp currently active?
    last_trigger: f32,        // Previous trigger value (for edge detection)
}

impl LineNode {
    /// Line - Linear ramp generator with trigger-based control
    ///
    /// Generates a linear ramp from start to end value over specified duration
    /// when triggered. Useful for envelopes, sweeps, and time-based modulation.
    ///
    /// # Parameters
    /// - `start_input`: Starting value
    /// - `end_input`: Ending value
    /// - `duration_input`: Ramp duration in seconds
    /// - `trigger_input`: Trigger signal (rising edge > 0.5 starts ramp)
    ///
    /// # Example
    /// ```phonon
    /// ~trig: "x ~ x ~" # bpm_to_impulse 120
    /// ~freq_ramp: ~trig # line 220 440 0.5
    /// out: sine ~freq_ramp * 0.5
    /// ```
    pub fn new(
        start_input: NodeId,
        end_input: NodeId,
        duration_input: NodeId,
        trigger_input: NodeId,
    ) -> Self {
        Self {
            start_input,
            end_input,
            duration_input,
            trigger_input,
            current_value: 0.0,
            elapsed_time: 0.0,
            is_active: false,
            last_trigger: 0.0,
        }
    }

    /// Get current line value
    pub fn value(&self) -> f32 {
        self.current_value
    }

    /// Check if ramp is currently active
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    /// Reset line to initial state
    pub fn reset(&mut self) {
        self.current_value = 0.0;
        self.elapsed_time = 0.0;
        self.is_active = false;
        self.last_trigger = 0.0;
    }
}

impl AudioNode for LineNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 4,
            "LineNode requires 4 inputs: start, end, duration, trigger"
        );

        let start_buffer = inputs[0];
        let end_buffer = inputs[1];
        let duration_buffer = inputs[2];
        let trigger_buffer = inputs[3];

        debug_assert_eq!(start_buffer.len(), output.len(), "Start buffer length mismatch");
        debug_assert_eq!(end_buffer.len(), output.len(), "End buffer length mismatch");
        debug_assert_eq!(duration_buffer.len(), output.len(), "Duration buffer length mismatch");
        debug_assert_eq!(trigger_buffer.len(), output.len(), "Trigger buffer length mismatch");

        for i in 0..output.len() {
            let start = start_buffer[i];
            let end = end_buffer[i];
            let duration = duration_buffer[i].max(0.001); // Minimum 1ms
            let trigger = trigger_buffer[i];

            // Detect trigger (rising edge: trigger > 0.5 and was previously <= 0.5)
            let trigger_rising = trigger > 0.5 && self.last_trigger <= 0.5;
            self.last_trigger = trigger;

            if trigger_rising {
                // Start new ramp
                self.current_value = start;
                self.elapsed_time = 0.0;
                self.is_active = true;
            }

            if self.is_active {
                // Linear interpolation: value = start + (end - start) * progress
                let progress = self.elapsed_time / duration;

                if progress >= 1.0 {
                    // Ramp complete: hold at end value
                    self.current_value = end;
                    self.is_active = false;
                } else {
                    // Ramp in progress
                    self.current_value = start + (end - start) * progress;
                }

                // Advance time by one sample period
                self.elapsed_time += 1.0 / sample_rate;
            }

            // Output current value
            output[i] = self.current_value;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![
            self.start_input,
            self.end_input,
            self.duration_input,
            self.trigger_input,
        ]
    }

    fn name(&self) -> &str {
        "LineNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_line_linear_ramp() {
        // Test 1: Verify linear ramp from start to end
        let sample_rate = 44100.0;
        let start_value = 0.0;
        let end_value = 1.0;
        let duration = 1.0; // 1 second = 44100 samples
        let block_size = 512;

        let mut start = ConstantNode::new(start_value);
        let mut end = ConstantNode::new(end_value);
        let mut duration_node = ConstantNode::new(duration);
        let mut trigger = ConstantNode::new(1.0); // Trigger on

        let mut line = LineNode::new(0, 1, 2, 3);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        // Generate input buffers
        let mut start_buf = vec![0.0; block_size];
        let mut end_buf = vec![0.0; block_size];
        let mut duration_buf = vec![0.0; block_size];
        let mut trigger_buf = vec![0.0; block_size];

        start.process_block(&[], &mut start_buf, sample_rate, &context);
        end.process_block(&[], &mut end_buf, sample_rate, &context);
        duration_node.process_block(&[], &mut duration_buf, sample_rate, &context);
        trigger.process_block(&[], &mut trigger_buf, sample_rate, &context);

        let inputs = vec![
            start_buf.as_slice(),
            end_buf.as_slice(),
            duration_buf.as_slice(),
            trigger_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        line.process_block(&inputs, &mut output, sample_rate, &context);

        // First sample should be at start value (triggered)
        assert_eq!(output[0], start_value, "First sample should be at start value");

        // Values should be increasing linearly
        assert!(output[100] > output[0], "Line should be rising");
        assert!(output[200] > output[100], "Line should continue rising");
        assert!(output[300] > output[200], "Line should continue rising");

        // Verify linear progression
        let expected_increment = (end_value - start_value) / (duration * sample_rate);
        let actual_increment = output[1] - output[0];
        assert!(
            (actual_increment - expected_increment).abs() < 0.0001,
            "Increment should be linear, expected {}, got {}",
            expected_increment,
            actual_increment
        );
    }

    #[test]
    fn test_line_reaches_end_value() {
        // Test 2: Verify ramp reaches and holds at end value
        let sample_rate = 44100.0;
        let start_value = 0.0;
        let end_value = 1.0;
        let duration = 0.01; // 10ms = 441 samples
        let block_size = 512;

        let mut line = LineNode::new(0, 1, 2, 3);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        let start_buf = vec![start_value; block_size];
        let end_buf = vec![end_value; block_size];
        let duration_buf = vec![duration; block_size];
        let trigger_buf = vec![1.0; block_size]; // Trigger on

        let inputs = vec![
            start_buf.as_slice(),
            end_buf.as_slice(),
            duration_buf.as_slice(),
            trigger_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        line.process_block(&inputs, &mut output, sample_rate, &context);

        // Calculate expected completion sample
        let completion_sample = (duration * sample_rate) as usize;

        // After completion, should be at end value
        if completion_sample < block_size {
            assert!(
                (output[completion_sample] - end_value).abs() < 0.01,
                "Should reach end value at sample {}, got {}",
                completion_sample,
                output[completion_sample]
            );

            // Should hold at end value
            assert_eq!(
                output[block_size - 1], end_value,
                "Should hold at end value"
            );
        }
    }

    #[test]
    fn test_line_duration_control() {
        // Test 3: Different durations produce different ramp speeds
        let sample_rate = 44100.0;
        let start_value = 0.0;
        let end_value = 1.0;
        let short_duration = 0.01; // 10ms
        let long_duration = 0.1;   // 100ms
        let block_size = 512;

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        // Fast ramp
        let mut line_fast = LineNode::new(0, 1, 2, 3);
        let start_buf = vec![start_value; block_size];
        let end_buf = vec![end_value; block_size];
        let duration_buf = vec![short_duration; block_size];
        let trigger_buf = vec![1.0; block_size];

        let inputs = vec![
            start_buf.as_slice(),
            end_buf.as_slice(),
            duration_buf.as_slice(),
            trigger_buf.as_slice(),
        ];

        let mut output_fast = vec![0.0; block_size];
        line_fast.process_block(&inputs, &mut output_fast, sample_rate, &context);

        // Slow ramp
        let mut line_slow = LineNode::new(0, 1, 2, 3);
        let duration_buf_slow = vec![long_duration; block_size];
        let inputs_slow = vec![
            start_buf.as_slice(),
            end_buf.as_slice(),
            duration_buf_slow.as_slice(),
            trigger_buf.as_slice(),
        ];

        let mut output_slow = vec![0.0; block_size];
        line_slow.process_block(&inputs_slow, &mut output_slow, sample_rate, &context);

        // At sample 100, fast ramp should be further along than slow ramp
        assert!(
            output_fast[100] > output_slow[100],
            "Fast ramp should progress faster than slow ramp, fast={}, slow={}",
            output_fast[100],
            output_slow[100]
        );
    }

    #[test]
    fn test_line_retrigger() {
        // Test 4: Retriggering should restart the ramp
        let sample_rate = 44100.0;
        let start_value = 0.0;
        let end_value = 1.0;
        let duration = 0.1; // 100ms
        let block_size = 512;

        let mut line = LineNode::new(0, 1, 2, 3);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        let start_buf = vec![start_value; block_size];
        let end_buf = vec![end_value; block_size];
        let duration_buf = vec![duration; block_size];

        // First trigger
        let trigger_buf = vec![1.0; block_size];
        let inputs = vec![
            start_buf.as_slice(),
            end_buf.as_slice(),
            duration_buf.as_slice(),
            trigger_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        line.process_block(&inputs, &mut output, sample_rate, &context);

        // Should be active and ramping
        assert!(line.is_active(), "Line should be active after trigger");
        assert!(output[block_size - 1] > start_value, "Line should have progressed");

        // Trigger off (low)
        let trigger_buf = vec![0.0; block_size];
        let inputs = vec![
            start_buf.as_slice(),
            end_buf.as_slice(),
            duration_buf.as_slice(),
            trigger_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        line.process_block(&inputs, &mut output, sample_rate, &context);

        // Should continue ramping (not restart)
        let value_before_retrigger = output[block_size - 1];

        // Retrigger (rising edge)
        let trigger_buf = vec![1.0; block_size];
        let inputs = vec![
            start_buf.as_slice(),
            end_buf.as_slice(),
            duration_buf.as_slice(),
            trigger_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        line.process_block(&inputs, &mut output, sample_rate, &context);

        // First sample should be at start (restarted)
        assert_eq!(
            output[0], start_value,
            "Retrigger should restart at start value"
        );
        assert!(
            output[0] < value_before_retrigger,
            "Retrigger should reset to lower value, was {}, now {}",
            value_before_retrigger,
            output[0]
        );
    }

    #[test]
    fn test_line_holds_at_end() {
        // Test 5: Line should hold at end value after completion
        let sample_rate = 44100.0;
        let start_value = 0.0;
        let end_value = 1.0;
        let duration = 0.005; // 5ms = 220 samples (completes in first block)
        let block_size = 512;

        let mut line = LineNode::new(0, 1, 2, 3);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        let start_buf = vec![start_value; block_size];
        let end_buf = vec![end_value; block_size];
        let duration_buf = vec![duration; block_size];
        let trigger_buf = vec![1.0; block_size]; // Trigger on first sample only

        let inputs = vec![
            start_buf.as_slice(),
            end_buf.as_slice(),
            duration_buf.as_slice(),
            trigger_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        line.process_block(&inputs, &mut output, sample_rate, &context);

        // After completion, should hold at end value
        let completion_sample = (duration * sample_rate) as usize + 10; // Add margin
        if completion_sample < block_size {
            assert_eq!(
                output[completion_sample], end_value,
                "Should hold at end value after completion"
            );
            assert_eq!(
                output[block_size - 1], end_value,
                "Should still hold at end value"
            );
            assert!(!line.is_active(), "Line should be inactive after completion");
        }
    }

    #[test]
    fn test_line_dependencies() {
        // Test 6: Verify input_nodes returns correct dependencies
        let line = LineNode::new(10, 20, 30, 40);
        let deps = line.input_nodes();

        assert_eq!(deps.len(), 4);
        assert_eq!(deps[0], 10); // start_input
        assert_eq!(deps[1], 20); // end_input
        assert_eq!(deps[2], 30); // duration_input
        assert_eq!(deps[3], 40); // trigger_input
    }

    #[test]
    fn test_line_with_constants() {
        // Test 7: Full ramp with constant parameters
        let sample_rate = 44100.0;
        let start_value = 100.0;
        let end_value = 500.0;
        let duration = 0.02; // 20ms = 882 samples
        let block_size = 512;

        let mut start_node = ConstantNode::new(start_value);
        let mut end_node = ConstantNode::new(end_value);
        let mut duration_node = ConstantNode::new(duration);
        let mut trigger_node = ConstantNode::new(1.0);

        let mut line = LineNode::new(0, 1, 2, 3);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        // Generate constant buffers
        let mut start_buf = vec![0.0; block_size];
        let mut end_buf = vec![0.0; block_size];
        let mut duration_buf = vec![0.0; block_size];
        let mut trigger_buf = vec![0.0; block_size];

        start_node.process_block(&[], &mut start_buf, sample_rate, &context);
        end_node.process_block(&[], &mut end_buf, sample_rate, &context);
        duration_node.process_block(&[], &mut duration_buf, sample_rate, &context);
        trigger_node.process_block(&[], &mut trigger_buf, sample_rate, &context);

        let inputs = vec![
            start_buf.as_slice(),
            end_buf.as_slice(),
            duration_buf.as_slice(),
            trigger_buf.as_slice(),
        ];

        // Process first block
        let mut output = vec![0.0; block_size];
        line.process_block(&inputs, &mut output, sample_rate, &context);

        // Should start at start_value
        assert_eq!(output[0], start_value, "Should start at start value");

        // Should be ramping towards end_value
        assert!(
            output[100] > start_value && output[100] < end_value,
            "Should be between start and end, got {}",
            output[100]
        );

        // Process second block to complete ramp
        trigger_buf.fill(0.0); // No more triggers
        let inputs = vec![
            start_buf.as_slice(),
            end_buf.as_slice(),
            duration_buf.as_slice(),
            trigger_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        line.process_block(&inputs, &mut output, sample_rate, &context);

        // Should reach and hold at end_value
        assert_eq!(
            output[block_size - 1], end_value,
            "Should hold at end value in second block"
        );
        assert!(!line.is_active(), "Should be inactive after completion");
    }

    #[test]
    fn test_line_negative_ramp() {
        // Test 8: Verify ramp works with descending values
        let sample_rate = 44100.0;
        let start_value = 1.0;
        let end_value = 0.0;
        let duration = 0.01; // 10ms
        let block_size = 512;

        let mut line = LineNode::new(0, 1, 2, 3);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        let start_buf = vec![start_value; block_size];
        let end_buf = vec![end_value; block_size];
        let duration_buf = vec![duration; block_size];
        let trigger_buf = vec![1.0; block_size];

        let inputs = vec![
            start_buf.as_slice(),
            end_buf.as_slice(),
            duration_buf.as_slice(),
            trigger_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        line.process_block(&inputs, &mut output, sample_rate, &context);

        // Should start at 1.0
        assert_eq!(output[0], start_value, "Should start at start value");

        // Should be descending
        assert!(output[100] < output[0], "Line should be descending");
        assert!(output[200] < output[100], "Line should continue descending");

        // Should reach 0.0
        let completion_sample = (duration * sample_rate) as usize;
        if completion_sample < block_size {
            assert!(
                (output[completion_sample] - end_value).abs() < 0.01,
                "Should reach end value (0.0), got {}",
                output[completion_sample]
            );
        }
    }
}
