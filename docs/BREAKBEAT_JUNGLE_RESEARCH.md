# Breakbeat and Jungle Patterns - Research Document

**Research Date**: 2025-01-28

## Executive Summary

Breakbeat and jungle are rhythm-centric genres built around manipulating sampled drum breaks, particularly the iconic "Amen break." This research documents the musical characteristics, programming techniques, and Tidal Cycles functions needed to implement authentic jungle/breakbeat patterns in Phonon.

---

## 1. Genre Characteristics

### Tempo Ranges
- **Jungle**: 160-175 BPM (classic), up to 185 BPM
- **Drum & Bass**: 170-180 BPM
- **Breakcore**: 160-200+ BPM (can go faster)

### Defining Features
- **Syncopated rhythms**: Intentionally "displaced" drum patterns
- **Polyrhythms**: Multiple breakbeats layered together
- **Ghost notes**: Quieter snare hits that add groove and swing
- **Chopped breaks**: Sample sliced and resequenced creatively
- **Time-stretching**: Pitch-independent tempo manipulation
- **Heavy processing**: Distortion, filtering, compression on drums

---

## 2. The Amen Break

The most sampled drum break in history, from "Amen, Brother" by The Winstons (1969), played by Gregory C. Coleman.

### Original Tempo
137 BPM (often pitched up to 160-180 BPM for jungle)

### Pattern Structure (4 bars)

**From Tidal's tidal-drum-patterns library:**

```
Kick (bd):   "[t ~ t ~] [~ ~ ~ ~] [~ ~ t t] [~ ~ ~ ~]"
Snare (sn):  "[~ ~ ~ ~] [t ~ ~ t] [~ t ~ ~] [t ~ ~ t]"
Closed HH:   "[t ~ t ~] [t ~ t ~] [t ~ t ~] [t ~ t ~]"
Open HH:     "[~ ~ ~ ~] [~ ~ ~ ~] [~ ~ t ~] [~ ~ ~ ~]"
```

**Key features:**
- Bars 1-2: Standard funk groove (good for hip-hop when slowed)
- Bars 3-4: More unstable/syncopated - the "tumbling" feel
- Critical: Snare on beat 4 of bars 3-4 is delayed by an eighth note
- Crash cymbal falls between beats 3 and 4 at the end

### Velocity/Dynamics
- First kick in double-hit is quieter than second
- End-of-bar snares are quieter (not quite ghost notes)
- Ride cymbal: consistent eighth notes throughout
- Ghost snare hits at 16th-note subdivisions before "and" of 3

### Why It Can't Be Perfectly Replicated
From Ethan Hein's analysis: "You can't adequately represent the Amen via MIDI or music notation. Its timbre is doing as much musical work as the placement and timing of drum hits."

The microtiming variations (swing, push/pull) are crucial to the feel.

---

## 3. Core Jungle Programming Techniques

### 3.1 Chopping and Slicing
The fundamental technique - divide breaks into pieces and resequence:

```
1. Slice break into equal parts (8, 16, 32 slices common)
2. Map each slice to a MIDI note or trigger
3. Program new patterns using the slices
4. Reverse slices, change order, stack layers
```

**From Tidal Cycles:**
```haskell
-- Basic slice reorder
d1 $ slice 8 "7 6 5 4 3 2 1 0" $ sound "breaks165"  -- Reverse

-- More complex pattern
d1 $ slice 8 "6 1 [2 3] ~ 4 1 6*2 7" $ sound "break:4"
```

### 3.2 Striate vs Chop (Tidal Semantics)

**chop**: Plays all parts of sample A, then all parts of sample B
```haskell
d1 $ chop 4 $ sound "break:8 break:9"
-- Plays: 8a 8b 8c 8d 9a 9b 9c 9d
```

**striate**: Interlaces parts from multiple samples
```haskell
d1 $ striate 4 $ sound "break:8 break:9"
-- Plays: 8a 9a 8b 9b 8c 9c 8d 9d
```

