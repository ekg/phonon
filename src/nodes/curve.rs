/// Curve Generator - Curved ramp generator with exponential/logarithmic curves
///
/// Generates a curved ramp from start to end value over a specified duration.
/// The curve parameter controls the shape of the interpolation:
/// - curve = 0: Linear (same as LineNode)
/// - curve > 0: Exponential (slow start, fast end)
/// - curve < 0: Logarithmic (fast start, slow end)
///
/// This is especially useful for:
/// - Natural-sounding pitch glides
/// - Smooth filter sweeps
/// - Musical volume fades
/// - Any parameter that needs non-linear transitions
///
/// # Algorithm
/// The curve interpolation uses the formula:
/// ```text
/// if curve ≈ 0:
///     output = start + (end - start) * progress  // Linear
/// else:
///     // Exponential curve
///     output = start + (end - start) * (exp(curve * progress) - 1) / (exp(curve) - 1)
/// ```
///
/// # Key Features
/// - Curve parameter: -10 (logarithmic) to +10 (exponential)
/// - Trigger input restarts the ramp
/// - Holds at end value after completion
/// - All parameters pattern-controllable
///
/// # References
/// - SuperCollider Line.kr with curve parameter
/// - Max/MSP curve~
/// - Exponential/logarithmic interpolation in audio DSP

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Curve Generator Node
///
/// # Inputs
/// 1. Trigger input (> 0.5 = trigger, rising edge detection)
/// 2. Start value - Beginning of the ramp
/// 3. End value - Target of the ramp
/// 4. Duration in seconds - Time to complete ramp
/// 5. Curve amount (-10 to +10, 0 = linear)
///
/// # Curve Shapes
/// ```text
/// curve = 0 (linear):     /
/// curve = 3 (exponential): ⌐
/// curve = -3 (logarithmic): ⌙
/// ```
///
/// # Example
/// ```ignore
/// // Create curve that exponentially ramps from 0 to 1 over 2 seconds
/// let trigger = ConstantNode::new(1.0);      // NodeId 0
/// let start = ConstantNode::new(0.0);        // NodeId 1
/// let end = ConstantNode::new(1.0);          // NodeId 2
/// let duration = ConstantNode::new(2.0);     // NodeId 3
/// let curve_amount = ConstantNode::new(3.0); // NodeId 4 (exponential)
/// let curve = CurveNode::new(0, 1, 2, 3, 4); // NodeId 5
/// ```
pub struct CurveNode {
    trigger_input: NodeId,    // Trigger to restart ramp
    start_input: NodeId,      // Starting value
    end_input: NodeId,        // Ending value
    duration_input: NodeId,   // Duration in seconds
    curve_input: NodeId,      // Curve amount (-10 to +10)
    current_value: f32,       // Current output value
    elapsed_time: f32,        // Time elapsed in current ramp (seconds)
    is_active: bool,          // Is ramp currently active?
    last_trigger: f32,        // Previous trigger value (for edge detection)
}

impl CurveNode {
    /// Create a new curve generator node
    ///
    /// # Arguments
    /// * `trigger_input` - NodeId providing trigger signal (> 0.5 = trigger)
    /// * `start_input` - NodeId providing start value
    /// * `end_input` - NodeId providing end value
    /// * `duration_input` - NodeId providing duration in seconds
    /// * `curve_input` - NodeId providing curve amount (-10 to +10, 0 = linear)
    pub fn new(
        trigger_input: NodeId,
        start_input: NodeId,
        end_input: NodeId,
        duration_input: NodeId,
        curve_input: NodeId,
    ) -> Self {
        Self {
            trigger_input,
            start_input,
            end_input,
            duration_input,
            curve_input,
            current_value: 0.0,
            elapsed_time: 0.0,
            is_active: false,
            last_trigger: 0.0,
        }
    }

    /// Get current curve value (for debugging/testing)
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

