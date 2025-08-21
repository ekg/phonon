//! Glicol-style DSP syntax implementation
//! 
//! Implements the graph-oriented >> operator for chaining audio nodes,
//! and ~ reference chains for lazy evaluation.

use crate::signal_graph::{SignalGraph, Node, NodeId, BusId, SourceType, ProcessorType};
use crate::pattern::{Pattern, State};
use std::collections::HashMap;
use fundsp::hacker::*;

/// DSP chain that can be connected with >> operator
#[derive(Clone, Debug)]
pub struct DspChain {
    pub nodes: Vec<DspNode>,
}

/// Individual DSP node in a chain
#[derive(Clone, Debug)]
pub enum DspNode {
    // Oscillators
    Sin { freq: f64 },
    Saw { freq: f64 },
    Square { freq: f64 },
    Triangle { freq: f64 },
    Noise,
    
    // Math operations
    Mul { value: f64 },
    Add { value: f64 },
    Div { value: f64 },
    Sub { value: f64 },
    
    // Filters
    Lpf { cutoff: f64, q: f64 },
    Hpf { cutoff: f64, q: f64 },
    Bpf { center: f64, q: f64 },
    Notch { center: f64, q: f64 },
    
    // Effects
    Delay { time: f64, feedback: f64 },
    Reverb { room: f64, damp: f64 },
    Chorus { rate: f64, depth: f64 },
    Phaser { rate: f64, depth: f64 },
    Distortion { gain: f64 },
    Compressor { threshold: f64, ratio: f64 },
    
    // Envelopes
    Adsr { attack: f64, decay: f64, sustain: f64, release: f64 },
    Env { stages: Vec<(f64, f64)> }, // (time, level) pairs
    
    // Modulators
    Lfo { freq: f64, shape: LfoShape },
    
    // Sequencer
    Seq { pattern: String }, // Mini-notation pattern
    Speed { factor: f64 },
    Choose { options: Vec<f64> },
    
    // Pattern integration
    Pattern { pattern: Pattern<String> },
    
    // Reference to another chain
    Ref { name: String },
    
    // Meta node for custom DSP
    Meta { code: String },
    
    // Sample playback
    Sp { sample: String },
    
    // Utilities
    Mix { sources: Vec<DspChain> },
    Pan { position: f64 },
    Gain { amount: f64 },
}

#[derive(Clone, Debug)]
pub enum LfoShape {
    Sine,
    Triangle,
    Square,
    Saw,
}

