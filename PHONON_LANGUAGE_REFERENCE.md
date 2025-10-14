# Phonon Language Reference

Complete reference for the Phonon `.ph` file format.

## Basic Syntax

```phonon
# Comments start with #
cps: 2.0              # Set tempo (cycles per second)
~bus: expression      # Define a named bus
out: expression       # Set output
```

## Oscillators

Basic waveform generators:

```phonon
sine(freq)            # Sine wave
saw(freq)             # Sawtooth wave
square(freq)          # Square wave
triangle(freq)        # Triangle wave
```

## Pattern-Triggered Synthesis

Pattern-triggered synths spawn polyphonic voices with ADSR envelopes from Tidal Cycles mini-notation:

```phonon
synth(notes, waveform, attack, decay, sustain, release)
# notes: Mini-notation pattern of note names or frequencies
# waveform: "sine", "saw", "square", or "triangle"
# attack: Attack time in seconds (0.001-1.0, default 0.01)
# decay: Decay time in seconds (0.001-1.0, default 0.1)
# sustain: Sustain level (0.0-1.0, default 0.7)
# release: Release time in seconds (0.001-2.0, default 0.2)
```

### Note Patterns

Notes can be specified as:
- **Note names**: "c4", "a3", "g#5" (MIDI-style note names)
- **Frequencies**: "220", "440", "880" (in Hz)
- **Chords**: "[c4, e4, g4]" (polyphonic)
- **Rests**: "~" (silence)

Examples:
```phonon
# Simple melody
~melody: synth("c4 e4 g4 c5", "saw", 0.01, 0.1, 0.7, 0.2)

# Bass line with rests
~bass: synth("c2 c2 ~ g2 ~ c2 g2 ~", "square", 0.01, 0.05, 0.8, 0.1)

# Polyphonic chords
~chords: synth("[c4, e4, g4] [d4, f4, a4]", "sine", 0.05, 0.2, 0.6, 0.3)

# Using frequencies directly
~arp: synth("220 330 440 550", "triangle", 0.005, 0.05, 0.5, 0.1)
```

### Polyphony & Voice Management

- **64 simultaneous voices** - automatic voice stealing when capacity reached
- **Per-voice ADSR envelopes** - each note has independent envelope
- **Pattern timing** - notes trigger on pattern events, not continuously

### ADSR Parameters

The envelope shapes the amplitude of each note over time:

```
ATTACK   DECAY    SUSTAIN      RELEASE
  /\      \         ___          \
 /  \      \       /   \          \
/    \      \     /     \          \___
     |<--A-->|<-D->|<-S->|<--R-->|
```

- **Attack (A)**: Time to reach peak amplitude (0.001-1.0s)
  - Short (0.001-0.01): Percussive, plucky sounds
  - Long (0.1-1.0): Slow fade-in, pads

- **Decay (D)**: Time to fall from peak to sustain level (0.001-1.0s)
  - Short (0.01-0.1): Quick decay to sustain
  - Long (0.2-1.0): Gradual decay

- **Sustain (S)**: Level held during note (0.0-1.0)
  - 0.0: No sustain (percussive)
  - 1.0: Full level sustained

- **Release (R)**: Time to fade to silence after note ends (0.001-2.0s)
  - Short (0.01-0.1): Abrupt cutoff
  - Long (0.5-2.0): Long tail, reverb-like

### Typical ADSR Presets

```phonon
# Piano (percussive with long release)
synth("c4 e4 g4", "triangle", 0.001, 0.1, 0.3, 0.8)

# Pad (slow attack, sustained)
synth("c3 e3 g3", "saw", 0.3, 0.2, 0.8, 1.0)

# Pluck (quick attack/decay, no sustain)
synth("c4 d4 e4", "sine", 0.001, 0.2, 0.0, 0.05)

# Organ (instant attack, full sustain)
synth("c4 e4 g4", "square", 0.001, 0.01, 1.0, 0.01)

# Bass (punchy with medium release)
synth("c2 ~ g2 ~", "saw", 0.01, 0.05, 0.8, 0.1)
```

## Synthesizers

SuperDirt-inspired synthesizers with rich, production-ready sounds:

### Drums

