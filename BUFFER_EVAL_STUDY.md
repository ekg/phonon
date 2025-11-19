# Buffer Evaluation Framework - Study Phase

**Date:** 2025-11-19
**Status:** 📚 Study
**Goal:** Design the core buffer evaluation API and understand migration requirements

---

## Current Architecture Analysis

### Sample-by-Sample Evaluation

**Current API:**
```rust
fn eval_node(&mut self, node_id: &NodeId) -> f32
```

**Call pattern in Phase 3:**
```rust
for i in 0..512 {
    buffer[i] = self.eval_node(&output_id);  // 512 calls!
}
```

**Key observations:**
1. Returns single `f32` value
2. Requires `&mut self` (exclusive access)
3. Recursively evaluates dependencies
4. Has value caching system (per-sample cache)
5. Handles stateful nodes (oscillators, filters) with `RefCell` for interior mutability

### Signal Types

```rust
pub enum Signal {
    Value(f32),              // Constant value
    Node(NodeId),            // Reference to another node
    Bus(String),             // Named signal bus
    Pattern(String),         // Inline pattern string
    Expression(Box<SignalExpr>),  // Arithmetic combinations
}
```

Current evaluation:
```rust
fn eval_signal(&mut self, signal: &Signal) -> f32
```

### Node Categories

**1. Stateless (Pure):**
- `Constant` - Just returns a value
- `Add`, `Multiply`, `Mix` - Arithmetic operations
- Value-only signals

**2. Stateful (Need Phase/History):**
- `Oscillator` - Tracks phase
- `LowPass`, `HighPass`, `BandPass` - Filter state
- `Delay` - Delay line buffer
- `Reverb` - Reverb state
- All effects with memory

**3. Pattern-Based:**
- `Pattern` - Queries pattern events
- `Sample` - Triggers voices from patterns

---

## Design: Buffer-Based Architecture

### Core API Design

```rust
impl UnifiedSignalGraph {
    /// Evaluate a node for an entire buffer
    ///
    /// This is the core buffer evaluation method that replaces sample-by-sample
    /// eval_node() calls with a single call that fills an entire buffer.
    ///
    /// # Arguments
    /// * `node_id` - The node to evaluate
    /// * `output` - The output buffer to fill (must be pre-allocated)
    ///
    /// # Performance
    /// - Reduces function call overhead from 512 calls to 1
    /// - Enables SIMD vectorization by compiler
    /// - Improves cache locality
    /// - Foundation for parallelization
    fn eval_node_buffer(&mut self, node_id: &NodeId, output: &mut [f32]) {
        // Get node (cheap Rc::clone)
        let node_rc = if let Some(Some(node_rc)) = self.nodes.get(node_id.0) {
            std::rc::Rc::clone(node_rc)
        } else {
            output.fill(0.0);
            return;
        };

        let node = &*node_rc;

        // Dispatch to node-specific buffer evaluation
        match node {
            SignalNode::Oscillator { freq, waveform, .. } => {
                self.eval_oscillator_buffer(node_id, freq, waveform, output)
            }
            SignalNode::LowPass { input, cutoff, q, .. } => {
                self.eval_lpf_buffer(node_id, input, cutoff, q, output)
            }
            SignalNode::Add { a, b } => {
                self.eval_add_buffer(a, b, output)
            }
            SignalNode::Constant { value } => {
                output.fill(*value)
            }
            // ... other nodes
            _ => {
                // Fallback for not-yet-migrated nodes
                for i in 0..output.len() {
                    output[i] = self.eval_node_old(node_id);
                }
            }
        }
    }

    /// Evaluate a signal for an entire buffer
    ///
    /// Fills output buffer with signal values. Handles all Signal variants.
    fn eval_signal_buffer(&mut self, signal: &Signal, output: &mut [f32]) {
        match signal {
            Signal::Value(v) => {
                // Constant: fill with same value
                output.fill(*v);
            }
            Signal::Node(id) => {
                // Node reference: evaluate node
                self.eval_node_buffer(id, output);
            }
            Signal::Bus(name) => {
                // Bus reference: evaluate bus node
                if let Some(&id) = self.buses.get(name) {
                    self.eval_node_buffer(&id, output);
                } else {
                    output.fill(0.0);
                }
            }
            Signal::Pattern(pattern_str) => {
                // Pattern: query for each sample in buffer
                self.eval_pattern_buffer(pattern_str, output);
            }
            Signal::Expression(expr) => {
                // Arithmetic expression: evaluate recursively
                self.eval_expression_buffer(expr, output);
            }
        }
    }
}
```

