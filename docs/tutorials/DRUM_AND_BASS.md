# Drum & Bass in Phonon

Drum & Bass (D&B, DnB) emerged from the UK rave scene in the early 1990s, evolving from jungle and breakbeat hardcore. This tutorial covers creating the fast, syncopated rhythms and heavy basslines that define the genre.

## Core Characteristics

- **Tempo**: 160-180 BPM (cps: 2.67-3.0)
- **Time Signature**: 4/4 (but feels like half-time due to snare placement)
- **Key Features**: Syncopated "two-step" beat, heavy sub-bass, chopped breakbeats
- **Sound**: Fast hi-hats, punchy kicks and snares, rolling basslines

## The Two-Step Foundation

D&B uses a distinctive "two-step" pattern where the kick and snare create syncopation:

```
Beat:  1 . . . 2 . . . 3 . . . 4 . . .
Kick:  X . . . . . X . . . . . . . . .
Snare: . . . . X . . . . . . . X . . .
```

The kick on 1 and the "and" of 2, snare on 2 and 4 creates the rolling feel.

## Step 1: The Basic Two-Step

```phonon
cps: 2.83  -- 170 BPM

~kick $ s "bd ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~ ~"
~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
out $ ~kick + ~snare
```

Simplified notation using rests:

```phonon
cps: 2.83

~kick $ s "bd ~ ~ ~ ~ ~ bd ~"
~snare $ s "~ ~ sn ~ ~ ~ ~ sn"
out $ ~kick + ~snare
```

## Step 2: Adding Hi-Hats

Fast hi-hats are essential to D&B energy:

```phonon
cps: 2.83

~kick $ s "bd ~ ~ ~ ~ ~ bd ~"
~snare $ s "~ ~ sn ~ ~ ~ ~ sn"
~hats $ s "hh*16" # gain 0.4
out $ ~kick + ~snare + ~hats
```

With velocity variation for groove:

```phonon
cps: 2.83

~kick $ s "bd ~ ~ ~ ~ ~ bd ~"
~snare $ s "~ ~ sn ~ ~ ~ ~ sn"
~hats $ s "hh*16" # gain "0.5 0.3 0.4 0.3 0.5 0.3 0.4 0.35 0.5 0.3 0.4 0.3 0.5 0.3 0.45 0.35"
out $ ~kick + ~snare + ~hats
```

## Step 3: Ghost Snares

Ghost notes add complexity and drive:

```phonon
cps: 2.83

~kick $ s "bd ~ ~ ~ ~ ~ bd ~"
~snare_main $ s "~ ~ sn ~ ~ ~ ~ sn"
~snare_ghost $ s "~ sn ~ ~ ~ sn ~ ~" # gain 0.25  -- Quiet ghost hits
~hats $ s "hh*16" # gain 0.4
out $ ~kick + ~snare_main + ~snare_ghost + ~hats
```

## Step 4: The Amen Break Vibe

The classic "amen break" feel:

```phonon
cps: 2.83

-- Amen-style pattern
~kick $ s "bd ~ ~ ~ bd ~ bd ~"
~snare $ s "~ ~ sn ~ ~ sn ~ sn"
~hats $ s "hh*16" # gain 0.35
~ride $ s "~ ride ~ ride" # gain 0.3
out $ ~kick + ~snare + ~hats + ~ride
```

More complex breakbeat pattern:

```phonon
cps: 2.83

~kick $ s "bd ~ ~ bd ~ ~ bd ~"
~snare $ s "~ ~ sn ~ sn ~ ~ sn"
~hats $ s "hh*16" # gain 0.35
out $ ~kick + ~snare + ~hats
```

## Step 5: Sub Bass

D&B bass is deep and powerful:

```phonon
cps: 2.83

~kick $ s "bd ~ ~ ~ ~ ~ bd ~"
~snare $ s "~ ~ sn ~ ~ ~ ~ sn"
~hats $ s "hh*16" # gain 0.4

-- Sub bass following root notes
~sub $ sine 36.71 # lpf 80 0.9  -- Low D
~sub_pattern $ ~sub * "1 ~ ~ 0.7 ~ ~ 1 ~"
out $ ~kick + ~snare + ~hats + ~sub_pattern * 0.5
```

