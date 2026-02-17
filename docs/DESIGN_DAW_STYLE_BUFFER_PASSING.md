# Design: DAW-Style Block-Based Buffer Passing

**Author**: Claude (design task)
**Date**: 2026-01-28
**Status**: Design Complete - Ready for Implementation Review

---

## Executive Summary

This document presents a design for migrating Phonon's audio processing from hybrid sample-by-sample evaluation to true DAW-style block-based buffer passing. The goal is to improve performance, enable better SIMD optimization, and reduce function call overhead while maintaining sample-accurate timing for pattern triggering.

**Key Insight**: Phonon already has significant block-based infrastructure (`VoiceBuffers`, `dag_buffer_cache`, `process_buffer_dag`). The design extends these patterns to **all node evaluation**, not just voice playback.

---

## 1. Current Architecture Analysis

### 1.1 What We Have

Phonon currently uses a **hybrid architecture**:

**Block-Based Components** (already optimized):
- `VoiceBuffers` - O(1) lookup, pre-rendered per buffer (`voice_manager.rs:115-195`)
- `dag_buffer_cache` - Caches buffers for DAG processing (`unified_graph.rs:4623`)
- `prev_node_buffers` - Feedback support via 1-block delay (`unified_graph.rs:4614`)
- `precompute_pattern_events()` - Queries patterns once per buffer (`unified_graph.rs:17421`)

**Per-Sample Components** (bottleneck):
- `eval_node()` - Called 512× per buffer per output node (`unified_graph.rs:9985`)
- `eval_node_buffer_dag()` - Falls back to per-sample loop (`unified_graph.rs:7617-7697`)
- `stateful_value_cache` - Cleared every sample (`unified_graph.rs:7625`)

### 1.2 Current Buffer Flow

```
process_buffer(&mut stereo_buffer)
│
├─ voice_manager.process_buffer_vec(512, max_node_id) → VoiceBuffers  [BLOCK-BASED ✓]
│
├─ precompute_pattern_events(512)                                      [BLOCK-BASED ✓]
│
├─ Synthesis voice generation (per-pitch buffers)                      [BLOCK-BASED ✓]
│
├─ DAG topological sort and processing:
│  └─ eval_node_buffer_dag() for each node:
│     └─ FOR i IN 0..512:                                              [PER-SAMPLE ✗]
│        ├─ Update cached_cycle_position
│        ├─ Clear stateful_value_cache
│        ├─ eval_node(&NodeId(node_id))
│        └─ Handle newly triggered voices
│
└─ Output mixing and limiter                                           [BLOCK-BASED ✓]
```

### 1.3 Identified Bottlenecks

| Bottleneck | Impact | Cause |
|------------|--------|-------|
| `eval_node()` calls | ~262K calls/sec for simple graph | 512 samples × 512 buffers/sec |
| `stateful_value_cache.clear()` | HashMap allocation overhead | Cleared every sample |
| Function call overhead | Branch prediction, stack frames | Deep recursion per sample |
| SIMD prevention | Cannot vectorize | Operations are sample-by-sample |

### 1.4 Why Per-Sample is Currently Required

The comment at line 7601-7604 explains:
> "Block-based optimization was attempted here but caused issues with Envelope nodes (timing/state not correctly preserved)."

**Root causes**:
1. **Envelope state** - ADSR envelopes advance per-sample; block evaluation breaks state continuity
2. **Voice triggering** - Sample nodes can trigger voices mid-buffer; pre-rendering misses them
3. **Oscillator phase** - Phase must advance continuously across samples

---

## 2. Proposed Architecture

### 2.1 Core Principle: "Buffers All The Way Down"

Every node receives **input buffers** and produces **output buffers**. No node ever calls `eval_node()` for a single sample.

```rust
// NEW: All nodes implement buffer-based evaluation
fn eval_node_buffer_v2(
    &mut self,
    node_id: &NodeId,
    inputs: &[&[f32]],    // Pre-rendered input buffers
    output: &mut [f32],    // Output buffer to fill
    buffer_size: usize,
    buffer_start_cycle: f64,
    sample_increment: f64,
);
```

### 2.2 The AudioBuffer Type

Introduce a dedicated buffer type optimized for DSP:

```rust
/// Fixed-size audio buffer for block processing
/// Designed for SIMD alignment and cache efficiency
#[repr(align(32))]  // AVX-256 alignment
pub struct AudioBuffer {
    data: [f32; MAX_BUFFER_SIZE],  // 512 samples max
    len: usize,                     // Actual samples in use
}

impl AudioBuffer {
    #[inline(always)]
    pub fn as_slice(&self) -> &[f32] {
        &self.data[..self.len]
    }

    #[inline(always)]
    pub fn as_mut_slice(&mut self) -> &mut [f32] {
        &mut self.data[..self.len]
    }
}

/// Maximum buffer size (powers of 2 for SIMD)
pub const MAX_BUFFER_SIZE: usize = 512;
```

### 2.3 Node Buffer Interface

```rust
/// Trait for nodes that can be evaluated in block mode
pub trait BlockEvaluable {
    /// Process an entire buffer at once
    ///
    /// # Arguments
    /// * `inputs` - Pre-rendered input buffers indexed by input port
    /// * `output` - Output buffer to fill
    /// * `ctx` - Buffer context (timing, sample rate, etc.)
    fn process_block(
        &mut self,
        inputs: &[&AudioBuffer],
        output: &mut AudioBuffer,
        ctx: &BufferContext,
    );

    /// Number of input ports this node expects
    fn input_count(&self) -> usize;

    /// Whether this node needs sample-accurate timing (patterns, triggers)
    fn needs_sample_accurate_timing(&self) -> bool;
}

/// Context provided to block processing
pub struct BufferContext {
    pub buffer_size: usize,
    pub sample_rate: f32,
    pub buffer_start_cycle: f64,
    pub sample_increment: f64,
    pub cps: f32,
}
```

### 2.4 Node Categories by Processing Strategy

**Category A: Pure Block Nodes** (SIMD-optimizable)
- Constant, Add, Multiply, Min, Max
- LowPass, HighPass, BandPass (stateful but deterministic)
- Delay, Reverb, Distortion (buffer-to-buffer transforms)
- Oscillator (phase accumulator across buffer)

**Category B: Timing-Sensitive Nodes** (hybrid)
- Sample (pattern queries + voice triggering)
- Pattern (converts pattern events to control signal)
- Envelope (may need per-sample for attack precision)

**Category C: Feedback Nodes** (special handling)
- UnitDelay (reads from previous buffer)
- Self-referential buses

### 2.5 Block Processing Strategy by Node Type

#### Category A: Pure Block Processing

```rust
// Example: Add node
impl BlockEvaluable for AddNode {
    fn process_block(
        &mut self,
        inputs: &[&AudioBuffer],
        output: &mut AudioBuffer,
        _ctx: &BufferContext,
    ) {
        let a = inputs[0].as_slice();
        let b = inputs[1].as_slice();
        let out = output.as_mut_slice();

        // SIMD-friendly loop (auto-vectorizes with -C opt-level=3)
        for i in 0..output.len {
            out[i] = a[i] + b[i];
        }
    }
}

// Example: LowPass filter (stateful)
impl BlockEvaluable for LowPassNode {
    fn process_block(
        &mut self,
        inputs: &[&AudioBuffer],
        output: &mut AudioBuffer,
        ctx: &BufferContext,
    ) {
        let input = inputs[0].as_slice();
        let cutoff = inputs[1].as_slice();  // Cutoff can be modulated
        let out = output.as_mut_slice();

        for i in 0..output.len {
            // Update filter coefficients if cutoff changed
            if (cutoff[i] - self.last_cutoff).abs() > 0.1 {
                self.update_coefficients(cutoff[i], ctx.sample_rate);
                self.last_cutoff = cutoff[i];
            }

            // Process sample through filter (maintains state across buffer)
            out[i] = self.process_sample(input[i]);
        }
    }
}
```

#### Category B: Timing-Sensitive (Hybrid)

Pattern nodes need sample-accurate timing but can still benefit from block organization:

```rust
impl BlockEvaluable for PatternNode {
    fn process_block(
        &mut self,
        _inputs: &[&AudioBuffer],
        output: &mut AudioBuffer,
        ctx: &BufferContext,
    ) {
        // Use pre-computed events from precompute_pattern_events()
        let events = self.get_cached_events(ctx.buffer_start_cycle);

        let out = output.as_mut_slice();
        let mut current_value = self.last_value;
        let mut event_idx = 0;

        for i in 0..output.len {
            let cycle_pos = ctx.buffer_start_cycle + (i as f64) * ctx.sample_increment;

            // Check if we've passed an event boundary
            while event_idx < events.len() &&
                  events[event_idx].time <= cycle_pos {
                current_value = events[event_idx].value;
                event_idx += 1;
            }

            out[i] = current_value;
        }

        self.last_value = current_value;
    }
}
```

