# Hip-Hop and Trap Beat Patterns for Phonon

## Overview

This document describes the rhythmic structures of hip-hop and trap beats, and how to express them in Phonon's pattern language.

---

## Hip-Hop Beat Fundamentals

### Tempo
- **Classic hip-hop**: 80-100 BPM (wide range depending on style)
- **Modern hip-hop**: 70-100 BPM (slower, more laid-back feel)

### Core Structure (16-step grid)

Hip-hop beats typically follow a 4/4 time signature with 16th-note subdivisions.

```
Step:  1  2  3  4  5  6  7  8  9  10 11 12 13 14 15 16
Kick:  X  .  .  .  .  .  X  .  .  .  X  .  .  .  .  .
Snare: .  .  .  .  X  .  .  .  .  .  .  .  X  .  .  .
HiHat: x  .  x  .  x  .  x  .  x  .  x  .  x  .  x  .
```

### Key Characteristics

1. **Kick Placement**
   - Primary kick on beat 1 (step 1)
   - Secondary kick often before beat 3 (step 7 or the 16th-note triplet before step 9)
   - "Magic hip-hop kick": 1/16th triplet right before beat 3 adds groove
   - Additional kick on step 11 or 16 for variation

2. **Snare Placement**
   - Snares on beats 2 and 4 (steps 5 and 13)
   - Syncopated ghost snares on off-beats add complexity
   - Final 16th note syncopated snare for fills

3. **Hi-Hat Patterns**
   - Basic: 8th notes throughout (every other step)
   - Variation: 16th notes with velocity variation
   - Triplet hi-hats for modern feel (used sparingly)
   - Open hi-hats on off-beats for accent

4. **Swing/Groove**
   - Swing applied to 16th notes (delays every other note)
   - Velocity variation for humanization
   - Slightly late snares for laid-back feel

### Phonon Hip-Hop Pattern Examples

```phonon
-- Basic boom-bap pattern (87 BPM)
bpm: 87

-- Classic hip-hop kick pattern
~kick $ s "bd ~ ~ ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ ~ ~"

-- Snare on 2 and 4
~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"

-- 8th note hi-hats
~hats $ s "hh*8"

-- Combined with swing
out $ stack [~kick, ~snare, ~hats] $ swing 0.08

-- Alternative using mini-notation grouping
out $ s "[bd ~ ~ ~] [sn ~ ~ ~] [~ ~ bd ~] [sn ~ ~ ~]" $ swing 0.1

-- Euclidean approximation of boom-bap kick (3 hits in 8 steps)
out $ s "bd(3,8) sn(2,8,2)"

-- With sample bank selection
out $ s "bd:2(3,8) sn:1(2,8,2) hh:0*8"
```

### Advanced Hip-Hop Patterns

```phonon
-- Two-bar pattern with variation
bpm: 90

~kick1 $ s "bd ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~ bd"
~kick2 $ s "bd ~ ~ ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ ~ ~"
~kick $ cat [~kick1, ~kick2]

~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~ghosts $ s "~ ~ sn:1 ~ ~ ~ ~ sn:1 ~ ~ ~ ~ ~ ~ sn:1 ~" $ gain 0.3

~hats $ s "hh*8" $ swing 0.05

out $ stack [~kick, ~snare, ~ghosts, ~hats]
```

---

## Trap Beat Fundamentals

### Tempo
- **Standard trap**: 130-150 BPM (140 BPM common)
- Often counted in half-time: 65-80 BPM feel
- Note: 140 BPM with half-time kick/snare = 70 BPM feel

### Core Structure

Trap distinguishes itself through complex hi-hat patterns and sustained 808 bass.

```
Step:   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16
Kick:   X   .   .   .   .   .   .   .   X   .   .   .   .   .   X   .
Clap:   .   .   .   .   .   .   X   .   .   .   .   .   .   .   X   .
HiHat:  x   x   x   x   x   x   x   x   x   x   x   x   x   x   x   x
Rolls:              [triplet]               [32nd roll leading to clap]
```

### Key Characteristics

1. **808 Kick/Bass**
   - Sustained 808 bass notes (not just kicks)
   - Kick and 808 often play together on beat 1
   - 808 patterns are melodic, following bass lines
   - Sharp attack kick layered with sustained 808

