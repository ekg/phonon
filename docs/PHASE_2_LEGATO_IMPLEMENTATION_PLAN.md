# Phase 2: Note Duration / Legato Capture - Implementation Plan

**Goal**: Capture how long each note is held and output as a `legato` parameter for expressive pattern playback.

## Overview

Currently, MIDI recording captures:
- ✅ Note pitches (c4, e4, g4)
- ✅ Velocities (0.79, 1.00, 0.63)
- ❌ Note durations (how long each note is held)

Phase 2 adds:
- Note-on → note-off duration tracking
- Normalization to pattern grid (0.0 = staccato, 1.0 = full sustain)
- `legato` parameter output
- Integration with smart paste

## Target Output

### Before Phase 2 (Current)
```phonon
~rec1: slow 4 $ n "c4 e4 g4 a4"
       # gain "0.8 1.0 0.6 0.9"
```

### After Phase 2 (With Legato)
```phonon
~rec1: slow 4 $ n "c4 e4 g4 a4"
       # gain "0.8 1.0 0.6 0.9"
       # legato "0.9 0.5 1.0 0.8"
```

Where:
- `0.9` = Long, sustained note (90% of its slot duration)
- `0.5` = Short, staccato note (50% of its slot duration)
- `1.0` = Fully tied/legato note (100% of its slot duration)

## Implementation Steps

### Step 1: Enhanced Note Tracking (30 min)

**Current**: `MidiRecorder` only tracks note-on events
**Goal**: Track both note-on AND note-off with timestamps

**File**: `src/midi_input.rs`

**Add NoteEvent struct**:
```rust
/// Represents a complete MIDI note with start and end times
#[derive(Debug, Clone)]
struct NoteEvent {
    pub note: u8,
    pub velocity: u8,
    pub start_us: u64,      // When note-on received
    pub end_us: Option<u64>, // When note-off received (None if still active)
}
```

**Modify MidiRecorder**:
```rust
pub struct MidiRecorder {
    // ...existing fields...

    /// Track active notes (note number → NoteEvent)
    active_notes: HashMap<u8, NoteEvent>,

    /// Completed notes with full duration info
    completed_notes: Vec<NoteEvent>,
}
```

**Update record_event()**:
```rust
pub fn record_event(&mut self, event: MidiEvent) {
    match event.message_type {
        MidiMessageType::NoteOn { note, velocity } if velocity > 0 => {
            // Start tracking this note
            self.active_notes.insert(note, NoteEvent {
                note,
                velocity,
                start_us: event.timestamp_us,
                end_us: None,
            });
        }
        MidiMessageType::NoteOff { note, .. } |
        MidiMessageType::NoteOn { note, velocity: 0 } => {
            // Complete the note duration
            if let Some(mut note_event) = self.active_notes.remove(&note) {
                note_event.end_us = Some(event.timestamp_us);
                self.completed_notes.push(note_event);
            }
        }
        _ => {}
    }
}
```

---

### Step 2: Legato Calculation (45 min)

**Goal**: Convert note duration (microseconds) to legato value (0.0-1.0)

**Add to MidiRecorder**:
```rust
impl MidiRecorder {
    /// Calculate legato value for a note
    ///
    /// # Arguments
    /// * `note_event` - The completed note with start/end times
    /// * `quantize_duration` - The quantized slot duration in beats
    ///
    /// # Returns
    /// Normalized legato value (0.0 = very short, 1.0 = full duration)
    fn calculate_legato(&self, note_event: &NoteEvent, quantize_duration: f64) -> f32 {
        let duration_us = match note_event.end_us {
            Some(end) => end.saturating_sub(note_event.start_us),
            None => return 1.0, // Still active = full sustain
        };

        // Convert microseconds to beats
        let duration_beats = duration_us as f64 / self.us_per_beat();

        // Normalize to slot duration
        let normalized = duration_beats / quantize_duration;

        // Clamp to 0.0-1.0 range
        normalized.min(1.0).max(0.0) as f32
    }

    /// Helper: Get microseconds per beat based on tempo
    fn us_per_beat(&self) -> f64 {
        60_000_000.0 / self.tempo
    }
}
```

**Example Calculation**:
```
Tempo: 120 BPM (2 CPS)
Quantize: 16th notes (0.25 beats)
Slot duration: 0.25 beats = 125,000 μs

Note held for 100,000 μs:
  duration_beats = 100,000 / 500,000 = 0.2 beats
  normalized = 0.2 / 0.25 = 0.8
  legato = 0.8 (80% of slot)

Note held for 125,000 μs or more:
  legato = 1.0 (full sustain)
```

