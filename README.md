# 🔊 Phonon

**Live coding audio synthesis for Android/Termux - TidalCycles patterns without SuperCollider**

## Setup

```bash
# Install dependencies (Termux/Android)
pkg install rust nodejs git pulseaudio

# For Linux/Ubuntu:
sudo apt install cargo nodejs git libasound2-dev
# Optional for JACK support:
sudo apt install jackd2 qjackctl

# Clone and build
git clone https://github.com/erikgarrison/phonon.git
cd phonon
./setup.sh  # Downloads samples (389MB), installs deps, builds

# Start playing
./phonon start
```

Edit `patterns.phonon` while running to live-code beats!

## Pattern Examples

```javascript
"bd ~ ~ ~"                    // Simple kick
"bd sn bd sn"                 // Basic beat  
"bd*2 sn hh*4"               // Multiply patterns
"bd ~ [sn cp] ~"             // Group patterns
```

## Features

- 🎵 **Real drum samples** - Uses TidalCycles/Dirt-Samples (1800+ sounds)
- 🎼 **Strudel syntax** - Compatible with TidalCycles patterns
- 🔥 **Live coding** - Change patterns while they play
- 📱 **Android native** - Runs entirely in Termux
- 🦀 **Rust synthesis** - Fast, reliable audio generation
- 🌐 **OSC control** - Network-based pattern communication

## Components

- **Fermion** - Rust synthesis engine (FunDSP + sample playback)
- **Boson** - JavaScript pattern sequencer (Strudel mini notation)
- **Parser** - Full Strudel/TidalCycles syntax support

## Architecture

```
📝 patterns.phonon (Live code file)
        ↓
    🔄 Quark (File watcher)
        ↓
    🎼 Boson (Pattern engine)
        ↓
    🎵 Fermion (Synthesis)
        ↓
    🔊 Audio output
```

## Author

Erik Garrison <erik.garrison@gmail.com>

## License

MIT