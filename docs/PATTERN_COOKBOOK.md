# Phonon Pattern Cookbook

A hands-on collection of 50+ patterns and techniques for Phonon, organized from simple to complex. Each example is ready to copy-paste and experiment with.

---

## Table of Contents

1. [Basic Patterns & Mini-Notation](#1-basic-patterns--mini-notation)
2. [Time Transformations](#2-time-transformations)
3. [Structural Transformations](#3-structural-transformations)
4. [Randomness & Probability](#4-randomness--probability)
5. [Synthesis & Effects](#5-synthesis--effects)
6. [Modulation & LFOs](#6-modulation--lfos)
7. [Complete Musical Examples](#7-complete-musical-examples)
8. [Genre-Specific Patterns](#8-genre-specific-patterns)
9. [Advanced Techniques](#9-advanced-techniques)

---

## 1. Basic Patterns & Mini-Notation

### Example 1.1: Your First Beat
The simplest possible drum pattern.

```phonon
cps: 2.0
out $ s "bd sn bd sn"
```

### Example 1.2: Four-on-the-Floor
Classic dance music kick pattern.

```phonon
cps: 2.0
out $ s "bd*4"
```

### Example 1.3: Rests and Silence
Use `~` for rests to create space.

```phonon
cps: 2.0
out $ s "bd ~ sn ~"
```

### Example 1.4: Subdivision with Brackets
`[a b]` plays both items in the time of one.

```phonon
cps: 2.0
-- The [bd bd] plays two kicks where one would normally go
out $ s "[bd bd] sn [bd bd bd] sn"
```

### Example 1.5: Repeat with Asterisk
`*n` repeats an element n times in its slot.

```phonon
cps: 2.0
out $ s "bd sn hh*4 cp"
-- hh*4 = four hi-hats in one beat
```

### Example 1.6: Nested Subdivision
Brackets can nest for complex rhythms.

```phonon
cps: 2.0
out $ s "bd [[sn sn] cp] hh [cp [hh hh]]"
```

### Example 1.7: Alternation with Angle Brackets
`<a b c>` cycles through options, one per cycle.

```phonon
cps: 2.0
-- Different bass sound each cycle
out $ s "<bd kick bass> sn hh cp"
```

### Example 1.8: Polyrhythm with Commas
Commas in brackets play patterns simultaneously.

```phonon
cps: 2.0
-- Three-against-four polyrhythm
out $ s "[bd bd bd, hh hh hh hh]"
```

### Example 1.9: Euclidean Rhythms
`(n,k)` distributes n hits over k steps evenly.

```phonon
cps: 2.0
-- Classic 3 over 8 (tresillo rhythm)
out $ s "bd(3,8)"
```

### Example 1.10: Combining Techniques
Layer multiple mini-notation features.

```phonon
cps: 2.0
out $ s "[bd(3,8), hh*8, ~ sn ~ <sn cp>]"
```

### Example 1.11: Sample Selection with Colon
`:n` selects a specific sample from a folder.

```phonon
cps: 2.0
-- Cycle through different kick samples
out $ s "bd:0 bd:1 bd:2 bd:3"
```

### Example 1.12: Degrade with Question Mark
`?` gives 50% chance of playing; `?0.3` gives 30% chance.

```phonon
cps: 2.0
out $ s "hh*8?" # gain 0.6
-- Each hi-hat has 50% chance of playing
```

---

## 2. Time Transformations

### Example 2.1: Fast - Speed Up
Double the pattern speed.

```phonon
cps: 2.0
out $ s "bd sn hh cp" $ fast 2
-- Now plays 8 events per cycle instead of 4
```

### Example 2.2: Slow - Stretch Out
Halve the pattern speed.

```phonon
cps: 2.0
out $ s "bd sn hh cp" $ slow 2
-- Pattern now spans 2 cycles
```

### Example 2.3: Reverse
Play the pattern backwards.

```phonon
cps: 2.0
out $ s "bd sn hh cp" $ rev
-- Plays: cp hh sn bd
```

### Example 2.4: Rotate Left (rotL)
Shift the pattern earlier in time.

```phonon
cps: 2.0
-- Shift by quarter cycle - sn now plays first
out $ s "bd sn hh cp" $ rotL 0.25
```

### Example 2.5: Rotate Right (rotR)
Shift the pattern later in time.

```phonon
cps: 2.0
-- Shift by half cycle
out $ s "bd sn hh cp" $ rotR 0.5
-- Plays: hh cp bd sn
```

### Example 2.6: Swing
Add shuffle/swing feel to even patterns.

```phonon
cps: 2.0
out $ s "hh*8" $ swing 0.2
-- Every other hi-hat is delayed slightly
```

### Example 2.7: Late and Early
Shift events forward or backward in time.

```phonon
cps: 2.0
~kick $ s "bd*4"
~snare $ s "~ sn ~ sn" $ late 0.02  -- Snare slightly behind the beat
out $ ~kick + ~snare
```

### Example 2.8: Zoom
Extract and stretch a portion of the pattern.

```phonon
cps: 2.0
-- Play only the first quarter, stretched to fill the cycle
out $ s "bd sn hh cp" $ zoom 0.0 0.25
```

### Example 2.9: Press and PressBy
Compress pattern into second half of cycle.

```phonon
cps: 2.0
out $ s "bd sn hh cp" $ press
-- Pattern plays in the latter portion of each cycle
```

### Example 2.10: FastGap
Speed up but leave gap.

```phonon
cps: 2.0
out $ s "bd sn hh cp" $ fastGap 2
-- Pattern plays twice as fast, with silence after
```

---

## 3. Structural Transformations

### Example 3.1: Every N Cycles
Apply transformation periodically.

```phonon
cps: 2.0
-- Reverse every 4th cycle
out $ s "bd sn hh cp" $ every 4 rev
```

### Example 3.2: Nested Every
Combine multiple periodic transformations.

```phonon
cps: 2.0
out $ s "bd sn hh cp" $ every 4 rev $ every 3 (fast 2)
-- Reverse every 4th, double speed every 3rd
```

### Example 3.3: Palindrome
Play forward then backward over two cycles.

```phonon
cps: 2.0
out $ s "bd sn hh cp" $ palindrome
-- Cycle 1: bd sn hh cp
-- Cycle 2: cp hh sn bd
```

### Example 3.4: Stutter
Repeat each event multiple times.

```phonon
cps: 2.0
out $ s "bd sn" $ stutter 3
-- Each hit becomes a rapid triplet
```

### Example 3.5: Ply
Another way to repeat events.

```phonon
cps: 2.0
out $ s "bd sn hh cp" $ ply 2
-- Each event plays twice
```

### Example 3.6: Chop
Slice each sample into pieces.

```phonon
cps: 2.0
out $ s "breaks:0" $ chop 8
-- Slices the breakbeat into 8 pieces
```

### Example 3.7: Scramble
Randomize the order of events.

```phonon
cps: 2.0
out $ s "bd sn hh cp" $ scramble 4
-- Order is randomized but consistent per cycle
```

### Example 3.8: Chunk
Apply transform to rotating chunks.

```phonon
cps: 2.0
out $ s "bd sn hh cp hh sn bd cp" $ chunk 4 rev
-- Each cycle, a different quarter is reversed
```

### Example 3.9: Iter
Shift through rotations progressively.

```phonon
cps: 2.0
out $ s "bd sn hh cp" $ iter 4
-- Cycle 0: bd sn hh cp
-- Cycle 1: sn hh cp bd
-- Cycle 2: hh cp bd sn
-- Cycle 3: cp bd sn hh
```

### Example 3.10: Within
Apply transform only within a time range.

```phonon
cps: 2.0
-- Only reverse the first half of each cycle
out $ s "bd sn hh cp" $ within 0.0 0.5 rev
```

### Example 3.11: Slice and Bite
Select specific slices dynamically.

```phonon
cps: 2.0
-- Pattern selects which slice to play
out $ s "breaks:0" $ slice 8 "0 2 3 1 4 6 7 5"
```

### Example 3.12: Jux
Apply transform to right channel only.

```phonon
cps: 2.0
out $ s "bd sn hh cp" $ jux rev
-- Left: bd sn hh cp
-- Right: cp hh sn bd
```

---

## 4. Randomness & Probability

### Example 4.1: Degrade
Drop 50% of events randomly.

```phonon
cps: 2.0
out $ s "hh*16" $ degrade
-- About half the hi-hats will play
```

### Example 4.2: DegradeBy
Control the probability of dropout.

```phonon
cps: 2.0
out $ s "hh*16" $ degradeBy 0.3
-- 30% chance each hit is dropped
```

### Example 4.3: Sometimes
Apply transform 50% of cycles.

```phonon
cps: 2.0
out $ s "bd sn hh cp" $ sometimes rev
-- Half the cycles play reversed
```

### Example 4.4: Rarely and Often
Control transformation frequency.

```phonon
cps: 2.0
out $ s "bd sn hh cp" $ often (fast 2) $ rarely rev
-- Often speeds up (~75%), rarely reverses (~10%)
```

### Example 4.5: Choose
Random selection from options.

```phonon
cps: 2.0
-- Randomly pick a sample each cycle
out $ s "{bd sn hh cp}"
```

### Example 4.6: Ghost Notes
Add quiet copies at offsets.

```phonon
cps: 2.0
out $ s "bd sn" $ ghost # gain 0.3
-- Adds ghost notes at 1/8 and 1/4 cycle offsets
```

### Example 4.7: Humanize with Swing and Degrade
Create natural-feeling patterns.

```phonon
cps: 2.0
~hats $ s "hh*16" $ swing 0.1 $ degradeBy 0.2
out $ ~hats # gain 0.5
```

---

## 5. Synthesis & Effects

### Example 5.1: Basic Oscillator
Simple sine wave.

```phonon
cps: 1.0
out $ sine 440 * 0.3
```

### Example 5.2: Pattern-Controlled Frequency
Mini-notation controls pitch.

```phonon
cps: 2.0
out $ sine "220 330 440 330" * 0.3
```

### Example 5.3: Sawtooth Bass
Rich harmonics for bass.

```phonon
cps: 1.0
~bass $ saw 55
out $ ~bass # lpf 400 1.2 * 0.3
```

### Example 5.4: Low-Pass Filter Sweep
Filter with resonance.

```phonon
cps: 1.0
~synth $ saw 110
out $ ~synth # lpf 2000 2.0 * 0.2
```

### Example 5.5: High-Pass Filter
Remove low frequencies.

```phonon
cps: 2.0
~drums $ s "bd sn hh*4 cp"
out $ ~drums # hpf 200 0.7
```

### Example 5.6: Delay Effect
Echo with feedback.

```phonon
cps: 2.0
~drums $ s "bd ~ sn ~"
out $ ~drums # delay 0.25 0.5 0.3
-- delay time, feedback, wet mix
```

### Example 5.7: Reverb
Add space and depth.

```phonon
cps: 2.0
~drums $ s "bd sn hh cp"
out $ ~drums # reverb 0.6 0.7 0.3
-- room size, damping, mix
```

### Example 5.8: Distortion
Add grit and harmonics.

```phonon
cps: 1.0
~bass $ saw 55
out $ ~bass # distortion 3.0 # lpf 800 0.9 * 0.2
```

### Example 5.9: Bitcrusher
Lo-fi digital effect.

```phonon
cps: 2.0
~drums $ s "bd sn hh cp"
out $ ~drums # bitcrush 8 11025
-- bit depth, sample rate reduction
```

### Example 5.10: Chorus
Thicken the sound.

```phonon
cps: 1.0
~pad $ sine "110 165 220"
out $ ~pad # chorus 1.0 0.5 0.4 * 0.3
```

### Example 5.11: Effect Chaining
Multiple effects in series.

```phonon
cps: 2.0
~drums $ s "bd sn hh*4 cp"
out $ ~drums # lpf 4000 0.8 # delay 0.375 0.4 0.2 # reverb 0.5 0.6 0.2
```

---

## 6. Modulation & LFOs

### Example 6.1: Basic LFO
Sine wave as control signal.

```phonon
cps: 0.5
~lfo # sine 0.5  -- 0.5 Hz = 2 second period
~bass $ saw 55
out $ ~bass # lpf (~lfo * 1500 + 500) 1.0 * 0.2
-- Filter sweeps 500-2000 Hz
```

### Example 6.2: Triangle LFO
Linear ramp up and down.

```phonon
cps: 0.5
~lfo # tri 0.25
~synth $ saw 110
out $ ~synth # lpf (~lfo * 2000 + 400) 1.5 * 0.2
```

### Example 6.3: Stepped Modulation
Pattern as control source.

```phonon
cps: 2.0
~cutoff # "500 1000 2000 1500"
~bass $ saw 55
out $ ~bass # lpf ~cutoff 0.9 * 0.3
```

### Example 6.4: Vibrato Effect
Fast pitch modulation.

```phonon
cps: 1.0
~vib # sine 5  -- 5 Hz vibrato rate
~melody $ sine (440 + ~vib * 10)  -- +/- 10 Hz variation
out $ ~melody * 0.3
```

### Example 6.5: Tremolo Effect
Amplitude modulation.

```phonon
cps: 1.0
~trem # sine 4  -- 4 Hz tremolo
~synth $ saw 220
~shaped $ ~trem * 0.5 + 0.5  -- Convert to 0-1 range
out $ ~synth * ~shaped * 0.2
```

### Example 6.6: Panning LFO
Automatic stereo movement.

```phonon
cps: 2.0
~autopan # sine 0.25  -- Slow pan sweep
~drums $ s "bd sn hh*4 cp"
out $ ~drums # pan ~autopan
```

### Example 6.7: Complex Modulation
Multiple LFOs interacting.

```phonon
cps: 0.5
~slow_lfo # sine 0.1
~fast_lfo # sine 2
~depth $ ~slow_lfo * 500 + 500  -- Slow LFO controls depth
~cutoff $ ~fast_lfo * ~depth + 800
~bass $ saw 55
out $ ~bass # lpf ~cutoff 1.2 * 0.2
```

---

## 7. Complete Musical Examples

### Example 7.1: Minimal Techno

```phonon
cps: 2.2
-- Kick: four on the floor
~kick $ s "bd*4"

-- Hi-hats: 16th notes with degradation
~hats $ s "hh*16" $ degradeBy 0.3 # gain 0.4

-- Snare: offbeat
~snare $ s "~ sn ~ sn"

-- Ride: sparse pattern
~ride $ s "~ ~ ride ~" $ sometimes (rotL 0.25) # gain 0.5

-- Mix and output
~drums $ ~kick + ~hats + ~snare + ~ride
out $ ~drums # reverb 0.3 0.7 0.15
```

### Example 7.2: House Beat with Bass

```phonon
cps: 2.1
-- Drums
~kick $ s "bd*4"
~snare $ s "~ cp ~ cp" # gain 0.8
~hats $ s "[~ hh]*4" $ swing 0.15 # gain 0.5

-- Bass line
~bass $ saw "55 55 82.5 55" # lpf 600 1.2 * 0.25

-- Combine
~mix $ ~kick + ~snare + ~hats + ~bass
out $ ~mix # reverb 0.2 0.6 0.1
```

### Example 7.3: Breakbeat

```phonon
cps: 2.8
-- Chopped break pattern
~break $ s "bd sn [bd bd] sn" $ every 4 (chop 8) # gain 0.9

-- Additional percussion
~perc $ s "~ [rim rim] ~ rim" $ sometimes rev # gain 0.6

-- Combine with effects
~drums $ ~break + ~perc
out $ ~drums # delay 0.125 0.3 0.2 # reverb 0.4 0.6 0.15
```

### Example 7.4: Ambient Drone

```phonon
cps: 0.25
-- Slow-moving drones
~drone1 $ sine 55 * 0.15
~drone2 $ sine 82.5 * 0.1
~drone3 $ sine 110 * 0.08

-- Very slow filter modulation
~lfo # sine 0.05
~filtered $ (~drone1 + ~drone2 + ~drone3) # lpf (~lfo * 2000 + 200) 0.8

-- Heavy reverb
out $ ~filtered # reverb 0.9 0.3 0.6
```

### Example 7.5: Dub Techno

```phonon
cps: 2.0
-- Minimal kick
~kick $ s "bd*4" # gain 0.9

-- Chord stab with long reverb
~chord $ s "~ ~ [stab:0 ~] ~" # reverb 0.8 0.5 0.6 # gain 0.4

-- Dubbed-out hi-hats
~hats $ s "hh*8" $ degradeBy 0.4 # delay 0.375 0.6 0.4 # pan "0.3 0.7" # gain 0.3

-- Rumbling sub
~sub $ sine "55 ~ 55 ~" # lpf 100 0.8 * 0.3

~mix $ ~kick + ~chord + ~hats + ~sub
out $ ~mix
```

### Example 7.6: IDM Glitch

```phonon
cps: 2.5
-- Glitchy drums with lots of transformation
~glitch $ s "bd sn hh cp" $ scramble 4 $ every 3 (fast 2) $ every 5 rev $ degradeBy 0.2

-- Stuttered additional hits
~stut $ s "bd" $ stutter 8 # gain 0.5

-- Bitcrushed percussion
~crushed $ s "hh*8" $ sometimes (chop 4) # bitcrush 6 16000 # gain 0.4

~mix $ ~glitch + ~stut * 0.5 + ~crushed
out $ ~mix # delay 0.125 0.4 0.3
```

### Example 7.7: Acid Bass Pattern

```phonon
cps: 2.2
-- Classic acid bass line
~acid $ saw "55 55 110 82.5 55 165 110 55"

-- Fast filter modulation
~env # sine 4
~cutoff $ ~env * 1500 + 400

-- Apply filter with high resonance
~filtered $ ~acid # lpf ~cutoff 3.0

-- Add distortion
out $ ~filtered # distortion 2.0 * 0.15
```

### Example 7.8: Polyrhythmic Percussion

```phonon
cps: 2.0
-- Different time signatures layered
~three $ s "bd bd bd" # gain 0.8
~four $ s "hh hh hh hh" # gain 0.5
~five $ s "rim rim rim rim rim" # gain 0.4
~seven $ s "cp cp cp cp cp cp cp" # gain 0.3

-- Stack them all
out $ ~three + ~four + ~five + ~seven # reverb 0.4 0.6 0.2
```

### Example 7.9: Melodic Sequence

```phonon
cps: 1.5
-- Melodic pattern (MIDI note-ish)
~notes $ sine "220 277 330 440 392 330 277 220"

-- Arpeggiated chord
~arp $ sine "[220 277 330 440]" $ fast 4 * 0.3

-- Add delay for texture
~delayed $ ~arp # delay 0.25 0.5 0.4

-- Filter modulation
~lfo # sine 0.25
out $ ~delayed # lpf (~lfo * 2000 + 500) 0.8 # reverb 0.5 0.7 0.3
```

### Example 7.10: Full Track Structure

```phonon
cps: 2.0
-- DRUMS
~kick $ s "bd*4"
~snare $ s "~ sn ~ sn"
~hats $ s "hh*8" $ swing 0.1 $ degradeBy 0.15 # gain 0.5
~perc $ s "~ ~ [rim rim] ~" $ every 4 rev # gain 0.4

-- BASS
~bass_notes $ saw "55 55 82.5 55"
~bass $ ~bass_notes # lpf 500 1.0 * 0.25

-- LEAD (appears every 4th cycle)
~lead_notes $ sine "440 330 392 440"
~lead $ ~lead_notes # lpf 2000 0.7 * 0.15

-- PADS
~pad $ sine "[110 165 220]" * 0.08
~pad_filtered $ ~pad # reverb 0.8 0.4 0.5

-- MIXING
~drums $ ~kick + ~snare + ~hats + ~perc
~music $ ~drums + ~bass + ~pad_filtered

-- MASTER CHAIN
out $ ~music # reverb 0.3 0.6 0.15
```

---

## 8. Genre-Specific Patterns

### Example 8.1: Drum and Bass

```phonon
cps: 2.9
~kick $ s "[bd ~] ~ [~ bd] ~"
~snare $ s "~ sn ~ sn" $ every 8 (stutter 4)
~hats $ s "hh*16" $ degradeBy 0.3 # gain 0.4
~break $ ~kick + ~snare + ~hats
out $ ~break # reverb 0.2 0.7 0.1
```

### Example 8.2: UK Garage

```phonon
cps: 2.2
~kick $ s "bd ~ ~ bd ~ bd ~ ~"
~snare $ s "~ ~ sn ~ ~ ~ sn ~"
~hats $ s "hh*8" $ swing 0.2 # gain 0.5
~perc $ s "[~ rim] [rim ~] rim [~ rim]" # gain 0.3
out $ ~kick + ~snare + ~hats + ~perc
```

### Example 8.3: Trap Beat

```phonon
cps: 2.3
~kick $ s "bd ~ ~ ~ bd ~ ~ ~"
~snare $ s "~ ~ ~ ~ sn ~ ~ ~" # gain 0.9
~hats $ s "hh*32" $ degradeBy 0.4 $ every 4 (fast 2) # gain 0.35
~808 $ sine "55 ~ ~ ~ 55 ~ 82.5 ~" * 0.4
out $ ~kick + ~snare + ~hats + ~808 # lpf 8000 0.7
```

### Example 8.4: Lo-Fi Hip Hop

```phonon
cps: 1.4
~kick $ s "bd ~ [~ bd] ~"
~snare $ s "~ sn ~ sn" $ swing 0.15
~hats $ s "hh*8" $ degradeBy 0.25 # gain 0.4
~vinyl $ s "noise" # lpf 200 0.5 * 0.02  -- Simulated vinyl crackle

~drums $ ~kick + ~snare + ~hats
~dusty $ ~drums # lpf 4000 0.8 # bitcrush 12 22050 # reverb 0.4 0.8 0.2 + ~vinyl
out $ ~dusty
```

### Example 8.5: Hardcore/Gabber

```phonon
cps: 3.0
~kick $ s "bd*4" # distortion 4.0 # gain 1.2
~snare $ s "~ sn ~ sn" $ every 4 (stutter 8) # distortion 2.0
~noise $ s "noise" # lpf 8000 0.5 # gain 0.1
out $ ~kick + ~snare + ~noise
```

---

## 9. Advanced Techniques

### Example 9.1: Generative Patterns
Let probability drive the pattern.

```phonon
cps: 2.0
~gen $ s "bd? sn? hh? cp?" $ fast 2 $ every 3 rev
out $ ~gen # reverb 0.5 0.7 0.3
```

### Example 9.2: Euclidean Variations
Multiple euclidean rhythms layered.

```phonon
cps: 2.0
~e3 $ s "bd(3,8)" # gain 0.9
~e5 $ s "sn(5,8)" # gain 0.7
~e7 $ s "hh(7,8)" # gain 0.4
out $ ~e3 + ~e5 + ~e7
```

### Example 9.3: Progressive Transformation
Pattern evolves over time using every.

```phonon
cps: 2.0
~evolve $ s "bd sn hh cp" $ every 2 (fast 1.5) $ every 3 rev $ every 5 (rotL 0.25) $ every 7 (chop 4)
out $ ~evolve
```

### Example 9.4: Call and Response
Two patterns that interlock.

```phonon
cps: 2.0
~call $ s "bd bd ~ ~"
~response $ s "~ ~ sn sn"
~conversation $ ~call + ~response $ every 4 (jux rev)
out $ ~conversation
```

### Example 9.5: Micro-Timing Variations
Create human-like groove.

```phonon
cps: 2.0
~kick $ s "bd*4"
~snare $ s "~ sn ~ sn" $ late 0.015  -- Slightly behind
~hats $ s "hh*8" $ swing 0.08 # gain 0.5
out $ ~kick + ~snare + ~hats
```

### Example 9.6: Feedback Delay Texture
Long delays create evolving textures.

```phonon
cps: 1.0
~sparse $ s "bd ~ ~ ~ sn ~ ~ ~" $ slow 2
~delayed $ ~sparse # delay 0.5 0.8 0.6 # reverb 0.8 0.5 0.4
out $ ~delayed
```

### Example 9.7: Sample Bank Sequencing
Sequence through different samples.

```phonon
cps: 2.0
~kicks $ s "bd:0 bd:1 bd:2 bd:3" $ every 4 scramble 4
~snares $ s "sn:0 ~ sn:1 ~"
out $ ~kicks + ~snares
```

### Example 9.8: Conditional Reverb
Apply effect only sometimes.

```phonon
cps: 2.0
~dry $ s "bd sn hh cp"
~wet $ ~dry # reverb 0.8 0.6 0.5
-- Use buses creatively for conditional effects
out $ ~dry + ~wet * 0.3
```

### Example 9.9: Stereo Width
Jux creates stereo interest.

```phonon
cps: 2.0
~mono $ s "bd sn hh cp"
~wide $ ~mono $ jux (fast 1.01)  -- Slight timing difference
out $ ~wide
```

### Example 9.10: Filter Percussion
Turn any sound into percussion.

```phonon
cps: 2.0
~noise $ s "noise*4"
~pitched $ ~noise # bpf "200 400 800 1600" 10.0 # gain 0.6
out $ ~pitched
```

---

## Quick Reference

### Mini-Notation Operators

| Syntax | Description | Example |
|--------|-------------|---------|
| `a b c` | Sequence | `"bd sn hh"` |
| `a*n` | Repeat n times | `"bd*4"` |
| `~` | Rest/silence | `"bd ~ sn ~"` |
| `[a b]` | Subdivision | `"[bd sn] hh"` |
| `<a b>` | Alternate per cycle | `"<bd sn>"` |
| `a(n,k)` | Euclidean rhythm | `"bd(3,8)"` |
| `a:n` | Sample selection | `"bd:2"` |
| `a?` | Maybe (50%) | `"bd?"` |
| `a?0.3` | Maybe (30%) | `"bd?0.3"` |
| `[a, b]` | Polyrhythm | `"[bd, hh hh]"` |

### Pattern Transformations

| Transform | Description | Example |
|-----------|-------------|---------|
| `fast n` | Speed up | `$ fast 2` |
| `slow n` | Slow down | `$ slow 2` |
| `rev` | Reverse | `$ rev` |
| `rotL n` | Rotate left | `$ rotL 0.25` |
| `rotR n` | Rotate right | `$ rotR 0.5` |
| `every n f` | Apply f every n cycles | `$ every 4 rev` |
| `sometimes f` | Apply ~50% of time | `$ sometimes rev` |
| `degrade` | Drop 50% events | `$ degrade` |
| `degradeBy n` | Drop n% events | `$ degradeBy 0.3` |
| `jux f` | Stereo: f on right | `$ jux rev` |
| `chop n` | Slice into n parts | `$ chop 8` |
| `scramble n` | Randomize order | `$ scramble 4` |
| `stutter n` | Repeat each event | `$ stutter 3` |
| `swing n` | Add swing | `$ swing 0.15` |

### DSP Effects (use `#`)

| Effect | Parameters | Example |
|--------|------------|---------|
| `lpf` | cutoff, resonance | `# lpf 1000 0.8` |
| `hpf` | cutoff, resonance | `# hpf 200 0.7` |
| `delay` | time, feedback, mix | `# delay 0.25 0.5 0.3` |
| `reverb` | room, damping, mix | `# reverb 0.5 0.7 0.3` |
| `distortion` | drive | `# distortion 2.0` |
| `bitcrush` | bits, samplerate | `# bitcrush 8 11025` |
| `chorus` | rate, depth, mix | `# chorus 1.0 0.5 0.3` |
| `gain` | amount | `# gain 0.8` |
| `pan` | position (-1 to 1) | `# pan 0.5` |
| `speed` | rate | `# speed 2.0` |

---

## Tips for Learning

1. **Start simple**: Master basic patterns before adding transformations
2. **One change at a time**: Modify one element, listen, repeat
3. **Use buses**: Name intermediate patterns for clarity
4. **Experiment**: Try unexpected combinations
5. **Listen actively**: Focus on how each change affects the sound
6. **Build a library**: Save patterns that work for future sessions

---

Happy pattern making!
