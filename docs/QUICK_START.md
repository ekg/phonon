# Phonon Quick Start Guide

Welcome to Phonon! This guide will get you making sound in 5 minutes.

## Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/phonon
cd phonon

# Build
cargo build --release

# Install (optional)
cargo install --path .
```

## Your First Sound

Create a file called `first.ph`:

```phonon
cps: 1.0
out $ sine 440
```

Render it to WAV:

```bash
phonon render first.ph --duration 2
```

You should hear a 440Hz sine wave!

## Basic Patterns

Phonon uses mini-notation from TidalCycles for rhythmic patterns:

```phonon
cps: 2.0
out $ s "bd sn bd sn"
```

This plays kick-snare-kick-snare at 2 cycles per second.

### Pattern Tricks

```phonon
cps: 2.0
out $ s "bd sn hh*4 cp"    -- hh*4 = 4 hi-hats in one beat
```

```phonon
cps: 2.0
out $ s "bd [sn cp] hh*4"  -- [sn cp] = subdivision
```

```phonon
cps: 2.0
out $ s "bd <sn cp hh>"    -- <> = alternation each cycle
```

## Pattern Transformations

Use `$` to transform patterns:

```phonon
cps: 2.0
out $ s "bd sn" $ fast 2           -- Double speed
```

```phonon
cps: 2.0
out $ s "bd sn hh cp" $ rev        -- Reverse
```

```phonon
cps: 2.0
out $ s "bd sn" $ every 4 rev      -- Reverse every 4th cycle
```

## Sample Control

Control samples with `#` chaining:

```phonon
cps: 2.0
out $ s "bd sn" # gain 0.8                    -- Adjust volume
```

```phonon
cps: 2.0
out $ s "bd sn hh cp" # pan "-1 0 0.5 1"     -- Stereo panning
```

```phonon
cps: 2.0
out $ s "bd sn" # speed "1 0.5 2"             -- Playback speed
```

## Synthesis

Phonon can generate oscillators:

```phonon
cps: 1.0
~bass $ saw 55                     -- 55Hz sawtooth
out $ ~bass # lpf 800 1.2          -- Low-pass filter
```

Pattern-controlled synthesis:

```phonon
cps: 2.0
~melody $ sine "220 330 440 330"  -- Pattern of frequencies
out $ ~melody * 0.3
```

## Effects

Add effects with `#`:

```phonon
cps: 2.0
~drums $ s "bd sn hh*4 cp"
out $ ~drums # reverb 0.5 0.8      -- Add reverb
```

```phonon
cps: 2.0
~drums $ s "bd sn"
out $ ~drums # delay 0.25 0.6 0.3  -- Delay effect
```

```phonon
cps: 1.0
~bass $ saw 55 # lpf 400 1.5
out $ ~bass # distortion 2.0       -- Distort that bass!
```

## Combining Patterns

Use buses (`~name`) to build complex compositions:

```phonon
cps: 2.0
~kick $ s "bd*4"
~snare $ s "~ sn ~ sn"
~hats $ s "hh*16" # gain 0.6
~mix $ ~kick + ~snare + ~hats
out $ ~mix # reverb 0.3 0.7
```

## Modulation

Patterns can modulate other patterns:

```phonon
cps: 0.5
~lfo # sine 0.25                   -- # for modifier bus
~bass $ saw 55
~cutoff $ ~lfo * 1500 + 500        -- LFO modulates cutoff
out $ ~bass # lpf ~cutoff 1.0
```

## Advanced Pattern Transformations

```phonon
cps: 2.0
-- Chop samples into pieces
out $ s "bd" $ chop 4
```

```phonon
cps: 2.0
-- Randomize event order
out $ s "bd sn hh cp" $ scramble 4
```

```phonon
cps: 2.0
-- Repeat events (stuttering)
out $ s "bd sn" $ stutter 3
```

```phonon
cps: 2.0
-- Probabilistic dropout
out $ s "bd*16" $ degradeBy 0.3    -- 30% chance of silence
```

## Live Coding

Start live mode to edit in real-time:

```bash
phonon live first.ph
```

Now edit `first.ph` in your editor. Phonon will auto-reload when you save!

Try this progression:

**Step 1** - Simple beat:
```phonon
cps: 2.0
out $ s "bd sn"
```

**Step 2** - Add hi-hats:
```phonon
cps: 2.0
~kick $ s "bd*2"
~snare $ s "~ sn"
~hats $ s "hh*8"
out $ ~kick + ~snare + ~hats
```

**Step 3** - Add transformations:
```phonon
cps: 2.0
~kick $ s "bd*2"
~snare $ s "~ sn"
~hats $ s "hh*8" $ every 4 (fast 2)
out $ ~kick + ~snare + ~hats
```

**Step 4** - Add effects:
```phonon
cps: 2.0
~kick $ s "bd*2"
~snare $ s "~ sn" # reverb 0.5 0.8
~hats $ s "hh*8" $ every 4 (fast 2) # gain 0.6
~drums $ ~kick + ~snare + ~hats
out $ ~drums # compressor -12 4.0 0.01 0.1 3.0
```

## Multi-Output

Route to different outputs for multi-track recording:

```phonon
cps: 2.0
o1 $ s "bd*4"                      -- Output 1: Kicks
o2 $ s "~ sn ~ sn"                 -- Output 2: Snares
o3 $ s "hh*8" # gain 0.5           -- Output 3: Hi-hats
```

## Silencing

In live mode, you can silence outputs:

```phonon
hush        # Silence all outputs
hush 1      # Silence output 1 only
panic       # Emergency silence (kills all voices)
```

## Tips

1. **Start Simple**: Begin with basic patterns, add complexity gradually
2. **Use Buses**: Name intermediate results with `~name` for clarity
3. **$ for Sources**: Use `~bass $ saw 55` for audio generators
4. **# for Modifiers**: Use `~lfo # sine 2` for parameter modulation
5. **Comment Your Code**: Use `--` for comments
6. **Experiment**: Try transforming patterns in different ways
7. **Listen**: Render often to hear your changes
8. **Save Often**: Live mode auto-reloads, so save frequently

## Common Patterns

### Four-on-the-floor:
```phonon
cps: 2.0
out $ s "bd*4"
```

### Breakbeat:
```phonon
cps: 2.0
~beat $ s "bd sn [bd bd] sn"
out $ ~beat $ fast 2
```

### Acid Bass:
```phonon
cps: 2.0
~bass $ saw "55 55 82.5 110"
~cutoff # sine 4 * 1000 + 500
out $ ~bass # lpf ~cutoff 2.0 # distortion 1.5
```

### Ambient Pad:
```phonon
cps: 0.5
~pad $ sine "110 165 220 330"
out $ ~pad # reverb 0.8 0.9 # chorus 0.5 0.8
```

## Next Steps

- Read [PHONON_LANGUAGE_REFERENCE.md](./PHONON_LANGUAGE_REFERENCE.md) for complete syntax
- Check [ROADMAP.md](./ROADMAP.md) to see what's implemented
- Browse [examples/](../examples/) for inspiration
- Join the community and share your creations!

## Getting Help

- Issues: https://github.com/yourusername/phonon/issues
- Discussions: https://github.com/yourusername/phonon/discussions

Happy live coding! 🎵
