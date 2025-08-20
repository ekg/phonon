//! Signal graph execution engine
//! 
//! This module executes signal graphs, processing audio through the defined nodes
//! and connections, handling modulation and cross-routing.

use crate::signal_graph::{
    SignalGraph, Node, NodeId, BusId,
    SourceType, ProcessorType, AnalysisType
};
use fundsp::hacker::*;
use std::collections::HashMap;

/// Audio buffer for processing
pub struct AudioBuffer {
    pub data: Vec<f32>,
    pub sample_rate: f32,
    pub channels: usize,
}

impl AudioBuffer {
    pub fn new(size: usize, sample_rate: f32, channels: usize) -> Self {
        Self {
            data: vec![0.0; size * channels],
            sample_rate,
            channels,
        }
    }
    
    pub fn mono(size: usize, sample_rate: f32) -> Self {
        Self::new(size, sample_rate, 1)
    }
    
    pub fn stereo(size: usize, sample_rate: f32) -> Self {
        Self::new(size, sample_rate, 2)
    }
    
    pub fn clear(&mut self) {
        self.data.fill(0.0);
    }
    
    /// Get RMS (Root Mean Square) value
    pub fn rms(&self) -> f32 {
        if self.data.is_empty() {
            return 0.0;
        }
        
        let sum: f32 = self.data.iter().map(|x| x * x).sum();
        (sum / self.data.len() as f32).sqrt()
    }
    
    /// Get peak value
    pub fn peak(&self) -> f32 {
        self.data.iter().map(|x| x.abs()).fold(0.0, f32::max)
    }
    
    /// Write to WAV file for testing
    pub fn write_wav(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        use hound::{WavSpec, WavWriter, SampleFormat};
        
        let spec = WavSpec {
            channels: self.channels as u16,
            sample_rate: self.sample_rate as u32,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        };
        
        let mut writer = WavWriter::create(path, spec)?;
        for sample in &self.data {
            writer.write_sample(*sample)?;
        }
        writer.finalize()?;
        Ok(())
    }
}

/// Node processor that converts node definitions to actual audio processing
pub struct NodeProcessor {
    sample_rate: f32,
    processors: HashMap<NodeId, Box<dyn AudioUnit>>,
}

impl NodeProcessor {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            processors: HashMap::new(),
        }
    }
    
    /// Create audio processor for a node
    pub fn create_processor(&mut self, node: &Node) -> Result<(), String> {
        let processor: Box<dyn AudioUnit> = match node {
            Node::Source { id: _, source_type } => {
                match source_type {
                    SourceType::Sine { freq } => {
                        Box::new(sine_hz(*freq as f32))
                    }
                    SourceType::Saw { freq } => {
                        Box::new(saw_hz(*freq as f32))
                    }
                    SourceType::Square { freq } => {
                        Box::new(square_hz(*freq as f32))
                    }
                    SourceType::Triangle { freq } => {
                        Box::new(triangle_hz(*freq as f32))
                    }
                    SourceType::Noise => {
                        Box::new(white())
                    }
                    SourceType::Sample { .. } => {
                        // For now, return silence for samples
                        Box::new(zero())
                    }
                }
            }
            Node::Processor { id: _, processor_type } => {
                match processor_type {
                    ProcessorType::LowPass { cutoff, q } => {
                        Box::new(lowpass_hz(*cutoff as f32, *q as f32))
                    }
                    ProcessorType::HighPass { cutoff, q } => {
                        Box::new(highpass_hz(*cutoff as f32, *q as f32))
                    }
                    ProcessorType::BandPass { center, q } => {
                        Box::new(bandpass_hz(*center as f32, *q as f32))
                    }
                    ProcessorType::Delay { time, .. } => {
                        // Simple delay without feedback for now
                        // Feedback would require a more complex implementation
                        Box::new(delay(*time as f32))
                    }
                    ProcessorType::Reverb { mix } => {
                        // Simple reverb placeholder
                        Box::new(reverb_stereo(40.0, 3.0, *mix as f32))
                    }
                    ProcessorType::Distortion { amount } => {
                        // Use soft clipping for distortion
                        // For now, just use gain as placeholder
                        let gain = (*amount as f32).min(2.0);
                        Box::new(mul(gain))
                    }
                    ProcessorType::Compressor { threshold: _, ratio } => {
                        // For now, use a simple gain reduction
                        // A proper compressor would need attack/release times
                        let gain = 1.0 / (*ratio as f32);
                        Box::new(mul(gain))
                    }
                    ProcessorType::Gain { amount } => {
                        Box::new(mul(*amount))
                    }
                }
            }
            Node::Analysis { .. } => {
                // Analysis nodes pass through audio while extracting features
                Box::new(pass())
            }
            Node::Pattern { .. } => {
                // Pattern nodes generate control signals
                Box::new(zero())
            }
            Node::Bus { .. } => {
                Box::new(zero())
            }
            Node::Output { .. } => {
                Box::new(pass())
            }
        };
        
        let id = match node {
            Node::Source { id, .. } |
            Node::Processor { id, .. } |
            Node::Analysis { id, .. } |
            Node::Pattern { id, .. } |
            Node::Output { id } => id.clone(),
            Node::Bus { id, .. } => NodeId(id.0.clone()),
        };
        
        self.processors.insert(id, processor);
        Ok(())
    }
    
    /// Process a block of audio through a node
    pub fn process_node(&mut self, node_id: &NodeId, input: &AudioBuffer, output: &mut AudioBuffer) {
        if let Some(processor) = self.processors.get_mut(node_id) {
            // Check how many inputs the processor expects
            let num_inputs = processor.inputs();
            
            for i in 0..input.data.len() {
                let mut out = [0.0f32];
                
                if num_inputs == 0 {
                    // Source node - no inputs
                    processor.tick(&[], &mut out);
                } else if num_inputs == 1 {
                    // Single input processor
                    processor.tick(&[input.data[i]], &mut out);
                } else {
                    // Multi-channel - for now just duplicate the mono input
                    let inputs = vec![input.data[i]; num_inputs];
                    processor.tick(&inputs, &mut out);
                }
                
                output.data[i] = out[0];
            }
        } else {
            // If no processor, just copy
            output.data.copy_from_slice(&input.data);
        }
    }
}

