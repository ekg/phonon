# Reversal and Transform Composition in Phonon

## Current State (What Works Now)

### Pattern Event Reversal (`rev`)

The `rev` transform reverses the **order of pattern events** within a cycle:

```phonon
-- Normal: bd, sn, hh, cp (in that order)
~drums: s "bd sn hh cp"

-- Reversed: cp, hh, sn, bd (reverse order)
~drums_rev: s "bd sn hh cp" $ rev

-- Chained with other transforms
~drums: s "bd sn hh cp" $ fast 2 $ rev
```

**What it does:**
- Takes events `[e1, e2, e3, e4]`
- Returns `[e4, e3, e2, e1]`
- Each sample still plays forward
- Only the triggering order changes

## Future Possibilities

### 1. Audio Reversal (Tape Reverse)

Play individual samples backwards (like hitting "reverse" on a tape):

```phonon
-- Hypothetical syntax option 1: Effect-style
~vocal: s "vocal" # reverse

-- Option 2: Pattern parameter
~vocal: s "vocal" $ reverse_samples

-- Option 3: Mini-notation flag
~vocal: s "vocal<r>"  -- <r> flag means reverse playback
```

**Implementation considerations:**
- Requires reading sample backwards from end to start
- Simple: just reverse the sample buffer
- Per-sample control: `s "bd sn<r> hh cp<r>"` (reverse sn and cp only)
- Could combine with pattern reversal: `s "bd sn hh cp" $ rev # reverse` (reverse order AND playback)

**Use cases:**
- Reverse cymbal swells
- Reverse vocals/speech
- Reverse snare (classic 80s sound)
- Creating "sucking" effects

### 2. Time-Stretch Reversal

Reverse the audio over time (different from sample reversal):

```phonon
-- Play the entire pattern backwards over time
~drums: s "bd sn hh cp" $ time_reverse
```

**What this would do:**
- Pattern plays forward in code
- But audio output is reversed in time
- Like recording the pattern, then playing the recording backwards
- Events happen in reverse temporal order

**Complex example:**
```phonon
-- These would sound different:
~a: s "bd sn hh cp" $ rev           -- Events in reverse order, samples forward
~b: s "bd sn hh cp" # reverse       -- Events forward, samples backwards
~c: s "bd sn hh cp" $ time_reverse  -- Entire temporal sequence reversed
```

### 3. Pattern vs Audio Reversal Matrix

| Transform | Event Order | Sample Playback | Temporal Direction |
|-----------|-------------|-----------------|-------------------|
| `rev` (current) | Reversed | Forward | Forward |
| `# reverse` (future) | Forward | Reversed | Forward |
| `$ rev # reverse` | Reversed | Reversed | Forward |
| `$ time_reverse` | Reversed | Forward | Reversed |

## Your Question: Transform Composition with Buses

**You asked:**
> Could I do `~r: rev` then `out: s "x" $ ~r` ??

**Current answer: No, but it's a brilliant idea!**

### Why It Doesn't Work Now

In current Phonon:
- `~` buses hold **signals** (audio)
- `$` applies **transforms** (functions)
- Transforms aren't first-class values you can store

```phonon
-- This works (bus holds audio signal):
~drums: s "bd sn hh cp"
out: ~drums * 0.5

-- This doesn't work (can't store transform in bus):
~r: rev  -- ❌ ERROR: rev is a transform, not a signal
out: s "x" $ ~r
```

### Why It's Interesting

Treating transforms as first-class values would enable:

1. **Transform Reuse:**
```phonon
-- Define transform once, apply many times
~myTransform: rev $ fast 2 $ every 4 (fast 2)

~drums: s "bd sn" $ ~myTransform
~bass: saw "55" $ ~myTransform
~synth: sine "220" $ ~myTransform
```

2. **Transform Composition:**
```phonon
-- Build complex transforms from simpler ones
~speedup: fast 2
~reverse: rev
~combo: ~speedup $ ~reverse

out: s "bd sn hh cp" $ ~combo
```

3. **Higher-Order Patterns:**
```phonon
-- Pattern of transforms!
~transforms: seq [rev, fast 2, slow 2]
out: s "bd sn" $ choose ~transforms  -- Randomly pick transform each cycle
```

## Proposed Syntax: `@` for Transform Buses

Since `~` is for signals and we need something for transforms, let's use `@`:

```phonon
-- @ holds transforms (pattern functions)
@speedup: fast 2
@reverse: rev
@complex: fast 2 $ rev $ every 4 (slow 2)

-- Apply stored transforms
~drums: s "bd sn hh cp" $ @speedup
~reversed: s "vocal" $ @reverse

-- Compose transforms
@combo: @speedup $ @reverse
out: s "x" $ @combo
```

### Alternative: Function Definition Syntax

More explicit function-like syntax:

```phonon
-- Define reusable transforms
transform glitch = fast 4 $ every 2 (rev)
transform stutter = fast 8 $ every 4 (slow 2)

-- Apply them
~drums: s "bd sn" $ glitch
~synth: sine "220" $ stutter $ glitch
```

## Implementation Considerations

### Easy (Low-Hanging Fruit)

**1. Sample Reversal (`# reverse`)**
```rust
// In unified_graph.rs
SignalNode::Reverse { input: Signal }

// In eval_node()
SignalNode::Reverse { input } => {
    // Just play sample buffer backwards
    // Already have sample buffers loaded
    // Simple index reversal
}
```

