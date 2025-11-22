# Dynamic Everything - Full Vision Implementation Plan

**Goal**: Enable full bidirectional flow between audio and patterns with feedback loops

**Vision**: Audio modulates patterns, patterns modulate audio, feedback everywhere

---

## What This Enables

```phonon
-- Audio modulating pattern speed
~kick: s "bd"
~envelope: ~kick # envelope_follower
~speed: ~envelope * 4 + 1
~hats: s "hh*8" $ fast ~speed  -- Kick envelope controls hat speed!

-- Feedback loops with delay
~input: sine 440
~delayed: ~feedback # delay 0.5 0.7  -- Delay breaks cycle
~feedback: ~delayed * 0.5
out: ~delayed

-- Audio modulating pattern selection
~rms: ~drums # rms
~pattern_selector: ~rms * 10  -- Audio level selects pattern
~bass: s $ choose ~pattern_selector ["bd", "sn", "hh"]

-- Complex feedback networks
~source: brown_noise 0.3
~comb1: ~source + ~feedback2 # comb_filter 0.009 0.7
~comb2: ~comb1 + ~feedback1 # comb_filter 0.013 0.6
~feedback1: ~comb1 * 0.3 # delay 0.001
~feedback2: ~comb2 * 0.4 # delay 0.001
out: ~comb2
```

---

## Architecture Changes Needed

### 1. Remove Cycle Blocking ✅

**Current**: `BlockProcessor::new()` rejects cycles
**New**: Allow cycles if delay nodes are present

**File**: `src/block_processor.rs`
**Change**:
```rust
// OLD:
if !dependency_graph.is_acyclic() {
    return Err("Dependency graph has cycles".to_string());
}

// NEW:
// Cycles are OK if they contain delay nodes (checked separately)
// Delay nodes provide the buffer that breaks instant feedback
```

### 2. Delay-Based Cycle Breaking ✅

**Concept**: Cycles are valid if they contain at least one delay node

**Implementation**:
- DelayNode, TapeDelayNode, CombFilterNode, etc. all have internal buffers
- These provide the "previous block" values needed for feedback
- No instant (zero-delay) loops allowed

**Validation**:
```rust
// Check each cycle for delay nodes
fn validate_cycles(graph: &DependencyGraph, nodes: &[Box<dyn AudioNode>]) -> Result<(), String> {
    for cycle in detect_cycles(graph) {
        if !cycle_contains_delay(cycle, nodes) {
            return Err(format!("Cycle has no delay: {:?}", cycle));
        }
    }
    Ok(())
}
```

### 3. Execution Order with Cycles ✅

**Current**: Topological sort (requires acyclic)
**New**: Schedule with feedback awareness

**Options**:

**Option A: Partial topological order + feedback initialization**
```rust
// 1. Process as much as possible in topological order
// 2. For cyclic nodes, use previous block's output
// 3. Initialize feedback buffers to zero on first block
```

**Option B: Two-pass processing**
```rust
// Pass 1: Process all nodes using previous cycle values
// Pass 2: Update feedback buffers for next cycle
```

**Option C: Relaxation iteration** (more complex, maybe later)
```rust
// Iterate until convergence within block
// May not be needed if delay buffers work
```

**DECISION**: Start with Option A (simplest, matches hardware DSP)

### 4. Audio → Pattern Modulation ✅

**Vision**: Audio signals control pattern parameters

**Implementation**:
```rust
// PatternNode that takes audio input for speed/density/etc.
pub struct DynamicPatternNode {
    pattern: Arc<Pattern<String>>,
    speed_input: Option<NodeId>,      // Audio controls speed
    density_input: Option<NodeId>,    // Audio controls density
    offset_input: Option<NodeId>,     // Audio controls offset
    // ...
}
```

**Example usage**:
```phonon
~envelope: ~kick # envelope_follower  -- Audio signal
~speed: ~envelope * 4 + 1             -- Map to speed range
~pattern: s "hh*8" $ fast ~speed      -- Dynamic speed!
```

**Key insight**: Pattern transforms like `fast`, `slow`, `density` need to accept NodeIds for their parameters, not just constants.

### 5. Dynamic Bus Re-evaluation 🔄

**Current**: Buses store NodeId (compiled once)
**New**: Buses can be dynamic

**Options**:

**Option A: Store Expr + NodeId** (hybrid)
```rust
struct BusData {
    expr: Expr,           // Original expression
    node_id: NodeId,      // Compiled node
    dependencies: Vec<String>,  // Bus dependencies
}
```

**Option B: Lazy compilation**
```rust
// Compile buses only when referenced
// Re-compile if dependencies change
```

**DECISION**: Option A for now (simpler, enables later optimizations)

---

## Implementation Phases

### Phase 1: Feedback Loops (Core Infrastructure) 🎯 START HERE

**Goal**: Enable cycles with delay buffers

