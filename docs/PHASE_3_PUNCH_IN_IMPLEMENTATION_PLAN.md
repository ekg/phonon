# Phase 3: Punch-in Recording - Implementation Plan

**Goal**: Record MIDI while audio is playing, synced to the current playback cycle

**Estimated Time**: 2 days (16 hours)

**Status**: Ready to implement

---

## Problem Statement

**Current Limitation**: MIDI recording always starts from "beat 0" (when Alt+R is pressed). This means:
- Cannot record while a pattern is already playing
- Cannot overdub on top of existing patterns
- Timestamps are relative to recording start, not playback grid

**Desired Behavior**: Press Alt+R at any time during playback (e.g., cycle 2.5), play MIDI keyboard, press Alt+R again. The recorded pattern should be aligned to the playback grid, not the arbitrary moment recording started.

---

## Technical Architecture

### Current Flow (Phase 0-2)
```
User presses Alt+R (recording starts)
  ↓
record_start_time = now()  // Arbitrary wall-clock time
  ↓
MIDI events arrive with timestamps
  ↓
Quantize events relative to record_start_time
  ↓
Generate pattern (beats counted from recording start)
```

**Problem**: Recording start time has no relationship to the playback cycle grid.

### New Flow (Phase 3)
```
Audio is playing → cycle 2.347 (graph keeps track)
  ↓
User presses Alt+R (recording starts mid-cycle)
  ↓
record_start_cycle = get_current_cycle()  // e.g., 2.347
  ↓
MIDI events arrive with timestamps
  ↓
Adjust timestamps: event_cycle = record_start_cycle + elapsed_cycles
  ↓
Quantize events relative to cycle 0 (absolute grid)
  ↓
Generate pattern (beats counted from cycle 0, not recording start)
```

**Solution**: Recording syncs to the playback cycle position.

---

## Implementation Steps

### Step 1: Add Cycle Position Tracking to UnifiedSignalGraph

**File**: `src/unified_graph.rs`

**Changes**:
```rust
pub struct UnifiedSignalGraph {
    // ... existing fields ...
    current_cycle: Arc<AtomicU64>,  // Current cycle position (as f64 bits)
    sample_rate: f32,
    tempo_bpm: f32,
    beats_per_cycle: f32,
}

impl UnifiedSignalGraph {
    /// Get current cycle position (thread-safe, callable from any thread)
    pub fn get_current_cycle(&self) -> f64 {
        f64::from_bits(self.current_cycle.load(Ordering::Relaxed))
    }

    /// Update cycle position during render (called from audio thread)
    fn update_cycle_position(&self, samples_rendered: usize) {
        let samples_per_cycle = (self.sample_rate * 60.0 * self.beats_per_cycle) / self.tempo_bpm;
        let cycles_elapsed = samples_rendered as f64 / samples_per_cycle as f64;

        let old_cycle = self.get_current_cycle();
        let new_cycle = old_cycle + cycles_elapsed;

        self.current_cycle.store(new_cycle.to_bits(), Ordering::Relaxed);
    }

    pub fn render(&mut self, num_samples: usize) -> Vec<f32> {
        // ... existing render logic ...

        // Update cycle position after rendering
        self.update_cycle_position(num_samples);

        buffer
    }
}
```

**Why AtomicU64 + f64::to_bits()**:
- AtomicF64 doesn't exist in stable Rust
- Store f64 as bits in AtomicU64 (common pattern)
- Thread-safe reads from UI thread without locks

**Testing Strategy**:
```rust
#[test]
fn test_cycle_position_tracking() {
    let mut graph = create_test_graph(44100.0, 120.0, 4.0); // 120 BPM, 4 beats/cycle

    assert_eq!(graph.get_current_cycle(), 0.0);

    // Render 1 cycle worth of samples
    let samples_per_cycle = (44100.0 * 60.0 * 4.0) / 120.0; // = 88200 samples
    graph.render(samples_per_cycle as usize);

    assert!((graph.get_current_cycle() - 1.0).abs() < 0.001);

    // Render another cycle
    graph.render(samples_per_cycle as usize);

    assert!((graph.get_current_cycle() - 2.0).abs() < 0.001);
}
```