impl DspChain {
    /// Create a new empty chain
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
        }
    }
    
    /// Create a chain from a single node
    pub fn from_node(node: DspNode) -> Self {
        let mut chain = Self::new();
        chain.nodes.push(node);
        chain
    }
    
    /// Chain operator >> to connect nodes
    pub fn chain(mut self, node: DspNode) -> Self {
        self.nodes.push(node);
        self
    }
    
    /// Build the signal graph from the chain
    pub fn build_graph(&mut self, sample_rate: f32) -> Result<SignalGraph, String> {
        let mut graph = SignalGraph::new(sample_rate);
        let mut last_node_id: Option<NodeId> = None;
        
        for (i, node) in self.nodes.iter().enumerate() {
            let node_id = NodeId(format!("node_{}", i));
            
            let graph_node = match node {
                DspNode::Sin { freq } => Node::Source {
                    id: node_id.clone(),
                    source_type: SourceType::Sine { freq: *freq },
                },
                DspNode::Saw { freq } => Node::Source {
                    id: node_id.clone(),
                    source_type: SourceType::Saw { freq: *freq },
                },
                DspNode::Square { freq } => Node::Source {
                    id: node_id.clone(),
                    source_type: SourceType::Square { freq: *freq },
                },
                DspNode::Triangle { freq } => Node::Source {
                    id: node_id.clone(),
                    source_type: SourceType::Triangle { freq: *freq },
                },
                DspNode::Noise => Node::Source {
                    id: node_id.clone(),
                    source_type: SourceType::Noise,
                },
                DspNode::Mul { value } => Node::Processor {
                    id: node_id.clone(),
                    processor_type: ProcessorType::Gain { amount: *value as f32 },
                },
                DspNode::Add { value } => Node::Processor {
                    id: node_id.clone(),
                    // Use gain for now, could be a bias node
                    processor_type: ProcessorType::Gain { amount: (1.0 + *value) as f32 },
                },
                DspNode::Lpf { cutoff, q } => Node::Processor {
                    id: node_id.clone(),
                    processor_type: ProcessorType::LowPass { cutoff: *cutoff, q: *q },
                },
                DspNode::Hpf { cutoff, q } => Node::Processor {
                    id: node_id.clone(),
                    processor_type: ProcessorType::HighPass { cutoff: *cutoff, q: *q },
                },
                DspNode::Delay { time, feedback } => Node::Processor {
                    id: node_id.clone(),
                    processor_type: ProcessorType::Delay { 
                        time: *time, 
                        feedback: *feedback
                    },
                },
                DspNode::Reverb { room, damp } => Node::Processor {
                    id: node_id.clone(),
                    processor_type: ProcessorType::Reverb { 
                        mix: *room as f64  // Use room as mix for now
                    },
                },
                DspNode::Gain { amount } => Node::Processor {
                    id: node_id.clone(),
                    processor_type: ProcessorType::Gain { amount: *amount as f32 },
                },
                DspNode::Seq { pattern } => {
                    Node::Pattern {
                        id: node_id.clone(),
                        pattern: pattern.clone(),
                    }
                },
                DspNode::Pattern { pattern } => {
                    // Convert Pattern to string representation
                    // For now, just use a placeholder
                    Node::Pattern {
                        id: node_id.clone(),
                        pattern: "pattern".to_string(),
                    }
                },
                DspNode::Sp { sample } => Node::Source {
                    id: node_id.clone(),
                    source_type: SourceType::Sample { name: sample.clone() },
                },
                _ => {
                    // For now, create a pass-through for unimplemented nodes
                    Node::Processor {
                        id: node_id.clone(),
                        processor_type: ProcessorType::Gain { amount: 1.0 },
                    }
                }
            };
            
            graph.add_node(graph_node);
            
            // Connect to previous node if exists
            if let Some(prev_id) = last_node_id {
                graph.connect(prev_id, node_id.clone(), 1.0);
            }
            
            last_node_id = Some(node_id);
        }
        
        // Add output node if we have nodes
        if let Some(last_id) = last_node_id {
            let output_id = NodeId("output".to_string());
            graph.add_node(Node::Output { id: output_id.clone() });
            graph.connect(last_id, output_id, 1.0);
        }
        
        Ok(graph)
    }
}

/// Reference chain for lazy evaluation
pub struct RefChain {
    pub name: String,
    pub chain: DspChain,
}

/// DSP environment holding all chains and references
pub struct DspEnvironment {
    pub output_chain: Option<DspChain>,
    pub ref_chains: HashMap<String, DspChain>,
}

impl DspEnvironment {
    pub fn new() -> Self {
        Self {
            output_chain: None,
            ref_chains: HashMap::new(),
        }
    }
    
    /// Add a reference chain (starts with ~)
    pub fn add_ref(&mut self, name: String, chain: DspChain) {
        self.ref_chains.insert(name, chain);
    }
    
    /// Set the output chain
    pub fn set_output(&mut self, chain: DspChain) {
        self.output_chain = Some(chain);
    }
    
