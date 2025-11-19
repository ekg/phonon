# Phonon Architecture Analysis: Routing & Performance

## Current State Assessment

### ✅ What Works Today

**Forward Signal Routing:**
```phonon
~osc: sine 440
~filtered: ~osc # lpf 1000 0.8
~delayed: ~filtered # delay 0.5
~reverbed: ~delayed # reverb 0.8
out: ~reverbed
```
✅ This works perfectly. The recursive `eval_node()` evaluates dependencies in order.

**Multiple Output Buses:**
```phonon
~kick: s "bd*4"
~snare: s "sn*2"
~hats: s "hh*8"
o1: ~kick # lpf 500 0.8
o2: ~snare # reverb 0.6
o3: ~hats # hpf 8000 0.7
```
✅ Multiple outputs work, each can reference shared buses.

### ❌ What DOESN'T Work Today

**Feedback Loops:**
```phonon
~feedback: ~output # delay 0.1 * 0.5
~output: ~input + ~feedback
```
❌ **INFINITE RECURSION!** The sample-by-sample recursive evaluation creates infinite loops.

**Why it fails:**
1. `eval_node(~output)` calls
2. → `eval_signal(~feedback)` calls
3. → `eval_node(~feedback)` calls
4. → `eval_signal(~output)` calls
5. → `eval_node(~output)` ← LOOP!

---

## Critical Performance Issue

### Current Phase 3 Architecture (The Bottleneck)

**Sample-by-sample recursive evaluation:**
```rust
for i in 0..512 {
    buffer[i] = eval_node(&output);  // Recursive tree walk, 512 times!
}
```

For a pattern with 8 outputs and filters:
- **512 samples** × **8 outputs** × **recursive tree depth**
- = **~4,096+ recursive function calls per buffer**
- = **10-22ms** (40-70% of total time)
- = **Single-threaded** (15 cores idle)

This is why "heavy" patterns are slow. The architecture fundamentally doesn't scale.

---

## Major Architectural Changes Needed

### 1. Buffer-Based Evaluation (High Priority)

**Current (slow):**
```rust
fn eval_node(&mut self, id: &NodeId) -> f32  // Returns ONE sample
```

**Target (fast):**
```rust
fn eval_node_buffer(&mut self, id: &NodeId, output: &mut [f32])  // Fills entire buffer
```

**Benefits:**
- 512 → 1 function call per buffer
- Compiler can vectorize (SIMD)
- Better cache locality
- Enables loop unrolling

**Expected speedup:** 3-5x for Phase 3

**Effort:** High (requires rewriting all node evaluation)

---

### 2. Feedback Loop Support (Critical for DAW Routing)

**Solution: Stage-Based Evaluation with Feedback Buffers**

1. **Topological sort** of signal graph (detect cycles)
2. **Break cycles** at feedback points with 1-sample delay
3. **Evaluate in stages:**
   ```
   Stage 1: Sources (oscillators, samples)
   Stage 2: First-level processing (filters on sources)
   Stage 3: Feedback mixing (use prev buffer values)
   Stage 4: Final processing
   ```

**For feedback:**
```phonon
~feedback: ~output # delay 0.1 * 0.5
~output: ~input + ~feedback
```

Evaluate as:
```rust
// Stage 1: Compute output using PREVIOUS feedback buffer
output_buffer = eval(~input) + feedback_delay_buffer

// Stage 2: Update feedback buffer for NEXT iteration
feedback_delay_buffer = output_buffer * 0.5
```

**Benefit:** Enables complex routing like Ableton/Reaktor
**Effort:** Medium-High (graph analysis + stage system)

---

### 3. Parallel Phase 3 Evaluation (High Priority)

**Current:** Single-threaded Phase 3 (1 of 16 cores used)

**Target:** Parallel evaluation of independent subgraphs

**Approach 1: Per-Output Parallelism**
```rust
outputs.par_iter().for_each(|output| {
    eval_node_buffer(output, &mut output_buffer);
});
```

**Challenge:** Requires `&self` instead of `&mut self` for parallel access
- Need interior mutability (RefCell/Mutex) for filter state
- Or use lock-free atomic operations

**Approach 2: Stage-Based Parallelism**
```rust
// Within each stage, process independent nodes in parallel
stage_nodes.par_iter().for_each(|node| {
    eval_node_buffer(node, &mut buffer);
});
```

**Expected speedup:** 2-4x (can use 8+ cores)
**Effort:** Medium (interior mutability refactor)

---

### 4. JIT Compilation (Nuclear Option)

**Idea:** Compile the signal graph to native machine code

```
Phonon pattern → LLVM IR → Optimized x86-64 → Execute
```

