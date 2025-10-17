# 🔊 Phonon

**Live coding synthesis + patterns in pure Rust**

Phonon is a live coding audio system where **patterns ARE signals**. Unlike Tidal/Strudel (event-based), Phonon uses a unified signal graph where patterns can modulate synthesis parameters in real-time.

---

## Quick Start

```bash
# Build
cargo build --release

# Live coding (auto-reloads on file save)
./target/release/phonon live mytrack.ph

# Render to WAV
./target/release/phonon render input.ph output.wav --duration 10
```

**Create `mytrack.ph`:**
```phonon
# Classic house beat (TidalCycles style!)
out = s("[bd*4, hh*8, ~ sn ~ sn]") * 0.8
```

Save it and hear it change instantly!

---

## What Makes Phonon Different?

### Patterns as Control Signals

**Tidal/Strudel** (event-based):
```haskell
d1 $ sound "bd sn"  # Triggers discrete events
```

**Phonon** (signal-based):
```phonon
out = sine("110 220 440") * 0.2  # Pattern IS the control signal
```

In Phonon, patterns evaluate **at sample rate** (44.1kHz) and can modulate any synthesis parameter:

```phonon
tempo 2.0
~lfo = sine(0.25)
~bass = saw("55 82.5 110") # lpf(~lfo * 2000 + 500, 0.8)
out = ~bass * 0.3
```

This is **not possible** in Tidal/Strudel - patterns trigger events, they don't continuously modulate synthesis.

---

## Features

✅ **Pattern System**:
- Mini-notation: `"bd sn cp hh"`
- Euclidean rhythms: `"bd(3,8)"`
- Alternation: `"bd <sn cp>"`
- Multiplication: `"bd*4"`
- Rests: `"bd . sn ."`
- Grouping: `"[bd sn] hh"`

✅ **Synthesis**:
- Oscillators: `sine`, `saw`, `square`, `noise`
- Filters: `lpf`, `hpf` with Q control
- Signal math: `~a + ~b`, `~osc * 0.5`
- Pattern modulation: `sine("110 220")`

✅ **Sample Playback**:
- Voice-based polyphonic engine (64 voices)
- Samples through effects: `s("bd sn") # lpf(2000, 0.8)`
- Pattern-controlled triggering
- Sample library compatible with Tidal/Dirt-Samples

✅ **Live Coding**:
- Auto-reload on file save
- Sub-millisecond latency
- Real-time audio output

---

## Examples

### Classic House Beat
```phonon
# Four-on-the-floor with hi-hats and snare
out = s("[bd*4, hh*8, ~ sn ~ sn]") * 0.8
```

### Euclidean Rhythms
```phonon
# Tresillo pattern (3-against-8) with layered hi-hats
out = s("[bd(3,8), hh(5,16)]") * 0.7
```

### Dynamic Parameter Patterns
```phonon
# Kicks with varying gain, panning, and speed
out = s("bd*4", "1.0 0.8 0.6 0.4", "-1 0 1 0", "1.0 1.2 0.8 1.5")
```

### Sample Selection
```phonon
# Cycle through different kick samples
out = s("bd:0 bd:1 bd:2 bd:3") * 0.8
```

### Synthesis + Samples
```phonon
# Combine SuperDirt synths with sample patterns
~kick = s("bd ~ bd ~")
~bass = supersaw("55 82.5 110", 0.5, 7)
~hats = s("hh*16", "0.6 0.8 0.7 0.9")
out = (~kick + ~bass * 0.2 + ~hats * 0.3) * 0.7
```

### Effects Processing
```phonon
# Drums through reverb and chorus
~drums = s("[bd sn, hh*8]")
out = reverb(chorus(~drums, 1.0, 0.5, 0.3), 0.7, 0.5, 0.4) * 0.8
```

### LFO Modulation
```phonon
# Pattern-controlled filter cutoff
~lfo = sine(0.25)
~bass = saw(55) # lpf(~lfo * 2000 + 500, 0.8)
out = ~bass * 0.3
```

---

## Installation

### Prerequisites
```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Audio libraries (Linux)
sudo apt install libasound2-dev  # ALSA
# Or for PipeWire/PulseAudio (usually already installed)

# Samples (optional, for s() function)
git clone https://github.com/tidalcycles/Dirt-Samples.git samples
```

### Build
```bash
git clone https://github.com/erikgarrison/phonon.git
cd phonon
cargo build --release
```

### Optional: Add to PATH
```bash
echo 'export PATH="$PATH:/path/to/phonon/target/release"' # ~/.bashrc
source ~/.bashrc
```

---

## Usage

### Live Coding
```bash
phonon live mytrack.ph        # Watches mytrack.ph for changes
phonon live                   # Watches live.ph (default)
```

Edit `mytrack.ph` in your favorite editor. Save to hear changes instantly!

