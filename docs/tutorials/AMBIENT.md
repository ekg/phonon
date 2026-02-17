# Ambient and Downtempo in Phonon

Ambient music focuses on atmosphere, texture, and mood rather than rhythm. This tutorial covers creating evolving soundscapes, downtempo beats, and meditative compositions using Phonon's synthesis and pattern capabilities.

## Core Characteristics

### Ambient
- **Tempo**: Often no fixed tempo, or very slow (40-80 BPM)
- **Key Features**: Long evolving textures, space, minimal beats
- **Sound**: Pads, drones, field recordings, subtle movement

### Downtempo/Chillout
- **Tempo**: 60-100 BPM (cps: 1.0-1.67)
- **Key Features**: Relaxed grooves, atmospheric production
- **Sound**: Warm, spacious, often sample-based

## Part 1: Pure Ambient (No Beats)

### Step 1: Simple Drone

Start with a sustained tone:

```phonon
cps: 0.5

~drone $ sine 110
out $ ~drone # reverb 0.8 0.95 # lpf 2000 0.3
```

Add subtle movement with filtering:

```phonon
cps: 0.5

~drone $ sine 110
~lfo # sine 0.05  -- Very slow modulation
~drone_filtered $ ~drone # lpf (~lfo * 1000 + 800) 0.4 # reverb 0.8 0.95

out $ ~drone_filtered * 0.3
```

### Step 2: Layered Drones

Multiple frequencies create richness:

```phonon
cps: 0.5

~root $ sine 55
~fifth $ sine 82.5     -- Perfect fifth
~octave $ sine 110
~drone $ ~root + ~fifth * 0.7 + ~octave * 0.5

~lfo # sine 0.03
~drone_evolving $ ~drone # lpf (~lfo * 1500 + 500) 0.3 # reverb 0.85 0.97

out $ ~drone_evolving * 0.2
```

### Step 3: Detuned Drones

Slight detuning creates natural beating:

```phonon
cps: 0.5

~osc1 $ sine 110
~osc2 $ sine 110.5   -- Slightly sharp
~osc3 $ sine 109.5   -- Slightly flat
~drone $ ~osc1 + ~osc2 * 0.8 + ~osc3 * 0.8

~drone_warm $ ~drone # lpf 1500 0.2 # reverb 0.9 0.98

out $ ~drone_warm * 0.15
```

### Step 4: Pad Textures

Saw waves create richer harmonics:

```phonon
cps: 0.5

~pad $ saw 55 + saw 82.5 + saw 110

~lfo_slow # sine 0.02
~lfo_fast # sine 0.15

~pad_filtered $ ~pad # lpf (~lfo_slow * 1000 + (~lfo_fast * 200) + 400) 0.3
~pad_spacious $ ~pad_filtered # reverb 0.85 0.97

out $ ~pad_spacious * 0.08
```

### Step 5: Evolving Chord Progressions

Slow harmonic movement:

```phonon
cps: 0.125  -- Very slow: 1 cycle = 8 seconds

-- Alternating between two chords
~chord1 $ sine 110 + sine 138.59 + sine 164.81  -- A minor
~chord2 $ sine 98 + sine 123.47 + sine 146.83   -- G major

~progression $ ~chord1 * "<1 0>" + ~chord2 * "<0 1>"
~progression_filtered $ ~progression # lpf 1500 0.3 # reverb 0.9 0.97

out $ ~progression_filtered * 0.12
```

### Step 6: Noise Textures

Filtered noise creates natural ambience:

```phonon
cps: 0.5

~drone $ sine 55 + sine 110
~drone_filtered $ ~drone # lpf 800 0.3

-- Noise as texture
~lfo # sine 0.04
~texture $ noise # lpf (~lfo * 500 + 200) 0.8 # reverb 0.9 0.95

out $ ~drone_filtered * 0.15 + ~texture * 0.03
```

### Step 7: Complete Ambient Piece

```phonon
cps: 0.0625  -- 1 cycle = 16 seconds

-- Root drone with beating
~drone1 $ sine 55
~drone2 $ sine 55.2
~drone3 $ sine 54.8
~drone $ ~drone1 + ~drone2 * 0.6 + ~drone3 * 0.6

-- Harmonic layer
~harmonics $ sine 110 + sine 164.81 + sine 220
~lfo1 # sine 0.015
~harmonics_filtered $ ~harmonics # lpf (~lfo1 * 1200 + 600) 0.25

-- High shimmer
~shimmer $ sine 880 + sine 1108.73
~lfo2 # sine 0.03
~shimmer_filtered $ ~shimmer # lpf (~lfo2 * 2000 + 1000) 0.2 # reverb 0.95 0.99

-- Noise texture
~lfo3 # sine 0.02
~texture $ noise # lpf (~lfo3 * 400 + 100) 0.5

-- Mix with heavy reverb
~mix $ ~drone * 0.15 + ~harmonics_filtered * 0.08 + ~shimmer_filtered * 0.02 + ~texture * 0.015
out $ ~mix # reverb 0.9 0.98
```

