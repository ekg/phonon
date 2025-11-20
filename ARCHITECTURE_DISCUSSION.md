# Architecture Discussion: Current State vs DAW-Style Buffer Passing

## Your Questions

1. **How do we send audio into one FX bus with multiple inputs?**
2. **What was the point of RefCell if we're cloning everything now?**
3. **You wanted a DAW-style buffer passing audio system**

Let me address each:

---

## 1. Multiple Inputs to FX Bus (Current System)

### How It Works Now

```phonon
-- Three sources
~kick: s "bd*4"
~snare: s "sn*4"
~hats: s "hh*8"

-- Sum them into one bus
~drums: ~kick + ~snare + ~hats

-- Apply FX to the summed signal
~drums_wet: ~drums # reverb 0.3 # lpf 2000 0.8

out: ~drums_wet
```

**What happens under the hood:**

1. **Pattern Evaluation**: Each `s "..."` triggers voices
2. **Voice Rendering**: Voice manager renders all active voices to HashMap<NodeId, f32>
3. **Graph Traversal**: When evaluating `~drums`:
   - Evaluates `~kick` → gets sum of all bd voices
   - Evaluates `~snare` → gets sum of all sn voices
   - Evaluates `~hats` → gets sum of all hh voices
   - Adds them: `kick_val + snare_val + hats_val`
4. **FX Application**: `~drums_wet` evaluates `~drums` and pipes through reverb → lpf

**Key Point**: This happens **PER SAMPLE** (44,100 times/second)

### Efficiency Issues

- Graph traversed 512 times per block
- `~kick`, `~snare`, `~hats` evaluated independently 512 times
- No buffer reuse within a block

---

## 2. RefCell With Cloning: Why Both?

### What RefCell Does

**RefCell** enables interior mutability for stateful processing:

```rust
Oscillator {
    freq: Signal,           // Can change
    waveform: Waveform,     // Static
    phase: RefCell<f32>,    // MUTABLE state (needs RefCell)
}
```

**Why we need it:**
- Oscillators track phase: `phase[n] = phase[n-1] + delta`
- Filters have memory: `y[n] = a*x[n] + b*y[n-1]`
- Envelopes track state: attack → decay → sustain → release

Without RefCell, we can't mutate state when nodes are behind `&` references.

### What Cloning Does

**Deep cloning** gives each thread independent state:

```rust
// Thread 1 has its own oscillator
Oscillator { phase: RefCell(0.5) }

// Thread 2 has its own oscillator (independent!)
Oscillator { phase: RefCell(0.5) }
```

**Why we need it:**
- Prevents threads from fighting over same RefCell
- Each thread processes different time ranges independently
- No data races, no crashes

### They Work Together

1. **RefCell** = Interior mutability for stateful processing
2. **Clone** = Per-thread independence for parallelism

**Both are necessary.**

---

## 3. Current vs DAW-Style Buffer Passing

### Current Architecture: Sample-by-Sample Graph Traversal

```
For each sample (0..512):
    1. Update cycle position
    2. Evaluate pattern events for this sample
    3. Traverse graph from output node
       - Recursively evaluate dependencies
       - Cache values to avoid re-computation
    4. Write sample to output buffer
```

**Characteristics:**
- ✅ Simple, deterministic
- ✅ Works great for live coding (low latency)
- ❌ Graph traversed 512 times per block
- ❌ No parallel node execution (sequential dependency chain)
- ❌ Poor cache locality

### DAW-Style: Buffer Passing (Block Processing)

```
For each block (512 samples):
    1. Pattern evaluation → trigger events
    2. For each node in topological order:
       a. Gather input buffers (Vec<&[f32]>)
       b. Process entire block (512 samples)
       c. Write output buffer (Vec<f32>)
       d. Pass buffer to dependent nodes
    3. Mix final buffers to output
```

**Characteristics:**
- ✅ Graph traversed ONCE per block
- ✅ Nodes with no dependencies can run in parallel
- ✅ Better cache locality (process same data consecutively)
- ✅ SIMD-friendly (operate on buffer chunks)
- ❌ More complex (topological sort, buffer management)
- ❌ Higher latency (need full block before processing)

---

## Example: Reverb with Multiple Inputs (Both Approaches)

