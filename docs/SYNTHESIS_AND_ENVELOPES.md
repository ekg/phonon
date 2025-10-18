# Synthesis and Envelopes in Phonon

## Overview

Phonon supports pattern-triggered synthesis with full ADSR envelopes, allowing you to create complex musical phrases with precise control over timbre and dynamics.

## The synth() Function

### Syntax

```phonon
synth NOTES WAVEFORM ATTACK DECAY SUSTAIN RELEASE [GAIN] [PAN]
```

**Parameters:**
- `NOTES`: Mini-notation pattern of note names (e.g., `"c4 e4 g4"`) or frequencies
- `WAVEFORM`: `"sine"`, `"saw"`, `"square"`, or `"triangle"`
- `ATTACK`: Attack time in seconds (0.0 to 10.0)
- `DECAY`: Decay time in seconds (0.0 to 10.0)
- `SUSTAIN`: Sustain level (0.0 to 1.0)
- `RELEASE`: Release time in seconds (0.0 to 10.0)
- `GAIN`: Optional, defaults to 0.3
- `PAN`: Optional, defaults to 0.0 (center)

### ADSR Envelope Explained

```
Amplitude
    ^
1.0 |    /\
    |   /  \___________
0.7 |  /    sustain    \
    | /                 \
0.0 |/___________________\____> Time
    A  D    S           R

    A = Attack   (0.01s)
    D = Decay    (0.1s)
    S = Sustain  (0.7 level)
    R = Release  (0.3s)
```

**Attack**: How quickly the sound fades in (0 = instant, higher = slow fade-in)
**Decay**: How quickly it falls from peak to sustain level
**Sustain**: The level it holds at while the note is active
**Release**: How quickly it fades out after the note ends

## Basic Examples

### 1. Simple Melody with ADSR

```phonon
cps: 2.0
~melody: synth "c4 e4 g4 c5" "saw" 0.01 0.1 0.7 0.3
~master: ~melody * 0.4
```

**Breakdown:**
- `"c4 e4 g4 c5"` - C major arpeggio
- `"saw"` - Sawtooth waveform (bright, buzzy sound)
- `0.01` - Fast attack (10ms)
- `0.1` - Quick decay (100ms)
- `0.7` - Sustain at 70% volume
- `0.3` - Release over 300ms

### 2. Plucky Bass (Short Percussive)

```phonon
cps: 1.0
~bass: synth "c3 c3 g3 c3" "square" 0.001 0.05 0.0 0.1
~master: ~bass * 0.5
```

**Character:**
- `0.001` - Instant attack (1ms) = percussive
- `0.05` - Very short decay (50ms)
- `0.0` - No sustain = note stops after decay
- `0.1` - Short release (100ms)

Result: Tight, punchy bass notes that don't overlap

### 3. Pad Sound (Slow, Atmospheric)

```phonon
cps: 0.5
~pad: synth "c4 e4 g4" "sine" 0.5 0.3 0.8 1.0
~master: ~pad * 0.3
```

**Character:**
- `0.5` - Slow fade-in (500ms) = smooth
- `0.3` - Gentle decay
- `0.8` - High sustain = long held notes
- `1.0` - Long release = notes blend together

Result: Atmospheric, evolving texture

## Effects Routing

### Per-Channel Effects

Apply effects to individual synth channels:

```phonon
cps: 2.0
~lead: synth "c5 e5 g5 c6" "saw" 0.01 0.1 0.7 0.3 # lpf 800 0.8
~bass: synth "c3 c3 g3 c3" "square" 0.001 0.05 0.0 0.1 # hpf 100 0.5
~master: ~lead + ~bass
```

**Effects:**
- `lpf 800 0.8` - Low-pass filter at 800Hz, resonance 0.8 (warm, dark lead)
- `hpf 100 0.5` - High-pass filter at 100Hz, resonance 0.5 (tight bass)

### Master Bus Effects

Apply effects to the entire mix:

```phonon
cps: 1.0
~d1: synth "c4 e4 g4 c5" "saw" 0.01 0.1 0.7 0.3
~d2: synth "c3 c3 g3 c3" "square" 0.001 0.05 0.0 0.1
~master: (~d1 + ~d2) * 0.3 # reverb 0.6 0.5 0.2
```

**Master reverb parameters:**
- `0.6` - Room size (60% = medium room)
- `0.5` - Damping (50% = natural decay)
- `0.2` - Mix (20% wet = subtle reverb tail)

### Multi-Effect Chain

Stack multiple effects on master:

```phonon
cps: 2.0
~melody: synth "c4 e4 g4 c5" "saw" 0.01 0.1 0.7 0.3
~bass: synth "c3*4" "square" 0.001 0.05 0.0 0.1
~mixed: ~melody + ~bass
~master: ~mixed # lpf 2000 0.7 # reverb 0.5 0.5 0.15 # dist 2.0 0.3
```

