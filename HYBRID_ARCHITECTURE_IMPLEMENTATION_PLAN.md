# Hybrid Architecture Implementation Plan

## Goal
Achieve <11.61ms performance target using SuperCollider/Glicol-inspired architecture

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│ PATTERN LAYER (Sample-Accurate)                            │
│ - Evaluate patterns at sample rate                          │
│ - Track cycle positions (pre-computed)                      │
│ - Trigger voices based on pattern events                    │
│ - Lightweight, no DSP                                        │
└─────────────────────────────────────────────────────────────┘
                           ↓ triggers
┌─────────────────────────────────────────────────────────────┐
│ VOICE LAYER (Block-Based)                                   │
│ - Render all active voices to buffers                       │
│ - SIMD-optimized (already done)                            │
│ - Returns HashMap<NodeId, Vec<f32>>                        │
│ - One buffer per Sample node                                │
└─────────────────────────────────────────────────────────────┘
                           ↓ buffers
┌─────────────────────────────────────────────────────────────┐
│ DSP LAYER (Block-Based, No Recursion)                      │
│ - Process nodes in topological order                        │
│ - Read from input buffers                                   │
│ - Write to output buffers                                   │
│ - No eval_node recursion!                                   │
└─────────────────────────────────────────────────────────────┘
```

## Implementation Phases

### Phase 1: Remove Broken Code ✅
- Remove debug statements from mix_output_buffers
- Keep: profiling, cycle optimization, dependency analysis
- Remove: broken block processing implementation

### Phase 2: Voice Manager Buffer API
**File:** `src/voice_manager.rs`

**Current:**
```rust
fn process() -> f32  // Returns single sample
```

**New:**
```rust
fn render_block(&mut self, block_size: usize) -> HashMap<usize, Vec<f32>> {
    // Returns one buffer per source node
    // Key: node_id (from set_default_source_node)
    // Value: buffer of block_size samples
}
```

**Implementation:**
1. Add `render_block()` method
2. Keep existing `process()` for compatibility
3. Accumulate voices by source_node_id
4. SIMD-optimize the accumulation

### Phase 3: Pattern Evaluation Loop
**File:** `src/unified_graph.rs`

**New method:**
```rust
fn evaluate_patterns_and_trigger_voices(&mut self, buffer_size: usize) {
    // Pre-compute cycle positions (already have this)
    let cycle_positions = self.precompute_cycle_positions(buffer_size);

    // For each sample, update position and evaluate Sample nodes
    for i in 0..buffer_size {
        self.cached_cycle_position = cycle_positions[i];

        // Find all Sample nodes and trigger voices
        for (node_id, node) in &self.nodes {
            if let Some(SignalNode::Sample { pattern, .. }) = node {
                // Query pattern, check for new events
                // If event should trigger, call voice_manager.trigger_voice()
                // DON'T render audio here!
            }
        }
    }
}
```

### Phase 4: Buffer-Based DSP Evaluation
**File:** `src/unified_graph.rs`

**New method:**
```rust
fn eval_node_from_buffers(
    &self,
    node_id: &NodeId,
    sample_idx: usize,
    buffers: &HashMap<NodeId, Vec<f32>>,
) -> f32 {
    match &self.nodes[node_id] {
        SignalNode::Sample { .. } => {
            // Read from voice buffers (passed in)
            buffers.get(node_id)
                .and_then(|buf| buf.get(sample_idx))
                .copied()
                .unwrap_or(0.0)
        }
        SignalNode::Add { a, b } => {
            // Read from dependency buffers
            let a_val = self.eval_signal_from_buffers(a, sample_idx, buffers);
            let b_val = self.eval_signal_from_buffers(b, sample_idx, buffers);
            a_val + b_val
        }
        // ... all other node types read from buffers
    }
}
```

### Phase 5: Hybrid process_buffer()
**File:** `src/unified_graph.rs`

**Replace process_buffer() with:**
```rust
pub fn process_buffer(&mut self, buffer: &mut [f32]) -> Result<(), String> {
    let buffer_size = buffer.len();

    // PHASE 1: Pattern Evaluation (sample-accurate)
    self.evaluate_patterns_and_trigger_voices(buffer_size);

    // PHASE 2: Voice Rendering (block-based)
    let voice_buffers = self.voice_manager.borrow_mut()
        .render_block(buffer_size);

    // PHASE 3: DSP Processing (block-based, topologically ordered)
    let stages = self.compute_execution_stages()?;
    let mut node_buffers = HashMap::new();

    // Merge voice_buffers into node_buffers
    for (node_id, voice_buf) in voice_buffers {
        node_buffers.insert(NodeId(node_id), voice_buf);
    }

    // Process each stage
    for stage in stages.stages {
        for &node_id in &stage {
            // Render full buffer for this node
            let mut samples = Vec::with_capacity(buffer_size);
            for i in 0..buffer_size {
                let sample = self.eval_node_from_buffers(&node_id, i, &node_buffers);
                samples.push(sample);
            }
            node_buffers.insert(node_id, samples);
        }
    }

    // PHASE 4: Mix to output
    self.mix_buffers_to_output(buffer, &node_buffers);

    Ok(())
}
```

### Phase 6: Testing Strategy

**Test 1: Correctness**
```bash
# Render same pattern with old and new code
cargo run --bin phonon -- render test.ph old.wav
USE_HYBRID=1 cargo run --bin phonon -- render test.ph new.wav

# Compare waveforms (should be identical)
diff old.wav new.wav
```

**Test 2: Performance**
```bash
USE_HYBRID=1 PROFILE_BUFFER=1 cargo run --release --bin phonon -- live stress_extreme.ph
# Target: <11.61ms
```

## Expected Performance Gains

**Current (sample-by-sample):**
- 64ms for 16 outputs (4ms per node × 16)
- eval_node recursion: 99.7% of time
- Cycle updates: 0.3%

**After hybrid:**
- Pattern evaluation: ~2ms (lightweight, no DSP)
- Voice rendering: ~5ms (already SIMD-optimized)
- DSP processing: ~2ms (no recursion overhead)
- **Total: ~9ms** ✅ UNDER BUDGET!

## Risk Mitigation

1. **Keep old code** - Don't delete process_sample(), use env var to switch
2. **Incremental testing** - Test after each phase
3. **Comprehensive logging** - Add debug output for hybrid mode
4. **Fallback** - If hybrid breaks, can revert to sample-by-sample

## Success Criteria

- ✅ Audio output identical to sample-by-sample mode
- ✅ Performance <11.61ms on stress_extreme.ph
- ✅ No audio glitches or artifacts
- ✅ All existing tests pass

## Timeline

- Phase 1: 30 min (cleanup)
- Phase 2: 2 hours (voice manager)
- Phase 3: 2 hours (pattern evaluation)
- Phase 4: 3 hours (buffer-based DSP)
- Phase 5: 1 hour (integration)
- Phase 6: 2 hours (testing)
- **Total: ~10-11 hours** (overnight work!)

Let's go! 🚀