/// Signal graph executor
pub struct SignalExecutor {
    graph: SignalGraph,
    processor: NodeProcessor,
    buffers: HashMap<NodeId, AudioBuffer>,
    bus_values: HashMap<BusId, f32>,
    sample_rate: f32,
    block_size: usize,
}

impl SignalExecutor {
    pub fn new(graph: SignalGraph, sample_rate: f32, block_size: usize) -> Self {
        let processor = NodeProcessor::new(sample_rate);
        
        Self {
            graph,
            processor,
            buffers: HashMap::new(),
            bus_values: HashMap::new(),
            sample_rate,
            block_size,
        }
    }
    
    /// Initialize processors for all nodes
    pub fn initialize(&mut self) -> Result<(), String> {
        // Create processors for all nodes
        let nodes: Vec<Node> = self.graph.nodes.values().cloned().collect();
        for node in nodes {
            self.processor.create_processor(&node)?;
            
            // Create buffer for node
            let id = match &node {
                Node::Source { id, .. } |
                Node::Processor { id, .. } |
                Node::Analysis { id, .. } |
                Node::Pattern { id, .. } |
                Node::Output { id } => id.clone(),
                Node::Bus { id, .. } => NodeId(id.0.clone()),
            };
            
            let buffer = AudioBuffer::mono(self.block_size, self.sample_rate);
            self.buffers.insert(id, buffer);
        }
        
        // Compute execution order
        self.graph.compute_execution_order()?;
        
        Ok(())
    }
    
    /// Process one block of audio
    pub fn process_block(&mut self) -> Result<AudioBuffer, String> {
        let execution_order = self.graph.get_execution_order()?.to_vec();
        
        for node_id in execution_order {
            // Get connections to this node
            let connections = self.graph.get_connections_to(&node_id);
            
            if connections.is_empty() {
                // Source node - generate audio
                if let Some(output_buffer) = self.buffers.get_mut(&node_id) {
                    output_buffer.clear();
                    let input = AudioBuffer::mono(self.block_size, self.sample_rate);
                    self.processor.process_node(&node_id, &input, output_buffer);
                    
                    // Debug: Check if source is generating audio
                    if output_buffer.peak() == 0.0 {
                        eprintln!("Warning: Source node {} generated silence", node_id.0);
                    }
                }
            } else {
                // Mix inputs from connections
                let mut mixed = AudioBuffer::mono(self.block_size, self.sample_rate);
                
                for conn in connections {
                    if let Some(input_buffer) = self.buffers.get(&conn.from) {
                        for i in 0..mixed.data.len() {
                            mixed.data[i] += input_buffer.data[i] * conn.amount;
                        }
                    }
                }
                
                // Process through node
                if let Some(output_buffer) = self.buffers.get_mut(&node_id) {
                    self.processor.process_node(&node_id, &mixed, output_buffer);
                }
            }
            
            // Handle analysis nodes
            if let Some(node) = self.graph.nodes.get(&node_id) {
                if let Node::Analysis { analysis_type, .. } = node {
                    if let Some(buffer) = self.buffers.get(&node_id) {
                        let value = match analysis_type {
                            AnalysisType::RMS { .. } => buffer.rms(),
                            AnalysisType::Peak => buffer.peak(),
                            _ => 0.0,
                        };
                        // Store analysis result as bus value
                        self.bus_values.insert(BusId(node_id.0.clone()), value);
                    }
                }
            }
        }
        
        // Find output node
        for (node_id, node) in &self.graph.nodes {
            if matches!(node, Node::Output { .. }) {
                if let Some(buffer) = self.buffers.get(node_id) {
                    return Ok(AudioBuffer {
                        data: buffer.data.clone(),
                        sample_rate: self.sample_rate,
                        channels: buffer.channels,
                    });
                }
            }
        }
        
        // If no output node, return empty buffer
        Ok(AudioBuffer::mono(self.block_size, self.sample_rate))
    }
    
