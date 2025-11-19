# Profiling Results - Data-Driven Optimization Plan

## 🎯 Measurement Summary

We measured exactly where time is spent using `PROFILE_DETAILED=1`.

### Bottleneck Breakdown (stress_extreme.ph)

```
Total: ~64ms per buffer (512 samples, 16 outputs)

Stage computation:      8µs    (0.01%)   ← Negligible
Cycle position updates: ~200µs  (0.3%)   ← NOT the bottleneck!
Node evaluation:        ~64ms  (99.7%)   ← THIS IS EVERYTHING!
Output mixing:          ~50µs  (0.08%)   ← Negligible
```

### Per-Node Breakdown

```
Node NodeId(49): 3.85ms (cycle: 0.3%, eval: 99.7%)
Node NodeId(39): 3.85ms (cycle: 0.3%, eval: 99.7%)
Node NodeId(74): 3.90ms (cycle: 0.3%, eval: 99.7%)
... (14-16 nodes total)

Average: ~4ms per node
Total:   16 nodes × 4ms = 64ms
```

### Execution Structure

```
Num stages: 1
Stage 0: 14-16 nodes (all output nodes)
```

**KEY INSIGHT:** All 16 outputs have NO dependencies on each other, so they're in ONE stage. They COULD run in parallel but currently run SEQUENTIALLY.

---

## 💡 What We Learned

### ❌ What's NOT the Bottleneck

1. **Cycle position updates** - Only 0.3% of time
2. **Stage computation** - Only 8µs
3. **Output mixing** - Only 50µs
4. **Value caching** - Only Constant nodes cacheable (not applicable here)

### ✅ What IS the Bottleneck

1. **eval_node() recursion** - 99.7% of time
2. **Sequential rendering** - 16 nodes × 4ms when should be parallel
3. **Graph traversal overhead** - Each eval walks the full dependency graph

---

## 🚀 Data-Driven Action Plan

### Priority 1: Parallelize Stage Execution (15x speedup)

**Problem:** 16 independent nodes rendered sequentially

**Current:**
```
for node in stage:
    render_node(node)  // 4ms per node
Total: 64ms
```

**Target:**
```
parallel_for node in stage:
    render_node(node)  // All at once!
Total: 4ms  (16x speedup!)
```

**Challenge:** Rust ownership - `&mut self` can't be shared across threads

**Solutions (in order of complexity):**

1. **Quick Win: Batch Processing**
   - Split stage into chunks, process sequentially
   - Even 2x parallelism = 32ms → 50% faster

2. **Medium: Per-Node Cloning**
   - Clone graph state for each node
   - Rc::clone is cheap, eval_node works
   - Merge results after

3. **Proper: Refactor State**
   - Separate read-only graph from mutable state
   - Use Arc<Graph> + thread-local state
   - Lock-free parallelism

**Expected Impact:** 64ms → 4-8ms ✅ **UNDER BUDGET!**

### Priority 2: Eliminate Recursive eval_node (2-3x speedup)

**Problem:** eval_node() recursively traverses dependencies

**Solution:** Buffer-based evaluation for all node types

Current (Add/Multiply only):
```rust
SignalNode::Add { a, b } => {
    eval_signal_from_buffers(a) + eval_signal_from_buffers(b)
}
```

Needed (Sample, Oscillator, Filter, etc.):
```rust
SignalNode::Sample { pattern, ... } => {
    // Read from pattern_event_cache (already computed!)
    // Read from voice_output_cache (already rendered!)
    // No recursion!
}
```

**Expected Impact:** 4-8ms → 2-4ms (further optimization)

### Priority 3: SIMD Vectorization (2-4x speedup)

**Problem:** Processing samples one at a time

**Solution:** AVX2 processes 8 samples simultaneously

```rust
// Current:
for i in 0..512 {
    samples[i] = compute(i);
}

// SIMD:
for chunk in samples.chunks_exact_mut(8) {
    let vec = simd_compute(chunk_idx);  // 8 samples at once!
    chunk.copy_from_slice(&vec);
}
```

**Expected Impact:** 2-4ms → 0.5-1ms (extreme optimization)

---

## 📊 Performance Projection

