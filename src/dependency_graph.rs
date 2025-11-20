/// Dependency graph analysis for audio node execution
///
/// This module analyzes the audio processing graph to determine:
/// - Execution order (topological sort)
/// - Parallel execution opportunities (independent nodes)
/// - Cycle detection (invalid graphs)

use crate::audio_node::{AudioNode, NodeId};
use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Direction;
use std::collections::{HashMap, HashSet};

/// Represents the audio processing dependency graph
///
/// # Graph Structure
/// - Nodes: Audio processing nodes (oscillators, filters, etc.)
/// - Edges: Dependencies (data flow from input → dependent)
///
/// # Usage
/// ```ignore
/// let graph = DependencyGraph::build(&nodes)?;
/// let exec_order = graph.execution_order()?;  // Topological sort
/// let batches = graph.parallel_batches();     // Parallel execution groups
/// ```
pub struct DependencyGraph {
    /// Directed acyclic graph of node dependencies
    graph: DiGraph<NodeId, ()>,

    /// Map NodeId → NodeIndex for graph operations
    node_map: HashMap<NodeId, NodeIndex>,
}

impl DependencyGraph {
    /// Build dependency graph from audio nodes
    ///
    /// # Arguments
    /// * `nodes` - Slice of audio nodes
    ///
    /// # Returns
    /// DependencyGraph or error if graph is invalid
    ///
    /// # Errors
    /// - If a node references non-existent input
    /// - If there's a cycle in dependencies (detected during topological sort)
    pub fn build(nodes: &[Box<dyn AudioNode>]) -> Result<Self, String> {
        let mut graph = DiGraph::new();
        let mut node_map = HashMap::new();

        // Add all nodes to graph
        for (node_id, _) in nodes.iter().enumerate() {
            let idx = graph.add_node(node_id);
            node_map.insert(node_id, idx);
        }

        // Add edges for dependencies
        for (node_id, node) in nodes.iter().enumerate() {
            let dependent_idx = node_map[&node_id];

            for input_id in node.input_nodes() {
                if let Some(&input_idx) = node_map.get(&input_id) {
                    // Edge: input → dependent (data flows this direction)
                    graph.add_edge(input_idx, dependent_idx, ());
                } else {
                    return Err(format!(
                        "Node {} references non-existent input node {}",
                        node_id, input_id
                    ));
                }
            }
        }

        Ok(Self { graph, node_map })
    }

    /// Get topologically sorted execution order
    ///
    /// Nodes are returned in an order where all dependencies are processed
    /// before their dependents. This is the order for sequential execution.
    ///
    /// # Returns
    /// Vec of NodeIds in execution order, or error if cycle detected
    ///
    /// # Errors
    /// - If there's a cycle in the dependency graph (invalid audio graph)
    pub fn execution_order(&self) -> Result<Vec<NodeId>, String> {
        match toposort(&self.graph, None) {
            Ok(order) => Ok(order.iter().map(|&idx| self.graph[idx]).collect()),
            Err(cycle) => Err(format!(
                "Cycle detected in audio graph at node {}",
                self.graph[cycle.node_id()]
            )),
        }
    }

