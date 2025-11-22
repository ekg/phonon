# Bus Variables, Patterns, and Feedback Loops - Technical Analysis

**Date**: 2025-11-22
**Question**: Can bus variables hold patterns and create feedback loops for complex processing chains?

---

## Executive Summary

✅ **Complex processing chains**: FULLY SUPPORTED
❌ **Buses holding dynamic patterns**: NOT CURRENTLY SUPPORTED
❌ **Graph-level feedback loops**: BLOCKED BY DESIGN
✅ **Internal node feedback**: FULLY SUPPORTED

---

## How Buses Currently Work

### Bus Compilation Process

When you write:
```phonon
~bass: saw 220 # lpf 1000 0.8
```

**What happens** (`src/compositional_compiler.rs:353-368`):
1. Expression is compiled to AudioNodes immediately
2. A NodeId (just a number) is stored in the buses HashMap
3. `ctx.buses.insert("bass", NodeId(5))`  // Example

**Bus storage type**:
```rust
buses: HashMap<String, NodeId>
```

### Bus Reference Resolution

When you reference a bus:
```phonon
out: ~bass * 0.5
```

**What happens** (`src/compositional_compiler.rs:1381-1387`):
```rust
Expr::BusRef(name) => {
    ctx.buses.get(&name)
        .cloned()
        .ok_or_else(|| format!("Undefined bus: ~{}", name))
}
```

It just returns the NodeId. There's no re-evaluation, no pattern re-query, no dynamic changes.

---

## Can Buses Hold Patterns?

### Current Behavior: ❌ NO (Patterns are compiled once)

**Example that SEEMS like it should work but is STATIC**:
```phonon
~lfo: sine 0.25             # Creates OscillatorNode, gets NodeId 0
~cutoff: ~lfo * 2000 + 500  # Creates MultiplicationNode + AdditionNode, gets NodeId 5
~bass: saw 110 # lpf ~cutoff 0.8  # Uses NodeId 5 for cutoff

out: ~bass
```

**This works perfectly!** But it's not "holding a pattern" - it's holding a reference to an already-compiled node graph.

**What you CANNOT do**:
```phonon
~pattern: "bd sn hh cp"     # ❌ This doesn't work the way you might expect
~drums: s ~pattern          # ❌ Pattern not re-evaluated dynamically
```

**Why**:
1. Buses store NodeId, not Expr
2. The expression is compiled once when assigned
3. No mechanism to re-compile or re-evaluate

### What Would Be Needed

To make buses hold dynamic patterns that can be changed:

1. **Store expressions, not NodeIds**:
```rust
buses: HashMap<String, Expr>  // Instead of NodeId
```

2. **Lazy compilation**: Compile bus expressions when referenced, not when assigned

3. **Re-compilation on changes**: Detect when bus expression changes and re-compile dependent nodes

**This would be a MAJOR architectural change.**

---

## Can Buses Create Feedback Loops?

### Current Behavior: ❌ NO (Cycles are detected and BLOCKED)

**The dependency graph analysis** (`src/dependency_graph.rs:203-209`):
```rust
/// Check if graph has a cycle
pub fn is_acyclic(&self) -> bool {
    toposort(&self.graph, None).is_ok()
}
```

**BlockProcessor validation** (`src/block_processor.rs:64-67`):
```rust
// Verify graph is acyclic
if !dependency_graph.is_acyclic() {
    return Err("Dependency graph has cycles".to_string());
}
```

**What this means**:
- Any attempt to create `~a: ~b` + `~b: ~a` will be REJECTED
- Topological sort fails on cycles
- Graph won't build

### Example of BLOCKED Feedback

```phonon
~delay: ~feedback # delay 0.5 0.7   # ❌ BLOCKED
~mix: ~input + ~delay                # ❌ BLOCKED
~feedback: ~mix * 0.5                # ❌ Circular dependency!
out: ~mix
```

**Error**: "Cycle detected in audio graph at node X"

### Why Cycles Are Blocked

**DAW-style block processing requires topological execution order**:
1. Determine execution order via topological sort
2. Process nodes in order (dependencies first)
3. Each node processed exactly once per block

**Cycles break this**:
- No valid topological order exists
- Can't determine which node to process first
- Would require iterative solving or delay-line insertion

---

## What About Feedback Effects?

### Current Behavior: ✅ YES (Internal node feedback works perfectly)

**Feedback EXISTS but is INTERNAL to nodes**, not at graph level.

**Example: CombFilterNode** (`src/nodes/comb_filter.rs:148-153`):
```rust
// Comb filter: output = input + feedback * delayed
pub struct CombFilterNode {
    buffer: Vec<f32>,             // Internal delay buffer
    feedback_input: NodeId,       // Feedback amount (0.0 to 0.99)
    // ... other fields
}

// In process_block:
let delayed = self.buffer[read_pos];
output[i] = sample + feedback * delayed;  // Internal feedback!
self.buffer[self.write_pos] = output[i];  // Write back to buffer
```

