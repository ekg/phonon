# Phonon Sample System

## Getting the Samples

Run this command to download the official Dirt-Samples (same as Strudel/TidalCycles):

```bash
chmod +x get-samples.sh
./get-samples.sh
```

This will clone the Dirt-Samples repository from:
https://github.com/tidalcycles/Dirt-Samples

## Sample Syntax (Strudel/Tidal Compatible)

### Basic Usage
- `"bd"` - plays the first kick drum sample (bd/BT0A0A7.wav)
- `"bd:1"` - plays the second kick drum sample  
- `"bd:2"` - plays the third kick drum sample
- etc.

### Available Sample Banks

From the Dirt-Samples collection:

| Pattern | Description | Example Files |
|---------|-------------|---------------|
| `bd` | Kick drums | BT0A0A7.wav, BT0AAD0.wav, ... |
| `sn` | Snare drums | ST0T0S0.wav, ST0T0S3.wav, ... |
| `hh` | Closed hi-hats | HH0-000.wav, HH0-001.wav, ... |
| `oh` | Open hi-hats | HH1-001.wav, HH1-002.wav, ... |
| `cp` | Hand claps | HANDCLAP0.wav, HANDCLP1.wav, ... |
| `cr` | Crash cymbals | 001_CRASHCYMBAL.wav, ... |
| `rs` | Rimshots | SIDESTICK.wav, ... |
| `cb` | Cowbells | COWBELL1.wav, ... |
| `lt` | Low toms | LOWTOM0.wav, ... |
| `mt` | Mid toms | MIDTOM0.wav, ... |
| `ht` | High toms | HIGHTOM0.wav, ... |
| `bass` | Bass sounds | BASS0.wav, BASS1.wav, ... |

### Pattern Examples

```javascript
// Basic beat
"bd ~ sn ~"

// Using different samples
"bd:0 bd:1 sn:2 hh:3"

// Combining with notes
"bd ~ sn ~ c4 e4 g4 ~"

// Complex pattern
"bd*2 [sn:1,hh] ~ cp:2"
```

### How It Works

1. When you write `"bd"`, the parser converts it to `{type: 'sample', value: 'bd', index: 0}`
2. When you write `"bd:3"`, it becomes `{type: 'sample', value: 'bd', index: 3}`
3. Boson sends an OSC message: `/sample bd 3 1.0`
4. Fermion looks in `dirt-samples/bd/` folder
5. Lists all .wav files and sorts them
6. Selects the file at index 3 (wrapping if necessary)
7. Loads and plays the sample

### Directory Structure

```
phonon/
├── dirt-samples/        # Cloned from GitHub
│   ├── bd/             # Kick drums
│   │   ├── BT0A0A7.wav # bd:0
│   │   ├── BT0AAD0.wav # bd:1
│   │   └── ...
│   ├── sn/             # Snares
│   ├── hh/             # Hi-hats
│   └── ...
└── samples/            # Symlink to dirt-samples
```

### Adding Custom Samples

1. Create a folder in `dirt-samples/` or `samples/`
2. Add WAV files to the folder
3. Use the folder name in patterns:
   ```javascript
   "mysamples:0 mysamples:1 mysamples:2"
   ```

### Aliases

The parser supports these aliases for convenience:

- `kick` → `bd`
- `snare` → `sn`
- `hihat`, `hat` → `hh`
- `openhat`, `openhihat` → `oh`
- `clap` → `cp`
- `crash`, `cymbal` → `cr`
- `rim`, `rimshot` → `rs`
- `cowbell` → `cb`

So `"kick:2"` is the same as `"bd:2"`

## Fallback Samples

If the Dirt-Samples aren't available, Fermion generates basic samples:
- `bd` - Synthetic kick (60Hz sine with envelope)
- `sn` - Synthetic snare (noise + 200Hz tone)
- `hh` - Synthetic hi-hat (short noise burst)

## Speed/Pitch Control

Coming soon: `"bd:2*2"` for double speed, `"bd:2/2"` for half speed

## Envelope Modifiers

Control how samples fade in and out using envelope modifiers with the chain operator `#`:

### Segments Envelope (Arbitrary Breakpoint)

Create custom envelope shapes by defining levels and times:

```phonon
-- Triangle envelope: 0 -> 1 -> 0
s "bd sn" # segments "0 1 0" "0.1 0.2"
```

- First string: level values (0.0 to 1.0)
- Second string: time durations in seconds
- N levels require N-1 times

Examples:
```phonon
-- Fast attack, slow release
s "hh*4" # segments "0 1 0" "0.01 0.3"

-- Complex shape
s "bd" # segments "0 0.8 1 0.5 0" "0.05 0.05 0.1 0.15"
```

### ADSR Envelope (Attack-Decay-Sustain-Release)

Classic synthesizer envelope for sustained sounds:

```phonon
-- syntax: adsr attack decay sustain release
s "bd sn" # adsr 0.01 0.1 0.5 0.2
```

Parameters:
- `attack`: Time to reach peak (seconds)
- `decay`: Time to reach sustain level (seconds)
- `sustain`: Sustain level (0.0 to 1.0)
- `release`: Time to fade to zero after note off (seconds)

Examples:
```phonon
-- Punchy kick with fast attack
s "bd*4" # adsr 0.001 0.05 0.3 0.1

-- Soft pad-like envelope
s "sn" # adsr 0.1 0.2 0.7 0.5
```

### Curve Envelope (Exponential/Logarithmic)

Create shaped ramps with exponential curves:

```phonon
-- syntax: curve start end duration curvature
s "hh*8" # curve 0 1 0.05 2
```

Parameters:
- `start`: Starting level (0.0 to 1.0)
- `end`: Ending level (0.0 to 1.0)
- `duration`: Duration in seconds
- `curvature`: Shape (-10 to +10, 0 = linear, positive = exponential, negative = logarithmic)

Examples:
```phonon
-- Linear fade out
s "bd" # curve 1 0 0.3 0

-- Exponential decay
s "sn" # curve 1 0 0.2 5

-- Logarithmic fade in
s "hh" # curve 0 1 0.1 -5
```

### Combining Envelopes with Other Parameters

Envelopes can be combined with other sample parameters:

```phonon
tempo: 2.0

-- Segments envelope with pattern gain
~drums: s "bd sn hh cp" # segments "0 1 0" "0.05 0.1" # gain "1 0.8 0.6 0.4"

-- ADSR with pan
~bass: s "bd*4" # adsr 0.01 0.1 0.5 0.2 # pan "-1 1"

-- Curve with reverb
~perc: s "cp" # curve 0 1 0.15 3 # reverb 0.8 0.5 0.3

out: ~drums * 0.4 + ~bass * 0.4 + ~perc * 0.2
```

### Default Envelope

If no envelope modifier is specified, samples use a simple percussion envelope:
- Attack: 0.01 seconds (hard-coded)
- Release: 0.2 seconds (default, can be overridden with `release` parameter)

```phonon
-- These are equivalent:
s "bd"
s "bd" # attack 0.01 # release 0.2
```