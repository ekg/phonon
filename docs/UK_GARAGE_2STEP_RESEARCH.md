# UK Garage and 2-Step Patterns Research

## Genre Overview

**UK Garage** (UKG) emerged in the mid-1990s from UK club culture, blending American house and garage with British sensibilities. **2-step** is a subgenre characterized by syncopated, skippy rhythms that break from the four-on-the-floor pulse of traditional house music.

The term "2-step" describes "a general rubric for all kinds of jittery, irregular rhythms that don't conform to garage's traditional four-on-the-floor pulse."

### Typical Tempo
- **130-140 BPM** (some tracks go as low as 122-128 BPM for rolling 2-step)

---

## Core Drum Pattern Structure

### The Fundamental 2-Step Kick Pattern

The defining characteristic: **kicks skip beats**, creating space and bounce.

**Basic pattern (16-step grid, one bar):**
```
Step:  1  2  3  4  5  6  7  8  9  10 11 12 13 14 15 16
Kick:  X  .  .  .  .  .  .  X  .  .  .  .  .  .  .  .
       ^                    ^
       |                    |
     beat 1            8th note between beat 3 and 4
```

In fractional terms:
- Kick 1: Position 0 (beat 1)
- Kick 2: Position 0.4375 (7/16) — the "and" of beat 3

**Variation with bounce (add a ghost kick):**
```
Step:  1  2  3  4  5  6  7  8  9  10 11 12 13 14 15 16
Kick:  X  .  .  .  .  .  x  X  .  .  .  .  .  .  .  .
                        ^
                        |
              16th before beat 3 (ghost kick for bounce)
```

### Snare/Clap Pattern

Snares follow a standard backbeat:
```
Step:  1  2  3  4  5  6  7  8  9  10 11 12 13 14 15 16
Snare: .  .  .  .  X  .  .  .  .  .  .  .  X  .  .  .
                   ^                       ^
               beat 2                   beat 4
```

**Ghost snares** are crucial for the UKG feel — quieter hits on offbeats drive the rhythm.

### Hi-Hat Patterns

1. **Offbeat hats** (every 8th note off the beat):
```
Step:  1  2  3  4  5  6  7  8  9  10 11 12 13 14 15 16
HH:    .  X  .  X  .  X  .  X  .  X  .  X  .  X  .  X
```

2. **Open hats on specific offbeats** (2nd, 4th, 8th 8th-notes):
```
8th:   1  2  3  4  5  6  7  8
Open:  .  X  .  X  .  .  .  X
```

---

## The Critical Element: Swing

**Swing is what makes UK Garage groove.** Without swing, a 2-step pattern sounds mechanical and wrong.

### Understanding Swing Ratios

- **50%** = No swing (straight timing, even divisions)
- **66%** = Perfect triplet swing (2:1 ratio)
- **60-65%** = The sweet spot for UK Garage

**How swing works:**
Roger Linn's MPC swing delays every other 16th note. A swing of 66% means the first 16th note of each pair gets 2/3 of the time, and the second 16th note gets 1/3.

```
Straight timing (50%):    |-----|-----|-----|-----|
Triplet swing (66%):      |--------|---|--------|---|
UKG swing (~62%):         |-------|--|-|-------|--|--
```

### MPC Swing Settings for UKG
- **MPC16 Swing 68/69** — Attack Magazine recommendation
- **SP1200 16 Swing-71** at 60% — Ableton Groove Pool option
- **Logic 16 Swing 55** with manual nudging

### Key Insight: Mixed Straight and Swung Elements

The shuffle feel comes from **both straight and swung elements playing together**:
- **Kick on beat 1** and **snares on 2 & 4**: Hard quantized (straight)
- **Hi-hats and ghost snares**: Swung
- **Rimshots and percussion**: Loose, off-grid

"When using swing to create shuffle in a 2-step beat, you should have enough of your drum sounds placed on the grid or your project will lose the shuffle feel."

---

## Sample Selection

### Kicks
- **Roland TR-909** — punchy, works well pitched up
- **TR-808** — deeper, longer decay for the offbeat hit
- Layer kicks: 909 for punch, 808 for body

### Snares
- **Alesis DM5** — snappy, high-pitched
- **Roland TR-808** — pitched up 2-3 semitones for snap
- **Roland TR-606/909** — classic sounds
- Layer multiple snares for texture

### Hi-Hats
- Keep decay between closed and open
- Open hats give the "cutting high-end"
- Cabasa as alternative hat sound

### Additional Elements
- **Rimshots** — syncopated, loose timing
- **Tambourine** — 16th-note pattern with velocity variation
- **Shakers** — varied velocity for groove
- **Vinyl crackle** — texture between hits

---

## Phonon Implementation

### Current Capabilities

Phonon already has the key tools needed:

1. **`swing amount`** — delays every other event
   - Located: `src/pattern_ops_extended.rs:317`
   - Takes Pattern<f64> so swing can be modulated

2. **`ghost` / `ghostWith offset1 offset2`** — adds ghost notes
   - Located: `src/pattern.rs:1217`
   - Adds quieter copies at specified offsets