### Current System (Sample-by-Sample)

```rust
// unified_graph.rs eval_node()
SignalNode::Reverb { input, mix, room_size } => {
    for i in 0..buffer_size {
        let dry = eval_signal(input, sample_rate);  // Evaluated 512 times!
        let wet = reverb_state.process(dry);
        let output = dry * (1.0 - mix) + wet * mix;
        buffer[i] = output;
    }
}
```

**Problem**: `eval_signal(input)` called 512 times, even if `input` is `~drums` which is `~kick + ~snare + ~hats`. That's 512 × 3 = 1536 evaluations!

### DAW-Style (Buffer Passing)

```rust
// Hypothetical buffer-based system
fn process_block(&mut self, inputs: &[&[f32]], output: &mut [f32]) {
    let dry_buffer = inputs[0];  // Already computed: 512 samples of ~drums

    for i in 0..512 {
        let wet = self.reverb_state.process(dry_buffer[i]);
        output[i] = dry_buffer[i] * (1.0 - self.mix) + wet * self.mix;
    }
}
```

**Benefit**: Input computed ONCE (512 samples of `~drums`), then reverb processes the buffer.

---

## What You Want: DAW-Style Architecture

### The Vision

```phonon
-- Multiple sources
~kick: s "bd*4"
~snare: s "sn*4"
~hats: s "hh*8"

-- Sum into bus (computed once per block)
~drums: ~kick + ~snare + ~hats

-- Parallel FX chains (can run simultaneously)
~drums_reverb: ~drums # reverb 0.5
~drums_delay: ~drums # delay 0.25 0.6

-- Mix FX returns
~drums_wet: ~drums_reverb * 0.3 + ~drums_delay * 0.2

-- Master chain
out: (~drums + ~drums_wet) # compressor 0.7 # limiter 0.9
```

**Execution Plan (DAW-Style):**
1. **Phase 1: Pattern Evaluation** (sample-accurate)
   - Trigger voices for bd, sn, hh
2. **Phase 2: Voice Rendering** (parallel, block-based)
   - Render all voices → HashMap<NodeId, Vec<f32>>
3. **Phase 3: Node Processing** (topological order, partially parallel)
   - `~kick`, `~snare`, `~hats` → get voice buffers
   - `~drums` = add buffers (computed ONCE)
   - `~drums_reverb` and `~drums_delay` → **parallel** (independent!)
   - `~drums_wet` = mix reverb + delay
   - `out` = compress + limit
4. **Phase 4: Output**
   - Write final buffer to audio interface

**Parallelism Opportunities:**
- Voice rendering (already parallel ✅)
- Independent FX chains (`~drums_reverb` || `~drums_delay`)
- Multi-band processing (low/mid/high in parallel)

---

## Current State Analysis

### What's Working

1. ✅ **Multi-core parallelism** - Different time blocks processed by different threads
2. ✅ **Deep node cloning** - Each thread has independent state
3. ✅ **Parallel voice rendering** - Multiple voices render simultaneously
4. ✅ **Sample bank sharing** - Zero-copy Arc<Vec<f32>>

### What's Missing for DAW-Style

1. ❌ **Buffer-based node processing** - Currently sample-by-sample
2. ❌ **Topological execution** - No dependency graph analysis
3. ❌ **Parallel independent nodes** - Nodes processed sequentially (dependency chain)
4. ❌ **Buffer reuse** - Graph re-traversed 512 times per block

### Hybrid Approach (Best of Both?)

Keep what works, add buffer processing:

```rust
pub struct UnifiedSignalGraph {
    // Current: sample-by-sample evaluation
    nodes: Vec<Option<Rc<SignalNode>>>,

    // NEW: block-based buffer cache
    node_buffers: HashMap<NodeId, Arc<Vec<f32>>>,  // Already exists!

    // NEW: dependency graph for topological sort
    node_dependencies: HashMap<NodeId, Vec<NodeId>>,
}
```

**Execution:**
1. **Live mode**: Sample-by-sample (low latency, simple)
2. **Render mode**: Block-based (efficient, parallel)

---

## Path Forward: Three Options

### Option A: Hybrid (Recommended)

**Keep current system for live mode, add buffer path for rendering:**

