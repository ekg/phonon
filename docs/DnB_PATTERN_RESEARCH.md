# Drum and Bass Pattern Research

## Overview

Drum and bass (DnB) emerged in the UK in the early 1990s, characterized by fast breakbeats (165-185 BPM) with heavy bass and sub-bass lines. The genre's defining feature is the **complex syncopation of the drum tracks' breakbeat**.

## The Two-Step Beat

The **two-step beat** is the quintessential DnB drum pattern. Its distinctive feel comes from:

1. **Snares on beats 2 and 4** of the bar
2. **Kicks on the 1st and 6th eighth notes** - the second kick is pushed later, creating the "stepping" feel
3. **Hi-hats on eighth notes** as foundation

### Standard Two-Step Pattern (16 steps at 174 BPM)

```
Step:  1  2  3  4  5  6  7  8  9  10 11 12 13 14 15 16
Kick:  X  .  .  .  .  X  .  .  .  .  .  .  .  .  .  .
Snare: .  .  .  .  X  .  .  .  .  .  .  .  X  .  .  .
HiHat: X  .  X  .  X  .  X  .  X  .  X  .  X  .  X  .
```

**In Phonon notation:**
```phonon
tempo: 2.9  -- ~174 BPM

-- Two-step beat
~kick: s "bd ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~ ~ ~"
~snare: s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~hats: s "hh*8"

out: ~kick + ~snare + ~hats * 0.5
```

### Half-Time Variation

Instead of snare on beats 2 and 4, a single snare lands on beat 3, "halving" the perceived speed:

```phonon
-- Half-time DnB
~kick: s "bd ~ ~ ~ ~ ~ ~ ~"
~snare: s "~ ~ ~ ~ sn ~ ~ ~"
```

## Classic Breakbeats

### The Amen Break

The most famous breakbeat in DnB, sampled from "Amen, Brother" by The Winstons (1969). Features:
- Rapid hi-hats
- Snappy snare
- Syncopated bass drum

**Chopping techniques:**
1. Slice into individual hits (kick, snares, hats, rides)
2. Rearrange slices to create new patterns
3. High-pass filter at 150-200 Hz to avoid bass clashes
4. Time-stretch with "repitch" mode for old-school metallic artifacts

**In Phonon (using sample slices):**
```phonon
-- Chopped Amen pattern
s "amen:0 amen:1 ~ amen:2 ~ ~ amen:3 ~ amen:4 ~ ~ ~ amen:5 ~ ~ ~"
```

### The Think Break

From "Think (About It)" by Lyn Collins, another jungle staple:
```phonon
s "bd ~ ~ bd ~ bd ~ ~, ~ ~ sn ~, think*2"
```

## Ghost Notes

Ghost notes add human feel and rhythmic complexity:

- Lower velocity snare hits between main snares
- Typically at 30-50% of main snare velocity
- Place on 16th notes around main hits
- Add slight timing shifts (5-10ms) for groove

**Swung hi-hats example (from Native Instruments):**
Add hi-hats on the 8th and 10th 16th notes at reduced velocity (76 vs 127) for subtle swing.

```phonon
-- Ghost snares (lower volume)
~main_snare: s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~ghost_snare: s "~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~ ~ ~" * 0.3
```

## DnB Subgenres and Their Patterns

### Jungle (Early 90s, 160-170 BPM)

Original breakbeat-heavy sound with reggae/ragga influences.

```phonon
tempo: 2.75  -- ~165 BPM

-- Classic jungle with chopped break
~break: s "bd ~ bd ~ ~ ~ bd ~, ~ sn ~ sn, amen*2"
~sub: sine "55 ~ ~ 55 ~ ~ ~ ~" * 0.4

out: ~break + ~sub
```

### Liquid DnB (165-175 BPM, typically 174)

Smooth, melodic, soulful. Uses real instruments and vocals.
- Flowing basslines that sound musical
- Pitched-up old-school breakbeats
- Subdued melodic sounds

```phonon
tempo: 2.9  -- 174 BPM

~drums: s "bd ~ ~ ~ ~ bd ~ ~, ~ ~ ~ ~ sn ~ ~ ~, hh*16" * 0.7
~bass: saw "55 55 82.5 73.4" # lpf 800 0.6
~pad: superfm 220 1.2 0.5 * 0.08

out: ~drums + ~bass * 0.3 + ~pad
```

### Neurofunk (170-180 BPM)

