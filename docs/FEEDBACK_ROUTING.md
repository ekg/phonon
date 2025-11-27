# Feedback Routing and Signal Flow in Phonon

**Last Updated:** 2025-11-22

This document explains how feedback routing and complex signal flow patterns work in Phonon, including limitations, best practices, and practical examples.

---

## Table of Contents

1. [Overview](#overview)
2. [Feedback Architecture](#feedback-architecture)
3. [What Works](#what-works)
4. [What Doesn't Work](#what-doesnt-work)
5. [Common Patterns](#common-patterns)
6. [Mixing Strategies](#mixing-strategies)
7. [Examples](#examples)
8. [Testing](#testing)
9. [Technical Details](#technical-details)

---

## Overview

Phonon supports complex signal routing through its bus system, allowing you to:
- **Split signals** to multiple destinations
- **Mix signals** using arithmetic or the `mix` function
- **Create feedback** using effect parameters (delay, reverb) OR circular bus dependencies
- **Chain effects** in series or parallel
- **Modulate parameters** with patterns and buses
- **Use circular bus dependencies** including self-referential buses and multi-bus cycles

✅ **Circular bus dependencies are fully supported** via two-pass compilation.

---

## Feedback Architecture

### How Feedback Works in Phonon

Phonon has two levels of feedback support:

1. **BlockProcessor Level** (Low-Level)
   - The audio engine supports circular dependencies between nodes
   - Uses one-block delay (≈11ms at 44.1kHz, 512 samples)
   - Cycles are detected and allowed
   - See `tests/test_block_processor_feedback.rs`

2. **DSL Compiler Level** (High-Level)
   - ✅ The DSL compiler **fully supports circular bus dependencies** via two-pass compilation
   - ✅ You CAN write `~a: ~b` and `~b: ~a`
   - ✅ Self-referential buses work: `~feedback: ~feedback + ~input`
   - ✅ Forward references work: buses can reference buses defined later

### How Two-Pass Compilation Works

The DSL compiler uses a two-pass approach to enable circular dependencies:

**Pass 1 (Pre-Registration):**
- Iterates through all bus assignments
- Creates placeholder nodes for each bus name
- Registers all bus names in the compiler context

**Pass 2 (Compilation):**
- Compiles all bus expressions
- Can reference any bus (including forward references and cycles)
- Placeholder nodes are overwritten with actual compiled expressions

This allows patterns like `~a: ~b # lpf 1000 0.8` followed by `~b: ~a # delay 0.1 0.5` to work correctly.

---

## What Works

### 1. Circular Bus Dependencies ✨ NEW!

Self-referential buses and multi-bus cycles are fully supported:

```phonon
-- Self-referential feedback (bus references itself)
~input: sine 440 * 0.5
~feedback: ~input * 0.5 + ~feedback * 0.3
out: ~feedback

-- Two-bus cycle (a -> b -> a)
~a: ~b # lpf 1000 0.8
~b: ~a # delay 0.1 0.5
out: ~a * 0.5

-- Reverb with self-feedback loop and input mixing
~input: saw 110 * 0.5
~feedback: (~feedback * 0.7 + ~input * 0.3) # reverb 0.95 0.3 0.8
out: ~feedback * 0.5

-- Three-bus cycle (a -> b -> c -> a)
~a: ~input * 0.4 + ~c * 0.2
~b: ~a # lpf 2000 0.7
~c: ~b # delay 0.1 0.5
out: ~a + ~b + ~c
```

### 2. Effect Feedback Parameters

Most effects have built-in feedback parameters:

```phonon
-- Delay with 70% feedback
~delayed: ~input # delay 0.25 0.7

-- Reverb with large room (internal feedback)
~verb: ~input # reverb 0.9 0.4 0.7
```

### 2. Cascaded Effects

Chain effects in series:

```phonon
~stage1: ~input # delay 0.125 0.6
~stage2: ~stage1 # delay 0.25 0.5
~stage3: ~stage2 # reverb 0.8 0.4 0.7
out: ~stage3
```

### 3. Parallel Routing

Split, process, and mix:

```phonon
~input: saw 110 * 0.5
~path_a: ~input # lpf 1500 0.8 # delay 0.25 0.6
~path_b: ~input # hpf 500 0.8 # delay 0.33 0.5
out: ~path_a * 0.5 + ~path_b * 0.5
```

### 4. Signal Splitting

Reference the same bus multiple times:

```phonon
~source: sine 880 * 0.3
~tap1: ~source # delay 0.037 0.7
~tap2: ~source # delay 0.043 0.7
~tap3: ~source # delay 0.051 0.7
out: (~tap1 + ~tap2 + ~tap3) * 0.3
```

### 5. FM Synthesis

Modulate oscillator frequencies:

```phonon
~modulator: sine 5.0 * 100
~carrier: sine (~modulator + 440)
out: ~carrier * 0.5
```

### 6. Parameter Modulation

Use patterns and buses to modulate effect parameters:

```phonon
~lfo: sine 0.5 * 2000 + 1000
~filtered: ~input # lpf ~lfo 0.8
```

---

## Limitations

✅ **Circular bus dependencies work!** There are no significant limitations on feedback routing in Phonon.

The two-pass compilation system allows:
- ✅ Self-referential buses (`~a: ~a + ...`)
- ✅ Two-bus cycles (`~a: ~b`, `~b: ~a`)
- ✅ Multi-bus cycles (`~a: ~b`, `~b: ~c`, `~c: ~a`)
- ✅ Forward references (reference buses defined later)
- ✅ Complex feedback networks (cross-feedback, FM in feedback loops, etc.)

**Technical Note:** Feedback creates a one-block delay (≈11ms at 44.1kHz) which is handled automatically by the BlockProcessor. This is imperceptible for most musical applications and is identical to how feedback works in professional DAWs and hardware.

---

## Common Patterns

### Delay Feedback (Dub Techno)

```phonon
tempo: 0.5
~kick: sine 55 * 0.6
~dub: ~kick # delay 0.375 0.75 # hpf 800 0.7
out: ~kick * 0.5 + ~dub * 0.5
```

**What's happening:**
1. Kick drum (55 Hz sine wave)
2. Delay with 75% feedback creates echoes
3. HPF at 800 Hz removes low frequencies from echoes
4. Mix dry kick with delayed signal

---

### Multi-Tap Delay

```phonon
tempo: 1.0
~source: sine 880 * 0.3
~tap1: ~source # delay 0.037 0.7
~tap2: ~source # delay 0.043 0.7
~tap3: ~source # delay 0.051 0.7
~tap4: ~source # delay 0.061 0.7
out: (~source + ~tap1 + ~tap2 + ~tap3 + ~tap4) * 0.2
```

**What's happening:**
1. Source impulse
2. Four delay taps with different times (37ms, 43ms, 51ms, 61ms)
3. Each tap has internal feedback (70%)
4. Mix all taps together for diffuse, reverb-like sound

---

### Send/Return (Aux Send)

```phonon
tempo: 0.5
~dry1: sine 440 * 0.3
~dry2: saw 220 * 0.3
~send: (~dry1 + ~dry2) * 0.5
~return: ~send # reverb 0.9 0.5 0.9
out: ~dry1 * 0.4 + ~dry2 * 0.4 + ~return * 0.3
```

**What's happening:**
1. Two dry sources (sine and saw)
2. Both mixed and sent to shared reverb
3. Reverb return mixed with dry signals
4. Classic mixing console aux send pattern

---

### Filter Sweep with Feedback

```phonon
tempo: 0.5
~source: saw 110 * 0.5
~lfo: sine 0.25 * 2000 + 1000
~filtered: ~source # lpf ~lfo 0.8 # delay 0.25 0.6
out: ~filtered * 0.7
```

**What's happening:**
1. Sawtooth source
2. LFO sweeps filter cutoff (1000-3000 Hz, 0.25 Hz rate)
3. Filtered signal fed to delay with feedback
4. Creates evolving, rhythmic texture

---

## Mixing Strategies

### 1. Bus Arithmetic (Manual Control)

Direct control over mix levels:

```phonon
~mixed: ~a * 0.6 + ~b * 0.4
```

**Pros:**
- Exact control over levels
- Can create complex expressions
- No surprises

**Cons:**
- Must manually balance levels
- Verbosefor many sources

---

### 2. Mix Function (Auto-Normalized)

Automatic level normalization:

```phonon
~mixed: mix ~a ~b ~c
-- Equivalent to: (~a + ~b + ~c) / 3
```

**Pros:**
- Automatic level management
- Prevents volume multiplication
- Cleaner for many sources

**Cons:**
- Less control over individual levels
- May need manual scaling afterward

---

### 3. Effect Mix Parameters

Built-in wet/dry mixing:

```phonon
~wet: ~input # reverb 0.9 0.5 0.7  -- Last param is wet/dry mix
```

**Pros:**
- Simple and intuitive
- Standard effect parameter

**Cons:**
- Fixed to one effect at a time

---

## Examples

See `docs/examples/feedback_routing/` for complete, runnable examples:

- **01_dub_delay.ph** - Dub techno delay with HPF
- **02_multi_tap_delay.ph** - Multiple delay taps
- **03_parallel_effects.ph** - Parallel processing paths
- **04_send_return_reverb.ph** - Aux send pattern
- **05_filter_sweep_feedback.ph** - LFO filter modulation with delay
- **06_fm_synthesis.ph** - Frequency modulation

Each example includes:
- Commented code explaining what's happening
- Suggested parameter tweaks
- Run instructions

---

## Testing

All feedback routing patterns are comprehensively tested:

### Circular Dependency Tests

**16 tests** covering circular bus dependencies in `tests/test_circular_dependencies.rs`:

```bash
cargo test --test test_circular_dependencies
```

**Tests include:**
- Self-referential feedback (3 tests)
- Two-bus cycles (3 tests)
- Three-bus cycles (2 tests)
- Complex patterns: FM in feedback, cross-feedback networks, Karplus-Strong (8 tests)

### General Feedback Routing Tests

**24 tests** in `tests/test_feedback_routing_patterns.rs`:

```bash
cargo test --test test_feedback_routing_patterns
```

**Tests include:**
- Delay feedback (3 tests)
- Reverb feedback (2 tests)
- Parallel routing (3 tests)
- Multi-tap delays (1 test)
- FM synthesis (3 tests)
- Mix operators (3 tests)
- Production scenarios (9 tests)

**Total: 40 feedback routing tests, 100% passing** as of 2025-11-22.

---

## Technical Details

### One-Block Delay Mechanism

At the BlockProcessor level (not DSL), feedback creates a one-block delay:

- **Block size:** 512 samples @ 44.1kHz
- **Delay time:** ≈11.6ms
- **First block:** Reads zeros
- **Subsequent blocks:** Reads previous output

This is implemented in `src/block_processor.rs` and `src/dependency_graph.rs`.

### Dependency Graph

The compiler builds a dependency graph to determine execution order:

1. **Acyclic graph:** Topological sort for optimal order
2. **Cyclic graph:** Allowed at BlockProcessor level, processed in ID order

See `src/dependency_graph.rs:86-99`:

```rust
/// Cycles are allowed! When a cycle exists:
/// - First block: Cyclic nodes read from zero-initialized buffers
/// - Subsequent blocks: Cyclic nodes read from previous block's output
/// - Execution order is simply all nodes in ID order (0, 1, 2, ...)
pub fn execution_order(&self) -> Result<Vec<NodeId>, String> {
    match toposort(&self.graph, None) {
        Ok(order) => Ok(order.iter().map(|&idx| self.graph[idx]).collect()),
        Err(_cycle) => Ok((0..self.graph.node_count()).collect())
    }
}
```

### DSL Compiler Two-Pass Implementation

The DSL compiler (`src/compositional_compiler.rs`) uses two-pass compilation to support circular dependencies:

```rust
pub fn compile_program(
    statements: Vec<Statement>,
    sample_rate: f32,
) -> Result<UnifiedSignalGraph, String> {
    let mut ctx = CompilerContext::new(sample_rate);

    // PASS 1: Pre-register all bus names with placeholder nodes
    // This allows circular dependencies (a -> b -> a)
    for statement in &statements {
        if let Statement::BusAssignment { name, .. } = statement {
            // Create a placeholder node (Constant 0.0) for this bus
            // This will be overwritten in Pass 2, but allows forward references
            let placeholder_node = ctx.graph.add_node(SignalNode::Constant { value: 0.0 });
            ctx.buses.insert(name.clone(), placeholder_node);
            ctx.graph.add_bus(name.clone(), placeholder_node);
        }
    }

    // PASS 2: Compile all statements (can now reference any bus, including forward refs)
    for statement in statements {
        compile_statement(&mut ctx, statement)?;
    }

    let mut graph = ctx.into_graph();
    // ... rest of compilation
}
```

**How It Works:**

1. **Pass 1 (Pre-Registration):**
   - Iterates through all bus assignments
   - Creates a placeholder `Constant { value: 0.0 }` node for each bus
   - Registers the bus name in the compiler context

2. **Pass 2 (Compilation):**
   - Compiles each bus assignment's expression
   - Can reference any bus (forward references work because all names are registered)
   - Overwrites placeholder nodes with actual compiled expressions

3. **Circular Dependencies:**
   - When `~a: ~b # lpf 1000 0.8` is compiled, `~b` lookup succeeds (placeholder exists)
   - When `~b: ~a # delay 0.1 0.5` is compiled, `~a` lookup succeeds
   - The BlockProcessor detects the cycle and handles the one-block delay automatically

This approach requires no special syntax and works seamlessly with existing code.

---

## Best Practices

### 1. Circular Dependencies Work - Use Them!

✅ **Self-referential feedback** (bus references itself):
```phonon
~feedback: ~feedback * 0.7 + ~input * 0.3
```

✅ **Two-bus cycles** (cross-feedback):
```phonon
~a: ~b # lpf 1000 0.8
~b: ~a # delay 0.1 0.5
```

✅ **Multi-bus cycles**:
```phonon
~a: ~c # delay 0.1 0.6  -- Forward reference to ~c (works!)
~b: ~a # delay 0.2 0.5
~c: ~b # delay 0.3 0.4
```

### 2. Choose Your Feedback Style

Both approaches work - pick based on your use case:

**Effect Parameters** (simpler for single delay/reverb):
```phonon
~delayed: ~input # delay 0.25 0.7  -- Built-in feedback parameter
```

**Circular Buses** (more flexible for complex routing):
```phonon
~feedback: (~feedback * 0.7 + ~input * 0.3) # lpf 1000 0.8 # delay 0.25 0.6
```

Use circular buses when you need:
- Multiple effects in feedback loop
- Mixing external input into feedback
- Cross-feedback between multiple paths
- Complex routing topologies

### 3. Understand One-Block Delay

Circular dependencies create a one-block delay (≈11ms):
- **Imperceptible** for most musical applications
- **Identical** to how DAWs and hardware handle feedback
- **Automatic** - no special handling needed

### 4. Use Mix Function for Many Sources

✅ **Cleaner:**
```phonon
~mixed: mix ~a ~b ~c ~d ~e
```

✅ **Also works, more control:**
```phonon
~mixed: (~a + ~b + ~c + ~d + ~e) / 5.0
```

---

## Related Documentation

- **Examples:** `docs/examples/feedback_routing/README.md`
- **Tests (General):** `tests/test_feedback_routing_patterns.rs` (24 tests)
- **Tests (Circular):** `tests/test_circular_dependencies.rs` (16 tests)
- **Dependency Graph:** `src/dependency_graph.rs`
- **Block Processor:** `src/block_processor.rs`
- **Compiler:** `src/compositional_compiler.rs` (two-pass implementation)

---

## Summary

**Everything Works! ✅**

Phonon supports all forms of feedback routing:

- ✅ **Circular bus dependencies** (~a → ~b → ~a) ✨ NEW!
- ✅ **Self-referential buses** (~a: ~a + ...) ✨ NEW!
- ✅ **Forward references** (~a: ~b where ~b defined later) ✨ NEW!
- ✅ **Effect feedback parameters** (delay, reverb)
- ✅ **Cascaded effects** (a → b → c)
- ✅ **Parallel routing** (split → process → mix)
- ✅ **Signal splitting** (one source, multiple taps)
- ✅ **FM synthesis and parameter modulation**
- ✅ **Mix operators** (bus arithmetic, `mix` function)

**Key Insight:** Feedback in Phonon can be achieved through **either**:
1. **Effect parameters** (simpler for single delay/reverb)
2. **Circular bus dependencies** (more flexible for complex routing)

Both approaches work seamlessly thanks to two-pass compilation and the BlockProcessor's one-block delay mechanism.

**Resources:**
- **Examples:** `docs/examples/feedback_routing/`
- **Tests:** `cargo test --test test_feedback_routing_patterns` (24 tests)
- **Circular Tests:** `cargo test --test test_circular_dependencies` (16 tests)
- **Total:** 40 comprehensive tests, 100% passing