### 3.3 loopAt - Tempo Sync
Fits sample playback to N cycles:
```haskell
d1 $ loopAt 4 $ chop 32 $ sound "breaks125"
-- Break compressed to 4 cycles, chopped into 32 pieces
```

### 3.4 Granular Techniques
For textural manipulation:
- Time-stretch slices to create ambient textures
- PaulStretch for extreme stretching (800%+)
- Cut into micro-fragments (1/64 notes, 10-50ms)
- Granulate for "shimmering cloud" effects

### 3.5 Layering
Classic technique for punch:
```
1. Take original break (provides texture/ambience)
2. Layer with clean individual hits (kick, snare)
3. High-pass filter the break (~100Hz)
4. Let layered kick provide the low-end
```

### 3.6 Classic Transformations
```haskell
-- Beat shift every 4 bars
d1 $ every 4 (0.25 <~) $ striate 128 $ sound "break"

-- Reverse every 4 bars
d1 $ every 4 rev $ striate 64 $ sound "break"

-- Speed/pitch manipulation
d1 $ striate 16 $ sound "break" # speed 1.5

-- Random cut-up (classic live coding)
d1 $ n "0 [1 5] 2*2 [3 4]" # sound "amencutup"
```

---

## 4. Phonon Implementation Status

### Available Sample Libraries

Phonon has **50+ breakbeat samples** from dirt-samples:

| Directory | Samples | Description |
|-----------|---------|-------------|
| `breaks125/` | 2 | Breaks @ 125 BPM (includes AMEN.WAV) |
| `breaks152/` | 1 | Breaks @ 152 BPM (AMEN.WAV) |
| `breaks157/` | 1 | Breaks @ 157 BPM |
| `breaks165/` | 1 | Breaks @ 165 BPM |
| `amencutup/` | 32 | Pre-sliced Amen break pieces |
| `jungle/` | 13 | Individual jungle drum hits |
| `jungbass/` | 4+ | Jungle bass sounds |

### Working Functions
- ✅ `chop` - Slice sample into n parts
- ✅ `loopAt` - Fit sample to N cycles
- ✅ `shuffle` - Randomize timing
- ✅ `scramble` - Randomize event order
- ✅ `every` - Conditional transforms
- ✅ `rev` - Reverse pattern

### Broken Functions (Critical for Jungle)
- 🔴 `striate` - Produces complete silence
- 🔴 `slice` - Produces complete silence
- 🔴 `legato` - Wrong envelope behavior
- 🔴 `speed` (negative) - Doesn't reverse playback

### Current Phonon Jungle Pattern Example
```phonon
-- From PATTERN_GUIDE.md - basic jungle rhythm
"bd ~ bd ~ ~ ~ bd ~, ~ sn ~ sn, amen*2"
```

---

## 5. Recommended Phonon Implementation

### 5.1 Fix Critical Functions

**Priority 1: `slice`** - Deterministic slice reordering
```phonon
-- Should work like this:
out $ s "breaks152" $ slice 8 "0 7 2 5 1 6 3 4"
```

**Priority 2: `striate`** - Interlaced granular playback
```phonon
-- Should work like this:
out $ s "breaks152 breaks165" $ striate 16
```

**Priority 3: `begin`/`end` parameters** - Manual slice control
```phonon
-- Direct slice control
out $ s "breaks152" # begin 0.25 # end 0.5
```

### 5.2 Add New Functions

**`splice`** - Like slice but with automatic pitch correction
```phonon
out $ s "breaks152" $ splice 8 "0 7 2 5"
```

**`randslice`** - Random slice selection
```phonon
out $ s "breaks152" $ randslice 16
```

**`cut`** - Sample cut groups (monophonic per group)
```phonon
out $ s "breaks152" $ cut 1  -- Cut group 1
```

### 5.3 Pattern Library Additions