### Render to WAV
```bash
phonon render input.ph output.wav --duration 10
phonon render input.ph output.wav --duration 30 --sample-rate 48000
```

### REPL Mode
```bash
phonon repl    # Interactive REPL (experimental)
```

---

## Language Reference

See **[docs/QUICK_START.md](docs/QUICK_START.md)** for tutorial.

See **[docs/PHONON_CURRENT_STATE.md](docs/PHONON_CURRENT_STATE.md)** for architecture deep-dive.

### Basic Syntax

```phonon
# Comment
tempo 2.0              # Cycles per second

# Bus assignment
~name = expression

# Output (required)
out = expression
```

### Oscillators
```phonon
sine(freq)             # Can use patterns: sine("110 220")
saw(freq)
square(freq)
noise
```

### Filters
```phonon
lpf(cutoff, q)         # Low-pass
hpf(cutoff, q)         # High-pass
```

### Samples
```phonon
s("bd sn cp hh")       # Plays from samples/ directory
```

### Signal Flow
```phonon
source # filter # effect    # Chain operator
```

### Math
```phonon
~a + ~b                # Add
~a * 0.5               # Multiply
~osc * "0.5 1.0"       # Pattern modulation
```

---

## Samples

Put sample files in `samples/` directory:

```
samples/
  bd/BD0000.WAV
  sn/SD0000.WAV
  cp/CP.WAV
  hh/HH0000.WAV
```

Compatible with [Tidal Dirt-Samples](https://github.com/tidalcycles/Dirt-Samples).

---

## Architecture

Phonon uses a **unified signal graph** where patterns, synthesis, and samples all evaluate at sample rate (44.1kHz):

```
Pattern Nodes → Oscillator Nodes → Filter Nodes → Output
     ↓               ↓                  ↓            ↓
Every sample    Every sample      Every sample  44.1kHz
```

This differs from Tidal/Strudel which use **event-based** architecture:

```
Pattern Engine → Events (per cycle) → SuperDirt/WebAudio (per event)
```

### Key Differences

| Feature | Tidal/Strudel | Phonon |
|---------|---------------|--------|
| Pattern evaluation | Per-cycle events | Sample-rate signals |
| Synthesis modulation | Limited | Continuous (patterns can modulate any param) |
| Sample playback | Event-triggered | Event-triggered + signal-graph routing |
| Language | Haskell/JS | Rust |
| Audio engine | SuperCollider/WebAudio | Pure Rust (cpal) |
| Latency | 10-50ms | <1ms |

---

## Status

**Current**: Beta - Full Tidal Cycles sample workflow implemented! 🎉

**Works** (48 tests passing):
- ✅ **s() function** - Full Tidal Cycles mini-notation support
- ✅ **All pattern features** - Euclidean, alternation, layers, subdivision, rests
- ✅ **Sample selection** - `s("bd:0 bd:1 bd:2")`
- ✅ **Parameter patterns** - `s("bd*4", "1.0 0.8 0.6 0.4", "-1 0 1")`
- ✅ **7 SuperDirt synths** - superkick, supersaw, superpwm, superchip, superfm, supersnare, superhat
- ✅ **4 effects** - reverb (Freeverb), distortion, bitcrush, chorus
- ✅ **Pattern modulation** - Any parameter can be pattern-driven
- ✅ **64-voice polyphony** - Sample playback engine
- ✅ **Live coding** - Auto-reload with sub-ms latency

**Architectural Limitations**:
- ⚠️  Synths are continuous (not event-triggered like samples)
- ⚠️  No polyphonic synth voices (workaround: use layered samples)

**Coming Soon**:
- ⏳ Multi-output (`out1`, `out2`, etc.)
- ⏳ Event-triggered synth notes (requires voice manager)
- ⏳ Pattern transformations (`fast`, `slow`, `rev`, `every`)

---

## Contributing

Phonon is experimental! Contributions welcome:

1. Try it out
2. File issues for bugs
3. Submit PRs for features
4. Share your tracks!

---

## Roadmap

See **[docs/PHONON_CURRENT_STATE.md](docs/PHONON_CURRENT_STATE.md)** for detailed discussion of design direction.

Key decisions:
- Stick with signal-graph architecture (not event-based like Tidal)
- Add essential pattern transformations (not all of Tidal)
- Keep simple Rust-native syntax
- Focus on what makes Phonon unique: patterns as control signals

---

## Author

Erik Garrison <erik.garrison@gmail.com>

## License

MIT

---

## Related Projects

- [TidalCycles](https://tidalcycles.org/) - The OG pattern language (Haskell)
- [Strudel](https://strudel.cc/) - Tidal in JavaScript/browser
- [Glicol](https://glicol.org/) - Graph-based live coding (Rust)
- [SuperCollider](https://supercollider.github.io/) - Audio synthesis platform
- [FunDSP](https://github.com/SamiPerttu/fundsp) - Rust audio DSP library

Phonon takes inspiration from all of these but charts its own path.
