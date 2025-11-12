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

### ✅ P0.1: Bus chaining fixed (with limitation)
**Status**: FIXED (with documented workaround)
**Impact**: Outputs now mix correctly

**Problem**: When using buses in chains, signals were dropped and outputs didn't mix.

**Root Cause**: Buses are compiled once to NodeIds, can't be re-instantiated with new inputs.

**Fix**: Bus chain now returns left signal (pass-through) with warning.

**Working now**:
```phonon
o1: s "arpy"     -- Works
o2: s "bd*4"     -- Both outputs mix correctly
```

**Known Limitation**:
```phonon
~feel: delay 0.334 0.3 # reverb 0.9 0.1
o1: s "arpy" # ~feel    -- ⚠️ ~feel effect ignored, use direct instead
```

**Workaround**:
```phonon
o1: s "arpy" # delay 0.334 0.3 # reverb 0.9 0.1    -- Use effects directly
```

**Future Work**: Store bus expressions (Expr) not nodes (NodeId) to enable re-instantiation.

**File**: `src/compositional_compiler.rs` compile_chain()

---

### ✅ P0.2: stack multiplies volume instead of mixing
**Status**: FIXED
**Impact**: HIGH - Was causing distortion/clipping, now fixed

**Problem**: `stack` was adding signals without normalization, causing volume multiplication.

**Example that was broken**:
```phonon
o2: stack [
  s "bd(<4 4 3>,8)",      -- Each pattern is loud
  s "~ cp" $ fast 2       -- Stacking made it LOUDER
]
-- Result: Severe clipping/distortion (Peak: 2.825)
```

**Fix**: Modified Mix node to normalize by dividing sum by N.

**Results**:
- Before: 2 patterns → RMS 0.901, Peak 2.825 (2.5x multiplication) ⚠️
- After: 2 patterns → RMS 0.450, Peak 1.413 (proper mixing) ✅
- 4 patterns: RMS 0.463, Peak 1.414 (stable, no multiplication) ✅

**Files**:
- `src/unified_graph.rs`: Mix node now normalizes (line 4849)
- `src/compositional_compiler.rs`: stack uses Mix node (line 1143)

---

### ✅ P0.3: Output volume affected by other outputs
**Status**: FIXED
**Impact**: HIGH - Was causing outputs to contaminate each other, now fixed

**Problem**: All outputs were returning the same mixed voice signal, so disabling one output changed volume of others.

**Root Cause**: Voice manager processed all voices once and returned a single global mix. ALL Sample nodes returned this same mix regardless of which output they belonged to.

**Example that was broken**:
```phonon
o1: s "bd*4"  -- Should only hear bd
o2: s "sn*4"  -- Should only hear sn
-- But both outputs returned the SAME mix (bd+sn)!
```

**Fix**: Tag voices with source node ID and return per-node mixes.
1. Added `source_node` field to Voice
2. Added `default_source_node` to VoiceManager (set before triggering)
3. Changed voice processing to return `HashMap<usize, f32>` (node → mix)
4. Sample nodes look up their node ID in the HashMap

**Results**:
- Before: o1 single RMS = 0.354, o1 dual RMS = 0.450 (contaminated) ⚠️
- After: o1 single RMS = 0.354, o1 dual RMS = 0.354 (independent) ✅
- o2 has different RMS (0.301) as expected for different samples ✅

**Files**:
- `src/voice_manager.rs`: Added source_node field and per-node processing
- `src/unified_graph.rs`: Process per-node, set default_source_node before Sample evaluation

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
