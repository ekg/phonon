# Hybrid Architecture Implementation Progress

## Session Goals
Implement hybrid architecture (SuperCollider/Glicol-inspired) to achieve <11.61ms performance target (currently 55-99ms).

---

## ✅ Completed

### Phase 1: Remove Broken Block Processing ✅
**Status:** Complete
**Commit:** ed0bd8f

**What was removed:**
- `process_buffer_stages()` - Broken DAW-style block rendering
- `render_node_to_buffer()` - Part of broken implementation
- `mix_output_buffers()` - Part of broken implementation
- `USE_BLOCK_PROCESSING` environment variable conditional
- Debug `eprintln!` statements

**What was kept:**
- `precompute_cycle_positions()` - Good infrastructure for hybrid
- `compute_execution_stages()` - Dependency analysis (topological sort)
- Profiling infrastructure (`PROFILE_DETAILED`)

**Why removed:**
The block processing produced silent audio because:
- `voice_output_cache` stored ONE value per node (per buffer)
- Block processing called `eval_node()` 512 times with different cycle positions
- Cache was stale → returned zeros

### Phase 2: Buffer-Based Voice Rendering ✅
**Status:** Complete
**Commit:** 149776d

**Added to `voice_manager.rs`:**
```rust
pub fn render_block(&mut self, block_size: usize) -> HashMap<usize, Vec<f32>>
```

**Returns:** One buffer per source node
- Key: source_node_id (from `set_default_source_node`)
- Value: `Vec<f32>` of block_size samples

**Implementation details:**
- Parallel path: Voices rendered in parallel when count ≥ `parallel_threshold`
- Sequential path: For low voice counts
- Direct accumulation by source_node_id (no transpose)
- Reuses existing SIMD/parallel infrastructure from `process_buffer_per_node()`

---

## 🚧 Phase 3 Challenge: Pattern Evaluation Loop

### The Intended Architecture

**Goal:** Separate pattern evaluation (sample-accurate) from audio rendering (block-based)

```rust
pub fn process_buffer(&mut self, buffer: &mut [f32]) {
    // PHASE 1: Pattern evaluation (sample-accurate)
    self.evaluate_patterns_and_trigger_voices(buffer_size);

    // PHASE 2: Voice rendering (block-based)
    let voice_buffers = self.voice_manager.borrow_mut().render_block(buffer_size);

    // PHASE 3: DSP processing (block-based)
    // Read from buffers, no eval_node recursion
}
```

### The Problem

**Timing Challenge:** When should voices start producing audio?

**Example scenario:**
- Buffer has 512 samples
- Pattern event triggers voice at sample position 100
- When we call `render_block()` after all triggers, voice has been triggered but is at position 0
- Voice renders 512 samples starting from position 0
- But it SHOULD produce zeros for samples 0-99, then start at sample 100

**Current voice manager limitation:**
- Voices don't track "trigger offset within buffer"
- They just have playback position (position within the sample data)
- No concept of "start producing audio at sample offset N"

### Current Architecture (Works But Slow)

```rust
pub fn process_buffer(&mut self, buffer: &mut [f32]) {
    // Pre-render all voices for entire buffer (voices already playing)
    let voice_buffers = self.voice_manager.borrow_mut().process_buffer_per_node(buffer.len());

    // For each sample:
    for i in 0..buffer.len() {
        self.update_cycle_position_from_clock();
        self.voice_output_cache = std::mem::take(&mut voice_buffers[i]);

        // eval_node() does:
        // 1. Query patterns (for Sample nodes)
        // 2. Trigger NEW voices
        // 3. Read from voice_output_cache (voices from previous samples)
        // 4. Return audio
        mixed_output = self.eval_node(&output_id);
        buffer[i] = mixed_output;
    }
}
```

**Why this works:**
- Newly triggered voices produce audio in SUBSEQUENT samples of same buffer
- Voice triggered at sample 100 produces audio at samples 101, 102, etc.
- Sample-accurate because triggering happens in the sample loop

**Why it's slow:**
- `eval_node()` called 512 times per buffer
- Recursive graph traversal is 99.7% of processing time

### Possible Solutions

#### Option A: Voice Trigger Offset Support
Modify voice manager to track when each voice was triggered within the buffer:

```rust
struct Voice {
    // ... existing fields ...
    buffer_trigger_offset: Option<usize>,  // Which sample in buffer this voice was triggered
}

impl VoiceManager {
    fn trigger_sample_at_offset(&mut self, sample: Arc<Vec<f32>>, offset: usize, ...) {
        // Set buffer_trigger_offset = offset
    }

    fn render_block(&mut self, block_size: usize) -> HashMap<usize, Vec<f32>> {
        // For each voice:
        // - Produce zeros for samples [0..buffer_trigger_offset)
        // - Produce audio for samples [buffer_trigger_offset..block_size)
    }
}
```