```phonon
superkick(freq, pitch_env, sustain, noise)
# freq: Base frequency (40-80 Hz typical)
# pitch_env: Pitch envelope amount (0.0-1.0, default 0.5)
# sustain: Sustain time (default 0.3)
# noise: Noise layer amount (0.0-1.0, default 0.1)
# Example: superkick(60, 0.5, 0.3, 0.1)

supersnare(freq, snappy, sustain)
# freq: Base frequency (150-250 Hz typical)
# snappy: Snappiness/noise amount (0.0-1.0, default 0.8)
# sustain: Decay time (default 0.15)
# Example: supersnare(200, 0.8, 0.15)

superhat(bright, sustain)
# bright: Brightness (0.0-1.0, default 0.7)
# sustain: Decay time (0.05 closed, 0.3 open)
# Example: superhat(0.7, 0.05)
```

### Melodic Synths

```phonon
supersaw(freq, detune, voices)
# freq: Base frequency
# detune: Detune amount (0.0-1.0, default 0.3)
# voices: Number of voices (2-7, default 7)
# Example: supersaw(110, 0.5, 7)

superpwm(freq, pwm_rate, pwm_depth)
# freq: Base frequency
# pwm_rate: LFO rate in Hz (0.1-10, default 0.5)
# pwm_depth: PWM depth (0.0-1.0, default 0.8)
# Example: superpwm(220, 0.5, 0.8)

superchip(freq, vibrato_rate, vibrato_depth)
# freq: Base frequency
# vibrato_rate: Vibrato LFO rate (default 5.0 Hz)
# vibrato_depth: Vibrato depth (default 0.05)
# Example: superchip(440, 6.0, 0.05)

superfm(freq, mod_ratio, mod_index)
# freq: Carrier frequency
# mod_ratio: Modulator/carrier ratio (default 2.0)
# mod_index: Modulation index (default 1.0)
# Example: superfm(440, 2.0, 1.5)
```

## Filters

```phonon
lpf(input, cutoff, q)   # Low-pass filter
hpf(input, cutoff, q)   # High-pass filter
```

Example:
```phonon
out: lpf(saw(110), 800, 2.0)
```

## Audio Effects

### Reverb

```phonon
reverb(input, room_size, damping, mix)
# room_size: 0.0-1.0 (controls feedback, default 0.7)
# damping: 0.0-1.0 (high frequency damping, default 0.5)
# mix: 0.0-1.0 (dry/wet balance, default 0.3)
```

Example:
```phonon
out: reverb(sine(440), 0.8, 0.5, 0.3)
```

### Distortion

```phonon
distortion(input, drive, mix)
dist(input, drive, mix)   # Short form
# drive: 1.0-100.0 (pre-gain amount, default 3.0)
# mix: 0.0-1.0 (dry/wet balance, default 0.5)
```

Example:
```phonon
out: dist(saw(110), 5.0, 0.5)
```

### Bitcrusher

```phonon
bitcrush(input, bits, rate_reduction)
# bits: 1.0-16.0 (bit depth, default 4.0)
# rate_reduction: 1.0-64.0 (sample rate reduction factor, default 4.0)
```

Example:
```phonon
out: bitcrush(superchip(880, 6.0, 0.05), 4.0, 8.0)
```

### Chorus

```phonon
chorus(input, rate, depth, mix)
# rate: 0.1-10.0 Hz (LFO frequency, default 1.0)
# depth: 0.0-1.0 (modulation amount, default 0.5)
# mix: 0.0-1.0 (dry/wet balance, default 0.3)
```

Example:
```phonon
out: chorus(superpwm(220, 0.5, 0.8), 1.5, 0.6, 0.4)
```

## Other Nodes

```phonon
delay(input, time, feedback, mix)
# time: Delay time in seconds (0.0-2.0)
# feedback: Feedback amount (0.0-0.99)
# mix: Dry/wet balance (0.0-1.0)

rms(input, window_size)
# window_size: Analysis window in seconds

when(input, condition)
# Conditional gate
```

## Patterns

Use Tidal Cycles mini-notation in quotes:

```phonon
"bd sn hh cp"         # Sequence
"bd*4"                # Repeat
"bd ~ ~ ~"            # Rests
"<bd sn cp>"          # Alternation
"[bd, sn]"            # Layering
"bd(3,8)"             # Euclidean rhythm
```

