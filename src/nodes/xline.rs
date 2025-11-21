/// Exponential line generator - exponential ramp from start to end value
///
/// XLine generates an exponential curve from a starting value to an ending value
/// over a specified duration. This is more natural-sounding than linear ramps for:
/// - Pitch sweeps (frequency transitions)
/// - Amplitude envelopes (volume fades)
/// - Filter cutoff modulation
/// - Any musical parameter that operates on a logarithmic scale
///
/// The exponential curve formula: `start * (end/start)^progress`
/// - Creates smooth, musical transitions
/// - Matches how humans perceive pitch and loudness
/// - More natural than linear interpolation for audio parameters
///
/// # Important Notes
/// - Both start and end values MUST be > 0 (exponential math requires positive values)
/// - The node automatically protects against log(0) by clamping to minimum 0.0001
/// - Trigger input (> 0.5) restarts the ramp from the beginning
/// - Once ramp completes, output holds at end value until retriggered

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Exponential line generator node
///
/// # Algorithm
/// ```text
/// for each sample:
///     start = abs(start_input[i]).max(0.0001)  // Ensure > 0
///     end = abs(end_input[i]).max(0.0001)
///     duration = duration_input[i].max(0.001)  // Minimum 1ms
///     trigger = trigger_input[i]
///
///     // Detect trigger (rising edge)
///     if trigger > 0.5 && last_trigger <= 0.5:
///         current_value = start
///         elapsed_time = 0.0
///         is_active = true
///
///     last_trigger = trigger
///
///     if is_active:
///         progress = elapsed_time / duration
///         if progress >= 1.0:
///             current_value = end
///             is_active = false
///         else:
///             // Exponential interpolation
///             current_value = start * (end / start).powf(progress)
///         elapsed_time += 1.0 / sample_rate
///
///     output[i] = current_value
/// ```
///
/// # Inputs
/// 1. Start value (must be > 0)
/// 2. End value (must be > 0)
/// 3. Duration in seconds
/// 4. Trigger input (> 0.5 = trigger, <= 0.5 = no trigger)
///
/// # Example
/// ```ignore
/// // Exponential pitch sweep from 440 Hz to 880 Hz over 1 second
/// let start = ConstantNode::new(440.0);      // NodeId 0
/// let end = ConstantNode::new(880.0);        // NodeId 1
/// let duration = ConstantNode::new(1.0);     // NodeId 2 (1 second)
/// let trigger = ConstantNode::new(1.0);      // NodeId 3 (trigger on)
/// let xline = XLineNode::new(0, 1, 2, 3);    // NodeId 4
/// ```
pub struct XLineNode {
    /// Starting value input (must be > 0)
    start_input: NodeId,

    /// Ending value input (must be > 0)
    end_input: NodeId,

    /// Duration in seconds input
    duration_input: NodeId,

    /// Trigger input (> 0.5 = restart ramp)
    trigger_input: NodeId,

    /// Current output value
    current_value: f32,

    /// Time elapsed since trigger (in seconds)
    elapsed_time: f32,

    /// Whether ramp is currently active
    is_active: bool,

    /// Previous trigger value (for edge detection)
    last_trigger: f32,
}

impl XLineNode {
    /// XLine - Exponential ramp generator for natural-sounding transitions
    ///
    /// Generates exponential curve from start to end value over duration.
    /// More natural than linear for pitch/frequency/amplitude transitions.
    ///
    /// # Parameters
    /// - `start_input`: Starting value (must be > 0)
    /// - `end_input`: Ending value (must be > 0)
    /// - `duration_input`: Duration in seconds
    /// - `trigger_input`: Trigger signal (rising edge restarts)
    ///
    /// # Example
    /// ```phonon
    /// ~trigger: lfo 1.0 0 1
    /// ~freq: xline 440 880 1.0 ~trigger
    /// out: sine ~freq
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

    /// Get current value (for debugging/testing)
    pub fn value(&self) -> f32 {
        self.current_value
    }

    /// Check if ramp is currently active
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    /// Get elapsed time since trigger
    pub fn elapsed_time(&self) -> f32 {
        self.elapsed_time
    }

    /// Reset to initial state
    pub fn reset(&mut self) {
        self.current_value = 0.0;
        self.elapsed_time = 0.0;
        self.is_active = false;
        self.last_trigger = 0.0;
    }
}