---

### Step 2: Update MidiRecorder to Accept Cycle Offset

**File**: `src/midi_input.rs`

**Changes**:
```rust
pub struct MidiRecorder {
    events: Vec<MidiEvent>,
    start_time: Instant,
    tempo_bpm: f64,
    quantize_division: u8,
    recording_start_us: u64,
    recording_start_cycle: f64,  // ← NEW: Cycle position when recording started
    active_notes: HashMap<u8, NoteEvent>,
    completed_notes: Vec<NoteEvent>,
}

impl MidiRecorder {
    pub fn start(&mut self) {
        self.events.clear();
        self.start_time = Instant::now();
        self.recording_start_us = 0;
        self.recording_start_cycle = 0.0;  // ← Will be set by caller
        self.active_notes.clear();
        self.completed_notes.clear();
    }

    /// Start recording at a specific cycle position (for punch-in)
    pub fn start_at_cycle(&mut self, cycle_position: f64) {
        self.start();
        self.recording_start_cycle = cycle_position;
    }

    /// Convert timestamp to absolute cycle position (accounting for punch-in offset)
    fn timestamp_to_cycle(&self, timestamp_us: u64) -> f64 {
        let relative_us = timestamp_us.saturating_sub(self.recording_start_us);
        let us_per_beat = 60_000_000.0 / self.tempo_bpm;
        let beats_per_cycle = 4.0; // TODO: Make configurable
        let relative_cycles = (relative_us as f64 / us_per_beat) / beats_per_cycle;

        // Add recording start offset to get absolute cycle position
        self.recording_start_cycle + relative_cycles
    }

    /// Quantize a cycle position to the nearest grid division
    fn quantize_cycle(&self, cycle: f64) -> f64 {
        let beats_per_cycle = 4.0;
        let slots_per_cycle = self.quantize_division as f64;
        let slot_duration = beats_per_cycle / slots_per_cycle;

        // Quantize to absolute grid (not relative to recording start)
        (cycle / slot_duration).round() * slot_duration
    }
}
```

**Key Changes**:
- `recording_start_cycle` field tracks when recording started (in cycles)
- `timestamp_to_cycle()` adds offset to get absolute cycle position
- `quantize_cycle()` quantizes to absolute grid (not recording-relative)

**Testing Strategy**:
```rust
#[test]
fn test_punch_in_cycle_offset() {
    let mut recorder = MidiRecorder::new(120.0);
    recorder.set_quantize(4); // Quarter notes

    // Start recording at cycle 2.5 (mid-playback)
    recorder.start_at_cycle(2.5);

    // Record note at relative time 0 (should map to cycle 2.5)
    recorder.record_event_at(60, 100, 0);
    recorder.record_event_at(60, 0, 100_000);

    // Record note at relative time 500ms (should map to cycle ~3.5)
    recorder.record_event_at(62, 100, 500_000);
    recorder.record_event_at(62, 0, 600_000);

    let pattern = recorder.to_recorded_pattern(4.0).unwrap();

    // Pattern should be quantized to cycle grid
    // Cycle 2.5 → quantizes to cycle 2.5 (or 3.0 depending on rounding)
    // Verify notes are in correct slots relative to cycle 0
}
```

---

### Step 3: Integrate Cycle Tracking with ModalEditor

**File**: `src/modal_editor/mod.rs`