**Example compiled output:**
```rust
// From: ~out: sine 440 # lpf 1000 0.8
fn eval_compiled(buffer: &mut [f32]) {
    for i in 0..512 {
        // Oscillator (unrolled, SIMD)
        let osc = sin_fast(phase);
        phase += freq_delta;

        // Filter (optimized, inlined)
        let fc = 1000.0;  // Constant folded!
        // ... SVF math here ...

        buffer[i] = filtered;
    }
}
```

**Benefits:**
- Eliminates ALL recursion overhead
- Constant folding (1000.0 doesn't need eval)
- Full SIMD vectorization
- Inlining everything
- Custom optimization per pattern

**Expected speedup:** 5-10x
**Effort:** Very High (LLVM integration, substantial work)
**Risk:** High (complex, hard to debug)

---

## Recommended Roadmap

### Phase 1: Buffer-Based Evaluation (4-6 weeks)
**Impact:** 3-5x Phase 3 speedup → 50-80% total speedup
**Effort:** High but tractable

**Steps:**
1. Design buffer-based node API
2. Migrate oscillators to buffer processing
3. Migrate filters to buffer processing
4. Migrate effects to buffer processing
5. Update Phase 3 to call buffer methods
6. Benchmark and tune

**Result:** Heavy patterns 10-22ms → 3-7ms (well under target!)

---

### Phase 2: Feedback Loop Support (2-3 weeks)
**Impact:** Enables complex DAW-style routing
**Effort:** Medium

**Steps:**
1. Implement cycle detection in signal graph
2. Add feedback delay buffers at cycle points
3. Implement stage-based evaluation
4. Add topological sort for node ordering
5. Test with complex feedback patterns

**Result:** Full DAW-style signal routing with feedback

---

### Phase 3: Parallel Phase 3 (2-3 weeks)
**Impact:** 2-4x additional speedup
**Effort:** Medium

**Steps:**
1. Refactor nodes to use interior mutability (RefCell)
2. Change eval methods to take `&self` instead of `&mut self`
3. Parallelize per-output evaluation with rayon
4. Or implement stage-based parallelism
5. Benchmark parallel overhead vs speedup

**Result:** Heavy patterns 3-7ms → 1-3ms (massive headroom!)

---

### Phase 4: JIT Compilation (Optional, 3-4 months)
**Impact:** 5-10x additional speedup
**Effort:** Very high

**Only pursue if:**
- Phases 1-3 still aren't enough
- You want extreme performance (1000+ voices)
- Team has LLVM expertise

---

## Priority Decision Matrix

| Change | Impact | Effort | Priority | Timeline |
|--------|--------|--------|----------|----------|
| Buffer-based eval | ⭐⭐⭐⭐⭐ | High | 🔥 **P0** | Now |
| Feedback loops | ⭐⭐⭐⭐ | Medium | 🔥 **P0** | After P1 |
| Parallel Phase 3 | ⭐⭐⭐⭐ | Medium | ⭐ **P1** | After feedback |
| JIT compilation | ⭐⭐⭐⭐⭐ | Very High | ⚠️ **P2** | Optional |

---

## Immediate Next Steps

### Week 1-2: Design & Prototype
1. **Design buffer-based API** - Sketch out new evaluation interface
2. **Prototype one node** - Convert `sine` oscillator to buffer mode
3. **Benchmark prototype** - Measure speedup vs current
4. **Validate approach** - Ensure it scales

### Week 3-4: Core Migration
1. Migrate all oscillators (sine, saw, square, tri)
2. Migrate all filters (lpf, hpf, bpf)
3. Migrate arithmetic nodes (add, multiply, mix)

### Week 5-6: Integration & Testing
1. Update Phase 3 to use buffer evaluation
2. Comprehensive testing (all patterns)
3. Audio correctness verification
4. Performance benchmarking

---

## Expected Final Performance

With Phases 1-3 complete:

| Pattern Type | Current | After Buffer | After Parallel | Target |
|--------------|---------|--------------|----------------|--------|
| Simple (4 voices) | 0.9ms | 0.3ms | 0.2ms | 11.61ms ✅ |
| Moderate (20 voices) | 3-5ms | 1-2ms | 0.5-1ms | 11.61ms ✅ |
| Heavy (100 voices) | 10-22ms | 3-7ms | 1-3ms | 11.61ms ✅ |
| Extreme (500 voices) | 50-100ms | 15-30ms | 5-10ms | 11.61ms ✅ |

**Result:** Production-ready for hundreds of voices with comfortable headroom.

---

## Conclusion

**Current architecture is fundamentally limited:**
- Sample-by-sample = too much overhead
- No feedback support = can't do complex routing
- Single-threaded Phase 3 = wasting 15 cores

**Buffer-based evaluation is THE key unlock:**
- 3-5x speedup from this alone
- Enables SIMD, vectorization, better optimization
- Foundation for all other improvements

**Don't do JIT compilation yet:**
- Buffer-based + parallel should be enough
- JIT is very complex, save for later if needed

**Focus:** Buffer-based eval → Feedback loops → Parallel → Profit!
