# Phonon: Current State & Design Direction

**Last Updated**: 2025-10-04

## Executive Summary

Phonon is at a **crossroads** between three possible directions:

1. **Tidal-like** (Haskell-based, event-based, pattern-centric)
2. **Strudel-like** (JavaScript-based, web-friendly, Tidal port)
3. **Phonon-native** (Rust-based, signal-graph-centric, hybrid approach)

**Current Status**: We have a working **hybrid system** that differs from both Tidal and Strudel in important ways.

---

## What Phonon Actually Is (Right Now)

### Architecture: Unified Signal Graph

Phonon uses a **sample-by-sample signal processing graph** where everything (patterns, synthesis, samples) flows through the same graph:

```
Pattern Nodes → Sample Nodes → Filter Nodes → Output Node
   ↓               ↓              ↓              ↓
Eval every     Trigger voices  Process audio  Mix & output
sample         at events       at sample rate at sample rate
```

This is **fundamentally different** from Tidal/Strudel:

| System | Architecture | Update Rate | Philosophy |
|--------|--------------|-------------|------------|
| **Tidal** | Event-based pattern engine + SuperDirt | Pattern evaluates per-cycle, synths spawned per-event | "Patterns control everything" |
| **Strudel** | Event-based pattern engine + WebAudio | Same as Tidal, JS implementation | "Tidal in the browser" |
| **Phonon** | Unified signal graph (sample-by-sample) | Everything evaluates at sample rate (44.1kHz) | "Patterns ARE signals" |

### What Works Right Now

✅ **Pattern System**:
- Mini-notation parsing: `"bd sn cp hh"`
- Euclidean rhythms: `"bd(3,8)"`
- Alternation: `"bd <sn cp>"`
- Concatenation: `"[bd sn] hh"`
- Multiplication: `"bd*4"`

✅ **Voice-Based Sample Playback**:
- 64-voice polyphonic engine
- Samples can overlap (multiple triggers)
- Sample bank loading from `samples/` directory

✅ **Pattern-to-Synthesis Integration** (NEW!):
- Patterns can control oscillator frequency: `sine("110 220")`
- Patterns can control filter cutoff: `saw(55) # lpf("500 2000", 0.8)`
- Patterns hold values between triggers (just fixed!)
- Patterns can be mixed/multiplied: `~osc * "0.5 1.0"`

✅ **Samples Through Effects** (NEW!):
- Sample audio flows through signal graph
- Can filter samples: `s("bd sn") # lpf(2000, 0.8)`
- Can apply math: `s("bd") * 0.5`

✅ **Synthesis**:
- Oscillators: `sine(440)`, `saw(110)`, `square(220)`, `noise`
- Filters: `lpf(cutoff, q)`, `hpf(cutoff, q)`
- Signal math: `~a + ~b`, `~osc * 0.5`
- Bus system: `~lfo = sine(0.5)`

✅ **Live Coding**:
- `phonon live x.ph` - watches file for changes
- Auto-reload on save
- Real-time audio output

### What Doesn't Work / Isn't Implemented

❌ **Pattern Transformations**:
- No `$` operator (docs describe it, not implemented)
- No `fast`, `slow`, `rev`, `every`, etc.
- No transformation pipeline

❌ **Multi-Output System**:
- Only one output: `out`
- No `out1`, `out2`, `out3`, etc.
- No `hush` or `panic` commands

❌ **Sample Selection** (`# n` equivalent):
- Can't select from numbered sample banks
- No `s("bd:0 bd:1 bd:2")`
- No `s("bd") # n("0 1 2 3")`

❌ **Tidal DSP Parameters**:
- No `gain`, `pan`, `speed`, `cut`, `crush`, etc. as pattern operators
- These would need to be implemented as per-voice controls

❌ **Parser Inconsistency**:
- Docs say `:` for assignment
- Code uses `=` for assignment
- Some examples use both

---

## The Three Paths Forward

### Path 1: Embrace Tidal Compatibility

**Goal**: Make Phonon a Rust implementation of Tidal syntax

