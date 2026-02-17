# Dub and Reggae Patterns for Phonon

This document describes the musical characteristics of dub and reggae music, with practical examples for Phonon live coding.

## Overview

Reggae and dub music are characterized by:
- **Tempo**: 60-90 BPM (dub tends toward slower end)
- **Time signature**: 4/4
- **Offbeat emphasis**: The "skank" on beats 2 and 4, or continuous offbeats
- **Heavy bass**: Deep, melodic basslines emphasizing root and fifth
- **Space**: Strategic use of silence and sparse arrangements
- **Effects**: Heavy use of delay, reverb (especially spring), and filtering

## The Three Core Reggae Drum Patterns

### 1. One Drop

The most iconic reggae pattern. Beat 1 is "dropped" (empty), with emphasis on beat 3.

**Characteristics:**
- Kick + cross-stick/rimshot together on beat 3
- Hi-hat plays continuous 8ths with swing
- Beat 1 is silent (the "drop")
- Relaxed, laid-back feel

```phonon
-- One Drop pattern at 70 BPM
cps: 1.167  -- 70 BPM = 70/60 cps

-- Kick only on beat 3 (third quarter of cycle)
~kick $ s "~ ~ bd ~"

-- Cross-stick with kick on beat 3
~stick $ s "~ ~ rim ~"

-- Hi-hat continuous 8ths with slight swing
~hats $ s "hh*8" $ swing 0.1

-- Full one-drop
out $ (~kick + ~stick + ~hats) * 0.7
```

### 2. Rockers (Roots Rock)

More driving than one drop, with kick on beats 1 and 3.

**Characteristics:**
- Kick on beats 1 and 3
- Snare/rimshot on beats 2 and 4
- Hi-hat on 8ths or 16ths
- Inspired by R&B/soul rhythms
- Tempo: 60-90 BPM

```phonon
-- Rockers pattern at 80 BPM
cps: 1.333  -- 80 BPM

-- Kick on 1 and 3
~kick $ s "bd ~ bd ~"

-- Snare on 2 and 4 (backbeat)
~snare $ s "~ sn ~ sn"

-- Hi-hat 8ths
~hats $ s "hh*8"

-- Full rockers beat
out $ (~kick + ~snare + ~hats) * 0.7
```

### 3. Steppers (Four on the Floor)

The most driving reggae pattern with kick on every beat.

**Characteristics:**
- Kick on all four beats (quarter notes)
- Snare/rimshot on 2 and 4
- Hi-hat continuous
- Tempo: 110-140 BPM (faster than other reggae styles)
- Propulsive, marching feel

```phonon
-- Steppers pattern at 120 BPM
cps: 2.0  -- 120 BPM

-- Four-on-the-floor kick
~kick $ s "bd*4"

-- Snare on backbeat
~snare $ s "~ sn ~ sn"

-- Hi-hat 16ths
~hats $ s "hh*16" # gain 0.6

-- Full steppers
out $ (~kick + ~snare + ~hats) * 0.7
```

## The Offbeat "Skank" (Guitar/Keys)

The skank is the rhythmic backbone of reggae - short, percussive chords on the offbeats.

**Characteristics:**
- Plays on the "and" of each beat (offbeats)
- Staccato, muted quality
- Higher frequencies (avoids bass territory)
- Creates the characteristic "choppy" feel

```phonon
-- Guitar skank pattern
-- Uses chord stabs on offbeats
cps: 1.333

-- Offbeat pattern: rest on beats, hit on &s
-- "~ x" within each beat = hit on the "and"
~skank $ s "[~ perc]*4"  -- or use chord samples

-- Alternative with mini-notation
~skank $ s "~ perc ~ perc ~ perc ~ perc"

-- With filter to thin the sound
out $ ~skank # hpf 500 0.5 * 0.5
```

## The Keyboard "Bubble"

The organ bubble is a low, pulsing rhythm that creates reggae's hypnotic groove.

**Characteristics:**
- Steady 8th-note pulse
- Low-mid frequency range
- Soft, rounded attack
- "Felt more than heard"
- Often uses Hammond organ tones

```phonon
-- Keyboard bubble with synth
cps: 1.167  -- 70 BPM

-- Low organ tone pulsing on 8ths
-- Using sine for soft, round tone
~bubble $ sine 110 # lpf 400 0.3 # gain "0.3 0.5 0.3 0.5 0.3 0.5 0.3 0.5"

-- Alternative: amplitude envelope shaping
~bubble $ sine 110 # lpf 300 0.5 * "0.4 0.6"

out $ ~bubble * 0.4
```

## The Reggae Bass

Deep, melodic bass emphasizing root and fifth with space.

**Characteristics:**
- Root and fifth are primary notes
- Octave jumps for interest
- Syncopated rhythms
- Deep, warm tone
- Plays in the spaces left by kick drum

```phonon
-- One-drop bass pattern
cps: 1.167

-- Root (55 Hz = A1) with octave jump
-- Pattern leaves space on beat 1, hits around beat 3
~bass $ saw "[~ 55] [110 ~] [~ 82.5] [55 ~]" # lpf 200 0.8

-- Simplified root-fifth pattern
~bass $ saw "55 ~ 82.5 ~" # lpf 200 0.8

out $ ~bass * 0.5
```

## Dub Effects Processing

Dub music is defined by its effects processing: delay, reverb, and filtering.

### Delay (Echo)

The signature dub effect. Key techniques:
- Triplet or dotted timing
- Feedback creates cascading echoes
- Filter the delayed signal (cut low end)
- "Riding" the feedback in real-time

