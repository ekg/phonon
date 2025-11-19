# Block Processing Analysis: The Architecture is WRONG

## 🚨 Critical Discovery

Block processing as currently implemented **DOES NOT WORK** - it produces silent audio.

**Root cause:** Sample nodes rely on `voice_output_cache` which is populated ONCE per buffer, but block processing calls eval_node 512 times, all reading the same stale cached value.

---

## How Real Audio Systems Work

### SuperCollider (Message-Passing + Block Processing)

**Architecture:**
```
Server (audio thread):          Client (language thread):
- Runs at audio rate            - Sends OSC messages
- Block size: 64 samples         - Pattern evaluation
- UGens process blocks           - Scheduling
- No recursion!                  - Timing logic
```

**Key insights:**
1. **Synths are pre-compiled** - No graph traversal during audio processing
2. **Block-based** - UGens process 64 samples at once
3. **No sample-by-sample recursion** - Everything is buffers
4. **Message-passing separation** - Client schedules, server renders

**Triggering:**
- Client evaluates patterns (like TidalCycles)
- Sends `/s_new` OSC messages at event times
- Server spawns synths with no graph traversal

### Pure Data / Max/MSP (Dataflow + Block Processing)

**Architecture:**
```
message domain → signal domain
- Messages: event times, note values
- Signals: 64-sample blocks
- Clear separation!
```

**Key insights:**
1. **Signal graph is static** - Compiled once, rendered repeatedly
2. **Block processing** - All signal objects process 64 samples
3. **Message triggers** - Separate from signal flow
4. **No recursion** - Topologically sorted, process in order

### Bitwig / Ableton / Reaper (DAW Block Processing)

**Architecture:**
```
1. Dependency analysis (once per graph change)
2. Topological sort (once)
3. For each block:
   - Process tracks in dependency order
   - Each plugin renders full block (512 samples)
   - No per-sample traversal!
```

**Key insights:**
1. **Static routing** - Graph doesn't change during rendering
2. **Buffer-based** - Everything operates on blocks
3. **Parallel stages** - Independent tracks render in parallel
4. **No recursion** - Pre-sorted execution order

---

## What We Built (Phonon's Current Hybrid)

**Sample-by-Sample Mode (WORKS):**
```rust
for sample in 0..512 {
    voice_manager.process();      // Update voices
    update_voice_output_cache();   // Cache voice outputs
    for output in outputs {
        value = eval_node(output);  // Recursively evaluates, reads cache
        buffer[sample] = value;
    }
}
```

✅ **Works** - Cache is fresh for each sample

**Block Processing Mode (BROKEN):**
```rust
voice_manager.process_buffer();   // Process ALL 512 samples at once
update_voice_output_cache();       // Cache has ONE value (not 512!)

for output in outputs {
    for sample in 0..512 {
        cached_cycle_position = positions[sample];
        value = eval_node(output);  // Reads STALE cache (same value 512 times)
        buffer[sample] = value;      // All zeros!
    }
}
```

❌ **Broken** - Cache is stale, doesn't match cycle position

---

## The Fundamental Problem

**Phonon's architecture mixes two incompatible approaches:**

1. **Recursive eval_node** - Walks graph at audio rate
2. **Block processing** - Wants to process buffers

These don't work together because:
- `eval_node` expects fresh state for each sample
- Block processing pre-computes state ONCE
- Sample nodes read from `voice_output_cache` which is per-buffer, not per-sample

---

## Three Architectural Paths Forward

### Option A: Pure Block Processing (SuperCollider-style)

**Completely redesign evaluation:**

```rust
// Pre-compiled graph, no recursion
struct CompiledGraph {
    execution_order: Vec<NodeId>,
    connections: Vec<(NodeId, NodeId)>,
}

fn process_block(&mut self, block_size: usize) {
    // 1. Voice manager renders all voices to buffers
    let voice_buffers: HashMap<VoiceId, Vec<f32>> =
        self.voice_manager.process_block(block_size);

    // 2. Process nodes in topological order
    for &node_id in &self.execution_order {
        match &self.nodes[node_id] {
            SignalNode::Sample { .. } => {
                // Read from voice_buffers (not a single cached value!)
                self.buffers[node_id] = get_voice_buffer_for_node(node_id);
            }
            SignalNode::Add { a, b } => {
                // Read from input buffers
                for i in 0..block_size {
                    self.buffers[node_id][i] = self.buffers[a][i] + self.buffers[b][i];
                }
            }
            // ... all nodes operate on buffers, NO RECURSION
        }
    }
}
```

**Pros:**
- ✅ Proper block processing (like all pro audio systems)
- ✅ Can parallelize stages
- ✅ No recursive overhead

**Cons:**
- ⚠️ Major rewrite (3-5 days of work)
- ⚠️ Breaks current architecture
- ⚠️ Need to re-think pattern evaluation

**Estimated time:** 20-30 hours

### Option B: Keep Sample-by-Sample, Optimize Differently

**Accept that recursive eval is the architecture, optimize within that:**

```rust
// Keep current approach, optimize hot paths:
1. SIMD vectorization (2-4x speedup)
2. Better caching
3. Lazy evaluation
4. JIT compilation of hot paths
```

**Pros:**
- ✅ Works with current architecture
- ✅ Incremental improvements
- ✅ No major rewrites