**Pros**:
- Huge community, lots of documentation
- People can copy/paste Tidal code
- Clear design direction (follow Tidal)

**Cons**:
- Tidal is tightly coupled to SuperDirt (SuperCollider)
- Event-based architecture doesn't fit Phonon's signal graph
- Would require rewriting most of the current codebase
- Haskell semantics are hard to replicate in Rust

**Implementation**:
```haskell
-- Target syntax (Tidal)
d1 $ sound "bd sn" # gain "0.8 1.0" # lpf "500 2000"
d2 $ s "hh*16" # pan (sine 0.25)
```

### Path 2: Follow Strudel Format

**Goal**: JavaScript-style syntax with Tidal semantics

**Pros**:
- More familiar to JS developers
- Strudel has modernized some Tidal rough edges
- Already ported to JS, easier to reference

**Cons**:
- Still event-based architecture
- JS-specific idioms (chaining, mini-notation in quotes)
- Doesn't leverage Rust's strengths

**Implementation**:
```javascript
// Target syntax (Strudel)
s("bd sn").gain("0.8 1.0").lpf("500 2000")
s("hh*16").pan(sine.slow(4))
```

### Path 3: Phonon Native (Hybrid Approach)

**Goal**: Lean into the signal-graph architecture, patterns as signals

**Pros**:
- Leverages existing architecture
- Unique approach: patterns ARE control signals
- Native Rust idioms
- Can do things Tidal/Strudel can't (modulate anything with patterns)
- Simple syntax: `out = sine("110 220") * 0.2`

