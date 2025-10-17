# Phonon Quick Start Guide

## Installation

```bash
cargo build --release
alias phonon='./target/release/phonon'
```

## Your First Sound

Create `hello.ph`:
```phonon
tempo 2.0
out = sine(440) * 0.2
```

Run it:
```bash
phonon live hello.ph    # Live coding (watches for changes)
phonon render hello.ph output.wav --duration 5  # Render to file
```

---

## 5-Minute Tutorial

### 1. Basic Synthesis

```phonon
tempo 1.0
out = sine(220) * 0.2     # A 220Hz sine wave
```

**Try changing**: `sine` → `saw`, `square`, or `noise`

### 2. Pattern-Controlled Frequency

```phonon
tempo 2.0
out = sine("110 220 440") * 0.2   # Pattern changes pitch every trigger
```

**What's happening**: Pattern triggers 3 times per cycle, changing the oscillator frequency

### 3. Drums (Tidal Cycles Style!)

```phonon
tempo 2.0
out = s("bd sn cp hh") * 0.5
```

Phonon supports **full Tidal Cycles mini-notation**:
- `bd sn cp hh` - sequence of samples
- `bd*4` - repeat bd 4 times (subdivision)
- `bd ~ sn ~` - tildes are rests
- `bd(3,8)` - Euclidean rhythm (3 hits in 8 steps)
- `<bd sn hh>` - alternation (cycles through options)
- `[bd, hh*8]` - layering (polyrhythms)
- `bd:0 bd:1 bd:2` - sample selection (different samples from same folder)

### 4. Filters

```phonon
tempo 2.0
~drums = s("bd sn hh*4 cp")
out = ~drums # lpf(2000, 0.8)    # Low-pass filter at 2kHz
```

**Try**: `lpf` → `hpf` (high-pass filter)

### 5. Pattern-Controlled Filter

```phonon
tempo 2.0
~drums = s("bd sn hh*8 cp")
out = ~drums # lpf("500 2000 1000", 0.8)   # Filter sweeps!
```

### 6. LFO Modulation

```phonon
tempo 1.0
~lfo = sine(0.25)                           # 0.25 Hz LFO
~bass = saw(55) # lpf(~lfo * 2000 + 500, 0.8)
out = ~bass * 0.3
```

**What's happening**:
- LFO oscillates between -1 and 1
- Multiply by 2000: -2000 to 2000
- Add 500: -1500 to 2500
- Filter cutoff sweeps from ~0Hz to 2500Hz

### 7. Mixing Multiple Sounds

```phonon
tempo 2.0
~kick = s("bd(3,8)")
~snare = s(". sn . sn")
~hats = s("hh*16") * 0.3
out = (~kick + ~snare + ~hats) * 0.5
```

---

## Tidal Cycles Mini-Notation Reference

Phonon implements **full Tidal Cycles pattern syntax** for the s() function. All features are tested and working!

### Basic Sequences
```phonon
s("bd sn cp hh")          # Play samples in sequence
s("bd sn")                # Two sounds per cycle
s("bd sn cp hh cp sn")    # Six sounds per cycle
```

### Subdivision (Repeat)
```phonon
s("bd*4")                 # Four kicks per cycle (16th notes)
s("hh*8")                 # Eight hi-hats per cycle (32nd notes)
s("bd*2 sn*2")            # Two kicks, then two snares
```

### Rests (Silence)
```phonon
s("bd ~ sn ~")            # Kick, rest, snare, rest
s("bd ~ ~ ~")             # Kick on beat 1 only
s("~ sn ~ sn")            # Snare on beats 2 and 4 (backbeat!)
```

### Euclidean Rhythms
```phonon
s("bd(3,8)")              # 3 kicks distributed over 8 steps
s("bd(5,16)")             # 5 kicks in 16 steps
s("bd(3,8,2)")            # 3 in 8, rotated by 2
```

**Classic Euclidean Patterns**:
- `bd(3,8)` - Tresillo (Cuban rhythm)
- `bd(5,8)` - Cinquillo
- `bd(5,12)` - York-Samai pattern

