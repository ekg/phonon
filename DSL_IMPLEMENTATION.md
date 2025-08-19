# Phonon DSL Implementation

## Overview
We've implemented a Strudel-inspired DSL for Phonon that supports pattern-based music creation with samples, notes, and advanced sequencing features.

## Features Implemented

### 1. Pattern Parser (`boson/parser.js`)
- **Frequencies**: Direct frequency specification (e.g., `"440 550 660"`)
- **Note Names**: Musical notation (e.g., `"c4 e4 g4"`) 
- **Samples**: Drum samples (e.g., `"bd sd hh"`)
- **Chords**: Multiple simultaneous notes (e.g., `"[c4,e4,g4]"`)
- **Repeats**: Pattern multiplication (e.g., `"bd*2"` = bd bd)
- **Rests**: Silence markers (`"~"` or `"."`)
- **Durations**: Note lengths (e.g., `"c4:0.5"`)

### 2. Sample Synthesis (`fermion/src/synth.rs`)
Built-in drum sample generation:
- **Kick (bd)**: Low sine wave with exponential decay
- **Snare (sd)**: Noise + tone with fast decay
- **Hi-hat (hh)**: Short noise burst

### 3. OSC Communication
- `/play` - Play frequency/note
- `/sample` - Play sample with speed control
- `/chord` - Play chord (multiple frequencies)
- `/fm` - FM synthesis

## Pattern Syntax

### Basic Patterns
```javascript
"bd ~ sd ~"          // Kick, rest, snare, rest
"c4 e4 g4 c5"        // C major arpeggio
"440 550 660"        // Frequencies in Hz
```

### Advanced Patterns
```javascript
"bd*2 sd hh*4"       // Repeats: kick×2, snare, hihat×4
"[c4,e4,g4] ~"       // Chord followed by rest
"bd:0.5 sd:0.2"      // Custom durations
"kick snare hat"     // Sample aliases
```

## Architecture

```
┌─────────────────┐
│  Pattern File   │ 
│ patterns.phonon │
└────────┬────────┘
         │ Watch & Parse
┌────────▼────────┐      OSC Messages      ┌─────────────┐
│     Boson       │◄──────────────────────►│   Fermion   │
│ (Pattern Engine)│      /play, /sample    │ (Synthesizer)│
└─────────────────┘                         └──────┬──────┘
                                                    │ WAV
                                            ┌───────▼──────┐
                                            │   mplayer    │
                                            │ (Audio Out)  │
                                            └──────────────┘
```

## Sample Mapping

| Pattern | Sample | Description |
|---------|--------|-------------|
| bd, kick | bd | Bass drum |
| sd, snare | sd | Snare drum |
| hh, hihat, hat | hh | Hi-hat |
| oh, openhat | hh | Open hi-hat (same as hh) |
| cp, clap | sd | Clap (mapped to snare) |

## Note Frequency Map
- Supports notes from C0 (16.35 Hz) to B5 (987.77 Hz)
- Sharps supported with # (e.g., c#4, f#3)
- Case-insensitive

## Usage Examples

### Simple Drum Beat
```javascript
"bd bd sd ~ bd ~ sd ~"
```

### Techno Pattern
```javascript
"bd*4 ~ [sd,hh] ~ bd*2 ~ sd hh*8"
```

### Melodic Pattern
```javascript
"c3 ~ e3 ~ g3 ~ c4 ~"
```

### Complex Pattern
```javascript
"[bd,c2]:1.0 ~ sd:0.2 hh*3 [bd,e2] ~ sd [hh,c5]*2"
```

## Future Enhancements

1. **Effects**: Reverb, delay, filters
2. **Euclidean Rhythms**: `"bd(3,8)"` syntax
3. **Pattern Functions**: stack, sequence, overlay
4. **Sample Loading**: Load custom WAV files
5. **MIDI Support**: MIDI note input/output
6. **Pattern Transformations**: reverse, shuffle, rotate
7. **Probability**: `"bd?0.5"` for 50% chance
8. **Variables**: Pattern definitions and reuse

## Implementation Status

✅ Basic pattern parsing
✅ Sample playback
✅ Note-to-frequency conversion
✅ Chord support
✅ Repeat notation
✅ Duration specification
✅ Rest/silence
✅ File watching
✅ OSC communication
✅ Built-in drum synthesis

## Testing

To test the DSL parser:
```bash
node test-dsl.js
```

To run the full system:
```bash
./phonon start
```

Then edit `patterns.phonon` to hear live changes!