    /// Group nodes into parallel execution batches
    ///
    /// Nodes in the same batch have no dependencies on each other and can
    /// be executed in parallel. Batches must be executed sequentially (batch N
    /// depends on outputs from batch N-1).
    ///
    /// # Algorithm
    /// 1. Start with topologically sorted order
    /// 2. For each node, check if all dependencies are in previous batches
    /// 3. If yes, add to current batch; if no, start new batch
    ///
    /// # Returns
    /// Vec of batches, where each batch is a Vec of NodeIds that can run in parallel
    ///
    /// # Example
    /// ```text
    /// Graph:
    ///   A → C → E
    ///   B → D → E
    ///
    /// Batches:
    ///   [A, B]     <- No dependencies, can run in parallel
    ///   [C, D]     <- Both depend on batch 0, can run in parallel
    ///   [E]        <- Depends on batch 1, runs alone
    /// ```
    pub fn parallel_batches(&self) -> Vec<Vec<NodeId>> {
        let order = match self.execution_order() {
            Ok(order) => order,
            Err(_) => return vec![],  // Invalid graph, return empty
        };

        // Track which batch level each node belongs to
        let mut node_batch: HashMap<NodeId, usize> = HashMap::new();
        let mut batches: Vec<Vec<NodeId>> = Vec::new();

        for &node_id in &order {
            let node_idx = self.node_map[&node_id];

            // Find the maximum batch level of all dependencies
            let max_dep_batch = self
                .graph
                .neighbors_directed(node_idx, Direction::Incoming)
                .map(|dep_idx| {
                    let dep_id = self.graph[dep_idx];
                    *node_batch.get(&dep_id).unwrap_or(&0)
                })
                .max()
                .unwrap_or(0);

            // This node goes in the batch AFTER its dependencies
            let this_batch = if self.graph.neighbors_directed(node_idx, Direction::Incoming).count() == 0 {
                // No dependencies → batch 0
                0
            } else {
                // Has dependencies → one batch after the max dependency
                max_dep_batch + 1
            };

            // Ensure we have enough batches
            while batches.len() <= this_batch {
                batches.push(Vec::new());
            }

            // Add node to its batch
            batches[this_batch].push(node_id);
            node_batch.insert(node_id, this_batch);
        }

        batches
    }

    /// Get all direct dependencies of a node
    ///
    /// # Arguments
    /// * `node_id` - NodeId to query
    ///
    /// # Returns
    /// Vec of NodeIds that this node directly depends on
    pub fn dependencies(&self, node_id: NodeId) -> Vec<NodeId> {
        if let Some(&node_idx) = self.node_map.get(&node_id) {
            self.graph
                .neighbors_directed(node_idx, Direction::Incoming)
                .map(|dep_idx| self.graph[dep_idx])
                .collect()
        } else {
            vec![]
        }
    }

    /// Get all nodes that depend on this node
    ///
    /// # Arguments
    /// * `node_id` - NodeId to query
    ///
    /// # Returns
    /// Vec of NodeIds that depend on this node
    pub fn dependents(&self, node_id: NodeId) -> Vec<NodeId> {
        if let Some(&node_idx) = self.node_map.get(&node_id) {
            self.graph
                .neighbors_directed(node_idx, Direction::Outgoing)
                .map(|dep_idx| self.graph[dep_idx])
                .collect()
        } else {
            vec![]
        }
    }

    /// Check if graph has a cycle
    ///
    /// # Returns
    /// true if graph is acyclic (valid), false if cycle detected
    pub fn is_acyclic(&self) -> bool {
        toposort(&self.graph, None).is_ok()
    }

    /// Get number of nodes in graph
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Get number of edges (dependencies) in graph
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_node::{AudioNode, ProcessContext};

    // Mock audio node for testing
    struct MockNode {
        id: NodeId,
        inputs: Vec<NodeId>,
    }

    impl AudioNode for MockNode {
        fn process_block(
            &mut self,
            _inputs: &[&[f32]],
            _output: &mut [f32],
            _sample_rate: f32,
            _context: &ProcessContext,
        ) {
        }

        fn input_nodes(&self) -> Vec<NodeId> {
            self.inputs.clone()
        }

        fn name(&self) -> &str {
            "MockNode"
        }
    }

    #[test]
    fn test_simple_linear_graph() {
        // Graph: 0 → 1 → 2
        let nodes: Vec<Box<dyn AudioNode>> = vec![
            Box::new(MockNode {
                id: 0,
                inputs: vec![],
            }),
            Box::new(MockNode {
                id: 1,
                inputs: vec![0],
            }),
            Box::new(MockNode {
                id: 2,
                inputs: vec![1],
            }),
        ];

        let graph = DependencyGraph::build(&nodes).unwrap();
        let order = graph.execution_order().unwrap();

        assert_eq!(order, vec![0, 1, 2]);
        assert!(graph.is_acyclic());
    }