### Alternation (Choose)
```phonon
s("bd <sn cp>")           # Cycle 1: bd sn, Cycle 2: bd cp
s("<bd sn hh>")           # Rotates through: bd, sn, hh
s("bd <sn cp hh>")        # bd + rotating second sound
```

### Layering (Polyrhythms)
```phonon
s("[bd, hh*8]")           # Kick AND hi-hats together
s("[bd*4, sn*2, hh*8]")   # Three layers: kicks, snares, hats
s("[bd, ~ sn ~ sn]")      # Kick with backbeat
```

### Sample Selection
```phonon
s("bd:0 bd:1 bd:2")       # Different kick samples
s("sn:0 sn:3 sn:7")       # Different snare samples
s("hh:0 hh:1")            # Alternate hi-hat samples
```

Sample numbers correspond to files:
- `bd:0` → `samples/bd/BD0000.WAV`
- `bd:1` → `samples/bd/BD0001.WAV`
- etc.

### Grouping
```phonon
s("[bd bd] sn")           # Two bds in one step: bd bd sn
s("bd [sn cp]")           # bd, then sn+cp together
```

### Classic House Beat
```phonon
tempo 2.0
out = s("[bd*4, hh*8, ~ sn ~ sn]") * 0.8
```
- Four-on-the-floor kick (`bd*4`)
- Eighth-note hi-hats (`hh*8`)
- Snare on beats 2 and 4 (`~ sn ~ sn`)

### Complex Patterns
```phonon
# Euclidean layers
s("[bd(3,8), hh(5,16), ~ sn ~ sn]")

# Alternating samples with rests
s("<bd:0 bd:1 bd:2> ~ sn ~")

# Rapid hi-hats with occasional snare
s("[hh*16, ~ ~ sn ~]")
```

---

## Parameter Patterns

The s() function accepts **pattern strings** for gain, pan, and speed parameters!

### Syntax
```phonon
s("sample_pattern", gain, pan, speed)
```

All parameters are optional and can be:
- **Constants**: `0.8`, `1.2`, `0.0`
- **Pattern strings**: `"1.0 0.8 0.6 0.4"`, `"-1 1 0"`, `"0.5 2.0"`

### Gain Patterns (Dynamics)
```phonon
# Decreasing velocity on repeated hits
s("bd*4", "1.0 0.8 0.6 0.4")

# Accent pattern (like 303)
s("bd*8", "1.0 0.3 0.6 0.3 1.0 0.3 0.6 0.3")

# Constant gain
s("bd sn", 0.8)
```

### Pan Patterns (Stereo)
```phonon
# Ping-pong delay effect
s("hh*8", 0.8, "-1 1 -1 1")

# Sweep left to right
s("bd*4", 1.0, "-1 -0.5 0.5 1")

# Center (default)
s("bd sn", 1.0, 0.0)
```

Pan values: `-1.0` = left, `0.0` = center, `1.0` = right

### Speed Patterns (Pitch)
```phonon
# Varying playback speeds
s("bd*4", 1.0, 0.0, "1.0 1.2 0.8 1.5")

# Octave jumps
s("bd*2", 1.0, 0.0, "1.0 2.0")  # Normal, then double speed (up 1 octave)

# Slow motion
s("bd", 1.0, 0.0, 0.5)          # Half speed (down 1 octave)
```

Speed values: `1.0` = normal, `2.0` = double speed (+1 octave), `0.5` = half speed (-1 octave)

### Combined Parameter Patterns
```phonon
# Full dynamic expression
s("bd*8",
  "1.0 0.7 0.8 0.6 1.0 0.7 0.8 0.6",  # gain
  "-1 -0.5 0 0.5 1 0.5 0 -0.5",        # pan
  "1.0 1.1 0.9 1.2 1.0 1.1 0.9 0.8")   # speed

# Practical: accented kick pattern
s("bd*4", "1.0 0.6 0.8 0.6")
```

---

## Common Patterns

### Kick Drum Pattern
```phonon
s("bd ~ ~ bd ~ ~ bd ~")
s("bd(3,8)")              # Same as above, Euclidean
s("bd*4")                 # Four-on-the-floor
```

