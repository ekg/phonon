# DAW-Style Buffer Passing Architecture - Implementation Plan

## Executive Summary

**Goal**: Replace sample-by-sample graph traversal with block-based buffer passing.

**Current Problem**: Graph traversed 512 times per block → fundamentally inefficient.

**Solution**: Process entire 512-sample buffers at once, traverse graph ONCE per block, enable parallel independent nodes.

**Timeline**: 6-8 weeks of focused work.

---

## Phase 1: Design & Foundation (Week 1-2)

### 1.1 Define AudioNode Trait

**New core abstraction** - every audio-producing entity implements this:

```rust
// src/audio_node.rs (NEW FILE)

/// Block-based audio processing trait
/// Replaces sample-by-sample SignalNode evaluation
pub trait AudioNode: Send {
    /// Process an entire block of audio
    ///
    /// # Arguments
    /// * `inputs` - Input buffers from dependent nodes (zero-copy via &[f32])
    /// * `output` - Output buffer to write to (512 samples)
    /// * `sample_rate` - Current sample rate (44100.0)
    /// * `context` - Processing context (cycle position, time, etc.)
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        context: &ProcessContext,
    );

    /// Return list of input node IDs this node depends on
    fn input_nodes(&self) -> Vec<NodeId>;

    /// Called once per block before processing (for pattern evaluation)
    fn prepare_block(&mut self, context: &ProcessContext) {}
}

/// Context passed to all nodes during block processing
pub struct ProcessContext {
    pub cycle_position: Fraction,
    pub sample_offset: usize,      // Which sample in the cycle
    pub block_size: usize,         // Usually 512
    pub tempo: f64,
}
```

**Why this works**:
- `inputs: &[&[f32]]` - Zero-copy input buffers (already computed)
- `output: &mut [f32]` - Write directly to output buffer
- `process_block()` called ONCE per node per block
- Node returns dependencies → enables topological sort

### 1.2 Buffer Management System

**Challenge**: Safely share buffers between nodes without copying.

**Solution**: Arc<Vec<f32>> + buffer pool for reuse.

```rust
// src/buffer_manager.rs (NEW FILE)

use std::sync::Arc;

/// Manages audio buffers for zero-copy sharing
pub struct BufferManager {
    /// Pre-allocated buffer pool (reuse across blocks)
    pool: Vec<Vec<f32>>,
    pool_size: usize,
    buffer_size: usize,
}

impl BufferManager {
    pub fn new(pool_size: usize, buffer_size: usize) -> Self {
        let mut pool = Vec::with_capacity(pool_size);
        for _ in 0..pool_size {
            pool.push(vec![0.0; buffer_size]);
        }
        Self { pool, pool_size, buffer_size }
    }

    /// Get a zeroed buffer from pool (or allocate if pool empty)
    pub fn get_buffer(&mut self) -> Vec<f32> {
        self.pool.pop().unwrap_or_else(|| vec![0.0; self.buffer_size])
    }

    /// Return buffer to pool for reuse
    pub fn return_buffer(&mut self, mut buffer: Vec<f32>) {
        if self.pool.len() < self.pool_size {
            // Clear and return to pool
            buffer.iter_mut().for_each(|x| *x = 0.0);
            self.pool.push(buffer);
        }
        // Otherwise drop (pool full)
    }
}

/// Node output storage - shared across dependents
pub struct NodeOutput {
    pub buffer: Arc<Vec<f32>>,
    pub ready: bool,
}
```

**Why Arc?**
- Multiple nodes can read same input buffer simultaneously
- Zero-copy: only Arc pointer is cloned, not buffer data
- Thread-safe: Arc uses atomic reference counting

### 1.3 Dependency Graph & Topological Sort

**Challenge**: Determine execution order + find parallelizable nodes.

**Solution**: Build dependency graph, topologically sort, identify parallel groups.

