# Tidal Pattern Parity Analysis

Analysis of TidalCycles patterns used in `~/livecode/` and their status in Phonon.

## Summary

Based on analysis of 30 .tidal files from ~/livecode, here's the feature comparison.

---

## Pattern Transforms

### Timing/Groove (Heavily Used) ✅ Mostly Supported

| Tidal Function | Phonon Status | Usage in livecode |
|----------------|---------------|-------------------|
| `swing` | ✅ Implemented | `d1 $ swing $ ...` |
| `nudge` | ✅ Implemented | `# nudge (fast 8 "0 0.015")` for micro-timing |
| `late` | ✅ Implemented | `late 0.1` delays pattern |
| `early` | ✅ Implemented | `early 0.1` advances pattern |
| `legato` | ✅ Implemented | `# legato 1` - sustain notes |
| `staccato` | ✅ Implemented | Short notes |
| `off` | ❌ **MISSING** | `off (1/8) (# crush 8)` - offset copy with transform |

**`off` is heavily used** - creates a delayed copy of pattern with transformation applied:
```haskell
-- Tidal: Create offset echo with effect
d1 $ off (1/8) (# crush 8) $ s "bd sn"
-- Creates: bd, bd+crush@1/8, sn, sn+crush@1/8
```

### Speed/Time ✅ Fully Supported

| Tidal Function | Phonon Status | Example |
|----------------|---------------|---------|
| `fast` | ✅ Implemented | `fast 2` |
| `slow` | ✅ Implemented | `slow 4` |
| `hurry` | ✅ Implemented | `hurry 2` (speed + pitch) |
| `rev` | ✅ Implemented | `rev` |
| `palindrome` | ✅ Implemented | `palindrome` |
| `iter` | ✅ Implemented | `iter 4` |
| `rotL` / `rotR` | ✅ Implemented | `rotL 0.25` |

### Conditional Transforms ✅ Fully Supported

| Tidal Function | Phonon Status | Example |
|----------------|---------------|---------|
| `every` | ✅ Implemented | `every 4 (fast 2)` |
| `every'` | ✅ Implemented | `every' 4 1 (fast 2)` |
| `whenmod` | ✅ Implemented | `whenmod 8 6 (fast 2)` |
| `sometimes` | ✅ Implemented | `sometimes (# crush 8)` |
| `sometimesBy` | ✅ Implemented | `sometimesBy 0.5 (...)` |
| `often` / `rarely` | ✅ Implemented | Probability variants |
| `foldEvery` | ✅ Implemented | `foldEvery [3,4] (fast 2)` |

### Sample Manipulation ✅ Mostly Supported

| Tidal Function | Phonon Status | Example |
|----------------|---------------|---------|
| `chop` | ✅ Implemented | `chop 16 $ s "break"` |
| `striate` | ✅ Implemented | `striate 8` |
| `slice` | ✅ Implemented (as bite) | `slice 16 "0 3 8 2"` |
| `loopAt` | ✅ Implemented | `loopAt 4 $ s "break"` |
| `struct` | ✅ Implemented | `struct "t(3,8)"` |

### Pattern Effects ✅ Supported

| Tidal Function | Phonon Status | Example |
|----------------|---------------|---------|
| `stut` | ✅ Implemented | `stut 4 0.5 (1/8)` |
| `echo` | ✅ Implemented | `echo 3 0.125 0.5` |
| `jux` | ✅ Implemented | `jux rev` |
| `juxBy` | ✅ Implemented | `juxBy 0.5 rev` |
| `superimpose` | ✅ Implemented | `superimpose (fast 2)` |

---

## Effects/Modifiers (# syntax)

### Audio Effects ✅ Mostly Supported

| Tidal Effect | Phonon Status | Example Usage |
|--------------|---------------|---------------|
| `lpf` / `cutoff` | ✅ Implemented | `# lpf 1000` |
| `hpf` / `hcutoff` | ✅ Implemented | `# hpf 500` |
| `resonance` | ✅ Implemented | `# resonance 0.3` |
| `gain` | ✅ Implemented | `# gain 0.8` |
| `pan` | ✅ Implemented | `# pan 0.3` |
| `speed` | ✅ Implemented | `# speed 0.5` |
| `crush` | ✅ Implemented | `# crush 8` (bitcrush) |
| `room` / `sz` (size) | ✅ Implemented | `# room 0.5 # sz 0.8` |
| `delay` / `delaytime` | ✅ Implemented | `# delay 0.5` |
| `squiz` | ⚠️ Old parser only | `# squiz 4` (frequency squashing) |
| `coarse` | ✅ Implemented | `# coarse 8` (sample rate reduction) |
| `vowel` | ✅ Implemented | `# vowel "a e i o u"` (formant filter) |
| `bpf` | ✅ Implemented | `# bpf 2000` (bandpass filter) |