**Nodes with internal feedback**:
- ✅ `comb_filter` - Resonant comb filtering
- ✅ `flanger` - Modulated delay with feedback
- ✅ `phaser` - Allpass filter cascade with feedback
- ✅ `reverb` - Multiple feedback delays
- ✅ `chorus` - Modulated delays
- ✅ `pingpong_delay` - Stereo ping-pong with feedback
- ✅ `tape_delay` - Tape-style delay with feedback
- ✅ `dattorro_reverb` - Complex feedback reverb
- ✅ `karplus_strong` - Feedback-based string synthesis
- ✅ `waveguide` - Physical modeling with feedback

**How they work**:
1. Node has internal state (delay buffer, filter state, etc.)
2. Feedback parameter controls how much delayed signal feeds back
3. All feedback is delay-based (at least 1 sample delay)
4. Stable because feedback < 1.0 and delay prevents instant loop

**This is fundamentally different from graph-level cycles**:
- Graph-level: `A → B → C → A` (BLOCKED)
- Node-internal: `A.process() { delayed = buffer[t-1]; output = input + feedback * delayed }` (WORKS)

---

## Complex Processing Chains

### Current Behavior: ✅ FULLY SUPPORTED

**You CAN build very complex processing chains**:

```phonon
-- Complex multi-stage processing
~source: s "bd sn hh cp"
~filtered: ~source # lpf 2000 0.8 # hpf 100 0.5
~delayed: ~filtered # delay 0.25 0.4
~reverbed: ~delayed # reverb 0.3 0.8
~compressed: ~reverbed # compressor 0.5 4.0 0.01 0.1
~limited: ~compressed # limiter 0.8

out: ~limited
```

**Multi-branch parallel processing**:
```phonon
-- Split signal into multiple processing paths
~source: saw 55

-- Low frequency path
~low: ~source # lpf 200 0.8 # distortion 0.5

-- Mid frequency path
~mid: ~source # bpf 1000 2.0 # chorus 0.5 0.3

-- High frequency path
~high: ~source # hpf 3000 0.5 # reverb 0.3 0.9

-- Mix all three paths
out: ~low * 0.4 + ~mid * 0.3 + ~high * 0.3
```

**This creates a dependency graph**:
```
source (NodeId 0)
  ├─→ low_lpf (1) → low_dist (2)
  ├─→ mid_bpf (3) → mid_chorus (4)
  └─→ high_hpf (5) → high_reverb (6)
        └─→ addition (7) → addition (8) → output (9)
```

**Execution order** (via topological sort):
```
Batch 0: [0]           # source (no dependencies)
Batch 1: [1, 3, 5]     # All filters (parallel execution possible!)
Batch 2: [2, 4, 6]     # All effects (parallel execution possible!)
Batch 3: [7]           # First addition
Batch 4: [8]           # Second addition
Batch 5: [9]           # Output
```

**This is extremely powerful**:
- Parallel processing of independent branches
- Multi-band processing (crossover, different FX per band)
- Complex routing without feedback loops
- All buses work fine as long as no cycles

---

## The Bus-in-Chain Problem

### Current Limitation: ❌ Bus references in chains are BROKEN

**From** `src/compositional_compiler.rs:5935-5950`:
```rust
Expr::BusRef(bus_name) => {
    // Bus references in chains are BROKEN
    // The correct behavior would be to re-instantiate the bus's effect chain
    // with the left signal as input, but buses are compiled to NodeIds which
    // are already-evaluated nodes, not templates.
    eprintln!("⚠️  Warning: Bus '~{}' used in chain - effect will be ignored", bus_name);
    compile_expr(ctx, left)  // Pass-through
}
```

**What doesn't work**:
```phonon
~reverb_settings: reverb 0.3 0.8
~drums: s "bd sn" # ~reverb_settings  # ❌ Bus ignored in chain!
```

**Why**: Buses are NodeIds, not "effect templates" that can be re-applied with different inputs.

**Workaround - Use explicit signals**:
```phonon
~drums: s "bd sn"
~drums_reverbed: ~drums # reverb 0.3 0.8  # ✅ Works
out: ~drums_reverbed
```

---

## Comparison: What Works vs. What Doesn't

### ✅ WORKS: Static processing graphs

```phonon
~osc1: sine 220
~osc2: saw 440
~mixed: ~osc1 * 0.5 + ~osc2 * 0.5
~filtered: ~mixed # lpf 2000 0.8
~reverbed: ~filtered # reverb 0.3 0.9
out: ~reverbed
```

**Why**: No cycles, clear dependency order, buses hold NodeIds.

### ✅ WORKS: Pattern-controlled parameters

```phonon
~cutoffs: p "<500 1000 2000>" $ fast 2
~bass: saw 55 # lpf ~cutoffs 0.8
out: ~bass
```

**Why**: `~cutoffs` is a PatternNode that outputs varying values. No cycles.

### ✅ WORKS: Complex parallel routing

```phonon
~source: brown_noise 0.3
~path1: ~source # lpf 500 0.8
~path2: ~source # hpf 2000 0.5
~path3: ~source # bpf 1000 2.0
out: ~path1 + ~path2 + ~path3
```

**Why**: Directed acyclic graph (DAG), parallel branches, clear execution order.

### ✅ WORKS: Internal feedback in effects

