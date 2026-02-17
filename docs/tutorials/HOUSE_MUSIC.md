# House Music in Phonon

House music originated in Chicago in the early 1980s and forms the foundation for much of modern electronic dance music. This tutorial teaches you to create house beats, basslines, and full tracks using Phonon.

## Core Characteristics

- **Tempo**: 120-130 BPM (cps: 2.0-2.17)
- **Time Signature**: 4/4
- **Key Feature**: Four-on-the-floor kick drum
- **Elements**: Steady hi-hats, claps/snares on 2 and 4, off-beat percussion

## Step 1: The Four-on-the-Floor Foundation

The defining element of house is the kick drum hitting every beat:

```phonon
cps: 2.0

out $ s "bd*4"
```

This gives us the iconic "boots and cats" pulse.

## Step 2: Adding Hi-Hats

Classic house uses eighth or sixteenth note hi-hats:

```phonon
cps: 2.0

~kick $ s "bd*4"
~hats $ s "hh*8"
out $ ~kick + ~hats
```

For a more open feel, try off-beat open hats:

```phonon
cps: 2.0

~kick $ s "bd*4"
~closed $ s "hh*8" # gain 0.5
~open $ s "~ oh ~ oh ~ oh ~ oh" # gain 0.7
out $ ~kick + ~closed + ~open
```

## Step 3: Claps and Snares

House typically places claps or snares on beats 2 and 4:

```phonon
cps: 2.0

~kick $ s "bd*4"
~clap $ s "~ cp ~ cp"
~hats $ s "hh*8"
out $ ~kick + ~clap + ~hats
```

For variation, layer a snare with the clap:

```phonon
cps: 2.0

~kick $ s "bd*4"
~clap $ s "~ cp ~ cp"
~snare $ s "~ sn ~ sn" # gain 0.4
~hats $ s "hh*8"
out $ ~kick + ~clap + ~snare + ~hats
```

## Step 4: Adding Groove with Swing

House often has a slight shuffle to give it that human feel:

```phonon
cps: 2.0

~kick $ s "bd*4"
~clap $ s "~ cp ~ cp"
~hats $ s "hh*16" $ swing 0.1
out $ ~kick + ~clap + ~hats
```

## Step 5: Percussion Layers

Add shakers, tambourines, or congas for texture:

```phonon
cps: 2.0

~kick $ s "bd*4"
~clap $ s "~ cp ~ cp"
~hats $ s "hh*8"
~shaker $ s "shaker*16" # gain 0.3
~rim $ s "~ ~ rim ~" # gain 0.4
out $ ~kick + ~clap + ~hats + ~shaker + ~rim
```

## Step 6: Classic House Bassline

House basslines are often syncopated and funky:

```phonon
cps: 2.0

-- Drums
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp"
~hats $ s "hh*8"

-- Bass: syncopated pattern
~bass $ saw "55 ~ 55 82.5 ~ 55 ~ 110"
~bass_filtered $ ~bass # lpf 400 1.2

out $ ~kick + ~clap + ~hats + ~bass_filtered * 0.4
```

A more driving bass pattern:

```phonon
cps: 2.0

~kick $ s "bd*4"
~clap $ s "~ cp ~ cp"
~hats $ s "hh*8"

-- Pumping bass that follows the kick
~bass $ saw "55*4" # lpf 300 1.0
~bass_shaped $ ~bass * "1 0.6 0.8 0.6"

out $ ~kick + ~clap + ~hats + ~bass_shaped * 0.3
```

## Step 7: Chord Stabs

House uses chord stabs, often filtered and with reverb:

```phonon
cps: 2.0

~kick $ s "bd*4"
~clap $ s "~ cp ~ cp"
~hats $ s "hh*8"

-- Chord stab pattern (C minor chord)
~chord $ saw "130.81" + saw "155.56" + saw "196"
~stab $ ~chord * "~ 1 ~ 0.5 ~ 1 ~ ~" # lpf 2000 0.8

out $ ~kick + ~clap + ~hats + ~stab * 0.2
```

## Step 8: Filter Modulation (The House Sound)

Sweeping filters are essential to house:

```phonon
cps: 2.0

~kick $ s "bd*4"
~clap $ s "~ cp ~ cp"
~hats $ s "hh*8"

-- Bass with LFO-controlled filter
~bass $ saw 55
~lfo # sine 0.125  -- Slow sweep over 8 beats
~cutoff $ ~lfo * 1500 + 500
~bass_filtered $ ~bass # lpf ~cutoff 1.5

out $ ~kick + ~clap + ~hats + ~bass_filtered * 0.4
```

## Step 9: Effects - Reverb and Delay

House uses space and delay for atmosphere:

```phonon
cps: 2.0

~kick $ s "bd*4"
~clap $ s "~ cp ~ cp" # reverb 0.4 0.7
~hats $ s "hh*8" # delay 0.375 0.3 0.2
~oh $ s "~ oh ~ ~" # reverb 0.6 0.9

~bass $ saw 55 # lpf 400 1.0

out $ ~kick + ~clap + ~hats + ~oh + ~bass * 0.3
```

## Step 10: Complete House Track

Putting it all together:

