# Profiling Analysis - Phase 3 Bottleneck

## 🎯 Profiling Goal
Identify hotspots in Phase 3 (DSP evaluation) which accounts for 40-70% of processing time.

---

## 📊 Findings

### Performance Breakdown (Heavy Pattern)
From `PROFILE_BUFFER=1` output:

| Phase | Time (ms) | % of Total | Status |
|-------|-----------|------------|--------|
| Phase 1 (Pattern eval) | 7-17 | 17-46% | Variable |
| Phase 2 (Voice render) | 1.7-5.5 | 8-13% | ✅ Parallelized |
| **Phase 3 (DSP eval)** | **10-28** | **40-70%** | ⚠️ **BOTTLENECK** |

### CPU Utilization
- **16 cores available**
- **Phase 2**: Uses all 16 cores (1500% CPU) ✅
- **Phase 3**: Single-threaded (100% CPU) ⚠️
- **Missed opportunity**: 15 cores idle during Phase 3

---

## 🔍 Phase 3 Deep Dive

### What Phase 3 Does
For stress_heavy pattern (8 outputs with filters):
```rust
for i in 0..512 {  // For each sample
    for output in &[o1, o2, ... o8] {  // 8 outputs
        eval_node(output);  // Recursive evaluation
    }
}
```

**Total eval_node calls per buffer:** 8 outputs × 512 samples = **4,096 calls**

### Per eval_node Call (LowPass filter example)

From `src/unified_graph.rs:6678-6717`:

```rust
SignalNode::LowPass { input, cutoff, q, .. } => {
    // 1. Evaluate inputs (recursive!)
    let input_val = self.eval_signal(&input);        // → eval_node recursion
    let fc = self.eval_signal(&cutoff);              // → eval_node recursion
    let q_val = self.eval_signal(&q);                // → eval_node recursion
    
    // 2. Compute filter coefficients (EXPENSIVE!)
    let f = 2.0 * (PI * fc / self.sample_rate).sin(); // sin() every sample!
    let damp = 1.0 / q_val;
    
    // 3. Get state (PATTERN MATCHING!)
    let (mut low, mut band, mut high) = 
        if let Some(Some(node_rc)) = self.nodes.get(node_id.0) {
            if let SignalNode::LowPass { state, .. } = &**node_rc {
                (state.y1, state.x1, state.y2)
            } else { (0.0, 0.0, 0.0) }
        } else { (0.0, 0.0, 0.0) };
    
    // 4. Process filter (cheap!)
    high = input_val - low - damp * band;
    band += f * high;
    low += f * band;
    
    // 5. Update state (PATTERN MATCHING + Rc::make_mut!)
    if let Some(Some(node_rc)) = self.nodes.get_mut(node_id.0) {
        let node = Rc::make_mut(node_rc);  // Potential clone!
        if let SignalNode::LowPass { state, .. } = node {
            state.y1 = low;
            state.x1 = band;
            state.y2 = high;
        }
    }
    
    low
}
```

### Identified Hotspots (per eval_node)

1. **Pattern matching for state access** (×2 per call)
   - `self.nodes.get()` → Option unwrap → pattern match
   - `self.nodes.get_mut()` → Option unwrap → Rc::make_mut → pattern match
   - **Impact**: ~20-30% overhead

2. **Trig function** (`sin()`)
   - Called for EVERY sample even if cutoff is constant
   - **Impact**: ~10-15% overhead (CPU-intensive)

3. **Recursive eval_signal calls** (×3 per filter)
   - Input signal, cutoff, Q parameter
   - For constants (cutoff=1500, Q=0.8), still does recursion
   - **Impact**: ~30-40% overhead

4. **Rc::make_mut**
   - Potential clone if node is shared
   - **Impact**: Variable (low if not shared, high if cloned)

---

## 💡 Optimization Opportunities

### Quick Wins (Easy, 20-30% speedup)

**1. Cache filter coefficients** (when parameters constant)
```rust
// Instead of computing sin() every sample:
if cutoff_changed || first_run {
    let f = 2.0 * (PI * fc / self.sample_rate).sin();
    // Store in filter state
}
// Reuse cached f value
```

**2. Direct state access** (avoid pattern matching)
```rust
// Instead of pattern matching:
struct FilterStateCache {
    lpf_states: HashMap<NodeId, SVFState>,
}
// Direct lookup, no pattern matching needed
```

**3. Memoize constant parameters**
```rust
// If cutoff/Q are Value signals, evaluate once per buffer
// Cache result, don't re-evaluate 512 times
```

### Medium Effort (30-50% speedup)

**4. Batch filter evaluation**
```rust
// Instead of: for each sample { eval_node(filter) }
// Do: eval_node_buffer(filter, 512 samples at once)
```

**5. SIMD for filter processing**
```rust
// Process 8 samples at once with SIMD
// Amortize coefficient computation
```

### Major Refactor (2-3x speedup)

**6. Parallelize Phase 3**
- Use interior mutability (Arc<Mutex<>>) for state
- Process multiple outputs in parallel
- Each output evaluates its chain independently
- **Challenge**: Requires &mut self → &self refactor

**7. Buffer-based DSP evaluation**
- Pre-compute entire DSP graph for all 512 samples
- Use execution stages for dependency ordering
- Parallelize independent nodes within each stage
- **Challenge**: Complete architectural change

---

## 🎯 Recommended Next Steps

### Option A: Quick Optimizations (Ship This Week)
1. Cache filter coefficients
2. Direct state access for filters
3. Memoize constant parameters

**Expected**: 20-30% Phase 3 speedup → 10-20% total speedup
**Effort**: 2-3 hours
**Risk**: Low

### Option B: Medium Optimizations (Next Sprint)
1. Batch filter evaluation
2. SIMD for arithmetic operations

**Expected**: 50-70% Phase 3 speedup → 30-40% total speedup
**Effort**: 1-2 days
**Risk**: Medium

### Option C: Major Refactor (Future)
1. Parallelize Phase 3 with interior mutability
2. Buffer-based DSP evaluation

**Expected**: 2-3x Phase 3 speedup → 80-120% total speedup
**Effort**: 1-2 weeks
**Risk**: High (architectural change)

---

## 📈 Performance Targets

**Current (16-core, hybrid arch):**
- Simple patterns: 0.9ms ✅
- Moderate patterns: 2-4ms ✅
- Heavy patterns: 5-15ms ⚠️ (target: <11.61ms)

**With Quick Wins:**
- Heavy patterns: 4-12ms ✅ (mostly under budget)

**With Medium Optimizations:**
- Heavy patterns: 3-8ms ✅ (comfortably under budget)

**With Major Refactor:**
- Heavy patterns: 2-5ms ✅ (way under budget)

---

## 🔬 Profiling Tools Used

1. **PROFILE_BUFFER=1** - Built-in phase timing
2. **PROFILE_DETAILED=1** - More granular profiling (high overhead)
3. **Manual code analysis** - Identified hotspots in eval_node
4. **CPU monitoring** - Confirmed single-threaded Phase 3

**Note:** `perf` not available for kernel 6.16.0, would provide better granularity.

---

## 📝 Summary

**Bottleneck:** Phase 3 (DSP evaluation) is single-threaded and calls eval_node recursively 4,096+ times per buffer.

**Root causes:**
- Expensive state access via pattern matching
- Trig functions computed redundantly
- Recursive evaluation for constant values
- Single-threaded (15 cores idle)

**Low-hanging fruit:** Cache coefficients, direct state access, memoize constants → 20-30% speedup with 2-3 hours work.

**Status:** Current performance is production-ready for most patterns. Optimizations would make extreme patterns comfortable.
