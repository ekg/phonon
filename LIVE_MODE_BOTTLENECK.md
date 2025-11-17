# Live Mode Underrun Issue - Root Cause & Solutions

## The Problem

**Symptom**: m.ph triggers underruns almost instantly in live mode
**Root Cause**: SignalNode.clone() bottleneck at line 4797 in unified_graph.rs
**Impact**: Single-threaded audio callback can't keep up (needs 11.61ms, takes 13.43ms)

## Why Parallel Processing Didn't Help Live Mode

The parallel optimization (5.4x speedup) **only works for render mode**:
- ✅ Render mode: Processes 512-sample blocks in parallel across 16 cores
- ❌ Live mode: Still single-threaded audio callback with clone bottleneck

## The Bottleneck (Line 4797)

```rust
fn eval_node(&mut self, node_id: &NodeId) -> f32 {
    // ...
    let mut node = if let Some(Some(node)) = self.nodes.get(node_id.0) {
        node.clone()  // ← BOTTLENECK: Deep copy of massive enum
    } else {
        return 0.0;
    };
    // ...
}
```

**Why it's slow**:
- Called thousands of times per buffer (once per node evaluation)
- SignalNode is ~50 enum variants with nested Signals, RefCells, state
- Deep copy allocates ~500 bytes per clone
- At 44.1kHz with complex patterns: **~25 MB/sec of allocations**

## Solutions (Ordered by Effort)

### Solution 1: Use Rc<SignalNode> (Medium effort, 10x speedup)

**Change**: `Vec<Option<SignalNode>>` → `Vec<Option<Rc<SignalNode>>>`

**Impact**:
- Clone becomes cheap (increment ref count, ~5ns vs ~500ns)
- **Estimated speedup**: 10x for eval_node hot path
- Should bring live mode under realtime budget

**Effort**:
- 1-2 days
- Update ~50-100 locations where nodes are created/accessed
- Minimal logic changes (mostly wrapping in Rc::new())

**Files to modify**:
- `src/unified_graph.rs` - Change storage type, wrap adds in Rc
- `src/compositional_compiler.rs` - Wrap new nodes in Rc
- `src/main.rs` - Update any direct node access
- All tests - Update node creation

**Implementation**:
```rust
// Before:
nodes: Vec<Option<SignalNode>>,

// After:
nodes: Vec<Option<Rc<SignalNode>>>,

// Node creation:
self.nodes.push(Some(Rc::new(SignalNode::Oscillator { ... })));

// Access (clone is now cheap):
let node = Rc::clone(&self.nodes[id].as_ref().unwrap());
```

### Solution 2: Separate State from Node Definitions (Hard, 50x speedup)

**Change**: Split SignalNode into immutable definition + mutable state

**Impact**:
- No cloning needed at all
- Better cache locality (state in flat arrays)
- **Estimated speedup**: 50x for eval_node

**Effort**:
- 2-4 weeks
- Major architectural refactor
- Rewrite eval_node and all node types

**Example**:
```rust
// Immutable definition (can share via reference)
enum NodeDef {
    Oscillator { freq: Signal, waveform: Waveform },
    // No RefCell, no state
}

// Mutable state (flat, indexed by NodeId)
struct GraphState {
    oscillator_phases: Vec<f32>,      // Indexed by node ID
    oscillator_pending: Vec<Option<f32>>,
    // All state in parallel arrays
}
```

### Solution 3: Increase Buffer Size (Quick workaround, partial fix)

**Change**: Use 1024 or 2048 sample buffers instead of 512

**Impact**:
- Gives more time per callback (11.61ms → 23.22ms or 46.44ms)
- Increases latency (user feels lag)
- **Doesn't fix root cause** but buys time

**Effort**: 5 minutes, change one constant

**Trade-off**: Latency vs stability

### Solution 4: Use Two-Process Architecture (Medium, different approach)

**Status**: Already implemented but not connected to edit mode

**Approach**:
- Audio process runs phonon-audio (dedicated audio thread)
- Pattern process sends updates via IPC (/tmp/phonon.sock)
- Audio process can use simpler graph or precomputed buffers

**Effort**:
- 1-2 days to connect edit mode to phonon-audio
- Requires IPC protocol for live updates

## Immediate Recommendations

### For Testing (Now)

**Simplify m.ph** to verify approach:
```phonon
# Instead of:
o2: jux rev $ stut 8 0.125 0.1 $ s "rave(3,8,1)" # ar 0.1 0.5

# Try:
out: s "bd sn hh cp"
```

The `stut 8` creates 8 delayed copies - this explodes the graph size.

### For Production (Next)

**Option A - Quick Fix (Recommended)**:
1. Increase buffer size to 1024 temporarily
2. Implement Rc<SignalNode> (1-2 days)
3. Test live mode performance
4. Revert buffer size if fixed

**Option B - Proper Fix**:
1. Implement Rc<SignalNode> first (gets 10x improvement)
2. Plan state separation refactor for v2.0
3. Target 50x improvement long-term

## Performance Targets

| Metric | Current | With Rc | With State Sep | Target |
|--------|---------|---------|----------------|---------|
| eval_node time | ~500ns | ~50ns | ~10ns | <20ns |
| Buffer process | 13.43ms | ~1.3ms | ~0.3ms | <11.61ms |
| CPU usage | 115% | ~12% | ~3% | <50% |
| Realtime capable | ❌ | ✅ | ✅✅ | ✅ |

## Test Case

```bash
# Current (fails):
cargo run --release --bin phonon -- edit m.ph

# With Rc (should work):
# After implementing Rc<SignalNode>

# Metric to track:
# Should have ZERO underruns with m.ph pattern
```

## Notes

- The parallel render optimization is great for offline work
- Live mode needs architectural fix (Rc or state separation)
- m.ph is a complex pattern (Euclidean + stut 8 + jux + multi-output)
- Simpler patterns might work even with current architecture
