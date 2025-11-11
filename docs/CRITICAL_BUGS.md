# Critical Bugs - MUST FIX IMMEDIATELY

**Last updated**: 2025-11-10

These are **blocking issues** that prevent Phonon from being usable in production.

---

## P0 - SHOWSTOPPERS (Fix First)

### 🔴 P0.0: ALL parameters must accept patterns, not just numbers
**Status**: ARCHITECTURAL ISSUE
**Impact**: CRITICAL - Breaks fundamental design principle

**Problem**: Many transforms and effects only accept bare numbers instead of patterns.

**Examples that should work but don't**:
```phonon
s "bd" $ fast "2 3 4"           -- fast should accept pattern
s "bd" # lpf "500 2000" 0.8     -- cutoff should be pattern
s "bd" # delay "0.25 0.5" 0.3   -- delay time should be pattern
s "bd" $ loopAt "1 2 4"         -- DOES work (just fixed)
```

**What needs fixing**:
- Review EVERY transform parameter
- Review EVERY effect parameter
- Change from `extract_number()` to pattern compilation
- Use pattern query at sample time for continuous control

**Scope**: This affects dozens of functions across:
- `src/compositional_compiler.rs` (all `extract_number()` calls)
- `src/pattern_ops_extended.rs` (methods taking `f64`)
- `src/unified_graph.rs` (effect parameters)

---

### 🔴 P0.1: Delay is not bus-specific and not chainable
**Status**: BROKEN
**Impact**: HIGH - Can't use delay on specific buses

**Problem**: Delay applies globally, can't be chained with `#` operator.

**Expected**:
```phonon
~feel: delay 0.334 0.3 # reverb 0.9 0.1  -- Chain delay + reverb on bus
o1: s "arpy" # ~feel                      -- Apply to specific pattern
o2: s "bd*4"                              -- Should NOT have delay
```

**Current behavior**: Delay applies to all outputs or doesn't chain properly.

**Fix needed**:
- Delay must be a signal node that can be part of bus chain
- Should work like `# lpf 2000 0.8 # reverb 0.5 0.5`

**File**: `src/compositional_compiler.rs` delay compilation

---

### 🔴 P0.2: stack multiplies volume instead of mixing
**Status**: BROKEN
**Impact**: HIGH - Stacking patterns creates distortion/clipping

**Problem**: `stack` adds signals without normalization, causing volume multiplication.

**Example**:
```phonon
o2: stack [
  s "bd(<4 4 3>,8)",      -- Each pattern is loud
  s "~ cp" $ fast 2       -- Stacking makes it LOUDER
]
-- Result: Clipping/distortion instead of mix
```

**Expected**: stack should mix signals (divide by N or use proper mixing).

**Current**: Signals are added directly, causing 2x, 3x, 4x volume increase.

**Fix needed**:
- Stack should normalize by dividing by pattern count
- OR use proper mixing algorithm (RMS-based)
- Must prevent clipping

**File**: `src/pattern.rs` stack implementation, `src/unified_graph.rs` mixing

---

### 🔴 P0.3: Output volume affected by other outputs
**Status**: BROKEN
**Impact**: HIGH - Unpredictable mixing behavior

**Problem**: Disabling one output changes volume of other outputs.

**Example**:
```phonon
o1: s "arpy(7,17)" # note "c4'min7"
o2: s "kick(4,17)"
-- Commenting out o2 makes o1 QUIETER
```

**This makes no sense**: Each output should have independent volume.

**Hypothesis**: Auto-routing mixer is incorrectly normalizing or cross-affecting outputs.

**Fix needed**:
- Each output (`o1`, `o2`, etc.) should have fixed independent gain
- Auto-routing mixer must not change individual output levels
- Investigate `src/unified_graph.rs` auto-routing logic

---

### 🔴 P0.4: Multi-threading not working / poor performance
**Status**: BROKEN
**Impact**: HIGH - Can't use multiple CPUs, choppy playback