```rust
// src/dependency_graph.rs (NEW FILE)

use std::collections::{HashMap, HashSet, VecDeque};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::algo::toposort;

pub type NodeId = usize;

/// Represents the audio processing dependency graph
pub struct DependencyGraph {
    /// Directed acyclic graph of node dependencies
    graph: DiGraph<NodeId, ()>,
    /// Map NodeId → NodeIndex for graph operations
    node_map: HashMap<NodeId, NodeIndex>,
}

impl DependencyGraph {
    /// Build graph from nodes
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
                }
            }
        }

        Ok(Self { graph, node_map })
    }

    /// Get topologically sorted execution order
    pub fn execution_order(&self) -> Result<Vec<NodeId>, String> {
        match toposort(&self.graph, None) {
            Ok(order) => Ok(order.iter().map(|&idx| self.graph[idx]).collect()),
            Err(_) => Err("Cycle detected in audio graph!".to_string()),
        }
    }

    /// Group nodes into parallel execution batches
    /// Nodes in same batch have no dependencies on each other
    pub fn parallel_batches(&self) -> Vec<Vec<NodeId>> {
        let order = self.execution_order().unwrap();
        let mut batches = Vec::new();
        let mut processed = HashSet::new();

        for &node_id in &order {
            let node_idx = self.node_map[&node_id];

            // Check if all dependencies are processed
            let dependencies_ready = self.graph
                .neighbors_directed(node_idx, petgraph::Direction::Incoming)
                .all(|dep_idx| processed.contains(&self.graph[dep_idx]));

            if dependencies_ready {
                // Can execute in current batch
                if batches.is_empty() || !dependencies_ready {
                    batches.push(Vec::new());
                }
                batches.last_mut().unwrap().push(node_id);
            } else {
                // Need new batch (dependencies not ready)
                batches.push(vec![node_id]);
            }

            processed.insert(node_id);
        }

        batches
    }
}
```

**Why petgraph?**
- Production-ready topological sort
- Detects cycles (invalid audio graphs)
- Efficient graph algorithms

**Add to Cargo.toml**:
```toml
[dependencies]
petgraph = "0.6"
```

---

## Phase 2: Core Node Implementations (Week 2-4)

### 2.1 Convert SignalNode Variants to AudioNode Implementations

**Strategy**: One-by-one migration of SignalNode enum variants.

**Start with simplest**: Constant, BusRef, Addition, Multiplication

#### Example: Constant Node

```rust
// src/nodes/constant.rs (NEW FILE)

use crate::audio_node::{AudioNode, ProcessContext};
use crate::unified_graph::NodeId;

/// Constant value node (e.g., `5.0`, `0.5`)
pub struct ConstantNode {
    value: f32,
}

impl ConstantNode {
    pub fn new(value: f32) -> Self {
        Self { value }
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
        vec![]  // No dependencies
    }
}
```

#### Example: Addition Node

```rust
// src/nodes/addition.rs (NEW FILE)

use crate::audio_node::{AudioNode, ProcessContext};
use crate::unified_graph::NodeId;

/// Addition node: out = a + b
pub struct AdditionNode {
    input_a: NodeId,
    input_b: NodeId,
}

impl AdditionNode {
    pub fn new(input_a: NodeId, input_b: NodeId) -> Self {
        Self { input_a, input_b }
    }
}

impl AudioNode for AdditionNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        // inputs[0] = buffer from input_a
        // inputs[1] = buffer from input_b
        let buf_a = inputs[0];
        let buf_b = inputs[1];

        // Vectorized addition
        for i in 0..output.len() {
            output[i] = buf_a[i] + buf_b[i];
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input_a, self.input_b]
    }
}
```

#### Example: Oscillator Node (Stateful)

```rust
// src/nodes/oscillator.rs (NEW FILE)

use crate::audio_node::{AudioNode, ProcessContext};
use crate::unified_graph::NodeId;
use std::f32::consts::PI;

/// Oscillator node with pattern-controlled frequency
pub struct OscillatorNode {
    freq_input: NodeId,     // NodeId providing frequency values
    waveform: Waveform,
    phase: f32,             // Internal state (0.0 to 1.0)
}

impl OscillatorNode {
    pub fn new(freq_input: NodeId, waveform: Waveform) -> Self {
        Self {
            freq_input,
            waveform,
            phase: 0.0,
        }
    }
}

impl AudioNode for OscillatorNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        let freq_buffer = inputs[0];  // Frequency modulation buffer

        for i in 0..output.len() {
            let freq = freq_buffer[i];

            // Generate sample based on waveform
            output[i] = match self.waveform {
                Waveform::Sine => (self.phase * 2.0 * PI).sin(),
                Waveform::Saw => 2.0 * self.phase - 1.0,
                Waveform::Square => if self.phase < 0.5 { 1.0 } else { -1.0 },
                Waveform::Triangle => {
                    if self.phase < 0.5 {
                        4.0 * self.phase - 1.0
                    } else {
                        -4.0 * self.phase + 3.0
                    }
                }
            };

            // Advance phase
            self.phase += freq / sample_rate;
            while self.phase >= 1.0 {
                self.phase -= 1.0;
            }
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.freq_input]
    }
}
```

