# Hip-Hop and Boom Bap in Phonon

Hip-hop emerged from the Bronx in the 1970s and has evolved into countless subgenres. This tutorial focuses on boom bap (the classic East Coast sound) and modern trap production techniques.

## Core Characteristics

### Boom Bap (80-100 BPM)
- **Tempo**: 80-100 BPM (cps: 1.33-1.67)
- **Key Features**: Heavy kick, snappy snare on 2 and 4, swing/shuffle
- **Sound**: Sample-based, vinyl crackle, chopped breaks

### Trap (130-160 BPM, half-time feel)
- **Tempo**: 130-160 BPM but feels slower due to half-time (cps: 2.17-2.67)
- **Key Features**: 808 bass, rapid hi-hats, sparse snares
- **Sound**: Synthetic, heavy sub-bass, rolling hi-hats

## Part 1: Boom Bap

### Step 1: The Boom (Kick Pattern)

The kick in boom bap is syncopated, not on every beat:

```phonon
cps: 1.5  -- 90 BPM

~kick $ s "bd ~ ~ bd ~ ~ bd ~"
out $ ~kick
```

A more complex pattern:

```phonon
cps: 1.5

~kick $ s "bd ~ ~ bd ~ bd ~ ~"
out $ ~kick
```

### Step 2: The Bap (Snare Pattern)

Snare hits hard on beats 2 and 4:

```phonon
cps: 1.5

~kick $ s "bd ~ ~ bd ~ ~ bd ~"
~snare $ s "~ ~ sn ~ ~ ~ sn ~"
out $ ~kick + ~snare
```

Layer for a fatter sound:

```phonon
cps: 1.5

~kick $ s "bd ~ ~ bd ~ ~ bd ~"
~snare $ s "~ ~ sn ~ ~ ~ sn ~"
~clap $ s "~ ~ cp ~ ~ ~ cp ~" # gain 0.5  -- Layer clap under snare
out $ ~kick + ~snare + ~clap
```

### Step 3: Hi-Hats with Swing

Boom bap needs swing for that head-nod feel:

```phonon
cps: 1.5

~kick $ s "bd ~ ~ bd ~ ~ bd ~"
~snare $ s "~ ~ sn ~ ~ ~ sn ~"
~hats $ s "hh*8" $ swing 0.15  -- Add swing
out $ ~kick + ~snare + ~hats * 0.6
```

### Step 4: Adding Open Hats

Open hats add accent and air:

```phonon
cps: 1.5

~kick $ s "bd ~ ~ bd ~ ~ bd ~"
~snare $ s "~ ~ sn ~ ~ ~ sn ~"
~hats_closed $ s "hh*8" $ swing 0.15 # gain 0.5
~hats_open $ s "~ ~ ~ ~ ~ ~ ~ oh" # gain 0.6
out $ ~kick + ~snare + ~hats_closed + ~hats_open
```

### Step 5: Boom Bap Bass

Classic boom bap uses sampled or synth bass following the kick:

```phonon
cps: 1.5

~kick $ s "bd ~ ~ bd ~ ~ bd ~"
~snare $ s "~ ~ sn ~ ~ ~ sn ~"
~hats $ s "hh*8" $ swing 0.15 # gain 0.5

-- Bass follows kick pattern
~bass $ sine "55 ~ ~ 55 ~ ~ 55 ~" # lpf 120 0.8
out $ ~kick + ~snare + ~hats + ~bass * 0.5
```

Walking bass variation:

```phonon
cps: 1.5

~kick $ s "bd ~ ~ bd ~ ~ bd ~"
~snare $ s "~ ~ sn ~ ~ ~ sn ~"
~hats $ s "hh*8" $ swing 0.15 # gain 0.5

-- Walking bass line
~bass $ sine "55 ~ 73.42 55 ~ 82.5 55 ~" # lpf 150 0.8
out $ ~kick + ~snare + ~hats + ~bass * 0.45
```

### Step 6: Percussion Layers

Add shakers, tambourines, or finger snaps:

```phonon
cps: 1.5

~kick $ s "bd ~ ~ bd ~ ~ bd ~"
~snare $ s "~ ~ sn ~ ~ ~ sn ~"
~hats $ s "hh*8" $ swing 0.15 # gain 0.5
~shaker $ s "shaker*16" # gain 0.2  -- Subtle shaker
~rim $ s "~ rim ~ ~ ~ rim ~ ~" # gain 0.4

out $ ~kick + ~snare + ~hats + ~shaker + ~rim
```

