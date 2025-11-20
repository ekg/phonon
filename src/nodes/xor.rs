/// Logical XOR node - exclusive OR operator for boolean signals
///
/// This node demonstrates exclusive OR logic for control flow and toggle operations.
/// Output[i] = 1.0 if exactly ONE of (Input_A[i] > threshold) OR (Input_B[i] > threshold), otherwise 0.0.
///
/// # Logic
/// Values greater than threshold are considered "true"
/// Values less than or equal to threshold are considered "false"
/// Only returns 1.0 when EXACTLY ONE input is true (not both, not neither)

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Logical XOR node: out = (a > threshold != b > threshold) ? 1.0 : 0.0
///
/// # Truth Table
/// ```text
/// a > threshold | b > threshold | output
/// --------------|---------------|-------
///     false     |     false     |  0.0
///     false     |     true      |  1.0
///     true      |     false     |  1.0
///     true      |     true      |  0.0
/// ```
///
/// # Example
/// ```ignore
/// // Toggle between two states
/// let state_a = ConstantNode::new(0.8);  // NodeId 0 (true)
/// let state_b = ConstantNode::new(0.3);  // NodeId 1 (false)
/// let toggle = XorNode::new(0, 1);       // NodeId 2
/// // Output will be 1.0 (exactly one is true)
/// ```
///
/// # Use Cases
///
/// **Toggle logic**:
/// ```ignore
/// // Alternate between two sources
/// let trigger_a = beat # gt 0.5;
/// let trigger_b = manual # gt 0.5;
/// let xor_gate = trigger_a # xor trigger_b;  // Only one at a time
/// ```
///
/// **Difference detection**:
/// ```ignore
/// // Detect when signals differ in state
/// let current_state = signal_a # gt 0.5;
/// let previous_state = signal_b # gt 0.5;
/// let changed = current_state # xor previous_state;  // 1.0 if state changed
/// ```
///
/// **Alternating gates**:
/// ```ignore
/// // Create alternating pattern
/// let even_beats = beat_counter % 2 # eq 0;
/// let odd_beats = beat_counter % 2 # eq 1;
/// let alternating = even_beats # xor odd_beats;  // Always 1.0, alternates source
/// ```
pub struct XorNode {
    input_a: NodeId,
    input_b: NodeId,
    threshold: f32,
}

impl XorNode {
    /// Create a new logical XOR node with default threshold (0.5)
    ///
    /// # Arguments
    /// * `input_a` - NodeId of first input
    /// * `input_b` - NodeId of second input
    ///
    /// # Default Threshold
    /// 0.5 - Values > 0.5 are considered "true"
    pub fn new(input_a: NodeId, input_b: NodeId) -> Self {
        Self {
            input_a,
            input_b,
            threshold: 0.5,
        }
    }

    /// Create a new logical XOR node with custom threshold
    ///
    /// # Arguments
    /// * `input_a` - NodeId of first input
    /// * `input_b` - NodeId of second input
    /// * `threshold` - Value above which signals are considered "true"
    pub fn with_threshold(input_a: NodeId, input_b: NodeId, threshold: f32) -> Self {
        Self {
            input_a,
            input_b,
            threshold,
        }
    }

    /// Get the first input node ID
    pub fn input_a(&self) -> NodeId {
        self.input_a
    }

    /// Get the second input node ID
    pub fn input_b(&self) -> NodeId {
        self.input_b
    }

    /// Get the threshold value
    pub fn threshold(&self) -> f32 {
        self.threshold
    }
}

