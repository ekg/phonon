//! AudioNodeGraph - DAW-style block-based audio graph
//!
//! This module provides the new block-based architecture that processes
//! entire 512-sample buffers at once, replacing the sample-by-sample
//! SignalNode evaluation.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use crate::block_processor::BlockProcessor;
use crate::pattern::Fraction;
use std::collections::HashMap;

/// DAW-style audio graph using AudioNode trait
///
/// This graph processes audio in blocks (typically 512 samples) rather than
/// sample-by-sample. It uses topological sorting to determine execution order
/// and enables parallel processing of independent nodes.
///
/// # Example
/// ```ignore
/// let mut graph = AudioNodeGraph::new(44100.0);
/// graph.set_tempo(2.0);
///
/// // Add nodes
/// let const_id = graph.add_audio_node(Box::new(ConstantNode::new(440.0)));
/// let osc_id = graph.add_audio_node(Box::new(OscillatorNode::new(const_id, Waveform::Sine)));
///
/// // Set output
/// graph.set_output(osc_id);
///
/// // Build processor
/// graph.build_processor()?;
///
/// // Process audio
/// let mut buffer = vec![0.0; 512];
/// graph.process_buffer(&mut buffer)?;
/// ```
pub struct AudioNodeGraph {
    /// All audio nodes in the graph
    audio_nodes: Vec<Box<dyn AudioNode>>,

    /// Sample rate (e.g., 44100.0)
    sample_rate: f32,

    /// Tempo in cycles per second (e.g., 2.0 = 120 BPM)
    tempo: f64,

    /// Current position in the cycle (0.0 to 1.0, wraps)
    cycle_position: Fraction,

    /// Sample counter for tracking position
    sample_count: u64,

    /// Main output node (out:)
    output_node: Option<NodeId>,

    /// Multi-output nodes (out1:, out2:, etc.)
    outputs: HashMap<usize, NodeId>,

    /// Channels that have been hushed (silenced)
    hushed_channels: Vec<usize>,

    /// Block processor (created after all nodes added)
    block_processor: Option<BlockProcessor>,

    /// Buffer size for block processing
    buffer_size: usize,
}

impl AudioNodeGraph {
    /// Create a new audio node graph
    ///
    /// # Arguments
    /// * `sample_rate` - Sample rate in Hz (e.g., 44100.0)
    pub fn new(sample_rate: f32) -> Self {
        Self {
            audio_nodes: Vec::new(),
            sample_rate,
            tempo: 2.0, // Default: 120 BPM
            cycle_position: Fraction::from_float(0.0),
            sample_count: 0,
            output_node: None,
            outputs: HashMap::new(),
            hushed_channels: Vec::new(),
            block_processor: None,
            buffer_size: 512, // Standard block size
        }
    }

    /// Add an audio node to the graph
    ///
    /// Returns the NodeId that can be used to reference this node
    pub fn add_audio_node(&mut self, node: Box<dyn AudioNode>) -> NodeId {
        let node_id = self.audio_nodes.len();
        self.audio_nodes.push(node);
        node_id
    }

    /// Set the main output node (out:)
    pub fn set_output(&mut self, node_id: NodeId) {
        self.output_node = Some(node_id);
    }

    /// Set a numbered output (out1:, out2:, etc.)
    pub fn set_numbered_output(&mut self, channel: usize, node_id: NodeId) {
        self.outputs.insert(channel, node_id);
    }

    /// Set the tempo in cycles per second
    pub fn set_tempo(&mut self, tempo: f64) {
        self.tempo = tempo;
    }

    /// Get the current sample rate
    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Get the current tempo
    pub fn tempo(&self) -> f64 {
        self.tempo
    }

    /// Hush a channel (silence it)
    pub fn hush(&mut self, channel: usize) {
        if !self.hushed_channels.contains(&channel) {
            self.hushed_channels.push(channel);
        }
    }

    /// Unhush a channel (enable it)
    pub fn unhush(&mut self, channel: usize) {
        self.hushed_channels.retain(|&ch| ch != channel);
    }

    /// Unhush all channels
    pub fn unhush_all(&mut self) {
        self.hushed_channels.clear();
    }

    /// Build the block processor from accumulated nodes
    ///
    /// This must be called after all nodes are added and before processing.
    /// It performs topological sort and creates the execution plan.
    ///
    /// # Errors
    /// - If no output node is set
    /// - If dependency graph has cycles
    /// - If output node is invalid
    pub fn build_processor(&mut self) -> Result<(), String> {
        // Determine final output node
        let output_node = if let Some(node_id) = self.output_node {
            node_id
        } else if !self.outputs.is_empty() {
            // Use first numbered output if no main output
            *self.outputs.values().next().unwrap()
        } else {
            return Err("No output node set (use set_output or set_numbered_output)".to_string());
        };

        // For multi-output support, we'll need to handle mixing later
        // For now, just use the main output or first numbered output

        // Move nodes into BlockProcessor (we no longer need them in audio_nodes)
        // This avoids the need for cloning which isn't possible with trait objects
        let nodes = std::mem::take(&mut self.audio_nodes);

        // Create BlockProcessor - this validates the graph and builds execution plan
        self.block_processor = Some(BlockProcessor::new(
            nodes,
            output_node,
            self.buffer_size,
        )?);

        Ok(())
    }