### Hi-hat Patterns
```phonon
s("hh*8")                 # 8th notes
s("hh*16")                # 16th notes
s("hh*4 ~ hh*4 ~")        # Syncopated
s("hh(5,16)")             # Euclidean groove
```

### Snare Patterns
```phonon
s("~ sn ~ sn")            # Backbeat (beats 2 and 4)
s("~ sn ~ <sn cp>")       # Alternating snare/clap
s("sn(3,8)")              # Tresillo snare
```

### Complete Rhythm Examples
```phonon
# House
s("[bd*4, hh*8, ~ sn ~ sn]")

# Techno
s("[bd(5,16), hh*16, ~ ~ sn ~]")

# Drum & Bass
s("[bd ~ bd(3,8), hh*32, ~ sn ~ sn]")

# Hip-hop
s("[bd ~ bd ~, ~ sn ~ sn, hh*8]")
```

---

## Signal Graph Operators

### Assignment
```phonon
~name = expression        # Create a named bus
```

### Signal Chain
```phonon
~chain = source # effect1 # effect2
```

### Math Operations
```phonon
~mixed = ~a + ~b          # Add signals
~scaled = ~a * 0.5        # Scale signal
~offset = ~a + 0.2        # Add DC offset
```

### Pattern Modulation
```phonon
~modulated = ~carrier * "0.5 1.0"   # Pattern controls amplitude
```

---

## Available Nodes

### Oscillators
```phonon
sine(freq)                # Sine wave
saw(freq)                 # Sawtooth
square(freq)              # Square wave
noise                     # White noise
```

### SuperDirt Synths
Phonon includes 7 synthesizers from SuperDirt/SuperCollider:

```phonon
superkick(freq, pitch_env, sustain, noise)
# Analog kick drum
# freq: base frequency (typically 40-80 Hz)
# pitch_env: pitch envelope amount (0.0-1.0)
# sustain: decay time (0.05-0.5)
# noise: noise mix (0.0-1.0)

supersaw(freq, amp, detune)
# Detuned sawtooth (analog supersaw)
# freq: base frequency
# amp: amplitude
# detune: detune amount in cents (higher = wider)

superpwm(freq, amp, pwm_rate)
# Pulse-width modulation synthesis
# freq: base frequency
# amp: amplitude
# pwm_rate: LFO rate for PWM (0.1-10 Hz)

superchip(freq, slide, decay)
# Chiptune square wave with pitch slide
# freq: base frequency
# slide: pitch slide amount (semitones)
# decay: envelope decay time

superfm(freq, mod_freq, mod_index)
# FM synthesis (frequency modulation)
# freq: carrier frequency
# mod_freq: modulator frequency (ratio)
# mod_index: modulation depth

supersnare(freq, tone, decay)
# Snare drum (noise + tone)
# freq: body frequency (150-250 Hz)
# tone: tone/noise balance (0.0=noise, 1.0=tone)
# decay: envelope decay

superhat(amp, decay)
# Hi-hat (filtered noise)
# amp: amplitude
# decay: envelope decay (0.01-0.3)
```

**Example synth patterns**:
```phonon
# Kick with pattern pitch envelope
~kick = superkick(60, "0.3 0.7 0.5", 0.1, 0.2)

# Bass line with pattern frequency
~bass = supersaw("55 82.5 110", 0.3, 5)

# Chiptune arpeggio
~arp = superchip("220 330 440 330", 2.0, 0.1)
```

**Note**: Synths are continuous (always on, decay naturally). For triggered notes, use samples with the s() function.

### Effects
```phonon
reverb(input, room_size, damping, wet)
# Freeverb algorithm
# room_size: 0.0-1.0 (room size)
# damping: 0.0-1.0 (high frequency damping)
# wet: 0.0-1.0 (dry/wet mix)

dist(input, drive, mix)
# Distortion (tanh waveshaper)
# drive: 1.0-10.0+ (distortion amount)
# mix: 0.0-1.0 (dry/wet mix)

bitcrush(input, bits, rate)
# Bit reduction
# bits: 1.0-16.0 (bit depth)
# rate: 1.0-16.0 (sample rate reduction)

chorus(input, rate, depth, mix)
# Chorus effect (modulated delay)
# rate: 0.1-10.0 (LFO rate)
# depth: 0.0-1.0 (modulation depth)
# mix: 0.0-1.0 (dry/wet mix)
```