#### Example: Reverb Node (Effect with State)

```rust
// src/nodes/reverb.rs (NEW FILE)

use crate::audio_node::{AudioNode, ProcessContext};
use crate::unified_graph::NodeId;
use crate::effects::ReverbState;  // Existing implementation

/// Reverb effect node
pub struct ReverbNode {
    input: NodeId,
    mix_input: NodeId,       // Pattern-controlled mix amount
    room_size_input: NodeId,
    state: ReverbState,      // Stateful delay lines
}

impl ReverbNode {
    pub fn new(input: NodeId, mix: NodeId, room_size: NodeId) -> Self {
        Self {
            input,
            mix_input: mix,
            room_size_input: room_size,
            state: ReverbState::new(44100.0),
        }
    }
}

impl AudioNode for ReverbNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        let dry_buffer = inputs[0];       // Input signal
        let mix_buffer = inputs[1];       // Mix amount (0.0 to 1.0)
        let room_size_buffer = inputs[2]; // Room size (0.0 to 1.0)

        for i in 0..output.len() {
            let dry = dry_buffer[i];
            let mix = mix_buffer[i].clamp(0.0, 1.0);
            let room_size = room_size_buffer[i].clamp(0.0, 1.0);

            // Update reverb parameters (if changed)
            self.state.set_room_size(room_size);

            // Process through reverb
            let wet = self.state.process(dry);

            // Mix dry/wet
            output[i] = dry * (1.0 - mix) + wet * mix;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.mix_input, self.room_size_input]
    }
}
```

### 2.2 Migration Strategy for Existing SignalNode Enum

**Goal**: Don't break everything at once.

**Approach**: Coexistence period where both systems work.

```rust
// src/unified_graph.rs

pub struct UnifiedSignalGraph {
    // OLD SYSTEM (will be removed)
    nodes: Vec<Option<Rc<SignalNode>>>,

    // NEW SYSTEM (being built)
    audio_nodes: Vec<Box<dyn AudioNode>>,
    node_outputs: HashMap<NodeId, NodeOutput>,
    dependency_graph: DependencyGraph,
    buffer_manager: BufferManager,

    // Flag to switch between systems
    use_block_processing: bool,
}

impl UnifiedSignalGraph {
    pub fn process_buffer(&mut self, buffer: &mut [f32]) {
        if self.use_block_processing {
            self.process_buffer_block_based(buffer);
        } else {
            self.process_buffer_sample_based(buffer);  // OLD CODE
        }
    }

    fn process_buffer_block_based(&mut self, buffer: &mut [f32]) {
        // NEW IMPLEMENTATION (Phase 3)
    }

    fn process_buffer_sample_based(&mut self, buffer: &mut [f32]) {
        // EXISTING CODE - unchanged during migration
        for i in 0..buffer.len() {
            // ... current sample-by-sample implementation
        }
    }
}
```

**Benefits**:
- Can test new system alongside old
- Gradual migration (one node type at a time)
- Can A/B compare performance
- Rollback if issues arise

---

## Phase 3: Block-Based Graph Execution (Week 4-5)

### 3.1 Implement Block Processor

**Core execution loop** - replaces sample-by-sample evaluation.

