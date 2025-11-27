# MIDI Monitoring & Advanced Recording Roadmap

## Vision

Transform MIDI recording from "capture and paste" into a complete **punch-in recording workflow** with real-time monitoring, note duration capture, and smart pattern generation.

## Target Workflow

```phonon
tempo: 0.5

# Connect MIDI device (Alt+M)
# MIDI plays through in real-time via ~midi1 bus

~piano: ~midi1 # saw 440 # adsr 0.01 0.1 0.7 0.2 # reverb 0.5 0.8

# Start recording (Alt+R)
# Play for 4 cycles
# Stop recording (Alt+R)

# Paste complete pattern (Alt+Shift+I):
~melody: slow 4 $ n "c4 e4 g4 ~ c5 d5 ~ e5 f5 g5 ~ a5 ~ ~ ~"
         # gain "0.8 1.0 0.6 ~ 0.9 0.7 ~ 0.85 0.95 1.0 ~ 0.8 ~ ~ ~"
         # legato "0.9 0.5 1.0 ~ 0.8 0.6 ~ 0.7 0.9 1.0 ~ 0.5 ~ ~ ~"

out: ~piano + ~melody * 0.5
```

## Implementation Phases

### ✅ Phase 0: Basic MIDI Recording (COMPLETE)
- [x] MIDI event capture with timing
- [x] Velocity recording
- [x] Pattern generation (notes, n-offsets)
- [x] Manual paste (Alt+I, Alt+N, Alt+V)
- [x] 34 tests passing

**Status**: Production-ready, documented in `MIDI_RECORDING_GUIDE.md`

---

### ✅ Phase 1: MIDI Monitoring (COMPLETE)

**Goal**: Hear MIDI input in real-time while recording

#### Features
- [x] Special `~midi` bus receives all MIDI channels
- [x] Per-channel buses: `~midi1` through `~midi16`
- [x] Real-time note triggering (note-on → trigger synth)
- [x] Note-off handling (release envelope)
- [x] Velocity → gain mapping (tracked in active_notes HashMap)
- [x] Multiple MIDI controllers simultaneously (via shared event queue)

#### Technical Implementation

**1. Add MIDI bus type:**
```rust
pub enum SignalNode {
    // ...
    MidiInput {
        channel: Option<u8>,  // None = all channels, Some(0-15) = specific
        active_notes: HashMap<u8, f32>,  // note → velocity
    }
}
```

**2. Compiler support:**
```rust
// In compositional_compiler.rs
fn is_midi_bus(name: &str) -> bool {
    name == "midi" ||
    (name.starts_with("midi") && name[4..].parse::<u8>().is_ok())
}
```

**3. Real-time triggering:**
```rust
// In modal_editor.rs process_midi_events()
if let MidiMessageType::NoteOn { note, velocity } = event.message_type {
    // Trigger synthesis in ~midi bus
    graph.trigger_midi_note(event.channel, note, velocity);
}
```

#### Example Usage
```phonon
tempo: 0.5

# All MIDI channels mixed
~all: ~midi # saw 440

# Specific channels
~piano: ~midi1 # saw 440 # adsr 0.01 0.1 0.7 0.2
~bass: ~midi2 # square 110 # lpf 500

out: ~piano * 0.6 + ~bass * 0.4
```

#### Tests
- [x] `test_midi_monitoring_basic` - Note-on triggers synth
- [x] `test_midi_monitoring_frequency_change` - Different notes change frequency
- [x] `test_midi_monitoring_channel_filtering` - Per-channel routing
- [x] `test_midi_monitoring_polyphony_tracking` - Multiple notes simultaneously
- [x] `test_midi_monitoring_note_off` - Note-off handling
- [x] `test_midi_to_saw_integration` - MIDI drives oscillator frequency

**Status**: ✅ Implemented in 1 session (6 tests passing)

---

### 📋 Phase 2: Note Duration Capture

**Goal**: Record how long each note is held (legato/staccato)

#### Features
- [ ] Track note-on AND note-off events
- [ ] Calculate duration between them
- [ ] Normalize to pattern grid (0.0-1.0)
  - 0.0 = very short (staccato)
  - 0.5 = half length
  - 1.0 = full sustain (legato)
- [ ] Output as `legato` parameter
- [ ] Aligned with note/velocity patterns

#### Technical Implementation

**1. Enhanced MidiEvent tracking:**
```rust
pub struct NoteEvent {
    pub note: u8,
    pub velocity: u8,
    pub start_us: u64,
    pub end_us: Option<u64>,  // Set when note-off received
}
```

**2. Duration calculation:**
```rust
fn calculate_legato(&self, event: &NoteEvent, quantize_duration: f64) -> f32 {
    let duration_us = event.end_us.unwrap_or(event.start_us) - event.start_us;
    let duration_beats = duration_us as f64 / self.us_per_beat();
    let normalized = duration_beats / quantize_duration;
    normalized.min(1.0) as f32
}
```

**3. Pattern output:**
```rust
pub struct RecordedPattern {
    pub notes: String,
    pub n_offsets: String,
    pub velocities: String,
    pub legato: String,  // NEW
    // ...
}
```

#### Example Usage
```phonon
~melody: n "c4 e4 g4"
         # gain "0.8 1.0 0.6"
         # legato "0.9 0.5 1.0"  # Long, short, tied
```

#### Tests
- [ ] `test_legato_staccato` - Short notes → low legato
- [ ] `test_legato_sustained` - Long notes → high legato
- [ ] `test_legato_alignment` - Legato pattern matches notes

**Estimated effort**: 1 day

---

### 📋 Phase 3: Punch-in Recording

