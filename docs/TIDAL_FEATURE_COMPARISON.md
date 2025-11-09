# TidalCycles vs Phonon: Feature Comparison

## Executive Summary

**Yes, Phonon is missing several sample manipulation features from Tidal!**

The most important ones to add:
1. ✅ **`speed` parameter** (including negative for reverse) - **COMPLETE**
2. ✅ **`gain` and `pan` parameters** - **COMPLETE**
3. ✅ **`begin` and `end` parameters** (sample slice points) - **COMPLETE**
4. **`cut` parameter** (stop previous samples) - MEDIUM
5. **Granular functions** (`chop`, `striate`, `slice`) - MEDIUM
6. **More effects** (we have some, missing many) - ONGOING

---

## Sample Playback Parameters

### Implemented in Phonon ✅

| Parameter | What It Does | Example | Status |
|-----------|--------------|---------|--------|
| **`speed`** | Playback speed/pitch. **Negative = reverse!** | `# speed "-1"` plays backwards | ✅ **COMPLETE** |
| **`gain`** | Sample volume/amplitude | `# gain "0.8"` quieter | ✅ **COMPLETE** |
| **`pan`** | Stereo positioning (-1=left, 0=center, 1=right) | `# pan "-1 1"` alternate L/R | ✅ **COMPLETE** |
| **`begin`** | Start point in sample (0-1) | `# begin "0.5"` start halfway | ✅ **COMPLETE** |
| **`end`** | End point in sample (0-1) | `# end "0.5"` stop halfway | ✅ **COMPLETE** |

### Tidal Has (Phonon Missing)

| Parameter | What It Does | Example | Priority |
|-----------|--------------|---------|----------|
| **`loop`** | Loop sample (1) or one-shot (0) | `# loop "1"` loops | 🟢 MEDIUM |
| **`cut`** | Stop previous instances (for hihat choke) | `# cut "1"` choke group | 🟢 MEDIUM |
| **`unit`** | Time unit (r=rate, c=cycles) | `# unit "c"` sync to cycle | 🟢 MEDIUM |
| **`accelerate`** | Pitch slide over sample duration | `# accelerate "1"` pitch up | 🔵 LOW |
| **`legato`** | Note length/overlap control | `# legato "1"` smooth | 🔵 LOW |

### What This Means

**Tidal's `speed` with negative values gives instant reverse playback:**

```haskell
-- Tidal: Reverse samples with negative speed
d1 $ s "vocal" # speed "-1"

-- Tidal: Half speed backwards
d1 $ s "vocal" # speed "-0.5"

-- Tidal: Pattern alternating forward/backward
d1 $ s "bd sn hh cp" # speed "1 -1"
```

**This is simpler than what we proposed!** Instead of adding a `# reverse` effect, we can just add a `speed` parameter where negative = reverse.

---

## Sample Manipulation Functions

### Granular Synthesis

| Function | What It Does | Phonon Has? |
|----------|--------------|-------------|
| `chop n` | Slice sample into n pieces | ❌ NO |
| `striate n` | Granulate with offset playback | ❌ NO |
| `striateBy n len` | Granulate with length control | ❌ NO |
| `slice n pattern` | Slice and rearrange | ❌ NO |
| `splice n pattern` | Slice with pitch adjustment | ❌ NO |
| `randslice n` | Random slice selection | ❌ NO |

**Example:**
```haskell
-- Tidal: Chop sample into 8 pieces
d1 $ chop 8 $ s "break"

-- Tidal: Slice and rearrange
d1 $ slice 8 "0 7 2 5" $ s "break"
```

### Time Manipulation

| Function | What It Does | Phonon Has? |
|----------|--------------|-------------|
| `loopAt n` | Fit sample to n cycles | ❌ NO |
| `fit n` | Time-stretch to cycles | ❌ NO |
| `hurry n` | Speed up pattern and pitch | ❌ NO |

---

## Audio Effects Comparison

### Effects Phonon HAS ✅

| Effect | Phonon | Tidal |
|--------|--------|-------|
| **Lowpass filter** | `lpf :cutoff :q` | `# lpf`, `# lpq` |
| **Highpass filter** | `hpf :cutoff :q` | `# hpf`, `# hpq` |
| **Bandpass filter** | `bpf :cutoff :q` | `# bandf`, `# bandq` |
| **Reverb** | `reverb :room_size :damping :mix` | `# room`, `# size`, `# dry` |
| **Delay** | `delay :time :feedback :mix` | `# delay`, `# delaytime`, `# delayfeedback` |
| **Distortion** | `distort :drive :mix` | `# distort` |
| **Chorus** | `chorus :rate :depth :mix` | (not documented as built-in) |
| **Bitcrush** | `bitcrush` | `# crush`, `# coarse` |
| **Compressor** | `compressor` | (not documented as built-in) |