```rust
// src/block_processor.rs (NEW FILE)

use crate::audio_node::{AudioNode, ProcessContext};
use crate::buffer_manager::{BufferManager, NodeOutput};
use crate::dependency_graph::DependencyGraph;
use std::collections::HashMap;
use std::sync::Arc;
use rayon::prelude::*;

pub struct BlockProcessor {
    nodes: Vec<Box<dyn AudioNode>>,
    dependency_graph: DependencyGraph,
    node_outputs: HashMap<NodeId, NodeOutput>,
    buffer_manager: BufferManager,
    output_node: NodeId,
}

impl BlockProcessor {
    pub fn new(
        nodes: Vec<Box<dyn AudioNode>>,
        output_node: NodeId,
        buffer_size: usize,
    ) -> Result<Self, String> {
        let dependency_graph = DependencyGraph::build(&nodes)?;
        let node_outputs = HashMap::new();
        let buffer_manager = BufferManager::new(nodes.len() * 2, buffer_size);

        Ok(Self {
            nodes,
            dependency_graph,
            node_outputs,
            buffer_manager,
            output_node,
        })
    }

    /// Process entire block - graph traversed ONCE
    pub fn process_block(
        &mut self,
        output: &mut [f32],
        context: &ProcessContext,
    ) -> Result<(), String> {
        let buffer_size = output.len();

        // Phase 1: Prepare all nodes (pattern evaluation)
        for node in &mut self.nodes {
            node.prepare_block(context);
        }

        // Phase 2: Get execution order
        let exec_order = self.dependency_graph.execution_order()?;

        // Phase 3: Process nodes in topological order
        for &node_id in &exec_order {
            // Gather input buffers
            let input_ids = self.nodes[node_id].input_nodes();
            let input_buffers: Vec<&[f32]> = input_ids
                .iter()
                .map(|&id| {
                    self.node_outputs
                        .get(&id)
                        .map(|output| output.buffer.as_slice())
                        .unwrap_or(&[])
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

            // Store output for dependents
            self.node_outputs.insert(
                node_id,
                NodeOutput {
                    buffer: Arc::new(node_buffer),
                    ready: true,
                },
            );
        }

        // Phase 4: Copy final output
        if let Some(final_output) = self.node_outputs.get(&self.output_node) {
            output.copy_from_slice(&final_output.buffer);
        } else {
            return Err("Output node not processed".to_string());
        }

        // Phase 5: Return buffers to pool
        for (_, node_output) in self.node_outputs.drain() {
            if let Ok(buffer) = Arc::try_unwrap(node_output.buffer) {
                self.buffer_manager.return_buffer(buffer);
            }
        }

        Ok(())
    }

    /// Process block with parallel independent nodes
    pub fn process_block_parallel(
        &mut self,
        output: &mut [f32],
        context: &ProcessContext,
    ) -> Result<(), String> {
        // Phase 1: Prepare all nodes
        for node in &mut self.nodes {
            node.prepare_block(context);
        }

        // Phase 2: Get parallel execution batches
        let batches = self.dependency_graph.parallel_batches();

        // Phase 3: Process batches (nodes in batch can run in parallel)
        for batch in batches {
            // Process all nodes in this batch in parallel
            let batch_outputs: Vec<_> = batch
                .par_iter()  // Rayon parallel iterator
                .map(|&node_id| {
                    // Gather inputs (already computed in previous batches)
                    let input_ids = self.nodes[node_id].input_nodes();
                    let input_buffers: Vec<Vec<f32>> = input_ids
                        .iter()
                        .map(|&id| {
                            self.node_outputs
                                .get(&id)
                                .map(|output| (*output.buffer).clone())
                                .unwrap_or_else(|| vec![0.0; output.len()])
                        })
                        .collect();

                    // Process node
                    let mut node_buffer = vec![0.0; output.len()];
                    let input_refs: Vec<&[f32]> =
                        input_buffers.iter().map(|b| b.as_slice()).collect();

                    self.nodes[node_id].process_block(
                        &input_refs,
                        &mut node_buffer,
                        context.sample_rate,
                        context,
                    );

                    (node_id, Arc::new(node_buffer))
                })
                .collect();

            // Store batch outputs
            for (node_id, buffer) in batch_outputs {
                self.node_outputs.insert(
                    node_id,
                    NodeOutput { buffer, ready: true },
                );
            }
        }

        // Phase 4: Copy final output
        if let Some(final_output) = self.node_outputs.get(&self.output_node) {
            output.copy_from_slice(&final_output.buffer);
        }

        Ok(())
    }
}
```

**Key Features**:
- Graph traversed ONCE per block (not 512 times!)
- Buffers passed between nodes (zero-copy via Arc)
- Sequential version: simple, correct
- Parallel version: independent nodes in same batch run simultaneously

### 3.2 Pattern Evaluation Integration

**Challenge**: Patterns trigger sample-accurate events, but we're processing 512-sample blocks.

**Solution**: Pre-evaluate patterns for entire block in `prepare_block()`.