```phonon
~signal: brown_noise 0.3
~resonant: ~signal # comb_filter 0.009 0.7  # Feedback comb
~flanged: ~resonant # flanger 0.5 0.6       # Feedback flanger
out: ~flanged
```

**Why**: Feedback is internal to the nodes, not at graph level.

### ❌ DOESN'T WORK: Bus as effect template in chain

```phonon
~my_reverb: reverb 0.3 0.8
~drums: s "bd sn" # ~my_reverb  # ❌ IGNORED
```

**Why**: Buses aren't templates, they're NodeIds.

### ❌ DOESN'T WORK: Graph-level feedback loops

```phonon
~input: sine 440
~delayed: ~feedback # delay 0.5 0.7  # ❌ CYCLE DETECTED
~feedback: ~delayed * 0.5             # ❌ BLOCKED
out: ~delayed
```

**Why**: Topological sort fails, cycle detection rejects graph.

### ❌ DOESN'T WORK: Dynamic pattern changes

```phonon
~pattern: "bd sn"
~drums: s ~pattern
-- Later change ~pattern to "hh cp"  ❌ NOT POSSIBLE
```

**Why**: Pattern compiled once, no re-evaluation mechanism.

---

## Architectural Options for Feedback

If you wanted to enable graph-level feedback, here are the options:

### Option 1: Unit Delay Insertion (SuperCollider approach)

**Concept**: Automatically insert 1-sample delay in feedback paths.

```phonon
~input: sine 440
~delayed: ~feedback # delay 0.5 0.7
~feedback: ~delayed * 0.5
out: ~delayed
```

**Compiler transforms to**:
```
input → delay → feedback → unit_delay → delayed
                              ↑          |
                              └──────────┘
```

**Implementation**:
1. Detect cycles in dependency graph
2. Insert UnitDelayNode at cycle break point
3. Now graph is acyclic (topological sort works)
4. Process normally

**Pros**: Enables feedback, relatively simple
**Cons**: Changes semantics (adds delay), may be unexpected

### Option 2: Iterative Block Solver

**Concept**: Process blocks iteratively until convergence.

```
for iteration in 0..MAX_ITERATIONS {
    for node in topological_partial_order {
        node.process_block();
    }
    if converged { break; }
}
```

**Pros**: True simultaneous equations, no added delay
**Cons**: Very complex, performance cost, may not converge

### Option 3: Explicit Feedback Nodes (Current approach - RECOMMENDED)

**Concept**: Use dedicated feedback nodes with internal delay.

```phonon
~input: sine 440
~delay_out: ~input # feedback_delay 0.5 0.7 ~feedback
~feedback: ~delay_out * 0.5
out: ~delay_out
```

**FeedbackDelayNode**:
- Has internal delay buffer
- Takes feedback signal as input
- No graph-level cycle (delay node has all state)

**Pros**: Explicit, predictable, no graph changes needed
**Cons**: Requires special nodes for each feedback type

**This is how most audio software works** (Max/MSP, Reaktor, VCV Rack, etc.)

---

## Recommendations

### For the current session: ✅ NO CHANGES NEEDED

The architecture is **working correctly as designed**:
1. ✅ Cycle detection prevents invalid graphs
2. ✅ Complex processing chains work great
3. ✅ Internal node feedback works perfectly
4. ✅ Parallel execution optimization works

### For future enhancements:

**High Priority**:
1. **Fix bus-in-chain limitation** (line 5935 in compiler)
   - Allow buses to be used as effect templates
   - Store both NodeId AND original Expr for buses
   - Re-compile with new input when used in chain

**Medium Priority**:
2. **Add explicit feedback nodes**
   - `FeedbackDelayNode` - General-purpose feedback
   - `FeedbackCombNode` - Simplified feedback comb
   - Document feedback patterns clearly

**Low Priority**:
3. **Dynamic pattern re-evaluation**
   - Store Expr alongside NodeId
   - Mechanism to trigger re-compilation
   - Pattern change notifications
   - Would enable live pattern editing

**Not Recommended**:
4. ❌ Automatic unit delay insertion
   - Too magical, changes semantics unexpectedly
   - Better to be explicit with feedback nodes

---

## Conclusion

**Can buses hold patterns and create feedback loops?**

**Answer**:
- ✅ Buses work great for complex processing chains
- ❌ Buses don't "hold patterns" (they hold NodeIds)
- ❌ Graph-level feedback loops are blocked by design (correctly!)
- ✅ Internal node feedback works perfectly
- ⚠️  Bus-in-chain usage is currently broken (known limitation)

**The current architecture is sound**. Feedback prevention is a feature, not a bug - it ensures graphs are processable with clear execution order. Internal node feedback provides all the musical feedback you need (reverb, delay, comb filters, etc.).

**For live coding**, the current system supports:
- Pattern-controlled everything (cutoffs, feedback amounts, delay times)
- Complex multi-stage processing
- Parallel routing and mixing
- Rich feedback effects (comb, flanger, reverb, etc.)

**This is production-ready for making music!** 🎉

The only real limitation is bus-in-chain (line 5935), which is a known issue with a clear fix path.