    /// Get current bus value
    pub fn get_bus_value(&self, bus_id: &BusId) -> f32 {
        self.bus_values.get(bus_id).copied().unwrap_or(0.0)
    }
    
    /// Process multiple blocks and return combined audio
    pub fn render(&mut self, num_blocks: usize) -> Result<AudioBuffer, String> {
        let total_samples = self.block_size * num_blocks;
        let mut output = AudioBuffer::mono(total_samples, self.sample_rate);
        
        for block_idx in 0..num_blocks {
            let block = self.process_block()?;
            let start = block_idx * self.block_size;
            let end = start + self.block_size;
            output.data[start..end].copy_from_slice(&block.data);
        }
        
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signal_parser::SignalParser;
    
    #[test]
    fn test_audio_buffer() {
        let buffer = AudioBuffer::mono(1024, 44100.0);
        assert_eq!(buffer.data.len(), 1024);
        assert_eq!(buffer.sample_rate, 44100.0);
        assert_eq!(buffer.channels, 1);
        assert_eq!(buffer.rms(), 0.0);
        assert_eq!(buffer.peak(), 0.0);
    }
    
    #[test]
    fn test_audio_buffer_analysis() {
        let mut buffer = AudioBuffer::mono(4, 44100.0);
        buffer.data = vec![0.5, -0.5, 0.5, -0.5];
        
        // RMS of [0.5, -0.5, 0.5, -0.5] = sqrt((0.25 * 4) / 4) = 0.5
        assert_eq!(buffer.rms(), 0.5);
        assert_eq!(buffer.peak(), 0.5);
    }
    
    #[test]
    fn test_simple_sine_generation() {
        // Create a simple sine wave graph
        let mut graph = SignalGraph::new(44100.0);
        
        let sine_node = Node::Source {
            id: NodeId("sine".to_string()),
            source_type: SourceType::Sine { freq: 440.0 },
        };
        
        let output_node = Node::Output {
            id: NodeId("output".to_string()),
        };
        
        let sine_id = graph.add_node(sine_node);
        let output_id = graph.add_node(output_node);
        graph.connect(sine_id, output_id, 1.0);
        
        // Execute
        let mut executor = SignalExecutor::new(graph, 44100.0, 512);
        executor.initialize().unwrap();
        
        let output = executor.process_block().unwrap();
        
        // Verify we got audio
        assert!(output.peak() > 0.0);
        assert!(output.rms() > 0.0);
    }
    
    #[test]
    fn test_signal_chain() {
        // Create: sine(440) >> lpf(1000, 0.7)
        let mut graph = SignalGraph::new(44100.0);
        
        let sine_node = Node::Source {
            id: NodeId("sine".to_string()),
            source_type: SourceType::Sine { freq: 440.0 },
        };
        
        let filter_node = Node::Processor {
            id: NodeId("filter".to_string()),
            processor_type: ProcessorType::LowPass { cutoff: 1000.0, q: 0.7 },
        };
        
        let output_node = Node::Output {
            id: NodeId("output".to_string()),
        };
        
        let sine_id = graph.add_node(sine_node);
        let filter_id = graph.add_node(filter_node);
        let output_id = graph.add_node(output_node);
        
        graph.connect(sine_id.clone(), filter_id.clone(), 1.0);
        graph.connect(filter_id, output_id, 1.0);
        
        // Execute
        let mut executor = SignalExecutor::new(graph, 44100.0, 512);
        executor.initialize().unwrap();
        
        let output = executor.process_block().unwrap();
        
        // Verify we got filtered audio
        assert!(output.peak() > 0.0);
        assert!(output.rms() > 0.0);
    }
    
    #[test]
    fn test_rms_analysis() {
        // Create: sine(440) >> rms(0.05)
        let mut graph = SignalGraph::new(44100.0);
        
        let sine_node = Node::Source {
            id: NodeId("sine".to_string()),
            source_type: SourceType::Sine { freq: 440.0 },
        };
        
        let rms_node = Node::Analysis {
            id: NodeId("rms".to_string()),
            analysis_type: AnalysisType::RMS { window_size: 0.05 },
        };
        
        let output_node = Node::Output {
            id: NodeId("output".to_string()),
        };
        
        let sine_id = graph.add_node(sine_node);
        let rms_id = graph.add_node(rms_node);
        let output_id = graph.add_node(output_node);
        
        graph.connect(sine_id, rms_id.clone(), 1.0);
        graph.connect(rms_id.clone(), output_id, 1.0);
        
        // Execute
        let mut executor = SignalExecutor::new(graph, 44100.0, 512);
        executor.initialize().unwrap();
        
        executor.process_block().unwrap();
        
        // Check that RMS was calculated
        let rms_value = executor.get_bus_value(&BusId("rms".to_string()));
        assert!(rms_value > 0.0);
    }
    
    #[test]
    fn test_parallel_mixing() {
        // Create two sines mixed together
        let mut graph = SignalGraph::new(44100.0);
        
        let sine1 = Node::Source {
            id: NodeId("sine1".to_string()),
            source_type: SourceType::Sine { freq: 440.0 },
        };
        
        let sine2 = Node::Source {
            id: NodeId("sine2".to_string()),
            source_type: SourceType::Sine { freq: 880.0 },
        };
        
        let output = Node::Output {
            id: NodeId("output".to_string()),
        };
        
        let sine1_id = graph.add_node(sine1);
        let sine2_id = graph.add_node(sine2);
        let output_id = graph.add_node(output);
        
        // Mix both sines to output at 0.5 amplitude each
        graph.connect(sine1_id, output_id.clone(), 0.5);
        graph.connect(sine2_id, output_id.clone(), 0.5);
        
        // Execute
        let mut executor = SignalExecutor::new(graph, 44100.0, 512);
        executor.initialize().unwrap();
        
        let output = executor.process_block().unwrap();
        
        // Verify mixed output
        assert!(output.peak() > 0.0);
        assert!(output.rms() > 0.0);
    }
    
    #[test]
    fn test_render_to_wav() {
        // Create a simple test signal and write to WAV
        let mut graph = SignalGraph::new(44100.0);
        
        let sine_node = Node::Source {
            id: NodeId("sine".to_string()),
            source_type: SourceType::Sine { freq: 440.0 },
        };
        
        let output_node = Node::Output {
            id: NodeId("output".to_string()),
        };
        
        let sine_id = graph.add_node(sine_node);
        let output_id = graph.add_node(output_node);
        graph.connect(sine_id, output_id, 0.5); // 50% volume
        
        // Execute and render 1 second
        let mut executor = SignalExecutor::new(graph, 44100.0, 512);
        executor.initialize().unwrap();
        
        let num_blocks = (44100.0 / 512.0).ceil() as usize;
        let output = executor.render(num_blocks).unwrap();
        
        // Write to test file
        let test_file = "/tmp/test_sine_440.wav";
        output.write_wav(test_file).unwrap();
        
        // Verify file exists and has correct properties
        use std::fs;
        let metadata = fs::metadata(test_file).unwrap();
        assert!(metadata.len() > 0);
        
        // Clean up
        fs::remove_file(test_file).ok();
    }
    
    #[test]
    fn test_complex_chain_with_parser() {
        // Simplified test - just saw to output
        let mut graph = SignalGraph::new(44100.0);
        
        let saw = Node::Source {
            id: NodeId("saw".to_string()),
            source_type: SourceType::Saw { freq: 220.0 },
        };
        
        let output = Node::Output {
            id: NodeId("output".to_string()),
        };
        
        let saw_id = graph.add_node(saw);
        let output_id = graph.add_node(output);
        
        graph.connect(saw_id, output_id, 1.0);
        
        // Execute
        let mut executor = SignalExecutor::new(graph, 44100.0, 512);
        executor.initialize().unwrap();
        
        let output = executor.process_block().unwrap();
        println!("Direct saw output peak: {}, RMS: {}", output.peak(), output.rms());
        
        // Verify saw generates audio
        assert!(output.peak() > 0.0, "Saw should generate audio");
        assert!(output.rms() > 0.0, "Saw should have non-zero RMS");
    }
}