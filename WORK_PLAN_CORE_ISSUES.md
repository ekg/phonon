# Work Plan: Resolving Core Phonon Limitations

These four issues are fundamental to Phonon's functionality and must be resolved.

## Issue 1: Bundled Sample Files
**Status**: Not started
**Complexity**: Low
**Files**: `samples/` directory, `src/sample_loader.rs`

### Problem
Sample-dependent tests fail without actual sample files. Tests use patterns like `s "bd sn hh cp"` but no samples exist.

### Solution
1. Create `samples/` directory in repo root
2. Add minimal samples: bd.wav, cp.wav, hh.wav, blip.wav (can generate synthetically)
3. Update sample loader to search:
   - `./samples/` (repo samples)
   - `~/.phonon/samples/` (user samples)
   - `~/dirt-samples/` (SuperDirt compatibility)
4. Generate samples using synthesis (no licensing issues)

### Tests to un-ignore
- `test_chopping_operations.rs` - all chopping tests
- `test_complex_feedback_networks.rs` - sample-dependent tests
- `test_sample_*.rs` - various sample tests

---

## Issue 2: Bus References as Function Parameters
**Status**: Not started
**Complexity**: Medium
**Files**: `src/compositional_parser.rs`, `src/compositional_compiler.rs`

### Problem
This fails:
```phonon
~cutoff $ 1000
out $ saw 55 # lpf ~cutoff 0.8
```
Parser doesn't recognize `~cutoff` as a valid argument to `lpf`.

### Current Behavior
- Parser treats function calls as: `func_name arg1 arg2 ...`
- Bus references (`~name`) are only parsed in specific contexts
- When `lpf` sees `~cutoff`, it doesn't know how to resolve it

### Solution
1. In `parse_expression()`, add bus reference as valid atom/expression
2. In compiler, when evaluating function args, check for BusRef variant
3. At eval time, resolve bus reference to its signal value
4. The bus value becomes the parameter input

### Key Insight
This is similar to how pattern strings work - they're resolved at eval time. Bus refs should work the same way.

### Tests to un-ignore
- `test_comb.rs` - `test_comb_pattern_delay`, `test_comb_flanging`
- `test_compositional_e2e.rs` - modulation tests
- Many filter/effect tests that want pattern-controlled params

---

## Issue 3: Circular Dependencies
**Status**: Not started
**Complexity**: High
**Files**: `src/compositional_compiler.rs`, `src/unified_graph.rs`

### Problem
This causes stack overflow:
```phonon
~feedback $ ~feedback * 0.5 + ~input * 0.5
```
The compiler recursively evaluates `~feedback` to define `~feedback`.

### Why It's Essential
Feedback is fundamental to:
- Reverb/delay effects
- Karplus-Strong synthesis
- Filter self-oscillation
- FM synthesis feedback
- Cross-coupled networks

### Solution: Delay-Line Cycle Breaking
1. **Detect cycles** during compilation (topological sort)
2. **Insert implicit delay** at cycle break points
3. **Use previous-sample value** for feedback references

Implementation:
```rust
// In compiler, when we detect a cycle:
// ~a depends on ~b, ~b depends on ~a
// Insert a 1-sample delay on one edge

struct BusState {
    current_value: f32,
    previous_value: f32,  // For feedback
}

// When evaluating ~feedback inside ~feedback's definition:
// Use previous_value, not current_value
```

### Key Files to Modify
- `compile_program()` - detect cycles via dependency graph
- `eval_node()` - handle feedback references
- Need `BusState` with previous/current values

### Tests to un-ignore
- `test_circular_dependencies.rs` - all tests
- `test_complex_feedback_networks.rs` - feedback tests

---

## Issue 4: Bus Triggering via s Pattern
**Status**: Not started
**Complexity**: Medium-High
**Files**: `src/voice_manager.rs`, `src/compositional_compiler.rs`

### Problem
This produces no audio:
```phonon
~synth $ sine 440
~trig $ s "~synth*4"
out $ ~trig
```
Expected: 4 sine tones per cycle, each triggered by the pattern

### Current Behavior
- `s "~synth"` is parsed as sample pattern
- Voice manager looks for sample file named "~synth"
- No such file exists, so silence

### What Should Happen
1. `s "~synth"` should recognize `~synth` as bus reference
2. When pattern triggers, it should:
   - Start/retrigger the synth bus
   - Apply default envelope (or specified envelope)
   - Mix into output

### Solution
1. In mini-notation parser, detect `~name` pattern
2. Create `BusTrigger` event type (vs `SampleTrigger`)
3. In voice manager, handle BusTrigger:
   - Look up bus signal graph
   - Create voice that renders bus with envelope
   - Track voice state for polyphony

### Key Insight
Bus triggering is like sample triggering but the "sample" is a signal graph, not a file.

### Tests to un-ignore
- `test_continuous_synthesis.rs` - all tests
- `test_bus_triggering.rs` - bus trigger tests
- `test_bus_synthesis_patterns.rs` - pattern synthesis

---

## Recommended Order

1. **Samples first** - Quick win, unblocks many tests
2. **Bus refs as params** - Medium effort, high value
3. **Bus triggering** - Complex but fundamental
4. **Circular deps** - Most complex, save for last

## Progress Tracking

- [ ] Issue 1: Bundled Samples
- [ ] Issue 2: Bus Refs as Params
- [ ] Issue 3: Circular Dependencies
- [ ] Issue 4: Bus Triggering