```rust
// Example: Sample trigger node

use crate::pattern::{Pattern, State, TimeSpan};

pub struct SampleTriggerNode {
    pattern: Pattern<String>,
    voice_manager: VoiceManager,
    // Pre-computed events for current block
    block_events: Vec<(usize, String)>,  // (sample_offset, sample_name)
}

impl AudioNode for SampleTriggerNode {
    fn prepare_block(&mut self, context: &ProcessContext) {
        // Query pattern for events in this block's time range
        let cycle_start = context.cycle_position;
        let cycle_end = context.cycle_position + Fraction::from_float(
            (context.block_size as f64) / (context.sample_rate as f64 * context.tempo)
        );

        let state = State {
            span: TimeSpan::new(cycle_start, cycle_end),
            controls: HashMap::new(),
        };

        let events = self.pattern.query(&state);

        // Convert events to sample-accurate triggers
        self.block_events.clear();
        for event in events {
            let sample_offset = /* calculate from event.span */;
            self.block_events.push((sample_offset, event.value.clone()));
        }

        // Trigger voices in voice manager
        for (offset, sample_name) in &self.block_events {
            self.voice_manager.trigger_voice(*offset, sample_name);
        }
    }

    fn process_block(
        &mut self,
        _inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        // Voices already triggered in prepare_block
        // Now just render them
        self.voice_manager.render_block(output);
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![]  // No dependencies (source node)
    }
}
```

**Why This Works**:
- Pattern evaluation happens ONCE per block
- Events converted to sample-accurate offsets
- Voice manager renders entire block
- Sample-accurate timing preserved

---

## Phase 4: Integration & Testing (Week 5-6)

### 4.1 Update Compiler to Generate AudioNodes

**Change compositional_compiler.rs** to build new node types.

```rust
// src/compositional_compiler.rs

use crate::nodes::*;

impl CompilerContext {
    pub fn compile_sine(&mut self, args: Vec<Expr>) -> Result<NodeId, String> {
        // Parse frequency argument
        let freq_expr = args.get(0).ok_or("sine requires frequency")?;
        let freq_node = self.compile_expr(freq_expr)?;

        // Create oscillator node
        let node = Box::new(OscillatorNode::new(freq_node, Waveform::Sine));
        let node_id = self.graph.audio_nodes.len();
        self.graph.audio_nodes.push(node);

        Ok(node_id)
    }

    pub fn compile_lpf(&mut self, args: Vec<Expr>) -> Result<NodeId, String> {
        // args[0] = input signal
        // args[1] = cutoff frequency
        // args[2] = resonance
        let input = self.compile_expr(&args[0])?;
        let cutoff = self.compile_expr(&args[1])?;
        let resonance = self.compile_expr(&args[2])?;

        let node = Box::new(LowPassFilterNode::new(input, cutoff, resonance));
        let node_id = self.graph.audio_nodes.len();
        self.graph.audio_nodes.push(node);

        Ok(node_id)
    }

    // ... similar for all other functions
}
```

### 4.2 Test Suite Updates

**Add block-processing tests** alongside existing sample-based tests.

```rust
// tests/test_block_processing.rs

#[test]
fn test_sine_oscillator_block() {
    let code = r#"
        tempo: 0.5
        out: sine 440
    "#;

    // Render with block processing enabled
    let audio = render_dsl_block_mode(code, 1.0);

    // Level 1: Frequency analysis (should have peak at 440 Hz)
    let spectrum = fft_analyze(&audio, 44100.0);
    assert_frequency_peak(&spectrum, 440.0, 10.0);

    // Level 2: RMS check
    let rms = calculate_rms(&audio);
    assert!(rms > 0.6 && rms < 0.8);  // Sine wave RMS ≈ 0.707
}

#[test]
fn test_sample_trigger_timing_block() {
    let code = r#"
        tempo: 0.5
        out: s "bd*4"
    "#;

    let audio = render_dsl_block_mode(code, 4.0);  // 4 cycles
    let onsets = detect_audio_events(&audio, 44100.0, 0.1);

    // Should have 16 events (4 per cycle × 4 cycles)
    assert_eq!(onsets.len(), 16);

    // Verify sample-accurate timing (even though processing in blocks)
    let expected_interval = (44100.0 / 2.0) / 4.0;  // tempo=2.0, 4 events/cycle
    for i in 1..onsets.len() {
        let interval = onsets[i].time - onsets[i-1].time;
        assert!((interval - expected_interval).abs() < 10.0);  // Within 10 samples
    }
}

#[test]
fn test_block_vs_sample_equivalence() {
    let code = r#"
        tempo: 0.5
        ~lfo: sine 0.5
        out: saw 110 # lpf (~lfo * 1000 + 500) 0.8
    "#;

    // Render with both methods
    let sample_mode = render_dsl_sample_mode(code, 2.0);
    let block_mode = render_dsl_block_mode(code, 2.0);

    // Results should be nearly identical
    assert_buffers_close(&sample_mode, &block_mode, 0.001);
}
```