impl AudioNode for XorNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "XorNode requires 2 inputs, got {}",
            inputs.len()
        );

        let buf_a = inputs[0];
        let buf_b = inputs[1];

        debug_assert_eq!(
            buf_a.len(),
            output.len(),
            "Input A length mismatch"
        );
        debug_assert_eq!(
            buf_b.len(),
            output.len(),
            "Input B length mismatch"
        );

        // Vectorized logical XOR
        for i in 0..output.len() {
            let a_true = buf_a[i] > self.threshold;
            let b_true = buf_b[i] > self.threshold;

            // XOR: true if exactly ONE input is true
            output[i] = if a_true != b_true { 1.0 } else { 0.0 };
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input_a, self.input_b]
    }

    fn name(&self) -> &str {
        "XorNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_xor_both_true() {
        let mut xor_node = XorNode::new(0, 1);

        // Both inputs > 0.5 (true)
        let input_a = vec![1.0, 0.8, 0.6, 1.0];
        let input_b = vec![1.0, 0.9, 0.7, 0.6];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        xor_node.process_block(&inputs, &mut output, 44100.0, &context);

        // All should be false (both true → XOR false)
        assert_eq!(output[0], 0.0);  // 1.0 XOR 1.0 → false
        assert_eq!(output[1], 0.0);  // 0.8 XOR 0.9 → false
        assert_eq!(output[2], 0.0);  // 0.6 XOR 0.7 → false
        assert_eq!(output[3], 0.0);  // 1.0 XOR 0.6 → false
    }

    #[test]
    fn test_xor_both_false() {
        let mut xor_node = XorNode::new(0, 1);

        // Both inputs <= 0.5 (false)
        let input_a = vec![0.0, 0.2, 0.4, 0.5];
        let input_b = vec![0.1, 0.3, 0.5, 0.0];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        xor_node.process_block(&inputs, &mut output, 44100.0, &context);

        // All should be false (both false → XOR false)
        assert_eq!(output[0], 0.0);  // 0.0 XOR 0.1 → false
        assert_eq!(output[1], 0.0);  // 0.2 XOR 0.3 → false
        assert_eq!(output[2], 0.0);  // 0.4 XOR 0.5 → false
        assert_eq!(output[3], 0.0);  // 0.5 XOR 0.0 → false
    }

    #[test]
    fn test_xor_first_true_second_false() {
        let mut xor_node = XorNode::new(0, 1);

        // First true, second false
        let input_a = vec![1.0, 0.8, 0.9, 0.6];
        let input_b = vec![0.2, 0.1, 0.3, 0.4];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        xor_node.process_block(&inputs, &mut output, 44100.0, &context);

        // All should be true (exactly one true → XOR true)
        assert_eq!(output[0], 1.0);  // 1.0 XOR 0.2 → true
        assert_eq!(output[1], 1.0);  // 0.8 XOR 0.1 → true
        assert_eq!(output[2], 1.0);  // 0.9 XOR 0.3 → true
        assert_eq!(output[3], 1.0);  // 0.6 XOR 0.4 → true
    }

    #[test]
    fn test_xor_first_false_second_true() {
        let mut xor_node = XorNode::new(0, 1);

        // First false, second true
        let input_a = vec![0.0, 0.2, 0.3, 0.4];
        let input_b = vec![1.0, 0.8, 0.9, 0.6];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        xor_node.process_block(&inputs, &mut output, 44100.0, &context);

        // All should be true (exactly one true → XOR true)
        assert_eq!(output[0], 1.0);  // 0.0 XOR 1.0 → true
        assert_eq!(output[1], 1.0);  // 0.2 XOR 0.8 → true
        assert_eq!(output[2], 1.0);  // 0.3 XOR 0.9 → true
        assert_eq!(output[3], 1.0);  // 0.4 XOR 0.6 → true
    }

    #[test]
    fn test_xor_threshold_boundary() {
        let mut xor_node = XorNode::new(0, 1);

        // Test exact threshold boundary (0.5)
        let input_a = vec![0.5, 0.50001, 0.49999, 0.6];
        let input_b = vec![0.6, 0.7, 0.8, 0.5];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        xor_node.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0);  // 0.5 <= 0.5 (false) XOR 0.6 > 0.5 (true) → true
        assert_eq!(output[1], 0.0);  // 0.50001 > 0.5 (true) XOR 0.7 > 0.5 (true) → false
        assert_eq!(output[2], 1.0);  // 0.49999 <= 0.5 (false) XOR 0.8 > 0.5 (true) → true
        assert_eq!(output[3], 1.0);  // 0.6 > 0.5 (true) XOR 0.5 <= 0.5 (false) → true
    }

    #[test]
    fn test_xor_custom_threshold() {
        let mut xor_node = XorNode::with_threshold(0, 1, 0.75);

        // Test with custom threshold 0.75
        let input_a = vec![0.8, 0.7, 0.9, 0.75];
        let input_b = vec![0.9, 0.8, 0.76, 0.8];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        xor_node.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0);  // 0.8 > 0.75 (true) XOR 0.9 > 0.75 (true) → false
        assert_eq!(output[1], 1.0);  // 0.7 <= 0.75 (false) XOR 0.8 > 0.75 (true) → true
        assert_eq!(output[2], 0.0);  // 0.9 > 0.75 (true) XOR 0.76 > 0.75 (true) → false
        assert_eq!(output[3], 1.0);  // 0.75 <= 0.75 (false) XOR 0.8 > 0.75 (true) → true
    }

    #[test]
    fn test_xor_negative_values() {
        let mut xor_node = XorNode::new(0, 1);

        // Test with negative values (all should be below threshold)
        let input_a = vec![-1.0, -0.5, -0.1, 0.2];
        let input_b = vec![-0.5, 1.0, 0.6, -0.3];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        xor_node.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0);  // -1.0 (false) XOR -0.5 (false) → false
        assert_eq!(output[1], 1.0);  // -0.5 (false) XOR 1.0 (true) → true
        assert_eq!(output[2], 1.0);  // -0.1 (false) XOR 0.6 (true) → true
        assert_eq!(output[3], 0.0);  // 0.2 (false) XOR -0.3 (false) → false
    }

    #[test]
    fn test_xor_with_constants() {
        let mut const_a = ConstantNode::new(0.8);
        let mut const_b = ConstantNode::new(0.9);
        let mut xor_node = XorNode::new(0, 1);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        // Process constants first
        let mut buf_a = vec![0.0; 512];
        let mut buf_b = vec![0.0; 512];

        const_a.process_block(&[], &mut buf_a, 44100.0, &context);
        const_b.process_block(&[], &mut buf_b, 44100.0, &context);

        // Now XOR them (0.8 > 0.5 XOR 0.9 > 0.5 → false, both true)
        let inputs = vec![buf_a.as_slice(), buf_b.as_slice()];
        let mut output = vec![0.0; 512];

        xor_node.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be 0.0 (both true → XOR false)
        for sample in &output {
            assert_eq!(*sample, 0.0);
        }
    }

    #[test]
    fn test_xor_entire_buffer() {
        let mut xor_node = XorNode::new(0, 1);

        // Test full buffer with alternating pattern
        let mut input_a = vec![0.0; 512];
        let mut input_b = vec![0.0; 512];

        // Create pattern where XOR alternates: 1, 0, 1, 0, ...
        for i in 0..512 {
            input_a[i] = if i % 2 == 0 { 1.0 } else { 0.0 };  // true, false, true, false
            input_b[i] = if i % 4 < 2 { 0.0 } else { 1.0 };   // false, false, true, true
        }

        let inputs = vec![input_a.as_slice(), input_b.as_slice()];
        let mut output = vec![0.0; 512];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        xor_node.process_block(&inputs, &mut output, 44100.0, &context);

        // Verify XOR logic: exactly one input true
        for i in 0..512 {
            let a_true = input_a[i] > 0.5;
            let b_true = input_b[i] > 0.5;
            let expected = if a_true != b_true { 1.0 } else { 0.0 };
            assert_eq!(output[i], expected, "Mismatch at sample {}", i);
        }
    }

    #[test]
    fn test_xor_dependencies() {
        let xor_node = XorNode::new(5, 10);
        let deps = xor_node.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 5);
        assert_eq!(deps[1], 10);
    }

    #[test]
    fn test_xor_getters() {
        let xor_node = XorNode::with_threshold(3, 7, 0.6);

        assert_eq!(xor_node.input_a(), 3);
        assert_eq!(xor_node.input_b(), 7);
        assert_eq!(xor_node.threshold(), 0.6);
    }
}