---

## Buffer Allocation Strategy

### Option 1: Stack Allocation (Rejected)
```rust
let mut temp_buffer = [0.0f32; 512];  // Stack allocation
```

**Pros:**
- Very fast
- No heap allocation overhead

**Cons:**
- Fixed size (not flexible)
- Large stack frames (512 × 4 bytes = 2KB per buffer)
- Recursion depth could cause stack overflow

**Decision:** ❌ Rejected - Too risky with deep recursion

### Option 2: Vec Allocation per Call (Simple but Slow)
```rust
fn eval_add_buffer(&mut self, a: &Signal, b: &Signal, output: &mut [f32]) {
    let mut a_buffer = vec![0.0; output.len()];  // Heap allocation
    let mut b_buffer = vec![0.0; output.len()];  // Heap allocation
    self.eval_signal_buffer(a, &mut a_buffer);
    self.eval_signal_buffer(b, &mut b_buffer);
    for i in 0..output.len() {
        output[i] = a_buffer[i] + b_buffer[i];
    }
}
```

**Pros:**
- Simple to implement
- Flexible size
- Easy to understand

**Cons:**
- Allocation overhead for every intermediate buffer
- Many allocations per buffer (could be 10-20+ for complex graphs)

**Decision:** ✅ Use for initial implementation, optimize later

### Option 3: Buffer Pool (Future Optimization)
```rust
struct BufferPool {
    available: Vec<Vec<f32>>,
    in_use: usize,
}

impl BufferPool {
    fn acquire(&mut self, size: usize) -> Vec<f32> {
        if let Some(mut buf) = self.available.pop() {
            buf.resize(size, 0.0);
            buf
        } else {
            vec![0.0; size]
        }
    }

    fn release(&mut self, buf: Vec<f32>) {
        self.available.push(buf);
    }
}
```

**Pros:**
- Reuses allocations
- Amortizes allocation cost

**Cons:**
- More complex
- Lifetime management tricky
- Need to measure if it's actually faster

**Decision:** 🔮 Future optimization if profiling shows allocation overhead

### Option 4: Workspace Buffers (Best Long-term)
```rust
struct EvalWorkspace {
    temp_buffers: Vec<Vec<f32>>,  // Pre-allocated temp buffers
    current: usize,
}

impl UnifiedSignalGraph {
    fn with_temp_buffer<F>(&mut self, size: usize, f: F)
    where F: FnOnce(&mut [f32])
    {
        let buf = &mut self.workspace.temp_buffers[self.workspace.current];
        buf.resize(size, 0.0);
        self.workspace.current += 1;
        f(buf);
        self.workspace.current -= 1;
    }
}
```

**Pros:**
- Zero allocations during evaluation
- Stack-like buffer management
- Fast

**Cons:**
- Requires careful buffer count estimation
- More complex implementation

**Decision:** 🎯 Target for Phase 2 (after basic buffer eval works)

---

## Stateful Node Handling

### The Challenge

Stateful nodes need to:
1. Maintain state across buffers (phase, filter state, delay lines)
2. Update state AFTER processing entire buffer
3. Handle per-sample state updates (filters need per-sample state)

### Design: Per-Sample State Updates Within Buffer

**For filters (SVF, biquad):**
```rust
fn eval_lpf_buffer(&mut self, node_id: &NodeId, input: &Signal, cutoff: &Signal, q: &Signal, output: &mut [f32]) {
    let buffer_size = output.len();

    // Evaluate input signal to temporary buffer
    let mut input_buffer = vec![0.0; buffer_size];
    self.eval_signal_buffer(input, &mut input_buffer);

    // Evaluate parameters (might be constant or modulated)
    let mut cutoff_buffer = vec![0.0; buffer_size];
    let mut q_buffer = vec![0.0; buffer_size];
    self.eval_signal_buffer(cutoff, &mut cutoff_buffer);
    self.eval_signal_buffer(q, &mut q_buffer);

    // Get current filter state
    let mut state = self.get_lpf_state(node_id);

    // Process buffer sample-by-sample (state updates each sample)
    for i in 0..buffer_size {
        let fc = cutoff_buffer[i].max(20.0).min(20000.0);
        let q_val = q_buffer[i].max(0.5).min(20.0);

        // Check if coefficients changed (caching)
        let params_changed = (fc - state.cached_fc).abs() > 0.1
            || (q_val - state.cached_q).abs() > 0.001;

        if params_changed {
            state.cached_fc = fc;
            state.cached_q = q_val;
            state.cached_f = 2.0 * (PI * fc / self.sample_rate).sin();
            state.cached_damp = 1.0 / q_val;
        }

        // SVF tick (updates state)
        let high = input_buffer[i] - state.low - state.cached_damp * state.band;
        state.band += state.cached_f * high;
        state.low += state.cached_f * state.band;

        output[i] = state.low;
    }

    // Update filter state in node
    self.update_lpf_state(node_id, state);
}
```

