/// Logical OR node - compares two input signals against a threshold
///
/// This node demonstrates threshold-based logic and boolean signal generation.
/// Output[i] = 1.0 if (Input_A[i] > threshold) OR (Input_B[i] > threshold), otherwise 0.0.
///
/// The OR operator is inclusive - either input (or both) can be above threshold to
/// produce a true (1.0) output. This is useful for combining alternative conditions,
/// creating fallback logic, and merging trigger sources.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Logical OR node: out = (a > threshold || b > threshold) ? 1.0 : 0.0
///
/// # Truth Table
/// ```text
/// a > threshold | b > threshold | output
/// --------------|---------------|-------
///     false     |     false     |  0.0
///     false     |     true      |  1.0
///     true      |     false     |  1.0
///     true      |     true      |  1.0
/// ```
///
/// # Example
/// ```ignore
/// // Combine two trigger sources
/// let trigger_a = /* some pattern */;  // NodeId 0
/// let trigger_b = /* some pattern */;  // NodeId 1
/// let or = OrNode::new(0, 1);          // NodeId 2
/// // Output will be 1.0 if either trigger is active
/// ```
///
/// # Musical Use Cases
///
/// **Alternative triggers**:
/// ```ignore
/// // Trigger from either source
/// let kick_pattern = /* kick pattern */ # gt 0.5;
/// let manual_trigger = /* manual input */ # gt 0.5;
/// let combined = kick_pattern # or manual_trigger;
/// let envelope = combined # adsr 0.01 0.1 0.5 0.2;
/// ```
///
/// **Fallback logic**:
/// ```ignore
/// // Use signal A if available, otherwise signal B
/// let a_active = signal_a # gt 0.01;
/// let b_active = signal_b # gt 0.01;
/// let any_active = a_active # or b_active;
/// ```
pub struct OrNode {
    input_a: NodeId,
    input_b: NodeId,
    threshold: f32,
}

impl OrNode {
    /// OrNode - Logical OR operation combining two signals
    ///
    /// Outputs 1.0 when either input is above threshold (0.5), 0.0 otherwise.
    /// Used for combining gate signals and pattern logic in synthesis.
    ///
    /// # Parameters
    /// - `input_a`: NodeId of first signal (threshold: 0.5)
    /// - `input_b`: NodeId of second signal (threshold: 0.5)
    ///
    /// # Example
    /// ```phonon
    /// ~gate1: 0.7
    /// ~gate2: 0.2
    /// ~combined: or ~gate1 ~gate2
    /// ```
    pub fn new(input_a: NodeId, input_b: NodeId) -> Self {
        Self {
            input_a,
            input_b,
            threshold: 0.5,
        }
    }

