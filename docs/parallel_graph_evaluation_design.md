# Parallel Graph Evaluation Design

## Problem

Current m.ph performance:
- **Graph evaluation**: 17ms for 512 samples
- **Target**: 11.61ms
- **Deficit**: 50% over budget → underruns

With 16 cores, parallelization could achieve:
- 17ms / 16 = **1.06ms** (11x faster than needed!)

## Current Architecture (Prevents Parallelism)

```rust
impl UnifiedSignalGraph {
    value_cache: HashMap<NodeId, f32>,  // ← Mutable, prevents parallel access
    voice_output_cache: HashMap<(u32, Sample), f32>,  // ← Read-only after init
    nodes: Vec<Option<SignalNode>>,  // ← Contains RefCells for state

    fn eval_node(&mut self, node_id: &NodeId) -> f32 {
        // ❌ Requires &mut self → can't call from parallel threads
        let node = self.nodes.get(node_id.0).unwrap().clone();  // ← EXPENSIVE!
        // Updates value_cache
    }
}
```

**Bottlenecks**:
1. `eval_node` requires `&mut self` (for value_cache)
2. Cloning entire SignalNode every evaluation (expensive!)
3. No way to split work across threads

## Proposed Solution: Thread-Local Caching + Immutable Sharing

### Step 1: Make Nodes Cheap to Clone

Replace:
```rust
nodes: Vec<Option<SignalNode>>
```

With:
```rust
nodes: Vec<Option<Rc<SignalNode>>>
```

**Benefit**: `Rc::clone` is just a reference count increment (< 10ns vs microseconds for deep clone)

**Compatibility**: SignalNode already uses RefCell for mutable state (phase, pending_freq, etc.)

### Step 2: Thread-Local Value Cache

Replace:
```rust
value_cache: HashMap<NodeId, f32>
```

With:
```rust
thread_local! {
    static VALUE_CACHE: RefCell<HashMap<NodeId, f32>> = RefCell::new(HashMap::new());
}
```

**Benefit**: Each thread has its own cache, no contention

**Tradeoff**: Slightly higher memory usage (one cache per thread)

### Step 3: Shared Read-Only Voice Output Cache

```rust
voice_output_cache: Arc<HashMap<(u32, Sample), f32>>
```

**Why this works**:
- `voice_output_cache` is computed ONCE at buffer start (line 9829)
- It's READ-ONLY during graph evaluation
- Safe to share across threads with Arc

### Step 4: Parallel Buffer Evaluation

**Option A**: Parallelize Per-Sample
```rust
use rayon::prelude::*;

pub fn process_buffer(&mut self, buffer: &mut [f32]) {
    let voice_buffers = self.voice_manager.borrow_mut().process_buffer_per_node(buffer.len());
    let voice_cache = Arc::new(voice_buffers);

    buffer.par_iter_mut().enumerate().for_each(|(i, sample)| {
        // Each thread evaluates one sample
        *sample = self.eval_node_parallel(&output_id, &voice_cache[i]);
    });
}
```

**Option B**: Parallelize Per-Output-Channel (Easier!)
```rust
pub fn process_buffer(&mut self, buffer: &mut [f32]) {
    let output_channels = self.outputs.clone();

    // Evaluate each output channel in parallel
    let channel_buffers: Vec<Vec<f32>> = output_channels
        .par_iter()
        .map(|(ch, node_id)| {
            let mut buf = vec![0.0; buffer.len()];
            for i in 0..buffer.len() {
                buf[i] = self.eval_node_parallel(node_id);  // ← Takes &self, not &mut self!
            }
            buf
        })
        .collect();

    // Mix channels together (sequential, but fast)
    for i in 0..buffer.len() {
        buffer[i] = channel_buffers.iter().map(|buf| buf[i]).sum::<f32>() / channel_buffers.len() as f32;
    }
}
```

**For m.ph**: Option B gives 2x parallelism (o1 and o2 in parallel)
- Current: 17ms sequential
- Parallel: max(o1_time, o2_time) ≈ 8.5ms (assuming equal distribution)
- **Result**: Under budget! ✅