### Major Effects Phonon IS MISSING ❌

| Effect | What It Does | How Tidal Does It |
|--------|--------------|-------------------|
| **DJ Filter** | Smooth low/high filter crossfade | `# djf "0.5"` (0=low, 1=high) |
| **Vowel Filter** | Formant filtering (a,e,i,o,u) | `# vowel "a e i"` |
| **Phaser** | Phase modulation | `# phaserrate`, `# phaserdepth` |
| **Tremolo** | Amplitude modulation | `# tremolorate`, `# tremolodepth` |
| **Ring Modulation** | Frequency multiplication | `# ring`, `# ringf` |
| **Frequency Shifter** | Pitch without time | `# fshift`, `# fshiftnote` |
| **Leslie Speaker** | Rotating speaker sim | `# leslie`, `# lrate` |
| **Spectral Effects** | FFT-based processing | `# scram`, `# binshift`, etc. |
| **Waveshaping** | Various distortions | `# shape`, `# triode`, `# squiz` |

---

## Pattern Transformations

### Pattern Ops Phonon HAS ✅

| Transform | Phonon | Tidal |
|-----------|--------|-------|
| Speed up | `$ fast 2` | `fast 2` |
| Slow down | `$ slow 2` | `slow 2` |
| Reverse events | `$ rev` | `rev` |
| Every Nth | `$ every 4 (fast 2)` | `every 4 (fast 2)` |

### What Phonon IS MISSING ❌

| Transform | What It Does | Tidal Example |
|-----------|--------------|---------------|
| `jux` | Stereo copy with transform | `jux rev` (reverse in right channel) |
| `chunk` | Apply transform to part of cycle | `chunk 4 (fast 2)` |
| `degradeBy` | Randomly remove events | `degradeBy 0.5` (50% chance) |
| `sometimesBy` | Randomly apply transform | `sometimesBy 0.3 (fast 2)` |
| `iter` | Rotate events | `iter 4` |
| `palindrome` | Forward then reverse | `palindrome` |
| `brak` | Syncopation pattern | `brak` |
| `whenmod` | Conditional based on cycle | `whenmod 8 6 (fast 2)` |

---

## Priority Implementation List

### Phase 1: Critical Sample Parameters (8 hours)

These are **essential** for basic parity:

1. **`speed` parameter** (including negative for reverse)
   ```phonon
   ~drums: s "bd sn" # speed "1 -1"  -- Forward, backward
   ~vocal: s "vocal" # speed "-0.5"  -- Slow reverse
   ```

2. **`gain` parameter** (volume control)
   ```phonon
   ~quiet: s "bd" # gain "0.3"
   ~loud: s "sn" # gain "1.5"
   ```

3. **`pan` parameter** (stereo positioning)
   ```phonon
   ~left: s "bd" # pan "0"     -- Full left
   ~center: s "sn" # pan "0.5" -- Center
   ~right: s "hh" # pan "1"    -- Full right
   ```

### Phase 2: Sample Slicing (12 hours)

4. **`begin` and `end` parameters**
   ```phonon
   -- Play just the attack
   ~attack: s "sn" # begin "0" # end "0.3"

   -- Skip the attack
   ~tail: s "sn" # begin "0.3" # end "1"
   ```

5. **`cut` parameter** (hihat choke groups)
   ```phonon
   -- Hihats choke each other
   ~hh: s "hh*8" # cut "1"
   ~hho: s "hh:1*4" # cut "1"  -- Same group, they choke
   ```

### Phase 3: Essential Effects (20 hours)

6. **DJ Filter** (`djf`)
7. **Vowel Filter** (`vowel`)
8. **Phaser** and **Tremolo**

### Phase 4: Granular Functions (30 hours)

9. **`chop`** - basic granular
10. **`slice`** - rearrange slices
11. **`striate`** - advanced granular

### Phase 5: Advanced Transforms (40 hours)

12. **`jux`** - stereo transforms
13. **`iter`**, `palindrome` - rotation/mirroring
14. **`sometimesBy`**, `degradeBy` - randomization

---

## The Big Missing Piece: `speed` with Reverse

**This is the easiest win!** Tidal doesn't have a separate `reverse` effect - it just uses **negative `speed`** values.

### Why This Is Brilliant

