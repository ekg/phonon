# Techno in Phonon

Techno emerged from Detroit in the mid-1980s, drawing from electro, industrial, and European electronic music. This tutorial covers the fundamentals of creating techno beats, from minimal to industrial styles.

## Core Characteristics

- **Tempo**: 130-150 BPM (cps: 2.17-2.5)
- **Time Signature**: 4/4
- **Key Features**: Driving four-on-the-floor, synthetic percussion, dark atmospheres
- **Sound**: Mechanical, hypnotic, repetitive with subtle variations

## Step 1: The Driving Kick

Techno kicks are often heavier and more processed than house:

```phonon
cps: 2.17

~kick $ s "bd*4" # gain 1.1
out $ ~kick
```

For a harder, punchier sound, layer with a higher sample:

```phonon
cps: 2.17

~kick_low $ s "bd:3*4"
~kick_click $ s "bd:7*4" # gain 0.4 # hpf 2000
out $ ~kick_low + ~kick_click
```

## Step 2: Minimal Hi-Hat Patterns

Techno often uses sparse, syncopated hi-hats rather than constant patterns:

```phonon
cps: 2.17

~kick $ s "bd*4"
~hats $ s "~ ~ hh ~" # gain 0.6
out $ ~kick + ~hats
```

Euclidean patterns work well for techno:

```phonon
cps: 2.17

~kick $ s "bd*4"
~hats $ s "hh(7,16)" # gain 0.5
out $ ~kick + ~hats
```

## Step 3: Claps and Snares

Techno claps are often minimal, hitting on beat 2 or 4, sometimes both:

```phonon
cps: 2.17

~kick $ s "bd*4"
~clap $ s "~ ~ cp ~"  -- Only on beat 3
~hats $ s "hh(7,16)" # gain 0.5
out $ ~kick + ~clap + ~hats
```

Add processing for that industrial edge:

```phonon
cps: 2.17

~kick $ s "bd*4"
~clap $ s "~ ~ cp ~" # reverb 0.2 0.4
~hats $ s "hh(5,16)" # gain 0.5
out $ ~kick + ~clap + ~hats
```

## Step 4: Rimshots and Metallic Percussion

Rim shots and metallic sounds define the techno palette:

```phonon
cps: 2.17

~kick $ s "bd*4"
~rim $ s "[~ rim]*2" # gain 0.6  -- Off-beat rimshots
~hats $ s "hh(5,8)" # gain 0.4
out $ ~kick + ~rim + ~hats
```

Layered metallic percussion:

```phonon
cps: 2.17

~kick $ s "bd*4"
~rim $ s "~ rim ~ rim" # gain 0.5
~cb $ s "~ ~ ~ cb" # gain 0.4  -- Cowbell accent
~hats $ s "hh(9,16)" # gain 0.4
out $ ~kick + ~rim + ~cb + ~hats
```

## Step 5: The Hypnotic Bassline

Techno bass is often a single note, pulsing with the kick:

```phonon
cps: 2.17

~kick $ s "bd*4"
~rim $ s "~ rim ~ rim" # gain 0.5
~hats $ s "hh(5,8)" # gain 0.4

-- Punchy bass following the kick
~bass $ saw 55 # lpf 150 1.5
~bass_shaped $ ~bass * "1 0 1 0"  -- Pulse with kick

out $ ~kick + ~rim + ~hats + ~bass_shaped * 0.4
```

More complex bass pattern:

```phonon
cps: 2.17

~kick $ s "bd*4"
~rim $ s "~ rim ~ rim"
~hats $ s "hh(7,16)" # gain 0.4

-- Dark, modulating bass
~bass $ saw "55 55 73.42 55"
~lfo # sine 0.25
~bass_filtered $ ~bass # lpf (~lfo * 400 + 100) 1.8

out $ ~kick + ~rim + ~hats + ~bass_filtered * 0.35
```

## Step 6: Acid Elements

The TB-303-style acid sound crosses into techno:

```phonon
cps: 2.17

~kick $ s "bd*4"
~hats $ s "hh(5,8)" # gain 0.4

-- Acid line with high resonance
~acid $ saw "55 ~ 55 110 55 ~ 82.5 55"
~accent # "1 0 0.7 1 0.8 0 1 0.6"
~acid_filtered $ ~acid # lpf (~accent * 3000 + 100) 4.0 # distortion 1.2

out $ ~kick + ~hats + ~acid_filtered * 0.25
```