## Step 6: Reese Bass

The iconic "Reese" bass sound uses detuned saws:

```phonon
cps: 2.83

~kick $ s "bd ~ ~ ~ ~ ~ bd ~"
~snare $ s "~ ~ sn ~ ~ ~ ~ sn"
~hats $ s "hh*16" # gain 0.35

-- Reese bass (detuned saws)
~reese1 $ saw 36.71
~reese2 $ saw 36.91  -- Slightly detuned
~reese $ ~reese1 + ~reese2
~reese_filtered $ ~reese # lpf 400 1.8 # distortion 0.8
~reese_pattern $ ~reese_filtered * "1 ~ ~ ~ ~ ~ 1 ~"

out $ ~kick + ~snare + ~hats + ~reese_pattern * 0.25
```

## Step 7: Wobble Bass

The dubstep-influenced wobble:

```phonon
cps: 2.83

~kick $ s "bd ~ ~ ~ ~ ~ bd ~"
~snare $ s "~ ~ sn ~ ~ ~ ~ sn"
~hats $ s "hh*16" # gain 0.35

-- Wobble bass
~bass $ saw 36.71
~lfo # sine 4  -- Wobble rate
~wobble $ ~bass # lpf (~lfo * 1500 + 200) 2.0
~wobble_pattern $ ~wobble * "1 ~ 1 ~ ~ ~ 1 ~"

out $ ~kick + ~snare + ~hats + ~wobble_pattern * 0.3
```

## Step 8: Rolling Basslines

The classic "rolling" D&B bass:

```phonon
cps: 2.83

~kick $ s "bd ~ ~ ~ ~ ~ bd ~"
~snare $ s "~ ~ sn ~ ~ ~ ~ sn"
~hats $ s "hh*16" # gain 0.35

-- Rolling 16th note bass
~bass $ saw "36.71 ~ 36.71 48.99 ~ 36.71 ~ 55"
~bass_filtered $ ~bass # lpf 300 1.5
out $ ~kick + ~snare + ~hats + ~bass_filtered * 0.3
```

## Step 9: Atmosphere and Pads

D&B often uses atmospheric elements:

```phonon
cps: 2.83

~kick $ s "bd ~ ~ ~ ~ ~ bd ~"
~snare $ s "~ ~ sn ~ ~ ~ ~ sn" # reverb 0.2 0.5
~hats $ s "hh*16" # gain 0.35

~sub $ sine 36.71 * "1 ~ ~ 0.7 ~ ~ 1 ~" # lpf 80 0.9

-- Atmospheric pad
~pad $ sine 146.83 + sine 220 + sine 293.66  -- D minor chord
~pad_atmo $ ~pad # lpf 1500 0.3 # reverb 0.7 0.95

out $ ~kick + ~snare + ~hats + ~sub * 0.4 + ~pad_atmo * 0.08
```

## Step 10: Complete D&B Track

```phonon
cps: 2.83

-- Drums
~kick $ s "bd ~ ~ ~ ~ ~ bd ~"
~snare_main $ s "~ ~ sn ~ ~ ~ ~ sn" # reverb 0.15 0.4
~snare_ghost $ s "~ sn ~ ~ ~ sn ~ ~" # gain 0.2
~hats $ s "hh*16" # gain "0.45 0.3 0.4 0.3 0.45 0.3 0.4 0.35 0.45 0.3 0.4 0.3 0.45 0.3 0.5 0.35"

-- Reese bass
~reese $ saw 36.71 + saw 36.91
~reese_filtered $ ~reese # lpf 350 1.5 # distortion 0.6
~reese_pattern $ ~reese_filtered * "1 ~ ~ 0.7 ~ ~ 1 ~"

-- Sub layer
~sub $ sine 36.71 * "1 ~ ~ 0.5 ~ ~ 1 ~" # lpf 70 0.8

-- Atmosphere
~pad $ sine 146.83 + sine 220
~pad_filtered $ ~pad # lpf 1200 0.3 # reverb 0.6 0.9

-- Mix
~drums $ ~kick + ~snare_main + ~snare_ghost + ~hats
out $ ~drums + ~reese_pattern * 0.22 + ~sub * 0.35 + ~pad_filtered * 0.06
```

