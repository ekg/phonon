# Kludges and Areas for Improvement

**Generated**: 2025-10-20
**Purpose**: Identify non-generic, brittle, or hacky code that needs refactoring

---

## 🔴 CRITICAL: Chain Operator Hack

**Location**: `src/compositional_compiler.rs:758`

**The Problem**:
```rust
/// Compile chain operator: a # b
fn compile_chain(ctx: &mut CompilerContext, left: Expr, right: Expr) -> Result<NodeId, String> {
    match right {
        Expr::Call { name, mut args } => {
            let left_node = compile_expr(ctx, left)?;
            args.insert(0, Expr::Number(left_node.0 as f64)); // Hack: store node ID
            compile_function_call(ctx, &name, args)
        }
        _ => { /* ... */ }
    }
}
```

**Why It's Bad**:
- **Type confusion**: NodeId (usize) stored as Expr::Number (f64)
- **Brittle**: Every effect function must check if first arg is "secretly" a NodeId
- **Not generic**: Breaks normal number handling
- **Error-prone**: Easy to forget to check in new functions

**Impact**:
Affects **every** chainable function:
- All filters: `lpf`, `hpf`, `bpf`
- All effects: `reverb`, `delay`, `distortion`, `chorus`, `bitcrush`
- Envelope: `env`

**Better Approach**:
Add a proper `ChainInput` variant to `Expr`:

```rust
pub enum Expr {
    // ... existing variants ...

    /// Special marker for chained input (only used internally by compiler)
    ChainInput(NodeId),
}
```

Then in `compile_chain`:
```rust
fn compile_chain(ctx: &mut CompilerContext, left: Expr, right: Expr) -> Result<NodeId, String> {
    match right {
        Expr::Call { name, mut args } => {
            let left_node = compile_expr(ctx, left)?;
            args.insert(0, Expr::ChainInput(left_node)); // Explicit, type-safe
            compile_function_call(ctx, &name, args)
        }
        _ => { /* ... */ }
    }
}
```

And in effect compilers:
```rust
fn compile_filter(ctx: &mut CompilerContext, filter_type: &str, args: Vec<Expr>) -> Result<NodeId, String> {
    let (input_signal, cutoff_expr, q_expr) = if args.len() == 3 {
        match &args[0] {
            Expr::ChainInput(node_id) => {
                // Explicitly a chained input
                (Signal::Node(*node_id), &args[1], &args[2])
            }
            _ => {
                // Regular standalone call
                let input_node = compile_expr(ctx, args[0].clone())?;
                (Signal::Node(input_node), &args[1], &args[2])
            }
        }
    } else {
        // ... handle 2-arg case ...
    }
    // ...
}
```

**Benefits**:
- ✅ Type-safe: NodeId stays as NodeId
- ✅ Explicit: Clear when something is chained vs standalone
- ✅ Generic: Works uniformly for all functions
- ✅ Maintainable: New effects follow same pattern

**Estimated effort**: 2-3 hours
**Priority**: MEDIUM-HIGH (not blocking but improves code quality significantly)

---

## 🟡 Reverb Args Indexing Bug

**Location**: `src/compositional_compiler.rs:374`

**The Problem**:
```rust
if args.len() == 3 {
    if let Expr::Number(node_id) = &args[0] {
        let input_node = NodeId(*node_id as usize);
        (Signal::Node(input_node), &args[1], &args[2], &args[2]) // TODO: Fix args indexing
    }
}
```

The third parameter `mix` is using `&args[2]` twice instead of having a proper third argument.

**Impact**:
- Reverb in chained form only gets 2 parameters (room_size, damping) instead of 3
- The `mix` parameter is duplicated from `damping`

**Fix**:
The chained form needs 4 args (node_id + 3 params), but the current code assumes 3 total.

**Two options**:
1. Change chain to pass 4 args: `input # reverb room_size damping mix`
2. Make `mix` optional with a default value

**Estimated effort**: 30 minutes
**Priority**: MEDIUM (reverb works, just less flexible)

---

## 🟡 Sample Parameters Not Implemented

**Location**: `src/compositional_compiler.rs:166`

**The Problem**:
```rust
// TODO: Handle sample-specific parameters from remaining args
// For now, create a basic sample node with defaults
let node = SignalNode::Sample {
    pattern_str: pattern_str.clone(),
    pattern,
    // ...
    gain: Signal::Value(1.0),
    pan: Signal::Value(0.0),
    speed: Signal::Value(1.0),
    cut_group: Signal::Value(0.0),
    // ...
};
```

**What's Missing**:
Per-sample DSP parameters:
- `gain` - amplitude control
- `pan` - stereo positioning
- `speed` - playback rate
- `cut` - cut groups (voice stealing)
- `attack`, `release` - envelope parameters