### Sample Selection ✅ Implemented

| Tidal Syntax | Phonon Status | Example |
|--------------|---------------|---------|
| `s "bd"` | ✅ Implemented | Sample trigger |
| `n "0 1 2"` | ✅ Implemented | Sample variant selection |
| `s "bd:0 bd:1"` | ✅ Implemented | Inline sample bank |
| `note "c4 e4"` | ✅ Implemented | Pitch control |

---

## Pattern Composition

### Layering ⚠️ Partially Supported

| Tidal Function | Phonon Status | Example |
|----------------|---------------|---------|
| `stack [ ]` | ✅ Via `+` operator | `~a + ~b` combines buses |
| `cat [ ]` | ⚠️ Via mini-notation | `<a b c d>` sequences |
| `seqPLoop` | ❌ Not needed | Use mini-notation |
| `overlay` | ✅ Via `+` | Bus arithmetic |

### LFO/Control Patterns ✅ Supported

| Tidal Pattern | Phonon Equivalent | Example |
|---------------|-------------------|---------|
| `range min max $ slow 4 sine` | `sine 0.25 * amount + offset` | LFO scaling |
| `sine` | ✅ `sine freq` | Oscillator |
| `saw` | ✅ `saw freq` | Ramp |
| `tri` | ✅ `tri freq` | Triangle |
| `square` | ✅ `square freq` | Pulse |

---

## Missing Features (Priority)

### High Priority

1. **`off` transform** - Heavily used for rhythmic interest
   ```haskell
   -- Tidal: Creates offset copy with transformation
   d1 $ off (1/8) (# crush 8) $ s "bd sn"
   -- Result: bd, bd+crush@1/8, sn, sn+crush@1/8 (layered)

   -- Phonon equivalent needed:
   ~drums $ s "bd sn" $ off (1/8) (# crush 8)
   ```

### Medium Priority

2. **`squiz`** - Frequency squashing effect (in old parser, needs compositional compiler)

### Lower Priority

3. **`spread`** - Apply list of functions across pattern
4. **`chunk`** - Apply function to nth chunk
5. **`bite`** (already have as slice)

---

## Example Translations

### Swing with nudge
```haskell
-- Tidal
d1 $ s "bd*4" # nudge (fast 8 "0 0.02")

-- Phonon
~drums $ s "bd*4" # nudge "0 0.02 0 0.02 0 0.02 0 0.02"
-- Or use swing transform:
~drums $ s "bd*4" $ swing 0.1
```

### Off pattern (NOT YET SUPPORTED)
```haskell
-- Tidal
d1 $ off (1/8) (# crush 8) $ s "bd sn hh cp"

-- Phonon (when implemented):
~drums $ s "bd sn hh cp" $ off (1/8) (# crush 8)
```

### Stack with effects
```haskell
-- Tidal
d1 $ stack [
  s "bd*4" # gain 1.1,
  s "~ sn ~ sn" # gain 0.9 # room 0.2
]

-- Phonon
~kick $ s "bd*4" # gain 1.1
~snare $ s "~ sn ~ sn" # gain 0.9 # room 0.2
out $ ~kick + ~snare
```

### Range with LFO
```haskell
-- Tidal
d1 $ s "bd*4" # lpf (range 500 2000 $ slow 4 sine)

-- Phonon
~lfo # sine 0.25
~drums $ s "bd*4" # lpf (~lfo * 1500 + 500) 0.8
```

### Every with effect
```haskell
-- Tidal
d1 $ every 4 (# crush 4) $ s "bd sn hh cp"

-- Phonon
~drums $ s "bd sn hh cp" $ every 4 (# crush 4)
```

---

## Implementation Roadmap

### Phase 1: Core Missing Transform
- [ ] Implement `off time transform` - creates offset pattern layer with transform

### Phase 2: Port Missing Effects
- [ ] Port `squiz` from nom_parser to compositional_compiler

### Phase 3: Polish
- [ ] Add `spread` for function lists
- [ ] Add `chunk` variants
- [ ] Documentation/examples for all transforms

---

## Notes

- Phonon's bus system (`~name $`) provides equivalent functionality to Tidal's `d1 $ ...`
- Pattern arithmetic (`~a + ~b * 0.5`) replaces Tidal's stack/overlay
- Mini-notation (`"bd sn <hh cp>"`) handles most sequencing needs
- `$` chains transforms left-to-right (same as Tidal's function application)
- `#` applies effects/modifiers (same as Tidal)

The main gap is the `off` transform which creates rhythmically interesting offset layers.