**Problem**:
- Thread count not respected
- Choppy rendering despite low CPU usage (30%)
- Available CPUs not utilized
- Appears to be scheduling/synchronization issue

**Observed**:
- Can run on 8 CPUs but performs same as 1 CPU
- Rendering is choppy/stuttery
- CPU usage stays around 30% instead of maxing out
- Suggests thread starvation or bad synchronization

**Fix needed**:
- Audit threading model in audio engine
- Check for locks/mutexes causing contention
- Profile to find bottlenecks
- May need to redesign real-time audio thread architecture

**Files**:
- `src/main.rs` (live mode audio thread)
- `src/unified_graph.rs` (rendering)
- `src/voice_manager.rs` (voice allocation)

---

## P1 - HIGH PRIORITY (Fix Soon)

### 🟠 P1.1: fast should speed up cycles, not just density
**Status**: DESIGN ISSUE
**Impact**: MEDIUM - Confusing behavior vs Tidal

**Problem**: `fast 3` speeds up pattern density but NOT playback tempo.

**Expected (Tidal behavior)**:
```phonon
setcps 2.0         -- 2 cycles per second (120 BPM)
fast 3             -- Should speed up to 6 cycles per second (360 BPM)
```

**Current (Phonon behavior)**:
```phonon
tempo: 2.0         -- 2 cycles per second
fast 3             -- Just makes 3x more events, SAME tempo
```

**Decision needed**:
- Should Phonon match Tidal's behavior?
- OR is this intentional design difference?
- Affects lots of existing code if changed

**Impact**: This might be a breaking change. Discuss before fixing.

---

### 🟠 P1.2: ar (attack/release envelope) doesn't exist
**Status**: MISSING FEATURE
**Impact**: MEDIUM - Can't control envelopes easily

**Problem**: User tried `# ar 0.1 0.9` but `ar` is not implemented.

**Expected**:
```phonon
s "arpy" # ar 0.1 0.9  -- Attack 0.1, Release 0.9
```

**Current**:
- Must use `# attack 0.1 # release 0.9` (verbose)
- No shorthand `ar` parameter

**Fix needed**:
- Add `ar` as shorthand for attack + release
- Common in Tidal/SuperCollider

**File**: `src/compositional_compiler.rs` DSP parameter compilation

---

### 🟠 P1.3: Can't render in live mode, processes at 30% CPU
**Status**: PERFORMANCE BUG
**Impact**: MEDIUM - Live mode unusable

**Problem**: Live mode stutters/can't keep up, only uses 30% CPU.

**Symptoms**:
- Audio dropouts/glitches in live mode
- CPU usage around 30% (should be higher if maxed out)
- Render mode works fine
- Suggests real-time scheduling issues

**Related to**: P0.4 (multi-threading issue)

**Fix needed**: Profile and optimize live mode audio callback.

---

## Testing Priority

1. **P0.0** - Pattern parameters (most fundamental)
2. **P0.2** - Stack volume (audio quality)
3. **P0.3** - Output independence (audio quality)
4. **P0.1** - Delay chaining (core feature)
5. **P0.4** - Performance (usability)

## Notes

- Many of these are interconnected (volume issues, mixing, threading)
- Pattern parameters (P0.0) is the most fundamental architectural issue
- Performance issues (P0.4, P1.3) may share root cause
- Volume issues (P0.2, P0.3) likely related to auto-routing mixer

## Action Plan

1. **Create test cases** for each issue in `broke.ph.*` files
2. **Fix P0.0** first (pattern parameters) - enables everything else
3. **Fix volume/mixing** issues (P0.2, P0.3) - critical for audio quality
4. **Fix delay** chaining (P0.1) - needed for effects
5. **Profile and fix** performance (P0.4, P1.3) - usability
6. **Add missing features** (P1.1, P1.2) - nice to have
