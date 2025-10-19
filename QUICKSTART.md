# Phonon Quickstart Guide

Get started with Phonon live coding in 5 minutes!

## Installation

```bash
# Clone and build
git clone https://github.com/erikgarrison/phonon.git
cd phonon
cargo build --release

# Download samples (optional, recommended)
git clone https://github.com/tidalcycles/Dirt-Samples.git samples
```

## Your First Pattern

Create `first.ph`:

```phonon
tempo: 2.0
out: s "bd sn" * 0.8
```

Run it:

```bash
./target/release/phonon live first.ph
```

You should hear a kick and snare pattern! 🎉

Try changing `"bd sn"` to `"bd*4 sn"` and save - it updates instantly!

## Basic Patterns

### Simple Rhythms

```phonon
tempo: 2.0

# Kick and snare
out: s "bd sn bd sn"

# With hi-hats
out: s "[bd sn, hh*8]"

# Rest with ~
out: s "bd ~ sn ~"
```

### Multiplication

```phonon
tempo: 2.0

# Play bd 4 times per cycle
out: s "bd*4"

# Combine with other sounds
out: s "[bd*4, sn*2, hh*8]"
```

### Euclidean Rhythms

```phonon
tempo: 2.0

# 3 hits distributed across 8 steps
out: s "bd(3,8)"

# Combine patterns
out: s "[bd(3,8), hh(5,16)]"
```

## DSP Parameters

### Gain (Volume)

```phonon
tempo: 2.0

# Constant gain
out: s "bd*4" # gain 0.5

# Pattern gain (each hit different volume)
out: s "bd*4" # gain "1.0 0.8 0.6 0.4"
```

### Pan (Stereo Position)

```phonon
tempo: 2.0

# Alternate left/right
out: s "hh*8" # pan "-1 1"

# Random panning
out: s "hh*16" # pan "-1 -0.5 0 0.5 1"
```

### Speed (Playback Rate / Pitch)

```phonon
tempo: 2.0

# Play sample at different speeds
out: s "bd*4" # speed "1 2 0.5 1.5"

# Half speed = lower pitch, double speed = higher pitch
out: s "bd bd" # speed "0.5 2.0"
```

### More DSP Parameters

```phonon
tempo: 2.0

# Sample selection (n) - cycle through different samples
out: s "bd bd bd bd" # n "0 1 2 3"

# Pitch shift in semitones (note)
out: s "bd*4" # note "0 12 -12 7"  # original, octave up, down, fifth

# Envelope shaping
out: s "bd*4" # attack 0.1 # release 0.5
```

## Audio Effects

### Reverb

```phonon
tempo: 2.0
# reverb: room_size, damping, mix
out: s "bd sn" # reverb 0.8 0.5 0.3
```

### Delay

```phonon
tempo: 2.0
# delay: time, feedback, mix
out: s "bd sn" # delay 0.25 0.6 0.5
```

### Distortion

```phonon
tempo: 2.0
# distortion: drive, mix
out: s "bd sn" # distortion 10.0 0.5
```

### Chaining Effects

```phonon
tempo: 2.0
out: s "bd sn"
  # delay 0.25 0.6 0.3
  # reverb 0.7 0.5 0.4
```

## Pattern Transforms

### Fast and Slow

```phonon
tempo: 2.0

# Play pattern twice as fast
~drums: s "bd sn" $ fast 2
out: ~drums

# Play pattern half speed
~drums: s "bd sn" $ slow 2
out: ~drums
```

### Reverse

```phonon
tempo: 2.0
~drums: s "bd sn cp hh" $ rev
out: ~drums
```

### Every N Cycles

```phonon
tempo: 2.0

# Reverse every 4 cycles
~drums: s "bd sn" $ every 4 rev
out: ~drums
```

## Complete Examples

### House Beat

```phonon
tempo: 2.0

~kick: s "bd*4"
~snare: s "~ sn ~ sn"
~hats: s "hh*8" # gain "0.6 0.8 0.7 0.9"

out: (~kick + ~snare + ~hats) * 0.7
```

### With Effects

```phonon
tempo: 2.5

~drums: s "[bd(3,8), hh*16]" # gain "1.0 0.6"
~wet: ~drums # delay 0.25 0.4 0.3 # reverb 0.6 0.5 0.2

out: ~wet * 0.8
```

## Tips

**Live Coding Workflow:**
1. Start simple
2. Save to hear changes
3. Add complexity gradually
4. Comment out parts with `#` while experimenting

**Finding Samples:**
Check `samples/` directory for available sounds: `bd/`, `sn/`, `hh/`, `cp/`, etc.

For more information, see README.md