**Amen break pattern (programmatic version):**
```phonon
-- Define Amen-style patterns
~amen_bd $ "[t ~ t ~] [~ ~ ~ ~] [~ ~ t t] [~ ~ ~ ~]"
~amen_sn $ "[~ ~ ~ ~] [t ~ ~ t] [~ t ~ ~] [t ~ ~ t]"
~amen_hh $ "[t ~ t ~]*4"

-- Use with struct
out $ struct ~amen_bd $ s "bd"
   + struct ~amen_sn $ s "sn"
   + struct ~amen_hh $ s "hh"
```

**Quick jungle from amencutup:**
```phonon
-- Random amen cutup (classic live coding pattern)
out $ n "0 [1 5] 2*2 [3 4]" $ s "amencutup" # lpf 4000 0.7
```

---

## 6. Example Phonon Jungle Patterns (Proposed)

### 6.1 Basic Chopped Break
```phonon
cps: 2.8  -- ~168 BPM

out $ s "breaks152" $ loopAt 2 $ chop 16
```

### 6.2 Resequenced Amen
```phonon
cps: 2.8

-- When slice is fixed:
out $ s "breaks152" $ slice 16 "0 4 8 12 2 6 10 14 1 5 9 13 3 7 11 15"
```

### 6.3 Layered Jungle
```phonon
cps: 2.8

~break $ s "breaks152" $ loopAt 1 $ chop 8 # lpf 4000 0.5
~kick $ s "bd" $ struct "[t ~ t ~] [~ ~ ~ ~] [~ ~ t ~] [~ ~ ~ ~]"
~snare $ s "sn" $ struct "[~ ~ ~ ~] [t ~ ~ ~] [~ ~ ~ ~] [t ~ ~ ~]"

out $ ~break * 0.4 + ~kick * 0.6 + ~snare * 0.8
```

### 6.4 Reverse Transform
```phonon
cps: 2.8

-- When striate is fixed:
out $ every 4 rev $ s "breaks165" $ striate 32
```

### 6.5 Random Amencutup
```phonon
cps: 2.8

out $ n "0 [1 5] 2*2 [3 4]" $ s "amencutup"
    # lpf (sine 0.25 * 3000 + 1000) 0.7
```

---

## 7. Sources

- [Tidal Cycles Sampling Reference](https://tidalcycles.org/docs/reference/sampling/)
- [Tidal Club - chop and striate lesson](https://club.tidalcycles.org/t/week-3-lesson-4-chop-and-striate/534)
- [Tidal Club - slice and splice lesson](https://club.tidalcycles.org/t/week-3-lesson-3-slice-and-splice/519)
- [tidal-drum-patterns (Amen module)](https://github.com/lvm/tidal-drum-patterns)
- [Amen Break Tidal Gist](https://gist.github.com/TonyStaffu/74a524732d0fb4cac0bd8c36053c8436)
- [Jungle Beat in Tidal Cycles](https://modulations.substack.com/p/001-jungle-beat-created-with-tidal)
- [Building the Amen Break - Ethan Hein](https://www.ethanhein.com/wp/2023/building-the-amen-break/)
- [Breakbeat in Jungle, DnB, and Breakcore - iMusician](https://imusician.pro/en/resources/blog/the-breakbeat-in-jungle-drum-bass-and-breakcore-music)
- [Wikipedia - Breakcore](https://en.wikipedia.org/wiki/Breakcore)
- [How to Make Breakbeats - Mix Elite](https://mixelite.com/blog/how-to-make-breakbeats/)

---

## 8. Next Steps / Tasks to Create

1. **Fix `striate` function** - Critical for jungle patterns
2. **Fix `slice` function** - Critical for break resequencing
3. **Fix negative `speed`** - Needed for reversed slices
4. **Add `begin`/`end` parameter support** - Manual slice control
5. **Add `splice` function** - Pitch-corrected slicing
6. **Add `randslice` function** - Random slice playback
7. **Create example jungle patterns** - For documentation
8. **Add velocity/dynamics support** - For ghost notes and swing