**Goal**: Record while audio is playing, synced to current cycle

#### Features
- [ ] Start recording mid-pattern (punch-in)
- [ ] Sync to current cycle position
- [ ] Quantize relative to playback beat
- [ ] Visual metronome during recording
- [ ] Pre-roll option (count-in before recording)

#### Technical Implementation

**1. Sync to playback:**
```rust
pub fn start_punch_in_recording(&mut self, graph_cycle_position: f64) {
    self.midi_recorder = Some(MidiRecorder::new_with_offset(
        tempo,
        graph_cycle_position
    ));
}
```

**2. Quantize to current beat:**
```rust
fn quantize_with_offset(&self, timestamp_us: u64, cycle_offset: f64) -> f64 {
    let elapsed = /* ... */;
    let beat = elapsed + cycle_offset;
    self.quantize_beat(beat)
}
```

#### Example Workflow
```
1. Pattern is playing: ~drums
2. Press Alt+R (punch-in)
3. Play MIDI keyboard (synced to current cycle)
4. Press Alt+R (punch-out)
5. Paste with automatic cycle alignment
```

#### Tests
- [ ] `test_punch_in_sync` - Recording syncs to current cycle
- [ ] `test_punch_in_quantize` - Events quantized to playback grid

**Estimated effort**: 2 days

---

### 📋 Phase 4: Smart Paste

**Goal**: One-paste solution with everything aligned and wrapped

#### Features
- [ ] Auto-wrap with `$ slow N` based on cycle count
- [ ] Combine note/velocity/legato into single paste
- [ ] Intelligent formatting (multi-line for readability)
- [ ] Preserve alignment with rests
- [ ] Optional: paste as separate patterns or combined

#### Technical Implementation

**1. Formatted output:**
```rust
impl RecordedPattern {
    pub fn to_combined_pattern(&self) -> String {
        let slow_wrapper = if self.cycle_count > 1 {
            format!("slow {} $ ", self.cycle_count)
        } else {
            String::new()
        };

        format!(
            "{}n \"{}\"\n         # gain \"{}\"\n         # legato \"{}\"",
            slow_wrapper,
            self.notes,
            self.velocities,
            self.legato
        )
    }
}
```

**2. Paste modes:**
- `Alt+I` - Note names only (current behavior)
- `Alt+N` - N-offsets only (current behavior)
- `Alt+V` - Velocities only (current behavior)
- `Alt+Shift+I` - **Complete pattern** (notes + velocity + legato + slow wrapper)

#### Example Output
```phonon
# After recording 4 cycles, Alt+Shift+I pastes:
~melody: slow 4 $ n "c4 e4 g4 ~ c5 d5 ~ e5"
         # gain "0.8 1.0 0.6 ~ 0.9 0.7 ~ 0.85"
         # legato "0.9 0.5 1.0 ~ 0.8 0.6 ~ 0.7"
```

#### Tests
- [ ] `test_smart_paste_formatting` - Correct multi-line format
- [ ] `test_smart_paste_slow_wrapper` - Auto-wraps with slow N
- [ ] `test_smart_paste_alignment` - All parameters aligned

**Estimated effort**: 1 day

---

### 📋 Phase 5: Advanced Features (Future)

#### 5a. MIDI CC Recording
- Record mod wheel, pitch bend, expression
- Output as control patterns
- Map to arbitrary parameters

#### 5b. Multi-take Management
- Record multiple takes
- Select best take
- Comp from multiple takes

#### 5c. MIDI File Import
- Load .mid files
- Convert to Phonon patterns
- Preserve tempo/timing

#### 5d. Pattern Editor
- Visual editor for recorded patterns
- Adjust timing/velocity/legato
- Quantize after recording

---

## Testing Strategy

### Unit Tests
- Each phase has dedicated test suite
- Cover edge cases (empty recordings, single notes, chords)
- Performance tests (latency, CPU usage)

### Integration Tests
- End-to-end workflow tests
- Cross-phase feature interaction
- Real MIDI device simulation

### Manual Testing
- Document test procedures
- Example MIDI files for reproducibility
- Performance on real hardware

---

## Success Criteria

**Phase 1 (MIDI Monitoring):**
- ✅ Can hear MIDI keyboard in real-time
- ✅ <10ms latency from key press to sound
- ✅ Multiple MIDI channels work independently
- ✅ No audio glitches during note triggering

**Phase 2 (Note Duration):**
- ✅ Legato patterns generated correctly
- ✅ Staccato vs sustained notes distinguished
- ✅ Aligned with note/velocity patterns

**Phase 3 (Punch-in):**
- ✅ Recording syncs to playback
- ✅ No timing drift over multiple cycles
- ✅ Visual feedback during punch-in

**Phase 4 (Smart Paste):**
- ✅ One-paste generates complete, playable pattern
- ✅ Correctly wrapped with `$ slow N`
- ✅ All parameters aligned

---

## Current Status

- **Phase 0**: ✅ Complete (34 tests passing)
- **Phase 1**: ✅ Complete (6 MIDI monitoring tests passing)
  - Real-time MIDI playthrough working
  - ~midi and ~midi1-~midi16 buses functional
  - Smart paste (Alt+Shift+I) with auto-generated ~rec1, ~rec2, etc.
- **Phase 2**: 📋 Next - Note Duration Capture (legato)
- **Phase 3**: 📋 Planned - Punch-in Recording
- **Phase 4**: 📋 Planned - Multi-line Smart Paste formatting
- **Phase 5**: 📋 Future - MIDI CC, Multi-take, MIDI File Import

**Next Action**: Implement Phase 2 (Note Duration / Legato)
