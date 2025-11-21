/// Timer node - measures elapsed time since last trigger reset
///
/// The timer outputs the elapsed time in seconds, counting up from 0.
/// When the trigger input has a rising edge (0→1 transition), the timer
/// resets to 0 and starts counting again.
///
/// # Algorithm
/// ```text
/// Detect rising edge: last_trigger < 0.5 AND trigger >= 0.5
/// If rising edge:
///   Reset elapsed_time = 0
/// Else:
///   Increment elapsed_time += 1/sample_rate
/// Output: elapsed_time (in seconds)
/// ```
///
/// # Applications
/// - Clock division (reset timer periodically)
/// - Measuring gate durations
/// - Creating time-based ramps/modulations
/// - Scheduling events
/// - Creating rhythmic variations
///
/// # Example
/// ```ignore
/// // Timer that resets every beat
/// let beat_pulse = PulseNode::new(1.0);  // NodeId 1 (1 Hz pulse)
/// let timer = TimerNode::new(1);         // NodeId 2
/// // Output: ramps from 0.0 to 1.0 every second
/// ```

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Timer state for time tracking
#[derive(Debug, Clone)]
struct TimerState {
    elapsed_time: f32,  // Current elapsed time in seconds
    last_trigger: f32,  // Previous trigger value (for edge detection)
}

impl Default for TimerState {
    fn default() -> Self {
        Self {
            elapsed_time: 0.0,
            last_trigger: 0.0,
        }
    }
}

/// Timer node: measures elapsed time since last reset
///
/// Outputs a continuously incrementing time value (in seconds) that
/// resets to 0 whenever the trigger input has a rising edge.
pub struct TimerNode {
    trigger_input: NodeId,
    state: TimerState,
}

impl TimerNode {
    /// Timer - Measures elapsed time since last trigger reset
    ///
    /// Counts up continuously in seconds. Resets to 0 on rising edge
    /// of trigger (0.5 threshold). Useful for time-based modulation.
    ///
    /// # Parameters
    /// - `trigger_input`: Trigger signal (rising edge resets timer)
    ///
    /// # Example
    /// ```phonon
    /// ~beat: lfo 1.0 0 1
    /// ~time: timer ~beat
    /// out: sine 440 * (cos (time * tau))
    /// ```
    pub fn new(trigger_input: NodeId) -> Self {
        Self {
            trigger_input,
            state: TimerState::default(),
        }
    }

    /// Get the trigger input node ID
    pub fn trigger_input(&self) -> NodeId {
        self.trigger_input
    }

    /// Reset timer state to 0
    pub fn reset(&mut self) {
        self.state = TimerState::default();
    }

    /// Get current elapsed time (for debugging/testing)
    pub fn elapsed_time(&self) -> f32 {
        self.state.elapsed_time
    }
}