### 4.3 Performance Benchmarks

**Verify we get expected gains.**

```rust
// tests/bench_block_processing.rs

#[test]
fn bench_complex_fx_chain() {
    let code = r#"
        tempo: 0.5
        ~drums: s "bd*8 sn*8 hh*16 cp*4"
        ~drums_verb: ~drums # reverb 0.5 0.8
        ~drums_delay: ~drums # delay 0.25 0.6
        ~drums_wet: ~drums_verb * 0.3 + ~drums_delay * 0.2
        out: ~drums + ~drums_wet
    "#;

    // Benchmark sample-mode
    let start = std::time::Instant::now();
    render_dsl_sample_mode(code, 10.0);
    let sample_time = start.elapsed();

    // Benchmark block-mode
    let start = std::time::Instant::now();
    render_dsl_block_mode(code, 10.0);
    let block_time = start.elapsed();

    println!("Sample-mode: {:?}", sample_time);
    println!("Block-mode: {:?}", block_time);
    println!("Speedup: {:.2}x", sample_time.as_secs_f64() / block_time.as_secs_f64());

    // Should be 2-5x faster
    assert!(block_time < sample_time / 2);
}
```

---

## Phase 5: Parallel Node Execution (Week 6-7)

### 5.1 Enable Parallel Batches

**Switch from sequential to parallel execution.**

```rust
// In UnifiedSignalGraph::process_buffer_block_based()

impl UnifiedSignalGraph {
    fn process_buffer_block_based(&mut self, buffer: &mut [f32]) {
        let context = ProcessContext {
            cycle_position: self.cycle_position,
            sample_offset: 0,
            block_size: buffer.len(),
            tempo: self.tempo,
        };

        // Use parallel processor
        self.block_processor
            .process_block_parallel(buffer, &context)
            .expect("Block processing failed");
    }
}
```

### 5.2 Test Parallel Scenarios

**Create tests that benefit from parallelism.**

```rust
#[test]
fn test_parallel_fx_chains() {
    let code = r#"
        tempo: 0.5
        ~input: s "bd*8"

        -- Four independent FX chains (can run in parallel)
        ~fx1: ~input # reverb 0.3 0.5
        ~fx2: ~input # delay 0.25 0.6
        ~fx3: ~input # chorus 0.5 2.0
        ~fx4: ~input # distortion 0.7

        out: ~fx1 * 0.25 + ~fx2 * 0.25 + ~fx3 * 0.25 + ~fx4 * 0.25
    "#;

    // Render with parallel processing
    let start = std::time::Instant::now();
    let audio = render_dsl_parallel(code, 10.0);
    let parallel_time = start.elapsed();

    // Render with sequential processing
    let start = std::time::Instant::now();
    let audio_seq = render_dsl_sequential(code, 10.0);
    let sequential_time = start.elapsed();

    println!("Sequential: {:?}", sequential_time);
    println!("Parallel: {:?}", parallel_time);
    println!("Speedup: {:.2}x", sequential_time.as_secs_f64() / parallel_time.as_secs_f64());

    // Audio should be identical
    assert_buffers_close(&audio, &audio_seq, 0.001);

    // Should see speedup (4 independent chains on 16 cores)
    assert!(parallel_time < sequential_time);
}
```

### 5.3 Multi-core Utilization Test

**Verify all cores are used.**

```rust
#[test]
fn test_full_core_utilization() {
    let code = r#"
        tempo: 0.5

        -- 16 independent oscillators (one per core)
        ~osc1: sine 110 # lpf 500 0.8
        ~osc2: saw 220 # lpf 600 0.7
        ~osc3: square 330 # lpf 700 0.8
        ~osc4: sine 440 # lpf 800 0.7
        // ... 12 more

        out: (~osc1 + ~osc2 + ~osc3 + ~osc4 + ...) * 0.0625
    "#;

    // Monitor CPU usage during render
    let start = std::time::Instant::now();
    render_dsl_parallel(code, 10.0);
    let elapsed = start.elapsed();

    println!("Render time: {:?}", elapsed);

    // With 16 cores, should complete much faster than sequential
    // (Check htop during test - should see all cores active)
}
```

