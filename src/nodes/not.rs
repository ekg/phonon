/// Logical NOT node - inverts boolean signal
///
/// This node demonstrates logical negation for control flow and gating.
/// Output[i] = 1.0 if Input[i] <= threshold, otherwise 0.0.
///
/// # Logic
/// Values greater than threshold are considered "true" → inverted to 0.0
/// Values less than or equal to threshold are considered "false" → inverted to 1.0
///
/// # Important
/// This is NOT the same as phase inversion (InvertNode)!
/// - NotNode: Boolean logic (true→false, false→true)
/// - InvertNode: Audio phase inversion (multiply by -1)
use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Logical NOT node: out = (input > threshold) ? 0.0 : 1.0
///
/// # Example
/// ```ignore
/// // Invert a gate signal
/// let gate = ConstantNode::new(0.8);  // NodeId 0 (true)
/// let not = NotNode::new(0);          // NodeId 1
/// // Output will be 0.0 (true inverted to false)
/// ```
///
/// # Use Cases
///
/// **Inverting conditions**:
/// ```ignore
/// // Apply effect when signal is NOT in range
/// let in_range = signal # gt 0.3 # and (signal # lt 0.7);
/// let not_in_range = in_range # not;  // Invert the condition
/// let output = signal * not_in_range;  // Gate when out of range
/// ```
///
/// **Complement gating**:
/// ```ignore
/// // Route signal based on condition
/// let condition = lfo # gt 0.5;
/// let not_condition = condition # not;
///
/// let path_a = signal_a * condition;      // When LFO high
/// let path_b = signal_b * not_condition;  // When LFO low
/// let output = path_a + path_b;            // Crossfade
/// ```
///
/// **Logic building**:
/// ```ignore
/// // NAND gate (NOT AND)
/// let and_result = input_a # and input_b;
/// let nand = and_result # not;
///
/// // NOR gate (NOT OR)
/// let or_result = input_a # or input_b;
/// let nor = or_result # not;
/// ```
///
/// **Event inverting**:
/// ```ignore
/// // Trigger on "off-beats" (when trigger is NOT active)
/// let trigger = beat # gt 0.5;
/// let off_beat = trigger # not;
/// let hi_hat = hi_hat_sample * off_beat;
/// ```
pub struct NotNode {
    input: NodeId,
    threshold: f32,
}

impl NotNode {
    /// NotNode - Logical NOT operation with configurable threshold
    ///
    /// Inverts boolean logic: outputs 1.0 for values below threshold, 0.0 for values
    /// above. Used in pattern logic and conditional signal routing.
    ///
    /// # Parameters
    /// - `input`: NodeId of the input signal (threshold: 0.5, >0.5 = true)
    ///
    /// # Example
    /// ```phonon
    /// ~gate: 0.7
    /// ~inverted: not ~gate
    /// ```
    pub fn new(input: NodeId) -> Self {
        Self {
            input,
            threshold: 0.5,
        }
    }

    /// Create a new logical NOT node with custom threshold
    ///
    /// # Arguments
    /// * `input` - NodeId of the input signal
    /// * `threshold` - Value above which signals are considered "true"
    pub fn with_threshold(input: NodeId, threshold: f32) -> Self {
        Self { input, threshold }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }

    /// Get the threshold value
    pub fn threshold(&self) -> f32 {
        self.threshold
    }
}

