# GitHub Repository Structure for Phonon Forge

## Main Repository
**Name:** `phonon-forge`  
**Description:** Live coding audio synthesis system for Android/Termux with Strudel/TidalCycles pattern support  
**URL:** `github.com/erikgarrison/phonon-forge`

### Repository Contents:
```
phonon-forge/
├── README.md                    # Project overview
├── LICENSE                      # MIT License
├── USAGE.md                     # User guide
├── DSL_IMPLEMENTATION.md        # Pattern language docs
├── SAMPLES.md                   # Sample system docs
├── SYSTEM_STATUS.md             # Current status report
├── .gitignore                   # Git ignore file
│
├── fermion/                     # Rust synthesis engine
│   ├── Cargo.toml
│   ├── Cargo.lock
│   └── src/
│       ├── main.rs
│       ├── server.rs
│       └── synth.rs
│
├── boson/                       # Pattern engine
│   ├── package.json
│   ├── package-lock.json
│   ├── boson.js                # Main pattern engine
│   ├── boson-strudel.js        # Strudel integration
│   └── parser.js               # DSL parser
│
├── patterns.phonon              # Live-editable patterns
├── phonon                       # Main orchestrator script
├── get-samples.sh              # Download Dirt-Samples
├── install-strudel.sh          # Install Strudel packages
└── examples/                    # Example patterns
    ├── house.phonon
    ├── techno.phonon
    └── ambient.phonon
```

## Optional Separate Repositories

### 1. `fermion-synth`
**If you want Fermion as a standalone project:**
- Rust audio synthesis engine using FunDSP
- OSC server for real-time control
- Sample generation and playback
- URL: `github.com/erikgarrison/fermion-synth`

### 2. `boson-sequencer`
**If you want Boson as a standalone project:**
- JavaScript pattern sequencer
- Strudel/TidalCycles compatible
- File watching for live coding
- URL: `github.com/erikgarrison/boson-sequencer`

## Setup Instructions for GitHub

```bash
# Create main repository
cd /data/data/com.termux/files/home/phonon-forge

# Add remote origin
git remote add origin https://github.com/erikgarrison/phonon-forge.git

# Create and switch to main branch
git branch -M main

# Push to GitHub
git push -u origin main

# Create release tag
git tag -a v1.1 -m "Phonon Forge v1.1 - Strudel DSL support"
git push origin v1.1
```

## Repository Settings

### Topics/Tags:
- live-coding
- audio-synthesis
- android
- termux
- rust
- tidal-cycles
- strudel
- osc
- fundsp
- algorithmic-music

### Description:
"Live coding audio synthesis for Android/Termux. TidalCycles/Strudel pattern support without SuperCollider. Rust synthesis engine + JavaScript sequencer."

### README Badges:
```markdown
![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Version](https://img.shields.io/badge/version-1.1-green.svg)
![Platform](https://img.shields.io/badge/platform-Android%2FTermux-orange.svg)
```

## Related Projects to Link

- [TidalCycles/Dirt-Samples](https://github.com/tidalcycles/Dirt-Samples) - Sample library
- [Strudel](https://strudel.cc) - Pattern inspiration
- [FunDSP](https://github.com/SamiPerttu/fundsp) - Audio synthesis library

## Actions/CI (Optional)

Create `.github/workflows/build.yml`:
```yaml
name: Build
on: [push, pull_request]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: Install Rust
        uses: actions-rs/toolchain@v1
      - name: Build Fermion
        run: cd fermion && cargo build --release
      - name: Test Fermion
        run: cd fermion && cargo test
```