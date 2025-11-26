# MIDI Recording Guide for Phonon

This guide demonstrates how to use MIDI recording in Phonon's live editor.

## Quick Start

1. **Launch Phonon editor**:
   ```bash
   cargo run --release --bin phonon -- edit
   ```

2. **Connect MIDI device**:
   - Press `Alt+M` to cycle through available MIDI devices
   - Status line shows: `🎹 MIDI: [device name]`

3. **Start recording**:
   - Press `Alt+R` to start recording
   - Status shows: `⏺️ Recording MIDI... (Alt+R to stop)`

4. **Play your MIDI keyboard**:
   - Play notes with varying dynamics
   - Timing is captured relative to tempo (default 120 BPM)

5. **Stop recording**:
   - Press `Alt+R` again
   - Status shows: `⏹️ 8 notes over 2 cycles (4.2s) - Alt+I/N/V to insert (use $ slow 2)`

6. **Insert pattern**:
   - `Alt+I` - Insert note names: `"c4 e4 g4 c5"`
   - `Alt+N` - Insert n-offsets: `"0 4 7 12"` (semitones from lowest note)
   - `Alt+V` - Insert velocities: `"0.79 1.00 0.63 0.55"`

## Usage Examples

### Example 1: Simple Melody with Dynamics

```phonon
tempo: 2.0

# Record a melody (Alt+R, play notes, Alt+R)
# Then insert with Alt+I
~melody: n "c4 e4 g4 e4"

# Insert velocity pattern with Alt+V
~vel: "0.8 1.0 0.6 0.7"

# Play with recorded dynamics
out: ~melody # gain ~vel
```

### Example 2: Multi-Cycle Patterns

If you record over 2 cycles, use `$ slow 2` to fit it properly:

```phonon
tempo: 2.0

# Recorded over 2 cycles: 8 notes
~long_melody: n "c4 d4 e4 f4 g4 a4 b4 c5" $ slow 2

# Velocities also recorded over 2 cycles
~dynamics: "0.5 0.6 0.7 0.8 0.9 1.0 0.8 0.6" $ slow 2

out: ~long_melody # gain ~dynamics
```

### Example 3: N-Offsets for Transposition

Using n-offsets lets you easily transpose patterns:

```phonon
tempo: 2.0

# Base note and intervals recorded with Alt+N
# Base: C4, pattern: C-E-G-C (0, 4, 7, 12)
~intervals: n "0 4 7 12"

# Transpose up an octave by adding 12
~transposed: ~intervals + 12

out: ~intervals # adsr 0.01 0.1 0.7 0.2
```

### Example 4: Hi-Hat with Accents

```phonon
tempo: 2.0

# Record hi-hat pattern with accents (hit harder on 1 and 3)
# Use velocity to control gain
~hats: n "42 42 42 42"  # MIDI note 42 = closed hi-hat
~accents: "1.0 0.5 0.8 0.5"  # Recorded velocities

out: ~hats # gain ~accents
```

## Quantization

MIDI recording uses quantization to snap notes to a grid:

- **Default**: 16th note grid (16 divisions per cycle)
- **Adjustable**: Can be set to 4, 8, 16, 32, etc.

Currently quantization is fixed at 16. Future versions will allow changing it via UI.

## Tips for Best Results

1. **Set tempo before recording**: The tempo determines the quantization grid
2. **Play with the metronome**: Align your playing to the cycle boundaries
3. **Use multiple cycles for long patterns**: Record over 2-4 cycles, then use `$ slow N`
4. **Record velocity separately if needed**: You can record notes first, then record velocity in a second pass
5. **N-offsets for reusable patterns**: Using n-offsets makes patterns more reusable

## Command Console Reference

Press `Alt+/` to open the command console, then type:

```
/help         - Show all commands
/functions    - List all Phonon functions
/search midi  - Search for MIDI-related functions
```

## MIDI Input Section:

```
Alt+M  - Connect to MIDI device (cycle through)
Alt+R  - Start/stop MIDI recording
Alt+I  - Insert recorded pattern (note names)
Alt+N  - Insert recorded pattern (n-offsets from lowest)
Alt+V  - Insert recorded velocities (as gain pattern)

Tip: If recorded over N cycles, use $ slow N to fit pattern
```

## Technical Details

### Pattern Format

**Note names** (`Alt+I`):
```
"c4 e4 g4"           # Simple notes
"c4 ~ g4"            # With rests
"c4 ~@3 g4"          # Multiple consecutive rests
"[c4,e4,g4]"         # Chords (simultaneous notes)
```

**N-offsets** (`Alt+N`):
```
"0 4 7"              # Semitone offsets from base note
"0 ~ 7"              # With rests
```

**Velocities** (`Alt+V`):
```
"0.79 1.00 0.63"     # Normalized 0.0-1.0 (MIDI 0-127)
"1.0 ~ 0.5"          # With rests (aligned to notes)
```

### Velocity Normalization

MIDI velocities (0-127) are normalized to 0.0-1.0:
- MIDI 1 → 0.008 (very soft)
- MIDI 64 → 0.504 (medium)
- MIDI 127 → 1.0 (maximum)

This makes them directly usable with Phonon's `gain` parameter.

### Timing Capture

- Timestamps are captured in microseconds
- Quantized to grid (4th, 8th, 16th note divisions)
- Rests automatically inserted for timing gaps
- Cycle count calculated based on pattern length

## Known Limitations

1. **Chord velocities**: Currently chords use the velocity of the first note
   - Future: Per-note velocity in chords
2. **Quantization not adjustable**: Fixed at 16th notes
   - Future: UI control for quantization level
3. **No visual feedback during recording**: Only status line
   - Future: Real-time note display
4. **No editing of recorded patterns**: Must re-record to change
   - Future: Pattern editor

## Troubleshooting

**No MIDI devices found**:
- Check MIDI device is connected and powered on
- On Linux: Check ALSA/JACK permissions
- Try `Alt+M` to refresh device list

**Notes not captured**:
- Ensure recording is started (`Alt+R`)
- Check note-on velocity > 0
- Some MIDI controllers send note-off as note-on with velocity 0

**Pattern spans too many cycles**:
- Use `$ slow N` to fit pattern
- Or re-record with tighter timing
- Status line shows cycle count after recording

**Timing feels off**:
- Check tempo matches your playing
- Quantization snaps to nearest grid point
- Try recording at slower tempo, then speed up

## Examples Directory

See `docs/examples/` for more MIDI recording examples:
- `midi_basic.ph` - Simple note recording
- `midi_dynamics.ph` - Velocity control examples
- `midi_polyrhythm.ph` - Multi-pattern recording

## Testing MIDI Recording

See `tests/test_midi_recording_comprehensive.rs` for automated tests covering:
- Velocity capture (soft to loud)
- Timing with rests
- N-offset patterns
- Multi-cycle recording
- Edge cases (uniform velocity, very soft notes)

Run tests:
```bash
cargo test --test test_midi_recording_comprehensive
```