impl AudioNode for XLineNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 4,
            "XLineNode requires 4 inputs: start, end, duration, trigger"
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
            // Read inputs with safety protections
            let start = start_buffer[i].abs().max(0.0001); // Prevent log(0)
            let end = end_buffer[i].abs().max(0.0001);
            let duration = duration_buffer[i].max(0.001); // Minimum 1ms
            let trigger = trigger_buffer[i];

            // Detect trigger (rising edge)
            if trigger > 0.5 && self.last_trigger <= 0.5 {
                self.current_value = start;
                self.elapsed_time = 0.0;
                self.is_active = true;
            }
            self.last_trigger = trigger;

            // Process exponential ramp if active
            if self.is_active {
                let progress = self.elapsed_time / duration;
                if progress >= 1.0 {
                    // Ramp complete - hold at end value
                    self.current_value = end;
                    self.is_active = false;
                } else {
                    // Exponential interpolation: start * (end/start)^progress
                    self.current_value = start * (end / start).powf(progress);
                }
                self.elapsed_time += 1.0 / sample_rate;
            }

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
        "XLineNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    fn create_context(block_size: usize) -> ProcessContext {
        ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, 44100.0)
    }

    #[test]
    fn test_xline_exponential_ramp() {
        // Test 1: Verify exponential curve shape (not linear)
        let mut xline = XLineNode::new(0, 1, 2, 3);

        // Exponential sweep from 100 to 1000 over 100 samples
        let start = vec![100.0; 100];
        let end = vec![1000.0; 100];
        let duration = vec![100.0 / 44100.0; 100]; // Duration in seconds
        let trigger = vec![1.0; 100]; // Trigger on first sample

        let inputs = vec![
            start.as_slice(),
            end.as_slice(),
            duration.as_slice(),
            trigger.as_slice(),
        ];

        let mut output = vec![0.0; 100];
        let context = create_context(100);

        xline.process_block(&inputs, &mut output, 44100.0, &context);

        // Verify exponential shape by checking midpoint
        // Linear: mid = (start + end) / 2 = 550
        // Exponential: mid = start * sqrt(end/start) = 100 * sqrt(10) ≈ 316.23
        let mid_sample = 50;
        let mid_value = output[mid_sample];

        // Exponential should be closer to start value at midpoint
        assert!(mid_value < 550.0, "Exponential curve should be < 550 at midpoint, got {}", mid_value);
        assert!(mid_value > 100.0, "Should be rising, got {}", mid_value);

        // Verify it's exponential not linear by checking ratio
        // For exponential: ratio between adjacent samples should be constant
        // For linear: difference between adjacent samples should be constant
        let ratio_early = output[25] / output[24];
        let ratio_late = output[75] / output[74];

        // Ratios should be approximately equal (within tolerance)
        let ratio_diff = (ratio_early - ratio_late).abs();
        assert!(ratio_diff < 0.01, "Exponential should have constant ratio, got diff = {}", ratio_diff);
    }

    #[test]
    fn test_xline_reaches_end_value() {
        // Test 2: Verify ramp reaches exact end value
        let mut xline = XLineNode::new(0, 1, 2, 3);

        let sample_rate = 44100.0;
        let duration_seconds = 0.01; // 10ms = 441 samples
        let block_size = 512;

        let start = vec![10.0; block_size];
        let end = vec![1000.0; block_size];
        let duration = vec![duration_seconds; block_size];
        let trigger = vec![1.0; block_size];

        let inputs = vec![
            start.as_slice(),
            end.as_slice(),
            duration.as_slice(),
            trigger.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        let context = create_context(block_size);

        xline.process_block(&inputs, &mut output, sample_rate, &context);

        // After duration, should reach end value
        let expected_samples = (duration_seconds * sample_rate) as usize;
        if expected_samples < block_size {
            let final_value = output[expected_samples];
            assert!(
                (final_value - 1000.0).abs() < 0.1,
                "Should reach end value 1000.0, got {}",
                final_value
            );

            // Should hold at end value after ramp completes
            assert_eq!(xline.is_active(), false, "Should be inactive after completion");
            let held_value = output[expected_samples + 10];
            assert!(
                (held_value - 1000.0).abs() < 0.1,
                "Should hold at end value, got {}",
                held_value
            );
        }
    }

    #[test]
    fn test_xline_duration_control() {
        // Test 3: Verify different durations produce different ramp speeds
        let mut xline_fast = XLineNode::new(0, 1, 2, 3);
        let mut xline_slow = XLineNode::new(0, 1, 2, 3);

        let block_size = 50;
        let sample_rate = 44100.0;

        // Fast ramp: 10ms
        let start_fast = vec![100.0; block_size];
        let end_fast = vec![1000.0; block_size];
        let duration_fast = vec![0.01; block_size]; // 10ms
        let trigger_fast = vec![1.0; block_size];

        let inputs_fast = vec![
            start_fast.as_slice(),
            end_fast.as_slice(),
            duration_fast.as_slice(),
            trigger_fast.as_slice(),
        ];

        let mut output_fast = vec![0.0; block_size];
        let context = create_context(block_size);

        xline_fast.process_block(&inputs_fast, &mut output_fast, sample_rate, &context);

        // Slow ramp: 100ms
        let start_slow = vec![100.0; block_size];
        let end_slow = vec![1000.0; block_size];
        let duration_slow = vec![0.1; block_size]; // 100ms
        let trigger_slow = vec![1.0; block_size];

        let inputs_slow = vec![
            start_slow.as_slice(),
            end_slow.as_slice(),
            duration_slow.as_slice(),
            trigger_slow.as_slice(),
        ];

        let mut output_slow = vec![0.0; block_size];
        xline_slow.process_block(&inputs_slow, &mut output_slow, sample_rate, &context);

        // Fast ramp should have changed more after same number of samples
        let fast_progress = output_fast[block_size - 1] - output_fast[0];
        let slow_progress = output_slow[block_size - 1] - output_slow[0];

        assert!(
            fast_progress > slow_progress * 5.0,
            "Fast ramp should progress more. Fast: {}, Slow: {}",
            fast_progress,
            slow_progress
        );
    }

    #[test]
    fn test_xline_retrigger() {
        // Test 4: Verify retriggering restarts the ramp
        let mut xline = XLineNode::new(0, 1, 2, 3);

        let sample_rate = 44100.0;
        let block_size = 100;

        // Initial trigger
        let start = vec![100.0; block_size];
        let end = vec![1000.0; block_size];
        let duration = vec![0.1; block_size]; // Long duration
        let mut trigger = vec![1.0; block_size];

        let inputs = vec![
            start.as_slice(),
            end.as_slice(),
            duration.as_slice(),
            trigger.as_slice(),
        ];

        let mut output1 = vec![0.0; block_size];
        let context = create_context(block_size);

        xline.process_block(&inputs, &mut output1, sample_rate, &context);

        let value_before_retrigger = xline.value();
        assert!(value_before_retrigger > 100.0, "Should have started ramping");

        // Retrigger: trigger goes low then high again
        trigger.fill(0.0); // Low
        trigger[50] = 1.0; // Retrigger at sample 50
        trigger[51..].fill(1.0);

        let inputs = vec![
            start.as_slice(),
            end.as_slice(),
            duration.as_slice(),
            trigger.as_slice(),
        ];

        let mut output2 = vec![0.0; block_size];
        xline.process_block(&inputs, &mut output2, sample_rate, &context);

        // At sample 51 (right after retrigger), should be near start value
        assert!(
            output2[51] < value_before_retrigger,
            "After retrigger, value should reset. Before: {}, After: {}",
            value_before_retrigger,
            output2[51]
        );
        assert!(
            (output2[51] - 100.0).abs() < 50.0,
            "Should be near start value after retrigger, got {}",
            output2[51]
        );
    }

    #[test]
    fn test_xline_natural_pitch_sweep() {
        // Test 5: Verify exponential is better for pitch sweeps than linear
        let mut xline = XLineNode::new(0, 1, 2, 3);

        // Pitch sweep: 110 Hz (A2) to 880 Hz (A5) - 3 octaves
        let start = vec![110.0; 100];
        let end = vec![880.0; 100];
        let duration = vec![100.0 / 44100.0; 100];
        let trigger = vec![1.0; 100];

        let inputs = vec![
            start.as_slice(),
            end.as_slice(),
            duration.as_slice(),
            trigger.as_slice(),
        ];

        let mut output = vec![0.0; 100];
        let context = create_context(100);

        xline.process_block(&inputs, &mut output, 44100.0, &context);

        // For 3 octaves (8x frequency), exponential should pass through octaves evenly
        // Sample 33 (~1/3 through): should be around 220 Hz (1 octave up)
        // Sample 66 (~2/3 through): should be around 440 Hz (2 octaves up)
        let octave1 = output[33];
        let octave2 = output[66];

        // Check that we're passing through the octaves reasonably
        // (within tolerance for quantization and sample-rate effects)
        assert!(
            octave1 > 180.0 && octave1 < 260.0,
            "At 1/3, should be near 220 Hz (1 octave), got {}",
            octave1
        );
        assert!(
            octave2 > 380.0 && octave2 < 500.0,
            "At 2/3, should be near 440 Hz (2 octaves), got {}",
            octave2
        );

        // Linear would give: 33% = 365 Hz, 66% = 620 Hz (not musical octaves)
        // Exponential gives even musical spacing
    }

    #[test]
    fn test_xline_dependencies() {
        // Test 6: Verify input_nodes returns correct dependencies
        let xline = XLineNode::new(10, 20, 30, 40);
        let deps = xline.input_nodes();

        assert_eq!(deps.len(), 4);
        assert_eq!(deps[0], 10); // start_input
        assert_eq!(deps[1], 20); // end_input
        assert_eq!(deps[2], 30); // duration_input
        assert_eq!(deps[3], 40); // trigger_input
    }

    #[test]
    fn test_xline_with_constants() {
        // Test 7: Full cycle with constant node inputs
        let sample_rate = 44100.0;
        let block_size = 512;

        let mut start_node = ConstantNode::new(50.0);
        let mut end_node = ConstantNode::new(500.0);
        let mut duration_node = ConstantNode::new(0.01); // 10ms
        let mut trigger_node = ConstantNode::new(1.0);

        let mut xline = XLineNode::new(0, 1, 2, 3);

        let context = create_context(block_size);

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

        let mut output = vec![0.0; block_size];
        xline.process_block(&inputs, &mut output, sample_rate, &context);

        // Should start at start value
        assert!(
            (output[0] - 50.0).abs() < 1.0,
            "Should start at 50.0, got {}",
            output[0]
        );

        // Should be rising
        assert!(output[100] > output[50], "Should be rising");
        assert!(output[200] > output[100], "Should continue rising");

        // Should reach end value after duration
        let expected_samples = (0.01 * sample_rate) as usize;
        if expected_samples < block_size {
            assert!(
                (output[expected_samples] - 500.0).abs() < 1.0,
                "Should reach end value, got {}",
                output[expected_samples]
            );
        }
    }

    #[test]
    fn test_xline_zero_protection() {
        // Test 8: Verify protection against log(0) errors
        let mut xline = XLineNode::new(0, 1, 2, 3);

        // Try to use 0 or negative values (should be clamped to 0.0001)
        let start = vec![0.0; 100];
        let end = vec![-100.0; 100]; // Negative value
        let duration = vec![0.001; 100];
        let trigger = vec![1.0; 100];

        let inputs = vec![
            start.as_slice(),
            end.as_slice(),
            duration.as_slice(),
            trigger.as_slice(),
        ];

        let mut output = vec![0.0; 100];
        let context = create_context(100);

        // Should not panic or produce NaN
        xline.process_block(&inputs, &mut output, 44100.0, &context);

        // All outputs should be finite (not NaN or infinite)
        for (i, &val) in output.iter().enumerate() {
            assert!(val.is_finite(), "Sample {} is not finite: {}", i, val);
        }

        // Should produce some valid ramp (using clamped values)
        assert!(output[0] >= 0.0, "Output should be non-negative");
    }

    #[test]
    fn test_xline_holds_after_completion() {
        // Test 9: Verify output holds at end value after ramp completes
        let mut xline = XLineNode::new(0, 1, 2, 3);

        let sample_rate = 44100.0;
        let duration_seconds = 0.001; // 1ms = ~44 samples
        let block_size = 512;

        let start = vec![100.0; block_size];
        let end = vec![200.0; block_size];
        let duration = vec![duration_seconds; block_size];
        let trigger = vec![1.0; 1].into_iter()
            .chain(vec![0.0; block_size - 1]) // Trigger only first sample
            .collect::<Vec<_>>();

        let inputs = vec![
            start.as_slice(),
            end.as_slice(),
            duration.as_slice(),
            trigger.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        let context = create_context(block_size);

        xline.process_block(&inputs, &mut output, sample_rate, &context);

        // Find where ramp should complete
        let completion_sample = (duration_seconds * sample_rate) as usize;

        // After completion, all samples should be at end value
        for i in (completion_sample + 10)..block_size {
            assert!(
                (output[i] - 200.0).abs() < 0.1,
                "Sample {} should hold at 200.0, got {}",
                i,
                output[i]
            );
        }

        assert_eq!(xline.is_active(), false, "Should be inactive after completion");
    }
}