---

### Step 3: Update RecordedPattern (15 min)

**File**: `src/midi_input.rs`

**Add legato field**:
```rust
#[derive(Debug, Clone)]
pub struct RecordedPattern {
    pub notes: String,          // "c4 e4 g4"
    pub n_offsets: String,      // "0 4 7"
    pub velocities: String,     // "0.79 1.00 0.63"
    pub legato: String,         // "0.9 0.5 1.0"  ← NEW
    pub base_note: u8,
    pub base_note_name: String,
    pub cycle_count: usize,
    pub quantize_division: u8,
}
```

**Update stop_recording() to generate legato**:
```rust
pub fn stop_recording(&mut self) -> Option<RecordedPattern> {
    // ...existing code to generate notes, velocities...

    // Generate legato pattern
    let legato = self.generate_legato_pattern();

    Some(RecordedPattern {
        notes,
        n_offsets,
        velocities,
        legato,  // ← NEW
        base_note,
        base_note_name,
        cycle_count,
        quantize_division: self.quantize_division,
    })
}
```

---

### Step 4: Generate Legato Pattern (1 hour)

**Add to MidiRecorder**:
```rust
impl MidiRecorder {
    /// Generate legato pattern string aligned with note pattern
    /// Returns: "0.9 0.5 1.0 ~ 0.8" (with rests aligned)
    fn generate_legato_pattern(&self) -> String {
        let mut legato_values = Vec::new();

        // Get quantized grid
        let grid_slots = self.calculate_grid_slots();

        for slot in grid_slots {
            if let Some(note_event) = slot.note_event {
                // Calculate legato for this note
                let legato = self.calculate_legato(&note_event, slot.duration_beats);
                legato_values.push(format!("{:.2}", legato));
            } else {
                // Rest
                if slot.rest_duration > 1 {
                    legato_values.push(format!("~@{}", slot.rest_duration));
                } else {
                    legato_values.push("~".to_string());
                }
            }
        }

        legato_values.join(" ")
    }
}
```

**Grid Slot Structure**:
```rust
struct GridSlot {
    note_event: Option<NoteEvent>,
    duration_beats: f64,
    rest_duration: usize,  // 1 for ~, N for ~@N
}
```

---

### Step 5: Update Smart Paste (30 min)

**File**: `src/modal_editor/mod.rs`

**Modify insert_midi_smart_paste()**:
```rust
fn insert_midi_smart_paste(&mut self) {
    if let Some(ref pattern) = self.midi_recorded_pattern.clone() {
        if let Some(ref velocity) = self.midi_recorded_velocity.clone() {
            if let Some(ref legato) = self.midi_recorded_legato.clone() {  // ← NEW
                self.recording_counter += 1;
                let rec_name = format!("~rec{}", self.recording_counter);

                let slow_wrapper = if self.midi_recorded_cycles > 1 {
                    format!("slow {} $ ", self.midi_recorded_cycles)
                } else {
                    String::new()
                };

                // Build complete pattern with legato
                let full_pattern = format!(
                    "{}: {}n \"{}\"\n       # gain \"{}\"\n       # legato \"{}\"",
                    rec_name,
                    slow_wrapper,
                    pattern,
                    velocity,
                    legato  // ← NEW
                );

                for c in full_pattern.chars() {
                    self.insert_char(c);
                }

                self.status_message = format!(
                    "📝 Inserted {} with dynamics + legato",
                    rec_name
                );
            }
        }
    }
}
```

**Add legato field to ModalEditor**:
```rust
pub struct ModalEditor {
    // ...existing fields...
    midi_recorded_legato: Option<String>,  // ← NEW
}
```

**Initialize in constructor**:
```rust
Self {
    // ...
    midi_recorded_legato: None,
    // ...
}
```

**Update recording stop handler**:
```rust
// When recording stops:
self.midi_recorded_pattern = Some(recorded.notes.clone());
self.midi_recorded_n_pattern = Some(recorded.n_offsets.clone());
self.midi_recorded_velocity = Some(recorded.velocities.clone());
self.midi_recorded_legato = Some(recorded.legato.clone());  // ← NEW
```

---

### Step 6: Add Legato-Only Paste (20 min)