**Key insight:** Even with buffer evaluation, filters still need per-sample processing for state updates. But we gain:
- Only ONE function call instead of 512
- Better cache locality (buffers sequential)
- Compiler can optimize the inner loop
- Coefficient caching still works

**For oscillators (phase accumulation):**
```rust
fn eval_oscillator_buffer(&mut self, node_id: &NodeId, freq: &Signal, waveform: &Waveform, output: &mut [f32]) {
    let buffer_size = output.len();

    // Evaluate frequency signal
    let mut freq_buffer = vec![0.0; buffer_size];
    self.eval_signal_buffer(freq, &mut freq_buffer);

    // Get current phase
    let mut phase = self.get_oscillator_phase(node_id);

    // Generate waveform
    for i in 0..buffer_size {
        match waveform {
            Waveform::Sine => {
                output[i] = phase.sin();
            }
            Waveform::Saw => {
                output[i] = self.polyblep_saw(phase, freq_buffer[i]);
            }
            // ... other waveforms
        }

        // Advance phase
        phase += 2.0 * PI * freq_buffer[i] / self.sample_rate;
        if phase >= 2.0 * PI {
            phase -= 2.0 * PI;
        }
    }

    // Update phase in node
    self.update_oscillator_phase(node_id, phase);
}
```

---

## Migration Strategy

### Phase 1: Coexistence

Both APIs exist side-by-side:
- `eval_node()` - Old API (keep for now)
- `eval_node_buffer()` - New API (add)

Nodes implement both:
```rust
match node {
    SignalNode::Sine { .. } => {
        // Buffer evaluation implemented
        self.eval_sine_buffer(...)
    }
    SignalNode::OtherNode { .. } => {
        // Not yet migrated, fall back to sample-by-sample
        for i in 0..output.len() {
            output[i] = self.eval_node(&node_id);
        }
    }
}
```

### Phase 2: Gradual Migration

Migrate nodes one-by-one:
1. Oscillators first (sources)
2. Arithmetic ops (simple)
3. Filters (stateful but understood)
4. Effects (complex stateful)

### Phase 3: Phase 3 Integration

Update `process_buffer_hybrid()`:
```rust
// OLD:
for i in 0..buffer_size {
    buffer[i] = self.eval_node(&output_id);
}

// NEW:
self.eval_node_buffer(&output_id, &mut buffer[0..buffer_size]);
```

### Phase 4: Cleanup

Remove old `eval_node()` once all nodes migrated.

---

## Testing Strategy

### Unit Test Template

For each migrated node:
```rust
#[test]
fn test_{node}_buffer_matches_sample() {
    let mut graph = create_test_graph();
    let node_id = graph.add_{node}(...);

    // Generate reference output (sample-by-sample)
    let mut sample_output = vec![0.0; 512];
    for i in 0..512 {
        sample_output[i] = graph.eval_node(&node_id);
    }

    // Generate buffer output
    graph.reset_state();  // Reset to same initial state
    let mut buffer_output = vec![0.0; 512];
    graph.eval_node_buffer(&node_id, &mut buffer_output);

    // Compare
    for i in 0..512 {
        let diff = (sample_output[i] - buffer_output[i]).abs();
        assert!(diff < 1e-6, "Sample {} differs: {} vs {} (diff: {})",
            i, sample_output[i], buffer_output[i], diff);
    }
}
```

### State Continuity Test

