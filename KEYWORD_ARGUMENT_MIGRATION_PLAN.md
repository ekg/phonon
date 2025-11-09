# Keyword Argument Migration Plan

## Current Status

### Functions WITH Keyword Support ✅
- `adsr`, `ad`, `asr` - Envelopes with optional parameters
- `lpf`, `hpf`, `bpf`, `notch` - Filters with optional Q parameter
- `reverb`, `chorus`, `delay`, `distortion` - Effects with optional mix parameters

### Functions WITHOUT Keyword Support ❌
- **Sample Modifiers**: `gain`, `pan`, `speed`, `begin`, `end`, `attack`, `release`, `cut`, `n`, `note`
- **Oscillators**: `sine`, `saw`, `square`, `triangle`, `noise`

## Decision: What Needs Keywords?

### ✅ High Priority - ADD Keyword Support
**Oscillators** - These should support keyword arguments for future extensibility:
- `sine :freq 440 :phase 0`
- `saw :freq 110 :detune 0.1`
- Reason: Oscillators will likely gain optional parameters (phase, detune, etc.)

### 🟡 Medium Priority - OPTIONAL
**Multi-parameter modifiers** (if they get optional params):
- `attack` / `release` could have `:curve linear|exp|log`
- Only implement if/when optional parameters are added

### ❌ Low Priority - KEEP Positional Only
**Single-parameter modifiers**:
- `gain`, `pan`, `speed`, `begin`, `end`, `cut`, `n`, `note`
- Reason: Only one parameter each, keyword would be redundant
- Example: `# gain :amount 0.8` is more verbose than `# gain 0.8`
- Keep these as positional-only for ergonomics

## Implementation Plan

### Phase 1: Oscillators (High Priority)

Convert oscillators to use `ParamExtractor` for keyword support:

**Files to modify:**
- `src/compositional_compiler.rs` - Update compile_sine, compile_saw, etc.

**Before:**
```rust
fn compile_sine(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 1 {
        return Err("sine requires 1 argument (frequency)".to_string());
    }
    let freq_node = compile_expr(ctx, args[0].clone())?;
    // ...
}
```

**After:**
```rust
fn compile_sine(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    let extractor = ParamExtractor::new(args);

    // Required parameter
    let freq_expr = extractor.get_required(0, "freq")?;
    let freq_node = compile_expr(ctx, freq_expr)?;

    // Future: Optional parameters
    // let phase_expr = extractor.get_optional(1, "phase", 0.0);

    // ...
}
```

**Syntax enabled:**
```phonon
-- Both work:
~osc1: sine 440
~osc2: sine :freq 440

-- Future (when optional params added):
~osc3: sine :freq 440 :phase 0.5
```

**Estimated effort:** 2-3 hours
- Update 4 oscillator functions
- Test positional and keyword syntax
- Update documentation

### Phase 2: Update Documentation

Update docs to reflect keyword support status:

**Files:**
- `docs/KEYWORD_ARGUMENTS.md` - Add oscillator section
- `src/modal_editor/completion/function_metadata.rs` - Add oscillator metadata

**Estimated effort:** 1 hour

### Phase 3: Future - Modifiers with Optional Params

Only if/when we add optional parameters to modifiers:

**Candidates:**
- `attack :time 0.01 :curve exp` - envelope curve shape
- `release :time 0.2 :curve log`
- `speed :rate 2.0 :interpolation cubic` - sample interpolation

**Implementation:** Same ParamExtractor pattern as oscillators

**Estimated effort:** 1-2 hours per modifier when needed

## Summary

### Immediate Action (This Session)
1. ✅ Convert oscillators to use ParamExtractor
2. ✅ Test both positional and keyword syntax
3. ✅ Update documentation

### Keep As-Is
- Sample modifiers with single parameters (gain, pan, speed, begin, end, cut)
- Reason: No benefit to keyword syntax for single parameters

### Future (As Needed)
- Add keyword support to modifiers only when adding optional parameters
- Follow the same ParamExtractor pattern

## Testing Requirements

For each converted function, test:
1. Positional syntax still works: `sine 440`
2. Keyword syntax works: `sine :freq 440`
3. Mixed syntax works (when optional params added)
4. Error messages are clear for missing required params

## Total Effort Estimate

- Phase 1 (Oscillators): **2-3 hours**
- Phase 2 (Documentation): **1 hour**
- **Total**: 3-4 hours

## Rationale

**Why not convert all modifiers?**
- Single-parameter modifiers get no benefit from keywords
- `# gain :amount 0.8` is more verbose than `# gain 0.8`
- Keeps live-coding syntax concise
- Can always add keywords later if needed

**Why convert oscillators?**
- Likely to gain optional parameters (phase, detune, pulse width, etc.)
- Establishes pattern for future UGen development
- Makes API more consistent with effects/filters
- No ergonomic cost (same keystrokes for basic use)

## Decision: Proceed?

**Recommendation:** YES - Convert oscillators now (Phase 1 + 2)
- Small scope (4 functions)
- Clear benefit (future extensibility)
- Low risk (backwards compatible)
- 3-4 hours total

**Hold off on:** Converting single-parameter modifiers
- No clear benefit
- Can revisit if optional parameters are added