**Tasks**:
1. ✅ Modify cycle detection to allow cycles with delays
2. ✅ Implement feedback-aware execution order
3. ✅ Add cycle validation (ensure delay present)
4. ✅ Test with simple feedback example
5. ✅ Test with complex feedback networks

**Files to modify**:
- `src/dependency_graph.rs` - Cycle detection
- `src/block_processor.rs` - Execution order
- `src/audio_node_graph.rs` - Validation
- `tests/test_feedback_loops.rs` - NEW

**Test case**:
```phonon
~input: sine 440
~delayed: ~feedback # delay 0.5 0.7
~feedback: ~delayed * 0.5
out: ~delayed
```

**Expected**: Self-oscillating delay loop

### Phase 2: Audio → Pattern Modulation

**Goal**: Audio signals control pattern parameters

**Tasks**:
1. ✅ Add dynamic parameter support to PatternNode
2. ✅ Implement speed_input, density_input, etc.
3. ✅ Add envelope_follower, rms, peak_detector nodes
4. ✅ Add pattern selection based on audio
5. ✅ Test with kick modulating hat speed

**Files to modify**:
- `src/nodes/sample_pattern.rs` - Dynamic parameters
- `src/nodes/envelope_follower.rs` - Already exists!
- `src/nodes/rms.rs` - Already exists!
- `src/compositional_compiler.rs` - Compile dynamic patterns
- `tests/test_audio_modulates_pattern.rs` - NEW

**Test case**:
```phonon
~kick: s "bd"
~envelope: ~kick # envelope_follower
~speed: ~envelope * 4 + 1
~hats: s "hh*8" $ fast ~speed
out: ~kick + ~hats
```

**Expected**: Hat speed varies with kick envelope

### Phase 3: Dynamic Pattern Transforms

**Goal**: All pattern transforms accept audio inputs

**Tasks**:
1. ✅ Modify `fast` to accept NodeId
2. ✅ Modify `slow` to accept NodeId
3. ✅ Modify `density` to accept NodeId
4. ✅ Modify `every` to accept NodeId
5. ✅ Test all transforms with audio modulation

**Files to modify**:
- `src/compositional_compiler.rs` - Transform compilation
- `src/pattern.rs` - Add dynamic transform support
- `tests/test_dynamic_transforms.rs` - NEW

### Phase 4: Dynamic Bus System

**Goal**: Buses can change and re-evaluate

**Tasks**:
1. ✅ Store Expr alongside NodeId in buses
2. ✅ Track bus dependencies
3. ✅ Implement re-compilation trigger
4. ✅ Add bus change detection
5. ✅ Test dynamic bus updates

**Files to modify**:
- `src/compositional_compiler.rs` - Bus storage
- `src/audio_node_graph.rs` - Dynamic updates
- `tests/test_dynamic_buses.rs` - NEW

### Phase 5: Complex Feedback Networks

**Goal**: Multi-path feedback, audio rate modulation

**Tasks**:
1. ✅ Test Karplus-Strong with feedback
2. ✅ Test Dattorro reverb with cross-feedback
3. ✅ Test audio-rate FM via feedback
4. ✅ Performance optimization
5. ✅ Documentation and examples

---

## Success Criteria

**Phase 1 Complete When**:
- ✅ Simple feedback loop works (delay + feedback)
- ✅ Complex feedback network works (multiple paths)
- ✅ Cycle detection properly validates delay presence
- ✅ No crashes, no infinite loops
- ✅ Tests passing

**Phase 2 Complete When**:
- ✅ Audio envelope modulates pattern speed
- ✅ Audio RMS modulates pattern density
- ✅ Audio peak modulates pattern offset
- ✅ Real-time responsive (< 10ms latency)
- ✅ Tests passing

**Full Vision Complete When**:
- ✅ All patterns can be modulated by audio
- ✅ All audio can modulate patterns
- ✅ Feedback loops work everywhere
- ✅ Dynamic buses enable live pattern changes
- ✅ Production-ready performance
- ✅ Comprehensive test coverage
- ✅ Documentation and examples

---

## Timeline

**Phase 1**: 2-3 hours (core infrastructure)
**Phase 2**: 2-3 hours (audio → pattern)
**Phase 3**: 1-2 hours (dynamic transforms)
**Phase 4**: 2-3 hours (dynamic buses)
**Phase 5**: 1-2 hours (polish and examples)

**Total**: 8-13 hours to full dynamic system

**START NOW**: Phase 1 - Feedback Loops

---

## Notes

This is a MAJOR architectural enhancement but aligns perfectly with Phonon's vision.

**Key insight**: Hardware DSP has always worked this way (delay lines, feedback paths, control rate modulation). We're bringing that power to pattern-based live coding.

**What makes this unique**: Unlike Tidal/Strudel (discrete events only), Phonon patterns become true control signals that can be modulated by audio in real-time.

**Performance**: Should be fine - we're just removing artificial cycle blocking. The delay buffers already exist in effect nodes.

Let's make Phonon the most expressive live coding system ever! 🚀