Dark, mechanical, intricate sound design. Pioneered by Optical, Noisia, Ed Rush.

- Complex synth work and modulation
- Glitchy percussion
- Dystopian atmospheres
- Second kick shifted to last 16th before beat 3

```phonon
tempo: 3.0  -- 180 BPM

-- Neurofunk pattern (shifted kick)
~kick: s "bd ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ bd ~"
~snare: s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~hats: s "hh*32" * 0.3

-- Neuro bass with modulation
~lfo: sine 4
~bass: saw 55 # lpf (~lfo * 1500 + 500) 0.9
~bass_dist: distort ~bass 8.0 0.6

out: ~kick + ~snare + ~hats + ~bass_dist * 0.25
```

### Jump-Up (170-180 BPM)

Raw energy, bouncy wobbling basslines, punchy drums.

```phonon
tempo: 3.0

-- Jump-up pattern
~drums: s "bd*2 bd ~ bd ~ bd ~, ~ sn ~ sn, hh*16"
~bass: saw "55 55 ~ 82.5" # lpf 1200 0.7

out: ~drums + ~bass * 0.3
```

### Rollers (174-180 BPM)

Long, hypnotic basslines that "roll" - emphasis on groove over switch-ups.

```phonon
tempo: 2.9

-- Roller pattern - minimal, hypnotic
~kick: s "bd ~ ~ ~ ~ bd ~ ~ bd ~ ~ ~ ~ ~ ~ ~"
~snare: s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~hats: s "hh*8" * 0.4

-- Long rolling bass
~bass: saw 55 * 0.4
~bass_filt: ~bass # lpf 600 0.8

out: ~kick + ~snare + ~hats + ~bass_filt
```

### Techstep (170-180 BPM)

Dark, industrial, mechanical. Cold metallic drums, deep growling bass.

```phonon
tempo: 3.0

~drums: s "bd ~ ~ bd ~ ~ bd ~, ~ ~ ~ ~ sn ~ ~ ~, hh(7,16)"
~bass: saw 41.2  -- Low E
~bass_dark: ~bass # lpf 400 0.9

out: ~drums + ~bass_dark * 0.35
```

## Essential Phonon Patterns

### Basic DnB Beat
```phonon
tempo: 2.9
s "bd ~ ~ ~ ~ bd ~ ~, ~ ~ ~ ~ sn ~ ~ ~, hh*8"
```

### Fast Hi-Hat Energy
```phonon
-- 32nd note hats for intense energy
s "hh*32" * 0.4
```

### Euclidean Rhythms for DnB
```phonon
-- Euclidean patterns work great for DnB percussion
s "bd(3,16), sn(2,8,4), hh*16, cp(7,16)"
```

### Reese Bass
```phonon
-- Classic Reese bass (detuned saws)
~bass: supersaw "55 55 55 82.5" 0.6 12
~lfo: sine 0.5 * 0.5 + 0.5
~bass_filtered: ~bass # lpf (~lfo * 1500 + 400) 0.85
```

### Sub Bass Layer
```phonon
-- Pure sub underneath main bass
~sub: sine "55 55 55 82.5" * 0.3
```

## Production Tips

1. **Tempo**: 165-185 BPM, most common is 174 BPM
2. **Kick EQ**: Boost around 50-60 Hz for low-end punch
3. **Snare layering**: Bright snare + lower resonant layer (200-250 Hz)
4. **Hi-hats**: 16th or 32nd notes for energy
5. **Swing**: Add subtle swing (5-10ms timing shifts) to prevent robotic feel
6. **Break processing**: High-pass filter breaks at 150-200 Hz to avoid bass mud
7. **Time-stretching**: Use repitch mode for classic metallic artifacts

## Sources

Research compiled from:
- [Native Instruments Blog - 7 Drum Patterns](https://blog.native-instruments.com/drum-patterns/)
- [EDMProd - How to Make Drum & Bass](https://www.edmprod.com/how-to-make-drum-and-bass/)
- [Drum and Bass UK - Subgenres Explained](https://drumandbassuk.com/news/article/drum-and-bass-subgenres-explained)
- [Wikipedia - Drum and Bass](https://en.wikipedia.org/wiki/Drum_and_bass)
- [United by Bass - DnB Subgenres 2025](https://www.unitedbybass.com/every-drum-bass-subgenre-you-need-to-know-2025-edition/)
- [EDMProd - How to Make Jungle Music](https://www.edmprod.com/how-to-make-jungle-music/)
