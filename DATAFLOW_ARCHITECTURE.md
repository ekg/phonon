# Dataflow Architecture - Continuous Message-Passing Model

**Status**: Design Document (Implementation in Progress)
**Date**: 2025-11-21

---

## Vision

Replace batch-synchronous processing with **streaming dataflow** where nodes run continuously as independent tasks, communicating via lock-free message channels.

## Core Principles

### 1. Continuous Processing
- Nodes run continuously, not in discrete batches
- No idle time waiting at synchronization barriers
- Process blocks as soon as inputs are available

### 2. Message-Based Communication
- Buffers flow as messages between nodes
- Lock-free channels (crossbeam or flume)
- Zero-copy via `Arc<Vec<f32>>`

### 3. Natural Parallelism
- Each node is an independent task
- Automatic scaling to available cores
- No manual batch coordination needed

### 4. Pipelining
- Multiple blocks in flight simultaneously
- Process block N+1 while N is still flowing downstream
- Better hardware utilization

---

## Architecture Components

### Node Task Structure

```rust
struct NodeTask {
    /// Node ID for debugging/tracing
    id: NodeId,

    /// The AudioNode implementation (oscillator, filter, etc.)
    node: Box<dyn AudioNode>,

    /// Input channels from dependencies
    input_rx: Vec<Receiver<Arc<Vec<f32>>>>,

    /// Output channel to downstream nodes
    output_tx: Vec<Sender<Arc<Vec<f32>>>>,

    /// Buffer pool for recycling allocations
    buffer_pool: Arc<BufferPool>,

    /// Current processing context
    context: ProcessContext,
}

impl NodeTask {
    async fn run(mut self) -> Result<(), String> {
        loop {
            // 1. Wait for all input buffers to arrive
            let inputs = self.receive_inputs().await?;

            // 2. Get a buffer from the pool (or allocate new)
            let mut output = self.buffer_pool.acquire();

            // 3. Process the block
            let input_slices: Vec<&[f32]> = inputs.iter()
                .map(|buf| buf.as_slice())
                .collect();
            self.node.process_block(&input_slices, &mut output, &self.context);

            // 4. Wrap in Arc and send to all downstream nodes
            let output_arc = Arc::new(output);
            for tx in &self.output_tx {
                tx.send(output_arc.clone()).await?;
            }

            // 5. Try to return input buffers to pool
            for input in inputs {
                if let Ok(buf) = Arc::try_unwrap(input) {
                    self.buffer_pool.release(buf);
                }
            }
        }
    }

    async fn receive_inputs(&self) -> Result<Vec<Arc<Vec<f32>>>, String> {
        let mut inputs = Vec::with_capacity(self.input_rx.len());
        for rx in &self.input_rx {
            inputs.push(rx.recv().await?);
        }
        Ok(inputs)
    }
}
```

### Buffer Pool

```rust
/// Lock-free buffer pool for recycling allocations
struct BufferPool {
    /// Available buffers
    free_buffers: ArrayQueue<Vec<f32>>,

    /// Buffer size (e.g., 512 samples)
    buffer_size: usize,

    /// Maximum pool size
    max_buffers: usize,
}

impl BufferPool {
    fn acquire(&self) -> Vec<f32> {
        self.free_buffers.pop()
            .unwrap_or_else(|| vec![0.0; self.buffer_size])
    }

    fn release(&self, mut buffer: Vec<f32>) {
        buffer.clear();
        buffer.resize(self.buffer_size, 0.0);
        let _ = self.free_buffers.push(buffer); // Ignore if full
    }
}
```

### Graph Coordinator

```rust
/// Coordinates the dataflow graph
struct DataflowGraph {
    /// All node tasks (running in background)
    tasks: Vec<JoinHandle<Result<(), String>>>,

    /// Input channel to first nodes (from audio callback)
    input_tx: Sender<Arc<Vec<f32>>>,

    /// Output channel from final node (to audio callback)
    output_rx: Receiver<Arc<Vec<f32>>>,

    /// Shared buffer pool
    buffer_pool: Arc<BufferPool>,

    /// Shutdown signal
    shutdown: Arc<AtomicBool>,
}

impl DataflowGraph {
    fn new(nodes: Vec<Box<dyn AudioNode>>, output_node: NodeId) -> Result<Self, String> {
        let dep_graph = DependencyGraph::build(&nodes)?;
        let buffer_pool = Arc::new(BufferPool::new(512, 128));

        // Create channels for all nodes
        let mut channels: HashMap<NodeId, (Sender<_>, Receiver<_>)> = HashMap::new();
        for node_id in 0..nodes.len() {
            channels.insert(node_id, bounded(4)); // Backpressure: max 4 blocks
        }

        // Spawn task for each node
        let mut tasks = Vec::new();
        for (node_id, node) in nodes.into_iter().enumerate() {
            let input_deps = dep_graph.dependencies(node_id);
            let input_rx: Vec<_> = input_deps.iter()
                .map(|&dep| channels[&dep].1.clone())
                .collect();

            let output_tx: Vec<_> = dep_graph.dependents(node_id).iter()
                .map(|&dep| channels[&dep].0.clone())
                .collect();

            let task = NodeTask {
                id: node_id,
                node,
                input_rx,
                output_tx,
                buffer_pool: buffer_pool.clone(),
                context: ProcessContext::default(),
            };

            tasks.push(tokio::spawn(task.run()));
        }

        // Extract input/output channels
        let source_nodes = dep_graph.source_nodes(); // Nodes with no inputs
        let input_tx = channels[&source_nodes[0]].0.clone();
        let output_rx = channels[&output_node].1.clone();

        Ok(Self {
            tasks,
            input_tx,
            output_rx,
            buffer_pool,
            shutdown: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Process one block (called from audio callback)
    async fn process_block(&mut self, output: &mut [f32]) -> Result<(), String> {
        // Send trigger to source nodes
        let trigger = Arc::new(vec![0.0; 512]); // Empty input for sources
        self.input_tx.send(trigger).await?;

        // Receive processed output
        let result = self.output_rx.recv().await?;
        output.copy_from_slice(&result);

        Ok(())
    }

    fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
        // Tasks will see shutdown flag and exit gracefully
    }
}
```

