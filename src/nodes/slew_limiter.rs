/// Slew limiter node - rate-of-change limiter for smooth transitions
///
/// This node limits how quickly a signal can change, useful for:
/// - Smoothing control signals (prevent zipper noise)
/// - Portamento/glide effects on pitch
/// - Envelope smoothing
/// - Parameter interpolation
/// - Creating low-pass filter-like smoothing
///
/// Unlike a low-pass filter, slew limiting preserves the target value
/// eventually - it just controls HOW FAST you get there.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Slew limiter node: limits rate of change in a signal
///
/// The algorithm:
/// ```text
/// for each sample:
///     target = input[i]
///     rise_time = max(0.0001, rise_time_input[i])
///     fall_time = max(0.0001, fall_time_input[i])
///
///     max_rise_per_sample = 1.0 / (rise_time * sample_rate)
///     max_fall_per_sample = 1.0 / (fall_time * sample_rate)
///
///     delta = target - last_value
///
///     if delta > 0.0:
///         // Rising: limit by rise rate
///         change = min(delta, max_rise_per_sample)
///         last_value += change
///     else:
///         // Falling: limit by fall rate
///         change = max(delta, -max_fall_per_sample)
///         last_value += change
///
///     output[i] = last_value
/// ```
///
/// # Example
/// ```ignore
/// // Smooth a step function with 0.1 second rise, 0.05 second fall
/// let input = ConstantNode::new(1.0);           // NodeId 0
/// let rise = ConstantNode::new(0.1);            // NodeId 1 (100ms rise)
/// let fall = ConstantNode::new(0.05);           // NodeId 2 (50ms fall)
/// let slew = SlewLimiterNode::new(0, 1, 2);     // NodeId 3
/// // Output will smoothly transition to 1.0 over 100ms
/// ```
pub struct SlewLimiterNode {
    /// Input signal to smooth
    input: NodeId,

    /// Rise time in seconds (time to rise from 0 to 1)
    rise_time_input: NodeId,

    /// Fall time in seconds (time to fall from 1 to 0)
    fall_time_input: NodeId,

    /// Previous output value (for calculating delta)
    last_value: f32,
}

impl SlewLimiterNode {
    /// Create a new slew limiter node
    ///
    /// # Arguments
    /// * `input` - NodeId of signal to smooth
    /// * `rise_time_input` - NodeId of rise time signal (in seconds)
    /// * `fall_time_input` - NodeId of fall time signal (in seconds)
    ///
    /// # Initial State
    /// - `last_value` starts at 0.0
    pub fn new(input: NodeId, rise_time_input: NodeId, fall_time_input: NodeId) -> Self {
        Self {
            input,
            rise_time_input,
            fall_time_input,
            last_value: 0.0,
        }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }

    /// Get the rise time input node ID
    pub fn rise_time_input(&self) -> NodeId {
        self.rise_time_input
    }

    /// Get the fall time input node ID
    pub fn fall_time_input(&self) -> NodeId {
        self.fall_time_input
    }

    /// Get the last output value (for debugging/testing)
    pub fn last_value(&self) -> f32 {
        self.last_value
    }
}