## Variations

### Liquid D&B (170-176 BPM)
Smooth, musical, melodic:

```phonon
cps: 2.88  -- 173 BPM

~kick $ s "bd ~ ~ ~ ~ ~ bd ~"
~snare $ s "~ ~ sn ~ ~ ~ ~ sn" # reverb 0.3 0.6
~hats $ s "hh*8" # gain 0.4  -- Less frenetic
~ride $ s "ride*4" # gain 0.3

-- Smooth sub bass
~sub $ sine "36.71 ~ 36.71 ~ 41.2 ~ 36.71 ~" # lpf 90 0.8

-- Musical piano-like element
~keys $ sine 293.66 + sine 369.99 + sine 440  -- D major
~keys_stab $ ~keys * "~ 1 ~ ~ ~ 0.7 ~ ~" # lpf 2500 0.4 # reverb 0.5 0.85

~drums $ ~kick + ~snare + ~hats + ~ride
out $ ~drums + ~sub * 0.4 + ~keys_stab * 0.12
```

### Jump Up (175-180 BPM)
Aggressive, dancefloor-focused:

```phonon
cps: 2.92  -- 175 BPM

~kick $ s "bd ~ ~ ~ ~ ~ bd ~"
~snare $ s "~ ~ sn ~ ~ ~ ~ sn" # distortion 0.3
~hats $ s "hh*16" # gain 0.4

-- Aggressive bassline
~bass $ saw "36.71 36.71 ~ 36.71 48.99 ~ 36.71 55"
~bass_nasty $ ~bass # lpf 500 2.5 # distortion 1.5

out $ ~kick + ~snare + ~hats + ~bass_nasty * 0.25
```

### Neurofunk (172-178 BPM)
Technical, complex, robotic:

```phonon
cps: 2.92

~kick $ s "bd ~ ~ bd ~ ~ bd ~"
~snare $ s "~ ~ sn ~ sn ~ ~ sn"
~hats $ s "hh*16" # gain 0.35

-- Complex modulated bass
~bass $ saw 36.71
~lfo1 # sine 6
~lfo2 # saw 3
~neuro $ ~bass # lpf (~lfo1 * 800 + ~lfo2 * 400 + 200) 3.0 # distortion 1.2
~neuro_pattern $ ~neuro * "1 ~ 1 ~ 0.7 ~ 1 ~"

out $ ~kick + ~snare + ~hats + ~neuro_pattern * 0.25
```

### Jungle (160-170 BPM)
Breakbeat-focused, ragga influences:

```phonon
cps: 2.75  -- 165 BPM

-- Chopped break pattern
~kick $ s "bd ~ bd ~ ~ ~ bd ~"
~snare $ s "~ ~ sn ~ sn ~ ~ sn"
~hats $ s "[hh hh] hh [hh hh hh] hh" # gain 0.4

-- Deep dub bass
~bass $ sine "36.71 ~ ~ 36.71 ~ 41.2 ~ ~" # lpf 100 0.9

-- Dub stab
~stab $ saw 146.83 + saw 220
~stab_dub $ ~stab * "~ 1 ~ ~ ~ ~ ~ ~" # delay 0.1875 0.5 0.4 # reverb 0.4 0.7

~drums $ ~kick + ~snare + ~hats
out $ ~drums + ~bass * 0.45 + ~stab_dub * 0.15
```

### Halftime (80-90 BPM feel)
Slow, heavy, cinematic:

```phonon
cps: 2.83  -- Still 170 BPM but arranged for halftime feel

~kick $ s "bd ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~"
~snare $ s "~ ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~" # reverb 0.3 0.6
~hats $ s "hh*8" # gain 0.35

-- Massive sub
~sub $ sine 27.5 # lpf 60 0.9  -- Very low A
~sub_pattern $ ~sub * "1 ~ ~ ~ ~ ~ ~ ~ 0.7 ~ ~ ~ ~ ~ ~ ~"

-- Cinematic pad
~pad $ sine 110 + sine 164.81 + sine 220
~pad_huge $ ~pad # lpf 800 0.4 # reverb 0.8 0.97

out $ ~kick + ~snare + ~hats + ~sub_pattern * 0.5 + ~pad_huge * 0.1
```

