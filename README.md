# 🔊 Phonon Forge

**A quantum-inspired live coding audio synthesis system for Android/Termux**

> *"Where sound particles collide to create music"*

## Components

- **Fermion** - The Rust synthesis engine (FunDSP-based audio generator)
- **Boson** - The pattern engine (Strudel-powered sequencer) 
- **Quark** - The file watcher (monitors and triggers pattern reloads)
- **Gluon** - The API server (binds everything together)
- **Neutrino** - The CLI interface (for direct interaction)

## Quick Start

```bash
# Start the complete system
./phonon start

# Or run components individually
./fermion/fermion serve     # Start synthesis server
./boson/boson watch         # Start pattern engine
./gluon/gluon api           # Start API server
```

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