Estimated effort: **4 hours**
- Add `SignalNode::Reverse`
- Implement backwards buffer reading
- Add parser support for `# reverse`
- Test with samples
- Document in KEYWORD_ARGUMENTS.md

**2. Per-Sample Reverse in Mini-Notation**
```phonon
s "bd sn<r> hh cp<r>"  -- <r> flag means reverse
```

Estimated effort: **6 hours**
- Extend mini-notation parser for `<r>` flag
- Pass reverse flag through pattern events
- Modify sample triggering to respect flag
- Test with various patterns
- Document syntax

### Medium (Requires Design)

**3. Pattern Event Reversal (already works!)**
✅ Already implemented as `rev` transform

**4. Time-Stretch Reversal**
```phonon
~drums: s "bd sn hh cp" $ time_reverse
```

Estimated effort: **20 hours**
- Buffer entire pattern cycle
- Reverse time axis
- Maintain event timing
- Handle cycle boundaries
- Test thoroughly

### Hard (Language Design)

**5. First-Class Transforms (`@` syntax)**

Requires fundamental language changes:
- Transforms as values (not just syntax)
- Type system to distinguish signals from transforms
- Transform composition semantics
- Storage and application mechanics

Estimated effort: **40+ hours**
- Design type system
- Modify parser for `@` syntax
- Implement transform storage
- Implement transform application
- Extensive testing
- Update all documentation

## Recommendations

### Phase 1: Quick Wins (Sample Reversal)

Implement `# reverse` effect first:

```phonon
-- Individual sample reversal
~vocal: s "vocal" # reverse

-- Combined with pattern reversal
~drums: s "bd sn hh cp" $ rev # reverse  -- Reversed order AND reversed samples

-- Per-sample control
~drums: s "bd sn<r> hh cp"  -- Only reverse sn
```

**Why start here:**
- Simple implementation
- Immediate musical utility
- Doesn't require language changes
- Tests infrastructure for future features

### Phase 2: Transform Libraries (No New Syntax)

Use code organization instead of language features:

```phonon
-- In a file: transforms/glitch.ph
-- Common transform patterns
-- Users copy-paste or include

-- glitch transform pattern
-- Usage: s "bd sn" $ fast 4 $ every 2 (rev)
```

**Why this works:**
- No language changes needed
- Users can share transform recipes
- Good enough for most use cases
- Establishes patterns before formalizing

### Phase 3: First-Class Transforms (If Needed)

Only implement `@` syntax if:
1. Phase 2 proves insufficient
2. Community strongly requests it
3. Clear use cases emerge

**Why wait:**
- Major language change
- Increases complexity
- May not be needed
- Better to evolve naturally

## Musical Examples

### Reverse Cymbal Swell

```phonon
-- Classic reverse cymbal (future):
~swell: s "cymbal<r>" # delay 0.5 :feedback 0.9
```

### Reverse Vocals

```phonon
-- Mysterious reverse speech:
~vocal: s "speech" # reverse # reverb 0.9 0.5
```

### Pattern + Sample Reversal

```phonon
-- Both reversed (very disorienting):
~drums: s "bd sn hh cp" $ rev # reverse
```

### Glitch Stutters

```phonon
-- Reverse every 4th sample:
~glitch: s "vocal*16" $ every 4 (# reverse)
```

## Open Questions

1. **Should reverse be per-sample or per-pattern?**
   - Per-sample: `s "bd<r> sn"`
   - Per-pattern: `s "bd sn" # reverse`
   - Both?

2. **How does reverse interact with other effects?**
   - `# reverse # delay` vs `# delay # reverse`
   - Order matters for some effects
   - Need clear semantics

3. **Do we need time_reverse if we have pattern + sample reverse?**
   - `$ rev # reverse` gives similar effect
   - But time_reverse has different semantics
   - Maybe not needed?

4. **Should transforms be first-class?**
   - Powerful but complex
   - Start with libraries instead?
   - Wait for community feedback

## Next Steps

1. ✅ Document all reversal concepts (this doc)
2. Implement `# reverse` effect (sample reversal)
3. Add `<r>` mini-notation flag
4. Test with musical examples
5. Gather user feedback
6. Decide on transform composition based on real usage

## Summary

**Currently Available:**
- Pattern event reversal: `$ rev`

**Easy to Add (Hours):**
- Sample playback reversal: `# reverse`
- Per-sample reverse flag: `s "bd<r>"`

**Moderate Effort (Days):**
- Time-stretch reversal: `$ time_reverse`

**Major Undertaking (Weeks):**
- First-class transforms: `@name: transform`

**Recommendation:**
Start with sample reversal (`# reverse` and `<r>` flag). It's musically powerful, technically simple, and doesn't require language changes. The transform composition idea is brilliant but should wait until we see if simpler approaches suffice.

---

**Your specific question:**
> Could I do `~r: rev` then `out: s "x" $ ~r` ??

Not currently, because `~` is for signals, not transforms. But we could add `@` for transforms, or use a `transform` keyword. The question is: do we need it, or can we get 90% of the benefits with simpler approaches like effect chains and copy-paste transform patterns?

Let's start with the easy stuff and see what users actually need!