## Essential Techniques

### The Classic Two-Step Variations
```phonon
cps: 2.83

-- Variation 1: Kick on 1 and 2-and
~kick1 $ s "bd ~ ~ ~ ~ ~ bd ~"
~snare1 $ s "~ ~ sn ~ ~ ~ ~ sn"

-- Variation 2: Double kick
~kick2 $ s "bd ~ ~ ~ bd ~ bd ~"
~snare2 $ s "~ ~ sn ~ ~ ~ ~ sn"

-- Variation 3: Offbeat snare
~kick3 $ s "bd ~ ~ ~ ~ ~ bd ~"
~snare3 $ s "~ ~ sn ~ ~ sn ~ sn"

out $ ~kick2 + ~snare2 + s "hh*16" * 0.4
```

### Building Energy with Hi-Hat Patterns
```phonon
cps: 2.83

~kick $ s "bd ~ ~ ~ ~ ~ bd ~"
~snare $ s "~ ~ sn ~ ~ ~ ~ sn"

-- Building from 8ths to 16ths
~hats_8 $ s "hh*8" # gain 0.4
~hats_16 $ s "hh*16" # gain 0.4

out $ ~kick + ~snare + ~hats_16
```

### Euclidean Rhythms for Breaks
```phonon
cps: 2.83

~kick $ s "bd(5,16)"  -- 5 kicks in 16 steps
~snare $ s "sn(3,8,1)"  -- 3 snares, rotated
~hats $ s "hh*16" # gain 0.35
out $ ~kick + ~snare + ~hats
```

### Processing Breaks
```phonon
cps: 2.83

~kick $ s "bd ~ ~ ~ ~ ~ bd ~"
~snare $ s "~ ~ sn ~ ~ ~ ~ sn"
~hats $ s "hh*16" # gain 0.35

-- Compress the drums together
~drums $ ~kick + ~snare + ~hats
~drums_processed $ ~drums # compressor -10 4.0 0.005 0.05 2.0

out $ ~drums_processed
```

### Bass Sound Design
```phonon
cps: 2.83

~kick $ s "bd ~ ~ ~ ~ ~ bd ~"
~snare $ s "~ ~ sn ~ ~ ~ ~ sn"
~hats $ s "hh*16" # gain 0.35

-- Layered bass: sub + mid
~sub $ sine 36.71 # lpf 70 0.8
~mid $ saw 36.71 # hpf 100 # lpf 600 1.5 # distortion 0.8
~bass_layered $ ~sub * 0.5 + ~mid * 0.3
~bass_pattern $ ~bass_layered * "1 ~ ~ 0.7 ~ ~ 1 ~"

out $ ~kick + ~snare + ~hats + ~bass_pattern
```

## Sample Recommendations

For authentic D&B sounds:
- `bd` - Punchy kicks (try `bd:3`, `bd:5`)
- `sn` - Snappy snares with some room
- `hh` - Tight, crisp hi-hats
- `ride` - Ride cymbals for jungle vibes
- `amen` - If available, classic amen break chops

## Production Tips

1. **Layer your bass** - sub underneath, midrange on top
2. **Ghost snares** add groove and drive
3. **Hi-hat velocity** variation is essential
4. **Leave space** around the snare hits
5. **Reverb on snares** creates space
6. **Sidechain** the bass to the kick subtly
7. **Less is more** with atmosphere - don't clutter

## Common Tempo Ranges

- **Liquid**: 170-176 BPM
- **Dancefloor**: 174-178 BPM
- **Jump Up**: 175-180 BPM
- **Neurofunk**: 172-178 BPM
- **Jungle**: 160-170 BPM

## Next Steps

1. Master the two-step pattern variations
2. Experiment with bass processing
3. Create complex hi-hat patterns
4. Layer drums for impact
5. Practice transitions and drops
6. Study classic D&B tracks

Keep it rolling!