#### Category C: Feedback Nodes

```rust
impl BlockEvaluable for UnitDelayNode {
    fn process_block(
        &mut self,
        inputs: &[&AudioBuffer],
        output: &mut AudioBuffer,
        _ctx: &BufferContext,
    ) {
        let out = output.as_mut_slice();

        // Copy previous buffer's output
        out.copy_from_slice(&self.prev_buffer[..output.len]);

        // Store current input for next buffer
        self.prev_buffer[..output.len].copy_from_slice(inputs[0].as_slice());
    }
}
```

---

## 3. Buffer Pool Architecture

### 3.1 The Problem

Creating `Vec<f32>` buffers for every node every buffer causes:
- Heap allocation overhead (~200ns per allocation)
- Fragmentation
- Cache misses

### 3.2 Solution: Pre-allocated Buffer Pool

```rust
/// Pre-allocated pool of audio buffers for zero-allocation processing
pub struct BufferPool {
    buffers: Vec<AudioBuffer>,
    free_list: Vec<usize>,       // Indices of available buffers
    allocated: HashMap<NodeId, usize>,  // Node -> buffer index mapping
}

impl BufferPool {
    pub fn new(max_nodes: usize) -> Self {
        let mut buffers = Vec::with_capacity(max_nodes);
        let mut free_list = Vec::with_capacity(max_nodes);

        for i in 0..max_nodes {
            buffers.push(AudioBuffer::new());
            free_list.push(i);
        }

        Self {
            buffers,
            free_list,
            allocated: HashMap::with_capacity(max_nodes),
        }
    }

    /// Acquire a buffer for a node (O(1))
    pub fn acquire(&mut self, node_id: NodeId) -> &mut AudioBuffer {
        if let Some(&idx) = self.allocated.get(&node_id) {
            return &mut self.buffers[idx];
        }

        let idx = self.free_list.pop().expect("Buffer pool exhausted");
        self.allocated.insert(node_id, idx);
        &mut self.buffers[idx]
    }

    /// Reset pool for next buffer (O(n))
    pub fn reset(&mut self) {
        for (_, idx) in self.allocated.drain() {
            self.free_list.push(idx);
        }
    }

    /// Get a buffer by node ID (O(1))
    pub fn get(&self, node_id: NodeId) -> Option<&AudioBuffer> {
        self.allocated.get(&node_id).map(|&idx| &self.buffers[idx])
    }
}
```

### 3.3 Integration with DAG Processing

```rust
pub fn process_buffer_dag_v2(
    &mut self,
    output_buffer: &mut [f32],
    buffer_start_cycle: f64,
    sample_increment: f64,
) {
    let buffer_size = output_buffer.len() / 2;  // Stereo

    let ctx = BufferContext {
        buffer_size,
        sample_rate: self.sample_rate,
        buffer_start_cycle,
        sample_increment,
        cps: self.cps,
    };

    // 1. Reset buffer pool
    self.buffer_pool.reset();

    // 2. Pre-compute pattern events (existing optimization)
    self.precompute_pattern_events(buffer_size);

    // 3. Process voice buffers (existing optimization)
    self.voice_buffers = self.voice_manager.borrow_mut()
        .process_buffer_vec(buffer_size, self.max_node_id);

    // 4. Process nodes in topological order
    for node_id in self.topological_order.iter() {
        let output_buf = self.buffer_pool.acquire(NodeId(*node_id));
        output_buf.len = buffer_size;

        // Gather input buffers from dependencies
        let input_ids = self.get_dependencies(*node_id);
        let inputs: Vec<&AudioBuffer> = input_ids.iter()
            .filter_map(|&id| self.buffer_pool.get(NodeId(id)))
            .collect();

        // Process node
        self.process_node_block(*node_id, &inputs, output_buf, &ctx);
    }

    // 5. Mix output node to stereo buffer
    if let Some(output_id) = self.output {
        if let Some(output_buf) = self.buffer_pool.get(output_id) {
            // Mono to stereo expansion
            for i in 0..buffer_size {
                output_buffer[i * 2] = output_buf.data[i];
                output_buffer[i * 2 + 1] = output_buf.data[i];
            }
        }
    }

    // 6. Swap buffers for feedback (UnitDelay nodes)
    self.swap_feedback_buffers();
}
```

---

## 4. Handling Sample-Accurate Voice Triggering

The main challenge with block processing is that Sample nodes can trigger voices at any sample within the buffer. The current solution processes newly triggered voices per-sample.

### 4.1 Proposed Solution: Deferred Triggering