impl AudioNode for NotNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 1,
            "NotNode requires 1 input, got {}",
            inputs.len()
        );

        let input_buf = inputs[0];

        debug_assert_eq!(input_buf.len(), output.len(), "Input length mismatch");

        // Logical NOT: invert boolean value
        for i in 0..output.len() {
            let input_true = input_buf[i] > self.threshold;
            output[i] = if input_true { 0.0 } else { 1.0 };
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input]
    }

    fn name(&self) -> &str {
        "NotNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_not_true_becomes_false() {
        let mut not_node = NotNode::new(0);

        // Input values > 0.5 (true)
        let input = vec![1.0, 0.8, 0.6, 0.9];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        not_node.process_block(&inputs, &mut output, 44100.0, &context);

        // All true inputs should become false (0.0)
        assert_eq!(output[0], 0.0);
        assert_eq!(output[1], 0.0);
        assert_eq!(output[2], 0.0);
        assert_eq!(output[3], 0.0);
    }

    #[test]
    fn test_not_false_becomes_true() {
        let mut not_node = NotNode::new(0);

        // Input values <= 0.5 (false)
        let input = vec![0.0, 0.2, 0.4, 0.5];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        not_node.process_block(&inputs, &mut output, 44100.0, &context);

        // All false inputs should become true (1.0)
        assert_eq!(output[0], 1.0);
        assert_eq!(output[1], 1.0);
        assert_eq!(output[2], 1.0);
        assert_eq!(output[3], 1.0);
    }

    #[test]
    fn test_not_threshold_boundary() {
        let mut not_node = NotNode::new(0);

        // Test exact threshold boundary (0.5)
        let input = vec![0.5, 0.50001, 0.49999, 0.5000001];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        not_node.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0); // 0.5 <= 0.5 (false) → 1.0
        assert_eq!(output[1], 0.0); // 0.50001 > 0.5 (true) → 0.0
        assert_eq!(output[2], 1.0); // 0.49999 <= 0.5 (false) → 1.0
        assert_eq!(output[3], 0.0); // 0.5000001 > 0.5 (true) → 0.0
    }

    #[test]
    fn test_not_custom_threshold() {
        let mut not_node = NotNode::with_threshold(0, 0.75);

        // Test with custom threshold 0.75
        let input = vec![0.8, 0.7, 0.9, 0.75, 0.76];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 5];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 5, 2.0, 44100.0);

        not_node.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0); // 0.8 > 0.75 (true) → 0.0
        assert_eq!(output[1], 1.0); // 0.7 <= 0.75 (false) → 1.0
        assert_eq!(output[2], 0.0); // 0.9 > 0.75 (true) → 0.0
        assert_eq!(output[3], 1.0); // 0.75 <= 0.75 (false) → 1.0
        assert_eq!(output[4], 0.0); // 0.76 > 0.75 (true) → 0.0
    }

    #[test]
    fn test_not_entire_buffer() {
        let mut not_node = NotNode::new(0);

        // Test full buffer with pattern
        let mut input = vec![0.0; 512];

        // Create alternating pattern: true, true, false, false
        for i in 0..512 {
            input[i] = if i % 4 < 2 { 1.0 } else { 0.0 };
        }

        let inputs = vec![input.as_slice()];
        let mut output = vec![0.0; 512];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        not_node.process_block(&inputs, &mut output, 44100.0, &context);

        // Verify pattern: inverted (false, false, true, true)
        for i in 0..512 {
            let expected = if i % 4 < 2 { 0.0 } else { 1.0 };
            assert_eq!(output[i], expected, "Mismatch at sample {}", i);
        }
    }

    #[test]
    fn test_not_with_constants() {
        let mut const_true = ConstantNode::new(0.8);
        let mut const_false = ConstantNode::new(0.3);
        let mut not_node = NotNode::new(0);

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        // Test with true constant (0.8)
        let mut buf_true = vec![0.0; 512];
        const_true.process_block(&[], &mut buf_true, 44100.0, &context);

        let inputs_true = vec![buf_true.as_slice()];
        let mut output_true = vec![0.0; 512];
        not_node.process_block(&inputs_true, &mut output_true, 44100.0, &context);

        // Every sample should be 0.0 (NOT true = false)
        for sample in &output_true {
            assert_eq!(*sample, 0.0);
        }

        // Test with false constant (0.3)
        let mut buf_false = vec![0.0; 512];
        const_false.process_block(&[], &mut buf_false, 44100.0, &context);

        let inputs_false = vec![buf_false.as_slice()];
        let mut output_false = vec![0.0; 512];
        not_node.process_block(&inputs_false, &mut output_false, 44100.0, &context);

        // Every sample should be 1.0 (NOT false = true)
        for sample in &output_false {
            assert_eq!(*sample, 1.0);
        }
    }

    #[test]
    fn test_not_dependencies() {
        let not_node = NotNode::new(42);
        let deps = not_node.input_nodes();

        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], 42);
    }

    #[test]
    fn test_not_negative_values() {
        let mut not_node = NotNode::new(0);

        // Negative values are all <= 0.5, so all false → should become true
        let input = vec![-1.0, -0.5, -0.1, -100.0];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        not_node.process_block(&inputs, &mut output, 44100.0, &context);

        // All negative values are false → NOT false = true
        assert_eq!(output[0], 1.0);
        assert_eq!(output[1], 1.0);
        assert_eq!(output[2], 1.0);
        assert_eq!(output[3], 1.0);
    }

    #[test]
    fn test_not_double_inversion() {
        let mut not1 = NotNode::new(0);
        let mut not2 = NotNode::new(1);

        // Original signal (mixed true/false)
        let input = vec![1.0, 0.2, 0.8, 0.4];
        let inputs1 = vec![input.as_slice()];

        let mut buf1 = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        // First NOT
        not1.process_block(&inputs1, &mut buf1, 44100.0, &context);

        // Second NOT
        let inputs2 = vec![buf1.as_slice()];
        let mut buf2 = vec![0.0; 4];
        not2.process_block(&inputs2, &mut buf2, 44100.0, &context);

        // Double NOT should restore original boolean values
        // 1.0 (true) → 0.0 (false) → 1.0 (true)
        assert_eq!(buf2[0], 1.0);
        // 0.2 (false) → 1.0 (true) → 0.0 (false)
        assert_eq!(buf2[1], 0.0);
        // 0.8 (true) → 0.0 (false) → 1.0 (true)
        assert_eq!(buf2[2], 1.0);
        // 0.4 (false) → 1.0 (true) → 0.0 (false)
        assert_eq!(buf2[3], 0.0);
    }

    #[test]
    fn test_not_getters() {
        let not_node = NotNode::with_threshold(7, 0.6);

        assert_eq!(not_node.input(), 7);
        assert_eq!(not_node.threshold(), 0.6);
    }

    #[test]
    fn test_not_mixed_values() {
        let mut not_node = NotNode::new(0);

        // Mix of values above and below threshold
        let input = vec![1.0, 0.1, 0.9, 0.0, 0.5, 0.6, 0.49, 0.51];
        let inputs = vec![input.as_slice()];

        let mut output = vec![0.0; 8];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 8, 2.0, 44100.0);

        not_node.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0); // 1.0 > 0.5 (true) → 0.0
        assert_eq!(output[1], 1.0); // 0.1 <= 0.5 (false) → 1.0
        assert_eq!(output[2], 0.0); // 0.9 > 0.5 (true) → 0.0
        assert_eq!(output[3], 1.0); // 0.0 <= 0.5 (false) → 1.0
        assert_eq!(output[4], 1.0); // 0.5 <= 0.5 (false) → 1.0
        assert_eq!(output[5], 0.0); // 0.6 > 0.5 (true) → 0.0
        assert_eq!(output[6], 1.0); // 0.49 <= 0.5 (false) → 1.0
        assert_eq!(output[7], 0.0); // 0.51 > 0.5 (true) → 0.0
    }
}
