# 🎵 Phonon Quick Start Guide

**Phonon = Tidal + Glicol**
Live coding music with Strudel/Tidal patterns + modular synthesis DSL

---

## What is Phonon?

Phonon combines:
- **Tidal/Strudel mini-notation** - Euclidean rhythms, pattern algebra, ~150 operators
- **Glicol-style DSL** - Signal routing, filters, modular synthesis
- **Rust performance** - Real-time audio with no compromises

---

## Mini-Notation Syntax

### Basic Patterns
```
"bd sn hh cp"           # Sequence of events
"bd*4"                  # Repeat 4 times
"[bd sn]*2"             # Group and repeat
"bd ~ sn ~"             # Rests (~)
"<bd sn cp>"            # Alternation (cycles through)
```

### Euclidean Rhythms
```
"1(3,8)"                # 3 pulses in 8 steps
"1(5,8,2)"              # 5 pulses in 8 steps, rotated by 2
"bd(4,16)"              # 4-on-the-floor kick
"hh(13,16,4)"           # Almost-constant hats with rotation
```

### Polyrhythms & Stacking
```
"[bd, ~ sn, hh*4]"      # Stack 3 layers (comma = stack)
"(bd,sn cp,hh*3)"       # Polyrhythm (different speeds)
```

### Operators
```
"bd sn" .fast(2)        # Double speed
"bd sn" .slow(2)        # Half speed
"bd sn" .rev()          # Reverse
"c e g" .every(4, rev)  # Reverse every 4th cycle
```

---

## Synthesis DSL

### Sample Playback (dirt-samples)
```phonon
# Play audio samples with Tidal-style mini-notation
s("bd sn hh cp")                              # Basic drums
s("bd sn", 0.8)                               # With gain
s("bd sn", 1.0, -0.5)                         # With pan
s("bd:5 sn:3", 1.0, 0.0, 1.0, 0, 0.001, 0.1) # Full DSP control

# Sample bank selection (different kicks, snares, etc.)
s("bd:0 bd:1 bd:2 bd:3")                      # Try different kicks
s("sn:0 sn:1 sn:2")                           # Different snares

# Envelope shaping (attack, release)
s("bd sn", 1.0, 0.0, 1.0, 0, 0.05, 0.3)      # Soft attack, long release
s("bd sn", 1.0, 0.0, 1.0, 0, 0.001, 0.05)    # Tight, percussive

# Pattern-controlled parameters
s("bd*4", "1.0 0.8 0.6 0.9")                 # Gain accents
s("hh*8", 1.0, "-1.0 -0.5 0.0 0.5")          # Pan sweeps
```

### Oscillators
```phonon
sine(440)               # Sine wave at 440 Hz
saw(110)                # Sawtooth
square(220)             # Square wave
tri(330)                # Triangle
noise                   # White noise
```

### Filters
```phonon
signal # lpf(1000, 0.7)    # Low-pass filter (cutoff, Q)
signal # hpf(500, 1.0)     # High-pass filter
signal # bpf(1000, 2.0)    # Band-pass filter
```

### Pattern-Driven Synthesis
```phonon
# Patterns can control ANY parameter!
freq_pattern = "[220 330 440 330]*2"
melody = saw(freq_pattern)

gate_pattern = "1(4,16)"
kick = sine(55) * gate_pattern

filter_mod = "[500 1000 2000]*4"
bass = saw(110) # lpf(filter_mod, 2.0)
```

### Buses & Routing
```phonon
~lfo: sine(2) * 100
~carrier: sine(440 + ~lfo)
~filtered: ~carrier # lpf(1000, 0.7)
out: ~filtered * 0.5
```

### Mixing
```phonon
kick = sine(55) * "1(4,16)"
snare = sine(150) * "~ 1 ~ 1"
out kick * 0.5 + snare * 0.3
```

---

## How To Use

### 1. Render to WAV
```bash
# Render 8 seconds (4 bars at 120 BPM)
cargo run --bin phonon render house_4x4.phonon output.wav --duration 8

# Analyze the output
cargo run --bin wav_analyze output.wav
```

### 2. Live Coding (Auto-reload on save)
```bash
# Start live session
cargo run --bin phonon live house_4x4.phonon --duration 4

# Now edit house_4x4.phonon in your editor
# Every time you save, Phonon re-renders and plays!
```

### 3. One-Shot Play
```bash
cargo run --bin phonon play house_4x4.phonon
```

### 4. Interactive REPL
```bash
cargo run --bin phonon repl
```

---

## Example 1: Sample-Based Drum Track

```phonon
cps: 2.0

# Kick - Punchy with short release
~kick: s("bd:5 ~ ~ ~ bd:5 ~ ~ ~", 1.0, 0.0, 1.0, 0, 0.001, 0.08)

# Snare - Natural with medium release
~snare: s("~ ~ sn:3 ~ ~ ~ sn:3 ~", 0.9, 0.1, 1.0, 0, 0.001, 0.15)

# Hi-hats - Closed with tight envelopes
~hh: s("hh:0*8", "1.0 0.7 0.8 0.7", "-0.2 0.2", 1.0, 1, 0.001, 0.05)

out: (~kick + ~snare + ~hh) * 0.4
```

## Example 2: 4-on-the-Floor House Beat (Synthesis)

```phonon
# Kick - Euclidean 4-on-the-floor
kick = sine(55) * "1(4,16)"

# Clap - On the 2 and 4
clap = sine(150) * "~ 1 ~ 1"

# Hi-hats - 16th notes with accents
hats = square(9000) * "[1 0.5]*8"

# Bass - Melodic pattern with rests
bass = saw("[110 110 ~ 165]*2") # lpf(800, 1.5)

# Mix
out kick * 0.3 + clap * 0.2 + hats * 0.05 + bass * 0.25
```

Try it:
```bash
cargo run --bin phonon render house_4x4.phonon house.wav --duration 8
cargo run --bin phonon live house_4x4.phonon
```

---

## What's Implemented

✅ **Mini-Notation** (Tidal/Strudel)
- Euclidean rhythms: `bd(3,8,2)`
- Rests: `bd ~ sn ~`
- Repetition: `bd*4`, `[bd sn]*2`
- Alternation: `<bd sn cp>`
- Stacking: `[bd, sn, hh]`
- Polyrhythms: `(bd,sn cp,hh*3)`
- ~150 pattern operators (fast, slow, rev, every, etc.)

✅ **Synthesis DSL** (Glicol-style)
- Oscillators: sine, saw, square, tri, noise
- Filters: lpf, hpf, bpf
- Bus routing: `~name:` syntax
- Signal arithmetic: `+`, `*` for mixing/modulation
- Pattern-driven parameters

✅ **Pattern-to-Synthesis Integration**
- Patterns drive oscillator frequencies
- Patterns drive filter cutoffs
- Patterns drive amplitude gating
- Full test coverage

---

## Try These Next

```bash
# Explore existing examples
ls *.phonon

# Analyze any example
cargo run --bin phonon render drum_kit.phonon drum_kit.wav --duration 4
cargo run --bin wav_analyze drum_kit.wav

# Start live coding!
cargo run --bin phonon live house_4x4.phonon
```

---

## Tips for Live Coding

1. **Start simple** - Get one sound working, then add layers
2. **Use patterns** - Let `"1(3,8)"` do the rhythm work
3. **Layer with stacks** - `[bd, ~ sn, hh*4]` is powerful
4. **Modulate everything** - Patterns can control filters, frequencies, gates
5. **Save often** - Live mode reloads on every save

Happy coding! 🎵