## Part 2: Downtempo

### Step 1: Minimal Beat Foundation

Start with sparse, relaxed drums:

```phonon
cps: 1.25  -- 75 BPM

~kick $ s "bd ~ ~ ~ ~ ~ ~ ~"
~snare $ s "~ ~ ~ ~ sn ~ ~ ~"
out $ ~kick + ~snare
```

### Step 2: Adding Subtle Hi-Hats

```phonon
cps: 1.25

~kick $ s "bd ~ ~ ~ ~ ~ ~ ~"
~snare $ s "~ ~ ~ ~ sn ~ ~ ~"
~hats $ s "hh*8" # gain 0.3
out $ ~kick + ~snare + ~hats
```

### Step 3: Warm Bass

```phonon
cps: 1.25

~kick $ s "bd ~ ~ ~ ~ ~ ~ ~"
~snare $ s "~ ~ ~ ~ sn ~ ~ ~"
~hats $ s "hh*8" # gain 0.3

-- Warm, simple bass
~bass $ sine 55 # lpf 150 0.6
~bass_pattern $ ~bass * "1 ~ ~ ~ ~ ~ ~ ~"

out $ ~kick + ~snare + ~hats + ~bass_pattern * 0.4
```

### Step 4: Atmospheric Pads

```phonon
cps: 1.25

~kick $ s "bd ~ ~ ~ ~ ~ ~ ~"
~snare $ s "~ ~ ~ ~ sn ~ ~ ~" # reverb 0.4 0.7
~hats $ s "hh*8" # gain 0.25

~bass $ sine "55 ~ ~ ~ 73.42 ~ ~ ~" # lpf 150 0.6

-- Lush pad
~pad $ saw 110 + saw 164.81 + saw 220
~lfo # sine 0.0625
~pad_filtered $ ~pad # lpf (~lfo * 1500 + 800) 0.3 # reverb 0.7 0.92

~drums $ ~kick + ~snare + ~hats
out $ ~drums + ~bass * 0.35 + ~pad_filtered * 0.08
```

### Step 5: Melodic Elements

```phonon
cps: 1.25

~kick $ s "bd ~ ~ ~ ~ ~ ~ ~"
~snare $ s "~ ~ ~ ~ sn ~ ~ ~" # reverb 0.4 0.7
~hats $ s "hh*8" # gain 0.25

~bass $ sine "55 ~ ~ ~ 73.42 ~ ~ ~" # lpf 150 0.6

-- Simple melody
~melody $ sine "~ ~ 329.63 ~ ~ ~ 293.66 ~"
~melody_processed $ ~melody # reverb 0.5 0.85 # delay 0.5 0.4 0.3

~drums $ ~kick + ~snare + ~hats
out $ ~drums + ~bass * 0.35 + ~melody_processed * 0.15
```

### Step 6: Effects Processing

```phonon
cps: 1.25

~kick $ s "bd ~ ~ ~ ~ ~ ~ ~"
~snare $ s "~ ~ ~ ~ sn ~ ~ ~" # reverb 0.4 0.75
~hats $ s "hh*8" # gain 0.25 # delay 0.25 0.2 0.15

~bass $ sine "55 ~ ~ ~ 73.42 ~ ~ ~" # lpf 150 0.6

-- Delayed, spacious melody
~melody $ sine "~ ~ 329.63 ~ ~ ~ 293.66 ~"
~melody_lush $ ~melody # delay 0.375 0.45 0.35 # reverb 0.6 0.9

-- Pad with chorus feel
~pad $ saw 110 + saw 164.81
~pad_chorus $ ~pad # chorus 0.5 0.6 # lpf 1200 0.3 # reverb 0.7 0.93

~drums $ ~kick + ~snare + ~hats
out $ ~drums + ~bass * 0.35 + ~melody_lush * 0.12 + ~pad_chorus * 0.06
```

### Step 7: Complete Downtempo Track

