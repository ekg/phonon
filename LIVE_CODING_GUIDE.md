# 🎵 Phonon Live Coding Guide

Edit `.phonon` files and hear changes instantly!

## Quick Start

```bash
cargo run --example phonon_live
```

Then edit `live.phonon` in Emacs/Vim and save - **changes reload automatically!**

## How It Works

1. **Run the watcher:**
   ```bash
   cargo run --example phonon_live           # watches live.phonon
   cargo run --example phonon_live bass.phonon   # watches bass.phonon
   ```

2. **Edit the .phonon file** in your editor

3. **Save** - audio updates immediately!

## Syntax (Simplified DSL)

### Basic Structure
```phonon
# Comments start with #
tempo 2.0              # Sets cycles per second

lfo = sine(0.5)        # Create named signals
bass = saw(110)        # Reference them later

out bass * lfo * 0.3   # Define output
```

### Oscillators
```phonon
sine(440)              # Sine wave at 440 Hz
saw(110)               # Sawtooth wave
square(220)            # Square wave
tri(330)               # Triangle wave
noise                  # White noise
```

### Patterns
```phonon
kick = "1 0 0 1"       # Tidal-style pattern
snare = "0 1 0 0"      # Triggers on 2nd beat
hats = "1 1 1 1"       # Every beat
```

### Operations
```phonon
a + b                  # Add signals
a * b                  # Multiply/ring modulate
sine(440) * 0.5        # Scale amplitude
```

### Example Files

**live.phonon** - Main playground
```phonon
tempo 2.0
lfo = sine(0.5) * 0.5 + 0.5
bass = saw(110)
rhythm = "1 0 1 0"
out bass * rhythm * 0.3
```

**bass.phonon** - Bass with modulation
```phonon
tempo 1.0
lfo = sine(0.25)
bass = saw(55)
out bass * (lfo * 0.5 + 0.5) * 0.3
```

**drums.phonon** - Rhythm patterns
```phonon
tempo 2.0
kick = "1 0 0 1"
snare = "0 1 0 1"
out kick * 0.5
```

**ambient.phonon** - Detuned pads
```phonon
tempo 0.5
osc1 = sine(220)
osc2 = sine(220.5)
osc3 = sine(330) * 0.3
out (osc1 + osc2 + osc3) * 0.1
```

## Live Coding Tips

### Start Simple
```phonon
out sine(440) * 0.2
```

### Add Movement
```phonon
lfo = sine(2)
out sine(440) * lfo * 0.2
```

### Add Rhythm
```phonon
beat = "1 0 1 0"
out sine(440) * beat * 0.3
```

### Build Complexity
```phonon
lfo = sine(0.5) * 0.5 + 0.5
bass = saw(55)
kick = "1 0 0 1"
out bass * (1 - kick * 0.5) * lfo * 0.3  # Sidechain!
```

## Common Patterns

### Sidechain Compression
```phonon
kick = "1 0 0 0"
sidechain = 1 - (kick * 0.8)
out bass * sidechain * 0.4
```

### Filter Sweep (conceptual)
```phonon
lfo = sine(0.25) * 0.5 + 0.5
# Filter not yet implemented, but you can simulate with amplitude
out saw(110) * lfo * 0.3
```

### Polyrhythms
```phonon
beat1 = "1 0 1 0"
beat2 = "1 0 0"
out (sine(220) * beat1 + saw(110) * beat2) * 0.2
```

## Limitations (Current)

The simplified parser currently supports:
- ✅ Basic oscillators (sine, saw, square, tri, noise)
- ✅ Patterns in quotes
- ✅ Basic math (+, *)
- ✅ Named signals/buses
- ⚠️ No filters yet (lpf, hpf)
- ⚠️ No effects (delay, reverb)
- ⚠️ No envelopes yet

These features exist in the UnifiedSignalGraph but need parser support.

## Why .phonon Files?

- **No compilation** - instant feedback
- **Simple syntax** - focus on sound, not code
- **Hot reload** - save and hear
- **Multiple files** - organize your patches
- **Version control** - track your compositions

## Troubleshooting

**No sound?**
- Check system audio settings
- Verify the output line: `out something * 0.2`

**Parse errors?**
- Check syntax - needs spaces around operators
- Patterns need quotes: `"1 0 1 0"`
- Numbers for frequencies: `sine(440)` not `sine(A4)`

**Crackling?**
- Lower the amplitude (multiply by smaller number)
- Simplify the patch

## Next Steps

Once comfortable:
1. Create your own .phonon files
2. Combine multiple oscillators
3. Experiment with patterns
4. Layer rhythms and melodies

The full DSL will eventually support:
- Filter chains: `saw(110) >> lpf(1000, 2)`
- Effects: `signal >> delay(0.25, 0.5)`
- Complex routing: `route lfo -> {cutoff: 1000, pan: 0.5}`
- Inline synths: `synthdef bass { ... }`

Happy live coding! 🎶