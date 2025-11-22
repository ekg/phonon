# Phase 2: Audio → Pattern Modulation - Architecture Design

**Date:** 2025-11-22
**Status:** 🔬 Research Complete → Design Phase
**Goal:** Enable audio signals to modulate pattern transform parameters

---

## Executive Summary

**BREAKTHROUGH DISCOVERY:** The pattern system **ALREADY supports dynamic parameters**! Pattern transforms like `fast`, `slow`, `degrade_by` etc. all accept `Pattern<f64>` parameters and query them at cycle time.

**What's working:**
- ✅ Pattern transforms accept `Pattern<f64>` parameters
- ✅ Transforms query parameters dynamically at cycle boundaries
- ✅ Pattern → Audio modulation works (patterns control filter cutoff, pan, etc.)

**What's missing:**
- ❌ Audio → Pattern modulation (audio can't control pattern transforms yet)
- ❌ Compiler doesn't handle `Expr::Bus` in transform parameters
- ❌ No bridge from `Signal` (audio) to `Pattern<f64>` (pattern parameter)

**The solution requires 3 changes:**
1. Add `SignalAsPattern` node to unified_graph.rs
2. Handle `Expr::Bus` case in apply_transform_to_pattern
3. Update DYNAMIC_EVERYTHING_PLAN.md with findings

---

## Architecture Research Findings

### 1. Pattern System is Already Dynamic!

**Discovery in src/pattern.rs:669-718**

```rust
pub fn fast(self, speed: Pattern<f64>) -> Self {
    Pattern::new(move |state| {
        // Query speed pattern at cycle start
        let speed_haps = speed.query(&speed_state);
        let factor = speed_haps.first().map(|h| h.value).unwrap_or(1.0);

        // Use queried value to transform pattern
        // ...
    })
}
```

**All pattern transforms follow this pattern:**
- `fast(speed: Pattern<f64>)`
- `slow(speed: Pattern<f64>)`
- `degrade_by(probability: Pattern<f64>)`
- `late(amount: Pattern<f64>)`
- `early(amount: Pattern<f64>)`

**This means:** The infrastructure for dynamic modulation ALREADY EXISTS!

---

### 2. Current Pattern → Audio Flow (Working)

**Example from unified_graph.rs:**

```rust
// Create a pattern that outputs numeric values
let cutoff_pattern = parse_mini_notation("200 500 1000 2000");
let cutoff_node = graph.add_node(SignalNode::Pattern {
    pattern_str: "200 500 1000 2000".to_string(),
    pattern: cutoff_pattern,
    last_value: 500.0,
    last_trigger_time: -1.0,
});

// Use pattern to control filter cutoff (audio parameter)
let filtered = graph.add_node(SignalNode::LowPass {
    input: Signal::Node(sample_node),
    cutoff: Signal::Node(cutoff_node),  // Pattern → Audio! ✅
    q: Signal::Value(2.0),
});
```

**Mechanism:**
1. `SignalNode::Pattern` queries pattern at current cycle position
2. Parses event value as number
3. Caches in `last_value`
4. Returns as audio signal (f32)
5. LPF reads this signal value for cutoff frequency

**Result:** Patterns can modulate any audio parameter ✅

---

### 3. Current DSL Compiler Flow (Missing Audio→Pattern)

**Example:** `~drums: s "bd*<1 2 4 8>" $ fast 2`

**What happens:**

1. **Parser** creates: `Transform::Fast(Box::new(Expr::Number(2.0)))`

2. **apply_transform_to_pattern** (src/compositional_compiler.rs:6533):
   ```rust
   Transform::Fast(speed_expr) => {
       let speed_pattern = match speed_expr.as_ref() {
           Expr::String(s) => {
               // fast "2 3 4" - pattern-based speed
               parse_mini_notation(s).fmap(|s| s.parse::<f64>().unwrap_or(1.0))
           }
           _ => {
               // fast 2 - constant speed
               let speed = extract_number(&speed_expr)?;
               Pattern::pure(speed)  // Wrap constant in pattern
           }
       };
       Ok(pattern.fast(speed_pattern))
   }
   ```

3. **Pattern transform** receives `Pattern<f64>` and queries it at cycle time

**What's MISSING:** Handling `Expr::Bus` case!

When user writes:
```phonon
~lfo: sine 0.5
~drums: s "bd*<1 2 4 8>" $ fast ~lfo  -- ❌ NOT SUPPORTED YET
```

The compiler creates `Expr::Bus("lfo")`, which hits the `_` case and tries `extract_number()`, which fails!

---

## The Missing Bridge: Signal → Pattern

### Challenge

**Two different execution models:**
1. **Patterns:** Queried at discrete event boundaries (cycle positions)
2. **Signals:** Evaluated continuously at sample rate (44.1kHz)

**The gap:** How do we convert a continuous audio signal into a pattern that can be queried?

### Solution Design

**Add a new `SignalNode` variant that samples audio signals for pattern use:**

```rust
/// Bridge from Signal (audio) to Pattern (pattern parameter)
/// Evaluates audio signal and exposes it as a queryable pattern
SignalAsPattern {
    signal: Signal,           // Input audio signal (e.g., LFO, envelope)
    last_sampled_value: f32,  // Last sampled value from signal
    last_sample_time: f32,    // Cycle position when last sampled
}
```

**Evaluation mechanism:**

```rust
SignalNode::SignalAsPattern {
    signal,
    last_sampled_value,
    last_sample_time,
} => {
    // Evaluate the input signal to get current audio value
    let current_value = self.eval_signal(signal)?;

    // Sample & hold: Update cached value at cycle boundaries
    let current_cycle = self.get_cycle_position();
    if (current_cycle - *last_sample_time).abs() > 0.01 {
        // New cycle - sample the signal
        *last_sampled_value = current_value;
        *last_sample_time = current_cycle;
    }

    // Return cached value
    *last_sampled_value
}
```

**Pattern creation:**

```rust
// In apply_transform_to_pattern, when handling Expr::Bus:
Expr::Bus(bus_name) => {
    // Create SignalAsPattern node that samples the audio signal
    let signal = Signal::Bus(bus_name.clone());
    let pattern_node = ctx.graph.add_node(SignalNode::SignalAsPattern {
        signal,
        last_sampled_value: 0.0,
        last_sample_time: -1.0,
    });

    // Create a pattern that queries this node's value
    create_signal_sampling_pattern(pattern_node)
}
```

**Where `create_signal_sampling_pattern` returns:**

```rust
Pattern::new(move |state| {
    // Query the graph to get current value of SignalAsPattern node
    // This is a bit tricky - needs access to graph state during pattern query
    // May need to redesign how patterns access runtime values
    vec![Hap {
        whole: Some(state.span.clone()),
        part: state.span.clone(),
        value: /* value from SignalAsPattern node */,
        context: HashMap::new(),
    }]
})
```

---

## Implementation Challenges

### Challenge 1: Pattern-Graph Coupling

**Problem:** Patterns are pure data structures, but now need to query graph state at runtime.

**Current:** Patterns are created at compile time, queried independently of signal graph.

**Needed:** Patterns must read from signal graph during query.

**Possible solutions:**

**A. Runtime Pattern Evaluation** (Requires rearchitecture)
- Patterns capture `NodeId` reference
- During query, access `UnifiedSignalGraph` to read node value
- Requires passing graph context into pattern queries

**B. Signal-Sampled Pattern** (Simpler, one-way binding)
- Create `SignalNode::Pattern` that reads from a signal bus
- Pattern queries return last-sampled signal value
- Simpler but less dynamic (only updates at pattern query time)

**C. Dynamic Pattern Parameters** (Most aligned with vision)
- Pattern transforms store `Signal` references
- When querying pattern, evaluate signals to get current parameters
- Truly dynamic - parameters change continuously

### Challenge 2: Compiler Context

**Problem:** `apply_transform_to_pattern` doesn't have access to `CompilerContext` or `UnifiedSignalGraph`.

**Current signature:**
```rust
fn apply_transform_to_pattern<T>(
    templates: &HashMap<String, Expr>,
    pattern: Pattern<T>,
    transform: Transform,
) -> Result<Pattern<T>, String>
```

**Needed:** Access to graph to create `SignalAsPattern` nodes:
```rust
fn apply_transform_to_pattern<T>(
    ctx: &mut CompilerContext,  // ← Need this!
    pattern: Pattern<T>,
    transform: Transform,
) -> Result<Pattern<T>, String>
```

**Impact:** Need to update all call sites (many places in compositional_compiler.rs)

---

## Proposed Implementation Plan

### Step 1: Add SignalAsPattern Node

**File:** `src/unified_graph.rs`

**Add to SignalNode enum:**
```rust
/// Bridge from Signal to Pattern parameters
/// Samples audio signal and exposes value for pattern queries
SignalAsPattern {
    signal: Signal,
    last_value: f32,
    last_cycle: f32,
},
```

**Add evaluation in eval_signal:**
```rust
SignalNode::SignalAsPattern {
    signal,
    last_value,
    last_cycle,
} => {
    let current_value = self.eval_signal(signal)?;
    let current_cycle = self.get_cycle_position();

    // Sample & hold at cycle boundaries
    if (current_cycle.floor() - last_cycle.floor()).abs() > 0.01 {
        *last_value = current_value;
        *last_cycle = current_cycle;
    }

    *last_value
}
```

### Step 2: Update Compiler to Handle Expr::Bus

**File:** `src/compositional_compiler.rs`

**Update `apply_transform_to_pattern` signature:**
```rust
fn apply_transform_to_pattern<T: Clone + Send + Sync + Debug + 'static>(
    ctx: &mut CompilerContext,  // ← Add context parameter
    pattern: Pattern<T>,
    transform: Transform,
) -> Result<Pattern<T>, String>
```

**Add Expr::Bus handling in Transform::Fast (and all other transforms):**
```rust
Transform::Fast(speed_expr) => {
    let speed_pattern = match speed_expr.as_ref() {
        Expr::String(s) => {
            // Pattern-based: fast "2 3 4"
            parse_mini_notation(s).fmap(|s| s.parse::<f64>().unwrap_or(1.0))
        }
        Expr::Bus(bus_name) => {
            // Audio signal: fast ~lfo
            let signal = Signal::Bus(bus_name.clone());
            let pattern_node = ctx.graph.add_node(SignalNode::SignalAsPattern {
                signal,
                last_value: 1.0,
                last_cycle: -1.0,
            });

            // Create pattern that reads from this node
            create_signal_pattern(pattern_node, &ctx.graph)
        }
        _ => {
            // Constant: fast 2
            Pattern::pure(extract_number(&speed_expr)?)
        }
    };
    Ok(pattern.fast(speed_pattern))
}
```

**Implement `create_signal_pattern` helper:**
```rust
fn create_signal_pattern(
    node_id: NodeId,
    graph: &UnifiedSignalGraph,
) -> Pattern<f64> {
    // This is the tricky part - need pattern to read graph state
    // May need to redesign pattern-graph interaction

    // Option A: Store NodeId and provide graph access during query
    // Option B: Use Arc<Mutex<>> to share state
    // Option C: Sample once at pattern creation (not truly dynamic)

    // Simplest (Option C): Sample current value once
    let current_value = graph.eval_signal_for_node(node_id).unwrap_or(1.0);
    Pattern::pure(current_value)

    // TODO: Make this truly dynamic in future iteration
}
```

### Step 3: Update All Call Sites

**Files to update:** All places calling `apply_transform_to_pattern`

**Changes needed:**
- Pass `ctx` parameter
- Update all ~50-100 call sites in compositional_compiler.rs

### Step 4: Test Audio → Pattern Modulation

**Test file:** `tests/test_audio_to_pattern.rs`

**Test cases:**
1. `~lfo: sine 0.5; ~drums: s "bd*4" $ fast ~lfo` - LFO modulates pattern speed
2. `~env: adsr; ~pattern: s "bd" $ degradeBy ~env` - Envelope modulates probability
3. Verify pattern speed changes as LFO oscillates
4. Test feedback: audio → pattern → audio

---

## Revised Phase 2 Goals

Based on research findings, Phase 2 should deliver:

**Minimum Viable Implementation:**
1. ✅ Add `SignalAsPattern` node (sample & hold audio for pattern parameters)
2. ✅ Handle `Expr::Bus` in `apply_transform_to_pattern`
3. ✅ Test basic audio → pattern modulation works
4. ✅ Document current limitations (sample & hold, not fully dynamic yet)

**Future enhancements (Phase 3+):**
- Truly dynamic pattern parameters (continuous resampling)
- Pattern queries access graph state in real-time
- Bidirectional modulation (audio ↔ pattern)

---

## Technical Constraints

### Limitation: Sample & Hold

**Current design:** SignalAsPattern samples audio once per cycle boundary.

**Implication:** Pattern speed doesn't change smoothly within a cycle, only at cycle starts.

**Example:**
```phonon
~lfo: sine 0.1  -- Very slow LFO
~drums: s "bd*8" $ fast ~lfo
```

**Behavior:** Pattern speed steps between values at cycle boundaries, doesn't interpolate smoothly.

**Future:** To get smooth modulation, would need to:
1. Re-query patterns every sample
2. Or: Make patterns themselves signal-aware
3. Or: Use higher-frequency sampling (query multiple times per cycle)

### Limitation: One-Way Binding

**Current design:** Pattern parameter is computed once when pattern is created/applied.

**Implication:** Can't have pattern parameters that change after pattern creation.

**Workaround:** Recreate pattern on every cycle with updated parameters (performance cost).

---

## Success Criteria

**Phase 2 is complete when:**

1. ✅ Can write: `~lfo: sine 0.5; ~pat: s "bd" $ fast ~lfo`
2. ✅ Pattern speed varies based on LFO value
3. ✅ Test suite verifies audio → pattern modulation works
4. ✅ No regressions in existing pattern or audio functionality
5. ✅ Documentation explains current capabilities and limitations

---

## Open Questions

1. **Performance:** How expensive is creating `SignalAsPattern` nodes for every parameter?
2. **Caching:** Should we cache `SignalAsPattern` nodes for the same bus reference?
3. **Timing:** Should we sample at cycle boundaries, or continuously?
4. **Scope:** Should we support audio → pattern in Phase 2, or defer to Phase 3?

---

## Conclusion

**The good news:** Pattern infrastructure is ALREADY dynamic! Pattern transforms accept `Pattern<f64>` parameters and query them at runtime.

**The gap:** Missing bridge from `Signal` (audio) to `Pattern<f64>` (pattern parameter).

**The solution:** Add `SignalAsPattern` node + handle `Expr::Bus` in compiler.

**The limitation:** Initial implementation will be sample & hold (stepped changes), not fully continuous modulation. But it's a solid foundation for the full vision!

---

**Next Steps:** Design review, then implementation of Step 1 (SignalAsPattern node).