Instead of triggering voices during evaluation, collect trigger events and apply them afterward:

```rust
/// Voice trigger event with sample-accurate timing
struct VoiceTrigger {
    sample_offset: usize,      // Sample index within buffer
    sample_name: String,
    gain: f32,
    pan: f32,
    speed: f32,
    source_node: usize,
}

pub fn process_sample_node_block(
    &mut self,
    node_id: usize,
    output: &mut AudioBuffer,
    ctx: &BufferContext,
) -> Vec<VoiceTrigger> {
    let mut triggers = Vec::new();

    // Get pre-computed pattern events
    let events = self.pattern_event_cache.get(&node_id);

    if let Some(events) = events {
        for event in events {
            // Convert cycle position to sample offset
            let cycle_offset = event.part.begin.to_float() - ctx.buffer_start_cycle;
            let sample_offset = (cycle_offset / ctx.sample_increment).round() as usize;

            if sample_offset < ctx.buffer_size {
                triggers.push(VoiceTrigger {
                    sample_offset,
                    sample_name: event.value.clone(),
                    gain: event.controls.get("gain").copied().unwrap_or(1.0),
                    pan: event.controls.get("pan").copied().unwrap_or(0.0),
                    speed: event.controls.get("speed").copied().unwrap_or(1.0),
                    source_node: node_id,
                });
            }
        }
    }

    // Fill output from voice_buffers (existing voices)
    let out = output.as_mut_slice();
    for i in 0..ctx.buffer_size {
        out[i] = self.voice_buffers.get(node_id, i);
    }

    triggers
}

// In the main processing loop:
pub fn process_buffer_dag_v2(&mut self, ...) {
    // ... (setup)

    // Collect all triggers
    let mut all_triggers: Vec<VoiceTrigger> = Vec::new();

    for node_id in self.topological_order.iter() {
        // ... (process node)

        if is_sample_node(*node_id) {
            let triggers = self.process_sample_node_block(*node_id, output_buf, &ctx);
            all_triggers.extend(triggers);
        }
    }

    // Sort triggers by sample offset
    all_triggers.sort_by_key(|t| t.sample_offset);

    // Process newly triggered voices for remainder of buffer
    for trigger in all_triggers {
        let voice_idx = self.voice_manager.borrow_mut().trigger_sample_with_offset(
            &trigger.sample_name,
            trigger.gain,
            trigger.pan,
            trigger.speed,
            trigger.source_node,
            trigger.sample_offset,  // Start rendering from this offset
        );

        // Add newly triggered voice's output to the buffer
        if let Some(idx) = voice_idx {
            let remaining_samples = ctx.buffer_size - trigger.sample_offset;
            self.add_voice_to_buffer(idx, trigger.source_node, trigger.sample_offset, remaining_samples);
        }
    }
}
```

---

## 5. SIMD Optimization Opportunities

### 5.1 Explicit SIMD with wide crate

```rust
use wide::f32x8;

impl BlockEvaluable for AddNode {
    fn process_block(
        &mut self,
        inputs: &[&AudioBuffer],
        output: &mut AudioBuffer,
        _ctx: &BufferContext,
    ) {
        let a = inputs[0].as_slice();
        let b = inputs[1].as_slice();
        let out = output.as_mut_slice();
        let len = output.len;

        // Process 8 samples at a time with SIMD
        let simd_len = len / 8 * 8;
        for i in (0..simd_len).step_by(8) {
            let va = f32x8::from(&a[i..i+8]);
            let vb = f32x8::from(&b[i..i+8]);
            let vr = va + vb;
            vr.store(&mut out[i..i+8]);
        }

        // Handle remaining samples
        for i in simd_len..len {
            out[i] = a[i] + b[i];
        }
    }
}
```

### 5.2 Operations That Benefit Most from SIMD

| Operation | SIMD Speedup | Notes |
|-----------|--------------|-------|
| Add/Multiply/Mix | 6-8× | Pure arithmetic |
| Filter processing | 2-4× | State dependencies limit parallelism |
| Oscillator generation | 4-6× | Phase accumulation can be vectorized |
| Envelope application | 4-6× | Multiply by envelope buffer |
| Panning | 6-8× | Separate L/R multiplication |

### 5.3 Alignment Requirements

```rust
// Ensure 32-byte alignment for AVX-256
#[repr(align(32))]
pub struct AudioBuffer {
    data: [f32; MAX_BUFFER_SIZE],
    // ...
}
```

---

## 6. Implementation Plan

### Phase 1: Foundation (Low Risk)

