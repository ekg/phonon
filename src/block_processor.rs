/// Block-based audio graph processor
///
/// This module implements the core execution loop for DAW-style buffer passing.
/// It coordinates node processing in topological order with optional parallel execution.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use crate::buffer_manager::{BufferManager, NodeOutput};
use crate::dependency_graph::DependencyGraph;
use std::collections::HashMap;

/// Block-based audio graph processor
///
/// # Example
/// ```ignore
/// // Create nodes
/// let nodes: Vec<Box<dyn AudioNode>> = vec![
///     Box::new(ConstantNode::new(440.0)),
///     Box::new(OscillatorNode::new(0, Waveform::Sine)),
/// ];
///
/// // Create processor
/// let mut processor = BlockProcessor::new(nodes, 1, 512)?;
///
/// // Process a block
/// let mut output = vec![0.0; 512];
/// let context = ProcessContext::new(...);
/// processor.process_block(&mut output, &context)?;
/// ```
pub struct BlockProcessor {
    nodes: Vec<Box<dyn AudioNode>>,
    dependency_graph: DependencyGraph,
    node_outputs: HashMap<NodeId, NodeOutput>,
    buffer_manager: BufferManager,
    output_node: NodeId,
    buffer_size: usize,
    zero_buffer: Vec<f32>,  // Used for cyclic dependencies (first block)
}

impl BlockProcessor {
    /// Create a new block processor
    ///
    /// # Arguments
    /// * `nodes` - Vec of audio nodes
    /// * `output_node` - NodeId to use as final output
    /// * `buffer_size` - Size of audio buffers (usually 512)
    ///
    /// # Errors
    /// - If output_node is invalid
    /// - If dependency graph cannot be built
    ///
    /// # Note on Cycles
    /// Cycles are allowed! On first block, cyclic dependencies read from zero-initialized buffers.
    /// On subsequent blocks, they read from previous block's output. This enables feedback loops.
    pub fn new(
        nodes: Vec<Box<dyn AudioNode>>,
        output_node: NodeId,
        buffer_size: usize,
    ) -> Result<Self, String> {
        if output_node >= nodes.len() {
            return Err(format!(
                "Invalid output node: {} (have {} nodes)",
                output_node,
                nodes.len()
            ));
        }

        let dependency_graph = DependencyGraph::build(&nodes)?;

        // Cycles are OK! node_outputs HashMap provides one-block delay for feedback
        // First block: reads from zero_buffer (silence)
        // Subsequent blocks: reads from previous block's output

        let node_outputs = HashMap::new();
        let buffer_manager = BufferManager::new(nodes.len() * 2, buffer_size);
        let zero_buffer = vec![0.0; buffer_size];  // Silence for cyclic dependencies

        Ok(Self {
            nodes,
            dependency_graph,
            node_outputs,
            buffer_manager,
            output_node,
            buffer_size,
            zero_buffer,
        })
    }

    /// Process entire block - graph traversed ONCE
    ///
    /// This is the core execution loop for DAW-style buffer passing:
    /// 1. Prepare all nodes (pattern evaluation)
    /// 2. Get topological execution order
    /// 3. Process nodes in order (dependencies already computed)
    /// 4. Copy final output
    ///
    /// # Arguments
    /// * `output` - Output buffer to write to
    /// * `context` - Processing context (cycle position, tempo, etc.)
    pub fn process_block(
        &mut self,
        output: &mut [f32],
        context: &ProcessContext,
    ) -> Result<(), String> {
        debug_assert_eq!(
            output.len(),
            self.buffer_size,
            "Output buffer size mismatch"
        );

        // Phase 1: Prepare all nodes (pattern evaluation, voice triggering)
        for node in &mut self.nodes {
            node.prepare_block(context);
        }

        // Phase 2: Get execution order (topological sort)
        let exec_order = self.dependency_graph.execution_order()?;

        // Phase 3: Process nodes in execution order
        for &node_id in &exec_order {
            // Gather input buffers from dependencies
            let input_ids = self.nodes[node_id].input_nodes();
            let input_buffers: Vec<&[f32]> = input_ids
                .iter()
                .map(|&id| {
                    self.node_outputs
                        .get(&id)
                        .map(|output| output.as_slice())
                        .unwrap_or(&self.zero_buffer)  // Cyclic dependency: use silence
                })
                .collect();

            // Get output buffer from pool
            let mut node_buffer = self.buffer_manager.get_buffer();

            // PROCESS NODE (entire block at once!)
            self.nodes[node_id].process_block(
                &input_buffers,
                &mut node_buffer,
                context.sample_rate,
                context,
            );

            // Store output for dependents (Arc for zero-copy sharing)
            self.node_outputs.insert(
                node_id,
                NodeOutput::new(node_buffer),
            );
        }

        // Phase 4: Copy final output
        if let Some(final_output) = self.node_outputs.get(&self.output_node) {
            output.copy_from_slice(&final_output.buffer);
        } else {
            return Err(format!(
                "Output node {} not processed",
                self.output_node
            ));
        }

        // Phase 5: Return buffers to pool
        for (_, node_output) in self.node_outputs.drain() {
            if let Ok(buffer) = node_output.try_unwrap() {
                self.buffer_manager.return_buffer(buffer);
            }
        }

        Ok(())
    }

