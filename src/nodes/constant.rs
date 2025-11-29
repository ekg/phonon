/// Constant value node - outputs a fixed value
///
/// This is the simplest possible AudioNode. It fills the output buffer
/// with a constant value (e.g., 440.0 for frequency, 0.5 for gain).
use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Constant value node
///
/// # Example
/// ```ignore
/// // Output 440.0 (frequency for A4 note)
/// let node = ConstantNode::new(440.0);
/// ```
pub struct ConstantNode {
    value: f32,
}

impl ConstantNode {
    /// Create a new constant value node
    pub fn new(value: f32) -> Self {
        Self { value }
    }

    /// Get the constant value
    pub fn value(&self) -> f32 {
        self.value
    }

    /// Set a new constant value
    pub fn set_value(&mut self, value: f32) {
        self.value = value;
    }
}

impl AudioNode for ConstantNode {
    fn process_block(
        &mut self,
        _inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        // Fill entire buffer with constant value
        output.iter_mut().for_each(|x| *x = self.value);
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![] // No dependencies (source node)
    }

    fn name(&self) -> &str {
        "ConstantNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::Fraction;

    #[test]
    fn test_constant_node_output() {
        let mut node = ConstantNode::new(440.0);
        let mut output = vec![0.0; 512];

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        node.process_block(&[], &mut output, 44100.0, &context);

        // Every sample should be 440.0
        for sample in &output {
            assert_eq!(*sample, 440.0);
        }
    }

    #[test]
    fn test_constant_node_set_value() {
        let mut node = ConstantNode::new(100.0);
        assert_eq!(node.value(), 100.0);

        node.set_value(200.0);
        assert_eq!(node.value(), 200.0);

        let mut output = vec![0.0; 512];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        node.process_block(&[], &mut output, 44100.0, &context);

        for sample in &output {
            assert_eq!(*sample, 200.0);
        }
    }

    #[test]
    fn test_constant_node_no_dependencies() {
        let node = ConstantNode::new(1.0);
        assert_eq!(node.input_nodes().len(), 0);
    }
}