**Changes**:
```rust
pub struct ModalEditor {
    // ... existing fields ...
    unified_graph: Option<Arc<RwLock<UnifiedSignalGraph>>>,  // ← Share graph reference
}

impl ModalEditor {
    /// Toggle MIDI recording (punch-in aware)
    fn toggle_midi_recording(&mut self) {
        if !self.midi_recording {
            // Start recording
            if let Some(ref mut recorder) = self.midi_recorder {
                // Get current cycle position from graph
                let current_cycle = self.unified_graph
                    .as_ref()
                    .map(|g| g.read().unwrap().get_current_cycle())
                    .unwrap_or(0.0);

                recorder.start_at_cycle(current_cycle);
                self.midi_recording = true;

                self.status_message = format!(
                    "🔴 Recording MIDI (started at cycle {:.2})...",
                    current_cycle
                );
            }
        } else {
            // Stop recording
            self.midi_recording = false;

            // Convert recorded events to pattern
            if let Some(ref recorder) = self.midi_recorder {
                let beats_per_cycle = 4.0;

                if let Some(recorded) = recorder.to_recorded_pattern(beats_per_cycle) {
                    self.midi_recorded_pattern = Some(recorded.notes.clone());
                    self.midi_recorded_n_pattern = Some(recorded.n_offsets.clone());
                    self.midi_recorded_velocity = Some(recorded.velocities.clone());
                    self.midi_recorded_legato = Some(recorded.legato.clone());
                    self.midi_recorded_base_note = Some(recorded.base_note_name.clone());
                    self.midi_recorded_cycles = recorded.cycle_count;

                    self.status_message = format!(
                        "✅ Recorded {} cycles (punch-in aligned to grid)",
                        recorded.cycle_count
                    );
                }
            }
        }
    }
}
```

**Key Changes**:
- Store reference to UnifiedSignalGraph (for cycle position)
- Call `start_at_cycle()` with current cycle position when recording starts
- Update status message to show punch-in cycle

---

### Step 4: Visual Feedback During Recording

**File**: `src/modal_editor/mod.rs`

**Enhancement**: Show cycle position in status line during recording

```rust
impl ModalEditor {
    /// Update status message during recording (called in main loop)
    fn update_recording_status(&mut self) {
        if self.midi_recording {
            if let Some(ref recorder) = self.midi_recorder {
                let current_cycle = self.unified_graph
                    .as_ref()
                    .map(|g| g.read().unwrap().get_current_cycle())
                    .unwrap_or(0.0);

                let start_cycle = recorder.recording_start_cycle;
                let elapsed_cycles = current_cycle - start_cycle;

                self.status_message = format!(
                    "🔴 Recording... Cycle {:.2} (elapsed: {:.2} cycles)",
                    current_cycle,
                    elapsed_cycles
                );
            }
        }
    }

    // Call this in the main event loop
    pub fn tick(&mut self) {
        if self.midi_recording {
            self.update_recording_status();
        }
    }
}
```

---

### Step 5: Update Pattern Generation for Cycle Alignment

**File**: `src/midi_input.rs`

**Changes to `to_recorded_pattern()`**:

```rust
pub fn to_recorded_pattern(&self, beats_per_cycle: f64) -> Option<RecordedPattern> {
    if self.events.is_empty() {
        return None;
    }

    // Collect note-on events with absolute cycle positions
    let note_ons: Vec<_> = self
        .events
        .iter()
        .filter_map(|e| match &e.message_type {
            MidiMessageType::NoteOn { note, velocity } if *velocity > 0 => {
                let cycle = self.timestamp_to_cycle(e.timestamp_us);
                Some((*note, *velocity, cycle))
            }
            _ => None,
        })
        .collect();

    if note_ons.is_empty() {
        return None;
    }

    // Find lowest note for n-offsets
    let base_note = note_ons.iter().map(|(n, _, _)| *n).min().unwrap();
    let base_note_name = MidiEvent::midi_to_note_name(base_note);

    // Calculate grid based on absolute cycles (not recording-relative)
    let slots_per_cycle = self.quantize_division as usize;
    let slot_duration_cycles = beats_per_cycle / slots_per_cycle as f64;

    // Find first and last cycles
    let first_cycle = note_ons.first().map(|(_, _, c)| *c).unwrap();
    let last_cycle = note_ons.last().map(|(_, _, c)| *c).unwrap();

    // Quantize to cycle boundaries
    let start_cycle = (first_cycle / beats_per_cycle).floor() * beats_per_cycle;
    let end_cycle = ((last_cycle / beats_per_cycle).ceil() * beats_per_cycle).max(start_cycle + beats_per_cycle);

    let num_cycles = ((end_cycle - start_cycle) / beats_per_cycle).ceil() as usize;
    let total_slots = slots_per_cycle * num_cycles;

    // Grid stores (notes, velocities) per slot (absolute positioning)
    let mut grid: Vec<Option<Vec<(u8, u8)>>> = vec![None; total_slots];

    for (note, velocity, cycle) in &note_ons {
        let quantized_cycle = self.quantize_cycle(*cycle);

        // Map to slot index (relative to start_cycle)
        let relative_cycle = quantized_cycle - start_cycle;
        let slot = ((relative_cycle / slot_duration_cycles).round() as usize)
            .min(total_slots.saturating_sub(1));

        if let Some(ref mut events) = grid[slot] {
            events.push((*note, *velocity));
        } else {
            grid[slot] = Some(vec![(*note, *velocity)]);
        }
    }

    // ... rest of pattern generation (same as before) ...
}
```

