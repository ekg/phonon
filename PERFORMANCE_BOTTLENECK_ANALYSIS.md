# Performance Bottleneck Analysis - 2025-11-17

## Problem: 115% CPU Usage (Cannot Run in Realtime)

**Symptoms**:
- User reported "dozens to hundreds of underruns per cycle"
- Profiling shows 115.7% CPU usage
- Each 512-sample block takes 13.43ms (budget: 11.61ms)
- Worst-case block time: 22.26ms (nearly 2x budget!)

## Root Cause Identified

**Location**: `src/unified_graph.rs:4795-4803` in `eval_node()` function

**The Bottleneck**:
```rust
let mut node = if let Some(Some(node)) = self.nodes.get(node_id.0) {
    node.clone()  // ← CATASTROPHIC PERFORMANCE ISSUE
} else {
    return 0.0;
};
```

**Why This Is Catastrophic**:
1. `eval_node()` is called for EVERY node evaluation
2. At 44.1kHz with a typical graph: **~50,000 clones/second**
3. `SignalNode` is a massive enum (50+ variants):
   - Each variant contains multiple `Signal` enums (which can be recursive)
   - `RefCell` wrappers for state management
   - Nested structs (filters, envelopes, effects state)
   - Total size can be hundreds of bytes per clone

**Impact Calculation**:
- If SignalNode averages 500 bytes (conservative estimate)
- 50,000 clones/sec × 500 bytes = **25 MB/sec of memory allocation**
- Plus the CPU cost of deep copying nested structures
- Plus cache pressure from all this memory traffic

## Why The Clone Exists

The borrow checker prevents this safer code:

```rust
// ❌ DOESN'T COMPILE: Can't hold reference while calling eval_signal()
let node_ref = &self.nodes[node_id.0];
match node_ref {
    SignalNode::Oscillator { freq, .. } => {
        self.eval_signal(freq)  // ← Recursive call borrows self again!
    }
}
```

The `eval_signal()` calls can recurse into `eval_node()`, which needs mutable access to `self.nodes` to update state. The borrow checker sees this as a potential conflict.

## Proposed Solutions

### Solution 1: Use `Rc<SignalNode>` (Simple, Partial Fix)

**Change**:
```rust
// Before:
nodes: Vec<Option<SignalNode>>,

// After:
nodes: Vec<Option<Rc<SignalNode>>>,
```

**Impact**:
- Clone becomes cheap (just increment refcount)
- **Estimated speedup**: 5-10x for eval_node()
- **Downside**: Still cloning, just cheaper. Doesn't fix architectural issue.

**Effort**: Medium - requires changes throughout codebase where nodes are created

### Solution 2: Separate State from Node Definition (Proper Fix)

**Concept**:
```rust
// Node definition (immutable, can be shared)
enum SignalNodeDef {
    Oscillator { freq: Signal, waveform: Waveform },
    // ... no RefCell state here
}

// State storage (mutable, separate from nodes)
struct NodeState {
    oscillator_phase: f32,
    oscillator_pending_freq: Option<f32>,
    // ... all state in flat arrays indexed by NodeId
}

pub struct UnifiedSignalGraph {
    node_defs: Vec<Option<SignalNodeDef>>,  // Immutable definitions
    node_state: NodeState,                   // All mutable state separate
    //...
}
```

**Benefits**:
- No cloning needed - definitions are immutable
- Better cache locality (state in flat arrays)
- Easier to parallelize (state can be per-voice)
- **Estimated speedup**: 10-50x for eval_node()

**Downside**: Major refactoring (1-2 weeks of work)

### Solution 3: Arena Allocator (Medium Fix)

Use an arena/bump allocator for node clones:

```rust
use bumpalo::Bump;

pub struct UnifiedSignalGraph {
    nodes: Vec<Option<SignalNode>>,
    eval_arena: Bump,  // Reset after each buffer
    //...
}

fn eval_node(&mut self, node_id: &NodeId) -> f32 {
    // Clone into arena (fast allocation, bulk deallocation)
    let node = self.eval_arena.alloc(self.nodes[node_id.0].clone());
    //...
}
```

**Benefits**:
- Faster allocation/deallocation
- Less memory fragmentation
- **Estimated speedup**: 2-3x for eval_node()

**Downside**: Still doing the deep copy, just faster

### Solution 4: Inline Critical Paths (Quick Win)

Add `#[inline]` to hot functions:

```rust
#[inline(always)]
fn eval_node(&mut self, node_id: &NodeId) -> f32 { ... }

#[inline(always)]
fn eval_signal(&mut self, signal: &Signal) -> f32 { ... }
```

**Benefits**:
- Compiler can optimize better
- Reduces function call overhead
- **Estimated speedup**: 1.2-1.5x

**Downside**: Minimal impact on the clone issue itself

## Recommended Approach

**Phase 1 (Quick Win - 1 hour)**:
1. Add `#[inline]` annotations
2. Test performance improvement

**Phase 2 (Medium Fix - 1 day)**:
1. Implement `Rc<SignalNode>`
2. Verify functionality
3. Should get us below 100% CPU

**Phase 3 (Proper Fix - 1-2 weeks)**:
1. Separate state from node definitions
2. Refactor eval_node to use immutable node defs
3. Target: <50% CPU for current patterns

## Testing Methodology

Use the new `--realtime` profiling flag:

```bash
cargo run --release --bin phonon -- render m.ph /tmp/test.wav --cycles 4 --realtime
```

**Success Criteria**:
- Avg block time < 11.61ms (100% CPU)
- Max block time < 20ms (worst-case under budget)
- Zero underruns in live mode

## References

- Initial analysis: `PERFORMANCE_OPTIMIZATIONS.md`
- Live mode architecture: `UNDERRUN_DETECTION_PROGRESS.md`
- SuperCollider approach: Never malloc in audio thread, use pre-allocated pools