**Example effects chains**:
```phonon
# Drums through reverb
~drums = s("[bd*4, hh*8]")
out = reverb(~drums, 0.7, 0.5, 0.3) * 0.8

# Synth through distortion and chorus
~synth = supersaw(110, 0.5, 7)
~distorted = dist(~synth, 3.0, 0.5)
out = chorus(~distorted, 1.0, 0.5, 0.4) * 0.3
```

### Filters
```phonon
lpf(cutoff, q)            # Low-pass filter
hpf(cutoff, q)            # High-pass filter
```

### Samples
```phonon
s("pattern")              # Play samples from samples/ directory
s("pattern", gain, pan, speed)  # With parameter patterns
```

All of these can take **patterns** as arguments:
```phonon
sine("110 220 440")
saw("55 82.5 110")
lpf("500 2000", 0.8)
superkick(60, "0.3 0.7", 0.1, 0.2)
```

---

## File Structure

### Samples
Put your samples in `samples/` directory:
```
samples/
  bd/
    BD0000.WAV
    BD0001.WAV
  sn/
    SD0000.WAV
  cp/
    CP.WAV
```

Name them by their base name in patterns:
```phonon
s("bd sn cp")    # Plays BD0000.WAV, SD0000.WAV, CP.WAV
```

### .ph Files

```phonon
# Comment with hash
tempo 2.0              # Cycles per second

# Define buses
~kick = s("bd(3,8)")
~bass = saw(55)

# Output (required)
out = ~kick + ~bass * 0.3
```

---

## Live Coding Workflow

1. **Start live mode**:
   ```bash
   phonon live mytrack.ph
   ```

2. **Edit mytrack.ph** in your editor

3. **Save** - Phonon auto-reloads!

4. **Stop** with Ctrl+C

### Quick Silence

If you're stuck with sound playing, edit your file to:
```phonon
out = 0
```

---

## What's Next?

- Add more samples to `samples/` directory
- Experiment with filter sweeps using LFOs
- Try combining multiple rhythm patterns
- Layer synthesis + samples
- Learn more in `docs/PHONON_LANGUAGE_REFERENCE.md` (once updated)

---

## Troubleshooting

### No sound?
- Check `samples/` directory exists
- Check sample files are .WAV format
- Try: `out = sine(440) * 0.5` to test without samples

### Sound too quiet?
- Increase the final multiply: `out = ~mix * 0.8`
- Or remove the multiply entirely (may clip!)

### Sound keeps playing after edit?
- Edit file to: `out = 0`
- Or press Ctrl+C to stop

### Can't find phonon command?
```bash
./target/release/phonon live mytrack.ph    # Use full path
# Or add alias to ~/.bashrc:
alias phonon='/path/to/phonon/target/release/phonon'
```

---

## Complete Examples

### Example 1: Classic House Track
```phonon
tempo 2.0  # 120 BPM (2 cycles per second)

# Four-on-the-floor with hi-hats and snare
~drums = s("[bd*4, hh*8, ~ sn ~ sn]")

# Bass line with pattern frequency
~bass = supersaw("55 82.5 110 82.5", 0.4, 5) # lpf(800, 0.8)

# Combine with effects
out = reverb(~drums + ~bass, 0.5, 0.5, 0.2) * 0.7
```

### Example 2: Euclidean Techno
```phonon
tempo 2.5  # 150 BPM

# Euclidean kick pattern
~kick = s("bd(5,16)", "1.0 0.7 0.9 0.6 0.8")

# Fast hi-hats with pan pattern
~hats = s("hh*16", 0.6, "-1 1 -0.5 0.5")

# Euclidean claps
~claps = s("cp(3,8)")

out = (~kick + ~hats + ~claps * 0.8) * 0.7
```