```phonon
cps: 2.0

-- Drums
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp" # reverb 0.3 0.6
~hats $ s "hh*16" $ swing 0.08 # gain 0.5
~oh $ s "~ oh ~ oh ~ oh ~ oh" # gain 0.4 # reverb 0.4 0.7

-- Bass
~lfo # sine 0.0625  -- 16-beat sweep
~bass $ saw "55 ~ 55 82.5 ~ 55 ~ 55"
~bass_filtered $ ~bass # lpf (~lfo * 1200 + 400) 1.3

-- Chord stabs
~chord $ saw 130.81 + saw 155.56 + saw 196
~stab $ ~chord * "~ 1 ~ ~ ~ 0.7 ~ ~" # lpf 3000 0.6 # reverb 0.5 0.8

-- Mix
~drums $ ~kick + ~clap + ~hats + ~oh
out $ ~drums + ~bass_filtered * 0.35 + ~stab * 0.15
```

## Variations

### Deep House (118-125 BPM)
Slower, more atmospheric, heavier on chords:

```phonon
cps: 1.97

~kick $ s "bd*4"
~clap $ s "~ cp ~ ~" # reverb 0.5 0.85
~hats $ s "hh*8" # gain 0.4
~rim $ s "~ ~ ~ rim" # gain 0.3

-- Deep, warm bass
~bass $ sine 55 # lpf 200 0.8
~bass_pattern $ ~bass * "1 ~ 0.7 ~ 0.8 ~ ~ 0.6"

-- Lush pad
~pad $ sine 130.81 + sine 155.56 + sine 196
~pad_slow $ ~pad # reverb 0.7 0.95 # lpf 1500 0.5

out $ ~kick + ~clap + ~hats + ~rim + ~bass_pattern * 0.4 + ~pad_slow * 0.15
```

### Acid House (120-130 BPM)
Characterized by the squelchy TB-303 bassline:

```phonon
cps: 2.0

~kick $ s "bd*4"
~clap $ s "~ cp ~ cp"
~hats $ s "hh*16" # gain 0.5

-- Acid bass: resonant filter with fast modulation
~bass $ saw "55 55 110 55 82.5 55 110 55"
~accent # "1 0.5 1 0.7 1 0.5 0.8 1"
~acid_cutoff $ ~accent * 2000 + 200
~acid $ ~bass # lpf ~acid_cutoff 3.5 # distortion 1.5

out $ ~kick + ~clap + ~hats + ~acid * 0.25
```

### Progressive House (126-132 BPM)
Longer builds, more complex arrangements:

```phonon
cps: 2.1

~kick $ s "bd*4"
~clap $ s "~ ~ ~ cp"  -- Clap on beat 4 only
~hats $ s "hh*8"
~ride $ s "~ ride*4" # gain 0.4

-- Arpeggiated synth
~arp $ sine "130.81 196 261.63 196" $ fast 2
~arp_filtered $ ~arp # lpf 4000 0.5 # delay 0.25 0.4 0.3

-- Building pad
~lfo_slow # sine 0.03125  -- Very slow 32-beat sweep
~pad $ saw 130.81 + saw 164.81 + saw 196
~pad_filtered $ ~pad # lpf (~lfo_slow * 3000 + 500) 0.7 # reverb 0.6 0.9

out $ ~kick + ~clap + ~hats + ~ride + ~arp_filtered * 0.15 + ~pad_filtered * 0.1
```

## Essential Techniques

### Euclidean Rhythms for Variation
```phonon
cps: 2.0

~kick $ s "bd*4"
~perc $ s "rim(5,8)" # gain 0.5  -- 5 hits in 8 steps
~hats $ s "hh(11,16)"  -- 11 hits in 16 steps
out $ ~kick + ~perc + ~hats
```

### Building Tension with `every`
```phonon
cps: 2.0

~kick $ s "bd*4"
~hats $ s "hh*8" $ every 4 (fast 2)  -- Double speed every 4th cycle
~clap $ s "~ cp ~ cp" $ every 8 (stutter 2)  -- Stutter every 8th
out $ ~kick + ~hats + ~clap
```

### Creating Drops with Dynamics
```phonon
cps: 2.0

~kick $ s "bd*4"
~hats $ s "hh*16" # gain "0.3 0.5 0.4 0.6 0.3 0.5 0.4 0.7"
~clap $ s "~ cp ~ cp"

-- Filter drop simulation
~lfo # saw 0.125  -- Ramp up over 8 beats
~bass $ saw 55 # lpf (~lfo * 3000 + 200) 1.5

out $ ~kick + ~hats + ~clap + ~bass * 0.4
```

## Sample Recommendations

For authentic house sounds, use these sample banks:
- `bd` - Kick drums (try `bd:3`, `bd:7` for different characters)
- `cp` - Claps (essential for house)
- `hh` - Hi-hats (closed)
- `oh` - Open hats
- `sn` - Snares
- `rim` - Rim shots

## Next Steps

1. Experiment with different BPM (try 124-128 for classic house feel)
2. Layer multiple percussion elements
3. Create filter automation with LFOs
4. Add vocal chops using sample patterns
5. Learn arrangement by muting/unmuting buses

Happy producing!