Ensure state carries across buffer boundaries:
```rust
#[test]
fn test_{node}_buffer_state_continuity() {
    let mut graph = create_test_graph();
    let node_id = graph.add_{node}(...);

    // Process 3 buffers
    let mut full_output = vec![0.0; 1536];  // 3 × 512

    let mut buffer = vec![0.0; 512];
    for buf_idx in 0..3 {
        graph.eval_node_buffer(&node_id, &mut buffer);
        full_output[buf_idx * 512..(buf_idx + 1) * 512].copy_from_slice(&buffer);
    }

    // Compare to continuous sample-by-sample
    graph.reset_state();
    let mut sample_output = vec![0.0; 1536];
    for i in 0..1536 {
        sample_output[i] = graph.eval_node(&node_id);
    }

    // Should match
    for i in 0..1536 {
        let diff = (sample_output[i] - full_output[i]).abs();
        assert!(diff < 1e-6, "Sample {} differs", i);
    }
}
```

---

## Performance Expectations

### Theoretical Speedup

**Function call overhead elimination:**
- Before: 512 calls × ~10ns = 5.1μs overhead
- After: 1 call × ~10ns = 0.01μs overhead
- **Savings:** ~5μs per node evaluation

For a graph with 20 nodes evaluated per sample:
- Before: 512 × 20 = 10,240 function calls
- After: 20 function calls
- **Savings:** ~100μs just from function call overhead

**Cache locality:**
- Sequential buffer access vs random node access
- Prefetcher can predict buffer access patterns
- **Expected:** 10-20% additional speedup

**Compiler optimizations:**
- Loop unrolling (process 4-8 samples at once)
- SIMD vectorization (process 4-8 samples in parallel)
- **Expected:** 2-4x speedup for simple operations (add, multiply)

**Combined:** 3-5x total speedup for Phase 3

### Measuring Performance

Add profiling:
```rust
let start = std::time::Instant::now();
self.eval_node_buffer(&output_id, buffer);
let duration = start.elapsed();
println!("Buffer eval: {:?}", duration);
```

Compare old vs new:
```
Old (sample-by-sample): 10-22ms
Target (buffer-based): 3-7ms
```

---

## Risks & Mitigation

### Risk 1: Allocation Overhead

**Risk:** Vec allocations for temp buffers could be slow

**Mitigation:**
- Profile first (might not be a problem)
- If needed, implement buffer pool
- Or workspace buffers

### Risk 2: Breaking Audio Correctness

**Risk:** Buffer evaluation produces different output than sample-by-sample

**Mitigation:**
- Comprehensive testing (unit tests compare outputs)
- Keep old code during transition for comparison
- Gradual migration (one node at a time)

### Risk 3: State Management Bugs

**Risk:** Stateful nodes don't carry state correctly across buffers

**Mitigation:**
- State continuity tests
- Test with long renders (multiple buffers)
- Visual inspection (plot waveforms)

### Risk 4: Complexity

**Risk:** Buffer-based code is harder to understand/maintain

**Mitigation:**
- Good documentation
- Clear code examples
- Gradual migration allows learning

---

## Next Steps

### Immediate (This Session)

1. ✅ Complete this study document
2. 🔨 **Begin implementation:** Add `eval_node_buffer()` skeleton
3. 🔨 Implement `eval_signal_buffer()`
4. 🔨 Implement one simple node (Constant)
5. 🧪 Write first test

### Short-term (This Week)

1. Implement Sine oscillator buffer evaluation
2. Write comprehensive tests
3. Verify correctness
4. Measure performance improvement
5. Document learnings

### Medium-term (Next 2 Weeks)

1. Migrate all oscillators
2. Migrate arithmetic operations
3. Update Phase 3 to use buffer eval
4. Verify all tests pass

---

## Design Decisions Summary

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Buffer allocation | Vec per call initially | Simple, optimize later if needed |
| State management | Per-sample updates within buffer | Required for filters, minimal overhead |
| Migration strategy | Gradual, node-by-node | Lower risk, easier to debug |
| Testing approach | Compare to sample-by-sample | Ensures correctness |
| API design | Similar to current, take `&mut [f32]` | Familiar, flexible |

---

## Conclusion

Buffer-based evaluation is architecturally sound and should provide 3-5x Phase 3 speedup with manageable implementation complexity. The key insights are:

1. **Reduce function calls:** 512 → 1 per buffer
2. **Enable compiler optimizations:** SIMD, loop unrolling
3. **Improve cache locality:** Sequential buffer access
4. **Gradual migration:** Coexist with old API during transition
5. **Comprehensive testing:** Ensure audio correctness

**Ready to proceed to Implementation phase!** ✅