## Step 7: Atmospheric Textures

Techno uses drones, noise, and atmospheric pads:

```phonon
cps: 2.17

~kick $ s "bd*4"
~rim $ s "~ rim ~ rim"
~hats $ s "hh(5,16)" # gain 0.4

~bass $ saw 55 # lpf 120 1.2

-- Dark pad
~pad $ sine 110 + sine 164.81 + sine 220
~pad_dark $ ~pad # lpf 800 0.5 # reverb 0.7 0.95

out $ ~kick + ~rim + ~hats + ~bass * 0.35 + ~pad_dark * 0.08
```

Adding noise for texture:

```phonon
cps: 2.17

~kick $ s "bd*4"
~hats $ s "hh(7,16)" # gain 0.4

-- Filtered noise sweep
~lfo # saw 0.0625  -- 16-beat ramp
~noise $ noise # lpf (~lfo * 4000 + 200) 2.0 # gain 0.15

out $ ~kick + ~hats + ~noise
```

## Step 8: Effects Processing

Techno relies heavily on delay and reverb:

```phonon
cps: 2.17

~kick $ s "bd*4"
~clap $ s "~ ~ cp ~" # delay 0.1875 0.4 0.3  -- Dotted eighth delay
~rim $ s "~ rim ~ ~" # reverb 0.3 0.6
~hats $ s "hh(5,8)" # gain 0.4 # delay 0.25 0.2 0.15

out $ ~kick + ~clap + ~rim + ~hats
```

## Step 9: Complete Minimal Techno Track

```phonon
cps: 2.17

-- Core drums
~kick $ s "bd*4" # gain 1.05
~clap $ s "~ ~ cp ~" # reverb 0.2 0.5
~rim $ s "~ rim ~ rim" # gain 0.5
~hats $ s "hh(7,16)" # gain 0.4

-- Bass
~lfo # sine 0.125
~bass $ saw 55 # lpf (~lfo * 300 + 80) 1.6
~bass_shaped $ ~bass * "1 0.3 0.8 0.3"

-- Atmospheric
~pad $ sine 110 + sine 165
~pad_filtered $ ~pad # lpf 600 0.4 # reverb 0.6 0.9

-- Mix
~drums $ ~kick + ~clap + ~rim + ~hats
out $ ~drums + ~bass_shaped * 0.35 + ~pad_filtered * 0.06
```

## Variations

### Berlin Techno (130-140 BPM)
Raw, industrial, heavy:

```phonon
cps: 2.25

-- Heavy kick with distortion
~kick $ s "bd:3*4" # distortion 0.5

-- Minimal percussion
~clap $ s "~ ~ cp ~" # reverb 0.15 0.3
~hats $ s "hh(3,8)" # gain 0.5

-- Industrial noise stab
~noise_stab $ noise * "~ ~ 1 ~ ~ ~ 0.7 ~" # lpf 3000 1.5 # distortion 2.0

-- Dark droning bass
~bass $ saw 41.2 # lpf 80 1.2

out $ ~kick + ~clap + ~hats + ~noise_stab * 0.15 + ~bass * 0.4
```

### Detroit Techno (125-135 BPM)
More melodic, futuristic:

```phonon
cps: 2.1

~kick $ s "bd*4"
~clap $ s "~ cp ~ cp" # reverb 0.4 0.7
~hats $ s "oh(3,8)" # gain 0.4
~rim $ s "rim(5,16)" # gain 0.4

-- String-like pad
~strings $ saw 130.81 + saw 196 + saw 261.63
~strings_filtered $ ~strings # lpf 3000 0.3 # reverb 0.6 0.9

-- Funky bass
~bass $ saw "55 ~ 73.42 ~ 55 82.5 ~ 55"
~bass_filtered $ ~bass # lpf 600 1.2

out $ ~kick + ~clap + ~hats + ~rim + ~strings_filtered * 0.1 + ~bass_filtered * 0.3
```

### Industrial Techno (140-150 BPM)
Harsh, mechanical, aggressive:

```phonon
cps: 2.4

-- Punishing kick
~kick $ s "bd*4" # distortion 0.8

-- Metallic percussion
~metal $ s "~ cb ~ cb" # distortion 1.5 # gain 0.5
~hats $ s "hh*8" # bitcrush 8 1.0 # gain 0.4

-- Noise bursts
~noise_burst $ noise * "1 ~ ~ ~ 0.5 ~ ~ ~" # hpf 2000 # distortion 2.0

-- Grinding bass
~bass $ saw 36.71 # lpf 60 2.0 # distortion 1.5

out $ ~kick + ~metal + ~hats + ~noise_burst * 0.2 + ~bass * 0.4
```

### Dub Techno (118-128 BPM)
Spacious, echo-laden, hypnotic:

```phonon
cps: 2.0

~kick $ s "bd*4" # reverb 0.1 0.3
~rim $ s "~ rim ~ ~" # delay 0.375 0.6 0.4 # reverb 0.5 0.8
~hats $ s "hh(5,16)" # gain 0.35 # delay 0.25 0.4 0.3

-- Dub chord stabs
~chord $ sine 130.81 + sine 164.81 + sine 196
~stab $ ~chord * "~ 1 ~ ~ ~ 0.6 ~ ~" # delay 0.375 0.5 0.35 # reverb 0.6 0.9

-- Sub bass
~bass $ sine 55 # lpf 100 0.8
~bass_pulse $ ~bass * "1 ~ 0.7 ~"

out $ ~kick + ~rim + ~hats + ~stab * 0.15 + ~bass_pulse * 0.4
```

## Essential Techniques

### Euclidean Polyrhythms
```phonon
cps: 2.17

~kick $ s "bd*4"
~rim $ s "rim(5,8)"      -- 5 in 8
~hats $ s "hh(7,16)"     -- 7 in 16
~perc $ s "cp(3,8,1)"    -- 3 in 8, rotated
out $ ~kick + ~rim * 0.5 + ~hats * 0.4 + ~perc * 0.6
```

### Pattern Transformations for Builds
```phonon
cps: 2.17

~kick $ s "bd*4"
~hats $ s "hh*4" $ every 2 (fast 2) $ every 4 (fast 4)  -- Build intensity
~rim $ s "~ rim ~ rim"
out $ ~kick + ~hats * 0.5 + ~rim
```

### Probability for Variation
```phonon
cps: 2.17

~kick $ s "bd*4"
~hats $ s "hh*8" $ degradeBy 0.2  -- 20% random dropout
~rim $ s "rim*4" $ sometimes (fast 2)  -- Sometimes double
out $ ~kick + ~hats * 0.4 + ~rim * 0.5
```

### Filter Automation for Movement
```phonon
cps: 2.17

~kick $ s "bd*4"
~hats $ s "hh(7,16)" # gain 0.4

-- 32-beat filter sweep
~lfo # saw 0.03125
~bass $ saw 55
~bass_sweep $ ~bass # lpf (~lfo * 2000 + 60) 1.5

out $ ~kick + ~hats + ~bass_sweep * 0.4
```

## Sound Design Tips

### Layered Kicks
```phonon
cps: 2.17

~kick_sub $ sine 55 * "1 0 0 0" # lpf 80 0.5  -- Sub layer
~kick_punch $ s "bd:3*4"  -- Punch layer
~kick_click $ s "bd:7*4" # hpf 3000 # gain 0.3  -- Click layer

out $ ~kick_sub * 0.5 + ~kick_punch + ~kick_click
```

### Creating Risers
```phonon
cps: 2.17

~kick $ s "bd*4"
~hats $ s "hh*8" # gain 0.4

-- Noise riser
~riser_lfo # saw 0.0625  -- 16-beat rise
~riser $ noise # lpf (~riser_lfo * 8000 + 500) 2.0 # gain (~riser_lfo * 0.3)

out $ ~kick + ~hats + ~riser
```

## Sample Recommendations

For authentic techno sounds:
- `bd:3`, `bd:7` - Punchy, processed kicks
- `rim` - Essential for techno percussion
- `cb` - Cowbell for accents
- `hh` - Closed hi-hats
- `oh` - Open hats (use sparingly)
- `cp` - Claps (often processed)

## Next Steps

1. Experiment with tempo (faster = harder)
2. Layer different percussion patterns
3. Create long filter sweeps for builds
4. Use delay and reverb for space
5. Practice arrangement with muting buses
6. Try adding distortion for industrial textures

Keep it hypnotic!