**Add Alt+L keybinding**:
```rust
// Alt+L: Insert recorded MIDI legato pattern
KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::ALT) => {
    self.insert_midi_legato_pattern();
    KeyResult::Continue
}
```

**Add insert function**:
```rust
fn insert_midi_legato_pattern(&mut self) {
    if let Some(ref pattern) = self.midi_recorded_legato.clone() {
        let pattern_str = format!("\"{}\"", pattern);
        for c in pattern_str.chars() {
            self.insert_char(c);
        }
        self.status_message = format!("📝 Inserted legato: {}", pattern_str);
    } else {
        self.status_message = "🎹 No recorded pattern (Alt+R to record)".to_string();
    }
}
```

**Update help text** (`src/modal_editor/command_console.rs`):
```rust
self.output.push("  Alt+L  - Insert recorded legato (note durations)".to_string());
```

---

### Step 7: Testing (1 hour)

**Create test file**: `tests/test_legato_capture.rs`

```rust
use phonon::midi_input::{MidiRecorder, MidiEvent, MidiMessageType};

#[test]
fn test_legato_short_notes_staccato() {
    // Test: Short notes (50% duration) → legato ~0.5
    let mut recorder = MidiRecorder::new(120.0); // 120 BPM
    recorder.start_recording();

    // Note-on at 0ms
    recorder.record_event_at(60, 100, 0);

    // Note-off at 125ms (half of 250ms quarter note)
    recorder.record_event_at_off(60, 125_000);

    let pattern = recorder.stop_recording().unwrap();

    // Should be ~0.5 (50% of slot duration)
    assert!(pattern.legato.contains("0.5"),
        "Expected legato ~0.5 for short note, got: {}", pattern.legato);
}

#[test]
fn test_legato_full_sustain() {
    // Test: Long notes (100%+ duration) → legato 1.0
    let mut recorder = MidiRecorder::new(120.0);
    recorder.start_recording();

    // Note-on at 0ms
    recorder.record_event_at(60, 100, 0);

    // Note-off at 250ms (full quarter note or more)
    recorder.record_event_at_off(60, 250_000);

    let pattern = recorder.stop_recording().unwrap();

    // Should be 1.0 (100% sustain)
    assert!(pattern.legato.contains("1.0"),
        "Expected legato 1.0 for sustained note, got: {}", pattern.legato);
}

#[test]
fn test_legato_alignment_with_rests() {
    // Test: Legato pattern aligns with note pattern (including rests)
    let mut recorder = MidiRecorder::new(120.0);
    recorder.start_recording();

    // C4 for 200ms (0.8 legato)
    recorder.record_event_at(60, 100, 0);
    recorder.record_event_at_off(60, 200_000);

    // Rest (500ms pause)

    // E4 for 125ms (0.5 legato)
    recorder.record_event_at(64, 100, 700_000);
    recorder.record_event_at_off(64, 825_000);

    let pattern = recorder.stop_recording().unwrap();

    // Should have: note, rest, note
    assert!(pattern.notes.contains("~"), "Should have rest in notes");
    assert!(pattern.legato.contains("~"), "Should have rest in legato");

    // Should have similar structure
    let note_parts: Vec<&str> = pattern.notes.split_whitespace().collect();
    let legato_parts: Vec<&str> = pattern.legato.split_whitespace().collect();
    assert_eq!(note_parts.len(), legato_parts.len(),
        "Legato pattern should align with note pattern");
}

#[test]
fn test_legato_pattern_with_velocities() {
    // Test: All three patterns (notes, velocities, legato) align
    let mut recorder = MidiRecorder::new(120.0);
    recorder.start_recording();

    // Soft, short note
    recorder.record_event_at(60, 50, 0);
    recorder.record_event_at_off(60, 100_000);

    // Loud, sustained note
    recorder.record_event_at(64, 127, 250_000);
    recorder.record_event_at_off(64, 500_000);

    let pattern = recorder.stop_recording().unwrap();

    // All three should have 2 values
    assert_eq!(pattern.notes.split_whitespace().count(), 2);
    assert_eq!(pattern.velocities.split_whitespace().count(), 2);
    assert_eq!(pattern.legato.split_whitespace().count(), 2);
}
```