Patterns can modulate any parameter:

```phonon
out: sine("110 220 440") * 0.2
```

## Math Operations

```phonon
a + b                 # Addition
a - b                 # Subtraction
a * b                 # Multiplication
a / b                 # Division
```

## Signal Chain

Use `>>` to chain signals:

```phonon
~bass: saw(55) >> lpf(800, 0.9)
out: ~bass * 0.3
```

## Complete Examples

### Example 1: Basic Synth with Reverb

```phonon
cps: 2.0
out: reverb(supersaw(110, 0.5, 7), 0.8, 0.5, 0.3) * 0.2
```

### Example 2: Drum Kit

```phonon
cps: 2.0
~kick: superkick(60, 0.5, 0.3, 0.1)
~snare: supersnare(200, 0.8, 0.15)
~hat: superhat(0.7, 0.05)
out: reverb(~kick + ~snare + ~hat, 0.6, 0.5, 0.2) * 0.3
```

### Example 3: Full Effects Chain

```phonon
cps: 2.0
out: reverb(
    chorus(
        dist(
            supersaw(110, 0.5, 5),
            3.0,
            0.3
        ),
        1.0,
        0.5,
        0.3
    ),
    0.7,
    0.5,
    0.4
) * 0.2
```

### Example 4: FM Bells

```phonon
cps: 1.0
out: reverb(superfm(440, 2.0, 1.5), 0.9, 0.3, 0.5) * 0.3
```

### Example 5: Lo-Fi Chiptune

```phonon
cps: 4.0
~chip: superchip(880, 6.0, 0.05)
out: bitcrush(~chip, 4.0, 8.0) * 0.5
```

### Example 6: Modulated Bass

```phonon
cps: 2.0
~lfo: sine(0.25)
~bass: saw("55 82.5 110") >> lpf(~lfo * 2000 + 500, 0.8)
out: dist(~bass, 2.0, 0.3) * 0.3
```

### Example 7: Pattern-Triggered Synth Melody

```phonon
cps: 2.0
~melody: synth("c4 e4 g4 c5", "saw", 0.01, 0.1, 0.7, 0.2)
~bass: synth("c2 c2 ~ g2", "square", 0.01, 0.05, 0.8, 0.1)
~chords: synth("[c4, e4, g4]", "sine", 0.05, 0.2, 0.6, 0.3)
out: reverb(~melody * 0.3 + ~bass * 0.4 + ~chords * 0.25, 0.7, 0.5, 0.3)
```

## Parameter Defaults

When parameters are omitted, sensible defaults are used:

- **synth**: `(notes, waveform, 0.01, 0.1, 0.7, 0.2)`
- **superkick**: `(freq, 0.5, 0.3, 0.1)`
- **supersaw**: `(freq, 0.3, 7)`
- **superpwm**: `(freq, 0.5, 0.8)`
- **superchip**: `(freq, 5.0, 0.05)`
- **superfm**: `(freq, 2.0, 1.0)`
- **supersnare**: `(freq, 0.8, 0.15)`
- **superhat**: `(0.7, 0.05)`
- **reverb**: `(input, 0.7, 0.5, 0.3)`
- **distortion**: `(input, 3.0, 0.5)`
- **bitcrush**: `(input, 4.0, 4.0)`
- **chorus**: `(input, 1.0, 0.5, 0.3)`

## Tips

1. **Start Simple**: Begin with a single oscillator or synth
2. **Layer Effects**: Chain multiple effects for complex sounds
3. **Use Buses**: Named buses make complex patches readable
4. **Experiment with Synth Parameters**: Small changes can dramatically affect the sound
5. **Mind Your Levels**: Use `* 0.3` or similar to prevent clipping
6. **Pattern Everything**: Even effect parameters can be pattern-driven

## Implementation Note

All synths and effects are based on professional DSP algorithms:
- Reverb uses the Freeverb algorithm (8 comb + 4 allpass filters)
- Distortion uses soft clipping (tanh waveshaping)
- Chorus uses LFO-modulated delay with interpolation
- Bitcrusher uses proper quantization and sample-rate reduction

See `SYNTH_AND_EFFECTS_SUMMARY.md` for implementation details.
