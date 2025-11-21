/// Latch node - holds value until triggered (similar to sample & hold)
///
/// This node implements a rising-edge triggered latch:
/// - Monitors a trigger signal for rising edges (low to high transition)
/// - Captures the input signal's value when trigger crosses 0.5 threshold
/// - Holds that value until the next rising edge
///
/// Common uses:
/// - Sample-and-hold with gate signals
/// - Stepped modulation effects
/// - Rhythmic parameter automation
/// - Latching control values on specific events

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Latch node: captures input when trigger rises above 0.5 threshold
///
/// The algorithm:
/// ```text
/// trigger_high = current_trigger > 0.5
/// if trigger_high && !trigger_was_high:  // Rising edge
///     held_value = current_input
/// output = held_value
/// trigger_was_high = trigger_high
/// ```
///
/// # Example
/// ```ignore
/// // Latch random noise on a clock signal
/// let noise = WhiteNoiseNode::new();           // NodeId 0
/// let clock = OscillatorNode::new(...);        // NodeId 1 (square wave)
/// let latch = LatchNode::new(0, 1);           // NodeId 2
/// // Output will be stepped random values, updating on each clock rising edge
/// ```
pub struct LatchNode {
    /// Input signal to sample
    input: NodeId,

    /// Trigger signal (rising edge above 0.5 triggers latching)
    trigger_input: NodeId,

    /// Currently held value
    held_value: f32,

    /// Previous trigger state (for detecting rising edges)
    trigger_was_high: bool,
}

impl LatchNode {
    /// Latch - Rising-edge triggered sample-and-hold
    ///
    /// Captures input value when trigger signal crosses 0.5 threshold and holds it
    /// until next rising edge. Useful for stepped modulation and rhythmic automation.
    ///
    /// # Parameters
    /// - `input`: Signal value to sample
    /// - `trigger_input`: Trigger signal (rising edge > 0.5 latches)
    ///
    /// # Example
    /// ```phonon
    /// ~random: noise 1
    /// ~clock: "x ~ x ~" # bpm_to_impulse 120
    /// ~stepped: ~random # latch ~clock
    /// out: sine ~stepped * 440 * 0.5
    /// ```
    pub fn new(input: NodeId, trigger_input: NodeId) -> Self {
        Self {
            input,
            trigger_input,
            held_value: 0.0,
            trigger_was_high: false,
        }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }

    /// Get the trigger input node ID
    pub fn trigger_input(&self) -> NodeId {
        self.trigger_input
    }

    /// Get the current held value (for debugging/testing)
    pub fn held_value(&self) -> f32 {
        self.held_value
    }
}