**Cons**:
- Not Tidal-compatible (can't copy/paste code)
- Smaller community (you're building it)
- Need to document everything ourselves
- Missing pattern transformations

**Implementation**:
```rust
// Current/target syntax (Phonon native)
tempo 2.0

~kick = s("bd(3,8)")
~bass = saw("55 82.5 110") # lpf("500 1000", 0.8)
~lfo = sine(0.25)

out = (~kick + ~bass * 0.5) # hpf(~lfo * 500 + 200, 0.7)
```

---

## Recommendation: Path 3 (Phonon Native) + Tidal Inspiration

**Why**: You already have a unique, working architecture. Don't throw it away.

### What Makes Phonon Special

1. **Patterns are first-class control signals**:
   ```
   out = sine("110 220 440") * 0.2  # Pattern controls frequency at sample rate
   ```
   Tidal can't do this - patterns trigger discrete events, can't modulate synthesis parameters continuously.

2. **Everything flows through one graph**:
   ```
   out = s("bd sn") # lpf(2000, 0.8)  # Samples through filters, just like synthesis
   ```
   Tidal has separation between pattern engine and audio engine.

3. **Simple, Rust-native syntax**:
   ```
   ~lfo = sine(0.5)
   out = saw(55) # lpf(~lfo * 2000 + 500, 0.8)
   ```
   No Haskell `$`, no JS chaining, just assignments and operators.

### What to Add (Priority Order)

1. **Multi-output system** (HIGH PRIORITY):
   ```
   out1 = s("bd sn")
   out2 = s("hh*16")
   out3 = saw("55 82.5")

   hush         # Silence all outputs
   hush 1       # Silence out1
   panic        # Kill all voices + outputs
   ```

2. **Sample bank selection** (HIGH PRIORITY):
   ```
   s("bd:0 bd:1 bd:2")           # Sample number inline
   s("bd", "0 1 2 3")            # Pattern for sample number
   ```

3. **Basic pattern transformations** (MEDIUM PRIORITY):
   Keep it simple - not all of Tidal, just the essentials:
   ```
   "bd sn" $ fast(2)           # Speed up
   "bd sn" $ rev               # Reverse
   "bd sn" $ every(4, rev)     # Conditional
   ```

4. **Pattern DSP parameters** (MEDIUM PRIORITY):
   Make these work like Tidal but in signal graph:
   ```
   s("bd sn", gain="0.8 1.0", pan="0 1")
   ```

5. **Better docs** (HIGH PRIORITY):
   - Update PHONON_LANGUAGE_REFERENCE.md to match reality
   - Add quick-start guide
   - Add comparison to Tidal/Strudel
   - Document what works, what doesn't

---

## Current Syntax (What Actually Works)

### File Structure
```phonon
# Comments with #
tempo 2.0              # Set cycles per second

# Bus assignment with =
~name = expression

# Output (exactly one required)
out = expression
```

### Patterns
```phonon
"bd sn"                # Mini-notation string
"bd*4"                 # Repetition
"bd sn . cp"           # Rests
"[bd sn] hh"           # Grouping
"bd <sn cp>"           # Alternation
"bd(3,8)"              # Euclidean
```

### Synthesis
```phonon
sine(freq)             # Oscillators
saw(freq)
square(freq)
noise                  # Noise

sine("110 220")        # Pattern-controlled frequency!
```

### Samples
```phonon
s("bd sn")             # Sample playback
s("bd*4")              # Patterns work
```

### Filters & Effects
```phonon
lpf(cutoff, q)         # Low-pass filter
hpf(cutoff, q)         # High-pass filter

# Patterns control filter params!
~osc # lpf("500 2000", 0.8)
```

### Signal Math
```phonon
~a + ~b                # Add signals
~a * 0.5               # Scale signal
~osc * "0.5 1.0"       # Pattern modulation
```

### Chain Operator
```phonon
~chain = saw(55) # lpf(1000, 0.8)   # Signal flow
```

---

## Next Steps (Concrete Actions)

### 1. Fix Documentation (IMMEDIATE)
- [ ] Update PHONON_LANGUAGE_REFERENCE.md to match actual syntax
- [ ] Remove `$` operator references (not implemented)
- [ ] Change `:` to `=` in all examples
- [ ] Add "What Works vs. What's Planned" section

### 2. Implement Multi-Output (THIS WEEK)
- [ ] Add `out1..outN` support in parser
- [ ] Add `hush` keyword (silence all)
- [ ] Add `hush N` (silence specific output)
- [ ] Add `panic` keyword (kill voices + silence)

### 3. Implement Sample Selection (THIS WEEK)
- [ ] Add `s("bd:0 bd:1")` syntax
- [ ] Add `s("bd", "0 1 2")` two-arg form
- [ ] Update sample loader to support numbered banks

### 4. Pattern Transformations (NEXT SPRINT)
Start with just 5 essential ones:
- [ ] `|> fast(n)` - speed up
- [ ] `|> slow(n)` - slow down
- [ ] `|> rev` - reverse
- [ ] `|> every(n, fn)` - conditional
- [ ] `|> rotate(n)` - shift

Don't try to implement all of Tidal. Pick the 80/20.

---

## Decision Point

**Question**: Which path do you want to take?

1. **Tidal-compatible** - Rewrite to match Tidal syntax/semantics exactly
2. **Strudel-compatible** - Use JS-style syntax with Tidal semantics
3. **Phonon-native** - Keep unique approach, add missing features incrementally

**Recommendation**: Path 3 (Phonon native)

You have something **unique and powerful** here:
- Patterns that modulate synthesis in real-time
- Simple Rust-native syntax
- Fast, sample-accurate control

Don't throw that away to chase Tidal compatibility. Instead:
- Fix the docs to match reality
- Add multi-output + hush/panic
- Add sample selection
- Add 5-10 essential pattern transformations
- Document what makes Phonon different

You can always add more Tidal features later if needed. But the core architecture - **patterns as control signals** - is novel and worth exploring.

---

## Questions to Answer

1. Do you want Tidal compatibility, or are you OK with Phonon being its own thing?
2. Is `=` vs `:` for assignment important to you?
3. Do you want `#` for signal flow, or would you prefer something else?
4. Should patterns use `$` for transformations, or stick with signal-graph operators?
5. How many outputs do you need? (4? 8? 16?)

Let me know your preferences and I'll update the docs + implement the features accordingly!