---

## Data Flow Example

```
Graph:
  ~lfo: sine 0.5
  ~bass: saw 55
  ~filtered: ~bass # lpf (~lfo * 1000 + 500) 0.8
  out: ~filtered

Dataflow:
  [sine task]  ─┬─> [multiply task] ─> [add task] ─┐
                │                                    │
  [saw task]  ──┼───────────────────────────────────┼─> [lpf task] ─> [output]
                │                                    │
                └────────────────────────────────────┘

Timeline:
  T=0:   sine(0.5) starts block 0
  T=1:   saw(55) starts block 0, sine finishes → sends to mul
  T=2:   mul starts, saw finishes → sends to lpf
  T=3:   add finishes → sends to lpf, sine starts block 1
  T=4:   lpf finishes block 0 → output, mul starts block 1
  T=5:   output block 0 to audio, lpf starts block 1, sine starts block 2

  All nodes continuously processing, 2-3 blocks in flight!
```

---

## Implementation Plan

### Phase 5.1: Core Dataflow Infrastructure (2 hours)
- [ ] BufferPool with lock-free queue
- [ ] NodeTask structure
- [ ] Channel creation and wiring
- [ ] Basic task spawning

### Phase 5.2: Graph Coordinator (1 hour)
- [ ] DataflowGraph struct
- [ ] Build from AudioNodes
- [ ] process_block() integration
- [ ] Shutdown handling

### Phase 5.3: Audio Callback Integration (1 hour)
- [ ] Integrate with AudioNodeGraph
- [ ] Handle backpressure
- [ ] Error recovery
- [ ] Latency monitoring

### Phase 5.4: Testing & Optimization (2 hours)
- [ ] Single-node tests
- [ ] Multi-node pipeline tests
- [ ] Complex graph tests (FM synthesis, etc.)
- [ ] Benchmark vs current implementation
- [ ] Tune channel buffer sizes

---

## Performance Expectations

### CPU Utilization
- **Current**: 30-40% on 8-core (sequential processing with idle time)
- **Target**: 70-90% on 8-core (continuous processing)

### Latency
- **Current**: 11.6ms (one block)
- **Target**: 11.6ms (same - no additional latency)

### Throughput
- **Current**: 1 block per 11.6ms
- **Target**: 3-4 blocks processing simultaneously (pipelined)

### Speedup
- **Expected**: 3-5x on 8+ cores for complex graphs

---

## Dependencies

```toml
[dependencies]
crossbeam = "0.8"          # Lock-free channels
crossbeam-queue = "0.3"    # ArrayQueue for buffer pool
tokio = { version = "1", features = ["sync", "rt-multi-thread"] }
```

---

## Migration Strategy

### Coexistence
- Keep current BlockProcessor as fallback
- Add flag: `const USE_DATAFLOW: bool = false`
- Gradually enable for testing

### Testing
- Run both implementations in parallel
- Compare outputs (should be identical)
- Benchmark performance differences

### Rollout
1. Implement dataflow (Phase 5.1-5.4)
2. Test extensively with existing integration tests
3. Enable by default: `USE_DATAFLOW = true`
4. Monitor for regressions
5. Remove old BlockProcessor after confidence period

---

## Open Questions

1. **Async vs threads?**
   - Async (tokio): Lower overhead, better for many nodes
   - Threads: Simpler, guaranteed parallelism
   - **Decision**: Start with tokio async, can switch to threads if needed

2. **Channel capacity?**
   - Too small: Backpressure, stalls
   - Too large: Memory usage, latency
   - **Decision**: Start with 4 blocks, tune based on benchmarks

3. **Buffer pool size?**
   - Need enough for blocks in flight + some spare
   - **Decision**: 128 buffers (enough for 32 nodes × 4 blocks)

4. **Error handling?**
   - What if a node task panics?
   - **Decision**: Supervisor task restarts failed nodes

---

## Success Criteria

- [ ] All 1772 tests still pass
- [ ] 16 integration tests still pass
- [ ] 3x+ speedup on 8-core for complex graphs
- [ ] No latency regression
- [ ] Stable under continuous load (24+ hour test)

---

**Next**: Begin implementation (Phase 5.1)