**Pros:**
- Clean separation of pattern evaluation and rendering
- Aligns with original hybrid plan

**Cons:**
- Requires modifying voice manager (complex, 60+ voice trigger methods)
- Need to track and reset offsets per buffer
- More state to manage

#### Option B: Incremental Hybrid (Simpler)
Keep sample-by-sample triggering, but make DSP buffer-based:

```rust
pub fn process_buffer(&mut self, buffer: &mut [f32]) {
    // PHASE 1: Trigger voices sample-by-sample (keeps timing correct)
    for i in 0..buffer.len() {
        self.update_cycle_position_from_clock();

        // Only evaluate Sample nodes to trigger voices
        for node_id in &self.sample_node_ids {
            self.eval_sample_node_for_triggering(node_id);  // Don't return audio
        }

        self.sample_count += 1;
    }

    // PHASE 2: Render all voices
    let voice_buffers = self.voice_manager.borrow_mut().render_block(buffer.len());

    // PHASE 3: DSP evaluation from buffers (no Sample node recursion)
    for i in 0..buffer.len() {
        buffer[i] = self.eval_dsp_from_buffers(i, &voice_buffers);
    }
}
```

**Issue:** Voices are already at position 512 after Phase 1 loop. render_block() would render from positions 512-1024, not 0-512.

#### Option C: Split Voice Update and Rendering
Separate voice state update from audio rendering:

```rust
impl Voice {
    fn update_state(&mut self) {
        // Advance envelope, check if finished, etc.
        // DON'T advance playback position
    }

    fn render_sample(&mut self) -> (f32, f32) {
        // Render one sample AND advance position
    }
}
```

Then:
```rust
// Phase 1: Update voice states + trigger new voices
for i in 0..buffer.len() {
    self.voice_manager.update_all_voice_states();  // Don't render
    // Evaluate patterns, trigger new voices
}

// Phase 2: Render all voices from their starting positions
let voice_buffers = self.voice_manager.render_block(buffer.len());
```

**Issue:** Voice state and rendering are currently tightly coupled. Significant refactoring needed.

---

## Recommended Path Forward

### Short-term: Optimize Within Current Architecture

Instead of radical restructuring, optimize the recursive `eval_node()`:

1. **Buffer-based evaluation for non-Sample nodes**
   - Sample nodes stay sample-by-sample (they're fast enough with caching)
   - Other nodes (Add, Multiply, Sine, etc.) become buffer-based
   - Reduces recursion overhead

2. **Memoization**
   - Cache full buffers for nodes, not just single values
   - Invalidate only when inputs change

3. **SIMD for node operations**
   - Process 8 samples at once for arithmetic nodes

**Expected speedup:** 2-4x (might not hit <11.61ms target)

### Long-term: True Hybrid with Voice Offset Support

Implement Option A (voice trigger offset) properly:

1. Add `trigger_offset` field to Voice
2. Update all 60+ trigger methods to accept offset parameter
3. Modify `render_block()` to respect offsets
4. Implement 3-phase process_buffer()

**Expected speedup:** 5-10x ✅ **Should hit target**

**Estimated work:** 4-6 hours (significant refactoring)

---

## Questions for User/Next Session

1. **Should we pursue incremental optimization** (2-4x, safer, faster to implement) **or true hybrid** (5-10x, higher risk, more work)?

2. **Is there a SuperCollider/Glicol example** showing how they handle sample-accurate event triggering with block-based rendering?

3. **Can we accept slight latency?** If voices can start up to 1 buffer late (11ms), implementation becomes much simpler.

---

## Files Modified This Session

- `src/unified_graph.rs` - Removed broken block processing (Phase 1)
- `src/voice_manager.rs` - Added `render_block()` method (Phase 2)
- `HYBRID_ARCHITECTURE_IMPLEMENTATION_PLAN.md` - Created original plan
- `HYBRID_IMPLEMENTATION_PROGRESS.md` - This file

## Commits

- `ed0bd8f` - Phase 1: Remove broken block processing code
- `149776d` - Phase 2: Implement buffer-based voice rendering

---

## Current Performance Baseline

- **Target:** <11.61ms per 512-sample buffer
- **Current:** 55-99ms (5-9x over budget)
- **Bottleneck:** `eval_node()` recursive calls (99.7% of time)

**Next steps depend on chosen strategy above.**