**Key Changes**:
- Use absolute cycle positions (from `timestamp_to_cycle()`)
- Quantize to absolute grid (not recording-relative)
- Align pattern to cycle boundaries

---

## Testing Strategy

### Test 1: Cycle Position Tracking

**Mock User Interaction**:
```rust
#[test]
fn test_cycle_tracking_accuracy() {
    let mut graph = create_test_graph(44100.0, 120.0, 4.0);

    // Simulate playback over 10 cycles
    let samples_per_cycle = calculate_samples_per_cycle(44100.0, 120.0, 4.0);

    for cycle_num in 0..10 {
        graph.render(samples_per_cycle);
        let actual_cycle = graph.get_current_cycle();
        let expected_cycle = cycle_num as f64 + 1.0;

        assert!((actual_cycle - expected_cycle).abs() < 0.001,
            "Cycle tracking drift at cycle {}", cycle_num);
    }
}
```

### Test 2: Punch-in at Arbitrary Cycle

**Mock User Interaction**:
```rust
#[test]
fn test_punch_in_mid_cycle() {
    let mut recorder = MidiRecorder::new(120.0);
    recorder.set_quantize(4);

    // USER ACTION: Starts recording at cycle 2.5
    recorder.start_at_cycle(2.5);

    // USER ACTION: Plays C4 immediately
    recorder.record_event_at(60, 100, 0);        // Note-on
    recorder.record_event_at(60, 0, 400_000);    // Note-off

    // USER ACTION: Plays D4 after 500ms
    recorder.record_event_at(62, 100, 500_000);  // Note-on
    recorder.record_event_at(62, 0, 900_000);    // Note-off

    let pattern = recorder.to_recorded_pattern(4.0).unwrap();

    // EXPECTED: Pattern aligned to cycle boundaries
    // C4 at cycle 2.5 → quantized
    // D4 at cycle ~3.5 → quantized
    // Pattern should show proper slot alignment
}
```

### Test 3: Multi-Cycle Punch-in Recording

**Mock User Interaction**:
```rust
#[test]
fn test_punch_in_multi_cycle() {
    let mut recorder = MidiRecorder::new(120.0);
    recorder.set_quantize(4);

    // USER ACTION: Starts recording at cycle 1.7
    recorder.start_at_cycle(1.7);

    // USER ACTION: Plays 8 notes over 2 cycles
    for i in 0..8 {
        let note = 60 + i;
        let timestamp_us = i * 250_000; // 250ms apart
        recorder.record_event_at(note, 100, timestamp_us);
        recorder.record_event_at(note, 0, timestamp_us + 200_000);
    }

    let pattern = recorder.to_recorded_pattern(4.0).unwrap();

    // EXPECTED: Pattern starts at cycle 1.7, spans 2 cycles
    assert_eq!(pattern.cycle_count, 2);

    // EXPECTED: All notes quantized to absolute grid
    // Verify slot positions are correct
}
```

### Test 4: Punch-in + Punch-out Cycle Alignment