### Step 7: Sample Chops and Melody

Add a melodic element:

```phonon
cps: 1.5

~kick $ s "bd ~ ~ bd ~ ~ bd ~"
~snare $ s "~ ~ sn ~ ~ ~ sn ~"
~hats $ s "hh*8" $ swing 0.15 # gain 0.5

-- Simple melody
~melody $ sine "261.63 ~ 293.66 ~ 329.63 ~ 293.66 ~"
~melody_filtered $ ~melody # lpf 2000 0.5 # reverb 0.3 0.6

out $ ~kick + ~snare + ~hats + ~melody_filtered * 0.2
```

### Step 8: Complete Boom Bap Beat

```phonon
cps: 1.5

-- Core drums
~kick $ s "bd ~ ~ bd ~ bd ~ ~"
~snare $ s "~ ~ sn ~ ~ ~ sn ~"
~clap $ s "~ ~ cp ~ ~ ~ cp ~" # gain 0.4

-- Hi-hats with swing
~hats_closed $ s "hh*8" $ swing 0.12 # gain 0.5
~hats_open $ s "~ ~ ~ ~ ~ oh ~ ~" # gain 0.5

-- Percussion
~rim $ s "~ rim ~ ~ rim ~ ~ ~" # gain 0.35

-- Bass
~bass $ sine "55 ~ 73.42 55 ~ 82.5 55 ~" # lpf 130 0.9

-- Sample (simulate vinyl warmth with filter)
~keys $ saw 261.63 + saw 329.63 + saw 392
~keys_warm $ ~keys * "~ 1 ~ ~ ~ 0.7 ~ 0.5" # lpf 1500 0.4 # reverb 0.4 0.7

-- Mix
~drums $ ~kick + ~snare + ~clap + ~hats_closed + ~hats_open + ~rim
out $ ~drums + ~bass * 0.45 + ~keys_warm * 0.1
```

## Part 2: Trap

### Step 1: The 808 Kick

Trap uses extended, tuned 808 kicks:

```phonon
cps: 2.33  -- 140 BPM

-- Long 808 kick
~808 $ sine 55 # lpf 100 0.8
~808_pattern $ ~808 * "1 ~ ~ ~ ~ 1 ~ ~ 1 ~ ~ ~ ~ ~ ~ ~"
out $ ~808_pattern * 0.6
```

### Step 2: Snare Placement

Trap snares hit less frequently - often just on beat 3:

```phonon
cps: 2.33

~808 $ sine 55 # lpf 100 0.8
~808_pattern $ ~808 * "1 ~ ~ ~ ~ 1 ~ ~ 1 ~ ~ ~ ~ ~ ~ ~"

~snare $ s "~ ~ ~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ ~ ~"  -- Beat 3 only
out $ ~808_pattern * 0.5 + ~snare
```

Alternate pattern with clap:

```phonon
cps: 2.33

~808 $ sine 55 * "1 ~ ~ ~ ~ 1 ~ ~ 1 ~ ~ ~ ~ ~ ~ ~" # lpf 100 0.8
~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"  -- Syncopated
~clap $ s "~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~ ~ ~" # gain 0.6

out $ ~808 * 0.5 + ~snare + ~clap
```

### Step 3: Rolling Hi-Hats

The signature of trap - rapid, complex hi-hat patterns:

```phonon
cps: 2.33

~808 $ sine 55 * "1 ~ ~ ~ ~ 1 ~ ~ 1 ~ ~ ~ ~ ~ ~ ~" # lpf 100 0.8
~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"

-- Rolling hi-hats with velocity variation
~hats $ s "hh*16" # gain "0.4 0.3 0.5 0.3 0.4 0.3 0.6 0.3 0.4 0.3 0.5 0.3 0.4 0.3 0.7 0.4"
out $ ~808 * 0.5 + ~snare + ~hats
```

Hi-hat rolls with speed variations:

```phonon
cps: 2.33

~808 $ sine 55 * "1 ~ ~ ~ ~ 1 ~ ~ 1 ~ ~ ~ ~ ~ ~ ~" # lpf 100 0.8
~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"

-- Hi-hats with occasional triplets
~hats_main $ s "hh*16" # gain 0.4
~hats_roll $ s "~ ~ ~ ~ ~ ~ [hh hh hh] ~ ~ ~ ~ ~ ~ ~ [hh hh hh] ~" # gain 0.5
out $ ~808 * 0.5 + ~snare + ~hats_main + ~hats_roll
```

