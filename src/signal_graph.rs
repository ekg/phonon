//! Core signal graph infrastructure for modular synthesis DSL
//!
//! This module implements the fundamental signal routing system that allows
//! any signal to modulate any parameter in real-time.

use std::collections::HashMap;

/// Unique identifier for nodes in the signal graph
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct NodeId(pub String);

/// Unique identifier for signal buses
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct BusId(pub String);

/// Types of signal analysis that can be performed
#[derive(Debug, Clone)]
pub enum AnalysisType {
    /// Root Mean Square with window size in seconds
    RMS { window_size: f64 },
    /// Pitch detection
    Pitch,
    /// Transient detection
    Transient,
    /// Spectral centroid (brightness)
    Centroid,
    /// Zero crossing rate
    ZeroCrossings,
    /// Peak detection
    Peak,
}

/// Types of signal taps for feature extraction
#[derive(Debug, Clone)]
pub enum TapType {
    /// Direct signal value
    Direct,
    /// Analyzed feature
    Analysis(AnalysisType),
}

/// Connection between nodes in the signal graph
#[derive(Debug, Clone)]
pub struct Connection {
    pub from: NodeId,
    pub to: NodeId,
    pub amount: f32,
    pub tap_type: Option<TapType>,
}

/// Types of nodes in the signal graph
#[derive(Debug, Clone)]
pub enum Node {
    /// Audio source (oscillator, sample player, etc.)
    Source { id: NodeId, source_type: SourceType },
    /// Signal bus that can be referenced by name
    Bus { id: BusId, value: f32 },
    /// Audio processor (filter, effect, etc.)
    Processor {
        id: NodeId,
        processor_type: ProcessorType,
    },
    /// Signal analysis node
    Analysis {
        id: NodeId,
        analysis_type: AnalysisType,
    },
    /// Pattern node for pattern integration
    Pattern { id: NodeId, pattern: String },
    /// Output node
    Output { id: NodeId },
}

/// Types of audio sources
#[derive(Debug, Clone)]
pub enum SourceType {
    Sine { freq: f64 },
    Saw { freq: f64 },
    Square { freq: f64 },
    Triangle { freq: f64 },
    Noise,
    Sample { name: String },
}

/// Types of audio processors
#[derive(Debug, Clone)]
pub enum ProcessorType {
    LowPass { cutoff: f64, q: f64 },
    HighPass { cutoff: f64, q: f64 },
    BandPass { center: f64, q: f64 },
    Delay { time: f64, feedback: f64 },
    Reverb { mix: f64 },
    Distortion { amount: f64 },
    Compressor { threshold: f64, ratio: f64 },
    Gain { amount: f32 },
}

/// Main signal graph structure
#[derive(Debug, Clone)]
pub struct SignalGraph {
    /// All nodes in the graph
    pub nodes: HashMap<NodeId, Node>,
    /// Named signal buses
    pub buses: HashMap<BusId, f32>,
    /// Connections between nodes
    pub connections: Vec<Connection>,
    /// Cached execution order for efficient processing
    pub execution_order: Option<Vec<NodeId>>,
    /// Sample rate for audio processing
    pub sample_rate: f32,
}

impl SignalGraph {
    /// Create a new empty signal graph
    pub fn new(sample_rate: f32) -> Self {
        Self {
            nodes: HashMap::new(),
            buses: HashMap::new(),
            connections: Vec::new(),
            execution_order: None,
            sample_rate,
        }
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, node: Node) -> NodeId {
        let id = match &node {
            Node::Source { id, .. } => id.clone(),
            Node::Bus { id, .. } => return NodeId(id.0.clone()),
            Node::Processor { id, .. } => id.clone(),
            Node::Analysis { id, .. } => id.clone(),
            Node::Pattern { id, .. } => id.clone(),
            Node::Output { id } => id.clone(),
        };

        self.nodes.insert(id.clone(), node);
        self.execution_order = None; // Invalidate cache
        id
    }

    /// Add a signal bus
    pub fn add_bus(&mut self, name: String, initial_value: f32) -> BusId {
        let bus_id = BusId(name);
        self.buses.insert(bus_id.clone(), initial_value);
        bus_id
    }