1. **Add AudioBuffer type** with alignment
2. **Add BufferPool** to UnifiedSignalGraph
3. **Implement BlockEvaluable trait**
4. **Convert simple nodes** (Constant, Add, Multiply)

**Testing**: Ensure output matches existing `eval_node()` path

### Phase 2: DAG Integration (Medium Risk)

1. **Modify DAG processing** to use buffer pool
2. **Convert stateless DSP nodes** (filters, effects)
3. **Implement feedback buffer swap**
4. **Add feature flag** to toggle new path

**Testing**: A/B comparison of audio output

### Phase 3: Pattern/Sample Integration (Higher Risk)

1. **Implement deferred voice triggering**
2. **Convert Sample node** to block mode
3. **Convert Pattern node** to block mode
4. **Handle mid-buffer triggers**

**Testing**: Verify sample-accurate timing with onset detection

### Phase 4: SIMD Optimization

1. **Add wide crate** dependency
2. **Implement SIMD paths** for hot nodes
3. **Benchmark and profile**
4. **Tune buffer alignment**

**Testing**: Performance benchmarks, CPU utilization

### Phase 5: Cleanup

1. **Remove per-sample fallback** code
2. **Update documentation**
3. **Remove feature flag** (make block mode default)

---

## 7. Performance Projections

### Current Performance (512 samples/buffer at 44.1kHz)

| Metric | Value |
|--------|-------|
| Buffers per second | 86 |
| eval_node() calls per second | ~44,000+ |
| Function call overhead | ~10-15% CPU |
| SIMD utilization | ~0% |

### Projected Performance (Block Mode)

| Metric | Value | Improvement |
|--------|-------|-------------|
| Buffers per second | 86 | - |
| eval_node_block() calls per second | 86 per node | ~500× fewer |
| Function call overhead | <1% CPU | 10-15× less |
| SIMD utilization | 60-80% | New capability |

### Expected Overall Improvement

- **Simple graphs**: 2-3× faster
- **Complex graphs (many nodes)**: 5-10× faster
- **SIMD-heavy operations**: 3-6× faster per operation
- **Latency**: No change (still 512 samples)

---

## 8. Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Envelope timing breaks | Preserve per-sample loop for envelope nodes initially |
| Voice triggering timing | Deferred triggering with sample-accurate offsets |
| Feedback loop breaks | Explicit prev_buffer swap at buffer boundaries |
| Regression in output | A/B testing framework, bit-exact comparison |
| Code complexity increase | Feature flag for gradual migration |

---

## 9. Open Questions

1. **Stereo throughout?** Currently mono internally, stereo only at output. Should buffers be stereo pairs?

2. **Multi-output support** (`out1:`, `out2:`)? Each output needs separate buffer chain.

3. **Variable buffer size?** Currently fixed at 512. Should we support runtime changes?

4. **Plugin latency compensation?** VST3 plugins may report latency that needs compensation.

---

## 10. References

### External Resources
- [FunDSP Library](https://github.com/SamiPerttu/fundsp) - Rust audio DSP with block processing
- [Fixed vs. Variable Buffer Processing](https://medium.com/@12264447666.williamashley/fixed-vs-variable-buffer-processing-in-real-time-audio-dsp-performance-determinism-and-66da78390b0f) - DSP architecture patterns
- [DAW/Sequencer design discussion](https://www.kvraudio.com/forum/viewtopic.php?t=478572) - KVR Audio forum

### Internal References
- `src/unified_graph.rs:7081-7700` - Current `process_buffer_dag()` implementation
- `src/voice_manager.rs:1737-1814` - `process_buffer_vec()` VoiceBuffers
- `docs/BUFFER_SIZE_CONFIGURATION.md` - Existing buffer documentation

---

## 11. Conclusion

The proposed DAW-style block-based buffer passing architecture builds on Phonon's existing strengths while addressing the per-sample evaluation bottleneck. By:

1. Making **all nodes operate on buffers** instead of single samples
2. Using a **pre-allocated buffer pool** to eliminate allocations
3. Processing nodes in **topological order** with buffer passing
4. Enabling **SIMD optimization** through aligned, contiguous buffers
5. Using **deferred voice triggering** for sample-accurate timing

We can achieve significant performance improvements (2-10× depending on graph complexity) while maintaining Phonon's core feature: pattern-controlled, sample-rate modulation of synthesis parameters.

The implementation is divided into 5 phases with increasing risk levels, allowing for iterative development with continuous testing.

---

*Design document complete. Ready for implementation review.*