---

## Phase 6: Migration & Cleanup (Week 7-8)

### 6.1 Feature Parity Checklist

**Before removing old system, verify new system can do everything:**

- [ ] All oscillator types (sine, saw, square, triangle)
- [ ] All filter types (lpf, hpf, bpf, notch)
- [ ] All effects (reverb, delay, distortion, chorus, compressor, bitcrush)
- [ ] Sample triggering with voice manager
- [ ] Pattern-controlled parameters
- [ ] Bus routing (~bus: expr)
- [ ] Multi-output (out:, out1:, out2:)
- [ ] All pattern transforms (fast, slow, rev, every, etc.)
- [ ] Live mode with hot-reload
- [ ] Render mode with multi-threading

**Test all 385 existing tests** with new system enabled.

### 6.2 Remove Old System

**Once new system passes all tests:**

```rust
// src/unified_graph.rs

pub struct UnifiedSignalGraph {
    // REMOVE: Old sample-by-sample system
    // nodes: Vec<Option<Rc<SignalNode>>>,

    // KEEP: New block-based system
    audio_nodes: Vec<Box<dyn AudioNode>>,
    block_processor: BlockProcessor,
    // ...
}

impl UnifiedSignalGraph {
    pub fn process_buffer(&mut self, buffer: &mut [f32]) {
        // REMOVE: Old implementation
        // if self.use_block_processing { ... } else { ... }

        // KEEP: Only block-based now
        self.process_buffer_block_based(buffer);
    }
}
```

**Delete old code**:
- Sample-by-sample `eval_node()` recursion
- Old `SignalNode` enum (replaced by AudioNode trait)
- Temporary compatibility flags

### 6.3 Documentation Updates

**Update all docs to reflect new architecture:**

- `ARCHITECTURE_DISCUSSION.md` - Mark as historical, link to new architecture
- Create `DAW_ARCHITECTURE.md` - Explain new design
- Update `CLAUDE.md` - Remove "CRITICAL REFACTOR IN PROGRESS" section
- Update `ROADMAP.md` - Mark architecture refactor complete
- Create tutorial: "How to Add a New AudioNode"

---

## Success Metrics

### Performance Goals (Must Achieve)

1. **Graph Traversal**: 512x reduction (traversed ONCE per block, not 512 times)
2. **Complex FX Pipeline**: 100+ voices + 4 parallel FX chains in < 11.6ms
3. **Multi-core Utilization**: All 16 cores active (1500%+ CPU usage)
4. **Headroom**: 70%+ headroom on complex scenarios
5. **Latency**: Block processing time < 5ms average (70% of real-time constraint)

### Correctness Goals (Must Pass)

1. All 385 existing tests pass with new system
2. Block-based output identical to sample-based (within 0.001 tolerance)
3. Sample-accurate timing preserved (onset detection tests)
4. Pattern-controlled parameters work (can modulate any parameter with pattern)
5. No audio glitches, clicks, or discontinuities

### Code Quality Goals

1. Clean separation of concerns (AudioNode trait, buffer management, execution)
2. Zero-copy buffer passing (Arc, no unnecessary clones)
3. Comprehensive test coverage (existing + new block-processing tests)
4. Documentation of new architecture
5. Easy to add new nodes (template + guide)

---

## Risks & Mitigation

### Risk 1: Pattern Timing Accuracy

**Risk**: Block processing might break sample-accurate event timing.

**Mitigation**:
- Pre-evaluate patterns in `prepare_block()` with exact sample offsets
- Test extensively with onset detection
- Compare block vs sample timing (should be identical)

### Risk 2: State Management in Parallel

**Risk**: Stateful nodes (oscillators, filters) might have issues with parallel execution.

**Mitigation**:
- Nodes only process in topological order (dependencies always ready)
- Each node has independent state (no sharing)
- Test parallel execution extensively

### Risk 3: Performance Regression

**Risk**: New system might be slower than expected.