### Step 4: Open Hat Accents

```phonon
cps: 2.33

~808 $ sine 55 * "1 ~ ~ ~ ~ 1 ~ ~ 1 ~ ~ ~ ~ ~ ~ ~" # lpf 100 0.8
~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~hats $ s "hh*16" # gain 0.4
~open_hat $ s "~ ~ ~ ~ ~ ~ ~ oh ~ ~ ~ ~ ~ ~ ~ ~" # gain 0.6

out $ ~808 * 0.5 + ~snare + ~hats + ~open_hat
```

### Step 5: 808 Bass Slides

Trap uses pitch bends on the 808:

```phonon
cps: 2.33

-- 808 with pitch slides (using pattern)
~808_pitch $ sine "55 ~ ~ ~ ~ 55 ~ ~ 41.2 ~ ~ ~ ~ ~ ~ ~" # lpf 100 0.9
~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~hats $ s "hh*16" # gain 0.4

out $ ~808_pitch * 0.55 + ~snare + ~hats
```

### Step 6: Synth Leads

Trap often uses simple, atmospheric synth lines:

```phonon
cps: 2.33

~808 $ sine 55 * "1 ~ ~ ~ ~ 1 ~ ~ 1 ~ ~ ~ ~ ~ ~ ~" # lpf 100 0.8
~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~hats $ s "hh*16" # gain 0.4

-- Dark synth melody
~lead $ sine "~ ~ ~ ~ 261.63 ~ ~ ~ ~ ~ 220 ~ ~ ~ ~ ~"
~lead_processed $ ~lead # reverb 0.5 0.8 # delay 0.25 0.3 0.2

out $ ~808 * 0.5 + ~snare + ~hats + ~lead_processed * 0.25
```

### Step 7: Complete Trap Beat

```phonon
cps: 2.33

-- 808 Bass
~808 $ sine "55 ~ ~ ~ ~ 55 ~ ~ 41.2 ~ ~ ~ ~ ~ ~ ~" # lpf 100 0.9

-- Drums
~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~clap $ s "~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~ ~ ~" # gain 0.5 # reverb 0.2 0.4

-- Complex hi-hat pattern
~hats_main $ s "hh*16" # gain "0.4 0.25 0.5 0.25 0.4 0.25 0.6 0.25 0.4 0.25 0.5 0.25 0.4 0.25 0.7 0.3"
~hats_roll $ s "~ ~ ~ ~ ~ ~ [hh hh hh] ~ ~ ~ ~ ~ ~ ~ [hh hh hh hh] ~" # gain 0.45
~open_hat $ s "~ ~ ~ ~ ~ ~ ~ oh ~ ~ ~ ~ ~ ~ ~ ~" # gain 0.5

-- Atmospheric pad
~pad $ sine 130.81 + sine 164.81
~pad_filtered $ ~pad # lpf 800 0.4 # reverb 0.6 0.9

-- Mix
~drums $ ~snare + ~clap + ~hats_main + ~hats_roll + ~open_hat
out $ ~808 * 0.5 + ~drums + ~pad_filtered * 0.08
```

## Variations

### Lo-Fi Hip Hop (70-90 BPM)
Dusty, nostalgic, chill:

```phonon
cps: 1.33  -- 80 BPM

~kick $ s "bd ~ ~ bd ~ ~ bd ~"
~snare $ s "~ ~ sn ~ ~ ~ sn ~"
~hats $ s "hh*8" $ swing 0.18 # gain 0.4

-- Warm, filtered bass
~bass $ sine "55 ~ 73.42 ~ 55 ~ 82.5 ~" # lpf 100 0.7

-- Lo-fi keys (filtered, warm)
~keys $ saw 261.63 + saw 329.63
~keys_lofi $ ~keys * "~ 1 ~ 0.6 ~ 1 ~ ~" # lpf 1200 0.3 # reverb 0.5 0.8

~drums $ ~kick + ~snare + ~hats
out $ ~drums + ~bass * 0.4 + ~keys_lofi * 0.12
```

### G-Funk (90-100 BPM)
West Coast, whiny synths, smooth:

```phonon
cps: 1.58  -- 95 BPM

~kick $ s "bd ~ ~ bd ~ bd ~ ~"
~snare $ s "~ ~ sn ~ ~ ~ sn ~"
~hats $ s "hh*8" # gain 0.5

-- Funk bass
~bass $ sine "55 ~ 55 73.42 ~ 55 82.5 55" # lpf 150 0.8

-- G-Funk whine synth (high, thin)
~lfo # sine 5  -- Vibrato
~whine $ sine 523.25  -- High C
~whine_mod $ ~whine * (~lfo * 0.1 + 1)  -- Add vibrato
~whine_filtered $ ~whine_mod * "~ ~ ~ 1 ~ ~ 1 ~" # lpf 3000 0.3

~drums $ ~kick + ~snare + ~hats
out $ ~drums + ~bass * 0.4 + ~whine_filtered * 0.15
```

### Drill (140 BPM)
Dark, sliding 808s, sparse:

```phonon
cps: 2.33  -- 140 BPM

-- Sliding 808
~808 $ sine "55 ~ ~ 55 ~ 41.2 ~ ~ 55 ~ ~ ~ 61.74 ~ ~ ~" # lpf 90 0.9

-- Minimal drums
~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ ~ ~ sn ~" # reverb 0.2 0.4
~hats $ s "hh*16" # gain 0.35
~perc $ s "~ ~ rim ~ ~ ~ ~ ~ ~ ~ rim ~ ~ ~ ~ ~" # gain 0.4

-- Dark atmosphere
~pad $ sine 82.41 + sine 110
~dark $ ~pad # lpf 400 0.5 # reverb 0.7 0.95

out $ ~808 * 0.55 + ~snare + ~hats + ~perc + ~dark * 0.05
```

## Essential Techniques

### Swing for Boom Bap
```phonon
cps: 1.5

~kick $ s "bd ~ ~ bd ~ ~ bd ~"
~snare $ s "~ ~ sn ~ ~ ~ sn ~"
~hats $ s "hh*16" $ swing 0.2  -- Heavy swing
out $ ~kick + ~snare + ~hats * 0.5
```

### Hi-Hat Humanization
```phonon
cps: 2.33

-- Velocity variation for human feel
~hats $ s "hh*16" # gain "0.5 0.3 0.6 0.35 0.55 0.3 0.65 0.4 0.5 0.35 0.6 0.3 0.55 0.35 0.7 0.45"
out $ ~hats
```

### Ghost Notes
```phonon
cps: 1.5

~kick $ s "bd ~ ~ bd ~ ~ bd ~"
~snare_main $ s "~ ~ sn ~ ~ ~ sn ~"
~snare_ghost $ s "~ sn ~ ~ sn ~ ~ sn" # gain 0.2  -- Quiet ghost notes
out $ ~kick + ~snare_main + ~snare_ghost
```

### Sidechain Effect
```phonon
cps: 1.5

~kick $ s "bd ~ ~ bd ~ ~ bd ~"
~snare $ s "~ ~ sn ~ ~ ~ sn ~"
~hats $ s "hh*8" # gain 0.5

-- Pad that "ducks" with kick
~pad $ sine 130.81 + sine 164.81
~duck $ "0.3 1 1 0.3 1 1 0.3 1"  -- Low when kick hits
~pad_ducked $ ~pad * ~duck # lpf 1000 0.4

out $ ~kick + ~snare + ~hats + ~pad_ducked * 0.15
```

## Sample Recommendations

### Boom Bap
- `bd` - Classic boom kicks
- `sn` - Punchy snares
- `cp` - Claps for layering
- `hh` - Crisp hi-hats
- `oh` - Open hats for accent
- `rim` - Rim shots

### Trap
- `bd:3` - Punchy 808 triggers
- `sn:3` - Trap snares
- `cp` - Sharp claps
- `hh` - Tight hi-hats
- `oh` - Open hats

## Production Tips

1. **Swing is essential** for boom bap - without it, beats sound stiff
2. **Less is more** in trap - leave space between elements
3. **Layer sounds** for impact - snare + clap, multiple kicks
4. **Hi-hat patterns** should vary in velocity for human feel
5. **808s need room** - don't clutter the low end
6. **Use reverb** on snares for depth
7. **Ghost notes** add groove and sophistication

## Next Steps

1. Practice different kick/snare patterns
2. Experiment with swing amounts
3. Create complex hi-hat patterns
4. Layer percussion elements
5. Add melodic samples or synths
6. Study classic beats and recreate them

Keep it grimy!