**Cons:**
- ❌ Can't use multi-core parallelization
- ❌ Will never match pro DAW performance
- ❌ Still has recursive overhead

**Estimated speedup:** 2-4x (not enough for target)

### Option C: Hybrid - Message Passing for Patterns, Block for DSP

**Separate pattern evaluation from DSP rendering:**

```rust
// Pattern layer (per-sample):
for sample in 0..512 {
    let triggers = pattern_engine.query(time);
    for trigger in triggers {
        voice_manager.trigger(trigger);
    }
}

// DSP layer (block-based):
let voice_blocks = voice_manager.render_block(512);
let output_block = dsp_graph.process_block(voice_blocks, 512);
```

**Pros:**
- ✅ Patterns stay flexible (sample-accurate)
- ✅ DSP can be parallelized
- ✅ Simpler than full rewrite

**Cons:**
- ⚠️ Still significant refactoring (2-3 days)
- ⚠️ Two execution models to maintain

**Estimated time:** 15-20 hours

---

## Recommendation: Option C (Hybrid)

**Why:**
1. **Preserves Phonon's strength** - Pattern-per-sample flexibility
2. **Fixes the critical bottleneck** - DSP can be block-based and parallel
3. **Achievable scope** - 2-3 days vs 5+ days for full rewrite
4. **Proven approach** - Similar to how TidalCycles + SuperCollider work

**Implementation Plan:**

### Phase 1: Separate Pattern from DSP (1 day)
```rust
// Pattern evaluation stays sample-by-sample
fn process_patterns(&mut self, buffer_size: usize) -> Vec<TriggerEvent> {
    let mut triggers = Vec::new();
    for i in 0..buffer_size {
        self.cached_cycle_position = self.cycle_positions[i];
        // Evaluate Sample nodes - trigger voices, don't render
        triggers.extend(self.eval_patterns());
    }
    triggers
}

// DSP rendering becomes block-based
fn process_dsp(&mut self, buffer_size: usize) {
    // Voice manager already has all triggers from above
    let voice_buffers = self.voice_manager.render_block(buffer_size);

    // Now process signal graph with buffers
    for stage in self.execution_stages {
        for node in stage {
            self.render_node_to_buffer_pure(node, buffer_size);
        }
    }
}
```

### Phase 2: Make Voice Manager Block-Based (1 day)
```rust
impl VoiceManager {
    fn render_block(&mut self, block_size: usize) -> HashMap<NodeId, Vec<f32>> {
        // Instead of returning one sample per node,
        // return entire buffer per node
        let mut buffers = HashMap::new();

        for &node_id in &self.active_nodes {
            let mut buffer = vec![0.0; block_size];
            for voice in &self.voices {
                if voice.source_node == node_id {
                    // Render this voice for entire block
                    for i in 0..block_size {
                        buffer[i] += voice.render_sample();
                    }
                }
            }
            buffers.insert(node_id, buffer);
        }

        buffers
    }
}
```

### Phase 3: Parallelize DSP Stages (0.5 days)
```rust
use rayon::prelude::*;

for stage in self.execution_stages {
    stage.par_iter().for_each(|&node_id| {
        self.render_node_to_buffer_pure(node_id, buffer_size);
    });
}
```

**Expected result:** 64ms → 4-8ms ✅ **UNDER BUDGET!**

---

## Immediate Action Items

1. ❌ **Abandon current block processing** - It's architecturally flawed
2. ✅ **Keep sample-by-sample for now** - It works correctly
3. 📋 **Plan hybrid architecture** - Separate patterns from DSP
4. 🚀 **Implement in next 2-3 sessions** - ~20 hours total work

---

## Key Learnings

1. **"Block processing" isn't just optimization** - It's a fundamental architecture change
2. **You can't bolt parallelization onto recursive eval** - The architecture must support it from the start
3. **Pro audio systems separate concerns** - Patterns/scheduling vs DSP rendering
4. **Phonon's recursive eval is both strength and weakness:**
   - ✅ Strength: Flexible, composable, easy to understand
   - ❌ Weakness: Can't parallelize, has overhead
5. **The right answer:** Keep pattern flexibility, make DSP block-based

---

## Why Current "Block Processing" Failed

**We tried to have our cake and eat it too:**
- Keep recursive eval_node (for flexibility)
- Process in blocks (for performance)
- These are fundamentally incompatible!

**The voice_output_cache issue is just a symptom:**
- Cache has one value per node (per buffer)
- But we're calling eval_node 512 times with different cycle positions
- Each call expects cache to match its cycle position
- It doesn't - so we get zeros

**No amount of Arc/DashMap/rayon fixes this architectural mismatch.**

---

## Next Steps

**Immediate (This Session):**
1. Revert block processing code (it doesn't work)
2. Document why it failed (this file)
3. Commit the learnings

**Next Session (Hybrid Architecture):**
1. Design pattern/DSP separation
2. Make voice_manager block-based
3. Implement buffer-based DSP evaluation
4. Test correctness
5. Add parallelization
6. Measure results

**Timeline:** 2-3 focused sessions (~20 hours) to proper architecture that hits performance target.

---

*"The right way is often harder than the wrong way. But it's still the only way."*

We learned what doesn't work. Now we know what must work. 🚀
