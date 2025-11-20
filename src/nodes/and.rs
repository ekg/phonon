/// Logical AND node - combines two boolean signals
///
/// This node demonstrates logical operations for control flow and gating.
/// Output[i] = 1.0 if both Input_A[i] > threshold AND Input_B[i] > threshold, otherwise 0.0.
///
/// # Logic
/// Values greater than threshold are considered "true"
/// Values less than or equal to threshold are considered "false"
/// Only returns 1.0 when BOTH inputs are true

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Logical AND node: out = (a > threshold && b > threshold) ? 1.0 : 0.0
///
/// # Example
/// ```ignore
/// // Gate signal only when both conditions are met
/// let cond_a = ConstantNode::new(0.8);  // NodeId 0 (true)
/// let cond_b = ConstantNode::new(0.6);  // NodeId 1 (true)
/// let gate = AndNode::new(0, 1);        // NodeId 2
/// // Output will be 1.0 (both > 0.5)
/// ```
///
/// # Use Cases
///
/// **Conditional processing**:
/// ```ignore
/// // Apply effect only when both conditions are met
/// let above_min = signal # gte 0.3;     // Signal above minimum?
/// let below_max = signal # lte 0.7;     // Signal below maximum?
/// let in_range = above_min # and below_max;  // Both conditions?
/// let output = signal * in_range;        // Gate the signal
/// ```
///
/// **Range detection**:
/// ```ignore
/// // Trigger only when value is in range [0.3, 0.7]
/// let check_low = signal # gt 0.3;
/// let check_high = signal # lt 0.7;
/// let trigger = check_low # and check_high;
/// ```
///
/// **Event gating**:
/// ```ignore
/// // Trigger only when beat AND velocity are high
/// let beat_gate = beat # gt 0.5;
/// let velocity_gate = velocity # gt 0.8;
/// let trigger = beat_gate # and velocity_gate;
/// ```
pub struct AndNode {
    input_a: NodeId,
    input_b: NodeId,
    threshold: f32,
}

impl AndNode {
    /// Create a new logical AND node with default threshold (0.5)
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

    /// Create a new logical AND node with custom threshold
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

impl AudioNode for AndNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "AndNode requires 2 inputs, got {}",
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