3. **Mini-notation** for complex patterns
   - `"bd ~ ~ bd ~ bd ~ ~"` — syncopated kicks
   - `"~ ~ sn ~ ~ ~ ~ sn"` — backbeat snares
   - `"hh*8"` — 8th note hats

### Example Phonon 2-Step Patterns

**Basic 2-step kick pattern:**
```phonon
-- 2-step kick: beat 1 and the "and" of beat 3
s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
```

**With Phonon's swing:**
```phonon
-- Apply swing to hi-hats (0.1 = delay by 1/10 of slot)
s "hh hh hh hh hh hh hh hh" $ swing 0.1
```

**Full 2-step pattern:**
```phonon
bpm: 132

-- Kicks: beat 1 and offbeat before beat 4
~kicks $ s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"

-- Snares with ghost notes
~snares $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~" $ ghost

-- Swung hi-hats
~hats $ s "~ hh ~ hh ~ hh ~ hh" $ swing 0.08

-- Open hat accents on 2nd and 4th 8th notes
~opens $ s "~ oh ~ oh ~ ~ ~ oh"

out $ ~kicks + ~snares + ~hats * 0.6 + ~opens * 0.7
```

### Proposed: `swingBy` Function

To match Tidal's `swingBy` which gives more control:

```
swingBy amount divisions pattern
```

- `amount`: how much to delay (0.5 = half the slot)
- `divisions`: how many slices per cycle (typically 4 or 8)

```phonon
-- Tidal-style swingBy: delay by 1/3, 4 divisions per cycle
s "hh*8" $ swingBy 0.33 4
```

**Implementation note:** Phonon's current `swing` delays by a fixed amount. For authentic UKG, we may want:
1. `swingBy amount divisions` — Tidal semantics
2. A `groove` system for MPC-style swing templates

### Proposed: `press` and `pressBy`

These Tidal functions delay events within their slot — useful for pushing the backbeat:

```phonon
-- Press delays all events by half their slot
s "bd ~ sn ~ bd ~ sn ~" $ press
```

---

## Pattern Examples by Style

### Classic 2-Step (130-135 BPM)
```phonon
bpm: 132

~k $ s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
~s $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~h $ s "~ hh ~ hh ~ hh ~ hh" $ swing 0.08
~o $ s "~ oh ~ oh ~ ~ ~ oh"

out $ ~k + ~s + ~h * 0.6 + ~o * 0.7
```

### Bouncy 2-Step (with ghost kick)
```phonon
bpm: 128

-- Ghost kick on 16th before beat 3
~k $ s "bd ~ ~ ~ ~ ~ bd bd ~ ~ ~ ~ ~ ~ ~ ~"
~s $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~" $ ghost
~h $ s "hh hh hh hh hh hh hh hh" $ swing 0.1

out $ ~k + ~s + ~h * 0.5
```

### Rolling 2-Step (slower, deeper)
```phonon
bpm: 124

-- No kick on bar 2 beat 1 for spaciousness
~k $ s "<[bd ~ ~ ~ ~ ~ ~ bd] [~ ~ ~ ~ ~ ~ ~ bd]>"
~s $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~h $ s "hh*8" $ swing 0.12
~rim $ s "~ ~ ~ rim ~ rim ~ ~" $ swing 0.08

out $ ~k + ~s + ~h * 0.5 + ~rim * 0.4
```

### Speed Garage (more four-on-floor influence)
```phonon
bpm: 138

~k $ s "bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~"
~s $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~h $ s "hh hh oh hh hh hh oh hh" $ swing 0.06

out $ ~k + ~s + ~h * 0.6
```

---

## Implementation Recommendations

### High Priority
1. **`swingBy amount divisions`** — Tidal-compatible swing with divisions parameter
2. **Groove templates** — pre-made MPC-style swing curves (MPC60, SP1200, etc.)
3. **Velocity/gain patterns** — for ghost notes at lower volume

### Medium Priority
1. **`nudge`** — fine timing adjustments (Tidal function)
2. **Per-element swing** — apply different swing amounts to different elements
3. **Groove quantize** — snap to a groove template

### Demo File
Create `demos/uk_garage.ph` showcasing:
- Classic 2-step pattern
- Rolling 2-step variation
- Speed garage variation
- With and without swing comparison

---

## Sources

- [2-step garage - Wikipedia](https://en.wikipedia.org/wiki/2-step_garage)
- [Rolling 2-Step - Attack Magazine](https://www.attackmagazine.com/technique/beat-dissected/rolling-2-step-garage/)
- [UK Garage - Attack Magazine](https://www.attackmagazine.com/technique/beat-dissected/uk-garage/)
- [Beat Breakdown: UK Garage Tutorial - Minimal Audio](https://blog.minimal.audio/uk-garage/)
- [Tidal Cycles Time Reference](https://tidalcycles.org/docs/reference/time/)
- [Strudel Time Modifiers](https://strudel.cc/learn/time-modifiers/)
- [MPC Swing in Reason - Melodiefabriek](https://melodiefabriek.com/sound-tech/mpc-swing-reason/)
- [Swing - zoeblade.com](https://notebook.zoeblade.com/Swing.html)