```phonon
cps: 1.25

-- Minimal drums with space
~kick $ s "bd ~ ~ ~ ~ ~ bd ~"
~snare $ s "~ ~ ~ ~ sn ~ ~ ~" # reverb 0.35 0.7
~rim $ s "~ ~ rim ~ ~ ~ ~ ~" # gain 0.4
~hats $ s "hh*8" $ swing 0.1 # gain 0.25

-- Warm bass
~bass $ sine "55 ~ ~ 55 73.42 ~ 55 ~" # lpf 140 0.7

-- Evolving pad
~lfo # sine 0.04
~pad $ saw 110 + saw 138.59 + saw 164.81  -- A minor
~pad_evolving $ ~pad # lpf (~lfo * 1000 + 600) 0.3 # reverb 0.75 0.94

-- Melodic motif
~motif $ sine "~ ~ 329.63 ~ ~ 293.66 ~ ~"
~motif_spacious $ ~motif # delay 0.5 0.4 0.35 # reverb 0.6 0.88

-- High texture
~shimmer $ sine 659.26 + sine 783.99
~shimmer_subtle $ ~shimmer * "~ ~ ~ 1 ~ ~ ~ ~" # lpf 3000 0.2 # reverb 0.8 0.95

-- Mix
~drums $ ~kick + ~snare + ~rim + ~hats
out $ ~drums + ~bass * 0.35 + ~pad_evolving * 0.06 + ~motif_spacious * 0.1 + ~shimmer_subtle * 0.04
```

## Variations

### Dark Ambient
Brooding, ominous:

```phonon
cps: 0.05  -- Very slow

-- Deep drone
~drone $ sine 27.5 + sine 41.2 + sine 55  -- Very low
~drone_dark $ ~drone # lpf 200 0.4 # reverb 0.9 0.98

-- Unsettling harmonics
~dissonance $ sine 116.54 + sine 123.47  -- Minor second
~lfo # sine 0.01
~dissonance_filtered $ ~dissonance # lpf (~lfo * 500 + 100) 0.3

-- Rumbling noise
~rumble $ noise # lpf 100 1.5 # gain 0.04

out $ ~drone_dark * 0.12 + ~dissonance_filtered * 0.03 + ~rumble
```

### Ambient Techno (100-115 BPM)
Rhythmic but atmospheric:

```phonon
cps: 1.83  -- 110 BPM

~kick $ s "bd ~ ~ ~ bd ~ ~ ~"
~rim $ s "~ ~ rim ~ ~ ~ rim ~" # reverb 0.3 0.6
~hats $ s "hh(5,16)" # gain 0.3

-- Droning bass
~bass $ sine 55 # lpf 100 0.8
~bass_pattern $ ~bass * "1 ~ 0.5 ~ 1 ~ 0.7 ~"

-- Atmospheric pad
~lfo # sine 0.03
~pad $ saw 110 + saw 164.81
~pad_atmo $ ~pad # lpf (~lfo * 1200 + 400) 0.25 # reverb 0.8 0.95

~drums $ ~kick + ~rim + ~hats
out $ ~drums + ~bass_pattern * 0.4 + ~pad_atmo * 0.06
```

### Trip-Hop (70-90 BPM)
Cinematic, hip-hop influenced:

```phonon
cps: 1.33  -- 80 BPM

~kick $ s "bd ~ ~ bd ~ ~ bd ~"
~snare $ s "~ ~ sn ~ ~ ~ sn ~" # reverb 0.4 0.7
~hats $ s "hh*8" $ swing 0.15 # gain 0.3

-- Moody bass
~bass $ saw "41.2 ~ 41.2 ~ 55 ~ 41.2 ~" # lpf 200 1.0

-- Cinematic strings
~strings $ saw 130.81 + saw 164.81 + saw 196  -- C major
~lfo # sine 0.05
~strings_filtered $ ~strings # lpf (~lfo * 1000 + 500) 0.3 # reverb 0.6 0.9

-- Vocal-like synth
~voice $ sine "~ ~ ~ 261.63 ~ ~ ~ ~"
~voice_ethereal $ ~voice # reverb 0.7 0.93 # delay 0.375 0.4 0.3

~drums $ ~kick + ~snare + ~hats
out $ ~drums + ~bass * 0.3 + ~strings_filtered * 0.06 + ~voice_ethereal * 0.1
```

### Space Music
Cosmic, exploratory:

```phonon
cps: 0.03  -- Extremely slow

-- Deep space drone
~space1 $ sine 36.71
~space2 $ sine 55
~space3 $ sine 82.5
~drone $ ~space1 + ~space2 * 0.8 + ~space3 * 0.6

~lfo1 # sine 0.008
~lfo2 # sine 0.012

-- Evolving filter
~drone_space $ ~drone # lpf (~lfo1 * 800 + 200) 0.3 # reverb 0.95 0.995

-- Sparkle layer
~sparkle $ sine 1318.51 + sine 1567.98
~sparkle_twinkle $ ~sparkle * (~lfo2 * 0.5 + 0.5) # lpf 4000 0.2 # reverb 0.9 0.98

-- Wash of noise
~wash $ noise # lpf 500 0.3 # reverb 0.95 0.99

out $ ~drone_space * 0.12 + ~sparkle_twinkle * 0.01 + ~wash * 0.02
```

