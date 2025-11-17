# Parallel Synthesis Plan - Multi-Core Utilization

## Current Status
- **Single-threaded**: 114.4% CPU (cannot run in realtime on 1 core)
- **Hardware available**: 16 cores @ 2GHz
- **Opportunity**: With proper parallelization, 114% could run at ~7% per core!

## The Challenge

The main audio loop (process_buffer) is inherently sequential:
```rust
for i in 0..buffer.len() {  // Must process samples in order
    self.update_cycle_position_from_clock();  // Time advances
    self.voice_output_cache = ...;
    buffer[i] = self.eval_node(&output_id);   // State changes
}
```

Each sample depends on state from the previous sample (cycle position, cache, voice state).

## Parallelization Strategies

### Strategy 1: Buffer-Level Pipeline (EASIEST - Implement This First)

Process multiple buffers in parallel by cloning the graph state:

```rust
// In render mode, prepare N graph instances
let num_threads = num_cpus::get();
let graphs: Vec<UnifiedSignalGraph> = (0..num_threads)
    .map(|_| graph.clone())
    .collect();

// Process buffers in parallel
let buffers: Vec<Vec<f32>> = (0..num_buffers)
    .into_par_iter()
    .map(|block_idx| {
        let graph_idx = block_idx % num_threads;
        let mut my_graph = graphs[graph_idx].clone();

        // Seek to correct time position
        my_graph.seek_to_sample(block_idx * 512);

        // Process this buffer
        let mut buffer = vec![0.0; 512];
        my_graph.process_buffer(&mut buffer);
        buffer
    })
    .collect();
```

**Benefits**:
- Easy to implement
- Scales linearly with cores (16x speedup potential!)
- No changes to core synthesis logic
- Perfect for offline rendering

**Downsides**:
- Requires cloning graph state (expensive)
- Only works for offline rendering, not live mode
- Memory usage scales with thread count

**Estimated Impact**: On 16 cores, 114% → ~7% per core

### Strategy 2: Output Channel Parallelization (MEDIUM)

Each output channel (out1, out2, out3, etc.) runs on a separate thread:

```rust
pub fn process_buffer_multi_threaded(&mut self, buffer_size: usize) -> HashMap<usize, Vec<f32>> {
    let outputs = &self.outputs;

    outputs.par_iter()
        .map(|(&channel, &node_id)| {
            // Clone graph for this channel
            let mut graph = self.clone();
            let mut buffer = vec![0.0; buffer_size];

            for i in 0..buffer_size {
                buffer[i] = graph.eval_node(&node_id);
            }

            (channel, buffer)
        })
        .collect()
}
```

**Benefits**:
- Natural parallelism (outputs are independent)
- Could work in live mode with careful synchronization
- Users with multi-output patterns benefit most

**Downsides**:
- Only helps if using multiple outputs
- Complex synchronization for shared state
- Graph cloning overhead

**Estimated Impact**: With 4 outputs, 4x speedup

### Strategy 3: SuperCollider-Style UGen Parallelization (HARD, PROPER FIX)

Analyze graph topology and run independent subgraphs in parallel:

```rust
// Analyze dependencies
let subgraphs = analyze_graph_parallelism(&self.nodes);

// Execute independent subgraphs in parallel
subgraphs.into_par_iter()
    .for_each(|subgraph| {
        process_subgraph(&mut self, subgraph);
    });
```

**Benefits**:
- Works in both live and render modes
- Scales with graph complexity
- Optimal CPU utilization

**Downsides**:
- Complex dependency analysis
- Requires major refactoring
- 2-4 weeks of work

**Estimated Impact**: Varies by patch, 2-8x speedup typical

### Strategy 4: SIMD Vectorization (HARD, SPECIALIZED)

Process 4-8 samples simultaneously using SIMD instructions:

```rust
use std::simd::f32x8;

// Process 8 samples at once
let mut samples = f32x8::splat(0.0);
samples = process_oscillator_simd(&freq_pattern);
```

**Benefits**:
- Massive speedup for simple operations (8x for 256-bit AVX)
- Works within single thread
- Great for oscillators, filters

**Downsides**:
- Requires rewriting all SignalNode evaluation
- Not all operations vectorize well
- Months of work

## Recommended Implementation Order

### Phase 1: Buffer-Level Pipeline (This Week)

1. Add `--parallel` flag to render command
2. Implement multi-buffer processing with Rayon
3. Test on m.ph - target < 10% CPU per core

**Expected**: 114% → 7-14% per core (8-16x speedup)

### Phase 2: Inline + Buffer Pipeline (Next Week)

1. Combine with existing inline optimizations
2. Add progress reporting for long renders
3. Auto-tune thread count based on available cores

**Expected**: Further 10-20% improvement

### Phase 3: Output Channel Parallelization (Future)

Only if users heavily use multi-output patterns.

### Phase 4: SuperCollider-Style (Long Term)

When we have time for proper architectural refactoring.

## Implementation: Buffer-Level Pipeline

Let me implement this now as it's the biggest win for the least effort.

### Code Location

Modify `src/main.rs` in the render command, around line 1299-1390.

### Key Changes

1. Add `--parallel` flag (default: true)
2. Split total_samples into 512-sample blocks
3. Use rayon `par_iter` to process blocks
4. Each thread gets a cloned graph with correct time offset
5. Concatenate results

### Testing

```bash
# Test serial (current)
time cargo run --release -- render m.ph /tmp/m_serial.wav --cycles 4

# Test parallel
time cargo run --release -- render m.ph /tmp/m_parallel.wav --cycles 4 --parallel

# Should see linear speedup with cores
```

## Success Metrics

**Goal**: With 16 cores, handle 16x current complexity
- Single pattern at 114% → run 16 patterns simultaneously
- Or: 16x longer patterns
- Or: 16x more effects/complexity

**Target**: < 50% CPU per core (comfortable headroom for live coding)