1. **Simple**: One parameter does speed AND reverse
2. **Expressive**: Can pattern reverse/forward easily
3. **Consistent**: Negative values = negative direction (intuitive)
4. **Powerful**: Combine with pattern for complex effects

### Tidal Examples We Can't Do Yet

```haskell
-- Tidal: Alternate forward/backward
d1 $ s "bd sn hh cp" # speed "1 -1"

-- Tidal: Progressive slowdown and reverse
d1 $ s "vocal" # speed "1 0.5 0 -0.5 -1"

-- Tidal: Pattern the speed
d1 $ s "break" # speed (range 0.5 2 $ sine)
```

### Proposed Phonon Syntax (After Implementation)

```phonon
-- Alternate forward/backward
~drums: s "bd sn hh cp" # speed "1 -1"

-- Slow reverse
~vocal: s "vocal" # speed "-0.5"

-- Speed modulated by LFO
~lfo: sine 0.25
~drums: s "break" # speed (~lfo * 2 + 1)  -- 0.5x to 1.5x

-- Progressive reverse
~weird: s "vocal*4" # speed "1 0.5 -0.5 -1"
```

---

## Implementation Strategy

### Start With Speed (Quick Win)

```rust
// In unified_graph.rs
SignalNode::SampleTrigger {
    // ... existing fields
    speed: Signal,  // NEW: playback speed (negative = reverse)
}

// In voice_manager.rs
fn trigger_sample(sample_buffer, speed) {
    if speed < 0.0 {
        // Play backwards
        read_sample_backwards(sample_buffer, speed.abs())
    } else {
        // Play forwards
        read_sample_forwards(sample_buffer, speed)
    }
}
```

**Estimated effort:** 4 hours
- Add `speed` parameter to sample triggering
- Implement forward/reverse playback
- Support pattern-controllable speed
- Test with various patterns

### Then Add Gain and Pan

**Estimated effort:** 2 hours each
- `gain`: multiply sample amplitude
- `pan`: stereo positioning (already have stereo output)

### Finally Begin/End

**Estimated effort:** 4 hours
- Sample slice start/end points
- Combine with speed for complex effects

---

## Recommendations

### ✅ Phase 1 Complete: Critical Sample Parameters

1. ✅ **COMPLETE:** `speed` parameter with reverse support
2. ✅ **COMPLETE:** `gain` parameter
3. ✅ **COMPLETE:** `pan` parameter
4. ✅ **COMPLETE:** `begin` and `end` parameters

**Status:** All core sample manipulation parameters are now implemented! Phonon now has feature parity with TidalCycles for basic sample playback control.

### Next Priority (Short Term)

5. Implement `cut` (choke groups) - for hihat choking effects
6. Add DJ filter effect
7. Implement `loop` parameter

### Long Term (As Needed)

- Granular functions (when users request)
- Advanced transforms (when users request)
- Exotic effects (lower priority)

---

## Summary

**What Phonon has:**
- ✅ Strong foundation with basic effects and good pattern system
- ✅ **All core sample manipulation parameters** (`speed`, `gain`, `pan`, `begin`, `end`)
- ✅ Negative speed values for reverse playback
- ✅ Pattern-based control of all parameters

**What Phonon is missing:**
- `cut` parameter for sample choking (hihat effects)
- `loop` parameter for sample looping control
- Some effects (DJ filter, vowel filter, etc.)
- Advanced granular functions

**Status:** ✅ **Phase 1 Complete!** Phonon now has feature parity with TidalCycles for core sample playback control.

**Next priorities:**
1. `cut` parameter (choke groups) - MEDIUM priority
2. DJ filter effect - MEDIUM priority
3. `loop` parameter - MEDIUM priority
4. Granular functions - as requested by users

The core sample manipulation capabilities are now complete. Future work will focus on advanced effects and granular processing based on user demand rather than trying to match Tidal 1:1.

---

## Answer to Your Question

> doesn't tidal have sample reversal stuff? are we missing stuff from tidal?

**Yes!** Tidal has **`speed`** parameter where **negative values play samples backwards**:

```haskell
d1 $ s "vocal" # speed "-1"  -- Full reverse
d1 $ s "bd sn" # speed "1 -1"  -- Alternate forward/backward
```

We should implement this instead of (or in addition to) a `# reverse` effect. It's more flexible and matches Tidal's design.

We're also missing `gain`, `pan`, `begin`, `end`, and `cut` - all essential for sample manipulation.

Want me to start implementing `speed` first? It's the biggest missing piece and gives us reverse for free!