**Mock Full User Workflow**:
```rust
#[test]
fn test_complete_punch_in_workflow() {
    // SETUP: Audio is playing, user wants to add overdub
    let mut graph = create_test_graph(44100.0, 120.0, 4.0);
    let mut recorder = MidiRecorder::new(120.0);
    recorder.set_quantize(4);

    // PLAYBACK: Render 2.5 cycles
    let samples_per_cycle = calculate_samples_per_cycle(44100.0, 120.0, 4.0);
    graph.render((samples_per_cycle as f32 * 2.5) as usize);

    let punch_in_cycle = graph.get_current_cycle();
    assert!((punch_in_cycle - 2.5).abs() < 0.001);

    // USER ACTION: Press Alt+R (punch-in)
    recorder.start_at_cycle(punch_in_cycle);

    // USER ACTION: Play melody
    recorder.record_event_at(60, 100, 0);
    recorder.record_event_at(60, 0, 400_000);
    recorder.record_event_at(62, 100, 500_000);
    recorder.record_event_at(62, 0, 900_000);
    recorder.record_event_at(64, 100, 1_000_000);
    recorder.record_event_at(64, 0, 1_400_000);

    // PLAYBACK: Continue playback
    graph.render(samples_per_cycle);

    let punch_out_cycle = graph.get_current_cycle();
    assert!((punch_out_cycle - 3.5).abs() < 0.1);

    // USER ACTION: Press Alt+R (punch-out / stop recording)
    let pattern = recorder.to_recorded_pattern(4.0).unwrap();

    // VERIFICATION: Pattern aligned to cycle boundaries
    assert!(pattern.cycle_count >= 1);

    // USER ACTION: Smart paste (Alt+Shift+I)
    // Result should align with existing playback grid
}
```

### Test 5: Zero-Drift Over Long Recording

**Mock User Interaction**:
```rust
#[test]
fn test_no_timing_drift_long_recording() {
    let mut graph = create_test_graph(44100.0, 120.0, 4.0);
    let mut recorder = MidiRecorder::new(120.0);
    recorder.set_quantize(4);

    // PLAYBACK: Run for 100 cycles
    let samples_per_cycle = calculate_samples_per_cycle(44100.0, 120.0, 4.0);
    for _ in 0..100 {
        graph.render(samples_per_cycle);
    }

    let final_cycle = graph.get_current_cycle();
    assert!((final_cycle - 100.0).abs() < 0.01,
        "Timing drift detected: expected 100.0, got {}", final_cycle);

    // USER ACTION: Punch-in at cycle 100
    recorder.start_at_cycle(final_cycle);

    // Record notes
    recorder.record_event_at(60, 100, 0);
    recorder.record_event_at(60, 0, 400_000);

    let pattern = recorder.to_recorded_pattern(4.0).unwrap();

    // VERIFICATION: Pattern correctly aligned even after long playback
}
```

---

## Success Criteria

- ✅ Graph tracks cycle position accurately (< 0.001 cycle drift)
- ✅ Recording starts at arbitrary cycle position
- ✅ Events quantized to absolute cycle grid (not recording-relative)
- ✅ Visual feedback shows current cycle during recording
- ✅ Smart paste produces cycle-aligned patterns
- ✅ No timing drift over long recordings
- ✅ All existing MIDI tests still pass

---

## Timeline

**Day 1** (8 hours):
- [ ] Step 1: Cycle position tracking (2 hours)
- [ ] Step 2: MidiRecorder cycle offset (2 hours)
- [ ] Step 3: ModalEditor integration (2 hours)
- [ ] Test suite setup (2 hours)

**Day 2** (8 hours):
- [ ] Step 4: Visual feedback (2 hours)
- [ ] Step 5: Pattern alignment (3 hours)
- [ ] Comprehensive testing (2 hours)
- [ ] Documentation (1 hour)

---

## Next Steps

1. Implement cycle position tracking in UnifiedSignalGraph
2. Write test to verify cycle tracking accuracy
3. Update MidiRecorder with cycle offset support
4. Integrate with ModalEditor
5. Add visual feedback
6. Comprehensive testing with mocked user interaction
7. Update documentation

**Ready to implement!** 🚀
