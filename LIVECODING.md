# Phonon Live Coding Guide

## Quick Start

### Play a .phonon file
```bash
# Play for default 4 seconds
./phonon-play.sh examples/live_beat.phonon

# Play for 8 seconds
./phonon-play.sh examples/live_beat.phonon 8

# Play inline DSL code
./phonon-play.sh -c $'~kick: impulse 4 >> mul 80 >> lpf 100 0.9\nout: ~kick' 2
```

### Live coding (auto-reload on save)
```bash
# Start live coding session with default file
./phonon-live.sh

# Live code a specific file
./phonon-live.sh my_beat.phonon 4

# Or use cargo directly
cargo run --example live_phonon examples/live_session.phonon 4
```

## DSL Syntax

### Basic Structure
```phonon
# Comments start with #
~name: chain >> of >> nodes    # Reference chain
out: final >> output >> chain   # Output (required)
```

### Oscillators & Generators
- `sin <freq>` - Sine wave
- `saw <freq>` - Sawtooth wave
- `square <freq>` - Square wave
- `triangle <freq>` - Triangle wave
- `noise` - White noise
- `pink` - Pink noise
- `brown` - Brown noise
- `impulse <freq>` - Impulse train (for rhythms)

### Math Operations
- `mul <value>` - Multiply (volume/amplitude)
- `add <value>` - Add offset
- `sub <value>` - Subtract
- `div <value>` - Divide

### Filters
- `lpf <cutoff> <resonance>` - Lowpass filter
- `hpf <cutoff> <resonance>` - Highpass filter

### Effects
- `delay <time> <feedback>` - Delay line
- `reverb <room> <damping>` - Simple reverb
- `env <attack> <decay> <sustain> <release>` - ADSR envelope
- `clip <min> <max>` - Hard clipping

### Signal Mixing
- `+` - Mix signals: `~a + ~b`
- `*` - Multiply signals: `~trigger * ~sound`

## Example Patterns

### Basic Kick Drum
```phonon
~kick: impulse 4 >> mul 100 >> lpf 80 0.95
out: ~kick
```

### Kick + Hihat
```phonon
~kick: impulse 4 >> mul 80 >> lpf 100 0.9
~hihat: impulse 8 >> mul 10 >> noise >> mul 0.2 >> hpf 8000 0.9
out: ~kick + ~hihat >> mul 0.8
```

### 4-on-floor with Clap
```phonon
~kick: impulse 4 >> mul 100 >> lpf 80 0.95
~clap: impulse 2 >> delay 0.25 0.0 >> mul 50 >> noise >> mul 0.4 >> hpf 1200 0.7
~hihat: impulse 8 >> mul 15 >> noise >> mul 0.2 >> hpf 7000 0.9
~drums: ~kick + ~clap + ~hihat
out: ~drums >> lpf 2500 0.6 >> mul 0.8
```

### Techno Pattern
```phonon
~kick: impulse 4 >> mul 120 >> lpf 50 0.98
~bass: impulse 4 >> delay 0.125 0.0 >> mul 30 >> lpf 200 0.9
~tick: impulse 16 >> mul 5 >> hpf 10000 0.95
out: ~kick + ~bass + ~tick >> mul 0.6
```

### Ambient Percussion
```phonon
~pulse: impulse 2 >> mul 40 >> lpf 300 0.7 >> reverb 0.8 0.3
~shimmer: impulse 8 >> mul 8 >> noise >> mul 0.1 >> hpf 12000 0.9 >> delay 0.125 0.4
out: ~pulse + ~shimmer >> mul 0.5
```

## Tips

1. **Rhythm Values**: 
   - `impulse 2` = half notes
   - `impulse 4` = quarter notes 
   - `impulse 8` = eighth notes
   - `impulse 16` = sixteenth notes

2. **Filter Cutoffs**: 
   - Bass: 50-200 Hz
   - Mid: 200-2000 Hz
   - Highs: 2000-20000 Hz

3. **Delay Times** (at 120 BPM):
   - `0.125` = sixteenth note
   - `0.25` = eighth note
   - `0.5` = quarter note
   - `0.75` = dotted quarter

4. **Mixing**: Keep total output around 0.8 to avoid clipping

## Current Limitations

- References (~name) cannot be used as modulation parameters yet
- Pattern strings ("bd*4") not fully integrated with DSP
- No LFO modulation of parameters (working on it!)

## Files

- `examples/live_session.phonon` - Interactive live coding template
- `examples/live_beat.phonon` - Basic drum beat
- `examples/demo_beat.rs` - Rust example showing DSP usage