    /// Connect two nodes
    pub fn connect(&mut self, from: NodeId, to: NodeId, amount: f32) {
        self.connections.push(Connection {
            from,
            to,
            amount,
            tap_type: None,
        });
        self.execution_order = None; // Invalidate cache
    }

    /// Connect with signal tap
    pub fn connect_with_tap(&mut self, from: NodeId, to: NodeId, amount: f32, tap: TapType) {
        self.connections.push(Connection {
            from,
            to,
            amount,
            tap_type: Some(tap),
        });
        self.execution_order = None; // Invalidate cache
    }

    /// Get current bus value
    pub fn get_bus_value(&self, bus_id: &BusId) -> Option<f32> {
        self.buses.get(bus_id).copied()
    }

    /// Set bus value
    pub fn set_bus_value(&mut self, bus_id: &BusId, value: f32) {
        if let Some(bus_value) = self.buses.get_mut(bus_id) {
            *bus_value = value;
        }
    }

    /// Compute execution order using topological sort
    pub fn compute_execution_order(&mut self) -> Result<(), String> {
        // Build adjacency list
        let mut graph: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        let mut in_degree: HashMap<NodeId, usize> = HashMap::new();

        // Initialize all nodes
        for node_id in self.nodes.keys() {
            graph.entry(node_id.clone()).or_default();
            in_degree.entry(node_id.clone()).or_insert(0);
        }

        // Build edges and count in-degrees
        for conn in &self.connections {
            graph
                .entry(conn.from.clone())
                .or_default()
                .push(conn.to.clone());
            *in_degree.entry(conn.to.clone()).or_insert(0) += 1;
        }

        // Topological sort using Kahn's algorithm
        let mut queue: Vec<NodeId> = Vec::new();
        let mut result: Vec<NodeId> = Vec::new();

        // Find all nodes with no incoming edges
        for (node, &degree) in &in_degree {
            if degree == 0 {
                queue.push(node.clone());
            }
        }

        while let Some(node) = queue.pop() {
            result.push(node.clone());

            if let Some(neighbors) = graph.get(&node) {
                for neighbor in neighbors {
                    if let Some(degree) = in_degree.get_mut(neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push(neighbor.clone());
                        }
                    }
                }
            }
        }

        // Check for cycles
        if result.len() != self.nodes.len() {
            return Err("Signal graph contains a cycle".to_string());
        }