impl AudioNode for SlewLimiterNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 3,
            "SlewLimiterNode requires 3 inputs, got {}",
            inputs.len()
        );

        let input_buf = inputs[0];
        let rise_time_buf = inputs[1];
        let fall_time_buf = inputs[2];

        debug_assert_eq!(
            input_buf.len(),
            output.len(),
            "Input buffer length mismatch"
        );
        debug_assert_eq!(
            rise_time_buf.len(),
            output.len(),
            "Rise time buffer length mismatch"
        );
        debug_assert_eq!(
            fall_time_buf.len(),
            output.len(),
            "Fall time buffer length mismatch"
        );

        // Process each sample
        for i in 0..output.len() {
            let target = input_buf[i];
            let rise_time = rise_time_buf[i].max(0.0001); // Prevent division by zero (0.1ms min)
            let fall_time = fall_time_buf[i].max(0.0001); // Prevent division by zero (0.1ms min)

            // Calculate max change per sample
            let max_rise_per_sample = 1.0 / (rise_time * sample_rate);
            let max_fall_per_sample = 1.0 / (fall_time * sample_rate);

            let delta = target - self.last_value;

            if delta > 0.0 {
                // Rising: limit by rise rate
                let change = delta.min(max_rise_per_sample);
                self.last_value += change;
            } else {
                // Falling: limit by fall rate
                let change = delta.max(-max_fall_per_sample);
                self.last_value += change;
            }

            output[i] = self.last_value;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.rise_time_input, self.fall_time_input]
    }

    fn name(&self) -> &str {
        "SlewLimiterNode"
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
    fn test_slew_limiter_limits_rise() {
        let mut slew = SlewLimiterNode::new(0, 1, 2);

        // Instant step from 0 to 1
        let input = vec![1.0, 1.0, 1.0, 1.0, 1.0];
        let rise_time = vec![0.001; 5]; // 1ms to rise from 0 to 1
        let fall_time = vec![0.001; 5];
        let inputs = vec![input.as_slice(), rise_time.as_slice(), fall_time.as_slice()];

        let mut output = vec![0.0; 5];
        let context = create_context(5);

        slew.process_block(&inputs, &mut output, 44100.0, &context);

        // With 1ms rise time at 44100 Hz:
        // max_rise_per_sample = 1.0 / (0.001 * 44100) = 0.02267
        // Sample 0: 0.0 + 0.02267 = 0.02267
        // Sample 1: 0.02267 + 0.02267 = 0.04535
        // Sample 2: 0.04535 + 0.02267 = 0.06802
        // Should be gradually rising, not instant
        assert!(output[0] > 0.0);
        assert!(output[0] < 0.1, "output[0] = {}", output[0]);
        assert!(output[1] > output[0]);
        assert!(output[2] > output[1]);
        assert!(output[3] > output[2]);
        assert!(output[4] > output[3]);

        // Should not reach target yet (5 samples * 0.02267 ≈ 0.113)
        assert!(output[4] < 0.5, "output[4] = {}", output[4]);
    }

    #[test]
    fn test_slew_limiter_limits_fall() {
        let mut slew = SlewLimiterNode::new(0, 1, 2);

        // Pre-set last_value to 1.0 (simulate previous state)
        slew.last_value = 1.0;

        // Instant step from 1 to 0
        let input = vec![0.0, 0.0, 0.0, 0.0, 0.0];
        let rise_time = vec![0.001; 5];
        let fall_time = vec![0.001; 5]; // 1ms to fall from 1 to 0
        let inputs = vec![input.as_slice(), rise_time.as_slice(), fall_time.as_slice()];

        let mut output = vec![0.0; 5];
        let context = create_context(5);

        slew.process_block(&inputs, &mut output, 44100.0, &context);

        // With 1ms fall time at 44100 Hz:
        // max_fall_per_sample = 1.0 / (0.001 * 44100) = 0.02267
        // Sample 0: 1.0 - 0.02267 = 0.97733
        // Sample 1: 0.97733 - 0.02267 = 0.95465
        // Should be gradually falling, not instant
        assert!(output[0] < 1.0);
        assert!(output[0] > 0.9, "output[0] = {}", output[0]);
        assert!(output[1] < output[0]);
        assert!(output[2] < output[1]);
        assert!(output[3] < output[2]);
        assert!(output[4] < output[3]);

        // Should not reach target yet
        assert!(output[4] > 0.5, "output[4] = {}", output[4]);
    }

    #[test]
    fn test_slew_limiter_asymmetric_rates() {
        let mut slew = SlewLimiterNode::new(0, 1, 2);

        // Fast rise (0.1ms), slow fall (10ms)
        let input = vec![1.0, 1.0, 1.0, 0.0, 0.0, 0.0];
        let rise_time = vec![0.0001; 6]; // 0.1ms rise (10x faster)
        let fall_time = vec![0.01; 6];   // 10ms fall
        let inputs = vec![input.as_slice(), rise_time.as_slice(), fall_time.as_slice()];

        let mut output = vec![0.0; 6];
        let context = create_context(6);

        slew.process_block(&inputs, &mut output, 44100.0, &context);

        // Rise rate: 1.0 / (0.0001 * 44100) = 0.2267 per sample (fast)
        // Fall rate: 1.0 / (0.01 * 44100) = 0.002267 per sample (slow)

        // First 3 samples: rising quickly
        let rise_delta_0_1 = output[1] - output[0];

        // Last 3 samples: falling slowly
        let fall_delta_3_4 = output[3] - output[4];

        // Rise should be much faster than fall
        assert!(rise_delta_0_1 > fall_delta_3_4 * 5.0,
                "rise_delta = {}, fall_delta = {}",
                rise_delta_0_1, fall_delta_3_4);
    }

    #[test]
    fn test_slew_limiter_no_change_instant() {
        let mut slew = SlewLimiterNode::new(0, 1, 2);
        slew.last_value = 0.5;

        // Input stays at current value
        let input = vec![0.5, 0.5, 0.5, 0.5];
        let rise_time = vec![0.001; 4];
        let fall_time = vec![0.001; 4];
        let inputs = vec![input.as_slice(), rise_time.as_slice(), fall_time.as_slice()];

        let mut output = vec![0.0; 4];
        let context = create_context(4);

        slew.process_block(&inputs, &mut output, 44100.0, &context);

        // No change - should stay at 0.5
        for &val in &output {
            assert!((val - 0.5).abs() < 0.0001);
        }
    }

    #[test]
    fn test_slew_limiter_smooths_step() {
        let mut slew = SlewLimiterNode::new(0, 1, 2);

        // Step function: 0 -> 1 -> 0
        let input = vec![0.0, 0.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0];
        let rise_time = vec![0.001; 8];
        let fall_time = vec![0.001; 8];
        let inputs = vec![input.as_slice(), rise_time.as_slice(), fall_time.as_slice()];

        let mut output = vec![0.0; 8];
        let context = create_context(8);

        slew.process_block(&inputs, &mut output, 44100.0, &context);

        // Should start at 0
        assert!((output[0] - 0.0).abs() < 0.0001);

        // Should gradually rise when input goes to 1
        assert!(output[2] > output[1]);
        assert!(output[3] > output[2]);
        assert!(output[4] > output[3]);

        // Should gradually fall when input goes to 0
        assert!(output[5] < output[4]);
        assert!(output[6] < output[5]);
        assert!(output[7] < output[6]);

        // Should smooth the transitions (not instant jumps)
        assert!(output[2] < 1.0); // Not instant rise
        assert!(output[5] > 0.0); // Not instant fall
    }

    #[test]
    fn test_slew_limiter_dependencies() {
        let slew = SlewLimiterNode::new(3, 7, 11);
        let deps = slew.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 3);
        assert_eq!(deps[1], 7);
        assert_eq!(deps[2], 11);
    }

    #[test]
    fn test_slew_limiter_with_constants() {
        let mut slew = SlewLimiterNode::new(0, 1, 2);

        // Constant rise/fall times
        let input = vec![1.0; 10];
        let rise_time = vec![0.0001; 10]; // Very fast rise (0.1ms)
        let fall_time = vec![0.01; 10];   // Slower fall (10ms)
        let inputs = vec![input.as_slice(), rise_time.as_slice(), fall_time.as_slice()];

        let mut output = vec![0.0; 10];
        let context = create_context(10);

        slew.process_block(&inputs, &mut output, 44100.0, &context);

        // Should rise quickly toward 1.0
        // Rise rate: 1.0 / (0.0001 * 44100) ≈ 0.2267 per sample
        // After 10 samples: 10 * 0.2267 ≈ 2.267 (clamped to 1.0)
        assert!(output[9] > 0.9, "output[9] = {}", output[9]);

        // Each sample should be higher than the last
        for i in 1..10 {
            assert!(output[i] >= output[i-1],
                    "output[{}] = {}, output[{}] = {}",
                    i, output[i], i-1, output[i-1]);
        }
    }

    #[test]
    fn test_slew_limiter_reaches_target_eventually() {
        let mut slew = SlewLimiterNode::new(0, 1, 2);

        // Very long buffer with enough samples to reach target
        let samples = 100;
        let input = vec![1.0; samples];
        let rise_time = vec![0.001; samples]; // 1ms rise time
        let fall_time = vec![0.001; samples];
        let inputs = vec![input.as_slice(), rise_time.as_slice(), fall_time.as_slice()];

        let mut output = vec![0.0; samples];
        let context = create_context(samples);

        slew.process_block(&inputs, &mut output, 44100.0, &context);

        // Rise rate: 1.0 / (0.001 * 44100) ≈ 0.02267 per sample
        // To reach 1.0: 1.0 / 0.02267 ≈ 44.1 samples
        // After 100 samples, should definitely reach 1.0
        assert!((output[99] - 1.0).abs() < 0.001, "output[99] = {}", output[99]);
    }

    #[test]
    fn test_slew_limiter_state_persistence() {
        // Test that slew state persists across multiple process_block calls
        let mut slew = SlewLimiterNode::new(0, 1, 2);

        // First block: start rising toward 1.0
        let input1 = vec![1.0; 5];
        let rise1 = vec![0.001; 5];
        let fall1 = vec![0.001; 5];
        let inputs1 = vec![input1.as_slice(), rise1.as_slice(), fall1.as_slice()];
        let mut output1 = vec![0.0; 5];
        let context = create_context(5);

        slew.process_block(&inputs1, &mut output1, 44100.0, &context);

        let end_of_block1 = output1[4];
        assert!(end_of_block1 > 0.0);
        assert!(end_of_block1 < 1.0);

        // Second block: continue rising from where we left off
        let input2 = vec![1.0; 5];
        let rise2 = vec![0.001; 5];
        let fall2 = vec![0.001; 5];
        let inputs2 = vec![input2.as_slice(), rise2.as_slice(), fall2.as_slice()];
        let mut output2 = vec![0.0; 5];

        slew.process_block(&inputs2, &mut output2, 44100.0, &context);

        // Should start where previous block ended and continue rising
        assert!((output2[0] - end_of_block1).abs() < 0.03,
                "output2[0] = {}, end_of_block1 = {}",
                output2[0], end_of_block1);
        assert!(output2[4] > output2[0]);
    }
}