**Add helper methods to MidiRecorder**:
```rust
#[cfg(test)]
impl MidiRecorder {
    /// Test helper: Record note-off event at specific time
    pub fn record_event_at_off(&mut self, note: u8, timestamp_us: u64) {
        self.record_event(MidiEvent {
            message_type: MidiMessageType::NoteOff { note, velocity: 0 },
            channel: 0,
            timestamp_us,
            message: vec![0x80, note, 0],
        });
    }
}
```

---

### Step 8: Documentation (30 min)

**Create example**: `docs/examples/legato_expression.ph`

```phonon
-- Legato Expression Example
-- Shows how note duration affects musical expression

tempo: 2.0

-- Example 1: Staccato (short, detached notes)
~staccato: n "c4 e4 g4 c5"
           # gain "1.0 1.0 1.0 1.0"
           # legato "0.3 0.3 0.3 0.3"  -- 30% duration

-- Example 2: Normal articulation
~normal: n "c4 e4 g4 c5"
         # gain "1.0 1.0 1.0 1.0"
         # legato "0.7 0.7 0.7 0.7"    -- 70% duration

-- Example 3: Legato (smooth, connected)
~legato: n "c4 e4 g4 c5"
         # gain "1.0 1.0 1.0 1.0"
         # legato "1.0 1.0 1.0 1.0"    -- 100% duration (tied)

-- Example 4: Mixed expression (recorded from MIDI)
~expressive: n "c4 d4 e4 f4 g4"
             # gain "0.8 0.6 1.0 0.9 0.7"
             # legato "0.9 0.5 1.0 0.8 0.6"
             --        ↑   ↑   ↑   ↑   ↑
             --      long short tied medium short

out: ~staccato * 0.3 + ~legato * 0.3 + ~expressive * 0.4
```

**Update MIDI_RECORDING_GUIDE.md**:
```markdown
### Legato Capture (Phase 2)

Legato captures how long you hold each note:
- Short notes (staccato): legato ~ 0.3-0.5
- Normal notes: legato ~ 0.7
- Sustained notes (legato): legato ~ 1.0

**Usage**:
1. Record pattern (Alt+R)
2. Play with expression (varying note lengths)
3. Smart paste (Alt+Shift+I) includes legato
4. Or paste legato only (Alt+L)

**Result**:
```phonon
~melody: n "c4 e4 g4"
         # gain "0.8 1.0 0.6"
         # legato "0.9 0.5 1.0"  ← Note duration
```
```

---

## Timeline Estimate

| Step | Task | Time | Cumulative |
|------|------|------|------------|
| 1 | Enhanced note tracking | 30 min | 30 min |
| 2 | Legato calculation | 45 min | 1h 15min |
| 3 | Update RecordedPattern | 15 min | 1h 30min |
| 4 | Generate legato pattern | 1 hour | 2h 30min |
| 5 | Update smart paste | 30 min | 3h |
| 6 | Add Alt+L keybinding | 20 min | 3h 20min |
| 7 | Testing | 1 hour | 4h 20min |
| 8 | Documentation | 30 min | 4h 50min |

**Total**: ~5 hours of focused work

## Success Criteria

- ✅ Note duration captured (note-on to note-off timing)
- ✅ Legato values calculated correctly (0.0-1.0 range)
- ✅ Legato pattern aligns with note/velocity patterns
- ✅ Smart paste includes legato line
- ✅ Alt+L keybinding works
- ✅ All tests passing
- ✅ Documentation complete with examples

## Testing Strategy

### Unit Tests
- Legato calculation (various durations)
- Short notes → low legato values
- Long notes → high legato values
- Pattern alignment (notes, velocities, legato)

### Integration Tests
- End-to-end recording with legato
- Smart paste format verification
- Multi-cycle legato patterns

### Manual Testing
```bash
# 1. Launch editor
cargo run --release --bin phonon -- edit

# 2. Record with varied note lengths
#    - Play some short, staccato notes
#    - Play some long, sustained notes
#    Alt+R (start), play, Alt+R (stop)

# 3. Smart paste (Alt+Shift+I)
#    Should see legato line with varied values

# 4. Paste legato only (Alt+L)
#    Should insert just the legato pattern
```

## Current Status

- **Phase 0**: ✅ Complete (MIDI recording)
- **Phase 1**: ✅ Complete (MIDI monitoring)
- **Phase 2**: 🚧 Starting now (Note duration / legato)
- **Phase 3**: 📋 Planned (Punch-in recording)
- **Phase 4**: 📋 Planned (Multi-line formatting)

**Let's build expressive pattern capture! 🎹✨**