        // Vectorized logical AND
        for i in 0..output.len() {
            let a_true = buf_a[i] > self.threshold;
            let b_true = buf_b[i] > self.threshold;

            output[i] = if a_true && b_true { 1.0 } else { 0.0 };
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input_a, self.input_b]
    }

    fn name(&self) -> &str {
        "AndNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_and_both_true() {
        let mut and_node = AndNode::new(0, 1);

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

        and_node.process_block(&inputs, &mut output, 44100.0, &context);

        // All should be true (1.0 && 1.0, 0.8 && 0.9, 0.6 && 0.7, 1.0 && 0.6)
        assert_eq!(output[0], 1.0);
        assert_eq!(output[1], 1.0);
        assert_eq!(output[2], 1.0);
        assert_eq!(output[3], 1.0);
    }

    #[test]
    fn test_and_both_false() {
        let mut and_node = AndNode::new(0, 1);

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

        and_node.process_block(&inputs, &mut output, 44100.0, &context);

        // All should be false (0.0 && 0.1, 0.2 && 0.3, 0.4 && 0.5, 0.5 && 0.0)
        assert_eq!(output[0], 0.0);
        assert_eq!(output[1], 0.0);
        assert_eq!(output[2], 0.0);
        assert_eq!(output[3], 0.0);
    }

    #[test]
    fn test_and_one_true_one_false() {
        let mut and_node = AndNode::new(0, 1);

        // Mixed: first true, second false
        let input_a = vec![1.0, 0.8, 0.2, 0.6];
        let input_b = vec![0.2, 0.1, 0.9, 0.3];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        and_node.process_block(&inputs, &mut output, 44100.0, &context);

        // (1.0 && 0.2) = false, (0.8 && 0.1) = false, (0.2 && 0.9) = false, (0.6 && 0.3) = false
        assert_eq!(output[0], 0.0);  // true && false
        assert_eq!(output[1], 0.0);  // true && false
        assert_eq!(output[2], 0.0);  // false && true
        assert_eq!(output[3], 0.0);  // true && false
    }

    #[test]
    fn test_and_threshold_boundary() {
        let mut and_node = AndNode::new(0, 1);

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

        and_node.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0);  // 0.5 <= 0.5 (false) && 0.6 > 0.5 (true) = false
        assert_eq!(output[1], 1.0);  // 0.50001 > 0.5 (true) && 0.7 > 0.5 (true) = true
        assert_eq!(output[2], 0.0);  // 0.49999 <= 0.5 (false) && 0.8 > 0.5 (true) = false
        assert_eq!(output[3], 0.0);  // 0.6 > 0.5 (true) && 0.5 <= 0.5 (false) = false
    }

    #[test]
    fn test_and_negative_values() {
        let mut and_node = AndNode::new(0, 1);

        // Test with negative values (all should be false)
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

        and_node.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0);  // -1.0 <= 0.5 (false) && -0.5 <= 0.5 (false) = false
        assert_eq!(output[1], 0.0);  // -0.5 <= 0.5 (false) && 1.0 > 0.5 (true) = false
        assert_eq!(output[2], 0.0);  // -0.1 <= 0.5 (false) && 0.6 > 0.5 (true) = false
        assert_eq!(output[3], 0.0);  // 0.2 <= 0.5 (false) && -0.3 <= 0.5 (false) = false
    }

    #[test]
    fn test_and_custom_threshold() {
        let mut and_node = AndNode::with_threshold(0, 1, 0.75);

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

        and_node.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0);  // 0.8 > 0.75 && 0.9 > 0.75 = true
        assert_eq!(output[1], 0.0);  // 0.7 <= 0.75 (false) && 0.8 > 0.75 (true) = false
        assert_eq!(output[2], 1.0);  // 0.9 > 0.75 && 0.76 > 0.75 = true
        assert_eq!(output[3], 0.0);  // 0.75 <= 0.75 (false) && 0.8 > 0.75 (true) = false
    }

    #[test]
    fn test_and_with_constants() {
        let mut const_a = ConstantNode::new(0.8);
        let mut const_b = ConstantNode::new(0.9);
        let mut and_node = AndNode::new(0, 1);

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

        // Now AND them (0.8 > 0.5 && 0.9 > 0.5 → true)
        let inputs = vec![buf_a.as_slice(), buf_b.as_slice()];
        let mut output = vec![0.0; 512];

        and_node.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be 1.0 (both true)
        for sample in &output {
            assert_eq!(*sample, 1.0);
        }
    }

    #[test]
    fn test_and_entire_buffer() {
        let mut and_node = AndNode::new(0, 1);

        // Test full buffer with pattern
        let mut input_a = vec![0.0; 512];
        let mut input_b = vec![0.0; 512];

        // Create alternating pattern
        for i in 0..512 {
            input_a[i] = if i % 4 < 2 { 1.0 } else { 0.0 };
            input_b[i] = if i % 4 == 0 || i % 4 == 1 { 1.0 } else { 0.0 };
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

        and_node.process_block(&inputs, &mut output, 44100.0, &context);

        // Verify pattern: both true only when i % 4 < 2
        for i in 0..512 {
            let expected = if i % 4 < 2 { 1.0 } else { 0.0 };
            assert_eq!(output[i], expected, "Mismatch at sample {}", i);
        }
    }

    #[test]
    fn test_and_dependencies() {
        let and_node = AndNode::new(5, 10);
        let deps = and_node.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 5);
        assert_eq!(deps[1], 10);
    }

    #[test]
    fn test_and_getters() {
        let and_node = AndNode::with_threshold(3, 7, 0.6);

        assert_eq!(and_node.input_a(), 3);
        assert_eq!(and_node.input_b(), 7);
        assert_eq!(and_node.threshold(), 0.6);
    }
}