2. **Clap/Snare Placement**
   - Claps on beat 3 of each bar (half-time feel)
   - Layered with snaps for texture
   - Additional ghost snares on even 16ths

3. **Hi-Hat Patterns (The Signature Element)**
   - **Base**: 8th or 16th notes
   - **Triplets**: 16th-note triplets (1/16T) for rapid-fire feel
   - **Rolls**: 32nd-note rolls leading into claps/downbeats
   - **Pitch automation**: Hi-hat pitch rises or falls during rolls
   - **Velocity dynamics**: Crescendo rolls, accent patterns

4. **Hi-Hat Roll Techniques**
   - 3-note triplet bursts
   - 32nd-note fills at phrase endings
   - Rolls placed before snare hits
   - End-of-bar flourishes

### Phonon Trap Pattern Examples

```phonon
-- Basic trap beat (140 BPM, half-time feel)
bpm: 140

-- Simple 808 kick pattern
~kick $ s "808bd ~ ~ ~ ~ ~ ~ ~ 808bd ~ ~ ~ ~ ~ 808bd ~"

-- Clap on beat 3 (step 5 in half-time = beat 3)
~clap $ s "~ ~ ~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~"

-- 16th note hi-hats
~hats $ s "hh*16"

out $ stack [~kick, ~clap, ~hats]

-- Hi-hat pattern with triplet rolls
-- Using alternation for triplet feel on specific beats
~hats_basic $ s "hh hh hh hh hh hh hh hh"
~hats_triplet $ s "[hh hh hh]*3" $ fast 2  -- triplet burst

-- Advanced: Using velocity for roll dynamics
~hats $ s "hh*16" $ gain "1 0.7 0.9 0.6 1 0.7 0.9 0.6 1 0.7 0.9 0.6 1 0.8 0.9 1"
```

### Hi-Hat Roll Patterns

```phonon
-- Triplet hi-hat roll (3 quick hits)
-- In trap, these often occur at end of 2-beat phrases
~triplet_roll $ s "[hh hh hh]"

-- 32nd note roll (6 hits in half a beat)
~roll_32 $ s "[hh*6]"

-- Building a trap hi-hat pattern with strategic rolls
bpm: 140

-- Base 8th notes with triplet at end of bar 1, roll before clap
~hats $ s "hh hh hh hh hh [hh hh hh] hh hh hh hh hh hh hh [hh*4] hh"

-- Alternative using fast for rolls
~hats_v2 $ s "hh*8" # every 4 ($ fast 2)  -- double speed every 4th cycle
```

### Full Trap Beat Example

```phonon
bpm: 140

-- 808 kick with sustained bass feel
~kick $ s "808bd:3 ~ ~ ~ ~ ~ ~ ~ 808bd:3 ~ ~ ~ 808bd:3 ~ ~ ~"

-- Layered clap + snare
~clap $ s "~ ~ ~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~"
~snare $ s "~ ~ ~ ~ ~ ~ sn:1 ~ ~ ~ ~ ~ ~ ~ sn:1 ~" $ gain 0.7

-- Complex hi-hat with triplets and rolls
~hats_base $ s "hh hh hh hh hh hh hh hh hh hh hh hh hh hh [hh*3] hh"

-- Open hi-hat accents
~oh $ s "~ ~ ~ ~ 808oh ~ ~ ~ ~ ~ ~ ~ 808oh ~ ~ ~" $ gain 0.6

out $ stack [~kick, ~clap, ~snare, ~hats_base, ~oh]
```

---

## Pattern Operators for Hip-Hop/Trap

### Essential Operators

| Operator | Usage | Effect |
|----------|-------|--------|
| `swing` | `$ swing 0.1` | Delays every other event (groove) |
| `fast` | `$ fast 2` | Double speed (for fills) |
| `slow` | `$ slow 2` | Half speed |
| `every` | `$ every 4 (fast 2)` | Apply transform every N cycles |
| `*` | `hh*16` | Replicate within step |
| `[]` | `[hh hh hh]` | Group into single step (triplets) |
| `gain` | `# gain 0.7` | Volume control |
| `stack` | `stack [~a, ~b]` | Layer patterns |
| `cat` | `cat [~bar1, ~bar2]` | Sequence patterns |

### Euclidean Rhythms for Hip-Hop