    /// Create a new logical OR node with custom threshold
    ///
    /// # Arguments
    /// * `input_a` - NodeId of first input
    /// * `input_b` - NodeId of second input
    /// * `threshold` - Custom threshold value
    ///
    /// # Example
    /// ```ignore
    /// // Gate opens if either signal exceeds 0.8
    /// let or = OrNode::with_threshold(0, 1, 0.8);
    /// ```
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

impl AudioNode for OrNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "OrNode requires 2 inputs, got {}",
            inputs.len()
        );

        let buf_a = inputs[0];
        let buf_b = inputs[1];

        debug_assert_eq!(buf_a.len(), output.len(), "Input A length mismatch");
        debug_assert_eq!(buf_b.len(), output.len(), "Input B length mismatch");

        // Vectorized logical OR with threshold comparison
        for i in 0..output.len() {
            let a_true = buf_a[i] > self.threshold;
            let b_true = buf_b[i] > self.threshold;

            output[i] = if a_true || b_true { 1.0 } else { 0.0 };
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input_a, self.input_b]
    }

    fn name(&self) -> &str {
        "OrNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_or_both_true() {
        let mut or = OrNode::new(0, 1);

        // Both inputs above threshold (> 0.5)
        let input_a = vec![1.0, 0.8, 0.9, 0.7];
        let input_b = vec![1.0, 0.6, 0.8, 0.9];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        or.process_block(&inputs, &mut output, 44100.0, &context);

        // All should be true (1.0) since both inputs are > 0.5
        assert_eq!(output[0], 1.0);  // 1.0 || 1.0 → true
        assert_eq!(output[1], 1.0);  // 0.8 || 0.6 → true
        assert_eq!(output[2], 1.0);  // 0.9 || 0.8 → true
        assert_eq!(output[3], 1.0);  // 0.7 || 0.9 → true
    }

    #[test]
    fn test_or_both_false() {
        let mut or = OrNode::new(0, 1);

        // Both inputs below threshold (<= 0.5)
        let input_a = vec![0.0, 0.1, 0.3, 0.5];
        let input_b = vec![0.0, 0.2, 0.4, 0.5];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        or.process_block(&inputs, &mut output, 44100.0, &context);

        // All should be false (0.0) since both inputs are <= 0.5
        assert_eq!(output[0], 0.0);  // 0.0 || 0.0 → false
        assert_eq!(output[1], 0.0);  // 0.1 || 0.2 → false
        assert_eq!(output[2], 0.0);  // 0.3 || 0.4 → false
        assert_eq!(output[3], 0.0);  // 0.5 || 0.5 → false (not > 0.5)
    }

    #[test]
    fn test_or_first_true_second_false() {
        let mut or = OrNode::new(0, 1);

        // First input true, second false
        let input_a = vec![1.0, 0.8, 0.9, 0.6];
        let input_b = vec![0.0, 0.2, 0.3, 0.4];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        or.process_block(&inputs, &mut output, 44100.0, &context);

        // All should be true (1.0) since first input is > 0.5
        assert_eq!(output[0], 1.0);  // 1.0 || 0.0 → true
        assert_eq!(output[1], 1.0);  // 0.8 || 0.2 → true
        assert_eq!(output[2], 1.0);  // 0.9 || 0.3 → true
        assert_eq!(output[3], 1.0);  // 0.6 || 0.4 → true
    }

    #[test]
    fn test_or_first_false_second_true() {
        let mut or = OrNode::new(0, 1);

        // First input false, second true
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

        or.process_block(&inputs, &mut output, 44100.0, &context);

        // All should be true (1.0) since second input is > 0.5
        assert_eq!(output[0], 1.0);  // 0.0 || 1.0 → true
        assert_eq!(output[1], 1.0);  // 0.2 || 0.8 → true
        assert_eq!(output[2], 1.0);  // 0.3 || 0.9 → true
        assert_eq!(output[3], 1.0);  // 0.4 || 0.6 → true
    }

    #[test]
    fn test_or_mixed_values() {
        let mut or = OrNode::new(0, 1);

        // Mix of true and false cases
        let input_a = vec![1.0, 0.3, 0.2, 0.8];
        let input_b = vec![0.2, 0.7, 0.4, 0.9];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        or.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 1.0);  // 1.0 || 0.2 → true (first > 0.5)
        assert_eq!(output[1], 1.0);  // 0.3 || 0.7 → true (second > 0.5)
        assert_eq!(output[2], 0.0);  // 0.2 || 0.4 → false (both <= 0.5)
        assert_eq!(output[3], 1.0);  // 0.8 || 0.9 → true (both > 0.5)
    }

    #[test]
    fn test_or_with_constants() {
        let mut const_a = ConstantNode::new(0.8);  // Above threshold
        let mut const_b = ConstantNode::new(0.3);  // Below threshold
        let mut or = OrNode::new(0, 1);

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

        // Now OR them (0.8 > 0.5 || 0.3 > 0.5 → true)
        let inputs = vec![buf_a.as_slice(), buf_b.as_slice()];
        let mut output = vec![0.0; 512];

        or.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be 1.0 (first input is > 0.5)
        for sample in &output {
            assert_eq!(*sample, 1.0);
        }
    }

    #[test]
    fn test_or_custom_threshold() {
        let mut or = OrNode::with_threshold(0, 1, 0.8);

        // Test with custom threshold of 0.8
        let input_a = vec![0.5, 0.85, 0.7, 0.9];
        let input_b = vec![0.5, 0.6, 0.85, 0.95];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        or.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0);  // 0.5 || 0.5 → false (both <= 0.8)
        assert_eq!(output[1], 1.0);  // 0.85 || 0.6 → true (first > 0.8)
        assert_eq!(output[2], 1.0);  // 0.7 || 0.85 → true (second > 0.8)
        assert_eq!(output[3], 1.0);  // 0.9 || 0.95 → true (both > 0.8)
    }

    #[test]
    fn test_or_negative_values() {
        let mut or = OrNode::new(0, 1);

        // Test with negative values (all below threshold 0.5)
        let input_a = vec![-1.0, -0.5, 0.0, 0.4];
        let input_b = vec![-2.0, -0.3, 0.1, 0.2];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        or.process_block(&inputs, &mut output, 44100.0, &context);

        // All should be false (0.0) since both are <= 0.5
        assert_eq!(output[0], 0.0);  // -1.0 || -2.0 → false
        assert_eq!(output[1], 0.0);  // -0.5 || -0.3 → false
        assert_eq!(output[2], 0.0);  // 0.0 || 0.1 → false
        assert_eq!(output[3], 0.0);  // 0.4 || 0.2 → false
    }

    #[test]
    fn test_or_entire_buffer() {
        let mut or = OrNode::new(0, 1);

        // Test with typical buffer size
        let size = 512;
        let mut input_a = vec![0.0; size];
        let mut input_b = vec![0.0; size];

        // First half: only A is true
        for i in 0..size / 2 {
            input_a[i] = 1.0;
            input_b[i] = 0.0;
        }

        // Second half: only B is true
        for i in size / 2..size {
            input_a[i] = 0.0;
            input_b[i] = 1.0;
        }

        let inputs = vec![input_a.as_slice(), input_b.as_slice()];
        let mut output = vec![0.0; size];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            size,
            2.0,
            44100.0,
        );

        or.process_block(&inputs, &mut output, 44100.0, &context);

        // All should be true (1.0) since at least one input is always > 0.5
        for sample in &output {
            assert_eq!(*sample, 1.0);
        }
    }

    #[test]
    fn test_or_dependencies() {
        let or = OrNode::new(5, 10);
        let deps = or.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 5);
        assert_eq!(deps[1], 10);
    }

    #[test]
    fn test_or_threshold_edge_cases() {
        let mut or = OrNode::with_threshold(0, 1, 0.0);

        // With threshold 0.0, any positive value should be true
        let input_a = vec![0.0, 0.001, -0.001, 0.1];
        let input_b = vec![-0.1, 0.0, 0.002, -0.5];
        let inputs = vec![input_a.as_slice(), input_b.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        or.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], 0.0);  // 0.0 || -0.1 → false (both <= 0.0)
        assert_eq!(output[1], 1.0);  // 0.001 || 0.0 → true (first > 0.0)
        assert_eq!(output[2], 1.0);  // -0.001 || 0.002 → true (second > 0.0)
        assert_eq!(output[3], 1.0);  // 0.1 || -0.5 → true (first > 0.0)
    }
}