    #[test]
    fn test_parallel_branches() {
        // Graph:
        //   0 → 1 → 3
        //   0 → 2 → 3
        let nodes: Vec<Box<dyn AudioNode>> = vec![
            Box::new(MockNode {
                id: 0,
                inputs: vec![],
            }),
            Box::new(MockNode {
                id: 1,
                inputs: vec![0],
            }),
            Box::new(MockNode {
                id: 2,
                inputs: vec![0],
            }),
            Box::new(MockNode {
                id: 3,
                inputs: vec![1, 2],
            }),
        ];

        let graph = DependencyGraph::build(&nodes).unwrap();
        let batches = graph.parallel_batches();

        // Batch 0: [0] (source)
        // Batch 1: [1, 2] (both depend on 0, can run in parallel)
        // Batch 2: [3] (depends on 1 and 2)
        assert_eq!(batches.len(), 3);
        assert_eq!(batches[0], vec![0]);
        assert_eq!(batches[1].len(), 2);
        assert!(batches[1].contains(&1) && batches[1].contains(&2));
        assert_eq!(batches[2], vec![3]);
    }

    #[test]
    fn test_complex_parallel_graph() {
        // Graph:
        //   0 → 2 → 4
        //   1 → 3 → 4
        // (Two independent chains merging)
        let nodes: Vec<Box<dyn AudioNode>> = vec![
            Box::new(MockNode {
                id: 0,
                inputs: vec![],
            }),
            Box::new(MockNode {
                id: 1,
                inputs: vec![],
            }),
            Box::new(MockNode {
                id: 2,
                inputs: vec![0],
            }),
            Box::new(MockNode {
                id: 3,
                inputs: vec![1],
            }),
            Box::new(MockNode {
                id: 4,
                inputs: vec![2, 3],
            }),
        ];

        let graph = DependencyGraph::build(&nodes).unwrap();
        let batches = graph.parallel_batches();

        // Batch 0: [0, 1] (sources)
        // Batch 1: [2, 3] (both depend on batch 0)
        // Batch 2: [4] (depends on both 2 and 3)
        assert_eq!(batches.len(), 3);
        assert_eq!(batches[0].len(), 2);
        assert_eq!(batches[1].len(), 2);
        assert_eq!(batches[2].len(), 1);
    }

    #[test]
    fn test_cycle_detection() {
        // Graph with cycle: 0 → 1 → 2 → 0
        let nodes: Vec<Box<dyn AudioNode>> = vec![
            Box::new(MockNode {
                id: 0,
                inputs: vec![2],  // Cycle!
            }),
            Box::new(MockNode {
                id: 1,
                inputs: vec![0],
            }),
            Box::new(MockNode {
                id: 2,
                inputs: vec![1],
            }),
        ];

        let graph = DependencyGraph::build(&nodes).unwrap();
        assert!(!graph.is_acyclic());
        assert!(graph.execution_order().is_err());
    }

    #[test]
    fn test_invalid_reference() {
        // Node 1 references non-existent node 99
        let nodes: Vec<Box<dyn AudioNode>> = vec![
            Box::new(MockNode {
                id: 0,
                inputs: vec![],
            }),
            Box::new(MockNode {
                id: 1,
                inputs: vec![99],  // Invalid!
            }),
        ];

        let result = DependencyGraph::build(&nodes);
        assert!(result.is_err());
    }

    #[test]
    fn test_dependencies_and_dependents() {
        // Graph: 0 → 1 → 2
        let nodes: Vec<Box<dyn AudioNode>> = vec![
            Box::new(MockNode {
                id: 0,
                inputs: vec![],
            }),
            Box::new(MockNode {
                id: 1,
                inputs: vec![0],
            }),
            Box::new(MockNode {
                id: 2,
                inputs: vec![1],
            }),
        ];

        let graph = DependencyGraph::build(&nodes).unwrap();

        // Node 1 depends on 0
        assert_eq!(graph.dependencies(1), vec![0]);

        // Node 0 has dependent 1
        assert_eq!(graph.dependents(0), vec![1]);

        // Node 2 depends on 1
        assert_eq!(graph.dependencies(2), vec![1]);

        // Node 2 has no dependents
        assert_eq!(graph.dependents(2).len(), 0);
    }
}