Euclidean patterns can approximate classic hip-hop rhythms:

```phonon
-- Boom-bap kick approximation
s "bd(3,8)"  -- 3 kicks in 8 steps: X..X..X.

-- Snare on 2 and 4
s "sn(2,8,2)"  -- 2 snares in 8 steps, rotated by 2: ..X...X.

-- Combined
out $ s "bd(3,8) sn(2,4,1) hh*8"
```

### Creating Triplet Feels

```phonon
-- Triplet subdivision
~triplets $ s "[hh hh hh]"  -- 3 hits in 1 step

-- Triplet swing (shuffle feel)
~shuffle $ s "hh*8" $ swing 0.167  -- 1/6 swing = triplet feel

-- Alternating straight and triplet
~complex $ s "hh hh [hh hh hh] hh"
```

---

## Sample Selection Guide

### 808 Samples (dirt-samples)

| Sample | Description | Use Case |
|--------|-------------|----------|
| `808bd:0-24` | TR-808 bass drums | Trap/hip-hop kicks |
| `808sd:0-24` | TR-808 snares | Snare hits |
| `808oh` | TR-808 open hi-hat | Open hat accents |
| `808hc` | TR-808 closed hi-hat | Hi-hat patterns |
| `808` | Mixed 808 sounds | Various |

### Standard Drum Samples

| Sample | Description |
|--------|-------------|
| `bd:0-N` | Acoustic bass drums |
| `sn:0-N` | Acoustic snares |
| `cp` | Claps |
| `hh:0-N` | Hi-hats |
| `realclaps` | Realistic clap samples |
| `clubkick` | Club-style kicks |

---

## Genre-Specific Patterns

### Boom-Bap (90s East Coast)

```phonon
bpm: 92
~drums $ s "[bd ~ ~ ~] [sn ~ ~ ~] [~ ~ bd ~] [sn ~ ~ ~]"
~hats $ s "hh*8" $ swing 0.12
out $ stack [~drums, ~hats]
```

### Lo-Fi Hip-Hop

```phonon
bpm: 75
-- Slow, relaxed feel with heavy swing
~drums $ s "[bd ~ ~ ~] [sn ~ ~ ~] [~ bd ~ ~] [sn ~ ~ ~]" $ swing 0.15
~hats $ s "hh*8" $ gain "0.6 0.4 0.5 0.4 0.6 0.4 0.5 0.4"
out $ stack [~drums, ~hats]
```

### Modern Trap

```phonon
bpm: 145
~kick $ s "808bd ~ ~ ~ ~ ~ ~ ~ 808bd ~ ~ ~ ~ ~ 808bd ~"
~clap $ s "~ ~ ~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~"
~hats $ s "hh*16" $ swing 0.03
~rolls $ s "~ ~ ~ ~ ~ ~ [hh*3] ~ ~ ~ ~ ~ ~ ~ [hh*6] ~" $ gain 0.8
out $ stack [~kick, ~clap, ~hats, ~rolls]
```

### Drill

```phonon
bpm: 140
-- Syncopated kick pattern, sliding 808s
~kick $ s "808bd ~ ~ 808bd ~ ~ ~ ~ 808bd ~ ~ ~ ~ 808bd ~ ~"
~snare $ s "~ ~ ~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~"
~hats $ s "hh*16" $ swing 0.05
out $ stack [~kick, ~snare, ~hats]
```

---

## Tips for Authentic Patterns

1. **Velocity Variation**: Use `gain` patterns for dynamics
2. **Swing Amount**:
   - Hip-hop: 0.08-0.15 (more pronounced)
   - Trap: 0.03-0.08 (subtler)
3. **Ghost Notes**: Layer quiet snares on off-beats
4. **Sample Selection**: Use `:N` syntax to try different variations
5. **Pattern Length**: 2-bar or 4-bar patterns with variation
6. **Rolls at Transitions**: Use `every` to add fills at phrase boundaries

---

## References

- [Native Instruments Drum Patterns Guide](https://blog.native-instruments.com/drum-patterns/)
- [eMastered Trap Drum Patterns](https://emastered.com/blog/trap-drum-patterns)
- [LANDR Trap Hi-Hat Techniques](https://blog.landr.com/trap-hats/)
- [EDP Drums Guide](https://www.edmprod.com/drums-guide/)