## Essential Techniques

### Creating Movement with LFOs
```phonon
cps: 0.5

~drone $ sine 110

-- Multiple LFOs at different rates
~lfo_slow # sine 0.02      -- Main sweep
~lfo_medium # sine 0.08    -- Secondary movement
~lfo_fast # sine 0.3       -- Subtle flutter

~cutoff $ ~lfo_slow * 1000 + ~lfo_medium * 200 + ~lfo_fast * 50 + 500
~drone_animated $ ~drone # lpf ~cutoff 0.4 # reverb 0.85 0.95

out $ ~drone_animated * 0.2
```

### Layering Frequencies
```phonon
cps: 0.5

-- Sub layer
~sub $ sine 27.5 # lpf 50 0.6

-- Bass layer
~bass $ sine 55 # lpf 200 0.4

-- Mid layer
~mid $ saw 110 + saw 165 # lpf 800 0.3

-- High layer
~high $ sine 440 + sine 550 # lpf 2000 0.2 # reverb 0.9 0.97

out $ ~sub * 0.3 + ~bass * 0.2 + ~mid * 0.08 + ~high * 0.03
```

### Reverb as an Instrument
```phonon
cps: 0.5

-- Short percussive sound
~ping $ sine 440 * "1 ~ ~ ~ ~ ~ ~ ~"

-- Heavy reverb turns it into a wash
~ping_wash $ ~ping # reverb 0.95 0.99

out $ ~ping_wash * 0.3
```

### Generative Patterns
```phonon
cps: 0.25

-- Random-feeling but deterministic
~notes $ sine "~ ~ 329.63 ~ 392 ~ ~ 440 ~ 349.23 ~ ~ ~ 493.88 ~ ~"
~notes_spacious $ ~notes # delay 0.5 0.5 0.4 # reverb 0.8 0.95

-- Background pad
~pad $ saw 110 + saw 164.81
~lfo # sine 0.02
~pad_filtered $ ~pad # lpf (~lfo * 800 + 400) 0.25 # reverb 0.85 0.96

out $ ~notes_spacious * 0.15 + ~pad_filtered * 0.06
```

### Building Tension and Release
```phonon
cps: 0.0625  -- 16-second cycle

-- Slow filter rise over the cycle
~lfo # saw 0.0625  -- Ramps up over cycle then resets
~pad $ saw 55 + saw 110 + saw 165
~pad_rising $ ~pad # lpf (~lfo * 3000 + 100) 0.5 # reverb 0.8 0.95

out $ ~pad_rising * 0.1
```

## Sound Design Tips

### Warm Pads
- Use saw waves filtered heavily
- Add slight detuning between layers
- Apply generous reverb with long decay
- Use slow LFOs for subtle movement

### Cold/Digital Pads
- Use sine waves for purity
- Less reverb, more delay
- Precise tuning (no detuning)
- Higher frequency content

### Natural Textures
- Filter noise at various frequencies
- Layer multiple noise sources
- Use slow, organic LFO rates
- Combine with tonal elements

## Recommended Settings

### Tempo Ranges
- Pure ambient: No tempo or very slow (cps: 0.01-0.1)
- Downtempo: 60-90 BPM (cps: 1.0-1.5)
- Ambient techno: 100-120 BPM (cps: 1.67-2.0)

### Reverb Settings
- Room size: 0.7-0.95 (large spaces)
- Decay: 0.85-0.99 (long tails)
- Use lower values for more intimate sounds

### Filter Ranges
- Bass: 50-200 Hz cutoff
- Pads: 400-2000 Hz cutoff
- Shimmer: 1000-4000 Hz cutoff

## Production Philosophy

1. **Patience** - Let sounds evolve slowly
2. **Space** - Don't fill every moment with sound
3. **Subtlety** - Small changes create interest
4. **Layering** - Build complexity through simple elements
5. **Listening** - Focus on how sounds interact
6. **Restraint** - Remove rather than add

## Next Steps

1. Experiment with very slow LFO rates
2. Layer different frequency ranges
3. Use reverb creatively as a sound source
4. Create pieces with no drums at all
5. Practice minimalism - what's essential?
6. Explore field recordings and textures

Let the sound breathe.
