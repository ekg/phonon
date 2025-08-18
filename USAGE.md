# Phonon Forge Usage Guide

## Quick Start

```bash
# Start the complete system
./phonon start

# The system will start playing the pattern automatically
# Edit patterns.phonon in another terminal to change the music live!
```

## Available Commands

- `./phonon start` - Start complete system (Fermion + Boson)
- `./phonon test` - Test audio output with a sine wave
- `./phonon edit` - Edit the pattern file
- `./phonon build` - Rebuild Fermion if needed
- `./phonon stop` - Stop all components

## Pattern Syntax

Edit `patterns.phonon` to change the music:

```javascript
// Frequencies in Hz
"440 550 660 550"

// Note names (c3-c5 range)
"c4 e4 g4 c5"

// With rests (~ = silence)
"440 ~ 660 ~"

// Examples:
"261.63 329.63 392 523.25"  // C major arpeggio
"60 ~ 200 ~"                 // Kick-snare pattern
```

## Live Coding Workflow

1. **Terminal 1**: Start the system
   ```bash
   ./phonon start
   ```

2. **Terminal 2**: Edit patterns
   ```bash
   nano patterns.phonon
   # Make changes and save - audio updates instantly!
   ```

## Architecture

- **Fermion** (Rust): Synthesis engine using FunDSP
  - Receives OSC messages on port 57120
  - Generates WAV files and plays via mplayer
  
- **Boson** (Node.js): Pattern engine using Strudel
  - Watches pattern file for changes
  - Sends OSC to Fermion
  - Handles timing and sequencing

## LLM Integration

You can control Phonon Forge programmatically:

```bash
# Change pattern via file
echo '"440 523 659 784"' > patterns.phonon

# Or use OSC directly (requires OSC client)
oscsend localhost 57120 /play f 440.0 f 0.25
```

## Tips

- Keep patterns simple - the parser is basic
- Frequencies work best between 60-2000 Hz
- Tempo is set to 120 BPM (edit in boson/boson.js)
- Each note plays for 200ms by default

## Troubleshooting

If no sound:
1. Check mplayer is installed: `which mplayer`
2. Check PulseAudio: `pactl info`
3. Check processes: `ps aux | grep -E "fermion|boson"`
4. Rebuild if needed: `./phonon build`

## Advanced

To run components separately:

```bash
# Terminal 1: Synthesis server
./fermion/target/release/fermion serve

# Terminal 2: Pattern engine
cd boson && node boson.js watch
```