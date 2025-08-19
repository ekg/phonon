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