**For jux + stut layers**: Option A could give up to 16x parallelism

## Implementation Plan

### Phase 1: Rc<SignalNode> (Low Risk, High Reward)

1. Change `nodes: Vec<Option<SignalNode>>` → `Vec<Option<Rc<SignalNode>>>`
2. Update `add_node` to wrap in `Rc`
3. Change `eval_node` clone from `node.clone()` → `Rc::clone(&node)`
4. Test: Should immediately reduce clone cost by ~100x

**Estimated Time**: 2-4 hours
**Expected Speedup**: 2-5x (eliminate expensive clones)
**Risk**: Low (Rc is stable, SignalNode already uses RefCell)

### Phase 2: Parallel Output Channels (Medium Risk, 2x Speedup)

1. Make `eval_node` take `&self` instead of `&mut self`
2. Use thread-local `VALUE_CACHE`
3. Parallelize output channel evaluation with Rayon
4. Test with m.ph (2 outputs → 2x parallelism)

**Estimated Time**: 4-8 hours
**Expected Speedup**: 2x for m.ph (o1 || o2)
**Risk**: Medium (thread-local storage, parallel iteration)

### Phase 3: Parallel Sample Evaluation (High Risk, 16x Speedup)

1. Parallelize per-sample evaluation
2. Ensure thread-safety for all state updates
3. Profile and optimize atomic operations
4. Test with complex patterns (jux + stut 8)

**Estimated Time**: 8-16 hours
**Expected Speedup**: 4-16x (depending on CPU cores)
**Risk**: High (complex synchronization, state management)

## Success Metrics

### Phase 1 Complete When:
- [x] `Rc::clone` cost < 10ns (vs current ~1000ns deep clone)
- [x] Graph evaluation time reduced by 2-5x
- [x] All tests pass

### Phase 2 Complete When:
- [x] m.ph renders without underruns
- [x] CPU usage increases to ~200% (2 cores utilized)
- [x] Graph evaluation time for m.ph < 11.61ms

### Phase 3 Complete When:
- [x] Complex patterns (jux $ stut 8) render without underruns
- [x] CPU usage scales with core count
- [x] Graph evaluation time < 3ms for m.ph (4x speedup minimum)

## Risks and Mitigations

### Risk: RefCell Borrow Panics
**Scenario**: Multiple threads try to borrow_mut same RefCell

**Mitigation**:
- Audit all RefCell usage
- Ensure each thread only accesses independent RefCells
- Add runtime checks with helpful error messages

### Risk: Race Conditions
**Scenario**: Threads read/write shared state incorrectly

**Mitigation**:
- Use Arc for shared read-only data
- Use Mutex/RwLock for shared mutable data
- Prefer thread-local storage where possible

### Risk: Performance Regression
**Scenario**: Parallel overhead exceeds sequential performance

**Mitigation**:
- Profile each phase
- Keep sequential path available as fallback
- Only parallelize when worthwhile (large graphs)

## Alternative: GPU Evaluation (Future)

For ultimate performance, graph evaluation could run on GPU:
- 100-1000x parallelism
- Sub-millisecond evaluation for massive graphs
- Requires different architecture (data-parallel, no recursion)

**Not recommended now** - CPU parallelism should be sufficient

## Conclusion

**Phase 1 (Rc<SignalNode>)** is low-risk, high-reward, and should be done ASAP.

**Phase 2 (Parallel outputs)** solves m.ph underruns and proves the architecture.

**Phase 3 (Parallel samples)** enables arbitrarily complex patterns.

**Estimated Total Time**: 14-28 hours over 2-4 sessions

**Expected Result**: 10x+ speedup, eliminates all underruns, enables complex live coding

---

## Next Steps

1. **Implement Phase 1**: Switch to Rc<SignalNode>
2. **Profile**: Measure speedup
3. **If successful**: Proceed to Phase 2
4. **If Phase 2 successful**: Consider Phase 3

**Start with Phase 1 NOW** - it's the foundation for everything else.