    /// Build complete graph resolving all references
    pub fn build_complete_graph(&self, sample_rate: f32) -> Result<SignalGraph, String> {
        let mut graph = SignalGraph::new(sample_rate);
        
        // First add all reference chains
        for (name, chain) in &self.ref_chains {
            let bus_id = BusId(name.clone());
            let mut chain_graph = chain.clone().build_graph(sample_rate)?;
            
            // Add as bus nodes
            for node in chain_graph.nodes.values() {
                graph.add_node(node.clone());
            }
        }
        
        // Then add output chain
        if let Some(output) = &self.output_chain {
            let mut output_graph = output.clone().build_graph(sample_rate)?;
            
            // Add all nodes from output chain
            for node in output_graph.nodes.values() {
                graph.add_node(node.clone());
            }
            
            // Add connections from output chain
            for conn in &output_graph.connections {
                graph.connections.push(conn.clone());
            }
            
            // The output node is already added by build_graph
        }
        
        Ok(graph)
    }
}

/// Helper functions to create DSP nodes
pub mod dsp {
    use super::*;
    
    pub fn sin(freq: f64) -> DspChain {
        DspChain::from_node(DspNode::Sin { freq })
    }
    
    pub fn saw(freq: f64) -> DspChain {
        DspChain::from_node(DspNode::Saw { freq })
    }
    
    pub fn square(freq: f64) -> DspChain {
        DspChain::from_node(DspNode::Square { freq })
    }
    
    pub fn triangle(freq: f64) -> DspChain {
        DspChain::from_node(DspNode::Triangle { freq })
    }
    
    pub fn noise() -> DspChain {
        DspChain::from_node(DspNode::Noise)
    }
    
    pub fn mul(value: f64) -> DspChain {
        DspChain::from_node(DspNode::Mul { value })
    }
    
    pub fn add(value: f64) -> DspChain {
        DspChain::from_node(DspNode::Add { value })
    }
    
    pub fn lpf(cutoff: f64, q: f64) -> DspChain {
        DspChain::from_node(DspNode::Lpf { cutoff, q })
    }
    
    pub fn hpf(cutoff: f64, q: f64) -> DspChain {
        DspChain::from_node(DspNode::Hpf { cutoff, q })
    }
    
    pub fn delay(time: f64, feedback: f64) -> DspChain {
        DspChain::from_node(DspNode::Delay { time, feedback })
    }
    
    pub fn reverb(room: f64, damp: f64) -> DspChain {
        DspChain::from_node(DspNode::Reverb { room, damp })
    }
    
    pub fn seq(pattern: &str) -> DspChain {
        DspChain::from_node(DspNode::Seq { pattern: pattern.to_string() })
    }
    
    pub fn sp(sample: &str) -> DspChain {
        DspChain::from_node(DspNode::Sp { sample: sample.to_string() })
    }
    
    pub fn gain(amount: f64) -> DspChain {
        DspChain::from_node(DspNode::Gain { amount })
    }
}

// Implement >> operator for chaining
impl std::ops::Shr for DspChain {
    type Output = DspChain;
    
    fn shr(self, rhs: DspChain) -> Self::Output {
        let mut result = self;
        for node in rhs.nodes {
            result.nodes.push(node);
        }
        result
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;
    use super::dsp::*;
    
    #[test]
    fn test_chain_building() {
        let chain = sin(440.0) >> mul(0.5) >> lpf(1000.0, 1.0);
        assert_eq!(chain.nodes.len(), 3);
    }
    
    #[test]
    fn test_graph_building() {
        let mut chain = sin(440.0) >> mul(0.5);
        let graph = chain.build_graph(48000.0).unwrap();
        assert_eq!(graph.nodes.len(), 3); // 2 nodes + output
    }
    
    #[test]
    fn test_environment() {
        let mut env = DspEnvironment::new();
        
        // Add reference chain
        let amp_chain = sin(1.0) >> mul(0.3) >> add(0.5);
        env.add_ref("amp".to_string(), amp_chain);
        
        // Set output chain
        let output = sin(440.0) >> mul(0.5);
        env.set_output(output);
        
        let graph = env.build_complete_graph(48000.0).unwrap();
        assert!(graph.nodes.len() > 0);
    }
}