    /// Reset curve to initial state
    pub fn reset(&mut self) {
        self.current_value = 0.0;
        self.elapsed_time = 0.0;
        self.is_active = false;
        self.last_trigger = 0.0;
    }

    /// Interpolate value using curve
    ///
    /// # Arguments
    /// * `progress` - Progress from 0.0 to 1.0
    /// * `curve` - Curve amount (-10 to +10, 0 = linear)
    ///
    /// # Returns
    /// Curved progress value (0.0 to 1.0)
    fn interpolate_curve(progress: f32, curve: f32) -> f32 {
        if curve.abs() < 0.001 {
            // Linear interpolation (curve ≈ 0)
            progress
        } else {
            // Exponential curve using formula:
            // y = (exp(curve * x) - 1) / (exp(curve) - 1)
            let exp_curve = curve.exp();
            let exp_progress = (curve * progress).exp();
            (exp_progress - 1.0) / (exp_curve - 1.0)
        }
    }
}

impl AudioNode for CurveNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 5,
            "CurveNode requires 5 inputs: trigger, start, end, duration, curve"
        );

        let trigger_buffer = inputs[0];
        let start_buffer = inputs[1];
        let end_buffer = inputs[2];
        let duration_buffer = inputs[3];
        let curve_buffer = inputs[4];

        debug_assert_eq!(trigger_buffer.len(), output.len(), "Trigger buffer length mismatch");
        debug_assert_eq!(start_buffer.len(), output.len(), "Start buffer length mismatch");
        debug_assert_eq!(end_buffer.len(), output.len(), "End buffer length mismatch");
        debug_assert_eq!(duration_buffer.len(), output.len(), "Duration buffer length mismatch");
        debug_assert_eq!(curve_buffer.len(), output.len(), "Curve buffer length mismatch");

        for i in 0..output.len() {
            let trigger = trigger_buffer[i];
            let start = start_buffer[i];
            let end = end_buffer[i];
            let duration = duration_buffer[i].max(0.001); // Minimum 1ms
            let curve = curve_buffer[i].clamp(-10.0, 10.0); // Clamp to reasonable range

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
                // Calculate progress (0.0 to 1.0)
                let progress = self.elapsed_time / duration;

                if progress >= 1.0 {
                    // Ramp complete: hold at end value
                    self.current_value = end;
                    self.is_active = false;
                } else {
                    // Ramp in progress - apply curve interpolation
                    let curved_progress = Self::interpolate_curve(progress, curve);
                    self.current_value = start + (end - start) * curved_progress;
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
            self.trigger_input,
            self.start_input,
            self.end_input,
            self.duration_input,
            self.curve_input,
        ]
    }

    fn name(&self) -> &str {
        "CurveNode"
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
    fn test_curve_linear_matches_line() {
        // Test 1: Curve with curve=0 should produce linear ramp (like LineNode)
        let sample_rate = 44100.0;
        let start_value = 0.0;
        let end_value = 1.0;
        let duration = 1.0; // 1 second
        let curve_amount = 0.0; // Linear
        let block_size = 512;

        let mut curve = CurveNode::new(0, 1, 2, 3, 4);

        let context = create_context(block_size);

        let trigger_buf = vec![1.0; block_size];
        let start_buf = vec![start_value; block_size];
        let end_buf = vec![end_value; block_size];
        let duration_buf = vec![duration; block_size];
        let curve_buf = vec![curve_amount; block_size];

        let inputs = vec![
            trigger_buf.as_slice(),
            start_buf.as_slice(),
            end_buf.as_slice(),
            duration_buf.as_slice(),
            curve_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        curve.process_block(&inputs, &mut output, sample_rate, &context);

        // First sample should be at start value
        assert_eq!(output[0], start_value, "First sample should be at start value");

        // Values should be increasing linearly
        assert!(output[100] > output[0], "Curve should be rising");
        assert!(output[200] > output[100], "Curve should continue rising");

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
    fn test_curve_exponential_slow_start() {
        // Test 2: Positive curve should produce exponential (slow start, fast end)
        let sample_rate = 44100.0;
        let start_value = 0.0;
        let end_value = 1.0;
        let duration = 0.01; // 10ms = 441 samples
        let curve_amount = 5.0; // Strong exponential
        let block_size = 512;

        let mut curve = CurveNode::new(0, 1, 2, 3, 4);

        let context = create_context(block_size);

        let trigger_buf = vec![1.0; block_size];
        let start_buf = vec![start_value; block_size];
        let end_buf = vec![end_value; block_size];
        let duration_buf = vec![duration; block_size];
        let curve_buf = vec![curve_amount; block_size];

        let inputs = vec![
            trigger_buf.as_slice(),
            start_buf.as_slice(),
            end_buf.as_slice(),
            duration_buf.as_slice(),
            curve_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        curve.process_block(&inputs, &mut output, sample_rate, &context);

        // Exponential curve should be below linear at midpoint
        let samples = (duration * sample_rate) as usize;
        let mid = samples / 2;
        let linear_mid = 0.5;

        if mid < block_size {
            assert!(
                output[mid] < linear_mid,
                "Exponential curve should be < 0.5 at midpoint, got {}",
                output[mid]
            );
        }

        // Progress should accelerate (second half should gain more than first half)
        // For exponential: delta in first quarter + delta in second quarter < delta in third quarter + delta in fourth quarter
        let q1 = output[samples / 4];
        let q2 = output[samples / 2];
        let q3 = output[samples * 3 / 4];
        let q4 = output[samples - 1];

        let delta_first_half = q2 - q1;  // Change in first half
        let delta_second_half = q4 - q3; // Change in second half

        assert!(
            delta_second_half > delta_first_half,
            "Second half should progress faster than first half: first_half_delta={:.4}, second_half_delta={:.4}",
            delta_first_half,
            delta_second_half
        );
    }

    #[test]
    fn test_curve_logarithmic_fast_start() {
        // Test 3: Negative curve should produce logarithmic (fast start, slow end)
        let sample_rate = 44100.0;
        let start_value = 0.0;
        let end_value = 1.0;
        let duration = 0.01; // 10ms = 441 samples
        let curve_amount = -5.0; // Strong logarithmic
        let block_size = 512;

        let mut curve = CurveNode::new(0, 1, 2, 3, 4);

        let context = create_context(block_size);

        let trigger_buf = vec![1.0; block_size];
        let start_buf = vec![start_value; block_size];
        let end_buf = vec![end_value; block_size];
        let duration_buf = vec![duration; block_size];
        let curve_buf = vec![curve_amount; block_size];

        let inputs = vec![
            trigger_buf.as_slice(),
            start_buf.as_slice(),
            end_buf.as_slice(),
            duration_buf.as_slice(),
            curve_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        curve.process_block(&inputs, &mut output, sample_rate, &context);

        // Logarithmic curve should be above linear at midpoint
        let samples = (duration * sample_rate) as usize;
        let mid = samples / 2;
        let linear_mid = 0.5;

        if mid < block_size {
            assert!(
                output[mid] > linear_mid,
                "Logarithmic curve should be > 0.5 at midpoint, got {}",
                output[mid]
            );
        }

        // Progress should decelerate (first half should gain more than second half)
        let first_half = output[samples / 4];
        let second_half = output[samples * 3 / 4];
        assert!(
            first_half > (1.0 - second_half),
            "First half should progress faster than second half"
        );
    }

    #[test]
    fn test_curve_extreme_values() {
        // Test 4: Extreme curve values should be clamped and work correctly
        let sample_rate = 44100.0;
        let start_value = 0.0;
        let end_value = 1.0;
        let duration = 0.01;
        let block_size = 512;

        // Test extreme positive curve
        let mut curve_pos = CurveNode::new(0, 1, 2, 3, 4);
        let context = create_context(block_size);

        let trigger_buf = vec![1.0; block_size];
        let start_buf = vec![start_value; block_size];
        let end_buf = vec![end_value; block_size];
        let duration_buf = vec![duration; block_size];
        let curve_buf = vec![100.0; block_size]; // Very high value (will be clamped to 10)

        let inputs = vec![
            trigger_buf.as_slice(),
            start_buf.as_slice(),
            end_buf.as_slice(),
            duration_buf.as_slice(),
            curve_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        curve_pos.process_block(&inputs, &mut output, sample_rate, &context);

        // Should not produce NaN or infinite values
        for (i, &val) in output.iter().enumerate() {
            assert!(val.is_finite(), "Sample {} is not finite: {}", i, val);
        }

        // Should still reach end value
        let samples = (duration * sample_rate) as usize;
        if samples < block_size {
            assert!(
                (output[samples] - end_value).abs() < 0.1,
                "Should reach end value, got {}",
                output[samples]
            );
        }

        // Test extreme negative curve
        let mut curve_neg = CurveNode::new(0, 1, 2, 3, 4);
        let curve_buf_neg = vec![-100.0; block_size]; // Very low value (will be clamped to -10)

        let inputs_neg = vec![
            trigger_buf.as_slice(),
            start_buf.as_slice(),
            end_buf.as_slice(),
            duration_buf.as_slice(),
            curve_buf_neg.as_slice(),
        ];

        let mut output_neg = vec![0.0; block_size];
        curve_neg.process_block(&inputs_neg, &mut output_neg, sample_rate, &context);

        // Should not produce NaN or infinite values
        for (i, &val) in output_neg.iter().enumerate() {
            assert!(val.is_finite(), "Sample {} is not finite: {}", i, val);
        }
    }

    #[test]
    fn test_curve_retrigger() {
        // Test 5: Retriggering should restart the ramp
        let sample_rate = 44100.0;
        let start_value = 0.0;
        let end_value = 1.0;
        let duration = 0.1; // 100ms
        let curve_amount = 3.0;
        let block_size = 512;

        let mut curve = CurveNode::new(0, 1, 2, 3, 4);

        let context = create_context(block_size);

        let start_buf = vec![start_value; block_size];
        let end_buf = vec![end_value; block_size];
        let duration_buf = vec![duration; block_size];
        let curve_buf = vec![curve_amount; block_size];

        // First trigger
        let trigger_buf = vec![1.0; block_size];
        let inputs = vec![
            trigger_buf.as_slice(),
            start_buf.as_slice(),
            end_buf.as_slice(),
            duration_buf.as_slice(),
            curve_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        curve.process_block(&inputs, &mut output, sample_rate, &context);

        // Should be active and ramping
        assert!(curve.is_active(), "Curve should be active after trigger");
        assert!(output[block_size - 1] > start_value, "Curve should have progressed");

        // Trigger off (low)
        let trigger_buf = vec![0.0; block_size];
        let inputs = vec![
            trigger_buf.as_slice(),
            start_buf.as_slice(),
            end_buf.as_slice(),
            duration_buf.as_slice(),
            curve_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        curve.process_block(&inputs, &mut output, sample_rate, &context);

        let value_before_retrigger = output[block_size - 1];

        // Retrigger (rising edge)
        let trigger_buf = vec![1.0; block_size];
        let inputs = vec![
            trigger_buf.as_slice(),
            start_buf.as_slice(),
            end_buf.as_slice(),
            duration_buf.as_slice(),
            curve_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        curve.process_block(&inputs, &mut output, sample_rate, &context);

        // First sample should be at start (restarted)
        assert_eq!(
            output[0], start_value,
            "Retrigger should restart at start value"
        );
        assert!(
            output[0] < value_before_retrigger,
            "Retrigger should reset to lower value"
        );
    }

    #[test]
    fn test_curve_duration_accuracy() {
        // Test 6: Verify ramp completes at exact duration
        let sample_rate = 44100.0;
        let start_value = 0.0;
        let end_value = 1.0;
        let duration = 0.01; // 10ms = 441 samples
        let curve_amount = 2.0;
        let block_size = 512;

        let mut curve = CurveNode::new(0, 1, 2, 3, 4);

        let context = create_context(block_size);

        let trigger_buf = vec![1.0; block_size];
        let start_buf = vec![start_value; block_size];
        let end_buf = vec![end_value; block_size];
        let duration_buf = vec![duration; block_size];
        let curve_buf = vec![curve_amount; block_size];

        let inputs = vec![
            trigger_buf.as_slice(),
            start_buf.as_slice(),
            end_buf.as_slice(),
            duration_buf.as_slice(),
            curve_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        curve.process_block(&inputs, &mut output, sample_rate, &context);

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

            assert!(!curve.is_active(), "Should be inactive after completion");
        }
    }

    #[test]
    fn test_curve_holds_at_end() {
        // Test 7: Curve should hold at end value after completion
        let sample_rate = 44100.0;
        let start_value = 0.0;
        let end_value = 1.0;
        let duration = 0.005; // 5ms = 220 samples (completes in first block)
        let curve_amount = 4.0;
        let block_size = 512;

        let mut curve = CurveNode::new(0, 1, 2, 3, 4);

        let context = create_context(block_size);

        let trigger_buf = vec![1.0; block_size];
        let start_buf = vec![start_value; block_size];
        let end_buf = vec![end_value; block_size];
        let duration_buf = vec![duration; block_size];
        let curve_buf = vec![curve_amount; block_size];

        let inputs = vec![
            trigger_buf.as_slice(),
            start_buf.as_slice(),
            end_buf.as_slice(),
            duration_buf.as_slice(),
            curve_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        curve.process_block(&inputs, &mut output, sample_rate, &context);

        // After completion, should hold at end value
        let completion_sample = (duration * sample_rate) as usize + 10;
        if completion_sample < block_size {
            assert_eq!(
                output[completion_sample], end_value,
                "Should hold at end value after completion"
            );
            assert_eq!(
                output[block_size - 1], end_value,
                "Should still hold at end value"
            );
            assert!(!curve.is_active(), "Curve should be inactive after completion");
        }
    }

    #[test]
    fn test_curve_modulation() {
        // Test 8: Verify curve parameter can be modulated during ramp
        let sample_rate = 44100.0;
        let start_value = 0.0;
        let end_value = 1.0;
        let duration = 0.01; // 10ms
        let block_size = 512;

        let mut curve = CurveNode::new(0, 1, 2, 3, 4);

        let context = create_context(block_size);

        // Create modulating curve parameter: starts linear, becomes exponential
        let mut curve_buf = vec![0.0; block_size];
        for i in 0..block_size {
            curve_buf[i] = (i as f32 / block_size as f32) * 10.0; // 0 to 10
        }

        let trigger_buf = vec![1.0; block_size];
        let start_buf = vec![start_value; block_size];
        let end_buf = vec![end_value; block_size];
        let duration_buf = vec![duration; block_size];

        let inputs = vec![
            trigger_buf.as_slice(),
            start_buf.as_slice(),
            end_buf.as_slice(),
            duration_buf.as_slice(),
            curve_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        curve.process_block(&inputs, &mut output, sample_rate, &context);

        // Should produce valid output
        for (i, &val) in output.iter().enumerate() {
            assert!(val.is_finite(), "Sample {} is not finite: {}", i, val);
        }

        // Should still be rising
        assert!(output[100] > output[0], "Should be rising");
    }

    #[test]
    fn test_curve_short_duration() {
        // Test 9: Very short duration should work correctly
        let sample_rate = 44100.0;
        let start_value = 0.0;
        let end_value = 1.0;
        let duration = 0.001; // 1ms = ~44 samples
        let curve_amount = 5.0;
        let block_size = 512;

        let mut curve = CurveNode::new(0, 1, 2, 3, 4);

        let context = create_context(block_size);

        let trigger_buf = vec![1.0; block_size];
        let start_buf = vec![start_value; block_size];
        let end_buf = vec![end_value; block_size];
        let duration_buf = vec![duration; block_size];
        let curve_buf = vec![curve_amount; block_size];

        let inputs = vec![
            trigger_buf.as_slice(),
            start_buf.as_slice(),
            end_buf.as_slice(),
            duration_buf.as_slice(),
            curve_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        curve.process_block(&inputs, &mut output, sample_rate, &context);

        // Should reach end value quickly
        let completion_sample = (duration * sample_rate) as usize;
        if completion_sample < block_size {
            assert!(
                (output[completion_sample] - end_value).abs() < 0.1,
                "Should reach end value, got {}",
                output[completion_sample]
            );
        }
    }

    #[test]
    fn test_curve_long_duration() {
        // Test 10: Long duration should work correctly
        let sample_rate = 44100.0;
        let start_value = 0.0;
        let end_value = 1.0;
        let duration = 10.0; // 10 seconds
        let curve_amount = 3.0;
        let block_size = 512;

        let mut curve = CurveNode::new(0, 1, 2, 3, 4);

        let context = create_context(block_size);

        let trigger_buf = vec![1.0; block_size];
        let start_buf = vec![start_value; block_size];
        let end_buf = vec![end_value; block_size];
        let duration_buf = vec![duration; block_size];
        let curve_buf = vec![curve_amount; block_size];

        let inputs = vec![
            trigger_buf.as_slice(),
            start_buf.as_slice(),
            end_buf.as_slice(),
            duration_buf.as_slice(),
            curve_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        curve.process_block(&inputs, &mut output, sample_rate, &context);

        // Should be rising slowly
        assert!(output[0] == start_value, "Should start at start value");
        assert!(output[block_size - 1] > start_value, "Should be rising");
        assert!(output[block_size - 1] < end_value, "Should not reach end yet");
        assert!(curve.is_active(), "Should still be active");
    }

    #[test]
    fn test_curve_dependencies() {
        // Test 11: Verify input_nodes returns correct dependencies
        let curve = CurveNode::new(10, 20, 30, 40, 50);
        let deps = curve.input_nodes();

        assert_eq!(deps.len(), 5);
        assert_eq!(deps[0], 10); // trigger_input
        assert_eq!(deps[1], 20); // start_input
        assert_eq!(deps[2], 30); // end_input
        assert_eq!(deps[3], 40); // duration_input
        assert_eq!(deps[4], 50); // curve_input
    }

    #[test]
    fn test_curve_with_constants() {
        // Test 12: Full ramp with constant parameters
        let sample_rate = 44100.0;
        let start_value = 100.0;
        let end_value = 500.0;
        let duration = 0.02; // 20ms = 882 samples
        let curve_amount = 4.0; // Exponential
        let block_size = 512;

        let mut trigger_node = ConstantNode::new(1.0);
        let mut start_node = ConstantNode::new(start_value);
        let mut end_node = ConstantNode::new(end_value);
        let mut duration_node = ConstantNode::new(duration);
        let mut curve_node = ConstantNode::new(curve_amount);

        let mut curve = CurveNode::new(0, 1, 2, 3, 4);

        let context = create_context(block_size);

        // Generate constant buffers
        let mut trigger_buf = vec![0.0; block_size];
        let mut start_buf = vec![0.0; block_size];
        let mut end_buf = vec![0.0; block_size];
        let mut duration_buf = vec![0.0; block_size];
        let mut curve_buf = vec![0.0; block_size];

        trigger_node.process_block(&[], &mut trigger_buf, sample_rate, &context);
        start_node.process_block(&[], &mut start_buf, sample_rate, &context);
        end_node.process_block(&[], &mut end_buf, sample_rate, &context);
        duration_node.process_block(&[], &mut duration_buf, sample_rate, &context);
        curve_node.process_block(&[], &mut curve_buf, sample_rate, &context);

        let inputs = vec![
            trigger_buf.as_slice(),
            start_buf.as_slice(),
            end_buf.as_slice(),
            duration_buf.as_slice(),
            curve_buf.as_slice(),
        ];

        // Process first block
        let mut output = vec![0.0; block_size];
        curve.process_block(&inputs, &mut output, sample_rate, &context);

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
            trigger_buf.as_slice(),
            start_buf.as_slice(),
            end_buf.as_slice(),
            duration_buf.as_slice(),
            curve_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        curve.process_block(&inputs, &mut output, sample_rate, &context);

        // Should reach and hold at end_value
        assert_eq!(
            output[block_size - 1], end_value,
            "Should hold at end value in second block"
        );
        assert!(!curve.is_active(), "Should be inactive after completion");
    }

    #[test]
    fn test_curve_negative_ramp() {
        // Test 13: Verify ramp works with descending values
        let sample_rate = 44100.0;
        let start_value = 1.0;
        let end_value = 0.0;
        let duration = 0.01; // 10ms
        let curve_amount = -3.0; // Logarithmic (fast start for descending)
        let block_size = 512;

        let mut curve = CurveNode::new(0, 1, 2, 3, 4);

        let context = create_context(block_size);

        let trigger_buf = vec![1.0; block_size];
        let start_buf = vec![start_value; block_size];
        let end_buf = vec![end_value; block_size];
        let duration_buf = vec![duration; block_size];
        let curve_buf = vec![curve_amount; block_size];

        let inputs = vec![
            trigger_buf.as_slice(),
            start_buf.as_slice(),
            end_buf.as_slice(),
            duration_buf.as_slice(),
            curve_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        curve.process_block(&inputs, &mut output, sample_rate, &context);

        // Should start at 1.0
        assert_eq!(output[0], start_value, "Should start at start value");

        // Should be descending
        assert!(output[100] < output[0], "Curve should be descending");
        assert!(output[200] < output[100], "Curve should continue descending");

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

    #[test]
    fn test_curve_pitch_sweep() {
        // Test 14: Musical pitch sweep use case
        let sample_rate = 44100.0;
        let start_freq = 110.0; // A2
        let end_freq = 880.0;   // A5 (3 octaves up)
        let duration = 0.02;    // 20ms sweep
        let curve_amount = 5.0; // Exponential for musical pitch
        let block_size = 1024;

        let mut curve = CurveNode::new(0, 1, 2, 3, 4);

        let context = create_context(block_size);

        let trigger_buf = vec![1.0; block_size];
        let start_buf = vec![start_freq; block_size];
        let end_buf = vec![end_freq; block_size];
        let duration_buf = vec![duration; block_size];
        let curve_buf = vec![curve_amount; block_size];

        let inputs = vec![
            trigger_buf.as_slice(),
            start_buf.as_slice(),
            end_buf.as_slice(),
            duration_buf.as_slice(),
            curve_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        curve.process_block(&inputs, &mut output, sample_rate, &context);

        // Should start at low frequency
        assert_eq!(output[0], start_freq, "Should start at start frequency");

        // Should sweep up to high frequency
        assert!(output[100] > start_freq, "Should be rising");
        assert!(output[100] < end_freq, "Should not reach end yet");

        // Should reach end frequency
        let completion_sample = (duration * sample_rate) as usize;
        if completion_sample < block_size {
            assert!(
                (output[completion_sample] - end_freq).abs() < 10.0,
                "Should reach end frequency, got {}",
                output[completion_sample]
            );
        }
    }
}