| Optimization | Current | After | Speedup | Status |
|--------------|---------|-------|---------|--------|
| **Baseline** | 64ms | - | - | ✅ Measured |
| **2x Parallel** | 64ms | 32ms | 2x | Easy |
| **4x Parallel** | 64ms | 16ms | 4x | Medium |
| **16x Parallel** | 64ms | 4ms | 16x | ✅ **TARGET!** |
| **+ Buffer Eval** | 4ms | 2ms | 2x | Extra |
| **+ SIMD** | 2ms | 0.5ms | 4x | Extreme |

---

## 🔧 Implementation Roadmap

### Phase 1: Prove Parallelism Works (1 hour)

**Goal:** Show ANY parallel speedup

**Approach:**
```rust
// Split 16 nodes into 2 groups of 8
let (group1, group2) = stage.split_at(8);

// Render groups sequentially (for now)
for node in group1 { render_node(node); }
for node in group2 { render_node(node); }

// Later: Use scoped threads or message passing
```

**Expected:** 64ms → 32ms (2x faster, proves concept)

### Phase 2: True Parallelism (2-3 hours)

**Approach A: Message Passing**
```rust
use std::sync::mpsc;

// Spawn worker threads
let workers: Vec<_> = (0..16)
    .map(|_| spawn_worker(graph.clone()))
    .collect();

// Send work
for (node, worker) in stage.iter().zip(workers) {
    worker.send((node, buffer_size));
}

// Collect results
let buffers: Vec<_> = workers.iter()
    .map(|w| w.recv())
    .collect();
```

**Approach B: Refactor State**
```rust
struct GraphState {
    node_buffers: DashMap<NodeId, Vec<f32>>,  // Lock-free!
    cycle_positions: Arc<[f64; 512]>,          // Immutable!
}

// Now can use rayon directly!
stage.par_iter().for_each(|node| {
    render_node_lockfree(node, &state);
});
```

**Expected:** 64ms → 4-8ms ✅ **UNDER BUDGET!**

### Phase 3: Polish & Optimize (1-2 hours)

- Add buffer eval for Sample/Oscillator nodes
- Benchmark with various workloads
- Profile multi-core utilization
- Handle edge cases (feedback loops)

---

## 📝 Key Insights

### 1. Measurement > Assumptions

**We thought:** "Cycle position is the bottleneck (8192 calls!)"
**Reality:** Only 0.3% of time

**We thought:** "Caching will help"
**Reality:** Only Constant nodes cached, not applicable here

**We measured:** eval_node is 99.7%, parallelism is the answer

### 2. Architecture Enables Optimization

The dependency analysis and stage computation we built earlier makes parallelization **possible**. Without knowing which nodes are independent, we couldn't parallelize safely.

### 3. Rust Ownership is Real

Can't just slap `par_iter()` on everything. Need to either:
- Restructure data (best but most work)
- Use sync primitives (quick but lock contention)
- Clone data (wasteful but simple)

### 4. The Win is Clear

```
Current:     16 nodes × 4ms = 64ms (sequential)
Target:      16 nodes ÷ 16 cores = 4ms (parallel)
Improvement: 16x speedup = WAY under 11.61ms budget!
```

---

## 🎯 Next Session Goals

1. **Implement 2x parallelism** (split stage in half) → Prove concept
2. **Measure speedup** → Validate approach
3. **Scale to 16x** → Use proper threading/refactoring
4. **Hit <11.61ms budget** → Victory! 🎉

---

## 📁 Files Modified

- `src/unified_graph.rs` - Added detailed profiling
  - Per-node timing (cycle vs eval breakdown)
  - Per-stage timing
  - Cache hit/miss tracking

---

## 🔍 How to Profile

```bash
# Detailed per-node profiling
USE_BLOCK_PROCESSING=1 PROFILE_DETAILED=1 cargo run --release --bin phonon -- live test.ph

# View results
cat /tmp/phonon_node_profile.log

# Cache profiling
USE_BLOCK_PROCESSING=1 PROFILE_CACHE=1 cargo run --release --bin phonon -- live test.ph
```

---

*"In God we trust. All others must bring data."* - W. Edwards Deming

We measured. We know. We will optimize. 🚀