```phonon
-- When delay is implemented in Phonon:
-- ~dub $ s "rim" # delay 0.6 # delaytime 0.375 # feedback 0.7

-- For now, use rhythmic patterns to simulate echo
cps: 1.167

~rim $ s "rim ~ ~ ~"
~echo1 $ s "~ rim:1 ~ ~" # gain 0.5  -- first echo
~echo2 $ s "~ ~ rim:2 ~" # gain 0.25  -- second echo

out $ (~rim + ~echo1 + ~echo2) * 0.6
```

### Reverb

Spring reverb is classic dub, but large plate/hall also used.

- Spring reverb on drums (especially snare)
- Plate reverb on vocals/melodics
- Large, spacious settings
- High-pass the reverb to keep low end clean

### Filtering

Real-time filter sweeps are key to dub mixing.

```phonon
-- LFO-controlled filter sweep
cps: 1.167

~lfo $ sine 0.25  -- slow LFO (one cycle per 4 seconds)
~drums $ s "bd ~ [~ rim] ~"
~filtered $ ~drums # lpf (~lfo * 2000 + 500) 0.6

out $ ~filtered * 0.7
```

## Complete Dub Pattern Examples

### Example 1: Minimal One-Drop Dub

```phonon
-- Minimal one-drop dub
cps: 1.0  -- 60 BPM, very slow

-- Drums: classic one-drop
~kick $ s "~ ~ bd ~"
~rim $ s "~ ~ rim ~"
~hats $ s "hh*8" $ swing 0.15 # gain 0.4

-- Bass: root and fifth with space
~bass $ saw "[~ 55] [~ ~] [82.5 ~] [~ 55]" # lpf 180 0.9

-- Skank: offbeat chords (using percussion as placeholder)
~skank $ s "[~ perc]*4" # hpf 800 0.3 # gain 0.5

-- Output mix
out $ ~kick * 0.8 + ~rim * 0.6 + ~hats * 0.4 + ~bass * 0.5 + ~skank * 0.3
```

### Example 2: Steppers Dub

```phonon
-- Steppers dub with filter modulation
cps: 2.0  -- 120 BPM

-- Four-on-floor kick
~kick $ s "bd*4"

-- Snare with ghost notes
~snare $ s "~ sn ~ sn"
~ghost $ s "~ [~ sn:1] ~ [~ sn:1]" # gain 0.3

-- Hi-hats with variation
~hats $ s "hh*16" # gain "0.5 0.3 0.6 0.3"

-- Driving bass
~bass $ saw "55*4" # lpf 150 0.9

-- Filter modulation on everything
~lfo $ sine 0.5
~mix $ (~kick + ~snare + ~ghost + ~hats) * 0.6 + ~bass * 0.4
out $ ~mix # lpf (~lfo * 1500 + 800) 0.5
```

### Example 3: Roots Rockers

```phonon
-- Classic roots rockers feel
cps: 1.25  -- 75 BPM

-- Rockers drums: kick on 1 and 3
~kick $ s "bd ~ bd ~"
~snare $ s "~ sn ~ sn"
~hats $ s "hh*8" $ swing 0.1

-- Melodic bass with octave jump
~bass $ saw "[55 ~] [~ 82.5] [55 110] [82.5 ~]" # lpf 200 0.8

-- Bubble organ simulation
~bubble $ sine 220 # lpf 500 0.3 * "0.3 0.5"

-- Skank
~skank $ s "[~ rim:2]*4" # hpf 1000 0.5 # gain 0.4

out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.4 + ~bass * 0.5 + ~bubble * 0.2 + ~skank * 0.3
```

## Pattern Variations

### Swing and Shuffle

Reggae often uses subtle swing on hi-hats:

```phonon
~hats $ s "hh*8" $ swing 0.1  -- subtle swing
~hats $ s "hh*8" $ swing 0.2  -- more pronounced shuffle
```

### Euclidean Rhythms for Dub

Euclidean patterns can create interesting dub variations:

```phonon
-- 3 hits spread over 8 steps
~kick $ s "bd(3,8)"

-- 5 hits over 16 for complex hi-hat
~hats $ s "hh(5,16)"
```

### Drop Outs

Creating space by removing elements:

```phonon
-- Remove snare every 4th bar
~snare $ s "~ sn ~ sn" $ every 4 (const (silence))

-- Or use degradeBy for random dropouts
-- (when implemented)
```

## Summary of Key Musical Elements

| Element | One Drop | Rockers | Steppers |
|---------|----------|---------|----------|
| Tempo | 60-80 BPM | 60-90 BPM | 110-140 BPM |
| Kick | Beat 3 only | Beats 1 & 3 | All beats |
| Snare | Beat 3 | Beats 2 & 4 | Beats 2 & 4 |
| Feel | Laid-back | Grooving | Driving |
| Hi-hat | Swung 8ths | Swung 8ths | Straight 16ths |

## References

- [Rhythm Notes: 3 Reggae Drum Beats](https://rhythmnotes.net/reggae-drum-beats/)
- [One Drop Rhythm - Wikipedia](https://en.wikipedia.org/wiki/One_drop_rhythm)
- [Modern Drummer: Steppers Beat](https://www.moderndrummer.com/article/february-2019-reggae-101-the-steppers-beat/)
- [The History of the Organ Bubble in Reggae - Dubmatix](https://bassculture.substack.com/p/the-history-of-the-organ-bubble-in)
- [The Reggae Guitar Skank - Dubmatix](https://bassculture.substack.com/p/the-reggae-guitar-skank)
- [Ska Stroke - Wikipedia](https://en.wikipedia.org/wiki/Ska_stroke)
- [Soundfingers: Authentic Reggae Dub Bass](https://soundfingers.com/blog/reggae-dub-production/authentic-reggae-dub-bass-tutorial/)
- [BPM Ranges by Genre](https://toolstud.io/music/bpm.php?bpm=75&title=Slow+dub/reggae)