**Effect chain:**
1. Low-pass filter (2000Hz) - Removes harsh highs
2. Reverb (subtle) - Adds space
3. Distortion (drive=2.0, mix=0.3) - Adds warmth/grit

## Pattern-Based Modulation

### Modulating Envelope Parameters

You can't directly modulate ADSR per-event (they're per-voice), but you can use multiple channels with different envelopes:

```phonon
cps: 2.0
~short: synth "c4 ~ ~ ~" "saw" 0.001 0.05 0.0 0.1    # Short percussive
~long: synth "~ e4 ~ ~" "saw" 0.1 0.2 0.8 0.5        # Sustained
~master: ~short + ~long
```

Each channel has its own envelope characteristics.

### Modulating Filter Cutoff with Patterns

```phonon
cps: 2.0
~synth: synth "c4*8" "saw" 0.01 0.1 0.5 0.2
~cutoff: pattern "500 1000 2000 4000"
~filtered: ~synth # lpf ~cutoff 0.8
~master: ~filtered * 0.3
```

The filter cutoff changes with each note based on the pattern!

## Complete Musical Example

```phonon
cps: 2.0

# Bass line - Tight and percussive
~bass: synth "c3 c3 g3 c3" "square" 0.001 0.05 0.0 0.1

# Lead melody - Bright and expressive
~lead: synth "c5 e5 g5 c6" "saw" 0.01 0.1 0.7 0.3 # lpf 1200 0.8

# Pad - Atmospheric background
~pad: synth "c4 e4 g4" "sine" 0.5 0.3 0.8 1.0

# Mix with individual levels
~mix: ~bass * 0.6 + ~lead * 0.4 + ~pad * 0.2

# Master processing
~master: ~mix # reverb 0.5 0.5 0.2
```

**This creates:**
- Rhythmic bass foundation
- Bright lead melody
- Atmospheric pad underneath
- Everything mixed and reverbed

## How Triggering Works

### Pattern Evaluation

When you write:
```phonon
~d1: synth "c4 e4 g4" "saw" 0.01 0.1 0.7 0.3
```

**What happens internally:**

1. **Pattern parsing**: `"c4 e4 g4"` is parsed into 3 events spread across 1 cycle
2. **Event triggering**: Each event triggers a new synth voice
3. **Voice allocation**: Phonon manages up to 64 simultaneous voices
4. **Envelope application**: Each voice gets its own ADSR envelope
5. **Voice mixing**: All active voices are mixed together
6. **DSP chain**: The mixed output flows through any `#` chained effects

### Timeline Example

```
Cycle:     [-----1.0 second @ 1.0 CPS-----]
Pattern:   c4        e4        g4
Events:    |         |         |
Voices:    [ADSR1---]
                     [ADSR2---]
                               [ADSR3---]
Output:    Mix all active voices at each sample
```

Each note gets its own voice with independent ADSR envelope.

### Polyphony

You can have up to 64 simultaneous voices:

```phonon
cps: 4.0
~chords: synth "[c4,e4,g4]*4" "saw" 0.01 0.1 0.7 0.5
~master: ~chords * 0.2
```

This creates 3 notes × 4 repetitions = 12 voices, all overlapping with release tails.

## Comparison: Bare Oscillators vs Synth Patterns

### Bare Oscillator (No Envelope)

```phonon
cps: 2.0
~d1: saw 440
~master: ~d1 * 0.3
```

**Result:** Continuous 440Hz sawtooth tone. No envelope, no pattern triggering, just raw waveform.

**Use case:** LFOs, drones, control signals

### Synth Pattern (Triggered + Envelope)

```phonon
cps: 2.0
~d1: synth "a4" "saw" 0.01 0.1 0.7 0.3
~master: ~d1 * 0.3
```

**Result:** Pattern-triggered sawtooth with ADSR envelope. Creates note events.

**Use case:** Melodies, rhythms, musical phrases

## Key Takeaways

1. **Use `synth` for musical notes** - It handles triggering, envelopes, and polyphony
2. **Use bare oscillators for control** - LFOs, modulation sources, drones
3. **ADSR shapes the sound** - Fast attack = percussive, slow = smooth
4. **Effects work everywhere** - On channels, on master, chainable with `#`
5. **Patterns control rhythm** - Mini-notation determines when notes play
6. **Master bus is for global processing** - Reverb, compression, final EQ

## Further Resources

- See `examples/synth_effects_demo.ph` for a complete working example
- Check `src/synth_voice_manager.rs` for voice allocation details
- Read `PHONON_LANGUAGE_REFERENCE.md` for complete syntax reference
