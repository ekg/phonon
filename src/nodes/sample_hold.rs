/// Sample-and-hold node - captures input when trigger crosses zero
///
/// This node implements classic analog-style sample-and-hold behavior:
/// - Monitors a trigger signal for zero crossings (negative to positive)
/// - Captures the input signal's value at the crossing point
/// - Holds that value until the next crossing
///
/// Common uses:
/// - Random voltage generation (sample noise on clock)
/// - Stepped modulation effects
/// - Rhythmic parameter automation
/// - Emulating classic analog synth S&H circuits

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Sample-and-hold node: captures input when trigger crosses from negative to positive
///
/// The algorithm:
/// ```text
/// if last_trigger <= 0.0 && current_trigger > 0.0:
///     held_value = current_input
/// output = held_value
/// last_trigger = current_trigger
/// ```
///
/// # Example
/// ```ignore
/// // Sample random noise on a clock signal
/// let noise = WhiteNoiseNode::new();           // NodeId 0
/// let clock = OscillatorNode::new(...);        // NodeId 1 (square wave)
/// let sample_hold = SampleAndHoldNode::new(0, 1);  // NodeId 2
/// // Output will be stepped random values, updating on each clock pulse
/// ```
pub struct SampleAndHoldNode {
    /// Input signal to sample
    input: NodeId,

    /// Trigger signal (crossing from negative to positive triggers sampling)
    trigger_input: NodeId,

    /// Currently held value
    held_value: f32,

    /// Previous trigger value (for detecting zero crossings)
    last_trigger: f32,
}

impl SampleAndHoldNode {
    /// SampleAndHoldNode - Analog-style sample-and-hold circuit
    ///
    /// Captures input signal's value when trigger crosses from negative to positive,
    /// holding that value until the next crossing. Classic for random stepped sequences.
    ///
    /// # Parameters
    /// - `input`: NodeId of signal to sample
    /// - `trigger_input`: NodeId of trigger signal (zero crossing triggers sampling)
    ///
    /// # Example
    /// ```phonon
    /// ~noise: random 1.0
    /// ~clock: square 4.0
    /// ~stepped: ~noise # sample_hold ~clock
    /// ```
    pub fn new(input: NodeId, trigger_input: NodeId) -> Self {
        Self {
            input,
            trigger_input,
            held_value: 0.0,
            last_trigger: 0.0,
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

impl AudioNode for SampleAndHoldNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "SampleAndHoldNode requires 2 inputs, got {}",
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
            let trigger = trigger_buf[i];
            let input = input_buf[i];

            // Detect zero crossing: trigger goes from negative (or zero) to positive
            if self.last_trigger <= 0.0 && trigger > 0.0 {
                // Sample the input
                self.held_value = input;
            }

            // Output the held value
            output[i] = self.held_value;

            // Update trigger state for next sample
            self.last_trigger = trigger;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.trigger_input]
    }

    fn name(&self) -> &str {
        "SampleAndHoldNode"
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
    fn test_sample_hold_basic_trigger() {
        let mut sh = SampleAndHoldNode::new(0, 1);

        let input = vec![1.0, 2.0, 3.0, 4.0];
        let trigger = vec![-1.0, 0.5, 0.3, -0.5]; // Crossing at index 1
        let inputs = vec![input.as_slice(), trigger.as_slice()];

        let mut output = vec![0.0; 4];
        let context = create_context(4);

        sh.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0); // Initial held value
        assert_eq!(output[1], 2.0); // Captured at crossing
        assert_eq!(output[2], 2.0); // Holding
        assert_eq!(output[3], 2.0); // Still holding
    }

    #[test]
    fn test_sample_hold_multiple_crossings() {
        let mut sh = SampleAndHoldNode::new(0, 1);

        let input = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0];
        let trigger = vec![-1.0, 0.5, -0.5, 0.8, 0.2, -0.1];
        // Crossings at index 1 and 3
        let inputs = vec![input.as_slice(), trigger.as_slice()];

        let mut output = vec![0.0; 6];
        let context = create_context(6);

        sh.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0);  // Initial
        assert_eq!(output[1], 20.0); // First crossing
        assert_eq!(output[2], 20.0); // Holding
        assert_eq!(output[3], 40.0); // Second crossing
        assert_eq!(output[4], 40.0); // Holding
        assert_eq!(output[5], 40.0); // Holding
    }

    #[test]
    fn test_sample_hold_no_crossing() {
        let mut sh = SampleAndHoldNode::new(0, 1);

        let input = vec![5.0, 6.0, 7.0, 8.0];
        let trigger = vec![0.5, 0.8, 0.3, 0.1]; // Always positive
        let inputs = vec![input.as_slice(), trigger.as_slice()];

        let mut output = vec![0.0; 4];
        let context = create_context(4);

        sh.process_block(&inputs, &mut output, 44100.0, &context);

        // First sample triggers (last_trigger starts at 0.0, 0.5 > 0.0)
        // So it captures 5.0 and holds it
        for sample in &output {
            assert_eq!(*sample, 5.0);
        }
    }

    #[test]
    fn test_sample_hold_exact_zero_crossing() {
        let mut sh = SampleAndHoldNode::new(0, 1);

        let input = vec![100.0, 200.0, 300.0];
        let trigger = vec![-0.5, 0.0, 0.5]; // Crosses at index 2 (0.0 is not > 0.0)
        let inputs = vec![input.as_slice(), trigger.as_slice()];

        let mut output = vec![0.0; 3];
        let context = create_context(3);

        sh.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0);   // Initial
        assert_eq!(output[1], 0.0);   // Still holding (0.0 is not > 0.0)
        assert_eq!(output[2], 300.0); // Crosses here (0.0 <= 0.0 && 0.5 > 0.0)
    }

    #[test]
    fn test_sample_hold_getter_methods() {
        let sh = SampleAndHoldNode::new(5, 10);

        assert_eq!(sh.input(), 5);
        assert_eq!(sh.trigger_input(), 10);
        assert_eq!(sh.held_value(), 0.0);
    }

    #[test]
    fn test_sample_hold_dependencies() {
        let sh = SampleAndHoldNode::new(3, 7);
        let deps = sh.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 3);
        assert_eq!(deps[1], 7);
    }

    #[test]
    fn test_sample_hold_state_persistence() {
        // Test that state persists across multiple process_block calls
        let mut sh = SampleAndHoldNode::new(0, 1);

        // First block: trigger and capture value
        let input1 = vec![5.0, 5.0];
        let trigger1 = vec![-1.0, 0.5];
        let inputs1 = vec![input1.as_slice(), trigger1.as_slice()];
        let mut output1 = vec![0.0; 2];
        let context = create_context(2);

        sh.process_block(&inputs1, &mut output1, 44100.0, &context);
        assert_eq!(output1[1], 5.0); // Captured 5.0

        // Second block: no trigger, different input
        let input2 = vec![100.0, 200.0];
        let trigger2 = vec![0.3, 0.1]; // No crossing
        let inputs2 = vec![input2.as_slice(), trigger2.as_slice()];
        let mut output2 = vec![0.0; 2];

        sh.process_block(&inputs2, &mut output2, 44100.0, &context);

        // Should still hold 5.0 from previous block
        assert_eq!(output2[0], 5.0);
        assert_eq!(output2[1], 5.0);
    }
}