**Old Approach (from ROADMAP.md)**:
```phonon
s("bd sn", gain="0.8 1.0", pan="0 1")  # ❌ Kwargs syntax - user rejected!
```

**New Approach (using stack)**:
```phonon
# Per-voice gain control
~kick: s "bd" * 0.8
~snare: s "sn" * 1.0
~hh: s "hh*4" * 0.4
~drums: stack [~kick, ~snare, ~hh]
out: ~drums
```

**Better Solution**:
Instead of kwargs, use pattern-based modulation:
```phonon
# Future syntax idea:
s "bd sn"
  | gain "0.8 1.0"
  | pan "0 1"
  | speed "1 0.5 2"
```

Or use DSL operators:
```phonon
s "bd sn"
  # gain_pattern "0.8 1.0"
  # pan_pattern "0 1"
```

**Estimated effort**: 3-4 days (needs design decision first)
**Priority**: MEDIUM-HIGH (user wants per-voice control, but stack solves immediate need)

---

## 🟡 Transforms Only Work on String Literals

**Location**: `src/compositional_compiler.rs:795`

**The Problem**:
```rust
fn compile_transform(ctx: &mut CompilerContext, expr: Expr, transform: Transform) -> Result<NodeId, String> {
    if let Expr::String(pattern_str) = expr {
        // ... applies transform ...
    }

    // For other expressions, compile them first then try to extract and transform
    // This is more complex and may not always work
    // For now, just compile the expression without the transform
    // TODO: Handle transforms on arbitrary expressions
    compile_expr(ctx, expr)
}
```

**What Doesn't Work**:
```phonon
~drums: s "bd sn"
~fast_drums: ~drums $ fast 2  # ❌ Transform ignored! Just returns ~drums
```

**Why It's Hard**:
- Bus references are already compiled to NodeIds
- Can't "rewind" and apply transform retroactively
- Would need to store pattern metadata alongside nodes

**Possible Solutions**:

### Option 1: Store Pattern Metadata
```rust
pub struct SignalNode {
    // ... existing fields ...

    /// Optional pattern metadata (for transforms)
    pattern_meta: Option<PatternMetadata>,
}

struct PatternMetadata {
    pattern: Pattern<String>,
    transforms: Vec<Transform>,
}
```

### Option 2: Defer Compilation
Don't compile patterns immediately - store as AST until needed:
```rust
enum BusValue {
    Compiled(NodeId),
    Deferred(Expr),  // Compile when referenced
}
```

### Option 3: Document Limitation
```phonon
# ❌ Doesn't work:
~drums: s "bd sn"
~fast: ~drums $ fast 2

# ✅ Works:
~fast: s "bd sn" $ fast 2
```

**Estimated effort**: 1-2 days (Option 1), 3-4 days (Option 2), 30 minutes (Option 3)
**Priority**: LOW (workaround is easy - apply transform when creating pattern)

---

## 🟢 60+ Tidal Operations Not Exposed

**Location**: `src/pattern.rs`, `src/pattern_ops.rs`, `src/pattern_ops_extended.rs`

**The Problem**:
From `TIDAL_OPERATORS_AUDIT.md`:
- 60+ pattern operations implemented in Rust
- Only ~10 exposed to DSL (fast, slow, rev, degrade, stutter, palindrome, stack)
- Missing: `cat`, `slowcat`, `jux`, `chop`, `striate`, `shuffle`, `scramble`, and 40+ more

**Recently Exposed**:
- ✅ `stack` - plays patterns simultaneously (2025-10-20)
- ✅ `cat` - concatenates patterns within each cycle (2025-10-20)
- ✅ `slowcat` - alternates between patterns on each cycle (2025-10-20)

**Next Priority (from audit)**:
1. **`jux`** - Stereo manipulation
2. **`chop`, `striate`** - Sample slicing
3. **`shuffle`, `scramble`** - Reordering
4. **`whenmod`, `every`** - Conditional transforms

**Why It's Easy**:
Same approach as `stack`:
1. Add function name to `compile_function_call` match
2. Write `compile_X` function
3. Call underlying `Pattern::X()` method
4. Write tests

**Example (cat combinator)**:
```rust
// In compile_function_call:
"cat" => compile_cat(ctx, args),

// New function:
fn compile_cat(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract list of patterns
    let exprs = match &args[0] {
        Expr::List(exprs) => exprs,
        _ => return Err("cat requires a list: cat [p1, p2, ...]".to_string()),
    };

    // ... similar to compile_stack ...
}
```

**Estimated effort**: 30 minutes per operation (once stack pattern is established)
**Priority**: MEDIUM (nice to have, not blocking)

---

## 🟢 Duplicate Code in Effect Compilers

**Location**: All effect compilers in `compositional_compiler.rs`

**The Problem**:
Every effect has nearly identical boilerplate:

```rust
fn compile_reverb(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    let (input_signal, room_size_expr, damping_expr, mix_expr) = if args.len() == 4 {
        // Chained: first arg is node ID
        if let Expr::Number(node_id) = &args[0] {
            let input_node = NodeId(*node_id as usize);
            (Signal::Node(input_node), &args[1], &args[2], &args[3])
        } else {
            // Standalone
            let input_node = compile_expr(ctx, args[0].clone())?;
            (Signal::Node(input_node), &args[1], &args[2], &args[3])
        }
    } else {
        return Err(format!("reverb requires 4 arguments, got {}", args.len()));
    };
    // ... compile parameters ...
}
```

**Better Approach**:
Generic helper function:

```rust
/// Extract chained input and remaining args
fn extract_chain_args(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<(Signal, Vec<Expr>), String> {
    if args.is_empty() {
        return Err("Function requires at least one argument".to_string());
    }

    match &args[0] {
        Expr::Number(node_id) => {
            // Chained input (hack - should be ChainInput variant)
            let input_node = NodeId(*node_id as usize);
            Ok((Signal::Node(input_node), args[1..].to_vec()))
        }
        _ => {
            // Standalone - first arg is input
            let input_node = compile_expr(ctx, args[0].clone())?;
            Ok((Signal::Node(input_node), args[1..].to_vec()))
        }
    }
}
```

Then effects become:
```rust
fn compile_reverb(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    let (input_signal, params) = extract_chain_args(ctx, args)?;

    if params.len() != 3 {
        return Err(format!("reverb requires 3 parameters (room_size, damping, mix), got {}", params.len()));
    }

    let room_node = compile_expr(ctx, params[0].clone())?;
    let damp_node = compile_expr(ctx, params[1].clone())?;
    let mix_node = compile_expr(ctx, params[2].clone())?;

    // ... create node ...
}
```

**Benefits**:
- Less duplication
- Easier to maintain
- Fixes chain hack in one place

**Estimated effort**: 2 hours
**Priority**: MEDIUM (quality of life, not blocking)

---

## 🟢 Effects Should Be Composable

**Current Limitation**:
Effects are hard-coded function names. Can't be passed around or composed dynamically.

**Vision**:
```phonon
# Define effect chains as values
~wet: reverb 0.5 0.7 0.3
~crunchy: distort 2.0 0.5

# Apply effect chains
~drums: s "bd sn"
~processed: ~drums # ~wet # ~crunchy
```

**Why It's Hard**:
- Effects need to be first-class values
- Would require partial application
- AST needs to represent "effect functions"

**Possible Solution**:
```rust
pub enum Expr {
    // ... existing variants ...

    /// Partial application of an effect
    PartialEffect { name: String, params: Vec<Expr> },
}
```

**Estimated effort**: 1 week (major feature)
**Priority**: LOW (interesting but not essential)

---

## Summary Table

| Issue | Location | Priority | Effort | Blocks |
|-------|----------|----------|--------|--------|
| **Chain operator hack** | compile_chain:758 | HIGH | 2-3h | Code quality |
| **Reverb args bug** | compile_reverb:374 | MEDIUM | 30min | Flexibility |
| **Sample parameters** | compile_s:166 | HIGH | 3-4 days | Per-voice control (but stack helps!) |
| **Transforms on buses** | compile_transform:795 | LOW | 1-2 days | Convenience |
| **Unexposed operations** | pattern*.rs | MEDIUM | 30min each | Tidal parity |
| **Duplicate effect code** | All effects | MEDIUM | 2h | Maintainability |
| **Non-composable effects** | N/A | LOW | 1 week | Advanced composition |

---

## Recommended Priority Order

### Phase 1: Fix Critical Kludges (1 day)
1. ✅ **Chain operator hack** - Make it type-safe with ChainInput variant
2. ✅ **Reverb args bug** - Fix the duplicated parameter
3. ✅ **Duplicate code** - Extract common effect pattern

### Phase 2: Expose Operations (2-3 days)
4. ✅ **cat/slowcat** - Essential combinators (DONE 2025-10-20)
5. **jux** - Stereo manipulation (NEXT)
6. **chop/striate** - Sample slicing
7. **10-15 more operations** - Systematic exposure

### Phase 3: Design Decisions (later)
8. **Sample parameters** - Decide on syntax (not urgent since stack works)
9. **Transforms on buses** - Evaluate if worth the complexity
10. **Composable effects** - Future vision

---

## Next Steps

**Immediate action**: Fix chain operator hack
- Low risk
- High impact on code quality
- Makes all future effects easier to write

**Then**: Systematically expose Tidal operations
- Follow stack pattern
- 30 minutes per operation
- Massive user-visible value

**Later**: Design sample parameter syntax
- Not urgent (stack solves immediate need)
- Needs careful thought
- Could use operator overloading or pattern modulation
