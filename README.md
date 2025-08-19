# 🔊 Phonon

**Live coding audio synthesis for Android/Termux - TidalCycles patterns without SuperCollider**

## Quick Start

```bash
# 1. Clone the repository
git clone https://github.com/erikgarrison/phonon.git
cd phonon

# 2. Run setup (downloads samples, builds everything)
./setup.sh

# 3. Start making music
./phonon start

# 4. Edit patterns.phonon to change the beat live!
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