impl AudioNode for LatchNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "LatchNode requires 2 inputs, got {}",
            inputs.len()
        );

        let input_buf = inputs[0];
        let trigger_buf = inputs[1];

        debug_assert_eq!(
            input_buf.len(),
            output.len(),
            "Input buffer length mismatch"
        );
        debug_assert_eq!(
            trigger_buf.len(),
            output.len(),
            "Trigger buffer length mismatch"
        );

        // Process each sample
        for i in 0..output.len() {
            let value = input_buf[i];
            let trigger = trigger_buf[i];

            let trigger_high = trigger > 0.5;

            // Detect rising edge: trigger goes from low to high
            if trigger_high && !self.trigger_was_high {
                // Latch new value
                self.held_value = value;
            }

            // Output the held value
            output[i] = self.held_value;

            // Update trigger state for next sample
            self.trigger_was_high = trigger_high;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.trigger_input]
    }

    fn name(&self) -> &str {
        "LatchNode"
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
    fn test_latch_holds_initial_zero() {
        let mut latch = LatchNode::new(0, 1);

        let input = vec![5.0, 6.0, 7.0, 8.0];
        let trigger = vec![0.0, 0.0, 0.0, 0.0]; // Never triggers
        let inputs = vec![input.as_slice(), trigger.as_slice()];

        let mut output = vec![0.0; 4];
        let context = create_context(4);

        latch.process_block(&inputs, &mut output, 44100.0, &context);

        // Should hold initial value of 0.0 (no triggers occurred)
        for sample in &output {
            assert_eq!(*sample, 0.0);
        }
    }

    #[test]
    fn test_latch_on_rising_edge() {
        let mut latch = LatchNode::new(0, 1);

        let input = vec![1.0, 2.0, 3.0, 4.0];
        let trigger = vec![0.0, 0.8, 0.3, 0.1]; // Rising edge at index 1
        let inputs = vec![input.as_slice(), trigger.as_slice()];

        let mut output = vec![0.0; 4];
        let context = create_context(4);

        latch.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0); // Initial held value
        assert_eq!(output[1], 2.0); // Latched at rising edge
        assert_eq!(output[2], 2.0); // Holding (trigger went low)
        assert_eq!(output[3], 2.0); // Still holding
    }

    #[test]
    fn test_latch_holds_value_while_high() {
        let mut latch = LatchNode::new(0, 1);

        let input = vec![10.0, 20.0, 30.0, 40.0];
        let trigger = vec![0.0, 1.0, 1.0, 1.0]; // Rising edge at 1, stays high
        let inputs = vec![input.as_slice(), trigger.as_slice()];

        let mut output = vec![0.0; 4];
        let context = create_context(4);

        latch.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0);  // Initial
        assert_eq!(output[1], 20.0); // Latched on rising edge
        assert_eq!(output[2], 20.0); // Holding (trigger still high, no edge)
        assert_eq!(output[3], 20.0); // Still holding
    }

    #[test]
    fn test_latch_holds_value_when_trigger_low() {
        let mut latch = LatchNode::new(0, 1);

        let input = vec![100.0, 200.0, 300.0, 400.0];
        let trigger = vec![0.0, 0.9, 0.2, 0.1]; // Rising edge at 1, then low
        let inputs = vec![input.as_slice(), trigger.as_slice()];

        let mut output = vec![0.0; 4];
        let context = create_context(4);

        latch.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0);   // Initial
        assert_eq!(output[1], 200.0); // Latched on rising edge
        assert_eq!(output[2], 200.0); // Holding (trigger went low)
        assert_eq!(output[3], 200.0); // Still holding
    }

    #[test]
    fn test_latch_multiple_triggers() {
        let mut latch = LatchNode::new(0, 1);

        let input = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0];
        let trigger = vec![0.0, 0.8, 0.2, 0.9, 0.1, 0.7];
        // Rising edges at index 1, 3, and 5
        let inputs = vec![input.as_slice(), trigger.as_slice()];

        let mut output = vec![0.0; 6];
        let context = create_context(6);

        latch.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0);  // Initial
        assert_eq!(output[1], 20.0); // First rising edge
        assert_eq!(output[2], 20.0); // Holding
        assert_eq!(output[3], 40.0); // Second rising edge
        assert_eq!(output[4], 40.0); // Holding
        assert_eq!(output[5], 60.0); // Third rising edge
    }

    #[test]
    fn test_latch_dependencies() {
        let latch = LatchNode::new(3, 7);
        let deps = latch.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 3);
        assert_eq!(deps[1], 7);
    }

    #[test]
    fn test_latch_with_constants() {
        let mut latch = LatchNode::new(0, 1);

        // Test with constant input value, pulsed trigger
        let input = vec![42.0, 42.0, 42.0, 42.0];
        let trigger = vec![0.0, 0.9, 0.1, 0.8]; // Rising edges at 1 and 3
        let inputs = vec![input.as_slice(), trigger.as_slice()];

        let mut output = vec![0.0; 4];
        let context = create_context(4);

        latch.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0);  // Initial
        assert_eq!(output[1], 42.0); // Latched
        assert_eq!(output[2], 42.0); // Holding
        assert_eq!(output[3], 42.0); // Re-latched same value
    }

    #[test]
    fn test_latch_getter_methods() {
        let latch = LatchNode::new(5, 10);

        assert_eq!(latch.input(), 5);
        assert_eq!(latch.trigger_input(), 10);
        assert_eq!(latch.held_value(), 0.0);
    }

    #[test]
    fn test_latch_state_persistence() {
        // Test that state persists across multiple process_block calls
        let mut latch = LatchNode::new(0, 1);

        // First block: trigger and capture value
        let input1 = vec![5.0, 5.0];
        let trigger1 = vec![0.0, 0.9];
        let inputs1 = vec![input1.as_slice(), trigger1.as_slice()];
        let mut output1 = vec![0.0; 2];
        let context = create_context(2);

        latch.process_block(&inputs1, &mut output1, 44100.0, &context);
        assert_eq!(output1[1], 5.0); // Captured 5.0

        // Second block: no trigger, different input
        let input2 = vec![100.0, 200.0];
        let trigger2 = vec![0.8, 0.3]; // Trigger high then low, no rising edge
        let inputs2 = vec![input2.as_slice(), trigger2.as_slice()];
        let mut output2 = vec![0.0; 2];

        latch.process_block(&inputs2, &mut output2, 44100.0, &context);

        // Should still hold 5.0 from previous block
        assert_eq!(output2[0], 5.0);
        assert_eq!(output2[1], 5.0);
    }

    #[test]
    fn test_latch_threshold_boundary() {
        let mut latch = LatchNode::new(0, 1);

        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let trigger = vec![0.5, 0.50001, 0.49999, 0.6, 0.4];
        // 0.5 is NOT > 0.5 (false)
        // 0.50001 > 0.5 (true) - rising edge!
        // 0.49999 is NOT > 0.5 (false)
        // 0.6 > 0.5 (true) - rising edge!
        // 0.4 is NOT > 0.5 (false)
        let inputs = vec![input.as_slice(), trigger.as_slice()];

        let mut output = vec![0.0; 5];
        let context = create_context(5);

        latch.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0); // Initial (0.5 is not > 0.5)
        assert_eq!(output[1], 2.0); // Rising edge (false -> true)
        assert_eq!(output[2], 2.0); // Holding
        assert_eq!(output[3], 4.0); // Rising edge (false -> true)
        assert_eq!(output[4], 4.0); // Holding
    }
}