    /// Process a buffer of audio
    ///
    /// This is the main entry point for block-based processing.
    /// The graph is traversed ONCE per buffer (not 512 times).
    ///
    /// # Arguments
    /// * `buffer` - Output buffer to fill (typically 512 samples)
    ///
    /// # Errors
    /// - If build_processor() hasn't been called
    /// - If block processing fails
    pub fn process_buffer(&mut self, buffer: &mut [f32]) -> Result<(), String> {
        let block_processor = self.block_processor.as_mut()
            .ok_or("build_processor() must be called before process_buffer()")?;

        // Create processing context
        let context = ProcessContext::new(
            self.cycle_position.clone(),
            0,
            buffer.len(),
            self.tempo,
            self.sample_rate,
        );

        // Process entire block at once - graph traversed ONCE!
        block_processor.process_block(buffer, &context)?;

        // Update cycle position
        self.update_cycle_position(buffer.len());

        Ok(())
    }

    /// Process buffer with multi-output support
    ///
    /// Returns a HashMap of channel → buffer for mixing
    pub fn process_buffer_multi_output(&mut self, buffer_size: usize) -> Result<HashMap<usize, Vec<f32>>, String> {
        if self.block_processor.is_none() {
            return Err("build_processor() must be called before processing".to_string());
        }

        let mut channel_buffers = HashMap::new();

        // Process main output if set
        if let Some(_) = self.output_node {
            let mut buffer = vec![0.0; buffer_size];
            self.process_buffer(&mut buffer)?;

            if !self.hushed_channels.contains(&0) {
                channel_buffers.insert(0, buffer);
            }
        }

        // TODO: Process numbered outputs (requires rebuilding processor for each)
        // For now, multi-output will need to be handled differently

        Ok(channel_buffers)
    }

    /// Render audio to a buffer
    ///
    /// Convenience method that creates a buffer and processes it.
    ///
    /// # Arguments
    /// * `num_samples` - Number of samples to render
    ///
    /// # Returns
    /// Vector containing rendered audio
    pub fn render(&mut self, num_samples: usize) -> Result<Vec<f32>, String> {
        let mut buffer = vec![0.0; num_samples];
        let mut offset = 0;

        // Process in fixed-size blocks
        while offset < num_samples {
            let chunk_size = (num_samples - offset).min(self.buffer_size);

            if chunk_size == self.buffer_size {
                // Full block - process directly
                self.process_buffer(&mut buffer[offset..offset + chunk_size])?;
            } else {
                // Partial block - process into temp buffer and copy
                let mut temp_buffer = vec![0.0; self.buffer_size];
                self.process_buffer(&mut temp_buffer)?;
                buffer[offset..offset + chunk_size].copy_from_slice(&temp_buffer[..chunk_size]);
            }

            offset += chunk_size;
        }

        Ok(buffer)
    }

    /// Update cycle position after processing samples
    fn update_cycle_position(&mut self, num_samples: usize) {
        self.sample_count += num_samples as u64;

        // Calculate how many cycles have passed
        let samples_per_cycle = self.sample_rate as f64 / self.tempo;
        let cycles_elapsed = num_samples as f64 / samples_per_cycle;

        // Update cycle position
        let new_position = self.cycle_position.to_float() + cycles_elapsed;
        self.cycle_position = Fraction::from_float(new_position % 1.0);
    }

    /// Reset the graph state (cycle position, sample count)
    pub fn reset(&mut self) {
        self.cycle_position = Fraction::from_float(0.0);
        self.sample_count = 0;
    }

    /// Get the number of nodes in the graph
    pub fn node_count(&self) -> usize {
        self.audio_nodes.len()
    }

    /// Check if the processor has been built
    pub fn is_ready(&self) -> bool {
        self.block_processor.is_some()
    }
}

impl Default for AudioNodeGraph {
    fn default() -> Self {
        Self::new(44100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::ConstantNode;

    #[test]
    fn test_audio_node_graph_creation() {
        let graph = AudioNodeGraph::new(44100.0);
        assert_eq!(graph.sample_rate(), 44100.0);
        assert_eq!(graph.tempo(), 2.0);
        assert_eq!(graph.node_count(), 0);
        assert!(!graph.is_ready());
    }

    #[test]
    fn test_add_audio_node() {
        let mut graph = AudioNodeGraph::new(44100.0);

        let node = Box::new(ConstantNode::new(440.0));
        let node_id = graph.add_audio_node(node);

        assert_eq!(node_id, 0);
        assert_eq!(graph.node_count(), 1);
    }

    #[test]
    fn test_set_output() {
        let mut graph = AudioNodeGraph::new(44100.0);

        let node = Box::new(ConstantNode::new(440.0));
        let node_id = graph.add_audio_node(node);

        graph.set_output(node_id);
        assert_eq!(graph.output_node, Some(0));
    }

    #[test]
    fn test_build_processor_requires_output() {
        let mut graph = AudioNodeGraph::new(44100.0);

        let node = Box::new(ConstantNode::new(440.0));
        let _node_id = graph.add_audio_node(node);

        // Should fail - no output set
        let result = graph.build_processor();
        assert!(result.is_err());
    }

    #[test]
    fn test_build_processor_success() {
        let mut graph = AudioNodeGraph::new(44100.0);

        let node = Box::new(ConstantNode::new(440.0));
        let node_id = graph.add_audio_node(node);

        graph.set_output(node_id);

        let result = graph.build_processor();
        assert!(result.is_ok());
        assert!(graph.is_ready());
    }

    #[test]
    fn test_tempo_setting() {
        let mut graph = AudioNodeGraph::new(44100.0);

        graph.set_tempo(3.0);
        assert_eq!(graph.tempo(), 3.0);
    }

    #[test]
    fn test_hush_unhush() {
        let mut graph = AudioNodeGraph::new(44100.0);

        graph.hush(0);
        assert!(graph.hushed_channels.contains(&0));

        graph.unhush(0);
        assert!(!graph.hushed_channels.contains(&0));

        graph.hush(0);
        graph.hush(1);
        graph.unhush_all();
        assert!(graph.hushed_channels.is_empty());
    }
}