        self.execution_order = Some(result);
        Ok(())
    }

    /// Get the execution order, computing it if necessary
    pub fn get_execution_order(&mut self) -> Result<&[NodeId], String> {
        if self.execution_order.is_none() {
            self.compute_execution_order()?;
        }
        Ok(self.execution_order.as_ref().unwrap())
    }

    /// Find connections from a specific node
    pub fn get_connections_from(&self, node_id: &NodeId) -> Vec<&Connection> {
        self.connections
            .iter()
            .filter(|conn| conn.from == *node_id)
            .collect()
    }

    /// Find connections to a specific node
    pub fn get_connections_to(&self, node_id: &NodeId) -> Vec<&Connection> {
        self.connections
            .iter()
            .filter(|conn| conn.to == *node_id)
            .collect()
    }

    /// Clear all nodes and connections
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.buses.clear();
        self.connections.clear();
        self.execution_order = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_signal_graph() {
        let graph = SignalGraph::new(44100.0);
        assert_eq!(graph.sample_rate, 44100.0);
        assert_eq!(graph.nodes.len(), 0);
    }

    #[test]
    fn test_add_nodes() {
        let mut graph = SignalGraph::new(44100.0);

        let osc = Node::Source {
            id: NodeId("osc1".to_string()),
            source_type: SourceType::Sine { freq: 440.0 },
        };

        let filter = Node::Processor {
            id: NodeId("filter1".to_string()),
            processor_type: ProcessorType::LowPass {
                cutoff: 1000.0,
                q: 0.7,
            },
        };

        let osc_id = graph.add_node(osc);
        let filter_id = graph.add_node(filter);

        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(osc_id, NodeId("osc1".to_string()));
        assert_eq!(filter_id, NodeId("filter1".to_string()));
    }

    #[test]
    fn test_add_bus() {
        let mut graph = SignalGraph::new(44100.0);

        let bus_id = graph.add_bus("~lfo".to_string(), 0.5);
        assert_eq!(graph.get_bus_value(&bus_id), Some(0.5));

        graph.set_bus_value(&bus_id, 0.8);
        assert_eq!(graph.get_bus_value(&bus_id), Some(0.8));
    }

    #[test]
    fn test_connections() {
        let mut graph = SignalGraph::new(44100.0);

        let osc = Node::Source {
            id: NodeId("osc1".to_string()),
            source_type: SourceType::Sine { freq: 440.0 },
        };

        let filter = Node::Processor {
            id: NodeId("filter1".to_string()),
            processor_type: ProcessorType::LowPass {
                cutoff: 1000.0,
                q: 0.7,
            },
        };

        let osc_id = graph.add_node(osc);
        let filter_id = graph.add_node(filter);

        graph.connect(osc_id.clone(), filter_id.clone(), 1.0);

        let connections = graph.get_connections_from(&osc_id);
        assert_eq!(connections.len(), 1);
        assert_eq!(connections[0].amount, 1.0);
    }

    #[test]
    fn test_topological_sort() {
        let mut graph = SignalGraph::new(44100.0);

        // Create a simple chain: osc -> filter -> output
        let osc_id = graph.add_node(Node::Source {
            id: NodeId("osc".to_string()),
            source_type: SourceType::Sine { freq: 440.0 },
        });

        let filter_id = graph.add_node(Node::Processor {
            id: NodeId("filter".to_string()),
            processor_type: ProcessorType::LowPass {
                cutoff: 1000.0,
                q: 0.7,
            },
        });

        let output_id = graph.add_node(Node::Output {
            id: NodeId("output".to_string()),
        });

        graph.connect(osc_id.clone(), filter_id.clone(), 1.0);
        graph.connect(filter_id.clone(), output_id.clone(), 1.0);

        let order = graph.get_execution_order().unwrap();

        // Verify correct order
        let osc_pos = order.iter().position(|n| *n == osc_id).unwrap();
        let filter_pos = order.iter().position(|n| *n == filter_id).unwrap();
        let output_pos = order.iter().position(|n| *n == output_id).unwrap();

        assert!(osc_pos < filter_pos);
        assert!(filter_pos < output_pos);
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = SignalGraph::new(44100.0);

        // Create a cycle: a -> b -> c -> a
        let a = graph.add_node(Node::Source {
            id: NodeId("a".to_string()),
            source_type: SourceType::Sine { freq: 440.0 },
        });

        let b = graph.add_node(Node::Processor {
            id: NodeId("b".to_string()),
            processor_type: ProcessorType::Gain { amount: 0.5 },
        });

        let c = graph.add_node(Node::Processor {
            id: NodeId("c".to_string()),
            processor_type: ProcessorType::Gain { amount: 0.5 },
        });

        graph.connect(a.clone(), b.clone(), 1.0);
        graph.connect(b.clone(), c.clone(), 1.0);
        graph.connect(c.clone(), a.clone(), 1.0); // Creates cycle

        let result = graph.compute_execution_order();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Signal graph contains a cycle");
    }

    #[test]
    fn test_signal_tap() {
        let mut graph = SignalGraph::new(44100.0);

        let source = graph.add_node(Node::Source {
            id: NodeId("bass".to_string()),
            source_type: SourceType::Saw { freq: 110.0 },
        });

        let analysis = graph.add_node(Node::Analysis {
            id: NodeId("bass_rms".to_string()),
            analysis_type: AnalysisType::RMS { window_size: 0.05 },
        });

        // Connect with RMS analysis tap
        graph.connect_with_tap(
            source.clone(),
            analysis.clone(),
            1.0,
            TapType::Analysis(AnalysisType::RMS { window_size: 0.05 }),
        );

        let connections = graph.get_connections_from(&source);
        assert_eq!(connections.len(), 1);
        assert!(connections[0].tap_type.is_some());
    }
}