**Mitigation**:
- Benchmark early and often
- Profile to find bottlenecks
- Optimize hot paths (buffer operations, node processing)
- SIMD for buffer operations where possible

### Risk 4: Migration Breaking Things

**Risk**: Removing old system too early causes regressions.

**Mitigation**:
- Coexistence period (both systems work side-by-side)
- Test parity extensively before removal
- Keep old system behind flag for rollback
- Incremental node migration (one type at a time)

---

## Daily Progress Tracking

### Week 1
- [ ] Day 1: Define AudioNode trait, ProcessContext
- [ ] Day 2: Implement BufferManager
- [ ] Day 3: Implement DependencyGraph with topological sort
- [ ] Day 4: Add petgraph dependency, test graph algorithms
- [ ] Day 5: Create basic ConstantNode and AdditionNode

### Week 2
- [ ] Day 1: Implement OscillatorNode
- [ ] Day 2: Implement filter nodes (LPF, HPF, BPF)
- [ ] Day 3: Implement ReverbNode, DelayNode
- [ ] Day 4: Set up coexistence (both systems in parallel)
- [ ] Day 5: First successful block-based render test

### Week 3
- [ ] Day 1: Implement SampleTriggerNode with VoiceManager
- [ ] Day 2: Test pattern timing accuracy
- [ ] Day 3: Implement remaining effect nodes
- [ ] Day 4: Implement MultiplicationNode, DivisionNode
- [ ] Day 5: Test complex signal flow

### Week 4
- [ ] Day 1: Implement BlockProcessor sequential mode
- [ ] Day 2: Test BlockProcessor with simple graphs
- [ ] Day 3: Test BlockProcessor with complex graphs
- [ ] Day 4: Debug any issues
- [ ] Day 5: Verify all simple cases work

### Week 5
- [ ] Day 1: Update compositional_compiler to generate AudioNodes
- [ ] Day 2: Test compiler with all node types
- [ ] Day 3: Run existing test suite with new system
- [ ] Day 4: Fix failing tests
- [ ] Day 5: Achieve test parity (all 385 tests pass)

### Week 6
- [ ] Day 1: Implement parallel batch execution
- [ ] Day 2: Test parallel FX chains
- [ ] Day 3: Benchmark performance gains
- [ ] Day 4: Profile and optimize bottlenecks
- [ ] Day 5: Verify multi-core utilization

### Week 7
- [ ] Day 1: Full system integration testing
- [ ] Day 2: Test live mode with hot-reload
- [ ] Day 3: Test render mode with multi-threading
- [ ] Day 4: Create comprehensive benchmark suite
- [ ] Day 5: Document new architecture

### Week 8
- [ ] Day 1: Final testing and verification
- [ ] Day 2: Remove old system code
- [ ] Day 3: Update all documentation
- [ ] Day 4: Create "How to Add AudioNode" guide
- [ ] Day 5: Celebrate! 🎉

---

## Implementation Notes

### When to Use Parallelism

**Parallel execution beneficial when**:
- Independent FX chains (reverb || delay || chorus)
- Multi-band processing (low || mid || high)
- Multiple oscillators with different frequencies
- Source nodes (no dependencies)

**Sequential execution needed when**:
- Nodes have dependencies (must wait for inputs)
- Simple graphs (parallelism overhead not worth it)
- < 4 nodes in batch (too little work to parallelize)

### Buffer Management Best Practices

1. **Reuse buffers**: Use pool to avoid allocations
2. **Arc for sharing**: Only Arc pointer cloned, not data
3. **try_unwrap**: Get buffer back from Arc when possible
4. **Clear on return**: Zero buffers when returning to pool
5. **Fixed size**: All buffers same size (512 samples) for simplicity

### Testing Strategy

1. **Unit tests**: Each node type tested in isolation
2. **Integration tests**: Complex signal flows
3. **Equivalence tests**: Block vs sample mode produces same output
4. **Performance tests**: Verify expected speedups
5. **Regression tests**: All existing tests must pass

---

## Next Immediate Action

**Start with Phase 1, Day 1:**

1. Create `src/audio_node.rs` with AudioNode trait
2. Create `src/buffer_manager.rs` with BufferManager
3. Add petgraph to Cargo.toml
4. Create `src/dependency_graph.rs`
5. Write tests for basic AudioNode implementations

**Ready to begin implementation?**