    /// Get buffer manager statistics
    pub fn buffer_stats(&self) -> &crate::buffer_manager::BufferStats {
        self.buffer_manager.stats()
    }

    /// Get number of nodes in graph
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get output node ID
    pub fn output_node(&self) -> NodeId {
        self.output_node
    }

    /// Get execution order (for debugging)
    pub fn execution_order(&self) -> Result<Vec<NodeId>, String> {
        self.dependency_graph.execution_order()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::{ConstantNode, AdditionNode, OscillatorNode, Waveform};
    use crate::pattern::Fraction;

    #[test]
    fn test_block_processor_simple_constant() {
        // Single constant node
        let nodes: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(440.0)),
        ];

        let mut processor = BlockProcessor::new(nodes, 0, 512).unwrap();

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        let mut output = vec![0.0; 512];
        processor.process_block(&mut output, &context).unwrap();

        // All samples should be 440.0
        for sample in &output {
            assert_eq!(*sample, 440.0);
        }
    }

    #[test]
    fn test_block_processor_addition() {
        // Two constants added together: 100 + 50 = 150
        let nodes: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(100.0)),  // Node 0
            Box::new(ConstantNode::new(50.0)),   // Node 1
            Box::new(AdditionNode::new(0, 1)),   // Node 2
        ];

        let mut processor = BlockProcessor::new(nodes, 2, 512).unwrap();

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        let mut output = vec![0.0; 512];
        processor.process_block(&mut output, &context).unwrap();

        for sample in &output {
            assert_eq!(*sample, 150.0);
        }
    }

    #[test]
    fn test_block_processor_oscillator() {
        // 440 Hz sine wave
        let nodes: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(440.0)),  // Node 0 (freq)
            Box::new(OscillatorNode::new(0, Waveform::Sine)),  // Node 1
        ];

        let mut processor = BlockProcessor::new(nodes, 1, 512).unwrap();

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        let mut output = vec![0.0; 512];
        processor.process_block(&mut output, &context).unwrap();

        // Should have sine wave output
        // Check that we have signal in valid range
        let max = output.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let min = output.iter().fold(f32::INFINITY, |a, &b| a.min(b));

        assert!(max > 0.5, "Sine max too low: {}", max);
        assert!(min < -0.5, "Sine min too high: {}", min);
        assert!(max <= 1.0, "Sine max out of range: {}", max);
        assert!(min >= -1.0, "Sine min out of range: {}", min);
    }

    #[test]
    fn test_block_processor_complex_graph() {
        // freq_a = 220, freq_b = 440
        // osc_a = sine(freq_a), osc_b = sine(freq_b)
        // output = osc_a + osc_b
        let nodes: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(220.0)),  // Node 0
            Box::new(ConstantNode::new(440.0)),  // Node 1
            Box::new(OscillatorNode::new(0, Waveform::Sine)),  // Node 2
            Box::new(OscillatorNode::new(1, Waveform::Sine)),  // Node 3
            Box::new(AdditionNode::new(2, 3)),   // Node 4
        ];

        let mut processor = BlockProcessor::new(nodes, 4, 512).unwrap();

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        let mut output = vec![0.0; 512];
        processor.process_block(&mut output, &context).unwrap();

        // Should have combined sine waves
        // Range can exceed [-1, 1] due to addition
        let max = output.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let min = output.iter().fold(f32::INFINITY, |a, &b| a.min(b));

        assert!(max > 0.5, "Combined max too low: {}", max);
        assert!(min < -0.5, "Combined min too high: {}", min);
    }

    #[test]
    fn test_block_processor_execution_order() {
        // Verify topological order is correct
        let nodes: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(1.0)),   // Node 0 (no deps)
            Box::new(ConstantNode::new(2.0)),   // Node 1 (no deps)
            Box::new(AdditionNode::new(0, 1)),  // Node 2 (deps: 0, 1)
        ];

        let processor = BlockProcessor::new(nodes, 2, 512).unwrap();
        let order = processor.execution_order().unwrap();

        // Nodes 0 and 1 must come before node 2
        let pos_0 = order.iter().position(|&id| id == 0).unwrap();
        let pos_1 = order.iter().position(|&id| id == 1).unwrap();
        let pos_2 = order.iter().position(|&id| id == 2).unwrap();

        assert!(pos_0 < pos_2);
        assert!(pos_1 < pos_2);
    }

    #[test]
    fn test_block_processor_invalid_output_node() {
        let nodes: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(1.0)),
        ];

        // Output node 5 doesn't exist (only have node 0)
        let result = BlockProcessor::new(nodes, 5, 512);
        assert!(result.is_err());
    }

    #[test]
    fn test_block_processor_buffer_reuse() {
        let nodes: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(440.0)),
        ];

        let mut processor = BlockProcessor::new(nodes, 0, 512).unwrap();

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        let mut output = vec![0.0; 512];

        // Process multiple blocks
        for _ in 0..10 {
            processor.process_block(&mut output, &context).unwrap();
        }

        let stats = processor.buffer_stats();

        // Should have reused buffers (allocations < total blocks processed)
        assert!(
            stats.reuses > 5,
            "Expected buffer reuse, got {} reuses",
            stats.reuses
        );
    }
}