impl AudioNode for TimerNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 1,
            "TimerNode requires 1 input, got {}",
            inputs.len()
        );

        let trigger_buf = inputs[0];

        debug_assert_eq!(
            trigger_buf.len(),
            output.len(),
            "Input buffer length mismatch"
        );

        let dt = 1.0 / sample_rate; // Time increment per sample

        // Process each sample
        for i in 0..output.len() {
            let trigger_val = trigger_buf[i];

            // Detect rising edge: last_trigger < 0.5 and trigger_val >= 0.5
            if self.state.last_trigger < 0.5 && trigger_val >= 0.5 {
                // Rising edge detected: reset timer
                self.state.elapsed_time = 0.0;
            } else {
                // No edge: increment elapsed time
                self.state.elapsed_time += dt;
            }

            // Output current elapsed time
            output[i] = self.state.elapsed_time;

            // Update last_trigger for next sample
            self.state.last_trigger = trigger_val;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.trigger_input]
    }

    fn name(&self) -> &str {
        "TimerNode"
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
    fn test_timer_counts_up() {
        // Test that timer increments over time
        let size = 512;
        let sample_rate = 44100.0;

        // No trigger (constant 0.0)
        let trigger = vec![0.0; size];
        let inputs: Vec<&[f32]> = vec![&trigger];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut timer = TimerNode::new(0);
        timer.process_block(&inputs, &mut output, sample_rate, &context);

        // Timer should increment linearly
        // First sample should be near dt = 1/44100
        let dt = 1.0 / sample_rate;
        assert!(
            (output[0] - dt).abs() < 0.000001,
            "First sample should be ~{}, got {}",
            dt,
            output[0]
        );

        // Last sample should be near size * dt
        let expected_last = (size as f32) * dt;
        assert!(
            (output[size - 1] - expected_last).abs() < 0.0001,
            "Last sample should be ~{}, got {}",
            expected_last,
            output[size - 1]
        );

        // Should be monotonically increasing
        for i in 1..size {
            assert!(
                output[i] > output[i - 1],
                "Timer should increase monotonically"
            );
        }
    }

    #[test]
    fn test_timer_resets_on_rising_edge() {
        // Test that timer resets when trigger rises
        let size = 512;
        let sample_rate = 44100.0;

        // Trigger: low for first half, high for second half
        let mut trigger = vec![0.0; size];
        for i in size / 2..size {
            trigger[i] = 1.0;
        }

        let inputs: Vec<&[f32]> = vec![&trigger];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut timer = TimerNode::new(0);
        timer.process_block(&inputs, &mut output, sample_rate, &context);

        // Timer should count up in first half
        let midpoint = size / 2;
        assert!(
            output[midpoint - 1] > 0.001,
            "Timer should have counted up before reset, got {}",
            output[midpoint - 1]
        );

        // Timer should reset at rising edge (midpoint)
        let dt = 1.0 / sample_rate;
        assert!(
            output[midpoint] < dt * 2.0,
            "Timer should reset at rising edge, got {}",
            output[midpoint]
        );

        // Timer should count up again in second half
        assert!(
            output[size - 1] > output[midpoint],
            "Timer should count up after reset"
        );
    }

    #[test]
    fn test_timer_multiple_resets() {
        // Test multiple resets in one block
        let size = 512;
        let sample_rate = 44100.0;

        // Create pulse train: trigger every 128 samples
        let mut trigger = vec![0.0; size];
        for i in (0..size).step_by(128) {
            if i < size {
                trigger[i] = 1.0;
            }
        }

        let inputs: Vec<&[f32]> = vec![&trigger];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut timer = TimerNode::new(0);
        timer.process_block(&inputs, &mut output, sample_rate, &context);

        // Check that timer resets at each pulse
        let dt = 1.0 / sample_rate;

        // After first pulse (index 0), timer resets
        assert!(
            output[0] < dt * 2.0,
            "Timer should reset at first pulse"
        );

        // Before second pulse (index 128), timer should have counted up
        assert!(
            output[127] > 0.001,
            "Timer should count up between pulses, got {}",
            output[127]
        );

        // At second pulse (index 128), timer resets
        assert!(
            output[128] < dt * 2.0,
            "Timer should reset at second pulse, got {}",
            output[128]
        );

        // Before third pulse, timer should count up again
        assert!(
            output[255] > 0.001,
            "Timer should count up after second reset"
        );
    }

    #[test]
    fn test_timer_no_reset_on_high_state() {
        // Test that timer does NOT reset if trigger stays high
        let size = 512;
        let sample_rate = 44100.0;

        // Trigger: all high (no rising edge)
        let trigger = vec![1.0; size];
        let inputs: Vec<&[f32]> = vec![&trigger];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut timer = TimerNode::new(0);

        // Pre-set last_trigger to high
        timer.state.last_trigger = 1.0;

        timer.process_block(&inputs, &mut output, sample_rate, &context);

        // Timer should count continuously (no resets)
        for i in 1..size {
            assert!(
                output[i] > output[i - 1],
                "Timer should increase continuously when trigger stays high"
            );
        }

        // Final value should be size * dt
        let expected = (size as f32) / sample_rate;
        assert!(
            (output[size - 1] - expected).abs() < 0.0001,
            "Timer should count continuously to {}, got {}",
            expected,
            output[size - 1]
        );
    }

    #[test]
    fn test_timer_edge_detection_threshold() {
        // Test that rising edge threshold is at 0.5
        let size = 512;
        let sample_rate = 44100.0;

        // Test values near threshold
        let test_cases = vec![
            (0.4, 0.6, true),   // Should trigger (crosses 0.5)
            (0.49, 0.51, true), // Should trigger (crosses 0.5)
            (0.3, 0.4, false),  // Should NOT trigger (below 0.5)
            (0.6, 0.7, false),  // Should NOT trigger (stays above 0.5)
        ];

        for (low_val, high_val, should_reset) in test_cases {
            let mut trigger = vec![low_val; size / 2];
            trigger.extend(vec![high_val; size / 2]);

            let inputs: Vec<&[f32]> = vec![&trigger];
            let mut output = vec![0.0; size];
            let context = create_context(size);

            let mut timer = TimerNode::new(0);
            timer.process_block(&inputs, &mut output, sample_rate, &context);

            let midpoint = size / 2;
            let dt = 1.0 / sample_rate;

            if should_reset {
                assert!(
                    output[midpoint] < dt * 2.0,
                    "Timer should reset when crossing 0.5 threshold ({} -> {}), got {}",
                    low_val,
                    high_val,
                    output[midpoint]
                );
            } else {
                assert!(
                    output[midpoint] > 0.001,
                    "Timer should NOT reset when not crossing threshold ({} -> {}), got {}",
                    low_val,
                    high_val,
                    output[midpoint]
                );
            }
        }
    }

    #[test]
    fn test_timer_node_interface() {
        // Test node getters
        let timer = TimerNode::new(5);

        assert_eq!(timer.trigger_input(), 5);

        let inputs = timer.input_nodes();
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0], 5);

        assert_eq!(timer.name(), "TimerNode");
    }

    #[test]
    fn test_timer_reset() {
        // Test that reset clears timer state
        let size = 512;
        let sample_rate = 44100.0;

        // No trigger
        let trigger = vec![0.0; size];
        let inputs: Vec<&[f32]> = vec![&trigger];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut timer = TimerNode::new(0);

        // Let timer count up
        timer.process_block(&inputs, &mut output, sample_rate, &context);
        let elapsed_before = timer.elapsed_time();
        assert!(elapsed_before > 0.0, "Timer should have counted up");

        // Reset
        timer.reset();
        assert_eq!(
            timer.elapsed_time(),
            0.0,
            "Timer should be 0 after reset"
        );
        assert_eq!(
            timer.state.last_trigger, 0.0,
            "Last trigger should be 0 after reset"
        );
    }

    #[test]
    fn test_timer_accuracy() {
        // Test timer accuracy over longer duration
        let size = 44100; // 1 second at 44.1kHz
        let sample_rate = 44100.0;

        // No trigger
        let trigger = vec![0.0; size];
        let inputs: Vec<&[f32]> = vec![&trigger];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut timer = TimerNode::new(0);
        timer.process_block(&inputs, &mut output, sample_rate, &context);

        // After 1 second (44100 samples), timer should read ~1.0 seconds
        let final_time = output[size - 1];
        assert!(
            (final_time - 1.0).abs() < 0.001,
            "After 1 second, timer should read ~1.0s, got {}",
            final_time
        );
    }
}