### Example 3: Drum & Bass with Breaks
```phonon
tempo 3.0  # 180 BPM

# Complex kick pattern with varied samples
~kick = s("bd:0*2 bd:1 ~ bd:0", "1.0 0.8 0.9 0.7")

# Rapid hi-hats with speed modulation
~hats = s("hh*32", 0.5, 0.0, "1.0 1.2 0.9 1.3")

# Snare on 2 and 4
~snare = s("~ sn ~ sn", 1.0, 0.0, "1.0 1.1")

# Distorted bass
~bass = supersaw(55, 0.6, 8) # dist(4.0, 0.6)

out = reverb(~kick + ~hats + ~snare + ~bass * 0.2, 0.3, 0.5, 0.15)
```

### Example 4: Ambient Soundscape
```phonon
tempo 0.5  # Slow (60 BPM)

# LFO for filter modulation
~lfo = sine(0.1) * 0.5 + 0.5

# Detuned saw pad
~pad = supersaw(110, 0.3, 15) # lpf(~lfo * 3000 + 500, 0.7)

# Sparse percussion with sample selection
~perc = s("<bd:2 ~ ~ ~, ~ ~ hh:3 ~>", 0.4)

# Heavy reverb for ambient feel
out = reverb(chorus(~pad + ~perc, 0.5, 0.3, 0.4), 0.9, 0.7, 0.6) * 0.4
```

### Example 5: Hip-Hop Beat with Swing
```phonon
tempo 1.8  # 108 BPM

# Kick pattern with dynamics
~kick = s("bd ~ ~ bd ~ bd ~ ~", "1.0 0.0 0.0 0.8 0.0 0.9 0.0 0.0")

# Snare on 2 and 4
~snare = s("~ sn ~ sn")

# Hi-hat with varied samples for groove
~hats = s("hh:0 hh:1 hh:0 hh:2 hh:0 hh:1 hh:3 hh:1", "0.7 0.5 0.6 0.4 0.7 0.5 0.8 0.5")

# Bitcrushed bass for lo-fi feel
~bass = supersaw("55 ~ 82.5 ~", 0.5, 3) # bitcrush(6.0, 4.0)

out = (~kick + ~snare + ~hats * 0.6 + ~bass * 0.3) * 0.8
```

### Example 6: Live Coding Session Pattern
```phonon
tempo 2.0

# Start with kick
~kick = s("bd*4")

# Add hats
~hats = s("hh*8", "0.6 0.8 0.7 0.9 0.6 0.8 0.7 1.0")

# Layer snare
~snare = s("~ sn ~ sn")

# Optional: add percolating hi-hat pattern
~perc = s("hh(5,16)", 0.4, "-1 1 0")

# Bassline (comment out to remove)
~bass = supersaw("55 55 82.5 55", 0.4, 5) # lpf(1200, 0.9)

# Mix everything
out = reverb(~kick + ~hats + ~snare + ~perc, 0.4, 0.5, 0.2) * 0.8
```

---

## More Examples

See `examples/*.ph` and `demos/*.ph` for more complex patches!

Key example files:
- `examples/house_4x4.ph` - Classic house patterns
- `examples/test_euclid.ph` - Euclidean rhythm experiments
- `demos/house.ph` - Full house track
- `demos/techno.ph` - Techno groove
- `demos/dnb.ph` - Drum & bass
- `demos/hiphop.ph` - Hip-hop beat

---

## Tips for Live Coding

1. **Start Simple**: Begin with just a kick: `out = s("bd*4")`
2. **Build Gradually**: Add one element at a time
3. **Use Buses**: Name your parts with `~` for easy mixing
4. **Comment Liberally**: Use `#` to disable/enable parts quickly
5. **Save Often**: Your editor auto-saves, Phonon auto-reloads!
6. **Experiment**: Try alternation `<>`, euclidean `()`, and layering `[]`
7. **Parameter Patterns**: Add dynamics with gain/pan/speed patterns
8. **Effects Last**: Add reverb/chorus/distortion to the final mix

### Quick Silence
If you need to stop the sound immediately:
```phonon
out = 0
```

### Quick Mute Specific Parts
Comment them out:
```phonon
~kick = s("bd*4")
# ~bass = supersaw(55, 0.5, 5)  # Muted
out = ~kick  # + ~bass
```

---

Happy live coding! 🎵