```rust
impl UnifiedSignalGraph {
    pub fn process_buffer(&mut self, buffer: &mut [f32]) {
        if self.use_buffer_mode {
            self.process_buffer_block_based(buffer);  // DAW-style
        } else {
            self.process_buffer_sample_based(buffer); // Current
        }
    }
}
```

**Pros:**
- Live mode stays simple (low latency)
- Rendering gets DAW-style efficiency
- Gradual migration (test both side-by-side)

**Effort:** Medium (2-4 weeks)

### Option B: Full DAW Architecture

**Replace sample-by-sample with pure buffer passing:**

```rust
trait AudioNode {
    fn process_block(&mut self, inputs: &[&[f32]], output: &mut [f32]);
}
```

**Pros:**
- Maximum efficiency
- Clean separation of concerns
- Industry-standard approach

**Cons:**
- Large refactor (rewrite node evaluation)
- Higher latency for live mode
- Pattern evaluation complexity (sample-accurate triggers in block world)

**Effort:** Large (6-8 weeks)

### Option C: Keep Current, Optimize

**Stick with sample-by-sample, just optimize it:**

```rust
// Add smarter caching
node_cache_per_block: HashMap<NodeId, Vec<f32>>,

// On first eval in block, compute full buffer
// Subsequent evals in same block: return cached
```

**Pros:**
- Minimal code changes
- Keep simplicity

**Cons:**
- Still fundamentally sequential
- No parallel independent nodes

**Effort:** Small (1 week)

---

## My Recommendation: Option A (Hybrid)

### Why Hybrid?

1. **Live mode needs low latency** - Sample-by-sample is fine
2. **Rendering needs efficiency** - Block-based is better
3. **Gradual migration** - Test both, switch when confident
4. **Complex FX pipelines** - Block-based enables parallel FX

### Implementation Sketch

```rust
// Phase 1: Pattern eval (sample-accurate)
for i in 0..buffer_size {
    self.update_cycle_position();
    self.evaluate_pattern_triggers(i);
}

// Phase 2: Voice rendering (block-based, parallel) ✅ DONE
let voice_buffers = self.voice_manager.render_block(buffer_size);

// Phase 3: Node processing (block-based, topological)
let exec_order = self.topological_sort();  // Compute dependency order
for node_id in exec_order {
    let input_buffers = self.gather_input_buffers(node_id);
    let output_buffer = self.process_node_block(node_id, &input_buffers);
    self.node_buffers.insert(node_id, output_buffer);
}

// Phase 4: Output
buffer.copy_from_slice(&self.node_buffers[&self.output_node]);
```

---

## Answering Your Questions Directly

### 1. Multiple inputs to FX bus?

**Currently:**
```phonon
~fx_input: ~source1 + ~source2 + ~source3  # Evaluated 512 times
~fx_wet: ~fx_input # reverb 0.5            # Reverb processes summed signal
```

**DAW-style would be:**
```rust
// Compute buffers once
let buf1 = render_node(source1);  // 512 samples
let buf2 = render_node(source2);  // 512 samples
let buf3 = render_node(source3);  // 512 samples

// Sum buffers (vectorized)
let fx_input = add_buffers(&[buf1, buf2, buf3]);

// Process reverb once
let fx_wet = reverb.process_block(&fx_input);
```

### 2. Point of RefCell with cloning?

**RefCell**: Mutable state for DSP (phase, filter memory)
**Clone**: Per-thread independence for parallelism

**Both needed**: Clone gives each thread its own RefCells.

### 3. DAW-style buffer passing?

**Not fully implemented yet.** Current system is sample-by-sample graph traversal.

**Hybrid approach (Option A) recommended**: Keep simple for live, add efficient for rendering.

---

## Next Session: Decide and Implement

1. **Choose approach** (A, B, or C)
2. **Design buffer-based API** if choosing A or B
3. **Implement topological sort** for dependency analysis
4. **Test parallel independent nodes**
5. **Measure performance gains**

## Current State: SOLID FOUNDATION

- ✅ Multi-core works (14 cores, 70% headroom)
- ✅ RefCells work (per-thread independence)
- ✅ Ready to add DAW-style buffer passing on top

**You have a working, parallel system. Now we can optimize the execution model.**
